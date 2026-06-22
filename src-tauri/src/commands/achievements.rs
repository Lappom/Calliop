use tauri::State;

use crate::achievements::AchievementsSummary;
use crate::AppState;

#[tauri::command]
pub fn get_achievements(state: State<'_, AppState>) -> Result<AchievementsSummary, String> {
    state
        .achievements
        .get_summary()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn mark_achievements_seen(
    state: State<'_, AppState>,
    ids: Option<Vec<String>>,
) -> Result<(), String> {
    state
        .store
        .mark_achievements_seen(ids)
        .map_err(|e| e.to_string())
}
