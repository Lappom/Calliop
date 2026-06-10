use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Instant;

use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};
use thiserror::Error;

use crate::audio::{AudioCapture, VadSegmenter};
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

#[derive(Debug, Clone, Serialize)]
pub struct PartialTranscriptEvent {
    pub text: String,
    #[serde(rename = "segmentIndex")]
    pub segment_index: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct LatencyMetricsEvent {
    #[serde(rename = "sttMs")]
    pub stt_ms: u64,
    #[serde(rename = "injectMs")]
    pub inject_ms: u64,
    #[serde(rename = "totalMs")]
    pub total_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioLevelEvent {
    pub level: f32,
}

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error(transparent)]
    Audio(#[from] crate::audio::AudioError),
    #[error(transparent)]
    Vad(#[from] crate::audio::VadError),
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

struct StreamingSession {
    stop_flag: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    level_listener: Option<JoinHandle<()>>,
}

pub struct PipelineOrchestrator {
    state: PipelineState,
    audio: AudioCapture,
    injector: TextInjector,
    whisper: Arc<Mutex<Option<WhisperEngine>>>,
    streaming: Option<StreamingSession>,
    segment_transcripts: Arc<Mutex<Vec<String>>>,
}

impl PipelineOrchestrator {
    pub fn new() -> Result<Self, PipelineError> {
        Ok(Self {
            state: PipelineState::Idle,
            audio: AudioCapture::new()?,
            injector: TextInjector::new()?,
            whisper: Arc::new(Mutex::new(None)),
            streaming: None,
            segment_transcripts: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn state(&self) -> PipelineState {
        self.state
    }

    pub fn set_whisper_engine(&mut self, engine: Arc<Mutex<Option<WhisperEngine>>>) {
        self.whisper = engine;
    }

    pub fn is_model_loaded(&self) -> bool {
        self.whisper.lock().is_some()
    }

    pub fn start(&mut self, app: &AppHandle) -> Result<(), PipelineError> {
        if self.state != PipelineState::Idle {
            return Err(PipelineError::Busy(self.state));
        }
        self.begin_recording(app)
    }

    pub fn stop(&mut self, app: &AppHandle) -> Result<(), PipelineError> {
        if self.state != PipelineState::Recording {
            return Err(PipelineError::Busy(self.state));
        }
        self.finish_dictation(app)
    }

    pub fn toggle(&mut self, app: &AppHandle) -> Result<(), PipelineError> {
        match self.state {
            PipelineState::Idle => self.begin_recording(app),
            PipelineState::Recording => self.finish_dictation(app),
            PipelineState::Transcribing | PipelineState::Injecting => {
                Err(PipelineError::Busy(self.state))
            }
        }
    }

    fn begin_recording(&mut self, app: &AppHandle) -> Result<(), PipelineError> {
        if self.whisper.lock().is_none() {
            return Err(PipelineError::ModelNotLoaded);
        }

        self.segment_transcripts.lock().clear();
        let _ = app.emit("partial-transcript-reset", ());

        let (chunk_tx, chunk_rx) = std::sync::mpsc::channel();
        let (level_tx, level_rx) = std::sync::mpsc::channel();
        self.audio
            .start_with_streaming(Some(chunk_tx), Some(level_tx))?;

        let stop_flag = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop_flag);
        let whisper = Arc::clone(&self.whisper);
        let app_worker = app.clone();

        let transcripts = Arc::clone(&self.segment_transcripts);
        let worker = thread::spawn(move || {
            streaming_worker(app_worker, chunk_rx, worker_stop, whisper, transcripts);
        });

        let app_level = app.clone();
        let level_stop = Arc::clone(&stop_flag);
        let level_listener = thread::spawn(move || {
            while !level_stop.load(Ordering::SeqCst) {
                match level_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                    Ok(level) => {
                        let _ = app_level.emit("audio-level", AudioLevelEvent { level });
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        self.streaming = Some(StreamingSession {
            stop_flag,
            worker: Some(worker),
            level_listener: Some(level_listener),
        });

        show_overlay(app);
        self.set_state(app, PipelineState::Recording, None);
        Ok(())
    }

    fn finish_dictation(&mut self, app: &AppHandle) -> Result<(), PipelineError> {
        let stop_instant = Instant::now();

        let samples = self.audio.stop()?;

        if let Some(session) = self.streaming.take() {
            session.stop_flag.store(true, Ordering::SeqCst);
            if let Some(worker) = session.worker {
                let _ = worker.join();
            }
            if let Some(listener) = session.level_listener {
                let _ = listener.join();
            }
        }

        self.set_state(app, PipelineState::Transcribing, None);

        let stt_start = Instant::now();
        let mut transcripts = self.segment_transcripts.lock().clone();

        // Fallback: if VAD produced no segments, transcribe the full buffer.
        if transcripts.is_empty() {
            let engine_guard = self.whisper.lock();
            let Some(engine) = engine_guard.as_ref() else {
                return Err(PipelineError::ModelNotLoaded);
            };
            if !samples.is_empty() {
                if let Ok(text) = engine.transcribe(&samples) {
                    if !text.is_empty() {
                        transcripts.push(text);
                    }
                }
            }
        }

        let stt_ms = stt_start.elapsed().as_millis() as u64;
        let full_text = transcripts.join(" ").trim().to_string();

        self.set_state(app, PipelineState::Injecting, None);
        let inject_start = Instant::now();
        if !full_text.is_empty() {
            self.injector.inject(&full_text)?;
        }
        let inject_ms = inject_start.elapsed().as_millis() as u64;
        let total_ms = stop_instant.elapsed().as_millis() as u64;

        let _ = app.emit(
            "latency-metrics",
            LatencyMetricsEvent {
                stt_ms,
                inject_ms,
                total_ms,
            },
        );

        hide_overlay(app);
        self.set_state(app, PipelineState::Idle, Some(full_text));
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

fn streaming_worker(
    app: AppHandle,
    chunk_rx: std::sync::mpsc::Receiver<Vec<f32>>,
    stop_flag: Arc<AtomicBool>,
    whisper: Arc<Mutex<Option<WhisperEngine>>>,
    transcripts: Arc<Mutex<Vec<String>>>,
) {
    let Ok(mut vad) = VadSegmenter::new() else {
        return;
    };

    let orchestrator_state = transcripts;
    let mut segment_index = 0_u32;

    while !stop_flag.load(Ordering::SeqCst) {
        match chunk_rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(chunk) => {
                let segments = match vad.push(&chunk) {
                    Ok(segments) => segments,
                    Err(err) => {
                        eprintln!("VAD error: {err}");
                        continue;
                    }
                };
                for segment in segments {
                    transcribe_segment(&app, &whisper, &orchestrator_state, segment, segment_index);
                    segment_index += 1;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    if let Ok(segments) = vad.flush() {
        for segment in segments {
            transcribe_segment(&app, &whisper, &orchestrator_state, segment, segment_index);
            segment_index += 1;
        }
    }
}

fn transcribe_segment(
    app: &AppHandle,
    whisper: &Arc<Mutex<Option<WhisperEngine>>>,
    transcripts: &Arc<Mutex<Vec<String>>>,
    segment: Vec<f32>,
    segment_index: u32,
) {
    let text = {
        let engine_guard = whisper.lock();
        let Some(engine) = engine_guard.as_ref() else {
            return;
        };
        match engine.transcribe(&segment) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("segment transcription failed: {err}");
                return;
            }
        }
    };

    if text.is_empty() {
        return;
    }

    transcripts.lock().push(text.clone());

    let _ = app.emit(
        "partial-transcript",
        PartialTranscriptEvent {
            text,
            segment_index,
        },
    );
}

pub fn show_overlay(app: &AppHandle) {
    let Some(overlay) = app.get_webview_window("overlay") else {
        return;
    };

    if let Ok(Some(monitor)) = overlay.current_monitor() {
        let monitor_size = monitor.size();
        let overlay_size = overlay.outer_size().unwrap_or(PhysicalSize {
            width: 300,
            height: 88,
        });
        let x = (monitor_size.width.saturating_sub(overlay_size.width)) / 2;
        let y = monitor_size
            .height
            .saturating_sub(overlay_size.height)
            .saturating_sub(48);
        let _ = overlay.set_position(PhysicalPosition::new(x as i32, y as i32));
    }

    let _ = overlay.show();
}

pub fn hide_overlay(app: &AppHandle) {
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.hide();
    }
}

pub fn spawn_start(app: AppHandle, orchestrator: Arc<Mutex<PipelineOrchestrator>>) {
    std::thread::spawn(move || {
        let result = {
            let mut guard = orchestrator.lock();
            guard.start(&app)
        };

        if let Err(err) = result {
            emit_error(&app, &orchestrator, err);
        }
    });
}

pub fn spawn_stop(app: AppHandle, orchestrator: Arc<Mutex<PipelineOrchestrator>>) {
    std::thread::spawn(move || {
        let result = {
            let mut guard = orchestrator.lock();
            guard.stop(&app)
        };

        if let Err(err) = result {
            emit_error(&app, &orchestrator, err);
        }
    });
}

pub fn spawn_toggle(app: AppHandle, orchestrator: Arc<Mutex<PipelineOrchestrator>>) {
    std::thread::spawn(move || {
        let result = {
            let mut guard = orchestrator.lock();
            guard.toggle(&app)
        };

        if let Err(err) = result {
            emit_error(&app, &orchestrator, err);
        }
    });
}

fn emit_error(
    app: &AppHandle,
    orchestrator: &Arc<Mutex<PipelineOrchestrator>>,
    err: PipelineError,
) {
    let _ = app.emit(
        "pipeline-state",
        PipelineStateEvent {
            state: "error".into(),
            message: Some(err.to_string()),
        },
    );
    let mut guard = orchestrator.lock();
    if guard.state() != PipelineState::Idle {
        hide_overlay(app);
        guard.set_state(app, PipelineState::Idle, None);
    }
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
