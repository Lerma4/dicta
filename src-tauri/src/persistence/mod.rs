use std::{path::Path, sync::Mutex};

use chrono::{Local, NaiveDate};
use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};

use crate::domain::{
    Dashboard, DashboardFilter, ErrorMetric, HeatmapPoint, HistogramPoint, ModelMetric, Session,
    Settings, TimelinePoint,
};

pub struct Repository {
    conn: Mutex<Connection>,
}

impl Repository {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(err)?;
        Self::from_connection(conn)
    }

    pub fn in_memory() -> Result<Self, String> {
        Self::from_connection(Connection::open_in_memory().map_err(err)?)
    }

    fn from_connection(conn: Connection) -> Result<Self, String> {
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(err)?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA secure_delete = ON;",
        )
        .map_err(err)?;
        let version: i64 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(err)?;
        if version > 1 {
            return Err(format!(
                "database schema version {version} is newer than this application"
            ));
        }
        if version == 0 {
            conn.execute_batch("BEGIN IMMEDIATE;
                CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                CREATE TABLE sessions (
                    id TEXT PRIMARY KEY, started_at_utc TEXT NOT NULL, ended_at_utc TEXT,
                    status TEXT NOT NULL, recorded_ms INTEGER NOT NULL DEFAULT 0,
                    inference_ms INTEGER NOT NULL DEFAULT 0, insertion_ms INTEGER NOT NULL DEFAULT 0,
                    words_count INTEGER NOT NULL DEFAULT 0, chars_count INTEGER NOT NULL DEFAULT 0,
                    language_code TEXT NOT NULL DEFAULT '', model_id TEXT NOT NULL DEFAULT '',
                    error_category TEXT, input_mode TEXT NOT NULL DEFAULT '', created_at_utc TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
                );
                CREATE TABLE transcripts (session_id TEXT PRIMARY KEY REFERENCES sessions(id) ON DELETE CASCADE, text TEXT NOT NULL);
                CREATE INDEX sessions_started_idx ON sessions(started_at_utc);
                CREATE INDEX sessions_language_idx ON sessions(language_code);
                CREATE INDEX sessions_model_idx ON sessions(model_id);
                PRAGMA user_version = 1;
                COMMIT;").map_err(err)?;
        }
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn settings(&self) -> Result<Settings, String> {
        let conn = self.lock()?;
        let raw: Option<String> = conn
            .query_row("SELECT value FROM settings WHERE key='settings'", [], |r| {
                r.get(0)
            })
            .optional()
            .map_err(err)?;
        let settings = raw
            .map(|s| serde_json::from_str::<Settings>(&s).map_err(err))
            .transpose()?
            .unwrap_or_default();
        settings.validate()?;
        Ok(settings)
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        settings.validate()?;
        let json = serde_json::to_string(settings).map_err(err)?;
        let mut conn = self.lock()?;
        let tx = conn.transaction().map_err(err)?;
        tx.execute("INSERT INTO settings(key,value) VALUES('settings',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [&json]).map_err(err)?;
        let deleted_transcripts = if !settings.history_enabled {
            tx.execute("DELETE FROM transcripts", []).map_err(err)?
        } else {
            0
        };
        let deleted_sessions = tx.execute("DELETE FROM sessions WHERE datetime(started_at_utc) < datetime('now', '-' || ?1 || ' days')", [settings.retention_days]).map_err(err)?;
        tx.commit().map_err(err)?;
        if deleted_transcripts + deleted_sessions > 0 {
            checkpoint_after_purge(&conn)?;
        }
        Ok(())
    }

    pub fn save_session(&self, session: &Session, history_enabled: bool) -> Result<(), String> {
        session.validate()?;
        let started = utc_timestamp(&session.started_at_utc)?;
        let ended = session
            .ended_at_utc
            .as_deref()
            .map(utc_timestamp)
            .transpose()?;
        let mut conn = self.lock()?;
        let tx = conn.transaction().map_err(err)?;
        tx.execute("INSERT INTO sessions(id,started_at_utc,ended_at_utc,status,recorded_ms,inference_ms,insertion_ms,words_count,chars_count,language_code,model_id,error_category,input_mode)
            VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)
            ON CONFLICT(id) DO UPDATE SET started_at_utc=excluded.started_at_utc,ended_at_utc=excluded.ended_at_utc,status=excluded.status,
            recorded_ms=excluded.recorded_ms,inference_ms=excluded.inference_ms,insertion_ms=excluded.insertion_ms,words_count=excluded.words_count,
            chars_count=excluded.chars_count,language_code=excluded.language_code,model_id=excluded.model_id,error_category=excluded.error_category,input_mode=excluded.input_mode",
            params![session.id, started, ended, session.status, session.recorded_ms as i64, session.inference_ms as i64, session.insertion_ms as i64,
                session.words_count as i64, session.chars_count as i64, session.language_code, session.model_id, session.error_category, session.input_mode]).map_err(err)?;
        let mut deleted_transcript = 0;
        if history_enabled {
            if let Some(text) = session.text.as_deref() {
                tx.execute("INSERT INTO transcripts(session_id,text) VALUES(?1,?2) ON CONFLICT(session_id) DO UPDATE SET text=excluded.text", params![session.id, text]).map_err(err)?;
            } else {
                deleted_transcript = tx
                    .execute("DELETE FROM transcripts WHERE session_id=?1", [&session.id])
                    .map_err(err)?;
            }
        } else {
            deleted_transcript = tx
                .execute("DELETE FROM transcripts WHERE session_id=?1", [&session.id])
                .map_err(err)?;
        }
        tx.commit().map_err(err)?;
        if deleted_transcript > 0 {
            checkpoint_after_purge(&conn)?;
        }
        Ok(())
    }

    pub fn recover_interrupted(&self) -> Result<u64, String> {
        let mut conn = self.lock()?;
        let raw: Option<String> = conn
            .query_row(
                "SELECT value FROM settings WHERE key='settings'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(err)?;
        let settings = raw
            .map(|json| serde_json::from_str::<Settings>(&json).map_err(err))
            .transpose()?
            .unwrap_or_default();
        settings.validate()?;
        let tx = conn.transaction().map_err(err)?;
        let purged = tx.execute("DELETE FROM sessions WHERE datetime(started_at_utc) < datetime('now', '-' || ?1 || ' days')", [settings.retention_days]).map_err(err)?;
        let recovered = tx.execute("UPDATE sessions SET status='failed', error_category='interrupted', ended_at_utc=COALESCE(ended_at_utc,started_at_utc) WHERE status IN ('started','preparing','recording','processing','inserting')", []).map_err(err)?;
        tx.commit().map_err(err)?;
        if purged > 0 {
            checkpoint_after_purge(&conn)?;
        }
        Ok(recovered as u64)
    }

    pub fn dashboard(&self, filter: &DashboardFilter) -> Result<Dashboard, String> {
        filter.validate()?;
        let (start, end) = range(filter)?;
        let start_date = start.format("%Y-%m-%d").to_string();
        let end_date = end.format("%Y-%m-%d").to_string();
        let conn = self.lock()?;
        let (where_sql, values) = filter_sql(filter, Some((&start_date, &end_date)));
        let sql = format!("SELECT COUNT(*), SUM(status='completed'), SUM(status IN ('failed','error')), SUM(status='cancelled'),
            COALESCE(SUM(words_count),0),COALESCE(SUM(chars_count),0),COALESCE(SUM(recorded_ms),0),COALESCE(SUM(inference_ms),0),COALESCE(SUM(insertion_ms),0),
            AVG(CASE WHEN status='completed' THEN inference_ms END), COUNT(DISTINCT date(started_at_utc,'localtime')) FROM sessions {where_sql}");
        let mut d = conn
            .query_row(&sql, params_from_iter(values.iter()), |r| {
                Ok(Dashboard {
                    started: r.get::<_, i64>(0)? as u64,
                    completed: r.get::<_, Option<i64>>(1)?.unwrap_or(0) as u64,
                    failed: r.get::<_, Option<i64>>(2)?.unwrap_or(0) as u64,
                    cancelled: r.get::<_, Option<i64>>(3)?.unwrap_or(0) as u64,
                    words: r.get::<_, i64>(4)? as u64,
                    chars: r.get::<_, i64>(5)? as u64,
                    recorded_ms: r.get::<_, i64>(6)? as u64,
                    inference_ms: r.get::<_, i64>(7)? as u64,
                    insertion_ms: r.get::<_, i64>(8)? as u64,
                    mean_inference_ms: r.get::<_, Option<f64>>(9)?.unwrap_or(0.0),
                    active_days: r.get::<_, i64>(10)? as u64,
                    ..Dashboard::default()
                })
            })
            .map_err(err)?;
        d.median_inference_ms = percentile(&conn, &where_sql, &values, 0.5)?;
        d.p95_inference_ms = percentile(&conn, &where_sql, &values, 0.95)?;
        d.wpm = if d.recorded_ms > 0 {
            d.words as f64 * 60_000.0 / d.recorded_ms as f64
        } else {
            0.0
        };
        d.throughput = if d.inference_ms > 0 {
            d.recorded_ms as f64 / d.inference_ms as f64
        } else {
            0.0
        };
        let mut previous = filter.clone();
        previous.start_date = Some(
            (start - chrono::Duration::days((end - start).num_days() + 1))
                .format("%Y-%m-%d")
                .to_string(),
        );
        previous.end_date = Some(
            (start - chrono::Duration::days(1))
                .format("%Y-%m-%d")
                .to_string(),
        );
        let (pw, pv) = filter_sql(
            &previous,
            previous
                .start_date
                .as_deref()
                .zip(previous.end_date.as_deref()),
        );
        let ps = format!("SELECT COALESCE(SUM(words_count),0) FROM sessions {pw}");
        d.previous_words = conn
            .query_row(&ps, params_from_iter(pv.iter()), |r| r.get::<_, i64>(0))
            .map_err(err)? as u64;
        d.words_change_pct = (d.previous_words != 0)
            .then(|| (d.words as f64 - d.previous_words as f64) * 100.0 / d.previous_words as f64);

        let bucket = match filter.granularity.as_str() {
            "week" => "date(date(started_at_utc,'localtime'), '-' || ((CAST(strftime('%w',started_at_utc,'localtime') AS INTEGER)+6)%7) || ' days')",
            "month" => "strftime('%Y-%m-01',started_at_utc,'localtime')",
            _ => "date(started_at_utc,'localtime')",
        };
        let q = format!("SELECT {bucket},COUNT(*),SUM(words_count),SUM(inference_ms) FROM sessions {where_sql} GROUP BY 1 ORDER BY 1");
        let mut st = conn.prepare(&q).map_err(err)?;
        d.timeline = st
            .query_map(params_from_iter(values.iter()), |r| {
                Ok(TimelinePoint {
                    date: r.get(0)?,
                    sessions: r.get::<_, i64>(1)? as u64,
                    words: r.get::<_, i64>(2)? as u64,
                    inference_ms: r.get::<_, i64>(3)? as u64,
                })
            })
            .map_err(err)?
            .collect::<Result<_, _>>()
            .map_err(err)?;
        let q = format!("SELECT (CAST(strftime('%w',started_at_utc,'localtime') AS INTEGER)+6)%7,CAST(strftime('%H',started_at_utc,'localtime') AS INTEGER),COUNT(*),SUM(words_count) FROM sessions {where_sql} GROUP BY 1,2 ORDER BY 1,2");
        let mut st = conn.prepare(&q).map_err(err)?;
        d.heatmap = st
            .query_map(params_from_iter(values.iter()), |r| {
                Ok(HeatmapPoint {
                    weekday: r.get::<_, i64>(0)? as u32,
                    hour: r.get::<_, i64>(1)? as u32,
                    sessions: r.get::<_, i64>(2)? as u64,
                    words: r.get::<_, i64>(3)? as u64,
                })
            })
            .map_err(err)?
            .collect::<Result<_, _>>()
            .map_err(err)?;
        let q = format!("SELECT CASE WHEN recorded_ms<30000 THEN '0-30s' WHEN recorded_ms<60000 THEN '30-60s' WHEN recorded_ms<180000 THEN '1-3m' ELSE '3m+' END,COUNT(*) FROM sessions {where_sql} GROUP BY 1 ORDER BY MIN(recorded_ms)");
        let mut st = conn.prepare(&q).map_err(err)?;
        d.duration_histogram = st
            .query_map(params_from_iter(values.iter()), |r| {
                Ok(HistogramPoint {
                    label: r.get(0)?,
                    count: r.get::<_, i64>(1)? as u64,
                })
            })
            .map_err(err)?
            .collect::<Result<_, _>>()
            .map_err(err)?;
        let q = format!("SELECT model_id,COUNT(*),AVG(inference_ms) FROM sessions {where_sql} AND status='completed' GROUP BY model_id ORDER BY COUNT(*) DESC");
        let mut st = conn.prepare(&q).map_err(err)?;
        d.models = st
            .query_map(params_from_iter(values.iter()), |r| {
                Ok(ModelMetric {
                    model_id: r.get(0)?,
                    sessions: r.get::<_, i64>(1)? as u64,
                    mean_inference_ms: r.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                })
            })
            .map_err(err)?
            .collect::<Result<_, _>>()
            .map_err(err)?;
        let q = format!("SELECT model_id FROM sessions {where_sql} AND model_id<>'' GROUP BY model_id ORDER BY COUNT(*) DESC LIMIT 1");
        d.top_model = conn
            .query_row(&q, params_from_iter(values.iter()), |r| r.get(0))
            .optional()
            .map_err(err)?;
        let q = format!("SELECT language_code,COUNT(*) FROM sessions {where_sql} AND language_code<>'' GROUP BY language_code ORDER BY COUNT(*) DESC LIMIT 1");
        d.top_language = conn
            .query_row(&q, params_from_iter(values.iter()), |r| r.get(0))
            .optional()
            .map_err(err)?;
        let q = format!("SELECT COALESCE(error_category,'unknown'),COUNT(*) FROM sessions {where_sql} AND status IN ('failed','error') GROUP BY error_category ORDER BY COUNT(*) DESC");
        let mut st = conn.prepare(&q).map_err(err)?;
        d.errors = st
            .query_map(params_from_iter(values.iter()), |r| {
                Ok(ErrorMetric {
                    category: r.get(0)?,
                    count: r.get::<_, i64>(1)? as u64,
                })
            })
            .map_err(err)?
            .collect::<Result<_, _>>()
            .map_err(err)?;
        Ok(d)
    }

    pub fn list_sessions(
        &self,
        filter: &DashboardFilter,
        search: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Session>, String> {
        filter.validate()?;
        if limit == 0 || limit > 500 {
            return Err("limit must be between 1 and 500".into());
        }
        if search.len() > 256 {
            return Err("search must be 256 bytes or fewer".into());
        }
        let (start, end) = range(filter)?;
        let start_date = start.format("%Y-%m-%d").to_string();
        let end_date = end.format("%Y-%m-%d").to_string();
        let (mut where_sql, mut values) = filter_sql(filter, Some((&start_date, &end_date)));
        if !search.is_empty() {
            where_sql.push_str(" AND (s.language_code LIKE ? ESCAPE '\\' OR s.model_id LIKE ? ESCAPE '\\' OR s.status LIKE ? ESCAPE '\\' OR EXISTS(SELECT 1 FROM transcripts t WHERE t.session_id=s.id AND t.text LIKE ? ESCAPE '\\'))");
            let escaped = format!(
                "%{}%",
                search
                    .replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_")
            );
            values.extend((0..4).map(|_| Value::Text(escaped.clone())));
        }
        let sql=format!("SELECT s.id,s.started_at_utc,s.ended_at_utc,s.status,s.recorded_ms,s.inference_ms,s.insertion_ms,s.words_count,s.chars_count,s.language_code,s.model_id,s.error_category,s.input_mode,t.text FROM sessions s LEFT JOIN transcripts t ON t.session_id=s.id {where_sql} ORDER BY s.started_at_utc DESC,s.id DESC LIMIT ? OFFSET ?");
        values.push(Value::Integer(limit as i64));
        values.push(Value::Integer(offset as i64));
        let conn = self.lock()?;
        let mut st = conn.prepare(&sql).map_err(err)?;
        let rows = st
            .query_map(params_from_iter(values.iter()), session_row)
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        Ok(rows)
    }

    pub fn delete_transcript(&self, id: &str) -> Result<(), String> {
        let conn = self.lock()?;
        if conn
            .execute("DELETE FROM transcripts WHERE session_id=?1", [id])
            .map_err(err)?
            > 0
        {
            checkpoint_after_purge(&conn)?;
        }
        Ok(())
    }

    pub fn clear_data(&self, kind: &str) -> Result<(), String> {
        let mut conn = self.lock()?;
        let tx = conn.transaction().map_err(err)?;
        let purged = match kind {
            "transcripts" => tx.execute("DELETE FROM transcripts", []).map_err(err)?,
            "metrics" => tx.execute("DELETE FROM sessions", []).map_err(err)?,
            "all" => {
                let deleted = tx.execute("DELETE FROM sessions", []).map_err(err)?;
                tx.execute("DELETE FROM settings", []).map_err(err)?;
                deleted
            }
            _ => return Err("kind must be transcripts, metrics, or all".into()),
        };
        tx.commit().map_err(err)?;
        if purged > 0 {
            checkpoint_after_purge(&conn)?;
        }
        Ok(())
    }

    pub fn export_data(&self, kind: &str, format: &str) -> Result<String, String> {
        if kind == "settings" {
            if format != "json" {
                return Err("Settings export supports JSON only.".into());
            }
            return serde_json::to_string_pretty(&self.settings()?).map_err(err);
        }
        if !matches!(kind, "sessions" | "transcripts") {
            return Err("kind must be sessions or transcripts".into());
        }
        if !matches!(format, "json" | "csv") {
            return Err("format must be json or csv".into());
        }
        let conn = self.lock()?;
        let sql = if kind == "transcripts" {
            "SELECT s.id,s.started_at_utc,s.ended_at_utc,s.status,s.recorded_ms,s.inference_ms,s.insertion_ms,s.words_count,s.chars_count,s.language_code,s.model_id,s.error_category,s.input_mode,t.text FROM sessions s JOIN transcripts t ON t.session_id=s.id ORDER BY s.started_at_utc"
        } else {
            "SELECT s.id,s.started_at_utc,s.ended_at_utc,s.status,s.recorded_ms,s.inference_ms,s.insertion_ms,s.words_count,s.chars_count,s.language_code,s.model_id,s.error_category,s.input_mode,t.text FROM sessions s LEFT JOIN transcripts t ON t.session_id=s.id ORDER BY s.started_at_utc"
        };
        let mut st = conn.prepare(sql).map_err(err)?;
        let rows = st
            .query_map([], session_row)
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        if format == "json" {
            return serde_json::to_string(&rows).map_err(err);
        }
        let mut out=String::from("id,started_at_utc,ended_at_utc,status,recorded_ms,inference_ms,insertion_ms,words_count,chars_count,language_code,model_id,error_category,input_mode,text\n");
        for s in rows {
            let cols = [
                s.id,
                s.started_at_utc,
                s.ended_at_utc.unwrap_or_default(),
                s.status,
                s.recorded_ms.to_string(),
                s.inference_ms.to_string(),
                s.insertion_ms.to_string(),
                s.words_count.to_string(),
                s.chars_count.to_string(),
                s.language_code,
                s.model_id,
                s.error_category.unwrap_or_default(),
                s.input_mode,
                s.text.unwrap_or_default(),
            ];
            out.push_str(&cols.iter().map(|v| csv(v)).collect::<Vec<_>>().join(","));
            out.push('\n');
        }
        Ok(out)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.conn
            .lock()
            .map_err(|_| "database lock poisoned".into())
    }
}

fn filter_sql(filter: &DashboardFilter, dates: Option<(&str, &str)>) -> (String, Vec<Value>) {
    let mut sql = String::from("WHERE 1=1");
    let mut vals = Vec::new();
    if let Some((start, end)) = dates {
        sql.push_str(
            " AND date(started_at_utc,'localtime')>=? AND date(started_at_utc,'localtime')<=?",
        );
        vals.push(Value::Text(start.into()));
        vals.push(Value::Text(end.into()));
    }
    if let Some(v) = filter.language.as_deref() {
        sql.push_str(" AND language_code=?");
        vals.push(Value::Text(v.into()));
    }
    if let Some(v) = filter.model_id.as_deref() {
        sql.push_str(" AND model_id=?");
        vals.push(Value::Text(v.into()));
    }
    (sql, vals)
}

fn range(f: &DashboardFilter) -> Result<(NaiveDate, NaiveDate), String> {
    let today = Local::now().date_naive();
    let end = f
        .end_date
        .as_deref()
        .map(|v| {
            NaiveDate::parse_from_str(v, "%Y-%m-%d").map_err(|_| "invalid end_date".to_string())
        })
        .transpose()?
        .unwrap_or(today);
    let start = f
        .start_date
        .as_deref()
        .map(|v| {
            NaiveDate::parse_from_str(v, "%Y-%m-%d").map_err(|_| "invalid start_date".to_string())
        })
        .transpose()?
        .unwrap_or(end - chrono::Duration::days(f.days as i64 - 1));
    Ok((start, end))
}

fn percentile(conn: &Connection, where_sql: &str, values: &[Value], p: f64) -> Result<f64, String> {
    let sql=format!("SELECT inference_ms FROM sessions {where_sql} AND status='completed' ORDER BY inference_ms");
    let mut st = conn.prepare(&sql).map_err(err)?;
    let xs = st
        .query_map(params_from_iter(values.iter()), |r| r.get::<_, i64>(0))
        .map_err(err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    if xs.is_empty() {
        return Ok(0.0);
    }
    if p == 0.5 {
        let upper = xs.len() / 2;
        let lower = (xs.len() - 1) / 2;
        return Ok((xs[lower] as f64 + xs[upper] as f64) / 2.0);
    }
    let index = (xs.len() as f64 * p).ceil() as usize - 1;
    Ok(xs[index.min(xs.len() - 1)] as f64)
}

fn utc_timestamp(value: &str) -> Result<String, String> {
    let parsed = chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|_| "session timestamps must be RFC 3339")?;
    Ok(parsed
        .with_timezone(&chrono::Utc)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
}

fn checkpoint_after_purge(conn: &Connection) -> Result<(), String> {
    let (busy, log_pages, checkpointed_pages): (i64, i64, i64) = conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(err)?;
    if busy != 0 || (log_pages >= 0 && log_pages != checkpointed_pages) {
        return Err(
            "SQLite could not securely purge deleted data because another reader is active".into(),
        );
    }
    Ok(())
}

fn session_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Session> {
    Ok(Session {
        id: r.get(0)?,
        started_at_utc: r.get(1)?,
        ended_at_utc: r.get(2)?,
        status: r.get(3)?,
        recorded_ms: r.get::<_, i64>(4)? as u64,
        inference_ms: r.get::<_, i64>(5)? as u64,
        insertion_ms: r.get::<_, i64>(6)? as u64,
        words_count: r.get::<_, i64>(7)? as u64,
        chars_count: r.get::<_, i64>(8)? as u64,
        language_code: r.get(9)?,
        model_id: r.get(10)?,
        error_category: r.get(11)?,
        input_mode: r.get(12)?,
        text: r.get(13)?,
    })
}
fn csv(v: &str) -> String {
    let formula_like = v.trim_start().starts_with(['=', '+', '-', '@', '\t', '\r']);
    let safe = if formula_like {
        format!("'{v}")
    } else {
        v.to_owned()
    };
    format!("\"{}\"", safe.replace('"', "\"\""))
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;
    fn session(id: &str, status: &str, date: &str, text: Option<&str>) -> Session {
        Session {
            id: id.into(),
            started_at_utc: format!("{date}T12:00:00Z"),
            ended_at_utc: Some(format!("{date}T12:00:03Z")),
            status: status.into(),
            recorded_ms: 3000,
            inference_ms: 1000,
            insertion_ms: 100,
            words_count: 10,
            chars_count: 50,
            language_code: "it".into(),
            model_id: "tiny".into(),
            error_category: if status == "failed" {
                Some("engine".into())
            } else {
                None
            },
            input_mode: "toggle".into(),
            text: text.map(str::to_owned),
        }
    }
    fn filter() -> DashboardFilter {
        DashboardFilter {
            days: 7,
            start_date: Some("2024-03-30".into()),
            end_date: Some("2024-04-01".into()),
            ..DashboardFilter::default()
        }
    }

    #[test]
    fn defaults_and_opt_in_storage_and_immediate_removal() {
        let repo = Repository::in_memory().unwrap();
        assert!(!repo.settings().unwrap().history_enabled);
        repo.save_session(
            &session("a", "completed", "2024-03-31", Some("private words")),
            false,
        )
        .unwrap();
        assert_eq!(repo.export_data("transcripts", "json").unwrap(), "[]");
        let mut s = repo.settings().unwrap();
        s.retention_days = 3650;
        s.history_enabled = true;
        repo.save_settings(&s).unwrap();
        repo.save_session(
            &session("a", "completed", "2024-03-31", Some("private, \"words\"")),
            true,
        )
        .unwrap();
        assert!(repo
            .export_data("transcripts", "json")
            .unwrap()
            .contains("private"));
        s.history_enabled = false;
        repo.save_settings(&s).unwrap();
        assert_eq!(repo.export_data("transcripts", "json").unwrap(), "[]");
        assert_eq!(repo.dashboard(&filter()).unwrap().words, 10);
    }

    #[test]
    fn aggregations_filters_and_recovery() {
        let repo = Repository::in_memory().unwrap();
        let mut settings = repo.settings().unwrap();
        settings.retention_days = 3650;
        repo.save_settings(&settings).unwrap();
        repo.save_session(&session("1", "recording", "2024-03-30", None), false)
            .unwrap();
        let mut first = session("1", "completed", "2024-03-30", None);
        first.inference_ms = 2000;
        repo.save_session(&first, false).unwrap();
        repo.save_session(&session("previous", "completed", "2024-03-29", None), false)
            .unwrap();
        repo.save_session(&session("2", "completed", "2024-03-31", None), false)
            .unwrap();
        repo.save_session(&session("3", "cancelled", "2024-03-31", None), false)
            .unwrap();
        repo.save_session(&session("4", "recording", "2024-04-01", None), false)
            .unwrap();
        let d = repo.dashboard(&filter()).unwrap();
        assert_eq!(d.started, 4);
        assert_eq!(d.completed, 2);
        assert_eq!(d.cancelled, 1);
        assert_eq!(d.words, 40);
        assert_eq!(d.median_inference_ms, 1500.0);
        assert_eq!(d.p95_inference_ms, 2000.0);
        assert_eq!(d.previous_words, 10);
        assert_eq!(d.words_change_pct, Some(300.0));
        assert_eq!(d.timeline.len(), 3);
        assert_eq!(d.heatmap.iter().map(|x| x.sessions).sum::<u64>(), 4);
        assert_eq!(repo.recover_interrupted().unwrap(), 1);
        assert_eq!(repo.dashboard(&filter()).unwrap().failed, 1);
    }

    #[test]
    fn search_escapes_wildcards_and_csv_quotes_fields() {
        let repo = Repository::in_memory().unwrap();
        let mut settings = repo.settings().unwrap();
        settings.history_enabled = true;
        repo.save_settings(&settings).unwrap();
        repo.save_session(
            &session("1", "completed", "2024-03-31", Some("100%_ \"quoted\"")),
            true,
        )
        .unwrap();
        repo.save_session(
            &session(
                "2",
                "completed",
                "2024-03-31",
                Some("=HYPERLINK(\"https://example.invalid\")"),
            ),
            true,
        )
        .unwrap();
        assert_eq!(repo.list_sessions(&filter(), "%_", 10, 0).unwrap().len(), 1);
        assert!(repo
            .list_sessions(&filter(), &"x".repeat(257), 10, 0)
            .is_err());
        let csv = repo.export_data("transcripts", "csv").unwrap();
        assert!(csv.contains("100%_ \"\"quoted\"\""));
        assert!(csv.contains("'=HYPERLINK"));
        repo.delete_transcript("2").unwrap();
        assert!(!repo
            .export_data("transcripts", "json")
            .unwrap()
            .contains("HYPERLINK"));
        assert_eq!(repo.dashboard(&filter()).unwrap().words, 20);
    }

    #[test]
    fn validation_rejects_invalid_trust_boundary_values() {
        let repo = Repository::in_memory().unwrap();
        assert_eq!(
            serde_json::from_str::<Settings>(&repo.export_data("settings", "json").unwrap())
                .unwrap(),
            Settings::default()
        );
        assert!(repo.export_data("settings", "csv").is_err());
        let mut s = repo.settings().unwrap();
        s.vad_threshold = f64::NAN;
        assert!(repo.save_settings(&s).is_err());
        assert!(repo
            .list_sessions(&DashboardFilter::default(), "", 501, 0)
            .is_err());
        s.language = "zz".into();
        assert!(repo.save_settings(&s).is_err());
    }

    #[test]
    fn retention_and_clear_modes_remove_related_rows() {
        let repo = Repository::in_memory().unwrap();
        let mut settings = repo.settings().unwrap();
        settings.history_enabled = true;
        repo.save_settings(&settings).unwrap();
        repo.save_session(
            &session("old", "completed", "2020-01-01", Some("old")),
            true,
        )
        .unwrap();
        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        repo.save_session(
            &session("recent", "completed", &today, Some("recent")),
            true,
        )
        .unwrap();
        settings.retention_days = 1;
        repo.save_settings(&settings).unwrap();
        let transcripts = repo.export_data("transcripts", "json").unwrap();
        assert!(transcripts.contains("recent"));
        assert!(!transcripts.contains("old"));
        repo.clear_data("metrics").unwrap();
        assert_eq!(repo.export_data("sessions", "json").unwrap(), "[]");
        assert_eq!(repo.export_data("transcripts", "json").unwrap(), "[]");
    }

    #[test]
    fn startup_recovery_applies_saved_retention_and_marks_only_active_sessions() {
        let repo = Repository::in_memory().unwrap();
        let mut settings = repo.settings().unwrap();
        settings.retention_days = 3650;
        repo.save_settings(&settings).unwrap();
        repo.save_session(&session("old", "completed", "2020-01-01", None), false)
            .unwrap();
        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        repo.save_session(&session("active", "recording", &today, None), false)
            .unwrap();
        settings.retention_days = 1;
        let value = serde_json::to_string(&settings).unwrap();
        repo.lock()
            .unwrap()
            .execute(
                "UPDATE settings SET value=?1 WHERE key='settings'",
                [&value],
            )
            .unwrap();
        assert_eq!(repo.recover_interrupted().unwrap(), 1);
        let count: i64 = repo
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
        let recovered = repo
            .list_sessions(
                &DashboardFilter {
                    days: 1,
                    ..DashboardFilter::default()
                },
                "",
                10,
                0,
            )
            .unwrap();
        assert_eq!(recovered[0].error_category.as_deref(), Some("interrupted"));
    }

    #[test]
    fn file_backed_transcript_purge_truncates_wal_and_secure_delete_is_enabled() {
        let path =
            std::env::temp_dir().join(format!("dicta-purge-{}.sqlite3", uuid::Uuid::new_v4()));
        let repo = Repository::open(&path).unwrap();
        let secure_delete: i64 = repo
            .lock()
            .unwrap()
            .pragma_query_value(None, "secure_delete", |row| row.get(0))
            .unwrap();
        assert_eq!(secure_delete, 1);
        let mut settings = repo.settings().unwrap();
        settings.history_enabled = true;
        repo.save_settings(&settings).unwrap();
        repo.save_session(
            &session(
                "sensitive",
                "completed",
                "2024-03-31",
                Some("sensitive transcript"),
            ),
            true,
        )
        .unwrap();
        repo.delete_transcript("sensitive").unwrap();
        let wal = std::path::PathBuf::from(format!("{}-wal", path.display()));
        assert!(std::fs::metadata(&wal)
            .map(|metadata| metadata.len() == 0)
            .unwrap_or(true));
        drop(repo);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&wal);
        let _ = std::fs::remove_file(format!("{}-shm", path.display()));
    }

    #[test]
    fn local_day_and_hour_buckets_follow_local_timezone_conversion() {
        let repo = Repository::in_memory().unwrap();
        let mut first = session("before", "completed", "2024-03-31", None);
        first.started_at_utc = "2024-03-31T00:30:00Z".into();
        let mut second = session("after", "completed", "2024-03-31", None);
        second.started_at_utc = "2024-03-31T01:30:00Z".into();
        repo.save_session(&first, false).unwrap();
        repo.save_session(&second, false).unwrap();
        let date = chrono::DateTime::parse_from_rfc3339(&first.started_at_utc)
            .unwrap()
            .with_timezone(&Local)
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        let filter = DashboardFilter {
            days: 1,
            start_date: Some(date.clone()),
            end_date: Some(date),
            ..DashboardFilter::default()
        };
        let dashboard = repo.dashboard(&filter).unwrap();
        let expected_hours = [
            first.started_at_utc.as_str(),
            second.started_at_utc.as_str(),
        ]
        .iter()
        .map(|timestamp| {
            chrono::DateTime::parse_from_rfc3339(timestamp)
                .unwrap()
                .with_timezone(&Local)
                .hour()
        })
        .collect::<Vec<_>>();
        for hour in expected_hours {
            assert!(dashboard.heatmap.iter().any(|point| point.hour == hour));
        }
        assert_eq!(
            dashboard
                .timeline
                .iter()
                .map(|point| point.sessions)
                .sum::<u64>(),
            2
        );
    }
}
