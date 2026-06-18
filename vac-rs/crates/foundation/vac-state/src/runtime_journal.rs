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
    "runtime_broker_intents",
    "runtime_broker_decisions",
    "runtime_broker_execution_records",
    "runtime_broker_evidence_records",
    "runtime_broker_denials",
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManifestSyncClassification {
    Current,
    BranchDrift,
    StaleManifest,
    GhostState,
    OrphanState,
}

impl ManifestSyncClassification {
    #[must_use]
    pub fn action_label(self) -> &'static str {
        match self {
            Self::Current => "usable",
            Self::BranchDrift => "warn_keep_usable",
            Self::StaleManifest => "mark_stale_block_authorization",
            Self::GhostState => "quarantine_require_plan_or_decision_refresh",
            Self::OrphanState => "quarantine_require_operator_review",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestSyncRecordProbe {
    pub record_manifest_set_hash: String,
    pub current_manifest_set_hash: String,
    pub known_snapshot_hashes: Vec<String>,
    pub git_head_changed: bool,
    pub would_authorize_current_action: bool,
}

#[must_use]
pub fn classify_manifest_sync_record(
    probe: &ManifestSyncRecordProbe,
) -> ManifestSyncClassification {
    if probe.record_manifest_set_hash.trim().is_empty()
        || !probe
            .known_snapshot_hashes
            .iter()
            .any(|hash| hash == &probe.record_manifest_set_hash)
    {
        return ManifestSyncClassification::OrphanState;
    }

    if probe.record_manifest_set_hash != probe.current_manifest_set_hash {
        return if probe.would_authorize_current_action {
            ManifestSyncClassification::GhostState
        } else {
            ManifestSyncClassification::StaleManifest
        };
    }

    if probe.git_head_changed {
        ManifestSyncClassification::BranchDrift
    } else {
        ManifestSyncClassification::Current
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeDecisionAuthorizationRequest {
    pub decision_id: String,
    pub decision_manifest_set_hash: String,
    pub current_manifest_set_hash: String,
    pub locked: bool,
    pub superseded_by: Option<String>,
    pub would_authorize_current_action: bool,
}

#[must_use]
pub fn evaluate_runtime_decision_authorization(
    request: &RuntimeDecisionAuthorizationRequest,
) -> RuntimeJournalAppendDecision {
    if request.decision_id.trim().is_empty() {
        return blocked("decision_id missing");
    }
    if !request.locked {
        return blocked("decision is not locked");
    }
    if request.superseded_by.is_some() {
        return blocked("decision has been superseded");
    }
    if request.decision_manifest_set_hash != request.current_manifest_set_hash {
        return if request.would_authorize_current_action {
            blocked("ghost_state: stale decision cannot authorize current action")
        } else {
            blocked("stale_manifest: refresh decision under current manifest_set_hash")
        };
    }

    RuntimeJournalAppendDecision {
        allow: true,
        reason: "locked current-manifest decision may authorize scoped work".to_string(),
        required_transaction: "BEGIN IMMEDIATE".to_string(),
        writes_manifest_bound_record: true,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeDerivedTrust {
    pub execution: String,
    pub custody: String,
    pub derivation: String,
    pub downgrade_reason: Option<String>,
    pub wording: String,
}

fn custody_requires_proof(custody: &str) -> bool {
    matches!(
        custody,
        "ci_attested" | "broker_attested" | "external_attested"
    )
}

#[must_use]
pub fn derive_runtime_trust_at_read(
    claim: &RuntimeTrustClaim,
    proof_material_verified: bool,
) -> RuntimeDerivedTrust {
    if custody_requires_proof(&claim.custody) && !proof_material_verified {
        return RuntimeDerivedTrust {
            execution: claim.execution.clone(),
            custody: "self_promoted".to_string(),
            derivation: "verified_downgrade".to_string(),
            downgrade_reason: Some("missing_or_invalid_proof".to_string()),
            wording: "shared cooperative record; not tamper-evident".to_string(),
        };
    }

    let wording = match (claim.execution.as_str(), claim.custody.as_str()) {
        ("observed_l1", "local_only") => "local self-reported trace; integrity hint only",
        ("observed_l1", "self_promoted") => "shared cooperative record; not tamper-evident",
        ("observed_l1", "ci_attested") => "CI-attested self-report; execution not mediated",
        ("mediated_l2", "broker_attested") => "broker-attested mediated execution",
        _ => "derived trust claim requires explicit surface wording",
    };

    RuntimeDerivedTrust {
        execution: claim.execution.clone(),
        custody: claim.custody.clone(),
        derivation: "verified".to_string(),
        downgrade_reason: None,
        wording: wording.to_string(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeDbMutationProbe {
    pub command_id: String,
    pub before_hashes: Vec<String>,
    pub after_hashes: Vec<String>,
}

#[must_use]
pub fn evaluate_subprocess_runtime_db_mutation(
    probe: &RuntimeDbMutationProbe,
) -> RuntimeJournalAppendDecision {
    if probe.before_hashes != probe.after_hashes {
        return blocked("runtime_db_touched_by_subprocess");
    }
    RuntimeJournalAppendDecision {
        allow: true,
        reason: format!(
            "structured command {} did not mutate .vac/db/**",
            probe.command_id
        ),
        required_transaction: "BEGIN IMMEDIATE".to_string(),
        writes_manifest_bound_record: true,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeBrokerMediatedRecordProbe {
    pub intent_id: String,
    pub decision_id: Option<String>,
    pub policy_snapshot_hash: String,
    pub current_policy_snapshot_hash: String,
    pub execution_mode: String,
    pub custody: String,
    pub broker_record_hash: Option<String>,
    pub broker_signature_hash: Option<String>,
    pub tool_supplied_policy_decision: bool,
}

#[must_use]
pub fn evaluate_runtime_broker_mediated_record(
    probe: &RuntimeBrokerMediatedRecordProbe,
) -> RuntimeJournalAppendDecision {
    if probe.tool_supplied_policy_decision {
        return blocked("tool_supplied_policy_decision_rejected");
    }
    if probe.intent_id.trim().is_empty() {
        return blocked("missing_broker_intent");
    }
    if probe
        .decision_id
        .as_ref()
        .is_none_or(|value| value.trim().is_empty())
    {
        return blocked("execution_record_without_broker_decision");
    }
    if probe.policy_snapshot_hash.trim().is_empty() {
        return blocked("missing_policy_snapshot_hash");
    }
    if probe.policy_snapshot_hash != probe.current_policy_snapshot_hash {
        return blocked("stale_policy_snapshot_hash");
    }
    if probe.execution_mode == "mediated_l2"
        && probe
            .broker_record_hash
            .as_ref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return blocked("mediated_l2_without_broker_record_hash");
    }
    if probe.custody == "broker_attested"
        && probe
            .broker_signature_hash
            .as_ref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return blocked("broker_attested_without_broker_signature_hash");
    }
    RuntimeJournalAppendDecision {
        allow: true,
        reason: "broker-mediated runtime journal record passes schema boundary".to_string(),
        required_transaction: "BEGIN IMMEDIATE".to_string(),
        writes_manifest_bound_record: true,
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
        let end = rest.find(";\n").map(|idx| idx + 1).unwrap_or(rest.len());
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
            "runtime_broker_intents",
            "runtime_broker_decisions",
            "runtime_broker_execution_records",
            "runtime_broker_evidence_records",
            "runtime_broker_denials",
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
    fn runtime_broker_mediated_schema_constraints_are_fail_closed() {
        let broker_intents =
            create_table_statement(RUNTIME_DB_MIGRATION_SQL, "runtime_broker_intents");
        assert!(broker_intents.contains("intent_id TEXT PRIMARY KEY"));
        assert!(broker_intents.contains("UNIQUE(session_id, seq)"));

        let broker_decisions =
            create_table_statement(RUNTIME_DB_MIGRATION_SQL, "runtime_broker_decisions");
        assert!(broker_decisions.contains("FOREIGN KEY(intent_id)"));
        assert!(broker_decisions.contains("policy_snapshot_hash TEXT NOT NULL"));

        let execution_records =
            create_table_statement(RUNTIME_DB_MIGRATION_SQL, "runtime_broker_execution_records");
        assert!(execution_records.contains("decision_id TEXT NOT NULL"));
        assert!(execution_records.contains("FOREIGN KEY(decision_id)"));
        assert!(execution_records.contains("execution_mode != 'mediated_l2'"));
        assert!(execution_records.contains("broker_record_hash IS NOT NULL"));

        let evidence_records =
            create_table_statement(RUNTIME_DB_MIGRATION_SQL, "runtime_broker_evidence_records");
        assert!(evidence_records.contains("custody != 'broker_attested'"));
        assert!(evidence_records.contains("broker_signature_hash IS NOT NULL"));

        let denials = create_table_statement(RUNTIME_DB_MIGRATION_SQL, "runtime_broker_denials");
        assert!(denials.contains("decision_id TEXT NOT NULL"));
        assert!(denials.contains("FOREIGN KEY(decision_id)"));
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

    #[test]
    fn manifest_sync_classifies_ghost_and_orphan_state() {
        let known_snapshot_hashes = vec!["sha256:old".to_string(), "sha256:current".to_string()];

        let ghost = classify_manifest_sync_record(&ManifestSyncRecordProbe {
            record_manifest_set_hash: "sha256:old".to_string(),
            current_manifest_set_hash: "sha256:current".to_string(),
            known_snapshot_hashes: known_snapshot_hashes.clone(),
            git_head_changed: false,
            would_authorize_current_action: true,
        });
        assert_eq!(ghost, ManifestSyncClassification::GhostState);
        assert_eq!(
            ghost.action_label(),
            "quarantine_require_plan_or_decision_refresh"
        );

        let orphan = classify_manifest_sync_record(&ManifestSyncRecordProbe {
            record_manifest_set_hash: "sha256:unknown".to_string(),
            current_manifest_set_hash: "sha256:current".to_string(),
            known_snapshot_hashes,
            git_head_changed: false,
            would_authorize_current_action: false,
        });
        assert_eq!(orphan, ManifestSyncClassification::OrphanState);
        assert_eq!(orphan.action_label(), "quarantine_require_operator_review");
    }

    #[test]
    fn stale_decision_cannot_authorize_current_action() {
        let decision =
            evaluate_runtime_decision_authorization(&RuntimeDecisionAuthorizationRequest {
                decision_id: "decision.test".to_string(),
                decision_manifest_set_hash: "sha256:old".to_string(),
                current_manifest_set_hash: "sha256:current".to_string(),
                locked: true,
                superseded_by: None,
                would_authorize_current_action: true,
            });
        assert!(!decision.allow);
        assert!(decision.reason.contains("ghost_state"));
        assert!(decision.reason.contains("stale decision cannot authorize"));
    }

    #[test]
    fn derived_trust_downgrades_unverified_attestation_at_read_time() {
        let claim = RuntimeTrustClaim {
            execution: "observed_l1".to_string(),
            custody: "ci_attested".to_string(),
            proof_ref: Some("proof.missing".to_string()),
        };
        let derived = derive_runtime_trust_at_read(&claim, false);
        assert_eq!(derived.custody, "self_promoted");
        assert_eq!(derived.derivation, "verified_downgrade");
        assert_eq!(
            derived.downgrade_reason.as_deref(),
            Some("missing_or_invalid_proof")
        );
        assert!(derived.wording.contains("not tamper-evident"));
    }

    fn broker_probe() -> RuntimeBrokerMediatedRecordProbe {
        RuntimeBrokerMediatedRecordProbe {
            intent_id: "intent.1".to_string(),
            decision_id: Some("decision.1".to_string()),
            policy_snapshot_hash: "sha256:policy".to_string(),
            current_policy_snapshot_hash: "sha256:policy".to_string(),
            execution_mode: "mediated_l2".to_string(),
            custody: "broker_attested".to_string(),
            broker_record_hash: Some("sha256:broker-record".to_string()),
            broker_signature_hash: Some("sha256:broker-signature".to_string()),
            tool_supplied_policy_decision: false,
        }
    }

    #[test]
    fn runtime_broker_mediated_record_negative_fixtures_fail_closed() {
        let duplicate_intent_schema =
            create_table_statement(RUNTIME_DB_MIGRATION_SQL, "runtime_broker_intents");
        assert!(duplicate_intent_schema.contains("intent_id TEXT PRIMARY KEY"));

        let mut missing_decision = broker_probe();
        missing_decision.decision_id = None;
        let decision = evaluate_runtime_broker_mediated_record(&missing_decision);
        assert!(!decision.allow);
        assert!(
            decision
                .reason
                .contains("execution_record_without_broker_decision")
        );

        let mut mediated_without_hash = broker_probe();
        mediated_without_hash.broker_record_hash = None;
        let mediated = evaluate_runtime_broker_mediated_record(&mediated_without_hash);
        assert!(!mediated.allow);
        assert!(
            mediated
                .reason
                .contains("mediated_l2_without_broker_record_hash")
        );

        let mut broker_attested_without_signature = broker_probe();
        broker_attested_without_signature.broker_signature_hash = None;
        let attested = evaluate_runtime_broker_mediated_record(&broker_attested_without_signature);
        assert!(!attested.allow);
        assert!(
            attested
                .reason
                .contains("broker_attested_without_broker_signature_hash")
        );

        let mut injected_decision = broker_probe();
        injected_decision.tool_supplied_policy_decision = true;
        let injected = evaluate_runtime_broker_mediated_record(&injected_decision);
        assert!(!injected.allow);
        assert!(
            injected
                .reason
                .contains("tool_supplied_policy_decision_rejected")
        );

        let mut stale_policy = broker_probe();
        stale_policy.policy_snapshot_hash = "sha256:old".to_string();
        let stale = evaluate_runtime_broker_mediated_record(&stale_policy);
        assert!(!stale.allow);
        assert!(stale.reason.contains("stale_policy_snapshot_hash"));

        let current = evaluate_runtime_broker_mediated_record(&broker_probe());
        assert!(current.allow);
        assert_eq!(current.required_transaction, "BEGIN IMMEDIATE");
        assert!(current.writes_manifest_bound_record);
    }

    #[test]
    fn subprocess_runtime_db_mutation_blocks_completion() {
        let touched = evaluate_subprocess_runtime_db_mutation(&RuntimeDbMutationProbe {
            command_id: "cargo.test.core".to_string(),
            before_hashes: vec!["runtime.db=sha256:before".to_string()],
            after_hashes: vec!["runtime.db=sha256:after".to_string()],
        });
        assert!(!touched.allow);
        assert_eq!(touched.reason, "runtime_db_touched_by_subprocess");

        let clean = evaluate_subprocess_runtime_db_mutation(&RuntimeDbMutationProbe {
            command_id: "cargo.test.core".to_string(),
            before_hashes: vec!["runtime.db=sha256:same".to_string()],
            after_hashes: vec!["runtime.db=sha256:same".to_string()],
        });
        assert!(clean.allow);
    }
}
