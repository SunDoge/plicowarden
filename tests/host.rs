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

#[test]
fn custom_header_routes_drag_once_and_keeps_toolbar_buttons_independent() {
    use plico_renderer::{
        HostAction, Modifiers, Point, PointerButton, PointerEvent, PointerPhase, UiEvent,
        WindowCommand,
    };
    let mut app = Warden::default();
    app.native_titlebar = true;
    let mut ui = Driver::new(Host::new(app, false), 1440, 1000).unwrap();
    click(&mut ui, "Settings");
    assert!(ui.source.view.app.settings_open);
    assert!(ui.source.take_host_action().is_none());
    let bounds = ui
        .snapshot()
        .exposed_nodes()
        .into_iter()
        .find(|n| n.label.as_deref() == Some("Move window"))
        .unwrap()
        .bounds;
    let position = Point {
        x: f64::from((bounds[0] + bounds[2]) / 2.),
        y: f64::from((bounds[1] + bounds[3]) / 2.),
    };
    for phase in [PointerPhase::Down, PointerPhase::Up, PointerPhase::Down] {
        ui.source.handle_event(UiEvent::Pointer(PointerEvent {
            phase,
            position,
            button: Some(PointerButton::Primary),
            buttons: if phase == PointerPhase::Down { 1 } else { 0 },
            modifiers: Modifiers::empty(),
            properties: Default::default(),
        }));
        ui.execute(Command::Snapshot).unwrap();
        if phase == PointerPhase::Down {
            assert!(matches!(
                ui.source.take_host_action(),
                Some(HostAction::Window(WindowCommand::StartDragging))
            ));
        } else {
            assert!(ui.source.take_host_action().is_none());
        }
        ui.execute(Command::Snapshot).unwrap();
        assert!(
            ui.source.take_host_action().is_none(),
            "drag command repeated while held"
        );
    }
}
