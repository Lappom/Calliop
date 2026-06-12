pub mod app_context;
pub mod audio;
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
pub mod ui;
pub mod update;
pub mod user_error;

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use calliop_prompt::ToneProfile;
use dictionary_notify::{DictionaryNotifier, DictionaryUpdatedPayload};
use inject::TextInjector;
use parking_lot::Mutex;
use pipeline::{
    expand_snippet_variables, spawn_start, spawn_stop, spawn_toggle, CorrectionRule,
    PipelineOrchestrator, PipelineState, PipelineStateEvent, SnippetVariableContext,
};
use serde::{Deserialize, Serialize};
use store::{
    extract_correction_words, is_valid_app_context_pattern, is_valid_dictionary_word,
    is_valid_snippet_content, is_valid_trigger, normalize_trigger, normalize_word,
    AppContextMatchType, AppContextRule, AppSettings, DictationEntry, DictionarySource,
    DictionaryWord, InferenceBackend, Insights, NewAppContextRule, Snippet, SnippetImport, Store,
    DEFAULT_LIST_LIMIT,
};
use stt::{SttLanguage, WhisperModel, WhisperPromptCache, MAX_INITIAL_PROMPT_WORDS};
use system::{resolve_perf_config, RuntimePerfConfig, SystemCapabilities};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, RunEvent, State, Theme, WindowEvent,
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

struct MicProbeState {
    capture: Mutex<Option<audio::AudioCapture>>,
    level_task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}

struct AppState {
    pipeline: Arc<Mutex<PipelineOrchestrator>>,
    whisper_engine: Arc<Mutex<Option<stt::WhisperEngine>>>,
    llm_engine: Arc<Mutex<Option<llm::LlamaEngine>>>,
    store: Arc<Store>,
    prompt_cache: Mutex<WhisperPromptCache>,
    dictionary_notifier: Arc<DictionaryNotifier>,
    capabilities: SystemCapabilities,
    perf_config: Mutex<RuntimePerfConfig>,
    last_activity: Mutex<Instant>,
    model_ready: AtomicBool,
    model_init: Arc<tokio::sync::Mutex<()>>,
    llm_ready: Arc<AtomicBool>,
    llm_init: Arc<tokio::sync::Mutex<()>>,
    hotkey_press: Mutex<HotkeyPressState>,
    deferred_llm_on_boot: AtomicBool,
    current_hotkey: Mutex<hotkey::HotkeyBinding>,
    hotkey_suspend_depth: AtomicU32,
    whisper_settings_change_depth: AtomicU32,
    loaded_whisper: Mutex<Option<WhisperModel>>,
    loaded_llm: Mutex<Option<llm::LlmModel>>,
    mic_probe: MicProbeState,
    pending_update: update::PendingUpdateStore,
}

/// Held while `set_settings` downloads or reloads the active Whisper model.
struct WhisperSettingsChangeGuard<'a> {
    depth: &'a AtomicU32,
}

