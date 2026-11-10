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
fn rejected_http_login_preserves_password_for_retry_and_can_return_to_demo() {
    let (tx, rx) = mpsc::channel();
    let mut host = Host::new(Warden::default(), false);
    host.set_wake_callback(Arc::new(move || {
        let _ = tx.send(());
    }));
    let mut ui = Driver::new(host, 1440, 1000).unwrap();
    click(&mut ui, "Connect account");
    click(&mut ui, "bitwarden.com");
    click(&mut ui, "Self-hosted");
    type_text(&mut ui, "Server URL", "https://example.invalid");
    click(&mut ui, "Done");
    type_text(&mut ui, "Email address", "test@example.invalid");
    click(&mut ui, "Continue");
    // Bypass UI validation deliberately to exercise the SDK worker error path.
    ui.source.view.app.server = "http://example.invalid".into();
    type_text(&mut ui, "Master password", "synthetic-test-password");
    let tree = serde_json::to_string(&ui.declarations()).unwrap();
    assert!(!tree.contains("synthetic-test-password"));
    click(&mut ui, "Sign in");
    assert!(!ui.source.view.app.password.is_empty());
    // Completion notification, not sleeping/polling for an arbitrary amount of time.
    while ui.source.view.app.busy {
        rx.recv_timeout(Duration::from_secs(60))
            .expect("account worker did not finish");
        ui.source.poll_async();
        ui.execute(Command::Snapshot).unwrap();
    }
    assert!(ui.source.view.app.error.contains("HTTPS"));
    assert!(!ui.source.view.app.password.is_empty());
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
    ui.source.view.app.zoom.set(1.25);
    ui.execute(Command::Snapshot).unwrap();
    click(&mut ui, "Save appearance");
    while ui.source.view.app.settings_saving {
        rx.recv_timeout(Duration::from_secs(60))
            .expect("appearance worker did not finish");
        ui.source.poll_async();
        ui.execute(Command::Snapshot).unwrap();
    }
    assert_eq!(
        plicowarden::appearance::Appearance::load(&path)
            .unwrap()
            .ui_scale,
        1.25
    );
}

#[test]
fn custom_header_routes_drag_once_and_keeps_toolbar_buttons_independent() {
    use plico_renderer::{
        HostAction, Modifiers, Point, PointerButton, PointerEvent, PointerPhase, UiEvent,
        WindowCommand,
    };
    let mut app = Warden::default();
    app.custom_titlebar = true;
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

#[test]
fn custom_window_buttons_dispatch_real_commands_on_vault_and_login() {
    use plico_renderer::{HostAction, WindowCommand};
    for login in [false, true] {
        let mut app = Warden::default();
        app.login_open = login;
        app.custom_titlebar = true;
        let mut ui = Driver::new(Host::new(app, false), 820, 640).unwrap();
        for (label, expected) in [
            ("Minimize window", WindowCommand::Minimize),
            ("Maximize or restore window", WindowCommand::ToggleMaximized),
            ("Maximize or restore window", WindowCommand::ToggleMaximized),
            ("Close window", WindowCommand::Close),
        ] {
            click(&mut ui, label);
            assert_eq!(
                ui.source.take_host_action(),
                Some(HostAction::Window(expected))
            );
            assert!(ui.source.take_host_action().is_none());
        }
    }
}

#[test]
fn saving_security_updates_live_preferences_only_after_successful_persistence() {
    for fail in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = if fail {
            directory.path().to_owned()
        } else {
            directory.path().join("security.json")
        };
        let (tx, rx) = mpsc::channel();
        let mut host = Host::new(Warden::default(), false).with_security_path(path.clone());
        host.set_wake_callback(Arc::new(move || {
            let _ = tx.send(());
        }));
        let mut ui = Driver::new(host, 820, 640).unwrap();
        click(&mut ui, "Settings");
        ui.execute(Command::Click {
            role: "button".into(),
            label: "Security".into(),
            occurrence: None,
        })
        .unwrap();
        ui.source.view.app.security.auto_lock_minutes = 1;
        ui.source.view.app.security.clipboard_seconds = 10;
        click(&mut ui, "Save changes");
        while ui.source.view.app.security_saving {
            rx.recv_timeout(Duration::from_secs(10))
                .expect("security worker did not finish");
            ui.source.poll_async();
            ui.execute(Command::Snapshot).unwrap();
        }
        if fail {
            assert!(!ui.source.view.app.error.is_empty());
            assert_eq!(ui.source.view.app.active_security.auto_lock_minutes, 5);
        } else {
            assert_eq!(
                plicowarden::security::load(&path)
                    .unwrap()
                    .auto_lock_minutes,
                1
            );
            assert_eq!(ui.source.view.app.active_security.clipboard_seconds, 10);
        }
    }
}

#[test]
fn copy_login_error_uses_native_clipboard_without_clearing_error_or_starting_account_work() {
    use plico_renderer::HostAction;
    let mut app = plicowarden::PreviewConfig {
        screen: "login".into(),
        ..Default::default()
    }
    .app();
    let message = "Connecting and authenticating…
Synthetic server error: request timed out.
Additional diagnostic detail.";
    app.error = message.into();
    let mut ui = Driver::new(Host::new(app, false), 1000, 720).unwrap();
    click(&mut ui, "Copy error");
    assert_eq!(
        ui.source.take_host_action(),
        Some(HostAction::SetClipboard(message.into()))
    );
    assert_eq!(ui.source.view.app.error, message);
    assert!(!ui.source.view.app.busy);
    assert!(ui.source.view.app.pending.is_none());
    assert!(!ui.source.view.app.authenticated);
    click(&mut ui, "Copy error");
    assert_eq!(
        ui.source.take_host_action(),
        Some(HostAction::SetClipboard(message.into()))
    );
}
