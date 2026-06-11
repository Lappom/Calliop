use std::collections::HashSet;

use rusqlite::params;
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

        let rows = stmt.query_map([], |row| {
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
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
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

    pub fn remove_word(&self, id: i64) -> Result<bool, StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        let changed = conn.execute("DELETE FROM dictionary WHERE id = ?1", params![id])?;
        Ok(changed > 0)
    }

    pub fn remove_word_by_normalized(&self, word: &str) -> Result<bool, StoreError> {
        let normalized = normalize_word(word);
        let conn = self.connection().lock().expect("store mutex poisoned");
        let changed =
            conn.execute("DELETE FROM dictionary WHERE word = ?1 COLLATE NOCASE", params![normalized])?;
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
    Substitute,
}

fn word_edit_ops(original_keys: &[String], corrected: &[(String, String)]) -> Vec<WordEditOp> {
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
            ops.push(WordEditOp::Substitute);
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

/// Returns words newly inserted in `corrected` relative to `original`.
pub fn extract_correction_words(original: &str, corrected: &str) -> Vec<String> {
    if original.trim() == corrected.trim() {
        return Vec::new();
    }

    let original_tokens = tokenize_words(original);
    let corrected_tokens = tokenize_words(corrected);
    let original_keys: Vec<String> = original_tokens.into_iter().map(|(key, _)| key).collect();
    let ops = word_edit_ops(&original_keys, &corrected_tokens);

    let mut seen = HashSet::new();
    let mut result = Vec::new();

    for op in ops {
        if let WordEditOp::Insert(word) = op {
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
    fn extract_correction_words_ignores_substitutions() {
        let words = extract_correction_words("le chat est ici", "la chat est ici");
        assert!(words.is_empty());
    }
}
