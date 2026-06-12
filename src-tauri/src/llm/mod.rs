//! Local LLM post-processing for auto-edits (Phase 3+).

mod client;
mod engine;
mod model;
mod prompt;

pub use engine::{ensure_engine_ready, LlamaEngine};
pub use model::{
    ensure_llm_model_blocking, ensure_llm_model_file_blocking, invalidate_corrupt_model_file,
    is_corrupt_model_load_error, is_valid_model_file, model_exists, model_path, LlmModel,
    LlmModelDownloadProgress, LlmModelError, DEFAULT_MODEL_FILE,
};
pub use prompt::{
    build_cleanup_user_message, validate_cleanup_output, QWEN3_CHAT_TEMPLATE, SYSTEM_PROMPT,
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("worker error: {0}")]
    Worker(String),
    #[error(transparent)]
    Prompt(#[from] prompt::PromptError),
}

pub fn module_name() -> &'static str {
    "llm"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_module_name() {
        assert_eq!(module_name(), "llm");
    }
}
