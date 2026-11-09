use super::*;
use bitwarden_exporters::{ExportFormat, ExporterClientExt};
use bitwarden_importers::{ImportOptions, ImporterClientExt};
use std::{
    io::{Read, Write},
    path::Path,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    Json,
    EncryptedJson,
}
pub struct TransferResult {
    pub message: String,
    pub snapshot: Option<VaultSnapshot>,
}

fn read_limited(path: &Path, limit: u64) -> Result<Zeroizing<Vec<u8>>, String> {
    let file = std::fs::File::open(path).map_err(|_| "Could not open the selected file.")?;
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read the selected file.")?;
    if bytes.len() as u64 > limit {
        return Err("The selected file is too large (maximum 10 MiB).".into());
    }
    Ok(bytes)
}
fn write_new_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            "The file already exists. Choose a new filename.".to_owned()
        } else {
            "Could not create the export file.".to_owned()
        }
    })?;
    if file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .is_err()
    {
        drop(file);
        let _ = std::fs::remove_file(path);
        return Err("Could not finish writing the export file.".into());
    }
    Ok(())
}
impl VaultService {
    pub async fn export_file(
        &self,
        path: &Path,
        format: ExportKind,
        password: Zeroizing<String>,
    ) -> Result<TransferResult, String> {
        let session = self.session.as_ref().ok_or_else(locked)?;
        touch(session);
        if format == ExportKind::EncryptedJson && password.chars().count() < 8 {
            return Err("Use at least 8 characters for the export password.".into());
        }
        // Synchronize before selecting the personal-vault scope.
        self.refresh().await?;
        let capture = session.capture.0.read().await;
        if capture.parse_failures > 0 {
            return Err(
                "Some items could not be loaded. Sync successfully before exporting.".into(),
            );
        }
        let ciphers: Vec<_> = capture
            .ciphers
            .iter()
            .filter(|cipher| cipher.organization_id.is_none() && cipher.deleted_date.is_none())
            .cloned()
            .collect();
        let folders = capture.folders.clone();
        drop(capture);
        for cipher in &ciphers {
            let id = cipher.id.ok_or("An item has no ID. Export cancelled.")?;
            // The same password-view and reprompt checks as explicit reveal/copy.
            let view = self.decrypt_one(&id.to_string()).await?;
            if !matches!(
                view.r#type,
                CipherType::Login
                    | CipherType::SecureNote
                    | CipherType::Card
                    | CipherType::Identity
                    | CipherType::SshKey
            ) {
                return Err("This vault contains an item type that this exporter does not support yet. No file was written.".into());
            }
        }
        let exporter = session.client.0.exporters();
        let json = Zeroizing::new(
            exporter
                .export_vault(folders.clone(), ciphers.clone(), ExportFormat::Json)
                .await
                .map_err(|_| "The SDK could not export this vault.")?,
        );
        #[derive(Deserialize)]
        struct Count {
            items: Vec<serde::de::IgnoredAny>,
        }
        let count: Count =
            serde_json::from_str(&json).map_err(|_| "The SDK returned an invalid export.")?;
        if count.items.len() != ciphers.len() {
            return Err(
                "The SDK could not export every selected item. No file was written.".into(),
            );
        }
        let payload = if format == ExportKind::EncryptedJson {
            Zeroizing::new(
                exporter
                    .export_vault(
                        folders,
                        ciphers,
                        ExportFormat::EncryptedJson {
                            password: password.to_string(),
                        },
                    )
                    .await
                    .map_err(|_| "The SDK could not encrypt the export.")?,
            )
        } else {
            json
        };
        write_new_private(path, payload.as_bytes())?;
        Ok(TransferResult {
            message: format!("Exported {} personal vault items.", count.items.len()),
            snapshot: None,
        })
    }
    pub async fn import_kdbx_file(
        &self,
        path: &Path,
        password: Zeroizing<String>,
        key_file: Option<&Path>,
    ) -> Result<TransferResult, String> {
        let session = self.session.as_ref().ok_or_else(locked)?;
        touch(session);
        let bytes = read_limited(path, 10 * 1024 * 1024)?;
        let key = key_file
            .map(|path| read_limited(path, 10 * 1024 * 1024))
            .transpose()?;
        let result = session.client.0.importers().import_kdbx(bytes.to_vec(), Some(password.to_string()),
            key.as_ref().map(|key| key.to_vec()), ImportOptions { organization_id: None, target_folder: None,
                target_collection: None, restricted_types: Vec::new() }).await
            .map_err(|_| "KeePass import failed. Check the database password and key file, then sync before retrying to avoid duplicates.")?;
        let count: u32 = result.ciphers.iter().map(|count| count.count).sum();
        let snapshot = self.refresh().await.map_err(|error| {
            format!(
                "Imported {count} items but refresh failed: {error}. Sync before importing again."
            )
        })?;
        Ok(TransferResult {
            message: format!(
                "Imported {count} items and {} folders from KeePass.",
                result.folders
            ),
            snapshot: Some(snapshot),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn export_files_are_private_and_existing_files_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("synthetic.json");
        write_new_private(&path, b"synthetic-only").unwrap();
        assert!(write_new_private(&path, b"replace").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"synthetic-only");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert!(read_limited(&path, 4).is_err());
    }
    #[tokio::test]
    async fn locked_transfers_do_not_touch_files_or_network() {
        let service = VaultService::default();
        let path = Path::new("/not-a-real-vault");
        assert!(
            service
                .export_file(path, ExportKind::Json, Zeroizing::new(String::new()))
                .await
                .is_err()
        );
        assert!(
            service
                .import_kdbx_file(path, Zeroizing::new(String::new()), None)
                .await
                .is_err()
        );
    }
}
