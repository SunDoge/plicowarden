use plico_renderer::FrameSource;
use plico_runtime::{
    HostView,
    testing::{Command, Driver},
};
use plicowarden::{Warden, host::Host};
use std::{
    sync::{Arc, mpsc},
    time::Duration,
};
fn click(ui: &mut Driver<Host>, label: &str) {
    ui.execute(Command::Click {
        role: "button".into(),
        label: label.into(),
        occurrence: None,
    })
    .unwrap();
}
fn type_text(ui: &mut Driver<Host>, label: &str, value: &str) {
    ui.execute(Command::Focus {
        role: "textbox".into(),
        label: label.into(),
        occurrence: None,
    })
    .unwrap();
    ui.execute(Command::Text { text: value.into() }).unwrap();
}
#[test]
fn rejected_http_login_wakes_ui_clears_password_and_can_return_to_demo() {
    let (tx, rx) = mpsc::channel();
    let mut host = Host::new(Warden::default(), false);
    host.set_wake_callback(Arc::new(move || {
        let _ = tx.send(());
    }));
    let mut ui = Driver::new(host, 1440, 1000).unwrap();
    click(&mut ui, "Connect account");
    type_text(&mut ui, "Server URL", "http://example.invalid");
    type_text(&mut ui, "Email address", "test@example.invalid");
    type_text(&mut ui, "Master password", "synthetic-test-password");
    let tree = serde_json::to_string(&ui.declarations()).unwrap();
    assert!(!tree.contains("synthetic-test-password"));
    click(&mut ui, "Sign in");
    assert!(ui.source.view.app.password.is_empty());
    // Completion notification, not sleeping/polling for an arbitrary amount of time.
    while ui.source.view.app.busy {
        rx.recv_timeout(Duration::from_secs(60))
            .expect("account worker did not finish");
        ui.source.poll_async();
        ui.execute(Command::Snapshot).unwrap();
    }
    assert!(ui.source.view.app.error.contains("HTTPS"));
    assert!(!ui.source.view.app.authenticated);
    click(&mut ui, "Browse demo vault");
    assert_eq!(ui.source.view.app.visible().len(), 12);
    assert_eq!(ui.source.view.app.selected, "github");
}
#[test]
fn saving_appearance_runs_in_host_and_reopens_the_saved_projection() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("appearance.json");
    let (tx, rx) = mpsc::channel();
    let mut host = Host::new(Warden::default(), false).with_settings_path(path.clone());
    host.set_wake_callback(Arc::new(move || {
        let _ = tx.send(());
    }));
    let mut ui = Driver::new(host, 1440, 1000).unwrap();
    click(&mut ui, "Settings");
    ui.execute(Command::Click {
        role: "switch".into(),
        label: "Liquid Glass".into(),
        occurrence: None,
    })
    .unwrap();
    click(&mut ui, "Save appearance");
    while ui.source.view.app.settings_saving {
        rx.recv_timeout(Duration::from_secs(60))
            .expect("appearance worker did not finish");
        ui.source.poll_async();
        ui.execute(Command::Snapshot).unwrap();
    }
    assert!(
        !plicowarden::appearance::Appearance::load(&path)
            .unwrap()
            .liquid_glass
    );
}
