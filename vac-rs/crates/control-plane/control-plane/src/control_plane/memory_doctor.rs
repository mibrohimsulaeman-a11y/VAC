use std::fs;
use std::future::Future;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqlitePoolOptions;
use tokio::runtime::Builder;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryDoctorReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub infos: Vec<String>,
}

impl MemoryDoctorReport {
    pub fn exit_code(&self) -> i32 {
        if self.errors.is_empty() { 0 } else { 1 }
    }

    pub fn render_text(&self) -> String {
        let status = if !self.errors.is_empty() {
            "FAIL"
        } else if !self.warnings.is_empty() {
            "WARN"
        } else {
            "PASS"
        };
        let mut lines = vec![format!("vac doctor memory: {status}")];
        for info in &self.infos {
            lines.push(format!("  INFO: {info}"));
        }
        for warning in &self.warnings {
            lines.push(format!("  WARN: {warning}"));
        }
        for error in &self.errors {
            lines.push(format!("  ERROR: {error}"));
        }
        lines.join("\n")
    }
}

pub fn load_memory_doctor_report(root: impl AsRef<Path>) -> MemoryDoctorReport {
    let root = root.as_ref();
    let mut report = MemoryDoctorReport::default();
    check_manifest(
        &mut report,
        root.join(".vac/capabilities/memory-read.yaml"),
        "memory-read capability",
    );
    check_manifest(
        &mut report,
        root.join(".vac/capabilities/memory-write.yaml"),
        "memory-write capability",
    );
    check_manifest(
        &mut report,
        root.join(".vac/workflows/maintenance.memory-consolidation.yaml"),
        "memory consolidation workflow",
    );

    check_sqlite_db(
        &mut report,
        root.join(".vac/memories/episodic.db"),
        &["execution_sessions", "episodic_traces"],
        "episodic",
    );
    check_semantic_db(&mut report, root.join(".vac/memories/semantic.db"));
    report
}

fn check_manifest(report: &mut MemoryDoctorReport, path: PathBuf, label: &str) {
    match fs::read_to_string(&path) {
        Ok(source) if source.contains("schema_version: 1") => {
            report.infos.push(format!("{label}: {}", path.display()));
        }
        Ok(_) => report.errors.push(format!(
            "{label} missing schema envelope at {}",
            path.display()
        )),
        Err(_) => report
            .warnings
            .push(format!("{label} missing at {}", path.display())),
    }
}

fn check_sqlite_db(report: &mut MemoryDoctorReport, path: PathBuf, tables: &[&str], label: &str) {
    if !path.exists() {
        report.warnings.push(format!(
            "{label} db missing at {}; runtime creates it on first memory write",
            path.display()
        ));
        return;
    }
    let check_path = path.clone();
    let table_names = tables
        .iter()
        .map(|table| (*table).to_string())
        .collect::<Vec<_>>();
    match run_sqlite(path.clone(), async move {
        let path = check_path;
        let pool = open_pool(&path).await?;
        for table in table_names {
            if !table_exists(&pool, &table).await? {
                return Err(format!("missing table {table}"));
            }
        }
        Ok(())
    }) {
        Ok(()) => report.infos.push(format!("{label} db schema: ready")),
        Err(err) => report.errors.push(format!("{}: {err}", path.display())),
    }
}

/// FTS5 query latency budget surfaced by `vac doctor memory` (Spec v1.4 §VIII.7 / §29).
const FTS_LATENCY_BUDGET_MS: u128 = 15;

fn check_semantic_db(report: &mut MemoryDoctorReport, path: PathBuf) {
    if !path.exists() {
        report.warnings.push(format!(
            "semantic db missing at {}; runtime creates it on first memory write",
            path.display()
        ));
        return;
    }
    let check_path = path.clone();
    match run_sqlite(path.clone(), async move {
        let pool = open_pool(&check_path).await?;
        for table in ["project_facts", "project_facts_fts"] {
            if !table_exists(&pool, table).await? {
                return Err(format!("missing table {table}"));
            }
        }
        for trigger in ["facts_ai", "facts_au", "facts_ad"] {
            if !trigger_exists(&pool, trigger).await? {
                return Err(format!("missing FTS maintenance trigger {trigger}"));
            }
        }
        let fact_count = sqlx::query("SELECT COUNT(*) AS count FROM project_facts")
            .fetch_one(&pool)
            .await
            .map_err(|err| format!("project_facts count failed: {err}"))?
            .get::<i64, _>("count");
        let started = std::time::Instant::now();
        sqlx::query("SELECT rowid FROM project_facts_fts WHERE project_facts_fts MATCH ? LIMIT 5")
            .bind("vac OR memory OR fact")
            .fetch_all(&pool)
            .await
            .map_err(|err| format!("project_facts_fts MATCH query failed: {err}"))?;
        let latency_ms = started.elapsed().as_millis();
        Ok((fact_count, latency_ms))
    }) {
        Ok((fact_count, latency_ms)) => {
            report.infos.push("semantic db schema: ready".to_string());
            report.infos.push(format!(
                "semantic FTS latency: {latency_ms}ms over {fact_count} fact(s) (budget {FTS_LATENCY_BUDGET_MS}ms)"
            ));
            if latency_ms > FTS_LATENCY_BUDGET_MS {
                report.warnings.push(format!(
                    "semantic FTS latency {latency_ms}ms exceeds {FTS_LATENCY_BUDGET_MS}ms budget over {fact_count} fact(s)"
                ));
            }
        }
        Err(err) => report.errors.push(format!("{}: {err}", path.display())),
    }
}

