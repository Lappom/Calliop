use calliop_prompt::ToneProfile;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::db::{Store, StoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppContextMatchType {
    Exe,
    TitleContains,
}

impl AppContextMatchType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exe => "exe",
            Self::TitleContains => "title_contains",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "exe" => Some(Self::Exe),
            "title_contains" => Some(Self::TitleContains),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppContextRule {
    pub id: i64,
    pub pattern: String,
    #[serde(rename = "matchType")]
    pub match_type: AppContextMatchType,
    pub tone: ToneProfile,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAppContextRule {
    pub pattern: String,
    pub match_type: AppContextMatchType,
    pub tone: ToneProfile,
}

pub fn normalize_exe_pattern(pattern: &str) -> String {
    let trimmed = pattern.trim();
    let basename = std::path::Path::new(trimmed)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(trimmed);
    basename.to_ascii_lowercase()
}

pub fn normalize_title_pattern(pattern: &str) -> String {
    pattern.trim().to_string()
}

pub fn is_valid_app_context_pattern(pattern: &str, match_type: AppContextMatchType) -> bool {
    let normalized = match match_type {
        AppContextMatchType::Exe => normalize_exe_pattern(pattern),
        AppContextMatchType::TitleContains => normalize_title_pattern(pattern),
    };
    normalized.chars().count() >= 2
}

impl Store {
    pub fn list_app_context_rules(&self) -> Result<Vec<AppContextRule>, StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, pattern, match_type, tone, created_at
             FROM app_context_rules
             ORDER BY id ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            let match_type_raw: String = row.get(2)?;
            let tone_raw: String = row.get(3)?;
            Ok(AppContextRule {
                id: row.get(0)?,
                pattern: row.get(1)?,
                match_type: AppContextMatchType::parse(&match_type_raw).ok_or_else(|| {
                    rusqlite::Error::InvalidColumnType(
                        2,
                        "match_type".into(),
                        rusqlite::types::Type::Text,
                    )
                })?,
                tone: ToneProfile::parse(&tone_raw).ok_or_else(|| {
                    rusqlite::Error::InvalidColumnType(
                        3,
                        "tone".into(),
                        rusqlite::types::Type::Text,
                    )
                })?,
                created_at: row.get(4)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    pub fn add_app_context_rule(&self, rule: &NewAppContextRule) -> Result<bool, StoreError> {
        let pattern = match rule.match_type {
            AppContextMatchType::Exe => normalize_exe_pattern(&rule.pattern),
            AppContextMatchType::TitleContains => normalize_title_pattern(&rule.pattern),
        };
        if !is_valid_app_context_pattern(&pattern, rule.match_type) {
            return Ok(false);
        }

        let conn = self.connection().lock().expect("store mutex poisoned");
        let changed = conn.execute(
            "INSERT INTO app_context_rules (pattern, match_type, tone) VALUES (?1, ?2, ?3)",
            params![pattern, rule.match_type.as_str(), rule.tone.as_str()],
        )?;
        Ok(changed > 0)
    }

    pub fn remove_app_context_rule(&self, id: i64) -> Result<bool, StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        let changed = conn.execute("DELETE FROM app_context_rules WHERE id = ?1", params![id])?;
        Ok(changed > 0)
    }

    pub fn seed_default_app_context_rules(&self) -> Result<usize, StoreError> {
        if !self.list_app_context_rules()?.is_empty() {
            return Ok(0);
        }

        let defaults = [
            NewAppContextRule {
                pattern: "slack.exe".into(),
                match_type: AppContextMatchType::Exe,
                tone: ToneProfile::Casual,
            },
            NewAppContextRule {
                pattern: "teams.exe".into(),
                match_type: AppContextMatchType::Exe,
                tone: ToneProfile::Casual,
            },
            NewAppContextRule {
                pattern: "outlook.exe".into(),
                match_type: AppContextMatchType::Exe,
                tone: ToneProfile::Formal,
            },
            NewAppContextRule {
                pattern: "code.exe".into(),
                match_type: AppContextMatchType::Exe,
                tone: ToneProfile::Technical,
            },
            NewAppContextRule {
                pattern: "cursor.exe".into(),
                match_type: AppContextMatchType::Exe,
                tone: ToneProfile::Technical,
            },
            NewAppContextRule {
                pattern: "windowsterminal.exe".into(),
                match_type: AppContextMatchType::Exe,
                tone: ToneProfile::Technical,
            },
            NewAppContextRule {
                pattern: "wt.exe".into(),
                match_type: AppContextMatchType::Exe,
                tone: ToneProfile::Technical,
            },
            NewAppContextRule {
                pattern: "powershell.exe".into(),
                match_type: AppContextMatchType::Exe,
                tone: ToneProfile::Technical,
            },
            NewAppContextRule {
                pattern: "cmd.exe".into(),
                match_type: AppContextMatchType::Exe,
                tone: ToneProfile::Technical,
            },
        ];

        let mut count = 0_usize;
        for rule in defaults {
            if self.add_app_context_rule(&rule)? {
                count += 1;
            }
        }
        Ok(count)
    }
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
            "CREATE TABLE app_context_rules (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                pattern TEXT NOT NULL,
                match_type TEXT NOT NULL,
                tone TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )
        .unwrap();
        Store::from_connection(conn)
    }

    #[test]
    fn app_context_rules_crud_roundtrip() {
        let store = test_store();
        assert!(store.list_app_context_rules().unwrap().is_empty());

        assert!(store
            .add_app_context_rule(&NewAppContextRule {
                pattern: "Slack.EXE".into(),
                match_type: AppContextMatchType::Exe,
                tone: ToneProfile::Casual,
            })
            .unwrap());

        let rules = store.list_app_context_rules().unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].pattern, "slack.exe");
        assert_eq!(rules[0].tone, ToneProfile::Casual);

        assert!(store.remove_app_context_rule(rules[0].id).unwrap());
        assert!(store.list_app_context_rules().unwrap().is_empty());
    }

    #[test]
    fn seed_defaults_only_when_empty() {
        let store = test_store();
        assert_eq!(store.seed_default_app_context_rules().unwrap(), 9);
        assert_eq!(store.seed_default_app_context_rules().unwrap(), 0);
        assert!(store.list_app_context_rules().unwrap().len() >= 9);
    }

    #[test]
    fn rejects_short_pattern() {
        let store = test_store();
        assert!(!store
            .add_app_context_rule(&NewAppContextRule {
                pattern: "a".into(),
                match_type: AppContextMatchType::Exe,
                tone: ToneProfile::Default,
            })
            .unwrap());
    }
}
