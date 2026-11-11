use plicowarden::Warden;
use plicowarden::model::{Editor, HostCommand, Page};
use plicowarden_sdk::{
    Event, Reply, Request,
    service::{EditData, TotpCode, VaultSnapshot},
};
use std::collections::BTreeMap;
use zeroize::Zeroizing;

#[test]
fn edit_patches_preserve_untouched_and_missing_fields_and_revisions() {
    let mut fields = BTreeMap::new();
    fields.insert("name".into(), "Synthetic login".into());
    fields.insert("username".into(), "demo-user".into());
    let mut editor = Editor::from_edit(EditData {
        id: "fixture-id".into(),
        kind: "login",
        revision: "revision-1".into(),
        fields,
        favorite: true,
        folder_id: None,
        custom_fields: Vec::new(),
    });
    // Merely declaring an absent field must not clear server data.
    editor.buffer("totp", true, false);
    assert!(editor.patch().fields.is_empty());
    editor
        .buffer("username", false, false)
        .set(Zeroizing::new("updated-demo-user".into()));
    let patch = editor.patch();
    assert_eq!(patch.fields.len(), 1);
    assert!(patch.fields.contains_key("username"));
    assert_eq!(patch.expected_revision.as_deref(), Some("revision-1"));
}

#[test]
fn expiring_totp_is_replaced_without_waiting_for_the_user() {
    let mut app = Warden::default();
    app.authenticated = true;
    app.selected = app.items[0].id.clone();
    app.items[0].has_totp = true;
    app.code = Some(TotpCode {
        code: "123456".into(),
        period: 30,
        expires_at: 300,
        valid_for: 12,
    });
    app.code_received_at = Some(std::time::Duration::from_secs(4));

    app.apply(Event {
        ticket: 0,
        result: Ok(Reply::TotpExpired(300)),
    });

    assert!(app.code.is_none() && app.code_received_at.is_none());
    assert!(matches!(
        app.commands.last(),
        Some(HostCommand::Account { request, .. })
            if matches!(request.as_ref(), Request::Totp(id) if id == &app.selected)
    ));
}
#[test]
fn lock_erases_editor_buffers_and_stale_replies_cannot_reveal_values() {
    let mut app = Warden::default();
    app.editor = Some(Editor::new());
    let editor = app.editor.as_mut().unwrap();
    let binding = editor.buffer("password", true, false);
    binding.set(Zeroizing::new("synthetic-editor-value".into()));
    app.pending = Some(7);
    app.busy = true;
    app.lock();
    assert!(binding.is_empty());
    assert!(app.pending.is_none());
    assert!(!app.busy);
    assert!(app.items.is_empty());
    assert!(matches!(app.commands.as_slice(), [HostCommand::Disconnect]));
    app.apply(Event {
        ticket: 7,
        result: Ok(Reply::Revealed(Zeroizing::new(
            "synthetic-delayed-reply".into(),
        ))),
    });
    assert!(app.shown_password.is_none());
    assert!(!app.reveal);
}
#[test]
fn generator_uses_each_requested_character_group_and_length() {
    for symbols in [true, false] {
        for length in [8, 24, 128] {
            let value = plicowarden::generator::generate(length, symbols).unwrap();
            assert_eq!(value.len(), length);
            assert!(value.bytes().any(|b| b.is_ascii_uppercase()));
            assert!(value.bytes().any(|b| b.is_ascii_lowercase()));
            assert!(value.bytes().any(|b| b.is_ascii_digit()));
            assert_eq!(value.bytes().any(|b| !b.is_ascii_alphanumeric()), symbols);
        }
    }
}
#[test]
fn appearance_is_the_only_persisted_projection_and_roundtrips() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("appearance.json");
    let value = plicowarden::appearance::Appearance { ui_scale: 0.85 };
    value.save(&path).unwrap();
    let restored = plicowarden::appearance::Appearance::load(&path).unwrap();
    assert_eq!(restored.ui_scale, 0.85);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains("password"));
    assert!(!text.contains("items"));
}

