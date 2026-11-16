//! Non-secret custom environment configuration shared by native UI and SDK setup.
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CustomServer {
    pub web_vault: Option<String>,
    pub api: Option<String>,
    pub identity: Option<String>,
    pub icons: Option<String>,
    pub notifications: Option<String>,
    pub events: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub web_vault: Option<String>,
    pub api: String,
    pub identity: String,
    pub icons: Option<String>,
    pub notifications: Option<String>,
    pub events: Option<String>,
}
impl CustomServer {
    pub fn resolve(&self, base: Option<&str>) -> Result<Environment, String> {
        let base = base
            .filter(|s| !s.trim().is_empty())
            .map(validate_url)
            .transpose()?;
        let endpoint =
            |value: &Option<String>, path: &str, label: &str| -> Result<Option<String>, String> {
                if let Some(value) = value.as_deref().filter(|s| !s.trim().is_empty()) {
                    validate_url(value)
                        .map(Some)
                        .map_err(|error| format!("{label}: {error}"))
                } else {
                    Ok(base.as_ref().map(|base| {
                        if path.is_empty() {
                            base.clone()
                        } else {
                            format!("{base}/{path}")
                        }
                    }))
                }
            };
        Ok(Environment {
            web_vault: endpoint(&self.web_vault, "", "Web vault URL")?,
            api: endpoint(&self.api, "api", "API URL")?
                .ok_or("Enter a Server URL or an API URL.")?,
            identity: endpoint(&self.identity, "identity", "Identity URL")?
                .ok_or("Enter a Server URL or an Identity URL.")?,
            icons: endpoint(&self.icons, "icons", "Icons URL")?,
            notifications: endpoint(&self.notifications, "notifications", "Notifications URL")?,
            events: endpoint(&self.events, "events", "Events URL")?,
        })
    }
}
pub fn validate_url(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    let raw = if raw.contains("://") {
        raw.to_owned()
    } else {
        format!("https://{raw}")
    };
    let mut url = Url::parse(&raw).map_err(|_| "Enter a valid server URL.".to_string())?;
    let local = url.host().is_some_and(|host| match host {
        url::Host::Domain(host) => host.eq_ignore_ascii_case("localhost"),
        url::Host::Ipv4(ip) => ip.is_loopback(),
        url::Host::Ipv6(ip) => ip.is_loopback(),
    });
    if url.host_str().is_none()
        || (url.scheme() != "https" && !(url.scheme() == "http" && local && cfg!(debug_assertions)))
    {
        return Err("Use HTTPS (local HTTP is allowed only in debug builds).".into());
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("URLs cannot contain credentials, a query or a fragment.".into());
    }
    let path = url.path().trim_end_matches('/').to_owned();
    url.set_path(&path);
    Ok(url.to_string().trim_end_matches('/').to_owned())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn base_paths_and_explicit_services_are_preserved() {
        let env = CustomServer {
            api: Some("https://api.example.test/v2/".into()),
            ..Default::default()
        }
        .resolve(Some("vault.example.test/bitwarden/"))
        .unwrap();
        assert_eq!(env.api, "https://api.example.test/v2");
        assert_eq!(
            env.identity,
            "https://vault.example.test/bitwarden/identity"
        );
        assert_eq!(
            env.icons.as_deref(),
            Some("https://vault.example.test/bitwarden/icons")
        );
        assert_eq!(
            env.web_vault.as_deref(),
            Some("https://vault.example.test/bitwarden")
        );
    }
    #[test]
    fn split_services_can_work_without_base() {
        let config = CustomServer {
            api: Some("api.example.test".into()),
            identity: Some("identity.example.test".into()),
            ..Default::default()
        };
        let env = config.resolve(None).unwrap();
        assert!(env.icons.is_none());
        assert!(CustomServer::default().resolve(None).is_err());
    }
    #[test]
    fn invalid_overrides_are_rejected_even_with_valid_base() {
        for url in [
            "http://example.test",
            "https://user:pass@example.test",
            "https://example.test?q=x",
            "https://example.test#x",
            "ftp://example.test",
        ] {
            assert!(
                CustomServer {
                    identity: Some(url.into()),
                    ..Default::default()
                }
                .resolve(Some("https://vault.example.test"))
                .is_err()
            );
        }
        assert_eq!(
            validate_url("http://[::1]:8080/subpath").is_ok(),
            cfg!(debug_assertions)
        );
        assert_eq!(
            validate_url("http://127.0.0.2:8080").is_ok(),
            cfg!(debug_assertions)
        );
    }
}
