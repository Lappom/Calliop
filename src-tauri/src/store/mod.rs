//! SQLite persistence for config, dictionary, and history (Phase 1+).

pub fn module_name() -> &'static str {
    "store"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_module_name() {
        assert_eq!(module_name(), "store");
    }
}
