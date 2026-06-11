//! SQLite persistence for config, dictionary, and history (Phase 1+).

mod app_context;
mod db;
mod dictionary;
mod history;
mod settings;
mod snippets;

pub use app_context::{
    exe_names_match, is_valid_app_context_pattern, normalize_exe_pattern, normalize_title_pattern,
    AppContextMatchType, AppContextRule, NewAppContextRule,
};
pub use db::{db_path, Store, StoreError};
pub use dictionary::{
    extract_correction_words, is_valid_dictionary_word, normalize_word, DictionaryCorrectionRule,
    DictionarySource, DictionaryWord,
};
pub use history::{
    count_words, dictation_wpm, wpm_vs_typing_percent, AppUsageEntry, DictationEntry, Insights,
    LatencySnapshot, NewDictation, DEFAULT_LIST_LIMIT, TYPING_SPEED_BASELINE_WPM,
};
pub use settings::{
    AppSettings, InferenceBackend, KEY_AUTO_EDIT, KEY_AUTO_LEARN, KEY_HOTKEY,
    KEY_INFERENCE_BACKEND, KEY_LLM_MODEL, KEY_ONBOARDING_DONE, KEY_STT_LANGUAGE, KEY_WHISPER_MODEL,
};
pub use snippets::{
    is_valid_snippet_content, is_valid_trigger, normalize_trigger, Snippet, SnippetImport,
    KEY_SNIPPET_USER_NAME,
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
