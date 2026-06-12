//! Global hotkey registration and handling (Phase 1+).

mod capture;
#[cfg(windows)]
mod capture_win;
#[cfg(windows)]
mod dictation_hook_win;
#[cfg(windows)]
mod modifiers;
mod shortcut;

pub use capture::{start as start_hotkey_capture, stop as stop_hotkey_capture};
pub use shortcut::{
    default_shortcut, format_hotkey_setting, format_shortcut, is_toggle_tap,
    is_valid_hotkey_setting, parse_hotkey_setting, parse_shortcut, shortcut_label,
    shortcut_label_from_setting, should_start_after_deferred_load, should_stop_ptt_on_release,
    HotkeyBinding, ShortcutParseError, DEFAULT_HOTKEY_ID, DEFAULT_HOTKEY_SETTING,
    PTT_RELEASE_THRESHOLD,
};

#[cfg(windows)]
pub(crate) use dictation_hook_win::{start_modifier_dictation_hook, stop_modifier_dictation_hook};

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
