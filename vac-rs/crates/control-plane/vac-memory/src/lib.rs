//! VAC governed memory contracts.
//!
//! Memory is a planning hint only; it can never relax policy. This module adds
//! redaction, anti-loop failure detection, reflection records, and SQLite schema
//! constants for episodic/semantic stores without granting memory authority.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryRecord {
    pub tier: MemoryTier,
    pub id: String,
    pub source: String,
    pub summary: String,
    pub authority: MemoryAuthority,
    #[serde(default)]
    pub references: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryTier {
    Working,
    Episodic,
    Semantic,
    Team,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryAuthority {
    PlanningHintOnly,
    HumanApprovedTeamRule,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FailureObservation {
    pub fingerprint: String,
    pub command_id: String,
    pub stderr_hash: String,
    pub attempted_fix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReflectionRecord {
    pub session_id: String,
    pub root_cause: String,
    pub solution_summary: String,
    pub promoted_to_semantic: bool,
    pub evidence_ref: String,
}

pub const EPISODIC_SQLITE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS execution_sessions (
  session_id TEXT PRIMARY KEY,
  started_at TEXT NOT NULL,
  capability TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS failure_observations (
  fingerprint TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  command_id TEXT NOT NULL,
  stderr_hash TEXT NOT NULL,
  attempted_fix TEXT NOT NULL,
  created_at TEXT NOT NULL
);
"#;

pub const SEMANTIC_SQLITE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS semantic_facts (
  fact_id TEXT PRIMARY KEY,
  capability TEXT NOT NULL,
  summary TEXT NOT NULL,
  evidence_ref TEXT NOT NULL,
  created_at TEXT NOT NULL
);
CREATE VIRTUAL TABLE IF NOT EXISTS semantic_facts_fts USING fts5(summary, capability, evidence_ref);
"#;

#[must_use]
pub fn redact_memory_text(input: &str) -> String {
    if input.contains("-----BEGIN ") && input.contains("PRIVATE KEY-----") {
        return "[REDACTED_PRIVATE_KEY_BLOCK]".to_string();
    }
    input
        .split_whitespace()
        .map(|token| {
            if is_secret_like_token(token) {
                "[REDACTED]".to_string()
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[must_use]
pub fn is_secret_like_token(token: &str) -> bool {
    let trimmed = token.trim_matches(|ch: char| ch == ',' || ch == ';' || ch == '"' || ch == '\'');
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("bearer ")
        || lower.contains("authorization:bearer")
        || lower.contains("token=")
        || lower.contains("apikey")
        || lower.contains("api_key")
        || lower.contains("password=")
        || lower.contains("secret=")
        || lower.contains("aws_secret_access_key")
        || lower.contains("private_key")
    {
        return true;
    }
    if looks_like_jwt(trimmed) || looks_like_cloud_key(trimmed) || looks_high_entropy(trimmed) {
        return true;
    }
    false
}

#[must_use]
pub fn looks_like_jwt(token: &str) -> bool {
    let parts = token.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts.iter().all(|part| {
            part.len() >= 8
                && part
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
        })
}

#[must_use]
pub fn looks_like_cloud_key(token: &str) -> bool {
    token.starts_with("AKIA")
        || token.starts_with("ASIA")
        || token.starts_with("AIza")
        || token.starts_with("ghp_")
        || token.starts_with("sk-")
}

#[must_use]
pub fn looks_high_entropy(token: &str) -> bool {
    if token.len() < 32 || token.len() > 256 {
        return false;
    }
    let classes = [
        token.chars().any(|ch| ch.is_ascii_lowercase()),
        token.chars().any(|ch| ch.is_ascii_uppercase()),
        token.chars().any(|ch| ch.is_ascii_digit()),
        token.chars().any(|ch| "_-+=/".contains(ch)),
    ];
    classes.into_iter().filter(|item| *item).count() >= 3
}

#[must_use]
pub fn redact_for_memory(input: &str) -> String {
    redact_memory_text(input)
}

#[must_use]
pub fn repeated_failure_seen(history: &[FailureObservation], next: &FailureObservation) -> bool {
    history.iter().any(|item| {
        item.fingerprint == next.fingerprint && item.attempted_fix == next.attempted_fix
    })
}

#[must_use]
pub fn memory_can_relax_policy(_record: &MemoryRecord) -> bool {
    false
}

pub const MEMORY_MIGRATIONS: &[(&str, &str)] = &[
    ("001_episodic", EPISODIC_SQLITE_SCHEMA),
    ("002_semantic_fts", SEMANTIC_SQLITE_SCHEMA),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DuplicateFactDecision {
    pub duplicate: bool,
    pub existing_fact_id: Option<String>,
    pub action: String,
}

#[must_use]
pub fn duplicate_fact_decision(
    existing_summaries: &[String],
    candidate: &str,
) -> DuplicateFactDecision {
    let normalized = candidate.trim().to_ascii_lowercase();
    if let Some(existing) = existing_summaries
        .iter()
        .find(|item| item.trim().to_ascii_lowercase() == normalized)
    {
        DuplicateFactDecision {
            duplicate: true,
            existing_fact_id: Some(existing.clone()),
            action: "deduplicate_and_reference_existing".to_string(),
        }
    } else {
        DuplicateFactDecision {
            duplicate: false,
            existing_fact_id: None,
            action: "append_with_evidence_ref".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AntiLoopDecision {
    pub blocked: bool,
    pub reason: String,
}

#[must_use]
pub fn anti_loop_decision(
    history: &[FailureObservation],
    next: &FailureObservation,
) -> AntiLoopDecision {
    if repeated_failure_seen(history, next) {
        AntiLoopDecision { blocked: true, reason: "same failure fingerprint and attempted fix already observed; require new plan or operator input".to_string() }
    } else {
        AntiLoopDecision {
            blocked: false,
            reason: "no repeated failure loop detected".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryWriteProposal {
    pub tier: MemoryTier,
    pub raw_text: String,
    pub redacted_text: String,
    pub evidence_ref: String,
    pub requires_operator_approval: bool,
}

#[must_use]
pub fn propose_memory_write(
    tier: MemoryTier,
    raw_text: &str,
    evidence_ref: &str,
) -> MemoryWriteProposal {
    MemoryWriteProposal {
        tier,
        raw_text: "[redacted-before-persist]".to_string(),
        redacted_text: redact_memory_text(raw_text),
        evidence_ref: evidence_ref.to_string(),
        requires_operator_approval: matches!(tier, MemoryTier::Team),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failure(attempted_fix: &str) -> FailureObservation {
        FailureObservation {
            fingerprint: "fp".to_string(),
            command_id: "cmd".to_string(),
            stderr_hash: "sha256:stderr".to_string(),
            attempted_fix: attempted_fix.to_string(),
        }
    }

    #[test]
    fn memory_redaction_removes_secret_like_tokens() {
        let redacted = redact_for_memory("token=abcdef ghp_abcdefghijklmnopqrstuvwxyz123456");

        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("ghp_abcdefghijklmnopqrstuvwxyz123456"));
    }

    #[test]
    fn memory_never_relaxes_policy() {
        let record = MemoryRecord {
            tier: MemoryTier::Semantic,
            id: "mem".to_string(),
            source: "test".to_string(),
            summary: "summary".to_string(),
            authority: MemoryAuthority::HumanApprovedTeamRule,
            references: vec![],
        };

        assert!(!memory_can_relax_policy(&record));
    }

    #[test]
    fn anti_loop_blocks_repeated_same_fix() {
        let previous = failure("retry same thing");
        let next = failure("retry same thing");

        let decision = anti_loop_decision(&[previous], &next);

        assert!(decision.blocked);
        assert!(decision.reason.contains("require new plan"));
    }
}
