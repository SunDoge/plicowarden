//! GPL-3.0-only Rust frontend of Warden, aligned with the independent C# app.
pub mod appearance;
mod bulk;
pub mod desktop;
mod detail;
pub mod display;
mod forms;
pub mod generator;
pub mod host;
mod icons;
pub mod login_progress;
mod lucide_icons;
pub mod model;
pub mod security;
pub mod transfer;
mod ui;
pub use model::Warden;

impl plico_dev::App for Warden {
    fn set_frame_time(&mut self, time: plico::FrameTime) {
        self.frame_time = time;
    }
    fn show(&mut self, ui: &mut plico::UiContext<'_>) {
        self.appearance.ui_scale = self.zoom.factor();
        self.show_ui(ui);
    }
    fn theme(&self) -> plico::Theme {
        // MyGO ui/theme.go (9b265767): keep the pinned framework's theme
        // aligned until its next Git dependency update carries these defaults.
        let mut theme = plico::Theme::light();
        theme.foreground = 0x18181b;
        theme.surface = 0xffffff;
        theme.muted = 0x71717a;
        theme.panel = 0xffffff;
        theme.control = 0xf4f4f5;
        theme.border = 0xd9d9de;
        theme.control_hover = 0xe9e9ec;
        theme.control_pressed = 0xdddde1;
        theme.selection = 0xc9d8fa;
        theme.accent = 0x2563eb;
        theme.focus = 0x87a9f4;
        theme.success = 0x16a34a;
        theme.warning = 0xd97706;
        theme.danger = 0xdc2626;
        // Match Triwarden's macOS hierarchy on every desktop backend; backing
        // scale must not silently turn Linux text into a larger design system.
        theme.metrics.font_size = 13;
        theme
    }
    fn on_key_event(&mut self, key: &plico::KeyEvent) -> bool {
        if key.phase == plico::KeyPhase::Down && !key.synthetic && key.modifiers.primary_shortcut()
        {
            self.notification_open = false;
            match key.key.as_str() {
                "f" | "F" => {
                    self.palette_query.clear();
                    self.palette_index = 0;
                    self.palette_modal.open();
                    return true;
                }
                "k" | "K" => {
                    if self.palette_modal.is_open() {
                        self.palette_modal.close();
                    } else {
                        self.settings_open = false;
                        self.palette_query.clear();
                        self.palette_index = 0;
                        self.palette_modal.open();
                    }
                    return true;
                }
                "," => {
                    self.palette_modal.close();
                    self.settings_open = !self.settings_open;
                    return true;
                }
                "g" | "G" => {
                    self.palette_modal.close();
                    self.settings_open = false;
                    self.set_filter("Generator");
                    return true;
                }
                "n" | "N" if self.authenticated && !self.busy => {
                    self.palette_modal.close();
                    self.settings_open = false;
                    self.editor = Some(crate::model::Editor::new());
                    return true;
                }
                "l" | "L" if self.authenticated => {
                    self.palette_modal.close();
                    self.lock();
                    return true;
                }
                _ => {}
            }
        }
        if key.phase == plico::KeyPhase::Down
            && !key.synthetic
            && key.modifiers.is_empty()
            && !self.login_open
            && !self.settings_open
            && self.editor.is_none()
            && !self.palette_modal.is_open()
        {
            match key.key.as_str() {
                "ArrowDown" if self.move_selection(1) => return true,
                "ArrowUp" if self.move_selection(-1) => return true,
                _ => {}
            }
        }
        key.phase == plico::KeyPhase::Down
            && !key.synthetic
            && key.modifiers.is_empty()
            && self.on_key(&key.key)
    }
    fn on_key(&mut self, key: &str) -> bool {
        if key == "Escape" && self.notification_open {
            self.notification_open = false;
            return true;
        }
        if key == "Escape" && !self.busy {
            if self.bulk_mode {
                self.bulk_mode = false;
                self.bulk_selected.clear();
                self.bulk_modal.close();
                return true;
            }
            if self.login_open {
                if self.environment_open {
                    self.environment_open = false;
                    return true;
                }
                if self.login_password_step || !self.factors.is_empty() {
                    self.back_to_email();
                    return true;
                }
            }
            if let Some(approval) = self.ssh_approvals.front() {
                self.decide_ssh(approval.id, false);
                return true;
            }
            if self.editor.is_some() {
                self.editor = None;
                return true;
            }
            if self.settings_open {
                self.settings_open = false;
                return true;
            }
        }
        false
    }
}
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct PreviewConfig {
    pub screen: String,
    pub query: String,
    pub filter: Option<String>,
    pub selected: Option<String>,
    /// Legacy preview input retained for old recipes; system transparency preferences now win.
    pub opaque: bool,
    #[serde(alias = "native_titlebar")]
    pub custom_titlebar: bool,
}
impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            screen: String::new(),
            query: String::new(),
            filter: None,
            selected: None,
            opaque: false,
            custom_titlebar: true,
        }
    }
}
impl PreviewConfig {
    pub fn app(&self) -> Warden {
        let mut app = Warden::default();
        // Reference fixtures intentionally use 100% density on every platform.
        app.zoom.set(1.);
        app.custom_titlebar = self.custom_titlebar;
        app.query = self.query.clone();
        if let Some(filter) = &self.filter {
            app.filter = filter.clone();
        }
        if let Some(selected) = &self.selected {
            app.selected = selected.clone();
        }
        match self.screen.as_str() {
            "accounts" => {
                for server in ["https://one.example.invalid", "https://two.example.invalid"] {
                    let config = plicowarden_sdk::service::LoginRequest {
                        email: "same@example.invalid".into(),
                        region: "self-hosted".into(),
                        server_url: Some(server.into()),
                        server_config: None,
                    };
                    app.saved_accounts
                        .push(plicowarden_sdk::cache::SavedAccount {
                            id: plicowarden_sdk::cache::account_id(&config)
                                .expect("valid synthetic account"),
                            config,
                            cached_at: 123,
                        });
                }
                app.active_account_id =
                    app.saved_accounts.first().map(|account| account.id.clone());
                app.accounts_modal.open();
            }
            "ssh-approval" => {
                // Synthetic preview only: no host/client, worker, socket or real key.
                app.authenticated = true;
                app.account_email = "recipe@example.invalid".into();
                app.settings_open = true;
                app.settings_tab = "ssh".into();
                app.ssh_socket = Some("/synthetic/plicowarden/agent.sock".into());
                app.apply(plicowarden_sdk::Event {
                    ticket: 0,
                    result: Ok(plicowarden_sdk::Reply::SshApproval(
                        plicowarden_sdk::ssh_agent::Approval {
                            id: 1,
                            name: "Synthetic SSH key".into(),
                            fingerprint: "SHA256:synthetic-preview-not-a-real-fingerprint".into(),
                            process_id: Some(1234),
                        },
                    )),
                });
            }
            "ssh-history" => {
                // Synthetic preview only: fixed public metadata, no worker or key.
                use plicowarden_sdk::ssh_agent::{AccessRecord, Outcome};
                app.authenticated = true;
                app.account_email = "recipe@example.invalid".into();
                app.settings_open = true;
                app.settings_tab = "ssh".into();
                app.ssh_view = crate::model::SSH_HISTORY;
                app.ssh_socket = Some("/synthetic/plicowarden/agent.sock".into());
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|elapsed| elapsed.as_secs() as i64)
                    .unwrap_or(0);
                for (id, name, outcome, age, process_id) in [
                    (1, "Synthetic SSH key", Outcome::Allowed, 15, Some(4242)),
                    (2, "Synthetic SSH key", Outcome::Denied, 180, Some(4242)),
                    (3, "Release signing key", Outcome::TimedOut, 7200, None),
                    (4, "Synthetic SSH key", Outcome::Locked, 90000, Some(31337)),
                ] {
                    app.ssh_history.push(AccessRecord {
                        id,
                        name: name.into(),
                        fingerprint: format!("SHA256:synthetic-preview-{id}"),
                        process_id,
                        outcome,
                        at: now - age,
                    });
                }
            }
            "transfer" => {
                app.authenticated = true;
                app.account_email = "recipe@example.invalid".into();
                app.transfer_modal.open();
            }
            "folders" => {
                app.authenticated = true;
                app.account_email = "recipe@example.invalid".into();
                app.folders = vec![
                    plicowarden_sdk::service::VaultFolder {
                        id: "00000000-0000-4000-8000-000000000001".into(),
                        name: "Work".into(),
                    },
                    plicowarden_sdk::service::VaultFolder {
                        id: "00000000-0000-4000-8000-000000000002".into(),
                        name: "Work/Backend".into(),
                    },
                    plicowarden_sdk::service::VaultFolder {
                        id: "00000000-0000-4000-8000-000000000003".into(),
                        name: "Work/Frontend".into(),
                    },
                    plicowarden_sdk::service::VaultFolder {
                        id: "00000000-0000-4000-8000-000000000004".into(),
                        name: "Personal".into(),
                    },
                ];
                for item in &mut app.items {
                    item.folder = (item.id == "github")
                        .then(|| "00000000-0000-4000-8000-000000000001".into());
                    if item.id == "notion" {
                        item.folder = Some("00000000-0000-4000-8000-000000000002".into());
                    }
                }
                app.details = Some(synthetic_folder_details());
            }
            "bulk" => {
                app.authenticated = true;
                app.account_email = "recipe@example.invalid".into();
            }
            "ssh-key" => {
                app.authenticated = true;
                app.account_email = "recipe@example.invalid".into();
                app.set_filter("SSH Keys");
                app.selected = "ssh".into();
                app.details = Some(synthetic_ssh_details());
            }
            "many-folders" => {
                app.authenticated = true;
                app.account_email = "recipe@example.invalid".into();
                app.folders = (0..24)
                    .map(|index| plicowarden_sdk::service::VaultFolder {
                        id: format!("00000000-0000-4000-8000-00000000{index:04}"),
                        name: format!("Work/Project {index:02}"),
                    })
                    .collect();
                app.folders_modal.open();
            }
            "notifications" => {
                app.authenticated = true;
                app.account_email = "recipe@example.invalid".into();
                app.ssh_socket = Some("/synthetic/plicowarden/agent.sock".into());
                app.push_notification(
                    crate::model::NotificationKind::Error,
                    "Background sync failed",
                    "The local vault is still available; retry when the network returns.",
                    None,
                );
                app.push_notification(
                    crate::model::NotificationKind::SessionExpired,
                    "Session expired",
                    "Sign in again to continue syncing this vault.",
                    None,
                );
                app.push_notification(
                    crate::model::NotificationKind::LoginRequest,
                    "Device signing request",
                    "Studio workstation · SHA256:synthetic-preview-notification",
                    Some(7),
                );
                app.notification_open = true;
            }
            "empty" => {
                app.authenticated = true;
                app.account_email = "recipe@example.invalid".into();
                app.items.clear();
                app.folders.clear();
                app.selected.clear();
                app.filter = "All Items".into();
                app.details = None;
            }
            "fields" => {
                app.authenticated = true;
                app.account_email = "recipe@example.invalid".into();
                app.selected = "github".into();
                app.details = Some(synthetic_fields_details());
            }
            "totp" => {
                app.authenticated = true;
                app.account_email = "recipe@example.invalid".into();
                app.set_filter("All Items");
                app.selected = "github".into();
                app.details = Some(synthetic_totp_details());
                app.code = Some(plicowarden_sdk::service::TotpCode {
                    code: "522 719".into(),
                    period: 30,
                    expires_at: 0,
                    valid_for: 30,
                });
                app.code_received_at = Some(std::time::Duration::ZERO);
            }
            "card" => {
                app.authenticated = true;
                app.account_email = "recipe@example.invalid".into();
                app.set_filter("Cards");
                app.selected = "amex".into();
                app.details = Some(synthetic_card_details());
            }
            "settings" => app.settings_open = true,
            "security" => {
                app.settings_open = true;
                app.settings_tab = "security".into();
            }
            "login" => app.login_open = true,
            "generator" => app.filter = "Generator".into(),
            "editor" => app.editor = Some(model::Editor::new()),
            _ => {}
        }
        app
    }
}

