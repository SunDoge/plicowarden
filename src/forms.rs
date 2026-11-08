use crate::{
    model::{Editor, Warden, factor_id, multiline_field, secret_field},
    ui::*,
};
use plico::prelude::*;
use plicowarden_sdk::{Request, service::TwoFactorSubmitRequest};
use zeroize::Zeroizing;

#[component]
pub fn picker(
    ui: &mut UiContext,
    label: &str,
    value: &mut String,
    popup: &mut PopoverState,
    entries: &[(String, String)],
) {
    let options: Vec<_> = entries
        .iter()
        .map(|(value, label)| {
            SelectOption::new(
                crate::model::business_key(value),
                value.clone(),
                label.as_str(),
            )
        })
        .collect();
    select(ui, popup, label, value, &options).width(260.);
}
#[component]
#[rustfmt::skip]
fn region_selector(ui: &mut UiContext, selected: &mut usize, disabled: bool) {
    row(ui).gap(2).padding(3).radius(6.).background(Color::Literal(0xf8f8fa)) + |ui| {
        for (index, label) in ["Self-hosted", "Bitwarden US", "Bitwarden EU"].iter().enumerate()
        {
            ui.keyed(index as u64, |ui| {
                if button(ui).text(*label).height(30.).padding_x(8).radius(5.).font_size(15)
                    .background(if *selected == index {
                    Color::Literal(0xffffff)
                } else {
                    Color::Literal(0xf8f8fa)
                }).border(if *selected == index { 1. } else { 0. }, Color::Border).disabled(disabled).clicked()
                {
                    *selected = index;
                }
            });
        }
    };
}
fn entries(values: &[(&str, &str)]) -> Vec<(String, String)> {
    values
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}
#[component]
#[rustfmt::skip]
fn edit_field(ui: &mut UiContext, draft: &mut Editor, key: &str, label: &str) {
    let secret = secret_field(key);
    let multiline = multiline_field(key);
    let buffer = draft.buffer(key, secret, multiline);
    column(ui).width_percent(100.).gap(7) + |ui| {
        text(ui, label).font_size(13).weight(600).color(MUTED);
        text_input(ui, label, &buffer).width_percent(100.).height(if multiline {
            if key == "notes" || key == "privateKey" { 120. } else { 80. }
        } else {
            36.
        });
    };
}
pub fn kind_fields(kind: &str) -> &'static [(&'static str, &'static str)] {
    match kind {
        "login" => &[
            ("username", "Username"),
            ("password", "Password"),
            ("uris", "Websites (one per line)"),
            ("totp", "Authenticator key"),
        ],
        "card" => &[
            ("cardholderName", "Cardholder"),
            ("brand", "Card brand"),
            ("number", "Card number"),
            ("expMonth", "Expiry month"),
            ("expYear", "Expiry year"),
            ("code", "Security code"),
        ],
        "identity" => &[
            ("title", "Title"),
            ("firstName", "First name"),
            ("middleName", "Middle name"),
            ("lastName", "Last name"),
            ("username", "Username"),
            ("email", "Email"),
            ("phone", "Phone"),
            ("company", "Company"),
            ("address1", "Address"),
            ("address2", "Address line 2"),
            ("address3", "Address line 3"),
            ("city", "City"),
            ("state", "State / province"),
            ("postalCode", "Postal code"),
            ("country", "Country"),
            ("ssn", "Social security number"),
            ("passportNumber", "Passport number"),
            ("licenseNumber", "License number"),
        ],
        "ssh-key" => &[
            ("privateKey", "OpenSSH private key"),
            ("publicKey", "Public key"),
            ("fingerprint", "Fingerprint"),
        ],
        _ => &[],
    }
}
impl Warden {
    #[component]
    #[rustfmt::skip]
    pub fn account(&mut self, ui: &mut UiContext) {
        let width = (ui.viewport_width() - 48.).clamp(240., 480.);
        let centered = !self.advanced && ui.viewport_size()[1] >= 800.;
        column(ui).width_percent(100.).height_percent(100.).align(Align::Center)
            .justify(if centered { Justify::Center } else { Justify::Start }).padding(24).scroll_y(true) + |ui|
        {
            surface(ui).width(width).padding(30).gap(16).shrink(0.) + |ui| {
                glyph(ui, "shield", 32., Color::Literal(0x69618e));
                text(ui, if self.factors.is_empty() { "Open your vault" } else { "Verify your identity" }).font_size(28).weight(700);
                text(ui, "Bitwarden Password Manager SDK · Rust backend").font_size(13).color(MUTED)
                    .width_percent(100.);
                if self.factors.is_empty() {
                    let mut region = match self.region.as_str() {
                        "us" => 1,
                        "eu" => 2,
                        _ => 0,
                    };
                    region_selector(ui, &mut region, self.busy);
                    self.region = ["self-hosted", "us", "eu"][region.min(2)].into();
                    if self.region == "self-hosted" {
                        text(ui, "Server URL").font_size(13).color(MUTED);
                        text_input_string(ui, "Server URL", &mut self.server)
                            .placeholder("https://vault.example.com").width_percent(100.).disabled(self.busy);
                        checkbox(ui, "Custom service endpoints", &mut self.advanced);
                        if self.advanced {
                            text(ui,"Overrides are optional. API and identity may use separate hosts.")
                                .font_size(12).color(MUTED).width_percent(100.);
                            text_input_string(ui, "API URL", &mut self.api_url).placeholder("API URL override")
                                .width_percent(100.).disabled(self.busy);
                            text_input_string(ui, "Identity URL", &mut self.identity_url)
                                .placeholder("Identity URL override").width_percent(100.).disabled(self.busy);
                            text_input_string(ui, "Icons URL", &mut self.icons_url)
                                .placeholder("Icons URL override").width_percent(100.).disabled(self.busy);
                        }
                    }
                    text(ui, "Email address").font_size(13).color(MUTED);
                    text_input_string(ui, "Email address", &mut self.email).placeholder("you@example.com")
                        .width_percent(100.).disabled(self.busy);
                    text(ui, "Master password").font_size(13).color(MUTED);
                    let submitted = text_input(ui, "Master password", &self.password).width_percent(100.)
                        .disabled(self.busy).response().submitted;
                    if button(ui).text("Sign in").primary().width_percent(100.).disabled(
                        self.busy
                            || self.email.trim().is_empty()
                            || self.password.is_empty(),
                    ).clicked()
                        || submitted
                    {
                        self.login();
                    }
                } else {
                    let choices: Vec<_> = self.factors.iter()
                        .map(|p| (factor_id(p.id).to_owned(), p.label.to_owned())).collect();
                    picker(ui, "Verification method", &mut self.factor,
                        self.popups.entry("factor".into()).or_default(), &choices);
                    let choice =
                        self.factors.iter().find(|p| factor_id(p.id) == self.factor);
                    let supported = choice.is_some_and(|p| p.supported);
                    if let Some(hint) = choice.and_then(|p| p.hint.as_deref()) {
                        text(ui, hint).font_size(13).color(MUTED).width_percent(100.);
                    }
                    text_input(ui, "Verification code", &self.otp).width_percent(100.)
                        .disabled(self.busy || !supported);
                    if !supported {
                        text(ui,"This verification provider is not supported by this client yet.")
                            .color(Color::Warning).width_percent(100.);
                    }
                    if button(ui).text("Verify").primary().width_percent(100.)
                        .disabled(self.busy || !supported || self.otp.is_empty()).clicked()
                    {
                        self.request(Request::TwoFactor(TwoFactorSubmitRequest {
                            provider: self.factor.clone(),
                            token: self.otp.take(),
                        }));
                    }
                    if self.factor == "email"
                        && button(ui).text("Send email code").ghost().background_alpha(0.).disabled(self.busy)
                            .clicked()
                    {
                        self.request(Request::SendEmail);
                    }
                }
                if self.busy {
                    text(ui, "Connecting…").color(MUTED);
                    if button(ui).text("Cancel sign-in").ghost().background_alpha(0.).clicked() {
                        self.lock();
                        self.notice = "Sign-in cancelled.".into();
                    }
                }
                if !self.error.is_empty() {
                    text(ui, &self.error).color(Color::Danger).width_percent(100.);
                }
                if !self.notice.is_empty() {
                    text(ui, &self.notice).color(MUTED).width_percent(100.);
                }
                if button(ui).text("Browse demo vault").ghost().background_alpha(0.).width_percent(100.)
                    .clicked()
                {
                    self.lock();
                    self.items = crate::model::demo_items();
                    self.selected = "github".into();
                    self.filter = "All Items".into();
                    self.login_open = false;
                    self.notice.clear();
                }
                text(ui,"No password or vault is persisted to disk. Online edits are encrypted before upload. AutoFill is not connected yet.")
                    .font_size(12).color(MUTED).width_percent(100.);
            };
        };
    }
    #[component]
    #[rustfmt::skip]
    pub fn editor_ui(&mut self, ui: &mut UiContext) {
        let Some(mut draft) = self.editor.take() else {
            return;
        };
        let mut cancel = false;
        let mut save = false;
        surface(ui).padding(24).gap(20) + |ui| {
            row(ui).width_percent(100.).justify(Justify::SpaceBetween).align(Align::Center).gap(12) + |ui| {
                column(ui).gap(5).width(0.).grow(1.) + |ui| {
                    text(ui, if draft.id.is_none() { "New item" } else { "Edit item" }).font_size(26).weight(700);
                    text(ui, "Changes are encrypted before they leave this device.").font_size(13)
                        .color(MUTED).width_percent(100.);
                };
                if button(ui).text("Cancel editing").ghost().background_alpha(0.).disabled(self.busy).clicked() {
                    cancel = true;
                }
            };
            if draft.id.is_none() {
                let before = draft.kind.clone();
                picker(
                    ui,
                    "Item type",
                    &mut draft.kind,
                    self.popups.entry("kind".into()).or_default(),
                    &entries(&[
                        ("login", "Login"),
                        ("note", "Secure note"),
                        ("card", "Card"),
                        ("identity", "Identity"),
                        ("ssh-key", "SSH key"),
                    ]),
                );
                if before != draft.kind {
                    draft.fields.retain(|k, _| k == "name" || k == "notes");
                }
            }
            ui.keyed("name", |ui| edit_field(ui, &mut draft, "name", "Item name"));
            row(ui).gap(12).width_percent(100.).align(Align::Center) + |ui| {
                checkbox(ui, "Favorite", &mut draft.favorite);
                let mut folders = vec![(String::new(), "No folder".into())];
                folders.extend(self.folders.iter().map(|f| (f.id.clone(), f.name.clone())));
                picker(ui, "Folder", &mut draft.folder, self.popups.entry("folder".into()).or_default(),
                    &folders);
            };
            for &(key, label) in kind_fields(&draft.kind) {
                ui.keyed(key, |ui| edit_field(ui, &mut draft, key, label));
            }
            if draft.kind == "login"
                && button(ui).text("Generate a strong password").ghost().background_alpha(0.).disabled(self.busy)
                    .clicked()
            {
                match crate::generator::generate(24, true) {
                    Ok(value) => draft.buffer("password", true, false).set(value),
                    Err(error) => self.error = error.to_string(),
                }
            }
            ui.keyed("notes", |ui| edit_field(ui, &mut draft, "notes", "Notes"));
            row(ui).width_percent(100.).justify(Justify::SpaceBetween).align(Align::Center) + |ui| {
                text(ui, "Custom fields").weight(650);
                if button(ui).text("Add custom field").ghost().background_alpha(0.).clicked() {
                    draft.custom.push(plicowarden_sdk::FieldView {
                        name: Some(String::new()),
                        value: Some(String::new()),
                        r#type: plicowarden_sdk::FieldType::Text,
                        linked_id: None,
                    });
                    draft.custom_keys.push(draft.next_custom_key);
                    draft.next_custom_key += 1;
                }
            };
            let mut remove = None;
            for (index, field) in draft.custom.iter_mut().enumerate() {
                let key = draft.custom_keys[index];
                ui.keyed(key, |ui| {
                    // Linked fields must survive untouched; they are not free-form text.
                    let kind = serde_json::to_value(field.r#type).ok().and_then(|v| v.as_u64()).unwrap_or(0);
                    if kind == 3 {
                        text(ui, field.name.as_deref().unwrap_or("Linked field")).color(MUTED);
                        return;
                    }
                    row(ui).width_percent(100.).gap(8).wrap(true).align(Align::Center) + |ui| {
                        let name = field.name.get_or_insert_with(String::new);
                        text_input_string(ui, "Field name", name).width(160.).grow(1.);
                        let value = field.value.get_or_insert_with(String::new);
                        if kind == 1 {
                            let buffer =
                                draft.custom_buffers.entry(key).or_insert_with(|| {
                                    let buffer = plico::TextBuffer::secret();
                                    buffer.set(Zeroizing::new(value.clone()));
                                    buffer
                                });
                            text_input(ui, "Field value", buffer).width(180.).grow(1.);
                            *value = buffer.with_value(str::to_owned);
                        } else {
                            text_input_string(ui, "Field value", value).width(180.).grow(1.);
                        }
                        let mut selected = kind.to_string();
                        picker(ui, "Field type", &mut selected,
                            self.popups.entry(format!("field-{key}")).or_default(),
                            &entries(&[("0", "Text"), ("1", "Hidden"), ("2", "Boolean")]));
                        if let Some(kind) = selected.parse::<u8>().ok()
                            .and_then(|n| plicowarden_sdk::FieldType::try_from(n).ok())
                        {
                            if field.r#type != kind {
                                draft.custom_buffers.remove(&key);
                            }
                            field.r#type = kind;
                        }
                        if button(ui).text("Remove field").ghost().background_alpha(0.).clicked() {
                            remove = Some(index);
                        }
                    };
                });
            }
            if let Some(index) = remove {
                draft.custom.remove(index);
                let key = draft.custom_keys.remove(index);
                if let Some(buffer) = draft.custom_buffers.remove(&key) {
                    buffer.clear();
                }
            }
            row(ui).width_percent(100.).justify(Justify::End).gap(10) + |ui| {
                let empty = draft.fields.get("name").is_none_or(|b| b.with_value(|s| s.trim().is_empty()));
                if self.busy {
                    text(ui, "Saving…").color(MUTED);
                }
                if button(ui).text("Save item").primary().disabled(self.busy || empty || !self.authenticated)
                    .clicked()
                {
                    save = true;
                }
            };
        };
        if !cancel {
            self.editor = Some(draft);
        }
        if save {
            self.save_editor();
        }
    }
    #[component]
    #[rustfmt::skip]
    pub fn folder_manager(&mut self, ui: &mut UiContext) {
        let mut state = std::mem::take(&mut self.folders_modal);
        modal(ui, &mut state).title("Folders").width(560.).children(|ui| {
            text(ui, "Folders").font_size(24).weight(700);
            text(ui, "Organize your personal vault. Folder names are encrypted.").width_percent(100.).color(MUTED);
            column(ui).width_percent(100.).max_height(360.).scroll_y(true).gap(10) + |ui| {
                for folder in self.folders.clone() {
                    ui.keyed(crate::model::business_key(&folder.id), |ui| {
                        row(ui).width_percent(100.).gap(8).align(Align::Center) + |ui| {
                            let value = self.folder_names.entry(folder.id.clone())
                                .or_insert_with(|| folder.name.clone());
                            text_input_string(ui, "Folder name", value).width(0.).grow(1.)
                                .disabled(self.busy);
                            let name = value.clone();
                            if button(ui).text("Rename").ghost().background_alpha(0.).disabled(
                                self.busy
                                    || name.trim().is_empty()
                                    || name.trim() == folder.name,
                            ).clicked()
                            {
                                self.request(Request::Folder {
                                    id: Some(folder.id.clone()),
                                    name,
                                    delete: false,
                                });
                            }
                            if button(ui).text("Delete folder").ghost().background_alpha(0.)
                                .disabled(self.busy).clicked()
                            {
                                self.ask(&folder.id, "deleteFolder");
                                ui.close_modal();
                            }
                        };
                    });
                }
            };
            row(ui).width_percent(100.).gap(8) + |ui| {
                text_input_string(ui, "New folder name", &mut self.folder_name).placeholder("New folder")
                    .width(0.).grow(1.).disabled(self.busy);
                if button(ui).text("Create folder").primary()
                    .disabled(self.busy || self.folder_name.trim().is_empty()).clicked()
                {
                    let name = std::mem::take(&mut self.folder_name);
                    self.request(Request::Folder {
                        id: None,
                        name,
                        delete: false,
                    });
                }
            };
            if !self.error.is_empty() {
                text(ui, &self.error).color(Color::Danger).width_percent(100.);
            }
            if button(ui).text("Done").ghost().background_alpha(0.).disabled(self.busy).clicked() {
                ui.close_modal();
            }
        });
        self.folders_modal = state;
    }
    #[component]
    #[rustfmt::skip]
    pub fn confirm_ui(&mut self, ui: &mut UiContext) {
        let Some((id, action)) = self.confirm_action.clone() else {
            return;
        };
        let mut state = std::mem::take(&mut self.confirm_modal);
        let (title, message, label) = match action.as_str() {
            "purge" => (
                "Delete permanently?",
                "This removes the encrypted item from your server. This cannot be undone.",
                "Delete permanently",
            ),
            "deleteFolder" => (
                "Delete folder?",
                "Items will remain in your vault without this folder.",
                "Delete folder",
            ),
            _ => (
                "Move item to trash?",
                "You can restore it from Trash later.",
                "Move to trash",
            ),
        };
        modal(ui, &mut state).title(title).width(480.).children(|ui| {
            text(ui, title).font_size(24).weight(700);
            text(ui, message).width_percent(100.).color(MUTED);
            row(ui).width_percent(100.).justify(Justify::End).gap(10) + |ui| {
                if button(ui).text("Cancel").ghost().background_alpha(0.).clicked() {
                    ui.close_modal();
                }
                if button(ui).text(label).primary().clicked() {
                    ui.close_modal();
                    if action == "deleteFolder" {
                        self.request(Request::Folder {
                            id: Some(id.clone()),
                            name: String::new(),
                            delete: true,
                        });
                    } else {
                        self.action(&id, &action);
                    }
                }
            };
        });
        self.confirm_modal = state;
    }
}

