use plicowarden::{PreviewConfig, Warden, host::Host};
use snafu::{ResultExt, Snafu};
use std::path::PathBuf;
#[derive(usage_rs::Cli)]
#[usage(about = "Plicowarden — a native Rust Bitwarden client using Plico")]
struct Options {
    #[usage(long)]
    capture: Option<PathBuf>,
    #[usage(long)]
    scenario: Option<PathBuf>,
    #[usage(long, default = "1440")]
    width: u32,
    #[usage(long, default = "1000")]
    height: u32,
    #[usage(long, default = "vault")]
    screen: String,
}
#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("Preview failed: {source}"))]
    Preview { source: plico_desktop::PreviewError },
    #[snafu(display("Capture failed: {source}"))]
    Capture { source: plico_desktop::CaptureError },
    #[snafu(display("Desktop failed: {source}"))]
    Desktop { source: plico_desktop::DesktopError },
}
fn main() -> Result<(), Error> {
    let options = Options::parse();
    if let Some(output) = options.capture {
        let from_file = options.scenario.is_some();
        let mut scenario = if let Some(path) = options.scenario {
            plico_desktop::PreviewScenario::<PreviewConfig>::load(path).context(PreviewSnafu)?
        } else {
            plico_desktop::PreviewScenario::<PreviewConfig>::default()
        };
        if scenario.state.screen.is_empty() {
            scenario.state.screen = options.screen;
        }
        // Capture only synthetic fixtures; this code path never opens an SDK client or loads preferences.
        if !from_file {
            scenario.viewport.width = options.width;
            scenario.viewport.height = options.height;
        }
        scenario
            .render(|config| plico_desktop::NativeApp(config.app()))
            .context(PreviewSnafu)?
            .write_png(output)
            .context(CaptureSnafu)?;
        return Ok(());
    }
    let (desktop, tray) = plicowarden::desktop::integration();
    let host = Host::new(Warden::default(), true).with_desktop(desktop);
    let zoom = host.app.zoom.clone();
    plico_desktop::run_source_with_extensions(
        plico_runtime::ZoomSource::with_controller(
            Box::new(plico_runtime::Source::new(host)),
            zoom,
        ),
        plicowarden::display::window_options(cfg!(target_os = "macos")),
        vec![Box::new(tray)],
    )
    .map(|_| ())
    .context(DesktopSnafu)
}
