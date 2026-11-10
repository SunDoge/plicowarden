use plicowarden::Warden;
use plicowarden::model::{Editor, HostCommand};
use plicowarden_sdk::{Event, Reply, service::EditData};
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
    for count in [0, 6, 9, 12, usize::MAX] {
        let result = plicowarden::generator::passphrase(count).unwrap();
        let parts: Vec<_> = result.split('-').collect();
        assert_eq!(parts.len(), count.clamp(6, 12));
        assert!(parts.iter().all(|word| words.contains(word)));
    }
    let value = plicowarden::generator::username().unwrap();
    let parts: Vec<_> = value.split('-').collect();
    assert_eq!(parts.len(), 3);
    assert!(words.contains(parts[0]) && words.contains(parts[1]));
    assert_eq!(parts[2].len(), 4);
    assert!(parts[2].bytes().all(|b| b.is_ascii_digit()));
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
    app.login_open = true;
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
    app.login_open = true;
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
    assert!(
        app.error.contains("synthetic transport timeout"),
        "background diagnostics must use the copyable error surface"
    );

    app.lock();
    assert!(app.search_matches.is_none() && app.search_ticket.is_none());
}
