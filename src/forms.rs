use crate::{
    model::{Editor, Warden, factor_id, multiline_field, secret_field},
    ui::*,
};
use plico::prelude::*;
use plicowarden_sdk::Request;
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
fn region_selector(
    ui: &mut UiContext,
    selected: &mut usize,
    disabled: bool,
    material: LiquidGlass,
) {
    row(ui).gap(2).padding(4).radius(19.).background(Color::Literal(0xe5edf2)) + |ui| {
        for (index, label) in ["Self-hosted", "Bitwarden US", "Bitwarden EU"].iter().enumerate()
        {
            ui.keyed(index as u64, |ui| {
                let control = button(ui).text(*label).height(30.).padding_x(8).radius(15.).font_size(13);
                let control = if *selected == index {
                    control.material(material).elevation(Elevation::SUBTLE)
                } else {
                    control.ghost().background_alpha(0.)
                };
                if control.disabled(disabled).clicked() {
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
        let width = (ui.viewport_width() - 48.).clamp(240., 432.);
        let top = if ui.viewport_size()[1] < 760. { 32 } else { 64 };
        let focus = std::mem::take(&mut self.login_focus);
        column(ui).width_percent(100.).height_percent(100.).align(Align::Center).padding(24).padding_top(top).gap(24)
            .scroll_y(true) + |ui|
        {
            row(ui).gap(10).align(Align::Center) + |ui| {
                glyph(ui, "shield", 24., Color::Accent);
                text(ui, "Plicowarden").font_size(22).weight(700).color(INK);
            };
            column(ui).width(width).gap(24).padding_top(24).padding_bottom(24) + |ui| {
                column(ui).width_percent(100.).align(Align::Center).gap(10) + |ui| {
                    glyph(ui, if self.login_password_step { "key" } else { "shield" }, 40., Color::Accent);
                    text(
                        ui,
                        if !self.factors.is_empty() {
                            "Verify your identity"
                        } else if self.login_password_step {
                            "Welcome back"
                        } else {
                            "Log in to your vault"
                        },
                    ).font_size(28).weight(700).color(INK);
                    if self.login_password_step {
                        text(ui, &self.email).font_size(14).color(MUTED);
                    }
                };
                surface(ui).width_percent(100.).padding(28).gap(18) + |ui| {
                    if self.factors.is_empty() {
                        if !self.login_password_step {
                            column(ui).width_percent(100.).gap(8) + |ui| {
                                text(ui, "Email address").font_size(14).weight(600).color(INK);
                                let submitted = text_input_string(ui, "Email address", &mut self.email).autofocus(true).request_focus(focus).placeholder("you@example.com")
                                    .height(42.).width_percent(100.).disabled(self.busy).response()
                                    .submitted;
                                if submitted {
                                    self.continue_login();
                                }
                            };
                            if button(ui).material(self.appearance.material()).radius(20.)
                                .corner_smoothing(1.).elevation(Elevation::CONTROL)
                                .icon(crate::icons::get("arrow-right")).text("Continue").primary()
                                .height(42.).width_percent(100.)
                                .disabled(self.busy || self.email.trim().is_empty()).clicked()
                            {
                                self.continue_login();
                            }
                        } else {
                            column(ui).width_percent(100.).gap(8) + |ui| {
                                text(ui, "Master password").font_size(14).weight(600).color(INK);
                                let submitted = text_input(ui, "Master password", &self.password).autofocus(true).request_focus(focus)
                                    .password_visible(self.login_show_password).height(42.)
                                    .width_percent(100.).disabled(self.busy).response().submitted;
                                if submitted {
                                    self.login();
                                }
                            };
                            checkbox(ui, "Show master password", &mut self.login_show_password).disabled(self.busy);
                            if button(ui).material(self.appearance.material()).radius(20.)
                                .corner_smoothing(1.).elevation(Elevation::CONTROL)
                                .icon(crate::icons::get("login"))
                                .text(if self.busy { "Logging in…" } else { "Sign in" }).primary()
                                .height(42.).width_percent(100.)
                                .disabled(self.busy || self.password.is_empty()).clicked()
                            {
                                self.login();
                            }
                        }
                    } else {
                        let previous = self.factor.clone();
                        let choices: Vec<_> = self.factors.iter()
                            .map(|p| (factor_id(p.id).to_owned(), p.label.to_owned())).collect();
                        column(ui).width_percent(100.).disabled(self.busy) + |ui| {
                            picker(ui, "Verification method", &mut self.factor,
                                self.popups.entry("factor".into()).or_default(), &choices);
                        };
                        if previous != self.factor {
                            self.otp.clear();
                            self.error.clear();
                            self.notice.clear();
                            self.login_focus = true;
                        }
                        let choice = self.factors.iter().find(|p| factor_id(p.id) == self.factor);
                        let supported = choice.is_some_and(|p| p.supported);
                        if let Some(hint) = choice.and_then(|p| p.hint.as_deref()) {
                            text(ui, hint).font_size(13).color(MUTED).width_percent(100.);
                        }
                        if supported {
                            text(ui, "Verification code").font_size(14).weight(600).color(INK);
                            let submitted =
                                text_input(ui, "Verification code", &self.otp).autofocus(true)
                                    .request_focus(focus).height(42.).width_percent(100.)
                                    .disabled(self.busy).response().submitted;
                            if button(ui).material(self.appearance.material()).radius(20.)
                                .corner_smoothing(1.).elevation(Elevation::CONTROL)
                                .icon(crate::icons::get("check"))
                                .text(if self.busy { "Verifying…" } else { "Verify" }).primary().height(42.)
                                .width_percent(100.).disabled(self.busy || self.otp.is_empty()).clicked()
                                || submitted
                            {
                                self.submit_verification();
                            }
                        } else {
                            text(ui, "This method is not available in Plicowarden. Choose another verification method or go back.")
                                .color(Color::Warning).width_percent(100.);
                        }
                        if self.factor == "email"
                            && button(ui).material(self.appearance.material()).radius(20.)
                                .corner_smoothing(1.).elevation(Elevation::CONTROL)
                                .icon(crate::icons::get("mail")).text("Send email code").ghost()
                                .background_alpha(0.).disabled(self.busy).clicked()
                        {
                            self.request(Request::SendEmail);
                        }
                    }
                    if self.busy
                        && button(ui).material(self.appearance.material()).radius(20.).corner_smoothing(1.)
                            .elevation(Elevation::CONTROL).icon(crate::icons::get("close"))
                            .text("Cancel sign-in").ghost().background_alpha(0.).clicked()
                    {
                        self.lock();
                        self.notice = "Sign-in cancelled.".into();
                    }
                    if !self.error.is_empty() {
                        text(ui, &self.error).color(Color::Danger).width_percent(100.);
                    }
                    if !self.notice.is_empty() {
                        text(ui, &self.notice).color(MUTED).width_percent(100.);
                    }
                    if (self.login_password_step || !self.factors.is_empty())
                        && !self.busy
                        && button(ui).material(self.appearance.material()).radius(20.).corner_smoothing(1.)
                            .elevation(Elevation::CONTROL).icon(crate::icons::get("arrow-left"))
                            .text("Back").width_percent(100.).height(40.).clicked()
                    {
                        self.back_to_email();
                    }
                };
                if !self.login_password_step && self.factors.is_empty() {
                    column(ui).width_percent(100.).gap(12).align(Align::Center) + |ui| {
                        row(ui).gap(4).align(Align::Center) + |ui| {
                            text(ui, "Accessing:").font_size(13).color(MUTED);
                            let label = match self.region.as_str() {
                                "us" => "bitwarden.com",
                                "eu" => "bitwarden.eu",
                                _ => "Self-hosted server",
                            };
                            if button(ui).material(self.appearance.material()).radius(20.)
                                .corner_smoothing(1.).elevation(Elevation::CONTROL)
                                .icon(crate::icons::get("external")).text(label).ghost()
                                .background_alpha(0.).color(Color::Accent).disabled(self.busy).clicked()
                            {
                                self.environment_open = !self.environment_open;
                                self.environment_error.clear();
                            }
                        };
                        if self.environment_open {
                            surface(ui).padding(20).gap(12) + |ui| {
                                text(ui, "Server").font_size(16).weight(650);
                                let mut region = match self.region.as_str() {
                                    "us" => 1,
                                    "eu" => 2,
                                    _ => 0,
                                };
                                region_selector(ui, &mut region, self.busy, self.appearance.material());
                                if self.region
                                    != ["self-hosted", "us", "eu"][region.min(2)]
                                {
                                    self.environment_error.clear();
                                }
                                self.region = ["self-hosted", "us", "eu"]
                                    [region.min(2)].into();
                                if self.region == "self-hosted" {
                                    text(ui, "Server URL").font_size(13).weight(600);
                                    text_input_string(ui, "Server URL", &mut self.server).placeholder("https://vault.example.com").width_percent(100.)
                                        .disabled(self.busy);
                                    checkbox(ui, "Custom service endpoints", &mut self.advanced);
                                    if self.advanced {
                                        for (label, value) in [
                                            ("API URL", &mut self.api_url),
                                            (
                                                "Identity URL",
                                                &mut self.identity_url,
                                            ),
                                            ("Icons URL", &mut self.icons_url),
                                        ] {
                                            text(ui, label).font_size(13).color(MUTED);
                                            text_input_string(ui, label, value)
                                                .placeholder("Optional override").width_percent(100.)
                                                .disabled(self.busy);
                                        }
                                    }
                                }
                                if !self.environment_error.is_empty() {
                                    text(ui, &self.environment_error).color(Color::Danger)
                                        .width_percent(100.);
                                }
                                if button(ui).material(self.appearance.material()).radius(20.)
                                    .corner_smoothing(1.).elevation(Elevation::CONTROL)
                                    .icon(crate::icons::get("check")).text("Done").width_percent(100.)
                                    .clicked()
                                    && self.validate_login_server()
                                {
                                    self.environment_open = false;
                                }
                            };
                        }
                    };
                }
                if button(ui).material(self.appearance.material()).radius(20.).corner_smoothing(1.)
                    .elevation(Elevation::CONTROL).icon(crate::icons::get("grid")).text("Browse demo vault")
                    .ghost().background_alpha(0.).width_percent(100.).disabled(self.busy).clicked()
                {
                    self.lock();
                    self.items = crate::model::demo_items();
                    self.selected = "github".into();
                    self.filter = "All Items".into();
                    self.login_open = false;
                    self.notice.clear();
                }
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
                if button(ui).material(self.appearance.material()).radius(20.).corner_smoothing(1.)
                    .elevation(Elevation::CONTROL).icon(crate::icons::get("close")).text("Cancel editing")
                    .ghost().background_alpha(0.).disabled(self.busy).clicked()
                {
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
                && button(ui).material(self.appearance.material()).radius(20.).corner_smoothing(1.)
                    .elevation(Elevation::CONTROL).icon(crate::icons::get("sync"))
                    .text("Generate a strong password").ghost().background_alpha(0.).disabled(self.busy).clicked()
            {
                match crate::generator::generate(24, true) {
                    Ok(value) => draft.buffer("password", true, false).set(value),
                    Err(error) => self.error = error.to_string(),
                }
            }
            ui.keyed("notes", |ui| edit_field(ui, &mut draft, "notes", "Notes"));
            row(ui).width_percent(100.).justify(Justify::SpaceBetween).align(Align::Center) + |ui| {
                text(ui, "Custom fields").weight(650);
                if button(ui).material(self.appearance.material()).radius(20.).corner_smoothing(1.)
                    .elevation(Elevation::CONTROL).icon(crate::icons::get("plus")).text("Add custom field")
                    .ghost().background_alpha(0.).clicked()
                {
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
                        if button(ui).material(self.appearance.material()).radius(20.).corner_smoothing(1.)
                            .elevation(Elevation::CONTROL).icon(crate::icons::get("close")).text("Remove field")
                            .ghost().background_alpha(0.).clicked()
                        {
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
                if button(ui).material(self.appearance.material()).radius(20.).corner_smoothing(1.)
                    .elevation(Elevation::CONTROL).icon(crate::icons::get("save")).text("Save item").primary()
                    .disabled(self.busy || empty || !self.authenticated).clicked()
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
    pub fn move_dialog(&mut self, ui: &mut UiContext) {
        let mut state = std::mem::take(&mut self.move_modal);
        let mut confirmed = false;
        modal(ui, &mut state).material(self.appearance.dialog_material()).title("Move to folder").width(440.)
            .children(|ui| {
            text!(
                ui,
                "Choose a destination for {} item{}.",
                self.move_ids.len(),
                if self.move_ids.len() == 1 { "" } else { "s" }
            ).font_size(13).color(MUTED).width_percent(100.);
            text(ui, "Destination folder").font_size(14).weight(600);
            let mut folders = vec![(String::new(), "No folder".to_owned())];
            folders.extend(self.folders.iter().map(|folder| (folder.id.clone(), folder.name.clone())));
            column(ui).width_percent(100.).disabled(self.busy) + |ui| {
                picker(ui, "Destination folder", &mut self.move_folder,
                    self.popups.entry("move-folder".into()).or_default(), &folders);
            };
            if !self.error.is_empty() {
                text(ui, &self.error).color(Color::Danger).width_percent(100.);
            }
            row(ui).width_percent(100.).gap(10).justify(Justify::End) + |ui| {
                if button(ui).text("Cancel").aria_label("Cancel move").ghost()
                    .material(self.appearance.material()).radius(20.).clicked()
                {
                    ui.close_modal();
                }
                if button(ui).text("Move").aria_label("Move selected items").icon(crate::icons::get("folder"))
                    .primary().material(self.appearance.material()).radius(20.).elevation(Elevation::CONTROL)
                    .disabled(self.busy || self.move_ids.is_empty()).clicked()
                {
                    confirmed = true;
                }
            };
        });
        self.move_modal = state;
        if confirmed {
            self.move_items();
        }
    }
    #[component]
    #[rustfmt::skip]
    pub fn folder_manager(&mut self, ui: &mut UiContext) {
        let mut state = std::mem::take(&mut self.folders_modal);
        modal(ui, &mut state).material(self.appearance.dialog_material()).header(false).title("Folders").width(560.)
            .children(|ui| {
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
                            if button(ui).material(self.appearance.material()).radius(20.)
                                .corner_smoothing(1.).elevation(Elevation::CONTROL)
                                .icon(crate::icons::get("edit")).text("Rename").ghost().background_alpha(0.)
                                .disabled(
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
                            if button(ui).material(self.appearance.material()).radius(20.)
                                .corner_smoothing(1.).elevation(Elevation::CONTROL)
                                .icon(crate::icons::get("trash")).text("Delete folder").ghost()
                                .background_alpha(0.).disabled(self.busy).clicked()
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
                if button(ui).material(self.appearance.material()).radius(20.).corner_smoothing(1.)
                    .elevation(Elevation::CONTROL).icon(crate::icons::get("plus")).text("Create folder")
                    .primary().disabled(self.busy || self.folder_name.trim().is_empty()).clicked()
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
            if button(ui).material(self.appearance.material()).radius(20.).corner_smoothing(1.)
                .elevation(Elevation::CONTROL).icon(crate::icons::get("check")).text("Done").ghost()
                .background_alpha(0.).disabled(self.busy).clicked()
            {
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
        modal(ui, &mut state).material(self.appearance.dialog_material()).title(title).width(480.).children(|ui| {
            text(ui, title).font_size(24).weight(700);
            text(ui, message).width_percent(100.).color(MUTED);
            row(ui).width_percent(100.).justify(Justify::End).gap(10) + |ui| {
                if button(ui).material(self.appearance.material()).radius(20.).corner_smoothing(1.)
                    .elevation(Elevation::CONTROL).icon(crate::icons::get("close")).text("Cancel").ghost()
                    .background_alpha(0.).clicked()
                {
                    ui.close_modal();
                }
                if button(ui).material(self.appearance.material()).radius(20.).corner_smoothing(1.)
                    .elevation(Elevation::CONTROL).icon(crate::icons::get("trash")).text(label).primary()
                    .clicked()
                {
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
        modal(ui, &mut state).material(self.appearance.dialog_material()).title("SSH signing request").width(560.)
            .children(|ui| {
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
                    if button(ui).material(self.appearance.material()).radius(20.).corner_smoothing(1.)
                        .elevation(Elevation::CONTROL).icon(crate::icons::get("close")).text("Deny SSH signature")
                        .clicked() { decision = Some(false); }
                    if button(ui).material(self.appearance.material()).radius(20.).corner_smoothing(1.)
                        .elevation(Elevation::CONTROL).icon(crate::icons::get("check")).text("Allow SSH signature once")
                        .primary().disabled(self.busy).clicked() { decision = Some(true); }
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
