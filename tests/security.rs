use plicowarden::{
    Warden,
    model::HostCommand,
    security::{self, SecuritySettings},
};
use plicowarden_sdk::{Event, Reply, Request};

#[test]
fn security_preferences_roundtrip_and_reject_unbounded_timeouts() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("security.json");
    assert_eq!(security::load(&path).unwrap(), SecuritySettings::default());
    let settings = SecuritySettings {
        auto_lock_minutes: 1,
        clipboard_seconds: 10,
        minimize_on_copy: true,
    };
    security::save(&path, settings).unwrap();
    assert_eq!(security::load(&path).unwrap(), settings);
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(json.as_object().unwrap().len(), 3);
    std::fs::write(
        &path,
        r#"{"autoLockMinutes":0,"clipboardSeconds":4294967295}"#,
    )
    .unwrap();
    assert_eq!(security::load(&path).unwrap(), SecuritySettings::default());
    std::fs::write(&path, r#"{"password":"unsupported"}"#).unwrap();
    assert!(security::load(&path).is_err());
}

#[test]
fn only_successful_copy_uses_the_saved_minimize_preference() {
    for (minimize, success) in [(false, true), (true, false), (true, true)] {
        let mut app = Warden::default();
        app.active_security.minimize_on_copy = minimize;
        app.active_security.clipboard_seconds = 10;
        app.request(Request::CopyText(String::from("synthetic").into()));
        let ticket = app.pending.unwrap();
        app.commands.clear();
        app.apply(Event {
            ticket,
            result: if success {
                Ok(Reply::Copied)
            } else {
                Err("Copy failed".into())
            },
        });
        assert_eq!(
            app.commands.iter().any(|command| matches!(
                command,
                HostCommand::Window(plico_renderer::WindowCommand::Minimize)
            )),
            minimize && success
        );
        if success {
            assert!(app.notice.contains("10 seconds"));
        }
    }
}

#[test]
fn queued_configuration_acknowledgements_do_not_rollback_saved_settings() {
    let mut app = Warden::default();
    app.active_security.auto_lock_minutes = 1;
    app.apply(Event {
        ticket: 0,
        result: Ok(Reply::SecurityConfigured(SecuritySettings::default())),
    });
    assert_eq!(app.active_security.auto_lock_minutes, 1);
}
