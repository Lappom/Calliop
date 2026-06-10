//! Text injection into the active application (Phase 1+).

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
