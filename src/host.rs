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
    desktop: Option<crate::desktop::DesktopHost>,
    window_actions: std::collections::VecDeque<plico_renderer::HostAction>,
    client: Option<Client>,
    transfer_generation: u64,
    transfer_tx: mpsc::Sender<(u64, Option<plicowarden_sdk::Request>)>,
    transfer_rx: mpsc::Receiver<(u64, Option<plicowarden_sdk::Request>)>,
    wake: Wake,
    settings_path: Option<PathBuf>,
    security_path: Option<PathBuf>,
    security_tx: mpsc::Sender<(crate::security::SecuritySettings, Result<(), String>)>,
    security_rx: mpsc::Receiver<(crate::security::SecuritySettings, Result<(), String>)>,
    settings_tx: mpsc::Sender<Result<(), String>>,
    settings_rx: mpsc::Receiver<Result<(), String>>,
}
impl Host {
    pub fn new(app: Warden, preferences: bool) -> Self {
        let (settings_tx, settings_rx) = mpsc::channel();
        let (transfer_tx, transfer_rx) = mpsc::channel();
        let (security_tx, security_rx) = mpsc::channel();
        let mut host = Self {
            app,
            desktop: None,
            window_actions: Default::default(),
            client: None,
            transfer_generation: 0,
            transfer_tx,
            transfer_rx,
            wake: Arc::new(|| {}),
            settings_path: Appearance::path(),
            security_path: crate::security::path(),
            security_tx,
            security_rx,
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
        if preferences && let Some(path) = &host.security_path {
            match crate::security::load(path) {
                Ok(settings) => {
                    host.app.security = settings;
                    host.app.active_security = settings;
                }
                Err(_) => {
                    host.app.notice =
                        "Saved security settings could not be read. Defaults are active.".into()
                }
            }
        }
        host.app.zoom.set(host.app.appearance.ui_scale);
        host
    }
    pub fn with_desktop(mut self, desktop: crate::desktop::DesktopHost) -> Self {
        self.desktop = Some(desktop);
        self
    }
    pub fn with_settings_path(mut self, path: PathBuf) -> Self {
        self.settings_path = Some(path);
        self
    }
    pub fn with_security_path(mut self, path: PathBuf) -> Self {
        self.security_path = Some(path);
        self
    }
    fn flush(&mut self) {
        for command in std::mem::take(&mut self.app.commands) {
            match command {
                HostCommand::SaveSecurity(settings) => {
                    let path = self.security_path.clone();
                    let tx = self.security_tx.clone();
                    let wake = self.wake.clone();
                    std::thread::spawn(move || {
                        let result = path
                            .ok_or_else(|| {
                                "The application data directory is unavailable.".to_owned()
                            })
                            .and_then(|path| {
                                crate::security::save(&path, settings)
                                    .map_err(|error| error.to_string())
                            });
                        let _ = tx.send((settings, result));
                        wake();
                    });
                }
                HostCommand::Transfer(intent) => {
                    self.transfer_generation = self.transfer_generation.wrapping_add(1);
                    let generation = self.transfer_generation;
                    let tx = self.transfer_tx.clone();
                    let wake = self.wake.clone();
                    std::thread::spawn(move || {
                        let request = intent.choose_file();
                        let _ = tx.send((generation, request));
                        wake();
                    });
                }
                HostCommand::CopyError(message) => self
                    .window_actions
                    .push_back(plico_renderer::HostAction::SetClipboard(message)),
                HostCommand::Window(command) => self
                    .window_actions
                    .push_back(plico_renderer::HostAction::Window(command)),
                HostCommand::SshDecision { id, allow } => {
                    if let Some(client) = &self.client
                        && let Err(error) =
                            client.send(0, plicowarden_sdk::Request::SshDecision { id, allow })
                    {
                        self.app.error = error.to_string();
                    }
                }
                HostCommand::Disconnect => {
                    self.transfer_generation = self.transfer_generation.wrapping_add(1);
                    self.app.transfer_dialog = false;
                    self.client.take();
                }
                HostCommand::Account { ticket, request } => {
                    let settings = self.app.active_security;
                    let client = self.client.get_or_insert_with(|| {
                        let client = Client::with_security(settings);
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
    fn on_user_activity(&mut self) {
        if let Some(client) = &self.client {
            client.note_activity();
        }
    }
    fn take_host_action(&mut self) -> Option<plico_renderer::HostAction> {
        self.window_actions.pop_front()
    }
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
        if let Some(desktop) = &self.desktop {
            desktop.set_wake(wake.clone());
        }
        self.wake = wake;
        if let Some(client) = &self.client {
            client.set_wake(Arc::clone(&self.wake));
        }
    }
    fn poll_updates(&mut self) -> bool {
        let mut changed = false;
        while let Some(event) = self
            .desktop
            .as_mut()
            .and_then(crate::desktop::DesktopHost::try_recv)
        {
            match event {
                crate::desktop::AppEvent::Lock => self.app.lock(),
                crate::desktop::AppEvent::Notice(message) => self.app.notice = message,
            }
            self.flush();
            changed = true;
        }
        while let Some(event) = self.client.as_ref().and_then(Client::try_recv) {
            let previous = self.app.ssh_approvals.len();
            self.app.apply(event);
            if self.app.ssh_approvals.len() > previous
                && let Some(desktop) = &self.desktop
            {
                desktop.notify_ssh();
            }
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
        while let Ok((settings, result)) = self.security_rx.try_recv() {
            self.app.security_saving = false;
            match result {
                Ok(()) => {
                    self.app.active_security = settings;
                    self.app.notice = "Security settings saved.".into();
                    if let Some(client) = &self.client
                        && client
                            .send(0, plicowarden_sdk::Request::ConfigureSecurity(settings))
                            .is_err()
                    {
                        self.app.error =
                            "Saved security settings will apply the next time you connect.".into();
                    }
                }
                Err(error) => self.app.error = error,
            }
            changed = true;
        }
        while let Ok((generation, request)) = self.transfer_rx.try_recv() {
            if generation != self.transfer_generation
                || !self.app.authenticated
                || !self.app.transfer_dialog
            {
                continue;
            }
            self.app.transfer_dialog = false;
            if let Some(request) = request {
                self.app.request(request);
            } else {
                self.app.notice = "File selection cancelled.".into();
            }
            changed = true;
        }
        self.flush();
        changed
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        // Revoke the signing gate synchronously, including native application quit.
        self.client.take();
        self.app.lock();
    }
}

#[cfg(test)]
mod transfer_tests {
    use super::*;
    #[test]
    fn stale_file_dialog_results_cannot_reuse_a_disconnected_session() {
        let mut app = Warden::default();
        app.authenticated = true;
        app.transfer_dialog = true;
        let mut host = Host::new(app, false);
        host.transfer_generation = 2;
        host.transfer_tx
            .send((1, Some(plicowarden_sdk::Request::Sync)))
            .unwrap();
        host.poll_updates();
        assert!(!host.app.busy);
        assert!(host.client.is_none());
        host.app.lock();
        host.flush();
        host.transfer_tx
            .send((2, Some(plicowarden_sdk::Request::Sync)))
            .unwrap();
        host.poll_updates();
        assert!(!host.app.busy);
        assert!(host.client.is_none());
    }
}
