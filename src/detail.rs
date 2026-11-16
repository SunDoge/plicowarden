use crate::{
    model::{Item, Warden},
    ui::*,
};
use plico::prelude::*;
use plicowarden_sdk::Request;

fn kind_label(kind: &str) -> &str {
    match kind {
        "ssh-key" => "SSH key",
        "note" => "Secure note",
        "identity" => "Identity",
        "card" => "Card",
        "login" => "Login",
        other => other,
    }
}
fn mask_card_number(number: &str, brand: Option<&str>) -> String {
    let suffix = number
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>();
    let inferred_len = match brand {
        Some("Amex" | "American Express") => 15,
        Some("Diners Club") => 14,
        _ => 16,
    };
    let len = if suffix.len() <= 4 {
        inferred_len
    } else {
        suffix.len()
    };
    let visible = suffix.chars().rev().take(4).collect::<Vec<_>>();
    let mut masked = vec!['•'; len];
    for (index, digit) in visible.into_iter().enumerate() {
        masked[len - 1 - index] = digit;
    }
    let blocks: &[usize] = match len {
        13 => &[4, 4, 5],
        14 => &[4, 6, 4],
        15 => &[4, 6, 5],
        19 => &[4, 4, 4, 4, 3],
        _ => &[4, 4, 4, 4],
    };
    let mut offset = 0;
    blocks
        .iter()
        .filter_map(|size| {
            let end = (offset + size).min(masked.len());
            let chunk = (offset < end).then(|| masked[offset..end].iter().collect::<String>());
            offset = end;
            chunk
        })
        .collect::<Vec<_>>()
        .join(" ")
}
#[component]
#[rustfmt::skip]
fn meta(ui: &mut UiContext, icon: &str, label: &str, value: &str) {
    row(ui).width_percent(100.).align(Align::Center).gap(10).padding_y(4) + |ui| {
        glyph(ui, icon, 18., Color::Literal(0x777781));
        text(ui, label).grow(1.);
        text(ui, value).font_size(13).color(MUTED);
    };
}
/// A continuous OTP ring that drains over the code's period, with the whole
/// remaining seconds in the middle. Plico paints the arc as one analytic WGSL
/// surface, so fractional DPI does not turn it into a jagged dotted wheel.
#[component]
#[rustfmt::skip]
fn totp_countdown(ui: &mut UiContext, remaining: u32, fraction: f32) {
    let size = 40.;
    let urgent = remaining <= 5;
    let color = if urgent { Color::Warning } else { Color::Literal(0x4f7fae) };
    column(ui).width(size).height(size).shrink(0.)
        .position(plico::Position::Relative) + |ui|
    {
        plico::widgets::progress_ring_sized(
            ui, fraction, size, 2.5, color, Color::Literal(0xe1dee8),
        );
        column(ui).absolute(0., 0.).width(size).height(size)
            .align(Align::Center).justify(Justify::Center) + |ui| {
            text!(ui, "{remaining}").font_size(12).weight(650)
                .color(if urgent { Color::Warning } else { MUTED });
        };
    };
}

fn request_countdown_update(ui: &mut UiContext, remaining: f32) {
    if ui.display_preferences().reduce_motion {
        let fractional = remaining.fract();
        let wait = if fractional > 0.001 { fractional } else { 1. };
        ui.request_rebuild_at(
            ui.now()
                .saturating_add(std::time::Duration::from_secs_f32(wait)),
        );
    } else {
        ui.request_animation_frame();
    }
}

fn demo_totp_remaining(ui: &mut UiContext, period: u32) -> (u32, f32) {
    let period = period.max(1);
    let now = ui.now();
    let elapsed = now.as_secs_f32() % period as f32;
    let remaining = (period as f32 - elapsed).max(0.);
    request_countdown_update(ui, remaining);
    (remaining.ceil() as u32, remaining / period as f32)
}

