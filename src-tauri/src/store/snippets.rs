use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::db::{Store, StoreError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snippet {
    pub id: i64,
    pub trigger: String,
    pub content: String,
    pub created_at: String,
}

/// Portable snippet shape for JSON import/export (no id / timestamps).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnippetImport {
    pub trigger: String,
    pub content: String,
}

impl Store {
    pub fn list_snippets(&self) -> Result<Vec<Snippet>, StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, trigger, content, created_at
             FROM snippets
             ORDER BY trigger COLLATE NOCASE ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Snippet {
                id: row.get(0)?,
                trigger: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    pub fn add_snippet(&self, trigger: &str, content: &str) -> Result<bool, StoreError> {
        let normalized_trigger = normalize_trigger(trigger);
        let trimmed_content = content.trim();
        if !is_valid_trigger(&normalized_trigger) || !is_valid_snippet_content(trimmed_content) {
            return Ok(false);
        }

        let conn = self.connection().lock().expect("store mutex poisoned");
        let changed = conn.execute(
            "INSERT INTO snippets (trigger, content) VALUES (?1, ?2)
             ON CONFLICT(trigger) DO NOTHING",
            params![normalized_trigger, trimmed_content],
        )?;

        Ok(changed > 0)
    }

    pub fn get_snippet_by_id(&self, id: i64) -> Result<Option<Snippet>, StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, trigger, content, created_at
             FROM snippets
             WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(Snippet {
                id: row.get(0)?,
                trigger: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
            }));
        }

        Ok(None)
    }

    pub fn remove_snippet(&self, id: i64) -> Result<bool, StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        let changed = conn.execute("DELETE FROM snippets WHERE id = ?1", params![id])?;
        Ok(changed > 0)
    }

    pub fn remove_snippet_by_trigger(&self, trigger: &str) -> Result<bool, StoreError> {
        let normalized = normalize_trigger(trigger);
        let conn = self.connection().lock().expect("store mutex poisoned");
        let changed = conn.execute(
            "DELETE FROM snippets WHERE trigger = ?1 COLLATE NOCASE",
            params![normalized],
        )?;
        Ok(changed > 0)
    }

    /// Upserts snippets by trigger; returns the number of rows inserted or updated.
    pub fn import_snippets(&self, entries: &[SnippetImport]) -> Result<usize, StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        let tx = conn.unchecked_transaction()?;
        let mut count = 0_usize;

        for entry in entries {
            let normalized_trigger = normalize_trigger(&entry.trigger);
            let trimmed_content = entry.content.trim();
            if !is_valid_trigger(&normalized_trigger) || !is_valid_snippet_content(trimmed_content)
            {
                continue;
            }

            let changed = tx.execute(
                "INSERT INTO snippets (trigger, content) VALUES (?1, ?2)
                 ON CONFLICT(trigger) DO UPDATE SET content = excluded.content",
                params![normalized_trigger, trimmed_content],
            )?;
            if changed > 0 {
                count += 1;
            }
        }

        tx.commit()?;
        Ok(count)
    }

    /// Replaces all snippets with the given entries (used to roll back failed imports).
    pub fn replace_all_snippets(&self, entries: &[SnippetImport]) -> Result<(), StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        let tx = conn.unchecked_transaction()?;
        tx.execute("DELETE FROM snippets", [])?;

        for entry in entries {
            let normalized_trigger = normalize_trigger(&entry.trigger);
            let trimmed_content = entry.content.trim();
            if !is_valid_trigger(&normalized_trigger) || !is_valid_snippet_content(trimmed_content)
            {
                continue;
            }

            tx.execute(
                "INSERT INTO snippets (trigger, content) VALUES (?1, ?2)",
                params![normalized_trigger, trimmed_content],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn export_snippet_imports(&self) -> Result<Vec<SnippetImport>, StoreError> {
        Ok(self
            .list_snippets()?
            .into_iter()
            .map(|snippet| SnippetImport {
                trigger: snippet.trigger,
                content: snippet.content,
            })
            .collect())
    }
}

pub fn normalize_trigger(trigger: &str) -> String {
    trigger
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_lowercase()
}

pub fn is_valid_trigger(trigger: &str) -> bool {
    normalize_trigger(trigger).chars().count() >= 2
}

pub fn is_valid_snippet_content(content: &str) -> bool {
    !content.trim().is_empty()
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
            "CREATE TABLE snippets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                trigger TEXT NOT NULL UNIQUE COLLATE NOCASE,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )
        .unwrap();
        Store::from_connection(conn)
    }

    #[test]
    fn snippets_crud_roundtrip() {
        let store = test_store();
        assert!(store.list_snippets().unwrap().is_empty());

        assert!(store
            .add_snippet("Mon Calendrier", "https://calendly.com/me")
            .unwrap());
        assert!(!store.add_snippet("mon calendrier", "duplicate").unwrap());

        let snippets = store.list_snippets().unwrap();
        assert_eq!(snippets.len(), 1);
        assert_eq!(snippets[0].trigger, "mon calendrier");
        assert_eq!(snippets[0].content, "https://calendly.com/me");

        assert!(store.remove_snippet(snippets[0].id).unwrap());
        assert!(store.list_snippets().unwrap().is_empty());
    }

    #[test]
    fn rejects_short_trigger_and_empty_content() {
        let store = test_store();
        assert!(!store.add_snippet("a", "content").unwrap());
        assert!(!store.add_snippet("valid trigger", "   ").unwrap());
    }

    #[test]
    fn import_snippets_upserts_by_trigger() {
        let store = test_store();
        let entries = vec![
            SnippetImport {
                trigger: "mon calendrier".into(),
                content: "v1".into(),
            },
            SnippetImport {
                trigger: "Mon Calendrier".into(),
                content: "v2".into(),
            },
            SnippetImport {
                trigger: "signature".into(),
                content: "Cordialement".into(),
            },
        ];

        assert_eq!(store.import_snippets(&entries).unwrap(), 3);
        let snippets = store.list_snippets().unwrap();
        assert_eq!(snippets.len(), 2);

        let calendar = snippets
            .iter()
            .find(|snippet| snippet.trigger == "mon calendrier")
            .expect("calendar snippet");
        assert_eq!(calendar.content, "v2");
    }

    #[test]
    fn replace_all_snippets_swaps_table_contents() {
        let store = test_store();
        store.add_snippet("old", "old content").unwrap();

        store
            .replace_all_snippets(&[SnippetImport {
                trigger: "new".into(),
                content: "new content".into(),
            }])
            .unwrap();

        let snippets = store.list_snippets().unwrap();
        assert_eq!(snippets.len(), 1);
        assert_eq!(snippets[0].trigger, "new");
        assert_eq!(snippets[0].content, "new content");
    }

    #[test]
    fn export_snippet_imports_roundtrip() {
        let store = test_store();
        store.add_snippet("hello world", "Hello, World!").unwrap();

        let exported = store.export_snippet_imports().unwrap();
        assert_eq!(
            exported,
            vec![SnippetImport {
                trigger: "hello world".into(),
                content: "Hello, World!".into(),
            }]
        );
    }
}
