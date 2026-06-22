use tauri::{AppHandle, State};

use crate::app_context;
use crate::store::{is_valid_app_context_pattern, NewAppContextRule};
use crate::user_error::{user_error_string, UserError};
use crate::{
    app_context_rule_to_payload, emit_app_context_updated, parse_match_type, parse_tone_profile,
    refresh_app_context_rules_state, AppContextRulePayload, AppState,
};

#[tauri::command]
pub fn get_active_window() -> Option<app_context::ActiveWindow> {
    app_context::get_active_window()
}

#[tauri::command]
pub fn list_app_context_rules(
    state: State<'_, AppState>,
) -> Result<Vec<AppContextRulePayload>, String> {
    state
        .store
        .list_app_context_rules()
        .map(|rules| rules.into_iter().map(app_context_rule_to_payload).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_app_context_rule(
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
pub fn remove_app_context_rule(
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
