use std::collections::HashSet;

use rusqlite::params;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use super::db::{Store, StoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DictionarySource {
    Manual,
    Learned,
}

impl DictionarySource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Learned => "learned",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "manual" => Some(Self::Manual),
            "learned" => Some(Self::Learned),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DictionaryWord {
    pub id: i64,
    pub word: String,
    pub source: DictionarySource,
    pub created_at: String,
}

impl Store {
    pub fn list_words(&self) -> Result<Vec<DictionaryWord>, StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, word, source, created_at
             FROM dictionary
             ORDER BY word COLLATE NOCASE ASC",
        )?;

        let rows = stmt.query_map([], Self::map_dictionary_row)?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    /// Words for the Whisper prompt, most recently added first.
    pub fn list_words_for_prompt(&self, limit: usize) -> Result<Vec<String>, StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT word FROM dictionary
             ORDER BY datetime(created_at) DESC, id DESC
             LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| row.get::<_, String>(0))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    pub fn add_words_batch(
        &self,
        words: &[String],
        source: DictionarySource,
    ) -> Result<Vec<String>, StoreError> {
        let mut added = Vec::new();
        let mut conn = self.connection().lock().expect("store mutex poisoned");
        let tx = conn.transaction()?;

        for word in words {
            let normalized = normalize_word(word);
            if !is_valid_dictionary_word(&normalized) {
                continue;
            }
            let changed = tx.execute(
                "INSERT INTO dictionary (word, source) VALUES (?1, ?2)
                 ON CONFLICT(word) DO NOTHING",
                params![normalized, source.as_str()],
            )?;
            if changed > 0 {
                added.push(normalized);
            }
        }

        tx.commit()?;
        Ok(added)
    }

    fn map_dictionary_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DictionaryWord> {
        let source_raw: String = row.get(2)?;
        let source = DictionarySource::parse(&source_raw).ok_or_else(|| {
            rusqlite::Error::InvalidColumnType(2, "source".into(), rusqlite::types::Type::Text)
        })?;

        Ok(DictionaryWord {
            id: row.get(0)?,
            word: row.get(1)?,
            source,
            created_at: row.get(3)?,
        })
    }

    pub fn add_word(&self, word: &str, source: DictionarySource) -> Result<bool, StoreError> {
        let normalized = normalize_word(word);
        if !is_valid_dictionary_word(&normalized) {
            return Ok(false);
        }

        let conn = self.connection().lock().expect("store mutex poisoned");
        let changed = conn.execute(
            "INSERT INTO dictionary (word, source) VALUES (?1, ?2)
             ON CONFLICT(word) DO NOTHING",
            params![normalized, source.as_str()],
        )?;

        Ok(changed > 0)
    }

    pub fn get_word_by_id(&self, id: i64) -> Result<Option<DictionaryWord>, StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, word, source, created_at
             FROM dictionary
             WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let source_raw: String = row.get(2)?;
            let source = DictionarySource::parse(&source_raw).ok_or_else(|| {
                rusqlite::Error::InvalidColumnType(2, "source".into(), rusqlite::types::Type::Text)
            })?;
            return Ok(Some(DictionaryWord {
                id: row.get(0)?,
                word: row.get(1)?,
                source,
                created_at: row.get(3)?,
            }));
        }

        Ok(None)
    }

    pub fn remove_word(&self, id: i64) -> Result<bool, StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        let changed = conn.execute("DELETE FROM dictionary WHERE id = ?1", params![id])?;
        Ok(changed > 0)
    }

    pub fn update_word(&self, id: i64, word: &str) -> Result<bool, StoreError> {
        let normalized = normalize_word(word);
        if !is_valid_dictionary_word(&normalized) {
            return Ok(false);
        }

        let conn = self.connection().lock().expect("store mutex poisoned");
        let conflict: Option<i64> = conn
            .query_row(
                "SELECT id FROM dictionary WHERE word = ?1 COLLATE NOCASE AND id != ?2",
                params![normalized, id],
                |row| row.get(0),
            )
            .optional()?;
        if conflict.is_some() {
            return Ok(false);
        }

        let changed = conn.execute(
            "UPDATE dictionary SET word = ?1 WHERE id = ?2",
            params![normalized, id],
        )?;

        Ok(changed > 0)
    }

    pub fn remove_word_by_normalized(&self, word: &str) -> Result<bool, StoreError> {
        let normalized = normalize_word(word);
        let conn = self.connection().lock().expect("store mutex poisoned");
        let changed = conn.execute(
            "DELETE FROM dictionary WHERE word = ?1 COLLATE NOCASE",
            params![normalized],
        )?;
        Ok(changed > 0)
    }
}

