//! Per-account encrypted offline snapshots. Master passwords never reach disk.
use crate::service::LoginRequest;
use bitwarden_api_api::models::SyncResponseModel;
use bitwarden_core::{
    client::{
        login_method::UserLoginMethod,
        persisted_state::{AUTHENTICATION_TOKENS, AuthenticationTokens, USER_LOGIN_METHOD},
    },
    key_management::{KeySlotIds, MasterPasswordUnlockData, SymmetricKeySlotId},
};
use bitwarden_crypto::{Decryptable, EncString, KeyStore, PrimitiveEncryptable};
use bitwarden_crypto_sync_handler::CryptoSyncData;
use bitwarden_state::{registry::StateRegistry, settings::Setting};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use zeroize::{Zeroize, Zeroizing};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedAccount {
    pub id: String,
    pub config: LoginRequest,
    pub cached_at: i64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    version: u32,
    account: SavedAccount,
    unlock: MasterPasswordUnlockData,
    payload: EncString,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Payload {
    pub account: SavedAccount,
    pub sync: SyncResponseModel,
    pub tokens: AuthenticationTokens,
    pub method: UserLoginMethod,
}
impl Drop for Payload {
    fn drop(&mut self) {
        self.tokens.access_token.zeroize();
        if let Some(token) = &mut self.tokens.refresh_token {
            token.zeroize();
        }
        if let UserLoginMethod::ApiKey { client_secret, .. } = &mut self.method {
            client_secret.zeroize();
        }
    }
}
pub(crate) struct State {
    tokens: Setting<AuthenticationTokens>,
    method: Setting<UserLoginMethod>,
}
impl State {
    pub(crate) fn new(registry: &StateRegistry) -> Self {
        Self {
            tokens: registry
                .setting(AUTHENTICATION_TOKENS)
                .expect("registered token setting"),
            method: registry
                .setting(USER_LOGIN_METHOD)
                .expect("registered login method setting"),
        }
    }
    pub(crate) async fn restore(&self, payload: &Payload) -> Result<(), String> {
        self.tokens
            .update(payload.tokens.clone())
            .await
            .map_err(|_| "Could not restore cached session")?;
        self.method
            .update(payload.method.clone())
            .await
            .map_err(|_| "Could not restore cached login method")?;
        Ok(())
    }
    pub(crate) async fn access_token(&self) -> Option<Zeroizing<String>> {
        let mut tokens = self.tokens.get().await.ok().flatten()?;
        let access_token = Zeroizing::new(std::mem::take(&mut tokens.access_token));
        if let Some(refresh_token) = &mut tokens.refresh_token {
            refresh_token.zeroize();
        }
        Some(access_token)
    }
}
pub fn directory() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "Plicowarden")
        .map(|d| d.data_local_dir().join("accounts-v1"))
}
pub fn account_id(config: &LoginRequest) -> Result<String, String> {
    let (environment, _) = crate::service::login_configuration(config)?;
    // Include identity/API endpoints so the same email on different servers is isolated.
    let identity = format!(
        "{}\n{}\n{}",
        config.email.trim().to_lowercase(),
        environment.api_url,
        environment.identity_url
    );
    Ok(format!("{:x}", Sha256::digest(identity.as_bytes())))
}
fn db(root: &Path) -> Result<rusqlite::Connection, String> {
    std::fs::create_dir_all(root).map_err(|_| "Could not create account database directory")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| "Could not protect account database directory")?;
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .mode(0o600)
            .open(root.join("accounts.sqlite3"))
            .map_err(|_| "Could not create private account database")?;
    }
    let db = rusqlite::Connection::open(root.join("accounts.sqlite3"))
        .map_err(|_| "Could not open account database")?;
    db.busy_timeout(std::time::Duration::from_secs(2))
        .map_err(|_| "Could not configure account database")?;
    let version: i64 = db
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| "Could not inspect account database version")?;
    if version > 1 {
        return Err("Account database was written by a newer application version".into());
    }
    db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")
        .map_err(|_| "Could not configure account database")?;
    if version == 0 {
        db.execute_batch("BEGIN IMMEDIATE; CREATE TABLE IF NOT EXISTS accounts(id TEXT PRIMARY KEY, metadata TEXT NOT NULL, updated INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS vault_snapshots(account_id TEXT PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE, envelope TEXT NOT NULL); PRAGMA user_version=1; COMMIT;").map_err(|_|"Could not initialize account database")?;
    }
    Ok(db)
}
pub fn accounts(root: &Path) -> Result<Vec<SavedAccount>, String> {
    if !root.join("accounts.sqlite3").is_file() {
        return Ok(Vec::new());
    }
    let db = db(root)?;
    let mut stmt = db
        .prepare("SELECT metadata FROM accounts ORDER BY updated DESC")
        .map_err(|_| "Could not list accounts")?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|_| "Could not list accounts")?;
    let mut result = Vec::new();
    for row in rows {
        let value = row.map_err(|_| "Could not read account metadata")?;
        let account: SavedAccount =
            serde_json::from_str(&value).map_err(|_| "Saved account metadata is damaged")?;
        if account_id(&account.config)? != account.id {
            return Err("Saved account identity is damaged".into());
        }
        result.push(account);
    }
    Ok(result)
}
pub fn forget(root: &Path, id: &str) -> Result<(), String> {
    db(root)?
        .execute("DELETE FROM accounts WHERE id=?1", [id])
        .map_err(|_| "Could not remove saved account")?;
    Ok(())
}
pub(crate) fn load(
    root: &Path,
    config: &LoginRequest,
    password: &str,
) -> Result<Option<(Payload, MasterPasswordUnlockData)>, String> {
    let id = account_id(config)?;
    if !root.join("accounts.sqlite3").is_file() {
        return Ok(None);
    }
    use rusqlite::OptionalExtension;
    let encoded: Option<String> = db(root)?
        .query_row(
            "SELECT envelope FROM vault_snapshots WHERE account_id=?1",
            [&id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| "Could not read cached vault")?;
    let Some(encoded) = encoded else {
        return Ok(None);
    };
    if encoded.len() > 128 * 1024 * 1024 {
        return Err("Cached vault is too large".into());
    }
    let envelope: Envelope = serde_json::from_str(&encoded)
        .map_err(|_| "Cached vault is damaged; connect to the server again")?;
    if envelope.version != 1
        || envelope.account.id != id
        || account_id(&envelope.account.config)? != id
    {
        return Err("Cached account does not match this server".into());
    }
    let store = KeyStore::<KeySlotIds>::default();
    let plaintext: Zeroizing<String> = {
        let mut context = store.context();
        let key = envelope
            .unlock
            .unwrap_to_context(password, &mut context)
            .map_err(|_| "Could not unlock local vault: wrong master password or damaged cache")?;
        Zeroizing::new(
            envelope
                .payload
                .decrypt(&mut context, key)
                .map_err(|_| "Cached vault failed integrity verification")?,
        )
    };
    let payload: Payload =
        serde_json::from_str(&plaintext).map_err(|_| "Cached vault payload is damaged")?;
    if payload.account.id != id
        || account_id(&payload.account.config)? != id
        || payload.account != envelope.account
    {
        return Err("Cached account identity failed verification".into());
    }
    Ok(Some((payload, envelope.unlock)))
}
pub(crate) async fn save(
    root: &Path,
    config: &LoginRequest,
    client: &bitwarden_pm::PasswordManagerClient,
    state: &State,
    response: &SyncResponseModel,
) -> Result<(), String> {
    let data =
        CryptoSyncData::try_from(response).map_err(|_| "Could not read offline unlock material")?;
    let Some(unlock) = data.user_decryption.and_then(|d| d.master_password_unlock) else {
        return Err("This account does not provide offline master-password unlock material".into());
    };
    let account = SavedAccount {
        id: account_id(config)?,
        config: config.clone(),
        cached_at: chrono::Utc::now().timestamp(),
    };
    let payload = Payload {
        account: account.clone(),
        sync: response.clone(),
        tokens: state
            .tokens
            .get()
            .await
            .map_err(|_| "Could not read session tokens")?
            .ok_or("Missing session tokens")?,
        method: state
            .method
            .get()
            .await
            .map_err(|_| "Could not read login method")?
            .ok_or("Missing login method")?,
    };
    let plaintext = Zeroizing::new(
        serde_json::to_string(&payload).map_err(|_| "Could not encode cached vault")?,
    );
    let encrypted = plaintext
        .as_str()
        .encrypt(
            &mut client.0.internal.get_key_store().context(),
            SymmetricKeySlotId::User,
        )
        .map_err(|_| "Could not encrypt cached vault")?;
    let envelope = Envelope {
        version: 1,
        account: account.clone(),
        unlock,
        payload: encrypted,
    };
    store(root, &envelope)
}
fn store(root: &Path, envelope: &Envelope) -> Result<(), String> {
    let account = &envelope.account;
    let encoded = serde_json::to_string(envelope).map_err(|_| "Could not encode cache envelope")?;
    if encoded.len() > 128 * 1024 * 1024 {
        return Err("Encrypted cache exceeds the supported size".into());
    }
    let mut db = db(root)?;
    let transaction = db
        .transaction()
        .map_err(|_| "Could not start cache transaction")?;
    transaction.execute("INSERT INTO accounts(id,metadata,updated) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET metadata=excluded.metadata,updated=excluded.updated",rusqlite::params![account.id,serde_json::to_string(&account).map_err(|_|"Could not encode account metadata")?,account.cached_at]).map_err(|_|"Could not update saved account")?;
    transaction.execute("INSERT INTO vault_snapshots(account_id,envelope) VALUES(?1,?2) ON CONFLICT(account_id) DO UPDATE SET envelope=excluded.envelope",rusqlite::params![account.id,encoded]).map_err(|_|"Could not update encrypted vault")?;
    transaction
        .commit()
        .map_err(|_| "Could not commit encrypted vault cache")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitwarden_crypto::{Kdf, KeyEncryptable, MasterKey};
    fn config(server: &str) -> LoginRequest {
        LoginRequest {
            region: "self-hosted".into(),
            server_url: Some(server.into()),
            server_config: None,
            email: "fixture@example.invalid".into(),
        }
    }
    fn fixture(config: LoginRequest) -> Envelope {
        let kdf = Kdf::default_pbkdf2();
        let master = MasterKey::derive("synthetic-cache-password", &config.email, &kdf).unwrap();
        let (user, wrapped) = master.make_user_key().unwrap();
        let account = SavedAccount {
            id: account_id(&config).unwrap(),
            config: config.clone(),
            cached_at: 123,
        };
        let payload = Payload {
            account: account.clone(),
            sync: serde_json::from_value(
                serde_json::json!({"profile":{"name":"private-cached-marker"}}),
            )
            .unwrap(),
            tokens: AuthenticationTokens {
                access_token: "private-access-token-marker".into(),
                refresh_token: Some("private-refresh-marker".into()),
                expires_on: 123,
            },
            method: UserLoginMethod::Username {
                client_id: "desktop".into(),
                email: config.email.clone(),
                kdf: kdf.clone(),
            },
        };
        let plaintext = Zeroizing::new(serde_json::to_string(&payload).unwrap());
        Envelope {
            version: 1,
            account,
            unlock: MasterPasswordUnlockData {
                kdf,
                salt: config.email,
                master_key_wrapped_user_key: wrapped,
            },
            payload: plaintext.as_str().encrypt_with_key(&user.0).unwrap(),
        }
    }
    #[test]
    fn database_roundtrip_is_account_isolated_authenticated_and_contains_no_plaintext_tokens() {
        let root = tempfile::tempdir().unwrap();
        let a = config("https://one.example.invalid");
        let b = config("https://two.example.invalid");
        let first = fixture(a.clone());
        let second = fixture(b.clone());
        assert_ne!(first.account.id, second.account.id);
        store(root.path(), &first).unwrap();
        store(root.path(), &second).unwrap();
        assert_eq!(accounts(root.path()).unwrap().len(), 2);
        let conn = db(root.path()).unwrap();
        conn.execute_batch("CREATE TRIGGER fail_snapshot BEFORE INSERT ON vault_snapshots BEGIN SELECT RAISE(ABORT,'synthetic transaction failure'); END;").unwrap();
        let mut interrupted = fixture(a.clone());
        interrupted.account.cached_at = 999;
        assert!(store(root.path(), &interrupted).is_err());
        assert_eq!(
            accounts(root.path())
                .unwrap()
                .iter()
                .find(|account| account.id == first.account.id)
                .unwrap()
                .cached_at,
            123
        );
        conn.execute_batch("DROP TRIGGER fail_snapshot;").unwrap();
        drop(conn);

        assert!(load(root.path(), &a, "wrong-password").is_err());
        let (payload, _) = load(root.path(), &a, "synthetic-cache-password")
            .unwrap()
            .unwrap();
        assert_eq!(payload.tokens.access_token, "private-access-token-marker");
        assert!(
            load(
                root.path(),
                &config("https://missing.example.invalid"),
                "synthetic-cache-password"
            )
            .unwrap()
            .is_none()
        );
        // The main database and sidecars must not contain vault/profile/token plaintext.
        for file in std::fs::read_dir(root.path()).unwrap() {
            let bytes = std::fs::read(file.unwrap().path()).unwrap();
            for secret in [
                "private-cached-marker",
                "private-access-token-marker",
                "private-refresh-marker",
                "synthetic-cache-password",
            ] {
                assert!(
                    !bytes
                        .windows(secret.len())
                        .any(|window| window == secret.as_bytes())
                );
            }
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(root.path().join("accounts.sqlite3"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        let mut tampered = fixture(a.clone());
        tampered.account.cached_at = 456;
        store(root.path(), &tampered).unwrap();
        assert!(load(root.path(), &a, "synthetic-cache-password").is_err());
        forget(root.path(), &first.account.id).unwrap();
        assert_eq!(accounts(root.path()).unwrap().len(), 1);
        assert!(
            load(root.path(), &a, "synthetic-cache-password")
                .unwrap()
                .is_none()
        );
        assert!(
            load(root.path(), &b, "synthetic-cache-password")
                .unwrap()
                .is_some()
        );
        let conn = db(root.path()).unwrap();
        conn.pragma_update(None, "user_version", 2).unwrap();
        drop(conn);
        assert!(accounts(root.path()).is_err());
        let conn = rusqlite::Connection::open(root.path().join("accounts.sqlite3")).unwrap();
        assert_eq!(
            conn.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            2
        );
    }
}
