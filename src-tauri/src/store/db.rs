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

    pub fn set_bool(&self, key: &str, value: bool) -> Result<(), StoreError> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, if value { "true" } else { "false" }],
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
            word TEXT NOT NULL UNIQUE COLLATE NOCASE,
            source TEXT NOT NULL DEFAULT 'manual',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;
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
