// SDK encryption/API transport adapted from Warden Desktop (GPL-3.0-only).
use super::*;
use bitwarden_api_api::models::CipherRequestModel;
use bitwarden_vault::{LoginView, PasswordHistoryView};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoginWrite {
    pub name: String,
    pub username: String,
    pub password: Zeroizing<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteResult {
    pub id: String,
}

impl VaultService {
    pub async fn write_login(
        &self,
        id: Option<&str>,
        input: LoginWrite,
    ) -> Result<WriteResult, String> {
        let session = self.session.as_ref().ok_or_else(locked)?;
        if input.name.trim().is_empty() {
            return Err("Enter an item name.".into());
        }
        let mut view = match id {
            Some(id) => self.decrypt_one(id).await?,
            None => new_login(),
        };
        if view.organization_id.is_some()
            || !view.edit
            || !view.view_password
            || view.deleted_date.is_some()
            || view.r#type != CipherType::Login
        {
            return Err("Only editable personal login items are supported.".into());
        }
        if let Some(previous) = view.login.as_ref().and_then(|l| l.password.as_ref())
            && previous != input.password.as_str()
            && !previous.is_empty()
        {
            let history = view.password_history.get_or_insert_with(Vec::new);
            history.insert(
                0,
                PasswordHistoryView {
                    password: previous.clone(),
                    last_used_date: Utc::now(),
                },
            );
            history.truncate(5);
        }
        view.name = input.name.trim().to_owned();
        let login = view.login.get_or_insert(LoginView {
            username: None,
            password: None,
            password_revision_date: None,
            uris: None,
            totp: None,
            autofill_on_page_load: None,
            fido2_credentials: None,
        });
        if login.password.as_deref() != Some(input.password.as_str()) {
            login.password_revision_date = Some(Utc::now());
        }
        login.username = Some(input.username);
        login.password = Some(input.password.to_string());
        let previous_revision = id.map(|_| view.revision_date.to_rfc3339());
        let encrypted = session
            .client
            .vault()
            .ciphers()
            .encrypt(view)
            .await
            .map_err(|_| "The SDK could not encrypt this item.".to_owned())?;
        let mut model: CipherRequestModel = encrypted
            .cipher
            .try_into()
            .map_err(|_| "The SDK could not encode this item.".to_owned())?;
        model.last_known_revision_date = previous_revision;
        model.encrypted_for = Some(encrypted.encrypted_for.into());
        let config = session.client.0.internal.get_api_configurations();
        let result = match id {
            Some(id) => {
                config
                    .api_client
                    .ciphers_api()
                    .put(id.parse().map_err(|_| "Invalid item ID.")?, Some(model))
                    .await
            }
            None => config.api_client.ciphers_api().post(Some(model)).await,
        }
        .map_err(|_| "The server rejected the item write.".to_owned())?;
        self.refresh().await?;
        Ok(WriteResult {
            id: result.id.ok_or("Missing written item ID.")?.to_string(),
        })
    }

    pub async fn trash_login(&self, id: &str) -> Result<VaultSnapshot, String> {
        let session = self.session.as_ref().ok_or_else(locked)?;
        let view = self.decrypt_one(id).await?;
        if view.organization_id.is_some()
            || !view.edit
            || !view.view_password
            || view.deleted_date.is_some()
            || view.r#type != CipherType::Login
        {
            return Err("Only editable personal login items can be trashed.".into());
        }
        let config = session.client.0.internal.get_api_configurations();
        config
            .api_client
            .ciphers_api()
            .put_delete(view.id.ok_or("Missing item ID.")?.into())
            .await
            .map_err(|_| "The server rejected moving this item to trash.".to_owned())?;
        self.refresh().await
    }
}
pub(super) fn new_login() -> CipherView {
    let now = Utc::now();
    CipherView {
        id: None,
        organization_id: None,
        folder_id: None,
        collection_ids: Vec::new(),
        key: None,
        name: String::new(),
        notes: None,
        r#type: CipherType::Login,
        login: None,
        identity: None,
        card: None,
        secure_note: None,
        ssh_key: None,
        bank_account: None,
        drivers_license: None,
        passport: None,
        favorite: false,
        reprompt: CipherRepromptType::None,
        organization_use_totp: false,
        edit: true,
        permissions: None,
        view_password: true,
        local_data: None,
        attachments: None,
        attachment_decryption_failures: None,
        fields: Some(Vec::new()),
        password_history: None,
        creation_date: now,
        deleted_date: None,
        revision_date: now,
        archived_date: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn locked_vault_rejects_every_write() {
        let service = VaultService::default();
        for id in [None, Some("00000000-0000-0000-0000-000000000001")] {
            let input = LoginWrite {
                name: "synthetic".into(),
                username: "user".into(),
                password: Zeroizing::new("synthetic".into()),
            };
            assert_eq!(
                service.write_login(id, input).await.err().as_deref(),
                Some("Vault is locked.")
            );
        }
        assert_eq!(
            service
                .trash_login("00000000-0000-0000-0000-000000000001")
                .await
                .err()
                .as_deref(),
            Some("Vault is locked.")
        );
    }
}
