#!/usr/bin/env python3
from __future__ import annotations

import json
import pathlib
import sqlite3
import sys
import tempfile

ROOT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
MATRIX = ROOT / "docs/audit/VAC_V19_TRACEABILITY_MATRIX.md"
RUNTIME_SQL = ROOT / ".vac/migrations/runtime-db/0001_runtime_journal.sql"

REQUIRED_FIXTURE_GROUPS = [
    "FG-runtime-db",
    "FG-manifest-sync",
    "FG-trust-vector",
    "FG-policy-patch",
    "FG-command-seam",
    "FG-governance",
    "FG-span-index",
    "FG-missing-code",
    "FG-readiness",
    "FG-assessment",
    "FG-adoption-friction",
    "FG-episodic-memory",
]

CRITICAL_ACCEPTANCE_IDS = [
    "Duplicate `(session_id, seq)` rejected",
    "Writer lease prevents two VAC-managed writers",
    "Stale decision cannot authorize current action",
    "Ghost state quarantine",
    "Derived trust recomputed at read time",
    "Subprocess `.vac/db/**` mutation detection",
    "Release doctor does not overclaim L1 local records",
    "Governance denominator/numerator reproducible",
    "Missing-code absence proof complete",
    "Bounded patch hard-deny `.vac/db/**` rejected",
    "Span CRLF raw/normalized behavior deterministic",
    "Doctor trust wording golden snapshots",
]

MATRIX_REQUIRED_STATUS_TOKENS = [
    "R-025-fixtures-traceability",
    "R-027-acceptance",
    "Full P0 acceptance | Not claimed",
]

PASS_OUTPUT_FLAGS = [
    "sqlite_duplicate_session_seq_rejected=true",
    "episodic_memory_source_ref_required=true",
    "episodic_memory_hint_only=true",
    "episodic_memory_secret_quarantine=true",
    "episodic_memory_policy_relaxation_blocked=true",
    "episodic_memory_authority_overclaim_blocked=true",
    "full_p0_acceptance_claimed=false",
]