impl Warden {
    #[component]
    #[rustfmt::skip]
    pub fn ssh_approval_ui(&mut self, ui: &mut UiContext) {
        let Some(approval) = self.ssh_approvals.front().cloned() else {
            return;
        };
        let mut state = std::mem::take(&mut self.ssh_modal);
        let mut decision = None;
        modal(ui, &mut state).title("SSH signing request").width(560.).children(|ui| {
            column(ui).gap(16).width_percent(100.) + |ui| {
                text(ui, "Allow one SSH signature?").font_size(24).weight(700).width_percent(100.);
                text(ui, &approval.name).font_size(18).weight(600).width_percent(100.);
                text(ui, &approval.fingerprint).font_size(13).width_percent(100.);
                if let Some(pid) = approval.process_id {
                    text!(ui, "Local socket peer PID: {pid}").color(MUTED).width_percent(100.);
                }
                text(ui, "The destination and purpose are not verified. Only allow a request you just initiated. This request expires after 30 seconds.")
                    .color(MUTED).width_percent(100.);
                row(ui).gap(12).justify(Justify::End).width_percent(100.) + |ui| {
                    if button(ui).text("Deny SSH signature").clicked() { decision = Some(false); }
                    if button(ui).text("Allow SSH signature once").primary().disabled(self.busy).clicked() { decision = Some(true); }
                };
            };
        });
        if !state.is_open() && decision.is_none() {
            decision = Some(false);
        }
        self.ssh_modal = state;
        if let Some(allow) = decision {
            self.decide_ssh(approval.id, allow);
            ui.request_rebuild();
        }
    }
}
