//! Clipboard ownership and conditional expiry stay outside the GUI/render thread.
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
use tokio::sync::oneshot;
use zeroize::Zeroizing;
const LIFETIME: Duration = Duration::from_secs(30);
enum Request {
    Set(Zeroizing<String>, oneshot::Sender<Result<(), String>>),
    Clear,
}
pub struct ClipboardService {
    sender: mpsc::SyncSender<Request>,
}
impl ClipboardService {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::sync_channel(8);
        std::thread::spawn(move || {
            let mut clipboard = arboard::Clipboard::new().ok();
            let mut owned: Option<(Zeroizing<String>, Instant)> = None;
            loop {
                match receiver.recv_timeout(Duration::from_millis(200)) {
                    Ok(Request::Set(text, reply)) => {
                        if clipboard.is_none() {
                            clipboard = arboard::Clipboard::new().ok();
                        }
                        let result = clipboard
                            .as_mut()
                            .ok_or_else(|| "System clipboard unavailable.".to_owned())
                            .and_then(|clipboard| {
                                clipboard.set_text(text.to_string()).map_err(|_| {
                                    "Could not copy to the system clipboard.".to_owned()
                                })
                            });
                        if result.is_ok() {
                            owned = Some((text, Instant::now()));
                        }
                        let _ = reply.send(result);
                    }
                    Ok(Request::Clear) => clear_owned(&mut clipboard, &mut owned),
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        clear_owned(&mut clipboard, &mut owned);
                        break;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                if owned
                    .as_ref()
                    .is_some_and(|(_, at)| at.elapsed() >= LIFETIME)
                {
                    clear_owned(&mut clipboard, &mut owned);
                }
            }
        });
        Self { sender }
    }
    pub async fn copy(&self, text: Zeroizing<String>) -> Result<(), String> {
        let (send, receive) = oneshot::channel();
        self.sender
            .try_send(Request::Set(text, send))
            .map_err(|_| "Clipboard worker busy.".to_owned())?;
        receive
            .await
            .map_err(|_| "Clipboard worker stopped.".to_owned())?
    }
    pub fn clear(&self) {
        let _ = self.sender.try_send(Request::Clear);
    }
}
fn clear_owned(
    clipboard: &mut Option<arboard::Clipboard>,
    owned: &mut Option<(Zeroizing<String>, Instant)>,
) {
    if let (Some(clipboard), Some((text, _))) = (clipboard.as_mut(), owned.take()) {
        // Preserve anything the user copied after us. Clipboard APIs provide no
        // cross-platform atomic compare-and-clear, so this remains best effort.
        if clipboard.get_text().ok().map(Zeroizing::new).as_deref() == Some(&text) {
            let _ = clipboard.clear();
        }
    }
}
