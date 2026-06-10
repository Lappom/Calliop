pub mod audio;
pub mod hotkey;
pub mod inject;
pub mod llm;
pub mod pipeline;
pub mod store;
pub mod stt;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use pipeline::{spawn_toggle, PipelineOrchestrator, PipelineState};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State};
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

struct AppState {
    pipeline: Arc<Mutex<PipelineOrchestrator>>,
    model_ready: AtomicBool,
}

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

    {
        let mut pipeline = state.pipeline.lock();
        pipeline.set_model_path(model_path);
    }

    state.model_ready.store(true, Ordering::SeqCst);
    let _ = app.emit("model-ready", ());
    let _ = app.emit(
        "pipeline-state",
        pipeline::PipelineStateEvent {
            state: PipelineState::Idle.as_str().into(),
            message: None,
        },
    );
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let pipeline = Arc::new(Mutex::new(
        PipelineOrchestrator::new().expect("failed to initialize pipeline orchestrator"),
    ));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let state = app.state::<AppState>();
                    spawn_toggle(app.clone(), state.pipeline.clone());
                })
                .build(),
        )
        .manage(AppState {
            pipeline: pipeline.clone(),
            model_ready: AtomicBool::new(false),
        })
        .setup(move |app| {
            let _modules = registered_modules();

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Calliop")
                .build(app)?;

            let shortcut = hotkey::default_shortcut();
            app.handle()
                .global_shortcut()
                .register(shortcut)
                .map_err(|e| e.to_string())?;

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = app_handle.state::<AppState>();
                if let Err(err) = ensure_model(app_handle.clone(), state).await {
                    eprintln!("model initialization failed: {err}");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_pipeline_state,
            toggle_dictation,
            ensure_model
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
