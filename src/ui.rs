use crate::{
    icons,
    model::{AppNotification, HostCommand, Item, NotificationKind, Page, Warden},
};
use plico::prelude::*;
use plico::{BoxShadow, Input, Layout, SvgAsset};
use plicowarden_sdk::{
    Request,
    ssh_agent::{AccessRecord, Outcome},
};

pub const INK: Color = Color::Foreground;
pub const MUTED: Color = Color::Muted;
pub const EDGE: Color = Color::Border;
pub const ICON: Color = Color::Muted;
static BACKDROP: ShaderAsset = ShaderAsset::new(include_str!("../assets/backdrop.wgsl"));
/// The window's pastel wash, drawn behind every floating panel.
#[component]
#[rustfmt::skip]
fn backdrop(ui: &mut UiContext) {
    if ui.display_preferences().reduce_transparency {
        return;
    }
    let size = ui.viewport_size();
    shader(ui, &BACKDROP)
        .absolute(0., 0.)
        .width(size[0])
        .height(size[1])
        .radius(0.)
        .border(0., EDGE)
        .animated(false)
        .paused(true);
}
/// MyGO's text-field recipe: the theme control fill, a hairline border and a
/// 6 px radius. The runtime paints the accent border while the input is focused.
pub fn field_input<'u, 'e, 't>(input: Input<'u, 'e, 't>) -> Input<'u, 'e, 't> {
    input
        .background(Color::Control)
        .border(1., Color::Border)
        .radius(6.)
        .padding_x(10.)
}
/// A solid card over the window wash.
#[track_caller]
pub fn surface<'u, 'e>(ui: &'u mut UiContext<'e>) -> Layout<'u, 'e> {
    column(ui)
        .width_percent(100.)
        .shrink(0.)
        .radius(18.)
        .corner_smoothing(1.)
        .background(Color::Literal(0xffffff))
        .border(1., Color::Literal(0xe7e4ee))
        .shadow(BoxShadow {
            offset_x: 0.,
            offset_y: 6.,
            blur: 18.,
            spread: 0.,
            color: Color::Literal(0x1b1826),
            opacity: 0.06,
        })
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
        (button_base(ui, label).width(32.).min_width(32.).max_width(32.).height(32.).min_height(32.).max_height(32.)
            .shrink(0.).padding(7.).radius(6.).corner_smoothing(1.)
            .disabled(disabled) + |ui|
        {
            glyph(ui, name, 16., ICON);
        }).response.clicked()
    }).inner
}
#[component]
#[rustfmt::skip]
fn footer_tool(ui: &mut UiContext, name: &str, label: &str, disabled: bool) -> bool {
    tooltip(ui, label).children(|ui| {
        (button_base(ui, label).width(26.).height(26.).min_width(26.).min_height(26.).max_width(26.).max_height(26.)
            .shrink(0.).padding(6.).radius(8.).corner_smoothing(1.).border(0., EDGE).disabled(disabled) + |ui|
        {
            glyph(ui, name, 14., ICON);
        }).response.clicked()
    }).inner
}
/// One SSH history record: who asked and which key on the left, a quiet
/// outlined outcome badge on the right, and the relative time below it.
#[component]
#[rustfmt::skip]
fn ssh_history_row(ui: &mut UiContext, record: &AccessRecord, now: i64) {
    let (icon, label, color) = match record.outcome {
        Outcome::Allowed => ("check", "Allowed", MUTED),
        Outcome::Denied => ("close", "Denied", Color::Danger),
        Outcome::TimedOut => ("clock", "Timed out", MUTED),
        Outcome::Locked => ("lock", "Vault locked", MUTED),
        Outcome::Failed => ("close", "Failed", Color::Danger),
    };
    let peer = match record.process_id {
        Some(pid) => format!("PID {pid}"),
        None => "Local process".into(),
    };
    row(ui).width_percent(100.).padding_y(10).gap(16).align(Align::Start) + |ui| {
        column(ui).width(0.).grow(1.).min_width(0.).gap(2) + |ui| {
            text(ui, &record.name).font_size(13).weight(600).width_percent(100.).ellipsis(true);
            text!(ui, "{} · {}", peer, record.fingerprint).font_size(11).color(MUTED)
                .width_percent(100.).ellipsis(true);
        };
        column(ui).shrink(0.).align(Align::End).gap(5) + |ui| {
            row(ui).padding_x(7).padding_y(3).radius(10.).corner_smoothing(0.)
                .border(1., Color::Border).gap(4).align(Align::Center) + |ui|
            {
                glyph(ui, icon, 9., color);
                text(ui, label).font_size(10).weight(600).color(color);
            };
            text(ui, relative_time(now, record.at)).font_size(10).color(MUTED);
        };
    };
}

pub(crate) fn unix_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}

pub(crate) fn relative_time(now: i64, at: i64) -> String {
    let seconds = (now - at).max(0);
    match seconds {
        0..=59 => "Just now".into(),
        60..=3599 => format!("{} min", seconds / 60),
        3600..=86399 => format!("{} hr", seconds / 3600),
        _ => format!("{} d", seconds / 86400),
    }
}

#[component]
#[rustfmt::skip]
pub fn brand(ui: &mut UiContext, item: &Item, size: f32) {
    column(ui).width(size).height(size).shrink(0.).radius(size * 0.25).corner_smoothing(1.)
        .background(Color::Panel).border(1., Color::Literal(0xeae9ee)).elevation(Elevation::SUBTLE)
        .align(Align::Center).justify(Justify::Center) + |ui|
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
    text(ui, name).padding_x(10).padding_y(8).font_size(11).weight(650).color(MUTED);
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
    (plico::widgets::nav_entry(ui, label, selected).height(28.).min_height(28.).max_height(28.)
        .padding_x(8.).padding_y(3.).gap(7.).radius(8.)
        .border(0., EDGE).justify(Justify::Start).disabled(disabled).opacity(if disabled { 0.45 } else { 1. })
        .color(INK)
        .background(Color::Literal(0xffffff)).background_alpha(if selected { 0.62 } else { 0. })
        .hover_background(Color::Literal(0xf1eef6)).transition(160) + |ui|
    {
        glyph(ui, name, 16., ICON);
        text(ui, label).grow(1.).font_size(13);
        if let Some(count) = count {
            text!(ui, "{count}").font_size(12).color(MUTED);
        }
    }).response.clicked()
}
#[component]
#[rustfmt::skip]
fn settings_nav(ui: &mut UiContext, icon: &str, label: &str, selected: bool) -> bool {
    (plico::widgets::nav_entry(ui, label, selected).height(30.).padding_x(10.).padding_y(4.)
        .gap(8.).radius(8.).justify(Justify::Start).border(0., EDGE)
        .color(INK)
        .background(Color::Literal(0xebe8f1)).background_alpha(if selected { 1. } else { 0. })
        .hover_background(Color::Literal(0xf3f1f7)).transition(140) + |ui|
    {
        glyph(ui, icon, 16., if selected { INK } else { MUTED });
        text(ui, label).font_size(13).weight(if selected { 600 } else { 500 })
            .width(0.).grow(1.).min_width(0.).ellipsis(true);
    }).response.clicked()
}

/// A collapsible sidebar section heading: a chevron and the quiet title.
#[component]
#[rustfmt::skip]
fn sidebar_section(ui: &mut UiContext, label: &str, open: bool) -> bool {
    (button_base(ui, label).width_percent(100.).height(28.).min_height(28.).max_height(28.)
        .padding_x(10.).padding_y(3.)
        .gap(6.).radius(6.).justify(Justify::Start).border(0., EDGE)
        .hover_background(Color::Literal(0xf3f1f7)) + |ui| {
        glyph(ui, if open { "chevron-down" } else { "chevron" }, 10., MUTED);
        text(ui, label).font_size(11).weight(650).color(MUTED);
    }).response.clicked()
}
fn sidebar_rows_height(rows: usize) -> f32 {
    if rows == 0 {
        0.
    } else {
        rows as f32 * 28. + (rows as f32 - 1.) * 4.
    }
}

const SIDEBAR_MOTION: std::time::Duration = std::time::Duration::from_millis(180);

fn sidebar_ease_out(t: f32) -> f32 {
    1. - (1. - t).powi(3)
}

#[derive(Clone, Copy)]
struct VaultContentLayout {
    hide_list: bool,
    hide_detail: bool,
    single: bool,
    narrow: bool,
    viewport_width: f32,
    list_width: f32,
    compact: bool,
    content_route: u64,
    content_transition: u16,
}

