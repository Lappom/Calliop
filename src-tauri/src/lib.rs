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

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;

use calliop_prompt::ToneProfile;
use dictionary_notify::{DictionaryNotifier, DictionaryUpdatedPayload};
use inject::TextInjector;
use parking_lot::Mutex;
use pipeline::{
    spawn_start, spawn_stop, spawn_toggle, PipelineOrchestrator, PipelineState, PipelineStateEvent,
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
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, RunEvent, State, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::Shortcut;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_updater::UpdaterExt;

/// Returns registered core module names (Phase 0 wiring check).
pub fn registered_modules() -> [&'static str; 10] {
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
}

struct TrayHandles {
    autostart_item: CheckMenuItem<tauri::Wry>,
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
    model_ready: AtomicBool,
    model_init: Arc<tokio::sync::Mutex<()>>,
    llm_ready: Arc<AtomicBool>,
    llm_init: Arc<tokio::sync::Mutex<()>>,
    hotkey_press: Mutex<HotkeyPressState>,
    deferred_llm_on_boot: AtomicBool,
    current_hotkey: Mutex<Shortcut>,
    hotkey_suspend_depth: AtomicU32,
    mic_probe: MicProbeState,
}

fn llm_engine_is_live(state: &AppState) -> bool {
    state.llm_ready.load(Ordering::SeqCst) && state.llm_engine.lock().is_some()
}

fn shutdown_llm_engine(state: &AppState) {
    state.llm_ready.store(false, Ordering::SeqCst);
    *state.llm_engine.lock() = None;
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
    }
}

fn file_size_bytes(path: &std::path::Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|meta| meta.len())
}

fn build_models_status(settings: &AppSettings) -> ModelsStatusPayload {
    let active_whisper = settings.whisper_model();
    let active_llm = settings.llm_model();

    ModelsStatusPayload {
        whisper: WhisperModel::all()
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
        llm: llm::LlmModel::all()
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
    if ctx.hotkey_changed {
        let prev_shortcut = hotkey::parse_shortcut(&previous.hotkey).map_err(|e| e.to_string())?;
        register_hotkey(&app_for_notify, prev_shortcut)?;
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
    let gs = app.global_shortcut();
    let current = *state.current_hotkey.lock();
    if gs.is_registered(current) {
        gs.unregister(current).map_err(|e| e.to_string())?;
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
                    let settings = state.store.load_settings().map_err(|e| e.to_string())?;
                    let shortcut =
                        hotkey::parse_shortcut(&settings.hotkey).map_err(|e| e.to_string())?;
                    register_hotkey(app, shortcut)?;
                }
                return Ok(());
            }
            Err(actual) => current = actual,
        }
    }
}

fn is_mic_probe_active(state: &AppState) -> bool {
    state.mic_probe.capture.lock().is_some()
}

fn register_hotkey(app: &AppHandle, shortcut: Shortcut) -> Result<(), String> {
    let gs = app.global_shortcut();
    let current = *app.state::<AppState>().current_hotkey.lock();
    if gs.is_registered(current) {
        gs.unregister(current).map_err(|e| e.to_string())?;
    }
    gs.register(shortcut).map_err(|e| e.to_string())?;
    *app.state::<AppState>().current_hotkey.lock() = shortcut;
    Ok(())
}

fn parse_stt_language(value: &str) -> Result<SttLanguage, String> {
    SttLanguage::parse(value).ok_or_else(|| format!("unsupported STT language: {value}"))
}

#[derive(Debug, Clone, Serialize)]
struct DictionaryWordPayload {
    id: i64,
    word: String,
    source: String,
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
    AppContextMatchType::parse(value)
        .ok_or_else(|| "Type de correspondance invalide (exe ou title_contains).".into())
}

fn parse_tone_profile(value: &str) -> Result<ToneProfile, String> {
    ToneProfile::parse(value)
        .ok_or_else(|| "Ton invalide (default, casual, formal ou technical).".into())
}

const MENU_OPEN: &str = "open";
const MENU_TOGGLE: &str = "toggle";
const MENU_AUTOSTART: &str = "autostart";
const MENU_QUIT: &str = "quit";

#[tauri::command]
fn get_pipeline_state(state: State<'_, AppState>) -> String {
    state.pipeline.lock().state().as_str().to_string()
}

