use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use rusqlite::{params, Connection};

use crate::settings::Settings;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub id: i64,
    pub started_at: i64,
    pub ended_at: i64,
    pub app_name: String,
    pub window_title: String,
    pub category: String,
    pub idle: bool,
}

pub struct Database {
    path: PathBuf,
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.busy_timeout(Duration::from_secs(3))
            .map_err(|e| e.to_string())?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| e.to_string())?;
        migrate(&conn)?;
        Ok(Self {
            path: path.to_path_buf(),
            conn: Mutex::new(conn),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load_settings(&self) -> Settings {
        let Some(raw) = self.get_raw("settings") else {
            return Settings::default();
        };
        serde_json::from_str(&raw)
            .unwrap_or_else(|_| Settings::default())
            .normalized()
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        let json = serde_json::to_string(&settings.clone().normalized()).map_err(|e| e.to_string())?;
        self.set_raw("settings", &json)
    }

    pub fn file_api_key(&self) -> Option<String> {
        let value = self.get_raw("ai_api_key_fallback")?;
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    }

    pub fn set_file_api_key(&self, key: &str) -> Result<(), String> {
        self.set_raw("ai_api_key_fallback", key)
    }

    pub fn clear_file_api_key(&self) -> Result<(), String> {
        self.set_raw("ai_api_key_fallback", "")
    }

    pub fn key_storage(&self) -> String {
        self.get_raw("ai_key_storage").unwrap_or_else(|| "none".into())
    }

    pub fn set_key_storage(&self, value: &str) -> Result<(), String> {
        self.set_raw("ai_key_storage", value)
    }

    pub fn insert_segment(
        &self,
        started_at: i64,
        ended_at: i64,
        app_name: &str,
        window_title: &str,
        category: &str,
        idle: bool,
    ) -> Result<i64, String> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO segments (started_at, ended_at, app_name, window_title, category, idle)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                started_at,
                ended_at,
                app_name,
                window_title,
                category,
                idle as i64
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    pub fn touch_segment(&self, id: i64, ended_at: i64) -> Result<(), String> {
        let conn = self.lock();
        conn.execute(
            "UPDATE segments SET ended_at = ?1 WHERE id = ?2 AND ended_at < ?1",
            params![ended_at, id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn segments_overlapping(&self, start_ms: i64, end_ms: i64) -> Result<Vec<Segment>, String> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, started_at, ended_at, app_name, window_title, category, idle
                 FROM segments
                 WHERE ended_at > ?1 AND started_at < ?2
                 ORDER BY started_at ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![start_ms, end_ms], |row| {
                Ok(Segment {
                    id: row.get(0)?,
                    started_at: row.get(1)?,
                    ended_at: row.get(2)?,
                    app_name: row.get(3)?,
                    window_title: row.get(4)?,
                    category: row.get(5)?,
                    idle: row.get::<_, i64>(6)? != 0,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn delete_measurements(&self) -> Result<usize, String> {
        let conn = self.lock();
        let n = conn
            .execute("DELETE FROM segments", [])
            .map_err(|e| e.to_string())?;
        conn.execute_batch("VACUUM").ok();
        Ok(n)
    }

    pub fn reset_all(&self) -> Result<(), String> {
        let conn = self.lock();
        conn.execute_batch("DELETE FROM segments; DELETE FROM settings; VACUUM;")
            .map_err(|e| e.to_string())
    }

    fn get_raw(&self, key: &str) -> Option<String> {
        let conn = self.lock();
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .ok()
    }

    fn set_raw(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn migrate(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS segments (
            id INTEGER PRIMARY KEY,
            started_at INTEGER NOT NULL,
            ended_at INTEGER NOT NULL,
            app_name TEXT NOT NULL,
            window_title TEXT NOT NULL DEFAULT '',
            category TEXT NOT NULL,
            idle INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_segments_range ON segments(started_at, ended_at);
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregate::build_receipt;
    use crate::settings::Assumptions;
    use crate::timeutil::{span_for, RangeKind};
    use chrono::{FixedOffset, NaiveDate};
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_db() -> Database {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "tidskvitto-core-{}-{n}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        Database::open(&path).unwrap()
    }

    #[test]
    fn segments_clip_at_midnight() {
        let db = temp_db();
        let tz = FixedOffset::east_opt(3600).unwrap();
        let day = NaiveDate::from_ymd_opt(2026, 11, 24).unwrap();
        let span = span_for(RangeKind::Day, day, &tz);
        // 20 minutes before midnight and 10 minutes after, one segment.
        let start = span.start_ms - 20 * 60_000;
        let end = span.start_ms + 10 * 60_000;
        db.insert_segment(start, end, "Outlook", "Hej", "epost", false)
            .unwrap();
        let previous = span_for(
            RangeKind::Day,
            NaiveDate::from_ymd_opt(2026, 11, 23).unwrap(),
            &tz,
        );
        let segs_today = db
            .segments_overlapping(span.start_ms, span.end_ms)
            .unwrap();
        let today = build_receipt(&segs_today, &span, &Assumptions::default());
        let segs_y = db
            .segments_overlapping(previous.start_ms, previous.end_ms)
            .unwrap();
        let yesterday = build_receipt(&segs_y, &previous, &Assumptions::default());
        assert_eq!(today.email_ms, 10 * 60_000);
        assert_eq!(yesterday.email_ms, 20 * 60_000);
    }

    #[test]
    fn delete_measurements_keeps_settings() {
        let db = temp_db();
        db.save_settings(&Settings::default()).unwrap();
        db.insert_segment(1, 2, "Code", "", "fokus", false).unwrap();
        db.delete_measurements().unwrap();
        assert!(db.segments_overlapping(0, 10).unwrap().is_empty());
        assert!(!db.load_settings().tracking_enabled);
        db.reset_all().unwrap();
        assert!(db.file_api_key().is_none());
    }
}
