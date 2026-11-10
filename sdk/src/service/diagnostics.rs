//! Opt-in diagnostics exclude URLs, headers, bodies and account identifiers.
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

fn enabled() -> bool {
    std::env::var_os("PLICOWARDEN_AUTH_TRACE").as_deref() == Some(std::ffi::OsStr::new("1"))
}

pub(super) fn client(settings: ClientSettings) -> PasswordManagerClient {
    PasswordManagerClient(
        bitwarden_core::ClientBuilder::new()
            .with_settings(settings)
            .with_token_handler(Arc::new(
                bitwarden_auth::token_management::PasswordManagerTokenHandler::default(),
            ))
            .with_middleware(vec![Arc::new(Transport)])
            .build(),
    )
}

pub(super) struct Timing(&'static str, Option<Instant>);
impl Timing {
    pub(super) fn new(stage: &'static str) -> Self {
        let start = enabled().then(Instant::now);
        if start.is_some() {
            tracing::info!(stage, event = "start", "authentication stage");
        }
        Self(stage, start)
    }
}
impl Drop for Timing {
    fn drop(&mut self) {
        if let Some(start) = self.1 {
            tracing::info!(
                stage = self.0,
                event = "end",
                elapsed_ms = start.elapsed().as_millis() as u64,
                "authentication stage ended (may indicate cancellation)"
            );
        }
    }
}

fn endpoint(path: &str) -> &'static str {
    if path.ends_with("/accounts/prelogin/password") {
        "prelogin"
    } else if path.ends_with("/connect/token") {
        "token"
    } else if path.ends_with("/sync") {
        "sync"
    } else {
        "other"
    }
}

fn transport_detail(error: &(dyn std::error::Error + 'static)) -> String {
    let mut detail = String::new();
    let mut current = Some(error);
    for _ in 0..16 {
        let Some(error) = current else { break };
        if let Some(error) = error.downcast_ref::<reqwest::Error>() {
            detail.push_str(&format!(
                " timeout={} connect={} body={} decode={}",
                error.is_timeout(),
                error.is_connect(),
                error.is_body(),
                error.is_decode()
            ));
        }
        if let Some(error) = error.downcast_ref::<std::io::Error>() {
            detail.push_str(&format!(
                " io_kind={:?} os_code={:?}",
                error.kind(),
                error.raw_os_error()
            ));
        }
        current = error.source();
    }
    // Never format arbitrary SDK errors: they can contain URLs or response bodies.
    if detail.is_empty() {
        " unclassified_transport_error".into()
    } else {
        detail
    }
}

pub(super) fn error_detail(error: &(dyn std::error::Error + 'static)) -> String {
    let detail = transport_detail(error);
    if detail.contains("unclassified_transport_error") {
        String::new()
    } else {
        format!(" (transport:{detail})")
    }
}

struct Transport;
#[async_trait]
impl reqwest_middleware::Middleware for Transport {
    async fn handle(
        &self,
        request: reqwest::Request,
        extensions: &mut http::Extensions,
        next: reqwest_middleware::Next<'_>,
    ) -> reqwest_middleware::Result<reqwest::Response> {
        if !enabled() {
            return next.run(request, extensions).await;
        }
        static IDS: AtomicU64 = AtomicU64::new(1);
        let id = IDS.fetch_add(1, Ordering::Relaxed);
        let endpoint = endpoint(request.url().path());
        let start = Instant::now();
        tracing::info!(request = id, endpoint, event = "start", "HTTP request");
        let result = next.run(request, extensions).await;
        match &result {
            Ok(response) => tracing::info!(
                request = id,
                endpoint,
                event = "headers",
                status = response.status().as_u16(),
                elapsed_ms = start.elapsed().as_millis() as u64,
                "HTTP response headers (body not yet read)"
            ),
            Err(error) => {
                tracing::warn!(request = id, endpoint, event = "failed", elapsed_ms = start.elapsed().as_millis() as u64, transport = %transport_detail(error), "HTTP transport failure")
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostics_only_expose_fixed_endpoints_and_transport_categories() {
        assert_eq!(endpoint("/identity/accounts/prelogin/password"), "prelogin");
        assert_eq!(endpoint("/identity/connect/token"), "token");
        assert_eq!(endpoint("/api/sync"), "sync");
        assert_eq!(endpoint("/api/accounts/private-account-id"), "other");
        let error = std::io::Error::new(
            std::io::ErrorKind::ConnectionReset,
            "sensitive-body-and-token",
        );
        let detail = transport_detail(&error);
        assert!(detail.contains("ConnectionReset"));
        assert!(!detail.contains("sensitive"));
    }
}
