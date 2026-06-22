use tauri::State;

use crate::inject::TextInjector;
use crate::store::{DictationEntry, Insights, DEFAULT_LIST_LIMIT};
use crate::user_error::{user_error_string, UserError};
use crate::AppState;

#[tauri::command]
pub fn list_dictations(
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
pub fn search_dictations(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<DictationEntry>, String> {
    let limit = limit.unwrap_or(DEFAULT_LIST_LIMIT).clamp(1, 200);
    let offset = offset.unwrap_or(0);
    state
        .store
        .search_dictations(&query, limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn count_dictations(state: State<'_, AppState>) -> Result<i64, String> {
    state.store.count_dictations().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn count_search_dictations(state: State<'_, AppState>, query: String) -> Result<i64, String> {
    state
        .store
        .count_search_dictations(&query)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn copy_dictation(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let entry = state
        .store
        .get_dictation(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| user_error_string(UserError::DictationNotFound))?;
    TextInjector::copy_to_clipboard(&entry.text).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reinject_dictation(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let entry = state
        .store
        .get_dictation(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| user_error_string(UserError::DictationNotFound))?;
    let injector = TextInjector::new().map_err(|e| e.to_string())?;
    injector.inject(&entry.text).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_insights(state: State<'_, AppState>) -> Result<Insights, String> {
    state.store.get_insights().map_err(|e| e.to_string())
}
