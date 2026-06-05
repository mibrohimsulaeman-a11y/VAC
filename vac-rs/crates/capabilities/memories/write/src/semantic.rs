use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqlitePoolOptions;

use crate::redaction::redact_memory_text;

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectFact {
    pub fact_id: String,
    pub category: String,
    pub subject: String,
    pub fact_summary: String,
    pub source_attribution: String,
    pub associated_tags: String,
    pub confidence_score: f64,
}

pub async fn open_semantic_db(path: impl AsRef<Path>) -> Result<SqlitePool> {
    if let Some(parent) = path.as_ref().parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("create memory db parent {}", parent.display()))?;
    }
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.as_ref().display()))?
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    ensure_semantic_schema(&pool).await?;
    Ok(pool)
}

pub async fn ensure_semantic_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS memory_schema_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS project_facts (
            fact_id TEXT PRIMARY KEY,
            category TEXT NOT NULL,
            subject TEXT NOT NULL,
            fact_summary TEXT NOT NULL,
            source_attribution TEXT NOT NULL,
            associated_tags TEXT NOT NULL,
            confidence_score REAL NOT NULL,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE VIRTUAL TABLE IF NOT EXISTS project_facts_fts USING fts5(
            subject,
            fact_summary,
            associated_tags,
            content='project_facts',
            content_rowid='rowid'
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE TRIGGER IF NOT EXISTS facts_ai AFTER INSERT ON project_facts BEGIN
            INSERT INTO project_facts_fts(rowid, subject, fact_summary, associated_tags)
            VALUES (new.rowid, new.subject, new.fact_summary, new.associated_tags);
        END",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE TRIGGER IF NOT EXISTS facts_ad AFTER DELETE ON project_facts BEGIN
            INSERT INTO project_facts_fts(project_facts_fts, rowid, subject, fact_summary, associated_tags)
            VALUES ('delete', old.rowid, old.subject, old.fact_summary, old.associated_tags);
        END",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE TRIGGER IF NOT EXISTS facts_au AFTER UPDATE ON project_facts BEGIN
            INSERT INTO project_facts_fts(project_facts_fts, rowid, subject, fact_summary, associated_tags)
            VALUES ('delete', old.rowid, old.subject, old.fact_summary, old.associated_tags);
            INSERT INTO project_facts_fts(rowid, subject, fact_summary, associated_tags)
            VALUES (new.rowid, new.subject, new.fact_summary, new.associated_tags);
        END",
    )
    .execute(pool)
    .await?;
    rebuild_fts_if_needed(pool).await?;
    Ok(())
}

