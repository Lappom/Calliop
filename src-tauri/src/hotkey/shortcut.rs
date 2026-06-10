use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

pub const DEFAULT_HOTKEY_ID: &str = "dictation-toggle";

/// Default dictation hotkey: Alt + Space (toggle).
pub fn default_shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::ALT), Code::Space)
}

pub fn shortcut_label() -> &'static str {
    "Alt+Espace"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shortcut_can_be_constructed() {
        let shortcut = default_shortcut();
        assert!(format!("{shortcut:?}").contains("Space"));
    }
}
