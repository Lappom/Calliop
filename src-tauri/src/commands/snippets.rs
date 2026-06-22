use tauri::{AppHandle, State};

use crate::inject::TextInjector;
use crate::pipeline::{expand_snippet_variables, SnippetVariableContext};
use crate::store::{
    is_valid_snippet_content, is_valid_trigger, normalize_trigger, SnippetImport,
};
use crate::user_error::{user_error_string, UserError};
use crate::{
    emit_snippets_updated, refresh_whisper_prompt_state, snippet_to_payload, AppState,
    SnippetPayload,
};

#[tauri::command]
pub fn list_snippets(state: State<'_, AppState>) -> Result<Vec<SnippetPayload>, String> {
    state
        .store
        .list_snippets()
        .map(|snippets| snippets.into_iter().map(snippet_to_payload).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_snippet(
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
pub fn update_snippet(
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
pub fn remove_snippet(app: AppHandle, state: State<'_, AppState>, id: i64) -> Result<(), String> {
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
pub fn import_snippets(
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
pub fn export_snippets(state: State<'_, AppState>) -> Result<String, String> {
    let entries = state
        .store
        .export_snippet_imports()
        .map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_snippet_user_name(state: State<'_, AppState>) -> Result<String, String> {
    state
        .store
        .get_snippet_user_name()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_snippet_user_name(state: State<'_, AppState>, name: String) -> Result<(), String> {
    state
        .store
        .set_snippet_user_name(&name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn preview_snippet_expansion(
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
