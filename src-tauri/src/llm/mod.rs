//! Local LLM post-processing for auto-edits (Phase 3+).

pub fn module_name() -> &'static str {
    "llm"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_module_name() {
        assert_eq!(module_name(), "llm");
    }
}
