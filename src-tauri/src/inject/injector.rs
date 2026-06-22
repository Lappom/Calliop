use std::sync::LazyLock;
use std::thread;
use std::time::Duration;

use arboard::{Clipboard, ImageData};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use parking_lot::Mutex;
use thiserror::Error;

/// Default delay before restoring the user's clipboard after a simulated paste.
pub const DEFAULT_PASTE_DELAY_MS: u64 = 120;

static INJECT_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[derive(Debug, Error)]
pub enum InjectError {
    #[error("clipboard unavailable: {0}")]
    Clipboard(String),
    #[error("failed to simulate paste: {0}")]
    Paste(String),
    #[error("failed to type text: {0}")]
    TypeText(String),
}

/// How text is delivered to the focused application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InjectionStrategy {
    /// Set clipboard and simulate Ctrl+V (default).
    #[default]
    ClipboardPaste,
    /// Type Unicode characters directly (slower, no clipboard restore needed).
    TypeDirect,
}

/// Runtime options for a single injection attempt.
#[derive(Debug, Clone)]
pub struct InjectConfig {
    pub strategy: InjectionStrategy,
    pub paste_delay_ms: u64,
    /// When clipboard paste fails, try typing directly before falling back to copy-only.
    pub fallback_to_typing: bool,
}

impl Default for InjectConfig {
    fn default() -> Self {
        Self {
            strategy: InjectionStrategy::ClipboardPaste,
            paste_delay_ms: DEFAULT_PASTE_DELAY_MS,
            fallback_to_typing: true,
        }
    }
}

/// Outcome of an injection attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InjectOutcome {
    /// Text was pasted via clipboard + Ctrl+V.
    Pasted,
    /// Text was typed character-by-character.
    Typed,
    /// Paste/typing failed; text was left on the clipboard for manual paste.
    CopiedToClipboardFallback,
}

impl InjectOutcome {
    pub fn succeeded(&self) -> bool {
        !matches!(self, Self::CopiedToClipboardFallback)
    }
}

/// Saved clipboard content for restoration after paste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedClipboard {
    text: Option<String>,
    image: Option<OwnedImage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OwnedImage {
    width: usize,
    height: usize,
    bytes: Vec<u8>,
}

impl SavedClipboard {
    pub fn from_text(text: Option<String>) -> Self {
        Self {
            text,
            image: None,
        }
    }

    pub fn should_restore(&self) -> bool {
        self.text.is_some() || self.image.is_some()
    }

    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    fn from_clipboard(clipboard: &mut Clipboard) -> Result<Self, InjectError> {
        let text = clipboard.get_text().ok();
        let image = clipboard
            .get_image()
            .ok()
            .map(|img| OwnedImage {
                width: img.width,
                height: img.height,
                bytes: img.bytes.into_owned(),
            });
        Ok(Self { text, image })
    }

    fn restore(&self, clipboard: &mut Clipboard) -> Result<(), InjectError> {
        if let Some(image) = &self.image {
            let data = ImageData {
                width: image.width,
                height: image.height,
                bytes: image.bytes.clone().into(),
            };
            clipboard
                .set_image(data)
                .map_err(|e| InjectError::Clipboard(e.to_string()))?;
            return Ok(());
        }
        if let Some(text) = &self.text {
            clipboard
                .set_text(text)
                .map_err(|e| InjectError::Clipboard(e.to_string()))?;
        }
        Ok(())
    }
}

pub struct TextInjector {
    config: InjectConfig,
}

impl TextInjector {
    pub fn new() -> Result<Self, InjectError> {
        Ok(Self {
            config: InjectConfig::default(),
        })
    }

    pub fn with_config(config: InjectConfig) -> Result<Self, InjectError> {
        Ok(Self { config })
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
        SavedClipboard::from_clipboard(&mut clipboard)
    }

    pub fn copy_to_clipboard(text: &str) -> Result<(), InjectError> {
        let _guard = INJECT_MUTEX.lock();
        let mut clipboard = Self::open_clipboard()?;
        clipboard
            .set_text(text)
            .map_err(|e| InjectError::Clipboard(e.to_string()))
    }