#[test]
fn passphrases_and_usernames_use_the_full_dictionary_and_bounds() {
    let words: std::collections::BTreeSet<_> = include_str!("../assets/eff-short-words.txt")
        .lines()
        .collect();
    assert_eq!(words.len(), 1296);
    // "yo-yo" is part of the EFF list, so splitting on '-' cannot count the
    // words. Parse the phrase instead: each word is followed by one separator.
    fn composed(phrase: &str, count: usize, words: &std::collections::BTreeSet<&str>) -> bool {
        if count == 0 {
            return phrase.is_empty();
        }
        words.iter().any(|word| {
            phrase.strip_prefix(word).is_some_and(|rest| {
                if count == 1 {
                    rest.is_empty()
                } else {
                    rest.strip_prefix('-')
                        .is_some_and(|rest| composed(rest, count - 1, words))
                }
            })
        })
    }
    for count in [0, 6, 9, 12, usize::MAX] {
        let result = plicowarden::generator::passphrase(count).unwrap();
        assert!(
            composed(&result, count.clamp(6, 12), &words),
            "{}",
            &*result
        );
    }
    let value = plicowarden::generator::username().unwrap();
    let (words_part, digits) = value.rsplit_once('-').unwrap();
    assert!(composed(words_part, 2, &words), "{}", &*value);
    assert_eq!(digits.len(), 4);
    assert!(digits.bytes().all(|b| b.is_ascii_digit()));
}

#[test]
fn history_reply_is_scoped_to_selection_and_erased_on_filter_and_lock() {
    let mut app = Warden::default();
    app.pending = Some(42);
    app.history_ticket = Some(42);
    app.apply(Event {
        ticket: 42,
        result: Ok(Reply::PasswordHistory {
            id: "github".into(),
            entries: vec![(123, Zeroizing::new("synthetic-old-password".into()))],
        }),
    });
    assert!(app.reveal_history);
    assert_eq!(app.previous_passwords.len(), 1);
    app.set_filter("Favorites");
    assert!(!app.reveal_history);
    assert!(app.previous_passwords.is_empty());
    app.pending = Some(43);
    app.apply(Event {
        ticket: 43,
        result: Ok(Reply::PasswordHistory {
            id: "another-item".into(),
            entries: vec![(123, Zeroizing::new("synthetic-unrelated".into()))],
        }),
    });
    assert!(app.previous_passwords.is_empty());
    app.pending = Some(44);
    app.history_ticket = Some(44);
    app.set_filter("All Items");
    app.apply(Event {
        ticket: 44,
        result: Ok(Reply::PasswordHistory {
            id: "github".into(),
            entries: vec![(123, Zeroizing::new("synthetic-delayed-history".into()))],
        }),
    });
    assert!(!app.reveal_history && app.previous_passwords.is_empty());
    app.generator_mode = plicowarden::generator::Mode::Passphrase;
    app.generate();
    assert_eq!(app.history.len(), 1);
    app.lock();
    assert!(
        app.generated.is_empty() && app.history.is_empty() && app.previous_passwords.is_empty()
    );
}

#[test]
fn verification_guards_and_back_navigation_drop_pending_secrets() {
    use plicowarden_sdk::service::{TwoFactorOption, TwoFactorProviderId};
    let mut app = Warden::default();
    app.open_login();
    app.login_password_step = true;
    app.email = "synthetic@example.invalid".into();
    app.factors = vec![TwoFactorOption {
        id: TwoFactorProviderId::Authenticator,
        label: "Authenticator",
        hint: None,
        supported: true,
    }];
    app.factor = "webauthn".into();
    app.otp.set(Zeroizing::new("123456".into()));
    app.submit_verification();
    assert!(!app.busy);
    app.factor = "authenticator".into();
    app.submit_verification();
    assert!(app.busy);
    assert!(app.otp.is_empty());
    let ticket = app.pending.unwrap();
    app.apply(Event {
        ticket,
        result: Err("Synthetic invalid verification code".into()),
    });
    assert!(app.login_focus);
    app.otp.set(Zeroizing::new("654321".into()));
    app.back_to_email();
    assert!(app.factors.is_empty());
    assert!(app.otp.is_empty());
    assert!(!app.login_password_step);
    assert_eq!(app.email, "synthetic@example.invalid");
    assert!(matches!(app.commands.last(), Some(HostCommand::Disconnect)));
}

