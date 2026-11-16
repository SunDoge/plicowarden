use crate::{appearance::Appearance, generator};
use plico::{ModalState, TextBuffer};
use plicowarden_sdk::{Reply, Request, service::*};
use std::collections::{BTreeMap, HashMap, VecDeque};
use zeroize::{Zeroize, Zeroizing};

#[derive(Clone)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub user: String,
    pub site: String,
    pub icon_url: Option<String>,
    pub icon: Option<plico::ImageAsset>,
    pub mark: String,
    pub color: u32,
    pub favorite: bool,
    pub kind: String,
    pub folder: Option<String>,
    pub deleted: bool,
    pub archived: bool,
    pub has_totp: bool,
    pub has_passkey: bool,
}
pub fn demo_items() -> Vec<Item> {
    [
        (
            "airbnb",
            "Airbnb",
            "alex.chen@example.com",
            "airbnb.com",
            "A",
            0xe85a6b,
            true,
            "login",
        ),
        (
            "alarm",
            "Alarm system code",
            "Home",
            "",
            "A",
            0x82858c,
            false,
            "note",
        ),
        (
            "alex",
            "Alex Chen",
            "Personal identity",
            "",
            "A",
            0x89858e,
            false,
            "identity",
        ),
        (
            "aliexpress",
            "AliExpress",
            "alex.chen@example.com",
            "aliexpress.com",
            "Ali",
            0xf86734,
            false,
            "login",
        ),
        (
            "amazon",
            "Amazon",
            "alex.chen@example.com",
            "amazon.com",
            "a",
            0xe89d35,
            true,
            "login",
        ),
        (
            "amex",
            "American Express",
            "alexchen88",
            "americanexpress.com",
            "AM",
            0x2462a1,
            false,
            "card",
        ),
        (
            "apple",
            "Apple account",
            "alex.chen@example.com",
            "apple.com",
            "A",
            0x353844,
            false,
            "login",
        ),
        (
            "figma",
            "Figma",
            "alex.chen@example.com",
            "figma.com",
            "F",
            0x9f64df,
            false,
            "login",
        ),
        (
            "github",
            "GitHub",
            "alexchen",
            "github.com",
            "G",
            0x24262c,
            true,
            "login",
        ),
        (
            "linear",
            "Linear",
            "alex.chen@example.com",
            "linear.app",
            "L",
            0x6761d9,
            true,
            "login",
        ),
        (
            "notion",
            "Notion",
            "alex.chen@example.com",
            "notion.so",
            "N",
            0x26262b,
            false,
            "login",
        ),
        (
            "ssh",
            "Studio workstation",
            "alex@studio",
            "",
            ">_",
            0x4c7769,
            false,
            "ssh-key",
        ),
    ]
    .into_iter()
    .map(|(id, name, user, site, mark, color, favorite, kind)| Item {
        id: id.into(),
        name: name.into(),
        user: user.into(),
        site: site.into(),
        icon_url: (kind == "login" && !site.is_empty())
            .then(|| format!("https://icons.bitwarden.net/{site}/icon.png")),
        icon: None,
        mark: mark.into(),
        color,
        favorite,
        kind: kind.into(),
        folder: None,
        deleted: false,
        archived: false,
        has_totp: false,
        has_passkey: id == "github",
    })
    .collect()
}
pub enum HostCommand {
    CopyError(String),
    Transfer(crate::transfer::TransferIntent),
    Window(plico_renderer::WindowCommand),
    SshDecision { id: u64, allow: bool },
    Account { ticket: u64, request: Box<Request> },
    Disconnect,
    SaveAppearance(Appearance),
    SaveSecurity(crate::security::SecuritySettings),
}
/// Settings › SSH agent view, matching the order of the segmented control.
pub const SSH_PERMISSIONS: usize = 0;
pub const SSH_HISTORY: usize = 1;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SshHistoryFilter {
    All,
    Allowed,
    NotAllowed,
}
impl SshHistoryFilter {
    pub fn includes(self, outcome: plicowarden_sdk::ssh_agent::Outcome) -> bool {
        use plicowarden_sdk::ssh_agent::Outcome;
        let refused = matches!(
            outcome,
            Outcome::Denied | Outcome::TimedOut | Outcome::Locked | Outcome::Failed
        );
        match self {
            Self::All => true,
            Self::Allowed => !refused,
            Self::NotAllowed => refused,
        }
    }
}
pub struct Editor {
    pub id: Option<String>,
    pub kind: String,
    pub revision: Option<String>,
    pub fields: BTreeMap<String, TextBuffer>,
    pub original: BTreeMap<String, Zeroizing<String>>,
    pub favorite: bool,
    pub folder: String,
    pub custom: Vec<plicowarden_sdk::FieldView>,
    pub custom_keys: Vec<u64>,
    pub custom_buffers: BTreeMap<u64, TextBuffer>,
    pub next_custom_key: u64,
}
impl Editor {
    pub fn new() -> Self {
        Self {
            id: None,
            kind: "login".into(),
            revision: None,
            fields: BTreeMap::new(),
            original: BTreeMap::new(),
            favorite: false,
            folder: String::new(),
            custom: Vec::new(),
            custom_keys: Vec::new(),
            custom_buffers: BTreeMap::new(),
            next_custom_key: 1,
        }
    }
    pub fn buffer(&mut self, key: &str, secret: bool, multiline: bool) -> TextBuffer {
        self.fields
            .entry(key.into())
            .or_insert_with(|| {
                if multiline {
                    TextBuffer::multiline()
                } else if secret {
                    TextBuffer::secret()
                } else {
                    TextBuffer::new()
                }
            })
            .clone()
    }
    pub fn from_edit(edit: EditData) -> Self {
        let mut result = Self::new();
        result.id = Some(edit.id);
        result.kind = edit.kind.into();
        result.revision = Some(edit.revision);
        result.favorite = edit.favorite;
        result.folder = edit.folder_id.unwrap_or_default();
        result.custom = edit.custom_fields;
        result.custom_keys = (1..=result.custom.len() as u64).collect();
        result.next_custom_key = result.custom.len() as u64 + 1;
        for (key, value) in edit.fields {
            let secret = secret_field(&key);
            let buffer = result.buffer(&key, secret, multiline_field(&key));
            buffer.set(Zeroizing::new(value.clone()));
            result.original.insert(key, Zeroizing::new(value));
        }
        result
    }
    pub fn patch(&self) -> ItemPatch {
        let fields = self
            .fields
            .iter()
            .filter_map(|(key, buffer)| {
                let changed = buffer.with_value(|value| {
                    self.original
                        .get(key)
                        .map_or(!value.is_empty(), |old| old.as_str() != value)
                });
                (self.id.is_none() || changed).then(|| {
                    (
                        key.clone(),
                        buffer.with_value(|value| Zeroizing::new(value.into())),
                    )
                })
            })
            .collect();
        ItemPatch {
            kind: Some(self.kind.clone()),
            fields,
            favorite: Some(self.favorite),
            folder_id: Some(self.folder.clone()),
            custom_fields: Some(self.custom.clone()),
            expected_revision: self.revision.clone(),
        }
    }
}
impl Drop for Editor {
    fn drop(&mut self) {
        for buffer in self.fields.values().chain(self.custom_buffers.values()) {
            buffer.clear();
        }
        for field in &mut self.custom {
            if let Some(value) = &mut field.value {
                use zeroize::Zeroize;
                value.zeroize();
            }
        }
    }
}
pub fn secret_field(key: &str) -> bool {
    matches!(
        key,
        "password"
            | "totp"
            | "code"
            | "number"
            | "ssn"
            | "passportNumber"
            | "licenseNumber"
            | "privateKey"
    )
}
pub fn multiline_field(key: &str) -> bool {
    matches!(key, "notes" | "uris" | "publicKey" | "privateKey")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationKind {
    Error,
    SessionExpired,
    LoginRequest,
}
#[derive(Clone)]
pub struct AppNotification {
    pub id: u64,
    pub kind: NotificationKind,
    pub title: String,
    pub detail: String,
    pub at: i64,
    pub read: bool,
    pub approval_id: Option<u64>,
}
/// Pages that replace one another. Settings, the command palette and approval
/// dialogs stay modal state above the current page. List filters stay on the vault.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, plico::Route)]
pub enum Page {
    #[default]
    #[route(id = "vault", label = "Vault")]
    Vault,
    #[route(id = "generator", label = "Generator")]
    Generator,
    #[route(id = "login", label = "Log in")]
    Login,
}
pub struct Warden {
    pub items: Vec<Item>,
    pub query: String,
    pub search_ticket: Option<u64>,
    pub search_submitted: String,
    pub search_matches: Option<std::collections::HashSet<String>>,
    pub filter: String,
    pub selected: String,
    pub ascending: bool,
    pub sort_by_type: bool,
    pub transfer: crate::transfer::TransferForm,
    pub transfer_dialog: bool,
    pub transfer_modal: ModalState,
    pub bulk_mode: bool,
    pub bulk_selected: std::collections::BTreeSet<String>,
    pub bulk_confirm: Option<String>,
    pub bulk_modal: ModalState,
    pub sidebar: bool,
    pub custom_titlebar: bool,
    pub titlebar_pressed: bool,
    pub reveal: bool,
    pub settings_open: bool,
    pub settings_modal: plico::ModalState,
    pub settings_tab: String,
    pub security: crate::security::SecuritySettings,
    pub active_security: crate::security::SecuritySettings,
    pub security_saving: bool,
    pub router: plico::Router<Page>,
    pub appearance: Appearance,
    pub zoom: plico_runtime::ZoomController,
    pub notice: String,
    pub error: String,
    pub authenticated: bool,
    pub active_account_id: Option<String>,
    pub adding_account: bool,
    pub offline: bool,
    pub last_sync_at: Option<i64>,
    pub deferred_snapshot: Option<Box<VaultSnapshot>>,
    pub saved_accounts: Vec<plicowarden_sdk::cache::SavedAccount>,
    pub accounts_modal: ModalState,
    pub account_email: String,
    pub folders: Vec<VaultFolder>,
    pub details: Option<ItemDetails>,
    /// The newest item-details request. Detail loading is intentionally kept
    /// separate from the global operation ticket so a row switch never tears
    /// down the detail pane or makes the whole vault busy.
    pub detail_ticket: Option<u64>,
    /// Loading feedback is delayed so a local-cache hit does not paint a
    /// one-frame spinner that reads as a flash.
    pub detail_loading_started_at: Option<std::time::Duration>,
    /// Presentation swap time, sampled from the injected frame clock. The UI
    /// uses one short opacity animation for the whole keyed pane.
    pub detail_transition_started_at: Option<std::time::Duration>,
    /// Per-item detail cache: switching back to an item shows its last details
    /// immediately while the refresh round-trips; the busy state still gates
    /// the sensitive actions.
    pub detail_cache: HashMap<String, ItemDetails>,
    pub shown_password: Option<Zeroizing<String>>,
    pub code: Option<TotpCode>,
    pub code_received_at: Option<std::time::Duration>,
    pub frame_time: plico::FrameTime,
    pub editor: Option<Editor>,
    pub move_modal: ModalState,
    pub move_ids: Vec<String>,
    pub move_folder: String,
    pub folders_modal: ModalState,
    pub confirm_modal: ModalState,
    pub confirm_action: Option<(String, String)>,
    pub folder_name: String,
    pub folder_names: BTreeMap<String, String>,
    pub popups: BTreeMap<String, plico::PopoverState>,
    pub login_password_step: bool,
    pub login_show_password: bool,
    pub environment_open: bool,
    pub environment_error: String,
    pub login_focus: bool,
    /// Set when the login identity should be written back to persisted state;
    /// writing it on every keystroke rebuilt the form and dropped selections.
    pub persist_login_email: bool,
    /// Server/region changes are written when the environment editor is
    /// confirmed, not while their inputs are being edited.
    pub persist_login_settings: bool,
    pub region: String,
    pub server: String,
    pub email: String,
    pub password: TextBuffer,
    pub otp: TextBuffer,
    pub api_url: String,
    pub identity_url: String,
    pub icons_url: String,
    pub advanced: bool,
    pub factors: Vec<TwoFactorOption>,
    pub factor: String,
    pub busy: bool,
    pub login_progress: crate::login_progress::LoginProgress,
    /// The floating sync-progress card is opened only by the footer's sync
    /// button; it never reserves layout space in the vault.
    pub sync_progress_open: bool,
    /// Narrow windows switch panes instead of squeezing three columns.
    pub compact_sidebar: bool,
    pub compact_detail: bool,
    pub sidebar_items_open: bool,
    pub sidebar_folders_open: bool,
    pub sidebar_utilities_open: bool,
    pub palette_modal: plico::ModalState,
    pub palette_query: String,
    pub palette_index: usize,
    pub notifications: VecDeque<AppNotification>,
    pub notification_seq: u64,
    pub notification_open: bool,
    /// Reveal the selected row in the item list once, after selection came
    /// from the palette, a filter or the initial load.
    pub reveal_list_selection: bool,
    pub sidebar_closed_folders: std::collections::BTreeSet<String>,
    pub login_stage: Option<plicowarden_sdk::service::LoginStage>,
    pub generated: Zeroizing<String>,
    pub history: VecDeque<Zeroizing<String>>,
    pub length: f64,
    pub symbols: bool,
    pub generator_mode: generator::Mode,
    pub word_count: f64,
    pub reveal_history: bool,
    pub reveal_card_number: bool,
    pub shown_card_number: Option<Zeroizing<String>>,
    pub history_ticket: Option<u64>,
    pub previous_passwords: Vec<(i64, Zeroizing<String>)>,
    pub commands: Vec<HostCommand>,
    pub pending: Option<u64>,
    pub settings_saving: bool,
    pub ssh_socket: Option<String>,
    pub ssh_approvals: VecDeque<plicowarden_sdk::ssh_agent::Approval>,
    pub ssh_view: usize,
    pub ssh_history: Vec<plicowarden_sdk::ssh_agent::AccessRecord>,
    pub ssh_history_filter: SshHistoryFilter,
    pub ssh_modal: ModalState,
    pub auth_requests: VecDeque<plicowarden_sdk::service::AuthRequest>,
    pub auth_request_modal: ModalState,
    ticket: u64,
    save_in_flight: bool,
}
impl Default for Warden {
    fn default() -> Self {
        Self {
            items: demo_items(),
            query: String::new(),
            search_ticket: None,
            search_submitted: String::new(),
            search_matches: None,
            filter: "All Items".into(),
            selected: "github".into(),
            ascending: true,
            sort_by_type: false,
            transfer: Default::default(),
            transfer_dialog: false,
            transfer_modal: ModalState::default(),
            bulk_mode: false,
            bulk_selected: Default::default(),
            bulk_confirm: None,
            bulk_modal: ModalState::default(),
            sidebar: true,
            custom_titlebar: Appearance::default().custom_titlebar,
            titlebar_pressed: false,
            reveal: false,
            settings_open: false,
            settings_modal: plico::ModalState::default(),
            settings_tab: "appearance".into(),
            security: Default::default(),
            active_security: Default::default(),
            security_saving: false,
            router: plico::Router::new(Page::Vault),
            appearance: Appearance::default(),
            zoom: Appearance::default().zoom_controller(),
            notice: String::new(),
            error: String::new(),
            authenticated: false,
            active_account_id: None,
            adding_account: false,
            offline: false,
            last_sync_at: None,
            deferred_snapshot: None,
            saved_accounts: Vec::new(),
            accounts_modal: ModalState::default(),
            account_email: String::new(),
            folders: Vec::new(),
            details: None,
            detail_ticket: None,
            detail_loading_started_at: None,
            detail_transition_started_at: None,
            detail_cache: HashMap::new(),
            shown_password: None,
            code: None,
            code_received_at: None,
            frame_time: Default::default(),
            editor: None,
            move_modal: ModalState::default(),
            move_ids: Vec::new(),
            move_folder: String::new(),
            folders_modal: ModalState::default(),
            confirm_modal: ModalState::default(),
            confirm_action: None,
            folder_name: String::new(),
            folder_names: BTreeMap::new(),
            popups: BTreeMap::new(),
            login_password_step: false,
            login_show_password: false,
            environment_open: false,
            environment_error: String::new(),
            login_focus: false,
            persist_login_email: false,
            persist_login_settings: false,
            region: "us".into(),
            server: String::new(),
            email: String::new(),
            password: TextBuffer::secret(),
            otp: TextBuffer::secret(),
            api_url: String::new(),
            identity_url: String::new(),
            icons_url: String::new(),
            advanced: false,
            factors: Vec::new(),
            factor: String::new(),
            busy: false,
            login_progress: Default::default(),
            sync_progress_open: false,
            compact_sidebar: false,
            compact_detail: false,
            sidebar_items_open: true,
            sidebar_folders_open: true,
            sidebar_utilities_open: true,
            palette_modal: plico::ModalState::default(),
            palette_query: String::new(),
            palette_index: 0,
            notifications: VecDeque::new(),
            notification_seq: 0,
            notification_open: false,
            reveal_list_selection: false,
            sidebar_closed_folders: Default::default(),
            login_stage: None,
            generated: Zeroizing::new(String::new()),
            history: VecDeque::new(),
            generator_mode: generator::Mode::Password,
            word_count: 6.,
            reveal_history: false,
            reveal_card_number: false,
            shown_card_number: None,
            history_ticket: None,
            previous_passwords: Vec::new(),
            length: 24.,
            symbols: true,
            commands: Vec::new(),
            pending: None,
            settings_saving: false,
            ssh_socket: None,
            ssh_approvals: VecDeque::new(),
            ssh_view: SSH_PERMISSIONS,
            ssh_history: Vec::new(),
            ssh_history_filter: SshHistoryFilter::All,
            ssh_modal: ModalState::default(),
            auth_requests: VecDeque::new(),
            auth_request_modal: ModalState::default(),
            ticket: 0,
            save_in_flight: false,
        }
    }
}
fn kind_rank(kind: &str) -> u8 {
    match kind {
        "login" => 0,
        "card" => 1,
        "identity" => 2,
        "note" => 3,
        "ssh-key" => 4,
        _ => 5,
    }
}
fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}
fn is_unauthorized(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    error.contains("401")
        || error.contains("unauthorized")
        || error.contains("invalid_grant")
        || error.contains("token has expired")
}
/// A folder's normalized path: trims each `/`-separated segment and joins it
/// back, so tree rows and item folders compare consistently.
pub fn folder_path(name: &str) -> String {
    name.split('/')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}
