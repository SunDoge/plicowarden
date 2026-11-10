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
    ui.source.view.0.appearance.ui_scale = 1.25;
    click(&mut ui, "Reset appearance");
    assert_eq!(
        ui.source.view.0.appearance.ui_scale,
        plicowarden::appearance::Appearance::default().ui_scale
    );
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
            .find(|n| n.label.as_deref() == Some("Continue"))
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
        selected.bounds[0] >= 370. && selected.bounds[0] <= 375.,
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
        heading.rect[0] > selected.bounds[2] + 60.,
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

#[test]
fn interface_scale_setting_changes_the_host_controller_and_resets() {
    let mut ui = Driver::new(NativeApp(Warden::default()), 1440, 1000).unwrap();
    click(&mut ui, "Settings");
    ui.execute(Command::SetValue {
        label: "Interface scale".into(),
        value: 0.85,
    })
    .unwrap();
    assert!((ui.source.view.0.zoom.factor() - 0.85).abs() < 0.001);
    click(&mut ui, "Save appearance");
    assert!(
        matches!(ui.source.view.0.commands.last(), Some(plicowarden::model::HostCommand::SaveAppearance(value)) if (value.ui_scale-0.85).abs()<0.001)
    );
    click(&mut ui, "Reset appearance");
    assert_eq!(
        ui.source.view.0.zoom.factor(),
        plicowarden::appearance::Appearance::default().ui_scale
    );
}

#[test]
fn login_steps_preserve_email_clear_password_and_keep_server_choices() {
    let mut ui = Driver::new(
        NativeApp(
            PreviewConfig {
                screen: "login".into(),
                ..Default::default()
            }
            .app(),
        ),
        820,
        640,
    )
    .unwrap();
    text(&mut ui, "Email address", "bad-address");
    click(&mut ui, "Continue");
    assert!(!ui.source.view.0.login_password_step);
    assert!(!ui.source.view.0.error.is_empty());
    ui.source.view.0.email = "person@example.invalid".into();
    ui.source.request_rebuild();
    ui.execute(Command::Snapshot).unwrap();
    click(&mut ui, "Continue");
    text(&mut ui, "Master password", "synthetic-secret");
    click(&mut ui, "Back");
    assert!(ui.source.view.0.password.is_empty());
    assert_eq!(ui.source.view.0.email, "person@example.invalid");
    click(&mut ui, "bitwarden.com");
    click(&mut ui, "Self-hosted");
    text(&mut ui, "Server URL", "https://vault.example.invalid");
    click(&mut ui, "Done");
    click(&mut ui, "Continue");
    assert_eq!(ui.source.view.0.region, "self-hosted");
    assert_eq!(ui.source.view.0.server, "https://vault.example.invalid");
}

#[test]
fn supported_window_sizes_show_navigation_items_and_details_by_default() {
    for (width, height) in [(1440, 1000), (1224, 850), (1000, 720), (820, 640)] {
        let ui = Driver::new(NativeApp(Warden::default()), width, height).unwrap();
        let snapshot = ui.snapshot();
        let nodes = snapshot.exposed_nodes();
        let nav = nodes
            .iter()
            .find(|n| {
                n.label.as_deref() == Some("All Items")
                    && n.role == plico_renderer::SemanticRole::Button
            })
            .unwrap();
        let item = nodes
            .iter()
            .find(|n| n.label.as_deref() == Some("Select GitHub"))
            .unwrap();
        let details = nodes
            .iter()
            .find(|n| {
                n.label.as_deref() == Some("Copy username")
                    && n.role == plico_renderer::SemanticRole::Button
            })
            .unwrap();
        assert!(
            nav.bounds[2] <= item.bounds[0],
            "navigation and list overlap at {width}"
        );
        assert!(
            item.bounds[2] < details.bounds[0],
            "list and details overlap at {width}"
        );
        assert!(
            details.bounds[2] <= width as f32,
            "details overflow at {width}"
        );
    }
}

