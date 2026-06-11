//! Incremental cache for the Whisper initial prompt built from snippets and dictionary words.

use std::sync::Arc;

use calliop_prompt::whisper_oral_vocabulary_word_count;

use super::engine::{
    build_whisper_initial_prompt, MAX_INITIAL_PROMPT_WORDS, MAX_SNIPPET_PROMPT_WORDS,
};

/// Cached Whisper initial prompt with incremental add/remove support.
#[derive(Debug, Clone, Default)]
pub struct WhisperPromptCache {
    snippet_triggers: Vec<String>,
    dictionary_words: Vec<String>,
    prompt: Option<Arc<str>>,
}

impl WhisperPromptCache {
    pub fn prompt(&self) -> Option<Arc<str>> {
        self.prompt.clone()
    }

    pub fn snippet_triggers(&self) -> &[String] {
        &self.snippet_triggers
    }

    pub fn dictionary_words(&self) -> &[String] {
        &self.dictionary_words
    }

    /// Full rebuild from snippet triggers and dictionary words (most recent first).
    pub fn rebuild(&mut self, snippet_triggers: Vec<String>, dictionary_words: Vec<String>) {
        self.snippet_triggers = truncate_snippets(snippet_triggers);
        self.dictionary_words = truncate_dictionary_words(&self.snippet_triggers, dictionary_words);
        self.prompt = build_whisper_initial_prompt(&self.snippet_triggers, &self.dictionary_words)
            .map(|value| Arc::from(value.as_str()));
    }

    /// Merge newly added words at the front (recency priority) and rebuild only if changed.
    pub fn apply_additions(&mut self, new_words: &[String]) -> bool {
        if new_words.is_empty() {
            return false;
        }

        let mut merged = Vec::with_capacity(new_words.len() + self.dictionary_words.len());
        for word in new_words {
            let key = word.to_lowercase();
            if !merged.iter().any(|w: &String| w.eq_ignore_ascii_case(&key)) {
                merged.push(word.clone());
            }
        }
        for word in &self.dictionary_words {
            if !merged.iter().any(|w| w.eq_ignore_ascii_case(word)) {
                merged.push(word.clone());
            }
        }

        let truncated = truncate_dictionary_words(&self.snippet_triggers, merged);
        if dictionary_words_unchanged(&truncated, &self.dictionary_words) {
            return false;
        }

        self.dictionary_words = truncated;
        self.prompt = build_whisper_initial_prompt(&self.snippet_triggers, &self.dictionary_words)
            .map(|value| Arc::from(value.as_str()));
        true
    }

    /// After removals, callers should pass the refreshed word list from the store.
    pub fn apply_full_dictionary(&mut self, dictionary_words: Vec<String>) -> bool {
        let truncated = truncate_dictionary_words(&self.snippet_triggers, dictionary_words);
        if dictionary_words_unchanged(&truncated, &self.dictionary_words) {
            return false;
        }
        self.dictionary_words = truncated;
        self.prompt = build_whisper_initial_prompt(&self.snippet_triggers, &self.dictionary_words)
            .map(|value| Arc::from(value.as_str()));
        true
    }

    pub fn set_snippets(&mut self, snippet_triggers: Vec<String>) {
        let truncated = truncate_snippets(snippet_triggers);
        if truncated == self.snippet_triggers {
            return;
        }
        self.snippet_triggers = truncated;
        self.dictionary_words =
            truncate_dictionary_words(&self.snippet_triggers, self.dictionary_words.clone());
        self.prompt = build_whisper_initial_prompt(&self.snippet_triggers, &self.dictionary_words)
            .map(|value| Arc::from(value.as_str()));
    }
}

fn dictionary_words_unchanged(current: &[String], previous: &[String]) -> bool {
    current.len() == previous.len()
        && current
            .iter()
            .zip(previous.iter())
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
}

fn truncate_snippets(snippet_triggers: Vec<String>) -> Vec<String> {
    snippet_triggers
        .into_iter()
        .take(MAX_SNIPPET_PROMPT_WORDS)
        .collect()
}

fn dictionary_word_budget(snippet_triggers: &[String]) -> usize {
    let hint_words = whisper_oral_vocabulary_word_count();
    let user_budget = MAX_INITIAL_PROMPT_WORDS.saturating_sub(hint_words);
    user_budget.saturating_sub(snippet_triggers.len().min(MAX_SNIPPET_PROMPT_WORDS))
}

fn truncate_dictionary_words(
    snippet_triggers: &[String],
    dictionary_words: Vec<String>,
) -> Vec<String> {
    let budget = dictionary_word_budget(snippet_triggers);
    dictionary_words.into_iter().take(budget).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebuild_includes_snippets_then_dictionary() {
        let mut cache = WhisperPromptCache::default();
        cache.rebuild(vec!["snippet".into()], vec!["Alpha".into(), "Beta".into()]);
        let prompt = cache.prompt().expect("prompt").to_string();
        assert!(prompt.contains("snippet"));
        assert!(prompt.contains("Alpha"));
    }

    #[test]
    fn apply_additions_prefixes_new_words() {
        let mut cache = WhisperPromptCache::default();
        cache.rebuild(vec![], vec!["old".into()]);
        assert!(cache.apply_additions(&["new".into()]));
        assert_eq!(cache.dictionary_words()[0], "new");
        assert!(cache.dictionary_words().contains(&"old".into()));
    }

    #[test]
    fn apply_additions_noop_when_duplicate() {
        let mut cache = WhisperPromptCache::default();
        cache.rebuild(vec![], vec!["Calliop".into()]);
        assert!(!cache.apply_additions(&["calliop".into()]));
    }

    #[test]
    fn truncates_dictionary_to_budget() {
        let mut cache = WhisperPromptCache::default();
        let words: Vec<String> = (0..300).map(|i| format!("word{i}")).collect();
        cache.rebuild(vec![], words);
        let budget = dictionary_word_budget(&[]);
        assert_eq!(cache.dictionary_words().len(), budget);
    }
}
