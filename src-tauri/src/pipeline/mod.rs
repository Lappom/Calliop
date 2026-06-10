//! Pipeline orchestration across capture, STT, LLM, and injection (Phase 1+).

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
