use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Instant;

use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};
use thiserror::Error;

use calliop_prompt::{join_transcript_segments, post_process_transcript, ToneProfile};

use crate::app_context::{get_active_window, resolve_tone, ActiveWindow};
use crate::audio::{AudioCapture, VadSegmenter, TARGET_SAMPLE_RATE};
use crate::inject::{InjectError, TextInjector};
use crate::llm::LlamaEngine;
use crate::observe::CorrectionHandler;
use crate::store::{AppContextRule, NewDictation, Snippet, Store};
use crate::stt::{SttError, SttLanguage, WhisperEngine};

use super::corrections::{apply_corrections, CorrectionRule};
use super::snippet_variables::SnippetVariableContext;
use super::snippets::{
    apply_snippets, finalize_llm_with_snippets, shield_snippet_triggers, unshield_snippets,
};

fn build_snippet_variable_context(store: Option<&Store>) -> SnippetVariableContext {
    let user_name = store
        .and_then(|s| s.get_snippet_user_name().ok())
        .unwrap_or_default();
    let clipboard = TextInjector::read_clipboard_text().ok().flatten();
    SnippetVariableContext::from_user_name(user_name).with_clipboard(clipboard)
}

fn prepare_display_transcript(
    raw: &str,
    snippets: &[Snippet],
    ctx: &SnippetVariableContext,
) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let (shielded, shields) = shield_snippet_triggers(trimmed, snippets);
    let processed = post_process_transcript(&shielded);
    if shields.is_empty() {
        processed
    } else {
        unshield_snippets(&processed, &shields, ctx)
    }
}

/// Live overlay only: expand snippets without oral punctuation (avoids premature « point » → « . »).
fn prepare_partial_transcript(
    raw: &str,
    snippets: &[Snippet],
    ctx: &SnippetVariableContext,
) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    apply_snippets(trimmed, snippets, ctx)
}

