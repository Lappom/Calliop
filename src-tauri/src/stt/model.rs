use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use thiserror::Error;

pub const DEFAULT_MODEL_FILE: &str = "ggml-small.bin";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WhisperModel {
    #[default]
    Auto,
    Small,
    DistilFrDec16,
}

impl WhisperModel {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "small" => Some(Self::Small),
            // Legacy: medium replaced by distil-fr-dec16 (option 3 tier lineup).
            "medium" | "distil-fr-dec16" => Some(Self::DistilFrDec16),
            _ => None,
        }
    }

    pub fn as_setting_value(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Small => "small",
            Self::DistilFrDec16 => "distil-fr-dec16",
        }
    }

    pub fn is_concrete(self) -> bool {
        !matches!(self, Self::Auto)
    }

    pub fn file_name(self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::Small => Some("ggml-small.bin"),
            Self::DistilFrDec16 => Some("whisper-distil-fr-dec16-q5_0.bin"),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Automatique (recommandé)",
            Self::Small => "Rapide — Small (~466 Mo)",
            Self::DistilFrDec16 => "Équilibré — Distil FR dec16 (~755 Mo)",
        }
    }

    pub fn min_bytes(self) -> u64 {
        match self {
            Self::Auto => 0,
            Self::Small => 450_000_000,
            Self::DistilFrDec16 => 750_000_000,
        }
    }

    pub fn download_urls(self) -> &'static [&'static str] {
        match self {
            Self::Auto => &[],
            Self::Small => {
                &["https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin"]
            }
            Self::DistilFrDec16 => &[
                "https://huggingface.co/bofenghuang/whisper-large-v3-french-distil-dec16/resolve/main/ggml-model-q5_0.bin",
            ],
        }
    }

    pub fn path(self) -> PathBuf {
        models_dir().join(self.file_name().expect("concrete whisper model path"))
    }

    pub fn is_installed(self) -> bool {
        if !self.is_concrete() {
            return false;
        }
        is_valid_model_file(self, &self.path())
    }

    pub fn all_concrete() -> [Self; 2] {
        [Self::Small, Self::DistilFrDec16]
    }

    pub fn all_selectable() -> [Self; 3] {
        [Self::Auto, Self::Small, Self::DistilFrDec16]
    }
}

pub const LEGACY_MEDIUM_MODEL_FILE: &str = "ggml-medium.bin";

pub fn models_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.calliop.app")
        .join("models")
}

pub fn legacy_medium_model_path() -> PathBuf {
    models_dir().join(LEGACY_MEDIUM_MODEL_FILE)
}

/// Remove orphaned Whisper medium weights after migration to distil-fr-dec16.
pub fn remove_legacy_medium_model() -> std::io::Result<()> {
    let path = legacy_medium_model_path();
    if path.exists() {
        std::fs::remove_file(path)
    } else {
        Ok(())
    }
}

pub fn model_path(model: WhisperModel) -> PathBuf {
    model.path()
}

pub fn model_exists(model: WhisperModel) -> bool {
    model.is_installed()
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelDownloadProgress {
    pub model_id: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percent: f32,
    pub source: String,
}

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("failed to create models directory: {0}")]
    CreateDir(std::io::Error),
    #[error("download failed from {url}: {message}")]
    Download { url: String, message: String },
    #[error("all download sources failed")]
    AllSourcesFailed,
    #[error("downloaded model is too small ({size} bytes, expected >= {min_bytes})")]
    ModelTooSmall { size: u64, min_bytes: u64 },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn is_valid_model_file(model: WhisperModel, path: &Path) -> bool {
    if !model.is_concrete() {
        return false;
    }
    std::fs::metadata(path)
        .map(|meta| meta.len() >= model.min_bytes())
        .unwrap_or(false)
}

pub fn ensure_model_blocking(
    app: Option<&AppHandle>,
    model: WhisperModel,
) -> Result<PathBuf, ModelError> {
    if !model.is_concrete() {
        return Err(ModelError::Download {
            url: "model".into(),
            message: "cannot download unresolved whisper model (auto)".into(),
        });
    }
    let path = model.path();
    if path.exists() {
        if is_valid_model_file(model, &path) {
            return Ok(path);
        }
        let _ = std::fs::remove_file(&path);
    }

    std::fs::create_dir_all(path.parent().ok_or_else(|| {
        ModelError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "model parent directory missing",
        ))
    })?)
    .map_err(ModelError::CreateDir)?;

    let rt = tokio::runtime::Runtime::new().map_err(|e| ModelError::Download {
        url: "runtime".into(),
        message: e.to_string(),
    })?;

    rt.block_on(download_model(app, model, &path))
}