#[test]
fn login_error_and_timeout_keep_secret_for_retry_but_explicit_lock_erases_it() {
    let mut app = Warden::default();
    app.open_login();
    app.login_password_step = true;
    app.email = "test@example.invalid".into();
    app.password
        .set(Zeroizing::new("synthetic-retry-password".into()));
    app.login();
    let ticket = app.pending.unwrap();
    app.apply(Event {
        ticket,
        result: Err("Synthetic timeout".into()),
    });
    assert!(!app.password.is_empty());
    app.apply(Event {
        ticket: 0,
        result: Ok(Reply::Locked),
    });
    assert!(app.login_password_step);
    app.password
        .with_value(|value| assert_eq!(value, "synthetic-retry-password"));
    app.login();
    assert!(app.busy);
    app.lock();
    assert!(app.password.is_empty());
}

fn remembered_account(server: &str) -> plicowarden_sdk::cache::SavedAccount {
    let config = plicowarden_sdk::service::LoginRequest {
        email: "same@example.invalid".into(),
        region: "self-hosted".into(),
        server_url: Some(server.into()),
        server_config: None,
    };
    plicowarden_sdk::cache::SavedAccount {
        id: plicowarden_sdk::cache::account_id(&config).unwrap(),
        config,
        cached_at: 123,
    }
}
#[test]
fn switching_accounts_clears_decrypted_state_and_restores_the_selected_server() {
    let first = remembered_account("https://one.example.invalid");
    let second = remembered_account("https://two.example.invalid");
    let mut app = Warden::default();
    app.saved_accounts = vec![first.clone(), second.clone()];
    app.authenticated = true;
    app.offline = true;
    app.active_account_id = Some(first.id);
    app.password
        .set(Zeroizing::new("synthetic-previous-account-password".into()));
    app.shown_password = Some(Zeroizing::new("synthetic-item-secret".into()));
    app.search_matches = Some(["private-result".into()].into_iter().collect());
    app.choose_account(&second);
    assert!(!app.authenticated);
    assert!(app.active_account_id.is_none());
    assert!(app.items.is_empty() && app.details.is_none() && app.shown_password.is_none());
    assert!(app.search_matches.is_none());
    assert!(app.password.is_empty());
    assert_eq!(app.server, "https://two.example.invalid");
    assert!(app.has_cached_account() && app.login_password_step);
    assert_eq!(app.saved_accounts.len(), 2);
    app.add_account();
    assert!(app.email.is_empty() && app.adding_account && !app.login_password_step);
}
#[test]
fn stale_search_and_foreign_account_background_sync_cannot_replace_active_state() {
    use plicowarden_sdk::service::VaultSnapshot;
    let mut app = Warden::default();
    app.authenticated = true;
    app.account_email = "same@example.invalid".into();
    app.active_account_id = Some("account-one".into());
    let id = app.items[0].id.clone();
    app.query = "notes-only-marker".into();
    app.update_local_search();
    let search_ticket = app.search_ticket.unwrap();
    app.select("alarm");
    let detail_ticket = app.pending;
    app.apply(Event {
        ticket: search_ticket,
        result: Ok(Reply::SearchResults {
            query: "notes-only-marker".into(),
            ids: vec![id.clone()],
        }),
    });
    assert_eq!(app.pending, detail_ticket);
    assert!(!app.busy);
    assert_eq!(
        app.visible()
            .iter()
            .map(|item| &item.id)
            .collect::<Vec<_>>(),
        vec![&id]
    );
    app.apply(Event {
        ticket: 0,
        result: Ok(Reply::BackgroundSnapshot(VaultSnapshot {
            email: app.account_email.clone(),
            items: Vec::new(),
            folders: Vec::new(),
            decrypt_failures: 0,
            offline: false,
            account_id: Some("account-two".into()),
            cache_warning: None,
        })),
    });
    assert!(!app.items.is_empty());
    app.apply(Event {
        ticket: 0,
        result: Ok(Reply::BackgroundSyncFailed(
            "synthetic transport timeout".into(),
        )),
    });
    assert!(app.authenticated);
    assert_eq!(app.pending, detail_ticket);
    assert!(app.error.is_empty());
    assert!(app.notifications.iter().any(|notification| {
        notification.kind == plicowarden::model::NotificationKind::Error
            && notification.detail.contains("synthetic transport timeout")
    }));

    app.lock();
    assert!(app.search_matches.is_none() && app.search_ticket.is_none());
}

