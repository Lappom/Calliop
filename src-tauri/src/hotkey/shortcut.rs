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

/// Short tap from idle while the model is still loading — start when load completes.
pub fn is_toggle_tap(was_idle_on_press: bool, press_duration: Duration) -> bool {
    was_idle_on_press && press_duration < PTT_RELEASE_THRESHOLD
}

/// Whether a deferred model load should start recording once Whisper is ready.
pub fn should_start_after_deferred_load(
    still_holding: bool,
    deferred_toggle_intent: bool,
) -> bool {
    still_holding || deferred_toggle_intent
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
        assert!(should_stop_ptt_on_release(true, Duration::from_millis(500)));
    }

    #[test]
    fn toggle_tap_is_short_press_from_idle() {
        assert!(is_toggle_tap(true, Duration::from_millis(100)));
        assert!(!is_toggle_tap(true, Duration::from_millis(500)));
        assert!(!is_toggle_tap(false, Duration::from_millis(100)));
    }

    #[test]
    fn deferred_load_starts_on_hold_or_toggle_tap() {
        assert!(should_start_after_deferred_load(true, false));
        assert!(should_start_after_deferred_load(false, true));
        assert!(!should_start_after_deferred_load(false, false));
    }
}
