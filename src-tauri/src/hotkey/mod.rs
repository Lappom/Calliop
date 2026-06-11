//! Global hotkey registration and handling (Phase 1+).

mod shortcut;

pub use shortcut::{
    default_shortcut, format_shortcut, is_toggle_tap, parse_shortcut, shortcut_label,
    shortcut_label_from_setting, should_start_after_deferred_load, should_stop_ptt_on_release,
    ShortcutParseError, DEFAULT_HOTKEY_ID, DEFAULT_HOTKEY_SETTING, PTT_RELEASE_THRESHOLD,
};

pub fn module_name() -> &'static str {
    "hotkey"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_module_name() {
        assert_eq!(module_name(), "hotkey");
    }
}
