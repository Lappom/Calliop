//! Speech-to-text via whisper bindings (Phase 1+).

mod engine;
mod language;
mod model;

pub use engine::{
    build_initial_prompt, build_whisper_initial_prompt, configure_full_params, SttError,
    TranscriptResult, WhisperEngine, DEFAULT_LANGUAGE, MAX_INITIAL_PROMPT_WORDS,
    MAX_SNIPPET_PROMPT_WORDS,
};
pub use language::{SttLanguage, DEFAULT_STT_LANGUAGE, STT_LANG_AUTO, SUPPORTED_STT_LANGUAGES};
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
