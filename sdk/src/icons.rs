//! Account-independent, bounded favicon downloads. No vault tokens are attached.
use std::sync::{Arc, RwLock, mpsc};
use tokio::{sync::mpsc as queue, task::JoinSet};

type ResultMessage = (String, Option<Vec<u8>>);
/// A separate worker keeps image I/O out of the account operation queue.
pub struct IconLoader {
    tx: queue::Sender<String>,
    rx: mpsc::Receiver<ResultMessage>,
    wake: Arc<RwLock<crate::Wake>>,
}
impl IconLoader {
    /// Start lazily, once a real vault contains a site icon URL.
    pub fn new(wake: crate::Wake) -> Option<Self> {
        let client = bitwarden_api_base::new_http_client_builder()
            .https_only(true)
            .timeout(std::time::Duration::from_secs(8))
            .redirect(reqwest::redirect::Policy::limited(3))
            .build()
            .ok()?;
        let (tx, mut requests) = queue::channel::<String>(16);
        let (results, rx) = mpsc::channel();
        let wake = Arc::new(RwLock::new(wake));
        let notify = wake.clone();
        std::thread::spawn(move || {
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                return;
            };
            runtime.block_on(async move {
                let mut jobs = JoinSet::new();
                loop {
                    tokio::select! {
                        request = requests.recv(), if jobs.len() < 4 => {
                            let Some(url) = request else { break };
                            let client = client.clone();
                            jobs.spawn(async move {
                                let bytes = download(&client, &url).await;
                                (url, bytes)
                            });
                        }
                        result = jobs.join_next(), if !jobs.is_empty() => {
                            if let Some(Ok(result)) = result {
                                if results.send(result).is_err() {break;}
                                let wake = notify.read().unwrap().clone();
                                wake();
                            }
                        }
                    }
                }
                jobs.abort_all();
            });
        });
        Some(Self { tx, rx, wake })
    }
    /// Queue capacity and four active downloads bound work and memory.
    pub fn request(&self, url: String) -> bool {
        self.tx.try_send(url).is_ok()
    }
    /// Drain completed images from the UI host.
    pub fn try_recv(&self) -> Option<ResultMessage> {
        self.rx.try_recv().ok()
    }
    /// Update the native host wake callback without retaining any UI state.
    pub fn set_wake(&self, wake: crate::Wake) {
        *self.wake.write().unwrap() = wake;
    }
}
async fn download(client: &reqwest::Client, url: &str) -> Option<Vec<u8>> {
    let url = url::Url::parse(url).ok()?;
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return None;
    }
    let mut response = client.get(url).send().await.ok()?.error_for_status().ok()?;
    const LIMIT: usize = 128 * 1024;
    if response.content_length().is_some_and(|n| n > LIMIT as u64) {
        return None;
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        if bytes.len() + chunk.len() > LIMIT {
            return None;
        }
        bytes.extend_from_slice(&chunk);
    }
    valid_icon(&bytes).then_some(bytes)
}
fn valid_icon(bytes: &[u8]) -> bool {
    image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()
        .and_then(|reader| reader.into_dimensions().ok())
        .is_some_and(|(w, h)| w > 0 && h > 0 && w <= 256 && h <= 256)
        && image::load_from_memory(bytes).is_ok()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_non_images_and_excessive_dimensions() {
        assert!(!valid_icon(b"<html>not an icon</html>"));
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbaImage::new(32, 32)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        assert!(valid_icon(bytes.get_ref()));
        bytes = std::io::Cursor::new(Vec::new());
        image::RgbaImage::new(257, 1)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        assert!(!valid_icon(bytes.get_ref()));
    }
}
