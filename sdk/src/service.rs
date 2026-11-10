mod diagnostics;
mod items;
mod transfer;
pub use items::{BatchFailure, BatchMutation, EditData, ItemPatch, Mutation};
pub use transfer::{ExportKind, TransferResult};
mod writes;
pub use writes::LoginWrite;
// Adapted from Warden Desktop's GPL-3.0-only SDK service (see the app README).
use async_trait::async_trait;
use bitwarden_api_api::models::SyncResponseModel;
use bitwarden_core::auth::login::{
    LoginError, PasswordLoginRequest, PasswordLoginResponse, TwoFactorEmailRequest,
    TwoFactorProvider, TwoFactorRequest, response::two_factor::TwoFactorProviders,
};
use bitwarden_core::{ClientSettings, DeviceType, FromClient};
use bitwarden_crypto_sync_handler::CryptoSyncHandler;
use bitwarden_pm::PasswordManagerClient;
use bitwarden_sync::{SyncClient, SyncHandler, SyncHandlerError, SyncRequest};
use bitwarden_vault::{
    Cipher, CipherListViewType, CipherRepromptType, CipherType, CipherView, FolderSyncHandler,
    VaultClientExt, generate_totp,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use zeroize::{Zeroize, Zeroizing};
const DEFAULT_AUTO_LOCK_AFTER: Duration = Duration::from_secs(300);
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub region: String,
    pub server_url: Option<String>,
    #[serde(default)]
    pub server_config: Option<crate::server::CustomServer>,
    pub email: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwoFactorSubmitRequest {
    pub provider: String,
    pub token: Zeroizing<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TwoFactorOption {
    pub id: TwoFactorProviderId,
    pub label: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    pub supported: bool,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TwoFactorProviderId {
    Authenticator,
    Email,
    Yubikey,
    Duo,
    Webauthn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginStage {
    Authenticating,
    Syncing,
    Decrypting,
}
impl LoginStage {
    pub fn label(self) -> &'static str {
        match self {
            Self::Authenticating => "Connecting and authenticating…",
            Self::Syncing => "Downloading encrypted vault…",
            Self::Decrypting => "Unlocking vault…",
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum LoginOutcome {
    Authenticated { snapshot: VaultSnapshot },
    TwoFactorRequired { providers: Vec<TwoFactorOption> },
}

#[derive(Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VaultItem {
    pub id: String,
    pub name: String,
    pub subtitle: String,
    pub kind: &'static str,
    pub favorite: bool,
    pub has_username: bool,
    pub has_password: bool,
    pub has_totp: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
    pub deleted: bool,
    pub archived: bool,
}

#[derive(Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VaultFolder {
    pub id: String,
    pub name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultSnapshot {
    pub email: String,
    pub items: Vec<VaultItem>,
    pub folders: Vec<VaultFolder>,
    pub decrypt_failures: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDetails {
    pub id: String,
    pub name: String,
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    pub uris: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub totp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cardholder_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_brand: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_exp_month: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_exp_year: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_code: Option<String>,
    pub favorite: bool,
    pub has_password: bool,
    pub has_totp: bool,
    pub has_card_number: bool,
    pub has_card_code: bool,
    pub editable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
    pub deleted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub middle_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passport_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_number: Option<String>,
    pub has_ssn: bool,
    pub has_passport_number: bool,
    pub has_license_number: bool,
    pub password_history: Vec<PasswordHistoryEntry>,
}

#[derive(Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PasswordHistoryEntry {
    pub last_used_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

#[derive(Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TotpCode {
    pub code: String,
    pub period: u32,
    pub expires_at: i64,
}

#[derive(Default)]
struct CapturedVault {
    ciphers: Vec<Cipher>,
    folders: Vec<bitwarden_vault::Folder>,
    parse_failures: usize,
    user_id: Option<uuid::Uuid>,
    organization_keys: Vec<(uuid::Uuid, String)>,
}

#[derive(Default)]
struct CipherCapture(RwLock<CapturedVault>);

#[async_trait]
impl SyncHandler for CipherCapture {
    async fn on_sync(&self, response: &SyncResponseModel) -> Result<(), SyncHandlerError> {
        let mut ciphers = Vec::new();
        let mut parse_failures = 0;
        for model in response.ciphers.as_deref().unwrap_or_default() {
            match Cipher::try_from(model.clone()) {
                Ok(cipher) => ciphers.push(cipher),
                Err(_) => parse_failures += 1,
            }
        }
        let mut folders = Vec::new();
        for model in response.folders.as_deref().unwrap_or_default() {
            match bitwarden_vault::Folder::try_from(model.clone()) {
                Ok(folder) => folders.push(folder),
                Err(_) => parse_failures += 1,
            }
        }
        *self.0.write().await = CapturedVault {
            ciphers,
            folders,
            parse_failures,
            user_id: response.profile.as_ref().and_then(|profile| profile.id),
            organization_keys: response
                .profile
                .as_ref()
                .and_then(|p| p.organizations.as_ref().or(p.organizations_new.as_ref()))
                .into_iter()
                .flatten()
                .filter_map(|org| Some((org.id?, org.key.clone()?)))
                .collect(),
        };
        Ok(())
    }
}

struct Session {
    email: String,
    client: PasswordManagerClient,
    sync: SyncClient,
    capture: Arc<CipherCapture>,
    last_activity: StdMutex<Instant>,
    icon_base_url: String,
}

struct PendingLogin {
    email: String,
    client: PasswordManagerClient,
    password: Zeroizing<String>,
    created_at: Instant,
    icon_base_url: String,
}

pub struct VaultService {
    session: Option<Session>,
    pending_login: Option<PendingLogin>,
    auto_lock_after: Duration,
    clipboard_seconds: u32,
    clipboard: std::sync::OnceLock<crate::clipboard::ClipboardService>,
}

impl Default for VaultService {
    fn default() -> Self {
        Self {
            session: None,
            pending_login: None,
            auto_lock_after: DEFAULT_AUTO_LOCK_AFTER,
            clipboard_seconds: 30,
            clipboard: std::sync::OnceLock::new(),
        }
    }
}

impl VaultService {
    pub fn configure_security(&mut self, settings: crate::security::SecuritySettings) {
        let settings = settings.validated();
        self.auto_lock_after = Duration::from_secs(u64::from(settings.auto_lock_minutes) * 60);
        self.clipboard_seconds = settings.clipboard_seconds;
        if let Some(clipboard) = self.clipboard.get() {
            clipboard.set_lifetime(settings.clipboard_seconds);
        }
    }
    pub fn note_activity(&mut self) -> bool {
        // Input cannot revive a session whose deadline has already elapsed.
        if self.lock_if_idle() {
            return true;
        }
        if let Some(session) = &self.session {
            touch(session);
        }
        false
    }
    pub async fn login(
        &mut self,
        request: LoginRequest,
        password: Zeroizing<String>,
    ) -> Result<LoginOutcome, String> {
        self.login_with_progress(request, password, |_| {}).await
    }

    pub async fn login_with_progress(
        &mut self,
        request: LoginRequest,
        password: Zeroizing<String>,
        progress: impl Fn(LoginStage) + Send + Sync,
    ) -> Result<LoginOutcome, String> {
        self.lock();
        let (settings, icon_base_url) = login_configuration(&request)?;
        progress(LoginStage::Authenticating);
        let client = diagnostics::client(settings);
        // SDK legacy decrypt silently replaces failed fields with null/empty.
        // A password client must report failures instead of showing damaged data.
        client
            .0
            .flags()
            .load(std::collections::HashMap::from([(
                "strict-cipher-decryption".into(),
                true,
            )]))
            .await;
        let email = request.email.trim().to_owned();

        let sdk_request = PasswordLoginRequest {
            email: email.clone(),
            password: password.to_string(),
            two_factor: None::<TwoFactorRequest>,
        };
        let login = isolated_password_login(&client, sdk_request).await?;
        if !login.authenticated {
            let providers = login
                .two_factor
                .ok_or_else(|| "The server did not authenticate this account.".to_string())?;
            let options = two_factor_options(&providers);
            self.pending_login = Some(PendingLogin {
                email,
                client,
                password,
                created_at: Instant::now(),
                icon_base_url,
            });
            return Ok(LoginOutcome::TwoFactorRequired { providers: options });
        }

        let snapshot = self
            .establish_session(email, client, icon_base_url, &progress)
            .await?;
        Ok(LoginOutcome::Authenticated { snapshot })
    }

    pub async fn submit_two_factor(
        &mut self,
        request: TwoFactorSubmitRequest,
    ) -> Result<LoginOutcome, String> {
        let mut pending = self
            .pending_login
            .take()
            .ok_or_else(|| "Two-step login has expired. Start again.".to_string())?;
        let provider = match parse_two_factor_provider(&request.provider) {
            Ok(provider) => provider,
            Err(error) => {
                self.pending_login = Some(pending);
                return Err(error);
            }
        };
        let token = request.token.trim();
        if token.is_empty() {
            self.pending_login = Some(pending);
            return Err("Enter the verification code.".into());
        }
        let sdk_request = PasswordLoginRequest {
            email: pending.email.clone(),
            password: pending.password.to_string(),
            two_factor: Some(TwoFactorRequest {
                token: token.to_owned(),
                provider,
                // Remembered-device tokens need secure persistence that this
                // prototype deliberately does not implement.
                remember: false,
            }),
        };
        let login = match isolated_password_login(&pending.client, sdk_request).await {
            Ok(login) => login,
            Err(_) => {
                self.pending_login = Some(pending);
                return Err("Verification failed. Check the code and try again.".into());
            }
        };
        if !login.authenticated {
            self.pending_login = Some(pending);
            return Err("Verification failed. Check the code and try again.".into());
        }
        let email = pending.email.clone();
        let client = pending.client;
        let icon_base_url = pending.icon_base_url;
        pending.password.zeroize();
        let snapshot = self
            .establish_session(email, client, icon_base_url, &|_| {})
            .await?;
        Ok(LoginOutcome::Authenticated { snapshot })
    }

    pub async fn send_two_factor_email(&self) -> Result<(), String> {
        let pending = self
            .pending_login
            .as_ref()
            .ok_or_else(|| "Two-step login has expired. Start again.".to_string())?;
        let mut request = TwoFactorEmailRequest {
            email: pending.email.clone(),
            password: pending.password.to_string(),
        };
        let result = pending
            .client
            .0
            .auth()
            .send_two_factor_email(&request)
            .await;
        request.password.zeroize();
        result.map_err(|_| "Could not send the email verification code.".to_string())
    }

    async fn establish_session(
        &mut self,
        email: String,
        client: PasswordManagerClient,
        icon_base_url: String,
        progress: &(dyn Fn(LoginStage) + Sync),
    ) -> Result<VaultSnapshot, String> {
        progress(LoginStage::Syncing);
        let sync_timing = diagnostics::Timing::new("initial_sync");
        let sync = client.sync();
        let capture = Arc::new(CipherCapture::default());
        // Crypto handles user/account state. Organization keys are installed
        // explicitly below: this SDK handler does not initialize them.
        sync.register_sync_handler(Arc::new(CryptoSyncHandler::new(client.0.clone())));
        sync.register_sync_handler(Arc::new(FolderSyncHandler::from_client(&client.0)));
        sync.register_sync_handler(capture.clone());
        sync.sync(SyncRequest {
            force: true,
            exclude_subdomains: None,
        })
        .await
        .map_err(|error| format!("Login succeeded, but the first vault sync failed: {error}"))?;
        let user_id =
            capture.0.read().await.user_id.ok_or_else(|| {
                "The sync response did not identify the signed-in user.".to_string()
            })?;
        client
            .0
            .internal
            .init_user_id(bitwarden_core::UserId::new(user_id))
            .await
            .map_err(|_| "The SDK rejected the synced user identity.".to_string())?;

        drop(sync_timing);
        progress(LoginStage::Decrypting);
        let _decrypt_timing = diagnostics::Timing::new("vault_decrypt");
        install_organization_keys(&client, &*capture.0.read().await).await?;
        self.session = Some(Session {
            email,
            client,
            sync,
            capture,
            last_activity: StdMutex::new(Instant::now()),
            icon_base_url,
        });
        self.pending_login = None;
        self.snapshot().await
    }

    pub async fn refresh(&self) -> Result<VaultSnapshot, String> {
        let session = self.session.as_ref().ok_or_else(locked)?;
        session
            .sync
            .sync(SyncRequest {
                force: true,
                exclude_subdomains: None,
            })
            .await
            .map_err(|error| format!("Vault sync failed: {error}"))?;
        self.snapshot().await
    }

    pub async fn snapshot(&self) -> Result<VaultSnapshot, String> {
        let session = self.session.as_ref().ok_or_else(locked)?;
        touch(session);
        let capture = session.capture.0.read().await;
        install_organization_keys(&session.client, &capture).await?;
        let result = session
            .client
            .vault()
            .ciphers()
            .decrypt_list_with_failures(capture.ciphers.clone())
            .await;
        let mut items = result
            .successes
            .into_iter()
            .filter_map(|view| to_vault_item(view, &session.icon_base_url))
            .collect::<Vec<_>>();
        items.sort_by_cached_key(|item| item.name.to_lowercase());
        let mut folders = session
            .client
            .0
            .vault()
            .folders()
            .list()
            .await
            .map_err(|error| format!("The SDK could not decrypt folders: {error}"))?
            .into_iter()
            .filter_map(|folder| {
                Some(VaultFolder {
                    id: folder.id?.to_string(),
                    name: folder.name,
                })
            })
            .collect::<Vec<_>>();
        folders.sort_by_cached_key(|folder| folder.name.to_lowercase());
        Ok(VaultSnapshot {
            email: session.email.clone(),
            items,
            folders,
            decrypt_failures: capture.parse_failures + result.failures.len(),
        })
    }

    pub async fn ssh_identities(&self) -> Result<Vec<crate::ssh_agent::KeyIdentity>, String> {
        use crate::ssh_agent::{KeyIdentity, ssh_key::PublicKey};
        let session = self.session.as_ref().ok_or_else(locked)?;
        let ciphers = session
            .capture
            .0
            .read()
            .await
            .ciphers
            .iter()
            .filter(|cipher| {
                cipher.r#type == CipherType::SshKey
                    && cipher.deleted_date.is_none()
                    && cipher.archived_date.is_none()
                    && cipher.reprompt == CipherRepromptType::None
            })
            .take(1024)
            .cloned()
            .collect::<Vec<_>>();
        let mut keys = Vec::new();
        for cipher in ciphers {
            let Ok(mut view) = session.client.vault().ciphers().decrypt(cipher).await else {
                continue;
            };
            if let Some(mut key) = view.ssh_key.take() {
                // Listing needs only public metadata. Erase decrypted private material now.
                key.private_key.zeroize();
                if !view.view_password {
                    continue;
                }
                if let (Some(id), Ok(public)) = (view.id, PublicKey::from_openssh(&key.public_key))
                {
                    keys.push(KeyIdentity {
                        id: id.to_string(),
                        name: view.name,
                        public,
                    });
                }
            }
        }
        Ok(keys)
    }
    pub async fn ssh_sign(
        &self,
        id: &str,
        request: &ssh_agent_lib::proto::SignRequest,
    ) -> Result<crate::ssh_agent::ssh_key::Signature, String> {
        let mut view = self.decrypt_one(id).await?;
        if !view.view_password
            || view.archived_date.is_some()
            || view.deleted_date.is_some()
            || view.reprompt != CipherRepromptType::None
            || view.r#type != CipherType::SshKey
        {
            return Err("This vault item cannot be used by the SSH agent.".into());
        }
        let key = view.ssh_key.take().ok_or("SSH key not found.")?;
        let pem = crate::protected::LockedSecret::from_string(Zeroizing::new(key.private_key))
            .map_err(|_| "Could not protect the SSH signing buffer in memory.")?;
        let private = crate::ssh_agent::ssh_key::PrivateKey::from_openssh(pem.as_bytes())
            .map_err(|_| "Could not parse the SSH private key.")?;
        drop(pem);
        crate::ssh_agent::sign_private(&private, request)
    }

    pub async fn password_history(
        &self,
        id: &str,
    ) -> Result<Vec<(i64, zeroize::Zeroizing<String>)>, String> {
        let view = self.decrypt_one(id).await?;
        if !view.view_password {
            return Err("This item does not allow password viewing.".into());
        }
        Ok(view
            .password_history
            .unwrap_or_default()
            .into_iter()
            .map(|entry| {
                (
                    entry.last_used_date.timestamp(),
                    zeroize::Zeroizing::new(entry.password),
                )
            })
            .collect())
    }

    pub async fn details(&self, id: &str) -> Result<ItemDetails, String> {
        self.item_details(id, false).await
    }

    pub async fn reveal_password(&self, id: &str) -> Result<Zeroizing<String>, String> {
        let view = self.decrypt_one(id).await?;
        if !view.view_password {
            return Err("This item does not allow password viewing.".into());
        }
        view.login
            .and_then(|login| login.password)
            .map(Zeroizing::new)
            .ok_or_else(|| "This item has no password.".into())
    }

    pub async fn totp_code(&self, id: &str) -> Result<TotpCode, String> {
        let view = self.decrypt_one(id).await?;
        totp_code_from_view(&view, Utc::now())
    }

    async fn item_details(&self, id: &str, include_secrets: bool) -> Result<ItemDetails, String> {
        let view = self.decrypt_one(id).await?;
        Ok(item_details_from_view(view, include_secrets))
    }

    async fn decrypt_one(&self, id: &str) -> Result<CipherView, String> {
        let session = self.session.as_ref().ok_or_else(locked)?;
        touch(session);
        let cipher = session
            .capture
            .0
            .read()
            .await
            .ciphers
            .iter()
            .find(|cipher| {
                cipher
                    .id
                    .is_some_and(|cipher_id| cipher_id.to_string() == id)
            })
            .cloned()
            .ok_or_else(|| "Vault item not found.".to_string())?;
        let view = session
            .client
            .vault()
            .ciphers()
            .decrypt(cipher)
            .await
            .map_err(|_| "The SDK could not decrypt this item.".to_string())?;
        if view.reprompt != CipherRepromptType::None {
            return Err("This item requires a master-password reprompt, which this reader does not support yet.".into());
        }
        Ok(view)
    }

    pub async fn copy_text(&self, text: Zeroizing<String>) -> Result<(), String> {
        self.clipboard
            .get_or_init(|| {
                crate::clipboard::ClipboardService::new(Duration::from_secs(u64::from(
                    self.clipboard_seconds,
                )))
            })
            .copy(text)
            .await
    }
    pub async fn copy_field(&self, id: &str, field: &str) -> Result<(), String> {
        let text = if field == "password" {
            self.reveal_password(id).await?
        } else {
            let view = self.decrypt_one(id).await?;
            Zeroizing::new(
                match field {
                    "username" => view.login.and_then(|login| login.username),
                    "notes" => view.notes,
                    _ => return Err("Choose a supported copy field.".into()),
                }
                .ok_or("This field is empty.")?,
            )
        };
        self.copy_text(text).await
    }

    pub fn lock(&mut self) {
        self.session = None;
        self.pending_login = None;
        if let Some(clipboard) = self.clipboard.get() {
            clipboard.clear();
        }
    }

    pub fn lock_if_idle(&mut self) -> bool {
        let expired = self.session.as_ref().is_some_and(|session| {
            session
                .last_activity
                .lock()
                .map(|activity| activity.elapsed() >= self.auto_lock_after)
                .unwrap_or(true)
        }) || self
            .pending_login
            .as_ref()
            .is_some_and(|pending| pending.created_at.elapsed() >= DEFAULT_AUTO_LOCK_AFTER);
        if expired {
            self.lock();
        }
        expired
    }

    pub fn is_locked(&self) -> bool {
        self.session.is_none()
    }
}
async fn install_organization_keys(
    client: &PasswordManagerClient,
    vault: &CapturedVault,
) -> Result<(), String> {
    let request = bitwarden_core::key_management::crypto::InitOrgCryptoRequest {
        organization_keys: vault
            .organization_keys
            .iter()
            .map(|(id, key)| {
                key.parse()
                    .map(|key| (bitwarden_core::OrganizationId::new(*id), key))
                    .map_err(|_| "The server returned an invalid organization key.".to_owned())
            })
            .collect::<Result<_, _>>()?,
    };
    client
        .crypto()
        .initialize_org_crypto(request)
        .await
        .map_err(|_| "The SDK could not initialize organization keys.".to_owned())
}

struct PasswordLoginRequestGuard(PasswordLoginRequest);

impl Drop for PasswordLoginRequestGuard {
    fn drop(&mut self) {
        self.0.password.zeroize();
    }
}

async fn isolated_password_login(
    client: &PasswordManagerClient,
    request: PasswordLoginRequest,
) -> Result<PasswordLoginResponse, String> {
    let _timing = diagnostics::Timing::new("password_login");
    let client = client.0.clone();
    let task = tokio::spawn(async move {
        let request = PasswordLoginRequestGuard(request);
        client.auth().login_password(&request.0).await
    });

    await_password_login_task(task).await
}

struct AbortLogin(tokio::task::AbortHandle);
impl Drop for AbortLogin {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn await_password_login_task(
    task: tokio::task::JoinHandle<Result<PasswordLoginResponse, LoginError>>,
) -> Result<PasswordLoginResponse, String> {
    let _abort_on_cancel = AbortLogin(task.abort_handle());
    match task.await {
        Ok(Ok(response)) => Ok(response),
        Ok(Err(error)) => Err(format!("Login failed: {error}{}", diagnostics::error_detail(&error))),
        Err(error) if error.is_panic() => Err(
            "The Bitwarden SDK could not interpret the server's login response. This account or server may not be compatible with the pinned SDK yet."
                .into(),
        ),
        Err(_) => Err("The Bitwarden SDK login task was cancelled.".into()),
    }
}

fn totp_code_from_view(view: &CipherView, now: chrono::DateTime<Utc>) -> Result<TotpCode, String> {
    let key = view
        .login
        .as_ref()
        .and_then(|login| login.totp.clone())
        .filter(|key| !key.is_empty())
        .ok_or_else(|| "This item does not contain an authenticator key.".to_string())?;
    let response = generate_totp(key, Some(now))
        .map_err(|_| "The authenticator key is invalid.".to_string())?;
    let response_period = response.period.max(1);
    let period = i64::from(response_period);
    let expires_at = (now.timestamp().div_euclid(period) + 1) * period;
    Ok(TotpCode {
        code: response.code,
        period: response_period,
        expires_at,
    })
}

fn item_details_from_view(view: CipherView, include_secrets: bool) -> ItemDetails {
    let login = view.login.as_ref();
    let card = view.card.as_ref();
    let identity = view.identity.as_ref();
    let revealed = |value: Option<&String>| {
        value.and_then(|value| {
            if value.is_empty() {
                None
            } else if include_secrets {
                Some(value.clone())
            } else {
                Some(last_chars(value, 4))
            }
        })
    };
    ItemDetails {
        id: view.id.map(|id| id.to_string()).unwrap_or_default(),
        name: view.name,
        kind: cipher_kind(view.r#type),
        username: login
            .and_then(|login| login.username.clone())
            .or_else(|| identity.and_then(|identity| identity.username.clone())),
        password: include_secrets
            .then(|| login.and_then(|login| login.password.clone()))
            .flatten(),
        uris: login
            .and_then(|login| login.uris.as_ref())
            .into_iter()
            .flatten()
            .filter_map(|uri| uri.uri.clone())
            .collect(),
        notes: view.notes.clone(),
        totp: include_secrets
            .then(|| login.and_then(|login| login.totp.clone()))
            .flatten(),
        cardholder_name: card.and_then(|card| card.cardholder_name.clone()),
        card_brand: card.and_then(|card| card.brand.clone()),
        card_number: revealed(card.and_then(|card| card.number.as_ref())),
        card_exp_month: card.and_then(|card| card.exp_month.clone()),
        card_exp_year: card.and_then(|card| card.exp_year.clone()),
        card_code: include_secrets
            .then(|| card.and_then(|card| card.code.clone()))
            .flatten(),
        favorite: view.favorite,
        has_password: login.and_then(|login| login.password.as_ref()).is_some(),
        has_totp: login.and_then(|login| login.totp.as_ref()).is_some(),
        has_card_number: card
            .and_then(|card| card.number.as_ref())
            .is_some_and(|number| !number.is_empty()),
        has_card_code: card
            .and_then(|card| card.code.as_ref())
            .is_some_and(|code| !code.is_empty()),
        editable: view.organization_id.is_none()
            && view.edit
            && view.deleted_date.is_none()
            && matches!(
                view.r#type,
                CipherType::Login
                    | CipherType::SecureNote
                    | CipherType::Card
                    | CipherType::Identity
                    | CipherType::SshKey
            ),
        folder_id: view.folder_id.map(|id| id.to_string()),
        deleted: view.deleted_date.is_some(),
        identity_title: identity.and_then(|identity| identity.title.clone()),
        first_name: identity.and_then(|identity| identity.first_name.clone()),
        middle_name: identity.and_then(|identity| identity.middle_name.clone()),
        last_name: identity.and_then(|identity| identity.last_name.clone()),
        address1: identity.and_then(|identity| identity.address1.clone()),
        address2: identity.and_then(|identity| identity.address2.clone()),
        address3: identity.and_then(|identity| identity.address3.clone()),
        city: identity.and_then(|identity| identity.city.clone()),
        state: identity.and_then(|identity| identity.state.clone()),
        postal_code: identity.and_then(|identity| identity.postal_code.clone()),
        country: identity.and_then(|identity| identity.country.clone()),
        company: identity.and_then(|identity| identity.company.clone()),
        email: identity.and_then(|identity| identity.email.clone()),
        phone: identity.and_then(|identity| identity.phone.clone()),
        ssn: revealed(identity.and_then(|identity| identity.ssn.as_ref())),
        passport_number: revealed(identity.and_then(|identity| identity.passport_number.as_ref())),
        license_number: revealed(identity.and_then(|identity| identity.license_number.as_ref())),
        has_ssn: identity
            .and_then(|identity| identity.ssn.as_ref())
            .is_some_and(|value| !value.is_empty()),
        has_passport_number: identity
            .and_then(|identity| identity.passport_number.as_ref())
            .is_some_and(|value| !value.is_empty()),
        has_license_number: identity
            .and_then(|identity| identity.license_number.as_ref())
            .is_some_and(|value| !value.is_empty()),
        password_history: view
            .password_history
            .iter()
            .flatten()
            .map(|entry| PasswordHistoryEntry {
                last_used_at: entry.last_used_date.timestamp(),
                password: include_secrets.then(|| entry.password.clone()),
            })
            .collect(),
    }
}

fn last_chars(value: &str, count: usize) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let start = chars.len().saturating_sub(count);
    chars[start..].iter().collect()
}

fn two_factor_options(providers: &TwoFactorProviders) -> Vec<TwoFactorOption> {
    let mut options = Vec::new();
    if providers.authenticator.is_some() {
        options.push(TwoFactorOption {
            id: TwoFactorProviderId::Authenticator,
            label: "Authenticator app",
            hint: None,
            supported: true,
        });
    }
    if let Some(email) = &providers.email {
        options.push(TwoFactorOption {
            id: TwoFactorProviderId::Email,
            label: "Email",
            hint: Some(email.email.clone()),
            supported: true,
        });
    }
    if providers.yubi_key.is_some() {
        options.push(TwoFactorOption {
            id: TwoFactorProviderId::Yubikey,
            label: "YubiKey OTP",
            hint: None,
            supported: true,
        });
    }
    if providers.duo.is_some() || providers.organization_duo.is_some() {
        options.push(TwoFactorOption {
            id: TwoFactorProviderId::Duo,
            label: "Duo",
            hint: None,
            supported: false,
        });
    }
    if providers.web_authn.is_some() {
        options.push(TwoFactorOption {
            id: TwoFactorProviderId::Webauthn,
            label: "WebAuthn",
            hint: None,
            supported: false,
        });
    }
    options
}

fn parse_two_factor_provider(provider: &str) -> Result<TwoFactorProvider, String> {
    match provider {
        "authenticator" => Ok(TwoFactorProvider::Authenticator),
        "email" => Ok(TwoFactorProvider::Email),
        "yubikey" => Ok(TwoFactorProvider::Yubikey),
        _ => Err("This two-step login provider is not supported yet.".into()),
    }
}

fn touch(session: &Session) {
    if let Ok(mut activity) = session.last_activity.lock() {
        *activity = Instant::now();
    }
}

fn to_vault_item(view: bitwarden_vault::CipherListView, icon_base_url: &str) -> Option<VaultItem> {
    let deleted = view.deleted_date.is_some();
    let id = view.id?.to_string();
    let (has_username, has_password, has_totp, icon_url) = match &view.r#type {
        CipherListViewType::Login(login) => (
            login
                .username
                .as_ref()
                .is_some_and(|value| !value.is_empty()),
            // The SDK intentionally keeps CopyableCipherFields private. The
            // detail view determines password availability after decrypting
            // the selected item.
            false,
            login.totp.is_some(),
            login.uris.as_ref().and_then(|uris| {
                uris.iter()
                    .find_map(|uri| uri.uri.as_deref().and_then(website_hostname))
                    .filter(|_| !icon_base_url.is_empty())
                    .map(|host| format!("{}/{host}/icon.png", icon_base_url.trim_end_matches('/')))
            }),
        ),
        _ => (false, false, false, None),
    };
    let folder_id = view.folder_id.map(|id| id.to_string());
    Some(VaultItem {
        id,
        name: view.name,
        subtitle: view.subtitle,
        kind: list_kind(&view.r#type),
        favorite: view.favorite,
        has_username,
        has_password,
        has_totp,
        icon_url,
        folder_id,
        deleted,
        archived: view.archived_date.is_some(),
    })
}

fn website_hostname(value: &str) -> Option<String> {
    let parsed = url::Url::parse(value)
        .or_else(|_| url::Url::parse(&format!("https://{value}")))
        .ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }

    let url::Host::Domain(host) = parsed.host()? else {
        return None;
    };
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    let private_suffixes = [
        "localhost",
        ".localhost",
        ".local",
        ".internal",
        ".lan",
        ".home",
        ".home.arpa",
        ".test",
        ".invalid",
        ".onion",
    ];
    (host.contains('.')
        && !private_suffixes
            .iter()
            .any(|suffix| host == *suffix || host.ends_with(suffix)))
    .then_some(host)
}

fn icon_base_url(region: &str, custom_server: Option<&str>) -> Result<String, String> {
    match region {
        "us" => Ok("https://icons.bitwarden.net".into()),
        "eu" => Ok("https://icons.bitwarden.eu".into()),
        "self-hosted" => Ok(crate::server::CustomServer::default()
            .resolve(custom_server)?
            .icons
            .unwrap_or_default()),
        _ => Err("Choose a supported Bitwarden region.".into()),
    }
}

fn list_kind(kind: &CipherListViewType) -> &'static str {
    match kind {
        CipherListViewType::Login(_) => "login",
        CipherListViewType::SecureNote => "note",
        CipherListViewType::Card(_) => "card",
        CipherListViewType::Identity => "identity",
        CipherListViewType::SshKey => "ssh-key",
        CipherListViewType::BankAccount(_) => "bank-account",
        CipherListViewType::Passport => "passport",
        CipherListViewType::DriversLicense => "drivers-license",
    }
}

fn cipher_kind(kind: CipherType) -> &'static str {
    match kind {
        CipherType::Login => "login",
        CipherType::SecureNote => "note",
        CipherType::Card => "card",
        CipherType::Identity => "identity",
        CipherType::SshKey => "ssh-key",
        CipherType::BankAccount => "bank-account",
        CipherType::DriversLicense => "drivers-license",
        CipherType::Passport => "passport",
    }
}

fn login_configuration(request: &LoginRequest) -> Result<(ClientSettings, String), String> {
    if request.region != "self-hosted" {
        return Ok((
            settings_for(&request.region, None)?,
            icon_base_url(&request.region, None)?,
        ));
    }
    let environment = request
        .server_config
        .as_ref()
        .cloned()
        .unwrap_or_default()
        .resolve(request.server_url.as_deref())?;
    let mut settings = settings_for("us", None)?;
    settings.api_url = environment.api;
    settings.identity_url = environment.identity;
    Ok((settings, environment.icons.unwrap_or_default()))
}

fn settings_for(region: &str, custom_server: Option<&str>) -> Result<ClientSettings, String> {
    let mut settings = ClientSettings {
        user_agent: "Warden Desktop/0.1".into(),
        device_type: desktop_device_type(),
        ..Default::default()
    };
    match region {
        "us" => {}
        "eu" => {
            settings.identity_url = "https://identity.bitwarden.eu".into();
            settings.api_url = "https://api.bitwarden.eu".into();
        }
        "self-hosted" => {
            let base = validate_server(custom_server.unwrap_or_default())?;
            settings.identity_url = format!("{base}/identity");
            settings.api_url = format!("{base}/api");
        }
        _ => return Err("Unknown Bitwarden region.".into()),
    }
    Ok(settings)
}

fn validate_server(raw: &str) -> Result<String, String> {
    crate::server::validate_url(raw)
}

const fn desktop_device_type() -> DeviceType {
    #[cfg(target_os = "windows")]
    return DeviceType::WindowsDesktop;
    #[cfg(target_os = "macos")]
    return DeviceType::MacOsDesktop;
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    return DeviceType::LinuxDesktop;
}

fn locked() -> String {
    "Vault is locked.".into()
}

#[async_trait]
impl crate::ssh_agent::VaultKeys for VaultService {
    async fn identities(&self) -> Result<Vec<crate::ssh_agent::KeyIdentity>, String> {
        self.ssh_identities().await
    }
    async fn sign(
        &self,
        id: &str,
        request: &ssh_agent_lib::proto::SignRequest,
    ) -> Result<crate::ssh_agent::ssh_key::Signature, String> {
        self.ssh_sign(id, request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn sync_replaces_organization_keys_including_removed_memberships() {
        let capture = CipherCapture::default();
        let id = uuid::Uuid::new_v4();
        let response: SyncResponseModel = serde_json::from_value(serde_json::json!({
            "profile": { "organizations": [{ "id": id, "key": "synthetic-encrypted-key" }] }
        }))
        .unwrap();
        capture.on_sync(&response).await.unwrap();
        assert_eq!(
            capture.0.read().await.organization_keys,
            vec![(id, "synthetic-encrypted-key".into())]
        );
        capture
            .on_sync(&SyncResponseModel::default())
            .await
            .unwrap();
        assert!(capture.0.read().await.organization_keys.is_empty());
    }

    #[test]
    fn pending_authentication_expires_and_lock_discards_it() {
        let mut service = VaultService {
            pending_login: Some(PendingLogin {
                email: "test@example.invalid".into(),
                client: PasswordManagerClient::new(None),
                password: Zeroizing::new("synthetic".into()),
                created_at: Instant::now() - Duration::from_secs(301),
                icon_base_url: String::new(),
            }),
            ..Default::default()
        };
        assert!(service.lock_if_idle());
        assert!(service.pending_login.is_none());
        assert!(service.is_locked());
    }
    fn idle_service(seconds: u64) -> VaultService {
        let client = PasswordManagerClient::new(None);
        VaultService {
            session: Some(Session {
                email: "test@example.invalid".into(),
                sync: client.sync(),
                client,
                capture: Arc::new(CipherCapture::default()),
                last_activity: StdMutex::new(Instant::now() - Duration::from_secs(seconds)),
                icon_base_url: String::new(),
            }),
            ..Default::default()
        }
    }
    #[test]
    fn activity_refreshes_a_live_session_but_cannot_revive_an_expired_session() {
        let settings = crate::security::SecuritySettings {
            auto_lock_minutes: 1,
            ..Default::default()
        };
        let mut service = idle_service(1);
        service.configure_security(settings);
        assert!(!service.note_activity());
        assert!(
            service
                .session
                .as_ref()
                .unwrap()
                .last_activity
                .lock()
                .unwrap()
                .elapsed()
                < Duration::from_secs(1)
        );
        let mut service = idle_service(61);
        service.configure_security(settings);
        assert!(service.note_activity());
        assert!(service.is_locked());
    }
    #[test]
    fn shortening_timeout_applies_to_the_existing_session() {
        let mut service = idle_service(61);
        assert!(!service.lock_if_idle());
        service.configure_security(crate::security::SecuritySettings {
            auto_lock_minutes: 1,
            ..Default::default()
        });
        assert!(service.lock_if_idle());
    }
    #[test]
    fn session_preferences_do_not_shorten_the_two_factor_challenge() {
        let mut service = VaultService {
            pending_login: Some(PendingLogin {
                email: "test@example.invalid".into(),
                client: PasswordManagerClient::new(None),
                password: Zeroizing::new("synthetic".into()),
                created_at: Instant::now() - Duration::from_secs(61),
                icon_base_url: String::new(),
            }),
            ..Default::default()
        };
        service.configure_security(crate::security::SecuritySettings {
            auto_lock_minutes: 1,
            ..Default::default()
        });
        assert!(!service.lock_if_idle());
        assert!(service.pending_login.is_some());
    }
    #[tokio::test]
    async fn cancelled_login_aborts_its_spawned_sdk_task() {
        struct Dropped(Arc<std::sync::atomic::AtomicBool>);
        impl Drop for Dropped {
            fn drop(&mut self) {
                self.0.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        }
        let dropped = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let observed = dropped.clone();
        let task = tokio::spawn(async move {
            let _guard = Dropped(observed);
            std::future::pending::<Result<PasswordLoginResponse, LoginError>>().await
        });
        tokio::task::yield_now().await;
        assert!(
            tokio::time::timeout(Duration::from_millis(1), await_password_login_task(task))
                .await
                .is_err()
        );
        tokio::task::yield_now().await;
        assert!(dropped.load(std::sync::atomic::Ordering::SeqCst));
    }
}