pub async fn write_project_fact(pool: &SqlitePool, fact: &ProjectFact) -> Result<()> {
    let fact = redacted_project_fact(fact);
    sqlx::query(
        "INSERT INTO project_facts
         (fact_id, category, subject, fact_summary, source_attribution, associated_tags, confidence_score)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(fact_id) DO UPDATE SET
            category = excluded.category,
            subject = excluded.subject,
            fact_summary = excluded.fact_summary,
            source_attribution = excluded.source_attribution,
            associated_tags = excluded.associated_tags,
            confidence_score = excluded.confidence_score,
            updated_at = CURRENT_TIMESTAMP",
    )
    .bind(&fact.fact_id)
    .bind(&fact.category)
    .bind(&fact.subject)
    .bind(&fact.fact_summary)
    .bind(&fact.source_attribution)
    .bind(&fact.associated_tags)
    .bind(fact.confidence_score)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn write_project_facts_bulk(pool: &SqlitePool, facts: &[ProjectFact]) -> Result<()> {
    if facts.is_empty() {
        return Ok(());
    }
    let mut tx = pool.begin().await?;
    for fact in facts {
        let fact = redacted_project_fact(fact);
        sqlx::query(
            "INSERT INTO project_facts
             (fact_id, category, subject, fact_summary, source_attribution, associated_tags, confidence_score)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(fact_id) DO UPDATE SET
                category = excluded.category,
                subject = excluded.subject,
                fact_summary = excluded.fact_summary,
                source_attribution = excluded.source_attribution,
                associated_tags = excluded.associated_tags,
                confidence_score = excluded.confidence_score,
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(&fact.fact_id)
        .bind(&fact.category)
        .bind(&fact.subject)
        .bind(&fact.fact_summary)
        .bind(&fact.source_attribution)
        .bind(&fact.associated_tags)
        .bind(fact.confidence_score)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn search_project_facts(
    pool: &SqlitePool,
    query: &str,
    limit: i64,
) -> Result<Vec<String>> {
    let rows = sqlx::query(
        "SELECT project_facts.fact_summary AS fact_summary
         FROM project_facts_fts
         JOIN project_facts ON project_facts_fts.rowid = project_facts.rowid
         WHERE project_facts_fts MATCH ?
         LIMIT ?",
    )
    .bind(query)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| row.get::<String, _>("fact_summary"))
        .collect())
}

pub async fn measure_fts_latency(pool: &SqlitePool, query: &str) -> Result<Duration> {
    let started = Instant::now();
    let _ = search_project_facts(pool, query, 10).await?;
    Ok(started.elapsed())
}

async fn rebuild_fts_if_needed(pool: &SqlitePool) -> Result<()> {
    let row = sqlx::query(
        "SELECT value FROM memory_schema_meta WHERE key = 'semantic_fts_trigger_version'",
    )
    .fetch_optional(pool)
    .await?;
    let version = row.map(|row| row.get::<String, _>("value"));
    if version.as_deref() == Some("2") {
        return Ok(());
    }
    sqlx::query("INSERT INTO project_facts_fts(project_facts_fts) VALUES ('rebuild')")
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT INTO memory_schema_meta (key, value)
         VALUES ('semantic_fts_trigger_version', '2')
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .execute(pool)
    .await?;
    Ok(())
}

fn redacted_project_fact(fact: &ProjectFact) -> ProjectFact {
    ProjectFact {
        fact_id: fact.fact_id.clone(),
        category: redact_memory_text(fact.category.as_str()),
        subject: redact_memory_text(fact.subject.as_str()),
        fact_summary: redact_memory_text(fact.fact_summary.as_str()),
        source_attribution: redact_memory_text(fact.source_attribution.as_str()),
        associated_tags: redact_memory_text(fact.associated_tags.as_str()),
        confidence_score: fact.confidence_score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn semantic_db_schema_supports_fts_project_facts() {
        let temp = tempfile::tempdir().unwrap();
        let pool = open_semantic_db(temp.path().join("semantic.db"))
            .await
            .unwrap();
        write_project_fact(
            &pool,
            &ProjectFact {
                fact_id: "fact.test".to_string(),
                category: "architecture".to_string(),
                subject: "VAC memory".to_string(),
                fact_summary: "VAC uses project_facts_fts for semantic lookup".to_string(),
                source_attribution: "test".to_string(),
                associated_tags: "memory,fts5".to_string(),
                confidence_score: 0.95,
            },
        )
        .await
        .unwrap();

        let rows = search_project_facts(&pool, "semantic", 5).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].contains("semantic lookup"));
    }

    #[tokio::test]
    async fn semantic_fts_tracks_upserted_fact_updates() {
        let temp = tempfile::tempdir().unwrap();
        let pool = open_semantic_db(temp.path().join("semantic.db"))
            .await
            .unwrap();
        let fact = ProjectFact {
            fact_id: "fact.update".to_string(),
            category: "architecture".to_string(),
            subject: "VAC memory".to_string(),
            fact_summary: "stale_marker semantic memory fact".to_string(),
            source_attribution: "test".to_string(),
            associated_tags: "memory,fts5".to_string(),
            confidence_score: 0.95,
        };
        write_project_fact(&pool, &fact).await.unwrap();

        let mut updated = fact;
        updated.fact_summary = "fresh_marker semantic memory fact".to_string();
        write_project_fact(&pool, &updated).await.unwrap();

        assert_eq!(
            search_project_facts(&pool, "stale_marker", 5)
                .await
                .unwrap()
                .len(),
            0
        );
        let rows = search_project_facts(&pool, "fresh_marker", 5)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].contains("fresh_marker"));
    }

    #[tokio::test]
    async fn semantic_fts_10000_facts_latency_under_15ms() {
        let temp = tempfile::tempdir().unwrap();
        let pool = open_semantic_db(temp.path().join("semantic.db"))
            .await
            .unwrap();
        let facts = (0..10_000)
            .map(|index| ProjectFact {
                fact_id: format!("fact.perf.{index:05}"),
                category: "perf".to_string(),
                subject: format!("VAC memory perf subject {index:05}"),
                fact_summary: format!("needle10k semantic fact summary {index:05}"),
                source_attribution: "semantic_fts_10000_facts_latency_under_15ms".to_string(),
                associated_tags: "memory,fts5,perf".to_string(),
                confidence_score: 0.9,
            })
            .collect::<Vec<_>>();
        write_project_facts_bulk(&pool, &facts).await.unwrap();

        let _ = search_project_facts(&pool, "needle10k", 10).await.unwrap();
        let mut best = Duration::MAX;
        for _ in 0..5 {
            best = best.min(measure_fts_latency(&pool, "needle10k").await.unwrap());
        }
        assert!(
            best <= Duration::from_millis(15),
            "expected 10k FTS query <= 15ms, best observed {best:?}"
        );
    }
}