impl<'a> WhisperSettingsChangeGuard<'a> {
    fn new(state: &'a AppState) -> Self {
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

fn is_dictation_start_blocked_by_settings(state: &AppState, pipeline_state: PipelineState) -> bool {
    pipeline_state == PipelineState::Idle && is_dictation_blocked_by_settings(state)
}

fn notify_dictation_blocked_by_settings(app: &AppHandle, state: &AppState) {
    clear_deferred_hotkey_start(state);
    let _ = app.emit(
        "dictation-blocked",
        DictationBlockedPayload {
            reason: "WHISPER_SETTINGS_CHANGE".into(),
        },
    );
}

fn llm_engine_is_live(state: &AppState) -> bool {
    state.llm_ready.load(Ordering::SeqCst) && state.llm_engine.lock().is_some()
}

fn shutdown_llm_engine(state: &AppState) {
    state.llm_ready.store(false, Ordering::SeqCst);
    *state.llm_engine.lock() = None;
    *state.loaded_llm.lock() = None;
}

fn refresh_perf_config(state: &AppState, settings: &AppSettings, start_minimized: bool) {
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
            invalidate_whisper_engine(&state);
            let _ = app.emit("model-unready", ());
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
struct SettingsPayload {
    auto_edit: bool,
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
struct ModelStatusEntry {
    id: String,
    label: String,
    installed: bool,
    size_bytes: Option<u64>,
    active: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ModelsStatusPayload {
    whisper: Vec<ModelStatusEntry>,
    llm: Vec<ModelStatusEntry>,
}

fn settings_to_payload(settings: &AppSettings) -> SettingsPayload {
    SettingsPayload {
        auto_edit: settings.auto_edit,
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

fn parse_ui_language(value: &str) -> String {
    if value.trim() == "en" {
        "en".into()
    } else {
        "fr".into()
    }
}

fn file_size_bytes(path: &std::path::Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|meta| meta.len())
}

fn build_models_status(state: &AppState, settings: &AppSettings) -> ModelsStatusPayload {
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

fn invalidate_whisper_engine(state: &AppState) {
    state.model_ready.store(false, Ordering::SeqCst);
    *state.whisper_engine.lock() = None;
    *state.loaded_whisper.lock() = None;
}

fn whisper_engine_matches(state: &AppState, expected: WhisperModel) -> bool {
    state.model_ready.load(Ordering::SeqCst)
        && state.whisper_engine.lock().is_some()
        && *state.loaded_whisper.lock() == Some(expected)
}

fn llm_engine_matches(state: &AppState, expected: llm::LlmModel) -> bool {
    llm_engine_is_live(state) && *state.loaded_llm.lock() == Some(expected)
}

fn llm_engine_stale(state: &AppState, expected: llm::LlmModel) -> bool {
    match *state.loaded_llm.lock() {
        Some(loaded) => loaded != expected,
        None => llm_engine_is_live(state),
    }
}

async fn ensure_whisper_model_file(app: &AppHandle, model: WhisperModel) -> Result<(), String> {
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

async fn ensure_llm_model_file(app: &AppHandle, model: llm::LlmModel) -> Result<(), String> {
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

struct SettingsRollbackContext {
    hotkey_changed: bool,
    stt_language_changed: bool,
    whisper_invalidated: bool,
}

async fn rollback_settings(
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
    state.pipeline.lock().set_auto_edit(previous.auto_edit);

    if ctx.whisper_invalidated {
        invalidate_whisper_engine(state);
        ensure_model_inner(app, state).await?;
    }

    if previous.auto_edit {
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

fn suspend_global_hotkey(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let prev = state.hotkey_suspend_depth.fetch_add(1, Ordering::SeqCst);
    if prev == 0 {
        unregister_current_hotkey(app, state)?;
    }
    Ok(())
}

fn resume_global_hotkey(app: &AppHandle, state: &AppState) -> Result<(), String> {
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

fn is_mic_probe_active(state: &AppState) -> bool {
    state.mic_probe.capture.lock().is_some()
}

fn register_hotkey_binding(app: &AppHandle, binding: hotkey::HotkeyBinding) -> Result<(), String> {
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
fn parse_stt_language(value: &str) -> Result<SttLanguage, String> {
    SttLanguage::parse(value).ok_or_else(|| user_error_string(UserError::UnsupportedSttLanguage))
}

#[derive(Debug, Clone, Serialize)]
struct DictionaryWordPayload {
    id: i64,
    word: String,
    source: String,
    misspelling: Option<String>,
    created_at: String,
}

fn dictionary_word_to_payload(word: DictionaryWord) -> DictionaryWordPayload {
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
struct SnippetPayload {
    id: i64,
    trigger: String,
    content: String,
    created_at: String,
}

fn snippet_to_payload(snippet: Snippet) -> SnippetPayload {
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

fn refresh_whisper_prompt_full(state: &AppState) -> Result<(), String> {
    refresh_whisper_prompt_with_cache(&state.store, &state.pipeline, &state.prompt_cache)
}

fn ensure_pipeline_snippets_loaded(store: &Store, pipeline: &Arc<Mutex<PipelineOrchestrator>>) {
    match store.list_snippets() {
        Ok(snippets) => pipeline.lock().set_snippets(snippets),
        Err(err) => eprintln!("failed to load snippets cache: {err}"),
    }
}

fn refresh_whisper_prompt_state(state: &AppState) -> Result<(), String> {
    refresh_whisper_prompt_full(state)
}

fn apply_dictionary_additions(state: &AppState, added: &[String]) -> Result<(), String> {
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
    Ok(added)
}

fn emit_snippets_updated(app: &AppHandle) {
    let _ = app.emit("snippets-updated", ());
}

fn emit_app_context_updated(app: &AppHandle) {
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

fn refresh_correction_rules(
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

fn refresh_app_context_rules_state(state: &AppState) -> Result<(), String> {
    refresh_app_context_rules(&state.store, &state.pipeline)
}

#[derive(Debug, Clone, Serialize)]
struct AppContextRulePayload {
    id: i64,
    pattern: String,
    #[serde(rename = "matchType")]
    match_type: String,
    tone: String,
    #[serde(rename = "createdAt")]
    created_at: String,
}

fn app_context_rule_to_payload(rule: AppContextRule) -> AppContextRulePayload {
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

fn parse_match_type(value: &str) -> Result<AppContextMatchType, String> {
    AppContextMatchType::parse(value).ok_or_else(|| user_error_string(UserError::InvalidMatchType))
}

fn parse_tone_profile(value: &str) -> Result<ToneProfile, String> {
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

#[tauri::command]
fn get_pipeline_state(state: State<'_, AppState>) -> String {
    state.pipeline.lock().state().as_str().to_string()
}

#[tauri::command]
fn is_model_ready(state: State<'_, AppState>) -> bool {
    state.model_ready.load(Ordering::SeqCst)
}

#[tauri::command]
async fn toggle_dictation(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    if is_mic_probe_active(&state) {
        return Err(user_error_string(UserError::MicProbeActiveBeforeDictation));
    }
    let pipeline_state = state.pipeline.lock().state();
    if is_dictation_start_blocked_by_settings(&state, pipeline_state) {
        notify_dictation_blocked_by_settings(&app, &state);
        return Ok(());
    }
    ensure_model_then_toggle(app).await;
    Ok(())
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<SettingsPayload, String> {
    let settings = state.store.load_settings().map_err(|e| e.to_string())?;
    Ok(settings_to_payload(&settings))
}

#[tauri::command]
fn get_models_status(state: State<'_, AppState>) -> Result<ModelsStatusPayload, String> {
    let settings = state.store.load_settings().map_err(|e| e.to_string())?;
    Ok(build_models_status(&state, &settings))
}

#[tauri::command]
fn get_inference_info(state: State<'_, AppState>) -> Result<inference::InferenceInfo, String> {
    let settings = state.store.load_settings().map_err(|e| e.to_string())?;
    Ok(inference::get_inference_info(
        &settings,
        &state.capabilities,
    ))
}

#[tauri::command]
fn is_onboarding_done(state: State<'_, AppState>) -> Result<bool, String> {
    state.store.is_onboarding_done().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_onboarding_done(state: State<'_, AppState>, done: bool) -> Result<(), String> {
    state
        .store
        .set_onboarding_done(done)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_model(
    state: State<'_, AppState>,
    kind: String,
    model_id: String,
) -> Result<(), String> {
    let settings = state.store.load_settings().map_err(|e| e.to_string())?;
    let effective_whisper =
        system::resolve_whisper_model(settings.whisper_model(), &state.capabilities);
    let effective_llm = system::resolve_llm_model(settings.llm_model(), &state.capabilities);
    let path = match kind.as_str() {
        "whisper" => {
            let model = WhisperModel::parse(&model_id)
                .ok_or_else(|| user_error_string(UserError::UnknownWhisperModel))?;
            if !model.is_concrete() {
                return Err(user_error_string(UserError::CannotDeleteAutoWhisperModel));
            }
            if model == effective_whisper {
                return Err(user_error_string(UserError::CannotDeleteActiveWhisperModel));
            }
            model.path()
        }
        "llm" => {
            let model = llm::LlmModel::parse(&model_id)
                .ok_or_else(|| user_error_string(UserError::UnknownLlmModel))?;
            if !model.is_concrete() {
                return Err(user_error_string(UserError::CannotDeleteAutoLlmModel));
            }
            if model == effective_llm {
                return Err(user_error_string(UserError::CannotDeleteActiveLlmModel));
            }
            model.path()
        }
        _ => return Err(user_error_string(UserError::InvalidModelKind)),
    };

    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn reinstall_model(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: String,
    model_id: String,
) -> Result<(), String> {
    let settings = state.store.load_settings().map_err(|e| e.to_string())?;
    let effective_whisper =
        system::resolve_whisper_model(settings.whisper_model(), &state.capabilities);
    let effective_llm = system::resolve_llm_model(settings.llm_model(), &state.capabilities);

    match kind.as_str() {
        "whisper" => {
            let model = WhisperModel::parse(&model_id)
                .ok_or_else(|| user_error_string(UserError::UnknownWhisperModel))?;
            if !model.is_concrete() {
                return Err(user_error_string(UserError::CannotDeleteAutoWhisperModel));
            }
            let reload_engine = model == effective_whisper;
            if reload_engine {
                invalidate_whisper_engine(&state);
            }
            let path = model.path();
            if path.exists() {
                std::fs::remove_file(&path).map_err(|e| e.to_string())?;
            }
            ensure_whisper_model_file(&app, model).await?;
            if reload_engine {
                ensure_model_inner(&app, &state).await?;
            }
        }
        "llm" => {
            let model = llm::LlmModel::parse(&model_id)
                .ok_or_else(|| user_error_string(UserError::UnknownLlmModel))?;
            if !model.is_concrete() {
                return Err(user_error_string(UserError::CannotDeleteAutoLlmModel));
            }
            let reload_engine = model == effective_llm && settings.auto_edit;
            if model == effective_llm {
                shutdown_llm_engine(&state);
                let _ = app.emit("llm-unready", ());
            }
            let path = model.path();
            if path.exists() {
                std::fs::remove_file(&path).map_err(|e| e.to_string())?;
            }
            ensure_llm_model_file(&app, model).await?;
            if reload_engine {
                ensure_llm_model_inner(&app, &state).await?;
            }
        }
        _ => return Err(user_error_string(UserError::InvalidModelKind)),
    }
    Ok(())
}

#[tauri::command]
async fn set_hotkey(
    app: AppHandle,
    state: State<'_, AppState>,
    hotkey: String,
) -> Result<(), String> {
    let binding = hotkey::parse_hotkey_setting(&hotkey).map_err(|e| e.to_string())?;
    let previous = state.store.load_settings().map_err(|e| e.to_string())?;
    let previous_hotkey = previous.hotkey.clone();
    let capture_active = state.hotkey_suspend_depth.load(Ordering::SeqCst) > 0;

    if !capture_active {
        register_hotkey_binding(&app, binding)?;
    }

    let mut next = previous.clone();
    next.hotkey = hotkey::format_hotkey_setting(binding);
    if let Err(err) = state.store.save_settings(&next).map_err(|e| e.to_string()) {
        if !capture_active {
            let rollback =
                hotkey::parse_hotkey_setting(&previous_hotkey).map_err(|e| e.to_string())?;
            let _ = register_hotkey_binding(&app, rollback);
        }
        return Err(err);
    }

    Ok(())
}

#[tauri::command]
fn set_hotkey_capture_active(
    app: AppHandle,
    state: State<'_, AppState>,
    active: bool,
) -> Result<bool, String> {
    if active {
        suspend_global_hotkey(&app, &state)?;
        hotkey::start_hotkey_capture(&app)
    } else {
        hotkey::stop_hotkey_capture()?;
        resume_global_hotkey(&app, &state)?;
        Ok(false)
    }
}

#[tauri::command]
fn get_stt_language(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state
        .pipeline
        .lock()
        .effective_stt_language()
        .as_setting_value())
}

#[tauri::command]
fn cycle_dictation_language(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let language = state
        .pipeline
        .lock()
        .cycle_session_language(&app)
        .map_err(|e| e.to_string())?;
    Ok(language.as_setting_value())
}

#[tauri::command]
async fn set_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: SettingsPayload,
) -> Result<(), String> {
    let previous = state.store.load_settings().map_err(|e| e.to_string())?;
    let stt_language = parse_stt_language(&settings.stt_language)?;
    let whisper_model = WhisperModel::parse(&settings.whisper_model)
        .ok_or_else(|| user_error_string(UserError::UnknownWhisperModel))?;
    let llm_model = llm::LlmModel::parse(&settings.llm_model)
        .ok_or_else(|| user_error_string(UserError::UnknownLlmModel))?;
    let inference_backend = InferenceBackend::parse(&settings.inference_backend)
        .ok_or_else(|| user_error_string(UserError::InvalidInferenceBackend))?;

    let prev_stt = previous.stt_language.clone();
    let next_stt = settings.stt_language.clone();
    let app_for_notify = app.clone();
    let stt_language_changed = prev_stt != next_stt;
    let ui_language_changed = previous.ui_language != parse_ui_language(&settings.ui_language);
    let whisper_changed = previous.whisper_model() != whisper_model;
    let llm_setting_changed =
        previous.llm_model.trim().to_lowercase() != llm_model.as_setting_value();
    let llm_changed = previous.llm_model() != llm_model || llm_setting_changed;
    let inference_changed = previous.inference_backend() != inference_backend;
    let low_power_changed = previous.low_power_mode != settings.low_power_mode;
    let adaptive_changed = previous.adaptive_perf != settings.adaptive_perf;
    let input_device_changed = previous.input_device != settings.input_device;

    let prev_effective_whisper =
        system::resolve_whisper_model(previous.whisper_model(), &state.capabilities);
    let next_effective_whisper = system::resolve_whisper_model(whisper_model, &state.capabilities);
    let prev_effective_llm = system::resolve_llm_model(previous.llm_model(), &state.capabilities);
    let next_effective_llm = system::resolve_llm_model(llm_model, &state.capabilities);
    let whisper_effective_changed = prev_effective_whisper != next_effective_whisper;
    let llm_effective_changed = prev_effective_llm != next_effective_llm;

    let binding = hotkey::parse_hotkey_setting(&settings.hotkey).map_err(|e| e.to_string())?;
    let next_hotkey = hotkey::format_hotkey_setting(binding);
    let hotkey_changed = previous.hotkey != next_hotkey;

    let next_settings = AppSettings {
        auto_edit: settings.auto_edit,
        auto_learn: settings.auto_learn,
        auto_update: settings.auto_update,
        stt_language: next_stt,
        whisper_model: whisper_model.as_setting_value().into(),
        llm_model: llm_model.as_setting_value().into(),
        hotkey: next_hotkey,
        inference_backend: inference_backend.as_setting_value().into(),
        low_power_mode: settings.low_power_mode,
        adaptive_perf: settings.adaptive_perf,
        ui_language: parse_ui_language(&settings.ui_language),
        input_device: settings.input_device.clone(),
    };

    let mut rollback_ctx = SettingsRollbackContext {
        hotkey_changed,
        stt_language_changed,
        whisper_invalidated: false,
    };

    state.pipeline.lock().set_auto_learn(settings.auto_learn);
    state.pipeline.lock().set_default_stt_language(stt_language);
    if input_device_changed {
        state
            .pipeline
            .lock()
            .set_input_device(next_settings.input_device.clone());
    }

    if let Err(err) = state
        .store
        .save_settings(&next_settings)
        .map_err(|e| e.to_string())
    {
        state
            .pipeline
            .lock()
            .set_default_stt_language(previous.stt_language_mode());
        state.pipeline.lock().set_auto_learn(previous.auto_learn);
        return Err(err);
    }

    if hotkey_changed {
        if let Err(err) = register_hotkey_binding(&app, binding) {
            if let Err(rollback_err) =
                rollback_settings(&app, &state, &previous, rollback_ctx).await
            {
                eprintln!("settings rollback failed: {rollback_err}");
            }
            return Err(err);
        }
    }

    refresh_perf_config(&state, &next_settings, should_start_minimized());

    if low_power_changed && settings.low_power_mode {
        shutdown_llm_engine(&state);
        let _ = app.emit("llm-unready", ());
        state
            .deferred_llm_on_boot
            .store(settings.auto_edit, Ordering::SeqCst);
    }

    let need_whisper_file = whisper_changed
        || whisper_effective_changed
        || (adaptive_changed && settings.adaptive_perf);

    let whisper_reload_needed = whisper_changed
        || inference_changed
        || whisper_effective_changed
        || (adaptive_changed && settings.adaptive_perf);

    let whisper_settings_busy = need_whisper_file || whisper_reload_needed;
    let _whisper_settings_guard =
        whisper_settings_busy.then(|| WhisperSettingsChangeGuard::new(&state));

    let need_llm_file =
        llm_changed || llm_effective_changed || (adaptive_changed && settings.adaptive_perf);

    let llm_engine_out_of_sync = llm_engine_stale(&state, next_effective_llm);
    let llm_reload_needed = llm_changed
        || inference_changed
        || llm_effective_changed
        || low_power_changed
        || (adaptive_changed && settings.adaptive_perf)
        || llm_engine_out_of_sync;
    let llm_lazy_load = state.perf_config.lock().llm_lazy_load;
    let llm_model_preference_changed = llm_changed || llm_effective_changed;
    let will_load_llm_engine = settings.auto_edit
        && (!llm_lazy_load || llm_model_preference_changed)
        && (llm_reload_needed || !llm_engine_is_live(&state));

    if need_whisper_file {
        if let Err(err) = ensure_whisper_model_file(&app, next_effective_whisper).await {
            if let Err(rollback_err) =
                rollback_settings(&app, &state, &previous, rollback_ctx).await
            {
                eprintln!("settings rollback failed: {rollback_err}");
            }
            return Err(err);
        }
    }

    if need_llm_file && !will_load_llm_engine {
        if let Err(err) = ensure_llm_model_file(&app, next_effective_llm).await {
            if let Err(rollback_err) =
                rollback_settings(&app, &state, &previous, rollback_ctx).await
            {
                eprintln!("settings rollback failed: {rollback_err}");
            }
            return Err(err);
        }
    }

    if whisper_reload_needed {
        invalidate_whisper_engine(&state);
        rollback_ctx.whisper_invalidated = true;
        if let Err(err) = ensure_model_inner(&app, &state).await {
            if let Err(rollback_err) =
                rollback_settings(&app, &state, &previous, rollback_ctx).await
            {
                eprintln!("settings rollback failed: {rollback_err}");
            }
            return Err(err);
        }
    }

    if settings.auto_edit {
        state.pipeline.lock().set_auto_edit(true);
        if llm_reload_needed || !llm_engine_is_live(&state) {
            shutdown_llm_engine(&state);
            let _ = app.emit("llm-unready", ());
            if !llm_lazy_load || llm_model_preference_changed || llm_engine_out_of_sync {
                if let Err(err) = ensure_llm_model_inner(&app, &state).await {
                    // Preference is already persisted; keep it and surface the load error.
                    eprintln!("llm reload after settings change failed: {err}");
                    if err == user_error_string(UserError::LlmModelCorrupt) {
                        return Err(err);
                    }
                    return Err(user_error_string(UserError::LlmEngineLoadFailed));
                }
            } else {
                state.deferred_llm_on_boot.store(true, Ordering::SeqCst);
            }
        }
    } else {
        state.pipeline.lock().set_auto_edit(false);
        shutdown_llm_engine(&state);
        let _ = app.emit("llm-unready", ());
    }

    if stt_language_changed {
        state
            .pipeline
            .lock()
            .notify_stt_language_changed(&app_for_notify);
    }

    if ui_language_changed {
        let _ = app.emit("ui-language-changed", next_settings.ui_language.clone());
    }

    sync_tray_menus(&app);
    Ok(())
}

async fn ensure_llm_model_inner(app: &AppHandle, state: &AppState) -> Result<(), String> {
    if !state.pipeline.lock().auto_edit_enabled() {
        return Ok(());
    }

    let expected = state.perf_config.lock().llm;
    if llm_engine_matches(state, expected) {
        return Ok(());
    }

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
    Ok(())
}

#[tauri::command]
async fn ensure_llm_model(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    ensure_llm_model_inner(&app, &state).await
}

#[tauri::command]
fn list_dictionary_words(state: State<'_, AppState>) -> Result<Vec<DictionaryWordPayload>, String> {
    state
        .store
        .list_words()
        .map(|words| words.into_iter().map(dictionary_word_to_payload).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn add_dictionary_word(
    app: AppHandle,
    state: State<'_, AppState>,
    word: String,
    misspelling: Option<String>,
) -> Result<bool, String> {
    let normalized = normalize_word(&word);
    if normalized.is_empty() {
        return Err(user_error_string(UserError::DictionaryWordEmpty));
    }
    if !is_valid_dictionary_word(&normalized) {
        return Err(user_error_string(UserError::DictionaryWordInvalid));
    }

    let normalized_misspelling = misspelling
        .as_deref()
        .map(normalize_word)
        .filter(|value| !value.is_empty());
    if let Some(ref incorrect) = normalized_misspelling {
        if !is_valid_dictionary_word(incorrect) {
            return Err(user_error_string(UserError::DictionaryMisspellingInvalid));
        }
        if normalized.eq_ignore_ascii_case(incorrect) {
            return Err(user_error_string(UserError::DictionaryMisspellingSame));
        }
    }

    let inserted_id = state
        .store
        .add_word(
            &normalized,
            DictionarySource::Manual,
            normalized_misspelling.as_deref(),
        )
        .map_err(|e| e.to_string())?;

    let Some(inserted_id) = inserted_id else {
        if normalized_misspelling.is_some() {
            return Err(user_error_string(UserError::DictionaryMisspellingExists));
        }
        return Err(user_error_string(UserError::DictionaryWordExists));
    };

    if let Err(err) = apply_dictionary_additions(&state, std::slice::from_ref(&normalized)) {
        let _ = state.store.remove_word(inserted_id);
        return Err(err);
    }
    if let Err(err) = refresh_correction_rules(&state.store, &state.pipeline) {
        let _ = state.store.remove_word(inserted_id);
        return Err(err);
    }

    state.dictionary_notifier.emit_immediate(
        &app,
        DictionaryUpdatedPayload {
            added: vec![normalized],
            removed: vec![],
            source: Some("manual".into()),
        },
    );

    Ok(true)
}

#[tauri::command]
fn update_dictionary_word(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    word: String,
) -> Result<bool, String> {
    let normalized = normalize_word(&word);
    if normalized.is_empty() {
        return Err(user_error_string(UserError::DictionaryWordEmpty));
    }
    if !is_valid_dictionary_word(&normalized) {
        return Err(user_error_string(UserError::DictionaryWordInvalid));
    }

    let previous = state
        .store
        .get_word_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| user_error_string(UserError::DictionaryWordNotFound))?;

    if previous.word == normalized {
        return Ok(true);
    }

    let updated = state
        .store
        .update_word(id, &normalized)
        .map_err(|e| e.to_string())?;

    if updated {
        if let Err(err) = refresh_whisper_prompt_full(&state) {
            let _ = state.store.update_word(id, &previous.word);
            return Err(err);
        }
        state.dictionary_notifier.emit_immediate(
            &app,
            DictionaryUpdatedPayload {
                added: vec![normalized.clone()],
                removed: vec![previous.word],
                source: Some("manual".into()),
            },
        );
    }

    Ok(updated)
}

#[tauri::command]
fn remove_dictionary_word(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    let entry = state
        .store
        .get_word_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| user_error_string(UserError::DictionaryWordNotFound))?;

    let removed = state.store.remove_word(id).map_err(|e| e.to_string())?;
    if !removed {
        return Err(user_error_string(UserError::DictionaryWordNotFound));
    }

    if let Err(err) = refresh_whisper_prompt_full(&state) {
        let _ = state
            .store
            .add_word(&entry.word, entry.source, entry.misspelling.as_deref());
        return Err(err);
    }

    if let Err(err) = refresh_correction_rules(&state.store, &state.pipeline) {
        let _ = state
            .store
            .add_word(&entry.word, entry.source, entry.misspelling.as_deref());
        return Err(err);
    }

    state.dictionary_notifier.emit_immediate(
        &app,
        DictionaryUpdatedPayload {
            added: vec![],
            removed: vec![entry.word],
            source: None,
        },
    );
    Ok(())
}

#[tauri::command]
fn learn_from_correction(
    app: AppHandle,
    state: State<'_, AppState>,
    original: String,
    corrected: String,
) -> Result<Vec<String>, String> {
    apply_learned_correction(&app, &state, &original, &corrected)
}

#[tauri::command]
fn list_snippets(state: State<'_, AppState>) -> Result<Vec<SnippetPayload>, String> {
    state
        .store
        .list_snippets()
        .map(|snippets| snippets.into_iter().map(snippet_to_payload).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn add_snippet(
    app: AppHandle,
    state: State<'_, AppState>,
    trigger: String,
    content: String,
) -> Result<bool, String> {
    let normalized_trigger = normalize_trigger(&trigger);
    if normalized_trigger.is_empty() {
        return Err(user_error_string(UserError::SnippetTriggerEmpty));
    }
    if !is_valid_trigger(&normalized_trigger) {
        return Err(user_error_string(UserError::SnippetTriggerTooShort));
    }
    if !is_valid_snippet_content(&content) {
        return Err(user_error_string(UserError::SnippetContentEmpty));
    }

    let inserted = state
        .store
        .add_snippet(&normalized_trigger, &content)
        .map_err(|e| e.to_string())?;

    if inserted {
        if let Err(err) = refresh_whisper_prompt_state(&state) {
            let _ = state.store.remove_snippet_by_trigger(&normalized_trigger);
            return Err(err);
        }
        emit_snippets_updated(&app);
    }

    Ok(inserted)
}

#[tauri::command]
fn update_snippet(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    trigger: String,
    content: String,
) -> Result<bool, String> {
    let normalized_trigger = normalize_trigger(&trigger);
    if normalized_trigger.is_empty() {
        return Err(user_error_string(UserError::SnippetTriggerEmpty));
    }
    if !is_valid_trigger(&normalized_trigger) {
        return Err(user_error_string(UserError::SnippetTriggerTooShort));
    }
    if !is_valid_snippet_content(&content) {
        return Err(user_error_string(UserError::SnippetContentEmpty));
    }

    let previous = state
        .store
        .get_snippet_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| user_error_string(UserError::SnippetNotFound))?;

    let updated = state
        .store
        .update_snippet(id, &normalized_trigger, &content)
        .map_err(|e| e.to_string())?;

    if updated {
        if let Err(err) = refresh_whisper_prompt_state(&state) {
            let _ = state
                .store
                .update_snippet(id, &previous.trigger, &previous.content);
            return Err(err);
        }
        emit_snippets_updated(&app);
    }

    Ok(updated)
}

#[tauri::command]
fn remove_snippet(app: AppHandle, state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let entry = state
        .store
        .get_snippet_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| user_error_string(UserError::SnippetNotFound))?;

    let removed = state.store.remove_snippet(id).map_err(|e| e.to_string())?;
    if !removed {
        return Err(user_error_string(UserError::SnippetNotFound));
    }

    if let Err(err) = refresh_whisper_prompt_state(&state) {
        let _ = state.store.add_snippet(&entry.trigger, &entry.content);
        return Err(err);
    }

    emit_snippets_updated(&app);
    Ok(())
}

#[tauri::command]
fn import_snippets(
    app: AppHandle,
    state: State<'_, AppState>,
    json: String,
) -> Result<usize, String> {
    let entries: Vec<SnippetImport> = serde_json::from_str(&json)
        .map_err(|_| user_error_string(UserError::InvalidSnippetJson))?;
    if entries.is_empty() {
        return Err(user_error_string(UserError::SnippetImportEmpty));
    }

    let previous = state
        .store
        .export_snippet_imports()
        .map_err(|e| e.to_string())?;

    let count = state
        .store
        .import_snippets(&entries)
        .map_err(|e| e.to_string())?;
    if count == 0 {
        return Err(user_error_string(UserError::SnippetImportNoValid));
    }

    if let Err(err) = refresh_whisper_prompt_state(&state) {
        let _ = state.store.replace_all_snippets(&previous);
        return Err(err);
    }
    emit_snippets_updated(&app);
    Ok(count)
}

#[tauri::command]
fn export_snippets(state: State<'_, AppState>) -> Result<String, String> {
    let entries = state
        .store
        .export_snippet_imports()
        .map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_snippet_user_name(state: State<'_, AppState>) -> Result<String, String> {
    state
        .store
        .get_snippet_user_name()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_snippet_user_name(state: State<'_, AppState>, name: String) -> Result<(), String> {
    state
        .store
        .set_snippet_user_name(&name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn preview_snippet_expansion(
    state: State<'_, AppState>,
    content: String,
) -> Result<String, String> {
    let user_name = state
        .store
        .get_snippet_user_name()
        .map_err(|e| e.to_string())?;
    let clipboard = TextInjector::read_clipboard_text().ok().flatten();
    let ctx = SnippetVariableContext::from_user_name(user_name).with_clipboard(clipboard);
    Ok(expand_snippet_variables(&content, &ctx))
}

#[tauri::command]
fn get_active_window() -> Option<app_context::ActiveWindow> {
    app_context::get_active_window()
}

#[tauri::command]
fn list_app_context_rules(
    state: State<'_, AppState>,
) -> Result<Vec<AppContextRulePayload>, String> {
    state
        .store
        .list_app_context_rules()
        .map(|rules| rules.into_iter().map(app_context_rule_to_payload).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn add_app_context_rule(
    app: AppHandle,
    state: State<'_, AppState>,
    pattern: String,
    match_type: String,
    tone: String,
) -> Result<bool, String> {
    let match_type = parse_match_type(&match_type)?;
    let tone = parse_tone_profile(&tone)?;
    if !is_valid_app_context_pattern(&pattern, match_type) {
        return Err(user_error_string(UserError::AppContextPatternTooShort));
    }

    let inserted = state
        .store
        .add_app_context_rule(&NewAppContextRule {
            pattern,
            match_type,
            tone,
        })
        .map_err(|e| e.to_string())?;

    if inserted {
        refresh_app_context_rules_state(&state)?;
        emit_app_context_updated(&app);
    }

    Ok(inserted)
}

#[tauri::command]
fn remove_app_context_rule(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    let removed = state
        .store
        .remove_app_context_rule(id)
        .map_err(|e| e.to_string())?;
    if !removed {
        return Err(user_error_string(UserError::AppContextRuleNotFound));
    }

    refresh_app_context_rules_state(&state)?;
    emit_app_context_updated(&app);
    Ok(())
}

#[tauri::command]
fn list_dictations(
    state: State<'_, AppState>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<DictationEntry>, String> {
    let limit = limit.unwrap_or(DEFAULT_LIST_LIMIT).clamp(1, 200);
    let offset = offset.unwrap_or(0);
    state
        .store
        .list_dictations(limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn search_dictations(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<DictationEntry>, String> {
    let limit = limit.unwrap_or(DEFAULT_LIST_LIMIT).clamp(1, 200);
    let offset = offset.unwrap_or(0);
    state
        .store
        .search_dictations(&query, limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn count_dictations(state: State<'_, AppState>) -> Result<i64, String> {
    state.store.count_dictations().map_err(|e| e.to_string())
}

#[tauri::command]
fn count_search_dictations(state: State<'_, AppState>, query: String) -> Result<i64, String> {
    state
        .store
        .count_search_dictations(&query)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn copy_dictation(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let entry = state
        .store
        .get_dictation(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| user_error_string(UserError::DictationNotFound))?;
    TextInjector::copy_to_clipboard(&entry.text).map_err(|e| e.to_string())
}

#[tauri::command]
fn reinject_dictation(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let entry = state
        .store
        .get_dictation(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| user_error_string(UserError::DictationNotFound))?;
    let injector = TextInjector::new().map_err(|e| e.to_string())?;
    injector.inject(&entry.text).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_insights(state: State<'_, AppState>) -> Result<Insights, String> {
    state.store.get_insights().map_err(|e| e.to_string())
}

#[tauri::command]
fn is_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn list_input_devices() -> Result<Vec<audio::InputDeviceInfo>, String> {
    audio::list_input_devices().map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_mic_probe(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if state.mic_probe.capture.lock().is_some() {
        return Err(user_error_string(UserError::MicProbeAlreadyActive));
    }
    if state.pipeline.lock().state() != PipelineState::Idle {
        return Err(user_error_string(UserError::DictationActiveMicProbe));
    }
    suspend_global_hotkey(&app, &state)?;

    let input_device = state
        .store
        .load_settings()
        .map_err(|e| e.to_string())?
        .input_device;

    let mut capture = audio::AudioCapture::new().map_err(|e| e.to_string())?;
    let (level_tx, level_rx) = std::sync::mpsc::channel::<audio::AudioLevelSample>();
    if let Err(err) = capture.start_with_streaming(None, Some(level_tx), Some(&input_device)) {
        let _ = resume_global_hotkey(&app, &state);
        return Err(err.to_string());
    }

    let app_clone = app.clone();
    let level_task = tauri::async_runtime::spawn(async move {
        while let Ok(sample) = level_rx.recv() {
            let _ = app_clone.emit(
                "audio-level",
                pipeline::AudioLevelEvent {
                    level: sample.level,
                    bands: sample.bands.to_vec(),
                },
            );
        }
    });

    let mut capture_slot = state.mic_probe.capture.lock();
    *state.mic_probe.level_task.lock() = Some(level_task);
    *capture_slot = Some(capture);
    Ok(())
}

fn stop_mic_probe_inner(app: &AppHandle, state: &AppState) -> Result<(), String> {
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

#[tauri::command]
async fn stop_mic_probe(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    stop_mic_probe_inner(&app, &state)
}

async fn wait_for_pipeline_idle_timeout(
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

#[tauri::command]
async fn prepare_onboarding_dictation(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    stop_mic_probe_inner(&app, &state)?;

    while state.hotkey_suspend_depth.load(Ordering::SeqCst) > 0 {
        resume_global_hotkey(&app, &state)?;
    }

    let pipeline = state.pipeline.clone();
    if pipeline.lock().state() != PipelineState::Idle {
        spawn_stop(app.clone(), Arc::clone(&pipeline));
        if !wait_for_pipeline_idle_timeout(&pipeline, Duration::from_secs(2)).await {
            return Err("Dictation is still active. Stop recording before continuing.".to_string());
        }
    }

    if state.hotkey_suspend_depth.load(Ordering::SeqCst) > 0 {
        eprintln!(
            "prepare_onboarding_dictation: hotkey_suspend_depth still {} after cleanup",
            state.hotkey_suspend_depth.load(Ordering::SeqCst)
        );
    }

    Ok(())
}

#[tauri::command]
fn set_autostart_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    let autolaunch = app.autolaunch();
    if enabled {
        autolaunch.enable().map_err(|e| e.to_string())?;
    } else {
        autolaunch.disable().map_err(|e| e.to_string())?;
    }
    sync_tray_menus(&app);
    Ok(())
}

fn set_auto_edit_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut settings = state.store.load_settings().map_err(|e| e.to_string())?;
    if settings.auto_edit == enabled {
        sync_tray_menus(&app);
        return Ok(());
    }

    settings.auto_edit = enabled;
    state
        .store
        .save_settings(&settings)
        .map_err(|e| e.to_string())?;
    state.pipeline.lock().set_auto_edit(enabled);

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

async fn ensure_model_inner(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let expected = state.perf_config.lock().whisper;
    if whisper_engine_matches(state, expected) {
        let _ = app.emit("model-ready", ());
        ensure_dictation_hotkey_registered(app, state)?;
        return Ok(());
    }

    if state.model_ready.load(Ordering::SeqCst) || state.whisper_engine.lock().is_some() {
        invalidate_whisper_engine(state);
    }

    let _init_guard = state.model_init.lock().await;

    if whisper_engine_matches(state, expected) {
        let _ = app.emit("model-ready", ());
        ensure_dictation_hotkey_registered(app, state)?;
        return Ok(());
    }

    if state.model_ready.load(Ordering::SeqCst) || state.whisper_engine.lock().is_some() {
        invalidate_whisper_engine(state);
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
    Ok(())
}

#[tauri::command]
async fn ensure_model(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    ensure_model_inner(&app, &state).await
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
    if !state.model_ready.load(Ordering::SeqCst) {
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
        spawn_start(app, pipeline);
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

async fn ensure_model_then_toggle(app: AppHandle) {
    let state = app.state::<AppState>();
    if is_mic_probe_active(&state) {
        return;
    }
    let pipeline_state = state.pipeline.lock().state();
    if is_dictation_start_blocked_by_settings(&state, pipeline_state) {
        notify_dictation_blocked_by_settings(&app, &state);
        return;
    }
    if !state.model_ready.load(Ordering::SeqCst) {
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
    spawn_toggle(app, pipeline);
}

#[cfg(windows)]
pub(crate) fn dispatch_dictation_hotkey(app: &AppHandle, shortcut_state: ShortcutState) {
    handle_hotkey(app, shortcut_state);
}

fn handle_hotkey(app: &AppHandle, shortcut_state: ShortcutState) {
    let state = app.state::<AppState>();
    if is_mic_probe_active(&state) {
        return;
    }
    if state.hotkey_suspend_depth.load(Ordering::SeqCst) > 0 {
        return;
    }

    match shortcut_state {
        ShortcutState::Pressed => {
            let mut press = state.hotkey_press.lock();
            if press.shortcut_down {
                // Key repeat while holding — keep original press_start / was_idle for PTT.
                return;
            }
            press.shortcut_down = true;

            let current = state.pipeline.lock().state();
            press.press_start = Some(Instant::now());
            press.was_idle_on_press = current == PipelineState::Idle;

            match current {
                PipelineState::Idle => {
                    if is_dictation_blocked_by_settings(&state) {
                        press.shortcut_down = false;
                        press.press_start = None;
                        drop(press);
                        notify_dictation_blocked_by_settings(app, &state);
                        return;
                    }
                    if state.model_ready.load(Ordering::SeqCst) {
                        press.deferred_start_pending = false;
                        press.deferred_toggle_intent = false;
                        spawn_start(app.clone(), state.pipeline.clone());
                    } else {
                        press.deferred_start_pending = true;
                        press.deferred_toggle_intent = false;
                        let app_clone = app.clone();
                        tauri::async_runtime::spawn(async move {
                            ensure_model_then_start(app_clone).await;
                        });
                    }
                }
                // Distinct second tap (after release) toggles off.
                PipelineState::Recording => {
                    spawn_stop(app.clone(), state.pipeline.clone());
                }
                PipelineState::Transcribing => {
                    press.busy_cancel_on_release = true;
                    drop(press);
                    pipeline::emit_dictation_busy(app, PipelineState::Transcribing, true);
                }
                PipelineState::Injecting => {
                    press.busy_cancel_on_release = false;
                    drop(press);
                    pipeline::emit_dictation_busy(app, PipelineState::Injecting, false);
                }
            }
        }
        ShortcutState::Released => {
            let mut press = state.hotkey_press.lock();
            if !press.shortcut_down {
                return;
            }
            press.shortcut_down = false;

            let cancel_busy = press.busy_cancel_on_release;
            press.busy_cancel_on_release = false;

            let Some(start) = press.press_start.take() else {
                if cancel_busy && state.pipeline.lock().state() == PipelineState::Transcribing {
                    pipeline::spawn_cancel(app.clone(), state.pipeline.clone());
                }
                return;
            };
            let was_idle = press.was_idle_on_press;
            let duration = start.elapsed();

            if press.deferred_start_pending {
                press.deferred_toggle_intent = hotkey::is_toggle_tap(was_idle, duration);
            }

            if cancel_busy && state.pipeline.lock().state() == PipelineState::Transcribing {
                pipeline::spawn_cancel(app.clone(), state.pipeline.clone());
                return;
            }

            if state.pipeline.lock().state() == PipelineState::Recording
                && hotkey::should_stop_ptt_on_release(was_idle, duration)
            {
                spawn_stop(app.clone(), state.pipeline.clone());
            }
        }
    }
}

fn should_start_minimized() -> bool {
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

fn sync_tray_menus(app: &AppHandle) {
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

        let autostart_enabled = app.autolaunch().is_enabled().unwrap_or(false);
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
    let autostart_checked = app.autolaunch().is_enabled().unwrap_or(false);
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
                let enabled = app.autolaunch().is_enabled().unwrap_or(false);
                let _ = set_autostart_enabled(app.clone(), !enabled);
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

        let updater = match app.updater() {
            Ok(updater) => updater,
            Err(err) => {
                eprintln!("auto-update: updater unavailable: {err}");
                return;
            }
        };

        match updater.check().await {
            Ok(Some(update)) => {
                if update::dismissed_update_version().as_deref() == Some(update.version.as_str()) {
                    return;
                }

                wait_before_app_update_download(&app, &settings).await;

                let version = update.version.clone();
                match update.download(|_, _| {}, || {}).await {
                    Ok(bytes) => {
                        let bytes_path = match update::store_pending_update_bytes(&bytes) {
                            Ok(path) => path,
                            Err(err) => {
                                eprintln!("auto-update: failed to store update: {err}");
                                return;
                            }
                        };

                        let payload = update::UpdateReadyPayload {
                            version: version.clone(),
                        };
                        let state = app.state::<AppState>();
                        *state.pending_update.lock() = Some(update::PendingUpdate {
                            version,
                            update,
                            bytes_path,
                        });
                        let _ = app.emit("update-ready", payload);
                    }
                    Err(err) => eprintln!("auto-update: download failed: {err}"),
                }
            }
            Ok(None) => {}
            Err(err) => eprintln!("auto-update: check failed: {err}"),
        }
    });
}

async fn wait_before_app_update_download(app: &AppHandle, settings: &AppSettings) {
    let preload_whisper = app.state::<AppState>().perf_config.lock().preload_whisper;
    let preload_llm = settings.auto_edit && app.state::<AppState>().perf_config.lock().preload_llm;

    loop {
        let ready = {
            let state = app.state::<AppState>();
            let pipeline_idle = state.pipeline.lock().state() == PipelineState::Idle;

            let whisper_busy = preload_whisper
                && !state.model_ready.load(Ordering::SeqCst)
                && state.whisper_engine.lock().is_none();
            let llm_busy = preload_llm
                && !state.llm_ready.load(Ordering::SeqCst)
                && state.llm_engine.lock().is_none();

            pipeline_idle && !whisper_busy && !llm_busy
        };
        if ready {
            return;
        }

        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}

async fn wait_for_pipeline_idle(pipeline: &Arc<Mutex<PipelineOrchestrator>>) {
    loop {
        if pipeline.lock().state() == PipelineState::Idle {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

#[tauri::command]
fn get_pending_update_version(state: State<'_, AppState>) -> Option<String> {
    state
        .pending_update
        .lock()
        .as_ref()
        .map(|pending| pending.version.clone())
}

#[tauri::command]
async fn install_pending_update(state: State<'_, AppState>) -> Result<(), String> {
    let pending = state.pending_update.lock().take();
    let Some(pending) = pending else {
        return Err("Aucune mise à jour en attente.".into());
    };

    wait_for_pipeline_idle(&state.pipeline).await;

    let bytes = update::read_pending_update_bytes(&pending.bytes_path)?;
    update::mark_show_after_update();
    update::clear_dismissed_update_version();
    let result = pending.update.install(&bytes);
    pending.clear();
    result.map_err(|err| format!("Échec de l'installation : {err}"))
}

#[tauri::command]
fn dismiss_pending_update(state: State<'_, AppState>) -> Result<(), String> {
    let dismissed_version = state.pending_update.lock().take().map(|pending| {
        let version = pending.version.clone();
        pending.clear();
        version
    });
    if let Some(version) = dismissed_version {
        update::mark_update_dismissed(&version);
    } else {
        update::clear_pending_update_files();
    }
    Ok(())
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

    if initial_settings.llm_model == "qwen3-4b" {
        initial_settings.llm_model = llm::LlmModel::Qwen3_5_4B.as_setting_value().into();
        if let Err(err) = store.save_settings(&initial_settings) {
            eprintln!("failed to migrate llm_model from qwen3-4b: {err}");
        }
    }

    if let Err(err) = stt::remove_legacy_medium_model() {
        eprintln!("failed to remove legacy whisper medium model: {err}");
    }

    if let Err(err) = llm::remove_legacy_qwen3_4b_model() {
        eprintln!("failed to remove legacy qwen3 4b model: {err}");
    }

    pipeline.lock().set_auto_edit(initial_settings.auto_edit);
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
            prompt_cache,
            dictionary_notifier,
            capabilities,
            perf_config: Mutex::new(initial_perf),
            last_activity: Mutex::new(Instant::now()),
            model_ready: AtomicBool::new(false),
            model_init: Arc::clone(&model_init),
            llm_ready: llm_ready.clone(),
            llm_init: Arc::clone(&llm_init),
            hotkey_press: Mutex::new(HotkeyPressState {
                press_start: None,
                was_idle_on_press: false,
                shortcut_down: false,
                deferred_start_pending: false,
                deferred_toggle_intent: false,
                busy_cancel_on_release: false,
            }),
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
        })
        .setup(move |app| {
            let _modules = registered_modules();
            build_tray(app.handle()).map_err(|e| e.to_string())?;
            sync_tray_menus(app.handle());
            let _ = app_context::get_active_window();

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
                    if let Err(err) = ensure_model(app_handle.clone(), state).await {
                        eprintln!("model initialization failed: {err}");
                        let _ = app_handle.emit("model-init-error", err);
                    }
                });
            }

            if preload_llm {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<AppState>();
                    if let Err(err) = ensure_llm_model(app_handle.clone(), state).await {
                        eprintln!("llm model initialization failed: {err}");
                    }
                });
            }

            {
                let app_for_update = app.handle().clone();
                let state = app.state::<AppState>();
                spawn_update_check_if_enabled(app_for_update, Arc::clone(&state.store));
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_pipeline_state,
            is_model_ready,
            toggle_dictation,
            ensure_model,
            ensure_llm_model,
            get_settings,
            set_settings,
            get_models_status,
            delete_model,
            reinstall_model,
            get_inference_info,
            set_hotkey,
            set_hotkey_capture_active,
            is_onboarding_done,
            set_onboarding_done,
            list_input_devices,
            start_mic_probe,
            stop_mic_probe,
            prepare_onboarding_dictation,
            get_stt_language,
            cycle_dictation_language,
            is_autostart_enabled,
            set_autostart_enabled,
            get_pending_update_version,
            install_pending_update,
            dismiss_pending_update,
            list_dictionary_words,
            add_dictionary_word,
            update_dictionary_word,
            remove_dictionary_word,
            learn_from_correction,
            list_snippets,
            add_snippet,
            update_snippet,
            remove_snippet,
            import_snippets,
            export_snippets,
            get_snippet_user_name,
            set_snippet_user_name,
            preview_snippet_expansion,
            get_active_window,
            list_app_context_rules,
            add_app_context_rule,
            remove_app_context_rule,
            list_dictations,
            search_dictations,
            count_dictations,
            count_search_dictations,
            copy_dictation,
            reinject_dictation,
            get_insights
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

        let loaded = Some(WhisperModel::Small);
        let expected = WhisperModel::DistilFrDec16;
        assert_ne!(loaded, Some(expected));
        assert_eq!(loaded, Some(WhisperModel::Small));
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