enum PaletteEntry {
    Command {
        icon: &'static str,
        label: &'static str,
    },
    Item(Box<Item>),
}
struct FolderNode {
    name: String,
    path: String,
    id: Option<String>,
    children: Vec<FolderNode>,
}
struct FolderRow {
    name: String,
    path: String,
    depth: usize,
    has_children: bool,
    open: bool,
    count: usize,
}
use crate::model::folder_path as normalized_folder_path;
fn folder_nodes(folders: &[plicowarden_sdk::service::VaultFolder]) -> Vec<FolderNode> {
    let mut roots = Vec::new();
    for folder in folders {
        let path = normalized_folder_path(&folder.name);
        let parts: Vec<&str> = path.split('/').collect();
        if !parts.is_empty() {
            insert_folder(&parts, "", Some(folder.id.as_str()), &mut roots);
        }
    }
    sort_folders(&mut roots);
    roots
}
fn insert_folder(parts: &[&str], prefix: &str, id: Option<&str>, nodes: &mut Vec<FolderNode>) {
    let Some(head) = parts.first() else { return };
    let path = if prefix.is_empty() {
        (*head).to_owned()
    } else {
        format!("{prefix}/{head}")
    };
    let index = match nodes.iter().position(|node| node.path == path) {
        Some(index) => index,
        None => {
            nodes.push(FolderNode {
                name: (*head).to_owned(),
                path: path.clone(),
                id: None,
                children: Vec::new(),
            });
            nodes.len() - 1
        }
    };
    if parts.len() == 1 {
        nodes[index].id = id.map(str::to_owned);
    } else {
        insert_folder(&parts[1..], &path, id, &mut nodes[index].children);
    }
}
fn sort_folders(nodes: &mut [FolderNode]) {
    nodes.sort_by_key(|node| node.name.to_lowercase());
    for node in nodes {
        sort_folders(&mut node.children);
    }
}
fn folder_count(
    path: &str,
    folders: &[plicowarden_sdk::service::VaultFolder],
    items: &[Item],
) -> usize {
    items
        .iter()
        .filter(|item| {
            !item.deleted
                && !item.archived
                && item.folder.as_deref().is_some_and(|id| {
                    folders
                        .iter()
                        .find(|folder| folder.id == id)
                        .is_some_and(|folder| {
                            let folder_path = normalized_folder_path(&folder.name);
                            folder_path == path || folder_path.starts_with(&format!("{path}/"))
                        })
                })
        })
        .count()
}
fn flatten_folders(
    nodes: &[FolderNode],
    depth: usize,
    closed: &std::collections::BTreeSet<String>,
    folders: &[plicowarden_sdk::service::VaultFolder],
    items: &[Item],
    rows: &mut Vec<FolderRow>,
) {
    for node in nodes {
        let open = !closed.contains(&node.path);
        rows.push(FolderRow {
            name: node.name.clone(),
            path: node.path.clone(),
            depth,
            has_children: !node.children.is_empty(),
            open,
            count: folder_count(&node.path, folders, items),
        });
        if open {
            flatten_folders(&node.children, depth + 1, closed, folders, items, rows);
        }
    }
}
#[component]
#[rustfmt::skip]
fn folder_row(ui: &mut UiContext, row: &FolderRow, selected: bool, disabled: bool) -> bool {
    (plico::widgets::nav_entry(ui, row.path.as_str(), selected).height(28.)
        .min_height(28.).max_height(28.)
        .padding_left(8. + row.depth as f32 * 14.).padding_right(8.).padding_y(3.)
        .gap(7.).radius(8.).border(0., EDGE).justify(Justify::Start).disabled(disabled)
        // Plicowarden uses a quiet light selection face, so its foreground
        // remains dark even while the navigation tree owns keyboard focus.
        .color(INK)
        .background(Color::Literal(0xffffff)).background_alpha(if selected { 0.62 } else { 0. })
        .hover_background(Color::Literal(0xf1eef6)).transition(160) + |ui| {
        // Keep one leading icon lane, like MyGo. A disclosure chevron replaces
        // the folder glyph for branches instead of inserting a second lane
        // that indents every real folder relative to "No folder".
        column(ui).width(16.).height(16.).shrink(0.)
            .align(Align::Center).justify(Justify::Center) + |ui| {
            if row.has_children {
                glyph(ui, if row.open { "chevron-down" } else { "chevron" }, 10., MUTED);
            } else {
                glyph(ui, "folder", 14., MUTED);
            }
        };
        text(ui, &row.name).grow(1.).font_size(13).width(0.).min_width(0.).ellipsis(true);
        if row.count > 0 {
            text!(ui, "{}", row.count).font_size(12).color(MUTED);
        }
    }).response.clicked()
}

