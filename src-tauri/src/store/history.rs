use chrono::{Duration, Local, NaiveDate};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::db::{Store, StoreError};
use super::DictionarySource;

pub const TYPING_SPEED_BASELINE_WPM: f64 = 40.0;
pub const DEFAULT_LIST_LIMIT: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DictationEntry {
    pub id: i64,
    pub text: String,
    #[serde(rename = "wordCount")]
    pub word_count: i64,
    #[serde(rename = "audioDurationMs")]
    pub audio_duration_ms: i64,
    #[serde(rename = "sttMs")]
    pub stt_ms: i64,
    #[serde(rename = "llmMs")]
    pub llm_ms: i64,
    #[serde(rename = "injectMs")]
    pub inject_ms: i64,
    #[serde(rename = "totalMs")]
    pub total_ms: i64,
    #[serde(rename = "appExe")]
    pub app_exe: Option<String>,
    #[serde(rename = "appTitle")]
    pub app_title: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDictation {
    pub text: String,
    pub audio_duration_ms: u64,
    pub stt_ms: u64,
    pub llm_ms: u64,
    pub inject_ms: u64,
    pub total_ms: u64,
    pub app_exe: Option<String>,
    pub app_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencySnapshot {
    #[serde(rename = "sttMs")]
    pub stt_ms: i64,
    #[serde(rename = "llmMs")]
    pub llm_ms: i64,
    #[serde(rename = "injectMs")]
    pub inject_ms: i64,
    #[serde(rename = "totalMs")]
    pub total_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppUsageEntry {
    #[serde(rename = "exeName")]
    pub exe_name: String,
    #[serde(rename = "dictationCount")]
    pub dictation_count: i64,
    #[serde(rename = "wordCount")]
    pub word_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyActivityEntry {
    pub date: String,
    #[serde(rename = "wordCount")]
    pub word_count: i64,
    #[serde(rename = "dictationCount")]
    pub dictation_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentLatencyEntry {
    #[serde(rename = "sttMs")]
    pub stt_ms: i64,
    #[serde(rename = "llmMs")]
    pub llm_ms: i64,
    #[serde(rename = "injectMs")]
    pub inject_ms: i64,
    #[serde(rename = "totalMs")]
    pub total_ms: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreakInfo {
    #[serde(rename = "currentStreak")]
    pub current_streak: i64,
    #[serde(rename = "bestStreak")]
    pub best_streak: i64,
    #[serde(rename = "activeToday")]
    pub active_today: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimatedTimeSaved {
    #[serde(rename = "minutesSaved")]
    pub minutes_saved: i64,
    #[serde(rename = "baselineWpm")]
    pub baseline_wpm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourAppHeatmapCell {
    pub hour: i64,
    #[serde(rename = "exeName")]
    pub exe_name: String,
    #[serde(rename = "wordCount")]
    pub word_count: i64,
    #[serde(rename = "dictationCount")]
    pub dictation_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insights {
    #[serde(rename = "lastLatency")]
    pub last_latency: Option<LatencySnapshot>,
    #[serde(rename = "wordsToday")]
    pub words_today: i64,
    #[serde(rename = "dictationsToday")]
    pub dictations_today: i64,
    #[serde(rename = "totalWords")]
    pub total_words: i64,
    #[serde(rename = "totalDictations")]
    pub total_dictations: i64,
    #[serde(rename = "averageWpm")]
    pub average_wpm: f64,
    #[serde(rename = "wpmVsTypingPercent")]
    pub wpm_vs_typing_percent: i64,
    #[serde(rename = "averageLatencyMs")]
    pub average_latency_ms: i64,
    #[serde(rename = "totalAudioMinutes")]
    pub total_audio_minutes: i64,
    #[serde(rename = "learnedCorrections")]
    pub learned_corrections: i64,
    #[serde(rename = "appUsage")]
    pub app_usage: Vec<AppUsageEntry>,
    #[serde(rename = "dailyActivity")]
    pub daily_activity: Vec<DailyActivityEntry>,
    #[serde(rename = "recentLatency")]
    pub recent_latency: Vec<RecentLatencyEntry>,
    pub streak: StreakInfo,
    #[serde(rename = "timeSaved")]
    pub time_saved: EstimatedTimeSaved,
    #[serde(rename = "hourAppHeatmap")]
    pub hour_app_heatmap: Vec<HourAppHeatmapCell>,
}

pub fn count_words(text: &str) -> usize {
    text.split_whitespace()
        .filter(|token| !token.is_empty())
        .count()
}

pub fn dictation_wpm(word_count: usize, audio_duration_ms: u64) -> f64 {
    if word_count == 0 || audio_duration_ms == 0 {
        return 0.0;
    }
    let minutes = audio_duration_ms as f64 / 60_000.0;
    if minutes <= 0.0 {
        return 0.0;
    }
    word_count as f64 / minutes
}

pub fn wpm_vs_typing_percent(wpm: f64) -> i64 {
    if wpm <= 0.0 {
        return 0;
    }
    ((wpm / TYPING_SPEED_BASELINE_WPM) * 100.0).round() as i64
}

/// Minutes saved vs typing the same words at `TYPING_SPEED_BASELINE_WPM`.
pub fn estimate_time_saved_minutes(total_words: i64, total_audio_ms: i64) -> i64 {
    if total_words <= 0 || total_audio_ms <= 0 {
        return 0;
    }
    let typing_minutes = total_words as f64 / TYPING_SPEED_BASELINE_WPM;
    let speaking_minutes = total_audio_ms as f64 / 60_000.0;
    (typing_minutes - speaking_minutes).max(0.0).round() as i64
}

pub fn compute_streak_info(active_days: &[String]) -> StreakInfo {
    if active_days.is_empty() {
        return StreakInfo {
            current_streak: 0,
            best_streak: 0,
            active_today: false,
        };
    }

    let parsed: Vec<NaiveDate> = active_days
        .iter()
        .filter_map(|day| NaiveDate::parse_from_str(day, "%Y-%m-%d").ok())
        .collect();

    if parsed.is_empty() {
        return StreakInfo {
            current_streak: 0,
            best_streak: 0,
            active_today: false,
        };
    }

    let today = Local::now().date_naive();
    let active_today = parsed.contains(&today);

    let mut best = 0_i64;
    let mut run = 1_i64;
    for index in 1..parsed.len() {
        if parsed[index].signed_duration_since(parsed[index - 1]).num_days() == 1 {
            run += 1;
        } else {
            best = best.max(run);
            run = 1;
        }
    }
    best = best.max(run);

    let last_active = *parsed.last().expect("parsed non-empty");
    let days_since_last = today.signed_duration_since(last_active).num_days();
    let current_streak = if days_since_last > 1 {
        0
    } else {
        let mut count = 0_i64;
        let mut cursor = last_active;
        let active_set: std::collections::HashSet<NaiveDate> = parsed.into_iter().collect();
        while active_set.contains(&cursor) {
            count += 1;
            cursor -= Duration::days(1);
        }
        count
    };

    StreakInfo {
        current_streak,
        best_streak: best,
        active_today,
    }
}

impl Store {
    pub fn insert_dictation(&self, entry: &NewDictation) -> Result<i64, StoreError> {
        let word_count = count_words(&entry.text) as i64;
        let conn = self.connection().lock().expect("store mutex poisoned");
        conn.execute(
            "INSERT INTO dictations (
                text, word_count, audio_duration_ms, stt_ms, llm_ms, inject_ms, total_ms,
                app_exe, app_title
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                entry.text,
                word_count,
                entry.audio_duration_ms as i64,
                entry.stt_ms as i64,
                entry.llm_ms as i64,
                entry.inject_ms as i64,
                entry.total_ms as i64,
                entry.app_exe,
                entry.app_title,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_dictations(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<DictationEntry>, StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, text, word_count, audio_duration_ms, stt_ms, llm_ms, inject_ms,
                    total_ms, app_exe, app_title, created_at
             FROM dictations
             ORDER BY datetime(created_at) DESC, id DESC
             LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt.query_map(params![limit as i64, offset as i64], map_dictation_row)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    pub fn search_dictations(
        &self,
        query: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<DictationEntry>, StoreError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return self.list_dictations(limit, offset);
        }

        let conn = self.connection().lock().expect("store mutex poisoned");
        let fts_query = build_fts_query(trimmed);
        let mut stmt = conn.prepare(
            "SELECT d.id, d.text, d.word_count, d.audio_duration_ms, d.stt_ms, d.llm_ms,
                    d.inject_ms, d.total_ms, d.app_exe, d.app_title, d.created_at
             FROM dictations_fts fts
             JOIN dictations d ON d.id = fts.rowid
             WHERE dictations_fts MATCH ?1
             ORDER BY datetime(d.created_at) DESC, d.id DESC
             LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(
            params![fts_query, limit as i64, offset as i64],
            map_dictation_row,
        )?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    pub fn count_dictations(&self) -> Result<i64, StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        conn.query_row("SELECT COUNT(*) FROM dictations", [], |row| row.get(0))
            .map_err(StoreError::from)
    }

    pub fn count_search_dictations(&self, query: &str) -> Result<i64, StoreError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return self.count_dictations();
        }

        let conn = self.connection().lock().expect("store mutex poisoned");
        let fts_query = build_fts_query(trimmed);
        conn.query_row(
            "SELECT COUNT(*)
             FROM dictations_fts fts
             JOIN dictations d ON d.id = fts.rowid
             WHERE dictations_fts MATCH ?1",
            params![fts_query],
            |row| row.get(0),
        )
        .map_err(StoreError::from)
    }

    pub fn get_dictation(&self, id: i64) -> Result<Option<DictationEntry>, StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, text, word_count, audio_duration_ms, stt_ms, llm_ms, inject_ms,
                    total_ms, app_exe, app_title, created_at
             FROM dictations
             WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(map_dictation_row(row)?)),
            None => Ok(None),
        }
    }

    pub fn get_insights(&self) -> Result<Insights, StoreError> {
        let conn = self.connection().lock().expect("store mutex poisoned");

        let words_today: i64 = conn.query_row(
            "SELECT COALESCE(SUM(word_count), 0)
             FROM dictations
             WHERE date(created_at, 'localtime') = date('now', 'localtime')",
            [],
            |row| row.get(0),
        )?;

        let dictations_today: i64 = conn.query_row(
            "SELECT COUNT(*)
             FROM dictations
             WHERE date(created_at, 'localtime') = date('now', 'localtime')",
            [],
            |row| row.get(0),
        )?;

        let total_words: i64 = conn.query_row(
            "SELECT COALESCE(SUM(word_count), 0) FROM dictations",
            [],
            |row| row.get(0),
        )?;

        let total_dictations: i64 =
            conn.query_row("SELECT COUNT(*) FROM dictations", [], |row| row.get(0))?;

        let average_latency_ms: i64 = conn.query_row(
            "SELECT COALESCE(CAST(ROUND(AVG(total_ms)) AS INTEGER), 0)
             FROM dictations
             WHERE total_ms > 0",
            [],
            |row| row.get(0),
        )?;

        let total_audio_minutes: i64 = conn.query_row(
            "SELECT COALESCE(CAST(ROUND(SUM(audio_duration_ms) / 60000.0) AS INTEGER), 0)
             FROM dictations",
            [],
            |row| row.get(0),
        )?;

        let (weighted_words, weighted_duration_ms): (i64, i64) = conn.query_row(
            "SELECT COALESCE(SUM(word_count), 0), COALESCE(SUM(audio_duration_ms), 0)
             FROM dictations
             WHERE audio_duration_ms > 0",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let average_wpm = if weighted_duration_ms > 0 {
            dictation_wpm(weighted_words as usize, weighted_duration_ms as u64)
        } else {
            0.0
        };

        let learned_corrections: i64 = conn.query_row(
            "SELECT COUNT(*) FROM dictionary WHERE source = ?1",
            params![DictionarySource::Learned.as_str()],
            |row| row.get(0),
        )?;

        let last_latency = conn
            .query_row(
                "SELECT stt_ms, llm_ms, inject_ms, total_ms
                 FROM dictations
                 ORDER BY datetime(created_at) DESC, id DESC
                 LIMIT 1",
                [],
                |row| {
                    Ok(LatencySnapshot {
                        stt_ms: row.get(0)?,
                        llm_ms: row.get(1)?,
                        inject_ms: row.get(2)?,
                        total_ms: row.get(3)?,
                    })
                },
            )
            .ok();

        let mut app_stmt = conn.prepare(
            "SELECT COALESCE(app_exe, 'autre') AS exe_name,
                    COUNT(*) AS dictation_count,
                    COALESCE(SUM(word_count), 0) AS word_count
             FROM dictations
             GROUP BY exe_name
             ORDER BY word_count DESC, dictation_count DESC
             LIMIT 8",
        )?;
        let app_rows = app_stmt.query_map([], |row| {
            Ok(AppUsageEntry {
                exe_name: row.get(0)?,
                dictation_count: row.get(1)?,
                word_count: row.get(2)?,
            })
        })?;
        let app_usage = app_rows.collect::<Result<Vec<_>, _>>()?;
        let daily_activity = fetch_daily_activity(&conn)?;
        let recent_latency = fetch_recent_latency(&conn)?;
        let active_days = fetch_active_days(&conn)?;
        let streak = compute_streak_info(&active_days);
        let total_audio_ms: i64 = conn.query_row(
            "SELECT COALESCE(SUM(audio_duration_ms), 0) FROM dictations",
            [],
            |row| row.get(0),
        )?;
        let time_saved = EstimatedTimeSaved {
            minutes_saved: estimate_time_saved_minutes(total_words, total_audio_ms),
            baseline_wpm: TYPING_SPEED_BASELINE_WPM,
        };
        let hour_app_heatmap = fetch_hour_app_heatmap(&conn)?;

        Ok(Insights {
            last_latency,
            words_today,
            dictations_today,
            total_words,
            total_dictations,
            average_wpm,
            wpm_vs_typing_percent: wpm_vs_typing_percent(average_wpm),
            average_latency_ms,
            total_audio_minutes,
            learned_corrections,
            app_usage,
            daily_activity,
            recent_latency,
            streak,
            time_saved,
            hour_app_heatmap,
        })
    }
}

fn fetch_daily_activity(
    conn: &rusqlite::Connection,
) -> Result<Vec<DailyActivityEntry>, StoreError> {
    let mut stmt = conn.prepare(
        "WITH RECURSIVE days(day) AS (
            SELECT date('now', 'localtime', '-6 days')
            UNION ALL
            SELECT date(day, '+1 day') FROM days WHERE day < date('now', 'localtime')
        )
        SELECT days.day,
               COALESCE(SUM(d.word_count), 0),
               COALESCE(COUNT(d.id), 0)
        FROM days
        LEFT JOIN dictations d ON date(d.created_at, 'localtime') = days.day
        GROUP BY days.day
        ORDER BY days.day ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(DailyActivityEntry {
            date: row.get(0)?,
            word_count: row.get(1)?,
            dictation_count: row.get(2)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(StoreError::from)
}

fn fetch_active_days(conn: &rusqlite::Connection) -> Result<Vec<String>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT date(created_at, 'localtime') AS day
         FROM dictations
         ORDER BY day ASC",
    )?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(StoreError::from)
}

fn fetch_hour_app_heatmap(
    conn: &rusqlite::Connection,
) -> Result<Vec<HourAppHeatmapCell>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT CAST(strftime('%H', created_at, 'localtime') AS INTEGER) AS hour,
                COALESCE(app_exe, 'autre') AS exe_name,
                COUNT(*) AS dictation_count,
                COALESCE(SUM(word_count), 0) AS word_count
         FROM dictations
         WHERE date(created_at, 'localtime') >= date('now', 'localtime', '-6 days')
         GROUP BY hour, exe_name
         HAVING word_count > 0
         ORDER BY word_count DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(HourAppHeatmapCell {
            hour: row.get(0)?,
            exe_name: row.get(1)?,
            dictation_count: row.get(2)?,
            word_count: row.get(3)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(StoreError::from)
}

fn fetch_recent_latency(
    conn: &rusqlite::Connection,
) -> Result<Vec<RecentLatencyEntry>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT stt_ms, llm_ms, inject_ms, total_ms, created_at
         FROM dictations
         ORDER BY datetime(created_at) DESC, id DESC
         LIMIT 8",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(RecentLatencyEntry {
            stt_ms: row.get(0)?,
            llm_ms: row.get(1)?,
            inject_ms: row.get(2)?,
            total_ms: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;
    let mut entries: Vec<RecentLatencyEntry> = rows.collect::<Result<Vec<_>, _>>()?;
    entries.reverse();
    Ok(entries)
}

fn map_dictation_row(row: &rusqlite::Row<'_>) -> Result<DictationEntry, rusqlite::Error> {
    Ok(DictationEntry {
        id: row.get(0)?,
        text: row.get(1)?,
        word_count: row.get(2)?,
        audio_duration_ms: row.get(3)?,
        stt_ms: row.get(4)?,
        llm_ms: row.get(5)?,
        inject_ms: row.get(6)?,
        total_ms: row.get(7)?,
        app_exe: row.get(8)?,
        app_title: row.get(9)?,
        created_at: row.get(10)?,
    })
}

fn build_fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .map(|token| {
            let escaped = token.replace('"', "\"\"");
            format!("\"{escaped}\"*")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    use crate::store::db::init_schema;

    fn test_store() -> Store {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        Store::from_connection(conn)
    }

    fn sample_dictation(text: &str) -> NewDictation {
        NewDictation {
            text: text.to_string(),
            audio_duration_ms: 6_000,
            stt_ms: 800,
            llm_ms: 1_200,
            inject_ms: 150,
            total_ms: 2_500,
            app_exe: Some("notepad.exe".into()),
            app_title: Some("Untitled - Notepad".into()),
        }
    }

    #[test]
    fn count_words_splits_on_whitespace() {
        assert_eq!(count_words("bonjour le monde"), 3);
        assert_eq!(count_words("  one   two  "), 2);
        assert_eq!(count_words(""), 0);
    }

    #[test]
    fn dictation_wpm_computes_rate() {
        assert!((dictation_wpm(10, 60_000) - 10.0).abs() < f64::EPSILON);
        assert_eq!(dictation_wpm(0, 1_000), 0.0);
        assert_eq!(dictation_wpm(5, 0), 0.0);
    }

    #[test]
    fn wpm_vs_typing_percent_uses_baseline() {
        assert_eq!(wpm_vs_typing_percent(80.0), 200);
        assert_eq!(wpm_vs_typing_percent(0.0), 0);
    }

    #[test]
    fn insert_and_list_roundtrip() {
        let store = test_store();
        let id = store
            .insert_dictation(&sample_dictation("bonjour Calliop"))
            .unwrap();
        assert!(id > 0);

        let entries = store.list_dictations(10, 0).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "bonjour Calliop");
        assert_eq!(entries[0].word_count, 2);
        assert_eq!(entries[0].app_exe.as_deref(), Some("notepad.exe"));
    }

    #[test]
    fn list_dictations_supports_pagination() {
        let store = test_store();
        for index in 0..5 {
            store
                .insert_dictation(&sample_dictation(&format!("entry {index}")))
                .unwrap();
        }

        let page_one = store.list_dictations(2, 0).unwrap();
        let page_two = store.list_dictations(2, 2).unwrap();

        assert_eq!(page_one.len(), 2);
        assert_eq!(page_two.len(), 2);
        assert_ne!(page_one[0].id, page_two[0].id);
        assert_eq!(store.count_dictations().unwrap(), 5);
    }

    #[test]
    fn fts_search_finds_matching_text() {
        let store = test_store();
        store
            .insert_dictation(&sample_dictation("message Slack pour l equipe"))
            .unwrap();
        store
            .insert_dictation(&sample_dictation("commit message technique"))
            .unwrap();

        let hits = store.search_dictations("Slack", 10, 0).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].text.contains("Slack"));
    }

    #[test]
    fn estimate_time_saved_minutes_compares_to_typing() {
        // 80 words at 40 WPM = 2 min typing; 1 min speaking -> 1 min saved
        assert_eq!(estimate_time_saved_minutes(80, 60_000), 1);
        assert_eq!(estimate_time_saved_minutes(0, 60_000), 0);
        assert_eq!(estimate_time_saved_minutes(40, 120_000), 0);
    }

    #[test]
    fn compute_streak_info_counts_consecutive_days() {
        let today = Local::now().date_naive();
        let yesterday = today - Duration::days(1);
        let days = vec![
            (today - Duration::days(4)).to_string(),
            (today - Duration::days(3)).to_string(),
            (today - Duration::days(2)).to_string(),
            yesterday.to_string(),
            today.to_string(),
        ];
        let streak = compute_streak_info(&days);
        assert_eq!(streak.current_streak, 5);
        assert_eq!(streak.best_streak, 5);
        assert!(streak.active_today);
    }

    #[test]
    fn compute_streak_info_resets_when_gap() {
        let today = Local::now().date_naive();
        let days = vec![
            (today - Duration::days(5)).to_string(),
            (today - Duration::days(3)).to_string(),
        ];
        let streak = compute_streak_info(&days);
        assert_eq!(streak.current_streak, 0);
        assert_eq!(streak.best_streak, 1);
    }

    #[test]
    fn get_insights_aggregates_counts() {
        let store = test_store();
        store
            .insert_dictation(&sample_dictation("un deux trois"))
            .unwrap();
        store
            .insert_dictation(&NewDictation {
                text: "quatre cinq".into(),
                audio_duration_ms: 3_000,
                stt_ms: 400,
                llm_ms: 0,
                inject_ms: 100,
                total_ms: 600,
                app_exe: Some("code.exe".into()),
                app_title: None,
            })
            .unwrap();
        store
            .add_word("Calliope", DictionarySource::Learned, None)
            .unwrap()
            .expect("word inserted");

        let insights = store.get_insights().unwrap();
        assert_eq!(insights.total_words, 5);
        assert_eq!(insights.words_today, 5);
        assert_eq!(insights.dictations_today, 2);
        assert_eq!(insights.total_dictations, 2);
        assert_eq!(insights.average_latency_ms, 1550);
        assert!(insights.total_audio_minutes >= 0);
        assert!(insights.average_wpm > 0.0);
        assert_eq!(insights.learned_corrections, 1);
        assert_eq!(insights.app_usage.len(), 2);
        assert!(insights.last_latency.is_some());
        assert_eq!(insights.daily_activity.len(), 7);
        assert_eq!(insights.recent_latency.len(), 2);
        assert!(insights.time_saved.minutes_saved >= 0);
        assert_eq!(insights.streak.current_streak, 1);
        assert!(!insights.hour_app_heatmap.is_empty());
    }
}