#[test]
fn folders_remain_visible_in_demo_and_loaded_folders_filter_items() {
    let mut ui = Driver::new(NativeApp(Warden::default()), 1440, 1000).unwrap();
    click(&mut ui, "No folder");
    assert_eq!(ui.source.view.0.visible().len(), 12);
    let mut app = PreviewConfig {
        screen: "folders".into(),
        ..Default::default()
    }
    .app();
    let folder = app.folders[0].clone();
    let expected = app
        .items
        .iter()
        .filter(|item| {
            item.folder.as_deref() == Some(folder.id.as_str()) && !item.deleted && !item.archived
        })
        .count();
    app.folders_modal.close();
    let mut ui = Driver::new(NativeApp(app), 1440, 1000).unwrap();
    click(&mut ui, &folder.name);
    assert_eq!(ui.source.view.0.filter, format!("folder:{}", folder.id));
    assert_eq!(ui.source.view.0.visible().len(), expected);
    click(&mut ui, "Manage folders");
    assert!(
        ui.snapshot()
            .exposed_nodes()
            .iter()
            .any(|node| node.label.as_deref() == Some("New folder name"))
    );
}

#[test]
fn navigation_columns_have_equal_width_at_desktop_and_compact_sizes() {
    for (width, height) in [(1440, 1000), (1000, 720), (820, 640)] {
        let ui = Driver::new(NativeApp(Warden::default()), width, height).unwrap();
        let snapshot = ui.snapshot();
        let nodes = snapshot.exposed_nodes();
        let nav = nodes
            .iter()
            .find(|n| {
                n.label.as_deref() == Some("All Items")
                    && n.role == plico_renderer::SemanticRole::Button
            })
            .unwrap();
        let item = nodes
            .iter()
            .find(|n| n.label.as_deref() == Some("Select GitHub"))
            .unwrap();
        // The sidebar has 12px inner padding; the list has 10px.
        let sidebar_width = nav.bounds[2] - nav.bounds[0] + 24.;
        let list_width = item.bounds[2] - item.bounds[0] + 20.;
        assert!(
            (sidebar_width - list_width).abs() <= 2.,
            "{width}: sidebar={sidebar_width}, list={list_width}"
        );
    }
}

#[test]
fn login_progress_keeps_request_pending_rejects_stale_updates_and_stops_on_failure() {
    use plicowarden_sdk::{Event, Reply, service::LoginStage};
    let app = PreviewConfig {
        screen: "login".into(),
        ..Default::default()
    }
    .app();
    let mut ui = Driver::new(NativeApp(app), 1000, 720).unwrap();
    ui.source.view.0.email = "recipe@example.invalid".into();
    ui.source.view.0.login_password_step = true;
    ui.source
        .view
        .0
        .password
        .edit(plico::InputEdit::Insert(zeroize::Zeroizing::new(
            "synthetic-only".into(),
        )));
    ui.source.view.0.login();
    let ticket = ui.source.view.0.pending.unwrap();
    ui.execute(Command::Snapshot).unwrap();
    assert!(
        ui.snapshot()
            .exposed_nodes()
            .iter()
            .any(|n| n.role == plico_renderer::SemanticRole::ProgressBar)
    );
    for stage in [
        LoginStage::Authenticating,
        LoginStage::Syncing,
        LoginStage::Decrypting,
    ] {
        ui.source.view.0.apply(Event {
            ticket,
            result: Ok(Reply::LoginProgress(stage)),
        });
        assert!(ui.source.view.0.busy);
        assert_eq!(ui.source.view.0.pending, Some(ticket));
        assert_eq!(ui.source.view.0.login_stage, Some(stage));
        ui.execute(Command::Snapshot).unwrap();
        assert!(
            ui.source
                .adapter
                .placed
                .iter()
                .any(|n| n.paint.text.as_deref() == Some(stage.label()))
        );
    }
    ui.source.view.0.apply(Event {
        ticket: ticket + 1,
        result: Ok(Reply::LoginProgress(LoginStage::Authenticating)),
    });
    assert_eq!(ui.source.view.0.login_stage, Some(LoginStage::Decrypting));
    ui.source.view.0.apply(Event {
        ticket,
        result: Err("Synthetic failure".into()),
    });
    assert!(!ui.source.view.0.busy);
    assert!(ui.source.view.0.login_stage.is_none());
    assert!(ui.source.view.0.error.contains("Unlocking vault"));
    ui.source.view.0.apply(Event {
        ticket: 0,
        result: Ok(Reply::Locked),
    });
    assert!(
        ui.source.view.0.error.contains("Synthetic failure"),
        "the cleanup lock must preserve the login error"
    );
    ui.execute(Command::Snapshot).unwrap();
    assert!(
        !ui.snapshot()
            .exposed_nodes()
            .iter()
            .any(|n| n.role == plico_renderer::SemanticRole::ProgressBar)
    );
}

