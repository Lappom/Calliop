use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use thiserror::Error;

use crate::audio::{AudioCapture, AudioError};
use crate::inject::{InjectError, TextInjector};
use crate::stt::{SttError, WhisperEngine};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineState {
    Idle,
    Recording,
    Transcribing,
    Injecting,
}

impl PipelineState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Recording => "recording",
            Self::Transcribing => "transcribing",
            Self::Injecting => "injecting",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PipelineStateEvent {
    pub state: String,
    pub message: Option<String>,
}

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error(transparent)]
    Audio(#[from] AudioError),
    #[error(transparent)]
    Stt(#[from] SttError),
    #[error(transparent)]
    Inject(#[from] InjectError),
    #[error("whisper model not loaded")]
    ModelNotLoaded,
    #[error("pipeline busy ({0:?})")]
    Busy(PipelineState),
    #[error("background task failed: {0}")]
    Background(String),
}

pub struct PipelineOrchestrator {
    state: PipelineState,
    audio: AudioCapture,
    injector: TextInjector,
    model_path: Option<PathBuf>,
}

impl PipelineOrchestrator {
    pub fn new() -> Result<Self, PipelineError> {
        Ok(Self {
            state: PipelineState::Idle,
            audio: AudioCapture::new()?,
            injector: TextInjector::new()?,
            model_path: None,
        })
    }

    pub fn state(&self) -> PipelineState {
        self.state
    }

    pub fn set_model_path(&mut self, path: PathBuf) {
        self.model_path = Some(path);
    }

    pub fn is_model_loaded(&self) -> bool {
        self.model_path.is_some()
    }

    pub fn toggle(&mut self, app: &AppHandle) -> Result<(), PipelineError> {
        match self.state {
            PipelineState::Idle => self.start_recording(app),
            PipelineState::Recording => self.finish_dictation(app),
            PipelineState::Transcribing | PipelineState::Injecting => {
                Err(PipelineError::Busy(self.state))
            }
        }
    }

    fn start_recording(&mut self, app: &AppHandle) -> Result<(), PipelineError> {
        self.audio.start()?;
        self.set_state(app, PipelineState::Recording, None);
        Ok(())
    }

    fn finish_dictation(&mut self, app: &AppHandle) -> Result<(), PipelineError> {
        let samples = self.audio.stop()?;
        self.set_state(app, PipelineState::Transcribing, None);

        let model_path = self
            .model_path
            .clone()
            .ok_or(PipelineError::ModelNotLoaded)?;
        let engine = WhisperEngine::new(&model_path)?;
        let text = engine.transcribe(&samples)?;

        self.set_state(app, PipelineState::Injecting, None);
        self.injector.inject(&text)?;
        self.set_state(app, PipelineState::Idle, Some(text));
        Ok(())
    }

    pub(crate) fn set_state(
        &mut self,
        app: &AppHandle,
        state: PipelineState,
        transcript: Option<String>,
    ) {
        self.state = state;
        let message = if state == PipelineState::Idle {
            transcript
        } else {
            None
        };
        let _ = app.emit(
            "pipeline-state",
            PipelineStateEvent {
                state: state.as_str().into(),
                message,
            },
        );
    }
}

pub fn spawn_toggle(app: AppHandle, orchestrator: Arc<Mutex<PipelineOrchestrator>>) {
    std::thread::spawn(move || {
        let result = {
            let mut guard = orchestrator.lock();
            guard.toggle(&app)
        };

        if let Err(err) = result {
            let _ = app.emit(
                "pipeline-state",
                PipelineStateEvent {
                    state: "error".into(),
                    message: Some(err.to_string()),
                },
            );
            let mut guard = orchestrator.lock();
            if guard.state() != PipelineState::Idle {
                guard.set_state(&app, PipelineState::Idle, None);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_state_serializes_lowercase() {
        assert_eq!(PipelineState::Recording.as_str(), "recording");
    }

    #[test]
    fn new_orchestrator_starts_idle_without_model() {
        let orchestrator = PipelineOrchestrator::new().expect("orchestrator");
        assert_eq!(orchestrator.state(), PipelineState::Idle);
        assert!(!orchestrator.is_model_loaded());
    }
}
