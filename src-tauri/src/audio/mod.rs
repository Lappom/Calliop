//! Audio capture and voice activity detection (Phase 1+).

pub fn module_name() -> &'static str {
    "audio"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_module_name() {
        assert_eq!(module_name(), "audio");
    }
}
