use std::time::Duration;

use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

pub const DEFAULT_HOTKEY_ID: &str = "dictation-toggle";
/// Short press threshold: below = toggle tap, at/above on release = push-to-talk stop.
pub const PTT_RELEASE_THRESHOLD: Duration = Duration::from_millis(400);

/// Default dictation hotkey: Alt + Space (toggle / push-to-talk hybrid).
pub fn default_shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::ALT), Code::Space)
}

pub fn shortcut_label() -> &'static str {
    "Alt+Espace"
}

/// Returns true when a held push-to-talk session should stop on key release.
pub fn should_stop_ptt_on_release(was_idle_on_press: bool, press_duration: Duration) -> bool {
    was_idle_on_press && press_duration >= PTT_RELEASE_THRESHOLD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shortcut_can_be_constructed() {
        let shortcut = default_shortcut();
        assert!(format!("{shortcut:?}").contains("Space"));
    }

    #[test]
    fn short_release_after_idle_keeps_recording() {
        assert!(!should_stop_ptt_on_release(
            true,
            Duration::from_millis(100)
        ));
    }

    #[test]
    fn short_release_while_recording_does_not_stop_ptt() {
        assert!(!should_stop_ptt_on_release(
            false,
            Duration::from_millis(100)
        ));
    }

    #[test]
    fn long_release_after_idle_stops_push_to_talk() {
        assert!(should_stop_ptt_on_release(
            true,
            Duration::from_millis(500)
        ));
    }
}