fn cached_detail(id: &str, name: &str) -> plicowarden_sdk::service::ItemDetails {
    plicowarden_sdk::service::ItemDetails {
        id: id.into(),
        name: name.into(),
        kind: "login",
        username: Some("synthetic-user".into()),
        password: None,
        uris: Vec::new(),
        notes: None,
        totp: None,
        cardholder_name: None,
        card_brand: None,
        card_number: None,
        card_exp_month: None,
        card_exp_year: None,
        card_code: None,
        favorite: false,
        has_password: false,
        has_totp: false,
        has_card_number: false,
        has_card_code: false,
        editable: true,
        folder_id: None,
        deleted: false,
        identity_title: None,
        first_name: None,
        middle_name: None,
        last_name: None,
        address1: None,
        address2: None,
        address3: None,
        city: None,
        state: None,
        postal_code: None,
        country: None,
        company: None,
        email: None,
        phone: None,
        ssn: None,
        passport_number: None,
        license_number: None,
        has_ssn: false,
        has_passport_number: false,
        has_license_number: false,
        password_history: Vec::new(),
    }
}

#[test]
fn cached_real_item_switch_does_not_flash_global_busy_or_loading_cards() {
    let mut app = Warden::default();
    app.authenticated = true;
    app.login_open = false;
    let first = app.items[0].clone();
    let second = app.items[1].clone();
    app.detail_cache
        .insert(first.id.clone(), cached_detail(&first.id, &first.name));
    app.detail_cache
        .insert(second.id.clone(), cached_detail(&second.id, &second.name));
    app.selected = first.id.clone();
    app.details = app.detail_cache.get(&first.id).cloned();
    let mut ui = Driver::new(NativeApp(app), 1440, 1000).unwrap();
    click(&mut ui, &format!("Select {}", second.name));
    assert!(!ui.source.view.0.busy);
    assert_eq!(ui.source.view.0.details.as_ref().unwrap().id, second.id);
    assert!(ui.source.view.0.pending.is_some());
    assert!(!ui.source.adapter.placed.iter().any(|n| {
        n.paint
            .text
            .as_deref()
            .is_some_and(|text| text.starts_with("Loading"))
    }));
    // Switching again is possible while the previous read is still in flight.
    click(&mut ui, &format!("Select {}", first.name));
    assert_eq!(ui.source.view.0.selected, first.id);
}

#[test]
fn downloaded_site_icon_is_declared_as_an_image_in_list_and_details() {
    let mut app = Warden::default();
    let png: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 207, 192, 240,
        31, 0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    app.items[0].icon = Some(plico::ImageAsset::from_bytes(png));
    app.selected = app.items[0].id.clone();
    let ui = Driver::new(NativeApp(app), 1440, 1000).unwrap();
    assert!(
        ui.source
            .adapter
            .placed
            .iter()
            .filter(|n| n.paint.image.is_some())
            .count()
            >= 2
    );
}

#[test]
fn account_picker_distinguishes_identical_emails_on_different_servers_and_shows_local_unlock() {
    let mut app = Warden::default();
    for server in ["https://one.example.invalid", "https://two.example.invalid"] {
        let config = plicowarden_sdk::service::LoginRequest {
            email: "same@example.invalid".into(),
            region: "self-hosted".into(),
            server_url: Some(server.into()),
            server_config: None,
        };
        app.saved_accounts
            .push(plicowarden_sdk::cache::SavedAccount {
                id: plicowarden_sdk::cache::account_id(&config).unwrap(),
                config,
                cached_at: 123,
            });
    }
    let mut ui = Driver::new(NativeApp(app), 1440, 1000).unwrap();
    click(&mut ui, "Switch account");
    click(
        &mut ui,
        "Unlock account same@example.invalid on https://two.example.invalid",
    );
    assert_eq!(ui.source.view.0.server, "https://two.example.invalid");
    assert!(ui.source.view.0.login_open && ui.source.view.0.login_password_step);
    assert!(ui.source.view.0.password.is_empty());
    assert!(ui.source.view.0.has_cached_account());
    ui.advance(std::time::Duration::from_millis(250)).unwrap();
    assert!(
        ui.snapshot()
            .exposed_nodes()
            .iter()
            .any(|node| node.label.as_deref() == Some("Unlock"))
    );
}
