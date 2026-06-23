pub mod achievements;
pub mod app_context;
pub mod audio;
mod commands;
pub mod dictionary_notify;
pub mod hotkey;
pub mod inference;
pub mod inject;
pub mod llm;
pub mod observe;
pub mod pipeline;
pub mod process_util;
pub mod store;
pub mod stt;
pub mod system;
pub mod system_notify;
pub mod ui;
pub mod update;
pub mod user_error;

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use calliop_prompt::ToneProfile;
use dictionary_notify::DictionaryNotifier;
use parking_lot::Mutex;
use pipeline::{
    request_dictation, CorrectionRule, DictationIntent, PipelineOrchestrator, PipelineState,
    PipelineStateEvent,
};
use serde::{Deserialize, Serialize};
use store::{
    extract_correction_words, AppContextMatchType, AppContextRule, AppSettings, DictionarySource,
    DictionaryWord, Snippet, Store, KEY_AUTOSTART,
};
use stt::{SttLanguage, WhisperModel, WhisperPromptCache, MAX_INITIAL_PROMPT_WORDS};
use system::{resolve_perf_config, RuntimePerfConfig, SystemCapabilities};
use system_notify::{notify_update_ready, ModelsReadyNotifier};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, RunEvent, Theme, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_updater::UpdaterExt;

use user_error::{user_error_string, UserError};

/// Returns registered core module names (Phase 0 wiring check).
pub fn registered_modules() -> [&'static str; 11] {
    [
        audio::module_name(),
        stt::module_name(),
        llm::module_name(),
        inference::module_name(),
        inject::module_name(),
        hotkey::module_name(),
        store::module_name(),
        observe::module_name(),
        app_context::module_name(),
        pipeline::module_name(),
        system::module_name(),
    ]
}

#[derive(Default)]
struct HotkeyPressState {
    press_start: Option<Instant>,
    was_idle_on_press: bool,
    /// True between Pressed and Released — filters OS key-repeat while holding.
    shortcut_down: bool,
    /// Press from idle while Whisper is still loading.
    deferred_start_pending: bool,
    /// Captured on release during deferred load (short toggle tap).
    deferred_toggle_intent: bool,
    /// Release during Transcribing cancels the in-flight pipeline.
    busy_cancel_on_release: bool,
}

#[derive(Debug, Clone, Serialize)]
struct DictationBlockedPayload {
    reason: String,
}

struct TrayHandles {
    open_item: MenuItem<tauri::Wry>,
    toggle_item: MenuItem<tauri::Wry>,
    language_item: MenuItem<tauri::Wry>,
    settings_item: MenuItem<tauri::Wry>,
    history_item: MenuItem<tauri::Wry>,
    dictionary_item: MenuItem<tauri::Wry>,
    snippets_item: MenuItem<tauri::Wry>,
    insight_item: MenuItem<tauri::Wry>,
    auto_edit_item: CheckMenuItem<tauri::Wry>,
    autostart_item: CheckMenuItem<tauri::Wry>,
    quit_item: MenuItem<tauri::Wry>,
}

#[derive(Debug, Clone, Serialize)]
struct NavigateViewPayload {
    view: String,
}

pub(crate) struct MicProbeState {
    pub(crate) capture: Mutex<Option<audio::AudioCapture>>,
    pub(crate) level_task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}

pub(crate) struct AppState {
    pub(crate) pipeline: Arc<Mutex<PipelineOrchestrator>>,
    pub(crate) whisper_engine: Arc<Mutex<Option<stt::WhisperEngine>>>,
    pub(crate) llm_engine: Arc<Mutex<Option<llm::LlamaEngine>>>,
    pub(crate) store: Arc<Store>,
    pub(crate) achievements: Arc<achievements::AchievementEngine>,
    pub(crate) prompt_cache: Mutex<WhisperPromptCache>,
    pub(crate) dictionary_notifier: Arc<DictionaryNotifier>,
    pub(crate) models_ready_notifier: Arc<ModelsReadyNotifier>,
    pub(crate) capabilities: SystemCapabilities,
    pub(crate) perf_config: Mutex<RuntimePerfConfig>,
    pub(crate) last_activity: Mutex<Instant>,
    pub(crate) model_ready: AtomicBool,
    pub(crate) model_init: Arc<tokio::sync::Mutex<()>>,
    pub(crate) llm_ready: Arc<AtomicBool>,
    pub(crate) llm_init: Arc<tokio::sync::Mutex<()>>,
    pub(crate) hotkey_press: Mutex<HotkeyPressState>,
    pub(crate) deferred_llm_on_boot: AtomicBool,
    pub(crate) current_hotkey: Mutex<hotkey::HotkeyBinding>,
    pub(crate) hotkey_suspend_depth: AtomicU32,
    pub(crate) whisper_settings_change_depth: AtomicU32,
    pub(crate) loaded_whisper: Mutex<Option<WhisperModel>>,
    pub(crate) loaded_llm: Mutex<Option<llm::LlmModel>>,
    pub(crate) mic_probe: MicProbeState,
    pub(crate) pending_update: update::PendingUpdateStore,
    pub(crate) update_check_in_progress: AtomicBool,
}

/// Held while `set_settings` downloads or reloads the active Whisper model.
pub(crate) struct WhisperSettingsChangeGuard<'a> {
    depth: &'a AtomicU32,
}

impl<'a> WhisperSettingsChangeGuard<'a> {
    pub(crate) fn new(state: &'a AppState) -> Self {
        Self::acquire(&state.whisper_settings_change_depth)
    }

    fn acquire(depth: &'a AtomicU32) -> Self {
        depth.fetch_add(1, Ordering::SeqCst);
        Self { depth }
    }
}

impl Drop for WhisperSettingsChangeGuard<'_> {
    fn drop(&mut self) {
        self.depth.fetch_sub(1, Ordering::SeqCst);
    }
}

fn is_dictation_blocked_by_settings(state: &AppState) -> bool {
    state.whisper_settings_change_depth.load(Ordering::SeqCst) > 0
}

pub(crate) fn is_dictation_start_blocked_by_settings(
    state: &AppState,
    pipeline_state: PipelineState,
) -> bool {
    pipeline_state == PipelineState::Idle && is_dictation_blocked_by_settings(state)
}

pub(crate) fn notify_dictation_blocked_by_settings(app: &AppHandle, state: &AppState) {
    clear_deferred_hotkey_start(state);
    let _ = app.emit(
        "dictation-blocked",
        DictationBlockedPayload {
            reason: "WHISPER_SETTINGS_CHANGE".into(),
        },
    );
}

pub(crate) fn llm_engine_is_live(state: &AppState) -> bool {
    state.llm_ready.load(Ordering::SeqCst) && state.llm_engine.lock().is_some()
}

pub(crate) fn shutdown_llm_engine(state: &AppState) {
    state.llm_ready.store(false, Ordering::SeqCst);
    *state.llm_engine.lock() = None;
    *state.loaded_llm.lock() = None;
}

pub(crate) fn refresh_perf_config(state: &AppState, settings: &AppSettings, start_minimized: bool) {
    let perf = resolve_perf_config(settings, &state.capabilities, start_minimized);
    state
        .pipeline
        .lock()
        .set_vad_chunk_size(perf.vad_chunk_size);
    *state.perf_config.lock() = perf;
}

pub fn touch_activity(app: &AppHandle) {
    let state = app.state::<AppState>();
    *state.last_activity.lock() = Instant::now();
}

pub fn spawn_llm_on_demand_if_needed(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        if !state.pipeline.lock().auto_edit_enabled() {
            return;
        }
        if llm_engine_is_live(&state) {
            return;
        }
        if !state.perf_config.lock().llm_lazy_load {
            return;
        }
        if let Err(err) = ensure_llm_model_inner(&app, &state).await {
            eprintln!("llm on-demand load failed: {err}");
        }
    });
}

fn spawn_whisper_idle_watchdog(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let state = app.state::<AppState>();
            let idle_limit = state.perf_config.lock().whisper_unload_idle;
            let Some(idle_limit) = idle_limit else {
                continue;
            };
            if is_mic_probe_active(&state) {
                continue;
            }
            if state.pipeline.lock().state() != PipelineState::Idle {
                continue;
            }
            if !state.model_ready.load(Ordering::SeqCst) {
                continue;
            }
            if state.last_activity.lock().elapsed() < idle_limit {
                continue;
            }
            if invalidate_whisper_engine(&state, false) {
                let _ = app.emit("model-unready", ());
            }
        }
    });
}