pub fn normalize_word(word: &str) -> String {
    word.trim().to_string()
}

pub fn is_valid_dictionary_word(word: &str) -> bool {
    let trimmed = word.trim();
    if trimmed.chars().count() < 2 {
        return false;
    }
    !trimmed.chars().all(|c| c.is_ascii_digit())
}

fn tokenize_words(text: &str) -> Vec<(String, String)> {
    text.split_whitespace()
        .map(|token| {
            let cleaned = token
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_string();
            let key = cleaned.to_lowercase();
            (key, cleaned)
        })
        .filter(|(key, _)| !key.is_empty())
        .collect()
}

enum WordEditOp {
    Equal,
    Insert(String),
    Delete,
    Substitute { from: String, to: String },
}

fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }

    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0; b.len() + 1];

    for (i, ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b.len()]
}

fn should_learn_substitution(original: &str, corrected: &str) -> bool {
    if !is_valid_dictionary_word(corrected) {
        return false;
    }

    let original_lower = original.to_lowercase();
    let corrected_lower = corrected.to_lowercase();
    if original_lower == corrected_lower {
        return false;
    }

    let distance = levenshtein_distance(&original_lower, &corrected_lower);
    if distance == 0 || distance > 3 {
        return false;
    }

    // Skip short function-word swaps (articles, prepositions, etc.).
    if original.chars().count() < 4 && corrected.chars().count() < 4 {
        return false;
    }

    true
}

#[allow(clippy::needless_range_loop)]
fn word_edit_ops(original: &[(String, String)], corrected: &[(String, String)]) -> Vec<WordEditOp> {
    let original_keys: Vec<String> = original.iter().map(|(key, _)| key.clone()).collect();
    let n = original_keys.len();
    let m = corrected.len();
    let mut dp = vec![vec![0u32; m + 1]; n + 1];

    for i in 1..=n {
        dp[i][0] = i as u32;
    }
    for j in 1..=m {
        dp[0][j] = j as u32;
    }

    for i in 1..=n {
        for j in 1..=m {
            if original_keys[i - 1] == corrected[j - 1].0 {
                dp[i][j] = dp[i - 1][j - 1];
            } else {
                dp[i][j] = (dp[i - 1][j - 1] + 1)
                    .min(dp[i - 1][j] + 1)
                    .min(dp[i][j - 1] + 1);
            }
        }
    }

    let mut ops = Vec::new();
    let mut i = n;
    let mut j = m;
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && original_keys[i - 1] == corrected[j - 1].0 {
            ops.push(WordEditOp::Equal);
            i -= 1;
            j -= 1;
        } else if i > 0 && j > 0 && dp[i][j] == dp[i - 1][j - 1] + 1 {
            ops.push(WordEditOp::Substitute {
                from: original[i - 1].1.clone(),
                to: corrected[j - 1].1.clone(),
            });
            i -= 1;
            j -= 1;
        } else if i > 0 && dp[i][j] == dp[i - 1][j] + 1 {
            ops.push(WordEditOp::Delete);
            i -= 1;
        } else {
            ops.push(WordEditOp::Insert(corrected[j - 1].1.clone()));
            j -= 1;
        }
    }

    ops.reverse();
    ops
}

