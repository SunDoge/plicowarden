use plico_runtime::{
    NativeApp,
    testing::{Command, Driver},
};
use plicowarden::{PreviewConfig, Warden, model::HostCommand};
use plicowarden_sdk::Request;
fn fixture() -> Warden {
    PreviewConfig {
        screen: "folders".into(),
        ..Default::default()
    }
    .app()
}
fn click(ui: &mut Driver<NativeApp<Warden>>, role: &str, label: &str) {
    ui.execute(Command::Click {
        role: role.into(),
        label: label.into(),
        occurrence: None,
    })
    .unwrap();
}
#[test]
fn single_move_can_remove_a_folder_and_keeps_the_current_item_selected() {
    let mut ui = Driver::new(NativeApp(fixture()), 820, 640).unwrap();
    click(&mut ui, "button", "Move to folder");
    assert_eq!(ui.source.view.0.move_ids, ["github"]);
    click(&mut ui, "combobox", "Destination folder");
    ui.advance(std::time::Duration::from_millis(250)).unwrap();
    click(&mut ui, "option", "No folder");
    ui.advance(std::time::Duration::from_millis(250)).unwrap();
    click(&mut ui, "button", "Move selected items");
    assert_eq!(ui.source.view.0.selected, "github");
    assert!(ui.source.view.0.commands.iter().any(|command| matches!(command,
        HostCommand::Account { request, .. } if matches!(request.as_ref(), Request::MoveToFolder { ids, folder_id: None } if ids == &["github"]))));
}
#[test]
fn bulk_move_uses_selected_ids_and_rejects_a_removed_destination() {
    let mut app = fixture();
    app.bulk_mode = true;
    app.bulk_selected.extend(["github".into(), "linear".into()]);
    app.open_move_dialog(app.bulk_selected.iter().cloned().collect());
    app.move_folder = app.folders.last().unwrap().id.clone();
    app.folders.pop();
    app.move_items();
    assert!(!app.busy);
    assert!(app.move_modal.is_open());
    assert!(app.error.contains("no longer exists"));
    app.move_folder = app.folders[0].id.clone();
    app.move_items();
    assert!(app.busy);
    assert!(
        matches!(app.commands.last(), Some(HostCommand::Account { request, .. }) if matches!(request.as_ref(), Request::MoveToFolder { ids, folder_id: Some(id) } if ids == &["github", "linear"] && id == &app.folders[0].id))
    );
    app.lock();
    assert!(app.move_ids.is_empty());
    assert!(!app.move_modal.is_open());
}
#[test]
fn unfiled_filter_excludes_archived_and_trashed_items_and_cancel_does_not_write() {
    let mut app = fixture();
    app.items
        .iter_mut()
        .find(|item| item.id == "linear")
        .unwrap()
        .archived = true;
    app.items
        .iter_mut()
        .find(|item| item.id == "airbnb")
        .unwrap()
        .deleted = true;
    app.set_filter("No folder");
    let expected = app
        .items
        .iter()
        .filter(|item| item.folder.is_none() && !item.deleted && !item.archived)
        .count();
    assert_eq!(app.visible().len(), expected);
    assert!(
        app.visible()
            .iter()
            .all(|item| item.folder.is_none() && !item.archived && !item.deleted)
    );
    app.open_move_dialog(vec!["github".into()]);
    app.move_modal.close();
    app.move_items();
    assert!(!app.busy && app.commands.is_empty());
}
