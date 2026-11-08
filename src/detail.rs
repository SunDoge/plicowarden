use crate::{
    model::{Item, Warden},
    ui::*,
};
use plico::prelude::*;
use plicowarden_sdk::Request;

#[component]
#[rustfmt::skip]
fn meta(ui: &mut UiContext, icon: &str, label: &str, value: &str) {
    row(ui).width_percent(100.).align(Align::Center).gap(10).padding_y(4) + |ui| {
        glyph(ui, icon, 18., Color::Literal(0x777781));
        text(ui, label).grow(1.);
        text(ui, value).font_size(14).color(MUTED);
    };
}
impl Warden {
    #[component]
    #[rustfmt::skip]
    pub fn detail(&mut self, ui: &mut UiContext, item: &Item, compact: bool) {
        if self.authenticated {
            self.live_detail(ui, item);
            return;
        }
        surface(ui).material(self.appearance.material()).padding(26).gap(10) + |ui| {
            row(ui).width_percent(100.).align(Align::Center).gap(18) + |ui| {
                brand(ui, item, 62.);
                column(ui).width(0.).grow(1.).min_width(0.).gap(5) + |ui| {
                    text(ui, &item.name).font_size(34).weight(750).width_percent(100.);
                    text(ui, if item.site.is_empty() { "Personal vault" } else { &item.site }).font_size(16).color(MUTED);
                };
                glyph(ui, if item.favorite { "star" } else { "shield" }, 22.,
                    if item.favorite { Color::Literal(0xc39831) } else { MUTED });
            };
        };
        surface(ui).padding(18).gap(8) + |ui| {
            text(ui, "Username").font_size(13).color(MUTED);
            row(ui).width_percent(100.).align(Align::Center).justify(Justify::SpaceBetween) + |ui| {
                text(ui, &item.user).font_size(17);
                if tool(ui, "copy", "Copy username", false) {
                    self.preview_notice();
                }
            };
        };
        row(ui).width_percent(100.).gap(12).align(Align::Stretch).wrap(compact).shrink(0.) + |ui| {
            surface(ui).width(0.).grow(1.7).min_width(if compact { 220. } else { 0. }).padding(18).gap(10) + |ui| {
                row(ui).width_percent(100.).align(Align::Center).justify(Justify::SpaceBetween) + |ui| {
                    text(ui, "Password").font_size(13).color(MUTED);
                    if tool(ui, "eye", if self.reveal { "Hide password" } else { "Show password" }, false) {
                        self.reveal = !self.reveal;
                    }
                };
                text(ui, if self.reveal { "sample-only-7X!" } else { "••••••••••••••••" }).font_size(22).weight(600);
                column(ui).width_percent(100.).height(4.).radius(2.).background(Color::Literal(0x64b778)).end();
                text(ui, "Strong · unique").font_size(12).color(Color::Literal(0x39784c));
            };
            surface(ui).width(0.).grow(1.).min_width(if compact { 220. } else { 0. }).padding(18).gap(12) + |ui| {
                text(ui, "One-time code").font_size(13).color(MUTED);
                row(ui).width_percent(100.).align(Align::Center).gap(12) + |ui| {
                    text(ui, "522 · 719").font_size(26).weight(650).grow(1.);
                    column(ui).width(32.).height(32.).shrink(0.).radius(16.)
                        .border(2., Color::Literal(0x598dba)).align(Align::Center).justify(Justify::Center) + |ui|
                    {
                        text(ui, "15").font_size(11).color(MUTED);
                    };
                };
                text(ui, "Demonstration code").font_size(12).color(MUTED);
            };
        };
        surface(ui).padding(20).gap(18) + |ui| {
            text(ui, "Website").font_size(13).color(MUTED);
            row(ui).width_percent(100.).align(Align::Center).gap(10) + |ui| {
                glyph(ui, "external", 18., ICON);
                let site = if item.site.is_empty() {
                    "No website".into()
                } else {
                    format!("https://{}", item.site)
                };
                text(ui, &site).width_percent(100.).grow(1.).font_size(16);
            };
            meta(ui, "person", "Vault", "My vault");
            meta(ui, "key", "Passkey", if item.site.is_empty() { "Not configured" } else { "Available" });
            meta(ui, "shield", "Watchtower", "No issues found");
            column(ui).width_percent(100.).height(1.).background(Color::Literal(0xe4e3e9)).background_alpha(0.7)
                .end();
            text(ui, "NOTES").font_size(13).color(MUTED);
            text(ui, "Recovery codes are in the “Account recovery” note.").width_percent(100.).font_size(15);
        };
        surface(ui).padding(20).gap(16) + |ui| {
            row(ui).width_percent(100.).justify(Justify::SpaceBetween) + |ui| {
                text(ui, "ATTACHMENTS").font_size(13).color(MUTED);
                text(ui, "2 files").font_size(12).color(MUTED);
            };
            for (name, size) in [
                ("recovery-codes.txt", "912 bytes"),
                ("backup-2fa.pdf", "48 KB"),
            ] {
                ui.keyed(name, |ui| {
                    row(ui).width_percent(100.).align(Align::Center).gap(12) + |ui| {
                        glyph(ui, "note", 24., Color::Literal(0x898994));
                        column(ui).grow(1.).gap(3) + |ui| {
                            text(ui, name).font_size(14);
                            text(ui, size).font_size(12).color(MUTED);
                        };
                        if tool(ui, "eye", &format!("Preview {name}"), false) {
                            self.preview_notice();
                        }
                    };
                });
            }
        };
        text(ui, "Sample data · no connection to Bitwarden or Vaultwarden").font_size(12).color(MUTED).padding_x(8);
    }
    fn preview_notice(&mut self) {
        self.notice = "Visual preview only · these are sample values and attachments.".into();
    }
    #[component]
    #[rustfmt::skip]
    fn live_detail(&mut self, ui: &mut UiContext, item: &Item) {
        let details = self.details.take();
        let editable = details.as_ref().is_some_and(|d| d.editable);
        row(ui).position(plico::Position::Relative).width_percent(100.).padding(12).gap(8).wrap(true)
            .align(Align::Center).radius(18.).corner_smoothing(1.).material(self.appearance.material()) + |ui|
        {
            if item.deleted {
                text(ui, "In Trash").color(MUTED).grow(1.);
                if button(ui).text("Restore item").primary().disabled(self.busy).clicked() {
                    self.action(&item.id, "restore");
                }
                if button(ui).text("Delete permanently").ghost().background_alpha(0.).disabled(self.busy).clicked() {
                    self.ask(&item.id, "purge");
                }
            } else {
                if button(ui).text("Edit item").primary().disabled(self.busy || !editable).clicked() {
                    self.request(Request::Edit(item.id.clone()));
                }
                if button(ui).text(if item.favorite { "Unfavorite" } else { "Favorite" }).ghost()
                    .background_alpha(0.).disabled(self.busy || !editable).clicked()
                {
                    self.action(&item.id, if item.favorite { "unfavorite" } else { "favorite" });
                }
                if button(ui).text("Duplicate item").ghost().background_alpha(0.).disabled(self.busy || !editable)
                    .clicked()
                {
                    self.action(&item.id, "clone");
                }
                if button(ui).text(if item.archived { "Unarchive" } else { "Archive item" }).ghost()
                    .background_alpha(0.).disabled(self.busy || !editable).clicked()
                {
                    self.action(&item.id, if item.archived { "unarchive" } else { "archive" });
                }
                column(ui).grow(1.).end();
                if button(ui).text("Move to trash").ghost().background_alpha(0.).disabled(self.busy || !editable)
                    .clicked()
                {
                    self.ask(&item.id, "trash");
                }
            }
        };
        surface(ui).material(self.appearance.material()).padding(26) + |ui| {
            row(ui).width_percent(100.).align(Align::Center).gap(18) + |ui| {
                brand(ui, item, 62.);
                column(ui).width(0.).grow(1.).min_width(0.).gap(6) + |ui| {
                    text(ui, &item.name).font_size(30).weight(750).width_percent(100.);
                    text(
                        ui,
                        details.as_ref().and_then(|d| d.uris.first())
                            .map_or(item.kind.as_str(), String::as_str),
                    ).color(MUTED).width_percent(100.);
                };
            };
        };
        if self.busy {
            text(ui, "Reading vault…").color(MUTED);
        }
        if let Some(detail) = &details {
            if let Some(username) = detail.username.as_deref().filter(|s| !s.is_empty()) {
                surface(ui).padding(20).gap(10) + |ui| {
                    text(ui, "Username").font_size(13).color(MUTED);
                    text(ui, username).font_size(17).width_percent(100.);
                    if button(ui).text("Copy username").ghost().background_alpha(0.).disabled(self.busy).clicked() {
                        self.request(Request::CopyField {
                            id: item.id.clone(),
                            field: "username".into(),
                        });
                    }
                };
            }
            if detail.has_password || detail.has_totp {
                row(ui).width_percent(100.).gap(12).wrap(true).align(Align::Stretch).shrink(0.) + |ui| {
                    if detail.has_password {
                        surface(ui).width(0.).grow(1.).min_width(230.).padding(20).gap(12) + |ui| {
                            row(ui).width_percent(100.).justify(Justify::SpaceBetween).align(Align::Center) + |ui| {
                                text(ui, "Password").font_size(13).color(MUTED);
                                if button(ui)
                                    .text(if self.reveal { "Hide password" } else { "Show password" })
                                    .ghost().background_alpha(0.).disabled(self.busy).clicked()
                                {
                                    if self.reveal {
                                        self.reveal = false;
                                        self.shown_password = None;
                                    } else {
                                        self.request(Request::Reveal(item.id.clone()));
                                    }
                                }
                            };
                            text(
                                ui,
                                if self.reveal {
                                    self.shown_password.as_deref().map_or("", String::as_str)
                                } else {
                                    "••••••••••••"
                                },
                            ).font_size(20).width_percent(100.);
                            if button(ui).text("Copy password").ghost().background_alpha(0.).disabled(self.busy)
                                .clicked()
                            {
                                self.request(Request::CopyField {
                                    id: item.id.clone(),
                                    field: "password".into(),
                                });
                            }
                        };
                    }
                    if detail.has_totp {
                        surface(ui).width(0.).grow(1.).min_width(230.).padding(20).gap(12) + |ui| {
                            text(ui, "One-time code").font_size(13).color(MUTED);
                            text(ui, self.code.as_ref().map_or("— — — — — —", |c| c.code.as_str())).font_size(25).weight(650);
                            if button(ui).text("Generate current code").ghost().background_alpha(0.)
                                .disabled(self.busy).clicked()
                            {
                                self.request(Request::Totp(item.id.clone()));
                            }
                        };
                    }
                };
            }
            surface(ui).padding(20).gap(16) + |ui| {
                for (index, uri) in detail.uris.iter().enumerate() {
                    ui.keyed(index as u64, |ui| {
                        text(ui, "Website").font_size(13).color(MUTED);
                        text(ui, uri).width_percent(100.);
                    });
                }
                meta(ui, "person", "Vault", &self.account_email);
                for (label, value) in [
                    ("Cardholder", detail.cardholder_name.as_deref()),
                    ("Card ending", detail.card_number.as_deref()),
                    ("First name", detail.first_name.as_deref()),
                    ("Last name", detail.last_name.as_deref()),
                    ("Email", detail.email.as_deref()),
                    ("Phone", detail.phone.as_deref()),
                ] {
                    if let Some(value) = value {
                        ui.keyed(label, |ui| meta(ui, "person", label, value));
                    }
                }
                if !detail.password_history.is_empty() {
                    text!(
                        ui,
                        "{} previous passwords retained",
                        detail.password_history.len()
                    ).color(MUTED).width_percent(100.);
                    if button(ui)
                        .text(if self.reveal_history { "Hide password history" } else { "Show password history" })
                        .ghost().disabled(self.busy).clicked()
                    {
                        if self.reveal_history {
                            self.reveal_history = false;
                            self.history_ticket = None;
                            ui.request_rebuild();
                            self.previous_passwords.clear();
                        } else {
                            self.request(Request::PasswordHistory(item.id.clone()));
                            self.history_ticket = self.pending;
                        }
                    }
                    if self.reveal_history {
                        for index in 0..self.previous_passwords.len() {
                            ui.keyed(index as u64, |ui| {
                                text!(
                                    ui,
                                    "Previous password {} · last used (Unix seconds): {}",
                                    index + 1,
                                    self.previous_passwords[index].0
                                ).font_size(12).color(MUTED).width_percent(100.);
                                text(ui, self.previous_passwords[index].1.as_str()).width_percent(100.);
                                {
                                    let value = self.previous_passwords[index].1.clone();
                                    if button(ui).text("Copy previous password").ghost().disabled(self.busy)
                                        .clicked()
                                    {
                                        self.copy_text(value);
                                    }
                                }
                            });
                        }
                    }
                }
                if let Some(notes) = detail.notes.as_deref().filter(|s| !s.is_empty()) {
                    text(ui, "Notes").font_size(13).color(MUTED);
                    text(ui, notes).width_percent(100.);
                }
            };
        }
        self.details = details;
        text(
            ui,
            "Locks after five minutes of inactivity · encrypted data is not cached locally yet",
        ).font_size(12).color(MUTED).width_percent(100.);
    }
    pub fn action(&mut self, id: &str, action: &str) {
        self.request(Request::Action {
            id: id.into(),
            action: action.into(),
        });
    }
    pub fn ask(&mut self, id: &str, action: &str) {
        self.confirm_action = Some((id.into(), action.into()));
        self.confirm_modal.open();
    }
}
