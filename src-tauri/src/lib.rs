pub mod app_context;
pub mod audio;
pub mod hotkey;
pub mod inject;
pub mod llm;
pub mod observe;
pub mod pipeline;
pub mod store;
pub mod stt;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use calliop_prompt::ToneProfile;
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
    DictionaryWord, Insights, NewAppContextRule, Snippet, SnippetImport, Store, DEFAULT_LIST_LIMIT,
};
use stt::{build_whisper_initial_prompt, SttLanguage};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, RunEvent, State, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_notification::NotificationExt;

/// Returns registered core module names (Phase 0 wiring check).
pub fn registered_modules() -> [&'static str; 9] {
    [
        audio::module_name(),
        stt::module_name(),
        llm::module_name(),
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
}

struct TrayHandles {
    autostart_item: CheckMenuItem<tauri::Wry>,
}

struct AppState {
    pipeline: Arc<Mutex<PipelineOrchestrator>>,
    whisper_engine: Arc<Mutex<Option<stt::WhisperEngine>>>,
    llm_engine: Arc<Mutex<Option<llm::LlamaEngine>>>,
    store: Arc<Store>,
    model_ready: AtomicBool,
    model_init: Arc<tokio::sync::Mutex<()>>,
    llm_ready: Arc<AtomicBool>,
    llm_init: Arc<tokio::sync::Mutex<()>>,
    hotkey_press: Mutex<HotkeyPressState>,
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
        if let Err(err) = ensure_llm_model(app.clone(), state).await {
            eprintln!("llm recovery after invalidation failed: {err}");
        }
    });
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SettingsPayload {
    auto_edit: bool,
    auto_learn: bool,
    stt_language: String,
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

fn refresh_whisper_prompt(
    store: &Store,
    pipeline: &Arc<Mutex<PipelineOrchestrator>>,
) -> Result<(), String> {
    let snippets = store.list_snippets().map_err(|e| e.to_string())?;
    {
        pipeline.lock().set_snippets(snippets.clone());
    }

    let words = store.list_words().map_err(|e| e.to_string())?;
    let snippet_triggers: Vec<String> = snippets
        .iter()
        .map(|snippet| snippet.trigger.clone())
        .collect();
    let dictionary_words: Vec<String> = words.into_iter().map(|entry| entry.word).collect();
    let prompt = build_whisper_initial_prompt(&snippet_triggers, &dictionary_words);
    pipeline.lock().set_dictionary_prompt(prompt);
    Ok(())
}

fn ensure_pipeline_snippets_loaded(store: &Store, pipeline: &Arc<Mutex<PipelineOrchestrator>>) {
    match store.list_snippets() {
        Ok(snippets) => pipeline.lock().set_snippets(snippets),
        Err(err) => eprintln!("failed to load snippets cache: {err}"),
    }
}

fn refresh_whisper_prompt_state(state: &AppState) -> Result<(), String> {
    refresh_whisper_prompt(&state.store, &state.pipeline)
}

fn refresh_dictionary_prompt(
    store: &Store,
    pipeline: &Arc<Mutex<PipelineOrchestrator>>,
) -> Result<(), String> {
    refresh_whisper_prompt(store, pipeline)
}

fn refresh_dictionary_prompt_state(state: &AppState) -> Result<(), String> {
    refresh_whisper_prompt_state(state)
}

pub(crate) fn apply_learned_correction(
    app: &AppHandle,
    store: &Store,
    pipeline: &Arc<Mutex<PipelineOrchestrator>>,
    original: &str,
    corrected: &str,
) -> Result<Vec<String>, String> {
    let candidates = extract_correction_words(original, corrected);
    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    let mut added = Vec::new();
    for word in candidates {
        let inserted = store
            .add_word(&word, DictionarySource::Learned)
            .map_err(|e| e.to_string())?;
        if inserted {
            added.push(word);
        }
    }

    if !added.is_empty() {
        if let Err(err) = refresh_dictionary_prompt(store, pipeline) {
            for word in &added {
                let _ = store.remove_word_by_normalized(word);
            }
            return Err(err);
        }
        send_dictionary_notification(app, &added);
        emit_dictionary_updated(app);
    }

    Ok(added)
}

fn emit_dictionary_updated(app: &AppHandle) {
    let _ = app.emit("dictionary-updated", ());
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

fn send_dictionary_notification(app: &AppHandle, words: &[String]) {
    if words.is_empty() {
        return;
    }

    let body = if words.len() == 1 {
        format!("Mot ajouté au dictionnaire : {}", words[0])
    } else {
        format!(
            "{} mots ajoutés au dictionnaire : {}",
            words.len(),
            words.join(", ")
        )
    };

    let _ = app
        .notification()
        .builder()
        .title("Calliop")
        .body(body)
        .show();
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
fn toggle_dictation(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    spawn_toggle(app.clone(), state.pipeline.clone());
    Ok(())
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<SettingsPayload, String> {
    let settings = state.store.load_settings().map_err(|e| e.to_string())?;
    Ok(SettingsPayload {
        auto_edit: settings.auto_edit,
        auto_learn: settings.auto_learn,
        stt_language: settings.stt_language,
    })
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
    let prev_stt = previous.stt_language.clone();
    let next_stt = settings.stt_language.clone();
    let app_for_notify = app.clone();
    let stt_language_changed = prev_stt != next_stt;

    let rollback = || {
        state
            .pipeline
            .lock()
            .set_default_stt_language(previous.stt_language_mode());
        state.pipeline.lock().set_auto_learn(previous.auto_learn);
        state.pipeline.lock().set_auto_edit(previous.auto_edit);
        if !previous.auto_edit {
            shutdown_llm_engine(&state);
        }
        if stt_language_changed {
            state
                .pipeline
                .lock()
                .notify_stt_language_changed(&app_for_notify);
        }
    };

    state.pipeline.lock().set_auto_learn(settings.auto_learn);
    state.pipeline.lock().set_default_stt_language(stt_language);

    if settings.auto_edit {
        state.pipeline.lock().set_auto_edit(true);

        if let Err(err) = ensure_llm_model(app, state.clone()).await {
            rollback();
            shutdown_llm_engine(&state);
            return Err(err);
        }
    } else {
        state.pipeline.lock().set_auto_edit(false);
    }

    if let Err(err) = state
        .store
        .save_settings(&AppSettings {
            auto_edit: settings.auto_edit,
            auto_learn: settings.auto_learn,
            stt_language: next_stt,
        })
        .map_err(|e| e.to_string())
    {
        rollback();
        return Err(err);
    }

    if !settings.auto_edit {
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

#[tauri::command]
async fn ensure_llm_model(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if !state.pipeline.lock().auto_edit_enabled() {
        return Ok(());
    }

    if llm_engine_is_live(&state) {
        return Ok(());
    }

    if state.llm_ready.load(Ordering::SeqCst) {
        state.llm_ready.store(false, Ordering::SeqCst);
    }

    let _init_guard = state.llm_init.lock().await;

    if !state.pipeline.lock().auto_edit_enabled() {
        return Ok(());
    }

    if llm_engine_is_live(&state) {
        return Ok(());
    }

    shutdown_llm_engine(&state);

    let app_for_download = app.clone();
    let _model_path = tauri::async_runtime::spawn_blocking(move || {
        llm::ensure_llm_model_blocking(Some(&app_for_download))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    if !state.pipeline.lock().auto_edit_enabled() {
        return Ok(());
    }

    let engine = tauri::async_runtime::spawn_blocking(|| {
        let mut engine = llm::LlamaEngine::start()?;
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
        if let Err(err) = refresh_dictionary_prompt_state(&state) {
            let _ = state.store.remove_word_by_normalized(&normalized);
            return Err(err);
        }
        emit_dictionary_updated(&app);
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

    if let Err(err) = refresh_dictionary_prompt_state(&state) {
        let _ = state.store.add_word(&entry.word, entry.source);
        return Err(err);
    }

    emit_dictionary_updated(&app);
    Ok(())
}

#[tauri::command]
fn learn_from_correction(
    app: AppHandle,
    state: State<'_, AppState>,
    original: String,
    corrected: String,
) -> Result<Vec<String>, String> {
    apply_learned_correction(&app, &state.store, &state.pipeline, &original, &corrected)
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

#[tauri::command]
async fn ensure_model(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if state.model_ready.load(Ordering::SeqCst) {
        return Ok(());
    }

    let _init_guard = state.model_init.lock().await;

    if state.model_ready.load(Ordering::SeqCst) {
        return Ok(());
    }

    let app_for_download = app.clone();
    let model_path = tauri::async_runtime::spawn_blocking(move || {
        stt::ensure_model_blocking(Some(&app_for_download))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let engine = tauri::async_runtime::spawn_blocking(move || stt::WhisperEngine::new(&model_path))
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

fn handle_hotkey(app: &AppHandle, shortcut_state: ShortcutState) {
    let state = app.state::<AppState>();

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
                PipelineState::Idle => spawn_start(app.clone(), state.pipeline.clone()),
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
                let state = app.state::<AppState>();
                spawn_toggle(app.clone(), state.pipeline.clone());
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let pipeline = Arc::new(Mutex::new(
        PipelineOrchestrator::new().expect("failed to initialize pipeline orchestrator"),
    ));
    let store = Arc::new(Store::open().expect("failed to open settings store"));
    let initial_settings = store
        .load_settings()
        .expect("failed to load initial settings");
    pipeline.lock().set_auto_edit(initial_settings.auto_edit);
    pipeline.lock().set_auto_learn(initial_settings.auto_learn);
    pipeline
        .lock()
        .set_default_stt_language(initial_settings.stt_language_mode());

    {
        let store = Arc::clone(&store);
        let pipeline_arc = Arc::clone(&pipeline);
        let handler = Arc::new(move |app: &AppHandle, original: &str, corrected: &str| {
            if !pipeline_arc.lock().auto_learn_enabled() {
                return;
            }
            if let Err(err) =
                apply_learned_correction(app, &store, &pipeline_arc, original, corrected)
            {
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

    if let Err(err) = refresh_whisper_prompt(&store, &pipeline) {
        eprintln!("failed to load whisper prompt and snippets cache: {err}");
        ensure_pipeline_snippets_loaded(&store, &pipeline);
    }
    pipeline.lock().set_history_store(Arc::clone(&store));

    let whisper_engine = Arc::new(Mutex::new(None));
    let llm_engine = Arc::new(Mutex::new(None));
    let llm_ready = Arc::new(AtomicBool::new(false));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
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
            model_ready: AtomicBool::new(false),
            model_init: Arc::new(tokio::sync::Mutex::new(())),
            llm_ready: llm_ready.clone(),
            llm_init: Arc::new(tokio::sync::Mutex::new(())),
            hotkey_press: Mutex::new(HotkeyPressState {
                press_start: None,
                was_idle_on_press: false,
                shortcut_down: false,
            }),
        })
        .setup(move |app| {
            let _modules = registered_modules();
            build_tray(app.handle()).map_err(|e| e.to_string())?;

            let shortcut = hotkey::default_shortcut();
            app.handle()
                .global_shortcut()
                .register(shortcut)
                .map_err(|e| e.to_string())?;

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

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_pipeline_state,
            toggle_dictation,
            ensure_model,
            ensure_llm_model,
            get_settings,
            set_settings,
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
