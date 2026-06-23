//! Unified entry points for dictation pipeline actions.

use std::sync::Arc;

use parking_lot::Mutex;
use tauri::AppHandle;

use super::{spawn_cancel, spawn_start, spawn_stop, spawn_toggle, PipelineOrchestrator};

/// High-level dictation action requested by hotkey, tray, or Tauri commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictationIntent {
    Start,
    Stop,
    Toggle,
    Cancel,
}

/// Dispatch a dictation intent to the appropriate pipeline worker.
pub fn request_dictation(
    app: AppHandle,
    orchestrator: Arc<Mutex<PipelineOrchestrator>>,
    intent: DictationIntent,
) {
    match intent {
        DictationIntent::Start => spawn_start(app, orchestrator),
        DictationIntent::Stop => spawn_stop(app, orchestrator),
        DictationIntent::Toggle => spawn_toggle(app, orchestrator),
        DictationIntent::Cancel => spawn_cancel(app, orchestrator),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictation_intent_variants_distinct() {
        assert_ne!(DictationIntent::Start, DictationIntent::Stop);
        assert_ne!(DictationIntent::Toggle, DictationIntent::Cancel);
    }
}
