use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use thiserror::Error;

use crate::stt::models_dir;

pub const DEFAULT_MODEL_FILE: &str = "qwen3-1.7b-instruct-q4_k_m.gguf";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LlmModel {
    #[default]
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "qwen3-0.6b")]
    Qwen3_0_6B,
    #[serde(rename = "qwen3-1.7b")]
    Qwen3_1_7B,
    #[serde(rename = "qwen3-4b")]
    Qwen3_4B,
}

impl LlmModel {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "qwen3-0.6b" | "qwen3_0.6b" => Some(Self::Qwen3_0_6B),
            "qwen3-1.7b" | "qwen3_1.7b" => Some(Self::Qwen3_1_7B),
            "qwen3-4b" | "qwen3_4b" => Some(Self::Qwen3_4B),
            _ => None,
        }
    }

    pub fn as_setting_value(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Qwen3_0_6B => "qwen3-0.6b",
            Self::Qwen3_1_7B => "qwen3-1.7b",
            Self::Qwen3_4B => "qwen3-4b",
        }
    }

    pub fn is_concrete(self) -> bool {
        !matches!(self, Self::Auto)
    }

    pub fn file_name(self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::Qwen3_0_6B => Some("qwen3-0.6b-instruct-q4_k_m.gguf"),
            Self::Qwen3_1_7B => Some("qwen3-1.7b-instruct-q4_k_m.gguf"),
            Self::Qwen3_4B => Some("qwen3-4b-instruct-q4_k_m.gguf"),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Automatique (recommandé)",
            Self::Qwen3_0_6B => "Qwen3 0.6B Instruct Q4_K_M (~378 Mo)",
            Self::Qwen3_1_7B => "Qwen3 1.7B Instruct Q4_K_M (~1,1 Go)",
            Self::Qwen3_4B => "Qwen3 4B Instruct Q4_K_M (~2,5 Go, GPU recommandé)",
        }
    }

    pub fn min_bytes(self) -> u64 {
        match self {
            Self::Auto => 0,
            Self::Qwen3_0_6B => 370_000_000,
            Self::Qwen3_1_7B => 1_000_000_000,
            Self::Qwen3_4B => 2_400_000_000,
        }
    }

    pub fn download_urls(self) -> &'static [&'static str] {
        match self {
            Self::Auto => &[],
            Self::Qwen3_0_6B => &[
                "https://huggingface.co/unsloth/Qwen3-0.6B-GGUF/resolve/main/Qwen3-0.6B-Q4_K_M.gguf",
            ],
            Self::Qwen3_1_7B => &[
                "https://huggingface.co/PatnaikAshish/Qwen3-1.7B-Instruct-Q4_K_M-GGUF/resolve/main/qwen3-1.7b-instruct-q4_k_m.gguf",
            ],
            Self::Qwen3_4B => &[
                "https://huggingface.co/unsloth/Qwen3-4B-Instruct-2507-GGUF/resolve/main/Qwen3-4B-Instruct-2507-Q4_K_M.gguf",
            ],
        }
    }

    pub fn path(self) -> PathBuf {
        models_dir().join(self.file_name().expect("concrete llm model path"))
    }

    pub fn is_installed(self) -> bool {
        if !self.is_concrete() {
            return false;
        }
        is_valid_model_file(self, &self.path())
    }

    pub fn all_concrete() -> [Self; 3] {
        [Self::Qwen3_0_6B, Self::Qwen3_1_7B, Self::Qwen3_4B]
    }

    pub fn all_selectable() -> [Self; 4] {
        [
            Self::Auto,
            Self::Qwen3_0_6B,
            Self::Qwen3_1_7B,
            Self::Qwen3_4B,
        ]
    }
}

pub fn model_path(model: LlmModel) -> PathBuf {
    model.path()
}

