use tauri::{AppHandle, Emitter, State};

use crate::inference;
use crate::llm;
use crate::stt::WhisperModel;
use crate::system;
use crate::user_error::{user_error_string, UserError};
use crate::{
    build_models_status, emit_model_unready_if_needed, ensure_llm_model_file,
    ensure_llm_model_inner, ensure_model_inner, ensure_whisper_model_file,
    invalidate_whisper_engine, shutdown_llm_engine, AppState, ModelsStatusPayload,
};

#[tauri::command]
pub fn get_models_status(state: State<'_, AppState>) -> Result<ModelsStatusPayload, String> {
    let settings = state.store.load_settings().map_err(|e| e.to_string())?;
    Ok(build_models_status(&state, &settings))
}

#[tauri::command]
pub fn get_inference_info(state: State<'_, AppState>) -> Result<inference::InferenceInfo, String> {
    let settings = state.store.load_settings().map_err(|e| e.to_string())?;
    Ok(inference::get_inference_info(
        &settings,
        &state.capabilities,
    ))
}

#[tauri::command]
pub async fn delete_model(
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
pub async fn reinstall_model(
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
                let was_live = invalidate_whisper_engine(&state, true);
                emit_model_unready_if_needed(&app, was_live);
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
