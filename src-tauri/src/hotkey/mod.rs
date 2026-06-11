//! Global hotkey registration and handling (Phase 1+).

mod shortcut;

pub use shortcut::{
    default_shortcut, shortcut_label, should_stop_ptt_on_release, DEFAULT_HOTKEY_ID,
    PTT_RELEASE_THRESHOLD,
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
