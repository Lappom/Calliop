use std::sync::Arc;

use chrono::{Datelike, Local, Timelike, Weekday};
use rusqlite::params;
use tauri::{AppHandle, Emitter};

use crate::store::{
    dictation_wpm, estimate_time_saved_minutes, Store, StoreError,
};

use super::conditions::Condition;
use super::definitions::{achievement_by_id, ALL_ACHIEVEMENTS};
use super::store::migrate_achievement_tables;
use super::types::{AchievementProgress, AchievementUnlockedPayload, AchievementsSummary};

#[derive(Debug, Clone)]
pub struct DictationEvent {
    pub text: String,
    pub word_count: u64,
    pub audio_duration_ms: u64,
    pub total_ms: u64,
    pub app_exe: Option<String>,
    pub inject_fallback: bool,
    pub llm_skipped: bool,
    pub stt_language: String,
}

impl DictationEvent {
    pub fn wpm(&self) -> f64 {
        dictation_wpm(self.word_count as usize, self.audio_duration_ms)
    }

    pub fn local_hour(&self) -> u8 {
        Local::now().hour() as u8
    }

    pub fn local_weekday(&self) -> Weekday {
        Local::now().weekday()
    }
}

#[derive(Debug, Default)]
struct AggregateStats {
    total_dictations: i64,
    total_words: i64,
    total_audio_ms: i64,
    dictations_today: i64,
    words_today: i64,
    current_streak: i64,
    distinct_active_days: i64,
    total_active_days: i64,
    learned_corrections: i64,
    manual_dictionary_words: i64,
    snippet_count: i64,
    app_context_rules: i64,
    onboarding_done: bool,
    time_saved_minutes: i64,
    average_wpm: f64,
    max_single_word_count: i64,
    min_total_ms: Option<i64>,
    max_wpm: f64,
    max_app_dictations: i64,
    distinct_apps: i64,
    has_calliop_text: bool,
    has_night_owl: bool,
    has_early_bird: bool,
    has_night_shift: bool,
    has_weekend: bool,
    has_long_audio: bool,
    has_three_words: bool,
    app_exes: Vec<String>,
    cancel_streak: u32,
    inject_fallback: bool,
    llm_skipped_event: bool,
    polyglot_today: bool,
}

pub struct AchievementEngine {
    store: Arc<Store>,
}

impl AchievementEngine {
    pub fn new(store: Arc<Store>) -> Self {
        Self { store }
    }

    pub fn store(&self) -> &Arc<Store> {
        &self.store
    }

    pub fn retroactive_scan(&self, _app: &AppHandle) -> Result<(), StoreError> {
        let _ = self.evaluate_all(true)?;
        Ok(())
    }

    pub fn on_dictation(&self, app: &AppHandle, event: &DictationEvent) -> Result<(), StoreError> {
        let _ = self.store.reset_cancel_streak();
        if event.inject_fallback {
            let _ = self.store.mark_inject_fallback();
        }
        if event.llm_skipped {
            let _ = self.store.mark_llm_skipped_event();
        }
        if !event.stt_language.is_empty() {
            let _ = self.store.record_dictation_language(&event.stt_language);
        }
        let newly = self.evaluate_with_event(Some(event), false)?;
        self.emit_unlocks(app, newly);
        Ok(())
    }

    pub fn on_cancel(&self, app: &AppHandle) -> Result<(), StoreError> {
        let streak = self.store.increment_cancel_streak()?;
        let mut newly = self.evaluate_all(false)?;
        if streak >= 3 {
            if self.store.unlock_achievement("triple_no", false)? {
                newly.push("triple_no".to_string());
            }
        }
        self.emit_unlocks(app, newly);
        Ok(())
    }

    pub fn on_learned_correction(&self, app: &AppHandle) -> Result<(), StoreError> {
        let newly = self.evaluate_all(false)?;
        self.emit_unlocks(app, newly);
        Ok(())
    }

    pub fn on_feature_change(&self, app: &AppHandle) -> Result<(), StoreError> {
        let newly = self.evaluate_all(false)?;
        self.emit_unlocks(app, newly);
        Ok(())
    }

    pub fn on_inject_fallback(&self, app: &AppHandle) -> Result<(), StoreError> {
        let _ = self.store.mark_inject_fallback();
        let newly = self.evaluate_all(false)?;
        self.emit_unlocks(app, newly);
        Ok(())
    }

