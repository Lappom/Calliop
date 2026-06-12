//! Native hotkey capture while the user records a shortcut in Settings.

#[cfg(windows)]
pub fn start(app: &tauri::AppHandle) -> Result<bool, String> {
    super::capture_win::start(app)
}

#[cfg(windows)]
pub fn stop() -> Result<(), String> {
    super::capture_win::stop()
}

#[cfg(not(windows))]
pub fn start(_app: &tauri::AppHandle) -> Result<bool, String> {
    Ok(false)
}

#[cfg(not(windows))]
pub fn stop() -> Result<(), String> {
    Ok(())
}
