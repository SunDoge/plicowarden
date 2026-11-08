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
    plico_desktop::run_host_view_with_options(
        Host::new(Warden::default(), true),
        plico_desktop::WindowOptions::new()
            .title("Plicowarden")
            .initial_inner_size(1440, 1000)
            .min_inner_size(820, 640),
    )
    .context(DesktopSnafu)
}