    pub fn on_llm_skipped(&self, app: &AppHandle) -> Result<(), StoreError> {
        let _ = self.store.mark_llm_skipped_event();
        let newly = self.evaluate_all(false)?;
        self.emit_unlocks(app, newly);
        Ok(())
    }

    pub fn get_summary(&self) -> Result<AchievementsSummary, StoreError> {
        let stats = self.load_stats()?;
        self.store.build_achievements_summary(|condition| {
            progress_for_condition(condition, &stats, None)
        })
    }

    fn evaluate_all(&self, silent: bool) -> Result<Vec<String>, StoreError> {
        self.evaluate_with_event(None, silent)
    }

    fn evaluate_with_event(
        &self,
        event: Option<&DictationEvent>,
        silent: bool,
    ) -> Result<Vec<String>, StoreError> {
        let stats = self.load_stats()?;
        let mut newly_unlocked = Vec::new();
        for def in ALL_ACHIEVEMENTS {
            if self.store.is_achievement_unlocked(def.id)? {
                continue;
            }
            if condition_met(&def.condition, &stats, event) {
                if self.store.unlock_achievement(def.id, silent)? {
                    newly_unlocked.push(def.id.to_string());
                }
            }
        }
        Ok(newly_unlocked)
    }

    fn emit_unlocks(&self, app: &AppHandle, ids: Vec<String>) {
        for id in ids {
            if let Some(def) = achievement_by_id(&id) {
                let _ = app.emit(
                    "achievement-unlocked",
                    AchievementUnlockedPayload {
                        id: id.clone(),
                        tier: def.tier.as_str().to_string(),
                        secret: def.secret,
                    },
                );
            }
        }
    }

    fn load_stats(&self) -> Result<AggregateStats, StoreError> {
        let insights = self.store.get_insights()?;
        let onboarding_done = self
            .store
            .get_bool(crate::store::KEY_ONBOARDING_DONE, false)
            .unwrap_or(false);
        let cancel_streak = self
            .store
            .get_progress("cancel_streak")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let inject_fallback = self.store.has_inject_fallback()?;
        let llm_skipped_event = self.store.has_llm_skipped_event()?;
        let polyglot_today = self.store.has_polyglot_today()?;

        let conn = self.store.connection().lock().expect("store mutex poisoned");

        let manual_dictionary_words: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM dictionary WHERE source = 'manual'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let snippet_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM snippets", [], |r| r.get(0))
            .unwrap_or(0);
        let app_context_rules: i64 = conn
            .query_row("SELECT COUNT(*) FROM app_context_rules", [], |r| r.get(0))
            .unwrap_or(0);
        let max_single_word_count: i64 = conn
            .query_row("SELECT COALESCE(MAX(word_count), 0) FROM dictations", [], |r| {
                r.get(0)
            })
            .unwrap_or(0);
        let min_total_ms: Option<i64> = conn
            .query_row(
                "SELECT MIN(total_ms) FROM dictations WHERE total_ms > 0",
                [],
                |r| r.get(0),
            )
            .ok();
        let max_wpm: f64 = conn
            .query_row(
                "SELECT COALESCE(MAX(
                    CAST(word_count AS REAL) * 60000.0 / NULLIF(audio_duration_ms, 0)
                ), 0) FROM dictations WHERE audio_duration_ms > 0",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0.0);
        let max_app_dictations: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(cnt), 0) FROM (
                    SELECT COUNT(*) AS cnt FROM dictations
                    WHERE app_exe IS NOT NULL AND app_exe != ''
                    GROUP BY app_exe
                )",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let distinct_apps: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT app_exe) FROM dictations
                 WHERE app_exe IS NOT NULL AND app_exe != ''",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let has_calliop_text: bool = conn
            .query_row(
                "SELECT 1 FROM dictations WHERE LOWER(text) LIKE '%calliop%' LIMIT 1",
                [],
                |_| Ok(()),
            )
            .is_ok();
        let has_night_owl: bool = hour_range_exists(&conn, 0, 5)?;
        let has_early_bird: bool = hour_range_exists(&conn, 5, 7)?;
        let has_night_shift: bool = hour_range_exists(&conn, 22, 24)?;
        let has_weekend: bool = conn
            .query_row(
                "SELECT 1 FROM dictations
                 WHERE CAST(strftime('%w', created_at, 'localtime') AS INTEGER) IN (0, 6)
                 LIMIT 1",
                [],
                |_| Ok(()),
            )
            .is_ok();
        let has_long_audio: bool = conn
            .query_row(
                "SELECT 1 FROM dictations WHERE audio_duration_ms >= 120000 LIMIT 1",
                [],
                |_| Ok(()),
            )
            .is_ok();
        let has_three_words: bool = conn
            .query_row(
                "SELECT 1 FROM dictations WHERE word_count = 3 LIMIT 1",
                [],
                |_| Ok(()),
            )
            .is_ok();
        let mut app_exes: Vec<String> = Vec::new();
        {
            let mut stmt = conn.prepare(
                "SELECT DISTINCT app_exe FROM dictations
                 WHERE app_exe IS NOT NULL AND app_exe != ''",
            )?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            for row in rows {
                app_exes.push(row?);
            }
        }

        let mut stats = AggregateStats {
            total_dictations: insights.total_dictations,
            total_words: insights.total_words,
            total_audio_ms: insights.total_audio_minutes * 60_000,
            dictations_today: insights.dictations_today,
            words_today: insights.words_today,
            current_streak: insights.streak.current_streak,
            distinct_active_days: count_total_active_days(&conn)?,
            total_active_days: count_total_active_days(&conn)?,
            learned_corrections: insights.learned_corrections,
            manual_dictionary_words,
            snippet_count,
            app_context_rules,
            onboarding_done,
            time_saved_minutes: insights.time_saved.minutes_saved,
            average_wpm: insights.average_wpm,
            max_single_word_count,
            min_total_ms,
            max_wpm,
            max_app_dictations,
            distinct_apps,
            has_calliop_text,
            has_night_owl,
            has_early_bird,
            has_night_shift,
            has_weekend,
            has_long_audio,
            has_three_words,
            app_exes,
            cancel_streak,
            inject_fallback,
            llm_skipped_event,
            polyglot_today,
        };

        stats.time_saved_minutes =
            estimate_time_saved_minutes(stats.total_words, stats.total_audio_ms);

        Ok(stats)
    }
}

