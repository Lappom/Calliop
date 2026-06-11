use std::path::Path;

use thiserror::Error;
use whisper_rs::{
    get_lang_str, FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters,
    WhisperState,
};

use calliop_prompt::{whisper_oral_vocabulary_word_count, WHISPER_ORAL_VOCABULARY_HINT};

use super::language::SttLanguage;

pub const DEFAULT_LANGUAGE: &str = "fr";
pub const MAX_INITIAL_PROMPT_WORDS: usize = 200;
/// Snippet triggers reserved in the Whisper initial prompt before dictionary words.
pub const MAX_SNIPPET_PROMPT_WORDS: usize = 50;

#[derive(Debug, Error)]
pub enum SttError {
    #[error("failed to load whisper model at {path}: {message}")]
    LoadModel { path: String, message: String },
    #[error("failed to create whisper state: {0}")]
    CreateState(String),
    #[error("transcription failed: {0}")]
    Transcribe(String),
    #[error("empty audio buffer")]
    EmptyAudio,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptResult {
    pub text: String,
    pub detected_language: Option<String>,
}

pub struct WhisperEngine {
    // Kept alive so WhisperState remains valid for the engine lifetime.
    #[allow(dead_code)]
    context: WhisperContext,
    state: WhisperState,
    language: SttLanguage,
    n_threads: i32,
}

impl WhisperEngine {
    pub fn new(model_path: &Path) -> Result<Self, SttError> {
        let path_str = model_path.to_str().ok_or_else(|| SttError::LoadModel {
            path: model_path.display().to_string(),
            message: "invalid UTF-8 path".into(),
        })?;

        let context =
            WhisperContext::new_with_params(path_str, WhisperContextParameters::default())
                .map_err(|e| SttError::LoadModel {
                    path: path_str.into(),
                    message: e.to_string(),
                })?;

        let state = context
            .create_state()
            .map_err(|e| SttError::CreateState(e.to_string()))?;

        Ok(Self {
            context,
            state,
            language: SttLanguage::default_fixed(),
            n_threads: std::thread::available_parallelism()
                .map(|n| n.get() as i32)
                .unwrap_or(4)
                .min(8),
        })
    }

    pub fn language(&self) -> &SttLanguage {
        &self.language
    }

    pub fn set_language(&mut self, language: SttLanguage) {
        self.language = language;
    }

    pub fn n_threads(&self) -> i32 {
        self.n_threads
    }

    pub fn transcribe(
        &mut self,
        audio: &[f32],
        initial_prompt: Option<&str>,
    ) -> Result<TranscriptResult, SttError> {
        self.transcribe_with_language(audio, initial_prompt, self.language)
    }

