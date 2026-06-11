-- VAC episodic memory SQLite schema.
-- Authority: local advisory memory only; cannot relax VAC policy or readiness gates.
CREATE TABLE IF NOT EXISTS episodic_events (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    event_kind TEXT NOT NULL,
    summary TEXT NOT NULL,
    evidence_ref TEXT,
    created_at TEXT NOT NULL,
    redaction_applied INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_episodic_events_session ON episodic_events(session_id, created_at);
CREATE TABLE IF NOT EXISTS episodic_failures (
    id TEXT PRIMARY KEY,
    fingerprint TEXT NOT NULL,
    occurrence_count INTEGER NOT NULL DEFAULT 1,
    last_seen_at TEXT NOT NULL,
    policy_relaxation_allowed INTEGER NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_episodic_failures_fingerprint ON episodic_failures(fingerprint);