#[tauri::command]
async fn toggle_dictation(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    if is_mic_probe_active(&state) {
        return Err("Arrêtez le test micro avant de dicter.".into());
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
    Ok(build_models_status(&settings))
}

#[tauri::command]
fn get_inference_info(state: State<'_, AppState>) -> Result<inference::InferenceInfo, String> {
    let settings = state.store.load_settings().map_err(|e| e.to_string())?;
    Ok(inference::get_inference_info(settings.inference_backend()))
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
    let path = match kind.as_str() {
        "whisper" => {
            let model = WhisperModel::parse(&model_id)
                .ok_or_else(|| format!("modèle Whisper inconnu: {model_id}"))?;
            if model == settings.whisper_model() {
                return Err("Impossible de supprimer le modèle Whisper actif.".into());
            }
            model.path()
        }
        "llm" => {
            let model = llm::LlmModel::parse(&model_id)
                .ok_or_else(|| format!("modèle LLM inconnu: {model_id}"))?;
            if model == settings.llm_model() {
                return Err("Impossible de supprimer le modèle LLM actif.".into());
            }
            model.path()
        }
        _ => return Err("Type de modèle invalide (whisper ou llm).".into()),
    };

    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn set_hotkey(
    app: AppHandle,
    state: State<'_, AppState>,
    hotkey: String,
) -> Result<(), String> {
    let shortcut = hotkey::parse_shortcut(&hotkey).map_err(|e| e.to_string())?;
    let previous = state.store.load_settings().map_err(|e| e.to_string())?;
    let previous_hotkey = previous.hotkey.clone();

    register_hotkey(&app, shortcut)?;

    let mut next = previous.clone();
    next.hotkey = hotkey::format_shortcut(&shortcut);
    if let Err(err) = state.store.save_settings(&next).map_err(|e| e.to_string()) {
        let rollback_shortcut =
            hotkey::parse_shortcut(&previous_hotkey).map_err(|e| e.to_string())?;
        let _ = register_hotkey(&app, rollback_shortcut);
        return Err(err);
    }

    Ok(())
}

#[tauri::command]
fn set_hotkey_capture_active(
    app: AppHandle,
    state: State<'_, AppState>,
    active: bool,
) -> Result<(), String> {
    if active {
        suspend_global_hotkey(&app, &state)
    } else {
        resume_global_hotkey(&app, &state)
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
        .ok_or_else(|| format!("modèle Whisper inconnu: {}", settings.whisper_model))?;
    let llm_model = llm::LlmModel::parse(&settings.llm_model)
        .ok_or_else(|| format!("modèle LLM inconnu: {}", settings.llm_model))?;
    let inference_backend = InferenceBackend::parse(&settings.inference_backend)
        .ok_or_else(|| format!("backend invalide: {}", settings.inference_backend))?;

    let prev_stt = previous.stt_language.clone();
    let next_stt = settings.stt_language.clone();
    let app_for_notify = app.clone();
    let stt_language_changed = prev_stt != next_stt;
    let whisper_changed = previous.whisper_model() != whisper_model;
    let llm_changed = previous.llm_model() != llm_model;
    let inference_changed = previous.inference_backend() != inference_backend;

    let hotkey_shortcut = hotkey::parse_shortcut(&settings.hotkey).map_err(|e| e.to_string())?;
    let next_hotkey = hotkey::format_shortcut(&hotkey_shortcut);
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
    };

    let mut rollback_ctx = SettingsRollbackContext {
        hotkey_changed,
        stt_language_changed,
        whisper_invalidated: false,
    };

    state.pipeline.lock().set_auto_learn(settings.auto_learn);
    state.pipeline.lock().set_default_stt_language(stt_language);

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
        if let Err(err) = register_hotkey(&app, hotkey_shortcut) {
            if let Err(rollback_err) =
                rollback_settings(&app, &state, &previous, rollback_ctx).await
            {
                eprintln!("settings rollback failed: {rollback_err}");
            }
            return Err(err);
        }
    }

    if whisper_changed || inference_changed {
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
        if llm_changed || inference_changed || !llm_engine_is_live(&state) {
            shutdown_llm_engine(&state);
            if let Err(err) = ensure_llm_model_inner(&app, &state).await {
                if let Err(rollback_err) =
                    rollback_settings(&app, &state, &previous, rollback_ctx).await
                {
                    eprintln!("settings rollback failed: {rollback_err}");
                }
                return Err(err);
            }
        }
    } else {
        state.pipeline.lock().set_auto_edit(false);
        shutdown_llm_engine(&state);
    }

    if stt_language_changed {
        state
            .pipeline
            .lock()
            .notify_stt_language_changed(&app_for_notify);
    }

    Ok(())
}

async fn ensure_llm_model_inner(app: &AppHandle, state: &AppState) -> Result<(), String> {
    if !state.pipeline.lock().auto_edit_enabled() {
        return Ok(());
    }

    if llm_engine_is_live(state) {
        return Ok(());
    }

    if state.llm_ready.load(Ordering::SeqCst) {
        state.llm_ready.store(false, Ordering::SeqCst);
    }

    let _init_guard = state.llm_init.lock().await;

    if !state.pipeline.lock().auto_edit_enabled() {
        return Ok(());
    }

    if llm_engine_is_live(state) {
        return Ok(());
    }

    shutdown_llm_engine(state);

    let settings = state.store.load_settings().map_err(|e| e.to_string())?;
    let llm_model = settings.llm_model();
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

    let engine = tauri::async_runtime::spawn_blocking(move || {
        let mut engine = llm::LlamaEngine::start_with_config(&model_path, n_gpu_layers)?;
        if let Err(err) = engine.cleanup("bonjour", ToneProfile::Default) {
            eprintln!("llm warmup failed (non-fatal): {err}");
        }
        Ok(engine)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e: llm::LlmError| e.to_string())?;

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
) -> Result<bool, String> {
    let normalized = normalize_word(&word);
    if normalized.is_empty() {
        return Err("Le mot ne peut pas être vide.".into());
    }
    if !is_valid_dictionary_word(&normalized) {
        return Err(
            "Le mot doit contenir au moins 2 caractères et ne peut pas être uniquement numérique."
                .into(),
        );
    }

    let inserted = state
        .store
        .add_word(&normalized, DictionarySource::Manual)
        .map_err(|e| e.to_string())?;

    if inserted {
        if let Err(err) = apply_dictionary_additions(&state, std::slice::from_ref(&normalized)) {
            let _ = state.store.remove_word_by_normalized(&normalized);
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
    }

    Ok(inserted)
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
        .ok_or_else(|| format!("Mot introuvable (id {id})."))?;

    let removed = state.store.remove_word(id).map_err(|e| e.to_string())?;
    if !removed {
        return Err(format!("Mot introuvable (id {id})."));
    }

    if let Err(err) = refresh_whisper_prompt_full(&state) {
        let _ = state.store.add_word(&entry.word, entry.source);
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
        return Err("Le déclencheur ne peut pas être vide.".into());
    }
    if !is_valid_trigger(&normalized_trigger) {
        return Err("Le déclencheur doit contenir au moins 2 caractères.".into());
    }
    if !is_valid_snippet_content(&content) {
        return Err("Le texte du snippet ne peut pas être vide.".into());
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
fn remove_snippet(app: AppHandle, state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let entry = state
        .store
        .get_snippet_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Snippet introuvable (id {id})."))?;

    let removed = state.store.remove_snippet(id).map_err(|e| e.to_string())?;
    if !removed {
        return Err(format!("Snippet introuvable (id {id})."));
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
    let entries: Vec<SnippetImport> =
        serde_json::from_str(&json).map_err(|err| format!("JSON invalide : {err}"))?;
    if entries.is_empty() {
        return Err("Le fichier ne contient aucun snippet.".into());
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
        return Err(
            "Aucun snippet valide importé (déclencheur ≥ 2 caractères, texte non vide).".into(),
        );
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
        return Err("Le motif doit contenir au moins 2 caractères.".into());
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
        return Err(format!("Règle introuvable (id {id})."));
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
) -> Result<Vec<DictationEntry>, String> {
    let limit = limit.unwrap_or(DEFAULT_LIST_LIMIT).clamp(1, 200);
    state
        .store
        .search_dictations(&query, limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn copy_dictation(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let entry = state
        .store
        .get_dictation(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Dictée introuvable (id {id})."))?;
    TextInjector::copy_to_clipboard(&entry.text).map_err(|e| e.to_string())
}

#[tauri::command]
fn reinject_dictation(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let entry = state
        .store
        .get_dictation(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Dictée introuvable (id {id})."))?;
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
async fn start_mic_probe(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if state.mic_probe.capture.lock().is_some() {
        return Err("Le test micro est déjà en cours.".into());
    }
    if state.pipeline.lock().state() != PipelineState::Idle {
        return Err("Une dictée est en cours. Arrêtez-la avant de tester le micro.".into());
    }
    suspend_global_hotkey(&app, &state)?;

    let mut capture = audio::AudioCapture::new().map_err(|e| e.to_string())?;
    let (level_tx, level_rx) = std::sync::mpsc::channel::<f32>();
    if let Err(err) = capture.start_with_streaming(None, Some(level_tx)) {
        let _ = resume_global_hotkey(&app, &state);
        return Err(err.to_string());
    }

    let app_clone = app.clone();
    let level_task = tauri::async_runtime::spawn(async move {
        while let Ok(level) = level_rx.recv() {
            let _ = app_clone.emit("audio-level", pipeline::AudioLevelEvent { level });
        }
    });

    let mut capture_slot = state.mic_probe.capture.lock();
    *state.mic_probe.level_task.lock() = Some(level_task);
    *capture_slot = Some(capture);
    Ok(())
}

#[tauri::command]
async fn stop_mic_probe(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
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
        resume_global_hotkey(&app, &state)?;
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
    sync_autostart_menu(&app);
    Ok(())
}

async fn ensure_model_inner(app: &AppHandle, state: &AppState) -> Result<(), String> {
    if state.model_ready.load(Ordering::SeqCst) {
        let _ = app.emit("model-ready", ());
        return Ok(());
    }

    let _init_guard = state.model_init.lock().await;

    if state.model_ready.load(Ordering::SeqCst) {
        return Ok(());
    }

    let settings = state.store.load_settings().map_err(|e| e.to_string())?;
    let whisper_model = settings.whisper_model();
    let use_gpu = inference::should_use_gpu(settings.inference_backend());

    let app_for_download = app.clone();
    let model_path = tauri::async_runtime::spawn_blocking(move || {
        stt::ensure_model_blocking(Some(&app_for_download), whisper_model)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let engine = tauri::async_runtime::spawn_blocking(move || {
        stt::WhisperEngine::new_with_gpu(&model_path, use_gpu)
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

fn handle_hotkey(app: &AppHandle, shortcut_state: ShortcutState) {
    let state = app.state::<AppState>();
    if is_mic_probe_active(&state) {
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
                PipelineState::Transcribing | PipelineState::Injecting => {}
            }
        }
        ShortcutState::Released => {
            let mut press = state.hotkey_press.lock();
            if !press.shortcut_down {
                return;
            }
            press.shortcut_down = false;

            let Some(start) = press.press_start.take() else {
                return;
            };
            let was_idle = press.was_idle_on_press;
            let duration = start.elapsed();

            if press.deferred_start_pending {
                press.deferred_toggle_intent = hotkey::is_toggle_tap(was_idle, duration);
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
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn sync_autostart_menu(app: &AppHandle) {
    let enabled = app.autolaunch().is_enabled().unwrap_or(false);
    if let Some(handles) = app.try_state::<TrayHandles>() {
        let _ = handles.autostart_item.set_checked(enabled);
    }
}

fn build_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let open_item = MenuItem::with_id(app, MENU_OPEN, "Ouvrir Calliop", true, None::<&str>)?;
    let toggle_item = MenuItem::with_id(
        app,
        MENU_TOGGLE,
        "Démarrer / arrêter la dictée",
        true,
        None::<&str>,
    )?;
    let autostart_checked = app.autolaunch().is_enabled().unwrap_or(false);
    let autostart_item = CheckMenuItem::with_id(
        app,
        MENU_AUTOSTART,
        "Lancer au démarrage",
        true,
        autostart_checked,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(app, MENU_QUIT, "Quitter", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &open_item,
            &toggle_item,
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

    app.manage(TrayHandles { autostart_item });

    Ok(())
}

fn spawn_update_check_if_enabled(
    app: AppHandle,
    store: Arc<Store>,
    pipeline: Arc<Mutex<PipelineOrchestrator>>,
) {
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
                let version = update.version.clone();
                let _ = app.emit("update-available", version.clone());
                wait_for_pipeline_idle(&pipeline).await;
                let _ = app
                    .notification()
                    .builder()
                    .title("Calliop")
                    .body(format!(
                        "Mise à jour {version} disponible. Téléchargement en cours…"
                    ))
                    .show();
                if let Err(err) = update.download_and_install(|_, _| {}, || {}).await {
                    eprintln!("auto-update: download failed: {err}");
                }
            }
            Ok(None) => {}
            Err(err) => eprintln!("auto-update: check failed: {err}"),
        }
    });
}

async fn wait_for_pipeline_idle(pipeline: &Arc<Mutex<PipelineOrchestrator>>) {
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

    if let Err(err) = stt::remove_legacy_medium_model() {
        eprintln!("failed to remove legacy whisper medium model: {err}");
    }

    pipeline.lock().set_auto_edit(initial_settings.auto_edit);
    pipeline.lock().set_auto_learn(initial_settings.auto_learn);
    pipeline
        .lock()
        .set_default_stt_language(initial_settings.stt_language_mode());

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
    pipeline.lock().set_history_store(Arc::clone(&store));

    let whisper_engine = Arc::new(Mutex::new(None));
    let llm_engine = Arc::new(Mutex::new(None));
    let llm_ready = Arc::new(AtomicBool::new(false));
    let model_init = Arc::new(tokio::sync::Mutex::new(()));
    let llm_init = Arc::new(tokio::sync::Mutex::new(()));
    let start_minimized = should_start_minimized();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
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
            }),
            deferred_llm_on_boot: AtomicBool::new(start_minimized && initial_settings.auto_edit),
            current_hotkey: Mutex::new(hotkey::default_shortcut()),
            hotkey_suspend_depth: AtomicU32::new(0),
            mic_probe: MicProbeState {
                capture: Mutex::new(None),
                level_task: Mutex::new(None),
            },
        })
        .setup(move |app| {
            let _modules = registered_modules();
            build_tray(app.handle()).map_err(|e| e.to_string())?;

            let hotkey_setting = initial_settings.hotkey.clone();
            let shortcut = hotkey::parse_shortcut(&hotkey_setting).unwrap_or_else(|err| {
                eprintln!("invalid stored hotkey ({hotkey_setting}): {err}, using default");
                hotkey::default_shortcut()
            });
            register_hotkey(app.handle(), shortcut)?;

            if let Some(main) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                main.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                });
            }

            if should_start_minimized() {
                hide_main_window(app.handle());
            }

            if !start_minimized {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<AppState>();
                    if let Err(err) = ensure_model(app_handle.clone(), state).await {
                        eprintln!("model initialization failed: {err}");
                        let _ = app_handle.emit("model-init-error", err);
                    }
                });

                if initial_settings.auto_edit {
                    let app_handle = app.handle().clone();
                    tauri::async_runtime::spawn(async move {
                        let state = app_handle.state::<AppState>();
                        if let Err(err) = ensure_llm_model(app_handle.clone(), state).await {
                            eprintln!("llm model initialization failed: {err}");
                        }
                    });
                }
            }

            {
                let app_for_update = app.handle().clone();
                let state = app.state::<AppState>();
                spawn_update_check_if_enabled(
                    app_for_update,
                    Arc::clone(&state.store),
                    Arc::clone(&state.pipeline),
                );
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_pipeline_state,
            toggle_dictation,
            ensure_model,
            ensure_llm_model,
            get_settings,
            set_settings,
            get_models_status,
            delete_model,
            get_inference_info,
            set_hotkey,
            set_hotkey_capture_active,
            is_onboarding_done,
            set_onboarding_done,
            start_mic_probe,
            stop_mic_probe,
            get_stt_language,
            cycle_dictation_language,
            is_autostart_enabled,
            set_autostart_enabled,
            list_dictionary_words,
            add_dictionary_word,
            remove_dictionary_word,
            learn_from_correction,
            list_snippets,
            add_snippet,
            remove_snippet,
            import_snippets,
            export_snippets,
            get_active_window,
            list_app_context_rules,
            add_app_context_rule,
            remove_app_context_rule,
            list_dictations,
            search_dictations,
            copy_dictation,
            reinject_dictation,
            get_insights
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app, event| {
            if let RunEvent::Ready = event {
                sync_autostart_menu(app);
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
            ]
        );
    }
}
