use std::path::Path;
use std::str::FromStr;

use anyhow::Context;
use anyhow::Result;
use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqlitePoolOptions;

use crate::redaction::redact_memory_text;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionSession {
    pub session_id: String,
    pub status: String,
    pub current_task: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodicTrace {
    pub trace_id: String,
    pub session_id: String,
    pub phase_name: String,
    pub action_taken: String,
    pub outcome_status: String,
    pub output_summary: String,
    pub recovery_hypothesis: Option<String>,
}

pub async fn open_episodic_db(path: impl AsRef<Path>) -> Result<SqlitePool> {
    open_db(path.as_ref()).await
}

pub async fn ensure_episodic_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS execution_sessions (
            session_id TEXT PRIMARY KEY,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            status TEXT NOT NULL,
            current_task TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS episodic_traces (
            trace_id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            phase_name TEXT NOT NULL,
            action_taken TEXT NOT NULL,
            outcome_status TEXT NOT NULL,
            output_summary TEXT NOT NULL,
            recovery_hypothesis TEXT,
            timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(session_id) REFERENCES execution_sessions(session_id)
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_episodic_outcomes
            ON episodic_traces(session_id, outcome_status)",
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn upsert_execution_session(pool: &SqlitePool, session: &ExecutionSession) -> Result<()> {
    sqlx::query(
        "INSERT INTO execution_sessions (session_id, status, current_task)
         VALUES (?, ?, ?)
         ON CONFLICT(session_id) DO UPDATE SET
            status = excluded.status,
            current_task = excluded.current_task",
    )
    .bind(&session.session_id)
    .bind(&session.status)
    .bind(redact_memory_text(session.current_task.as_str()))
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn write_episodic_trace(pool: &SqlitePool, trace: &EpisodicTrace) -> Result<()> {
    sqlx::query(
        "INSERT INTO episodic_traces
         (trace_id, session_id, phase_name, action_taken, outcome_status, output_summary, recovery_hypothesis)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&trace.trace_id)
    .bind(&trace.session_id)
    .bind(&trace.phase_name)
    .bind(redact_memory_text(trace.action_taken.as_str()))
    .bind(&trace.outcome_status)
    .bind(redact_memory_text(trace.output_summary.as_str()))
    .bind(
        trace
            .recovery_hypothesis
            .as_deref()
            .map(redact_memory_text),
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn count_repeated_failures(
    pool: &SqlitePool,
    session_id: &str,
    action_taken: &str,
) -> Result<i64> {
    let row = sqlx::query(
        "SELECT COUNT(*) AS count
         FROM episodic_traces
         WHERE session_id = ? AND action_taken = ? AND outcome_status = 'failure'",
    )
    .bind(session_id)
    .bind(redact_memory_text(action_taken))
    .fetch_one(pool)
    .await?;
    Ok(row.get::<i64, _>("count"))
}

async fn open_db(path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("create memory db parent {}", parent.display()))?;
    }
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))?
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    ensure_episodic_schema(&pool).await?;
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn episodic_db_schema_accepts_redacted_failure_trace() {
        let temp = tempfile::tempdir().unwrap();
        let pool = open_episodic_db(temp.path().join("episodic.db"))
            .await
            .unwrap();
        upsert_execution_session(
            &pool,
            &ExecutionSession {
                session_id: "session.test".to_string(),
                status: "open".to_string(),
                current_task: "fix api_key=sk-abcdefghijklmnopqrstuvwxyz".to_string(),
            },
        )
        .await
        .unwrap();
        write_episodic_trace(
            &pool,
            &EpisodicTrace {
                trace_id: "trace.test".to_string(),
                session_id: "session.test".to_string(),
                phase_name: "validation".to_string(),
                action_taken: "cargo test".to_string(),
                outcome_status: "failure".to_string(),
                output_summary: "token=secretvalue12345".to_string(),
                recovery_hypothesis: Some("try a narrower fix".to_string()),
            },
        )
        .await
        .unwrap();

        let count = count_repeated_failures(&pool, "session.test", "cargo test")
            .await
            .unwrap();
        assert_eq!(count, 1);
    }
}
