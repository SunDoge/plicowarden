//! Opt-in live regression: credentials arrive on stdin; the database is temporary.
//! PLICOWARDEN_TEST_SERVER is required. PLICOWARDEN_TEST_WRITE=1 additionally
//! creates one synthetic fixture and removes it even if offline checks fail.
use plicowarden_sdk::service::{ItemPatch, LoginOutcome, LoginRequest, VaultService};
use snafu::{ResultExt, Snafu};
use std::collections::BTreeMap;
use zeroize::Zeroizing;
#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("Could not read test credentials: {source}"))]
    Input { source: std::io::Error },
    #[snafu(display("{message}"))]
    Probe { message: String },
}
async fn remove_fixture(online: &VaultService, name: &str) -> Result<(), String> {
    for _ in 0..4 {
        let snapshot = online.refresh().await?;
        let Some(item) = snapshot.items.iter().find(|item| item.name == name) else {
            return Ok(());
        };
        if !item.deleted {
            let _ = online.item_action(&item.id, "trash").await;
        } else {
            let _ = online.item_action(&item.id, "purge").await;
        }
    }
    if online
        .refresh()
        .await?
        .items
        .iter()
        .any(|item| item.name == name)
    {
        return Err(format!(
            "Synthetic fixture cleanup requires attention: {name}"
        ));
    }
    Ok(())
}
#[tokio::main]
async fn main() -> Result<(), Error> {
    let server = std::env::var("PLICOWARDEN_TEST_SERVER").map_err(|_| Error::Probe {
        message: "Set PLICOWARDEN_TEST_SERVER; use a dedicated authorized test account".into(),
    })?;
    let mut email = String::new();
    let mut password = Zeroizing::new(String::new());
    std::io::stdin().read_line(&mut email).context(InputSnafu)?;
    std::io::stdin()
        .read_line(&mut password)
        .context(InputSnafu)?;
    while password.ends_with(['\r', '\n']) {
        password.pop();
    }
    let config = LoginRequest {
        email: email.trim().into(),
        region: "self-hosted".into(),
        server_url: Some(server),
        server_config: None,
    };
    let root = tempfile::tempdir().context(InputSnafu)?;
    let mut online = VaultService::default().with_account_cache(Some(root.path().into()));
    let authenticated = online
        .login(config.clone(), password.clone())
        .await
        .map_err(|message| Error::Probe { message })?;
    let LoginOutcome::Authenticated { snapshot } = authenticated else {
        return Err(Error::Probe {
            message: "Test account requires 2FA; this probe does not alter it".into(),
        });
    };
    if let Some(message) = snapshot.cache_warning {
        return Err(Error::Probe { message });
    }
    let mut fixture = None;
    let fixture_name = format!("plicowarden-cache-probe-{}", uuid::Uuid::new_v4());
    if std::env::var("PLICOWARDEN_TEST_WRITE").as_deref() == Ok("1") {
        let patch = ItemPatch {
            kind: Some("login".into()),
            fields: BTreeMap::from([
                ("name".into(), Zeroizing::new(fixture_name.clone())),
                (
                    "username".into(),
                    Zeroizing::new("cache-fixture-user".into()),
                ),
                (
                    "password".into(),
                    Zeroizing::new("synthetic-cache-secret-771".into()),
                ),
                (
                    "notes".into(),
                    Zeroizing::new("离线缓存 项目 marker-notes".into()),
                ),
            ]),
            favorite: None,
            folder_id: None,
            custom_fields: None,
            expected_revision: None,
        };
        match online.save_item(None, patch).await {
            Ok(result) => fixture = result.id,
            Err(message) => {
                remove_fixture(&online, &fixture_name)
                    .await
                    .map_err(|message| Error::Probe { message })?;
                return Err(Error::Probe { message });
            }
        }
    }
    let work = async {
        assert_eq!(plicowarden_sdk::cache::accounts(root.path())?.len(), 1);
        let mut offline = VaultService::default().with_account_cache(Some(root.path().into()));
        assert!(
            offline
                .login(
                    config.clone(),
                    Zeroizing::new("wrong-cache-password".into())
                )
                .await
                .is_err()
        );
        let start = std::time::Instant::now();
        let LoginOutcome::Authenticated { snapshot } =
            offline.login(config.clone(), password.clone()).await?
        else {
            return Err("Offline unlock requested 2FA".into());
        };
        assert!(snapshot.offline);
        assert_eq!(snapshot.decrypt_failures, 0);
        println!(
            "PASS fresh-client local unlock elapsed_ms={} items={}",
            start.elapsed().as_millis(),
            snapshot.items.len()
        );
        if let Some(id) = &fixture {
            assert!(snapshot.items.iter().any(|item| &item.id == id));
            assert_eq!(
                offline.reveal_password(id).await?.as_str(),
                "synthetic-cache-secret-771"
            );
            assert!(offline.search("缓存 marker-notes").await?.contains(id));
            assert!(
                offline
                    .search("cache-fixture-user 项目")
                    .await?
                    .contains(id)
            );
            assert!(
                !offline
                    .search("synthetic-cache-secret-771")
                    .await?
                    .contains(id)
            );
            println!("PASS encrypted fixture read, notes/CJK search and password-field exclusion");
        }
        let snapshot = offline.refresh().await?;
        assert!(!snapshot.offline);
        assert_eq!(snapshot.decrypt_failures, 0);
        println!("PASS cached tokens authenticate a fresh online sync");
        offline.lock();
        assert!(offline.search("marker-notes").await.is_err());
        Ok::<_, String>(())
    };
    let result = tokio::time::timeout(std::time::Duration::from_secs(120), work)
        .await
        .unwrap_or_else(|_| Err("Cache probe timed out".into()));
    if fixture.is_some() {
        remove_fixture(&online, &fixture_name)
            .await
            .map_err(|message| Error::Probe { message })?;
        println!("PASS synthetic fixture removed");
    }

    result.map_err(|message| Error::Probe { message })?;
    println!("CACHE_ROUNDTRIP_COMPLETE; temporary SQLite database removed on exit");
    Ok(())
}
