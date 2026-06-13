use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Instant;

use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};
use thiserror::Error;

use calliop_prompt::{
    find_latest_frozen_boundary, fits_llm_cleanup_budget, join_transcript_segments_with_pauses,
    post_process_transcript, ToneProfile,
};

use crate::app_context::{get_active_window, resolve_tone, ActiveWindow};
use crate::audio::{AudioCapture, SpeechSegment, VadSegmenter, TARGET_SAMPLE_RATE};
use crate::inject::{InjectError, TextInjector};
use crate::llm::LlamaEngine;
use crate::observe::CorrectionHandler;
use crate::store::{AppContextRule, NewDictation, Snippet, Store};
use crate::stt::{SttError, SttLanguage, WhisperEngine};

use super::corrections::{apply_corrections, CorrectionRule};
use super::snippet_variables::SnippetVariableContext;
use super::snippets::{
    apply_snippets, finalize_llm_with_snippets, shield_snippet_triggers, unshield_snippets,
    ShieldedSnippet,
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

/// Live overlay: deterministic post-processing + snippet triggers (no oral punctuation expansion).
fn prepare_partial_transcript(
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

/// Maximum time to wait for LLM cleanup before inject or worker kill.
const LLM_CLEANUP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(45);
const LLM_CLEANUP_TIMEOUT_MAX: std::time::Duration = std::time::Duration::from_secs(120);

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

    /// Hotkey release during this state can cancel the in-flight dictation.
    pub fn hotkey_cancelable(self) -> bool {
        matches!(self, Self::Transcribing)
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
pub struct SttSegmentProgressEvent {
    pub completed: u32,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LlmStatus {
    Applied,
    Skipped,
    Failed,
    Disabled,
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
    #[serde(rename = "llmStatus")]
    pub llm_status: LlmStatus,
    #[serde(rename = "llmSkipReason", skip_serializing_if = "Option::is_none")]
    pub llm_skip_reason: Option<String>,
    #[serde(rename = "recordStartMs", skip_serializing_if = "Option::is_none")]
    pub record_start_ms: Option<u64>,
    #[serde(rename = "micOpenMs", skip_serializing_if = "Option::is_none")]
    pub mic_open_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordStartMetricsEvent {
    #[serde(rename = "recordStartMs")]
    pub record_start_ms: u64,
    #[serde(rename = "micOpenMs")]
    pub mic_open_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DictationBusyEvent {
    pub state: String,
    pub cancelable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioLevelEvent {
    pub level: f32,
    pub bands: Vec<f32>,
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

struct PendingSegment {
    segment: SpeechSegment,
    index: u32,
}

struct StreamingSession {
    stop_flag: Arc<AtomicBool>,
    vad_worker: Option<JoinHandle<()>>,
    stt_worker: Option<JoinHandle<()>>,
    level_listener: Option<JoinHandle<()>>,
}

struct FailedSegment {
    samples: Vec<f32>,
    language: SttLanguage,
}

type SegmentTranscript = (String, u32);

struct FrozenLlmCoordinator {
    generation: u64,
    frozen_up_to_index: Option<usize>,
    prefix_shields: Vec<ShieldedSnippet>,
    cleaned_prefix: Option<String>,
    job: Option<LlmCleanupJob>,
    pending_boundary: Option<usize>,
}

impl FrozenLlmCoordinator {
    fn new() -> Self {
        Self {
            generation: 0,
            frozen_up_to_index: None,
            prefix_shields: Vec::new(),
            cleaned_prefix: None,
            job: None,
            pending_boundary: None,
        }
    }

    fn reset(&mut self) {
        if let Some(job) = self.job.take() {
            if job.is_running() {
                if let Some(pid) = job.worker_pid {
                    force_kill_sidecar(pid);
                }
            }
            let _ = job.join_worker();
        }
        self.generation = 0;
        self.frozen_up_to_index = None;
        self.prefix_shields.clear();
        self.cleaned_prefix = None;
        self.pending_boundary = None;
    }

    fn poll_completed_job(&mut self) {
        let Some(job) = self.job.take() else {
            return;
        };
        if job.is_running() {
            self.job = Some(job);
            return;
        }
        let LlmCleanupWait::Completed {
            text, llm_status, ..
        } = job.wait_for_inject(std::time::Duration::from_millis(0));
        if matches!(llm_status, LlmStatus::Applied) {
            self.cleaned_prefix = Some(text);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn maybe_start_background(
        &mut self,
        segments: &[SegmentTranscript],
        auto_edit: bool,
        llm: &Arc<Mutex<Option<LlamaEngine>>>,
        llm_ready: &Arc<AtomicBool>,
        auto_edit_flag: &Arc<AtomicBool>,
        rules: &[CorrectionRule],
        snippets: &[Snippet],
        tone: ToneProfile,
    ) {
        if !auto_edit || llm.lock().is_none() {
            return;
        }
        let Some(boundary) = find_latest_frozen_boundary(segments) else {
            return;
        };
        if self.frozen_up_to_index == Some(boundary) && self.job.is_some() {
            return;
        }

        self.poll_completed_job();

        if let Some(job) = self.job.as_ref() {
            if job.is_running() {
                if self.frozen_up_to_index != Some(boundary) {
                    self.pending_boundary = Some(boundary);
                }
                return;
            }
        }

        if self.job.is_some() {
            self.poll_completed_job();
        }

        let target_boundary = self.pending_boundary.take().unwrap_or(boundary);
        if self.frozen_up_to_index == Some(target_boundary) && self.job.is_some() {
            return;
        }

        let prefix_segments = &segments[..=target_boundary];
        let Some((full_text, shields)) = build_llm_ready_text(prefix_segments, rules, snippets)
        else {
            return;
        };
        if !fits_llm_cleanup_budget(&full_text) {
            return;
        }

        self.generation = self.generation.saturating_add(1);
        self.frozen_up_to_index = Some(target_boundary);
        self.prefix_shields = shields;
        self.cleaned_prefix = None;
        let job = start_llm_cleanup(
            Arc::clone(llm),
            Arc::clone(llm_ready),
            Arc::clone(auto_edit_flag),
            &full_text,
            tone,
        );
        self.job = Some(job);
    }

    /// Waits for an in-flight pipelined cleanup job and returns its prefix snapshot when successful.
    /// Also returns a cached prefix when the background job already completed during recording.
    fn finish_inflight_job(&mut self, timeout: std::time::Duration) -> Option<FrozenLlmSnapshot> {
        if let Some(job) = self.job.take() {
            let frozen_up_to_index = self.frozen_up_to_index?;
            let LlmCleanupWait::Completed {
                text, llm_status, ..
            } = job.wait_for_inject(timeout);
            if !matches!(llm_status, LlmStatus::Applied) {
                return None;
            }
            self.cleaned_prefix = Some(text.clone());
            return Some(FrozenLlmSnapshot {
                frozen_up_to_index,
                cleaned_prefix: text,
                prefix_shields: self.prefix_shields.clone(),
            });
        }

        let frozen_up_to_index = self.frozen_up_to_index?;
        let cleaned_prefix = self.cleaned_prefix.clone()?;
        Some(FrozenLlmSnapshot {
            frozen_up_to_index,
            cleaned_prefix,
            prefix_shields: self.prefix_shields.clone(),
        })
    }
}

struct FrozenLlmSnapshot {
    frozen_up_to_index: usize,
    cleaned_prefix: String,
    #[allow(dead_code)]
    prefix_shields: Vec<ShieldedSnippet>,
}

fn build_llm_ready_text(
    segments: &[SegmentTranscript],
    rules: &[CorrectionRule],
    snippets: &[Snippet],
) -> Option<(String, Vec<ShieldedSnippet>)> {
    if segments.is_empty() {
        return None;
    }
    let joined = join_transcript_segments_with_pauses(segments)
        .trim()
        .to_string();
    if joined.is_empty() {
        return None;
    }
    let corrected = apply_corrections(&joined, rules);
    let (shielded, shields) = shield_snippet_triggers(&corrected, snippets);
    let full_text = post_process_transcript(shielded.trim());
    if full_text.is_empty() {
        return None;
    }
    Some((full_text, shields))
}

fn merge_cleaned_segments(prefix: &str, tail: &str) -> String {
    let prefix = prefix.trim_end();
    let tail = tail.trim_start();
    if prefix.is_empty() {
        return tail.to_string();
    }
    if tail.is_empty() {
        return prefix.to_string();
    }
    if prefix.ends_with('\n') {
        format!("{prefix}{tail}")
    } else {
        format!("{prefix} {tail}")
    }
}

fn llm_cleanup_timeout(text: &str) -> std::time::Duration {
    let extra_secs = (text.len() as u64 / 100).min(
        LLM_CLEANUP_TIMEOUT_MAX
            .as_secs()
            .saturating_sub(LLM_CLEANUP_TIMEOUT.as_secs()),
    );
    LLM_CLEANUP_TIMEOUT + std::time::Duration::from_secs(extra_secs)
}

fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?' | '…') || ch == '\n' {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                sentences.push(trimmed);
            }
            current.clear();
        }
    }
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        sentences.push(trimmed);
    }
    if sentences.is_empty() && !text.trim().is_empty() {
        sentences.push(text.trim().to_string());
    }
    sentences
}

fn split_by_char_budget(text: &str) -> Vec<String> {
    let max_chars = calliop_prompt::LLM_CLEANUP_INPUT_TOKEN_BUDGET * 3;
    let mut parts = Vec::new();
    let mut start = 0;
    let chars: Vec<char> = text.chars().collect();
    while start < chars.len() {
        let end = (start + max_chars).min(chars.len());
        let mut split_at = end;
        if end < chars.len() {
            if let Some(rel) = chars[start..end].iter().rposition(|ch| ch.is_whitespace()) {
                split_at = start + rel;
            }
        }
        if split_at <= start {
            split_at = end;
        }
        parts.push(
            chars[start..split_at]
                .iter()
                .collect::<String>()
                .trim()
                .to_string(),
        );
        start = split_at;
        while start < chars.len() && chars[start].is_whitespace() {
            start += 1;
        }
    }
    parts.retain(|part| !part.is_empty());
    parts
}

fn split_oversized_llm_text(
    text: String,
    shields: Vec<ShieldedSnippet>,
) -> Vec<(String, Vec<ShieldedSnippet>)> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for sentence in split_sentences(&text) {
        let candidate = if current.is_empty() {
            sentence.clone()
        } else {
            format!("{} {}", current.trim_end(), sentence.trim_start())
        };
        if fits_llm_cleanup_budget(&candidate) {
            current = candidate;
        } else if !current.is_empty() {
            chunks.push((current.clone(), shields.clone()));
            if fits_llm_cleanup_budget(&sentence) {
                current = sentence;
            } else {
                for part in split_by_char_budget(&sentence) {
                    chunks.push((part, shields.clone()));
                }
                current.clear();
            }
        } else if fits_llm_cleanup_budget(&sentence) {
            current = sentence;
        } else {
            for part in split_by_char_budget(&sentence) {
                chunks.push((part, shields.clone()));
            }
            current.clear();
        }
    }
    if !current.is_empty() {
        chunks.push((current, shields));
    }
    chunks
}

fn plan_llm_chunks_from_segments(
    segments: &[SegmentTranscript],
    rules: &[CorrectionRule],
    snippets: &[Snippet],
) -> Vec<(String, Vec<ShieldedSnippet>)> {
    if segments.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut start = 0usize;
    while start < segments.len() {
        let mut best_end = start;
        for end in start..segments.len() {
            let Some((text, _)) = build_llm_ready_text(&segments[start..=end], rules, snippets)
            else {
                continue;
            };
            if fits_llm_cleanup_budget(&text) {
                best_end = end;
            } else {
                break;
            }
        }

        if best_end >= start {
            if let Some(chunk) = build_llm_ready_text(&segments[start..=best_end], rules, snippets)
            {
                chunks.push(chunk);
            }
            start = best_end + 1;
            continue;
        }

        if let Some((text, shields)) =
            build_llm_ready_text(&segments[start..=start], rules, snippets)
        {
            chunks.extend(split_oversized_llm_text(text, shields));
        }
        start += 1;
    }
    chunks
}

struct LlmCleanupAggregate {
    text: String,
    llm_ms: u64,
    llm_status: LlmStatus,
    llm_skip_reason: Option<String>,
    invalidated: bool,
}

#[allow(clippy::too_many_arguments)]
fn run_llm_cleanup_for_segments(
    llm: Arc<Mutex<Option<LlamaEngine>>>,
    llm_ready: Arc<AtomicBool>,
    auto_edit: Arc<AtomicBool>,
    segments: &[SegmentTranscript],
    rules: &[CorrectionRule],
    snippets: &[Snippet],
    tone: ToneProfile,
    cancel_requested: &Arc<AtomicBool>,
) -> LlmCleanupAggregate {
    let chunks = plan_llm_chunks_from_segments(segments, rules, snippets);
    if chunks.is_empty() {
        return LlmCleanupAggregate {
            text: String::new(),
            llm_ms: 0,
            llm_status: LlmStatus::Disabled,
            llm_skip_reason: None,
            invalidated: false,
        };
    }

    let mut merged = String::new();
    let mut llm_ms = 0_u64;
    let mut llm_status = LlmStatus::Applied;
    let mut llm_skip_reason = None;
    let mut invalidated = false;

    for (chunk_text, _shields) in chunks {
        if cancel_requested.load(Ordering::SeqCst) {
            break;
        }
        if llm.lock().is_none() {
            llm_status = LlmStatus::Skipped;
            llm_skip_reason = Some("not_loaded".into());
            break;
        }

        let timeout = llm_cleanup_timeout(&chunk_text);
        let job = start_llm_cleanup(
            Arc::clone(&llm),
            Arc::clone(&llm_ready),
            Arc::clone(&auto_edit),
            &chunk_text,
            tone,
        );
        let LlmCleanupWait::Completed {
            text,
            llm_ms: chunk_ms,
            invalidated: chunk_inv,
            llm_status: chunk_status,
            llm_skip_reason: chunk_reason,
        } = job.wait_for_inject(timeout);

        llm_ms = llm_ms.saturating_add(chunk_ms);
        invalidated |= chunk_inv;

        if matches!(chunk_status, LlmStatus::Applied) {
            merged = merge_cleaned_segments(&merged, &text);
        } else {
            llm_status = chunk_status;
            llm_skip_reason = chunk_reason;
            merged = merge_cleaned_segments(&merged, &chunk_text);
        }
    }

    if merged.is_empty() {
        if let Some((fallback, _)) = build_llm_ready_text(segments, rules, snippets) {
            merged = fallback;
        }
    }

    LlmCleanupAggregate {
        text: merged,
        llm_ms,
        llm_status,
        llm_skip_reason,
        invalidated,
    }
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
    segment_transcripts: Arc<Mutex<Vec<SegmentTranscript>>>,
    failed_segments: Arc<Mutex<Vec<FailedSegment>>>,
    frozen_llm: Arc<Mutex<FrozenLlmCoordinator>>,
    streaming_stt_ms: Arc<AtomicU64>,
    dictionary_prompt: Arc<RwLock<Option<Arc<str>>>>,
    snippets: Arc<RwLock<Vec<Snippet>>>,
    correction_rules: Arc<RwLock<Vec<CorrectionRule>>>,
    app_context_rules: Arc<RwLock<Vec<AppContextRule>>>,
    session_active_window: Arc<RwLock<Option<ActiveWindow>>>,
    session_tone: Arc<RwLock<ToneProfile>>,
    cancel_requested: Arc<AtomicBool>,
    auto_learn: Arc<AtomicBool>,
    observer_generation: Arc<AtomicU64>,
    correction_handler: Option<CorrectionHandler>,
    default_stt_language: Arc<RwLock<SttLanguage>>,
    session_stt_language: Arc<RwLock<SttLanguage>>,
    session_detected_language: Arc<RwLock<Option<SttLanguage>>>,
    pending_detection: Arc<Mutex<Option<SttLanguage>>>,
    history_store: Option<Arc<Store>>,
    vad_chunk_size: usize,
    input_device: String,
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
            frozen_llm: Arc::new(Mutex::new(FrozenLlmCoordinator::new())),
            streaming_stt_ms: Arc::new(AtomicU64::new(0)),
            dictionary_prompt: Arc::new(RwLock::new(None)),
            snippets: Arc::new(RwLock::new(Vec::new())),
            correction_rules: Arc::new(RwLock::new(Vec::new())),
            app_context_rules: Arc::new(RwLock::new(Vec::new())),
            session_active_window: Arc::new(RwLock::new(None)),
            session_tone: Arc::new(RwLock::new(ToneProfile::Default)),
            cancel_requested: Arc::new(AtomicBool::new(false)),
            auto_learn: Arc::new(AtomicBool::new(true)),
            observer_generation: Arc::new(AtomicU64::new(0)),
            correction_handler: None,
            default_stt_language: Arc::new(RwLock::new(SttLanguage::default_fixed())),
            session_stt_language: Arc::new(RwLock::new(SttLanguage::default_fixed())),
            session_detected_language: Arc::new(RwLock::new(None)),
            pending_detection: Arc::new(Mutex::new(None)),
            history_store: None,
            vad_chunk_size: crate::system::DEFAULT_VAD_CHUNK_SIZE,
            input_device: crate::audio::DEFAULT_INPUT_DEVICE_ID.into(),
        })
    }

    pub fn set_input_device(&mut self, device_id: String) {
        self.input_device = device_id;
    }

    pub fn set_vad_chunk_size(&mut self, chunk_size: usize) {
        self.vad_chunk_size = chunk_size;
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
        if let Some(window) = self.session_active_window.read().clone() {
            let tone = *self.session_tone.read();
            return (tone, Some(window));
        }
        let rules = self.app_context_rules.read().clone();
        match get_active_window() {
            Some(window) => {
                let tone = resolve_tone(&window, &rules);
                (tone, Some(window))
            }
            None => (ToneProfile::Default, None),
        }
    }

    pub fn request_cancel(&self) {
        self.cancel_requested.store(true, Ordering::SeqCst);
    }

    pub fn cancel_processing(&mut self, app: &AppHandle) -> bool {
        match self.state {
            PipelineState::Recording => {
                self.request_cancel();
                self.abort_session(app);
                self.finish_cancelled(app);
                true
            }
            PipelineState::Transcribing => {
                self.request_cancel();
                let _ = app.emit(
                    "dictation-cancelled",
                    serde_json::json!({ "during": "transcribing" }),
                );
                true
            }
            PipelineState::Injecting | PipelineState::Idle => false,
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancel_requested.load(Ordering::SeqCst)
    }

    fn finish_cancelled(&mut self, app: &AppHandle) {
        self.frozen_llm.lock().reset();
        self.cancel_requested.store(false, Ordering::SeqCst);
        *self.session_active_window.write() = None;
        hide_overlay(app);
        self.set_state(app, PipelineState::Idle, None);
    }

    fn check_cancelled(&mut self, app: &AppHandle) -> bool {
        if !self.is_cancelled() {
            return false;
        }
        self.finish_cancelled(app);
        true
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

        let record_start = Instant::now();
        self.cancel_requested.store(false, Ordering::SeqCst);
        self.segment_transcripts.lock().clear();
        self.failed_segments.lock().clear();
        self.frozen_llm.lock().reset();
        self.streaming_stt_ms.store(0, Ordering::SeqCst);
        let _ = app.emit("partial-transcript-reset", ());

        let rules = self.app_context_rules.read().clone();
        let active_window = get_active_window();
        let session_tone = active_window
            .as_ref()
            .map(|window| resolve_tone(window, &rules))
            .unwrap_or(ToneProfile::Default);
        *self.session_active_window.write() = active_window;
        *self.session_tone.write() = session_tone;

        *self.session_stt_language.write() = *self.default_stt_language.read();
        *self.session_detected_language.write() = None;
        *self.pending_detection.lock() = None;
        self.sync_whisper_language();
        emit_stt_language_changed(app, &self.effective_stt_language(), false);

        let vad = VadSegmenter::with_chunk_size(self.vad_chunk_size)?;

        let (chunk_tx, chunk_rx) = std::sync::mpsc::channel::<Vec<f32>>();
        let (level_tx, level_rx) = std::sync::mpsc::channel();
        self.audio.start_with_streaming(
            Some(chunk_tx),
            Some(level_tx),
            Some(self.input_device.as_str()),
        )?;
        let mic_open_ms = record_start.elapsed().as_millis() as u64;

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
        let frozen_llm = Arc::clone(&self.frozen_llm);
        let llm = Arc::clone(&self.llm);
        let llm_ready = Arc::clone(&self.llm_ready);
        let auto_edit = Arc::clone(&self.auto_edit);
        let correction_rules = Arc::clone(&self.correction_rules);
        let session_tone_arc = Arc::clone(&self.session_tone);
        let (segment_tx, segment_rx) = std::sync::mpsc::channel::<PendingSegment>();

        let stt_context = StreamingSttContext {
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
            frozen_llm,
            llm,
            llm_ready,
            auto_edit,
            correction_rules,
            session_tone: session_tone_arc,
        };

        let stt_stop = Arc::clone(&stop_flag);
        let stt_worker = thread::spawn(move || {
            stt_segment_worker(app_worker, segment_rx, stt_stop, stt_context);
        });

        let vad_worker = thread::spawn(move || {
            streaming_worker(chunk_rx, worker_stop, vad, segment_tx);
        });

        let app_level = app.clone();
        let level_stop = Arc::clone(&stop_flag);
        let level_listener = thread::spawn(move || {
            while let Ok(sample) = level_rx.recv() {
                let _ = app_level.emit(
                    "audio-level",
                    AudioLevelEvent {
                        level: sample.level,
                        bands: sample.bands.to_vec(),
                    },
                );
                if level_stop.load(Ordering::SeqCst) {
                    break;
                }
            }
        });

        self.streaming = Some(StreamingSession {
            stop_flag,
            vad_worker: Some(vad_worker),
            stt_worker: Some(stt_worker),
            level_listener: Some(level_listener),
        });

        show_overlay(app);
        self.set_state(app, PipelineState::Recording, None);

        let record_start_ms = record_start.elapsed().as_millis() as u64;
        let _ = app.emit(
            "record-start-metrics",
            RecordStartMetricsEvent {
                record_start_ms,
                mic_open_ms,
            },
        );

        Ok(())
    }

    fn finish_dictation(&mut self, app: &AppHandle) -> Result<(), PipelineError> {
        let stop_instant = Instant::now();

        let samples = self.audio.stop()?;
        let audio_duration_ms =
            (samples.len() as u64).saturating_mul(1_000) / u64::from(TARGET_SAMPLE_RATE);

        self.set_state(app, PipelineState::Transcribing, None);

        if let Some(session) = self.streaming.take() {
            session.stop_flag.store(true, Ordering::SeqCst);
            if let Some(worker) = session.vad_worker {
                let _ = worker.join();
            }
            if let Some(worker) = session.stt_worker {
                let _ = worker.join();
            }
            if let Some(listener) = session.level_listener {
                let _ = listener.join();
            }
        }

        if self.check_cancelled(app) {
            return Ok(());
        }

        let streaming_stt_ms = self.streaming_stt_ms.load(Ordering::SeqCst);

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
                        transcripts.push((result.text, 0));
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
                    transcripts.push((result.text, 0));
                }
            }
        }

        let stt_ms = streaming_stt_ms + fallback_stt_ms;
        let rules = self.correction_rules.read().clone();
        let raw = {
            let joined = join_transcript_segments_with_pauses(&transcripts)
                .trim()
                .to_string();
            apply_corrections(&joined, &rules)
        };
        let stt_wait_ms = stop_instant.elapsed().as_millis() as u64;

        if self.check_cancelled(app) {
            return Ok(());
        }

        if stt_wait_ms > audio_duration_ms.saturating_mul(2).max(500) {
            eprintln!(
                "stt_wait_ms ({stt_wait_ms}) exceeds 2x audio duration ({audio_duration_ms}) — check segment backlog"
            );
        }

        // Tone and target app captured at recording start (before LLM wait/inject).
        let (active_tone, active_window) = self.resolve_active_context();

        let auto_edit = self.auto_edit.load(Ordering::SeqCst);
        let snippets = self.snippets.read().clone();
        let snippet_ctx = build_snippet_variable_context(self.history_store.as_deref());
        let snippet_fallback = prepare_display_transcript(&raw, &snippets, &snippet_ctx);
        let llm_input = if !raw.is_empty() && auto_edit {
            build_llm_ready_text(&transcripts, &rules, &snippets)
        } else {
            None
        };

        let pipelined = if auto_edit && !raw.is_empty() {
            self.frozen_llm
                .lock()
                .finish_inflight_job(LLM_CLEANUP_TIMEOUT_MAX)
        } else {
            None
        };

        if !raw.is_empty() && auto_edit && self.llm.lock().is_none() && pipelined.is_none() {
            wait_for_llm_engine(&self.llm, LLM_CLEANUP_TIMEOUT, &self.cancel_requested);
        }

        if self.check_cancelled(app) {
            return Ok(());
        }

        let mut llm_blocked_ms = 0_u64;
        let (text_to_inject, llm_ms, llm_invalidated, llm_status, llm_skip_reason) =
            if let Some((full_text, snippet_shields)) = llm_input {
                if self.llm.lock().is_none() {
                    let text = if snippet_shields.is_empty() {
                        full_text
                    } else {
                        unshield_snippets(&full_text, &snippet_shields, &snippet_ctx)
                    };
                    (
                        text,
                        0,
                        false,
                        LlmStatus::Skipped,
                        Some("not_loaded".into()),
                    )
                } else {
                    let snippets_snapshot = snippets.clone();
                    let llm_wait_start = Instant::now();

                    let (merged_cleaned, llm_ms, llm_status, llm_skip_reason, llm_invalidated) =
                        if let Some(frozen) = pipelined {
                            let tail_start = frozen.frozen_up_to_index.saturating_add(1);
                            if tail_start >= transcripts.len() {
                                (
                                    frozen.cleaned_prefix,
                                    0_u64,
                                    LlmStatus::Applied,
                                    None,
                                    false,
                                )
                            } else {
                                let tail_result = run_llm_cleanup_for_segments(
                                    Arc::clone(&self.llm),
                                    Arc::clone(&self.llm_ready),
                                    Arc::clone(&self.auto_edit),
                                    &transcripts[tail_start..],
                                    &rules,
                                    &snippets,
                                    active_tone,
                                    &self.cancel_requested,
                                );
                                let merged = if matches!(tail_result.llm_status, LlmStatus::Applied)
                                {
                                    merge_cleaned_segments(
                                        &frozen.cleaned_prefix,
                                        &tail_result.text,
                                    )
                                } else {
                                    full_text
                                };
                                (
                                    merged,
                                    tail_result.llm_ms,
                                    tail_result.llm_status,
                                    tail_result.llm_skip_reason,
                                    tail_result.invalidated,
                                )
                            }
                        } else {
                            let result = run_llm_cleanup_for_segments(
                                Arc::clone(&self.llm),
                                Arc::clone(&self.llm_ready),
                                Arc::clone(&self.auto_edit),
                                &transcripts,
                                &rules,
                                &snippets,
                                active_tone,
                                &self.cancel_requested,
                            );
                            (
                                result.text,
                                result.llm_ms,
                                result.llm_status,
                                result.llm_skip_reason,
                                result.invalidated,
                            )
                        };

                    let snippets = self.snippets.read().clone();
                    let text = if self.auto_edit.load(Ordering::SeqCst) {
                        if snippets == snippets_snapshot {
                            finalize_llm_with_snippets(
                                &merged_cleaned,
                                &snippet_shields,
                                &snippet_fallback,
                                &snippets,
                                &snippet_ctx,
                            )
                        } else {
                            let from_cleaned =
                                apply_snippets(&merged_cleaned, &snippets, &snippet_ctx);
                            if from_cleaned != merged_cleaned {
                                from_cleaned
                            } else {
                                snippet_fallback.clone()
                            }
                        }
                    } else {
                        snippet_fallback.clone()
                    };
                    llm_blocked_ms = llm_wait_start.elapsed().as_millis() as u64;
                    (text, llm_ms, llm_invalidated, llm_status, llm_skip_reason)
                }
            } else if raw.is_empty() {
                (String::new(), 0, false, LlmStatus::Disabled, None)
            } else {
                (snippet_fallback, 0, false, LlmStatus::Disabled, None)
            };

        if llm_invalidated && self.auto_edit.load(Ordering::SeqCst) {
            let _ = app.emit("llm-unready", ());
            crate::spawn_llm_recovery_if_needed(app.clone());
        }

        if self.check_cancelled(app) {
            return Ok(());
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
                llm_status,
                llm_skip_reason: llm_skip_reason.clone(),
                record_start_ms: None,
                mic_open_ms: None,
            },
        );

        if matches!(llm_status, LlmStatus::Skipped | LlmStatus::Failed) {
            let _ = app.emit(
                "llm-skipped",
                serde_json::json!({
                    "status": llm_status,
                    "reason": llm_skip_reason,
                }),
            );
        }

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
        *self.session_active_window.write() = None;
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
            if let Some(worker) = session.vad_worker {
                let _ = worker.join();
            }
            if let Some(worker) = session.stt_worker {
                let _ = worker.join();
            }
            if let Some(listener) = session.level_listener {
                let _ = listener.join();
            }
        }

        self.segment_transcripts.lock().clear();
        self.failed_segments.lock().clear();
        self.frozen_llm.lock().reset();
        self.streaming_stt_ms.store(0, Ordering::SeqCst);
        let _ = app.emit("partial-transcript-reset", ());
        hide_overlay(app);
    }
}

