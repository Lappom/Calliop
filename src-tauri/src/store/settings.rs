use serde::{Deserialize, Serialize};

use super::db::{Store, StoreError};

pub const KEY_AUTO_EDIT: &str = "auto_edit";
pub const KEY_AUTO_LEARN: &str = "auto_learn";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    pub auto_edit: bool,
    pub auto_learn: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_edit: false,
            auto_learn: true,
        }
    }
}

impl AppSettings {
    pub fn verbatim() -> Self {
        Self {
            auto_edit: false,
            auto_learn: true,
        }
    }
}

impl Store {
    pub fn load_settings(&self) -> Result<AppSettings, StoreError> {
        Ok(AppSettings {
            auto_edit: self.get_bool(KEY_AUTO_EDIT, false)?,
            auto_learn: self.get_bool(KEY_AUTO_LEARN, true)?,
        })
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), StoreError> {
        self.set_bool(KEY_AUTO_EDIT, settings.auto_edit)?;
        self.set_bool(KEY_AUTO_LEARN, settings.auto_learn)
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
            })
            .expect("save");
        let loaded = store.load_settings().expect("load");
        assert!(loaded.auto_edit);
        assert!(!loaded.auto_learn);

        let _ = std::fs::remove_dir_all(dir);
    }
}
