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


#[cfg(test)]
mod tests {
    use super::*;

    const RUNTIME_DB_MIGRATION_SQL: &str =
        include_str!("../../../../../.vac/migrations/runtime-db/0001_runtime_journal.sql");

    fn binding(hash: &str) -> RuntimeManifestBinding {
        RuntimeManifestBinding {
            manifest_set_hash: hash.to_string(),
            compiled_snapshot_id: "snapshot.test".to_string(),
            git_head: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
            git_dirty_tree_hash: Some("sha256:dirty".to_string()),
        }
    }

    fn event_draft(hash: &str) -> RuntimeJournalEventDraft {
        RuntimeJournalEventDraft {
            session_id: "session.test".to_string(),
            phase: "plan_draft".to_string(),
            event_type: "runtime_projection_written".to_string(),
            severity: "info".to_string(),
            summary: "runtime projection was written from manifest-bound journal state".to_string(),
            manifest_binding: binding(hash),
            payload_cbor_sha256: Some("sha256:payload".to_string()),
            trust_claim_override: None,
            proof_ref: None,
        }
    }

    fn create_table_statement(sql: &str, table: &str) -> String {
        let needle = format!("CREATE TABLE IF NOT EXISTS {table}");
        let start = sql
            .find(&needle)
            .unwrap_or_else(|| panic!("missing table statement for {table}"));
        let rest = &sql[start..];
        let end = rest
            .find(";\n")
            .map(|idx| idx + 1)
            .unwrap_or(rest.len());
        rest[..end].to_string()
    }

    #[test]
    fn migration_declares_v19_runtime_journal_tables_and_pragmas() {
        assert!(runtime_db_migration_has_required_tables(RUNTIME_DB_MIGRATION_SQL).is_empty());
        assert!(runtime_db_migration_has_required_pragmas(RUNTIME_DB_MIGRATION_SQL).is_empty());
    }

    #[test]
    fn runtime_state_tables_are_manifest_bound() {
        for table in [
            "runtime_sessions",
            "runtime_events",
            "runtime_decisions",
            "runtime_plan_revisions",
            "runtime_validation_summaries",
            "runtime_evidence_hints",
            "runtime_specsync_proposals",
        ] {
            let statement = create_table_statement(RUNTIME_DB_MIGRATION_SQL, table);
            assert!(
                statement.contains("manifest_set_hash TEXT NOT NULL"),
                "{table} must carry manifest_set_hash to authorize future work"
            );
        }

        for table in ["runtime_sessions", "runtime_plan_revisions"] {
            let statement = create_table_statement(RUNTIME_DB_MIGRATION_SQL, table);
            assert!(
                statement.contains("compiled_snapshot_id TEXT NOT NULL"),
                "{table} must carry compiled_snapshot_id for snapshot provenance"
            );
        }
    }

    #[test]
    fn runtime_journal_write_plan_requires_begin_immediate_and_wal() {
        let plan = runtime_journal_write_plan();
        assert_eq!(plan.transaction_mode, "BEGIN IMMEDIATE");
        assert!(plan.pragmas.contains(&"PRAGMA journal_mode = WAL;"));
        assert!(plan.pragmas.contains(&"PRAGMA foreign_keys = ON;"));
        assert!(plan.pragmas.contains(&"PRAGMA busy_timeout = 5000;"));
        assert!(plan.lease_sql.starts_with("BEGIN IMMEDIATE;"));
        assert!(plan.lease_sql.contains("heartbeat_counter"));
        assert!(plan.sequence_sql.contains("RETURNING next_seq - 1"));
        assert!(plan.insert_event_sql.contains("manifest_set_hash"));
    }

    #[test]
    fn runtime_event_append_requires_current_manifest_binding() {
        let current_hash = "sha256:current";

        let stale = evaluate_runtime_event_append(&event_draft("sha256:old"), current_hash);
        assert!(!stale.allow);
        assert!(stale.reason.contains("stale manifest_set_hash"));
        assert!(!stale.writes_manifest_bound_record);

        let missing = evaluate_runtime_event_append(&event_draft(""), current_hash);
        assert!(!missing.allow);
        assert!(missing.reason.contains("missing manifest_set_hash"));

        let current = evaluate_runtime_event_append(&event_draft(current_hash), current_hash);
        assert!(current.allow);
        assert_eq!(current.required_transaction, "BEGIN IMMEDIATE");
        assert!(current.writes_manifest_bound_record);
    }
}
