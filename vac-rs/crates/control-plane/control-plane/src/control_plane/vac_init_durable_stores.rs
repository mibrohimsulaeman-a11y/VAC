#![allow(dead_code)]
//! VAC-Init durable store contracts.
//!
//! Production Hardening D makes persistence explicit for init state, source
//! inventory, ownership reports, risk findings, approvals, trajectory, and
//! memory. The concrete stores in later runtime code must use the same
//! fail-closed path and atomic-write invariants captured here.

use std::fmt;
use std::path::Path;
use std::path::PathBuf;

pub const DURABLE_STORES_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DurableStoreKind {
    InitState,
    SourceInventory,
    OwnershipReport,
    RiskFinding,
    Approval,
    TrajectoryIndex,
    TrajectoryFile,
    MemoryRecord,
}

impl DurableStoreKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InitState => "init_state",
            Self::SourceInventory => "source_inventory",
            Self::OwnershipReport => "ownership_report",
            Self::RiskFinding => "risk_finding",
            Self::Approval => "approval",
            Self::TrajectoryIndex => "trajectory_index",
            Self::TrajectoryFile => "trajectory_file",
            Self::MemoryRecord => "memory_record",
        }
    }

    pub const fn default_relative_path(self) -> &'static str {
        match self {
            Self::InitState => ".vac/.init/state.yaml",
            Self::SourceInventory => ".vac/.init/source_inventory.yaml",
            Self::OwnershipReport => ".vac/registry/ownership/report.yaml",
            Self::RiskFinding => ".vac/.init/risk_findings.yaml",
            Self::Approval => ".vac/registry/approvals/<approval-id>.yaml",
            Self::TrajectoryIndex => ".vac/registry/trajectory/index.yaml",
            Self::TrajectoryFile => ".vac/registry/trajectory/<capability>/<file>.trajectory.yaml",
            Self::MemoryRecord => ".vac/registry/memory/<tier>/<record-id>.yaml",
        }
    }

    pub const fn expected_kind(self) -> &'static str {
        match self {
            Self::InitState => "init_state",
            Self::SourceInventory => "registry_status",
            Self::OwnershipReport => "ownership_report",
            Self::RiskFinding => "risk_finding",
            Self::Approval => "approval_request",
            Self::TrajectoryIndex | Self::TrajectoryFile => "trajectory",
            Self::MemoryRecord => "memory_record",
        }
    }
}

impl fmt::Display for DurableStoreKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreEnvelope {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
}