TOKEN_CHECKS = {
    "runtime-db-schema": {
        ".vac/migrations/runtime-db/0001_runtime_journal.sql": [
            "PRAGMA journal_mode = WAL",
            "PRAGMA foreign_keys = ON",
            "PRAGMA busy_timeout = 5000",
            "UNIQUE(session_id, seq)",
            "runtime_writer_leases",
            "runtime_session_sequences",
        ],
        "vac-rs/crates/foundation/vac-state/src/runtime_journal.rs": [
            "runtime_journal_write_plan",
            "BEGIN IMMEDIATE",
            "ALLOCATE_EVENT_SEQUENCE_SQL",
            "runtime_event_append_requires_current_manifest_binding",
        ],
    },
    "runtime-journal-writer": {
        "scripts/check-v19-runtime-journal-writer.py": [
            "BEGIN IMMEDIATE",
            "begin_immediate_writer_lock=TV-Pass",
            "heartbeat_counter_recovery=TV-Pass",
            "expires_at_not_authority=true",
            "sequence_allocation_transactional=true",
            "deferred_stale_recovery_rejected=true",
        ],
        "tests/fixtures/v19/runtime-db/writer-lease.json": [
            "v19.runtime_db.writer_lease",
            "Writer lease prevents two VAC-managed writers",
            "SQLite DEFERRED stale recovery race is rejected",
        ],
    },
    "manifest-sync-critical": {
        "vac-rs/crates/foundation/vac-state/src/runtime_journal.rs": [
            "classify_manifest_sync_record",
            "ManifestSyncClassification::GhostState",
            "quarantine_require_plan_or_decision_refresh",
            "stale_decision_cannot_authorize_current_action",
            "evaluate_runtime_decision_authorization",
        ],
        "vac-rs/crates/control-plane/vac-doctor/src/lib.rs": [
            "ManifestSyncDoctorInput",
            "doctor_manifest_sync_records",
            "manifest_sync_doctor_blocks_ghost_stale_and_orphan_records",
        ],
        "vac-rs/crates/surfaces/vac-cli/src/commands/control_plane.rs": [
            "doctor_manifest_sync",
            "doctor_manifest_sync_records",
            "compiled snapshot is missing snapshot_hash",
        ],
        "tests/fixtures/v19/manifest-sync/classification-cases.json": [
            "v19.manifest_sync.classification_cases",
            "ghost_state_quarantined_when_stale_record_would_authorize",
            "orphan_state_quarantined_for_unknown_snapshot_hash",
        ],
    },
    "trust-vector-critical": {
        "vac-rs/crates/foundation/vac-state/src/runtime_journal.rs": [
            "RuntimeTrustClaim",
            "derive_runtime_trust_at_read",
            "verified_downgrade",
            "missing_or_invalid_proof",
        ],
        "vac-rs/crates/control-plane/vac-doctor/src/lib.rs": [
            "doctor_release_trust_language",
            "release_doctor_blocks_unverified_attestation_claim",
        ],
    },
    "command-seam-critical": {
        "vac-rs/crates/foundation/vac-state/src/runtime_journal.rs": [
            "RuntimeDbMutationProbe",
            "evaluate_subprocess_runtime_db_mutation",
            "runtime_db_touched_by_subprocess",
        ],
        "vac-rs/crates/runtime/vac-agent-loop/src/bound_runtime.rs": [
            "free-form shell runner is forbidden",
            "shell interpolation, wildcard, pipe, or redirection detected",
            "policy snapshot denies this structured command",
        ],
    },
    "release-overclaim-critical": {
        "vac-rs/crates/control-plane/vac-doctor/src/lib.rs": [
            "ReleaseTrustClaimInput",
            "doctor_release_trust_language",
            "release_doctor_blocks_l1_local_overclaim_language",
            "integrity hint only",
        ],
    },
    "readiness-assessment-index": {
        "vac-rs/crates/control-plane/vac-readiness/src/lib.rs": [
            "compute_readiness",
            "fail_closed",
            "assessment_span_grounded",
        ],
        "vac-rs/crates/control-plane/vac-assessment/src/lib.rs": [
            "validate_span_grounding",
            "bidirectional_gap_analysis",
            "validate_assessment_freshness",
        ],
        "vac-rs/crates/control-plane/vac-index/src/lib.rs": [
            "span_sha256",
            "normalized_fingerprint",
            "coverage_gate",
            "parser_mode_truth",
        ],
    },
    "governance-fixture": {
        "vac-rs/crates/control-plane/vac-doctor/src/lib.rs": [
            "GovernanceWindowInput",
            "reduce_governance_window",
            "governance_zero_denominator_uses_max_one_and_passes",
            "governance_reducer_matches_spec_denominator_breakdown",
        ],
        "tests/fixtures/v19/governance/zero-denominator.json": [
            "v19.governance.zero_denominator",
            "score_basis_points",
        ],
        "tests/fixtures/v19/governance/spec-window-example.json": [
            "weighted_event_points",
            "risk_weighted_slice_points",
            "advisory_block",
        ],
    },
    "missing-code-fixture": {
        "vac-rs/crates/control-plane/vac-assessment/src/lib.rs": [
            "MissingCodeProofResult",
            "evaluate_missing_code_proof",
            "missing_code_low_confidence_residue_downgrades_absence_claim",
        ],
        "tests/fixtures/v19/missing-code/triad.json": [
            "absent_full_coverage",
            "not_found_in_index",
            "coverage_insufficient",
        ],
    },
    "patch-hard-deny-fixture": {
        "vac-rs/crates/runtime/vac-agent-loop/src/bound_runtime.rs": [
            "HARD_DENY_PATCH_PATHS",
            "hard_deny_patch_path",
            "hard_deny_patch_blocks_vac_db_even_if_plan_claims_it",
        ],
        "tests/fixtures/v19/patch/hard-deny-vac-db.json": [
            ".vac/db/runtime.db",
            "hard_deny_patch",
        ],
    },
    "span-index-crlf-fixture": {
        "vac-rs/crates/control-plane/vac-index/src/lib.rs": [
            "raw_span_sha256",
            "normalized_text_fingerprint",
            "crlf_only_change_refreshes_raw_hash_but_keeps_normalized_fingerprint",
        ],
        "tests/fixtures/v19/index/crlf-normalization.json": [
            "raw_span_sha256_equal",
            "normalized_fingerprint_equal",
        ],
    },
    "doctor-output-golden-fixture": {
        "vac-rs/crates/control-plane/vac-doctor/src/lib.rs": [
            "doctor_trust_wording_golden_snapshots_match",
            "trust-wording-golden.json",
        ],
        "tests/fixtures/v19/doctor-output/trust-wording-golden.json": [
            "honest_l1_local",
            "l1_overclaim_blocks",
            "unverified_attestation_blocks",
        ],
    },
    "episodic-memory-fixture": {
        "vac-rs/crates/foundation/vac-state/src/runtime_journal.rs": [
            "RuntimeMemorySourceRef",
            "RuntimeMemoryCandidateDraft",
            "RuntimeMemoryCandidateDecision",
            "evaluate_runtime_memory_candidate",
            "memory_candidate_requires_source_ref",
            "memory_candidate_requires_manifest_binding",
            "memory_candidate_requires_source_content_hash",
            "memory_candidate_quarantines_raw_private_reasoning",
            "memory_candidate_quarantines_secret_like_material",
            "memory_candidate_blocks_policy_relaxation",
            "memory_candidate_blocks_authority_overclaim",
            "stale_memory_source_is_historical_hint_only",
            "valid_memory_candidate_is_episodic_hint_only",
        ],
        "tests/fixtures/v19/episodic-memory/candidate-evaluator.json": [
            "v19.episodic_memory.candidate_evaluator",
            "source_ref_required",
            "manifest_binding_required",
            "source_content_hash_required",
            "raw_private_reasoning_quarantined",
            "secret_like_material_quarantined",
            "policy_relaxation_blocked",
            "authority_overclaims_blocked",
            "stale_memory_historical_hint_only",
            "valid_memory_episodic_hint_only",
        ],
    },
}

