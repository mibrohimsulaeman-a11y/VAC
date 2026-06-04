//! Production-readiness contract crate for the `ownership` VAC capability domain.
//!
//! The heavy legacy implementation is currently reached through `vac-core`
//! compatibility exports while ownership, policy, evidence, and surface metadata
//! are represented here as the stable capability boundary. This keeps the
//! domain addressable by cargo, doctor gates, and `.vac` manifests before the
//! final removal of legacy compatibility includes.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityContract {
    pub id: &'static str,
    pub owner: &'static str,
    pub surface: &'static str,
    pub policy: &'static str,
    pub evidence: &'static str,
    pub status: &'static str,
}

pub const CAPABILITY: CapabilityContract = CapabilityContract {
    id: "ownership",
    owner: "vac.capabilities.ownership",
    surface: ".vac/capabilities/ownership.yaml",
    policy: ".vac/policies/default-local.yaml",
    evidence: ".vac/registry/capability-readiness.yaml",
    status: "ready",
};

pub const fn readiness_contract() -> CapabilityContract {
    CAPABILITY
}

/// Direct ownership-domain workspace classifier exported without `vac-core` path bridges.
#[path = "core_migrated/project_workspace.rs"]
pub mod project_workspace;
