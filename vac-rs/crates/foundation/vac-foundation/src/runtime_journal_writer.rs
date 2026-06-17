//! Live SQLite writer for the VAC v1.9 runtime journal.
//!
//! This module is deliberately gated behind the `sqlite` feature.  The
//! schema/contracts live in `vac-state`; this layer turns those contracts into
//! a real local SQLite writer with WAL, `BEGIN IMMEDIATE`, writer lease checks,
//! and transaction-scoped event sequence allocation.

use crate::sqlite::{apply_connection_pragmas, apply_database_pragmas};
use libsql::{Connection, Database};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use vac_state::{
    RuntimeJournalEventDraft, RuntimeJournalOpenRequest, RuntimeManifestBinding, RuntimeTrustClaim,
    evaluate_runtime_event_append,
};

const RUNTIME_DB_MIGRATION_SQL: &str =
    include_str!("../../../../../.vac/migrations/runtime-db/0001_runtime_journal.sql");

#[derive(Debug, thiserror::Error)]
pub enum RuntimeJournalWriterError {
    #[error("invalid runtime journal open request: {0}")]
    InvalidOpenRequest(String),

    #[error("runtime journal path error: {0}")]
    Path(String),

    #[error("runtime journal database error: {0}")]
    Database(String),

    #[error("runtime journal pragma error: {0}")]
    Pragma(String),

    #[error("runtime journal writer lease required: {0}")]
    LeaseRequired(String),

