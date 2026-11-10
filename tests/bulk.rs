use plico_runtime::{
    NativeApp,
    testing::{Command, Driver},
};
use plicowarden::{PreviewConfig, Warden, model::HostCommand};
use plicowarden_sdk::Request;

fn click(ui: &mut Driver<NativeApp<Warden>>, label: &str) {
    ui.execute(Command::Click {
        role: "button".into(),
        label: label.into(),
        occurrence: None,
    })
    .unwrap();
}
#[test]
fn bulk_selection_is_independent_of_details_and_destructive_actions_require_confirmation() {
    let mut ui = Driver::new(
        NativeApp(
            PreviewConfig {
                screen: "bulk".into(),
                ..Default::default()
            }
            .app(),
        ),
        1000,
        720,
    )
    .unwrap();
    click(&mut ui, "Bulk selection");
    click(&mut ui, "Toggle bulk selection for GitHub");
    click(&mut ui, "Toggle bulk selection for Linear");
    assert_eq!(ui.source.view.0.selected, "github");
    assert_eq!(ui.source.view.0.bulk_selected.len(), 2);
    click(&mut ui, "Trash selected");
    assert!(!ui.source.view.0.busy);
    assert!(ui.source.view.0.bulk_modal.is_open());
    click(&mut ui, "Cancel bulk action");
    ui.advance(std::time::Duration::from_millis(250)).unwrap();
    assert!(!ui.source.view.0.busy);
    click(&mut ui, "Trash selected");
    click(&mut ui, "Confirm bulk action");
    assert!(ui.source.view.0.busy);
    assert!(ui.source.view.0.commands.iter().any(|command| matches!(command,
        HostCommand::Account { request, .. } if matches!(request.as_ref(), Request::BatchAction { ids, action } if ids == &["github", "linear"] && action == "trash"))));
    ui.source.view.0.lock();
    assert!(ui.source.view.0.bulk_selected.is_empty());
    assert!(!ui.source.view.0.bulk_modal.is_open());
}
#[test]
fn demo_cannot_send_bulk_mutations_and_filters_clear_selection() {
    let mut app = Warden::default();
    app.bulk_selected.insert("github".into());
    app.apply_bulk_action("purge");
    assert!(!app.busy);
    app.set_filter("Favorites");
    assert!(app.bulk_selected.is_empty());
}

#[test]
fn partial_bulk_results_keep_failed_ids_and_report_successes() {
    use plicowarden_sdk::{
        Event, Reply,
        service::{BatchFailure, BatchMutation, VaultItem, VaultSnapshot},
    };
    let mut app = Warden::default();
    app.authenticated = true;
    app.bulk_mode = true;
    app.bulk_selected.extend(["github".into(), "linear".into()]);
    app.apply_bulk_action("archive");
    let ticket = app.pending.unwrap();
    app.apply(Event {
        ticket,
        result: Ok(Reply::BatchMutation(BatchMutation {
            succeeded: vec!["github".into()],
            failed: vec![BatchFailure {
                id: "linear".into(),
                message: "Synthetic permission failure".into(),
            }],
            snapshot: VaultSnapshot {
                offline: false,
                account_id: None,
                cache_warning: None,
                email: "recipe@example.invalid".into(),
                folders: vec![],
                decrypt_failures: 0,
                items: vec![VaultItem {
                    id: "linear".into(),
                    name: "Linear".into(),
                    subtitle: "synthetic".into(),
                    kind: "login",
                    favorite: false,
                    has_username: true,
                    has_password: true,
                    has_totp: false,
                    icon_url: None,
                    folder_id: None,
                    deleted: false,
                    archived: false,
                }],
            },
        })),
    });
    assert_eq!(
        app.bulk_selected.into_iter().collect::<Vec<_>>(),
        vec!["linear"]
    );
    assert!(app.bulk_mode);
    assert!(app.notice.contains("Updated 1 item"));
    assert!(app.error.contains("permission failure"));
}
