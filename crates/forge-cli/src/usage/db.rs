//! SQLite database for usage tracking.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Usage database for tracking API requests.
pub struct UsageDb {
    conn: Connection,
}

impl UsageDb {
    /// Open or create the usage database at the given path.
    pub fn open(path: impl AsRef<Path>) -> SqliteResult<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Open an in-memory database for testing.
    #[allow(dead_code)]
    pub fn open_memory() -> SqliteResult<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> SqliteResult<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                tool TEXT NOT NULL DEFAULT 'unknown'
            );

            CREATE TABLE IF NOT EXISTS requests (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                model TEXT NOT NULL,
                endpoint TEXT NOT NULL,
                prompt_tokens INTEGER NOT NULL DEFAULT 0,
                completion_tokens INTEGER NOT NULL DEFAULT 0,
                total_tokens INTEGER NOT NULL DEFAULT 0,
                latency_ms INTEGER,
                status_code INTEGER,
                error TEXT,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );

            CREATE TABLE IF NOT EXISTS pricing (
                model TEXT PRIMARY KEY,
                input_cost_per_1k REAL NOT NULL,
                output_cost_per_1k REAL NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_requests_session ON requests(session_id);
            CREATE INDEX IF NOT EXISTS idx_requests_timestamp ON requests(timestamp);
            CREATE INDEX IF NOT EXISTS idx_requests_model ON requests(model);
            "#,
        )
    }

    /// Start a new session.
    pub fn start_session(&self, session_id: &str, tool: &str) -> SqliteResult<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO sessions (id, started_at, tool) VALUES (?1, ?2, ?3)",
            params![session_id, now, tool],
        )?;
        Ok(())
    }

    /// End a session.
    pub fn end_session(&self, session_id: &str) -> SqliteResult<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE sessions SET ended_at = ?1 WHERE id = ?2",
            params![now, session_id],
        )?;
        Ok(())
    }

    /// Record a request.
    pub fn record_request(&self, request: &RequestRecord) -> SqliteResult<i64> {
        self.conn.execute(
            r#"
            INSERT INTO requests (
                session_id, timestamp, model, endpoint,
                prompt_tokens, completion_tokens, total_tokens,
                latency_ms, status_code, error
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                request.session_id,
                request.timestamp.to_rfc3339(),
                request.model,
                request.endpoint,
                request.prompt_tokens,
                request.completion_tokens,
                request.total_tokens,
                request.latency_ms,
                request.status_code,
                request.error,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Set pricing for a model.
    pub fn set_pricing(
        &self,
        model: &str,
        input_cost_per_1k: f64,
        output_cost_per_1k: f64,
    ) -> SqliteResult<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO pricing (model, input_cost_per_1k, output_cost_per_1k, updated_at)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![model, input_cost_per_1k, output_cost_per_1k, now],
        )?;
        Ok(())
    }

    /// Get usage summary for a time period.
    pub fn get_usage_summary(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> SqliteResult<UsageSummary> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                COUNT(*) as request_count,
                SUM(prompt_tokens) as total_prompt_tokens,
                SUM(completion_tokens) as total_completion_tokens,
                SUM(total_tokens) as total_tokens,
                AVG(latency_ms) as avg_latency_ms,
                COUNT(DISTINCT session_id) as session_count
            FROM requests
            WHERE timestamp >= ?1 AND timestamp < ?2
            "#,
        )?;

        let summary = stmt.query_row(params![start.to_rfc3339(), end.to_rfc3339()], |row| {
            Ok(UsageSummary {
                request_count: row.get(0)?,
                total_prompt_tokens: row.get::<_, Option<i64>>(1)?.unwrap_or(0) as u64,
                total_completion_tokens: row.get::<_, Option<i64>>(2)?.unwrap_or(0) as u64,
                total_tokens: row.get::<_, Option<i64>>(3)?.unwrap_or(0) as u64,
                avg_latency_ms: row.get::<_, Option<f64>>(4)?,
                session_count: row.get(5)?,
                start,
                end,
                estimated_cost: None,
            })
        })?;

        Ok(summary)
    }

    /// Get usage breakdown by model.
    pub fn get_usage_by_model(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> SqliteResult<Vec<ModelUsage>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                model,
                COUNT(*) as request_count,
                SUM(prompt_tokens) as total_prompt_tokens,
                SUM(completion_tokens) as total_completion_tokens,
                SUM(total_tokens) as total_tokens
            FROM requests
            WHERE timestamp >= ?1 AND timestamp < ?2
            GROUP BY model
            ORDER BY total_tokens DESC
            "#,
        )?;

        let rows = stmt.query_map(params![start.to_rfc3339(), end.to_rfc3339()], |row| {
            Ok(ModelUsage {
                model: row.get(0)?,
                request_count: row.get(1)?,
                prompt_tokens: row.get::<_, Option<i64>>(2)?.unwrap_or(0) as u64,
                completion_tokens: row.get::<_, Option<i64>>(3)?.unwrap_or(0) as u64,
                total_tokens: row.get::<_, Option<i64>>(4)?.unwrap_or(0) as u64,
                estimated_cost: None,
            })
        })?;

        rows.collect()
    }

    /// Get list of sessions in a time period.
    pub fn get_sessions(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> SqliteResult<Vec<SessionRecord>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                s.id,
                s.started_at,
                s.ended_at,
                s.tool,
                COUNT(r.id) as request_count,
                COALESCE(SUM(r.total_tokens), 0) as total_tokens
            FROM sessions s
            LEFT JOIN requests r ON s.id = r.session_id
            WHERE s.started_at >= ?1 AND s.started_at < ?2
            GROUP BY s.id
            ORDER BY s.started_at DESC
            "#,
        )?;

        let rows = stmt.query_map(params![start.to_rfc3339(), end.to_rfc3339()], |row| {
            let started: String = row.get(1)?;
            let ended: Option<String> = row.get(2)?;

            Ok(SessionRecord {
                id: row.get(0)?,
                started_at: DateTime::parse_from_rfc3339(&started)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                ended_at: ended.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .map(|dt| dt.with_timezone(&Utc))
                        .ok()
                }),
                tool: row.get(3)?,
                request_count: row.get(4)?,
                total_tokens: row.get::<_, i64>(5)? as u64,
            })
        })?;

        rows.collect()
    }
}

