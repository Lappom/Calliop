use std::path::Path;

use thiserror::Error;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

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

pub struct WhisperEngine {
    context: WhisperContext,
    language: String,
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

        Ok(Self {
            context,
            language: DEFAULT_LANGUAGE.into(),
            n_threads: std::thread::available_parallelism()
                .map(|n| n.get() as i32)
                .unwrap_or(4)
                .min(8),
        })
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    pub fn n_threads(&self) -> i32 {
        self.n_threads
    }

    pub fn transcribe(
        &self,
        audio: &[f32],
        initial_prompt: Option<&str>,
    ) -> Result<String, SttError> {
        if audio.is_empty() {
            return Err(SttError::EmptyAudio);
        }

        let mut state = self
            .context
            .create_state()
            .map_err(|e| SttError::CreateState(e.to_string()))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        configure_full_params(
            &mut params,
            self.language(),
            self.n_threads(),
            initial_prompt,
        );
        state
            .full(params, audio)
            .map_err(|e| SttError::Transcribe(e.to_string()))?;

        collect_transcript(&state)
    }
}

/// Merges snippet triggers and dictionary words into a capped Whisper initial prompt.
pub fn build_whisper_initial_prompt(
    snippet_triggers: &[String],
    dictionary_words: &[String],
) -> Option<String> {
    let mut words = Vec::with_capacity(MAX_INITIAL_PROMPT_WORDS);
    words.extend(
        snippet_triggers
            .iter()
            .take(MAX_SNIPPET_PROMPT_WORDS)
            .cloned(),
    );
    let remaining = MAX_INITIAL_PROMPT_WORDS.saturating_sub(words.len());
    words.extend(dictionary_words.iter().take(remaining).cloned());
    build_initial_prompt(&words)
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

pub fn configure_full_params<'a>(
    params: &mut FullParams<'a, 'a>,
    language: &'a str,
    n_threads: i32,
    initial_prompt: Option<&str>,
) {
    params.set_language(Some(language));
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
    fn build_whisper_initial_prompt_reserves_dictionary_slots() {
        let snippets: Vec<String> = (0..MAX_SNIPPET_PROMPT_WORDS + 20)
            .map(|i| format!("snippet{i}"))
            .collect();
        let dictionary: Vec<String> = (0..MAX_INITIAL_PROMPT_WORDS)
            .map(|i| format!("word{i}"))
            .collect();

        let prompt = build_whisper_initial_prompt(&snippets, &dictionary).expect("prompt");
        let entries: Vec<&str> = prompt.split(", ").collect();
        assert_eq!(entries.len(), MAX_INITIAL_PROMPT_WORDS);
        assert_eq!(entries[0], "snippet0");
        assert_eq!(entries[MAX_SNIPPET_PROMPT_WORDS - 1], "snippet49");
        assert_eq!(entries[MAX_SNIPPET_PROMPT_WORDS], "word0");
    }
}