pub fn model_exists(model: LlmModel) -> bool {
    model.is_installed()
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmModelDownloadProgress {
    pub model_id: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percent: f32,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmModelDownloadFailed {
    pub model_id: String,
}

fn emit_llm_download_failed(app: Option<&AppHandle>, model: LlmModel) {
    if let Some(handle) = app {
        let _ = handle.emit(
            "llm-model-download-failed",
            LlmModelDownloadFailed {
                model_id: model.as_setting_value().into(),
            },
        );
    }
}

#[derive(Debug, Error)]
pub enum LlmModelError {
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

pub fn is_valid_model_file(model: LlmModel, path: &Path) -> bool {
    if !model.is_concrete() {
        return false;
    }
    std::fs::metadata(path)
        .map(|meta| meta.len() >= model.min_bytes())
        .unwrap_or(false)
}

pub fn ensure_llm_model_blocking(
    app: Option<&AppHandle>,
    model: LlmModel,
) -> Result<PathBuf, LlmModelError> {
    if !model.is_concrete() {
        return Err(LlmModelError::Download {
            url: "model".into(),
            message: "cannot download unresolved llm model (auto)".into(),
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
        LlmModelError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "model parent directory missing",
        ))
    })?)
    .map_err(LlmModelError::CreateDir)?;

    let rt = tokio::runtime::Runtime::new().map_err(|e| LlmModelError::Download {
        url: "runtime".into(),
        message: e.to_string(),
    })?;

    rt.block_on(download_model(app, model, &path))
}

/// Ensures model weights exist on disk without loading the inference engine.
pub fn ensure_llm_model_file_blocking(
    app: Option<&AppHandle>,
    model: LlmModel,
) -> Result<PathBuf, LlmModelError> {
    ensure_llm_model_blocking(app, model)
}

pub async fn download_model(
    app: Option<&AppHandle>,
    model: LlmModel,
    dest: &Path,
) -> Result<PathBuf, LlmModelError> {
    let client = reqwest::Client::builder()
        .user_agent("Calliop/0.1")
        .build()
        .map_err(|e| LlmModelError::Download {
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
        emit_llm_download_failed(app, model);
        return Err(LlmModelError::Download { url, message });
    }

    emit_llm_download_failed(app, model);
    Err(LlmModelError::AllSourcesFailed)
}

async fn try_download(
    client: &reqwest::Client,
    app: Option<&AppHandle>,
    model: LlmModel,
    url: &str,
    dest: &Path,
) -> Result<(), LlmModelError> {
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|e| LlmModelError::Download {
            url: url.into(),
            message: e.to_string(),
        })?;

    if !response.status().is_success() {
        return Err(LlmModelError::Download {
            url: url.into(),
            message: format!("HTTP {}", response.status()),
        });
    }

    let total = response.content_length();
    let mut downloaded: u64 = 0;
    let mut file = tokio::fs::File::create(dest).await?;

    use tokio::io::AsyncWriteExt;

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| LlmModelError::Download {
            url: url.into(),
            message: e.to_string(),
        })?
    {
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        if let Some(handle) = app {
            let percent = total
                .map(|t| (downloaded as f32 / t as f32) * 100.0)
                .unwrap_or(0.0);
            let _ = handle.emit(
                "llm-model-download-progress",
                LlmModelDownloadProgress {
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

    let size = std::fs::metadata(dest).map_err(LlmModelError::Io)?.len();
    let min_bytes = model.min_bytes();
    if size < min_bytes {
        return Err(LlmModelError::ModelTooSmall { size, min_bytes });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_urls_use_huggingface_primary() {
        for model in LlmModel::all_concrete() {
            let urls = model.download_urls();
            assert!(urls[0].contains("huggingface.co"));
        }
    }

    #[test]
    fn rejects_too_small_model_files() {
        let dir = std::env::temp_dir().join("calliop-llm-model-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("tiny.gguf");
        std::fs::write(&path, vec![0u8; 1024]).unwrap();
        assert!(!is_valid_model_file(LlmModel::Qwen3_1_7B, &path));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn qwen3_0_6b_min_bytes_fits_huggingface_release() {
        // Published unsloth Qwen3-0.6B-Q4_K_M.gguf size (bytes).
        const HUGGINGFACE_QWEN3_0_6B_BYTES: u64 = 396_705_472;
        assert!(HUGGINGFACE_QWEN3_0_6B_BYTES >= LlmModel::Qwen3_0_6B.min_bytes());
    }

    #[test]
    fn ensure_llm_model_file_rejects_auto() {
        assert!(ensure_llm_model_file_blocking(None, LlmModel::Auto).is_err());
    }

    #[test]
    fn parses_model_ids() {
        assert_eq!(LlmModel::parse("qwen3-0.6b"), Some(LlmModel::Qwen3_0_6B));
        assert_eq!(LlmModel::parse("qwen3-1.7b"), Some(LlmModel::Qwen3_1_7B));
        assert_eq!(LlmModel::parse("qwen3-4b"), Some(LlmModel::Qwen3_4B));
    }
}
