use std::path::Path;

use thiserror::Error;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

pub const DEFAULT_LANGUAGE: &str = "fr";

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

    pub fn transcribe(&self, audio: &[f32]) -> Result<String, SttError> {
        if audio.is_empty() {
            return Err(SttError::EmptyAudio);
        }

        let mut state = self
            .context
            .create_state()
            .map_err(|e| SttError::CreateState(e.to_string()))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        configure_full_params(&mut params, self.language(), self.n_threads());
        state
            .full(params, audio)
            .map_err(|e| SttError::Transcribe(e.to_string()))?;

        collect_transcript(&state)
    }
}

pub fn configure_full_params<'a>(
    params: &mut FullParams<'a, 'a>,
    language: &'a str,
    n_threads: i32,
) {
    params.set_language(Some(language));
    params.set_translate(false);
    params.set_n_threads(n_threads);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
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
}
