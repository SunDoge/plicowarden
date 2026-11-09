//! Synthetic native integration probe, paired with scripts/check-native-tray.py.
//! No credentials, SDK connection or user preferences are loaded by this host.
use plicowarden::{Warden, host::Host};
fn main() -> Result<(), plico_desktop::DesktopError> {
    let (desktop, tray) = plicowarden::desktop::integration();
    desktop.notify_ssh();
    let host = Host::new(Warden::default(), false).with_desktop(desktop);
    plico_desktop::run_source_with_extensions(
        plico_runtime::Source::new(host),
        plicowarden::display::window_options(false),
        vec![Box::new(tray)],
    )
    .map(|_| ())
}
