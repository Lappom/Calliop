//! Global hotkey registration and handling (Phase 1+).

mod shortcut;

pub use shortcut::{
    default_shortcut, is_toggle_tap, shortcut_label, should_start_after_deferred_load,
    should_stop_ptt_on_release, DEFAULT_HOTKEY_ID, PTT_RELEASE_THRESHOLD,
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
