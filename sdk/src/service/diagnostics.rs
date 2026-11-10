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
        if let Some(error) = error.downcast_ref::<reqwest_middleware::Error>() {
            current = Some(match error {
                reqwest_middleware::Error::Middleware(error) => error.as_ref(),
                reqwest_middleware::Error::Reqwest(error) => error,
            });
            continue;
        }
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
        let sync_download =
            request.method() == http::Method::GET && endpoint(request.url().path()) == "sync";
        if !sync_download {
            return send(request, extensions, next, false).await;
        }
        // Only the read-only sync download can be replayed. Fully consume the
        // body before returning, so handlers never see a partial first attempt.
        let retry = request.try_clone();
        let result = send(request, extensions, next.clone(), true).await;
        match (result, retry) {
            (Err(error), Some(request)) if retryable(&error) => {
                if enabled() {
                    tracing::warn!(
                        endpoint = "sync",
                        event = "retry",
                        "retrying read-only sync download once"
                    );
                }
                send(request, extensions, next, true).await
            }
            (result, _) => result,
        }
    }
}

fn retryable(error: &reqwest_middleware::Error) -> bool {
    match error {
        reqwest_middleware::Error::Reqwest(error) => {
            error.is_timeout() || error.is_connect() || error.is_body()
        }
        reqwest_middleware::Error::Middleware(error) => error.chain().any(|cause| {
            cause
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::TimedOut)
        }),
    }
}

fn stalled() -> reqwest_middleware::Error {
    reqwest_middleware::Error::middleware(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        "Sync download stopped making progress",
    ))
}

async fn send(
    request: reqwest::Request,
    extensions: &mut http::Extensions,
    next: reqwest_middleware::Next<'_>,
    read_body: bool,
) -> reqwest_middleware::Result<reqwest::Response> {
    static IDS: AtomicU64 = AtomicU64::new(1);
    let id = IDS.fetch_add(1, Ordering::Relaxed);
    let endpoint = endpoint(request.url().path());
    let start = Instant::now();
    let trace = enabled();
    if trace {
        tracing::info!(request = id, endpoint, event = "start", "HTTP request");
    }
    let result = async {
        let mut response = if read_body {
            tokio::time::timeout(Duration::from_secs(30), next.run(request, extensions))
                .await
                .map_err(|_| stalled())??
        } else {
            next.run(request, extensions).await?
        };
        if trace {
            tracing::info!(
                request = id,
                endpoint,
                event = "headers",
                status = response.status().as_u16(),
                elapsed_ms = start.elapsed().as_millis() as u64,
                "HTTP response headers (body not yet read)"
            );
        }
        if !read_body {
            return Ok(response);
        }
        use reqwest::ResponseBuilderExt;
        let mut builder = http::Response::builder()
            .status(response.status())
            .version(response.version())
            .url(response.url().clone());
        *builder.headers_mut().expect("valid response headers") = response.headers().clone();
        let mut bytes = Vec::new();
        while let Some(chunk) = tokio::time::timeout(Duration::from_secs(15), response.chunk())
            .await
            .map_err(|_| stalled())??
        {
            bytes.extend_from_slice(&chunk);
        }
        if trace {
            tracing::info!(
                request = id,
                endpoint,
                event = "body_complete",
                bytes = bytes.len(),
                elapsed_ms = start.elapsed().as_millis() as u64,
                "HTTP response body downloaded"
            );
        }
        Ok(builder.body(bytes).expect("valid buffered response").into())
    }
    .await;
    if let Err(error) = &result
        && trace
    {
        tracing::warn!(request = id, endpoint, event = "failed", elapsed_ms = start.elapsed().as_millis() as u64, transport = %transport_detail(error), "HTTP transport failure");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn read_headers(stream: &mut tokio::net::TcpStream) {
        use tokio::io::AsyncReadExt;
        let mut bytes = Vec::new();
        while !bytes.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).await.unwrap();
            bytes.push(byte[0]);
        }
    }

    #[tokio::test]
    async fn sync_retries_stalled_body_and_preserves_complete_response() {
        use tokio::{io::AsyncWriteExt, net::TcpListener};
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/api/sync", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut first, _) = listener.accept().await.unwrap();
            read_headers(&mut first).await;
            first
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n{")
                .await
                .unwrap();
            // Hold the first response open without finishing its advertised body.
            let (mut second, _) = listener.accept().await.unwrap();
            read_headers(&mut second).await;
            second.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}").await.unwrap();
            drop(first);
        });
        let _sdk = client(ClientSettings::default());
        let client = reqwest_middleware::ClientBuilder::new(reqwest::Client::new())
            .with(Transport)
            .build();
        let response = client
            .get(&url)
            .timeout(Duration::from_millis(100))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.url().as_str(), url);
        assert_eq!(response.headers()["content-type"], "application/json");
        assert_eq!(response.text().await.unwrap(), "{}");
        server.await.unwrap();
    }

    #[tokio::test]
    async fn failed_write_is_not_replayed() {
        use tokio::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/api/sync", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut first, _) = listener.accept().await.unwrap();
            read_headers(&mut first).await;
            drop(first);
            assert!(
                tokio::time::timeout(Duration::from_millis(150), listener.accept())
                    .await
                    .is_err()
            );
        });
        let _sdk = client(ClientSettings::default());
        let client = reqwest_middleware::ClientBuilder::new(reqwest::Client::new())
            .with(Transport)
            .build();
        assert!(client.post(url).send().await.is_err());
        server.await.unwrap();
    }

    #[test]
    fn explicit_idle_timeout_is_retryable_and_reported_without_sensitive_detail() {
        let error = stalled();
        assert!(retryable(&error));
        assert!(transport_detail(&error).contains("TimedOut"));
    }

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