    /// Inject text using the configured strategy. On failure, copies to clipboard and returns
    /// `CopiedToClipboardFallback` instead of propagating an error.
    pub fn inject_resilient(&self, text: &str) -> Result<InjectOutcome, InjectError> {
        let _guard = INJECT_MUTEX.lock();
        match self.config.strategy {
            InjectionStrategy::ClipboardPaste => {
                let saved = Self::read_clipboard_inner()?;
                match self.inject_clipboard_paste(text, &saved) {
                    Ok(()) => Ok(InjectOutcome::Pasted),
                    Err(paste_err) => {
                        eprintln!("clipboard paste failed: {paste_err}");
                        if self.config.fallback_to_typing {
                            match self.type_text_direct(text) {
                                Ok(()) => {
                                    let _ = saved.restore(&mut Self::open_clipboard()?);
                                    return Ok(InjectOutcome::Typed);
                                }
                                Err(type_err) => eprintln!("direct typing fallback failed: {type_err}"),
                            }
                        }
                        Self::copy_text_inner(text)?;
                        Ok(InjectOutcome::CopiedToClipboardFallback)
                    }
                }
            }
            InjectionStrategy::TypeDirect => match self.type_text_direct(text) {
                Ok(()) => Ok(InjectOutcome::Typed),
                Err(type_err) => {
                    eprintln!("direct typing failed: {type_err}");
                    Self::copy_text_inner(text)?;
                    Ok(InjectOutcome::CopiedToClipboardFallback)
                }
            },
        }
    }

    /// Legacy inject API — propagates errors. Prefer `inject_resilient` in the pipeline.
    pub fn inject(&self, text: &str) -> Result<(), InjectError> {
        match self.inject_resilient(text)? {
            InjectOutcome::CopiedToClipboardFallback => Err(InjectError::Paste(
                "injection failed; text copied to clipboard".into(),
            )),
            _ => Ok(()),
        }
    }

    pub fn inject_with_saved(&self, text: &str, saved: &SavedClipboard) -> Result<(), InjectError> {
        let _guard = INJECT_MUTEX.lock();
        self.inject_with_saved_inner(text, saved)
    }

    fn copy_text_inner(text: &str) -> Result<(), InjectError> {
        let mut clipboard = Self::open_clipboard()?;
        clipboard
            .set_text(text)
            .map_err(|e| InjectError::Clipboard(e.to_string()))
    }

    fn inject_clipboard_paste(
        &self,
        text: &str,
        saved: &SavedClipboard,
    ) -> Result<(), InjectError> {
        self.inject_with_saved_inner(text, saved)
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
        thread::sleep(Duration::from_millis(self.config.paste_delay_ms));
        match Self::open_clipboard() {
            Ok(mut clipboard) => {
                if let Err(err) = saved.restore(&mut clipboard) {
                    eprintln!("clipboard restore failed after paste: {err}");
                }
            }
            Err(err) => eprintln!("clipboard restore failed after paste: {err}"),
        }
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

    fn type_text_direct(&self, text: &str) -> Result<(), InjectError> {
        let mut enigo =
            Enigo::new(&Settings::default()).map_err(|e| InjectError::TypeText(e.to_string()))?;
        for ch in text.chars() {
            enigo
                .key(Key::Unicode(ch), Direction::Click)
                .map_err(|e| InjectError::TypeText(e.to_string()))?;
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

    #[test]
    fn inject_config_defaults() {
        let config = InjectConfig::default();
        assert_eq!(config.strategy, InjectionStrategy::ClipboardPaste);
        assert_eq!(config.paste_delay_ms, DEFAULT_PASTE_DELAY_MS);
        assert!(config.fallback_to_typing);
    }

    #[test]
    fn inject_outcome_succeeded() {
        assert!(InjectOutcome::Pasted.succeeded());
        assert!(InjectOutcome::Typed.succeeded());
        assert!(!InjectOutcome::CopiedToClipboardFallback.succeeded());
    }
}
