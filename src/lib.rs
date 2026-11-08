//! GPL-3.0-only Rust frontend of Warden, aligned with the independent C# app.
pub mod appearance;
mod detail;
mod forms;
pub mod generator;
pub mod host;
mod icons;
pub mod model;
mod ui;
pub use model::Warden;

impl plico_dev::App for Warden {
    fn show(&mut self, ui: &mut plico::UiContext<'_>) {
        self.show_ui(ui);
    }
    fn theme(&self) -> plico::Theme {
        let mut theme = plico::Theme::light();
        theme.foreground = 0x243443;
        theme.muted = 0x647684;
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
#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct PreviewConfig {
    pub screen: String,
    pub query: String,
    pub filter: Option<String>,
    pub selected: Option<String>,
    pub opaque: bool,
}
impl PreviewConfig {
    pub fn app(&self) -> Warden {
        let mut app = Warden::default();
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
            "settings" => app.settings_open = true,
            "login" => app.login_open = true,
            "generator" => app.filter = "Generator".into(),
            "editor" => app.editor = Some(model::Editor::new()),
            _ => {}
        }
        app
    }
}
