//! Native host owns all workers. Workers receive values, never references to UI state.
use crate::{
    appearance::Appearance,
    model::{HostCommand, Warden},
};
use plico_dev::App;
use plico_runtime::HostView;
use plicowarden_sdk::{Client, Wake};
use std::{
    path::PathBuf,
    sync::{Arc, mpsc},
};

pub struct Host {
    pub app: Warden,
    client: Option<Client>,
    wake: Wake,
    settings_path: Option<PathBuf>,
    settings_tx: mpsc::Sender<Result<(), String>>,
    settings_rx: mpsc::Receiver<Result<(), String>>,
}
impl Host {
    pub fn new(app: Warden, preferences: bool) -> Self {
        let (settings_tx, settings_rx) = mpsc::channel();
        let mut host = Self {
            app,
            client: None,
            wake: Arc::new(|| {}),
            settings_path: Appearance::path(),
            settings_tx,
            settings_rx,
        };
        if preferences && let Some(path) = &host.settings_path {
            match Appearance::load(path) {
                Ok(value) => host.app.appearance = value,
                Err(_) => {
                    host.app.notice =
                        "Saved appearance could not be read. Defaults are active.".into()
                }
            }
        }
        host.app.zoom.set(host.app.appearance.ui_scale);
        host
    }
    pub fn with_settings_path(mut self, path: PathBuf) -> Self {
        self.settings_path = Some(path);
        self
    }
    fn flush(&mut self) {
        for command in std::mem::take(&mut self.app.commands) {
            match command {
                HostCommand::SshDecision { id, allow } => {
                    if let Some(client) = &self.client
                        && let Err(error) =
                            client.send(0, plicowarden_sdk::Request::SshDecision { id, allow })
                    {
                        self.app.error = error.to_string();
                    }
                }
                HostCommand::Disconnect => {
                    self.client.take();
                }
                HostCommand::Account { ticket, request } => {
                    let client = self.client.get_or_insert_with(|| {
                        let client = Client::new();
                        client.set_wake(Arc::clone(&self.wake));
                        client
                    });
                    if let Err(error) = client.send(ticket, *request) {
                        self.app.apply(plicowarden_sdk::Event {
                            ticket,
                            result: Err(error.to_string()),
                        });
                    }
                }
                HostCommand::SaveAppearance(value) => {
                    let path = self.settings_path.clone();
                    let tx = self.settings_tx.clone();
                    let wake = Arc::clone(&self.wake);
                    std::thread::spawn(move || {
                        let result = path
                            .ok_or_else(|| "The application data directory is unavailable.".into())
                            .and_then(|path| value.save(&path).map_err(|error| error.to_string()));
                        let _ = tx.send(result);
                        wake();
                    });
                }
            }
        }
    }
}
impl HostView for Host {
    fn show(&mut self, ui: &mut plico::UiContext<'_>) {
        App::show(&mut self.app, ui);
        self.flush();
    }
    fn theme(&self) -> plico::Theme {
        App::theme(&self.app)
    }
    fn set_frame_time(&mut self, time: plico::FrameTime) {
        App::set_frame_time(&mut self.app, time);
    }
    fn on_unhandled_key(&mut self, key: &plico_renderer::KeyEvent) -> bool {
        let handled = App::on_key_event(&mut self.app, key);
        self.flush();
        handled
    }
    fn set_wake_callback(&mut self, wake: plico_renderer::WakeCallback) {
        self.wake = wake;
        if let Some(client) = &self.client {
            client.set_wake(Arc::clone(&self.wake));
        }
    }
    fn poll_updates(&mut self) -> bool {
        let mut changed = false;
        while let Some(event) = self.client.as_ref().and_then(Client::try_recv) {
            self.app.apply(event);
            changed = true;
        }
        while let Ok(result) = self.settings_rx.try_recv() {
            self.app.settings_saving = false;
            match result {
                Ok(()) => self.app.notice = "Appearance saved.".into(),
                Err(error) => self.app.error = error,
            }
            changed = true;
        }
        self.flush();
        changed
    }
}
