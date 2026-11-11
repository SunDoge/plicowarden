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
fn demo_totp_countdown_uses_the_injected_clock() {
    let mut ui = Driver::new(NativeApp(Warden::default()), 950, 720).unwrap();
    assert!(
        ui.source
            .adapter
            .placed
            .iter()
            .any(|node| node.paint.text.as_deref() == Some("30"))
    );
    ui.advance(std::time::Duration::from_secs(1)).unwrap();
    assert!(
        ui.source
            .adapter
            .placed
            .iter()
            .any(|node| node.paint.text.as_deref() == Some("29"))
    );
}

#[test]
fn totp_progress_ring_is_a_valid_single_shader_surface() {
    let app = PreviewConfig {
        screen: "totp".into(),
        ..Default::default()
    }
    .app();
    let ui = Driver::new(NativeApp(app), 1000, 720).unwrap();
    let diagnostics = ui.source.adapter.shader_diagnostics().collect::<Vec<_>>();
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
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
    assert_eq!(ui.source.view.0.history.len(), 2);
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
        for screen in [
            "vault",
            "settings",
            "ssh-history",
            "login",
            "editor",
            "generator",
        ] {
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
fn narrow_windows_switch_panes_instead_of_squeezing_three_columns() {
    // 820: list and detail share the window; the sidebar is a separate pane.
    let mut ui = Driver::new(NativeApp(Warden::default()), 820, 640).unwrap();
    let labels = |ui: &Driver<NativeApp<Warden>>| {
        ui.snapshot()
            .exposed_nodes()
            .into_iter()
            .filter_map(|node| node.label.clone())
            .collect::<Vec<_>>()
    };
    assert!(!labels(&ui).iter().any(|label| label == "All Items"));
    assert!(labels(&ui).iter().any(|label| label == "Select GitHub"));
    assert!(labels(&ui).iter().any(|label| label == "Copy username"));
    click(&mut ui, "Toggle sidebar");
    assert!(labels(&ui).iter().any(|label| label == "All Items"));
    assert!(!labels(&ui).iter().any(|label| label == "Select GitHub"));
    click(&mut ui, "Back");
    assert!(labels(&ui).iter().any(|label| label == "Select GitHub"));

    // 600: one pane at a time; selecting an item opens the detail, Back returns.
    let mut ui = Driver::new(NativeApp(Warden::default()), 600, 640).unwrap();
    assert!(labels(&ui).iter().any(|label| label == "Select GitHub"));
    assert!(!labels(&ui).iter().any(|label| label == "Copy username"));
    click(&mut ui, "Select GitHub");
    assert!(labels(&ui).iter().any(|label| label == "Copy username"));
    assert!(!labels(&ui).iter().any(|label| label == "Select GitHub"));
    click(&mut ui, "Back");
    assert!(labels(&ui).iter().any(|label| label == "Select GitHub"));
    assert!(!labels(&ui).iter().any(|label| label == "Copy username"));
}

#[test]
fn login_uses_a_branded_split_or_centered_card_and_vault_columns_keep_their_widths() {
    for (width, height) in [(1440, 1000), (1000, 720), (820, 640), (700, 640)] {
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
        let center = (signin.bounds[0] + signin.bounds[2]) * 0.5;
        // A wide login splits into the brand pane and the form pane; narrow
        // windows keep the single centered card.
        let expected = if width >= 900 {
            let brand = (width as f32 * 0.44).clamp(280., 560.);
            width as f32 * 0.5 + brand * 0.5
        } else {
            width as f32 * 0.5
        };
        assert!(
            (center - expected).abs() < 2.,
            "{width}: {center}, expected {expected}"
        );
        assert!(signin.bounds[0] >= 24. && signin.bounds[2] <= width as f32 - 24.);
    }
    let ui = Driver::new(NativeApp(Warden::default()), 1440, 1000).unwrap();
    let snapshot = ui.snapshot();
    let selected = snapshot
        .exposed_nodes()
        .into_iter()
        .find(|n| n.label.as_deref() == Some("Select GitHub"))
        .unwrap();
    // 8 px window inset + 260 px sidebar + 8 px gap + 6 px list padding.
    assert!(
        selected.bounds[0] >= 279. && selected.bounds[0] <= 285.,
        "selection bounds {:?}",
        selected.bounds
    );
    let heading = ui
        .source
        .adapter
        .placed
        .iter()
        .find(|n| n.paint.text.as_deref() == Some("GitHub") && n.paint.font_size >= 28.)
        .unwrap();
    assert!(
        heading.rect[0] > selected.bounds[2] + 12.,
        "heading bounds {:?}",
        heading.rect
    );
}

#[test]
fn app_shortcuts_toggle_palette_settings_and_vault_actions() {
    use plico::{KeyEvent, KeyLocation, KeyPhase, Modifiers};
    use plico_dev::App;
    fn shortcut(key: &str, modifiers: Modifiers) -> KeyEvent {
        KeyEvent {
            phase: KeyPhase::Down,
            key: key.into(),
            key_without_modifiers: key.into(),
            code: format!("Key{}", key.to_uppercase()),
            text: None,
            text_with_all_modifiers: None,
            location: KeyLocation::Standard,
            modifiers,
            repeat: false,
            synthetic: false,
        }
    }
    let mut app = Warden::default();
    let primary = if cfg!(target_os = "macos") {
        Modifiers::META
    } else {
        Modifiers::CONTROL
    };
    assert!(app.on_key_event(&shortcut("f", primary)));
    assert!(app.palette_modal.is_open());
    app.palette_modal.close();
    assert!(app.on_key_event(&shortcut("k", primary)));
    assert!(app.palette_modal.is_open());
    assert!(app.on_key_event(&shortcut("k", primary)));
    assert!(!app.palette_modal.is_open());
    assert!(app.on_key_event(&shortcut(",", primary)));
    assert!(app.settings_open);
    assert!(app.on_key_event(&shortcut(",", primary)));
    assert!(!app.settings_open);
    assert!(app.on_key_event(&shortcut("g", primary)));
    assert_eq!(app.filter, "Generator");
    app.authenticated = true;
    assert!(app.on_key_event(&shortcut("n", primary)));
    assert!(app.editor.is_some());
    assert!(app.on_key_event(&shortcut("l", primary)));
    assert!(!app.authenticated);
    assert!(!app.on_key_event(&shortcut("k", Modifiers::empty())));
}

#[test]
fn arrow_keys_move_the_list_selection() {
    use plico::{KeyEvent, KeyLocation, KeyPhase, Modifiers};
    use plico_dev::App;
    fn arrow(key: &str) -> KeyEvent {
        KeyEvent {
            phase: KeyPhase::Down,
            key: key.into(),
            key_without_modifiers: key.into(),
            code: key.into(),
            text: None,
            text_with_all_modifiers: None,
            location: KeyLocation::Standard,
            modifiers: Modifiers::empty(),
            repeat: false,
            synthetic: false,
        }
    }
    let mut app = Warden::default();
    let initial = app.selected.clone();
    assert!(app.on_key_event(&arrow("ArrowDown")));
    assert_ne!(app.selected, initial);
    assert!(app.on_key_event(&arrow("ArrowUp")));
    assert_eq!(app.selected, initial);
}

#[test]
fn generator_mode_controls_produce_values_through_real_buttons() {
    let mut ui = Driver::new(NativeApp(Warden::default()), 1440, 1000).unwrap();
    click(&mut ui, "Generator");
    click(&mut ui, "Passphrase");
    click(&mut ui, "Generate passphrase");
    assert_eq!(ui.source.view.0.generated.split('-').count(), 6);
    click(&mut ui, "Username");
    assert_eq!(ui.source.view.0.generated.split('-').count(), 3);
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
fn ssh_access_history_records_outcomes_filters_and_clears() {
    use plicowarden_sdk::ssh_agent::{AccessRecord, Outcome};
    let app = PreviewConfig {
        screen: "settings".into(),
        ..Default::default()
    }
    .app();
    let mut ui = Driver::new(NativeApp(app), 1440, 1000).unwrap();
    for (id, name, outcome) in [
        (1, "Key One", Outcome::Allowed),
        (2, "Key Two", Outcome::Denied),
        (3, "Key Three", Outcome::TimedOut),
    ] {
        ui.source.view.0.apply(plicowarden_sdk::Event {
            ticket: 0,
            result: Ok(plicowarden_sdk::Reply::SshAccess(AccessRecord {
                id,
                name: name.into(),
                fingerprint: format!("SHA256:synthetic-{id}"),
                process_id: Some(4000 + id as u32),
                outcome,
                at: 1,
            })),
        });
    }
    assert_eq!(ui.source.view.0.ssh_history.len(), 3);
    click(&mut ui, "SSH agent");
    click(&mut ui, "History");
    assert_eq!(ui.source.view.0.ssh_view, plicowarden::model::SSH_HISTORY);
    let shown = |ui: &Driver<NativeApp<Warden>>, value: &str| {
        ui.source
            .adapter
            .placed
            .iter()
            .any(|node| node.paint.text.as_deref() == Some(value))
    };
    ui.execute(Command::Snapshot).unwrap();
    assert!(shown(&ui, "Key One"));
    assert!(shown(&ui, "Key Two"));
    assert!(shown(&ui, "Denied"));
    assert!(shown(&ui, "PID 4002 · SHA256:synthetic-2"));
    click(&mut ui, "Not allowed");
    ui.execute(Command::Snapshot).unwrap();
    assert!(!shown(&ui, "Key One"));
    assert!(shown(&ui, "Key Two"));
    assert!(shown(&ui, "Key Three"));
    click(&mut ui, "Clear history");
    assert!(ui.source.view.0.ssh_history.is_empty());
    assert!(matches!(
        ui.source.view.0.commands.last(),
        Some(plicowarden::model::HostCommand::Account { request, .. })
            if matches!(request.as_ref(), plicowarden_sdk::Request::ClearSshHistory)
    ));
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
    for (width, height) in [(1440, 1000), (1224, 850), (1000, 720)] {
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
    let path = plicowarden::model::folder_path(&folder.name);
    let expected = app
        .items
        .iter()
        .filter(|item| {
            !item.deleted
                && !item.archived
                && item.folder.as_deref().is_some_and(|id| {
                    app.folders
                        .iter()
                        .find(|candidate| candidate.id == id)
                        .is_some_and(|candidate| {
                            let candidate = plicowarden::model::folder_path(&candidate.name);
                            candidate == path || candidate.starts_with(&format!("{path}/"))
                        })
                })
        })
        .count();
    app.folders_modal.close();
    let mut ui = Driver::new(NativeApp(app), 1440, 1000).unwrap();
    click(&mut ui, &folder.name);
    assert_eq!(ui.source.view.0.filter, format!("path:{}", folder.name));
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
fn navigation_columns_follow_triwarden_widths_at_desktop_and_compact_sizes() {
    for (width, height, sidebar, list) in [(1440, 1000, 260., 320.), (1000, 720, 220., 300.)] {
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
        // The row inset exposes the panel width: 10 px on each side of the
        // navigation row, 6 px of list padding on each side of the item row.
        let sidebar_width = nav.bounds[2] - nav.bounds[0] + 20.;
        let list_width = item.bounds[2] - item.bounds[0] + 12.;
        assert!(
            (sidebar_width - sidebar).abs() <= 2.,
            "{width}: sidebar={sidebar_width}, expected {sidebar}"
        );
        assert!(
            (list_width - list).abs() <= 2.,
            "{width}: list={list_width}, expected {list}"
        );
    }
}

#[test]
fn overflowing_sidebar_accepts_wheel_scroll() {
    use plico_renderer::{
        FrameSource, GesturePhase, Modifiers, Point, UiEvent, WheelDeltaMode, WheelEvent,
    };

    let mut app = Warden::default();
    app.folders.clear();
    let mut ui = Driver::new(NativeApp(app), 1000, 500).unwrap();
    let sidebar = ui
        .source
        .adapter
        .placed
        .iter()
        .find(|node| {
            node.scroll.scrollable[1]
                && node.scroll.range[1] > 0.
                && node.rect[0] < 40.
                && node.rect[2] < 300.
        })
        .expect("compact sidebar content should overflow instead of shrinking")
        .clone();
    let position = Point {
        x: (sidebar.rect[0] + sidebar.rect[2]) as f64 * 0.5,
        y: (sidebar.rect[1] + sidebar.rect[3]) as f64 * 0.5,
    };
    assert!(
        ui.source
            .handle_event(UiEvent::Wheel(WheelEvent {
                position,
                delta_x: 0.,
                delta_y: 96.,
                delta_mode: WheelDeltaMode::Pixel,
                phase: GesturePhase::Changed,
                modifiers: Modifiers::empty(),
            }))
            .handled
    );
    ui.execute(Command::Snapshot).unwrap();
    let offset = ui
        .source
        .adapter
        .placed
        .iter()
        .find(|node| node.node_id == sidebar.node_id)
        .unwrap()
        .scroll
        .offset[1];
    assert!(offset > 0., "wheel should move the sidebar, got {offset}");
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
    use plicowarden::login_progress::StepState;
    assert_eq!(
        ui.source
            .view
            .0
            .login_progress
            .steps
            .iter()
            .map(|step| step.state)
            .collect::<Vec<_>>(),
        [StepState::Complete, StepState::Complete, StepState::Failed]
    );

    ui.source.view.0.apply(Event {
        ticket: 0,
        result: Ok(Reply::Locked),
    });
    assert_eq!(
        ui.source.view.0.login_progress.steps[2].state,
        StepState::Failed
    );

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
        fields: Vec::new(),
        ssh_public_key: None,
        ssh_fingerprint: None,
        has_private_key: false,
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
