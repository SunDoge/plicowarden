use crate::{
    icons,
    model::{HostCommand, Item, Warden},
};
use plico::prelude::*;
use plico::{BoxShadow, Layout, SvgAsset};
use plicowarden_sdk::Request;

pub const INK: Color = Color::Foreground;
pub const MUTED: Color = Color::Muted;
pub const EDGE: Color = Color::Border;
pub const ICON: Color = Color::Literal(0x62626c);
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
        .border(1., Color::Literal(0xd9d9de))
        .elevation(Elevation::PANEL)
}
/// Ordinary diagnostics use the native host clipboard, independently of the
/// account worker, so copying never clears an error or starts an SDK session.
#[component]
#[rustfmt::skip]
pub fn copyable_error(ui: &mut UiContext, message: &str) -> bool {
    if message.is_empty() {
        return false;
    }
    let mut copied = false;
    column(ui).width_percent(100.).gap(6) + |ui| {
        column(ui).width_percent(100.).selectable(true) + |ui| {
            text(ui, message).color(Color::Danger).width_percent(100.);
        };
        copied = button(ui).text("Copy error").icon(crate::icons::get("copy")).radius(6.).font_size(13).clicked();
    };
    copied
}
#[component]
pub fn glyph(ui: &mut UiContext, name: &str, size: f32, color: Color) {
    icon(ui, icons::get(name)).size(size, size).color(color);
}
#[component]
#[rustfmt::skip]
pub fn tool(ui: &mut UiContext, name: &str, label: &str, disabled: bool) -> bool {
    tooltip(ui, label).children(|ui| {
        (button_base(ui, label).width(38.).min_width(38.).max_width(38.).height(38.).min_height(38.).max_height(38.)
            .shrink(0.).padding(9.).radius(19.).corner_smoothing(0.).border(1., EDGE).background(Color::Control)
            .elevation(Elevation::CONTROL).disabled(disabled).opacity(if disabled { 0.45 } else { 1. }) + |ui|
        {
            glyph(ui, name, 18., ICON);
        }).response.clicked()
    }).inner
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
        if let Some(icon) = &item.icon {
            image(ui, icon).size(size * 0.7, size * 0.7);
        } else if let Some(asset) = asset {
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
    (plico::widgets::nav_entry(ui, label, selected).height(39.).padding_x(12).padding_y(6).gap(10).radius(10.)
        .corner_smoothing(1.).border(0., EDGE).justify(Justify::Start)
        .disabled(disabled).opacity(if disabled { 0.45 } else { 1. })
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
        self.apply_deferred_snapshot();
        self.update_local_search();
        let visible = self.visible();
        self.bulk_selected.retain(|id| visible.iter().any(|item| &item.id == id));
        if self.editor.is_none()
            && !self.busy
            && !visible.is_empty()
            && !visible.iter().any(|i| i.id == self.selected)
        {
            self.select(&visible[0].id);
        }
        let content_route = crate::model::business_key(if self.settings_open {"settings"} else if self.editor.is_some() {"editor"} else {self.filter.as_str()});
        let content_transition = ui.theme().metrics.interaction_ms;
        let compact = ui.viewport_width() < 1120.;
        let navigation_width = if compact { 240. } else { 330. };
        let show_sidebar = self.sidebar && ui.viewport_width() >= 780.;
        let header_height = 56_f32.max(56. / self.zoom.factor() as f32);
        column(ui).position(plico::Position::Relative).width_percent(100.).height_percent(100.)
            .background(Color::Literal(0xf4f4f5)) + |ui|
        {
            if self.login_open {
                self.account(ui);
                self.accounts_ui(ui);
                if self.custom_titlebar {
                    self.window_controls(ui);
                    let inset = (88. / self.zoom.factor()) as f32;
                    let width = (ui.viewport_width() - inset).max(1.);
                    row(ui).absolute(inset, 0.).width(width).height((56. / self.zoom.factor()) as f32) + |ui| {
                        self.window_drag_region(ui, false);
                    };
                }
                return;
            }
            row(ui).width_percent(100.).height_percent(100.).padding(12).gap(18) + |ui| {
                if show_sidebar {
                    self.sidebar_ui(ui, navigation_width);
                }
                column(ui).width(0.).grow(1.).min_width(0.).height_percent(100.).gap(18) + |ui| {
                    row(ui).position(plico::Position::Relative).width_percent(100.).height(header_height)
                        .shrink(0.).radius(14.).corner_smoothing(1.).background(Color::Panel)
                        .border(1., EDGE).elevation(Elevation::SUBTLE).padding_x(12).align(Align::Center)
                        .gap(12) + |ui|
                    {
                        if self.custom_titlebar && !show_sidebar {
                            column(ui).width((88. / self.zoom.factor()) as f32).shrink(0.).end();
                        }
                        if tool(ui, "sidebar", "Toggle sidebar", false) {
                            self.sidebar = !self.sidebar;
                        }
                        self.window_drag_region(ui, true);
                        let settings_clicked = if compact {
                            tool(ui, "settings", "Settings", self.busy || self.editor.is_some())
                        } else {
                            button(ui).radius(6.).corner_smoothing(1.).elevation(Elevation::CONTROL)
                                .icon(crate::icons::get("settings")).text("Settings").ghost()
                                .background_alpha(0.).disabled(self.busy || self.editor.is_some())
                                .clicked()
                        };
                        if settings_clicked {
                            self.settings_open = true;
                        }
                        if self.authenticated {
                            let new_clicked = if compact {
                                tool(ui, "plus", "New item", self.busy || self.editor.is_some())
                            } else {
                                button(ui).radius(6.).corner_smoothing(1.).elevation(Elevation::CONTROL)
                                    .icon(crate::icons::get("plus")).text("New item").primary()
                                    .disabled(self.busy || self.editor.is_some()).clicked()
                            };
                            if new_clicked {
                                self.editor = Some(crate::model::Editor::new());
                                self.settings_open = false;
                            }
                            if !compact {
                                text(ui, &self.account_email).font_size(12).color(MUTED).max_width(180.)
                                    .ellipsis(true);
                            }
                            let sync_clicked = if compact {
                                tool(ui, "sync", "Sync", self.busy)
                            } else {
                                button(ui).radius(6.).corner_smoothing(1.).elevation(Elevation::CONTROL)
                                    .icon(crate::icons::get("sync")).text("Sync").ghost()
                                    .background_alpha(0.).disabled(self.busy).clicked()
                            };
                            if sync_clicked {
                                self.request(Request::Sync);
                            }
                            if tool(ui, "lock", "Lock vault", false) {
                                self.lock();
                            }
                        } else if if compact {
                            tool(ui, "login", "Connect account", false)
                        } else {
                            button(ui).radius(6.).corner_smoothing(1.).elevation(Elevation::CONTROL)
                                .icon(crate::icons::get("login")).text("Connect account").ghost()
                                .background_alpha(0.).clicked()
                        } {
                            self.login_open = true;
                            self.notice.clear();
                        }
                    };
                    if self.bulk_mode && !self.settings_open {
                        self.bulk_toolbar(ui, &visible);
                    }
                    row(ui).width_percent(100.).height(0.).grow(1.).gap(22) + |ui| {
                        if !self.settings_open {
                            self.item_list(ui, &visible, navigation_width);
                        }
                        column(ui).key(content_route).width(0.).grow(1.).min_width(0.).height_percent(100.).scroll_y(true)
                            .transition(content_transition).enter_from(plico::Presentation {opacity:0.85,translate:[0.,3.],..Default::default()})
                            .gap(20).padding_y(1) + |ui|
                        {
                            self.login_progress_ui(ui);
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
                            if copyable_error(ui, &self.error) {
                                self.commands.push(
                                    crate::model::HostCommand::CopyError(self.error.clone()));
                            }
                            if !self.notice.is_empty() {
                                text(ui, &self.notice).font_size(13).color(MUTED)
                                    .width_percent(100.);
                            }
                        };
                    };
                };
            };
            self.accounts_ui(ui);
            self.transfer_modal_ui(ui);
            self.move_dialog(ui);
            self.folder_manager(ui);
            self.confirm_ui(ui);
            self.bulk_confirmation(ui);
            self.ssh_approval_ui(ui);
            if self.custom_titlebar {
                self.window_controls(ui);
            }
        };
    }
    #[component]
    #[rustfmt::skip]
    fn window_controls(&mut self, ui: &mut UiContext) {
        use plico_renderer::WindowCommand;
        // Window chrome stays at OS point size regardless of content zoom.
        let unit = (1. / self.zoom.factor()) as f32;
        row(ui).absolute(26. * unit, 28. * unit).gap(0).height(24. * unit) + |ui| {
            for (label, name, color, ink, command) in [
                (
                    "Close window",
                    "close",
                    0xff5f57,
                    0x78211c,
                    WindowCommand::Close,
                ),
                (
                    "Minimize window",
                    "minus",
                    0xfebc2e,
                    0x795600,
                    WindowCommand::Minimize,
                ),
                (
                    "Maximize or restore window",
                    "expand",
                    0x28c840,
                    0x075e16,
                    WindowCommand::ToggleMaximized,
                ),
            ] {
                ui.keyed(crate::model::business_key(label), |ui| {
                    if (button_base(ui, label).width(20. * unit).min_width(20. * unit).height(24. * unit).padding(0)
                        .background_alpha(0.).border(0., EDGE).align(Align::Center).justify(Justify::Center) + |ui|
                    {
                        let reveal = ui.hovered() || ui.pressed();
                        column(ui).width(12. * unit).height(12. * unit).radius(6. * unit).corner_smoothing(0.)
                            .background(Color::Literal(color)).align(Align::Center).justify(Justify::Center) + |ui|
                        {
                            if reveal {
                                glyph(ui, name, 8. * unit, Color::Literal(ink));
                            }
                        };
                    }).response.clicked()
                    {
                        self.commands.push(HostCommand::Window(command));
                    }
                });
            }
        };
    }
    #[component]
    #[rustfmt::skip]
    fn window_drag_region(&mut self, ui: &mut UiContext, title: bool) {
        if !self.custom_titlebar {
            text(ui, "Plicowarden").weight(700).color(INK);
            column(ui).grow(1.).end();
            return;
        }
        button_base(ui, "Move window").width(0.).grow(1.).min_width(32.).height_percent(100.).background_alpha(0.)
            .border(0., EDGE).justify(Justify::Start).align(Align::Center).padding(0).children(|ui| {
            let pressed = ui.pressed();
            if pressed && !self.titlebar_pressed {
                self.commands.push(HostCommand::Window(plico_renderer::WindowCommand::StartDragging));
            }
            self.titlebar_pressed = pressed;
            if title {
                text(ui, "Plicowarden").weight(700).color(INK).width(0.).grow(1.).min_width(0.).ellipsis(true);
            }
        });
    }
    #[component]
    #[rustfmt::skip]
    fn sidebar_ui(&mut self, ui: &mut UiContext, width: f32) {
        column(ui).position(plico::Position::Relative).width(width).shrink(0.).height_percent(100.).padding(12).gap(4)
            .radius(14.).corner_smoothing(1.).background(Color::Panel).border(1., EDGE).elevation(Elevation::SUBTLE)
        + |ui|
        {
            row(ui).width_percent(100.)
                .height(if self.custom_titlebar { 44_f32.max((56. / self.zoom.factor()) as f32) } else { 44. })
                .shrink(0.).align(Align::Center).gap(10).padding_x(10) + |ui|
            {
                if !self.custom_titlebar {
                    glyph(ui, "shield", 25., Color::Literal(0x68618d));
                    text(ui, "My workspace").weight(650);
                }
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
                }
                section(ui, "Folders");
                let unfiled = self.items.iter()
                    .filter(|item| item.folder.is_none() && !item.deleted && !item.archived).count();
                if nav(ui, "folder", "No folder", Some(unfiled), self.filter == "No folder",
                    self.busy || self.editor.is_some()) {
                    self.set_filter("No folder");
                }
                for folder in self.folders.clone() {
                    ui.keyed(crate::model::business_key(&folder.id), |ui| {
                        let filter = format!("folder:{}", folder.id);
                        let count = self.items.iter().filter(|item| {
                            item.folder.as_deref() == Some(folder.id.as_str())
                                && !item.deleted
                                && !item.archived
                        }).count();
                        if nav(ui, "folder", &folder.name, Some(count), self.filter == filter,
                            self.busy || self.editor.is_some()) {
                            self.set_filter(&filter);
                        }
                    });
                }
                if button(ui).radius(6.).corner_smoothing(1.).elevation(Elevation::CONTROL)
                    .icon(crate::icons::get("folder")).text("Manage folders").ghost().background_alpha(0.)
                    .width_percent(100.).disabled(!self.authenticated || self.busy || self.editor.is_some())
                    .clicked()
                {
                    self.folders_modal.open();
                }
                section(ui, "Utilities");
                if nav(ui, "sync", "Import / Export", None, self.filter == "Import / Export",
                    self.busy || self.editor.is_some()) {
                    self.transfer_modal.open();
                }
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
                        ).font_size(13).weight(600).width_percent(100.).ellipsis(true);
                        text(ui, if self.offline { "Cached vault" } else if self.authenticated { "Encrypted vault" } else { "Demo vault" }).font_size(12).color(MUTED).width_percent(100.);
                    };
                };
                if button(ui).text("Switch account").icon(crate::icons::get("login")).ghost().width_percent(100.).disabled(self.busy).clicked() {self.accounts_modal.open();}
            };
        };
    }
    #[component]
    #[rustfmt::skip]
    fn item_list(&mut self, ui: &mut UiContext, visible: &[Item], width: f32) {
        column(ui).width(width).shrink(0.).height_percent(100.).gap(12) + |ui| {
            row(ui).width_percent(100.).align(Align::Center).gap(8) + |ui| {
                row(ui).width(0.).grow(1.).height(40.).radius(6.).corner_smoothing(1.)
                    .background(Color::Literal(0xffffff)).background_alpha(1.).border(1., EDGE)
                    .focus_within_border(1., Color::Literal(0x7294a6)).elevation(Elevation::SUBTLE)
                    .padding_x(12).gap(8).align(Align::Center) + |ui|
                {
                    column(ui).width(17.).height(17.).shrink(0.) + |ui| {
                        glyph(ui, "search", 17., ICON);
                    };
                    text_input_string(ui, "Search all vaults", &mut self.query)
                        .placeholder("Search all vaults").grow(1.).width(0.).height(36.).unstyled();
                };
                if tool(ui, "check", "Bulk selection", self.busy || self.editor.is_some()) {
                    self.bulk_mode = !self.bulk_mode;
                    self.bulk_selected.clear();
                }
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
                        let mut clicked = false;
                        row(ui).width_percent(100.).height(68.).shrink(0.).gap(6).align(Align::Center) + |ui| {
                            if self.bulk_mode {
                                let checked = self.bulk_selected.contains(&item.id);
                                if (button_base(
                                    ui,
                                    format!("Toggle bulk selection for {}", item.name),
                                ).width(24.).height(24.).radius(6.).padding(3).border(1., Color::Border)
                                    .selected(checked)
                                    .background(if checked { Color::Accent } else { Color::Control })
                                    .disabled(self.busy) + |ui|
                                {
                                    if checked {
                                        glyph(ui, "check", 16., Color::OnAccent);
                                    }
                                }).response.clicked()
                                {
                                    if checked {
                                        self.bulk_selected.remove(&item.id);
                                    } else if self.bulk_selected.len() < 500 {
                                        self.bulk_selected.insert(item.id.clone());
                                    }
                                }
                            }
                            clicked =
                                (button_base(ui, format!("Select {}", item.name)).width(0.).grow(1.)
                                    .min_width(0.).height(68.).padding(10).gap(12).justify(Justify::Start)
                                    .radius(12.).corner_smoothing(1.)
                                    .border(if selected { 1. } else { 0. }, EDGE).background(if selected {
                                    Color::Literal(0xe8effe)
                                } else {
                                    Color::Literal(0xffffff)
                                }).background_alpha(if selected { 1. } else { 0. })
                                    .disabled(self.busy || self.editor.is_some()) + |ui|
                                {
                                    brand(ui, item, 40.);
                                    column(ui).width(0.).grow(1.).min_width(0.).gap(3) + |ui| {
                                        text(ui, &item.name).weight(650).font_size(15)
                                            .width_percent(100.).ellipsis(true);
                                        text(ui, &item.user).font_size(12).color(MUTED)
                                            .width_percent(100.).ellipsis(true);
                                    };
                                }).response.clicked();
                        };
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
        surface(ui).padding(28).gap(16) + |ui| {
            row(ui).width_percent(100.).justify(Justify::SpaceBetween).align(Align::Center) + |ui| {
                text(ui, "Settings").font_size(28).weight(700);
                if button(ui).radius(6.).corner_smoothing(1.).elevation(Elevation::CONTROL)
                    .icon(crate::icons::get("close")).text("Close settings").ghost().background_alpha(0.)
                    .clicked()
                {
                    self.settings_open = false;
                }
            };
            row(ui).gap(8).wrap(true) + |ui| {
                for (label, tab) in [
                    ("Appearance", "appearance"),
                    ("Security", "security"),
                    ("SSH agent", "ssh"),
                ] {
                    ui.keyed(tab, |ui| {
                        let control = button(ui).text(label).radius(18.).height(34.).padding_x(10).font_size(13)
                            .elevation(Elevation::SUBTLE);
                        let control = if self.settings_tab == tab {
                            control.primary()
                        } else {
                            control.ghost()
                        };
                        if control.clicked() {
                            self.settings_tab = tab.to_owned();
                        }
                    });
                }
            };
            if self.settings_tab == "security" {
                self.security_ui(ui);
                return;
            }
            if self.settings_tab == "appearance" {
                text(ui, "Appearance").font_size(18).weight(650);
                let before = self.appearance.ui_scale;
                slider(ui, "Interface scale", &mut self.appearance.ui_scale, 0.5..=3.0).step(0.05).width_percent(100.);
                text!(
                    ui,
                    "Interface scale: {:.0}%",
                    self.appearance.ui_scale * 100.
                ).font_size(13).color(MUTED).width_percent(100.);
                if self.appearance.ui_scale != before {
                    self.zoom.set(self.appearance.ui_scale);
                    ui.request_rebuild();
                }
                text(
                    ui,
                    if cfg!(target_os = "macos") {
                        "⌘ + / − to zoom · ⌘ 0 for 100% · Save appearance to keep your scale"
                    } else {
                        "Ctrl + / − to zoom · Ctrl 0 for 100% · Save appearance to keep your scale"
                    },
                ).font_size(12).color(MUTED).width_percent(100.);
                text(ui, "Solid surfaces, subtle borders and standard controls keep the vault clear and easy to read.")
                    .color(MUTED).font_size(13).width_percent(100.);
                row(ui).width_percent(100.).gap(10).wrap(true) + |ui| {
                    if button(ui).radius(6.).corner_smoothing(1.).elevation(Elevation::CONTROL)
                        .icon(crate::icons::get("save")).text("Save appearance").primary()
                        .disabled(self.settings_saving).clicked()
                    {
                        self.settings_saving = true;
                        self.commands.push(HostCommand::SaveAppearance(self.appearance.clone()));
                    }
                    if button(ui).radius(6.).corner_smoothing(1.).elevation(Elevation::CONTROL)
                        .icon(crate::icons::get("restore")).text("Reset appearance").ghost()
                        .background_alpha(0.).clicked()
                    {
                        self.appearance = Default::default();
                        self.zoom.set(self.appearance.ui_scale);
                        ui.request_rebuild();
                        self.notice = "Defaults restored. Save to keep them.".into();
                    }
                };
                return;
            }
            text(ui, "SSH agent").font_size(18).weight(650);
            text(ui, "Use vault SSH keys with OpenSSH and Git. Each signature requires your approval; locking disables the agent.")
                .color(MUTED).width_percent(100.);
            if let Some(socket) = &self.ssh_socket {
                text(ui, "Agent socket · set SSH_AUTH_SOCK to this path").font_size(12).color(MUTED)
                    .width_percent(100.);
                text(ui, socket).width_percent(100.);
            }
            if button(ui).radius(6.).corner_smoothing(1.).elevation(Elevation::CONTROL)
                .icon(crate::icons::get(if self.ssh_socket.is_some() { "power" } else { "terminal" }))
                .text(if self.ssh_socket.is_some() { "Disable SSH agent" } else { "Enable SSH agent" }).disabled(
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
                if button(ui).radius(6.).corner_smoothing(1.).elevation(Elevation::CONTROL)
                    .icon(crate::icons::get("sync")).text(match self.generator_mode {
                    crate::generator::Mode::Password => "Generate password",
                    crate::generator::Mode::Passphrase => "Generate passphrase",
                    crate::generator::Mode::Username => "Generate username",
                }).primary().clicked()
                {
                    self.generate();
                    ui.request_rebuild();
                }
                if button(ui).radius(6.).corner_smoothing(1.).elevation(Elevation::CONTROL)
                    .icon(crate::icons::get("copy"))
                    .text(if self.generator_mode == crate::generator::Mode::Password {
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