fn live_totp_remaining(
    ui: &mut UiContext,
    code: &plicowarden_sdk::service::TotpCode,
    received_at: std::time::Duration,
) -> (u32, f32) {
    let elapsed = ui.now().saturating_sub(received_at).as_secs_f32();
    let remaining = (code.valid_for as f32 - elapsed).max(0.);
    if remaining > 0. {
        request_countdown_update(ui, remaining);
    }
    (
        remaining.ceil() as u32,
        remaining / code.period.max(1) as f32,
    )
}
impl Warden {
    #[component]
    #[rustfmt::skip]
    pub fn detail(&mut self, ui: &mut UiContext, item: &Item, compact: bool) {
        if self.authenticated {
            self.live_detail(ui, item);
            return;
        }
        surface(ui).padding_x(20).padding_y(18).gap(8) + |ui| {
            row(ui).width_percent(100.).align(Align::Center).gap(18) + |ui| {
                brand(ui, item, 52.);
                column(ui).width(0.).grow(1.).min_width(0.).gap(5) + |ui| {
                    text(ui, &item.name).font_size(28).weight(750).width_percent(100.);
                    text(ui, if item.site.is_empty() { "Personal vault" } else { &item.site }).font_size(14).color(MUTED);
                };
                glyph(ui, if item.favorite { "star" } else { "shield" }, 22.,
                    if item.favorite { Color::Literal(0xc39831) } else { MUTED });
            };
        };
        surface(ui).padding_x(16).padding_y(12).gap(6) + |ui| {
            text(ui, "Username").font_size(12).color(MUTED);
            row(ui).width_percent(100.).align(Align::Center).justify(Justify::SpaceBetween) + |ui| {
                text(ui, &item.user).font_size(14);
                if tool(ui, "copy", "Copy username", false) {
                    self.preview_notice();
                }
            };
        };
        row(ui).width_percent(100.).gap(12).align(Align::Stretch).wrap(compact).shrink(0.) + |ui| {
            surface(ui).width(0.).grow(1.7).min_width(if compact { 220. } else { 0. })
                .padding_x(16).padding_y(14).gap(8) + |ui| {
                row(ui).width_percent(100.).align(Align::Center).justify(Justify::SpaceBetween) + |ui| {
                    text(ui, "Password").font_size(12).color(MUTED);
                    if tool(ui, if self.reveal { "eye-off" } else { "eye" },
                        if self.reveal { "Hide password" } else { "Show password" }, false) {
                        self.reveal = !self.reveal;
                    }
                };
                text(ui, if self.reveal { "sample-only-7X!" } else { "••••••••••••••••" }).font_size(16).weight(600);
                column(ui).width_percent(100.).height(4.).radius(2.).background(Color::Literal(0x64b778)).end();
                text(ui, "Strong · unique").font_size(12).color(Color::Literal(0x39784c));
            };
            surface(ui).width(0.).grow(1.).min_width(if compact { 220. } else { 0. })
                .padding_x(16).padding_y(14).gap(8) + |ui| {
                text(ui, "One-time code").font_size(12).color(MUTED);
                row(ui).width_percent(100.).align(Align::Center).gap(12) + |ui| {
                    text(ui, "522 · 719").font_size(22).weight(650).grow(1.);
                    let (remaining, fraction) = demo_totp_remaining(ui, 30);
                    totp_countdown(ui, remaining, fraction);
                };
                text(ui, "Demonstration code").font_size(12).color(MUTED);
            };
        };
        surface(ui).padding_x(16).padding_y(12).gap(10) + |ui| {
            text(ui, "Website").font_size(12).color(MUTED);
            row(ui).width_percent(100.).align(Align::Center).gap(10) + |ui| {
                glyph(ui, "external", 18., ICON);
                let site = if item.site.is_empty() {
                    "No website".into()
                } else {
                    format!("https://{}", item.site)
                };
                text(ui, &site).width_percent(100.).grow(1.).font_size(14);
            };
            meta(ui, "person", "Vault", "My vault");
            meta(ui, "key", "Passkey", if item.has_passkey { "1 passkey" } else { "Not configured" });
            meta(ui, "shield", "Watchtower", "No issues found");
            column(ui).width_percent(100.).height(1.).background(Color::Literal(0xe4e3e9)).background_alpha(0.7)
                .end();
            text(ui, "NOTES").font_size(11).color(MUTED);
            text(ui, "Recovery codes are in the “Account recovery” note.").width_percent(100.).font_size(13);
        };
        surface(ui).padding_x(16).padding_y(12).gap(10) + |ui| {
            row(ui).width_percent(100.).justify(Justify::SpaceBetween) + |ui| {
                text(ui, "ATTACHMENTS").font_size(11).color(MUTED);
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
                            text(ui, name).font_size(13);
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
    /// Item actions live in the window header, as in Triwarden, instead of
    /// taking a row above the detail cards.
    #[component]
    #[rustfmt::skip]
    pub fn item_actions(&mut self, ui: &mut UiContext, item: &Item, editable: bool) {
        if item.deleted {
            text(ui, "In Trash").color(MUTED).max_width(90.).ellipsis(true);
            if button(ui).icon(crate::icons::get("restore")).text("Restore item").primary()
                .disabled(self.busy).clicked()
            {
                self.action(&item.id, "restore");
            }
            if toolbar_tool(ui, "trash", "Delete permanently", self.busy) {
                self.ask(&item.id, "purge");
            }
            return;
        }
        if toolbar_tool(ui, "edit", "Edit item", self.busy || !editable) {
            self.request(Request::Edit(item.id.clone()));
        }
        if toolbar_tool(ui, "star", if item.favorite { "Unfavorite" } else { "Favorite" },
            self.busy || !editable) {
            self.action(&item.id, if item.favorite { "unfavorite" } else { "favorite" });
        }
        if toolbar_tool(ui, "copy", "Duplicate item", self.busy || !editable) {
            self.action(&item.id, "clone");
        }
        if toolbar_tool(ui, "archive", if item.archived { "Unarchive" } else { "Archive item" },
            self.busy || !editable) {
            self.action(&item.id, if item.archived { "unarchive" } else { "archive" });
        }
        if toolbar_tool(ui, "folder", "Move to folder", self.busy || !editable) {
            self.open_move_dialog(vec![item.id.clone()]);
        }
        if toolbar_tool(ui, "trash", "Move to trash", self.busy || !editable) {
            self.ask(&item.id, "trash");
        }
    }
    #[component]
    #[rustfmt::skip]
    fn live_detail(&mut self, ui: &mut UiContext, item: &Item) {
        let details = self.details.take();
        surface(ui).padding_x(20).padding_y(18) + |ui| {
            row(ui).width_percent(100.).align(Align::Center).gap(18) + |ui| {
                brand(ui, item, 52.);
                column(ui).width(0.).grow(1.).min_width(0.).gap(6) + |ui| {
                    text(ui, &item.name).font_size(28).weight(750).width_percent(100.);
                    text(
                        ui,
                        details.as_ref().and_then(|d| d.uris.first())
                            .map_or(kind_label(&item.kind), String::as_str),
                    ).color(MUTED).width_percent(100.);
                };
                let loading_visible = self.detail_ticket.is_some()
                    && self.selected != item.id
                    && self.detail_loading_started_at.is_some_and(|started| {
                        let reveal_at = started.saturating_add(std::time::Duration::from_millis(180));
                        if ui.now() < reveal_at {
                            ui.request_rebuild_at(reveal_at);
                            false
                        } else {
                            true
                        }
                    });
                if loading_visible {
                    let progress = ui.loop_progress(
                        "detail-switch-progress",
                        std::time::Duration::from_millis(900),
                        plico::Easing::Linear,
                    );
                    row(ui).shrink(0.).align(Align::Center).gap(8) + |ui| {
                        plico::widgets::progress_ring_sized(
                            ui,
                            progress.max(0.08),
                            20.,
                            2.,
                            Color::Accent,
                            Color::Literal(0xe1dee8),
                        );
                        text(ui, "Opening…").font_size(12).color(MUTED);
                    };
                }
            };
        };
        // While details round-trip, keep the pane's shape: same-sized
        // placeholder cards pulse in place of the real fields, so switching items never collapses and
        // re-expands the layout. Screen readers get one Loading status.
        if details.is_none() && self.detail_ticket.is_some() {
            let pulse = ui.loop_progress("detail-pulse", std::time::Duration::from_millis(1200),
                plico::Easing::EaseInOut);
            let shade = 0.55 + 0.45 * pulse;
            let bone = |ui: &mut UiContext, label: &str| {
                let _ = label;
                row(ui).width_percent(100.).align(Align::Center).gap(10) + |ui| {
                    column(ui).width(0.).grow(1.).gap(8) + |ui| {
                        column(ui).height(15.).width_percent(72.).radius(4.)
                            .background(Color::Literal(0xd4d4d8)).background_alpha(shade);
                        column(ui).height(13.).width_percent(45.).radius(4.)
                            .background(Color::Literal(0xe4e4e7)).background_alpha(shade);
                    };
                };
            };
            surface(ui).padding_x(16).padding_y(12).gap(8) + |ui| {
                text(ui, "Loading details").font_size(13).color(MUTED);
                bone(ui, "username");
            };
            surface(ui).padding_x(16).padding_y(12).gap(8) + |ui| {
                text(ui, "Loading fields").font_size(13).color(MUTED);
                bone(ui, "password");
                bone(ui, "totp");
            };
        }
        if let Some(detail) = &details {
            if let Some(username) = detail.username.as_deref().filter(|s| !s.is_empty()) {
                surface(ui).padding_x(16).padding_y(12).gap(6) + |ui| {
                    text(ui, "Username").font_size(12).color(MUTED);
                    text(ui, username).font_size(14).width_percent(100.);
                    if button(ui).icon(crate::icons::get("copy")).text("Copy username").ghost().disabled(self.busy)
                        .clicked()
                    {
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
                        surface(ui).width(0.).grow(1.).min_width(230.).padding_x(16).padding_y(14).gap(8) + |ui| {
                            row(ui).width_percent(100.).justify(Justify::SpaceBetween).align(Align::Center) + |ui| {
                                text(ui, "Password").font_size(12).color(MUTED);
                                if button(ui)
                                    .icon(crate::icons::get(if self.reveal { "eye-off" } else { "eye" }))
                                    .text(if self.reveal { "Hide password" } else { "Show password" })
                                    .ghost().disabled(self.busy).clicked()
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
                            ).font_size(16).width_percent(100.);
                            if button(ui).icon(crate::icons::get("copy")).text("Copy password").ghost()
                                .disabled(self.busy).clicked()
                            {
                                self.request(Request::CopyField {
                                    id: item.id.clone(),
                                    field: "password".into(),
                                });
                            }
                        };
                    }
                    if detail.has_totp {
                        surface(ui).width(0.).grow(1.).min_width(230.).padding_x(16).padding_y(14).gap(8) + |ui| {
                            text(ui, "One-time code").font_size(12).color(MUTED);
                            row(ui).width_percent(100.).align(Align::Center).gap(12) + |ui| {
                                text(ui, self.code.as_ref().map_or("— — — — — —", |c| c.code.as_str()))
                                    .font_size(22).weight(650).grow(1.);
                                if let (Some(code), Some(received_at)) = (&self.code, self.code_received_at) {
                                    let (remaining, fraction) = live_totp_remaining(ui, code, received_at);
                                    totp_countdown(ui, remaining, fraction);
                                }
                            };
                            row(ui).width_percent(100.).gap(8.).wrap(true) + |ui| {
                                if button(ui).icon(crate::icons::get("clock")).text("Generate current code").ghost()
                                    .disabled(self.busy).clicked()
                                {
                                    self.request(Request::Totp(item.id.clone()));
                                }
                                let current = self.code.as_ref().map(|code| code.code.clone());
                                if button(ui).icon(crate::icons::get("copy")).text("Copy code").ghost()
                                    .disabled(self.busy || current.is_none()).clicked()
                                    && let Some(code) = current
                                {
                                    self.copy_text(zeroize::Zeroizing::new(code));
                                }
                            };
                        };
                    }
                };
            }
            if !detail.passkeys.is_empty() {
                surface(ui).padding_x(16).padding_y(14).gap(10) + |ui| {
                    text!(
                        ui,
                        "Passkeys · {}",
                        detail.passkeys.len()
                    ).font_size(12).color(MUTED).width_percent(100.);
                    for (index, passkey) in detail.passkeys.iter().enumerate() {
                        ui.keyed(index as u64, |ui| {
                            row(ui).width_percent(100.).align(Align::Center).gap(10) + |ui| {
                                glyph(ui, "key", 20., ICON);
                                column(ui).width(0.).grow(1.).min_width(0.).gap(3) + |ui| {
                                    text(
                                        ui,
                                        passkey.rp_name.as_deref().unwrap_or(&passkey.rp_id),
                                    ).font_size(14).weight(650).width_percent(100.).ellipsis(true);
                                    let account = passkey.user_display_name.as_deref()
                                        .or(passkey.user_name.as_deref())
                                        .unwrap_or("Account name unavailable");
                                    text(ui, account).font_size(12).color(MUTED)
                                        .width_percent(100.).ellipsis(true);
                                };
                                text(
                                    ui,
                                    relative_time(unix_seconds(), passkey.created_at),
                                ).font_size(12).color(MUTED);
                            };
                        });
                    }
                };
            }
            surface(ui).padding_x(16).padding_y(12).gap(10) + |ui| {
                for (index, uri) in detail.uris.iter().enumerate() {
                    ui.keyed(index as u64, |ui| {
                        text(ui, "Website").font_size(12).color(MUTED);
                        row(ui).width_percent(100.).align(Align::Center).gap(8.) + |ui| {
                            text(ui, uri).width(0.).grow(1.).min_width(0.).ellipsis(true);
                            if tool(ui, "copy", "Copy website", self.busy) {
                                self.copy_text(zeroize::Zeroizing::new(uri.clone()));
                            }
                        };
                    });
                }
                meta(ui, "person", "Vault", &self.account_email);
                if let Some(folder) = detail.folder_id.as_deref()
                    .and_then(|id| self.folders.iter().find(|folder| folder.id == id))
                {
                    let path = folder.name.split('/')
                        .map(str::trim)
                        .filter(|part| !part.is_empty())
                        .collect::<Vec<_>>()
                        .join(" › ");
                    meta(ui, "folder", "Folder", &path);
                }
                for (label, value) in [
                    ("Cardholder", detail.cardholder_name.as_deref()),
                    ("Card brand", detail.card_brand.as_deref()),
                    ("Title", detail.identity_title.as_deref()),
                    ("First name", detail.first_name.as_deref()),
                    ("Middle name", detail.middle_name.as_deref()),
                    ("Last name", detail.last_name.as_deref()),
                    ("Email", detail.email.as_deref()),
                    ("Phone", detail.phone.as_deref()),
                ] {
                    if let Some(value) = value {
                        ui.keyed(label, |ui| meta(ui, "person", label, value));
                    }
                }
                if let Some(number) = detail.card_number.as_deref() {
                    let masked = mask_card_number(number, detail.card_brand.as_deref());
                    let full_number_is_local = number.chars().filter(char::is_ascii_digit).count() > 4;
                    row(ui).width_percent(100.).align(Align::Center).gap(8.) + |ui| {
                        column(ui).width(0.).grow(1.).gap(2.) + |ui| {
                            text(ui, "Card number").font_size(12).color(MUTED);
                            text(ui, if self.reveal_card_number {
                                self.shown_card_number.as_deref().map_or(masked.as_str(), |number| number.as_str())
                            } else { masked.as_str() }).
                                font_size(15);
                        };
                        if tool(ui, "copy", "Copy card number", self.busy) {
                            if full_number_is_local {
                                self.copy_text(zeroize::Zeroizing::new(number.to_owned()));
                            } else {
                                self.request(Request::CopyField {
                                    id: item.id.clone(),
                                    field: "cardNumber".into(),
                                });
                            }
                        }
                        let (icon, label) = if self.reveal_card_number {
                            ("eye-off", "Hide card number")
                        } else {
                            ("eye", "Show card number")
                        };
                        if tool(ui, icon, label, self.busy) {
                            if self.reveal_card_number {
                                self.reveal_card_number = false;
                                self.shown_card_number = None;
                                ui.request_rebuild();
                            } else if full_number_is_local {
                                self.shown_card_number = Some(zeroize::Zeroizing::new(number.to_owned()));
                                self.reveal_card_number = true;
                                ui.request_rebuild();
                            } else {
                                self.request(Request::RevealCardNumber(item.id.clone()));
                            }
                        }
                    };
                }
                if let (Some(month), Some(year)) =
                    (detail.card_exp_month.as_deref(), detail.card_exp_year.as_deref())
                {
                    meta(ui, "clock", "Expires", &format!("{month}/{year}"));
                }
                if detail.has_card_code {
                    meta(ui, "lock", "Security code", "Available");
                }
                if detail.kind == "ssh-key" {
                    if let Some(public_key) = detail.ssh_public_key.as_deref() {
                        row(ui).width_percent(100.).align(Align::Center).gap(8.) + |ui| {
                            text(ui, "Public key").font_size(12).color(MUTED).grow(1.);
                            if tool(ui, "copy", "Copy public key", self.busy) {
                                self.copy_text(zeroize::Zeroizing::new(public_key.to_owned()));
                            }
                        };
                        text(ui, public_key).font_size(12).width_percent(100.);
                    }
                    if let Some(fingerprint) = detail.ssh_fingerprint.as_deref() {
                        row(ui).width_percent(100.).align(Align::Center).gap(10).padding_y(4) + |ui| {
                            glyph(ui, "key", 18., Color::Literal(0x777781));
                            text(ui, "Fingerprint").grow(1.);
                            text(ui, fingerprint).font_size(13).color(MUTED);
                            if tool(ui, "copy", "Copy fingerprint", self.busy) {
                                self.copy_text(zeroize::Zeroizing::new(fingerprint.to_owned()));
                            }
                        };
                    }
                    if detail.has_private_key {
                        meta(ui, "lock", "Private key", "Available");
                    }
                }
                if detail.kind == "identity" {
                    let address = [detail.address1.as_deref(), detail.address2.as_deref(),
                        detail.address3.as_deref()].into_iter().flatten()
                        .filter(|line| !line.trim().is_empty()).collect::<Vec<_>>().join(", ");
                    let fields = [
                        ("Company", detail.company.as_deref()),
                        ("Address", Some(address.as_str()).filter(|value| !value.is_empty())),
                        ("City", detail.city.as_deref()),
                        ("State", detail.state.as_deref()),
                        ("Postal code", detail.postal_code.as_deref()),
                        ("Country", detail.country.as_deref()),
                    ];
                    for (label, value) in fields {
                        if let Some(value) = value {
                            ui.keyed(label, |ui| meta(ui, "person", label, value));
                        }
                    }
                    for (label, present) in [
                        ("SSN", detail.has_ssn),
                        ("Passport number", detail.has_passport_number),
                        ("License number", detail.has_license_number),
                    ] {
                        if present {
                            ui.keyed(label, |ui| meta(ui, "lock", label, "Available"));
                        }
                    }
                }
                if !detail.password_history.is_empty() {
                    text!(
                        ui,
                        "{} previous passwords retained",
                        detail.password_history.len()
                    ).color(MUTED).width_percent(100.);
                    if button(ui).icon(crate::icons::get("clock"))
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
                                    "Previous password {} · last used {}",
                                    index + 1,
                                    relative_time(unix_seconds(), self.previous_passwords[index].0)
                                ).font_size(12).color(MUTED).width_percent(100.);
                                text(ui, self.previous_passwords[index].1.as_str()).width_percent(100.);
                                {
                                    let value = self.previous_passwords[index].1.clone();
                                    if button(ui).icon(crate::icons::get("copy")).text("Copy previous password")
                                        .ghost().disabled(self.busy).clicked()
                                    {
                                        self.copy_text(value);
                                    }
                                }
                            });
                        }
                    }
                }
                if let Some(notes) = detail.notes.as_deref().filter(|s| !s.is_empty()) {
                    row(ui).width_percent(100.).align(Align::Center).gap(8.) + |ui| {
                        text(ui, "Notes").font_size(11).color(MUTED).grow(1.);
                        if tool(ui, "copy", "Copy notes", self.busy) {
                            self.copy_text(zeroize::Zeroizing::new(notes.to_owned()));
                        }
                    };
                    text(ui, notes).width_percent(100.);
                }
            };
        }
        if let Some(detail) = details.as_ref().filter(|detail| !detail.fields.is_empty()) {
            surface(ui).padding_x(16).padding_y(14).gap(10) + |ui| {
                text(ui, "Custom fields").font_size(11).color(MUTED).width_percent(100.);
                for (index, field) in detail.fields.iter().cloned().enumerate() {
                    ui.keyed(index as u64, |ui| {
                        row(ui).width_percent(100.).align(Align::Center).gap(8.) + |ui| {
                            column(ui).width(0.).grow(1.).min_width(0.).gap(3.) + |ui| {
                                text(ui, field.name.as_deref().unwrap_or("Custom field"))
                                    .font_size(12).color(MUTED).width_percent(100.).ellipsis(true);
                                let value = if field.hidden {
                                    "••••••••••••".to_owned()
                                } else if field.boolean {
                                    field.value.as_deref()
                                        .map(|value| if value == "true" { "Yes" } else { "No" })
                                        .unwrap_or("No").to_owned()
                                } else {
                                    field.value.clone().unwrap_or_default()
                                };
                                text(ui, &value).font_size(14).width_percent(100.).ellipsis(true);
                            };
                            if !field.hidden && !field.boolean
                                && let Some(value) = field.value.as_deref()
                                && tool(ui, "copy", &format!("Copy {}", field.name.as_deref()
                                    .unwrap_or("custom field")), self.busy)
                            {
                                self.copy_text(zeroize::Zeroizing::new(value.to_owned()));
                            }
                        };
                    });
                }
            };
        }
        self.details = details;
        text!(
            ui,
            "Auto-lock: {} minutes · no offline vault stored on this device",
            self.active_security.auto_lock_minutes
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
