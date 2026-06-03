#![allow(dead_code)]
//! Memory governance contract for VAC-Init.
//!
//! The contract keeps memory tiered, size-bounded, TTL-aware, and explicitly
//! forbids credential/secret-like content in every tier.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemoryTier {
    Working,
    Episodic,
    Semantic,
    Team,
}

impl MemoryTier {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::Episodic => "episodic",
            Self::Semantic => "semantic",
            Self::Team => "team",
        }
    }

    pub const fn max_size_bytes(self) -> usize {
        match self {
            Self::Working => 64 * 1024,
            Self::Episodic => 1024 * 1024,
            Self::Semantic => 10 * 1024 * 1024,
            Self::Team => 1024 * 1024,
        }
    }

    pub const fn default_ttl_days(self) -> Option<u16> {
        match self {
            Self::Working => None,
            Self::Episodic => Some(7),
            Self::Semantic => None,
            Self::Team => None,
        }
    }

    pub const fn agent_can_write_without_approval(self) -> bool {
        matches!(self, Self::Working | Self::Episodic)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemorySource {
    Agent,
    Operator,
    Scan,
    Workflow,
    HumanAdmin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemoryContentType {
    Fact,
    Rule,
    Concept,
    Event,
    Decision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryContent {
    pub content_type: MemoryContentType,
    pub text: String,
    pub files: Vec<String>,
    pub capabilities: Vec<String>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryAccess {
    pub readable_by: Vec<String>,
    pub writable_by: Vec<String>,
    pub deletable_by: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRecord {
    pub id: String,
    pub tier: MemoryTier,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub source: MemorySource,
    pub content: MemoryContent,
    pub access: MemoryAccess,
    pub approval_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryGovernanceError {
    InvalidId(String),
    EmptyContent(String),
    SizeExceeded {
        tier: MemoryTier,
        size: usize,
        max: usize,
    },
    CredentialLikeContent(String),
    MissingTeamApproval(String),
    InvalidAccess(String),
    InvalidReference(String),
    WorkingMemoryPersistence(String),
}

pub fn memory_record_size(record: &MemoryRecord) -> usize {
    record.id.len()
        + record.created_at.len()
        + record.expires_at.as_ref().map(|v| v.len()).unwrap_or(0)
        + record.content.text.len()
        + record.content.files.iter().map(String::len).sum::<usize>()
        + record
            .content
            .capabilities
            .iter()
            .map(String::len)
            .sum::<usize>()
        + record
            .content
            .evidence
            .iter()
            .map(String::len)
            .sum::<usize>()
}

pub fn contains_credential_like_content(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let suspicious = [
        "password=",
        "passwd=",
        "api_key",
        "apikey",
        "secret=",
        "client_secret",
        "private_key",
        "access_token",
        "refresh_token",
        "bearer ",
        "-----begin private key-----",
    ];
    suspicious.iter().any(|needle| lower.contains(needle))
}

fn validate_mem_id(tier: MemoryTier, id: &str) -> Result<(), MemoryGovernanceError> {
    let expected = format!("mem.{}.", tier.as_str());
    if !id.starts_with(&expected) {
        return Err(MemoryGovernanceError::InvalidId(format!(
            "memory id must start with {}",
            expected
        )));
    }
    if !id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
    {
        return Err(MemoryGovernanceError::InvalidId(format!(
            "memory id contains invalid characters: {}",
            id
        )));
    }
    Ok(())
}

fn validate_refs(record: &MemoryRecord) -> Result<(), MemoryGovernanceError> {
    for capability in &record.content.capabilities {
        if !capability.starts_with("vac.") {
            return Err(MemoryGovernanceError::InvalidReference(format!(
                "capability ref must start with vac.: {}",
                capability
            )));
        }
    }
    for evidence in &record.content.evidence {
        if !evidence.starts_with("evidence.") {
            return Err(MemoryGovernanceError::InvalidReference(format!(
                "evidence ref must start with evidence.: {}",
                evidence
            )));
        }
    }
    for file in &record.content.files {
        if file.starts_with('/') || file.contains("..") || file.contains('\\') {
            return Err(MemoryGovernanceError::InvalidReference(format!(
                "file ref must be normalized workspace-relative: {}",
                file
            )));
        }
    }
    Ok(())
}

pub fn validate_memory_access(record: &MemoryRecord) -> Result<(), MemoryGovernanceError> {
    if !record
        .access
        .readable_by
        .iter()
        .any(|v| v == "agent" || v == "operator")
    {
        return Err(MemoryGovernanceError::InvalidAccess(
            "memory record must be readable by agent or operator".to_string(),
        ));
    }
    if record.tier == MemoryTier::Team && record.access.writable_by.iter().any(|v| v == "agent") {
        return Err(MemoryGovernanceError::InvalidAccess(
            "team memory must not be directly writable by agent".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_memory_record(record: &MemoryRecord) -> Result<(), MemoryGovernanceError> {
    validate_mem_id(record.tier, &record.id)?;
    if record.content.text.trim().is_empty() {
        return Err(MemoryGovernanceError::EmptyContent(record.id.clone()));
    }
    if contains_credential_like_content(&record.content.text) {
        return Err(MemoryGovernanceError::CredentialLikeContent(
            record.id.clone(),
        ));
    }
    let size = memory_record_size(record);
    let max = record.tier.max_size_bytes();
    if size > max {
        return Err(MemoryGovernanceError::SizeExceeded {
            tier: record.tier,
            size,
            max,
        });
    }
    if record.tier == MemoryTier::Working && record.expires_at.is_none() {
        return Err(MemoryGovernanceError::WorkingMemoryPersistence(
            "working memory must be task-bounded with expires_at/session cleanup marker"
                .to_string(),
        ));
    }
    if record.tier == MemoryTier::Team {
        match (&record.source, &record.approval_ref) {
            (MemorySource::HumanAdmin, Some(approval)) if approval.starts_with("approval.") => {}
            (MemorySource::Operator, Some(approval)) if approval.starts_with("approval.") => {}
            _ => {
                return Err(MemoryGovernanceError::MissingTeamApproval(
                    "team memory writes require human/admin source and approval ref".to_string(),
                ));
            }
        }
    }
    validate_refs(record)?;
    validate_memory_access(record)?;
    Ok(())
}

pub fn may_write_memory(
    tier: MemoryTier,
    source: MemorySource,
    approval_ref: Option<&str>,
) -> bool {
    match tier {
        MemoryTier::Working | MemoryTier::Episodic => {
            matches!(
                source,
                MemorySource::Agent | MemorySource::Workflow | MemorySource::Operator
            )
        }
        MemoryTier::Semantic => {
            matches!(
                source,
                MemorySource::Scan | MemorySource::Workflow | MemorySource::Operator
            ) && approval_ref
                .map(|v| v.starts_with("approval."))
                .unwrap_or(false)
        }
        MemoryTier::Team => {
            matches!(source, MemorySource::HumanAdmin | MemorySource::Operator)
                && approval_ref
                    .map(|v| v.starts_with("approval."))
                    .unwrap_or(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn access() -> MemoryAccess {
        MemoryAccess {
            readable_by: vec!["agent".to_string(), "operator".to_string()],
            writable_by: vec!["operator".to_string()],
            deletable_by: vec!["operator".to_string()],
        }
    }

    fn record(tier: MemoryTier) -> MemoryRecord {
        MemoryRecord {
            id: format!("mem.{}.abc", tier.as_str()),
            tier,
            created_at: "2026-05-29T00:00:00Z".to_string(),
            expires_at: Some("2026-05-30T00:00:00Z".to_string()),
            source: MemorySource::Operator,
            content: MemoryContent {
                content_type: MemoryContentType::Rule,
                text: "Ledger mutation must be append-only.".to_string(),
                files: vec!["vac-rs/core/src/ledger.rs".to_string()],
                capabilities: vec!["vac.financial.ledger".to_string()],
                evidence: vec!["evidence.2026-05-29-ledger".to_string()],
            },
            access: access(),
            approval_ref: Some("approval.abc".to_string()),
        }
    }

    #[test]
    fn validates_team_memory_with_approval() {
        assert!(validate_memory_record(&record(MemoryTier::Team)).is_ok());
    }

    #[test]
    fn rejects_team_memory_without_approval() {
        let mut rec = record(MemoryTier::Team);
        rec.approval_ref = None;
        assert!(validate_memory_record(&rec).is_err());
    }

    #[test]
    fn rejects_credential_like_content() {
        let mut rec = record(MemoryTier::Episodic);
        rec.content.text = "api_key=super-secret".to_string();
        assert!(matches!(
            validate_memory_record(&rec),
            Err(MemoryGovernanceError::CredentialLikeContent(_))
        ));
    }

    #[test]
    fn rejects_unbounded_working_memory() {
        let mut rec = record(MemoryTier::Working);
        rec.expires_at = None;
        assert!(validate_memory_record(&rec).is_err());
    }

    #[test]
    fn rejects_agent_writable_team_memory() {
        let mut rec = record(MemoryTier::Team);
        rec.access.writable_by = vec!["agent".to_string()];
        assert!(validate_memory_record(&rec).is_err());
    }

    #[test]
    fn write_policy_requires_approval_for_team() {
        assert!(!may_write_memory(
            MemoryTier::Team,
            MemorySource::Agent,
            None
        ));
        assert!(may_write_memory(
            MemoryTier::Team,
            MemorySource::HumanAdmin,
            Some("approval.mem")
        ));
    }

    #[test]
    fn write_policy_requires_approval_for_semantic() {
        assert!(!may_write_memory(
            MemoryTier::Semantic,
            MemorySource::Scan,
            None
        ));
        assert!(may_write_memory(
            MemoryTier::Semantic,
            MemorySource::Scan,
            Some("approval.semantic")
        ));
    }
}
