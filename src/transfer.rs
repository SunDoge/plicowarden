use crate::{
    model::{HostCommand, Warden},
    ui::*,
};
use plico::prelude::*;
use plicowarden_sdk::{Request, service::ExportKind};
use zeroize::Zeroizing;

pub struct TransferForm {
    pub format: String,
    pub import: bool,
    pub export_password: TextBuffer,
    pub confirmation: TextBuffer,
    pub import_password: TextBuffer,
    pub key_file: String,
    pub plaintext_ack: bool,
    pub import_ack: bool,
}
impl Default for TransferForm {
    fn default() -> Self {
        Self {
            format: "encrypted".into(),
            import: false,
            export_password: TextBuffer::secret(),
            confirmation: TextBuffer::secret(),
            import_password: TextBuffer::secret(),
            key_file: String::new(),
            plaintext_ack: false,
            import_ack: false,
        }
    }
}
impl TransferForm {
    pub fn clear_secrets(&mut self) {
        self.export_password.clear();
        self.confirmation.clear();
        self.import_password.clear();
        self.plaintext_ack = false;
        self.import_ack = false;
    }
    pub fn export_ready(&self) -> bool {
        if self.format == "json" {
            return self.plaintext_ack;
        }
        self.export_password.with_value(|password| {
            password.chars().count() >= 8
                && self
                    .confirmation
                    .with_value(|confirmation| password == confirmation)
        })
    }
}
pub enum TransferIntent {
    Export {
        format: ExportKind,
        password: Zeroizing<String>,
    },
    ImportKdbx {
        password: Zeroizing<String>,
        key_file: Option<std::path::PathBuf>,
    },
}
impl TransferIntent {
    /// Host-only file selection. No UI state is captured by the dialog worker.
    pub(crate) fn choose_file(self) -> Option<Request> {
        match self {
            Self::Export { format, password } => {
                let path = rfd::FileDialog::new()
                    .set_title("Export personal vault")
                    .add_filter("Bitwarden JSON", &["json"])
                    .set_file_name("plicowarden-export.json")
                    .save_file()?;
                Some(Request::ExportVault {
                    path,
                    format,
                    password,
                })
            }
            Self::ImportKdbx { password, key_file } => {
                let path = rfd::FileDialog::new()
                    .set_title("Import KeePass database")
                    .add_filter("KeePass database", &["kdbx"])
                    .pick_file()?;
                Some(Request::ImportKdbx {
                    path,
                    password,
                    key_file,
                })
            }
        }
    }
}
impl Warden {
    pub fn begin_export(&mut self) {
        if !self.authenticated || self.busy || self.transfer_dialog || !self.transfer.export_ready()
        {
            return;
        }
        let format = if self.transfer.format == "json" {
            ExportKind::Json
        } else {
            ExportKind::EncryptedJson
        };
        let password = self.transfer.export_password.take();
        self.transfer.confirmation.clear();
        self.transfer_dialog = true;
        self.error.clear();
        self.notice.clear();
        self.commands
            .push(HostCommand::Transfer(TransferIntent::Export {
                format,
                password,
            }));
    }
    pub fn begin_import(&mut self) {
        if !self.authenticated || self.busy || self.transfer_dialog || !self.transfer.import_ack {
            return;
        }
        let password = self.transfer.import_password.take();
        let key_file = (!self.transfer.key_file.trim().is_empty())
            .then(|| self.transfer.key_file.trim().into());
        self.transfer_dialog = true;
        self.error.clear();
        self.notice.clear();
        self.commands
            .push(HostCommand::Transfer(TransferIntent::ImportKdbx {
                password,
                key_file,
            }));
    }
    #[component]
    #[rustfmt::skip]
    pub fn transfer_modal_ui(&mut self, ui: &mut UiContext) {
        let mut state = std::mem::take(&mut self.transfer_modal);
        ui.modal(&mut state).header(false).title("Import / Export").width(600.).children(|ui| {
            self.transfer_ui(ui);
        });
        self.transfer_modal = state;
        if !self.transfer_modal.is_open() {
            self.transfer.clear_secrets();
        }
    }
    #[component]
    #[rustfmt::skip]
    pub fn transfer_ui(&mut self, ui: &mut UiContext) {
        let disabled = !self.authenticated || self.busy || self.transfer_dialog;
        column(ui).width_percent(100.).gap(12) + |ui| {
            row(ui).width_percent(100.).align(Align::Center).gap(12) + |ui| {
                text(ui, "Import / Export").font_size(22).weight(700).grow(1.);
                if tool(ui, "close", "Close import / export", false) {
                    ui.close_modal();
                }
            };
            row(ui).gap(8) + |ui| {
                choice(ui, "Export", &mut self.transfer.import, false);
                choice(ui, "Import", &mut self.transfer.import, true);
            };
            if !self.authenticated {
                text(ui, "Connect your account before importing or exporting your vault.").color(MUTED).width_percent(100.);
            }
            if !self.transfer.import {
                text(ui, "Export personal vault").font_size(18).weight(650);
                text(ui, "JSON preserves supported item types and folders. Organization items, trash and attachment files are excluded.")
                    .font_size(13).color(MUTED).width_percent(100.);
                let options = [
                    ("encrypted".into(), "Password-protected JSON".into()),
                    ("json".into(), "JSON (unencrypted)".into()),
                ];
                column(ui).width_percent(100.).disabled(disabled) + |ui| {
                    crate::forms::picker(ui, "Export format", &mut self.transfer.format,
                        self.popups.entry("export-format".into()).or_default(), &options);
                };
                if self.transfer.format == "encrypted" {
                    text(ui, "Export password").font_size(13).weight(600);
                    text_input(ui, "Export password", &self.transfer.export_password)
                        .width_percent(100.).disabled(disabled);
                    text(ui, "Confirm export password").font_size(13).weight(600);
                    text_input(ui, "Confirm export password", &self.transfer.confirmation)
                        .width_percent(100.).disabled(disabled);
                    text(ui, "Use at least 8 characters. You will need this password to restore the export.")
                        .font_size(12).color(MUTED).width_percent(100.);
                } else {
                    text(ui, "This file will contain your passwords in plain text.").color(Color::Warning)
                        .width_percent(100.);
                    checkbox(ui, "Export without encryption", &mut self.transfer.plaintext_ack).disabled(disabled);
                }
                text(ui, "Choose a new filename; existing files will not be overwritten.").font_size(12).color(MUTED).width_percent(100.);
                if button(ui).text("Export vault to file").icon(crate::icons::get("save")).primary().radius(6.)
                    .elevation(Elevation::CONTROL).disabled(disabled || !self.transfer.export_ready()).clicked()
                {
                    self.begin_export();
                }
            } else {
                text(ui, "Import KeePass database").font_size(18).weight(650);
                text(ui, "Choose a .kdbx file up to 10 MiB. Imported items are added to your personal vault; importing twice creates duplicates.")
                    .font_size(13).color(MUTED).width_percent(100.);
                text(ui, "Database password").font_size(13).weight(600);
                text_input(ui, "Database password", &self.transfer.import_password)
                    .width_percent(100.).disabled(disabled);
                text(ui, "Key file path (optional)").font_size(13).weight(600);
                text_input_string(ui, "Key file path", &mut self.transfer.key_file)
                    .width_percent(100.).disabled(disabled);
                checkbox(ui, "Import into my personal vault", &mut self.transfer.import_ack).disabled(disabled);
                if button(ui).text("Choose database and import").icon(crate::icons::get("plus")).primary()
                    .radius(6.).elevation(Elevation::CONTROL).disabled(disabled || !self.transfer.import_ack)
                    .clicked()
                {
                    self.begin_import();
                }
            }
            if self.transfer_dialog {
                text(ui, "Choose a file in the system dialog.").color(MUTED).width_percent(100.);
            }
        };
    }
}
