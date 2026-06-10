//! Global hotkey registration and handling (Phase 1+).

mod shortcut;

pub use shortcut::{default_shortcut, shortcut_label, DEFAULT_HOTKEY_ID};

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