    #[error("runtime journal append rejected: {0}")]
    AppendRejected(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeJournalSessionDraft {
    pub status: String,
    pub user_prompt_summary: String,
    pub current_phase: String,
    pub default_trust_claim: RuntimeTrustClaim,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeJournalValidationSummaryDraft {
    pub validation_id: String,
    pub command_id: String,
    pub structured_command_hash: String,
    pub exit_code: Option<i64>,
    pub stdout_hash: Option<String>,
    pub stderr_hash: Option<String>,
    pub duration_ms: Option<i64>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeJournalDecisionDraft {
    pub decision_id: String,
    pub decision_class: String,
    pub decision_type: String,
    pub subject_type: String,
    pub subject_id: String,
    pub decided_by: String,
    pub decision: String,
    pub reason_summary: String,
    pub scope_hash: String,
    pub policy_snapshot_hash: Option<String>,
    pub locked: bool,
    pub proof_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeJournalEvidenceHintDraft {
    pub evidence_id: String,
    pub capability_id: String,
    pub evidence_class: String,
    pub content_hash: String,
    pub previous_hash: Option<String>,
    pub trust_claim: RuntimeTrustClaim,
    pub proof_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeJournalProductProjectionCounts {
    pub sessions: i64,
    pub events: i64,
    pub validation_summaries: i64,
    pub decisions: i64,
    pub evidence_hints: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeJournalLeaseResult {
    pub acquired: bool,
    pub holder_id: String,
    pub heartbeat_counter: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeJournalAppendResult {
    pub event_id: String,
    pub seq: i64,
    pub content_hash: String,
}

#[derive(Debug)]
pub struct RuntimeJournalWriter {
    db: Database,
    workspace_id: String,
    writer_id: String,
    session_id: String,
    lease_reason: String,
    manifest_binding: RuntimeManifestBinding,
}

impl RuntimeJournalWriter {
    pub async fn open(
        request: RuntimeJournalOpenRequest,
    ) -> Result<Self, RuntimeJournalWriterError> {
        validate_open_request(&request)?;
        let db_path = PathBuf::from(&request.db_path);
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                RuntimeJournalWriterError::Path(format!(
                    "failed to create runtime DB directory {}: {err}",
                    parent.display()
                ))
            })?;
        }

        let db = libsql::Builder::new_local(&db_path)
            .build()
            .await
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
        let writer = Self {
            db,
            workspace_id: request.workspace_id,
            writer_id: request.writer_id,
            session_id: request.session_id,
            lease_reason: request.lease_reason,
            manifest_binding: request.manifest_binding,
        };
        writer.initialize_schema().await?;
        Ok(writer)
    }

    pub async fn acquire_writer_lease(
        &self,
        expected_heartbeat_counter: i64,
    ) -> Result<RuntimeJournalLeaseResult, RuntimeJournalWriterError> {
        let conn = self.connection().await?;
        let now = now_utc_string();
        conn.execute("BEGIN IMMEDIATE", ())
            .await
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;

        let result = async {
            conn.execute(
                "INSERT INTO runtime_writer_leases(
                    workspace_id, holder_id, holder_process, lease_reason, acquired_at,
                    heartbeat_at, heartbeat_counter, expires_at, session_id
                  ) VALUES (?1, ?2, ?3, ?4, ?5, ?5, 1, NULL, ?6)
                  ON CONFLICT(workspace_id) DO UPDATE SET
                    holder_id=excluded.holder_id,
                    holder_process=excluded.holder_process,
                    lease_reason=excluded.lease_reason,
                    heartbeat_at=excluded.heartbeat_at,
                    heartbeat_counter=runtime_writer_leases.heartbeat_counter + 1,
                    expires_at=excluded.expires_at,
                    session_id=excluded.session_id
                  WHERE runtime_writer_leases.heartbeat_counter = ?7",
                libsql::params![
                    self.workspace_id.clone(),
                    self.writer_id.clone(),
                    std::process::id().to_string(),
                    self.lease_reason.clone(),
                    now,
                    self.session_id.clone(),
                    expected_heartbeat_counter,
                ],
            )
            .await
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
            let counter = self.lease_counter_on(&conn).await?;
            Ok::<_, RuntimeJournalWriterError>(counter)
        }
        .await;

        commit_or_rollback(&conn, result)
            .await
            .map(|counter| RuntimeJournalLeaseResult {
                acquired: true,
                holder_id: self.writer_id.clone(),
                heartbeat_counter: counter,
            })
    }

    pub async fn ensure_session(
        &self,
        draft: &RuntimeJournalSessionDraft,
    ) -> Result<(), RuntimeJournalWriterError> {
        self.require_writer_lease().await?;
        let conn = self.connection().await?;
        conn.execute(
            "INSERT INTO runtime_sessions(
                session_id, started_at, status, user_prompt_summary, current_phase,
                manifest_set_hash, compiled_snapshot_id, git_head, git_dirty_tree_hash,
                default_execution_claim, default_custody_claim
              ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
              ON CONFLICT(session_id) DO UPDATE SET
                status=excluded.status,
                current_phase=excluded.current_phase,
                manifest_set_hash=excluded.manifest_set_hash,
                compiled_snapshot_id=excluded.compiled_snapshot_id,
                git_head=excluded.git_head,
                git_dirty_tree_hash=excluded.git_dirty_tree_hash,
                default_execution_claim=excluded.default_execution_claim,
                default_custody_claim=excluded.default_custody_claim",
            libsql::params![
                self.session_id.clone(),
                now_utc_string(),
                draft.status.clone(),
                draft.user_prompt_summary.clone(),
                draft.current_phase.clone(),
                self.manifest_binding.manifest_set_hash.clone(),
                self.manifest_binding.compiled_snapshot_id.clone(),
                self.manifest_binding.git_head.clone(),
                self.manifest_binding.git_dirty_tree_hash.clone(),
                draft.default_trust_claim.execution.clone(),
                draft.default_trust_claim.custody.clone(),
            ],
        )
        .await
        .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
        Ok(())
    }

    pub async fn append_event(
        &self,
        draft: RuntimeJournalEventDraft,
    ) -> Result<RuntimeJournalAppendResult, RuntimeJournalWriterError> {
        let decision =
            evaluate_runtime_event_append(&draft, self.manifest_binding.manifest_set_hash.as_str());
        if !decision.allow {
            return Err(RuntimeJournalWriterError::AppendRejected(decision.reason));
        }
        if draft.session_id != self.session_id {
            return Err(RuntimeJournalWriterError::AppendRejected(format!(
                "event draft session_id {} does not match writer session_id {}",
                draft.session_id, self.session_id
            )));
        }

        let conn = self.connection().await?;
        conn.execute("BEGIN IMMEDIATE", ())
            .await
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
        let result = async {
            self.require_writer_lease_on(&conn).await?;
            let seq = allocate_sequence(&conn, &self.session_id).await?;
            let event_id = format!("event.{}.{}", self.session_id, seq);
            let content_hash = event_content_hash(&draft, seq);
            let trust_claim_override = draft
                .trust_claim_override
                .as_ref()
                .map(serde_json::to_vec)
                .transpose()
                .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
            let payload_cbor = draft.payload_cbor_sha256.clone().map(Vec::from);
            conn.execute(
                "INSERT INTO runtime_events(
                    event_id, session_id, seq, occurred_at, phase, event_type, severity,
                    summary, manifest_set_hash, git_head, payload_cbor, content_hash,
                    previous_hash, trust_claim_override_cbor, proof_ref
                  ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                libsql::params![
                    event_id.clone(),
                    self.session_id.clone(),
                    seq,
                    now_utc_string(),
                    draft.phase.clone(),
                    draft.event_type.clone(),
                    draft.severity.clone(),
                    draft.summary.clone(),
                    draft.manifest_binding.manifest_set_hash.clone(),
                    draft.manifest_binding.git_head.clone(),
                    payload_cbor,
                    content_hash.clone(),
                    Option::<String>::None,
                    trust_claim_override,
                    draft.proof_ref.clone(),
                ],
            )
            .await
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
            Ok::<_, RuntimeJournalWriterError>(RuntimeJournalAppendResult {
                event_id,
                seq,
                content_hash,
            })
        }
        .await;

        commit_or_rollback(&conn, result).await
    }

    pub async fn record_validation_summary(
        &self,
        draft: &RuntimeJournalValidationSummaryDraft,
    ) -> Result<(), RuntimeJournalWriterError> {
        validate_required("validation_id", &draft.validation_id)?;
        validate_required("command_id", &draft.command_id)?;
        validate_required("structured_command_hash", &draft.structured_command_hash)?;
        validate_required("status", &draft.status)?;

        let conn = self.connection().await?;
        conn.execute("BEGIN IMMEDIATE", ())
            .await
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
        let result = async {
            self.require_writer_lease_on(&conn).await?;
            conn.execute(
                "INSERT INTO runtime_validation_summaries(
                    validation_id, session_id, command_id, structured_command_hash, exit_code,
                    stdout_hash, stderr_hash, duration_ms, status, manifest_set_hash, git_head, recorded_at
                  ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                  ON CONFLICT(validation_id) DO UPDATE SET
                    command_id=excluded.command_id,
                    structured_command_hash=excluded.structured_command_hash,
                    exit_code=excluded.exit_code,
                    stdout_hash=excluded.stdout_hash,
                    stderr_hash=excluded.stderr_hash,
                    duration_ms=excluded.duration_ms,
                    status=excluded.status,
                    manifest_set_hash=excluded.manifest_set_hash,
                    git_head=excluded.git_head,
                    recorded_at=excluded.recorded_at",
                libsql::params![
                    draft.validation_id.clone(),
                    self.session_id.clone(),
                    draft.command_id.clone(),
                    draft.structured_command_hash.clone(),
                    draft.exit_code,
                    draft.stdout_hash.clone(),
                    draft.stderr_hash.clone(),
                    draft.duration_ms,
                    draft.status.clone(),
                    self.manifest_binding.manifest_set_hash.clone(),
                    self.manifest_binding.git_head.clone(),
                    now_utc_string(),
                ],
            )
            .await
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
            Ok::<_, RuntimeJournalWriterError>(())
        }
        .await;
        commit_or_rollback(&conn, result).await
    }

    pub async fn record_decision(
        &self,
        draft: &RuntimeJournalDecisionDraft,
    ) -> Result<(), RuntimeJournalWriterError> {
        for (label, value) in [
            ("decision_id", draft.decision_id.as_str()),
            ("decision_class", draft.decision_class.as_str()),
            ("decision_type", draft.decision_type.as_str()),
            ("subject_type", draft.subject_type.as_str()),
            ("subject_id", draft.subject_id.as_str()),
            ("decided_by", draft.decided_by.as_str()),
            ("decision", draft.decision.as_str()),
            ("reason_summary", draft.reason_summary.as_str()),
            ("scope_hash", draft.scope_hash.as_str()),
        ] {
            validate_required(label, value)?;
        }
        let content_hash = content_hash_from_parts(&[
            "runtime_decision".to_string(),
            self.session_id.clone(),
            draft.decision_id.clone(),
            draft.decision_class.clone(),
            draft.decision_type.clone(),
            draft.subject_type.clone(),
            draft.subject_id.clone(),
            draft.decision.clone(),
            draft.scope_hash.clone(),
            self.manifest_binding.manifest_set_hash.clone(),
        ]);

        let conn = self.connection().await?;
        conn.execute("BEGIN IMMEDIATE", ())
            .await
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
        let result = async {
            self.require_writer_lease_on(&conn).await?;
            conn.execute(
                "INSERT INTO runtime_decisions(
                    decision_id, session_id, decision_class, decision_type, subject_type, subject_id,
                    decided_by, decided_at, decision, reason_summary, scope_hash, policy_snapshot_hash,
                    manifest_set_hash, git_head, content_hash, locked, superseded_by, proof_ref
                  ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, NULL, ?17)
                  ON CONFLICT(decision_id) DO UPDATE SET
                    decision_class=excluded.decision_class,
                    decision_type=excluded.decision_type,
                    subject_type=excluded.subject_type,
                    subject_id=excluded.subject_id,
                    decided_by=excluded.decided_by,
                    decided_at=excluded.decided_at,
                    decision=excluded.decision,
                    reason_summary=excluded.reason_summary,
                    scope_hash=excluded.scope_hash,
                    policy_snapshot_hash=excluded.policy_snapshot_hash,
                    manifest_set_hash=excluded.manifest_set_hash,
                    git_head=excluded.git_head,
                    content_hash=excluded.content_hash,
                    locked=excluded.locked,
                    superseded_by=NULL,
                    proof_ref=excluded.proof_ref",
                libsql::params![
                    draft.decision_id.clone(),
                    self.session_id.clone(),
                    draft.decision_class.clone(),
                    draft.decision_type.clone(),
                    draft.subject_type.clone(),
                    draft.subject_id.clone(),
                    draft.decided_by.clone(),
                    now_utc_string(),
                    draft.decision.clone(),
                    draft.reason_summary.clone(),
                    draft.scope_hash.clone(),
                    draft.policy_snapshot_hash.clone(),
                    self.manifest_binding.manifest_set_hash.clone(),
                    self.manifest_binding.git_head.clone(),
                    content_hash,
                    i64::from(draft.locked),
                    draft.proof_ref.clone(),
                ],
            )
            .await
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
            Ok::<_, RuntimeJournalWriterError>(())
        }
        .await;
        commit_or_rollback(&conn, result).await
    }

    pub async fn record_evidence_hint(
        &self,
        draft: &RuntimeJournalEvidenceHintDraft,
    ) -> Result<(), RuntimeJournalWriterError> {
        for (label, value) in [
            ("evidence_id", draft.evidence_id.as_str()),
            ("capability_id", draft.capability_id.as_str()),
            ("evidence_class", draft.evidence_class.as_str()),
            ("content_hash", draft.content_hash.as_str()),
        ] {
            validate_required(label, value)?;
        }
        let conn = self.connection().await?;
        conn.execute("BEGIN IMMEDIATE", ())
            .await
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
        let result = async {
            self.require_writer_lease_on(&conn).await?;
            conn.execute(
                "INSERT INTO runtime_evidence_hints(
                    evidence_id, session_id, capability_id, evidence_class, content_hash, previous_hash,
                    trust_execution_claim, trust_custody_claim, proof_ref, manifest_set_hash, git_head, recorded_at
                  ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                  ON CONFLICT(evidence_id) DO UPDATE SET
                    capability_id=excluded.capability_id,
                    evidence_class=excluded.evidence_class,
                    content_hash=excluded.content_hash,
                    previous_hash=excluded.previous_hash,
                    trust_execution_claim=excluded.trust_execution_claim,
                    trust_custody_claim=excluded.trust_custody_claim,
                    proof_ref=excluded.proof_ref,
                    manifest_set_hash=excluded.manifest_set_hash,
                    git_head=excluded.git_head,
                    recorded_at=excluded.recorded_at",
                libsql::params![
                    draft.evidence_id.clone(),
                    self.session_id.clone(),
                    draft.capability_id.clone(),
                    draft.evidence_class.clone(),
                    draft.content_hash.clone(),
                    draft.previous_hash.clone(),
                    draft.trust_claim.execution.clone(),
                    draft.trust_claim.custody.clone(),
                    draft.proof_ref.clone(),
                    self.manifest_binding.manifest_set_hash.clone(),
                    self.manifest_binding.git_head.clone(),
                    now_utc_string(),
                ],
            )
            .await
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
            Ok::<_, RuntimeJournalWriterError>(())
        }
        .await;
        commit_or_rollback(&conn, result).await
    }

    pub async fn close_session(
        &self,
        status: &str,
        current_phase: &str,
    ) -> Result<(), RuntimeJournalWriterError> {
        validate_required("status", status)?;
        validate_required("current_phase", current_phase)?;
        let conn = self.connection().await?;
        conn.execute("BEGIN IMMEDIATE", ())
            .await
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
        let result = async {
            self.require_writer_lease_on(&conn).await?;
            conn.execute(
                "UPDATE runtime_sessions
                 SET status = ?1, current_phase = ?2, closed_at = ?3
                 WHERE session_id = ?4",
                libsql::params![
                    status.to_string(),
                    current_phase.to_string(),
                    now_utc_string(),
                    self.session_id.clone(),
                ],
            )
            .await
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
            Ok::<_, RuntimeJournalWriterError>(())
        }
        .await;
        commit_or_rollback(&conn, result).await
    }

    pub async fn product_projection_counts(
        &self,
    ) -> Result<RuntimeJournalProductProjectionCounts, RuntimeJournalWriterError> {
        let conn = self.connection().await?;
        Ok(RuntimeJournalProductProjectionCounts {
            sessions: count_session_rows(&conn, "runtime_sessions", &self.session_id).await?,
            events: count_session_rows(&conn, "runtime_events", &self.session_id).await?,
            validation_summaries: count_session_rows(
                &conn,
                "runtime_validation_summaries",
                &self.session_id,
            )
            .await?,
            decisions: count_session_rows(&conn, "runtime_decisions", &self.session_id).await?,
            evidence_hints: count_session_rows(&conn, "runtime_evidence_hints", &self.session_id)
                .await?,
        })
    }

    pub async fn event_sequences(&self) -> Result<Vec<i64>, RuntimeJournalWriterError> {
        let conn = self.connection().await?;
        let mut rows = conn
            .query(
                "SELECT seq FROM runtime_events WHERE session_id = ? ORDER BY seq",
                libsql::params![self.session_id.clone()],
            )
            .await
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?
        {
            let seq: i64 = row
                .get(0)
                .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
            out.push(seq);
        }
        Ok(out)
    }

    async fn initialize_schema(&self) -> Result<(), RuntimeJournalWriterError> {
        let conn = self.connect_raw()?;
        apply_database_pragmas(&conn)
            .await
            .map_err(|err| RuntimeJournalWriterError::Pragma(err.to_string()))?;
        conn.execute_batch(RUNTIME_DB_MIGRATION_SQL)
            .await
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
        Ok(())
    }

    fn connect_raw(&self) -> Result<Connection, RuntimeJournalWriterError> {
        self.db
            .connect()
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))
    }

    async fn connection(&self) -> Result<Connection, RuntimeJournalWriterError> {
        let conn = self.connect_raw()?;
        apply_connection_pragmas(&conn)
            .await
            .map_err(|err| RuntimeJournalWriterError::Pragma(err.to_string()))?;
        Ok(conn)
    }

    async fn require_writer_lease(&self) -> Result<(), RuntimeJournalWriterError> {
        let conn = self.connection().await?;
        self.require_writer_lease_on(&conn).await
    }

    async fn require_writer_lease_on(
        &self,
        conn: &Connection,
    ) -> Result<(), RuntimeJournalWriterError> {
        let mut rows = conn
            .query(
                "SELECT heartbeat_counter FROM runtime_writer_leases WHERE workspace_id = ? AND holder_id = ?",
                libsql::params![self.workspace_id.clone(), self.writer_id.clone()],
            )
            .await
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
        if rows
            .next()
            .await
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?
            .is_none()
        {
            return Err(RuntimeJournalWriterError::LeaseRequired(format!(
                "writer {} has not acquired lease for workspace {}",
                self.writer_id, self.workspace_id
            )));
        }
        Ok(())
    }

    async fn lease_counter_on(&self, conn: &Connection) -> Result<i64, RuntimeJournalWriterError> {
        let mut rows = conn
            .query(
                "SELECT heartbeat_counter FROM runtime_writer_leases WHERE workspace_id = ? AND holder_id = ?",
                libsql::params![self.workspace_id.clone(), self.writer_id.clone()],
            )
            .await
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
        let row = rows
            .next()
            .await
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?
            .ok_or_else(|| {
                RuntimeJournalWriterError::LeaseRequired(format!(
                    "writer {} did not acquire lease for workspace {}",
                    self.writer_id, self.workspace_id
                ))
            })?;
        row.get(0)
            .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))
    }
}

