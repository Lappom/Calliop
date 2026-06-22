use tauri::{AppHandle, Emitter, State};

use crate::audio;
use crate::pipeline::PipelineState;
use crate::user_error::{user_error_string, UserError};
use crate::{resume_global_hotkey, stop_mic_probe_inner, suspend_global_hotkey, AppState};

#[tauri::command]
pub fn list_input_devices() -> Result<Vec<audio::InputDeviceInfo>, String> {
    audio::list_input_devices().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_mic_probe(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
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
    let (level_tx, level_rx) = std::sync::mpsc::sync_channel::<audio::AudioLevelSample>(8);
    if let Err(err) = capture.start_with_streaming(None, Some(level_tx), Some(&input_device)) {
        let _ = resume_global_hotkey(&app, &state);
        return Err(err.to_string());
    }

    let app_clone = app.clone();
    let level_task = tauri::async_runtime::spawn(async move {
        while let Ok(sample) = level_rx.recv() {
            let _ = app_clone.emit(
                "audio-level",
                crate::pipeline::AudioLevelEvent {
                    level: sample.level,
                    bands: sample.bands,
                },
            );
        }
    });

    let mut capture_slot = state.mic_probe.capture.lock();
    *state.mic_probe.level_task.lock() = Some(level_task);
    *capture_slot = Some(capture);
    Ok(())
}

#[tauri::command]
pub async fn stop_mic_probe(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    stop_mic_probe_inner(&app, &state)
}