fn wait_for_llm_engine(
    llm: &Arc<Mutex<Option<LlamaEngine>>>,
    timeout: std::time::Duration,
    cancel_requested: &Arc<AtomicBool>,
) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if cancel_requested.load(Ordering::SeqCst) {
            return;
        }
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
    ValidationFailed {
        engine: LlamaEngine,
    },
    NotLoaded,
    WorkerFailed,
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
        llm_status: LlmStatus,
        llm_skip_reason: Option<String>,
    },
}

impl LlmCleanupJob {
    fn is_running(&self) -> bool {
        matches!(
            self.rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        )
    }

    fn join_worker(self) -> thread::Result<()> {
        self.worker.join()
    }

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
                    llm_status: LlmStatus::Applied,
                    llm_skip_reason: None,
                }
            }
            Ok(LlmCleanupOutcome::ValidationFailed { engine }) => {
                restore_llm_engine(&self.llm, &self.auto_edit, engine);
                let _ = self.worker.join();
                LlmCleanupWait::Completed {
                    text: self.raw,
                    llm_ms: self.start.elapsed().as_millis() as u64,
                    invalidated: false,
                    llm_status: LlmStatus::Failed,
                    llm_skip_reason: Some("validation_failed".into()),
                }
            }
            Ok(LlmCleanupOutcome::NotLoaded) => {
                let _ = self.worker.join();
                let invalidated =
                    invalidate_llm_engine_from_cleanup(&self.llm, &self.llm_ready, self.worker_pid);
                LlmCleanupWait::Completed {
                    text: self.raw,
                    llm_ms: self.start.elapsed().as_millis() as u64,
                    invalidated,
                    llm_status: LlmStatus::Skipped,
                    llm_skip_reason: Some("not_loaded".into()),
                }
            }
            Ok(LlmCleanupOutcome::WorkerFailed) => {
                let _ = self.worker.join();
                let invalidated =
                    invalidate_llm_engine_from_cleanup(&self.llm, &self.llm_ready, self.worker_pid);
                LlmCleanupWait::Completed {
                    text: self.raw,
                    llm_ms: self.start.elapsed().as_millis() as u64,
                    invalidated,
                    llm_status: LlmStatus::Failed,
                    llm_skip_reason: Some("worker_error".into()),
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
                    llm_status: LlmStatus::Failed,
                    llm_skip_reason: Some("timeout".into()),
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
                    llm_status: LlmStatus::Failed,
                    llm_skip_reason: Some("worker_error".into()),
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
                Err(crate::llm::LlmError::Prompt(prompt_err)) => {
                    eprintln!(
                        "llm cleanup validation failed, using raw transcript: {prompt_err}"
                    );
                    LlmCleanupOutcome::ValidationFailed { engine }
                }
                Err(crate::llm::LlmError::Worker(msg)) => {
                    eprintln!("llm cleanup worker error, using raw transcript: {msg}");
                    LlmCleanupOutcome::WorkerFailed
                }
            },
            None => {
                eprintln!("llm cleanup failed: llm engine not loaded");
                LlmCleanupOutcome::NotLoaded
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
    transcripts: Arc<Mutex<Vec<SegmentTranscript>>>,
    failed_segments: Arc<Mutex<Vec<FailedSegment>>>,
    stt_time_ms: Arc<AtomicU64>,
    dictionary_prompt: Arc<RwLock<Option<Arc<str>>>>,
    session_stt_language: Arc<RwLock<SttLanguage>>,
    session_detected_language: Arc<RwLock<Option<SttLanguage>>>,
    pending_detection: Arc<Mutex<Option<SttLanguage>>>,
    snippets: Arc<RwLock<Vec<Snippet>>>,
    history_store: Option<Arc<Store>>,
    frozen_llm: Arc<Mutex<FrozenLlmCoordinator>>,
    llm: Arc<Mutex<Option<LlamaEngine>>>,
    llm_ready: Arc<AtomicBool>,
    auto_edit: Arc<AtomicBool>,
    correction_rules: Arc<RwLock<Vec<CorrectionRule>>>,
    session_tone: Arc<RwLock<ToneProfile>>,
}

fn streaming_worker(
    chunk_rx: std::sync::mpsc::Receiver<Vec<f32>>,
    stop_flag: Arc<AtomicBool>,
    mut vad: VadSegmenter,
    segment_tx: std::sync::mpsc::Sender<PendingSegment>,
) {
    let mut segment_index = 0_u32;

    let enqueue_segments = |segments: Vec<SpeechSegment>, index: &mut u32| {
        for segment in segments {
            let _ = segment_tx.send(PendingSegment {
                segment,
                index: *index,
            });
            *index += 1;
        }
    };

    while let Ok(chunk) = chunk_rx.recv() {
        let segments = match vad.push(&chunk) {
            Ok(segments) => segments,
            Err(err) => {
                eprintln!("VAD error: {err}");
                continue;
            }
        };
        enqueue_segments(segments, &mut segment_index);
        if stop_flag.load(Ordering::SeqCst) {
            while let Ok(chunk) = chunk_rx.try_recv() {
                let segments = match vad.push(&chunk) {
                    Ok(segments) => segments,
                    Err(err) => {
                        eprintln!("VAD error: {err}");
                        continue;
                    }
                };
                enqueue_segments(segments, &mut segment_index);
            }
            break;
        }
    }

    if let Ok(segments) = vad.flush() {
        enqueue_segments(segments, &mut segment_index);
    }
}

fn stt_segment_worker(
    app: AppHandle,
    segment_rx: std::sync::mpsc::Receiver<PendingSegment>,
    _stop_flag: Arc<AtomicBool>,
    stt: StreamingSttContext,
) {
    while let Ok(pending) = segment_rx.recv() {
        transcribe_segment(&app, &stt, pending.segment, pending.index);
    }
}

fn transcribe_segment(
    app: &AppHandle,
    stt: &StreamingSttContext,
    segment: SpeechSegment,
    segment_index: u32,
) {
    let stt_start = Instant::now();
    let prompt = stt.dictionary_prompt.read().clone();
    let segment_language = *stt.session_stt_language.read();
    let text = {
        let mut engine_guard = stt.whisper.lock();
        let Some(engine) = engine_guard.as_mut() else {
            stt.failed_segments.lock().push(FailedSegment {
                samples: segment.samples,
                language: segment_language,
            });
            return;
        };
        match engine.transcribe_with_language(&segment.samples, prompt.as_deref(), segment_language)
        {
            Ok(result) => result,
            Err(err) => {
                eprintln!("segment transcription failed: {err}");
                stt.failed_segments.lock().push(FailedSegment {
                    samples: segment.samples,
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
        transcripts.push((text.text.clone(), segment.leading_silence_ms));
        join_transcript_segments_with_pauses(transcripts.as_slice())
    };

    if stt.auto_edit.load(Ordering::SeqCst) {
        let segments_snapshot = stt.transcripts.lock().clone();
        let rules = stt.correction_rules.read().clone();
        let snippets = stt.snippets.read().clone();
        let tone = *stt.session_tone.read();
        stt.frozen_llm.lock().maybe_start_background(
            &segments_snapshot,
            true,
            &stt.llm,
            &stt.llm_ready,
            &stt.auto_edit,
            &rules,
            &snippets,
            tone,
        );
    }

    let snippet_ctx = build_snippet_variable_context(stt.history_store.as_deref());
    let display = prepare_partial_transcript(&raw, &stt.snippets.read(), &snippet_ctx);

    let _ = app.emit(
        "stt-segment-progress",
        SttSegmentProgressEvent {
            completed: segment_index.saturating_add(1),
        },
    );

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

pub fn spawn_cancel(app: AppHandle, orchestrator: Arc<Mutex<PipelineOrchestrator>>) {
    std::thread::spawn(move || {
        let mut guard = orchestrator.lock();
        let _ = guard.cancel_processing(&app);
    });
}

pub fn emit_dictation_busy(app: &AppHandle, state: PipelineState, cancelable: bool) {
    let _ = app.emit(
        "dictation-busy",
        DictationBusyEvent {
            state: state.as_str().into(),
            cancelable,
        },
    );
}

pub fn spawn_start(app: AppHandle, orchestrator: Arc<Mutex<PipelineOrchestrator>>) {
    std::thread::spawn(move || {
        crate::touch_activity(&app);
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

        if result.is_ok() {
            crate::touch_activity(&app);
            crate::spawn_llm_on_demand_if_needed(app.clone());
        }

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
        let result = prepare_display_transcript(
            "voici mon email",
            &snippets,
            &SnippetVariableContext::default(),
        );
        assert_eq!(result, "Voici contact at gmail point com");
    }

    #[test]
    fn find_latest_frozen_boundary_detects_sentence_end_with_long_pause() {
        use calliop_prompt::find_latest_frozen_boundary;

        let segments: Vec<(String, u32)> = vec![
            ("Première phrase.".into(), 0),
            ("Deuxième phrase".into(), 1600),
        ];
        assert_eq!(find_latest_frozen_boundary(&segments), Some(0));
    }

    #[test]
    fn merge_cleaned_segments_joins_with_space() {
        assert_eq!(
            merge_cleaned_segments("Bonjour.", "Au revoir."),
            "Bonjour. Au revoir."
        );
    }

    #[test]
    fn transcribing_is_hotkey_cancelable_not_injecting() {
        assert!(PipelineState::Transcribing.hotkey_cancelable());
        assert!(!PipelineState::Injecting.hotkey_cancelable());
        assert!(!PipelineState::Recording.hotkey_cancelable());
    }

    #[test]
    fn prepare_partial_transcript_applies_post_process() {
        let result = prepare_partial_transcript(
            "bonjour euh monde",
            &[],
            &SnippetVariableContext::default(),
        );
        assert!(!result.contains("euh"));
    }

    #[test]
    fn plan_llm_chunks_splits_long_transcript() {
        let long_segment = ("mot ".repeat(500)).trim().to_string();
        let segments = vec![(long_segment.clone(), 0), (long_segment, 700)];
        let chunks = plan_llm_chunks_from_segments(&segments, &[], &[]);
        assert!(chunks.len() > 1);
        for (text, _) in chunks {
            assert!(fits_llm_cleanup_budget(&text));
        }
    }

    #[test]
    fn split_sentences_keeps_punctuation() {
        let parts = split_sentences("Bonjour. Au revoir!");
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "Bonjour.");
        assert_eq!(parts[1], "Au revoir!");
    }

    #[test]
    fn llm_cleanup_timeout_scales_with_text_length() {
        let short = llm_cleanup_timeout("hello");
        let long = llm_cleanup_timeout(&"x".repeat(10_000));
        assert!(long > short);
        assert!(long <= LLM_CLEANUP_TIMEOUT_MAX);
    }
}
