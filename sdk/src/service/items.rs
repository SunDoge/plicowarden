//! Personal-vault mutations. Patch existing payloads rather than rebuilding them:
//! URI match rules, passkeys, custom fields and future optional data survive edits.
use super::*;
use bitwarden_api_api::models::CipherRequestModel;
use bitwarden_vault::{FieldView, FolderAddEditRequest, FolderId, PasswordHistoryView};
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemPatch {
    pub kind: Option<String>,
    #[serde(default)]
    pub fields: BTreeMap<String, Zeroizing<String>>,
    pub favorite: Option<bool>,
    /// Empty string clears the folder; omission preserves it.
    pub folder_id: Option<String>,
    pub custom_fields: Option<Vec<FieldView>>,
    pub expected_revision: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mutation {
    pub snapshot: VaultSnapshot,
    pub id: Option<String>,
}
pub struct BatchFailure {
    pub id: String,
    pub message: String,
}
pub struct BatchMutation {
    pub snapshot: VaultSnapshot,
    pub succeeded: Vec<String>,
    pub failed: Vec<BatchFailure>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditData {
    pub id: String,
    pub kind: &'static str,
    pub revision: String,
    pub fields: BTreeMap<String, String>,
    pub favorite: bool,
    pub folder_id: Option<String>,
    pub custom_fields: Vec<FieldView>,
}
const LOGIN: &[&str] = &["username", "password", "totp"];
const CARD: &[&str] = &[
    "cardholderName",
    "brand",
    "number",
    "expMonth",
    "expYear",
    "code",
];
const IDENTITY: &[&str] = &[
    "title",
    "firstName",
    "middleName",
    "lastName",
    "address1",
    "address2",
    "address3",
    "city",
    "state",
    "postalCode",
    "country",
    "company",
    "email",
    "phone",
    "ssn",
    "username",
    "passportNumber",
    "licenseNumber",
];
const SSH: &[&str] = &["privateKey", "publicKey", "fingerprint"];

fn kind_fields(kind: CipherType) -> Result<(&'static str, &'static [&'static str]), String> {
    match kind {
        CipherType::Login => Ok(("login", LOGIN)),
        CipherType::SecureNote => Ok(("secureNote", &[])),
        CipherType::Card => Ok(("card", CARD)),
        CipherType::Identity => Ok(("identity", IDENTITY)),
        CipherType::SshKey => Ok(("sshKey", SSH)),
        _ => Err("This item type is read-only in this client.".into()),
    }
}
fn editable(view: &CipherView) -> Result<(), String> {
    if view.organization_id.is_some() || !view.edit || !view.view_password {
        return Err("This operation currently requires an editable personal item.".into());
    }
    kind_fields(view.r#type)?;
    Ok(())
}
fn set_folder_preserving_item(
    view: &mut CipherView,
    folder: Option<FolderId>,
) -> Result<bool, String> {
    editable(view)?;
    if view.deleted_date.is_some() {
        return Err("Restore this item before moving it.".into());
    }
    if view.reprompt != CipherRepromptType::None {
        return Err("This item requires a master-password reprompt.".into());
    }
    let changed = view.folder_id != folder;
    view.folder_id = folder;
    Ok(changed)
}
fn apply_patch(view: &mut CipherView, input: &ItemPatch) -> Result<(), String> {
    if let Some(kind) = &input.kind
        && kind != cipher_kind(view.r#type)
    {
        return Err("An existing item cannot change type.".into());
    }
    let (slot, allowed) = kind_fields(view.r#type)?;
    if input.fields.keys().any(|key| {
        !["name", "notes", "uris"].contains(&key.as_str()) && !allowed.contains(&key.as_str())
    }) {
        return Err("This item patch contains unsupported fields.".into());
    }
    if let Some(name) = input.fields.get("name") {
        if name.trim().is_empty() {
            return Err("Enter an item name.".into());
        }
        view.name = name.trim().to_owned();
    }
    if view.name.trim().is_empty() {
        return Err("Enter an item name.".into());
    }
    if let Some(notes) = input.fields.get("notes") {
        view.notes = Some(notes.to_string());
    }
    if let Some(favorite) = input.favorite {
        view.favorite = favorite;
    }
    if let Some(folder) = &input.folder_id {
        view.folder_id = if folder.is_empty() {
            None
        } else {
            Some(folder.parse().map_err(|_| "Invalid folder ID.")?)
        };
    }
    if let Some(fields) = &input.custom_fields {
        view.fields = Some(fields.clone());
    }
    let original_password = view.login.as_ref().and_then(|login| login.password.clone());
    let mut value = serde_json::to_value(&*view).map_err(|_| "Could not prepare item patch.")?;
    let payload = value.get_mut(slot).ok_or("Missing item payload.")?;
    if payload.is_null() {
        *payload = serde_json::json!({});
    }
    for key in allowed {
        if let Some(text) = input.fields.get(*key) {
            payload[*key] = serde_json::Value::String(text.to_string());
        }
    }
    if let Some(uris) = input.fields.get("uris") {
        if view.r#type != CipherType::Login {
            return Err("Only logins have website URIs.".into());
        }
        payload["uris"] = serde_json::Value::Array(
            uris.lines()
                .filter(|line| !line.trim().is_empty())
                .map(|uri| serde_json::json!({"uri": uri.trim()}))
                .collect(),
        );
    }
    *view = serde_json::from_value(value).map_err(|_| "The SDK rejected these item fields.")?;
    if let Some(new_password) = input.fields.get("password")
        && original_password.as_deref() != Some(new_password.as_str())
    {
        if let Some(previous) = original_password.filter(|p| !p.is_empty()) {
            let history = view.password_history.get_or_insert_with(Vec::new);
            history.insert(
                0,
                PasswordHistoryView {
                    password: previous,
                    last_used_date: Utc::now(),
                },
            );
            history.truncate(5);
        }
        if let Some(login) = &mut view.login {
            login.password_revision_date = Some(Utc::now());
        }
    }
    Ok(())
}
async fn submit(session: &Session, view: CipherView, create: bool) -> Result<String, String> {
    let id = view.id;
    let revision = (!create).then(|| view.revision_date.to_rfc3339());
    let encrypted = session
        .client
        .vault()
        .ciphers()
        .encrypt(view)
        .await
        .map_err(|_| "Could not encrypt item.")?;
    let mut model: CipherRequestModel = encrypted
        .cipher
        .try_into()
        .map_err(|_| "Could not encode item.")?;
    model.encrypted_for = Some(encrypted.encrypted_for.into());
    model.last_known_revision_date = revision;
    let config = session.client.0.internal.get_api_configurations();
    let response = if create {
        config.api_client.ciphers_api().post(Some(model)).await
    } else {
        config
            .api_client
            .ciphers_api()
            .put(id.ok_or("Missing item ID.")?.into(), Some(model))
            .await
    }
    .map_err(|_| "Item write failed; sync and retry if another client edited it.")?;
    Ok(response.id.ok_or("Missing written item ID.")?.to_string())
}
impl VaultService {
    pub async fn edit_data(&self, id: &str) -> Result<EditData, String> {
        let view = self.decrypt_one(id).await?;
        editable(&view)?;
        if view.deleted_date.is_some() {
            return Err("Restore this item first.".into());
        }
        let (slot, allowed) = kind_fields(view.r#type)?;
        let value = serde_json::to_value(&view).map_err(|_| "Could not prepare editor.")?;
        let mut fields = BTreeMap::from([
            ("name".into(), view.name.clone()),
            ("notes".into(), view.notes.clone().unwrap_or_default()),
        ]);
        for key in allowed {
            fields.insert(
                (*key).into(),
                value[slot][*key].as_str().unwrap_or_default().into(),
            );
        }
        if let Some(login) = &view.login {
            fields.insert(
                "uris".into(),
                login
                    .uris
                    .as_ref()
                    .into_iter()
                    .flatten()
                    .filter_map(|uri| uri.uri.as_deref())
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
        Ok(EditData {
            id: id.into(),
            kind: cipher_kind(view.r#type),
            revision: view.revision_date.to_rfc3339(),
            fields,
            favorite: view.favorite,
            folder_id: view.folder_id.map(|id| id.to_string()),
            custom_fields: view.fields.unwrap_or_default(),
        })
    }
    pub async fn save_item(&self, id: Option<&str>, input: ItemPatch) -> Result<Mutation, String> {
        let session = self.session.as_ref().ok_or_else(locked)?;
        touch(session);
        let mut view = if let Some(id) = id {
            let view = self.decrypt_one(id).await?;
            editable(&view)?;
            if view.deleted_date.is_some() {
                return Err("Restore this item first.".into());
            }
            if input.expected_revision.as_deref() != Some(view.revision_date.to_rfc3339().as_str())
            {
                return Err("The item changed; reopen the editor before saving.".into());
            }
            view
        } else {
            let mut view = super::writes::new_login();
            match input.kind.as_deref().unwrap_or("login") {
                "login" => {
                    view.login = Some(
                        serde_json::from_value(serde_json::json!({}))
                            .map_err(|_| "Invalid login defaults.")?,
                    );
                }
                "note" => {
                    view.r#type = CipherType::SecureNote;
                    view.secure_note = Some(bitwarden_vault::SecureNoteView {
                        r#type: bitwarden_vault::SecureNoteType::Generic,
                    });
                }
                "card" => {
                    view.r#type = CipherType::Card;
                    view.card = Some(
                        serde_json::from_value(serde_json::json!({}))
                            .map_err(|_| "Invalid card defaults.")?,
                    );
                }
                "identity" => {
                    view.r#type = CipherType::Identity;
                    view.identity = Some(
                        serde_json::from_value(serde_json::json!({}))
                            .map_err(|_| "Invalid identity defaults.")?,
                    );
                }
                "ssh-key" => {
                    view.r#type = CipherType::SshKey;
                    view.ssh_key = Some(bitwarden_vault::SshKeyView {
                        private_key: String::new(),
                        public_key: String::new(),
                        fingerprint: String::new(),
                    });
                }
                _ => return Err("Choose a supported item type.".into()),
            };
            view
        };
        apply_patch(&mut view, &input)?;
        let id = submit(session, view, id.is_none()).await?;
        Ok(Mutation {
            snapshot: self.refresh().await?,
            id: Some(id),
        })
    }
    async fn apply_item_action(&self, id: &str, action: &str) -> Result<Option<String>, String> {
        let session = self.session.as_ref().ok_or_else(locked)?;
        let mut view = self.decrypt_one(id).await?;
        editable(&view)?;
        let key = view.id.ok_or("Missing item ID.")?;
        let config = session.client.0.internal.get_api_configurations();
        let api = config.api_client.ciphers_api();
        match action {
            "trash" if view.deleted_date.is_none() => api
                .put_delete(key.into())
                .await
                .map(|_| ())
                .map_err(|_| "Could not move item to trash.")?,
            "restore" if view.deleted_date.is_some() => {
                api.put_restore(key.into())
                    .await
                    .map_err(|_| "Could not restore item.")?;
            }
            "purge" if view.deleted_date.is_some() => api
                .delete(key.into())
                .await
                .map_err(|_| "Could not permanently delete item.")?,
            "archive" if view.deleted_date.is_none() => {
                api.put_archive(key.into())
                    .await
                    .map_err(|_| "Could not archive item.")?;
            }
            "unarchive" if view.deleted_date.is_none() => {
                api.put_unarchive(key.into())
                    .await
                    .map_err(|_| "Could not unarchive item.")?;
            }
            "favorite" | "unfavorite" | "clone" if view.deleted_date.is_none() => {
                match action {
                    "favorite" => view.favorite = true,
                    "unfavorite" => view.favorite = false,
                    "archive" => view.archived_date = Some(Utc::now()),
                    "unarchive" => view.archived_date = None,
                    "clone" => {
                        view.id = None;
                        view.name.push_str(" (copy)");
                        view.attachments = None;
                        view.password_history = None;
                        if let Some(login) = &mut view.login {
                            login.fido2_credentials = None;
                        }
                    }
                    _ => unreachable!(),
                }
                let id = submit(session, view, action == "clone").await?;
                return Ok(Some(id));
            }
            _ => return Err("This action is not valid for the item's current state.".into()),
        }
        Ok((action == "restore").then(|| id.to_owned()))
    }
    pub async fn item_action(&self, id: &str, action: &str) -> Result<Mutation, String> {
        let id = self.apply_item_action(id, action).await?;
        Ok(Mutation {
            snapshot: self.refresh().await?,
            id,
        })
    }
    pub async fn batch_action(
        &self,
        ids: Vec<String>,
        action: &str,
    ) -> Result<BatchMutation, String> {
        self.session.as_ref().ok_or_else(locked)?;
        if !matches!(
            action,
            "favorite" | "unfavorite" | "archive" | "unarchive" | "trash" | "restore" | "purge"
        ) {
            return Err("Unsupported bulk action.".into());
        }
        let ids: std::collections::BTreeSet<_> = ids.into_iter().collect();
        if ids.is_empty() || ids.len() > 500 {
            return Err("Select between 1 and 500 items.".into());
        }
        // Check ownership/reprompt/read-only guards before any server mutation.
        for id in &ids {
            editable(&self.decrypt_one(id).await?)?;
        }
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();
        for id in ids {
            match self.apply_item_action(&id, action).await {
                Ok(_) => succeeded.push(id),
                Err(message) => failed.push(BatchFailure { id, message }),
            }
        }
        let snapshot = self.refresh().await.map_err(|error| {
            format!(
                "{} items updated, {} failed; refresh failed: {error}. Sync before retrying.",
                succeeded.len(),
                failed.len()
            )
        })?;
        Ok(BatchMutation {
            snapshot,
            succeeded,
            failed,
        })
    }
    pub async fn move_to_folder(
        &self,
        ids: Vec<String>,
        folder_id: Option<&str>,
    ) -> Result<BatchMutation, String> {
        let session = self.session.as_ref().ok_or_else(locked)?;
        touch(session);
        let ids: std::collections::BTreeSet<_> = ids.into_iter().collect();
        if ids.is_empty() || ids.len() > 500 {
            return Err("Select between 1 and 500 items.".into());
        }
        let folder = folder_id
            .filter(|id| !id.is_empty())
            .map(str::parse::<FolderId>)
            .transpose()
            .map_err(|_| "Invalid destination folder.")?;
        if let Some(folder) = folder {
            let folders = session
                .client
                .0
                .vault()
                .folders()
                .list()
                .await
                .map_err(|_| "Could not load folders.")?;
            if !folders.iter().any(|entry| entry.id == Some(folder)) {
                return Err(
                    "The destination folder no longer exists. Sync and choose again.".into(),
                );
            }
        }
        // Preserve complete decrypted views and their revision dates. Validate all
        // permissions before the first write; submit keeps revision preconditions.
        let mut prepared = Vec::new();
        for id in ids {
            let mut view = self.decrypt_one(&id).await?;
            let changed = set_folder_preserving_item(&mut view, folder)?;
            prepared.push((id, view, changed));
        }
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();
        for (id, view, changed) in prepared {
            if !changed {
                succeeded.push(id);
                continue;
            }
            match submit(session, view, false).await {
                Ok(_) => succeeded.push(id),
                Err(message) => failed.push(BatchFailure { id, message }),
            }
        }
        let snapshot = self.refresh().await.map_err(|error| {
            format!(
                "Moved {} items, {} failed; refresh failed: {error}. Sync before retrying.",
                succeeded.len(),
                failed.len()
            )
        })?;
        Ok(BatchMutation {
            snapshot,
            succeeded,
            failed,
        })
    }
    pub async fn folder_action(
        &self,
        id: Option<&str>,
        name: &str,
        delete: bool,
    ) -> Result<Mutation, String> {
        let session = self.session.as_ref().ok_or_else(locked)?;
        touch(session);
        let folders = session.client.0.vault().folders();
        if delete {
            let id = id.ok_or("Choose a folder.")?;
            id.parse::<FolderId>().map_err(|_| "Invalid folder ID.")?;
            session
                .client
                .0
                .internal
                .get_api_configurations()
                .api_client
                .folders_api()
                .delete(id)
                .await
                .map_err(|_| "Could not delete folder.")?;
        } else {
            if name.trim().is_empty() {
                return Err("Enter a folder name.".into());
            }
            let input = FolderAddEditRequest {
                name: name.trim().into(),
            };
            if let Some(id) = id {
                folders
                    .edit(id.parse().map_err(|_| "Invalid folder ID.")?, input)
                    .await
                    .map_err(|_| "Could not rename folder.")?;
            } else {
                folders
                    .create(input)
                    .await
                    .map_err(|_| "Could not create folder.")?;
            }
        }
        Ok(Mutation {
            snapshot: self.refresh().await?,
            id: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn login() -> CipherView {
        let mut view = super::super::writes::new_login();
        view.name = "Before".into();
        view.login = Some(
            serde_json::from_value(serde_json::json!({
                "username": "before", "password": "previous-password", "totp": "secret-totp",
                "uris": [{ "uri": "https://example.invalid", "match": 0 }],
                "autofillOnPageLoad": true
            }))
            .unwrap(),
        );
        view
    }
    fn patch(fields: &[(&str, &str)]) -> ItemPatch {
        ItemPatch {
            kind: Some("login".into()),
            fields: fields
                .iter()
                .map(|(key, value)| ((*key).into(), Zeroizing::new((*value).into())))
                .collect(),
            favorite: None,
            folder_id: None,
            custom_fields: None,
            expected_revision: None,
        }
    }
    #[test]
    fn folder_move_changes_only_folder_and_keeps_revision_and_all_payloads() {
        let mut view = login();
        let before = serde_json::to_value(&view).unwrap();
        let folder: FolderId = "00000000-0000-4000-8000-000000000001".parse().unwrap();
        assert!(set_folder_preserving_item(&mut view, Some(folder)).unwrap());
        let after = serde_json::to_value(&view).unwrap();
        for (key, value) in before.as_object().unwrap() {
            if key != "folderId" {
                assert_eq!(after.get(key), Some(value), "moving changed {key}");
            }
        }
        assert!(!set_folder_preserving_item(&mut view, Some(folder)).unwrap());
        assert!(set_folder_preserving_item(&mut view, None).unwrap());
        view.deleted_date = Some(Utc::now());
        assert!(set_folder_preserving_item(&mut view, Some(folder)).is_err());
        view.deleted_date = None;
        view.organization_id = Some("00000000-0000-4000-8000-000000000002".parse().unwrap());
        assert!(set_folder_preserving_item(&mut view, Some(folder)).is_err());
    }
    #[test]
    fn name_patch_preserves_login_configuration_and_sensitive_fields() {
        let mut view = login();
        let original = view.login.clone();
        apply_patch(&mut view, &patch(&[("name", "After")])).unwrap();
        assert_eq!(view.name, "After");
        assert_eq!(view.login, original);
    }
    #[test]
    fn password_patch_records_history_without_resetting_totp_or_uri_rules() {
        let mut view = login();
        let uris = view.login.as_ref().unwrap().uris.clone();
        apply_patch(&mut view, &patch(&[("password", "replacement")])).unwrap();
        let login = view.login.as_ref().unwrap();
        assert_eq!(login.password.as_deref(), Some("replacement"));
        assert_eq!(login.totp.as_deref(), Some("secret-totp"));
        assert_eq!(login.uris, uris);
        assert_eq!(
            view.password_history.as_ref().unwrap()[0].password,
            "previous-password"
        );
    }
    #[test]
    fn unknown_fields_and_type_changes_are_rejected() {
        let mut view = login();
        assert!(apply_patch(&mut view, &patch(&[("unknown", "invalid")])).is_err());
        let mut input = patch(&[("name", "After")]);
        input.kind = Some("card".into());
        assert!(apply_patch(&mut view, &input).is_err());
    }
    #[tokio::test]
    async fn locked_service_cannot_mutate_or_open_an_editor() {
        let service = VaultService::default();
        assert!(
            service
                .save_item(None, patch(&[("name", "synthetic")]))
                .await
                .is_err()
        );
        assert!(service.item_action("id", "purge").await.is_err());
        assert!(
            service
                .folder_action(None, "synthetic", false)
                .await
                .is_err()
        );
        assert!(service.edit_data("id").await.is_err());
    }
}

#[cfg(test)]
mod batch_tests {
    use super::*;
    #[tokio::test]
    async fn locked_bulk_actions_fail_before_network_or_partial_writes() {
        let service = VaultService::default();
        assert!(
            service
                .move_to_folder(vec!["synthetic-item".into()], None)
                .await
                .is_err()
        );
        for action in [
            "trash",
            "restore",
            "purge",
            "favorite",
            "archive",
            "unsupported",
        ] {
            assert!(
                service
                    .batch_action(vec!["synthetic-item".into()], action)
                    .await
                    .is_err()
            );
        }
    }
}
