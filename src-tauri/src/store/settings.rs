use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::db::{Store, StoreError};

use crate::hotkey::DEFAULT_HOTKEY_SETTING;
use crate::llm::LlmModel;
use crate::stt::{SttLanguage, WhisperModel, DEFAULT_STT_LANGUAGE};

pub const KEY_AUTO_EDIT: &str = "auto_edit";
pub const KEY_AUTO_LEARN: &str = "auto_learn";
pub const KEY_STT_LANGUAGE: &str = "stt_language";
pub const KEY_WHISPER_MODEL: &str = "whisper_model";
pub const KEY_LLM_MODEL: &str = "llm_model";
pub const KEY_HOTKEY: &str = "hotkey";
pub const KEY_INFERENCE_BACKEND: &str = "inference_backend";
pub const KEY_ONBOARDING_DONE: &str = "onboarding_done";
pub const KEY_AUTO_UPDATE: &str = "auto_update";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InferenceBackend {
    #[default]
    Auto,
    Cpu,
}

impl InferenceBackend {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "cpu" => Some(Self::Cpu),
            _ => None,
        }
    }

    pub fn as_setting_value(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Cpu => "cpu",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    pub auto_edit: bool,
    pub auto_learn: bool,
    pub auto_update: bool,
    pub stt_language: String,
    pub whisper_model: String,
    pub llm_model: String,
    pub hotkey: String,
    pub inference_backend: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_edit: true,
            auto_learn: true,
            auto_update: false,
            stt_language: DEFAULT_STT_LANGUAGE.to_string(),
            whisper_model: WhisperModel::default().as_setting_value().into(),
            llm_model: LlmModel::default().as_setting_value().into(),
            hotkey: DEFAULT_HOTKEY_SETTING.into(),
            inference_backend: InferenceBackend::default().as_setting_value().into(),
        }
    }
}

impl AppSettings {
    pub fn verbatim() -> Self {
        Self::default()
    }

    pub fn stt_language_mode(&self) -> SttLanguage {
        SttLanguage::parse(&self.stt_language).unwrap_or_default()
    }

    pub fn whisper_model(&self) -> WhisperModel {
        WhisperModel::parse(&self.whisper_model).unwrap_or_default()
    }

    pub fn llm_model(&self) -> LlmModel {
        LlmModel::parse(&self.llm_model).unwrap_or_default()
    }

    pub fn inference_backend(&self) -> InferenceBackend {
        InferenceBackend::parse(&self.inference_backend).unwrap_or_default()
    }
}

impl Store {
    pub fn load_settings(&self) -> Result<AppSettings, StoreError> {
        Ok(AppSettings {
            auto_edit: self.get_bool(KEY_AUTO_EDIT, true)?,
            auto_learn: self.get_bool(KEY_AUTO_LEARN, true)?,
            auto_update: self.get_bool(KEY_AUTO_UPDATE, false)?,
            stt_language: self
                .get_string(KEY_STT_LANGUAGE, DEFAULT_STT_LANGUAGE)?
                .to_string(),
            whisper_model: self
                .get_string(
                    KEY_WHISPER_MODEL,
                    WhisperModel::default().as_setting_value(),
                )?
                .to_string(),
            llm_model: self
                .get_string(KEY_LLM_MODEL, LlmModel::default().as_setting_value())?
                .to_string(),
            hotkey: self
                .get_string(KEY_HOTKEY, DEFAULT_HOTKEY_SETTING)?
                .to_string(),
            inference_backend: self
                .get_string(
                    KEY_INFERENCE_BACKEND,
                    InferenceBackend::default().as_setting_value(),
                )?
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
            params![
                KEY_AUTO_UPDATE,
                if settings.auto_update {
                    "true"
                } else {
                    "false"
                }
            ],
        )?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![KEY_STT_LANGUAGE, settings.stt_language.as_str()],
        )?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![KEY_WHISPER_MODEL, settings.whisper_model.as_str()],
        )?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![KEY_LLM_MODEL, settings.llm_model.as_str()],
        )?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![KEY_HOTKEY, settings.hotkey.as_str()],
        )?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![KEY_INFERENCE_BACKEND, settings.inference_backend.as_str()],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn is_onboarding_done(&self) -> Result<bool, StoreError> {
        self.get_bool(KEY_ONBOARDING_DONE, false)
    }

    pub fn set_onboarding_done(&self, done: bool) -> Result<(), StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![KEY_ONBOARDING_DONE, if done { "true" } else { "false" }],
        )?;
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
        assert!(settings.auto_edit);
        assert!(settings.auto_learn);
        assert!(!settings.auto_update);
        assert_eq!(settings.stt_language, DEFAULT_STT_LANGUAGE);
        assert_eq!(settings.whisper_model, "small");
        assert_eq!(settings.llm_model, "qwen3-1.7b");
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
                auto_update: true,
                stt_language: "en".into(),
                whisper_model: "distil-fr-dec16".into(),
                llm_model: "qwen3-0.6b".into(),
                hotkey: "Ctrl+Shift+Space".into(),
                inference_backend: "cpu".into(),
            })
            .expect("save");
        let loaded = store.load_settings().expect("load");
        assert!(loaded.auto_edit);
        assert!(!loaded.auto_learn);
        assert!(loaded.auto_update);
        assert_eq!(loaded.stt_language, "en");
        assert_eq!(loaded.whisper_model, "distil-fr-dec16");
        assert_eq!(loaded.llm_model, "qwen3-0.6b");

        let _ = std::fs::remove_dir_all(dir);
    }
}
