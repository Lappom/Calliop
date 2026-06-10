//! Speech-to-text via whisper bindings (Phase 1+).

pub fn module_name() -> &'static str {
    "stt"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_module_name() {
        assert_eq!(module_name(), "stt");
    }
}