fn hour_range_exists(
    conn: &rusqlite::Connection,
    start_hour: u8,
    end_hour: u8,
) -> Result<bool, StoreError> {
    let sql = format!(
        "SELECT 1 FROM dictations
         WHERE CAST(strftime('%H', created_at, 'localtime') AS INTEGER) >= ?1
           AND CAST(strftime('%H', created_at, 'localtime') AS INTEGER) < ?2
         LIMIT 1"
    );
    Ok(conn
        .query_row(&sql, params![start_hour, end_hour], |_| Ok(()))
        .is_ok())
}

fn count_total_active_days(conn: &rusqlite::Connection) -> Result<i64, StoreError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT date(created_at, 'localtime')) FROM dictations",
        [],
        |r| r.get(0),
    )?;
    Ok(count)
}

fn exe_matches_list(exe: &str, apps: &[&str]) -> bool {
    let lower = exe.to_ascii_lowercase();
    apps.iter()
        .any(|app| lower == app.to_ascii_lowercase() || lower.ends_with(&format!("\\{}", app.to_ascii_lowercase())))
}

fn condition_met(condition: &Condition, stats: &AggregateStats, event: Option<&DictationEvent>) -> bool {
    match condition {
        Condition::TotalDictations(threshold) => stats.total_dictations >= *threshold as i64,
        Condition::TotalWords(threshold) => stats.total_words >= *threshold as i64,
        Condition::CurrentStreak(threshold) => stats.current_streak >= *threshold as i64,
        Condition::DistinctActiveDays(threshold) => stats.distinct_active_days >= *threshold as i64,
        Condition::TotalActiveDays(threshold) => stats.total_active_days >= *threshold as i64,
        Condition::LearnedCorrections(threshold) => stats.learned_corrections >= *threshold as i64,
        Condition::ManualDictionaryWords(threshold) => {
            stats.manual_dictionary_words >= *threshold as i64
        }
        Condition::SnippetCount(threshold) => stats.snippet_count >= *threshold as i64,
        Condition::AppContextRules(threshold) => stats.app_context_rules >= *threshold as i64,
        Condition::OnboardingDone => stats.onboarding_done,
        Condition::DictationInApp(apps) => {
            if let Some(event) = event {
                if let Some(exe) = &event.app_exe {
                    return exe_matches_list(exe, apps);
                }
            }
            stats
                .app_exes
                .iter()
                .any(|exe| exe_matches_list(exe, apps))
        }
        Condition::DictationHourRange(start, end) => {
            if let Some(event) = event {
                let hour = event.local_hour();
                return hour >= *start && hour < *end;
            }
            match (*start, *end) {
                (0, 5) => stats.has_night_owl,
                (5, 7) => stats.has_early_bird,
                (22, 24) => stats.has_night_shift,
                _ => false,
            }
        }
        Condition::DictationWordCountMin(threshold) => {
            stats.max_single_word_count >= *threshold as i64
        }
        Condition::DictationWordCountExact(threshold) => {
            if *threshold == 3 && stats.has_three_words {
                return true;
            }
            event.is_some_and(|e| e.word_count == *threshold)
        }
        Condition::DictationTotalMsMax(threshold) => {
            stats.min_total_ms.is_some_and(|ms| ms < *threshold as i64)
                || event.is_some_and(|e| e.total_ms > 0 && e.total_ms < *threshold)
        }
        Condition::DictationWpmMin(threshold) => stats.max_wpm >= *threshold,
        Condition::AverageWpmMin(threshold) => stats.average_wpm >= *threshold,
        Condition::TimeSavedMinutes(threshold) => stats.time_saved_minutes >= *threshold as i64,
        Condition::DictationsInSameApp(threshold) => stats.max_app_dictations >= *threshold as i64,
        Condition::DistinctApps(threshold) => stats.distinct_apps >= *threshold as i64,
        Condition::TextContains(needle) => stats.has_calliop_text
            || event.is_some_and(|e| e.text.to_ascii_lowercase().contains(&needle.to_ascii_lowercase())),
        Condition::WeekendDictation => stats.has_weekend
            || event.is_some_and(|e| {
                matches!(e.local_weekday(), Weekday::Sat | Weekday::Sun)
            }),
        Condition::AudioDurationMsMin(threshold) => {
            stats.has_long_audio || event.is_some_and(|e| e.audio_duration_ms >= *threshold)
        }
        Condition::InjectFallbackUsed => stats.inject_fallback,
        Condition::ConsecutiveCancels(threshold) => stats.cancel_streak >= *threshold,
        Condition::PolyglotSameDay => stats.polyglot_today,
        Condition::LlmSkippedDictation => stats.llm_skipped_event,
        Condition::DictationsToday(threshold) => stats.dictations_today >= *threshold as i64,
        Condition::WordsToday(_threshold) => false,
    }
}