#[test]
fn type_sort_groups_vault_kinds_in_sidebar_order() {
    let mut app = Warden::default();
    app.sort_by_type = true;
    let rank = |kind: &str| match kind {
        "login" => 0,
        "card" => 1,
        "identity" => 2,
        "note" => 3,
        "ssh-key" => 4,
        _ => 5,
    };
    let kinds: Vec<_> = app.visible().iter().map(|item| item.kind.clone()).collect();
    assert!(kinds.len() > 1);
    assert!(
        kinds
            .windows(2)
            .all(|pair| rank(&pair[0]) <= rank(&pair[1]))
    );
    assert!(kinds.iter().any(|kind| kind == "card"));
    assert!(kinds.iter().any(|kind| kind == "ssh-key"));
    app.sort_by_type = false;
    assert!(
        app.visible()
            .windows(2)
            .all(|pair| { pair[0].name.to_lowercase() <= pair[1].name.to_lowercase() })
    );
}

#[test]
fn online_snapshots_record_the_last_sync_time() {
    let snapshot = |offline| VaultSnapshot {
        email: "sync@example.invalid".into(),
        items: Vec::new(),
        folders: Vec::new(),
        decrypt_failures: 0,
        offline,
        account_id: Some("account".into()),
        cache_warning: None,
    };
    let mut app = Warden::default();
    app.snapshot(snapshot(true));
    assert!(app.last_sync_at.is_none());
    app.snapshot(snapshot(false));
    assert!(app.last_sync_at.is_some());
    app.lock();
    assert!(app.last_sync_at.is_none());
}

#[test]
fn unauthorized_background_sync_returns_to_sign_in() {
    let mut app = Warden::default();
    app.authenticated = true;
    app.account_email = "user@example.invalid".into();
    app.email = "user@example.invalid".into();
    app.apply(Event {
        ticket: 0,
        result: Ok(Reply::BackgroundSyncFailed(
            "Background sync failed: error in response: status code 401 Unauthorized".into(),
        )),
    });
    assert!(app.authenticated);
    assert!(app.notice.contains("Session expired"));
    assert!(app.error.is_empty());
    assert_eq!(app.unread_notifications(), 1);
    assert!(app.notifications.iter().any(|notification| {
        notification.kind == plicowarden::model::NotificationKind::SessionExpired
    }));
    app.expire_session();
    assert!(!app.authenticated);
    assert!(app.router.show(Page::Login));
    assert!(!app.login_password_step);
    assert_eq!(app.email, "user@example.invalid");
    assert!(app.notice.contains("Session expired"));
}

#[test]
fn login_returns_to_the_page_that_opened_it() {
    let mut app = Warden::default();
    app.open_generator();
    app.open_login();
    assert!(app.router.show(Page::Login));
    app.close_login();
    assert!(app.router.show(Page::Generator));
    app.set_filter("Favorites");
    assert!(app.router.show(Page::Vault));
    assert_eq!(app.filter, "Favorites");
}

#[test]
fn notifications_stay_unread_until_the_panel_opens() {
    let mut app = Warden::default();
    app.push_notification(
        plicowarden::model::NotificationKind::Error,
        "Error",
        "Background sync failed",
        None,
    );
    app.push_notification(
        plicowarden::model::NotificationKind::LoginRequest,
        "Device signing request",
        "Studio workstation",
        None,
    );
    assert_eq!(app.unread_notifications(), 2);
    app.toggle_notifications();
    assert!(app.notification_open);
    assert_eq!(app.unread_notifications(), 1);
    app.clear_read_notifications();
    assert_eq!(app.notifications.len(), 1);
    assert_eq!(
        app.notifications
            .front()
            .map(|notification| notification.kind),
        Some(plicowarden::model::NotificationKind::LoginRequest)
    );
    app.toggle_notifications();
    assert!(!app.notification_open);
}
