use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager, State};

use crate::pipeline::{request_dictation, DictationIntent, PipelineState};
use crate::user_error::{user_error_string, UserError};
use crate::{
    ensure_llm_model_inner, ensure_model_inner, ensure_model_then_toggle,
    is_dictation_start_blocked_by_settings, is_mic_probe_active,
    notify_dictation_blocked_by_settings, resume_global_hotkey, stop_mic_probe_inner,
    wait_for_pipeline_idle_timeout, whisper_is_live, AppState,
};

#[tauri::command]
pub fn get_pipeline_state(state: State<'_, AppState>) -> String {
    state.pipeline.lock().state().as_str().to_string()
}

#[tauri::command]
pub fn is_model_ready(state: State<'_, AppState>) -> bool {
    whisper_is_live(state.inner())
}

#[tauri::command]
pub async fn toggle_dictation(app: AppHandle) -> Result<(), String> {
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
pub fn get_stt_language(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state
        .pipeline
        .lock()
        .effective_stt_language()
        .as_setting_value())
}

#[tauri::command]
pub fn cycle_dictation_language(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let language = state
        .pipeline
        .lock()
        .cycle_session_language(&app)
        .map_err(|e| e.to_string())?;
    Ok(language.as_setting_value())
}

#[tauri::command]
pub async fn ensure_llm_model(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    ensure_llm_model_inner(&app, &state).await
}

#[tauri::command]
pub async fn ensure_model(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    ensure_model_inner(&app, &state).await
}

#[tauri::command]
pub async fn prepare_onboarding_dictation(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    stop_mic_probe_inner(&app, &state)?;

    while state.hotkey_suspend_depth.load(Ordering::SeqCst) > 0 {
        resume_global_hotkey(&app, &state)?;
    }

    let pipeline = state.pipeline.clone();
    if pipeline.lock().state() != PipelineState::Idle {
        request_dictation(app.clone(), Arc::clone(&pipeline), DictationIntent::Stop);
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