// Synthetic UI fixture only; never constructed by the real account host.
fn synthetic_folder_details() -> plicowarden_sdk::service::ItemDetails {
    plicowarden_sdk::service::ItemDetails {
        id: "github".into(),
        name: "GitHub".into(),
        kind: "login",
        username: Some("recipe@example.invalid".into()),
        password: None,
        uris: vec!["https://github.com".into()],
        notes: None,
        totp: None,
        cardholder_name: None,
        card_brand: None,
        card_number: None,
        card_exp_month: None,
        card_exp_year: None,
        card_code: None,
        favorite: true,
        has_password: true,
        has_totp: false,
        has_card_number: false,
        has_card_code: false,
        editable: true,
        folder_id: Some("00000000-0000-4000-8000-000000000001".into()),
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
        password_history: vec![],
    }
}

fn synthetic_card_details() -> plicowarden_sdk::service::ItemDetails {
    let mut detail = synthetic_folder_details();
    detail.id = "amex".into();
    detail.name = "American Express".into();
    detail.kind = "card";
    detail.username = None;
    detail.password = None;
    detail.uris = Vec::new();
    detail.cardholder_name = Some("Alex Chen".into());
    detail.card_brand = Some("American Express".into());
    detail.card_number = Some("3782 822463 10005".into());
    detail.card_exp_month = Some("12".into());
    detail.card_exp_year = Some("2030".into());
    detail.card_code = Some("1234".into());
    detail.has_card_number = true;
    detail.has_card_code = true;
    detail.has_password = false;
    detail.has_totp = false;
    detail.editable = true;
    detail
}

