//! VAC root facade crate.
//!
//! The v1.9 workspace keeps implementation crates under `crates/**`, but
//! exposes this small facade so downstream code and tests have a stable
//! `vac_core` boundary. It intentionally contains no broker/OS sandbox
//! authority and must not claim stronger than L1 cooperative execution.

use serde::{Deserialize, Serialize};

pub use vac_brand::{BINARY_NAME, CONFIG_DIR as CONTROL_PLANE_DIR, PRODUCT_NAME};
pub use vac_control_plane::{CompiledCapability, SchemaEnvelope, WorkspaceStatus};
pub use vac_readiness::{ReadinessLevel, ReadinessTriplet as ReadinessState};
pub use vac_runtime_jobs::{RuntimeJobRecord as RuntimeJob, RuntimeJobsSnapshot};
pub use vac_types::{CapabilityId, EvidenceId, PlanId, Sha256Hex, sha256_hex};

/// Execution enforcement level for user-facing honesty labels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementLevel {
    /// VAC-managed local discipline and drift detection only. Direct local
    /// shell/Git/SQLite/filesystem bypass remains possible.
    ObservedL1,
    /// Reserved for future broker/OS-mediated execution. Do not emit this
    /// without verified broker mediation and external key custody.
    MediatedL2,
}

/// Return the effective VAC product identity without consulting inherited brands.
pub fn product_identity() -> &'static str {
    PRODUCT_NAME
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_identity_is_vac() {
        assert_eq!(product_identity(), "VAC");
        assert_eq!(CONTROL_PLANE_DIR, ".vac");
        assert_eq!(BINARY_NAME, "vac");
    }

    #[test]
    fn default_surface_is_l1_honest() {
        let level = EnforcementLevel::ObservedL1;
        assert_eq!(serde_json::to_string(&level).unwrap(), "\"observed_l1\"");
    }
}
