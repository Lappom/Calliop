//! Pipeline orchestration across capture, STT, LLM, and injection (Phase 1+).

mod orchestrator;
mod snippets;

pub use orchestrator::{
    hide_overlay, show_overlay, spawn_start, spawn_stop, spawn_toggle, AudioLevelEvent,
    LatencyMetricsEvent, PartialTranscriptEvent, PipelineError, PipelineOrchestrator,
    PipelineState, PipelineStateEvent,
};
pub use snippets::apply_snippets;

pub fn module_name() -> &'static str {
    "pipeline"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_module_name() {
        assert_eq!(module_name(), "pipeline");
    }
}
