#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// v1.9 manifest binding stamped on every runtime record that can authorize
/// future VAC-managed work. Stored verdict fields are not trusted as derived
/// truth; readers recompute trust from this binding and proof references.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeManifestBinding {
    pub manifest_set_hash: String,
    pub compiled_snapshot_id: String,
    pub git_head: Option<String>,
    pub git_dirty_tree_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeTrustClaim {
    pub execution: String,
    pub custody: String,
    pub proof_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeJournalRecordEnvelope {
    pub record_id: String,
    pub record_type: String,
    pub session_id: String,
    pub manifest_binding: RuntimeManifestBinding,
    pub trust_claim: RuntimeTrustClaim,
    pub content_hash: String,
    pub previous_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeJournalOpenRequest {
    pub workspace_id: String,
    pub db_path: String,
    pub manifest_binding: RuntimeManifestBinding,
    pub writer_id: String,
    pub session_id: String,
    pub lease_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeJournalWritePlan {
    pub transaction_mode: String,
    pub pragmas: Vec<&'static str>,
    pub migration_files: Vec<&'static str>,
    pub lease_sql: &'static str,
    pub sequence_sql: &'static str,
    pub insert_event_sql: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeJournalEventDraft {
    pub session_id: String,
    pub phase: String,
    pub event_type: String,
    pub severity: String,
    pub summary: String,
    pub manifest_binding: RuntimeManifestBinding,
    pub payload_cbor_sha256: Option<String>,
    pub trust_claim_override: Option<RuntimeTrustClaim>,
    pub proof_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeJournalAppendDecision {
    pub allow: bool,
    pub reason: String,
    pub required_transaction: String,
    pub writes_manifest_bound_record: bool,
}

pub const RUNTIME_DB_REQUIRED_TABLES: &[&str] = &[
    "runtime_schema_migrations",
    "runtime_sessions",
    "runtime_events",
    "runtime_decisions",
    "runtime_plan_revisions",
    "runtime_validation_summaries",
    "runtime_evidence_hints",
    "runtime_specsync_proposals",
    "runtime_writer_leases",
    "runtime_lease_observations",
    "runtime_session_sequences",
];

pub const RUNTIME_DB_REQUIRED_PRAGMAS: &[&str] = &[
    "PRAGMA journal_mode = WAL;",
    "PRAGMA foreign_keys = ON;",
    "PRAGMA busy_timeout = 5000;",
];

pub const ACQUIRE_WRITER_LEASE_SQL: &str = "BEGIN IMMEDIATE; INSERT INTO runtime_writer_leases(workspace_id, holder_id, lease_reason, acquired_at, heartbeat_at, heartbeat_counter, session_id) VALUES (?1, ?2, ?3, ?4, ?4, 1, ?5) ON CONFLICT(workspace_id) DO UPDATE SET holder_id=excluded.holder_id, lease_reason=excluded.lease_reason, heartbeat_at=excluded.heartbeat_at, heartbeat_counter=runtime_writer_leases.heartbeat_counter + 1, session_id=excluded.session_id WHERE runtime_writer_leases.heartbeat_counter = ?6;";

pub const ALLOCATE_EVENT_SEQUENCE_SQL: &str = "INSERT INTO runtime_session_sequences(session_id, next_seq, updated_at) VALUES (?1, 2, ?2) ON CONFLICT(session_id) DO UPDATE SET next_seq=runtime_session_sequences.next_seq + 1, updated_at=excluded.updated_at RETURNING next_seq - 1;";

pub const INSERT_EVENT_SQL: &str = "INSERT INTO runtime_events(event_id, session_id, seq, occurred_at, phase, event_type, severity, summary, manifest_set_hash, git_head, payload_cbor, content_hash, previous_hash, trust_claim_override_cbor, proof_ref) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15);";

#[must_use]
pub fn runtime_db_migration_has_required_tables(sql: &str) -> Vec<String> {
    RUNTIME_DB_REQUIRED_TABLES
        .iter()
        .filter(|table| !sql.contains(**table))
        .map(|table| (*table).to_string())
        .collect()
}

#[must_use]
pub fn runtime_db_migration_has_required_pragmas(sql: &str) -> Vec<String> {
    RUNTIME_DB_REQUIRED_PRAGMAS
        .iter()
        .filter(|pragma| !sql.contains(**pragma))
        .map(|pragma| (*pragma).to_string())
        .collect()
}

#[must_use]
pub fn runtime_journal_write_plan() -> RuntimeJournalWritePlan {
    RuntimeJournalWritePlan {
        transaction_mode: "BEGIN IMMEDIATE".to_string(),
        pragmas: RUNTIME_DB_REQUIRED_PRAGMAS.to_vec(),
        migration_files: vec![".vac/migrations/runtime-db/0001_runtime_journal.sql"],
        lease_sql: ACQUIRE_WRITER_LEASE_SQL,
        sequence_sql: ALLOCATE_EVENT_SEQUENCE_SQL,
        insert_event_sql: INSERT_EVENT_SQL,
    }
}

#[must_use]
pub fn evaluate_runtime_event_append(
    draft: &RuntimeJournalEventDraft,
    current_manifest_set_hash: &str,
) -> RuntimeJournalAppendDecision {
    if draft.manifest_binding.manifest_set_hash.trim().is_empty() {
        return blocked("missing manifest_set_hash");
    }
    if draft.manifest_binding.manifest_set_hash != current_manifest_set_hash {
        return blocked(
            "stale manifest_set_hash; run manifest-sync refresh before authorizing new work",
        );
    }
    if draft.session_id.trim().is_empty() || draft.event_type.trim().is_empty() {
        return blocked("event draft missing session_id or event_type");
    }
    RuntimeJournalAppendDecision {
        allow: true,
        reason: "manifest-bound event may be appended under writer lease".to_string(),
        required_transaction: "BEGIN IMMEDIATE".to_string(),
        writes_manifest_bound_record: true,
    }
}

#[must_use]
fn blocked(reason: &str) -> RuntimeJournalAppendDecision {
    RuntimeJournalAppendDecision {
        allow: false,
        reason: reason.to_string(),
        required_transaction: "BEGIN IMMEDIATE".to_string(),
        writes_manifest_bound_record: false,
    }
}