fn run_sqlite<T, F>(path: PathBuf, future: F) -> Result<T, String>
where
    T: Send + 'static,
    F: Future<Output = Result<T, String>> + Send + 'static,
{
    let path_for_error = path.display().to_string();
    std::thread::spawn(move || {
        Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| {
                format!(
                    "failed to create sqlite runtime for {}: {err}",
                    path.display()
                )
            })?
            .block_on(future)
    })
    .join()
    .map_err(|_| format!("sqlite runtime thread panicked for {path_for_error}"))?
}

async fn open_pool(path: &Path) -> Result<SqlitePool, String> {
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))
        .map_err(|err| err.to_string())?
        .create_if_missing(false);
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|err| err.to_string())
}

async fn table_exists(pool: &SqlitePool, table: &str) -> Result<bool, String> {
    let row = sqlx::query(
        "SELECT COUNT(*) AS count FROM sqlite_master WHERE type IN ('table', 'view') AND name = ?",
    )
    .bind(table)
    .fetch_one(pool)
    .await
    .map_err(|err| err.to_string())?;
    Ok(row.get::<i64, _>("count") > 0)
}

async fn trigger_exists(pool: &SqlitePool, trigger: &str) -> Result<bool, String> {
    let row = sqlx::query(
        "SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'trigger' AND name = ?",
    )
    .bind(trigger)
    .fetch_one(pool)
    .await
    .map_err(|err| err.to_string())?;
    Ok(row.get::<i64, _>("count") > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_doctor_warns_until_runtime_dbs_exist() {
        let temp = tempfile::tempdir().unwrap();
        let vac = temp.path().join(".vac");
        std::fs::create_dir_all(vac.join("capabilities")).unwrap();
        std::fs::create_dir_all(vac.join("workflows")).unwrap();
        std::fs::write(
            vac.join("capabilities/memory-read.yaml"),
            "schema_version: 1\nkind: capability\nid: capability.memory.read\n",
        )
        .unwrap();
        std::fs::write(
            vac.join("capabilities/memory-write.yaml"),
            "schema_version: 1\nkind: capability\nid: capability.memory.write\n",
        )
        .unwrap();
        std::fs::write(
            vac.join("workflows/maintenance.memory-consolidation.yaml"),
            "schema_version: 1\nkind: workflow\nid: maintenance.memory-consolidation\n",
        )
        .unwrap();

        let report = load_memory_doctor_report(temp.path());
        assert_eq!(report.exit_code(), 0, "{}", report.render_text());
        assert!(report.render_text().contains("episodic db missing"));
    }

    #[test]
    fn memory_doctor_checks_semantic_fts_maintenance_triggers() {
        let temp = tempfile::tempdir().unwrap();
        let vac = temp.path().join(".vac");
        std::fs::create_dir_all(vac.join("capabilities")).unwrap();
        std::fs::create_dir_all(vac.join("workflows")).unwrap();
        std::fs::create_dir_all(vac.join("memories")).unwrap();
        std::fs::write(
            vac.join("capabilities/memory-read.yaml"),
            "schema_version: 1\nkind: capability\nid: capability.memory.read\n",
        )
        .unwrap();
        std::fs::write(
            vac.join("capabilities/memory-write.yaml"),
            "schema_version: 1\nkind: capability\nid: capability.memory.write\n",
        )
        .unwrap();
        std::fs::write(
            vac.join("workflows/maintenance.memory-consolidation.yaml"),
            "schema_version: 1\nkind: workflow\nid: maintenance.memory-consolidation\n",
        )
        .unwrap();
        let semantic_db = vac.join("memories/semantic.db");
        run_sqlite(semantic_db.clone(), async move {
            let options =
                SqliteConnectOptions::from_str(&format!("sqlite://{}", semantic_db.display()))
                    .map_err(|err| err.to_string())?
                    .create_if_missing(true);
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(options)
                .await
                .map_err(|err| err.to_string())?;
            sqlx::query(
                "CREATE TABLE project_facts (
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
            .execute(&pool)
            .await
            .map_err(|err| err.to_string())?;
            sqlx::query(
                "CREATE VIRTUAL TABLE project_facts_fts USING fts5(
                    subject,
                    fact_summary,
                    associated_tags,
                    content='project_facts',
                    content_rowid='rowid'
                )",
            )
            .execute(&pool)
            .await
            .map_err(|err| err.to_string())?;
            sqlx::query(
                "CREATE TRIGGER facts_ai AFTER INSERT ON project_facts BEGIN
                    INSERT INTO project_facts_fts(rowid, subject, fact_summary, associated_tags)
                    VALUES (new.rowid, new.subject, new.fact_summary, new.associated_tags);
                END",
            )
            .execute(&pool)
            .await
            .map_err(|err| err.to_string())?;
            Ok(())
        })
        .unwrap();

        let report = load_memory_doctor_report(temp.path());
        assert_eq!(report.exit_code(), 1, "{}", report.render_text());
        assert!(
            report
                .render_text()
                .contains("missing FTS maintenance trigger facts_au")
        );
    }

    #[test]
    fn memory_doctor_reports_semantic_fts_latency() {
        let temp = tempfile::tempdir().unwrap();
        let vac = temp.path().join(".vac");
        std::fs::create_dir_all(vac.join("capabilities")).unwrap();
        std::fs::create_dir_all(vac.join("workflows")).unwrap();
        std::fs::create_dir_all(vac.join("memories")).unwrap();
        std::fs::write(
            vac.join("capabilities/memory-read.yaml"),
            "schema_version: 1\nkind: capability\nid: capability.memory.read\n",
        )
        .unwrap();
        std::fs::write(
            vac.join("capabilities/memory-write.yaml"),
            "schema_version: 1\nkind: capability\nid: capability.memory.write\n",
        )
        .unwrap();
        std::fs::write(
            vac.join("workflows/maintenance.memory-consolidation.yaml"),
            "schema_version: 1\nkind: workflow\nid: maintenance.memory-consolidation\n",
        )
        .unwrap();
        let semantic_db = vac.join("memories/semantic.db");
        run_sqlite(semantic_db.clone(), async move {
            let options =
                SqliteConnectOptions::from_str(&format!("sqlite://{}", semantic_db.display()))
                    .map_err(|err| err.to_string())?
                    .create_if_missing(true);
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(options)
                .await
                .map_err(|err| err.to_string())?;
            sqlx::query(
                "CREATE TABLE project_facts (
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
            .execute(&pool)
            .await
            .map_err(|err| err.to_string())?;
            sqlx::query(
                "CREATE VIRTUAL TABLE project_facts_fts USING fts5(
                    subject,
                    fact_summary,
                    associated_tags,
                    content='project_facts',
                    content_rowid='rowid'
                )",
            )
            .execute(&pool)
            .await
            .map_err(|err| err.to_string())?;
            sqlx::query(
                "CREATE TRIGGER facts_ai AFTER INSERT ON project_facts BEGIN
                    INSERT INTO project_facts_fts(rowid, subject, fact_summary, associated_tags)
                    VALUES (new.rowid, new.subject, new.fact_summary, new.associated_tags);
                END",
            )
            .execute(&pool)
            .await
            .map_err(|err| err.to_string())?;
            sqlx::query(
                "CREATE TRIGGER facts_ad AFTER DELETE ON project_facts BEGIN
                    INSERT INTO project_facts_fts(project_facts_fts, rowid, subject, fact_summary, associated_tags)
                    VALUES('delete', old.rowid, old.subject, old.fact_summary, old.associated_tags);
                END",
            )
            .execute(&pool)
            .await
            .map_err(|err| err.to_string())?;
            sqlx::query(
                "CREATE TRIGGER facts_au AFTER UPDATE ON project_facts BEGIN
                    INSERT INTO project_facts_fts(project_facts_fts, rowid, subject, fact_summary, associated_tags)
                    VALUES('delete', old.rowid, old.subject, old.fact_summary, old.associated_tags);
                    INSERT INTO project_facts_fts(rowid, subject, fact_summary, associated_tags)
                    VALUES (new.rowid, new.subject, new.fact_summary, new.associated_tags);
                END",
            )
            .execute(&pool)
            .await
            .map_err(|err| err.to_string())?;
            for index in 0..5 {
                sqlx::query(
                    "INSERT INTO project_facts (fact_id, category, subject, fact_summary, source_attribution, associated_tags, confidence_score) VALUES (?, 'concept', ?, 'memory fact summary', 'scan', 'vac,memory', 0.9)",
                )
                .bind(format!("mem.semantic.{index}"))
                .bind(format!("subject vac memory {index}"))
                .execute(&pool)
                .await
                .map_err(|err| err.to_string())?;
            }
            Ok(())
        })
        .unwrap();

        let report = load_memory_doctor_report(temp.path());
        assert_eq!(report.exit_code(), 0, "{}", report.render_text());
        assert!(
            report.render_text().contains("semantic FTS latency"),
            "{}",
            report.render_text()
        );
    }
}
