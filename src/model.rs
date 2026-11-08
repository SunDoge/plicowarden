use crate::{appearance::Appearance, generator};
use plico::{ModalState, TextBuffer};
use plicowarden_sdk::{Reply, Request, service::*};
use std::collections::{BTreeMap, VecDeque};
use zeroize::{Zeroize, Zeroizing};

#[derive(Clone)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub user: String,
    pub site: String,
    pub mark: String,
    pub color: u32,
    pub favorite: bool,
    pub kind: String,
    pub folder: Option<String>,
    pub deleted: bool,
    pub archived: bool,
    pub has_totp: bool,
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
        mark: mark.into(),
        color,
        favorite,
        kind: kind.into(),
        folder: None,
        deleted: false,
        archived: false,
        has_totp: false,
    })
    .collect()
}
pub enum HostCommand {
    Window(plico_renderer::WindowCommand),
    SshDecision { id: u64, allow: bool },
    Account { ticket: u64, request: Box<Request> },
    Disconnect,
    SaveAppearance(Appearance),
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

pub struct Warden {
    pub items: Vec<Item>,
    pub query: String,
    pub filter: String,
    pub selected: String,
    pub ascending: bool,
    pub sidebar: bool,
    pub native_titlebar: bool,
    pub titlebar_pressed: bool,
    pub reveal: bool,
    pub settings_open: bool,
    pub login_open: bool,
    pub appearance: Appearance,
    pub zoom: plico_runtime::ZoomController,
    pub notice: String,
    pub error: String,
    pub authenticated: bool,
    pub account_email: String,
    pub folders: Vec<VaultFolder>,
    pub details: Option<ItemDetails>,
    pub shown_password: Option<Zeroizing<String>>,
    pub code: Option<TotpCode>,
    pub editor: Option<Editor>,
    pub folders_modal: ModalState,
    pub confirm_modal: ModalState,
    pub confirm_action: Option<(String, String)>,
    pub folder_name: String,
    pub folder_names: BTreeMap<String, String>,
    pub popups: BTreeMap<String, plico::PopoverState>,
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
    pub generated: Zeroizing<String>,
    pub history: VecDeque<Zeroizing<String>>,
    pub length: f64,
    pub symbols: bool,
    pub generator_mode: generator::Mode,
    pub word_count: f64,
    pub reveal_history: bool,
    pub history_ticket: Option<u64>,
    pub previous_passwords: Vec<(i64, Zeroizing<String>)>,
    pub commands: Vec<HostCommand>,
    pub pending: Option<u64>,
    pub settings_saving: bool,
    pub ssh_socket: Option<String>,
    pub ssh_approvals: VecDeque<plicowarden_sdk::ssh_agent::Approval>,
    pub ssh_modal: ModalState,
    ticket: u64,
    save_in_flight: bool,
}
impl Default for Warden {
    fn default() -> Self {
        Self {
            items: demo_items(),
            query: String::new(),
            filter: "All Items".into(),
            selected: "github".into(),
            ascending: true,
            sidebar: true,
            native_titlebar: cfg!(target_os = "macos"),
            titlebar_pressed: false,
            reveal: false,
            settings_open: false,
            login_open: false,
            appearance: Appearance::default(),
            zoom: Appearance::default().zoom_controller(),
            notice: String::new(),
            error: String::new(),
            authenticated: false,
            account_email: String::new(),
            folders: Vec::new(),
            details: None,
            shown_password: None,
            code: None,
            editor: None,
            folders_modal: ModalState::default(),
            confirm_modal: ModalState::default(),
            confirm_action: None,
            folder_name: String::new(),
            folder_names: BTreeMap::new(),
            popups: BTreeMap::new(),
            region: "self-hosted".into(),
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
            generated: Zeroizing::new(String::new()),
            history: VecDeque::new(),
            generator_mode: generator::Mode::Password,
            word_count: 6.,
            reveal_history: false,
            history_ticket: None,
            previous_passwords: Vec::new(),
            length: 24.,
            symbols: true,
            commands: Vec::new(),
            pending: None,
            settings_saving: false,
            ssh_socket: None,
            ssh_approvals: VecDeque::new(),
            ssh_modal: ModalState::default(),
            ticket: 0,
            save_in_flight: false,
        }
    }
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
                    && match self.filter.as_str() {
                        "Favorites" => i.favorite,
                        "Logins" => i.kind == "login",
                        "Cards" => i.kind == "card",
                        "Identities" => i.kind == "identity",
                        "Secure Notes" => i.kind == "note",
                        "SSH Keys" => i.kind == "ssh-key",
                        "One-time Codes" => i.has_totp,
                        _ => true,
                    }
                    && format!("{} {} {}", i.name, i.user, i.site)
                        .to_lowercase()
                        .contains(&query)
            })
            .cloned()
            .collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        if !self.ascending {
            result.reverse();
        }
        result
    }
    pub fn request(&mut self, request: Request) {
        if self.busy {
            return;
        }
        self.ticket = self.ticket.wrapping_add(1);
        self.pending = Some(self.ticket);
        self.busy = true;
        self.error.clear();
        self.commands.push(HostCommand::Account {
            ticket: self.ticket,
            request: Box::new(request),
        });
    }
    pub fn select(&mut self, id: &str) {
        if self.selected == id {
            return;
        }
        self.selected = id.into();
        self.details = None;
        self.shown_password = None;
        self.reveal = false;
        self.reveal_history = false;
        self.history_ticket = None;
        self.previous_passwords.clear();
        self.code = None;
        self.notice.clear();
        if self.authenticated {
            self.request(Request::Details(id.into()));
        }
    }
    pub fn set_filter(&mut self, filter: &str) {
        self.filter = filter.into();
        self.reveal = false;
        self.reveal_history = false;
        self.history_ticket = None;
        self.previous_passwords.clear();
        self.shown_password = None;
    }
    pub fn lock(&mut self) {
        self.commands.clear();
        self.commands.push(HostCommand::Disconnect);
        self.pending = None;
        self.busy = false;
        self.authenticated = false;
        self.ssh_socket = None;
        self.ssh_approvals.clear();
        self.ssh_modal.close();
        self.items.clear();
        self.folders.clear();
        self.details = None;
        self.shown_password = None;
        self.code = None;
        self.editor = None;
        self.password.clear();
        self.otp.clear();
        self.factors.clear();
        self.account_email.clear();
        self.folder_names.clear();
        self.history.clear();
        self.generated.zeroize();
        self.query.clear();
        self.selected.clear();
        self.reveal = false;
        self.reveal_history = false;
        self.history_ticket = None;
        self.previous_passwords.clear();
        self.login_open = true;
        self.settings_open = false;
        self.folders_modal.close();
        self.confirm_modal.close();
        self.confirm_action = None;
        self.error.clear();
        self.notice = "Vault locked.".into();
    }
    pub fn login(&mut self) {
        if self.busy {
            return;
        }
        if self.email.trim().is_empty() || self.password.is_empty() {
            self.error = "Enter your email and master password.".into();
            return;
        }
        self.commands.push(HostCommand::Disconnect);
        self.factors.clear();
        let config = LoginRequest {
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
        };
        self.request(Request::Login {
            config,
            password: self.password.take(),
        });
    }
    pub fn snapshot(&mut self, snapshot: VaultSnapshot) {
        self.authenticated = true;
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
                color: 0x65618e,
                favorite: i.favorite,
                kind: i.kind.into(),
                folder: i.folder_id,
                deleted: i.deleted,
                archived: i.archived,
                has_totp: i.has_totp,
            })
            .collect();
        self.login_open = false;
        self.password.clear();
        self.otp.clear();
        self.factors.clear();
        self.details = None;
        self.shown_password = None;
        self.reveal = false;
        self.reveal_history = false;
        self.history_ticket = None;
        self.previous_passwords.clear();
        self.code = None;
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
        if !self.selected.is_empty() && self.editor.is_none() && self.filter != "Generator" {
            self.request(Request::Details(self.selected.clone()));
        }
    }
    pub fn apply(&mut self, event: plicowarden_sdk::Event) {
        if let Ok(Reply::TotpExpired(expiry)) = &event.result {
            if self
                .code
                .as_ref()
                .is_some_and(|code| code.expires_at == *expiry)
            {
                self.code = None;
            }
            return;
        }

        if matches!(&event.result, Ok(Reply::Locked)) {
            self.lock();
            self.notice = "Locked after inactivity or session expiry.".into();
            return;
        }
        if event.ticket == 0 {
            match &event.result {
                Ok(Reply::SshApproval(approval)) => {
                    if self.authenticated
                        && self.ssh_socket.is_some()
                        && !self.ssh_approvals.iter().any(|a| a.id == approval.id)
                    {
                        self.ssh_approvals.push_back(approval.clone());
                        self.ssh_modal.open();
                    }
                    return;
                }
                Ok(Reply::SshClosed(id)) => {
                    self.close_ssh_approval(*id);
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
            }
            return;
        }
        if self.pending != Some(event.ticket) {
            return;
        }
        self.pending = None;
        self.busy = false;
        match event.result {
            Err(error) => {
                self.error = error;
                self.save_in_flight = false;
            }
            Ok(reply) => match reply {
                Reply::Login(LoginOutcome::Authenticated { snapshot }) => {
                    self.filter = "All Items".into();
                    self.query.clear();
                    self.snapshot(snapshot);
                }
                Reply::Login(LoginOutcome::TwoFactorRequired { providers }) => {
                    self.factor = providers
                        .iter()
                        .find(|p| p.supported)
                        .map(|p| factor_id(p.id).into())
                        .unwrap_or_default();
                    self.factors = providers;
                }
                Reply::SshStatus(socket) => {
                    self.ssh_socket = if self.authenticated { socket } else { None };
                    if self.ssh_socket.is_none() {
                        self.ssh_approvals.clear();
                        self.ssh_modal.close();
                    }
                }
                Reply::SshClosed(id) => self.close_ssh_approval(id),
                Reply::SshApproval(_) => {}
                Reply::Snapshot(snapshot) => self.snapshot(snapshot),
                Reply::PasswordHistory { id, entries } => {
                    if self.selected == id && self.history_ticket == Some(event.ticket) {
                        self.previous_passwords = entries;
                        self.reveal_history = true;
                    }
                    self.history_ticket = None;
                }
                Reply::Details(details) => {
                    if self.selected == details.id {
                        self.details = Some(*details);
                    }
                }
                Reply::Revealed(password) => {
                    self.shown_password = Some(password);
                    self.reveal = true;
                }
                Reply::Totp(code) => self.code = Some(code),
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
                    self.notice = "Copied. Clipboard clears after 30 seconds if unchanged.".into()
                }
                Reply::EmailSent => self.notice = "Verification email sent.".into(),
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
        if self.ssh_approvals.is_empty() {
            self.ssh_modal.close();
        }
    }

    pub fn decide_ssh(&mut self, id: u64, allow: bool) {
        self.close_ssh_approval(id);
        self.commands.push(HostCommand::SshDecision { id, allow });
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
