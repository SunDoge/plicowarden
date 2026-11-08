use crate::{
    icons,
    model::{HostCommand, Item, Warden},
};
use plico::prelude::*;
use plico::{BoxShadow, Layout, SvgAsset};
use plicowarden_sdk::Request;

pub const INK: Color = Color::Literal(0x243443);
pub const MUTED: Color = Color::Literal(0x647684);
pub const EDGE: Color = Color::Literal(0xffffff);
pub const ICON: Color = Color::Literal(0x62626c);
static AMBIENCE: plico::ShaderAsset = plico::shader_asset!("../assets/ambience.wgsl");
pub fn shadow(y: f32, blur: f32, spread: f32, opacity: f32) -> BoxShadow {
    BoxShadow {
        offset_x: 0.,
        offset_y: y,
        blur,
        spread,
        color: Color::Literal(0x000000),
        opacity,
    }
}
#[track_caller]
pub fn surface<'u, 'e>(ui: &'u mut UiContext<'e>) -> Layout<'u, 'e> {
    column(ui)
        .width_percent(100.)
        .shrink(0.)
        .radius(14.)
        .corner_smoothing(1.)
        .background(Color::Literal(0xffffff))
        .background_alpha(0.84)
        .border(1., Color::Literal(0xe3eaf0))
        .elevation(Elevation::PANEL)
}
#[component]
pub fn glyph(ui: &mut UiContext, name: &str, size: f32, color: Color) {
    icon(ui, icons::get(name)).size(size, size).color(color);
}
#[component]
#[rustfmt::skip]
pub fn tool(ui: &mut UiContext, name: &str, label: &str, disabled: bool) -> bool {
    (button_base(ui, label).width(38.).height(38.).padding(9.).radius(19.).corner_smoothing(1.).border(1., EDGE)
        .background(Color::Literal(0xffffff)).background_alpha(0.84).elevation(Elevation::CONTROL).disabled(disabled)
        .opacity(if disabled { 0.45 } else { 1. }) + |ui|
    {
        glyph(ui, name, 18., ICON);
    }).response.clicked()
}
#[component]
#[rustfmt::skip]
pub fn brand(ui: &mut UiContext, item: &Item, size: f32) {
    column(ui).width(size).height(size).shrink(0.).radius(size * 0.25).corner_smoothing(1.)
        .background(Color::Literal(0xffffff)).border(1., Color::Literal(0xeae9ee)).align(Align::Center)
        .justify(Justify::Center).shadow(shadow(if size > 50. { 3. } else { 1. }, if size > 50. { 10. } else { 4. },
            0., if size > 50. { 0.13 } else { 0.07 })) + |ui|
    {
        let asset = match item.id.as_str() {
            "airbnb" => Some(SvgAsset::new(include_str!("../assets/airbnb.svg"))),
            "apple" => Some(SvgAsset::new(include_str!("../assets/apple.svg"))),
            "figma" => Some(SvgAsset::new(include_str!("../assets/figma.svg"))),
            "github" => Some(SvgAsset::new(include_str!("../assets/github.svg"))),
            "linear" => Some(SvgAsset::new(include_str!("../assets/linear.svg"))),
            "notion" => Some(SvgAsset::new(include_str!("../assets/notion.svg"))),
            _ => None,
        };
        if let Some(asset) = asset {
            svg(ui, &asset).size(size * 0.62, size * 0.62).color(Color::Literal(item.color));
        } else {
            text(ui, &item.mark).font_size((size * 0.42) as u16).weight(700).color(Color::Literal(item.color));
        }
    };
}
#[component]
#[rustfmt::skip]
pub fn section(ui: &mut UiContext, name: &str) {
    text(ui, name).padding_x(12).padding_y(12).font_size(12).weight(650).color(MUTED);
}
#[component]
#[rustfmt::skip]
fn nav(
    ui: &mut UiContext,
    name: &str,
    label: &str,
    count: Option<usize>,
    selected: bool,
    disabled: bool,
) -> bool {
    (button_base(ui, label).width_percent(100.).height(39.).padding_x(12).padding_y(6).gap(10).radius(10.)
        .corner_smoothing(1.).border(0., EDGE).justify(Justify::Start)
        .background(if selected { Color::Literal(0xc8dfeb) } else { Color::Literal(0xffffff) })
        .background_alpha(if selected { 0.65 } else { 0. }).disabled(disabled).opacity(if disabled { 0.45 } else { 1. })
    + |ui|
    {
        glyph(ui, name, 18., ICON);
        text(ui, label).grow(1.).font_size(15);
        if let Some(count) = count {
            text!(ui, "{count}").font_size(13).color(MUTED);
        }
    }).response.clicked()
}
impl Warden {
    #[component]
    #[rustfmt::skip]
    pub fn show_ui(&mut self, ui: &mut UiContext) {
        let visible = self.visible();
        if self.editor.is_none()
            && !self.busy
            && !visible.is_empty()
            && !visible.iter().any(|i| i.id == self.selected)
        {
            self.select(&visible[0].id);
        }
        let compact = ui.viewport_width() < 1120.;
        column(ui).position(plico::Position::Relative).width_percent(100.).height_percent(100.)
            .background(Color::Literal(0xeef4f8)) + |ui|
        {
            shader(ui, &AMBIENCE).absolute(0., 0.).radius(0.).width_percent(100.).height_percent(100.);
            if self.login_open {
                self.account(ui);
                return;
            }
            row(ui).width_percent(100.).height_percent(100.).padding(12).gap(18) + |ui| {
                if self.sidebar && ui.viewport_width() >= 980. {
                    self.sidebar_ui(ui);
                }
                column(ui).width(0.).grow(1.).min_width(0.).height_percent(100.).gap(18) + |ui| {
                    row(ui).position(plico::Position::Relative).width_percent(100.).height(56.).shrink(0.)
                        .radius(18.).corner_smoothing(1.).elevation(Elevation::FLOATING)
                        .material(self.appearance.material()).padding_x(12).align(Align::Center).gap(12) + |ui|
                    {
                        if tool(ui, "sidebar", "Toggle sidebar", false) {
                            self.sidebar = !self.sidebar;
                        }
                        text(ui, "Plicowarden").weight(700).color(INK);
                        column(ui).grow(1.).end();
                        if button(ui).text("Settings").ghost().background_alpha(0.)
                            .disabled(self.busy || self.editor.is_some()).clicked()
                        {
                            self.settings_open = true;
                        }
                        if self.authenticated {
                            if button(ui).text("New item").primary()
                                .disabled(self.busy || self.editor.is_some()).clicked()
                            {
                                self.editor = Some(crate::model::Editor::new());
                                self.settings_open = false;
                            }
                            text(ui, &self.account_email).font_size(12).color(MUTED);
                            if button(ui).text("Sync").ghost().background_alpha(0.).disabled(self.busy)
                                .clicked()
                            {
                                self.request(Request::Sync);
                            }
                            if tool(ui, "lock", "Lock vault", false) {
                                self.lock();
                            }
                        } else if button(ui).text("Connect account").ghost().background_alpha(0.)
                            .clicked()
                        {
                            self.login_open = true;
                            self.notice.clear();
                        }
                    };
                    row(ui).width_percent(100.).height(0.).grow(1.).gap(22) + |ui| {
                        self.item_list(ui, &visible, if compact { 270. } else { 330. });
                        column(ui).width(0.).grow(1.).min_width(0.).height_percent(100.).scroll_y(true)
                            .gap(20).padding_y(1) + |ui|
                        {
                            if self.settings_open {
                                self.settings(ui);
                            } else if self.editor.is_some() {
                                self.editor_ui(ui);
                            } else if self.filter == "Generator" {
                                self.generator_ui(ui);
                            } else if visible.is_empty() {
                                surface(ui).padding(32).gap(12) + |ui| {
                                    text(ui, "No matching items").font_size(24).weight(650);
                                    text(ui,if self.authenticated {"Create an item, or change your search and filters."}else{"Try a name, username or website."})
                                        .color(MUTED).width_percent(100.);
                                };
                            } else if let Some(item) = self.items.iter()
                                .find(|i| i.id == self.selected).cloned()
                            {
                                self.detail(ui, &item, compact);
                            }
                            if !self.error.is_empty() {
                                text(ui, &self.error).color(Color::Danger).width_percent(100.);
                            }
                            if !self.notice.is_empty() {
                                text(ui, &self.notice).font_size(13).color(MUTED)
                                    .width_percent(100.);
                            }
                        };
                    };
                };
            };
            self.folder_manager(ui);
            self.confirm_ui(ui);
            self.ssh_approval_ui(ui);
        };
    }
    #[component]
    #[rustfmt::skip]
    fn sidebar_ui(&mut self, ui: &mut UiContext) {
        column(ui).position(plico::Position::Relative).width(224.).shrink(0.).height_percent(100.).padding(12).gap(4)
            .radius(22.).corner_smoothing(1.).elevation(Elevation::OVERLAY).material(self.appearance.material())
        + |ui|
        {
            row(ui).width_percent(100.).height(44.).shrink(0.).align(Align::Center).gap(10).padding_x(10) + |ui| {
                glyph(ui, "shield", 25., Color::Literal(0x68618d));
                text(ui, "My workspace").weight(650);
            };
            column(ui).width_percent(100.).height(0.).grow(1.).scroll_y(true).gap(4) + |ui| {
                section(ui, "Items");
                for (icon, label, kind) in [
                    ("grid", "All Items", ""),
                    ("star", "Favorites", "*"),
                    ("key", "Logins", "login"),
                    ("card", "Cards", "card"),
                    ("person", "Identities", "identity"),
                    ("note", "Secure Notes", "note"),
                    ("terminal", "SSH Keys", "ssh-key"),
                ] {
                    let count = self.items.iter().filter(|i| {
                        kind.is_empty()
                            || if kind == "*" { i.favorite } else { i.kind == kind }
                    }).count();
                    ui.keyed(label, |ui| {
                        if nav(ui, icon, label, Some(count), self.filter == label,
                            self.busy || self.editor.is_some()) {
                            self.set_filter(label);
                        }
                    });
                }
                if self.authenticated {
                    for (icon, label, archived) in
                        [("note", "Archive", true), ("folder", "Trash", false)]
                    {
                        let count = self.items.iter().filter(|i| {
                            if archived { i.archived && !i.deleted } else { i.deleted }
                        }).count();
                        ui.keyed(label, |ui| {
                            if nav(ui, icon, label, Some(count), self.filter == label,
                                self.busy || self.editor.is_some()) {
                                self.set_filter(label);
                            }
                        });
                    }
                    section(ui, "Folders");
                    for folder in self.folders.clone() {
                        ui.keyed(crate::model::business_key(&folder.id), |ui| {
                            if button(ui).text(&folder.name).ghost().background_alpha(0.).width_percent(100.)
                                .disabled(self.busy || self.editor.is_some()).clicked()
                            {
                                self.filter = format!("folder:{}", folder.id);
                            }
                        });
                    }
                    if button(ui).text("Manage folders").ghost().background_alpha(0.).width_percent(100.)
                        .disabled(self.busy || self.editor.is_some()).clicked()
                    {
                        self.folders_modal.open();
                    }
                }
                section(ui, "Utilities");
                if nav(ui, "clock", "One-time Codes", None, self.filter == "One-time Codes",
                    !self.authenticated || self.busy || self.editor.is_some()) {
                    self.set_filter("One-time Codes");
                }
                if nav(ui, "sliders", "Generator", None, self.filter == "Generator",
                    self.busy || self.editor.is_some()) {
                    self.set_filter("Generator");
                }
            };
            surface(ui).padding(12).gap(4) + |ui| {
                row(ui).gap(10).align(Align::Center) + |ui| {
                    avatar(ui).fallback("A").size(34.).font_size(13).background(Color::Selection)
                        .color(Color::Literal(0x1d4ed8));
                    column(ui).width(0.).grow(1.).min_width(0.).gap(3) + |ui| {
                        text(
                            ui,
                            if self.authenticated { &self.account_email } else { "alex@example.com" },
                        ).font_size(13).weight(600).width_percent(100.);
                        text(
                            ui,
                            if self.authenticated {
                                "Encrypted vault"
                            } else {
                                "Demo vault · local only"
                            },
                        ).font_size(12).color(MUTED).width_percent(100.);
                    };
                };
            };
        };
    }
    #[component]
    #[rustfmt::skip]
    fn item_list(&mut self, ui: &mut UiContext, visible: &[Item], width: f32) {
        column(ui).width(width).shrink(0.).height_percent(100.).gap(12) + |ui| {
            row(ui).width_percent(100.).align(Align::Center).gap(8) + |ui| {
                row(ui).width(0.).grow(1.).height(40.).radius(20.).corner_smoothing(1.)
                    .background(Color::Literal(0xffffff)).background_alpha(0.85).border(1., EDGE)
                    .focus_within_border(1., Color::Literal(0x7294a6)).elevation(Elevation::SUBTLE)
                    .padding_x(12).gap(8).align(Align::Center) + |ui|
                {
                    column(ui).width(17.).height(17.).shrink(0.) + |ui| {
                        glyph(ui, "search", 17., ICON);
                    };
                    text_input_string(ui, "Search all vaults", &mut self.query)
                        .placeholder("Search all vaults").grow(1.).width(0.).height(36.).unstyled();
                };
                if tool(ui, "sort", "Reverse sort", false) {
                    self.ascending = !self.ascending;
                }
            };
            surface(ui).height(0.).grow(1.).scroll_y(true).padding(10).gap(5) + |ui| {
                row(ui).width_percent(100.).height(16.).shrink(0.).padding_x(10).padding_y(8)
                    .justify(Justify::SpaceBetween) + |ui|
                {
                    text(ui, &self.filter).font_size(12).weight(600).color(MUTED);
                    text!(ui, "{}", visible.len()).font_size(12).color(MUTED);
                };
                for item in visible {
                    ui.keyed(crate::model::business_key(&item.id), |ui| {
                        let selected = self.selected == item.id;
                        let clicked = (button_base(ui, format!("Select {}", item.name)).width_percent(100.)
                            .height(68.).padding(10).gap(12).justify(Justify::Start).radius(12.)
                            .corner_smoothing(1.).border(if selected { 1. } else { 0. }, EDGE)
                            .background(if selected {
                            Color::Literal(0xe0edf3)
                        } else {
                            Color::Literal(0xffffff)
                        }).background_alpha(if selected { 0.85 } else { 0. })
                            .disabled(self.busy || self.editor.is_some()) + |ui|
                        {
                            brand(ui, item, 40.);
                            column(ui).width(0.).grow(1.).min_width(0.).gap(3) + |ui| {
                                text(ui, &item.name).weight(650).font_size(15).width_percent(100.)
                                    .ellipsis(true);
                                text(ui, &item.user).font_size(12).color(MUTED).width_percent(100.)
                                    .ellipsis(true);
                            };
                        }).response.clicked();
                        if clicked {
                            self.select(&item.id);
                        }
                    });
                }
            };
        };
    }
    #[component]
    #[rustfmt::skip]
    pub fn settings(&mut self, ui: &mut UiContext) {
        surface(ui).padding(28).gap(24) + |ui| {
            row(ui).width_percent(100.).justify(Justify::SpaceBetween).align(Align::Center) + |ui| {
                text(ui, "Settings").font_size(28).weight(700);
                if button(ui).text("Close settings").ghost().background_alpha(0.).clicked() {
                    self.settings_open = false;
                }
            };
            text(ui, "Appearance").font_size(18).weight(650);
            text(ui,"Liquid Glass uses the scene behind each pane for blur and edge refraction. It does not require macOS desktop effects.")
                .color(MUTED).width_percent(100.);
            switch(ui, "Liquid Glass", &mut self.appearance.liquid_glass);
            switch(ui, "Animate reflections", &mut self.appearance.animate_reflections);
            switch(ui, "Reduce motion", &mut self.appearance.reduce_motion);
            slider(ui, "Glass frost", &mut self.appearance.frost, 0.0..=1.0).width_percent(100.);
            slider(ui, "Glass tint", &mut self.appearance.tint, 0.2..=0.9).width_percent(100.);
            text(ui,"Turn off Liquid Glass for opaque surfaces. Reduce motion stops the moving reflection without removing the glass.")
                .color(MUTED).font_size(13).width_percent(100.);
            column(ui).width_percent(100.).padding(22).gap(8).radius(18.).corner_smoothing(1.)
                .material(self.appearance.material()) + |ui|
            {
                text(ui, "Live preview").font_size(20).weight(650);
                text(ui, "Text stays sharp; only the background is refracted.").width_percent(100.);
            };
            row(ui).width_percent(100.).gap(10).wrap(true) + |ui| {
                if button(ui).text("Save appearance").primary().disabled(self.settings_saving).clicked() {
                    self.settings_saving = true;
                    self.commands.push(HostCommand::SaveAppearance(self.appearance.clone()));
                }
                if button(ui).text("Reset appearance").ghost().background_alpha(0.).clicked() {
                    self.appearance = Default::default();
                    self.notice = "Defaults restored. Save to keep them.".into();
                }
            };
            text(ui, "SSH agent").font_size(18).weight(650);
            text(ui, "Use vault SSH keys with OpenSSH and Git. Each signature requires your approval; locking disables the agent.")
                .color(MUTED).width_percent(100.);
            if let Some(socket) = &self.ssh_socket {
                text(ui, "Agent socket · set SSH_AUTH_SOCK to this path").font_size(12).color(MUTED)
                    .width_percent(100.);
                text(ui, socket).width_percent(100.);
            }
            if button(ui).text(if self.ssh_socket.is_some() { "Disable SSH agent" } else { "Enable SSH agent" })
                .disabled(
                self.busy
                    || (!self.authenticated && self.ssh_socket.is_none())
                    || !cfg!(unix),
            ).clicked()
            {
                self.request(Request::SshAgent(self.ssh_socket.is_none()));
            }
            if !cfg!(unix) {
                text(
                    ui,
                    "SSH agent currently requires a Unix socket; Windows support is pending.",
                ).font_size(12).color(MUTED).width_percent(100.);
            } else if !self.authenticated {
                text(ui, "Connect your account to enable SSH agent.").font_size(12).color(MUTED)
                    .width_percent(100.);
            }
            text(ui, "Account security").font_size(18).weight(650);
            text(ui,"The vault locks after five minutes of inactivity. Credentials and decrypted vault data are not saved with appearance preferences.")
                .color(MUTED).width_percent(100.);
        };
    }
    #[component]
    #[rustfmt::skip]
    fn generator_ui(&mut self, ui: &mut UiContext) {
        surface(ui).padding(28).gap(20) + |ui| {
            text(ui, "Generator").font_size(28).weight(700);
            text(ui,"Generated on this device using the operating system’s cryptographic random source.")
                .color(MUTED).width_percent(100.);
            row(ui).gap(8).wrap(true) + |ui| {
                for (label, mode) in [
                    ("Password", crate::generator::Mode::Password),
                    ("Passphrase", crate::generator::Mode::Passphrase),
                    ("Username", crate::generator::Mode::Username),
                ] {
                    ui.keyed(label, |ui| {
                        if choice(ui, label, &mut self.generator_mode, mode).clicked() {
                            ui.request_rebuild();
                            self.generated.clear();
                        }
                    });
                }
            };
            match self.generator_mode {
                crate::generator::Mode::Password => {
                    slider(ui, "Password length", &mut self.length, 8.0..=128.0).step(1.).width_percent(100.);
                    text!(ui, "{} characters", self.length.round() as usize).color(MUTED);
                    switch(ui, "Include symbols", &mut self.symbols);
                }
                crate::generator::Mode::Passphrase => {
                    slider(ui, "Word count", &mut self.word_count, 6.0..=12.0).step(1.).width_percent(100.);
                    text!(
                        ui,
                        "{} independently selected EFF words",
                        self.word_count.round() as usize
                    ).color(MUTED);
                }
                crate::generator::Mode::Username => {
                    text(ui, "Random username · availability is not checked · not a password").color(MUTED).width_percent(100.);
                }
            }
            if !self.generated.is_empty() {
                text(ui, self.generated.as_str()).font_size(22).width_percent(100.);
            }
            row(ui).width_percent(100.).gap(10).wrap(true) + |ui| {
                if button(ui).text(match self.generator_mode {
                    crate::generator::Mode::Password => "Generate password",
                    crate::generator::Mode::Passphrase => "Generate passphrase",
                    crate::generator::Mode::Username => "Generate username",
                }).primary().clicked()
                {
                    self.generate();
                    ui.request_rebuild();
                }
                if button(ui).text(if self.generator_mode == crate::generator::Mode::Password {
                    "Copy generated password"
                } else {
                    "Copy generated value"
                }).ghost().background_alpha(0.).disabled(self.generated.is_empty() || self.busy).clicked()
                {
                    self.copy_text(self.generated.clone());
                }
            };
            text(
                ui,
                "Recent generated values stay in memory until you lock or close the app.",
            ).font_size(13).color(MUTED).width_percent(100.);
            for (index, value) in self.history.iter().enumerate() {
                ui.keyed(index as u64, |ui| {
                    text(ui, value.as_str()).font_size(14).width_percent(100.);
                });
            }
        };
    }
}