pub(crate) fn spawn_llm_recovery_if_needed(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        if !state.pipeline.lock().auto_edit_enabled() {
            return;
        }
        if llm_engine_is_live(&state) {
            return;
        }
        if let Err(err) = ensure_llm_model_inner(&app, &state).await {
            eprintln!("llm recovery after invalidation failed: {err}");
        }
    });
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SettingsPayload {
    auto_edit: bool,
    auto_edit_mode: String,
    pause_preset: String,
    auto_learn: bool,
    auto_update: bool,
    stt_language: String,
    whisper_model: String,
    llm_model: String,
    hotkey: String,
    inference_backend: String,
    low_power_mode: bool,
    adaptive_perf: bool,
    ui_language: String,
    input_device: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ModelStatusEntry {
    id: String,
    label: String,
    installed: bool,
    size_bytes: Option<u64>,
    active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ModelsStatusPayload {
    whisper: Vec<ModelStatusEntry>,
    llm: Vec<ModelStatusEntry>,
}

pub(crate) fn settings_to_payload(settings: &AppSettings) -> SettingsPayload {
    SettingsPayload {
        auto_edit: settings.auto_edit_mode.uses_llm(),
        auto_edit_mode: settings.auto_edit_mode.as_setting_value().into(),
        pause_preset: settings.pause_preset.as_setting_value().into(),
        auto_learn: settings.auto_learn,
        auto_update: settings.auto_update,
        stt_language: settings.stt_language.clone(),
        whisper_model: settings.whisper_model.clone(),
        llm_model: settings.llm_model.clone(),
        hotkey: settings.hotkey.clone(),
        inference_backend: settings.inference_backend.clone(),
        low_power_mode: settings.low_power_mode,
        adaptive_perf: settings.adaptive_perf,
        ui_language: settings.ui_language.clone(),
        input_device: settings.input_device.clone(),
    }
}

fn current_ui_language(app: &AppHandle) -> String {
    app.try_state::<AppState>()
        .and_then(|state| state.store.load_settings().ok())
        .map(|settings| settings.ui_language)
        .unwrap_or_else(ui::locale::default_ui_language)
}

pub(crate) fn parse_ui_language(value: &str) -> String {
    if value.trim() == "en" {
        "en".into()
    } else {
        "fr".into()
    }
}

fn file_size_bytes(path: &std::path::Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|meta| meta.len())
}

pub(crate) fn build_models_status(state: &AppState, settings: &AppSettings) -> ModelsStatusPayload {
    let active_whisper =
        system::resolve_whisper_model(settings.whisper_model(), &state.capabilities);
    let active_llm = system::resolve_llm_model(settings.llm_model(), &state.capabilities);

    ModelsStatusPayload {
        whisper: WhisperModel::all_concrete()
            .into_iter()
            .map(|model| {
                let path = model.path();
                ModelStatusEntry {
                    id: model.as_setting_value().into(),
                    label: model.label().into(),
                    installed: model.is_installed(),
                    size_bytes: if path.exists() {
                        file_size_bytes(&path)
                    } else {
                        None
                    },
                    active: model == active_whisper,
                }
            })
            .collect(),
        llm: llm::LlmModel::all_concrete()
            .into_iter()
            .map(|model| {
                let path = model.path();
                ModelStatusEntry {
                    id: model.as_setting_value().into(),
                    label: model.label().into(),
                    installed: model.is_installed(),
                    size_bytes: if path.exists() {
                        file_size_bytes(&path)
                    } else {
                        None
                    },
                    active: model == active_llm,
                }
            })
            .collect(),
    }
}

/// True when Whisper is loaded and safe to use for dictation.
pub(crate) fn whisper_is_live(state: &AppState) -> bool {
    state.model_ready.load(Ordering::SeqCst) && state.whisper_engine.lock().is_some()
}

/// Returns true when a previously live Whisper engine was invalidated.
pub(crate) fn invalidate_whisper_engine(state: &AppState, force: bool) -> bool {
    if !force && state.pipeline.lock().state() != PipelineState::Idle {
        return false;
    }
    let was_live = whisper_is_live(state);
    state.model_ready.store(false, Ordering::SeqCst);
    *state.whisper_engine.lock() = None;
    *state.loaded_whisper.lock() = None;
    state.models_ready_notifier.reset();
    was_live
}

pub(crate) fn emit_model_unready_if_needed(app: &AppHandle, was_live: bool) {
    if was_live {
        let _ = app.emit("model-unready", ());
    }
}

/// Re-load Whisper then start dictation if the hotkey intent is still valid.
pub(crate) fn request_deferred_dictation_start(app: &AppHandle) {
    let state = app.state::<AppState>();
    let spawn_load = {
        let mut press = state.hotkey_press.lock();
        if press.deferred_start_pending {
            false
        } else {
            press.deferred_start_pending = true;
            press.deferred_toggle_intent = true;
            true
        }
    };
    if !spawn_load {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        ensure_model_then_start(app).await;
    });
}

fn whisper_engine_matches(state: &AppState, expected: WhisperModel) -> bool {
    whisper_is_live(state) && *state.loaded_whisper.lock() == Some(expected)
}

fn llm_engine_matches(state: &AppState, expected: llm::LlmModel) -> bool {
    llm_engine_is_live(state) && *state.loaded_llm.lock() == Some(expected)
}

pub(crate) fn llm_engine_stale(state: &AppState, expected: llm::LlmModel) -> bool {
    match *state.loaded_llm.lock() {
        Some(loaded) => loaded != expected,
        None => llm_engine_is_live(state),
    }
}

pub(crate) async fn ensure_whisper_model_file(
    app: &AppHandle,
    model: WhisperModel,
) -> Result<(), String> {
    if !model.is_concrete() {
        return Ok(());
    }
    let app_for_download = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        stt::ensure_model_file_blocking(Some(&app_for_download), model)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) async fn ensure_llm_model_file(
    app: &AppHandle,
    model: llm::LlmModel,
) -> Result<(), String> {
    if !model.is_concrete() {
        return Ok(());
    }
    let app_for_download = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        llm::ensure_llm_model_file_blocking(Some(&app_for_download), model)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) struct SettingsRollbackContext {
    hotkey_changed: bool,
    stt_language_changed: bool,
    whisper_invalidated: bool,
}

pub(crate) async fn rollback_settings(
    app: &AppHandle,
    state: &AppState,
    previous: &AppSettings,
    ctx: SettingsRollbackContext,
) -> Result<(), String> {
    let app_for_notify = app.clone();
    state
        .store
        .save_settings(previous)
        .map_err(|e| e.to_string())?;
    refresh_perf_config(state, previous, should_start_minimized());
    if ctx.hotkey_changed {
        let prev_binding =
            hotkey::parse_hotkey_setting(&previous.hotkey).map_err(|e| e.to_string())?;
        register_hotkey_binding(&app_for_notify, prev_binding)?;
    }
    state
        .pipeline
        .lock()
        .set_default_stt_language(previous.stt_language_mode());
    state.pipeline.lock().set_auto_learn(previous.auto_learn);
    state
        .pipeline
        .lock()
        .set_auto_edit_mode(previous.auto_edit_mode);
    state
        .pipeline
        .lock()
        .set_pause_preset(previous.pause_preset);

    if ctx.whisper_invalidated {
        let was_live = invalidate_whisper_engine(state, true);
        emit_model_unready_if_needed(app, was_live);
        ensure_model_inner(app, state).await?;
    }

    if previous.auto_edit_mode.uses_llm() {
        shutdown_llm_engine(state);
        ensure_llm_model_inner(app, state).await?;
    } else {
        shutdown_llm_engine(state);
    }

    if ctx.stt_language_changed {
        state
            .pipeline
            .lock()
            .notify_stt_language_changed(&app_for_notify);
    }
    Ok(())
}

fn unregister_current_hotkey(app: &AppHandle, state: &AppState) -> Result<(), String> {
    match *state.current_hotkey.lock() {
        hotkey::HotkeyBinding::KeyCombo(shortcut) => {
            let gs = app.global_shortcut();
            if gs.is_registered(shortcut) {
                gs.unregister(shortcut).map_err(|e| e.to_string())?;
            }
        }
        hotkey::HotkeyBinding::ModifiersOnly(_) => {
            #[cfg(windows)]
            hotkey::stop_modifier_dictation_hook()?;
            #[cfg(not(windows))]
            return Err("modifier-only hotkeys are only supported on Windows".into());
        }
    }
    Ok(())
}

pub(crate) fn suspend_global_hotkey(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let prev = state.hotkey_suspend_depth.fetch_add(1, Ordering::SeqCst);
    if prev == 0 {
        unregister_current_hotkey(app, state)?;
        // A release event may be lost while the hotkey is unregistered during model load.
        let mut press = state.hotkey_press.lock();
        press.shortcut_down = false;
        press.press_start = None;
        press.busy_cancel_on_release = false;
    }
    Ok(())
}

