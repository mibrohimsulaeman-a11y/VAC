-- VAC semantic memory SQLite schema.
-- Authority: local advisory facts only; memory cannot override compiled policy, approval, or readiness.
CREATE TABLE IF NOT EXISTS semantic_facts (
    id TEXT PRIMARY KEY,
    namespace TEXT NOT NULL,
    subject TEXT NOT NULL,
    predicate TEXT NOT NULL,
    object TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 0.5,
    evidence_ref TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    redaction_applied INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_semantic_facts_lookup ON semantic_facts(namespace, subject, predicate);
CREATE TABLE IF NOT EXISTS semantic_embeddings (
    fact_id TEXT PRIMARY KEY REFERENCES semantic_facts(id) ON DELETE CASCADE,
    model TEXT NOT NULL,
    vector_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);