pub async fn download_model(
    app: Option<&AppHandle>,
    model: WhisperModel,
    dest: &Path,
) -> Result<PathBuf, ModelError> {
    let client = reqwest::Client::builder()
        .user_agent("Calliop/0.1")
        .build()
        .map_err(|e| ModelError::Download {
            url: "client".into(),
            message: e.to_string(),
        })?;

    let mut last_error = None;

    for url in model.download_urls() {
        match try_download(&client, app, model, url, dest).await {
            Ok(()) => return Ok(dest.to_path_buf()),
            Err(err) => {
                let _ = std::fs::remove_file(dest);
                last_error = Some((url.to_string(), err.to_string()));
            }
        }
    }

    if let Some((url, message)) = last_error {
        return Err(ModelError::Download { url, message });
    }

    Err(ModelError::AllSourcesFailed)
}

async fn try_download(
    client: &reqwest::Client,
    app: Option<&AppHandle>,
    model: WhisperModel,
    url: &str,
    dest: &Path,
) -> Result<(), ModelError> {
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|e| ModelError::Download {
            url: url.into(),
            message: e.to_string(),
        })?;

    if !response.status().is_success() {
        return Err(ModelError::Download {
            url: url.into(),
            message: format!("HTTP {}", response.status()),
        });
    }

    let total = response.content_length();
    let mut downloaded: u64 = 0;
    let mut file = tokio::fs::File::create(dest).await?;

    use tokio::io::AsyncWriteExt;

    while let Some(chunk) = response.chunk().await.map_err(|e| ModelError::Download {
        url: url.into(),
        message: e.to_string(),
    })? {
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        if let Some(handle) = app {
            let percent = total
                .map(|t| (downloaded as f32 / t as f32) * 100.0)
                .unwrap_or(0.0);
            let _ = handle.emit(
                "model-download-progress",
                ModelDownloadProgress {
                    model_id: model.as_setting_value().into(),
                    downloaded,
                    total,
                    percent,
                    source: url.to_string(),
                },
            );
        }
    }

    file.flush().await?;

    let size = std::fs::metadata(dest).map_err(ModelError::Io)?.len();
    let min_bytes = model.min_bytes();
    if size < min_bytes {
        return Err(ModelError::ModelTooSmall { size, min_bytes });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_urls_use_huggingface_primary() {
        for model in WhisperModel::all_concrete() {
            let urls = model.download_urls();
            assert!(urls[0].contains("huggingface.co"));
        }
    }

    #[test]
    fn rejects_too_small_model_files() {
        let dir = std::env::temp_dir().join("calliop-model-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("tiny.bin");
        std::fs::write(&path, vec![0u8; 1024]).unwrap();
        assert!(!is_valid_model_file(WhisperModel::Small, &path));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn default_model_is_auto() {
        assert_eq!(WhisperModel::default(), WhisperModel::Auto);
    }

    #[test]
    fn concrete_small_model_filename() {
        assert!(model_path(WhisperModel::Small).ends_with(DEFAULT_MODEL_FILE));
    }

    #[test]
    fn legacy_medium_path_uses_expected_filename() {
        assert!(legacy_medium_model_path().ends_with(LEGACY_MEDIUM_MODEL_FILE));
    }

    #[test]
    fn remove_legacy_medium_is_ok_when_file_absent() {
        assert!(remove_legacy_medium_model().is_ok());
    }

    #[test]
    fn parses_model_ids() {
        assert_eq!(
            WhisperModel::parse("medium"),
            Some(WhisperModel::DistilFrDec16)
        );
        assert_eq!(
            WhisperModel::parse("distil-fr-dec16"),
            Some(WhisperModel::DistilFrDec16)
        );
        assert_eq!(WhisperModel::parse("unknown"), None);
    }
}
