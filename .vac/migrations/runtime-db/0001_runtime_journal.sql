-- VAC runtime DB migration 0001: v1.9 local runtime journal.
-- This schema is source authority for the local SQLite journal, but runtime.db itself is ignored state.
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;

CREATE TABLE IF NOT EXISTS runtime_schema_migrations (
  id TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL,
  manifest_set_hash TEXT NOT NULL,
  sql_hash TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS runtime_sessions (
  session_id TEXT PRIMARY KEY,
  started_at TEXT NOT NULL,
  closed_at TEXT,
  status TEXT NOT NULL,
  user_prompt_summary TEXT NOT NULL,
  current_phase TEXT NOT NULL,
  manifest_set_hash TEXT NOT NULL,
  compiled_snapshot_id TEXT NOT NULL,
  git_head TEXT,
  git_dirty_tree_hash TEXT,
  default_execution_claim TEXT NOT NULL,
  default_custody_claim TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS runtime_events (
  event_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  seq INTEGER NOT NULL,
  occurred_at TEXT NOT NULL,
  phase TEXT NOT NULL,
  event_type TEXT NOT NULL,
  severity TEXT NOT NULL,
  summary TEXT NOT NULL,
  manifest_set_hash TEXT NOT NULL,
  git_head TEXT,
  payload_cbor BLOB,
  content_hash TEXT NOT NULL,
  previous_hash TEXT,
  trust_claim_override_cbor BLOB,
  proof_ref TEXT,
  UNIQUE(session_id, seq),
  FOREIGN KEY(session_id) REFERENCES runtime_sessions(session_id)
);

CREATE TABLE IF NOT EXISTS runtime_decisions (
  decision_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  decision_class TEXT NOT NULL,
  decision_type TEXT NOT NULL,
  subject_type TEXT NOT NULL,
  subject_id TEXT NOT NULL,
  decided_by TEXT NOT NULL,
  decided_at TEXT NOT NULL,
  decision TEXT NOT NULL,
  reason_summary TEXT NOT NULL,
  scope_hash TEXT NOT NULL,
  policy_snapshot_hash TEXT,
  manifest_set_hash TEXT NOT NULL,
  git_head TEXT,
  content_hash TEXT NOT NULL,
  locked INTEGER NOT NULL DEFAULT 1,
  superseded_by TEXT,
  proof_ref TEXT,
  FOREIGN KEY(session_id) REFERENCES runtime_sessions(session_id)
);

CREATE TABLE IF NOT EXISTS runtime_plan_revisions (
  plan_revision_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  plan_id TEXT NOT NULL,
  status TEXT NOT NULL,
  plan_hash TEXT NOT NULL,
  manifest_set_hash TEXT NOT NULL,
  compiled_snapshot_id TEXT NOT NULL,
  git_head TEXT,
  payload_cbor BLOB,
  created_at TEXT NOT NULL,
  superseded_by TEXT,
  FOREIGN KEY(session_id) REFERENCES runtime_sessions(session_id)
);

CREATE TABLE IF NOT EXISTS runtime_validation_summaries (
  validation_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  command_id TEXT NOT NULL,
  structured_command_hash TEXT NOT NULL,
  exit_code INTEGER,
  stdout_hash TEXT,
  stderr_hash TEXT,
  duration_ms INTEGER,
  status TEXT NOT NULL,
  manifest_set_hash TEXT NOT NULL,
  git_head TEXT,
  recorded_at TEXT NOT NULL,
  FOREIGN KEY(session_id) REFERENCES runtime_sessions(session_id)
);

CREATE TABLE IF NOT EXISTS runtime_evidence_hints (
  evidence_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  capability_id TEXT NOT NULL,
  evidence_class TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  previous_hash TEXT,
  trust_execution_claim TEXT NOT NULL,
  trust_custody_claim TEXT NOT NULL,
  proof_ref TEXT,
  manifest_set_hash TEXT NOT NULL,
  git_head TEXT,
  recorded_at TEXT NOT NULL,
  FOREIGN KEY(session_id) REFERENCES runtime_sessions(session_id)
);

CREATE TABLE IF NOT EXISTS runtime_specsync_proposals (
  proposal_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  capability_id TEXT NOT NULL,
  changed_path TEXT NOT NULL,
  proposal_hash TEXT NOT NULL,
  status TEXT NOT NULL,
  manifest_set_hash TEXT NOT NULL,
  git_head TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY(session_id) REFERENCES runtime_sessions(session_id)
);

CREATE TABLE IF NOT EXISTS runtime_writer_leases (
  workspace_id TEXT PRIMARY KEY,
  holder_id TEXT NOT NULL,
  holder_process TEXT,
  lease_reason TEXT NOT NULL,
  acquired_at TEXT NOT NULL,
  heartbeat_at TEXT NOT NULL,
  heartbeat_counter INTEGER NOT NULL DEFAULT 0,
  expires_at TEXT,
  session_id TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS runtime_lease_observations (
  workspace_id TEXT NOT NULL,
  observer_id TEXT NOT NULL,
  observed_holder_id TEXT NOT NULL,
  observed_counter INTEGER NOT NULL,
  stable_read_count INTEGER NOT NULL DEFAULT 1,
  first_observed_at TEXT NOT NULL,
  last_observed_at TEXT NOT NULL,
  clock_regression_detected INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (workspace_id, observer_id)
);

CREATE TABLE IF NOT EXISTS runtime_session_sequences (
  session_id TEXT PRIMARY KEY,
  next_seq INTEGER NOT NULL DEFAULT 1,
  updated_at TEXT NOT NULL
);