fn kind_group_label(kind: &str) -> &'static str {
    match kind {
        "login" => "Logins",
        "card" => "Cards",
        "identity" => "Identities",
        "note" => "Secure notes",
        "ssh-key" => "SSH keys",
        _ => "Other",
    }
}
#[component]
#[rustfmt::skip]
fn notification_row(ui: &mut UiContext, notification: &AppNotification,
    review: &mut Option<u64>, sign_in: &mut bool, copy: &mut Option<String>) {
    let (icon, color) = match notification.kind {
        NotificationKind::Error => ("alert", Color::Danger),
        NotificationKind::SessionExpired => ("lock", Color::Warning),
        NotificationKind::LoginRequest => ("shield", Color::Accent),
    };
    row(ui).width_percent(100.).padding(10.).gap(10.).align(Align::Start)
        .radius(8.).background(Color::Panel) + |ui| {
        column(ui).width(22.).height(22.).shrink(0.).radius(6.).background(Color::Control)
            .align(Align::Center).justify(Justify::Center) + |ui| {
            glyph(ui, icon, 13., color);
        };
        column(ui).width(0.).grow(1.).min_width(0.).gap(3.) + |ui| {
            row(ui).width_percent(100.).gap(6.) + |ui| {
                text(ui, &notification.title).font_size(12).weight(650)
                    .width(0.).grow(1.).ellipsis(true);
                text(ui, relative_time(unix_seconds(), notification.at))
                    .font_size(10).color(MUTED);
            };
            text(ui, &notification.detail).font_size(11).color(MUTED)
                .width_percent(100.).ellipsis(true);
            row(ui).gap(6.).padding_top(3.) + |ui| {
                if notification.kind == NotificationKind::LoginRequest
                    && let Some(id) = notification.approval_id
                    && button(ui).text("Review").ghost().height(24.).clicked()
                {
                    *review = Some(id);
                }
                if notification.kind == NotificationKind::SessionExpired
                    && button(ui).text("Sign in").ghost().height(24.).clicked()
                {
                    *sign_in = true;
                }
                if notification.kind == NotificationKind::Error
                    && button(ui).icon(crate::icons::get("copy")).text("Copy")
                        .ghost().height(24.).clicked()
                {
                    *copy = Some(notification.detail.clone());
                }
            };
        };
    };
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
        let content_route = crate::model::business_key(if self.editor.is_some() {
            "editor"
        } else {
            self.filter.as_str()
        });
        let content_transition = ui.theme().metrics.interaction_ms;
        let compact = ui.viewport_width() < 1120.;
        // Triwarden gives the navigation and list different widths: a narrow
        // sidebar and a wider item list, both compact enough to leave the
        // detail pane the room it needs at the supported minimum size.
        let (navigation_width, list_width) = if compact { (220., 300.) } else { (260., 320.) };
        let viewport_width = ui.viewport_width();
        // Below Triwarden's pair breakpoint the panes switch in place rather
        // than squeezing a third column; at 640 the detail replaces the list.
        let narrow = viewport_width < 900.;
        let single = viewport_width < 640.;
        let hide_list = single && self.compact_detail && !self.compact_sidebar;
        let hide_detail = single && !self.compact_detail && !self.compact_sidebar;
        let detail_visible = !single || (self.compact_detail && !self.compact_sidebar);
        let action_item = (self.authenticated
            && detail_visible
            && !self.settings_open
            && self.editor.is_none()
            && !self.router.show(Page::Generator))
            .then(|| {
                self.items
                    .iter()
                    .find(|item| item.id == self.selected)
                    .cloned()
            })
            .flatten();
        let action_editable = self.details.as_ref().is_some_and(|details| details.editable);
        column(ui).position(plico::Position::Relative).width_percent(100.).height_percent(100.)
            .background(Color::Literal(0xf2f2fa)) + |ui|
        {
            backdrop(ui);
            if self.router.show(Page::Login) {
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
            // The custom window controls keep OS point size, so at UI zoom
            // levels below 100% the header must grow in logical units to keep
            // the panes clear of them.
            let header_height = 48_f32.max(48. / self.zoom.factor() as f32 - 16.);
            column(ui).width_percent(100.).height_percent(100.).padding(8).gap(8) + |ui| {
                row(ui).position(plico::Position::Relative).width_percent(100.).height(header_height)
                    .shrink(0.).padding_x(4).align(Align::Center).gap(12) + |ui|
                {
                    if self.custom_titlebar {
                        column(ui).width((88. / self.zoom.factor()) as f32).shrink(0.).end();
                    }
                    if narrow {
                        if self.compact_sidebar {
                            if tool(ui, "back", "Back", false) {
                                self.compact_sidebar = false;
                            }
                        } else if single && self.compact_detail {
                            if tool(ui, "back", "Back", false) {
                                self.compact_detail = false;
                            }
                        } else if tool(ui, "sidebar", "Toggle sidebar", false) {
                            self.compact_sidebar = true;
                        }
                    } else if tool(ui, "sidebar", "Toggle sidebar", false) {
                        let now = self.frame_time.now;
                        let old_target = if self.sidebar { 1. } else { 0. };
                        let current = self.sidebar_motion_started_at.map_or(old_target, |started| {
                            let progress = (now.saturating_sub(started).as_secs_f32()
                                / SIDEBAR_MOTION.as_secs_f32()).clamp(0., 1.);
                            self.sidebar_motion_from
                                + (old_target - self.sidebar_motion_from) * sidebar_ease_out(progress)
                        });
                        self.sidebar = !self.sidebar;
                        self.sidebar_motion_from = current;
                        self.sidebar_motion_started_at = (!ui.display_preferences().reduce_motion)
                            .then_some(now);
                    }
                    self.window_drag_region(ui, true);
                    if viewport_width >= 1000. {
                        let pill = 340.;
                        let x = (viewport_width * 0.5 - pill * 0.5 - 8.).max(120.);
                        let y = (header_height - 32.) * 0.5;
                        if (button_base(ui, "Search or run a command").absolute(x, y)
                            .width(pill).height(32.).padding_x(12.).gap(8.).radius(16.)
                            .border(1., Color::Literal(0xe2deea))
                            .background(Color::Literal(0xfbfafd))
                            .hover_background(Color::Literal(0xf3f1f7))
                            .justify(Justify::Start) + |ui| {
                            glyph(ui, "search", 14., MUTED);
                            text(ui, "Search or run a command").font_size(12).color(MUTED)
                                .width(0.).grow(1.).min_width(0.).ellipsis(true);
                            text(ui, if cfg!(target_os = "macos") { "⌘K" } else { "Ctrl K" })
                                .font_size(11).color(MUTED);
                        }).response.clicked()
                        {
                            self.palette_query.clear();
                            self.palette_index = 0;
                            self.palette_modal.open();
                            ui.request_rebuild();
                        }
                    } else if tool(ui, "search", "Search or run a command", false) {
                        self.palette_query.clear();
                        self.palette_index = 0;
                        self.palette_modal.open();
                        ui.request_rebuild();
                    }
                    if let Some(item) = &action_item {
                        self.item_actions(ui, item, action_editable);
                    }
                    if self.authenticated {
                        let unread = self.unread_notifications();
                        let description = if unread > 0 {
                            format!("Notifications, {unread} unread")
                        } else {
                            "Notifications".to_owned()
                        };
                        if tooltip(ui, description).children(|ui| {
                            (button_base(ui, "Notifications").width(34.).height(32.).radius(8.)
                                .border(0., EDGE).justify(Justify::Center).gap(3.)
                                .background(Color::Panel).background_alpha(0.)
                                .hover_background(Color::Literal(0xf3f1f7)) + |ui| {
                                glyph(ui, "bell", 15., if unread > 0 { INK } else { MUTED });
                                if unread > 0 {
                                    text!(ui, "{unread}").font_size(10).weight(700).color(Color::Danger);
                                }
                            }).response.clicked()
                        }).inner {
                            self.toggle_notifications();
                        }
                    }
                    let show_global_actions = !(single && self.compact_detail && !self.compact_sidebar);
                    if show_global_actions {
                        if tool(ui, "settings", "Settings", self.busy || self.editor.is_some()) {
                            self.settings_open = true;
                        }
                        if self.authenticated {
                            if tool(ui, "plus", "New item", self.busy || self.editor.is_some()) {
                                self.editor = Some(crate::model::Editor::new());
                                self.settings_open = false;
                            }
                        } else if tool(ui, "login", "Connect account", false) {
                            self.open_login();
                            self.notice.clear();
                        }
                    }
                };
                row(ui).position(plico::Position::Relative).width_percent(100.).height(0.).grow(1.) + |ui| {
                    if narrow && self.compact_sidebar {
                        self.sidebar_ui(ui, (viewport_width - 16.).max(1.), true, false, false, 1.);
                    } else {
                        if !narrow {
                            let elapsed = self.sidebar_motion_started_at
                                .map(|started| self.frame_time.now.saturating_sub(started));
                            let active = !ui.display_preferences().reduce_motion
                                && elapsed.is_some_and(|elapsed| elapsed < SIDEBAR_MOTION);
                            let progress = elapsed.map_or(1., |elapsed| {
                                (elapsed.as_secs_f32() / SIDEBAR_MOTION.as_secs_f32()).clamp(0., 1.)
                            });
                            let progress = sidebar_ease_out(progress);
                            let target = if self.sidebar { 1. } else { 0. };
                            let visibility = if ui.display_preferences().reduce_motion {
                                target
                            } else {
                                self.sidebar_motion_from
                                    + (target - self.sidebar_motion_from) * progress
                            };
                            if active {
                                ui.request_rebuild_at(
                                    self.frame_time.now
                                        .saturating_add(std::time::Duration::from_millis(16)),
                                );
                            } else {
                                self.sidebar_motion_started_at = None;
                                self.sidebar_motion_from = target;
                            }
                            if self.sidebar || active {
                                // Follow MyGo's sidebar motion: animate the width that participates
                                // in layout, then slide a fixed-width panel through a clipped
                                // viewport. The content therefore gains or loses room on every
                                // frame instead of snapping to its final width and being visually
                                // translated over the already-changed layout.
                                let full_extent = navigation_width + 8.;
                                let extent = full_extent * visibility;
                                column(ui).position(plico::Position::Relative)
                                    .width(extent).height_percent(100.).shrink(0.).clip(true) + |ui| {
                                    column(ui).position(plico::Position::Relative)
                                        .width(full_extent).height_percent(100.).shrink(0.)
                                        .translate(extent - full_extent, 0.) + |ui| {
                                        self.sidebar_ui(ui, navigation_width, self.sidebar, false, false, 1.);
                                    };
                                };
                            }
                            self.vault_content(ui, &visible, VaultContentLayout {
                                hide_list, hide_detail, single, narrow, viewport_width, list_width,
                                compact, content_route, content_transition,
                            });
                        } else {
                            self.vault_content(ui, &visible, VaultContentLayout {
                                hide_list, hide_detail, single, narrow, viewport_width, list_width,
                                compact, content_route, content_transition,
                            });
                        }
                    }
                };
            };
            if !self.settings_open && self.editor.is_none() {
                self.vault_footer(ui);
                if self.sync_progress_open && !self.login_progress.steps.is_empty() {
                    self.sync_progress_float(ui);
                }
            }
            self.notifications_panel(ui, header_height);
            if self.settings_open {
                self.settings_modal.open();
            } else {
                self.settings_modal.close();
            }
            let mut settings_state = std::mem::take(&mut self.settings_modal);
            let settings_dismissed = self.settings_modal_ui(ui, &mut settings_state);
            self.settings_modal = settings_state;
            if settings_dismissed {
                self.settings_open = false;
            }
            let mut palette_state = std::mem::take(&mut self.palette_modal);
            self.palette_ui(ui, &mut palette_state);
            self.palette_modal = palette_state;
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
    fn vault_content(&mut self, ui: &mut UiContext, visible: &[Item], layout: VaultContentLayout) {
        let VaultContentLayout { hide_list, hide_detail, single, narrow, viewport_width, list_width,
            compact, content_route, content_transition } = layout;
        column(ui).width(0.).grow(1.).min_width(0.).height_percent(100.)
            .gap(0) + |ui| {
            if self.bulk_mode && !self.settings_open {
                self.bulk_toolbar(ui, visible);
            }
            row(ui).width_percent(100.).height(0.).grow(1.).gap(8) + |ui| {
                if !hide_list {
                    let width = if single { (viewport_width - 16.).max(1.) }
                        else if narrow { 300. } else { list_width };
                    self.item_list(ui, visible, width);
                }
                if hide_detail {
                    column(ui).width(0.).grow(1.).end();
                } else {
                    column(ui).key(content_route).width(0.).grow(1.).min_width(0.)
                        .height_percent(100.).scroll_y(self.editor.is_none())
                        .padding_bottom(if self.editor.is_some() { 0. } else { 38. })
                        .transition(content_transition)
                        .gap(12).padding_x(16.).padding_top(0.) + |ui|
                    {
                        if self.editor.is_some() {
                            self.editor_ui(ui);
                        } else if self.router.show(Page::Generator) {
                            self.generator_ui(ui);
                        } else if visible.is_empty() {
                            column(ui).width_percent(100.).height_percent(100.)
                                .align(Align::Center).justify(Justify::Center)
                                .padding_bottom(60.) + |ui| {
                            surface(ui).padding(32).gap(12) + |ui| {
                                let empty_vault = self.items.is_empty();
                                text(ui, if empty_vault { "Your vault is empty" } else { "No matching items" })
                                    .font_size(20).weight(650);
                                text(ui, if empty_vault {
                                    "Create your first item. It is encrypted before it leaves this device."
                                } else if self.authenticated {
                                    "Create an item, or change your search and filters."
                                } else {
                                    "Try a name, username or website."
                                }).color(MUTED).width_percent(100.);
                                if empty_vault && self.authenticated {
                                    if button(ui).icon(crate::icons::get("plus")).text("Add item")
                                        .primary().disabled(self.busy).clicked()
                                    {
                                        self.editor = Some(crate::model::Editor::new());
                                    }
                                } else {
                                    row(ui).gap(10.).wrap(true) + |ui| {
                                        if !self.query.is_empty()
                                            && button(ui).text("Clear search").ghost().clicked()
                                        {
                                            self.clear_search();
                                            ui.request_rebuild();
                                        }
                                        if self.filter != "All Items"
                                            && button(ui).text("Show all items").ghost().clicked()
                                        {
                                            self.set_filter("All Items");
                                        }
                                    };
                                }
                            };
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
                }
            };
        };
    }
    #[component]
    #[rustfmt::skip]
    fn window_controls(&mut self, ui: &mut UiContext) {
        use plico_renderer::WindowCommand;
        // Window chrome stays at OS point size regardless of content zoom.
        let unit = (1. / self.zoom.factor()) as f32;
        row(ui).absolute(26. * unit, 20. * unit).gap(0).height(24. * unit) + |ui| {
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
                        .border(0., EDGE).align(Align::Center).justify(Justify::Center) + |ui|
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
        button_base(ui, "Move window").width(0.).grow(1.).min_width(32.).height_percent(100.).border(0., EDGE)
            .justify(Justify::Start).align(Align::Center).padding(0).children(|ui| {
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
    /// Triwarden keeps Sync and Lock in a floating capsule at the window's
    /// lower right, clear of the scrolling detail column.
    #[component]
    #[rustfmt::skip]
    fn vault_footer(&mut self, ui: &mut UiContext) {
        let [width, height] = ui.viewport_size();
        row(ui).absolute((width - 118.).max(8.), (height - 46.).max(8.))
            .height(32.).padding_x(4.).gap(4.).radius(16.).corner_smoothing(1.)
            .background(Color::Panel).border(1., Color::Literal(0xe2deea))
            .shadow(BoxShadow {
                offset_x: 0., offset_y: 3., blur: 10., spread: 0.,
                color: Color::Literal(0x241d36), opacity: 0.12,
            })
            .align(Align::Center) + |ui| {
            let syncing = self.login_progress.steps.iter().any(|step| {
                matches!(step.state, crate::login_progress::StepState::Loading
                    | crate::login_progress::StepState::Waiting)
            });
            let sync_clicked = (button_base(ui, "Sync").width(26.).height(26.)
                .min_width(26.).min_height(26.).max_width(26.).max_height(26.)
                .shrink(0.).padding(5.).radius(8.).corner_smoothing(1.)
                .border(0., EDGE).disabled(!self.authenticated || (self.busy && !syncing)) + |ui| {
                if syncing {
                    plico::widgets::spinner_sized(ui, 16.);
                } else {
                    glyph(ui, "sync", 14., ICON);
                }
            }).response.clicked();
            if sync_clicked {
                self.sync_progress_open = !self.sync_progress_open;
                if !syncing {
                    self.request(Request::Sync);
                }
            }
            column(ui).width(1.).height(14.).background(Color::Literal(0xd9d5e2));
            if footer_tool(ui, "lock", "Lock vault", false) {
                self.lock();
            }
        };
    }
    /// The sync progress card floats above the footer; it never takes part in
    /// the vault's layout, so entering the vault leaves no reserved block.
    #[component]
    #[rustfmt::skip]
    fn sync_progress_float(&mut self, ui: &mut UiContext) {
        let [width, height] = ui.viewport_size();
        let mut close = false;
        column(ui).absolute((width - 350.).max(8.), (height - 340.).max(8.))
            .width(330.).max_height(260.).scroll_y(true).padding(14.).gap(8.)
            .radius(14.).corner_smoothing(1.).background(Color::Panel)
            .border(1., Color::Literal(0xe2deea))
            .shadow(BoxShadow {
                offset_x: 0., offset_y: 10., blur: 28., spread: 0.,
                color: Color::Literal(0x241d36), opacity: 0.16,
            }) + |ui| {
            row(ui).width_percent(100.).align(Align::Center).gap(8) + |ui| {
                text(ui, "Sync progress").font_size(13).weight(650).width(0.).grow(1.);
                if (button_base(ui, "Close sync progress").width(24.).height(24.)
                    .min_width(24.).min_height(24.).max_width(24.).max_height(24.)
                    .shrink(0.).padding(5.).radius(6.).border(0., EDGE) + |ui| {
                    glyph(ui, "close", 12., MUTED);
                }).response.clicked()
                {
                    close = true;
                }
            };
            self.progress_steps(ui);
        };
        if close {
            self.sync_progress_open = false;
        }
    }
    #[component]
    #[rustfmt::skip]
    fn sidebar_ui(&mut self, ui: &mut UiContext, width: f32, interactive: bool,
        top_reserved: bool, overlay: bool, visibility: f32) {
        let mut panel = column(ui).position(plico::Position::Relative).width(width).shrink(0.)
            .height_percent(100.).padding(0.).gap(0)
            .radius(18.).corner_smoothing(1.).clip(true).opacity(visibility)
            .translate(-16. * (1. - visibility), 0.)
            .disabled(!interactive)
            .background(Color::Panel)
            .border(1., Color::Literal(0xe7e4ee))
            .shadow(BoxShadow {
                offset_x: 0., offset_y: 8., blur: 24., spread: 0.,
                color: Color::Literal(0x241d36), opacity: 0.07,
            });
        if overlay {
            panel = panel.absolute(0., 0.);
        }
        panel + |ui| {
            if top_reserved {
                row(ui).width_percent(100.)
                    .height(if self.custom_titlebar { 44_f32.max((50. / self.zoom.factor()) as f32) } else { 40. })
                    .shrink(0.).align(Align::Center).gap(10).padding_x(20) + |ui|
                {
                    if !self.custom_titlebar {
                        glyph(ui, "shield", 25., Color::Literal(0x68618d));
                        text(ui, "My workspace").weight(650);
                    }
                };
            }
            // The scroll port reaches the panel edge while its content keeps
            // MyGO's 10-DIP inset. The overlay thumb therefore occupies the
            // quiet outer lane instead of cutting through labels and counts.
            column(ui).width_percent(100.).height(0.).min_height(0.).grow(1.).shrink(1.)
                .scroll_y(true).padding(10.).gap(4) + |ui| {
                let items_height = sidebar_rows_height(7);
                if sidebar_section(ui, "Items", self.sidebar_items_open) {
                    self.sidebar_items_open = !self.sidebar_items_open;
                    ui.request_rebuild();
                }
                column(ui).width_percent(100.).height(if self.sidebar_items_open { items_height } else { 0. })
                    .shrink(0.).clip(true).disabled(!self.sidebar_items_open).gap(4.) + |ui| {
                    for (icon, label, kind) in [
                        ("grid", "All Items", ""),
                        ("star", "Favorites", "*"),
                        ("key", "Logins", "login"),
                        ("card", "Cards", "card"),
                        ("person", "Identities", "identity"),
                        ("note", "Secure Notes", "note"),
                        ("terminal", "SSH Keys", "ssh-key"),
                    ] {
                        let count = self.items.iter().filter(|item| {
                            !item.deleted && !item.archived && (kind.is_empty()
                                || if kind == "*" { item.favorite } else { item.kind == kind })
                        }).count();
                        ui.keyed(label, |ui| {
                            if nav(ui, icon, label, Some(count),
                                self.router.show(Page::Vault) && self.filter == label,
                                self.busy || self.editor.is_some()) {
                                self.set_filter(label);
                            }
                        });
                    }
                };
                if self.authenticated {
                    for (icon, label, archived) in
                        [("note", "Archive", true), ("folder", "Trash", false)]
                    {
                        let count = self.items.iter().filter(|i| {
                            if archived { i.archived && !i.deleted } else { i.deleted }
                        }).count();
                        ui.keyed(label, |ui| {
                            if nav(ui, icon, label, Some(count),
                                self.router.show(Page::Vault) && self.filter == label,
                                self.busy || self.editor.is_some()) {
                                self.set_filter(label);
                            }
                        });
                    }
                }
                let tree = folder_nodes(&self.folders);
                let mut folder_rows = Vec::new();
                flatten_folders(&tree, 0, &self.sidebar_closed_folders, &self.folders,
                    &self.items, &mut folder_rows);
                let folders_height = sidebar_rows_height(1 + folder_rows.len() + 1);
                if sidebar_section(ui, "Folders", self.sidebar_folders_open) {
                    self.sidebar_folders_open = !self.sidebar_folders_open;
                    ui.request_rebuild();
                }
                column(ui).width_percent(100.).height(if self.sidebar_folders_open { folders_height } else { 0. })
                    .shrink(0.).clip(true).disabled(!self.sidebar_folders_open).gap(4.) + |ui| {
                    let unfiled = self.items.iter()
                        .filter(|item| item.folder.is_none() && !item.deleted && !item.archived).count();
                    if nav(ui, "folder", "No folder", Some(unfiled),
                        self.router.show(Page::Vault) && self.filter == "No folder",
                        self.busy || self.editor.is_some()) {
                        self.set_filter("No folder");
                    }
                    let mut toggle = None;
                    let mut choose = None;
                    for row in &folder_rows {
                        let selected = self.router.show(Page::Vault)
                            && self.filter == format!("path:{}", row.path);
                        ui.keyed(crate::model::business_key(&row.path), |ui| {
                            if folder_row(ui, row, selected, self.busy || self.editor.is_some()) {
                                choose = Some(row.path.clone());
                                if row.has_children {
                                    toggle = Some(row.path.clone());
                                }
                            }
                        });
                    }
                    if let Some(path) = toggle {
                        if !self.sidebar_closed_folders.remove(&path) {
                            self.sidebar_closed_folders.insert(path);
                        }
                        ui.request_rebuild();
                    }
                    if let Some(path) = choose {
                        self.set_filter(&format!("path:{path}"));
                    }
                    if (button_base(ui, "Manage folders").width_percent(100.).height(28.)
                        .min_height(28.).max_height(28.).padding_x(10.).padding_y(3.).gap(8.)
                        .radius(8.).justify(Justify::Start).border(0., EDGE)
                        .hover_background(Color::Literal(0xf1eef6))
                        .disabled(!self.authenticated || self.busy || self.editor.is_some()) + |ui| {
                        glyph(ui, "folder", 14., MUTED);
                        text(ui, "Manage folders").font_size(13).color(MUTED).grow(1.);
                    }).response.clicked()
                    {
                        self.folders_modal.open();
                    }
                };
                let utilities_height = sidebar_rows_height(3);
                if sidebar_section(ui, "Utilities", self.sidebar_utilities_open) {
                    self.sidebar_utilities_open = !self.sidebar_utilities_open;
                    ui.request_rebuild();
                }
                column(ui).width_percent(100.).height(if self.sidebar_utilities_open { utilities_height } else { 0. })
                    .shrink(0.).clip(true).disabled(!self.sidebar_utilities_open).gap(4.) + |ui| {
                    if nav(ui, "sync", "Import / Export", None, self.filter == "Import / Export",
                        self.busy || self.editor.is_some()) {
                        self.transfer_modal.open();
                    }
                    if nav(ui, "clock", "One-time Codes", None,
                        self.router.show(Page::Vault) && self.filter == "One-time Codes",
                        !self.authenticated || self.busy || self.editor.is_some()) {
                        self.set_filter("One-time Codes");
                    }
                    if nav(ui, "sliders", "Generator", None, self.router.show(Page::Generator),
                        self.busy || self.editor.is_some()) {
                        self.open_generator();
                    }
                };
            };
            column(ui).width_percent(100.).shrink(0.).padding_x(10.).padding_bottom(10.) + |ui| {
                if (button_base(ui, "Switch account").width_percent(100.).padding(8.).gap(10.)
                    .radius(14.).corner_smoothing(1.).border(1., Color::Literal(0xe7e4ee))
                    .background(Color::Panel).hover_background(Color::Literal(0xf3f1f7))
                    .shadow(BoxShadow {
                        offset_x: 0., offset_y: 2., blur: 8., spread: 0.,
                        color: Color::Literal(0x241d36), opacity: 0.06,
                    }).disabled(self.busy) + |ui| {
                    avatar(ui).fallback("A").size(34.).font_size(13).background(Color::Selection)
                        .color(Color::Literal(0x1d4ed8));
                    column(ui).width(0.).grow(1.).min_width(0.).gap(3) + |ui| {
                        text(
                            ui,
                            if self.authenticated { &self.account_email } else { "alex@example.com" },
                        ).font_size(13).weight(600).width_percent(100.).ellipsis(true);
                        let status = if self.offline {
                            "Cached vault".to_owned()
                        } else if let Some(synced) = self.last_sync_at {
                            format!("Synced {}", relative_time(unix_seconds(), synced))
                        } else if self.authenticated {
                            "Encrypted vault".to_owned()
                        } else {
                            "Demo vault".to_owned()
                        };
                        text(ui, &status).font_size(12).color(MUTED).width_percent(100.);
                    };
                    glyph(ui, "chevron", 12., MUTED);
                }).response.clicked()
                {
                    self.accounts_modal.open();
                }
            };
        };
    }
    #[component]
    #[rustfmt::skip]
    fn item_list(&mut self, ui: &mut UiContext, visible: &[Item], width: f32) {
        column(ui).width(width).shrink(0.).height_percent(100.).radius(18.).corner_smoothing(1.)
            .background(Color::Panel).border(1., Color::Literal(0xe7e4ee)).shadow(BoxShadow {
                offset_x: 0., offset_y: 6., blur: 22., spread: 0.,
                color: Color::Literal(0x241d36), opacity: 0.07,
            }).clip(true) + |ui| {
            row(ui).width_percent(100.).height(48.).shrink(0.).padding_x(8.).align(Align::Center).gap(6) + |ui| {
                column(ui).width(0.).grow(1.) + |ui| {
                    search_field(ui, "Search all vaults", "Search all vaults", &mut self.query);
                };
                if tool(ui, "check", "Bulk selection", self.busy || self.editor.is_some()) {
                    self.bulk_mode = !self.bulk_mode;
                    self.bulk_selected.clear();
                }
                if tool(ui, "sort", "Reverse sort", false) {
                    self.ascending = !self.ascending;
                }
                if tool(ui, "sliders", if self.sort_by_type {
                    "Sort by name"
                } else {
                    "Sort by type"
                }, false) {
                    self.sort_by_type = !self.sort_by_type;
                }
            };
            if self.filter != "All Items" {
                let label = self.filter.strip_prefix("path:").unwrap_or(&self.filter)
                    .split('/').filter(|part| !part.is_empty())
                    .collect::<Vec<_>>().join(" › ");
                row(ui).width_percent(100.).height(28.).shrink(0.).padding_x(10.)
                    .align(Align::Center) + |ui| {
                    if (button_base(ui, "Clear filter").height(22.).width(150.)
                        .padding_x(8.).gap(6.).radius(8.).border(1., Color::Border)
                        .background(Color::Control).hover_background(Color::ControlHover)
                        .justify(Justify::Start) + |ui| {
                        text(ui, &label).font_size(11).weight(600)
                            .width(0.).grow(1.).min_width(0.).ellipsis(true);
                        glyph(ui, "close", 9., MUTED);
                    }).response.clicked()
                    {
                        self.set_filter("All Items");
                    }
                };
            }
            column(ui).key(crate::model::business_key(&format!("list:{}", self.filter)))
                .width_percent(100.).height(0.).grow(1.).scroll_y(true).padding(6).gap(6)
                .transition(180) + |ui| {
                    if self.reveal_list_selection {
                        self.reveal_list_selection = false;
                        let mut top = 6.;
                        let mut last = None;
                        let mut selected_top = None;
                        for item in visible {
                            let group = if self.sort_by_type {
                                kind_group_label(&item.kind).to_owned()
                            } else {
                                item.name.chars().next()
                                    .map(|c| c.to_uppercase().next().unwrap_or(c))
                                    .filter(|c| c.is_alphanumeric()).unwrap_or('#')
                                    .to_string()
                            };
                            if last.as_deref() != Some(group.as_str()) {
                                last = Some(group);
                                top += 20. + 6.;
                            }
                            if item.id == self.selected {
                                selected_top = Some(top);
                            }
                            top += 58. + 6.;
                        }
                        if let Some(selected_top) = selected_top {
                            let needed = ui.scroll_viewport().is_none_or(|[offset, height]| {
                                selected_top < offset + 8.
                                    || selected_top + 58. > offset + height - 8.
                            });
                            if needed {
                                ui.scroll_to_y((selected_top - 84.).max(0.));
                            }
                        }
                    }
                    let mut group = None;
                    let mut group_at = 0usize;
                    for item in visible {
                        let group_label = if self.sort_by_type {
                            kind_group_label(&item.kind).to_owned()
                        } else {
                            item.name.chars().next()
                                .map(|c| c.to_uppercase().next().unwrap_or(c))
                                .filter(|c| c.is_alphanumeric()).unwrap_or('#')
                                .to_string()
                        };
                        if group.as_deref() != Some(group_label.as_str()) {
                            group = Some(group_label.clone());
                            group_at += 1;
                            let key = format!("Group {group_at} {group_label}");
                            ui.keyed(crate::model::business_key(&key), |ui| {
                                row(ui).width_percent(100.).height(20.).shrink(0.).padding_x(8)
                                    .align(Align::Center) + |ui| {
                                    text(ui, &group_label).font_size(11).weight(650).color(MUTED);
                                };
                            });
                        }
                        ui.keyed(crate::model::business_key(&item.id), |ui| {
                            let selected = self.selected == item.id;
                            let mut clicked = false;
                            row(ui).width_percent(100.).height(58.).shrink(0.).gap(6).align(Align::Center) + |ui| {
                                if self.bulk_mode {
                                    let checked = self.bulk_selected.contains(&item.id);
                                    if (button_base(
                                        ui,
                                        format!("Toggle bulk selection for {}", item.name),
                                    ).width(22.).height(22.).radius(6.).padding(2).border(1., Color::Border)
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
                                        .min_width(0.).height(56.).padding(9.).gap(12.).justify(Justify::Start)
                                        .radius(13.).corner_smoothing(1.).border(0., EDGE)
                                        .background(Color::Literal(0xffffff))
                                        .background_alpha(if selected { 1. } else { 0. })
                                        .hover_background(Color::Literal(0xf4f1f8))
                                        .elevation(if selected { Elevation::CONTROL } else { Elevation::NONE })
                                        .disabled(self.busy || self.editor.is_some()) + |ui|
                                    {
                                        brand(ui, item, 38.);
                                        column(ui).width(0.).grow(1.).min_width(0.).gap(1) + |ui| {
                                            text(ui, &item.name).weight(700).font_size(14)
                                                .width_percent(100.).ellipsis(true);
                                            text(ui, &item.user).font_size(12).color(MUTED)
                                                .width_percent(100.).ellipsis(true);
                                        };
                                        if item.favorite || item.has_totp {
                                            row(ui).shrink(0.).gap(4.).align(Align::Center).padding_x(2.) + |ui| {
                                                if item.favorite {
                                                    glyph(ui, "star", 12., Color::Literal(0xc39831));
                                                }
                                                if item.has_totp {
                                                    glyph(ui, "clock", 12., MUTED);
                                                }
                                            };
                                        }
                                    }).response.clicked();
                            };
                            if clicked {
                                self.select(&item.id);
                                if ui.viewport_width() < 640. {
                                    self.compact_detail = true;
                                    ui.request_rebuild();
                                }
                            }
                        });
                    }
                };
            };
    }
    /// Triwarden's command palette: a floating search sheet for vault items
    /// and the few global actions that do not need a trip to Settings. Arrow
    /// keys move the highlight, Enter runs the highlighted entry.
    #[component]
    #[rustfmt::skip]
    fn palette_ui(&mut self, ui: &mut UiContext, state: &mut plico::ModalState) {
        let query = self.palette_query.trim().to_lowercase();
        let mut entries: Vec<PaletteEntry> = [
            ("plus", "New item"),
            ("sliders", "Generator"),
            ("settings", "Settings"),
            ("sync", "Sync now"),
            ("lock", "Lock vault"),
            ("person", "Accounts"),
        ].into_iter()
            .filter(|(_, label)| query.is_empty() || label.to_lowercase().contains(&query))
            .map(|(icon, label)| PaletteEntry::Command { icon, label })
            .collect();
        entries.extend(
            self.items.iter()
                .filter(|item| !item.deleted && !item.archived)
                .filter(|item| {
                    query.is_empty()
                        || item.name.to_lowercase().contains(&query)
                        || item.user.to_lowercase().contains(&query)
                        || item.site.to_lowercase().contains(&query)
                })
                .take(8)
                .cloned()
                .map(|item| PaletteEntry::Item(Box::new(item))),
        );
        let len = entries.len();
        self.palette_index = self.palette_index.min(len.saturating_sub(1));
        let mut activate = false;
        let mut cancel = false;
        let response = modal(ui, state)
            .header(false)
            .title("Search")
            .width(560.)
            .motion(plico::Motion::None)
            .children(|ui| {
            let mut scroll_to = None;
            column(ui).width_percent(100.).gap(10.) + |ui| {
                let response = field_input(text_input_string(ui, "Search vault or commands",
                    &mut self.palette_query))
                    .navigation(true)
                    .autofocus(true).width_percent(100.)
                    .placeholder("Search items or commands")
                    .response();
                if response.changed {
                    self.palette_index = 0;
                }
                if let Some(navigation) = response.navigation {
                    let step = match navigation {
                        InputNavigation::Previous => -1,
                        InputNavigation::Next => 1,
                        InputNavigation::PageUp => -5,
                        InputNavigation::PageDown => 5,
                        InputNavigation::Cancel => 0,
                    };
                    if matches!(navigation, InputNavigation::Cancel) {
                        cancel = true;
                    } else {
                        self.palette_index = (self.palette_index as i32 + step)
                            .clamp(0, len.saturating_sub(1) as i32) as usize;
                        // Keep the keyboard highlight visible in the scroll
                        // sheet: row heights differ between commands and items.
                        let commands = entries.iter()
                            .filter(|entry| matches!(entry, PaletteEntry::Command { .. }))
                            .count();
                        let index = self.palette_index;
                        let top = if index < commands {
                            54. + index as f32 * 44.
                        } else {
                            54. + commands as f32 * 44. + 24.
                                + (index - commands) as f32 * 54.
                        };
                        scroll_to = Some((top - 48.).max(0.));
                    }
                }
                if response.submitted {
                    activate = true;
                }
                column(ui).width_percent(100.).max_height(330.).scroll_y(true).gap(10.) + |ui| {
                if let Some(offset) = scroll_to {
                    ui.scroll_to_y(offset);
                }
                let highlighted = self.palette_index;
                let mut command_at = 0;
                let mut item_at = 0;
                if entries.iter().any(|entry| matches!(entry, PaletteEntry::Command { .. })) {
                    text(ui, "COMMANDS").font_size(10).weight(650).color(MUTED)
                        .padding_x(6.).width_percent(100.);
                }
                for (index, entry) in entries.iter().enumerate() {
                    let PaletteEntry::Command { icon, label } = entry else { continue };
                    let selected = index == highlighted;
                    command_at += 1;
                    ui.keyed(crate::model::business_key(&format!("command {command_at}")), |ui| {
                        if (button_base(ui, *label).width_percent(100.).height(34.)
                            .padding_x(8.).gap(8.).radius(8.).border(0., EDGE)
                            .justify(Justify::Start)
                            .background(Color::Literal(0xebe8f1))
                            .background_alpha(if selected { 1. } else { 0. })
                            .hover_background(Color::Literal(0xf3f1f7)) + |ui| {
                            glyph(ui, icon, 15., MUTED);
                            text(ui, *label).font_size(13).width(0.).grow(1.).min_width(0.);
                        }).response.clicked()
                        {
                            self.palette_index = index;
                            activate = true;
                        }
                    });
                }
                if entries.iter().any(|entry| matches!(entry, PaletteEntry::Item(_))) {
                    text(ui, "ITEMS").font_size(10).weight(650).color(MUTED)
                        .padding_x(6.).width_percent(100.);
                }
                for (index, entry) in entries.iter().enumerate() {
                    let PaletteEntry::Item(item) = entry else { continue };
                    let selected = index == highlighted;
                    item_at += 1;
                    let label = format!("Open {}", item.name);
                    ui.keyed(crate::model::business_key(&format!("palette {item_at}")), |ui| {
                        if (button_base(ui, &label).width_percent(100.).height(44.)
                            .padding_x(8.).gap(10.).radius(8.).border(0., EDGE)
                            .justify(Justify::Start)
                            .background(Color::Literal(0xebe8f1))
                            .background_alpha(if selected { 1. } else { 0. })
                            .hover_background(Color::Literal(0xf3f1f7)) + |ui| {
                            brand(ui, item, 30.);
                            column(ui).width(0.).grow(1.).min_width(0.).gap(1.) + |ui| {
                                text(ui, &item.name).font_size(13).weight(650)
                                    .width_percent(100.).ellipsis(true);
                                text(ui, &item.user).font_size(11).color(MUTED)
                                    .width_percent(100.).ellipsis(true);
                            };
                        }).response.clicked()
                        {
                            self.palette_index = index;
                            activate = true;
                        }
                    });
                }
                if entries.is_empty() {
                    text(ui, "No matches").font_size(13).color(MUTED)
                        .padding(12.).width_percent(100.);
                }
                };
            };
        });
        if response.dismissed || cancel {
            state.close();
        }
        if activate && let Some(entry) = entries.get(self.palette_index) {
            match entry {
                PaletteEntry::Command { label, .. } => match *label {
                    "New item" => {
                        self.editor = Some(crate::model::Editor::new());
                        self.settings_open = false;
                    }
                    "Generator" => self.open_generator(),
                    "Settings" => self.settings_open = true,
                    "Sync now" => {
                        if self.authenticated {
                            self.request(Request::Sync);
                        }
                    }
                    "Lock vault" => self.lock(),
                    "Accounts" => self.accounts_modal.open(),
                    _ => {}
                },
                PaletteEntry::Item(item) => {
                    let id = item.id.clone();
                    self.set_filter("All Items");
                    self.select(&id);
                    self.reveal_list_selection = true;
                    if ui.viewport_width() < 640. {
                        self.compact_detail = true;
                    }
                }
            }
            state.close();
            ui.request_rebuild();
        }
    }
    #[component]
    #[rustfmt::skip]
    fn notifications_panel(&mut self, ui: &mut UiContext, header_height: f32) {
        if !self.notification_open {
            return;
        }
        let [viewport_width, viewport_height] = ui.viewport_size();
        let panel_width = 364.;
        let panel_height = (viewport_height - header_height - 28.).clamp(220., 460.);
        let mut review = None;
        let mut sign_in = false;
        let mut close = false;
        let mut copy = None;
        if (button_base(ui, "Dismiss notifications").absolute(0., 0.)
            .width(viewport_width).height(viewport_height).radius(0.)
            .border(0., EDGE).background(Color::Literal(0x000000)).background_alpha(0.12)
            .hover_background(Color::Literal(0x000000)) + |_ui| {}).response.clicked()
        {
            self.notification_open = false;
        }
        column(ui).absolute((viewport_width - panel_width - 8.).max(8.), header_height + 10.)
            .width(panel_width).height(panel_height)
            .background(Color::Panel).border(1., Color::Literal(0xe7e4ee)).radius(14.)
            .shadow(BoxShadow {
                offset_x: 0., offset_y: 10., blur: 28., spread: 0.,
                color: Color::Literal(0x241d36), opacity: 0.14,
            }) + |ui| {
            row(ui).width_percent(100.).height(44.).shrink(0.).padding_x(14.)
                .align(Align::Center).gap(8.) + |ui| {
                text(ui, "Notifications").font_size(15).weight(700);
                column(ui).width(0.).grow(1.);
                if !self.notifications.is_empty()
                    && button(ui).text("Clear read").ghost().height(26.).clicked()
                {
                    self.clear_read_notifications();
                }
                if tool(ui, "close", "Close notifications", false) {
                    close = true;
                }
            };
            separator(ui);
            if self.notifications.is_empty() {
                column(ui).width_percent(100.).height(0.).grow(1.).align(Align::Center)
                    .justify(Justify::Center).gap(6.).padding(24.) + |ui| {
                    glyph(ui, "bell", 22., MUTED);
                    text(ui, "No notifications").font_size(14).weight(600);
                    text(ui, "Errors and device signing requests appear here.")
                        .font_size(12).color(MUTED).width_percent(100.);
                };
            } else {
                column(ui).width_percent(100.).height(0.).grow(1.).scroll_y(true)
                    .padding(8.).gap(2.) + |ui| {
                    let notifications = self.notifications.clone();
                    for (index, notification) in notifications.iter().enumerate() {
                        ui.keyed(notification.id, |ui| {
                            notification_row(ui, notification, &mut review, &mut sign_in, &mut copy);
                        });
                        if index + 1 < notifications.len() {
                            ui.keyed(index as u64 + 1_000_000, |ui| separator(ui));
                        }
                    }
                };
            }
        };
        if close {
            self.notification_open = false;
        }
        if let Some(id) = review {
            self.notification_open = false;
            if self.ssh_approvals.iter().any(|approval| approval.id == id) {
                self.ssh_modal.open();
            }
            ui.request_rebuild();
        }
        if let Some(detail) = copy {
            self.copy_text(zeroize::Zeroizing::new(detail));
        }
        if sign_in {
            self.notification_open = false;
            self.expire_session();
            ui.request_rebuild();
        }
    }
    /// Settings opens over the vault as a compact sheet with its own section
    /// sidebar, like Triwarden's System Settings layout, instead of replacing
    /// the detail column.
    #[component]
    #[rustfmt::skip]
    fn settings_modal_ui(&mut self, ui: &mut UiContext, state: &mut plico::ModalState) -> bool {
        let [viewport_width, viewport_height] = ui.viewport_size();
        let width = (viewport_width - 48.).clamp(560., 840.);
        // Short panes stay a compact sheet; the SSH pane uses the extra room
        // for history. A fixed tall sheet left a large blank area under
        // Appearance.
        let preferred = match self.settings_tab.as_str() {
            "ssh" => 500.,
            "account" => 640.,
            "security" => 390.,
            _ => 360.,
        };
        let height = (viewport_height - 160.).clamp(320., preferred);
        // Settings is a full sheet of controls; an enter animation over that
        // much content reads as a stutter. It opens and closes in one step,
        // like a system settings window.
        let response = modal(ui, state)
            .header(false)
            .title("Settings")
            .width(width)
            .motion(plico::Motion::None)
            .children(|ui| {
            column(ui).width_percent(100.).height(height).gap(0) + |ui| {
                row(ui).width_percent(100.).height(48.).shrink(0.).align(Align::Center).gap(12) + |ui| {
                    text(ui, "Settings").font_size(20).weight(700);
                    column(ui).width(0.).grow(1.).end();
                    if button(ui).icon(crate::icons::get("close")).text("Close settings").ghost()
                        .clicked()
                    {
                        self.settings_open = false;
                    }
                };
                column(ui).width_percent(100.).height(1.).shrink(0.)
                    .background(Color::Literal(0xe7e4ee));
                row(ui).width_percent(100.).height(0.).grow(1.).gap(0) + |ui| {
                    column(ui).width(178.).shrink(0.).height_percent(100.).padding_x(8.).padding_y(12.).gap(2) + |ui| {
                        if settings_nav(ui, "sliders", "Appearance", self.settings_tab == "appearance") {
                            self.settings_tab = "appearance".into();
                            ui.request_rebuild();
                        }
                        if settings_nav(ui, "shield", "Security", self.settings_tab == "security") {
                            self.settings_tab = "security".into();
                            ui.request_rebuild();
                        }
                        if settings_nav(ui, "terminal", "SSH agent", self.settings_tab == "ssh") {
                            self.settings_tab = "ssh".into();
                            ui.request_rebuild();
                        }
                        if settings_nav(ui, "person", "Account", self.settings_tab == "account") {
                            self.settings_tab = "account".into();
                            ui.request_rebuild();
                        }
                    };
                    column(ui).width(1.).height_percent(100.).shrink(0.)
                        .background(Color::Literal(0xe7e4ee));
                    column(ui).width(0.).grow(1.).min_width(0.).height_percent(100.)
                        .scroll_y(true).padding(20.).gap(14) + |ui| {
                        let title = match self.settings_tab.as_str() {
                            "security" => "Security",
                            "ssh" => "SSH agent",
                            "account" => "Account",
                            _ => "Appearance",
                        };
                        text(ui, title).font_size(22).weight(700).width_percent(100.);
                        match self.settings_tab.as_str() {
                            "account" => self.account_settings(ui),
                            "security" => self.security_ui(ui),
                            "ssh" => {
                                text(ui, "Sign with vault SSH keys from OpenSSH and Git.")
                                    .font_size(13).color(MUTED).width_percent(100.);
                                self.ssh_settings_ui(ui);
                            }
                            _ => self.appearance_settings(ui),
                        }
                    };
                };
            };
        });
        if response.dismissed {
            return true;
        }
        false
    }
    #[component]
    #[rustfmt::skip]
    fn account_settings(&mut self, ui: &mut UiContext) {
        let email = if self.account_email.is_empty() {
            "alex@example.com".to_owned()
        } else {
            self.account_email.clone()
        };
        let initial = email.chars().next().map(|c| c.to_uppercase().next().unwrap_or(c))
            .unwrap_or('A');
        surface(ui).padding(18.).gap(14.) + |ui| {
            row(ui).width_percent(100.).align(Align::Center).gap(12.) + |ui| {
                column(ui).width(42.).height(42.).radius(21.).shrink(0.)
                    .background(Color::Accent).align(Align::Center).justify(Justify::Center) + |ui| {
                    text!(ui, "{initial}").font_size(18).weight(700).color(Color::OnAccent);
                };
                column(ui).width(0.).grow(1.).min_width(0.).gap(2.) + |ui| {
                    text(ui, &email).font_size(15).weight(650).width_percent(100.).ellipsis(true);
                    text(ui, if self.authenticated { "Encrypted vault" } else { "Demo vault" })
                        .font_size(12).color(MUTED);
                };
                if tool(ui, "copy", "Copy email", self.busy) {
                    self.copy_text(zeroize::Zeroizing::new(email.clone()));
                }
            };
            separator(ui);
            row(ui).width_percent(100.).gap(10.).wrap(true) + |ui| {
                if button(ui).icon(crate::icons::get("sync")).text("Sync now").ghost()
                    .disabled(!self.authenticated || self.busy).clicked()
                {
                    self.request(Request::Sync);
                }
                if button(ui).icon(crate::icons::get("person")).text("Manage accounts").ghost()
                    .disabled(self.busy).clicked()
                {
                    self.accounts_modal.open();
                }
            };
        };
        surface(ui).padding(18.).gap(10.) + |ui| {
            text(ui, "Connection").font_size(13).weight(650);
            let server = if self.region == "self-hosted" {
                self.server.clone()
            } else if self.region == "eu" {
                "https://bitwarden.eu".to_owned()
            } else {
                "https://bitwarden.com".to_owned()
            };
            let synced = self.last_sync_at
                .map(|at| format!("Synced {}", relative_time(unix_seconds(), at)))
                .unwrap_or_else(|| "Not yet".to_owned());
            for (label, value) in [
                ("Server", server.as_str()),
                ("Region", match self.region.as_str() {
                    "eu" => "Bitwarden EU",
                    "self-hosted" => "Self-hosted",
                    _ => "Bitwarden US",
                }),
                ("Status", if self.authenticated { "Unlocked" } else { "Demo vault" }),
                ("Last sync", synced.as_str()),
            ] {
                ui.keyed(label, |ui| {
                    row(ui).width_percent(100.).padding_y(4.).align(Align::Center).gap(10.) + |ui| {
                        text(ui, label).font_size(13).color(MUTED);
                        column(ui).width(0.).grow(1.);
                        text(ui, value).font_size(13).ellipsis(true);
                        if label == "Server" && tool(ui, "copy", "Copy server URL", self.busy) {
                            self.copy_text(zeroize::Zeroizing::new(value.to_owned()));
                        }
                    };
                });
            }
            text(ui, "Change the server from the lock screen.").font_size(12).color(MUTED)
                .width_percent(100.);
        };
        surface(ui).padding(18.).gap(10.) + |ui| {
            text(ui, "Session").font_size(13).weight(650);
            text!(ui, "Auto-lock after {} minutes while the app is idle.",
                self.active_security.auto_lock_minutes).font_size(12).color(MUTED)
                .width_percent(100.);
            row(ui).width_percent(100.).gap(10.).wrap(true) + |ui| {
                if button(ui).icon(crate::icons::get("lock")).text("Lock vault").ghost()
                    .disabled(!self.authenticated || self.busy).clicked()
                {
                    self.lock();
                }
            };
        };
    }
    #[component]
    #[rustfmt::skip]
    fn appearance_settings(&mut self, ui: &mut UiContext) {
        surface(ui).padding(18.).gap(12.) + |ui| {
            text(ui, "Interface scale").font_size(13).weight(600);
            let before = self.appearance.ui_scale;
            slider(ui, "Interface scale", &mut self.appearance.ui_scale, 0.5..=3.0).step(0.05).width_percent(100.);
            text!(ui, "Interface scale: {:.0}%", self.appearance.ui_scale * 100.)
                .font_size(12).color(MUTED).width_percent(100.);
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
            row(ui).width_percent(100.).gap(10.).wrap(true) + |ui| {
                if button(ui).icon(crate::icons::get("save")).text("Save appearance").primary()
                    .disabled(self.settings_saving).clicked()
                {
                    self.settings_saving = true;
                    self.commands.push(HostCommand::SaveAppearance(self.appearance.clone()));
                }
                if button(ui).icon(crate::icons::get("restore")).text("Reset appearance").ghost().clicked() {
                    self.appearance = Default::default();
                    self.zoom.set(self.appearance.ui_scale);
                    ui.request_rebuild();
                    self.notice = "Defaults restored. Save to keep them.".into();
                }
            };
        };
        text(ui, "Panels, rows and controls keep one compact scale across the vault, forms and settings.")
            .font_size(13).color(MUTED).width_percent(100.);
    }
    #[component]
    #[rustfmt::skip]
    fn ssh_settings_ui(&mut self, ui: &mut UiContext) {
        row(ui).width_percent(100.).wrap(true).align(Align::Center)
            .justify(Justify::End).gap(12) + |ui|
        {
            if segmented(ui, &mut self.ssh_view, &["Permissions", "History"]) {
                ui.request_rebuild();
            }
        };
        if self.ssh_view == crate::model::SSH_HISTORY {
            self.ssh_history_ui(ui);
            return;
        }
        let enabled = self.ssh_socket.is_some();
        surface(ui).padding(16).gap(12) + |ui| {
            row(ui).width_percent(100.).gap(12).align(Align::Center) + |ui| {
                column(ui).width(34.).height(34.).shrink(0.).radius(10.).background(Color::Control)
                    .align(Align::Center).justify(Justify::Center) + |ui|
                {
                    glyph(ui, "key", 16., if enabled { INK } else { MUTED });
                };
                column(ui).width(0.).grow(1.).min_width(0.).gap(2) + |ui| {
                    text(ui, if enabled { "Agent enabled" } else { "Agent off" })
                        .font_size(13).weight(600).width_percent(100.);
                    text(ui, if enabled {
                        "Local tools can ask this vault to sign. Each request waits for your decision for 30 seconds."
                    } else {
                        "Enable the agent to let local tools ask this vault to sign."
                    }).font_size(12).color(MUTED).width_percent(100.);
                };
            };
            if let Some(socket) = self.ssh_socket.clone() {
                column(ui).width_percent(100.).gap(4).padding(10).radius(8.)
                    .background(Color::Control) + |ui|
                {
                    text(ui, "Socket · set SSH_AUTH_SOCK to this path").font_size(12)
                        .color(MUTED).width_percent(100.);
                    row(ui).width_percent(100.).align(Align::Center).gap(8.) + |ui| {
                        text(ui, &socket).font_size(12).width(0.).grow(1.).min_width(0.).ellipsis(true);
                        if tool(ui, "copy", "Copy socket path", self.busy) {
                            self.copy_text(zeroize::Zeroizing::new(socket.clone()));
                        }
                    };
                };
            }
            if button(ui).icon(crate::icons::get(if enabled { "power" } else { "terminal" }))
                .text(if enabled { "Disable SSH agent" } else { "Enable SSH agent" })
                .disabled(self.busy || (!self.authenticated && !enabled) || !cfg!(unix))
                .clicked()
            {
                self.request(Request::SshAgent(!enabled));
            }
            text(ui, "Private keys stay in the unlocked vault. Locking or disabling the agent denies any request still waiting.")
                .font_size(12).color(MUTED).width_percent(100.);
            if !cfg!(unix) {
                text(ui, "SSH agent currently requires a Unix socket; Windows support is pending.")
                    .font_size(12).color(MUTED).width_percent(100.);
            } else if !self.authenticated {
                text(ui, "Connect your account to enable SSH agent.").font_size(12)
                    .color(MUTED).width_percent(100.);
            }
        };
        surface(ui).padding(20).gap(6) + |ui| {
            text(ui, "Nothing has lasting access").font_size(14).weight(650)
                .width_percent(100.);
            text(ui, "Every signature is approved once. No app receives standing access, so there are no grants to end here.")
                .font_size(12).color(MUTED).width_percent(100.);
            text(ui, "Finished requests appear under History with the key, the requesting process and the outcome.")
                .font_size(12).color(MUTED).width_percent(100.);
        };
    }
    #[component]
    #[rustfmt::skip]
    fn ssh_history_ui(&mut self, ui: &mut UiContext) {
        let now = unix_seconds();
        row(ui).width_percent(100.).gap(8).wrap(true).align(Align::Center) + |ui| {
            for (label, filter) in [
                ("All results", crate::model::SshHistoryFilter::All),
                ("Allowed", crate::model::SshHistoryFilter::Allowed),
                ("Not allowed", crate::model::SshHistoryFilter::NotAllowed),
            ] {
                ui.keyed(label, |ui| {
                    if choice(ui, label, &mut self.ssh_history_filter, filter).clicked() {
                        ui.request_rebuild();
                    }
                });
            }
            column(ui).grow(1.);
            if !self.ssh_history.is_empty()
                && button(ui).icon(crate::icons::get("trash")).text("Clear history").ghost().clicked()
            {
                self.clear_ssh_history();
            }
        };
        let records: Vec<_> = self.ssh_history.iter()
            .filter(|record| self.ssh_history_filter.includes(record.outcome))
            .cloned().collect();
        if records.is_empty() {
            surface(ui).padding(28).gap(8) + |ui| {
                text(ui, if self.ssh_history.is_empty() {
                    "No SSH requests yet"
                } else {
                    "No matching requests"
                }).font_size(15).weight(650);
                text(ui, if self.ssh_history.is_empty() {
                    "Each time an app asks to sign with an SSH key, the request and its outcome appear here."
                } else {
                    "Try another result filter."
                }).font_size(13).color(MUTED).width_percent(100.);
            };
            return;
        }
        surface(ui).padding_x(14).padding_y(4) + |ui| {
            for (index, record) in records.iter().enumerate() {
                ui.keyed(record.id, |ui| {
                    ssh_history_row(ui, record, now);
                    if index + 1 < records.len() {
                        separator(ui);
                    }
                });
            }
        };
        text(ui, "Last 200 signing requests. Key listings and signatures are not recorded.")
            .font_size(12).color(MUTED).width_percent(100.);
    }
    #[component]
    #[rustfmt::skip]
    fn generator_ui(&mut self, ui: &mut UiContext) {
        if self.generated.is_empty() {
            self.generate();
            ui.request_rebuild();
        }
        let (generate_label, copy_label, value_label) = match self.generator_mode {
            crate::generator::Mode::Password => ("Generate password", "Copy generated password", "Generated password"),
            crate::generator::Mode::Passphrase => ("Generate passphrase", "Copy generated value", "Generated passphrase"),
            crate::generator::Mode::Username => ("Generate username", "Copy generated value", "Generated username"),
        };
        column(ui).width_percent(100.).gap(16) + |ui| {
            surface(ui).padding(20.).gap(12.) + |ui| {
                text(ui, "Generator").font_size(24).weight(700);
                text(ui, "Generated on this device using the operating system’s cryptographic random source.")
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
            };
            surface(ui).padding(20.).gap(12.) + |ui| {
                text(ui, value_label).font_size(12).color(MUTED);
                if self.generated.is_empty() {
                    text(ui, "Generate a value to see it here.").font_size(18).color(MUTED)
                        .width_percent(100.);
                } else {
                    text(ui, self.generated.as_str()).font_size(24).weight(650)
                        .width_percent(100.);
                }
                row(ui).width_percent(100.).gap(10.).wrap(true) + |ui| {
                    if button(ui).icon(crate::icons::get("sync")).text(generate_label)
                        .primary().clicked()
                    {
                        self.generate();
                        ui.request_rebuild();
                    }
                    if button(ui).icon(crate::icons::get("copy")).text(copy_label)
                        .ghost().disabled(self.generated.is_empty() || self.busy).clicked()
                    {
                        self.copy_text(self.generated.clone());
                    }
                };
            };
            row(ui).width_percent(100.).gap(12.).align(Align::Start).wrap(true) + |ui| {
                surface(ui).width(0.).grow(1.).min_width(260.).padding(18.).gap(12.) + |ui| {
                    text(ui, "Options").font_size(13).weight(650);
                    match self.generator_mode {
                        crate::generator::Mode::Password => {
                            slider(ui, "Password length", &mut self.length, 8.0..=128.0)
                                .step(1.).width_percent(100.);
                            text!(ui, "{} characters", self.length.round() as usize).color(MUTED);
                            switch(ui, "Include symbols", &mut self.symbols);
                        }
                        crate::generator::Mode::Passphrase => {
                            slider(ui, "Word count", &mut self.word_count, 6.0..=12.0)
                                .step(1.).width_percent(100.);
                            text!(ui, "{} independently selected EFF words",
                                self.word_count.round() as usize).color(MUTED);
                        }
                        crate::generator::Mode::Username => {
                            text(ui, "Random username · availability is not checked · not a password")
                                .color(MUTED).width_percent(100.);
                        }
                    }
                };
                if !self.history.is_empty() {
                    surface(ui).width(300.).shrink(0.).max_height(260.).scroll_y(true)
                        .padding(18.).gap(10.) + |ui| {
                        text(ui, "History").font_size(13).weight(650);
                        let history: Vec<_> = self.history.iter().cloned().collect();
                        for (index, value) in history.iter().enumerate() {
                            ui.keyed(index as u64, |ui| {
                                row(ui).width_percent(100.).align(Align::Center).gap(6.) + |ui| {
                                    text(ui, value.as_str()).font_size(12).width(0.).grow(1.)
                                        .min_width(0.).ellipsis(true);
                                    if tool(ui, "copy", "Copy history value", self.busy) {
                                        self.copy_text(value.clone());
                                    }
                                };
                            });
                        }
                    };
                }
            };
            text(ui, "Recent generated values stay in memory until you lock or close the app.")
                .font_size(12).color(MUTED).width_percent(100.);
        };
    }
}
