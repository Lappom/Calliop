use std::collections::HashMap;

use rusqlite::{params, Connection};

use crate::store::{Store, StoreError};

use super::definitions::ALL_ACHIEVEMENTS;
use super::types::{AchievementProgress, AchievementState, AchievementsSummary};

const PROGRESS_CANCEL_STREAK: &str = "cancel_streak";
const PROGRESS_INJECT_FALLBACK: &str = "inject_fallback_used";
const PROGRESS_LLM_SKIPPED: &str = "llm_skipped_done";
const PROGRESS_POLYLOT_PREFIX: &str = "polyglot:";

#[derive(Debug, Clone)]
pub struct UnlockedRow {
    pub id: String,
    pub unlocked_at: String,
    pub seen: bool,
}

pub fn migrate_achievement_tables(conn: &Connection) -> Result<(), StoreError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS achievements (
            id TEXT PRIMARY KEY,
            unlocked_at TEXT NOT NULL DEFAULT (datetime('now')),
            seen INTEGER NOT NULL DEFAULT 0
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS achievement_progress (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}

impl Store {
    pub fn is_achievement_unlocked(&self, id: &str) -> Result<bool, StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare("SELECT 1 FROM achievements WHERE id = ?1 LIMIT 1")?;
        Ok(stmt.exists(params![id])?)
    }

    pub fn unlock_achievement(&self, id: &str, seen: bool) -> Result<bool, StoreError> {
        if self.is_achievement_unlocked(id)? {
            return Ok(false);
        }
        let conn = self.connection().lock().expect("store mutex poisoned");
        conn.execute(
            "INSERT INTO achievements (id, unlocked_at, seen) VALUES (?1, datetime('now'), ?2)",
            params![id, if seen { 1 } else { 0 }],
        )?;
        Ok(true)
    }

    pub fn list_unlocked_achievements(&self) -> Result<HashMap<String, UnlockedRow>, StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        let mut stmt =
            conn.prepare("SELECT id, unlocked_at, seen FROM achievements ORDER BY unlocked_at")?;
        let rows = stmt.query_map([], |row| {
            Ok(UnlockedRow {
                id: row.get(0)?,
                unlocked_at: row.get(1)?,
                seen: row.get::<_, i64>(2)? != 0,
            })
        })?;
        let mut map = HashMap::new();
        for row in rows {
            let row = row?;
            map.insert(
                row.id.clone(),
                UnlockedRow {
                    id: row.id,
                    unlocked_at: row.unlocked_at,
                    seen: row.seen,
                },
            );
        }
        Ok(map)
    }

    pub fn count_unseen_achievements(&self) -> Result<i64, StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM achievements WHERE seen = 0",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn mark_achievements_seen(&self, ids: Option<Vec<String>>) -> Result<(), StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        match ids {
            Some(ids) if !ids.is_empty() => {
                for id in ids {
                    conn.execute(
                        "UPDATE achievements SET seen = 1 WHERE id = ?1",
                        params![id],
                    )?;
                }
            }
            _ => {
                conn.execute("UPDATE achievements SET seen = 1 WHERE seen = 0", [])?;
            }
        }
        Ok(())
    }

    pub fn get_progress(&self, key: &str) -> Result<Option<String>, StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare("SELECT value FROM achievement_progress WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    pub fn set_progress(&self, key: &str, value: &str) -> Result<(), StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        conn.execute(
            "INSERT INTO achievement_progress (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn increment_cancel_streak(&self) -> Result<u32, StoreError> {
        let current = self
            .get_progress(PROGRESS_CANCEL_STREAK)?
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let next = current + 1;
        self.set_progress(PROGRESS_CANCEL_STREAK, &next.to_string())?;
        Ok(next)
    }

    pub fn reset_cancel_streak(&self) -> Result<(), StoreError> {
        self.set_progress(PROGRESS_CANCEL_STREAK, "0")
    }

    pub fn mark_inject_fallback(&self) -> Result<(), StoreError> {
        self.set_progress(PROGRESS_INJECT_FALLBACK, "true")
    }

    pub fn has_inject_fallback(&self) -> Result<bool, StoreError> {
        Ok(self.get_progress(PROGRESS_INJECT_FALLBACK)? == Some("true".into()))
    }

    pub fn mark_llm_skipped_event(&self) -> Result<(), StoreError> {
        self.set_progress(PROGRESS_LLM_SKIPPED, "true")
    }

    pub fn has_llm_skipped_event(&self) -> Result<bool, StoreError> {
        Ok(self.get_progress(PROGRESS_LLM_SKIPPED)? == Some("true".into()))
    }

    pub fn record_dictation_language(&self, lang: &str) -> Result<(), StoreError> {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let key = format!("{PROGRESS_POLYLOT_PREFIX}{today}");
        let mut langs: Vec<String> = self
            .get_progress(&key)?
            .map(|v| v.split(',').map(str::to_string).collect())
            .unwrap_or_default();
        let normalized = lang.to_ascii_lowercase();
        if !langs.iter().any(|l| l == &normalized) {
            langs.push(normalized);
            self.set_progress(&key, &langs.join(","))?;
        }
        Ok(())
    }

    pub fn has_polyglot_today(&self) -> Result<bool, StoreError> {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let key = format!("{PROGRESS_POLYLOT_PREFIX}{today}");
        let Some(value) = self.get_progress(&key)? else {
            return Ok(false);
        };
        let langs: Vec<&str> = value.split(',').filter(|s| !s.is_empty()).collect();
        Ok(langs.len() >= 2)
    }

    pub fn build_achievements_summary(
        &self,
        progress_fn: impl Fn(&crate::achievements::conditions::Condition) -> Option<AchievementProgress>,
    ) -> Result<AchievementsSummary, StoreError> {
        let unlocked = self.list_unlocked_achievements()?;
        let unseen_count = self.count_unseen_achievements()?;
        let total_count = ALL_ACHIEVEMENTS.len() as i64;

        let achievements = ALL_ACHIEVEMENTS
            .iter()
            .map(|def| {
                let row = unlocked.get(def.id);
                let unlocked_flag = row.is_some();
                AchievementState {
                    id: def.id.to_string(),
                    tier: def.tier.as_str().to_string(),
                    category: def.category.as_str().to_string(),
                    secret: def.secret,
                    unlocked: unlocked_flag,
                    unlocked_at: row.map(|r| r.unlocked_at.clone()),
                    seen: row.map(|r| r.seen).unwrap_or(false),
                    progress: if unlocked_flag || def.secret {
                        None
                    } else {
                        progress_fn(&def.condition)
                    },
                }
            })
            .collect();

        Ok(AchievementsSummary {
            unlocked_count: unlocked.len() as i64,
            total_count,
            unseen_count,
            achievements,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn test_store() -> Store {
        let conn = Connection::open_in_memory().unwrap();
        crate::store::init_test_schema(&conn).unwrap();
        migrate_achievement_tables(&conn).unwrap();
        Store::from_connection(conn)
    }

    #[test]
    fn unlock_is_idempotent() {
        let store = test_store();
        assert!(store.unlock_achievement("first_breath", false).unwrap());
        assert!(!store.unlock_achievement("first_breath", false).unwrap());
    }

    #[test]
    fn cancel_streak_tracks() {
        let store = test_store();
        assert_eq!(store.increment_cancel_streak().unwrap(), 1);
        assert_eq!(store.increment_cancel_streak().unwrap(), 2);
        store.reset_cancel_streak().unwrap();
        assert_eq!(store.increment_cancel_streak().unwrap(), 1);
    }
}