fn progress_for_condition(
    condition: &Condition,
    stats: &AggregateStats,
    _event: Option<&DictationEvent>,
) -> Option<AchievementProgress> {
    let (current, target) = match condition {
        Condition::TotalDictations(t) => (stats.total_dictations, *t as i64),
        Condition::TotalWords(t) => (stats.total_words, *t as i64),
        Condition::CurrentStreak(t) => (stats.current_streak, *t as i64),
        Condition::DistinctActiveDays(t) => (stats.distinct_active_days, *t as i64),
        Condition::TotalActiveDays(t) => (stats.total_active_days, *t as i64),
        Condition::LearnedCorrections(t) => (stats.learned_corrections, *t as i64),
        Condition::ManualDictionaryWords(t) => (stats.manual_dictionary_words, *t as i64),
        Condition::SnippetCount(t) => (stats.snippet_count, *t as i64),
        Condition::AppContextRules(t) => (stats.app_context_rules, *t as i64),
        Condition::TimeSavedMinutes(t) => (stats.time_saved_minutes, *t as i64),
        Condition::DistinctApps(t) => (stats.distinct_apps, *t as i64),
        Condition::DictationsInSameApp(t) => (stats.max_app_dictations, *t as i64),
        _ => return None,
    };
    if target <= 0 {
        return None;
    }
    Some(AchievementProgress {
        current: current.min(target),
        target,
    })
}

pub fn ensure_achievement_tables(store: &Store) -> Result<(), StoreError> {
    let conn = store.connection().lock().expect("store mutex poisoned");
    migrate_achievement_tables(&conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_dictations_condition() {
        let stats = AggregateStats {
            total_dictations: 3,
            ..AggregateStats::default()
        };
        assert!(condition_met(
            &Condition::TotalDictations(1),
            &stats,
            None
        ));
        assert!(!condition_met(
            &Condition::TotalDictations(10),
            &stats,
            None
        ));
    }

    #[test]
    fn secret_word_count_exact_from_event() {
        let stats = AggregateStats::default();
        let event = DictationEvent {
            text: "one two three".into(),
            word_count: 3,
            audio_duration_ms: 1000,
            total_ms: 500,
            app_exe: None,
            inject_fallback: false,
            llm_skipped: false,
            stt_language: "fr".into(),
        };
        assert!(condition_met(
            &Condition::DictationWordCountExact(3),
            &stats,
            Some(&event),
        ));
    }
}
