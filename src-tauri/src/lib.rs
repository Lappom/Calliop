pub mod audio;
pub mod hotkey;
pub mod inject;
pub mod llm;
pub mod pipeline;
pub mod store;
pub mod stt;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use pipeline::{
    spawn_start, spawn_stop, spawn_toggle, PipelineOrchestrator, PipelineState, PipelineStateEvent,
};
use serde::{Deserialize, Serialize};
use store::{AppSettings, Store};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, RunEvent, State, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

/// Returns registered core module names (Phase 0 wiring check).
pub fn registered_modules() -> [&'static str; 7] {
    [
        audio::module_name(),
        stt::module_name(),
        llm::module_name(),
        inject::module_name(),
        hotkey::module_name(),
        store::module_name(),
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
    })
}

#[tauri::command]
async fn set_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: SettingsPayload,
) -> Result<(), String> {
    let previous = state.store.load_settings().map_err(|e| e.to_string())?;

    if settings.auto_edit {
        state.pipeline.lock().set_auto_edit(true);

        if let Err(err) = ensure_llm_model(app, state.clone()).await {
            state.pipeline.lock().set_auto_edit(previous.auto_edit);
            shutdown_llm_engine(&state);
            return Err(err);
        }

        if let Err(err) = state
            .store
            .save_settings(&AppSettings {
                auto_edit: true,
            })
            .map_err(|e| e.to_string())
        {
            state.pipeline.lock().set_auto_edit(previous.auto_edit);
            shutdown_llm_engine(&state);
            return Err(err);
        }
    } else {
        state
            .store
            .save_settings(&AppSettings {
                auto_edit: false,
            })
            .map_err(|e| e.to_string())?;
        state.pipeline.lock().set_auto_edit(false);
        shutdown_llm_engine(&state);
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
        if let Err(err) = engine.cleanup("bonjour") {
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
        let mut pipeline = state.pipeline.lock();
        pipeline.set_whisper_engine(Arc::clone(&state.whisper_engine));
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

    let whisper_engine = Arc::new(Mutex::new(None));
    let llm_engine = Arc::new(Mutex::new(None));
    let llm_ready = Arc::new(AtomicBool::new(false));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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
            is_autostart_enabled,
            set_autostart_enabled
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
            ["audio", "stt", "llm", "inject", "hotkey", "store", "pipeline",]
        );
    }
}
