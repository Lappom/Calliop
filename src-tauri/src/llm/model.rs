use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use thiserror::Error;

use crate::stt::models_dir;

pub const DEFAULT_MODEL_FILE: &str = "qwen3-1.7b-instruct-q4_k_m.gguf";

// Published Hugging Face release sizes (bytes).
const QWEN3_0_6B_BYTES: u64 = 396_705_472;
const QWEN3_1_7B_BYTES: u64 = 1_107_404_512;
const QWEN3_5_4B_BYTES: u64 = 2_740_937_888;

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
    #[serde(rename = "qwen3.5-4b")]
    Qwen3_5_4B,
}

impl LlmModel {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "qwen3-0.6b" | "qwen3_0.6b" => Some(Self::Qwen3_0_6B),
            "qwen3-1.7b" | "qwen3_1.7b" => Some(Self::Qwen3_1_7B),
            "qwen3-4b" | "qwen3_4b" | "qwen3.5-4b" | "qwen3_5_4b" => Some(Self::Qwen3_5_4B),
            _ => None,
        }
    }

    pub fn as_setting_value(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Qwen3_0_6B => "qwen3-0.6b",
            Self::Qwen3_1_7B => "qwen3-1.7b",
            Self::Qwen3_5_4B => "qwen3.5-4b",
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
            Self::Qwen3_5_4B => Some("qwen3.5-4b-instruct-q4_k_m.gguf"),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Automatique (recommandé)",
            Self::Qwen3_0_6B => "Qwen3 0.6B Instruct Q4_K_M (~378 Mo)",
            Self::Qwen3_1_7B => "Qwen3 1.7B Instruct Q4_K_M (~1,1 Go)",
            Self::Qwen3_5_4B => "Qwen3.5 4B Instruct Q4_K_M (~2,7 Go, GPU recommandé)",
        }
    }

    pub fn min_bytes(self) -> u64 {
        match self {
            Self::Auto => 0,
            Self::Qwen3_0_6B => 370_000_000,
            Self::Qwen3_1_7B => 1_000_000_000,
            Self::Qwen3_5_4B => 2_600_000_000,
        }
    }

    pub fn expected_bytes(self) -> Option<u64> {
        match self {
            Self::Auto => None,
            Self::Qwen3_0_6B => Some(QWEN3_0_6B_BYTES),
            Self::Qwen3_1_7B => Some(QWEN3_1_7B_BYTES),
            Self::Qwen3_5_4B => Some(QWEN3_5_4B_BYTES),
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
            Self::Qwen3_5_4B => &[
                "https://huggingface.co/unsloth/Qwen3.5-4B-GGUF/resolve/main/Qwen3.5-4B-Q4_K_M.gguf",
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
        [Self::Qwen3_0_6B, Self::Qwen3_1_7B, Self::Qwen3_5_4B]
    }

    pub fn all_selectable() -> [Self; 4] {
        [
            Self::Auto,
            Self::Qwen3_0_6B,
            Self::Qwen3_1_7B,
            Self::Qwen3_5_4B,
        ]
    }
}

pub const LEGACY_QWEN3_4B_MODEL_FILE: &str = "qwen3-4b-instruct-q4_k_m.gguf";

pub fn legacy_qwen3_4b_model_path() -> PathBuf {
    models_dir().join(LEGACY_QWEN3_4B_MODEL_FILE)
}

/// Remove orphaned Qwen3 4B weights after migration to Qwen3.5 4B.
pub fn remove_legacy_qwen3_4b_model() -> std::io::Result<()> {
    let path = legacy_qwen3_4b_model_path();
    if path.exists() {
        std::fs::remove_file(path)
    } else {
        Ok(())
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
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    let size = meta.len();
    if size < model.min_bytes() {
        return false;
    }
    model
        .expected_bytes()
        .is_none_or(|expected| size == expected)
}

pub fn is_corrupt_model_load_error(err: &str) -> bool {
    let lower = err.to_ascii_lowercase();
    lower.contains("corrupted")
        || lower.contains("incomplete")
        || lower.contains("not within the file bounds")
}

/// Removes a model file when llama.cpp reports corruption so the next ensure retries download.
pub fn invalidate_corrupt_model_file(model: LlmModel, err: &str) -> bool {
    if !model.is_concrete() || !is_corrupt_model_load_error(err) {
        return false;
    }
    let path = model.path();
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
    true
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
    if let Some(expected) = total {
        if size != expected {
            return Err(LlmModelError::Download {
                url: url.into(),
                message: format!("size mismatch: got {size} bytes, expected {expected}"),
            });
        }
    } else if let Some(expected) = model.expected_bytes() {
        if size != expected {
            return Err(LlmModelError::Download {
                url: url.into(),
                message: format!("size mismatch: got {size} bytes, expected {expected}"),
            });
        }
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
    fn llm_model_release_sizes_fit_min_bytes() {
        assert!(QWEN3_0_6B_BYTES >= LlmModel::Qwen3_0_6B.min_bytes());
        assert!(QWEN3_1_7B_BYTES >= LlmModel::Qwen3_1_7B.min_bytes());
        assert!(QWEN3_5_4B_BYTES >= LlmModel::Qwen3_5_4B.min_bytes());
    }

    #[test]
    fn rejects_truncated_qwen3_5_4b_file() {
        let dir = std::env::temp_dir().join("calliop-llm-35-4b-truncated-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("qwen3.5-4b-instruct-q4_k_m.gguf");
        std::fs::write(&path, vec![0u8; 2_600_000_000]).unwrap();
        assert!(!is_valid_model_file(LlmModel::Qwen3_5_4B, &path));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn detects_corrupt_model_load_errors() {
        assert!(is_corrupt_model_load_error(
            "tensor data is not within the file bounds, model is corrupted or incomplete"
        ));
        assert!(!is_corrupt_model_load_error(
            "worker ready handshake failed"
        ));
    }

    #[test]
    fn ensure_llm_model_file_rejects_auto() {
        assert!(ensure_llm_model_file_blocking(None, LlmModel::Auto).is_err());
    }

    #[test]
    fn legacy_qwen3_4b_path_uses_expected_filename() {
        assert!(legacy_qwen3_4b_model_path().ends_with(LEGACY_QWEN3_4B_MODEL_FILE));
    }

    #[test]
    fn remove_legacy_qwen3_4b_is_ok_when_file_absent() {
        assert!(remove_legacy_qwen3_4b_model().is_ok());
    }

    #[test]
    fn parses_model_ids() {
        assert_eq!(LlmModel::parse("qwen3-0.6b"), Some(LlmModel::Qwen3_0_6B));
        assert_eq!(LlmModel::parse("qwen3-1.7b"), Some(LlmModel::Qwen3_1_7B));
        assert_eq!(LlmModel::parse("qwen3.5-4b"), Some(LlmModel::Qwen3_5_4B));
        assert_eq!(LlmModel::parse("qwen3-4b"), Some(LlmModel::Qwen3_5_4B));
    }
}