pub(crate) fn resume_global_hotkey(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let mut current = state.hotkey_suspend_depth.load(Ordering::SeqCst);
    loop {
        if current == 0 {
            return Ok(());
        }
        match state.hotkey_suspend_depth.compare_exchange_weak(
            current,
            current - 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => {
                if current - 1 == 0 {
                    register_hotkey_from_settings(app, state)?;
                }
                return Ok(());
            }
            Err(actual) => current = actual,
        }
    }
}

struct HotkeySuspendGuard<'a> {
    app: &'a AppHandle,
    state: &'a AppState,
    active: bool,
}

impl Drop for HotkeySuspendGuard<'_> {
    fn drop(&mut self) {
        if self.active {
            let _ = resume_global_hotkey(self.app, self.state);
        }
    }
}

fn register_hotkey_from_settings(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let settings = state.store.load_settings().map_err(|e| e.to_string())?;
    let binding = hotkey::parse_hotkey_setting(&settings.hotkey).unwrap_or_else(|err| {
        eprintln!(
            "invalid stored hotkey ({}): {err}, using default",
            settings.hotkey
        );
        hotkey::HotkeyBinding::KeyCombo(hotkey::default_shortcut())
    });
    register_hotkey_binding(app, binding)
}

fn ensure_dictation_hotkey_registered(app: &AppHandle, state: &AppState) -> Result<(), String> {
    if state.hotkey_suspend_depth.load(Ordering::SeqCst) > 0 {
        return Ok(());
    }
    register_hotkey_from_settings(app, state)
}

pub(crate) fn is_mic_probe_active(state: &AppState) -> bool {
    state.mic_probe.capture.lock().is_some()
}

