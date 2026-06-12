//! In-app update helpers: post-restart window restore and pending update storage.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;
use serde::Serialize;
use tauri_plugin_updater::Update;

const SHOW_AFTER_UPDATE_FLAG: &str = ".show-after-update";
const PENDING_UPDATE_BYTES: &str = "pending-update.bin";
const DISMISSED_UPDATE_VERSION: &str = ".dismissed-update-version";

pub fn app_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.calliop.app")
}

fn show_after_update_flag_path() -> PathBuf {
    app_data_dir().join(SHOW_AFTER_UPDATE_FLAG)
}

fn pending_update_bytes_path() -> PathBuf {
    app_data_dir().join(PENDING_UPDATE_BYTES)
}

pub fn mark_show_after_update() {
    let path = show_after_update_flag_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, b"1");
}

pub fn take_show_after_update_flag() -> bool {
    let path = show_after_update_flag_path();
    if path.is_file() {
        let _ = std::fs::remove_file(path);
        true
    } else {
        false
    }
}

pub fn clear_pending_update_files() {
    let path = pending_update_bytes_path();
    if path.is_file() {
        let _ = std::fs::remove_file(path);
    }
}

pub fn dismissed_update_version() -> Option<String> {
    let path = app_data_dir().join(DISMISSED_UPDATE_VERSION);
    std::fs::read_to_string(path).ok().map(|value| value.trim().to_string())
}

pub fn mark_update_dismissed(version: &str) {
    let path = app_data_dir().join(DISMISSED_UPDATE_VERSION);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, version);
}

pub fn clear_dismissed_update_version() {
    let path = app_data_dir().join(DISMISSED_UPDATE_VERSION);
    if path.is_file() {
        let _ = std::fs::remove_file(path);
    }
}

pub fn store_pending_update_bytes(bytes: &[u8]) -> Result<PathBuf, String> {
    let path = pending_update_bytes_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    std::fs::write(&path, bytes).map_err(|err| err.to_string())?;
    Ok(path)
}

pub fn read_pending_update_bytes(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|err| err.to_string())
}

#[derive(Clone)]
pub struct PendingUpdate {
    pub version: String,
    pub update: Update,
    pub bytes_path: PathBuf,
}

impl PendingUpdate {
    pub fn clear(&self) {
        if self.bytes_path.is_file() {
            let _ = std::fs::remove_file(&self.bytes_path);
        }
    }
}

pub type PendingUpdateStore = Arc<Mutex<Option<PendingUpdate>>>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateReadyPayload {
    pub version: String,
}
