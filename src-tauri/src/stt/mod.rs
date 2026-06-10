//! Speech-to-text via whisper bindings (Phase 1+).

mod engine;
mod model;

pub use engine::{configure_full_params, SttError, WhisperEngine, DEFAULT_LANGUAGE};
pub use model::{
    ensure_model_blocking, is_valid_model_file, model_download_urls, model_exists, model_path,
    models_dir, ModelDownloadProgress, ModelError, DEFAULT_MODEL_FILE, EXPECTED_MODEL_MIN_BYTES,
};

pub fn module_name() -> &'static str {
    "stt"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_module_name() {
        assert_eq!(module_name(), "stt");
    }
}