pub(crate) fn register_hotkey_binding(
    app: &AppHandle,
    binding: hotkey::HotkeyBinding,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    unregister_current_hotkey(app, state.inner())?;

    match binding {
        hotkey::HotkeyBinding::KeyCombo(shortcut) => {
            let gs = app.global_shortcut();
            gs.register(shortcut).map_err(|e| e.to_string())?;
        }
        #[cfg(windows)]
        hotkey::HotkeyBinding::ModifiersOnly(modifiers) => {
            hotkey::start_modifier_dictation_hook(app, modifiers)?;
        }
        #[cfg(not(windows))]
        hotkey::HotkeyBinding::ModifiersOnly(_) => {
            return Err("modifier-only hotkeys are only supported on Windows".into());
        }
    }

    *state.current_hotkey.lock() = binding;
    Ok(())
}
pub(crate) fn parse_stt_language(value: &str) -> Result<SttLanguage, String> {
    SttLanguage::parse(value).ok_or_else(|| user_error_string(UserError::UnsupportedSttLanguage))
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DictionaryWordPayload {
    id: i64,
    word: String,
    source: String,
    misspelling: Option<String>,
    created_at: String,
}

pub(crate) fn dictionary_word_to_payload(word: DictionaryWord) -> DictionaryWordPayload {
    DictionaryWordPayload {
        id: word.id,
        word: word.word,
        source: match word.source {
            DictionarySource::Manual => "manual".into(),
            DictionarySource::Learned => "learned".into(),
        },
        misspelling: word.misspelling,
        created_at: word.created_at,
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SnippetPayload {
    id: i64,
    trigger: String,
    content: String,
    created_at: String,
}

pub(crate) fn snippet_to_payload(snippet: Snippet) -> SnippetPayload {
    SnippetPayload {
        id: snippet.id,
        trigger: snippet.trigger,
        content: snippet.content,
        created_at: snippet.created_at,
    }
}

fn sync_prompt_cache_to_pipeline(state: &AppState) {
    let prompt = state.prompt_cache.lock().prompt();
    state.pipeline.lock().set_dictionary_prompt_arc(prompt);
}

fn refresh_whisper_prompt_with_cache(
    store: &Store,
    pipeline: &Arc<Mutex<PipelineOrchestrator>>,
    cache: &Mutex<WhisperPromptCache>,
) -> Result<(), String> {
    let snippets = store.list_snippets().map_err(|e| e.to_string())?;
    let snippet_triggers: Vec<String> = snippets.iter().map(|s| s.trigger.clone()).collect();
    let dictionary_words = store
        .list_words_for_prompt(MAX_INITIAL_PROMPT_WORDS)
        .map_err(|e| e.to_string())?;

    {
        let mut guard = cache.lock();
        guard.rebuild(snippet_triggers, dictionary_words);
    }

    // Lock cache before pipeline (never hold pipeline while acquiring cache).
    let prompt = cache.lock().prompt();
    let mut pipeline_guard = pipeline.lock();
    pipeline_guard.set_snippets(snippets);
    pipeline_guard.set_dictionary_prompt_arc(prompt);
    Ok(())
}

pub(crate) fn refresh_whisper_prompt_full(state: &AppState) -> Result<(), String> {
    refresh_whisper_prompt_with_cache(&state.store, &state.pipeline, &state.prompt_cache)
}

fn ensure_pipeline_snippets_loaded(store: &Store, pipeline: &Arc<Mutex<PipelineOrchestrator>>) {
    match store.list_snippets() {
        Ok(snippets) => pipeline.lock().set_snippets(snippets),
        Err(err) => eprintln!("failed to load snippets cache: {err}"),
    }
}

pub(crate) fn refresh_whisper_prompt_state(state: &AppState) -> Result<(), String> {
    refresh_whisper_prompt_full(state)
}

pub(crate) fn apply_dictionary_additions(state: &AppState, added: &[String]) -> Result<(), String> {
    let changed = {
        let mut cache = state.prompt_cache.lock();
        cache.apply_additions(added)
    };
    if changed {
        sync_prompt_cache_to_pipeline(state);
    }
    Ok(())
}

pub(crate) fn apply_learned_correction(
    app: &AppHandle,
    state: &AppState,
    original: &str,
    corrected: &str,
) -> Result<Vec<String>, String> {
    let candidates = extract_correction_words(original, corrected);
    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    let added = state
        .store
        .add_words_batch(&candidates, DictionarySource::Learned)
        .map_err(|e| e.to_string())?;

    if added.is_empty() {
        return Ok(Vec::new());
    }

    if let Err(err) = apply_dictionary_additions(state, &added) {
        for word in &added {
            let _ = state.store.remove_word_by_normalized(word);
        }
        return Err(err);
    }

    state.dictionary_notifier.queue_added(app, added.clone());
    if let Err(err) = state.achievements.on_learned_correction(app) {
        eprintln!("achievement evaluation failed: {err}");
    }
    Ok(added)
}

pub(crate) fn emit_snippets_updated(app: &AppHandle) {
    let _ = app.emit("snippets-updated", ());
}

pub(crate) fn emit_app_context_updated(app: &AppHandle) {
    let _ = app.emit("app-context-updated", ());
}

fn refresh_app_context_rules(
    store: &Store,
    pipeline: &Arc<Mutex<PipelineOrchestrator>>,
) -> Result<(), String> {
    let rules = store.list_app_context_rules().map_err(|e| e.to_string())?;
    pipeline.lock().set_app_context_rules(rules);
    Ok(())
}

pub(crate) fn refresh_correction_rules(
    store: &Store,
    pipeline: &Arc<Mutex<PipelineOrchestrator>>,
) -> Result<(), String> {
    let rules = store
        .list_correction_rules()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(CorrectionRule::from)
        .collect();
    pipeline.lock().set_correction_rules(rules);
    Ok(())
}

pub(crate) fn refresh_app_context_rules_state(state: &AppState) -> Result<(), String> {
    refresh_app_context_rules(&state.store, &state.pipeline)
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AppContextRulePayload {
    id: i64,
    pattern: String,
    #[serde(rename = "matchType")]
    match_type: String,
    tone: String,
    #[serde(rename = "createdAt")]
    created_at: String,
}

pub(crate) fn app_context_rule_to_payload(rule: AppContextRule) -> AppContextRulePayload {
    AppContextRulePayload {
        id: rule.id,
        pattern: rule.pattern,
        match_type: match rule.match_type {
            AppContextMatchType::Exe => "exe".into(),
            AppContextMatchType::TitleContains => "title_contains".into(),
        },
        tone: rule.tone.as_str().into(),
        created_at: rule.created_at,
    }
}

pub(crate) fn parse_match_type(value: &str) -> Result<AppContextMatchType, String> {
    AppContextMatchType::parse(value).ok_or_else(|| user_error_string(UserError::InvalidMatchType))
}

pub(crate) fn parse_tone_profile(value: &str) -> Result<ToneProfile, String> {
    ToneProfile::parse(value).ok_or_else(|| user_error_string(UserError::InvalidTone))
}

const MENU_OPEN: &str = "open";
const MENU_TOGGLE: &str = "toggle";
const MENU_CYCLE_LANGUAGE: &str = "cycle_language";
const MENU_OPEN_SETTINGS: &str = "open_settings";
const MENU_OPEN_HISTORY: &str = "open_history";
const MENU_OPEN_DICTIONARY: &str = "open_dictionary";
const MENU_OPEN_SNIPPETS: &str = "open_snippets";
const MENU_OPEN_INSIGHT: &str = "open_insight";
const MENU_AUTO_EDIT: &str = "auto_edit";
const MENU_AUTOSTART: &str = "autostart";
const MENU_QUIT: &str = "quit";

pub(crate) async fn ensure_llm_model_inner(
    app: &AppHandle,
    state: &AppState,
) -> Result<(), String> {
    if !state.pipeline.lock().auto_edit_enabled() {
        return Ok(());
    }

    let expected = state.perf_config.lock().llm;
    if llm_engine_matches(state, expected) {
        return Ok(());
    }

    state.models_ready_notifier.reset_for_llm_reload();

    if state.llm_ready.load(Ordering::SeqCst) || state.llm_engine.lock().is_some() {
        shutdown_llm_engine(state);
    }

    let _init_guard = state.llm_init.lock().await;

    if !state.pipeline.lock().auto_edit_enabled() {
        return Ok(());
    }

    if llm_engine_matches(state, expected) {
        return Ok(());
    }

    shutdown_llm_engine(state);

    let settings = state.store.load_settings().map_err(|e| e.to_string())?;
    let llm_model = state.perf_config.lock().llm;
    let n_gpu_layers = inference::gpu_layers(settings.inference_backend());

    let app_for_download = app.clone();
    let model_path = tauri::async_runtime::spawn_blocking(move || {
        llm::ensure_llm_model_blocking(Some(&app_for_download), llm_model)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    if !state.pipeline.lock().auto_edit_enabled() {
        return Ok(());
    }

    let engine = match tauri::async_runtime::spawn_blocking(move || {
        let mut engine = llm::LlamaEngine::start_with_config(&model_path, n_gpu_layers)?;
        if let Err(err) = engine.cleanup("bonjour", ToneProfile::Default) {
            eprintln!("llm warmup failed (non-fatal): {err}");
        }
        Ok::<llm::LlamaEngine, llm::LlmError>(engine)
    })
    .await
    .map_err(|e| e.to_string())?
    {
        Ok(engine) => engine,
        Err(err) => {
            let msg = err.to_string();
            if llm::invalidate_corrupt_model_file(llm_model, &msg) {
                return Err(user_error_string(UserError::LlmModelCorrupt));
            }
            return Err(msg);
        }
    };

    if !state.pipeline.lock().auto_edit_enabled() {
        return Ok(());
    }

    {
        *state.llm_engine.lock() = Some(engine);
    }
    {
        let mut pipeline = state.pipeline.lock();
        pipeline.set_llm_engine(Arc::clone(&state.llm_engine));
        pipeline.set_llm_ready(Arc::clone(&state.llm_ready));
    }

    *state.loaded_llm.lock() = Some(llm_model);
    state.llm_ready.store(true, Ordering::SeqCst);
    let _ = app.emit("llm-ready", ());
    state.models_ready_notifier.on_llm_loaded(app, &state.store);
    Ok(())
}

pub(crate) fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let autolaunch = app.autolaunch();
    if enabled {
        autolaunch.enable().map_err(|e| e.to_string())?;
    } else {
        autolaunch.disable().map_err(|e| e.to_string())?;
    }
    sync_tray_menus(app);
    Ok(())
}

fn autostart_enabled_from_store(app: &AppHandle) -> bool {
    app.try_state::<AppState>()
        .and_then(|state| state.store.get_autostart().ok())
        .unwrap_or(true)
}

fn bootstrap_autostart_setting(store: &Store, app: &AppHandle) {
    if store.has_setting(KEY_AUTOSTART).unwrap_or(false) {
        return;
    }

    let seed = if store.is_onboarding_done().unwrap_or(false) {
        app.autolaunch().is_enabled().unwrap_or(false)
    } else {
        true
    };

    if let Err(err) = store.set_autostart(seed) {
        eprintln!("failed to seed autostart setting: {err}");
    }
}

fn sync_autostart_from_settings(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };

    bootstrap_autostart_setting(&state.store, app);

    match state.store.get_autostart() {
        Ok(enabled) => {
            if let Err(err) = apply_autostart(app, enabled) {
                eprintln!("failed to apply autostart setting: {err}");
            }
        }
        Err(err) => eprintln!("failed to read autostart setting: {err}"),
    }
}

pub(crate) fn stop_mic_probe_inner(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let mut capture_slot = state.mic_probe.capture.lock();
    let was_active = capture_slot.is_some();
    if let Some(mut capture) = capture_slot.take() {
        let _ = capture.stop();
    }
    if let Some(handle) = state.mic_probe.level_task.lock().take() {
        handle.abort();
    }
    drop(capture_slot);

    if was_active {
        resume_global_hotkey(app, state)?;
    }
    Ok(())
}

pub(crate) async fn wait_for_pipeline_idle_timeout(
    pipeline: &Arc<Mutex<PipelineOrchestrator>>,
    timeout: Duration,
) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if pipeline.lock().state() == PipelineState::Idle {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    pipeline.lock().state() == PipelineState::Idle
}

fn set_auto_edit_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut settings = state.store.load_settings().map_err(|e| e.to_string())?;
    let next_mode = if enabled {
        crate::store::AutoEditMode::Full
    } else {
        crate::store::AutoEditMode::Off
    };
    if settings.auto_edit_mode == next_mode {
        sync_tray_menus(&app);
        return Ok(());
    }

    settings.auto_edit_mode = next_mode;
    settings.auto_edit = next_mode.uses_llm();
    state
        .store
        .save_settings(&settings)
        .map_err(|e| e.to_string())?;
    state.pipeline.lock().set_auto_edit_mode(next_mode);

    if enabled {
        let app_clone = app.clone();
        tauri::async_runtime::spawn(async move {
            let state = app_clone.state::<AppState>();
            if let Err(err) = ensure_llm_model_inner(&app_clone, &state).await {
                eprintln!("llm init after tray auto-edit toggle failed: {err}");
            }
        });
    } else {
        shutdown_llm_engine(&state);
    }

    sync_tray_menus(&app);
    Ok(())
}

fn cycle_stt_language_from_tray(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let next = {
        let mut pipeline = state.pipeline.lock();
        if pipeline.state() == PipelineState::Recording {
            pipeline
                .cycle_session_language(&app)
                .map_err(|e| e.to_string())?
        } else {
            let next = pipeline.effective_stt_language().cycle();
            pipeline.set_default_stt_language(next);
            next
        }
    };

    if state.pipeline.lock().state() != PipelineState::Recording {
        let mut settings = state.store.load_settings().map_err(|e| e.to_string())?;
        settings.stt_language = next.as_setting_value();
        state
            .store
            .save_settings(&settings)
            .map_err(|e| e.to_string())?;
        state.pipeline.lock().notify_stt_language_changed(&app);
    }

    sync_tray_menus(&app);
    Ok(())
}

pub(crate) async fn ensure_model_inner(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let expected = state.perf_config.lock().whisper;
    if whisper_engine_matches(state, expected) {
        let _ = app.emit("model-ready", ());
        ensure_dictation_hotkey_registered(app, state)?;
        return Ok(());
    }

    let _init_guard = state.model_init.lock().await;

    if whisper_engine_matches(state, expected) {
        let _ = app.emit("model-ready", ());
        ensure_dictation_hotkey_registered(app, state)?;
        return Ok(());
    }

    if state.model_ready.load(Ordering::SeqCst) || state.whisper_engine.lock().is_some() {
        let was_live = invalidate_whisper_engine(state, true);
        emit_model_unready_if_needed(app, was_live);
    }

    suspend_global_hotkey(app, state)?;
    let _hotkey_guard = HotkeySuspendGuard {
        app,
        state,
        active: true,
    };

    let settings = state.store.load_settings().map_err(|e| e.to_string())?;
    let perf = *state.perf_config.lock();
    let whisper_model = perf.whisper;
    let use_gpu = inference::should_use_gpu(settings.inference_backend());
    let n_threads = perf.stt_threads;

    let app_for_download = app.clone();
    let model_path = tauri::async_runtime::spawn_blocking(move || {
        stt::ensure_model_blocking(Some(&app_for_download), whisper_model)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let engine = tauri::async_runtime::spawn_blocking(move || {
        stt::WhisperEngine::new_with_config(&model_path, use_gpu, n_threads)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    {
        *state.whisper_engine.lock() = Some(engine);
    }
    {
        let stt_language = state
            .store
            .load_settings()
            .map_err(|e| e.to_string())?
            .stt_language_mode();
        let mut pipeline = state.pipeline.lock();
        pipeline.set_whisper_engine(Arc::clone(&state.whisper_engine));
        pipeline.set_default_stt_language(stt_language);
    }

    state.model_ready.store(true, Ordering::SeqCst);
    *state.loaded_whisper.lock() = Some(whisper_model);
    *state.last_activity.lock() = Instant::now();
    let _ = app.emit("model-ready", ());
    let _ = app.emit(
        "pipeline-state",
        PipelineStateEvent {
            state: PipelineState::Idle.as_str().into(),
            message: None,
        },
    );
    state.models_ready_notifier.on_whisper_loaded(
        app,
        &state.store,
        settings.auto_edit,
        perf.preload_llm,
    );
    Ok(())
}

fn spawn_deferred_llm_if_needed(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        if !state.deferred_llm_on_boot.swap(false, Ordering::SeqCst) {
            return;
        }
        if !state.pipeline.lock().auto_edit_enabled() {
            return;
        }
        if let Err(err) = ensure_llm_model_inner(&app, &state).await {
            eprintln!("llm model initialization failed: {err}");
        }
    });
}

async fn ensure_model_then_start(app: AppHandle) {
    let state = app.state::<AppState>();
    if !whisper_is_live(&state) {
        let _ = app.emit(
            "pipeline-state",
            PipelineStateEvent {
                state: PipelineState::Idle.as_str().into(),
                message: Some("Chargement du modèle…".into()),
            },
        );
        if let Err(err) = ensure_model_inner(&app, &state).await {
            let _ = app.emit("model-init-error", err);
            clear_deferred_hotkey_start(app.state::<AppState>().inner());
            return;
        }
        spawn_deferred_llm_if_needed(app.clone());
    }

    let state = app.state::<AppState>();
    let should_start = take_deferred_hotkey_start(state.inner());
    let pipeline = state.pipeline.clone();
    if should_start && pipeline.lock().state() == PipelineState::Idle {
        request_dictation(app, pipeline, DictationIntent::Start);
    }
}

fn clear_deferred_hotkey_start(state: &AppState) {
    let mut press = state.hotkey_press.lock();
    press.deferred_start_pending = false;
    press.deferred_toggle_intent = false;
}

fn take_deferred_hotkey_start(state: &AppState) -> bool {
    let mut press = state.hotkey_press.lock();
    if !press.deferred_start_pending {
        return false;
    }
    let should_start =
        hotkey::should_start_after_deferred_load(press.shortcut_down, press.deferred_toggle_intent);
    press.deferred_start_pending = false;
    press.deferred_toggle_intent = false;
    should_start
}

pub(crate) async fn ensure_model_then_toggle(app: AppHandle) {
    let state = app.state::<AppState>();
    if is_mic_probe_active(&state) {
        return;
    }
    let pipeline_state = state.pipeline.lock().state();
    if is_dictation_start_blocked_by_settings(&state, pipeline_state) {
        notify_dictation_blocked_by_settings(&app, &state);
        return;
    }
    if !whisper_is_live(&state) {
        let _ = app.emit(
            "pipeline-state",
            PipelineStateEvent {
                state: PipelineState::Idle.as_str().into(),
                message: Some("Chargement du modèle…".into()),
            },
        );
        if let Err(err) = ensure_model_inner(&app, &state).await {
            let _ = app.emit("model-init-error", err);
            return;
        }
        spawn_deferred_llm_if_needed(app.clone());
    }

    let pipeline = app.state::<AppState>().pipeline.clone();
    request_dictation(app, pipeline, DictationIntent::Toggle);
}

#[cfg(windows)]
pub(crate) fn dispatch_dictation_hotkey(app: &AppHandle, shortcut_state: ShortcutState) {
    handle_hotkey(app, shortcut_state);
}

fn hotkey_press_snapshot(press: &HotkeyPressState) -> hotkey::HotkeyPressSnapshot {
    hotkey::HotkeyPressSnapshot {
        press_start: press.press_start,
        was_idle_on_press: press.was_idle_on_press,
        shortcut_down: press.shortcut_down,
        deferred_start_pending: press.deferred_start_pending,
        deferred_toggle_intent: press.deferred_toggle_intent,
        busy_cancel_on_release: press.busy_cancel_on_release,
    }
}

fn handle_hotkey(app: &AppHandle, shortcut_state: ShortcutState) {
    let state = app.state::<AppState>();
    let event = match shortcut_state {
        ShortcutState::Pressed => hotkey::HotkeyEvent::Pressed,
        ShortcutState::Released => hotkey::HotkeyEvent::Released,
    };

    let suspended = state.hotkey_suspend_depth.load(Ordering::SeqCst) > 0;
    let pipeline_state = state.pipeline.lock().state();
    let ctx = hotkey::HotkeyDecisionContext {
        pipeline_state,
        whisper_live: whisper_is_live(&state),
        dictation_blocked: is_dictation_blocked_by_settings(&state),
        mic_probe_active: is_mic_probe_active(&state),
        hotkey_suspended: suspended,
    };

    let mut press = state.hotkey_press.lock();
    let snapshot = hotkey_press_snapshot(&press);
    let action = hotkey::decide_action(&ctx, &snapshot, event);

    match event {
        hotkey::HotkeyEvent::Pressed => {
            if matches!(action, hotkey::HotkeyAction::Ignore) {
                return;
            }
            if press.shortcut_down {
                return;
            }

            match action {
                hotkey::HotkeyAction::NotifyBlocked => {
                    notify_dictation_blocked_by_settings(app, &state);
                }
                hotkey::HotkeyAction::StartRecording => {
                    press.shortcut_down = true;
                    press.press_start = Some(Instant::now());
                    press.was_idle_on_press = pipeline_state == PipelineState::Idle;
                    if whisper_is_live(&state) {
                        press.deferred_start_pending = false;
                        press.deferred_toggle_intent = false;
                        let pipeline = state.pipeline.clone();
                        drop(press);
                        request_dictation(app.clone(), pipeline, DictationIntent::Start);
                    } else {
                        press.deferred_start_pending = true;
                        press.deferred_toggle_intent = false;
                        let app_clone = app.clone();
                        drop(press);
                        tauri::async_runtime::spawn(async move {
                            ensure_model_then_start(app_clone).await;
                        });
                    }
                }
                hotkey::HotkeyAction::StopRecording => {
                    press.shortcut_down = true;
                    press.press_start = Some(Instant::now());
                    press.was_idle_on_press = pipeline_state == PipelineState::Idle;
                    let pipeline = state.pipeline.clone();
                    drop(press);
                    request_dictation(app.clone(), pipeline, DictationIntent::Stop);
                }
                hotkey::HotkeyAction::EmitBusy { cancelable } => {
                    press.shortcut_down = true;
                    press.press_start = Some(Instant::now());
                    press.was_idle_on_press = pipeline_state == PipelineState::Idle;
                    press.busy_cancel_on_release = cancelable;
                    let busy_state = pipeline_state;
                    drop(press);
                    pipeline::emit_dictation_busy(app, busy_state, cancelable);
                }
                _ => {}
            }
        }
        hotkey::HotkeyEvent::Released => match action {
            hotkey::HotkeyAction::ResetPressState => {
                press.shortcut_down = false;
                press.press_start = None;
                press.busy_cancel_on_release = false;
            }
            hotkey::HotkeyAction::Ignore => {
                if !press.shortcut_down {
                    return;
                }
                let cancel_busy = press.busy_cancel_on_release;
                let had_start = press.press_start.take().is_some();
                press.shortcut_down = false;
                press.busy_cancel_on_release = false;
                if !had_start && cancel_busy && pipeline_state == PipelineState::Transcribing {
                    let pipeline = state.pipeline.clone();
                    drop(press);
                    pipeline::request_dictation(app.clone(), pipeline, DictationIntent::Cancel);
                }
            }
            hotkey::HotkeyAction::CaptureDeferredToggleIntent { duration } => {
                if !press.shortcut_down {
                    return;
                }
                press.shortcut_down = false;
                press.busy_cancel_on_release = false;
                press.press_start.take();
                if press.deferred_start_pending {
                    press.deferred_toggle_intent =
                        hotkey::is_toggle_tap(press.was_idle_on_press, duration);
                }
            }
            hotkey::HotkeyAction::CancelTranscribing => {
                press.shortcut_down = false;
                press.busy_cancel_on_release = false;
                press.press_start.take();
                let pipeline = state.pipeline.clone();
                drop(press);
                pipeline::request_dictation(app.clone(), pipeline, DictationIntent::Cancel);
            }
            hotkey::HotkeyAction::StopRecording => {
                press.shortcut_down = false;
                press.busy_cancel_on_release = false;
                press.press_start.take();
                let pipeline = state.pipeline.clone();
                drop(press);
                request_dictation(app.clone(), pipeline, DictationIntent::Stop);
            }
            _ => {
                if press.shortcut_down {
                    press.shortcut_down = false;
                    press.busy_cancel_on_release = false;
                    press.press_start.take();
                }
            }
        },
    }
}

pub(crate) fn should_start_minimized() -> bool {
    std::env::args().any(|arg| arg == "--minimized")
}

fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

fn show_main_window(app: &AppHandle) {
    show_main_window_view(app, None);
}

fn show_main_window_view(app: &AppHandle, view: Option<&str>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.maximize();
        let _ = window.set_focus();
    }
    if let Some(view) = view {
        let _ = app.emit(
            "navigate-view",
            NavigateViewPayload {
                view: view.to_string(),
            },
        );
    }
}

fn tray_language_menu_text(app: &AppHandle) -> String {
    let ui_language = current_ui_language(app);
    let label = if let Some(state) = app.try_state::<AppState>() {
        state
            .pipeline
            .lock()
            .effective_stt_language()
            .display_label()
    } else {
        "FR"
    };
    ui::locale::tr_with_vars("tray.dictationLanguage", &ui_language, &[("label", label)])
}

pub(crate) fn sync_tray_menus(app: &AppHandle) {
    if let Some(handles) = app.try_state::<TrayHandles>() {
        let ui_language = current_ui_language(app);
        let _ = handles
            .open_item
            .set_text(ui::locale::tr("tray.open", &ui_language));
        let _ = handles
            .toggle_item
            .set_text(ui::locale::tr("tray.toggle", &ui_language));
        let _ = handles
            .settings_item
            .set_text(ui::locale::tr("tray.settings", &ui_language));
        let _ = handles
            .history_item
            .set_text(ui::locale::tr("tray.history", &ui_language));
        let _ = handles
            .dictionary_item
            .set_text(ui::locale::tr("tray.dictionary", &ui_language));
        let _ = handles
            .snippets_item
            .set_text(ui::locale::tr("tray.snippets", &ui_language));
        let _ = handles
            .insight_item
            .set_text(ui::locale::tr("tray.insight", &ui_language));
        let _ = handles
            .quit_item
            .set_text(ui::locale::tr("tray.quit", &ui_language));
        let _ = handles
            .auto_edit_item
            .set_text(ui::locale::tr("tray.autoEdit", &ui_language));
        let _ = handles
            .autostart_item
            .set_text(ui::locale::tr("tray.autostart", &ui_language));

        let autostart_enabled = autostart_enabled_from_store(app);
        let _ = handles.autostart_item.set_checked(autostart_enabled);

        let auto_edit_enabled = app
            .try_state::<AppState>()
            .map(|state| state.pipeline.lock().auto_edit_enabled())
            .unwrap_or(false);
        let _ = handles.auto_edit_item.set_checked(auto_edit_enabled);
        let _ = handles.language_item.set_text(tray_language_menu_text(app));
    }
}

fn build_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let ui_language = current_ui_language(app);
    let open_item = MenuItem::with_id(
        app,
        MENU_OPEN,
        ui::locale::tr("tray.open", &ui_language),
        true,
        None::<&str>,
    )?;
    let toggle_item = MenuItem::with_id(
        app,
        MENU_TOGGLE,
        ui::locale::tr("tray.toggle", &ui_language),
        true,
        None::<&str>,
    )?;
    let language_item = MenuItem::with_id(
        app,
        MENU_CYCLE_LANGUAGE,
        tray_language_menu_text(app),
        true,
        None::<&str>,
    )?;
    let settings_item = MenuItem::with_id(
        app,
        MENU_OPEN_SETTINGS,
        ui::locale::tr("tray.settings", &ui_language),
        true,
        None::<&str>,
    )?;
    let history_item = MenuItem::with_id(
        app,
        MENU_OPEN_HISTORY,
        ui::locale::tr("tray.history", &ui_language),
        true,
        None::<&str>,
    )?;
    let dictionary_item = MenuItem::with_id(
        app,
        MENU_OPEN_DICTIONARY,
        ui::locale::tr("tray.dictionary", &ui_language),
        true,
        None::<&str>,
    )?;
    let snippets_item = MenuItem::with_id(
        app,
        MENU_OPEN_SNIPPETS,
        ui::locale::tr("tray.snippets", &ui_language),
        true,
        None::<&str>,
    )?;
    let insight_item = MenuItem::with_id(
        app,
        MENU_OPEN_INSIGHT,
        ui::locale::tr("tray.insight", &ui_language),
        true,
        None::<&str>,
    )?;
    let auto_edit_checked = app
        .try_state::<AppState>()
        .map(|state| state.pipeline.lock().auto_edit_enabled())
        .unwrap_or(false);
    let auto_edit_item = CheckMenuItem::with_id(
        app,
        MENU_AUTO_EDIT,
        ui::locale::tr("tray.autoEdit", &ui_language),
        true,
        auto_edit_checked,
        None::<&str>,
    )?;
    let autostart_checked = autostart_enabled_from_store(app);
    let autostart_item = CheckMenuItem::with_id(
        app,
        MENU_AUTOSTART,
        ui::locale::tr("tray.autostart", &ui_language),
        true,
        autostart_checked,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(
        app,
        MENU_QUIT,
        ui::locale::tr("tray.quit", &ui_language),
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &open_item,
            &toggle_item,
            &language_item,
            &separator,
            &settings_item,
            &history_item,
            &dictionary_item,
            &snippets_item,
            &insight_item,
            &separator,
            &auto_edit_item,
            &autostart_item,
            &separator,
            &quit_item,
        ],
    )?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Calliop")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_OPEN => show_main_window(app),
            MENU_TOGGLE => {
                let app_clone = app.clone();
                tauri::async_runtime::spawn(async move {
                    ensure_model_then_toggle(app_clone).await;
                });
            }
            MENU_CYCLE_LANGUAGE => {
                let _ = cycle_stt_language_from_tray(app.clone());
            }
            MENU_OPEN_SETTINGS => show_main_window_view(app, Some("settings")),
            MENU_OPEN_HISTORY => show_main_window_view(app, Some("history")),
            MENU_OPEN_DICTIONARY => show_main_window_view(app, Some("dictionary")),
            MENU_OPEN_SNIPPETS => show_main_window_view(app, Some("snippets")),
            MENU_OPEN_INSIGHT => show_main_window_view(app, Some("insight")),
            MENU_AUTO_EDIT => {
                let enabled = app
                    .try_state::<AppState>()
                    .map(|state| state.pipeline.lock().auto_edit_enabled())
                    .unwrap_or(false);
                let _ = set_auto_edit_enabled(app.clone(), !enabled);
            }
            MENU_AUTOSTART => {
                let enabled = autostart_enabled_from_store(app);
                let _ = commands::set_autostart_enabled(app.clone(), !enabled);
            }
            MENU_QUIT => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    app.manage(TrayHandles {
        open_item,
        toggle_item,
        language_item,
        settings_item,
        history_item,
        dictionary_item,
        snippets_item,
        insight_item,
        auto_edit_item,
        autostart_item,
        quit_item,
    });

    Ok(())
}

pub(crate) async fn fetch_available_update(
    app: &AppHandle,
    ignore_dismissed: bool,
) -> Result<Option<tauri_plugin_updater::Update>, String> {
    let updater = app
        .updater()
        .map_err(|err| format!("Updater unavailable: {err}"))?;

    match updater.check().await {
        Ok(Some(update)) => {
            if !ignore_dismissed
                && update::dismissed_update_version().as_deref() == Some(update.version.as_str())
            {
                return Ok(None);
            }
            Ok(Some(update))
        }
        Ok(None) => Ok(None),
        Err(err) => Err(format!("Update check failed: {err}")),
    }
}

async fn download_and_store_update(
    app: &AppHandle,
    update: tauri_plugin_updater::Update,
    pending_update: update::PendingUpdateStore,
) -> Result<(), String> {
    let version = update.version.clone();
    let downloaded_acc = Arc::new(AtomicU64::new(0));
    let app_for_progress = app.clone();
    let version_for_progress = version.clone();
    let downloaded_for_progress = Arc::clone(&downloaded_acc);

    let bytes = update
        .download(
            move |chunk_len, total| {
                let current = downloaded_for_progress
                    .fetch_add(chunk_len as u64, Ordering::Relaxed)
                    + chunk_len as u64;
                let percent = total
                    .map(|total_bytes| (current as f32 / total_bytes as f32) * 100.0)
                    .unwrap_or(0.0);
                let _ = app_for_progress.emit(
                    "update-download-progress",
                    update::UpdateDownloadProgress {
                        version: version_for_progress.clone(),
                        downloaded: current,
                        total,
                        percent,
                    },
                );
            },
            || {},
        )
        .await
        .map_err(|err| format!("Update download failed: {err}"))?;

    let bytes_path = update::store_pending_update_bytes(&bytes)?;
    *pending_update.lock() = Some(update::PendingUpdate {
        version: version.clone(),
        update,
        bytes_path,
    });
    let version_for_notify = version.clone();
    let _ = app.emit("update-ready", update::UpdateReadyPayload { version });
    let state = app.state::<AppState>();
    notify_update_ready(app, &state.store, &version_for_notify);
    Ok(())
}

pub(crate) async fn run_update_download(
    app: AppHandle,
    _store: Arc<Store>,
    update: tauri_plugin_updater::Update,
) {
    let pipeline = Arc::clone(&app.state::<AppState>().pipeline);
    wait_for_pipeline_idle(&pipeline).await;

    let state = app.state::<AppState>();
    let pending_update = Arc::clone(&state.pending_update);
    if let Err(err) = download_and_store_update(&app, update, pending_update).await {
        eprintln!("auto-update: {err}");
        let _ = app.emit(
            "update-check-failed",
            update::UpdateCheckFailedPayload { message: err },
        );
    }
    app.state::<AppState>()
        .update_check_in_progress
        .store(false, Ordering::SeqCst);
}

fn spawn_update_check_if_enabled(app: AppHandle, store: Arc<Store>) {
    tauri::async_runtime::spawn(async move {
        if cfg!(debug_assertions) {
            return;
        }

        let settings = match store.load_settings() {
            Ok(settings) => settings,
            Err(err) => {
                eprintln!("auto-update: failed to load settings: {err}");
                return;
            }
        };
        if !settings.auto_update {
            return;
        }
        if !store.is_onboarding_done().unwrap_or(false) {
            return;
        }

        let update = match fetch_available_update(&app, false).await {
            Ok(Some(update)) => update,
            Ok(None) => return,
            Err(err) => {
                eprintln!("auto-update: {err}");
                return;
            }
        };

        let state = app.state::<AppState>();
        if state.update_check_in_progress.swap(true, Ordering::SeqCst) {
            return;
        }

        run_update_download(app, store, update).await;
    });
}

pub(crate) async fn wait_for_pipeline_idle(pipeline: &Arc<Mutex<PipelineOrchestrator>>) {
    loop {
        if pipeline.lock().state() == PipelineState::Idle {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let pipeline = Arc::new(Mutex::new(
        PipelineOrchestrator::new().expect("failed to initialize pipeline orchestrator"),
    ));
    let store = Arc::new(Store::open().expect("failed to open settings store"));
    let mut initial_settings = store
        .load_settings()
        .expect("failed to load initial settings");

    if initial_settings.whisper_model == "medium" {
        initial_settings.whisper_model = WhisperModel::DistilFrDec16.as_setting_value().into();
        if let Err(err) = store.save_settings(&initial_settings) {
            eprintln!("failed to migrate whisper_model from medium: {err}");
        }
    }

    if initial_settings.whisper_model == "small" {
        initial_settings.whisper_model = WhisperModel::DistilFrV02.as_setting_value().into();
        if let Err(err) = store.save_settings(&initial_settings) {
            eprintln!("failed to migrate whisper_model from small: {err}");
        }
    }

    if initial_settings.llm_model == "qwen3-4b" {
        initial_settings.llm_model = llm::LlmModel::Qwen3_5_4B.as_setting_value().into();
        if let Err(err) = store.save_settings(&initial_settings) {
            eprintln!("failed to migrate llm_model from qwen3-4b: {err}");
        }
    }

    if initial_settings.llm_model == "qwen3-0.6b" {
        initial_settings.llm_model = llm::LlmModel::Qwen3_5_0_8B.as_setting_value().into();
        if let Err(err) = store.save_settings(&initial_settings) {
            eprintln!("failed to migrate llm_model from qwen3-0.6b: {err}");
        }
    }

    if initial_settings.llm_model == "qwen3-1.7b" {
        initial_settings.llm_model = llm::LlmModel::Qwen3_5_2B.as_setting_value().into();
        if let Err(err) = store.save_settings(&initial_settings) {
            eprintln!("failed to migrate llm_model from qwen3-1.7b: {err}");
        }
    }

    if let Err(err) = stt::remove_legacy_medium_model() {
        eprintln!("failed to remove legacy whisper medium model: {err}");
    }

    if let Err(err) = stt::remove_legacy_small_model() {
        eprintln!("failed to remove legacy whisper small model: {err}");
    }

    if let Err(err) = llm::remove_legacy_qwen3_4b_model() {
        eprintln!("failed to remove legacy qwen3 4b model: {err}");
    }

    if let Err(err) = llm::remove_legacy_qwen3_0_6b_model() {
        eprintln!("failed to remove legacy qwen3 0.6b model: {err}");
    }

    if let Err(err) = llm::remove_legacy_qwen3_1_7b_model() {
        eprintln!("failed to remove legacy qwen3 1.7b model: {err}");
    }

    if !store.has_setting(KEY_AUTOSTART).unwrap_or(false)
        && !store.is_onboarding_done().unwrap_or(false)
    {
        if let Err(err) = store.set_autostart(true) {
            eprintln!("failed to seed autostart default: {err}");
        }
    }

    match store.apply_auto_edit_default() {
        Ok(enabled) => {
            initial_settings.auto_edit = enabled;
            if enabled {
                initial_settings.auto_edit_mode = crate::store::AutoEditMode::Full;
            }
        }
        Err(err) => eprintln!("failed to apply auto_edit default: {err}"),
    }

    pipeline
        .lock()
        .set_auto_edit_mode(initial_settings.auto_edit_mode);
    pipeline
        .lock()
        .set_pause_preset(initial_settings.pause_preset);
    pipeline.lock().set_auto_learn(initial_settings.auto_learn);
    pipeline
        .lock()
        .set_default_stt_language(initial_settings.stt_language_mode());
    pipeline
        .lock()
        .set_input_device(initial_settings.input_device.clone());

    {
        let handler = Arc::new(move |app: &AppHandle, original: &str, corrected: &str| {
            let state = app.state::<AppState>();
            if !state.pipeline.lock().auto_learn_enabled() {
                return;
            }
            if let Err(err) = apply_learned_correction(app, &state, original, corrected) {
                eprintln!("auto-learn correction failed: {err}");
            }
        });
        pipeline.lock().set_correction_handler(handler);
    }

    if let Err(err) = store.seed_default_app_context_rules() {
        eprintln!("failed to seed default app context rules: {err}");
    }
    if let Err(err) = refresh_app_context_rules(&store, &pipeline) {
        eprintln!("failed to load app context rules cache: {err}");
    }

    let prompt_cache = Mutex::new(WhisperPromptCache::default());
    let dictionary_notifier = Arc::new(DictionaryNotifier::new());
    if let Err(err) = refresh_whisper_prompt_with_cache(&store, &pipeline, &prompt_cache) {
        eprintln!("failed to load whisper prompt and snippets cache: {err}");
        ensure_pipeline_snippets_loaded(&store, &pipeline);
    }
    if let Err(err) = refresh_correction_rules(&store, &pipeline) {
        eprintln!("failed to load dictionary correction rules cache: {err}");
    }
    pipeline.lock().set_history_store(Arc::clone(&store));
    let achievements = Arc::new(achievements::AchievementEngine::new(Arc::clone(&store)));
    pipeline
        .lock()
        .set_achievement_engine(Arc::clone(&achievements));

    let whisper_engine = Arc::new(Mutex::new(None));
    let llm_engine = Arc::new(Mutex::new(None));
    let llm_ready = Arc::new(AtomicBool::new(false));
    let model_init = Arc::new(tokio::sync::Mutex::new(()));
    let llm_init = Arc::new(tokio::sync::Mutex::new(()));
    let start_minimized = should_start_minimized();
    let capabilities = SystemCapabilities::detect();
    let initial_perf = resolve_perf_config(&initial_settings, &capabilities, start_minimized);
    pipeline
        .lock()
        .set_vad_chunk_size(initial_perf.vad_chunk_size);
    pipeline
        .lock()
        .set_whisper_engine(Arc::clone(&whisper_engine));
    pipeline.lock().set_llm_engine(Arc::clone(&llm_engine));
    pipeline.lock().set_llm_ready(Arc::clone(&llm_ready));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    handle_hotkey(app, event.state());
                })
                .build(),
        )
        .manage(AppState {
            pipeline: pipeline.clone(),
            whisper_engine: whisper_engine.clone(),
            llm_engine: llm_engine.clone(),
            store,
            achievements,
            prompt_cache,
            dictionary_notifier,
            models_ready_notifier: Arc::new(ModelsReadyNotifier::new()),
            capabilities,
            perf_config: Mutex::new(initial_perf),
            last_activity: Mutex::new(Instant::now()),
            model_ready: AtomicBool::new(false),
            model_init: Arc::clone(&model_init),
            llm_ready: llm_ready.clone(),
            llm_init: Arc::clone(&llm_init),
            hotkey_press: Mutex::new(HotkeyPressState::default()),
            deferred_llm_on_boot: AtomicBool::new(
                initial_settings.auto_edit && initial_perf.llm_lazy_load,
            ),
            current_hotkey: Mutex::new(hotkey::HotkeyBinding::KeyCombo(hotkey::default_shortcut())),
            hotkey_suspend_depth: AtomicU32::new(0),
            whisper_settings_change_depth: AtomicU32::new(0),
            loaded_whisper: Mutex::new(None),
            loaded_llm: Mutex::new(None),
            mic_probe: MicProbeState {
                capture: Mutex::new(None),
                level_task: Mutex::new(None),
            },
            pending_update: Arc::new(Mutex::new(None)),
            update_check_in_progress: AtomicBool::new(false),
        })
        .setup(move |app| {
            let _modules = registered_modules();

            {
                let app_for_update = app.handle().clone();
                let state = app.state::<AppState>();
                spawn_update_check_if_enabled(app_for_update, Arc::clone(&state.store));
            }

            sync_autostart_from_settings(app.handle());
            build_tray(app.handle()).map_err(|e| e.to_string())?;
            sync_tray_menus(app.handle());
            let _ = app_context::get_active_window();

            {
                let state = app.state::<AppState>();
                if let Err(err) = state.achievements.retroactive_scan(app.handle()) {
                    eprintln!("achievement retroactive scan failed: {err}");
                }
            }

            let preload_whisper = initial_perf.preload_whisper;
            let preload_llm = initial_perf.preload_llm && initial_settings.auto_edit;

            if !preload_whisper {
                let state = app.state::<AppState>();
                if let Err(err) = register_hotkey_from_settings(app.handle(), state.inner()) {
                    eprintln!("failed to register dictation hotkey: {err}");
                }
            }

            if let Some(overlay) = app.get_webview_window("overlay") {
                let _ = overlay.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));
            }

            let show_after_update = update::take_show_after_update_flag();

            if let Some(main) = app.get_webview_window("main") {
                let _ = main.set_theme(Some(Theme::Dark));
                let app_handle = app.handle().clone();
                main.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                });

                if show_after_update {
                    let _ = main.show();
                    let _ = main.unminimize();
                    let _ = main.maximize();
                    let _ = main.set_focus();
                } else if !should_start_minimized() {
                    let _ = main.maximize();
                }
            }

            if should_start_minimized() && !show_after_update {
                hide_main_window(app.handle());
            }

            let app_for_watchdog = app.handle().clone();
            spawn_whisper_idle_watchdog(app_for_watchdog);

            if preload_whisper {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<AppState>();
                    if let Err(err) = commands::ensure_model(app_handle.clone(), state).await {
                        eprintln!("model initialization failed: {err}");
                        let _ = app_handle.emit("model-init-error", err);
                    }
                });
            }

            if preload_llm {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<AppState>();
                    if let Err(err) = commands::ensure_llm_model(app_handle.clone(), state).await {
                        eprintln!("llm model initialization failed: {err}");
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_pipeline_state,
            commands::is_model_ready,
            commands::toggle_dictation,
            commands::ensure_model,
            commands::ensure_llm_model,
            commands::get_settings,
            commands::set_settings,
            commands::get_models_status,
            commands::delete_model,
            commands::reinstall_model,
            commands::get_inference_info,
            commands::set_hotkey,
            commands::set_hotkey_capture_active,
            commands::is_onboarding_done,
            commands::set_onboarding_done,
            commands::list_input_devices,
            commands::start_mic_probe,
            commands::stop_mic_probe,
            commands::prepare_onboarding_dictation,
            commands::get_stt_language,
            commands::cycle_dictation_language,
            commands::is_autostart_enabled,
            commands::set_autostart_enabled,
            commands::get_pending_update_version,
            commands::check_for_updates,
            commands::install_pending_update,
            commands::dismiss_pending_update,
            commands::list_dictionary_words,
            commands::add_dictionary_word,
            commands::update_dictionary_word,
            commands::remove_dictionary_word,
            commands::learn_from_correction,
            commands::list_snippets,
            commands::add_snippet,
            commands::update_snippet,
            commands::remove_snippet,
            commands::import_snippets,
            commands::export_snippets,
            commands::get_snippet_user_name,
            commands::set_snippet_user_name,
            commands::preview_snippet_expansion,
            commands::get_active_window,
            commands::list_app_context_rules,
            commands::add_app_context_rule,
            commands::remove_app_context_rule,
            commands::list_dictations,
            commands::search_dictations,
            commands::count_dictations,
            commands::count_search_dictations,
            commands::copy_dictation,
            commands::reinject_dictation,
            commands::get_insights,
            commands::get_achievements,
            commands::mark_achievements_seen
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app, event| {
            if let RunEvent::Ready = event {
                sync_tray_menus(app);
            }
        });
}

