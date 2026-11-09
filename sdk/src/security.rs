use serde::{Deserialize, Serialize};
pub const AUTO_LOCK_MINUTES: &[u32] = &[1, 5, 15, 30, 60];
pub const CLIPBOARD_SECONDS: &[u32] = &[10, 30, 60, 120];
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct SecuritySettings {
    pub auto_lock_minutes: u32,
    pub clipboard_seconds: u32,
    pub minimize_on_copy: bool,
}
impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            auto_lock_minutes: 5,
            clipboard_seconds: 30,
            minimize_on_copy: false,
        }
    }
}
impl SecuritySettings {
    pub fn validated(mut self) -> Self {
        if !AUTO_LOCK_MINUTES.contains(&self.auto_lock_minutes) {
            self.auto_lock_minutes = 5;
        }
        if !CLIPBOARD_SECONDS.contains(&self.clipboard_seconds) {
            self.clipboard_seconds = 30;
        }
        self
    }
}