async fn count_session_rows(
    conn: &Connection,
    table: &str,
    session_id: &str,
) -> Result<i64, RuntimeJournalWriterError> {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE session_id = ?1");
    let mut rows = conn
        .query(&sql, libsql::params![session_id.to_string()])
        .await
        .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
    let row = rows
        .next()
        .await
        .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?
        .ok_or_else(|| {
            RuntimeJournalWriterError::Database(format!("count returned no row for {table}"))
        })?;
    row.get(0)
        .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))
}

async fn allocate_sequence(
    conn: &Connection,
    session_id: &str,
) -> Result<i64, RuntimeJournalWriterError> {
    let mut rows = conn
        .query(
            "INSERT INTO runtime_session_sequences(session_id, next_seq, updated_at)
             VALUES (?1, 2, ?2)
             ON CONFLICT(session_id) DO UPDATE SET
               next_seq=runtime_session_sequences.next_seq + 1,
               updated_at=excluded.updated_at
             RETURNING next_seq - 1",
            libsql::params![session_id.to_string(), now_utc_string()],
        )
        .await
        .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
    let row = rows
        .next()
        .await
        .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?
        .ok_or_else(|| {
            RuntimeJournalWriterError::Database("sequence allocation returned no row".to_string())
        })?;
    row.get(0)
        .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))
}