#[cfg(test)]
mod integration_tests {
    use super::registered_modules;

    #[test]
    fn all_modules_are_wired() {
        assert_eq!(
            registered_modules(),
            [
                "audio",
                "stt",
                "llm",
                "inference",
                "inject",
                "hotkey",
                "store",
                "observe",
                "app_context",
                "pipeline",
                "system",
            ]
        );
    }

    #[test]
    fn loaded_whisper_identity_requires_matching_model() {
        use crate::stt::WhisperModel;

        let loaded = Some(WhisperModel::DistilFrV02);
        let expected = WhisperModel::DistilFrDec16;
        assert_ne!(loaded, Some(expected));
        assert_eq!(loaded, Some(WhisperModel::DistilFrV02));
    }

    #[test]
    fn low_power_enables_llm_lazy_load() {
        use crate::store::AppSettings;
        use crate::system::{resolve_perf_config, SystemCapabilities};

        let settings = AppSettings {
            low_power_mode: true,
            auto_edit: true,
            ..AppSettings::default()
        };
        let caps = SystemCapabilities {
            total_ram_bytes: 16 * 1024 * 1024 * 1024,
            avail_ram_bytes: 8 * 1024 * 1024 * 1024,
            cpu_logical_cores: 8,
            gpu_compiled: false,
        };
        let perf = resolve_perf_config(&settings, &caps, false);
        assert!(perf.llm_lazy_load);
        assert!(!perf.preload_llm);
    }

