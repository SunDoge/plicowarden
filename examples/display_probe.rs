//! Synthetic density/Retina probe; never starts an account or preferences worker.
use plico_desktop::NativeApp;
use plico_runtime::{Source, ZoomSource};
use plicowarden::Warden;
use snafu::{ResultExt, Snafu};
use std::path::PathBuf;

#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("Cannot create output directory: {source}"))]
    Io { source: std::io::Error },
    #[snafu(display("Display capture failed: {source}"))]
    Capture { source: plico_desktop::CaptureError },
}

#[derive(usage_rs::Cli)]
struct Options {
    #[usage(long)]
    output: PathBuf,
}
fn main() -> Result<(), Error> {
    let options = Options::parse();
    std::fs::create_dir_all(&options.output).context(IoSnafu)?;
    for (name, factor, backing, size, busy) in [
        ("compact-1x", 0.85, 1., (1224, 850), false),
        ("compact-2x", 0.85, 2., (1224, 850), false),
        ("full-2x", 1., 2., (1224, 850), false),
        ("busy-retina", 1., 2., (1440, 1000), true),
    ] {
        let mut app = Warden::default();
        app.busy = busy;
        app.zoom.set(factor);
        let zoom = app.zoom.clone();
        let source = Source::with_clock(NativeApp(app), plico::VirtualClock::default());
        let mut source = ZoomSource::with_controller(Box::new(source), zoom);
        plico_desktop::capture(&mut source)
            .size(size.0, size.1)
            .scale(backing)
            .renderer(plico_desktop::RenderBackend::Wgpu)
            .write_png(options.output.join(format!("{name}.png")))
            .context(CaptureSnafu)?;
    }
    Ok(())
}