async fn commit_or_rollback<T>(
    conn: &Connection,
    result: Result<T, RuntimeJournalWriterError>,
) -> Result<T, RuntimeJournalWriterError> {
    match result {
        Ok(value) => {
            conn.execute("COMMIT", ())
                .await
                .map_err(|err| RuntimeJournalWriterError::Database(err.to_string()))?;
            Ok(value)
        }
        Err(err) => {
            let _ = conn.execute("ROLLBACK", ()).await;
            Err(err)
        }
    }
}

fn validate_required(label: &str, value: &str) -> Result<(), RuntimeJournalWriterError> {
    if value.trim().is_empty() {
        return Err(RuntimeJournalWriterError::InvalidOpenRequest(format!(
            "{label} is required"
        )));
    }
    Ok(())
}

fn content_hash_from_parts(parts: &[String]) -> String {
    let mut hasher = Sha256::new();
    for item in parts {
        hasher.update(item.as_bytes());
        hasher.update([0]);
    }
    format!("sha256:{}", hex_lower(&hasher.finalize()))
}

fn validate_open_request(
    request: &RuntimeJournalOpenRequest,
) -> Result<(), RuntimeJournalWriterError> {
    for (label, value) in [
        ("workspace_id", request.workspace_id.as_str()),
        ("db_path", request.db_path.as_str()),
        (
            "manifest_set_hash",
            request.manifest_binding.manifest_set_hash.as_str(),
        ),
        (
            "compiled_snapshot_id",
            request.manifest_binding.compiled_snapshot_id.as_str(),
        ),
        ("writer_id", request.writer_id.as_str()),
        ("session_id", request.session_id.as_str()),
        ("lease_reason", request.lease_reason.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(RuntimeJournalWriterError::InvalidOpenRequest(format!(
                "{label} is required"
            )));
        }
    }
    Ok(())
}

