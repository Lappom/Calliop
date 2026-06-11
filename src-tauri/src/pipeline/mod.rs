//! Pipeline orchestration across capture, STT, LLM, and injection (Phase 1+).

mod corrections;
mod orchestrator;
mod snippet_variables;
mod snippets;

pub use corrections::{apply_corrections, CorrectionRule};
pub use orchestrator::{
    hide_overlay, show_overlay, spawn_start, spawn_stop, spawn_toggle, AudioLevelEvent,
    LatencyMetricsEvent, PartialTranscriptEvent, PipelineError, PipelineOrchestrator,
    PipelineState, PipelineStateEvent, SttLanguageChangedEvent,
};
pub use snippet_variables::{
    expand_snippet_variables, format_local_date_french, SnippetVariableContext,
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