impl Warden {
    pub fn visible(&self) -> Vec<Item> {
        let query = self.query.to_lowercase();
        let mut result: Vec<_> = self
            .items
            .iter()
            .filter(|i| {
                (if self.filter == "Trash" {
                    i.deleted
                } else {
                    !i.deleted
                }) && (if self.filter == "Archive" {
                    i.archived
                } else {
                    self.filter == "Trash" || !i.archived
                }) && (!self.filter.starts_with("folder:")
                    || i.folder.as_deref() == Some(&self.filter[7..]))
                    && (!self.filter.starts_with("path:")
                        || i.folder.as_deref().is_some_and(|id| {
                            self.folders
                                .iter()
                                .find(|folder| folder.id == id)
                                .is_some_and(|folder| {
                                    let path = folder_path(&folder.name);
                                    let selected = &self.filter[5..];
                                    path == selected || path.starts_with(&format!("{selected}/"))
                                })
                        }))
                    && match self.filter.as_str() {
                        "No folder" => i.folder.is_none(),
                        "Favorites" => i.favorite,
                        "Logins" => i.kind == "login",
                        "Cards" => i.kind == "card",
                        "Identities" => i.kind == "identity",
                        "Secure Notes" => i.kind == "note",
                        "SSH Keys" => i.kind == "ssh-key",
                        "One-time Codes" => i.has_totp,
                        "Passkeys" => i.has_passkey,
                        _ => true,
                    }
                    && if self.authenticated
                        && self.search_submitted == self.query
                        && self.search_ticket.is_none()
                        && !query.is_empty()
                    {
                        self.search_matches.as_ref().map_or_else(
                            || {
                                format!("{} {} {}", i.name, i.user, i.site)
                                    .to_lowercase()
                                    .contains(&query)
                            },
                            |ids| ids.contains(&i.id),
                        )
                    } else if self.authenticated
                        && self.search_ticket.is_some()
                        && !query.is_empty()
                    {
                        self.search_matches
                            .as_ref()
                            .is_none_or(|ids| ids.contains(&i.id))
                    } else {
                        let target = format!("{} {} {}", i.name, i.user, i.site).to_lowercase();
                        query.split_whitespace().all(|part| target.contains(part))
                    }
            })
            .cloned()
            .collect();
        result.sort_by(|a, b| {
            let rank = if self.sort_by_type {
                kind_rank(&a.kind).cmp(&kind_rank(&b.kind))
            } else {
                std::cmp::Ordering::Equal
            };
            rank.then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        if !self.ascending {
            result.reverse();
        }
        result
    }
    pub fn open_move_dialog(&mut self, ids: Vec<String>) {
        if !self.authenticated || self.busy || self.editor.is_some() || ids.is_empty() {
            return;
        }
        let mut unique = std::collections::BTreeSet::new();
        if ids.len() > 500
            || ids.iter().any(|id| {
                !unique.insert(id.clone())
                    || !self
                        .items
                        .iter()
                        .any(|item| &item.id == id && !item.deleted)
            })
        {
            return;
        }
        let folders: std::collections::BTreeSet<_> = ids
            .iter()
            .map(|id| {
                self.items
                    .iter()
                    .find(|item| &item.id == id)
                    .and_then(|item| item.folder.clone())
            })
            .collect();
        self.move_folder = if folders.len() == 1 {
            folders.into_iter().next().flatten().unwrap_or_default()
        } else {
            String::new()
        };
        self.error.clear();
        self.move_ids = ids;
        self.move_modal.open();
    }
    pub fn move_items(&mut self) {
        if !self.authenticated
            || self.busy
            || !self.move_modal.is_open()
            || self.move_ids.is_empty()
        {
            return;
        }
        if !self.move_folder.is_empty()
            && !self
                .folders
                .iter()
                .any(|folder| folder.id == self.move_folder)
        {
            self.error = "The destination folder no longer exists. Sync and choose again.".into();
            return;
        }
        let ids = std::mem::take(&mut self.move_ids);
        let folder_id = (!self.move_folder.is_empty()).then(|| self.move_folder.clone());
        self.move_modal.close();
        self.request(Request::MoveToFolder { ids, folder_id });
    }
    pub fn apply_bulk_action(&mut self, action: &str) {
        if !self.authenticated || self.busy || self.bulk_selected.is_empty() {
            return;
        }
        self.request(Request::BatchAction {
            ids: self.bulk_selected.iter().cloned().collect(),
            action: action.into(),
        });
    }
    pub fn request(&mut self, request: Request) {
        if self.busy {
            return;
        }
        self.ticket = self.ticket.wrapping_add(1);
        if matches!(&request, Request::Details(_)) {
            self.detail_ticket = Some(self.ticket);
            self.detail_loading_started_at = Some(self.frame_time.now);
            self.commands.push(HostCommand::Account {
                ticket: self.ticket,
                request: Box::new(request),
            });
            return;
        }
        self.pending = Some(self.ticket);
        self.busy = !matches!(&request, Request::Totp(_));
        self.login_stage = if matches!(&request, Request::Login { .. }) && self.has_cached_account()
        {
            Some(plicowarden_sdk::service::LoginStage::UnlockingCache)
        } else {
            matches!(
                &request,
                Request::Login { .. } | Request::Reconnect { .. } | Request::TwoFactor(_)
            )
            .then_some(plicowarden_sdk::service::LoginStage::Authenticating)
        };
        if matches!(&request, Request::Sync) {
            self.login_progress.begin_sync();
        } else if matches!(&request, Request::Login { .. } | Request::Reconnect { .. }) {
            self.login_progress
                .begin(matches!(&request, Request::Login { .. }) && self.has_cached_account());
        } else if matches!(&request, Request::TwoFactor(_)) {
            self.login_progress
                .activate(plicowarden_sdk::service::LoginStage::Authenticating);
        }
        self.error.clear();
        self.commands.push(HostCommand::Account {
            ticket: self.ticket,
            request: Box::new(request),
        });
    }
    pub fn apply_deferred_snapshot(&mut self) {
        if self.authenticated
            && self.pending.is_none()
            && self.editor.is_none()
            && !self.busy
            && let Some(snapshot) = self.deferred_snapshot.take()
        {
            self.snapshot(*snapshot);
            self.notice = "Vault is up to date.".into();
        }
    }
    pub fn update_local_search(&mut self) {
        if !self.authenticated || self.busy || self.search_submitted == self.query {
            return;
        }
        self.search_submitted = self.query.clone();
        if self.query.trim().is_empty() {
            self.search_matches = None;
            self.search_ticket = None;
            return;
        }
        self.ticket = self.ticket.wrapping_add(1);
        self.search_ticket = Some(self.ticket);
        self.commands.push(HostCommand::Account {
            ticket: self.ticket,
            request: Box::new(Request::Search(self.query.clone())),
        });
    }
    pub fn select(&mut self, id: &str) {
        self.reveal_list_selection = true;
        if self.selected == id {
            return;
        }
        self.selected = id.into();
        // A cache hit swaps immediately; on a first visit the previous complete
        // pane stays mounted until the new details arrive, rather than flashing
        // a structurally different skeleton tree for one or two frames.
        if let Some(details) = self.detail_cache.get(id).cloned() {
            self.detail_transition_started_at = Some(self.frame_time.now);
            self.details = Some(details);
        } else if !self.authenticated {
            self.detail_transition_started_at = Some(self.frame_time.now);
        }
        self.shown_password = None;
        self.reveal = false;
        self.reveal_history = false;
        self.reveal_card_number = false;
        self.shown_card_number = None;
        self.history_ticket = None;
        self.previous_passwords.clear();
        self.code = None;
        self.code_received_at = None;
        self.notice.clear();
        if self.authenticated {
            self.request(Request::Details(id.into()));
        }
    }
    /// Move the list selection by one row; returns false for an empty list.
    pub fn move_selection(&mut self, delta: i32) -> bool {
        let items = self.visible();
        if items.is_empty() {
            return false;
        }
        let current = items
            .iter()
            .position(|item| item.id == self.selected)
            .map(|index| index as i32)
            .unwrap_or(if delta > 0 { -1 } else { items.len() as i32 });
        let next = (current + delta).clamp(0, items.len() as i32 - 1) as usize;
        let id = items[next].id.clone();
        self.select(&id);
        self.reveal_list_selection = true;
        true
    }
    pub fn clear_search(&mut self) {
        self.query.clear();
        self.search_submitted.clear();
        self.search_ticket = None;
        self.search_matches = None;
    }
    pub fn set_filter(&mut self, filter: &str) {
        if self.router.show(Page::Generator) {
            self.router.go(Page::Vault);
        }
        self.compact_sidebar = false;
        self.compact_detail = false;
        self.bulk_selected.clear();
        self.filter = filter.into();
        self.reveal = false;
        self.reveal_history = false;
        self.reveal_card_number = false;
        self.shown_card_number = None;
        self.history_ticket = None;
        self.previous_passwords.clear();
        self.shown_password = None;
    }
    pub fn open_login(&mut self) {
        self.router.go(Page::Login);
    }
    /// Show the vault, recording the previous page so Back can return to it.
    pub fn show_vault(&mut self) {
        if !self.router.show(Page::Vault) {
            self.router.go(Page::Vault);
        }
    }
    pub fn open_generator(&mut self) {
        self.router.go(Page::Generator);
        self.compact_sidebar = false;
        self.compact_detail = false;
        self.bulk_selected.clear();
        self.filter = "All Items".into();
        self.reveal = false;
        self.reveal_history = false;
        self.reveal_card_number = false;
        self.shown_card_number = None;
        self.history_ticket = None;
        self.previous_passwords.clear();
        self.shown_password = None;
    }
    /// Login finished or was dismissed. Return to the page that opened it.
    pub fn close_login(&mut self) {
        if self.router.show(Page::Login) {
            if self.router.can_back() {
                self.router.back();
            } else {
                self.router.replace(Page::Vault);
            }
        }
    }
    pub fn lock(&mut self) {
        self.commands.clear();
        self.move_modal.close();
        self.move_ids.clear();
        self.move_folder.clear();
        self.transfer_modal.close();
        self.transfer.clear_secrets();
        self.transfer_dialog = false;
        self.bulk_selected.clear();
        self.bulk_mode = false;
        self.bulk_confirm = None;
        self.bulk_modal.close();
        self.commands.push(HostCommand::Disconnect);
        self.pending = None;
        self.detail_ticket = None;
        self.detail_loading_started_at = None;
        self.detail_transition_started_at = None;
        self.busy = false;
        self.login_stage = None;
        self.login_progress = Default::default();
        self.sync_progress_open = false;
        self.compact_sidebar = false;
        self.compact_detail = false;
        self.authenticated = false;
        self.active_account_id = None;
        self.offline = false;
        self.last_sync_at = None;
        self.deferred_snapshot = None;
        self.accounts_modal.close();
        self.ssh_socket = None;
        self.ssh_approvals.clear();
        self.auth_requests.clear();
        self.notifications
            .retain(|notification| notification.kind != NotificationKind::LoginRequest);
        self.ssh_history.clear();
        self.ssh_modal.close();
        self.auth_request_modal.close();
        self.items.clear();
        self.folders.clear();
        self.detail_cache.clear();
        self.details = None;
        self.shown_password = None;
        self.code = None;
        self.code_received_at = None;
        self.editor = None;
        self.password.clear();
        self.login_password_step = false;
        self.login_show_password = false;
        self.environment_open = false;
        self.environment_error.clear();
        self.login_focus = true;
        self.otp.clear();
        self.factors.clear();
        self.account_email.clear();
        self.folder_names.clear();
        self.history.clear();
        self.generated.zeroize();
        self.query.clear();
        self.search_ticket = None;
        self.search_submitted.clear();
        self.search_matches = None;
        self.selected.clear();
        self.reveal = false;
        self.reveal_history = false;
        self.history_ticket = None;
        self.previous_passwords.clear();
        self.open_login();
        self.settings_open = false;
        self.settings_modal.close();
        self.folders_modal.close();
        self.confirm_modal.close();
        self.confirm_action = None;
        self.error.clear();
        self.notice = "Vault locked.".into();
        self.login_password_step = self.has_cached_account();
    }
    pub fn validate_login_server(&mut self) -> bool {
        self.environment_error.clear();
        if self.region == "self-hosted" {
            let config = plicowarden_sdk::server::CustomServer {
                api: optional(&self.api_url),
                identity: optional(&self.identity_url),
                icons: optional(&self.icons_url),
                ..Default::default()
            };
            if let Err(error) = config.resolve(Some(&self.server)) {
                self.environment_error = error;
                self.environment_open = true;
                return false;
            }
        }
        true
    }
    pub fn submit_verification(&mut self) {
        if self.busy
            || self.otp.is_empty()
            || !self
                .factors
                .iter()
                .any(|p| factor_id(p.id) == self.factor && p.supported)
        {
            return;
        }
        self.request(Request::TwoFactor(
            plicowarden_sdk::service::TwoFactorSubmitRequest {
                provider: self.factor.clone(),
                token: self.otp.take(),
            },
        ));
    }
    pub fn continue_login(&mut self) {
        if self.busy {
            return;
        }
        let email = self.email.trim();
        if !email.split_once('@').is_some_and(|(name, domain)| {
            !name.is_empty() && domain.contains('.') && !email.contains(char::is_whitespace)
        }) {
            self.error = "Enter a valid email address.".into();
            return;
        }
        if !self.validate_login_server() {
            return;
        }
        self.email = self.email.trim().to_owned();
        self.persist_login_email = true;
        self.error.clear();
        self.notice.clear();
        self.environment_open = false;
        self.login_password_step = true;
        self.login_focus = true;
        self.login_show_password = false;
    }
    pub fn back_to_email(&mut self) {
        self.persist_login_email = true;
        if self.busy {
            return;
        }
        if !self.factors.is_empty() {
            self.commands.push(HostCommand::Disconnect);
        }
        self.login_progress = Default::default();
        self.sync_progress_open = false;
        self.compact_sidebar = false;
        self.compact_detail = false;
        self.factors.clear();
        self.password.clear();
        self.otp.clear();
        self.login_focus = true;
        self.login_password_step = false;
        self.login_show_password = false;
        self.error.clear();
        self.notice.clear();
    }
    pub fn choose_account(&mut self, account: &plicowarden_sdk::cache::SavedAccount) {
        self.lock();
        self.adding_account = false;
        self.email = account.config.email.clone();
        self.region = account.config.region.clone();
        self.server = account.config.server_url.clone().unwrap_or_default();
        let custom = account.config.server_config.clone().unwrap_or_default();
        self.api_url = custom.api.unwrap_or_default();
        self.identity_url = custom.identity.unwrap_or_default();
        self.icons_url = custom.icons.unwrap_or_default();
        self.login_password_step = true;
        self.notice = "Unlock the locally cached vault with your master password.".into();
    }
    pub fn add_account(&mut self) {
        self.lock();
        self.adding_account = true;
        self.email.clear();
        self.login_password_step = false;
        self.notice = "Connect another account.".into();
    }
    pub fn account_configuration(&self) -> LoginRequest {
        LoginRequest {
            region: self.region.clone(),
            email: self.email.trim().into(),
            server_url: (self.region == "self-hosted").then(|| self.server.clone()),
            server_config: (self.region == "self-hosted").then(|| {
                plicowarden_sdk::server::CustomServer {
                    api: optional(&self.api_url),
                    identity: optional(&self.identity_url),
                    icons: optional(&self.icons_url),
                    ..Default::default()
                }
            }),
        }
    }
    pub fn has_cached_account(&self) -> bool {
        let Ok(id) = plicowarden_sdk::cache::account_id(&self.account_configuration()) else {
            return false;
        };
        self.saved_accounts.iter().any(|account| account.id == id)
    }
    pub fn reconnect(&mut self) {
        self.start_login(true);
    }
    pub fn login(&mut self) {
        self.start_login(false);
    }
    fn start_login(&mut self, force_online: bool) {
        if self.busy {
            return;
        }
        if self.email.trim().is_empty() || self.password.is_empty() {
            self.error = "Enter your email and master password.".into();
            return;
        }
        self.commands.push(HostCommand::Disconnect);
        self.factors.clear();
        let config = self.account_configuration();
        let password = self
            .password
            .with_value(|value| Zeroizing::new(value.to_owned()));
        self.request(if force_online {
            Request::Reconnect { config, password }
        } else {
            Request::Login { config, password }
        });
    }
    pub fn snapshot(&mut self, snapshot: VaultSnapshot) {
        self.login_progress
            .finish(snapshot.offline, snapshot.decrypt_failures);
        self.login_progress.visible = false;
        self.adding_account = false;
        self.deferred_snapshot = None;
        self.authenticated = true;
        self.notifications
            .retain(|notification| notification.kind != NotificationKind::SessionExpired);
        self.offline = snapshot.offline;
        if !self.offline {
            self.last_sync_at = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|elapsed| elapsed.as_secs() as i64)
                    .unwrap_or(0),
            );
        }
        if self.active_account_id != snapshot.account_id {
            // Signing history is vault-scoped public metadata; never carry it
            // into another account's session.
            self.ssh_history.clear();
            self.notifications.clear();
        }
        self.active_account_id = snapshot.account_id;
        self.search_ticket = None;
        self.search_matches = None;
        self.search_submitted.clear();
        if let Some(warning) = snapshot.cache_warning {
            self.record_error(format!("Local vault cache could not be updated: {warning}"));
        } else if self.offline {
            self.notice = "Unlocked local cache. Synchronizing in the background…".into();
        }
        self.account_email = snapshot.email;
        self.folders = snapshot.folders;
        self.items = snapshot
            .items
            .into_iter()
            .map(|i| Item {
                mark: i
                    .name
                    .chars()
                    .next()
                    .map(|c| c.to_uppercase().to_string())
                    .unwrap_or_else(|| "?".into()),
                id: i.id,
                name: i.name,
                user: i.subtitle,
                site: String::new(),
                icon_url: i.icon_url,
                icon: None,
                color: 0x65618e,
                favorite: i.favorite,
                kind: i.kind.into(),
                folder: i.folder_id,
                deleted: i.deleted,
                archived: i.archived,
                has_totp: i.has_totp,
                has_passkey: i.has_passkey,
            })
            .collect();
        self.close_login();
        self.password.clear();
        self.otp.clear();
        self.factors.clear();
        // A fresh sync may rename or remove items: keep cached details only
        // for items that still exist (and invalidate the live slot when its
        // item is gone, so the pane does not show a deleted item).
        self.detail_cache
            .retain(|id, _| self.items.iter().any(|i| &i.id == id));
        let selected = self.selected.clone();
        self.details = self.detail_cache.get(&selected).cloned();
        self.shown_password = None;
        self.reveal = false;
        self.reveal_history = false;
        self.reveal_card_number = false;
        self.shown_card_number = None;
        self.history_ticket = None;
        self.previous_passwords.clear();
        self.code = None;
        self.code_received_at = None;
        let visible = self.visible();
        if !visible.iter().any(|i| i.id == self.selected) {
            self.selected = visible.first().map(|i| i.id.clone()).unwrap_or_default();
        }
        if snapshot.decrypt_failures > 0 {
            self.error = format!(
                "{} items could not be decrypted. Sync again before editing.",
                snapshot.decrypt_failures
            );
        }
        if !self.selected.is_empty() && self.editor.is_none() && !self.router.show(Page::Generator)
        {
            self.request(Request::Details(self.selected.clone()));
        }
    }
    pub fn expire_session(&mut self) {
        let email = self.account_email.clone();
        self.lock();
        if !email.is_empty() {
            self.email = email;
        }
        self.login_password_step = false;
        self.notice = "Session expired. Sign in again.".into();
    }
    pub fn push_notification(
        &mut self,
        kind: NotificationKind,
        title: impl Into<String>,
        detail: impl Into<String>,
        approval_id: Option<u64>,
    ) {
        let title = title.into();
        let detail = detail.into();
        let now = now_seconds();
        if let Some(existing) = self.notifications.iter_mut().find(|notification| {
            notification.kind == kind
                && notification.title == title
                && notification.detail == detail
        }) {
            existing.at = now;
            existing.read = false;
            return;
        }
        self.notification_seq += 1;
        self.notifications.push_front(AppNotification {
            id: self.notification_seq,
            kind,
            title,
            detail,
            at: now,
            read: false,
            approval_id,
        });
        self.notifications.truncate(30);
    }
    pub fn unread_notifications(&self) -> usize {
        self.notifications
            .iter()
            .filter(|notification| !notification.read)
            .count()
    }
    pub fn toggle_notifications(&mut self) {
        if self.notification_open {
            self.notification_open = false;
        } else {
            self.notification_open = true;
            for notification in &mut self.notifications {
                if notification.kind != NotificationKind::LoginRequest {
                    notification.read = true;
                }
            }
        }
    }
    pub fn clear_read_notifications(&mut self) {
        self.notifications.retain(|notification| {
            notification.kind == NotificationKind::LoginRequest || !notification.read
        });
    }
    fn handle_unauthorized(&mut self) {
        self.push_notification(
            NotificationKind::SessionExpired,
            "Session expired",
            "Sign in again to continue syncing this vault.",
            None,
        );
        self.notice = "Session expired. Sign in to sync.".into();
    }
    fn record_error(&mut self, message: String) {
        self.push_notification(NotificationKind::Error, "Error", message, None);
    }
    pub fn apply(&mut self, event: plicowarden_sdk::Event) {
        if self.authenticated
            && let Err(error) = &event.result
            && is_unauthorized(error)
        {
            self.expire_session();
            return;
        }
        if let Ok(Reply::SearchResults { query, ids }) = &event.result {
            if self.search_ticket == Some(event.ticket) && query == &self.query {
                self.search_ticket = None;
                self.search_matches = Some(ids.iter().cloned().collect());
            }
            return;
        }
        if self.search_ticket == Some(event.ticket)
            && let Err(error) = &event.result
        {
            self.search_ticket = None;
            self.search_matches = None;
            self.error = format!("Local search failed: {error}");
            return;
        }
        if let Ok(Reply::BackgroundProgress(stage)) = &event.result {
            if self.authenticated && !self.login_progress.steps.is_empty() {
                self.login_progress.activate(*stage);
            }
            return;
        }
        if matches!(&event.result, Ok(Reply::BackgroundSyncPaused)) {
            self.login_progress.pause();
            return;
        }
        let event = match event {
            plicowarden_sdk::Event {
                result: Ok(Reply::BackgroundSyncFailed(error)),
                ..
            } => {
                if self.authenticated {
                    if is_unauthorized(&error) {
                        self.handle_unauthorized();
                    } else {
                        self.login_progress.fail();
                        self.notice =
                            "Using the local vault. You can keep browsing while offline.".into();
                        self.record_error(format!("Background synchronization failed: {error}"));
                    }
                }
                return;
            }
            plicowarden_sdk::Event {
                result: Ok(Reply::BackgroundSnapshot(snapshot)),
                ..
            } => {
                if self.authenticated
                    && self.account_email == snapshot.email
                    && self.active_account_id == snapshot.account_id
                {
                    if self.pending.is_none() && self.editor.is_none() {
                        self.snapshot(snapshot);
                        self.offline = false;
                        self.notice = "Vault is up to date.".into();
                    } else {
                        self.deferred_snapshot = Some(Box::new(snapshot));
                    }
                }
                return;
            }
            event => event,
        };

        if let Ok(Reply::TotpExpired(expiry)) = &event.result {
            if self
                .code
                .as_ref()
                .is_some_and(|code| code.expires_at == *expiry)
            {
                self.code = None;
                self.code_received_at = None;
                if self.authenticated
                    && self
                        .items
                        .iter()
                        .any(|item| item.id == self.selected && item.has_totp)
                    && !self.selected.is_empty()
                {
                    self.request(Request::Totp(self.selected.clone()));
                }
            }
            return;
        }

        if matches!(&event.result, Ok(Reply::Locked)) {
            // The worker reports a timeout error before its mandatory lock.
            // Clearing that error here made failed sign-ins look like a silent reset.
            let login_error = (self.router.show(Page::Login) && !self.authenticated)
                .then(|| std::mem::take(&mut self.error))
                .filter(|error| !error.is_empty());
            let retry_password = login_error.as_ref().map(|_| self.password.take());
            let progress = self.login_progress.clone();
            self.lock();
            if let Some(error) = login_error {
                self.login_progress = progress;
                if let Some(password) = retry_password {
                    self.password.set(password);
                    self.login_password_step = true;
                }
                self.error = error;
                self.notice = "Sign-in failed. Try again.".into();
            } else {
                self.notice = "Locked after inactivity or session expiry.".into();
            }
            return;
        }
        if matches!(&event.result, Ok(Reply::SecurityConfigured(_))) {
            // The host's last successfully saved settings remain authoritative;
            // an older queued worker acknowledgement must not roll them back.
            return;
        }
        if self.detail_ticket == Some(event.ticket) {
            self.detail_ticket = None;
            self.detail_loading_started_at = None;
            match event.result {
                Ok(Reply::Details(boxed)) => {
                    if self.detail_cache.len() >= 512 {
                        self.detail_cache.clear();
                    }
                    let details = *boxed;
                    let current = self.selected == details.id;
                    let generate_totp = current && details.has_totp;
                    let detail_id = details.id.clone();
                    if current {
                        if self
                            .details
                            .as_ref()
                            .is_none_or(|shown| shown.id != details.id)
                        {
                            self.detail_transition_started_at = Some(self.frame_time.now);
                        }
                        self.details = Some(details.clone());
                    }
                    self.detail_cache.insert(details.id.clone(), details);
                    if generate_totp {
                        self.request(Request::Totp(detail_id));
                    }
                }
                Err(error) => {
                    // The old pane is still valid. Restore its row highlight
                    // so the list and detail never disagree after a failed
                    // first load, and keep the error copyable below the pane.
                    if let Some(details) = &self.details {
                        self.selected = details.id.clone();
                    }
                    self.error = format!("Could not load item details: {error}");
                }
                Ok(_) => {}
            }
            return;
        }
        if event.ticket == 0 {
            match &event.result {
                Ok(Reply::AuthRequests(requests)) => {
                    if !self.authenticated {
                        return;
                    }
                    let current: std::collections::HashSet<_> =
                        requests.iter().map(|request| request.id.as_str()).collect();
                    self.auth_requests
                        .retain(|request| current.contains(request.id.as_str()));
                    for request in requests {
                        if !self
                            .auth_requests
                            .iter()
                            .any(|pending| pending.id == request.id)
                        {
                            self.auth_requests.push_back(request.clone());
                        }
                    }
                    if !self.auth_requests.is_empty() {
                        self.auth_request_modal.open();
                    }
                    return;
                }
                Ok(Reply::BackgroundLogin(LoginOutcome::Authenticated { snapshot })) => {
                    self.filter = "All Items".into();
                    self.query.clear();
                    self.snapshot(snapshot.clone());
                    self.notice = "Vault is up to date.".into();
                    return;
                }
                Ok(Reply::BackgroundLogin(LoginOutcome::TwoFactorRequired { providers })) => {
                    self.open_login();
                    self.login_password_step = true;
                    self.login_progress.verification();
                    self.factor = providers
                        .iter()
                        .find(|provider| provider.supported)
                        .map(|provider| factor_id(provider.id).into())
                        .unwrap_or_default();
                    self.factors = providers.clone();
                    self.login_focus = true;
                    self.notice = "Verify your identity to finish syncing. Your master password was reused once and has been cleared.".into();
                    return;
                }
                Ok(Reply::SshApproval(approval)) => {
                    if self.authenticated
                        && self.ssh_socket.is_some()
                        && !self.ssh_approvals.iter().any(|a| a.id == approval.id)
                    {
                        self.ssh_approvals.push_back(approval.clone());
                        self.ssh_modal.open();
                        self.push_notification(
                            NotificationKind::LoginRequest,
                            "Device signing request",
                            format!("{} · {}", approval.name, approval.fingerprint),
                            Some(approval.id),
                        );
                    }
                    return;
                }
                Ok(Reply::SshClosed(id)) => {
                    self.close_ssh_approval(*id);
                    return;
                }
                Ok(Reply::SshAccess(record)) => {
                    self.record_ssh_access(record.clone());
                    return;
                }
                Ok(Reply::SshHistory(records)) => {
                    self.ssh_history = records.clone();
                    return;
                }
                Ok(Reply::SshStatus(None)) => {
                    self.ssh_socket = None;
                    self.ssh_approvals.clear();
                    self.ssh_modal.close();
                    return;
                }
                _ => {}
            }
            if let Err(error) = event.result {
                self.busy = false;
                self.pending = None;
                self.error = error;
                self.login_focus = self.router.show(Page::Login);
            }
            return;
        }
        if self.pending != Some(event.ticket) {
            return;
        }
        if let Ok(Reply::LoginProgress(stage)) = &event.result {
            self.login_stage = Some(*stage);
            self.login_progress.activate(*stage);
            return;
        }
        self.pending = None;
        self.busy = false;
        let login_stage = self.login_stage.take();
        match event.result {
            Err(error) => {
                self.error = if let Some(stage) = login_stage {
                    self.login_progress.fail();
                    format!("{} {error}", stage.label())
                } else {
                    error
                };
                self.login_focus = self.router.show(Page::Login);
                self.save_in_flight = false;
            }
            Ok(reply) => match reply {
                Reply::LoginProgress(_) => {
                    unreachable!("progress is handled without completing the request")
                }
                Reply::SecurityConfigured(_) => {}
                Reply::Transfer(result) => {
                    if let Some(snapshot) = result.snapshot {
                        self.snapshot(snapshot);
                    }
                    self.notice = result.message;
                }
                Reply::BatchMutation(result) => {
                    let was_bulk = self.bulk_mode;
                    let count = result.succeeded.len();
                    let errors: Vec<_> = result
                        .failed
                        .iter()
                        .map(|failure| failure.message.clone())
                        .collect();
                    let failed: std::collections::BTreeSet<_> = result
                        .failed
                        .into_iter()
                        .map(|failure| failure.id)
                        .collect();
                    self.snapshot(result.snapshot);
                    self.bulk_selected = failed;
                    self.bulk_mode = was_bulk && !self.bulk_selected.is_empty();
                    self.notice =
                        format!("Updated {count} item{}.", if count == 1 { "" } else { "s" });
                    if !errors.is_empty() {
                        if !self.error.is_empty() {
                            self.error.push('\n');
                        }
                        self.error.push_str(&errors.join("\n"));
                    }
                }
                Reply::Login(LoginOutcome::Authenticated { snapshot }) => {
                    self.filter = "All Items".into();
                    self.query.clear();
                    let cached = snapshot.offline;
                    self.snapshot(snapshot);
                    if cached {
                        self.commands.push(HostCommand::Account {
                            ticket: 0,
                            request: Box::new(Request::BackgroundSync),
                        });
                    }
                }
                Reply::Login(LoginOutcome::TwoFactorRequired { providers }) => {
                    self.login_progress.verification();
                    self.factor = providers
                        .iter()
                        .find(|p| p.supported)
                        .map(|p| factor_id(p.id).into())
                        .unwrap_or_default();
                    self.factors = providers;
                    self.login_focus = true;
                }
                Reply::SshStatus(socket) => {
                    self.ssh_socket = if self.authenticated { socket } else { None };
                    if self.ssh_socket.is_none() {
                        self.ssh_approvals.clear();
                        self.ssh_modal.close();
                    }
                }
                Reply::SshClosed(id) => self.close_ssh_approval(id),
                Reply::SshAccess(record) => self.record_ssh_access(record),
                Reply::SshHistory(records) => self.ssh_history = records,
                Reply::SshApproval(_) => {}
                Reply::AuthRequests(_) => {}
                Reply::AuthRequestAnswered { id, approved } => {
                    self.auth_requests.retain(|request| request.id != id);
                    if self.auth_requests.is_empty() {
                        self.auth_request_modal.close();
                    } else {
                        self.auth_request_modal.open();
                    }
                    self.notice = if approved {
                        "Sign-in approved.".into()
                    } else {
                        "Sign-in denied.".into()
                    };
                }
                Reply::Snapshot(snapshot) => self.snapshot(snapshot),
                Reply::PasswordHistory { id, entries } => {
                    if self.selected == id && self.history_ticket == Some(event.ticket) {
                        self.previous_passwords = entries;
                        self.reveal_history = true;
                    }
                    self.history_ticket = None;
                }
                Reply::Details(_) => {
                    unreachable!("detail replies use the independent detail ticket")
                }
                Reply::Revealed(password) => {
                    self.shown_password = Some(password);
                    self.reveal = true;
                }
                Reply::CardNumberRevealed { id, number } => {
                    if self.selected == id {
                        self.shown_card_number = Some(number);
                        self.reveal_card_number = true;
                    }
                }
                Reply::Totp(code) => {
                    self.code_received_at = Some(self.frame_time.now);
                    self.code = Some(code);
                }
                Reply::Edit(edit) => self.editor = Some(Editor::from_edit(edit)),
                Reply::Mutation(mutation) => {
                    if self.save_in_flight {
                        self.editor = None;
                        self.save_in_flight = false;
                    }
                    if let Some(id) = mutation.id {
                        self.selected = id;
                    }
                    self.snapshot(mutation.snapshot);
                    self.notice = "Vault updated.".into();
                }
                Reply::Copied => {
                    self.notice = format!(
                        "Copied. Clipboard clears after {} seconds if unchanged.",
                        self.active_security.clipboard_seconds
                    );
                    if self.active_security.minimize_on_copy {
                        self.commands
                            .push(HostCommand::Window(plico_renderer::WindowCommand::Minimize));
                    }
                }
                Reply::EmailSent => self.notice = "Verification email sent.".into(),
                Reply::SavedAccounts(accounts) => {
                    self.lock();
                    self.saved_accounts = accounts;
                    self.adding_account = true;
                    self.email.clear();
                    self.login_password_step = false;
                    self.notice = "Account removed from this device.".into();
                }
                Reply::SearchResults { .. } => unreachable!("search replies handled separately"),
                Reply::BackgroundSnapshot(_)
                | Reply::BackgroundLogin(_)
                | Reply::BackgroundSyncFailed(_)
                | Reply::BackgroundProgress(_)
                | Reply::BackgroundSyncPaused => {
                    unreachable!("background replies handled separately")
                }
                Reply::Locked | Reply::TotpExpired(_) => {}
            },
        }
    }
    pub fn save_editor(&mut self) {
        if let Some(editor) = &self.editor {
            let request = Request::Save {
                id: editor.id.clone(),
                patch: editor.patch(),
            };
            self.save_in_flight = true;
            self.request(request);
        }
    }
    pub fn generate(&mut self) {
        let result = match self.generator_mode {
            generator::Mode::Password => {
                generator::generate(self.length.round() as usize, self.symbols)
            }
            generator::Mode::Passphrase => generator::passphrase(self.word_count.round() as usize),
            generator::Mode::Username => generator::username(),
        };
        match result {
            Ok(value) => {
                self.generated = value;
                self.history.push_front(self.generated.clone());
                self.history.truncate(5);
            }
            Err(error) => self.error = error.to_string(),
        }
    }
    pub fn close_ssh_approval(&mut self, id: u64) {
        self.ssh_approvals.retain(|approval| approval.id != id);
        self.notifications
            .retain(|notification| notification.approval_id != Some(id));
        if self.ssh_approvals.is_empty() {
            self.ssh_modal.close();
        }
    }