/// Maximum time to wait for LLM cleanup before inject or worker kill.
const LLM_CLEANUP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(45);

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
    /// Cumulative Whisper inference time (overlaps with recording when streaming).
    #[serde(rename = "sttMs")]
    pub stt_ms: u64,
    /// Wall-clock STT drain after hotkey release until the transcript is ready.
    #[serde(rename = "sttWaitMs")]
    pub stt_wait_ms: u64,
    #[serde(rename = "llmMs")]
    pub llm_ms: u64,
    /// Wall-clock time blocked waiting for LLM before injection (0 when fast-path used).
    #[serde(rename = "llmBlockedMs")]
    pub llm_blocked_ms: u64,
    #[serde(rename = "injectMs")]
    pub inject_ms: u64,
    #[serde(rename = "totalMs")]
    pub total_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioLevelEvent {
    pub level: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SttLanguageChangedEvent {
    pub language: String,
    pub detected: bool,
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
    #[error(transparent)]
    Llm(#[from] crate::llm::LlmError),
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

struct FailedSegment {
    samples: Vec<f32>,
    language: SttLanguage,
}

pub struct PipelineOrchestrator {
    state: PipelineState,
    audio: AudioCapture,
    injector: TextInjector,
    whisper: Arc<Mutex<Option<WhisperEngine>>>,
    llm: Arc<Mutex<Option<LlamaEngine>>>,
    llm_ready: Arc<AtomicBool>,
    auto_edit: Arc<AtomicBool>,
    streaming: Option<StreamingSession>,
    segment_transcripts: Arc<Mutex<Vec<String>>>,
    failed_segments: Arc<Mutex<Vec<FailedSegment>>>,
    streaming_stt_ms: Arc<AtomicU64>,
    dictionary_prompt: Arc<RwLock<Option<Arc<str>>>>,
    snippets: Arc<RwLock<Vec<Snippet>>>,
    correction_rules: Arc<RwLock<Vec<CorrectionRule>>>,
    app_context_rules: Arc<RwLock<Vec<AppContextRule>>>,
    auto_learn: Arc<AtomicBool>,
    observer_generation: Arc<AtomicU64>,
    correction_handler: Option<CorrectionHandler>,
    default_stt_language: Arc<RwLock<SttLanguage>>,
    session_stt_language: Arc<RwLock<SttLanguage>>,
    session_detected_language: Arc<RwLock<Option<SttLanguage>>>,
    pending_detection: Arc<Mutex<Option<SttLanguage>>>,
    history_store: Option<Arc<Store>>,
}

impl PipelineOrchestrator {
    pub fn new() -> Result<Self, PipelineError> {
        Ok(Self {
            state: PipelineState::Idle,
            audio: AudioCapture::new()?,
            injector: TextInjector::new()?,
            whisper: Arc::new(Mutex::new(None)),
            llm: Arc::new(Mutex::new(None)),
            llm_ready: Arc::new(AtomicBool::new(false)),
            auto_edit: Arc::new(AtomicBool::new(false)),
            streaming: None,
            segment_transcripts: Arc::new(Mutex::new(Vec::new())),
            failed_segments: Arc::new(Mutex::new(Vec::new())),
            streaming_stt_ms: Arc::new(AtomicU64::new(0)),
            dictionary_prompt: Arc::new(RwLock::new(None)),
            snippets: Arc::new(RwLock::new(Vec::new())),
            correction_rules: Arc::new(RwLock::new(Vec::new())),
            app_context_rules: Arc::new(RwLock::new(Vec::new())),
            auto_learn: Arc::new(AtomicBool::new(true)),
            observer_generation: Arc::new(AtomicU64::new(0)),
            correction_handler: None,
            default_stt_language: Arc::new(RwLock::new(SttLanguage::default_fixed())),
            session_stt_language: Arc::new(RwLock::new(SttLanguage::default_fixed())),
            session_detected_language: Arc::new(RwLock::new(None)),
            pending_detection: Arc::new(Mutex::new(None)),
            history_store: None,
        })
    }

    pub fn set_history_store(&mut self, store: Arc<Store>) {
        self.history_store = Some(store);
    }

    pub fn set_dictionary_prompt(&mut self, prompt: Option<String>) {
        *self.dictionary_prompt.write() = prompt.map(|value| Arc::from(value.as_str()));
    }

    pub fn set_dictionary_prompt_arc(&mut self, prompt: Option<Arc<str>>) {
        *self.dictionary_prompt.write() = prompt;
    }

    pub fn set_snippets(&mut self, snippets: Vec<Snippet>) {
        *self.snippets.write() = snippets;
    }

    pub fn set_correction_rules(&mut self, rules: Vec<CorrectionRule>) {
        *self.correction_rules.write() = rules;
    }

    pub fn set_app_context_rules(&mut self, rules: Vec<AppContextRule>) {
        *self.app_context_rules.write() = rules;
    }

    fn resolve_active_context(&self) -> (ToneProfile, Option<ActiveWindow>) {
        let rules = self.app_context_rules.read().clone();
        match get_active_window() {
            Some(window) => {
                let tone = resolve_tone(&window, &rules);
                (tone, Some(window))
            }
            None => (ToneProfile::Default, None),
        }
    }

    pub fn state(&self) -> PipelineState {
        self.state
    }

    pub fn set_whisper_engine(&mut self, engine: Arc<Mutex<Option<WhisperEngine>>>) {
        self.whisper = engine;
    }

    pub fn set_llm_engine(&mut self, engine: Arc<Mutex<Option<LlamaEngine>>>) {
        self.llm = engine;
    }

    pub fn set_llm_ready(&mut self, ready: Arc<AtomicBool>) {
        self.llm_ready = ready;
    }

    pub fn set_auto_edit(&mut self, enabled: bool) {
        self.auto_edit.store(enabled, Ordering::SeqCst);
    }

    pub fn auto_edit_enabled(&self) -> bool {
        self.auto_edit.load(Ordering::SeqCst)
    }

    pub fn set_auto_learn(&mut self, enabled: bool) {
        let was_enabled = self.auto_learn.load(Ordering::SeqCst);
        self.auto_learn.store(enabled, Ordering::SeqCst);
        if was_enabled && !enabled {
            self.observer_generation.fetch_add(1, Ordering::SeqCst);
        }
    }

    pub fn auto_learn_enabled(&self) -> bool {
        self.auto_learn.load(Ordering::SeqCst)
    }

    pub fn set_correction_handler(&mut self, handler: CorrectionHandler) {
        self.correction_handler = Some(handler);
    }

    pub fn is_llm_loaded(&self) -> bool {
        self.llm.lock().is_some()
    }

    pub fn is_model_loaded(&self) -> bool {
        self.whisper.lock().is_some()
    }

    pub fn set_default_stt_language(&mut self, language: SttLanguage) {
        *self.default_stt_language.write() = language;
        if self.state == PipelineState::Idle {
            *self.session_stt_language.write() = language;
            *self.session_detected_language.write() = None;
            *self.pending_detection.lock() = None;
            self.sync_whisper_language();
        }
    }

    pub fn notify_stt_language_changed(&self, app: &AppHandle) {
        emit_stt_language_changed(app, &self.effective_stt_language(), false);
    }

    /// Language shown in the UI and used as the base for mid-dictation cycling.
    pub fn effective_stt_language(&self) -> SttLanguage {
        let session = *self.session_stt_language.read();
        match session {
            SttLanguage::Auto => self.session_detected_language.read().unwrap_or(session),
            fixed => fixed,
        }
    }

    pub fn cycle_session_language(
        &mut self,
        app: &AppHandle,
    ) -> Result<SttLanguage, PipelineError> {
        if self.state != PipelineState::Recording {
            return Err(PipelineError::Busy(self.state));
        }

        let next = self.effective_stt_language().cycle();
        *self.session_stt_language.write() = next;
        *self.session_detected_language.write() = None;
        *self.pending_detection.lock() = None;
        self.sync_whisper_language();
        emit_stt_language_changed(app, &next, false);
        Ok(next)
    }

    fn sync_whisper_language(&self) {
        let language = *self.session_stt_language.read();
        if let Some(engine) = self.whisper.lock().as_mut() {
            engine.set_language(language);
        }
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
        self.failed_segments.lock().clear();
        self.streaming_stt_ms.store(0, Ordering::SeqCst);
        let _ = app.emit("partial-transcript-reset", ());

        *self.session_stt_language.write() = *self.default_stt_language.read();
        *self.session_detected_language.write() = None;
        *self.pending_detection.lock() = None;
        self.sync_whisper_language();
        emit_stt_language_changed(app, &self.effective_stt_language(), false);

        // Fail fast before opening the mic if VAD cannot initialize.
        VadSegmenter::new()?;

        let (chunk_tx, chunk_rx) = std::sync::mpsc::channel::<Vec<f32>>();
        let (level_tx, level_rx) = std::sync::mpsc::channel();
        self.audio
            .start_with_streaming(Some(chunk_tx), Some(level_tx))?;

        let stop_flag = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop_flag);
        let whisper = Arc::clone(&self.whisper);
        let app_worker = app.clone();

        let transcripts = Arc::clone(&self.segment_transcripts);
        let failed_segments = Arc::clone(&self.failed_segments);
        let stt_time_ms = Arc::clone(&self.streaming_stt_ms);
        let dictionary_prompt = Arc::clone(&self.dictionary_prompt);
        let session_stt_language = Arc::clone(&self.session_stt_language);
        let session_detected_language = Arc::clone(&self.session_detected_language);
        let pending_detection = Arc::clone(&self.pending_detection);
        let snippets = Arc::clone(&self.snippets);
        let history_store = self.history_store.clone();
        let worker = thread::spawn(move || {
            streaming_worker(
                app_worker,
                chunk_rx,
                worker_stop,
                StreamingSttContext {
                    whisper,
                    transcripts,
                    failed_segments,
                    stt_time_ms,
                    dictionary_prompt,
                    session_stt_language,
                    session_detected_language,
                    pending_detection,
                    snippets,
                    history_store,
                },
            );
        });

        let app_level = app.clone();
        let level_stop = Arc::clone(&stop_flag);
        let level_listener = thread::spawn(move || {
            while let Ok(level) = level_rx.recv() {
                let _ = app_level.emit("audio-level", AudioLevelEvent { level });
                if level_stop.load(Ordering::SeqCst) {
                    break;
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
        let audio_duration_ms =
            (samples.len() as u64).saturating_mul(1_000) / u64::from(TARGET_SAMPLE_RATE);

        if let Some(session) = self.streaming.take() {
            session.stop_flag.store(true, Ordering::SeqCst);
            if let Some(worker) = session.worker {
                let _ = worker.join();
            }
            if let Some(listener) = session.level_listener {
                let _ = listener.join();
            }
        }

        let streaming_stt_ms = self.streaming_stt_ms.load(Ordering::SeqCst);

        self.set_state(app, PipelineState::Transcribing, None);

        let mut transcripts = self.segment_transcripts.lock().clone();
        let failed_segments = self.failed_segments.lock().drain(..).collect::<Vec<_>>();

        let mut fallback_stt_ms = 0_u64;
        {
            let mut engine_guard = self.whisper.lock();
            let Some(engine) = engine_guard.as_mut() else {
                return Err(PipelineError::ModelNotLoaded);
            };

            let prompt = self.dictionary_prompt.read().clone();
            for FailedSegment { samples, language } in failed_segments {
                let retry_start = Instant::now();
                match engine.transcribe_with_language(&samples, prompt.as_deref(), language) {
                    Ok(result) if !result.text.is_empty() => {
                        fallback_stt_ms += retry_start.elapsed().as_millis() as u64;
                        transcripts.push(result.text);
                    }
                    Ok(_) => {}
                    Err(err) => eprintln!("failed segment retry transcription failed: {err}"),
                }
            }

            // Fallback: if VAD produced no segments, transcribe the full buffer.
            if transcripts.is_empty() && !samples.is_empty() {
                let fallback_start = Instant::now();
                let fallback_language = *self.session_stt_language.read();
                let result = engine.transcribe_with_language(
                    &samples,
                    prompt.as_deref(),
                    fallback_language,
                )?;
                fallback_stt_ms += fallback_start.elapsed().as_millis() as u64;
                if !result.text.is_empty() {
                    transcripts.push(result.text);
                }
            }
        }

        let stt_ms = streaming_stt_ms + fallback_stt_ms;
        let raw = {
            let joined = join_transcript_segments(&transcripts).trim().to_string();
            let rules = self.correction_rules.read().clone();
            apply_corrections(&joined, &rules)
        };
        let stt_wait_ms = stop_instant.elapsed().as_millis() as u64;

        // After STT the user may return to the target app; capture before LLM wait/inject.
        let (active_tone, active_window) = self.resolve_active_context();

        let auto_edit = self.auto_edit.load(Ordering::SeqCst);
        let snippets = self.snippets.read().clone();
        let snippet_ctx = build_snippet_variable_context(
            self.history_store.as_deref(),
        );
        let snippet_fallback =
            prepare_display_transcript(&raw, &snippets, &snippet_ctx);
        let llm_input = if !raw.is_empty() && auto_edit {
            let (shielded_raw, snippet_shields) = shield_snippet_triggers(&raw, &snippets);
            let full_text = post_process_transcript(shielded_raw.trim());
            Some((full_text, snippet_shields))
        } else {
            None
        };

        if !raw.is_empty() && auto_edit && self.llm.lock().is_none() {
            wait_for_llm_engine(&self.llm, LLM_CLEANUP_TIMEOUT);
        }

        let mut llm_blocked_ms = 0_u64;
        let (text_to_inject, llm_ms, llm_invalidated) =
            if let Some((full_text, snippet_shields)) = llm_input {
                if self.llm.lock().is_none() {
                    let text = if snippet_shields.is_empty() {
                        full_text
                    } else {
                        unshield_snippets(&full_text, &snippet_shields, &snippet_ctx)
                    };
                    (text, 0, false)
                } else {
                    let snippets_snapshot = snippets.clone();
                    let job = start_llm_cleanup(
                        Arc::clone(&self.llm),
                        Arc::clone(&self.llm_ready),
                        Arc::clone(&self.auto_edit),
                        &full_text,
                        active_tone,
                    );
                    let llm_wait_start = Instant::now();
                    let LlmCleanupWait::Completed {
                        text,
                        llm_ms,
                        invalidated,
                    } = job.wait_for_inject(LLM_CLEANUP_TIMEOUT);
                    let snippets = self.snippets.read().clone();
                    let text = if self.auto_edit.load(Ordering::SeqCst) {
                        if snippets == snippets_snapshot {
                            finalize_llm_with_snippets(
                                &text,
                                &snippet_shields,
                                &snippet_fallback,
                                &snippets,
                                &snippet_ctx,
                            )
                        } else {
                            let from_cleaned = apply_snippets(&text, &snippets, &snippet_ctx);
                            if from_cleaned != text {
                                from_cleaned
                            } else {
                                snippet_fallback.clone()
                            }
                        }
                    } else {
                        snippet_fallback.clone()
                    };
                    llm_blocked_ms = llm_wait_start.elapsed().as_millis() as u64;
                    (text, llm_ms, invalidated)
                }
            } else if raw.is_empty() {
                (String::new(), 0, false)
            } else {
                (snippet_fallback, 0, false)
            };

        if llm_invalidated && self.auto_edit.load(Ordering::SeqCst) {
            let _ = app.emit("llm-unready", ());
            crate::spawn_llm_recovery_if_needed(app.clone());
        }

        self.set_state(app, PipelineState::Injecting, None);
        let inject_start = Instant::now();
        if !text_to_inject.is_empty() {
            self.injector.inject(&text_to_inject)?;
            self.maybe_spawn_correction_watcher(app, &text_to_inject);
        }
        let inject_ms = inject_start.elapsed().as_millis() as u64;
        let total_ms = stop_instant.elapsed().as_millis() as u64;

        let _ = app.emit(
            "latency-metrics",
            LatencyMetricsEvent {
                stt_ms,
                stt_wait_ms,
                llm_ms,
                llm_blocked_ms,
                inject_ms,
                total_ms,
            },
        );

        if !text_to_inject.is_empty() {
            if let Some(store) = &self.history_store {
                let entry = NewDictation {
                    text: text_to_inject.clone(),
                    audio_duration_ms,
                    stt_ms,
                    llm_ms,
                    inject_ms,
                    total_ms,
                    app_exe: active_window.as_ref().map(|w| w.exe_name.clone()),
                    app_title: active_window.as_ref().map(|w| w.title.clone()),
                };
                if let Err(err) = store.insert_dictation(&entry) {
                    eprintln!("failed to persist dictation history: {err}");
                } else {
                    let _ = app.emit("history-updated", ());
                }
            }
        }

        hide_overlay(app);
        self.set_state(app, PipelineState::Idle, Some(text_to_inject));
        Ok(())
    }

    fn maybe_spawn_correction_watcher(&self, app: &AppHandle, injected_text: &str) {
        if !self.auto_learn.load(Ordering::SeqCst) || !crate::observe::supports_correction_watcher()
        {
            return;
        }

        let Some(handler) = self.correction_handler.clone() else {
            return;
        };

        let watch_generation = self.observer_generation.fetch_add(1, Ordering::SeqCst) + 1;
        crate::observe::spawn_correction_watcher(
            app.clone(),
            injected_text.to_string(),
            Arc::clone(&self.observer_generation),
            watch_generation,
            handler,
        );
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

    fn abort_session(&mut self, app: &AppHandle) {
        match self.audio.stop() {
            Ok(_) | Err(crate::audio::AudioError::NotRecording) => {}
            Err(err) => eprintln!("abort_session audio stop failed: {err}"),
        }

        if let Some(session) = self.streaming.take() {
            session.stop_flag.store(true, Ordering::SeqCst);
            if let Some(worker) = session.worker {
                let _ = worker.join();
            }
            if let Some(listener) = session.level_listener {
                let _ = listener.join();
            }
        }

        self.segment_transcripts.lock().clear();
        self.failed_segments.lock().clear();
        self.streaming_stt_ms.store(0, Ordering::SeqCst);
        let _ = app.emit("partial-transcript-reset", ());
        hide_overlay(app);
    }
}

fn wait_for_llm_engine(llm: &Arc<Mutex<Option<LlamaEngine>>>, timeout: std::time::Duration) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if llm.lock().is_some() {
            return;
        }
        thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// Clears the global LLM slot only if it is still vacant from this cleanup job, or still
/// holds the same sidecar PID. Avoids wiping a newer engine installed while cleanup ran
/// in the background.
fn invalidate_llm_engine_from_cleanup(
    llm: &Arc<Mutex<Option<LlamaEngine>>>,
    llm_ready: &Arc<AtomicBool>,
    cleanup_pid: Option<u32>,
) -> bool {
    let mut guard = llm.lock();
    let should_invalidate = match guard.as_ref() {
        None => true,
        Some(engine) => cleanup_pid == Some(engine.pid()),
    };
    if should_invalidate {
        *guard = None;
        llm_ready.store(false, Ordering::SeqCst);
    }
    should_invalidate
}

fn restore_llm_engine(
    llm: &Arc<Mutex<Option<LlamaEngine>>>,
    auto_edit: &Arc<AtomicBool>,
    engine: LlamaEngine,
) {
    if !auto_edit.load(Ordering::SeqCst) {
        return;
    }
    let mut guard = llm.lock();
    if guard.is_none() {
        *guard = Some(engine);
    }
}

enum LlmCleanupOutcome {
    Success {
        cleaned: String,
        engine: LlamaEngine,
    },
    Failed,
}

struct LlmCleanupJob {
    rx: std::sync::mpsc::Receiver<LlmCleanupOutcome>,
    worker: thread::JoinHandle<()>,
    llm: Arc<Mutex<Option<LlamaEngine>>>,
    llm_ready: Arc<AtomicBool>,
    auto_edit: Arc<AtomicBool>,
    raw: String,
    worker_pid: Option<u32>,
    start: Instant,
}

enum LlmCleanupWait {
    Completed {
        text: String,
        llm_ms: u64,
        invalidated: bool,
    },
}

impl LlmCleanupJob {
    fn wait_for_inject(self, timeout: std::time::Duration) -> LlmCleanupWait {
        match self.rx.recv_timeout(timeout) {
            Ok(LlmCleanupOutcome::Success { cleaned, engine }) => {
                restore_llm_engine(&self.llm, &self.auto_edit, engine);
                let _ = self.worker.join();
                let text = if self.auto_edit.load(Ordering::SeqCst) {
                    cleaned
                } else {
                    self.raw
                };
                LlmCleanupWait::Completed {
                    text,
                    llm_ms: self.start.elapsed().as_millis() as u64,
                    invalidated: false,
                }
            }
            Ok(LlmCleanupOutcome::Failed) => {
                let _ = self.worker.join();
                let invalidated =
                    invalidate_llm_engine_from_cleanup(&self.llm, &self.llm_ready, self.worker_pid);
                LlmCleanupWait::Completed {
                    text: self.raw,
                    llm_ms: self.start.elapsed().as_millis() as u64,
                    invalidated,
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                eprintln!(
                    "llm cleanup timed out after {:?}, using post-processed transcript",
                    timeout
                );
                if let Some(pid) = self.worker_pid {
                    force_kill_sidecar(pid);
                }
                let _ = self.worker.join();
                let invalidated =
                    invalidate_llm_engine_from_cleanup(&self.llm, &self.llm_ready, self.worker_pid);
                LlmCleanupWait::Completed {
                    text: self.raw,
                    llm_ms: self.start.elapsed().as_millis() as u64,
                    invalidated,
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                let _ = self.worker.join();
                let invalidated =
                    invalidate_llm_engine_from_cleanup(&self.llm, &self.llm_ready, self.worker_pid);
                LlmCleanupWait::Completed {
                    text: self.raw,
                    llm_ms: self.start.elapsed().as_millis() as u64,
                    invalidated,
                }
            }
        }
    }
}

fn start_llm_cleanup(
    llm: Arc<Mutex<Option<LlamaEngine>>>,
    llm_ready: Arc<AtomicBool>,
    auto_edit: Arc<AtomicBool>,
    raw: &str,
    tone: ToneProfile,
) -> LlmCleanupJob {
    let raw = raw.to_string();
    let start = Instant::now();
    let (tx, rx) = std::sync::mpsc::channel();
    let raw_for_worker = raw.clone();
    let llm_for_worker = Arc::clone(&llm);
    let worker_pid = llm.lock().as_ref().map(LlamaEngine::pid);

    let worker = thread::spawn(move || {
        let engine = llm_for_worker.lock().take();
        let outcome = match engine {
            Some(mut engine) => match engine.cleanup(&raw_for_worker, tone) {
                Ok(cleaned) => LlmCleanupOutcome::Success { cleaned, engine },
                Err(err) => {
                    eprintln!("llm cleanup failed, using raw transcript: {err}");
                    LlmCleanupOutcome::Failed
                }
            },
            None => {
                eprintln!("llm cleanup failed: llm engine not loaded");
                LlmCleanupOutcome::Failed
            }
        };
        let _ = tx.send(outcome);
    });

    LlmCleanupJob {
        rx,
        worker,
        llm,
        llm_ready,
        auto_edit,
        raw,
        worker_pid,
        start,
    }
}

fn force_kill_sidecar(pid: u32) {
    #[cfg(windows)]
    {
        let mut command = std::process::Command::new("taskkill");
        crate::process_util::hide_console(&mut command);
        let _ = command
            .args(["/PID", &pid.to_string(), "/F", "/T"])
            .output();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output();
    }
}

struct StreamingSttContext {
    whisper: Arc<Mutex<Option<WhisperEngine>>>,
    transcripts: Arc<Mutex<Vec<String>>>,
    failed_segments: Arc<Mutex<Vec<FailedSegment>>>,
    stt_time_ms: Arc<AtomicU64>,
    dictionary_prompt: Arc<RwLock<Option<Arc<str>>>>,
    session_stt_language: Arc<RwLock<SttLanguage>>,
    session_detected_language: Arc<RwLock<Option<SttLanguage>>>,
    pending_detection: Arc<Mutex<Option<SttLanguage>>>,
    snippets: Arc<RwLock<Vec<Snippet>>>,
    history_store: Option<Arc<Store>>,
}

fn streaming_worker(
    app: AppHandle,
    chunk_rx: std::sync::mpsc::Receiver<Vec<f32>>,
    stop_flag: Arc<AtomicBool>,
    stt: StreamingSttContext,
) {
    let Ok(mut vad) = VadSegmenter::new() else {
        eprintln!("streaming_worker: VAD init failed after preflight check");
        return;
    };

    let mut segment_index = 0_u32;

    while let Ok(chunk) = chunk_rx.recv() {
        let segments = match vad.push(&chunk) {
            Ok(segments) => segments,
            Err(err) => {
                eprintln!("VAD error: {err}");
                continue;
            }
        };
        for segment in segments {
            transcribe_segment(&app, &stt, segment, segment_index);
            segment_index += 1;
        }
        if stop_flag.load(Ordering::SeqCst) {
            while let Ok(chunk) = chunk_rx.try_recv() {
                let segments = match vad.push(&chunk) {
                    Ok(segments) => segments,
                    Err(err) => {
                        eprintln!("VAD error: {err}");
                        continue;
                    }
                };
                for segment in segments {
                    transcribe_segment(&app, &stt, segment, segment_index);
                    segment_index += 1;
                }
            }
            break;
        }
    }

    if let Ok(segments) = vad.flush() {
        for segment in segments {
            transcribe_segment(&app, &stt, segment, segment_index);
            segment_index += 1;
        }
    }
}

fn transcribe_segment(
    app: &AppHandle,
    stt: &StreamingSttContext,
    segment: Vec<f32>,
    segment_index: u32,
) {
    let stt_start = Instant::now();
    let prompt = stt.dictionary_prompt.read().clone();
    let segment_language = *stt.session_stt_language.read();
    let text = {
        let mut engine_guard = stt.whisper.lock();
        let Some(engine) = engine_guard.as_mut() else {
            stt.failed_segments.lock().push(FailedSegment {
                samples: segment,
                language: segment_language,
            });
            return;
        };
        match engine.transcribe_with_language(&segment, prompt.as_deref(), segment_language) {
            Ok(result) => result,
            Err(err) => {
                eprintln!("segment transcription failed: {err}");
                stt.failed_segments.lock().push(FailedSegment {
                    samples: segment,
                    language: segment_language,
                });
                return;
            }
        }
    };
    stt.stt_time_ms
        .fetch_add(stt_start.elapsed().as_millis() as u64, Ordering::SeqCst);

    if let Some(detected) = text.detected_language.as_deref() {
        maybe_commit_auto_detection(app, stt, detected);
    }

    if text.text.is_empty() {
        return;
    }

    let raw = {
        let mut transcripts = stt.transcripts.lock();
        transcripts.push(text.text.clone());
        join_transcript_segments(transcripts.as_slice())
    };
    let snippet_ctx = build_snippet_variable_context(stt.history_store.as_deref());
    let display = prepare_partial_transcript(&raw, &stt.snippets.read(), &snippet_ctx);

    let _ = app.emit(
        "partial-transcript",
        PartialTranscriptEvent {
            text: display,
            segment_index,
        },
    );
}

fn maybe_commit_auto_detection(app: &AppHandle, stt: &StreamingSttContext, detected_code: &str) {
    if !matches!(*stt.session_stt_language.read(), SttLanguage::Auto) {
        return;
    }

    let Some(detected) = SttLanguage::parse(detected_code) else {
        return;
    };
    if matches!(detected, SttLanguage::Auto) {
        return;
    }

    if *stt.session_detected_language.read() == Some(detected) {
        return;
    }

    let mut pending = stt.pending_detection.lock();
    if *pending == Some(detected) {
        *pending = None;
        drop(pending);
        *stt.session_detected_language.write() = Some(detected);
        emit_stt_language_changed(app, &detected, true);
    } else {
        *pending = Some(detected);
    }
}

fn emit_stt_language_changed(app: &AppHandle, language: &SttLanguage, detected: bool) {
    let _ = app.emit(
        "stt-language-changed",
        SttLanguageChangedEvent {
            language: language.as_setting_value(),
            detected,
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
            width: 184,
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
    // Benign race: duplicate start/stop while state already transitioned.
    if matches!(err, PipelineError::Busy(_)) {
        return;
    }

    let _ = app.emit(
        "pipeline-state",
        PipelineStateEvent {
            state: "error".into(),
            message: Some(err.to_string()),
        },
    );
    let mut guard = orchestrator.lock();
    if guard.state() != PipelineState::Idle {
        guard.abort_session(app);
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

    #[test]
    fn prepare_display_transcript_preserves_snippet_body() {
        let snippets = vec![Snippet {
            id: 1,
            trigger: "mon email".into(),
            content: "contact at gmail point com".into(),
            created_at: "now".into(),
        }];
        let result = prepare_display_transcript("voici mon email", &snippets, &SnippetVariableContext::default());
        assert_eq!(result, "Voici contact at gmail point com");
    }
}
