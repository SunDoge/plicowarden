use crate::{
    model::{HostCommand, Warden},
    ui::*,
};
use plico::prelude::*;
pub use plicowarden_sdk::security::SecuritySettings;
use snafu::{ResultExt, Snafu};
use std::path::{Path, PathBuf};
#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Security preferences could not be read or saved: {source}"))]
    Io { source: std::io::Error },
    #[snafu(display("Security preferences are invalid: {source}"))]
    Json { source: serde_json::Error },
    #[snafu(display("Security preferences are too large."))]
    TooLarge,
}
pub fn path() -> Option<PathBuf> {
    crate::appearance::Appearance::path().map(|path| path.with_file_name("security.json"))
}
pub fn load(path: &Path) -> Result<SecuritySettings, Error> {
    if !path.exists() {
        return Ok(Default::default());
    }
    if std::fs::metadata(path).context(IoSnafu)?.len() > 16384 {
        return Err(Error::TooLarge);
    }
    let settings: SecuritySettings =
        serde_json::from_slice(&std::fs::read(path).context(IoSnafu)?).context(JsonSnafu)?;
    Ok(settings.validated())
}
pub fn save(path: &Path, settings: SecuritySettings) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context(IoSnafu)?;
    }
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let result = (|| {
        std::fs::write(
            &temporary,
            serde_json::to_vec_pretty(&settings.validated()).context(JsonSnafu)?,
        )
        .context(IoSnafu)?;
        std::fs::rename(&temporary, path).context(IoSnafu)
    })();
    let _ = std::fs::remove_file(temporary);
    result
}
impl Warden {
    #[component]
    #[rustfmt::skip]
    pub fn security_ui(&mut self, ui: &mut UiContext) {
        surface(ui).padding(18.).gap(12.) + |ui| {
        text(ui, "Applies to this device.").font_size(13).color(MUTED).width_percent(100.);
        let mut lock = self.security.auto_lock_minutes.to_string();
        let locks: Vec<_> = plicowarden_sdk::security::AUTO_LOCK_MINUTES.iter().map(|value| {
            (
                value.to_string(),
                format!("{value} minute{}", if *value == 1 { "" } else { "s" }),
            )
        }).collect();
        let mut clipboard = self.security.clipboard_seconds.to_string();
        let clipboards: Vec<_> = plicowarden_sdk::security::CLIPBOARD_SECONDS.iter()
            .map(|value| (value.to_string(), format!("{value} seconds"))).collect();
        column(ui).width_percent(100.).gap(8).disabled(self.security_saving) + |ui| {
            text(ui, "Auto-lock after").font_size(14).weight(600);
            crate::forms::picker(ui, "Auto-lock after", &mut lock,
                self.popups.entry("auto-lock".into()).or_default(), &locks);
            text(ui, "Clear clipboard after").font_size(14).weight(600);
            crate::forms::picker(ui, "Clear clipboard after", &mut clipboard,
                self.popups.entry("clipboard-expiry".into()).or_default(), &clipboards);
            switch(ui, "Minimize after copying", &mut self.security.minimize_on_copy);
        };
        self.security.auto_lock_minutes = lock.parse().unwrap_or(5);
        self.security.clipboard_seconds = clipboard.parse().unwrap_or(30);
        text(ui, "Clipboard expiry leaves newer clipboard content untouched.").font_size(12).color(MUTED).width_percent(100.);
        row(ui).width_percent(100.).gap(10).wrap(true) + |ui| {
            if button(ui).text("Save changes").icon(crate::icons::get("save")).primary().radius(6.)
                .elevation(Elevation::CONTROL).disabled(self.security_saving).clicked()
            {
                self.security = self.security.validated();
                self.security_saving = true;
                self.commands.push(HostCommand::SaveSecurity(self.security));
            }
            if button(ui).text("Reset").icon(crate::icons::get("restore")).ghost().radius(6.)
                .disabled(self.security_saving).clicked()
            {
                self.security = Default::default();
            }
        };
        };
    }
}
