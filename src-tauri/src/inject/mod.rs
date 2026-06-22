//! Text injection into the active application (Phase 1+).

mod injector;

pub use injector::{
    InjectConfig, InjectError, InjectOutcome, InjectionStrategy, SavedClipboard, TextInjector,
    DEFAULT_PASTE_DELAY_MS,
};

pub fn module_name() -> &'static str {
    "inject"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_module_name() {
        assert_eq!(module_name(), "inject");
    }
}
