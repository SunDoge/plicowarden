use plico_runtime::{
    NativeApp,
    testing::{Command, Driver},
};
use plicowarden::{PreviewConfig, Warden};

fn click(ui: &mut Driver<NativeApp<Warden>>, label: &str) {
    ui.execute(Command::Click {
        role: "button".into(),
        label: label.into(),
        occurrence: None,
    })
    .unwrap();
}
fn text(ui: &mut Driver<NativeApp<Warden>>, label: &str, value: &str) {
    ui.execute(Command::Focus {
        role: "textbox".into(),
        label: label.into(),
        occurrence: None,
    })
    .unwrap();
    ui.execute(Command::Key { key: "Home".into() }).unwrap();
    // The test resets the bound query directly before typing through native input.
    ui.execute(Command::Text { text: value.into() }).unwrap();
}
#[test]
fn csharp_demo_selection_search_reveal_filters_and_empty_recovery() {
    let mut ui = Driver::new(NativeApp(Warden::default()), 1440, 1000).unwrap();
    assert_eq!(ui.source.view.0.selected, "github");
    click(&mut ui, "Select Linear");
    assert_eq!(ui.source.view.0.selected, "linear");
    text(&mut ui, "Search all vaults", "github");
    assert_eq!(ui.source.view.0.visible().len(), 1);
    assert_eq!(ui.source.view.0.selected, "github");
    click(&mut ui, "Show password");
    assert!(ui.source.view.0.reveal);
    click(&mut ui, "Hide password");
    assert!(!ui.source.view.0.reveal);
    ui.source.view.0.query = "nothing-matches".into();
    ui.source.request_rebuild();
    ui.execute(Command::Snapshot).unwrap();
    assert!(ui.source.view.0.visible().is_empty());
    ui.source.view.0.query.clear();
    ui.source.request_rebuild();
    ui.execute(Command::Snapshot).unwrap();
    click(&mut ui, "Favorites");
    assert_eq!(ui.source.view.0.visible().len(), 4);
    click(&mut ui, "All Items");
    assert_eq!(ui.source.view.0.visible().len(), 12);
    click(&mut ui, "Reverse sort");
    assert_eq!(
        ui.source.view.0.visible().first().unwrap().name,
        "Studio workstation"
    );
}
#[test]
fn csharp_settings_generator_and_cancelled_login_paths() {
    let mut ui = Driver::new(NativeApp(Warden::default()), 1440, 1000).unwrap();
    click(&mut ui, "Settings");
    assert!(ui.source.view.0.settings_open);
    ui.execute(Command::Click {
        role: "switch".into(),
        label: "Liquid Glass".into(),
        occurrence: None,
    })
    .unwrap();
    assert!(!ui.source.view.0.appearance.liquid_glass);
    click(&mut ui, "Reset appearance");
    assert!(ui.source.view.0.appearance.liquid_glass);
    click(&mut ui, "Close settings");
    click(&mut ui, "Generator");
    click(&mut ui, "Generate password");
    assert_eq!(ui.source.view.0.generated.len(), 24);
    assert_eq!(ui.source.view.0.history.len(), 1);
    click(&mut ui, "Connect account");
    assert!(ui.source.view.0.login_open);
    click(&mut ui, "Browse demo vault");
    assert!(!ui.source.view.0.login_open);
    assert!(ui.source.view.0.generated.is_empty());
    assert_eq!(ui.source.view.0.visible().len(), 12);
}
#[test]
fn preview_forms_and_compact_layout_have_valid_native_trees() {
    for (width, height) in [(1440, 1000), (1000, 720), (820, 640)] {
        for screen in ["vault", "settings", "login", "editor", "generator"] {
            let app = PreviewConfig {
                screen: screen.into(),
                ..Default::default()
            }
            .app();
            let ui = Driver::new(NativeApp(app), width, height).unwrap();
            assert!(!ui.snapshot().exposed_nodes().is_empty());
            for node in &ui.source.adapter.placed {
                assert!(node.rect.iter().all(|v| v.is_finite()));
            }
        }
    }
}