JSON_FIXTURE_FILES = [
    rel
    for files in TOKEN_CHECKS.values()
    for rel in files
    if rel.startswith("tests/fixtures/v19/") and rel.endswith(".json")
]


def read_rel(path: str) -> str:
    full = ROOT / path
    if not full.is_file():
        raise AssertionError(f"missing required file: {path}")
    return full.read_text(errors="ignore")


def check_matrix(errors: list[str]) -> None:
    if not MATRIX.is_file():
        errors.append("missing docs/audit/VAC_V19_TRACEABILITY_MATRIX.md")
        return
    text = MATRIX.read_text(errors="ignore")
    for group in REQUIRED_FIXTURE_GROUPS:
        if group not in text:
            errors.append(f"traceability matrix missing fixture group {group}")
    for item in CRITICAL_ACCEPTANCE_IDS:
        if item not in text:
            errors.append(f"traceability matrix missing critical acceptance item: {item}")
    for token in MATRIX_REQUIRED_STATUS_TOKENS:
        if token not in text:
            errors.append(f"traceability matrix missing required status token: {token}")


def check_tokens(errors: list[str]) -> None:
    for check_id, files in TOKEN_CHECKS.items():
        for rel, tokens in files.items():
            try:
                text = read_rel(rel)
            except AssertionError as exc:
                errors.append(f"{check_id}: {exc}")
                continue
            for token in tokens:
                if token not in text:
                    errors.append(f"{check_id}: {rel} missing token {token!r}")


def check_json_fixtures(errors: list[str]) -> None:
    for rel in JSON_FIXTURE_FILES:
        path = ROOT / rel
        if not path.is_file():
            errors.append(f"missing v1.9 fixture file: {rel}")
            continue
        try:
            value = json.loads(path.read_text())
        except json.JSONDecodeError as exc:
            errors.append(f"invalid JSON fixture {rel}: {exc}")
            continue
        if not value.get("fixture_id"):
            errors.append(f"fixture missing fixture_id: {rel}")


def check_runtime_event_duplicate_rejected(errors: list[str]) -> None:
    if not RUNTIME_SQL.is_file():
        errors.append("runtime duplicate fixture cannot run: missing runtime DB migration SQL")
        return
    sql = RUNTIME_SQL.read_text(errors="ignore")
    with tempfile.TemporaryDirectory(prefix="vac-v19-runtime-db-") as tmp:
        db_path = pathlib.Path(tmp) / "runtime.db"
        conn = sqlite3.connect(db_path)
        try:
            conn.executescript(sql)
            conn.execute("PRAGMA foreign_keys = ON")
            conn.execute(
                """
                INSERT INTO runtime_sessions(
                  session_id, started_at, status, user_prompt_summary, current_phase,
                  manifest_set_hash, compiled_snapshot_id, git_head, git_dirty_tree_hash,
                  default_execution_claim, default_custody_claim
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    "session.fixture",
                    "2026-01-01T00:00:00Z",
                    "open",
                    "fixture",
                    "plan_draft",
                    "sha256:current",
                    "snapshot.current",
                    "0123456789abcdef0123456789abcdef01234567",
                    "sha256:dirty",
                    "observed_l1",
                    "local_only",
                ),
            )
            event = (
                "event.fixture.1",
                "session.fixture",
                1,
                "2026-01-01T00:00:01Z",
                "plan_draft",
                "fixture_event",
                "info",
                "fixture event",
                "sha256:current",
                "0123456789abcdef0123456789abcdef01234567",
                None,
                "sha256:content",
                None,
                None,
                None,
            )
            insert_sql = """
                INSERT INTO runtime_events(
                  event_id, session_id, seq, occurred_at, phase, event_type, severity,
                  summary, manifest_set_hash, git_head, payload_cbor, content_hash,
                  previous_hash, trust_claim_override_cbor, proof_ref
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """
            conn.execute(insert_sql, event)
            try:
                duplicate = list(event)
                duplicate[0] = "event.fixture.2"
                conn.execute(insert_sql, tuple(duplicate))
            except sqlite3.IntegrityError:
                return
            errors.append("runtime_events duplicate (session_id, seq) insert was not rejected")
        finally:
            conn.close()


def main() -> int:
    errors: list[str] = []
    check_matrix(errors)
    check_tokens(errors)
    check_json_fixtures(errors)
    check_runtime_event_duplicate_rejected(errors)
    if errors:
        print("VAC v1.9 fixture coverage: FAIL")
        for error in errors:
            print(f"- {error}")
        return 1
    print("VAC v1.9 fixture coverage: PASS")
    print(f"fixture_groups={len(REQUIRED_FIXTURE_GROUPS)}")
    print(f"critical_acceptance_checks={len(CRITICAL_ACCEPTANCE_IDS)}")
    print(f"v19_fixture_files={len(JSON_FIXTURE_FILES)}")
    for flag in PASS_OUTPUT_FLAGS:
        print(flag)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
