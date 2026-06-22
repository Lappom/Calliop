use tauri::{AppHandle, Manager, State};

use crate::{
    apply_autostart, parse_stt_language, parse_ui_language, refresh_perf_config,
    register_hotkey_binding, resume_global_hotkey, rollback_settings, settings_to_payload,
    should_start_minimized, shutdown_llm_engine, suspend_global_hotkey, sync_tray_menus,
    AppState, SettingsPayload, SettingsRollbackContext, WhisperSettingsChangeGuard,
};
use crate::hotkey;
use crate::llm;
use crate::pipeline::PipelineState;
use crate::store::{AppSettings, InferenceBackend};
use crate::stt::WhisperModel;
use crate::system;
use crate::user_error::{user_error_string, UserError};
use crate::{
    emit_model_unready_if_needed, ensure_llm_model_file, ensure_llm_model_inner,
    ensure_model_inner, ensure_whisper_model_file, invalidate_whisper_engine, llm_engine_is_live,
    llm_engine_stale,
};
use std::sync::atomic::Ordering;
use tauri::Emitter;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<SettingsPayload, String> {
    let settings = state.store.load_settings().map_err(|e| e.to_string())?;
    Ok(settings_to_payload(&settings))
}

#[tauri::command]
pub async fn set_hotkey(
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
pub fn set_hotkey_capture_active(
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
pub async fn set_settings(
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
        if state.pipeline.lock().state() != PipelineState::Idle {
            if let Err(rollback_err) =
                rollback_settings(&app, &state, &previous, rollback_ctx).await
            {
                eprintln!("settings rollback failed: {rollback_err}");
            }
            return Err(user_error_string(UserError::PipelineBusy));
        }
        let was_live = invalidate_whisper_engine(&state, true);
        emit_model_unready_if_needed(&app, was_live);
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

#[tauri::command]
pub fn is_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    let state = app.state::<AppState>();
    state.store.get_autostart().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_autostart_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    state
        .store
        .set_autostart(enabled)
        .map_err(|e| e.to_string())?;
    apply_autostart(&app, enabled)
}
