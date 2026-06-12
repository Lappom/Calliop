use std::sync::LazyLock;
use std::thread;
use std::time::Duration;

use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use parking_lot::Mutex;
use thiserror::Error;

const PASTE_DELAY_MS: u64 = 80;

static INJECT_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[derive(Debug, Error)]
pub enum InjectError {
    #[error("clipboard unavailable: {0}")]
    Clipboard(String),
    #[error("failed to simulate paste: {0}")]
    Paste(String),
}

/// Saved clipboard content for restoration after paste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedClipboard {
    text: Option<String>,
}

impl SavedClipboard {
    pub fn from_text(text: Option<String>) -> Self {
        Self { text }
    }

    pub fn should_restore(&self) -> bool {
        self.text.is_some()
    }

    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }
}

pub struct TextInjector;

impl TextInjector {
    pub fn new() -> Result<Self, InjectError> {
        Ok(Self)
    }

    fn open_clipboard() -> Result<Clipboard, InjectError> {
        Clipboard::new().map_err(|e| InjectError::Clipboard(e.to_string()))
    }

    pub fn save_clipboard() -> Result<SavedClipboard, InjectError> {
        let _guard = INJECT_MUTEX.lock();
        Self::read_clipboard_inner()
    }

    pub fn read_clipboard_text() -> Result<Option<String>, InjectError> {
        let _guard = INJECT_MUTEX.lock();
        Self::read_clipboard_inner().map(|saved| saved.text().map(str::to_string))
    }

    fn read_clipboard_inner() -> Result<SavedClipboard, InjectError> {
        let mut clipboard = Self::open_clipboard()?;
        let text = clipboard.get_text().ok();
        Ok(SavedClipboard::from_text(text))
    }

    pub fn copy_to_clipboard(text: &str) -> Result<(), InjectError> {
        let _guard = INJECT_MUTEX.lock();
        let mut clipboard = Self::open_clipboard()?;
        clipboard
            .set_text(text)
            .map_err(|e| InjectError::Clipboard(e.to_string()))
    }

    pub fn inject(&self, text: &str) -> Result<(), InjectError> {
        let _guard = INJECT_MUTEX.lock();
        let saved = Self::save_clipboard_inner()?;
        self.inject_with_saved_inner(text, &saved)
    }

    pub fn inject_with_saved(&self, text: &str, saved: &SavedClipboard) -> Result<(), InjectError> {
        let _guard = INJECT_MUTEX.lock();
        self.inject_with_saved_inner(text, saved)
    }

    fn save_clipboard_inner() -> Result<SavedClipboard, InjectError> {
        Self::read_clipboard_inner()
    }

    fn inject_with_saved_inner(
        &self,
        text: &str,
        saved: &SavedClipboard,
    ) -> Result<(), InjectError> {
        {
            let mut clipboard = Self::open_clipboard()?;
            clipboard
                .set_text(text)
                .map_err(|e| InjectError::Clipboard(e.to_string()))?;
        }

        self.simulate_ctrl_v()?;
        thread::sleep(Duration::from_millis(PASTE_DELAY_MS));
        self.restore_clipboard(saved)?;
        Ok(())
    }

    fn simulate_ctrl_v(&self) -> Result<(), InjectError> {
        let mut enigo =
            Enigo::new(&Settings::default()).map_err(|e| InjectError::Paste(e.to_string()))?;
        enigo
            .key(Key::Control, Direction::Press)
            .map_err(|e| InjectError::Paste(e.to_string()))?;
        enigo
            .key(Key::Unicode('v'), Direction::Click)
            .map_err(|e| InjectError::Paste(e.to_string()))?;
        enigo
            .key(Key::Control, Direction::Release)
            .map_err(|e| InjectError::Paste(e.to_string()))?;
        Ok(())
    }

    fn restore_clipboard(&self, saved: &SavedClipboard) -> Result<(), InjectError> {
        if let Some(text) = saved.text() {
            let mut clipboard = Self::open_clipboard()?;
            clipboard
                .set_text(text)
                .map_err(|e| InjectError::Clipboard(e.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_clipboard_restore_when_text_present() {
        let saved = SavedClipboard::from_text(Some("previous".into()));
        assert!(saved.should_restore());
        assert_eq!(saved.text(), Some("previous"));
    }

    #[test]
    fn saved_clipboard_skip_restore_when_empty() {
        let saved = SavedClipboard::from_text(None);
        assert!(!saved.should_restore());
        assert_eq!(saved.text(), None);
    }
}
