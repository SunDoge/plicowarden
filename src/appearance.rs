use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use std::path::{Path, PathBuf};
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Appearance {
    pub ui_scale: f64,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            ui_scale: crate::display::default_scale(cfg!(target_os = "macos")),
        }
    }
}
#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Appearance preferences could not be read or saved: {source}"))]
    Io { source: std::io::Error },
    #[snafu(display("Appearance preferences are invalid: {source}"))]
    Json { source: serde_json::Error },
    #[snafu(display("Appearance preferences are too large."))]
    TooLarge,
}
impl Appearance {
    pub fn path() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "Plicowarden")
            .map(|d| d.data_local_dir().join("appearance.json"))
    }
    pub fn zoom_controller(&self) -> plico_runtime::ZoomController {
        let controller = plico_runtime::ZoomController::default();
        controller.set(self.clone().validated().ui_scale);
        controller
    }
    pub fn validated(mut self) -> Self {
        self.ui_scale = if self.ui_scale.is_finite() {
            self.ui_scale.clamp(0.5, 3.)
        } else {
            Self::default().ui_scale
        };
        self
    }
    pub fn load(path: &Path) -> Result<Self, Error> {
        if !path.exists() {
            return Ok(Self::default());
        }
        if std::fs::metadata(path).context(IoSnafu)?.len() > 16384 {
            return Err(Error::TooLarge);
        }
        Ok(
            serde_json::from_slice::<Self>(&std::fs::read(path).context(IoSnafu)?)
                .context(JsonSnafu)?
                .validated(),
        )
    }
    pub fn save(&self, path: &Path) -> Result<(), Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context(IoSnafu)?;
        }
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        let result = (|| {
            std::fs::write(
                &temporary,
                serde_json::to_vec_pretty(&self.clone().validated()).context(JsonSnafu)?,
            )
            .context(IoSnafu)?;
            std::fs::rename(&temporary, path).context(IoSnafu)
        })();
        let _ = std::fs::remove_file(temporary);
        result
    }
}
