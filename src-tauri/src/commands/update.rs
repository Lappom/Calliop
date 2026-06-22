use std::sync::atomic::Ordering;
use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::update;
use crate::{fetch_available_update, run_update_download, wait_for_pipeline_idle, AppState};

#[tauri::command]
pub async fn check_for_updates(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<update::UpdateCheckResult, String> {
    if cfg!(debug_assertions) {
        return Ok(update::UpdateCheckResult::UnavailableInDev);
    }

    if let Some(version) = state
        .pending_update
        .lock()
        .as_ref()
        .map(|pending| pending.version.clone())
    {
        return Ok(update::UpdateCheckResult::Ready { version });
    }

    if state.update_check_in_progress.swap(true, Ordering::SeqCst) {
        return Err("Une vérification de mise à jour est déjà en cours.".into());
    }

    let update = match fetch_available_update(&app, true).await {
        Ok(Some(update)) => update,
        Ok(None) => {
            state
                .update_check_in_progress
                .store(false, Ordering::SeqCst);
            return Ok(update::UpdateCheckResult::UpToDate);
        }
        Err(err) => {
            state
                .update_check_in_progress
                .store(false, Ordering::SeqCst);
            return Err(err);
        }
    };

    let version = update.version.clone();
    let store = Arc::clone(&state.store);
    let app_for_download = app.clone();

    tauri::async_runtime::spawn(async move {
        run_update_download(app_for_download, store, update).await;
    });

    Ok(update::UpdateCheckResult::Downloading { version })
}

#[tauri::command]
pub fn get_pending_update_version(state: State<'_, AppState>) -> Option<String> {
    state
        .pending_update
        .lock()
        .as_ref()
        .map(|pending| pending.version.clone())
}

#[tauri::command]
pub async fn install_pending_update(state: State<'_, AppState>) -> Result<(), String> {
    let pending = state.pending_update.lock().take();
    let Some(pending) = pending else {
        return Err("Aucune mise à jour en attente.".into());
    };

    wait_for_pipeline_idle(&state.pipeline).await;

    let bytes = update::read_pending_update_bytes(&pending.bytes_path)?;
    update::mark_show_after_update();
    update::clear_dismissed_update_version();
    let result = pending.update.install(&bytes);
    pending.clear();
    result.map_err(|err| format!("Échec de l'installation : {err}"))
}

#[tauri::command]
pub fn dismiss_pending_update(state: State<'_, AppState>) -> Result<(), String> {
    let dismissed_version = state.pending_update.lock().take().map(|pending| {
        let version = pending.version.clone();
        pending.clear();
        version
    });
    if let Some(version) = dismissed_version {
        update::mark_update_dismissed(&version);
    } else {
        update::clear_pending_update_files();
    }
    Ok(())
}