/// Record for a single API request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestRecord {
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub model: String,
    pub endpoint: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub latency_ms: Option<u32>,
    pub status_code: Option<u32>,
    pub error: Option<String>,
}

/// Summary of usage over a time period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummary {
    pub request_count: u64,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub total_tokens: u64,
    pub avg_latency_ms: Option<f64>,
    pub session_count: u64,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub estimated_cost: Option<f64>,
}

/// Usage breakdown by model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    pub model: String,
    pub request_count: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub estimated_cost: Option<f64>,
}

/// Session record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub tool: String,
    pub request_count: u64,
    pub total_tokens: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_db() {
        let db = UsageDb::open_memory().unwrap();
        db.start_session("test-session", "forge-cli").unwrap();
    }

    #[test]
    fn test_record_request() {
        let db = UsageDb::open_memory().unwrap();
        db.start_session("test-session", "forge-cli").unwrap();

        let request = RequestRecord {
            session_id: "test-session".to_string(),
            timestamp: Utc::now(),
            model: "gpt-4o".to_string(),
            endpoint: "/chat/completions".to_string(),
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            latency_ms: Some(1500),
            status_code: Some(200),
            error: None,
        };

        let id = db.record_request(&request).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn test_usage_summary() {
        let db = UsageDb::open_memory().unwrap();
        db.start_session("test-session", "forge-cli").unwrap();

        let request = RequestRecord {
            session_id: "test-session".to_string(),
            timestamp: Utc::now(),
            model: "gpt-4o".to_string(),
            endpoint: "/chat/completions".to_string(),
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            latency_ms: Some(1500),
            status_code: Some(200),
            error: None,
        };

        db.record_request(&request).unwrap();

        let start = Utc::now() - chrono::Duration::hours(1);
        let end = Utc::now() + chrono::Duration::hours(1);
        let summary = db.get_usage_summary(start, end).unwrap();

        assert_eq!(summary.request_count, 1);
        assert_eq!(summary.total_tokens, 150);
    }
}