    pub fn transcribe_with_language(
        &mut self,
        audio: &[f32],
        initial_prompt: Option<&str>,
        language: SttLanguage,
    ) -> Result<TranscriptResult, SttError> {
        if audio.is_empty() {
            return Err(SttError::EmptyAudio);
        }

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        configure_full_params(&mut params, language, self.n_threads(), initial_prompt);
        self.state
            .full(params, audio)
            .map_err(|e| SttError::Transcribe(e.to_string()))?;

        let text = collect_transcript(&self.state)?;
        let detected_language = if matches!(language, SttLanguage::Auto) {
            let lang_id = self.state.full_lang_id_from_state();
            get_lang_str(lang_id).map(str::to_string)
        } else {
            None
        };

        Ok(TranscriptResult {
            text,
            detected_language,
        })
    }
}

/// Merges oral-symbol hint, snippet triggers, and dictionary words into a capped Whisper prompt.
pub fn build_whisper_initial_prompt(
    snippet_triggers: &[String],
    dictionary_words: &[String],
) -> Option<String> {
    let hint_words = whisper_oral_vocabulary_word_count();
    let user_budget = MAX_INITIAL_PROMPT_WORDS.saturating_sub(hint_words);

    let mut user_words = Vec::with_capacity(user_budget);
    user_words.extend(
        snippet_triggers
            .iter()
            .take(MAX_SNIPPET_PROMPT_WORDS)
            .cloned(),
    );
    let remaining = user_budget.saturating_sub(user_words.len());
    user_words.extend(dictionary_words.iter().take(remaining).cloned());

    if user_words.is_empty() {
        Some(WHISPER_ORAL_VOCABULARY_HINT.to_string())
    } else {
        Some(format!(
            "{} {}",
            WHISPER_ORAL_VOCABULARY_HINT,
            user_words.join(", ")
        ))
    }
}

pub fn build_initial_prompt(words: &[String]) -> Option<String> {
    if words.is_empty() {
        return None;
    }

    let prompt = words
        .iter()
        .take(MAX_INITIAL_PROMPT_WORDS)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");

    if prompt.is_empty() {
        None
    } else {
        Some(prompt)
    }
}

pub fn configure_full_params(
    params: &mut FullParams<'_, '_>,
    language: SttLanguage,
    n_threads: i32,
    initial_prompt: Option<&str>,
) {
    match language {
        SttLanguage::Auto => {
            params.set_detect_language(true);
        }
        SttLanguage::Fixed(code) => {
            params.set_language(Some(code));
        }
    }
    params.set_translate(false);
    params.set_n_threads(n_threads);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    if let Some(prompt) = initial_prompt {
        if !prompt.is_empty() {
            params.set_initial_prompt(prompt);
        }
    }
}

fn collect_transcript(state: &WhisperState) -> Result<String, SttError> {
    let text = state
        .as_iter()
        .map(|segment| segment.to_string())
        .collect::<Vec<_>>()
        .join(" ");

    Ok(text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_language_is_french() {
        assert_eq!(DEFAULT_LANGUAGE, "fr");
    }

    #[test]
    fn build_initial_prompt_joins_words() {
        let words = vec!["Calliop".into(), "Whisper".into()];
        assert_eq!(
            build_initial_prompt(&words),
            Some("Calliop, Whisper".into())
        );
    }

    #[test]
    fn build_initial_prompt_caps_word_count() {
        let words = (0..MAX_INITIAL_PROMPT_WORDS + 10)
            .map(|i| format!("word{i}"))
            .collect::<Vec<_>>();
        let prompt = build_initial_prompt(&words).expect("prompt");
        assert_eq!(prompt.matches(", ").count() + 1, MAX_INITIAL_PROMPT_WORDS);
    }

    #[test]
    fn build_initial_prompt_returns_none_for_empty_input() {
        assert_eq!(build_initial_prompt(&[]), None);
    }

    #[test]
    fn build_whisper_initial_prompt_always_includes_oral_hint() {
        let prompt = build_whisper_initial_prompt(&[], &[]).expect("prompt");
        assert!(prompt.starts_with(WHISPER_ORAL_VOCABULARY_HINT));
        assert!(prompt.contains("arobase"));
        assert!(prompt.contains("slash"));
    }

    #[test]
    fn build_whisper_initial_prompt_reserves_dictionary_slots() {
        let snippets: Vec<String> = (0..MAX_SNIPPET_PROMPT_WORDS + 20)
            .map(|i| format!("snippet{i}"))
            .collect();
        let dictionary: Vec<String> = (0..MAX_INITIAL_PROMPT_WORDS)
            .map(|i| format!("word{i}"))
            .collect();

        let prompt = build_whisper_initial_prompt(&snippets, &dictionary).expect("prompt");
        assert!(prompt.starts_with(WHISPER_ORAL_VOCABULARY_HINT));
        let user_part = prompt
            .strip_prefix(WHISPER_ORAL_VOCABULARY_HINT)
            .expect("hint prefix")
            .trim_start();
        let entries: Vec<&str> = user_part.split(", ").collect();
        let hint_words = whisper_oral_vocabulary_word_count();
        let expected_user_words = MAX_INITIAL_PROMPT_WORDS.saturating_sub(hint_words);
        assert_eq!(entries.len(), expected_user_words);
        assert_eq!(entries[0], "snippet0");
        assert_eq!(entries[MAX_SNIPPET_PROMPT_WORDS - 1], "snippet49");
        assert_eq!(entries[MAX_SNIPPET_PROMPT_WORDS], "word0");
    }
}
