use tauri::{AppHandle, State};

use crate::AppState;

#[tauri::command]
pub fn is_onboarding_done(state: State<'_, AppState>) -> Result<bool, String> {
    state.store.is_onboarding_done().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_onboarding_done(
    app: AppHandle,
    state: State<'_, AppState>,
    done: bool,
) -> Result<(), String> {
    state
        .store
        .set_onboarding_done(done)
        .map_err(|e| e.to_string())?;
    if done {
        if let Err(err) = state.achievements.on_feature_change(&app) {
            eprintln!("achievement evaluation failed: {err}");
        }
    }
    Ok(())
}
