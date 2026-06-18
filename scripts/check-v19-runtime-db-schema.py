#!/usr/bin/env python3
"""Validate VAC v1.9 runtime DB source schema without creating/tracking runtime.db."""
from __future__ import annotations
import pathlib, re, sys

ROOT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
SQL = ROOT / ".vac/migrations/runtime-db/0001_runtime_journal.sql"
REQUIRED_TABLES = [
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
]
REQUIRED_FIELDS = [
    "manifest_set_hash",
    "compiled_snapshot_id",
    "git_head",
    "git_dirty_tree_hash",
    "default_execution_claim",
    "default_custody_claim",
    "content_hash",
    "previous_hash",
]
errors: list[str] = []
if not SQL.exists():
    errors.append("missing .vac/migrations/runtime-db/0001_runtime_journal.sql")
    sql = ""
else:
    sql = SQL.read_text(errors="ignore")
for pragma in ["PRAGMA journal_mode = WAL", "PRAGMA foreign_keys = ON", "PRAGMA busy_timeout = 5000"]:
    if pragma not in sql:
        errors.append(f"missing SQLite pragma: {pragma}")
for table in REQUIRED_TABLES:
    if not re.search(rf"CREATE TABLE IF NOT EXISTS\s+{re.escape(table)}\b", sql, flags=re.I):
        errors.append(f"missing required runtime journal table: {table}")
for field in REQUIRED_FIELDS:
    if field not in sql:
        errors.append(f"missing required manifest/trust binding field: {field}")
runtime_journal = ROOT / "vac-rs/crates/foundation/vac-state/src/runtime_journal.rs"
if not runtime_journal.is_file():
    errors.append("missing vac-state runtime_journal.rs source API")
else:
    journal_src = runtime_journal.read_text(errors="ignore")
    for token in ["RuntimeJournalOpenRequest", "RuntimeJournalEventDraft", "RuntimeJournalAppendDecision", "RuntimeBrokerMediatedRecordProbe", "BEGIN IMMEDIATE", "runtime_journal_write_plan", "evaluate_runtime_event_append", "evaluate_runtime_broker_mediated_record"]:
        if token not in journal_src:
            errors.append(f"runtime_journal.rs missing v1.9 operational API token: {token}")
if (ROOT / ".vac/db/runtime.db").exists():
    errors.append(".vac/db/runtime.db must not be present in source tree/checkpoint")
ignore = ROOT / ".vac/db/.gitignore"
if not ignore.exists() or "*" not in ignore.read_text(errors="ignore"):
    errors.append(".vac/db/.gitignore must ignore local runtime DB state")
if errors:
    print("VAC v1.9 runtime DB schema: FAIL")
    for e in errors:
        print("-", e)
    sys.exit(1)
print("VAC v1.9 runtime DB schema: PASS")
print("runtime_db_source=.vac/migrations/runtime-db/0001_runtime_journal.sql")
print("runtime_db_state=.vac/db/runtime.db ignored/not packaged")
print(f"required_tables={len(REQUIRED_TABLES)}")
