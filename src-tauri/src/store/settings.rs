use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::db::{Store, StoreError};

use crate::stt::{SttLanguage, DEFAULT_STT_LANGUAGE};

pub const KEY_AUTO_EDIT: &str = "auto_edit";
pub const KEY_AUTO_LEARN: &str = "auto_learn";
pub const KEY_STT_LANGUAGE: &str = "stt_language";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    pub auto_edit: bool,
    pub auto_learn: bool,
    pub stt_language: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_edit: false,
            auto_learn: true,
            stt_language: DEFAULT_STT_LANGUAGE.to_string(),
        }
    }
}

impl AppSettings {
    pub fn verbatim() -> Self {
        Self {
            auto_edit: false,
            auto_learn: true,
            stt_language: DEFAULT_STT_LANGUAGE.to_string(),
        }
    }

    pub fn stt_language_mode(&self) -> SttLanguage {
        SttLanguage::parse(&self.stt_language).unwrap_or_default()
    }
}

impl Store {
    pub fn load_settings(&self) -> Result<AppSettings, StoreError> {
        Ok(AppSettings {
            auto_edit: self.get_bool(KEY_AUTO_EDIT, false)?,
            auto_learn: self.get_bool(KEY_AUTO_LEARN, true)?,
            stt_language: self
                .get_string(KEY_STT_LANGUAGE, DEFAULT_STT_LANGUAGE)?
                .to_string(),
        })
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![
                KEY_AUTO_EDIT,
                if settings.auto_edit { "true" } else { "false" }
            ],
        )?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![
                KEY_AUTO_LEARN,
                if settings.auto_learn { "true" } else { "false" }
            ],
        )?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![KEY_STT_LANGUAGE, settings.stt_language.as_str()],
        )?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::db::Store;

    #[test]
    fn default_settings_are_verbatim() {
        let settings = AppSettings::default();
        assert!(!settings.auto_edit);
        assert!(settings.auto_learn);
        assert_eq!(settings.stt_language, DEFAULT_STT_LANGUAGE);
    }

    #[test]
    fn save_and_load_settings() {
        let dir = std::env::temp_dir().join(format!(
            "calliop-settings-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let db_file = dir.join("settings.db");
        let conn = rusqlite::Connection::open(&db_file).unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        let store = Store::from_connection(conn);

        store
            .save_settings(&AppSettings {
                auto_edit: true,
                auto_learn: false,
                stt_language: "en".into(),
            })
            .expect("save");
        let loaded = store.load_settings().expect("load");
        assert!(loaded.auto_edit);
        assert!(!loaded.auto_learn);
        assert_eq!(loaded.stt_language, "en");

        let _ = std::fs::remove_dir_all(dir);
    }
}
