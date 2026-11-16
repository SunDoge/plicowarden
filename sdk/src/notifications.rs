//! Authenticated Bitwarden SignalR wakeups. Payloads never replace API reads.
use crate::cache;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest, http::header},
};
use zeroize::Zeroizing;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Signal {
    Connected,
    AuthRequest,
}

pub(crate) struct Task {
    pub(crate) receiver: mpsc::Receiver<Signal>,
    handle: tokio::task::JoinHandle<()>,
}
impl Drop for Task {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

pub(crate) fn spawn(url: String, state: Arc<cache::State>) -> Task {
    let (sender, receiver) = mpsc::channel(8);
    let handle = tokio::spawn(async move {
        let mut retry = std::time::Duration::from_secs(5);
        loop {
            let Some(token) = state.access_token().await else {
                tokio::time::sleep(retry).await;
                retry = (retry * 2).min(std::time::Duration::from_secs(60));
                continue;
            };
            if run_connection(&url, token, &sender).await.is_ok() {
                retry = std::time::Duration::from_secs(5);
            }
            tokio::time::sleep(retry).await;
            retry = (retry * 2).min(std::time::Duration::from_secs(60));
        }
    });
    Task { receiver, handle }
}

async fn run_connection(
    url: &str,
    token: Zeroizing<String>,
    sender: &mpsc::Sender<Signal>,
) -> Result<(), ()> {
    let websocket_url = websocket_url(url)?;
    let mut request = websocket_url.into_client_request().map_err(|_| ())?;
    let authorization = Zeroizing::new(format!("Bearer {}", token.as_str()));
    request.headers_mut().insert(
        header::AUTHORIZATION,
        authorization.as_str().parse().map_err(|_| ())?,
    );
    drop(token);
    let (mut socket, _) = connect_async(request).await.map_err(|_| ())?;
    socket
        .send(Message::Text(
            "{\"protocol\":\"messagepack\",\"version\":1}\u{1e}".into(),
        ))
        .await
        .map_err(|_| ())?;
    let Some(handshake) = socket.next().await else {
        return Err(());
    };
    match handshake.map_err(|_| ())? {
        Message::Text(value) if value.contains("{}") && !value.contains("error") => {}
        _ => return Err(()),
    }
    sender.send(Signal::Connected).await.map_err(|_| ())?;
    while let Some(message) = socket.next().await {
        match message.map_err(|_| ())? {
            Message::Binary(bytes) => {
                if contains_auth_request(&bytes) {
                    sender.send(Signal::AuthRequest).await.map_err(|_| ())?;
                }
            }
            Message::Ping(bytes) => socket.send(Message::Pong(bytes)).await.map_err(|_| ())?,
            Message::Close(_) => return Ok(()),
            _ => {}
        }
    }
    Ok(())
}

fn websocket_url(url: &str) -> Result<String, ()> {
    let mut url = url::Url::parse(url).map_err(|_| ())?;
    let scheme = match url.scheme() {
        "https" => "wss",
        "http" => "ws",
        "wss" | "ws" => return Ok(url.into()),
        _ => return Err(()),
    };
    url.set_scheme(scheme).map_err(|_| ())?;
    Ok(url.into())
}

fn contains_auth_request(mut bytes: &[u8]) -> bool {
    while !bytes.is_empty() {
        let Some((length, prefix)) = frame_length(bytes) else {
            return false;
        };
        let Some(frame) = bytes.get(prefix..prefix.saturating_add(length)) else {
            return false;
        };
        if is_auth_request(frame) {
            return true;
        }
        bytes = &bytes[prefix + length..];
    }
    false
}

fn frame_length(bytes: &[u8]) -> Option<(usize, usize)> {
    let mut value = 0usize;
    for (index, byte) in bytes.iter().copied().take(5).enumerate() {
        value |= usize::from(byte & 0x7f) << (index * 7);
        if byte & 0x80 == 0 {
            return Some((value, index + 1));
        }
    }
    None
}

fn is_auth_request(frame: &[u8]) -> bool {
    let Ok(Value::Array(message)) = rmp_serde::from_slice::<Value>(frame) else {
        return false;
    };
    if message.get(3).and_then(Value::as_str) != Some("ReceiveMessage") {
        return false;
    }
    message
        .get(4)
        .and_then(Value::as_array)
        .and_then(|arguments| arguments.first())
        .and_then(Value::as_object)
        .and_then(|notification| {
            notification
                .get("type")
                .or_else(|| notification.get("Type"))
        })
        .and_then(Value::as_u64)
        == Some(15)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(value: Value) -> Vec<u8> {
        let payload = rmp_serde::to_vec(&value).unwrap();
        let mut length = payload.len();
        let mut result = Vec::new();
        loop {
            let mut byte = (length & 0x7f) as u8;
            length >>= 7;
            if length > 0 {
                byte |= 0x80;
            }
            result.push(byte);
            if length == 0 {
                break;
            }
        }
        result.extend(payload);
        result
    }

    #[test]
    fn signalr_messagepack_framing_only_wakes_for_auth_requests() {
        let ping = frame(serde_json::json!([6]));
        let sync = frame(serde_json::json!([1, {}, null, "ReceiveMessage", [{"type": 0}]]));
        let auth = frame(serde_json::json!([1, {}, null, "ReceiveMessage", [{"Type": 15}]]));
        assert!(!contains_auth_request(&ping));
        assert!(!contains_auth_request(&sync));
        assert!(contains_auth_request(&auth));
        assert!(contains_auth_request(&[ping, auth].concat()));
        assert!(!contains_auth_request(&[0x81]));
    }

    #[test]
    fn notification_http_urls_are_upgraded_to_websocket_urls() {
        assert_eq!(
            websocket_url("https://notifications.example.test/hub").unwrap(),
            "wss://notifications.example.test/hub"
        );
        assert_eq!(
            websocket_url("ws://localhost:8080/hub").unwrap(),
            "ws://localhost:8080/hub"
        );
        assert!(websocket_url("file:///tmp/hub").is_err());
    }

    #[tokio::test]
    async fn websocket_handshake_emits_connected_and_auth_request_signals() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
            assert!(matches!(socket.next().await, Some(Ok(Message::Text(text)))
                if text == "{\"protocol\":\"messagepack\",\"version\":1}\u{1e}"));
            socket.send(Message::Text("{}\u{1e}".into())).await.unwrap();
            socket
                .send(Message::Binary(
                    frame(serde_json::json!([
                        1,
                        {},
                        null,
                        "ReceiveMessage",
                        [{"type": 15}]
                    ]))
                    .into(),
                ))
                .await
                .unwrap();
            socket.close(None).await.unwrap();
        });
        let (sender, mut receiver) = mpsc::channel(2);
        run_connection(
            &format!("ws://{address}/hub"),
            Zeroizing::new("synthetic-access-token".into()),
            &sender,
        )
        .await
        .unwrap();
        assert_eq!(receiver.recv().await, Some(Signal::Connected));
        assert_eq!(receiver.recv().await, Some(Signal::AuthRequest));
        server.await.unwrap();
    }
}