/// Returns vocabulary candidates newly introduced in `corrected` relative to `original`.
pub fn extract_correction_words(original: &str, corrected: &str) -> Vec<String> {
    if original.trim() == corrected.trim() {
        return Vec::new();
    }

    let original_tokens = tokenize_words(original);
    let corrected_tokens = tokenize_words(corrected);
    let ops = word_edit_ops(&original_tokens, &corrected_tokens);

    let mut seen = HashSet::new();
    let mut result = Vec::new();

    for op in ops {
        let candidate = match op {
            WordEditOp::Insert(word) => Some(word),
            WordEditOp::Substitute { from, to } if should_learn_substitution(&from, &to) => {
                Some(to)
            }
            _ => None,
        };

        if let Some(word) = candidate {
            let key = word.to_lowercase();
            if is_valid_dictionary_word(&word) && seen.insert(key) {
                result.push(word);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::db::Store;
    use rusqlite::Connection;

    fn test_store() -> Store {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE dictionary (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                word TEXT NOT NULL UNIQUE COLLATE NOCASE,
                source TEXT NOT NULL DEFAULT 'manual',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )
        .unwrap();
        Store::from_connection(conn)
    }

    #[test]
    fn dictionary_crud_roundtrip() {
        let store = test_store();
        assert!(store.list_words().unwrap().is_empty());

        assert!(store.add_word("Calliop", DictionarySource::Manual).unwrap());
        assert!(!store
            .add_word("calliop", DictionarySource::Learned)
            .unwrap());

        let words = store.list_words().unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].word, "Calliop");
        assert_eq!(words[0].source, DictionarySource::Manual);

        assert!(store.remove_word(words[0].id).unwrap());
        assert!(store.list_words().unwrap().is_empty());
    }

    #[test]
    fn rejects_short_and_numeric_words() {
        let store = test_store();
        assert!(!store.add_word("a", DictionarySource::Manual).unwrap());
        assert!(!store.add_word("42", DictionarySource::Manual).unwrap());
    }

    #[test]
    fn extract_correction_words_detects_new_tokens() {
        let words = extract_correction_words(
            "bonjour ceci est un test",
            "bonjour ceci est un test Calliop",
        );
        assert_eq!(words, vec!["Calliop".to_string()]);
    }

    #[test]
    fn extract_correction_words_ignores_unchanged_text() {
        let words = extract_correction_words("hello world", "hello world");
        assert!(words.is_empty());
    }

    #[test]
    fn extract_correction_words_is_case_insensitive() {
        let words = extract_correction_words("bonjour", "Bonjour Calliop");
        assert_eq!(words, vec!["Calliop".to_string()]);
    }

    #[test]
    fn extract_correction_words_ignores_short_substitutions() {
        let words = extract_correction_words("le chat est ici", "la chat est ici");
        assert!(words.is_empty());
    }

    #[test]
    fn extract_correction_words_learns_typo_substitutions() {
        let words = extract_correction_words("bonjour Calliop", "bonjour Calliope");
        assert_eq!(words, vec!["Calliope".to_string()]);
    }

    #[test]
    fn add_words_batch_inserts_in_one_transaction() {
        let store = test_store();
        let added = store
            .add_words_batch(
                &["Alpha".into(), "alpha".into(), "Beta".into()],
                DictionarySource::Learned,
            )
            .unwrap();
        assert_eq!(added, vec!["Alpha".to_string(), "Beta".to_string()]);
        assert_eq!(store.list_words().unwrap().len(), 2);
    }

    #[test]
    fn update_word_changes_spelling() {
        let store = test_store();
        assert!(store.add_word("Calliop", DictionarySource::Manual).unwrap());
        let id = store.list_words().unwrap()[0].id;

        assert!(store.update_word(id, "Calliope").unwrap());
        let words = store.list_words().unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].word, "Calliope");
    }

    #[test]
    fn update_word_rejects_duplicate() {
        let store = test_store();
        store.add_word("Alpha", DictionarySource::Manual).unwrap();
        store.add_word("Beta", DictionarySource::Manual).unwrap();
        let beta_id = store
            .list_words()
            .unwrap()
            .into_iter()
            .find(|entry| entry.word == "Beta")
            .unwrap()
            .id;

        assert!(!store.update_word(beta_id, "alpha").unwrap());
    }

    #[test]
    fn list_words_for_prompt_returns_recent_first() {
        let store = test_store();
        store.add_word("Older", DictionarySource::Manual).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        store.add_word("Newer", DictionarySource::Manual).unwrap();

        let words = store.list_words_for_prompt(10).unwrap();
        assert_eq!(words.first().map(String::as_str), Some("Newer"));
    }
}
