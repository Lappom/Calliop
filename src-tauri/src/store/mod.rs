//! SQLite persistence for config, dictionary, and history (Phase 1+).

mod db;
mod dictionary;
mod settings;
mod snippets;

pub use db::{db_path, Store, StoreError};
pub use dictionary::{
    extract_correction_words, is_valid_dictionary_word, normalize_word, DictionarySource,
    DictionaryWord,
};
pub use settings::{AppSettings, KEY_AUTO_EDIT, KEY_AUTO_LEARN};
pub use snippets::{
    is_valid_snippet_content, is_valid_trigger, normalize_trigger, Snippet, SnippetImport,
};

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