impl StoreEnvelope {
    pub fn valid_for(&self, store_kind: DurableStoreKind) -> bool {
        self.schema_version == DURABLE_STORES_SCHEMA_VERSION
            && self.kind == store_kind.expected_kind()
            && is_dotted_id(&self.id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomicStoreWritePlan {
    pub store_kind: DurableStoreKind,
    pub final_path: PathBuf,
    pub temp_path: PathBuf,
    pub create_parent_dirs: bool,
    pub fsync_parent: bool,
}

impl AtomicStoreWritePlan {
    pub fn new(
        store_kind: DurableStoreKind,
        workspace_root: impl AsRef<Path>,
        relative_path: &str,
    ) -> Result<Self, DurableStoreError> {
        validate_relative_store_path(relative_path)?;
        let final_path = workspace_root.as_ref().join(relative_path);
        let mut temp_path = final_path.clone();
        let extension = final_path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("tmp");
        temp_path.set_extension(format!("{extension}.tmp"));
        Ok(Self {
            store_kind,
            final_path,
            temp_path,
            create_parent_dirs: true,
            fsync_parent: true,
        })
    }

    pub fn uses_temp_then_rename(&self) -> bool {
        self.final_path != self.temp_path
            && self
                .temp_path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.ends_with(".tmp"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurableStoreError {
    AbsolutePath,
    ParentTraversal,
    BackslashPath,
    EmptyPath,
    MissingEnvelope,
    EnvelopeMismatch {
        expected_kind: &'static str,
        actual_kind: String,
    },
}

impl fmt::Display for DurableStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AbsolutePath => f.write_str("store path must be workspace-relative"),
            Self::ParentTraversal => f.write_str("store path must not contain parent traversal"),
            Self::BackslashPath => f.write_str("store path must use forward slashes"),
            Self::EmptyPath => f.write_str("store path must not be empty"),
            Self::MissingEnvelope => {
                f.write_str("store record requires schema_version/kind/id envelope")
            }
            Self::EnvelopeMismatch {
                expected_kind,
                actual_kind,
            } => write!(
                f,
                "store envelope kind mismatch: expected `{expected_kind}`, got `{actual_kind}`"
            ),
        }
    }
}

pub fn validate_relative_store_path(path: &str) -> Result<(), DurableStoreError> {
    let value = path.trim();
    if value.is_empty() {
        return Err(DurableStoreError::EmptyPath);
    }
    if value.starts_with('/') {
        return Err(DurableStoreError::AbsolutePath);
    }
    if value.contains('\\') {
        return Err(DurableStoreError::BackslashPath);
    }
    if value.split('/').any(|part| part == "..") {
        return Err(DurableStoreError::ParentTraversal);
    }
    Ok(())
}

pub fn validate_store_envelope(
    store_kind: DurableStoreKind,
    envelope: Option<&StoreEnvelope>,
) -> Result<(), DurableStoreError> {
    let Some(envelope) = envelope else {
        return Err(DurableStoreError::MissingEnvelope);
    };
    if !envelope.valid_for(store_kind) {
        return Err(DurableStoreError::EnvelopeMismatch {
            expected_kind: store_kind.expected_kind(),
            actual_kind: envelope.kind.clone(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalReplayBinding {
    pub approval_id: String,
    pub status: String,
    pub plan_hash: String,
    pub diff_hash: String,
    pub policy_snapshot_hash: String,
    pub nonce: String,
    pub expires_at_unix: u64,
}

impl ApprovalReplayBinding {
    pub fn validate_for(
        &self,
        now_unix: u64,
        plan_hash: &str,
        diff_hash: &str,
        policy_snapshot_hash: &str,
        seen_nonces: &[String],
    ) -> Result<(), ApprovalReplayError> {
        if self.status != "approved" {
            return Err(ApprovalReplayError::NotApproved);
        }
        if now_unix > self.expires_at_unix {
            return Err(ApprovalReplayError::Expired);
        }
        if self.plan_hash != plan_hash {
            return Err(ApprovalReplayError::PlanHashChanged);
        }
        if self.diff_hash != diff_hash {
            return Err(ApprovalReplayError::DiffHashChanged);
        }
        if self.policy_snapshot_hash != policy_snapshot_hash {
            return Err(ApprovalReplayError::PolicyHashChanged);
        }
        if seen_nonces.iter().any(|nonce| nonce == &self.nonce) {
            return Err(ApprovalReplayError::NonceReused);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalReplayError {
    NotApproved,
    Expired,
    PlanHashChanged,
    DiffHashChanged,
    PolicyHashChanged,
    NonceReused,
}

fn is_dotted_id(value: &str) -> bool {
    value == "init.state"
        || (value.contains('.')
            && !value.starts_with('.')
            && !value.ends_with('.')
            && value.split('.').all(|part| {
                !part.is_empty()
                    && part
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
            }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durable_store_paths_are_workspace_relative() {
        assert_eq!(
            validate_relative_store_path("/tmp/state.yaml"),
            Err(DurableStoreError::AbsolutePath)
        );
        assert_eq!(
            validate_relative_store_path(".vac/../secret.yaml"),
            Err(DurableStoreError::ParentTraversal)
        );
        assert_eq!(
            validate_relative_store_path(".vac\\state.yaml"),
            Err(DurableStoreError::BackslashPath)
        );
        assert!(validate_relative_store_path(".vac/.init/state.yaml").is_ok());
    }

    #[test]
    fn atomic_write_plan_uses_tmp_then_rename() {
        let plan = AtomicStoreWritePlan::new(
            DurableStoreKind::InitState,
            "/workspace",
            ".vac/.init/state.yaml",
        )
        .unwrap();
        assert!(plan.uses_temp_then_rename());
        assert!(plan.create_parent_dirs);
        assert!(plan.fsync_parent);
    }

    #[test]
    fn store_envelope_must_match_expected_kind() {
        let ok = StoreEnvelope {
            schema_version: 1,
            kind: "init_state".to_string(),
            id: "init.state".to_string(),
        };
        assert!(validate_store_envelope(DurableStoreKind::InitState, Some(&ok)).is_ok());

        let wrong = StoreEnvelope {
            schema_version: 1,
            kind: "registry_status".to_string(),
            id: "init.state".to_string(),
        };
        assert_eq!(
            validate_store_envelope(DurableStoreKind::InitState, Some(&wrong)),
            Err(DurableStoreError::EnvelopeMismatch {
                expected_kind: "init_state",
                actual_kind: "registry_status".to_string()
            })
        );
    }

    #[test]
    fn every_durable_store_has_expected_path_and_kind() {
        for kind in [
            DurableStoreKind::InitState,
            DurableStoreKind::SourceInventory,
            DurableStoreKind::OwnershipReport,
            DurableStoreKind::RiskFinding,
            DurableStoreKind::Approval,
            DurableStoreKind::TrajectoryIndex,
            DurableStoreKind::TrajectoryFile,
            DurableStoreKind::MemoryRecord,
        ] {
            assert!(!kind.default_relative_path().is_empty());
            assert!(!kind.expected_kind().is_empty());
        }
    }

    #[test]
    fn approval_replay_binding_rejects_drift_and_reuse() {
        let binding = ApprovalReplayBinding {
            approval_id: "approval.test".to_string(),
            status: "approved".to_string(),
            plan_hash: "plan".to_string(),
            diff_hash: "diff".to_string(),
            policy_snapshot_hash: "policy".to_string(),
            nonce: "nonce-1".to_string(),
            expires_at_unix: 200,
        };
        assert!(
            binding
                .validate_for(100, "plan", "diff", "policy", &[])
                .is_ok()
        );
        assert_eq!(
            binding.validate_for(201, "plan", "diff", "policy", &[]),
            Err(ApprovalReplayError::Expired)
        );
        assert_eq!(
            binding.validate_for(100, "plan2", "diff", "policy", &[]),
            Err(ApprovalReplayError::PlanHashChanged)
        );
        assert_eq!(
            binding.validate_for(100, "plan", "diff", "policy", &["nonce-1".to_string()]),
            Err(ApprovalReplayError::NonceReused)
        );
    }
}