#[test]
fn login_is_centered_and_vault_columns_match_the_csharp_reference() {
    for (width, height) in [(1440, 1000), (1000, 720), (820, 640)] {
        let app = PreviewConfig {
            screen: "login".into(),
            ..Default::default()
        }
        .app();
        let ui = Driver::new(NativeApp(app), width, height).unwrap();
        let snapshot = ui.snapshot();
        let signin = snapshot
            .exposed_nodes()
            .into_iter()
            .find(|n| n.label.as_deref() == Some("Sign in"))
            .unwrap();
        assert!(((signin.bounds[0] + signin.bounds[2]) * 0.5 - width as f32 * 0.5).abs() < 1.0);
        assert!(signin.bounds[0] >= 24. && signin.bounds[2] <= width as f32 - 24.);
    }
    let ui = Driver::new(NativeApp(Warden::default()), 1440, 1000).unwrap();
    let snapshot = ui.snapshot();
    let selected = snapshot
        .exposed_nodes()
        .into_iter()
        .find(|n| n.label.as_deref() == Some("Select GitHub"))
        .unwrap();
    assert!(
        (selected.bounds[0] - 266.).abs() <= 1.0,
        "selection bounds {:?}",
        selected.bounds
    );
    let heading = ui
        .source
        .adapter
        .placed
        .iter()
        .find(|n| n.paint.text.as_deref() == Some("GitHub") && n.paint.font_size > 30.)
        .unwrap();
    assert!(
        (heading.rect[0] - 712.).abs() <= 1.0,
        "heading bounds {:?}",
        heading.rect
    );
}

#[test]
fn generator_mode_controls_produce_values_through_real_buttons() {
    let mut ui = Driver::new(NativeApp(Warden::default()), 1440, 1000).unwrap();
    click(&mut ui, "Generator");
    click(&mut ui, "Passphrase");
    click(&mut ui, "Generate passphrase");
    assert_eq!(ui.source.view.0.generated.split('-').count(), 6);
    click(&mut ui, "Username");
    assert!(ui.source.view.0.generated.is_empty());
    click(&mut ui, "Generate username");
    assert_eq!(ui.source.view.0.generated.split('-').count(), 3);
    click(&mut ui, "Copy generated value");
    assert!(
        matches!(ui.source.view.0.commands.last(), Some(plicowarden::model::HostCommand::Account { request, .. }) if matches!(request.as_ref(), plicowarden_sdk::Request::CopyText(_)))
    );
}

#[test]
fn ssh_approval_requires_an_explicit_decision_and_lock_discards_it() {
    let app = PreviewConfig {
        screen: "ssh-approval".into(),
        ..Default::default()
    }
    .app();
    let mut ui = Driver::new(NativeApp(app), 1440, 1000).unwrap();
    assert_eq!(ui.source.view.0.ssh_approvals.len(), 1);
    assert!(ui.source.view.0.commands.is_empty());
    click(&mut ui, "Allow SSH signature once");
    assert!(ui.source.view.0.ssh_approvals.is_empty());
    assert!(matches!(
        ui.source.view.0.commands.last(),
        Some(plicowarden::model::HostCommand::SshDecision { id: 1, allow: true })
    ));
    ui.source.view.0.lock();
    assert!(ui.source.view.0.ssh_socket.is_none());
    assert!(!ui.source.view.0.ssh_modal.is_open());
    // Stale public approval after lock cannot reopen the signing dialog.
    ui.source.view.0.apply(plicowarden_sdk::Event {
        ticket: 0,
        result: Ok(plicowarden_sdk::Reply::SshApproval(
            plicowarden_sdk::ssh_agent::Approval {
                id: 2,
                name: "stale".into(),
                fingerprint: "synthetic".into(),
                process_id: None,
            },
        )),
    });
    assert!(ui.source.view.0.ssh_approvals.is_empty());
}