    pub fn decide_ssh(&mut self, id: u64, allow: bool) {
        self.close_ssh_approval(id);
        self.commands.push(HostCommand::SshDecision { id, allow });
    }

    pub fn record_ssh_access(&mut self, record: plicowarden_sdk::ssh_agent::AccessRecord) {
        self.ssh_history.insert(0, record);
        self.ssh_history
            .truncate(plicowarden_sdk::ssh_agent::HISTORY_CAPACITY);
    }

    pub fn clear_ssh_history(&mut self) {
        if self.ssh_history.is_empty() {
            return;
        }
        self.ssh_history.clear();
        self.commands.push(HostCommand::Account {
            ticket: 0,
            request: Box::new(Request::ClearSshHistory),
        });
    }

    pub fn copy_text(&mut self, value: Zeroizing<String>) {
        self.request(Request::CopyText(value));
    }
}
fn optional(s: &str) -> Option<String> {
    (!s.trim().is_empty()).then(|| s.trim().into())
}
pub fn factor_id(id: TwoFactorProviderId) -> &'static str {
    match id {
        TwoFactorProviderId::Authenticator => "authenticator",
        TwoFactorProviderId::Email => "email",
        TwoFactorProviderId::Yubikey => "yubikey",
        TwoFactorProviderId::Duo => "duo",
        TwoFactorProviderId::Webauthn => "webauthn",
    }
}

pub fn business_key(value: &str) -> u64 {
    // Stable string business identity without leaking dynamic strings into static keys.
    value.bytes().fold(0xcbf29ce484222325, |hash, b| {
        (hash ^ u64::from(b)).wrapping_mul(0x100000001b3)
    })
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}