fn synthetic_ssh_details() -> plicowarden_sdk::service::ItemDetails {
    let mut detail = synthetic_folder_details();
    detail.id = "ssh".into();
    detail.name = "Studio workstation".into();
    detail.kind = "ssh-key";
    detail.username = Some("alex@studio".into());
    detail.password = None;
    detail.uris = Vec::new();
    detail.folder_id = None;
    detail.has_password = false;
    detail.has_totp = false;
    detail.ssh_public_key =
        Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIExampleKeyForPreviewOnly alex@studio".into());
    detail.ssh_fingerprint = Some("SHA256:8f3Jc1n0YpQ2r5s7t9uVwXyZaBcDeFgHiJkLmNoPqRs".into());
    detail.has_private_key = true;
    detail
}

fn synthetic_totp_details() -> plicowarden_sdk::service::ItemDetails {
    let mut detail = synthetic_folder_details();
    detail.id = "github".into();
    detail.name = "GitHub".into();
    detail.kind = "login";
    detail.has_totp = true;
    detail.totp = Some("otpauth://totp/GitHub:alexchen?secret=PREVIEW".into());
    detail
}

fn synthetic_fields_details() -> plicowarden_sdk::service::ItemDetails {
    let mut detail = synthetic_folder_details();
    detail.fields = vec![
        plicowarden_sdk::service::DetailField {
            name: Some("Recovery email".into()),
            value: Some("backup@example.invalid".into()),
            hidden: false,
            boolean: false,
        },
        plicowarden_sdk::service::DetailField {
            name: Some("API token".into()),
            value: None,
            hidden: true,
            boolean: false,
        },
        plicowarden_sdk::service::DetailField {
            name: Some("2FA enabled".into()),
            value: Some("true".into()),
            hidden: false,
            boolean: true,
        },
    ];
    detail
}
