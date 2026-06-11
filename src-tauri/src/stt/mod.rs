//! Speech-to-text via whisper bindings (Phase 1+).

mod engine;
mod language;
mod model;
mod prompt_cache;
mod wer;

pub use engine::{
    build_initial_prompt, build_whisper_initial_prompt, configure_full_params, SttError,
    TranscriptResult, WhisperEngine, DEFAULT_LANGUAGE, MAX_INITIAL_PROMPT_WORDS,
    MAX_SNIPPET_PROMPT_WORDS,
};
pub use language::{SttLanguage, DEFAULT_STT_LANGUAGE, STT_LANG_AUTO, SUPPORTED_STT_LANGUAGES};
pub use model::{
    ensure_model_blocking, is_valid_model_file, legacy_medium_model_path, model_exists, model_path,
    models_dir, remove_legacy_medium_model, ModelDownloadProgress, ModelError, WhisperModel,
    DEFAULT_MODEL_FILE, LEGACY_MEDIUM_MODEL_FILE,
};
pub use prompt_cache::WhisperPromptCache;
pub use wer::{tokenize_for_wer, word_edit_distance, word_error_rate};

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