fn event_content_hash(draft: &RuntimeJournalEventDraft, seq: i64) -> String {
    let mut hasher = Sha256::new();
    for item in [
        draft.session_id.as_str(),
        seq.to_string().as_str(),
        draft.phase.as_str(),
        draft.event_type.as_str(),
        draft.severity.as_str(),
        draft.summary.as_str(),
        draft.manifest_binding.manifest_set_hash.as_str(),
        draft.manifest_binding.git_head.as_deref().unwrap_or(""),
        draft.proof_ref.as_deref().unwrap_or(""),
    ] {
        hasher.update(item.as_bytes());
        hasher.update([0]);
    }
    format!("sha256:{}", hex_lower(&hasher.finalize()))
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn now_utc_string() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn binding() -> RuntimeManifestBinding {
        RuntimeManifestBinding {
            manifest_set_hash: "sha256:current".to_string(),
            compiled_snapshot_id: "snapshot.current".to_string(),
            git_head: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
            git_dirty_tree_hash: Some("sha256:dirty".to_string()),
        }
    }

    fn trust_claim() -> RuntimeTrustClaim {
        RuntimeTrustClaim {
            execution: "observed_l1".to_string(),
            custody: "local_only".to_string(),
            proof_ref: None,
        }
    }

    fn open_request(dir: &TempDir, writer_id: &str) -> RuntimeJournalOpenRequest {
        RuntimeJournalOpenRequest {
            workspace_id: "workspace.live-writer".to_string(),
            db_path: dir.path().join("runtime.db").display().to_string(),
            manifest_binding: binding(),
            writer_id: writer_id.to_string(),
            session_id: "session.live-writer".to_string(),
            lease_reason: "runtime journal writer test".to_string(),
        }
    }

    fn session_draft() -> RuntimeJournalSessionDraft {
        RuntimeJournalSessionDraft {
            status: "open".to_string(),
            user_prompt_summary: "live writer test".to_string(),
            current_phase: "executing".to_string(),
            default_trust_claim: trust_claim(),
        }
    }

    fn event_draft(event_type: &str) -> RuntimeJournalEventDraft {
        RuntimeJournalEventDraft {
            session_id: "session.live-writer".to_string(),
            phase: "executing".to_string(),
            event_type: event_type.to_string(),
            severity: "info".to_string(),
            summary: format!("{event_type} summary"),
            manifest_binding: binding(),
            payload_cbor_sha256: None,
            trust_claim_override: None,
            proof_ref: None,
        }
    }

    fn validation_draft(status: &str) -> RuntimeJournalValidationSummaryDraft {
        RuntimeJournalValidationSummaryDraft {
            validation_id: "validation.live-writer".to_string(),
            command_id: "cargo.test.live-writer".to_string(),
            structured_command_hash: "sha256:command".to_string(),
            exit_code: Some(0),
            stdout_hash: Some("sha256:stdout".to_string()),
            stderr_hash: Some("sha256:stderr".to_string()),
            duration_ms: Some(12),
            status: status.to_string(),
        }
    }

    fn decision_draft() -> RuntimeJournalDecisionDraft {
        RuntimeJournalDecisionDraft {
            decision_id: "decision.live-writer".to_string(),
            decision_class: "slice_local".to_string(),
            decision_type: "completion_lock_refresh".to_string(),
            subject_type: "session".to_string(),
            subject_id: "session.live-writer".to_string(),
            decided_by: "vac-runtime".to_string(),
            decision: "current".to_string(),
            reason_summary: "refresh decision under current manifest".to_string(),
            scope_hash: "sha256:scope".to_string(),
            policy_snapshot_hash: None,
            locked: true,
            proof_ref: None,
        }
    }

    fn evidence_draft() -> RuntimeJournalEvidenceHintDraft {
        RuntimeJournalEvidenceHintDraft {
            evidence_id: "evidence.live-writer".to_string(),
            capability_id: "vac.runtime.agent_loop".to_string(),
            evidence_class: "completion_summary".to_string(),
            content_hash: "sha256:evidence".to_string(),
            previous_hash: None,
            trust_claim: trust_claim(),
            proof_ref: None,
        }
    }

    #[tokio::test]
    async fn live_writer_acquires_lease_creates_session_and_appends_manifest_bound_events() {
        let dir = tempfile::tempdir().expect("tempdir");
        let writer = RuntimeJournalWriter::open(open_request(&dir, "writer.a"))
            .await
            .expect("open writer");

        let lease = writer
            .acquire_writer_lease(0)
            .await
            .expect("acquire writer lease");
        assert_eq!(lease.heartbeat_counter, 1);
        writer
            .ensure_session(&session_draft())
            .await
            .expect("ensure session");

        let first = writer
            .append_event(event_draft("first_event"))
            .await
            .expect("append first event");
        let second = writer
            .append_event(event_draft("second_event"))
            .await
            .expect("append second event");

        assert_eq!(first.seq, 1);
        assert_eq!(second.seq, 2);
        assert_ne!(first.content_hash, second.content_hash);
        assert_eq!(
            writer.event_sequences().await.expect("sequences"),
            vec![1, 2]
        );
    }

    #[tokio::test]
    async fn live_writer_records_completion_projection_tables() {
        let dir = tempfile::tempdir().expect("tempdir");
        let writer = RuntimeJournalWriter::open(open_request(&dir, "writer.a"))
            .await
            .expect("open writer");
        writer
            .acquire_writer_lease(0)
            .await
            .expect("acquire writer lease");
        writer
            .ensure_session(&session_draft())
            .await
            .expect("ensure session");
        writer
            .append_event(event_draft("runtime_journal_product_projection"))
            .await
            .expect("append product projection event");
        writer
            .record_validation_summary(&validation_draft("done"))
            .await
            .expect("record validation summary");
        writer
            .record_decision(&decision_draft())
            .await
            .expect("record decision");
        writer
            .record_evidence_hint(&evidence_draft())
            .await
            .expect("record evidence hint");
        writer
            .close_session("done", "done")
            .await
            .expect("close session");

        let counts = writer
            .product_projection_counts()
            .await
            .expect("projection counts");
        assert_eq!(counts.sessions, 1);
        assert_eq!(counts.events, 1);
        assert_eq!(counts.validation_summaries, 1);
        assert_eq!(counts.decisions, 1);
        assert_eq!(counts.evidence_hints, 1);
    }

    #[tokio::test]
    async fn live_writer_rejects_append_without_writer_lease() {
        let dir = tempfile::tempdir().expect("tempdir");
        let writer = RuntimeJournalWriter::open(open_request(&dir, "writer.a"))
            .await
            .expect("open writer");

        let err = writer
            .ensure_session(&session_draft())
            .await
            .expect_err("session write must require lease");
        assert!(matches!(err, RuntimeJournalWriterError::LeaseRequired(_)));
    }

    #[tokio::test]
    async fn live_writer_rejects_stale_manifest_event() {
        let dir = tempfile::tempdir().expect("tempdir");
        let writer = RuntimeJournalWriter::open(open_request(&dir, "writer.a"))
            .await
            .expect("open writer");
        writer
            .acquire_writer_lease(0)
            .await
            .expect("acquire writer lease");
        writer
            .ensure_session(&session_draft())
            .await
            .expect("ensure session");

        let mut stale = event_draft("stale_event");
        stale.manifest_binding.manifest_set_hash = "sha256:stale".to_string();
        let err = writer
            .append_event(stale)
            .await
            .expect_err("stale event must be rejected");
        assert!(matches!(err, RuntimeJournalWriterError::AppendRejected(_)));
        assert_eq!(
            writer.event_sequences().await.expect("sequences"),
            Vec::<i64>::new()
        );
    }

    #[tokio::test]
    async fn live_writer_lease_recovery_uses_expected_heartbeat_counter() {
        let dir = tempfile::tempdir().expect("tempdir");
        let writer_a = RuntimeJournalWriter::open(open_request(&dir, "writer.a"))
            .await
            .expect("open writer a");
        let writer_b = RuntimeJournalWriter::open(open_request(&dir, "writer.b"))
            .await
            .expect("open writer b");

        let a = writer_a
            .acquire_writer_lease(0)
            .await
            .expect("writer a lease");
        assert_eq!(a.heartbeat_counter, 1);

        let b = writer_b
            .acquire_writer_lease(1)
            .await
            .expect("writer b recovery");
        assert_eq!(b.heartbeat_counter, 2);

        let stale = writer_a.acquire_writer_lease(1).await;
        assert!(
            stale.is_err(),
            "stale expected counter must not recover lease"
        );
    }
}
