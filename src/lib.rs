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
pub mod model;
pub mod security;
pub mod transfer;
mod ui;
pub use model::Warden;

impl plico_dev::App for Warden {
    fn show(&mut self, ui: &mut plico::UiContext<'_>) {
        self.appearance.ui_scale = self.zoom.factor();
        self.show_ui(ui);
    }
    fn theme(&self) -> plico::Theme {
        let mut theme = plico::Theme::light();
        theme.foreground = 0x243443;
        theme.muted = 0x526674;
        theme.panel = 0xffffff;
        theme.control = 0xf8f8fa;
        theme.border = 0xe5e4e9;
        theme.control_hover = 0xefedf3;
        theme.control_pressed = 0xe7e3ee;
        theme.selection = 0xe6e1ee;
        theme.accent = 0x286c85;
        theme.metrics.font_size = 15;
        theme
    }
    fn on_key(&mut self, key: &str) -> bool {
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
        app.appearance.liquid_glass = !self.opaque;
        match self.screen.as_str() {
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
                        name: "Personal".into(),
                    },
                ];
                for item in &mut app.items {
                    item.folder = (item.id == "github")
                        .then(|| "00000000-0000-4000-8000-000000000001".into());
                }
                app.details = Some(synthetic_folder_details());
            }
            "bulk" => {
                app.authenticated = true;
                app.account_email = "recipe@example.invalid".into();
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
