//! Plicowarden policy and preference persistence over Plico desktop services.
pub use plico_desktop::desktop::DesktopExtension as TrayExtension;
use plico_desktop::desktop::{
    self, DesktopClient, DesktopEvent, DesktopOptions, MenuAction, TrayImage, TrayItem,
};
use plico_desktop::{NotificationRequest, WakeCallback};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, mpsc};
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Preferences {
    pub close_to_tray: bool,
    pub ssh_notifications: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            close_to_tray: true,
            ssh_notifications: true,
        }
    }
}
#[derive(Debug)]
pub enum AppEvent {
    Lock,
    Notice(String),
}

pub struct DesktopHost {
    native: DesktopClient,
    preferences: Preferences,
    save: mpsc::SyncSender<Preferences>,
    errors: mpsc::Receiver<AppEvent>,
    wake: Arc<Mutex<Option<WakeCallback>>>,
}
impl DesktopHost {
    pub fn set_wake(&self, wake: WakeCallback) {
        *self.wake.lock().unwrap() = Some(wake);
    }
    pub fn try_recv(&mut self) -> Option<AppEvent> {
        while let Some(event) = self.native.try_recv() {
            if let Some(event) = self.process(event) {
                return Some(event);
            }
        }
        self.errors.try_recv().ok()
    }
    fn process(&mut self, event: DesktopEvent) -> Option<AppEvent> {
        match event {
            DesktopEvent::Menu { id, checked: _ } if id == "lock" => Some(AppEvent::Lock),
            DesktopEvent::Menu {
                id,
                checked: Some(value),
            } if id == "close" || id == "notifications" => {
                if id == "close" {
                    self.preferences.close_to_tray = value;
                    self.native.handle().set_close_to_tray(value);
                } else {
                    self.preferences.ssh_notifications = value;
                    self.native.handle().set_notifications_enabled(value);
                }
                if self.save.try_send(self.preferences).is_err() {
                    return Some(AppEvent::Notice(
                        "Desktop preferences could not be saved. Changes apply until quit.".into(),
                    ));
                }
                None
            }
            DesktopEvent::TrayUnavailable => Some(AppEvent::Notice(
                "Tray unavailable. Closing the window will quit Plicowarden.".into(),
            )),
            DesktopEvent::NotificationFailed(_) => Some(AppEvent::Notice(
                "System notification unavailable. Review pending approvals in Plicowarden.".into(),
            )),
            _ => None,
        }
    }
    pub fn notify_ssh(&self) {
        // Notification contents are application policy. No approval metadata or secrets.
        let _ = self.native.handle().notify(
            1,
            NotificationRequest {
                title: "SSH signing approval".into(),
                body: Some("Open Plicowarden to review a pending request.".into()),
                icon: None,
                silent: false,
            },
        );
    }
    pub fn notify_sign_in(&self) {
        // Do not expose the account, server, address or fingerprint on a lock screen.
        let _ = self.native.handle().notify(
            2,
            NotificationRequest {
                title: "Sign-in approval".into(),
                body: Some("Open Plicowarden to review a pending device request.".into()),
                icon: None,
                silent: false,
            },
        );
    }
}
pub fn integration() -> (DesktopHost, TrayExtension) {
    let preferences = preferences_path()
        .as_deref()
        .and_then(load_preferences)
        .unwrap_or_default();
    let action = |id: &str, label: &str, action| TrayItem::Action {
        id: id.into(),
        label: label.into(),
        action,
    };
    let toggle = |id: &str, label: &str, checked| TrayItem::Toggle {
        id: id.into(),
        label: label.into(),
        checked,
    };
    let (native, extension) = desktop::integration(DesktopOptions {
        app_id: "plicowarden".into(),
        title: "Plicowarden".into(),
        window: plico_desktop::initial_window_resource_key(0),
        image: TrayImage {
            rgba: icon_rgba(),
            width: 32,
            height: 32,
            macos_template: true,
        },
        menu: vec![
            action("open", "Open Plicowarden", MenuAction::ShowWindow),
            action("lock", "Lock vault", MenuAction::Emit),
            TrayItem::Separator,
            toggle(
                "close",
                "Keep running when closed",
                preferences.close_to_tray,
            ),
            toggle(
                "notifications",
                "Approval notifications",
                preferences.ssh_notifications,
            ),
            TrayItem::Separator,
            action("quit", "Quit Plicowarden", MenuAction::Exit),
        ],
        close_to_tray: preferences.close_to_tray,
        notifications_enabled: preferences.ssh_notifications,
    })
    .expect("static Plicowarden desktop declaration must be valid");
    let (save, receiver) = mpsc::sync_channel::<Preferences>(8);
    let (errors_tx, errors) = mpsc::channel();
    let wake: Arc<Mutex<Option<WakeCallback>>> = Arc::default();
    let worker_wake = wake.clone();
    std::thread::spawn(move || {
        while let Ok(preferences) = receiver.recv() {
            if save_preferences(preferences).is_err() {
                let _ = errors_tx.send(AppEvent::Notice(
                    "Desktop preferences could not be saved. Changes apply until quit.".into(),
                ));
                if let Some(wake) = worker_wake.lock().ok().and_then(|slot| slot.clone()) {
                    wake();
                }
            }
        }
    });
    (
        DesktopHost {
            native,
            preferences,
            save,
            errors,
            wake,
        },
        extension,
    )
}
fn save_preferences(preferences: Preferences) -> Result<(), ()> {
    let path = preferences_path().ok_or(())?;
    std::fs::create_dir_all(path.parent().ok_or(())?).map_err(|_| ())?;
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let result = serde_json::to_vec_pretty(&preferences)
        .map_err(|_| ())
        .and_then(|bytes| std::fs::write(&temporary, bytes).map_err(|_| ()))
        .and_then(|()| std::fs::rename(&temporary, &path).map_err(|_| ()));
    let _ = std::fs::remove_file(temporary);
    result
}
fn load_preferences(path: &std::path::Path) -> Option<Preferences> {
    if std::fs::metadata(path).ok()?.len() > 16_384 {
        return None;
    }
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}
fn preferences_path() -> Option<std::path::PathBuf> {
    crate::security::path().map(|path| path.with_file_name("desktop.json"))
}
/// Small supersampled shield/keyhole mark, with transparency for macOS templates.
fn icon_rgba() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32 {
        for x in 0..32 {
            let mut covered = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let x = (x as f32 + (sx as f32 + 0.5) / 4. - 16.) / 12.;
                    let y = (y as f32 + (sy as f32 + 0.5) / 4. - 3.) / 26.;
                    let shield = (0. ..=1.).contains(&y)
                        && x.abs()
                            < if y < 0.55 {
                                1.
                            } else {
                                ((1. - y) / 0.45).sqrt()
                            };
                    let hole = x * x + (y - 0.4).powi(2) * 9. < 0.075
                        || (x.abs() < 0.12 && (0.4..0.69).contains(&y));
                    covered += u8::from(shield && !hole);
                }
            }
            bytes.extend_from_slice(&[35, 85, 102, ((u16::from(covered) * 255) / 16) as u8]);
        }
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_menu_and_failure_events_are_mapped_to_application_policy() {
        let (mut desktop, _) = integration();
        assert!(matches!(
            desktop.process(DesktopEvent::Menu {
                id: "lock".into(),
                checked: None
            }),
            Some(AppEvent::Lock)
        ));
        assert!(
            desktop
                .process(DesktopEvent::NotificationActivated(1))
                .is_none()
        );
        assert!(matches!(
            desktop.process(DesktopEvent::NotificationFailed(1)),
            Some(AppEvent::Notice(_))
        ));
    }
    #[test]
    fn desktop_preferences_are_small_and_do_not_accept_account_fields() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("desktop.json");
        std::fs::write(&path, r#"{"closeToTray":false,"sshNotifications":false}"#).unwrap();
        let preferences = load_preferences(&path).unwrap();
        assert!(!preferences.close_to_tray && !preferences.ssh_notifications);
        std::fs::write(&path, r#"{"password":"unsupported"}"#).unwrap();
        assert!(load_preferences(&path).is_none());
        std::fs::write(&path, vec![b' '; 16_385]).unwrap();
        assert!(load_preferences(&path).is_none());
        let pixels = icon_rgba();
        assert_eq!(pixels.len(), 32 * 32 * 4);
        assert!(pixels.as_chunks::<4>().0.iter().any(|p| p[3] == 0));
        assert!(pixels.as_chunks::<4>().0.iter().any(|p| p[3] == 255));
    }
}