    #[test]
    fn whisper_settings_guard_raii() {
        use std::sync::atomic::{AtomicU32, Ordering};

        use super::WhisperSettingsChangeGuard;

        let depth = AtomicU32::new(0);
        {
            let _guard = WhisperSettingsChangeGuard::acquire(&depth);
            assert_eq!(depth.load(Ordering::SeqCst), 1);
        }
        assert_eq!(depth.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn dictation_start_blocked_only_when_idle_and_settings_busy() {
        use crate::pipeline::PipelineState;

        let depth = 0_u32;
        assert!(
            !(PipelineState::Idle == PipelineState::Idle && depth > 0),
            "idle with no settings change should not block"
        );
        assert!(
            !(PipelineState::Recording == PipelineState::Idle && depth > 0),
            "recording should not block even without settings change"
        );

        let depth = 1_u32;
        assert!(
            PipelineState::Idle == PipelineState::Idle && depth > 0,
            "idle during settings change should block start"
        );
        assert!(
            !(PipelineState::Recording == PipelineState::Idle && depth > 0),
            "recording during settings change should still allow stop"
        );
    }

    #[test]
    fn hotkey_busy_cancel_only_during_transcribing() {
        use crate::pipeline::PipelineState;

        assert!(PipelineState::Transcribing.hotkey_cancelable());
        assert!(!PipelineState::Injecting.hotkey_cancelable());
    }
}
