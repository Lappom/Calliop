use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use thiserror::Error;

pub const DEFAULT_MODEL_FILE: &str = "ggml-small.bin";

/// ggml-small.bin is ~466 MiB; reject truncated or HTML error-page downloads.
pub const EXPECTED_MODEL_MIN_BYTES: u64 = 450_000_000;

/// Ordered download sources. Hugging Face is primary until a Calliop mirror exists.
pub fn model_download_urls() -> &'static [&'static str] {
    &[
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
        // Future Calliop mirror (enable when the release asset is published).
        // "https://github.com/calliop-app/calliop/releases/download/models-v0/ggml-small.bin",
    ]
}

pub fn models_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.calliop.app")
        .join("models")
}

pub fn model_path() -> PathBuf {
    models_dir().join(DEFAULT_MODEL_FILE)
}

pub fn model_exists() -> bool {
    model_path().exists()
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelDownloadProgress {
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
    #[error(
        "downloaded model is too small ({size} bytes, expected >= {EXPECTED_MODEL_MIN_BYTES})"
    )]
    ModelTooSmall { size: u64 },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn is_valid_model_file(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|meta| meta.len() >= EXPECTED_MODEL_MIN_BYTES)
        .unwrap_or(false)
}

pub fn ensure_model_blocking(app: Option<&AppHandle>) -> Result<PathBuf, ModelError> {
    let path = model_path();
    if path.exists() {
        if is_valid_model_file(&path) {
            return Ok(path);
        }
        // Corrupt or partial download from a previous run — remove and retry.
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

    rt.block_on(download_model(app, &path))
}

pub async fn download_model(app: Option<&AppHandle>, dest: &Path) -> Result<PathBuf, ModelError> {
    let client = reqwest::Client::builder()
        .user_agent("Calliop/0.1")
        .build()
        .map_err(|e| ModelError::Download {
            url: "client".into(),
            message: e.to_string(),
        })?;

    let mut last_error = None;

    for url in model_download_urls() {
        match try_download(&client, app, url, dest).await {
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
    if size < EXPECTED_MODEL_MIN_BYTES {
        return Err(ModelError::ModelTooSmall { size });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_urls_use_huggingface_primary() {
        let urls = model_download_urls();
        assert!(urls[0].contains("huggingface.co"));
    }

    #[test]
    fn rejects_too_small_model_files() {
        let dir = std::env::temp_dir().join("calliop-model-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("tiny.bin");
        std::fs::write(&path, vec![0u8; 1024]).unwrap();
        assert!(!is_valid_model_file(&path));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn default_model_filename() {
        assert!(model_path().ends_with(DEFAULT_MODEL_FILE));
    }
}
