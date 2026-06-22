use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::{params, Connection};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("failed to open database: {0}")]
    Open(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid setting value for {key}: {value}")]
    InvalidValue { key: String, value: String },
}

pub fn db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.calliop.app")
        .join("calliop.db")
}

pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    pub(crate) fn connection(&self) -> &Mutex<Connection> {
        &self.conn
    }

    pub fn open() -> Result<Self, StoreError> {
        let path = db_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        init_schema(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    #[cfg(test)]
    pub fn from_connection(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    pub fn has_setting(&self, key: &str) -> Result<bool, StoreError> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare("SELECT 1 FROM settings WHERE key = ?1 LIMIT 1")?;
        let mut rows = stmt.query(params![key])?;
        Ok(rows.next()?.is_some())
    }

    pub fn get_bool(&self, key: &str, default: bool) -> Result<bool, StoreError> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;

        match rows.next()? {
            Some(row) => {
                let value: String = row.get(0)?;
                parse_bool(&value).ok_or_else(|| StoreError::InvalidValue {
                    key: key.to_string(),
                    value,
                })
            }
            None => Ok(default),
        }
    }

    pub fn get_string(&self, key: &str, default: &str) -> Result<String, StoreError> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;

        match rows.next()? {
            Some(row) => Ok(row.get(0)?),
            None => Ok(default.to_string()),
        }
    }

    pub fn set_bool(&self, key: &str, value: bool) -> Result<(), StoreError> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, if value { "true" } else { "false" }],
        )?;
        Ok(())
    }

    pub fn set_string(&self, key: &str, value: &str) -> Result<(), StoreError> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
}

pub(crate) fn init_schema(conn: &Connection) -> Result<(), StoreError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS dictionary (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            word TEXT NOT NULL COLLATE NOCASE,
            source TEXT NOT NULL DEFAULT 'manual',
            misspelling TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;
    migrate_dictionary_misspelling(conn)?;
    migrate_dictionary_unique_constraints(conn)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS snippets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            trigger TEXT NOT NULL UNIQUE COLLATE NOCASE,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS app_context_rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            pattern TEXT NOT NULL,
            match_type TEXT NOT NULL,
            tone TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS dictations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT NOT NULL,
            word_count INTEGER NOT NULL,
            audio_duration_ms INTEGER NOT NULL DEFAULT 0,
            stt_ms INTEGER NOT NULL DEFAULT 0,
            llm_ms INTEGER NOT NULL DEFAULT 0,
            inject_ms INTEGER NOT NULL DEFAULT 0,
            total_ms INTEGER NOT NULL DEFAULT 0,
            app_exe TEXT,
            app_title TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS dictations_fts USING fts5(
            text,
            content='dictations',
            content_rowid='id'
        )",
        [],
    )?;
    conn.execute_batch(
        "CREATE TRIGGER IF NOT EXISTS dictations_ai AFTER INSERT ON dictations BEGIN
            INSERT INTO dictations_fts(rowid, text) VALUES (new.id, new.text);
        END;
        CREATE TRIGGER IF NOT EXISTS dictations_ad AFTER DELETE ON dictations BEGIN
            INSERT INTO dictations_fts(dictations_fts, rowid, text)
            VALUES ('delete', old.id, old.text);
        END;
        CREATE TRIGGER IF NOT EXISTS dictations_au AFTER UPDATE ON dictations BEGIN
            INSERT INTO dictations_fts(dictations_fts, rowid, text)
            VALUES ('delete', old.id, old.text);
            INSERT INTO dictations_fts(rowid, text) VALUES (new.id, new.text);
        END;",
    )?;
    crate::achievements::migrate_achievement_tables(conn)?;
    Ok(())
}

fn migrate_dictionary_misspelling(conn: &Connection) -> Result<(), StoreError> {
    let mut stmt = conn.prepare("PRAGMA table_info(dictionary)")?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let has_misspelling = columns
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .any(|name| name == "misspelling");

    if !has_misspelling {
        conn.execute("ALTER TABLE dictionary ADD COLUMN misspelling TEXT", [])?;
    }
    Ok(())
}

fn migrate_dictionary_unique_constraints(conn: &Connection) -> Result<(), StoreError> {
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type = 'index' AND name = 'idx_dictionary_word_plain'",
    )?;
    if stmt.exists([])? {
        return Ok(());
    }

    // Rebuild to drop legacy UNIQUE(word) and allow multiple correction rules per replacement.
    conn.execute_batch(
        "CREATE TABLE dictionary_migrated (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            word TEXT NOT NULL COLLATE NOCASE,
            source TEXT NOT NULL DEFAULT 'manual',
            misspelling TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        INSERT INTO dictionary_migrated (id, word, source, misspelling, created_at)
            SELECT id, word, source, misspelling, created_at FROM dictionary;
        DROP TABLE dictionary;
        ALTER TABLE dictionary_migrated RENAME TO dictionary;
        CREATE UNIQUE INDEX idx_dictionary_word_plain
            ON dictionary(word COLLATE NOCASE) WHERE misspelling IS NULL;
        CREATE UNIQUE INDEX idx_dictionary_misspelling
            ON dictionary(misspelling COLLATE NOCASE) WHERE misspelling IS NOT NULL;",
    )?;
    Ok(())
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (Store, PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "calliop-store-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&path);
        let db_file = path.join("test.db");
        let conn = Connection::open(&db_file).unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )
        .unwrap();
        (
            Store {
                conn: Mutex::new(conn),
            },
            path,
        )
    }

    #[test]
    fn bool_roundtrip() {
        let (store, dir) = temp_store();
        assert!(!store.get_bool("auto_edit", false).unwrap());
        store.set_bool("auto_edit", true).unwrap();
        assert!(store.get_bool("auto_edit", false).unwrap());
        let _ = std::fs::remove_dir_all(dir);
    }
}
