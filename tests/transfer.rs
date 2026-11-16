use plico_desktop::NativeApp;
use plico_runtime::testing::{Command, Driver};
use plicowarden::{PreviewConfig, Warden, model::HostCommand, transfer::TransferIntent};
use zeroize::Zeroizing;
#[test]
fn export_requires_matching_passwords_and_drops_ui_secrets_before_file_selection() {
    let mut app = PreviewConfig {
        screen: "transfer".into(),
        ..Default::default()
    }
    .app();
    app.begin_export();
    assert!(!app.transfer_dialog);
    app.transfer
        .export_password
        .set(Zeroizing::new("synthetic-export-password".into()));
    app.transfer
        .confirmation
        .set(Zeroizing::new("different-password".into()));
    app.begin_export();
    assert!(!app.transfer_dialog);
    app.transfer
        .confirmation
        .set(Zeroizing::new("synthetic-export-password".into()));
    app.begin_export();
    assert!(app.transfer_dialog);
    assert!(app.transfer.export_password.is_empty() && app.transfer.confirmation.is_empty());
    assert!(matches!(
        app.commands.last(),
        Some(HostCommand::Transfer(TransferIntent::Export { .. }))
    ));
    app.lock();
    assert!(!app.transfer_dialog);
    assert!(app.transfer.import_password.is_empty());
}
#[test]
fn plaintext_export_and_import_require_explicit_choices_and_an_account() {
    let mut app = Warden::default();
    app.transfer.format = "json".into();
    app.transfer.plaintext_ack = true;
    app.transfer.import_ack = true;
    app.begin_export();
    app.begin_import();
    assert!(!app.transfer_dialog);
    app.authenticated = true;
    app.transfer.plaintext_ack = false;
    app.begin_export();
    assert!(!app.transfer_dialog);
    app.transfer.import_ack = false;
    app.begin_import();
    assert!(!app.transfer_dialog);
}
#[test]
fn transfer_passwords_do_not_enter_declaration_snapshots() {
    let app = PreviewConfig {
        screen: "transfer".into(),
        ..Default::default()
    }
    .app();
    app.transfer
        .export_password
        .set(Zeroizing::new("synthetic-export-secret".into()));
    let mut ui = Driver::new(NativeApp(app), 1000, 720).unwrap();
    ui.execute(Command::Snapshot).unwrap();
    assert!(
        !serde_json::to_string(&ui.declarations())
            .unwrap()
            .contains("synthetic-export-secret")
    );
}
