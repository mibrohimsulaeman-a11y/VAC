//! Canonical root seed catalog for Plan 08 hardening.
//!
//! Keep this list close to registry diagnostics so root seed coverage cannot
//! silently drift while donor-backed manifests are introduced.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootSeedCapabilityRequirement {
    pub feature: &'static str,
    pub capability_id: &'static str,
    pub expected_source_roots: &'static [&'static str],
    pub expected_surfaces: &'static [&'static str],
    pub expected_policy: &'static [&'static str],
    pub expected_validation: &'static [&'static str],
    pub expected_ownership: &'static [&'static str],
    pub current_status: &'static str,
    pub target_status: &'static str,
}

pub const ROOT_SEED_CAPABILITY_REQUIREMENTS: &[RootSeedCapabilityRequirement] = &[
    RootSeedCapabilityRequirement {
        feature: "chat",
        capability_id: "vac.chat",
        expected_source_roots: &["vac-rs/tui/src/chatwidget.rs", "vac-rs/tui/src/chatwidget/"],
        expected_surfaces: &["surface.tui", "surface.slash"],
        expected_policy: &["vac.policy.approval", "vac.policy.tools"],
        expected_validation: &["cargo test -p vac-core chat", "vac doctor registry"],
        expected_ownership: &["vac-core::chat", "vac-core::runtime"],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "approvals",
        capability_id: "vac.approvals",
        expected_source_roots: &["vac-rs/core/src/control_plane/approval_*.rs"],
        expected_surfaces: &["surface.tui", "surface.slash"],
        expected_policy: &["vac.policy.approval"],
        expected_validation: &["cargo test -p vac-core approval", "vac doctor registry"],
        expected_ownership: &["vac-core::control_plane::approval"],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "tools",
        capability_id: "vac.tools",
        expected_source_roots: &["vac-rs/core/src/tools/", "vac-rs/tools/"],
        expected_surfaces: &["surface.tui", "surface.palette", "surface.slash"],
        expected_policy: &["vac.policy.tools", "vac.policy.sandbox"],
        expected_validation: &["cargo test -p vac-core tools", "vac doctor registry"],
        expected_ownership: &["vac-core::tools", "vac-tools"],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "sandbox",
        capability_id: "vac.sandbox",
        expected_source_roots: &["vac-rs/core/src/sandboxing/", "vac-rs/sandboxing/"],
        expected_surfaces: &["surface.cli", "surface.tui"],
        expected_policy: &["vac.policy.sandbox", "vac.policy.filesystem"],
        expected_validation: &["cargo test -p vac-sandbox", "vac doctor policy"],
        expected_ownership: &["vac-sandbox", "vac-core::sandbox"],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "sessions",
        capability_id: "vac.sessions",
        expected_source_roots: &["vac-rs/core/src/session/"],
        expected_surfaces: &["surface.tui", "surface.slash", "surface.palette"],
        expected_policy: &["vac.policy.approval", "vac.default-local"],
        expected_validation: &["cargo test -p vac-core sessions", "vac doctor registry"],
        expected_ownership: &["vac-core::sessions"],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "workflow",
        capability_id: "vac.workflow",
        expected_source_roots: &[
            "vac-rs/core/src/control_plane/workflow_*.rs",
            ".vac/workflows/",
        ],
        expected_surfaces: &["surface.tui", "surface.slash", "surface.palette"],
        expected_policy: &["vac.policy.approval", "vac.policy.tools"],
        expected_validation: &[
            "cargo test -p vac-core workflow_runner",
            "vac doctor workflow",
        ],
        expected_ownership: &["vac-core::control_plane::workflow"],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "build",
        capability_id: "vac.build",
        expected_source_roots: &[
            "vac-rs/core/src/control_plane/build_check.rs",
            "vac-rs/cli/src/doctor_cli.rs",
        ],
        expected_surfaces: &["surface.cli"],
        expected_policy: &["vac.default-local"],
        expected_validation: &["vac doctor build .", "cargo build -p vac-surface-cli"],
        expected_ownership: &["vac-cli::build_cli"],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "identity",
        capability_id: "vac.identity",
        expected_source_roots: &[
            "vac-rs/cli/src/auth_cli.rs",
            "vac-rs/login/src/auth/",
            "vac-rs/agent-identity/src/lib.rs",
        ],
        expected_surfaces: &["surface.cli", "surface.tui"],
        expected_policy: &["vac.policy.approval"],
        expected_validation: &["cargo test -p vac-core identity", "vac doctor registry"],
        expected_ownership: &["vac-core::identity", "vac-cli::identity_cli"],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "identity-check",
        capability_id: "vac.identity.check",
        expected_source_roots: &[
            "vac-rs/core/src/control_plane/identity_check.rs",
            ".vac/workflows/maintenance.identity-check.yaml",
        ],
        expected_surfaces: &["surface.cli", "surface.tui"],
        expected_policy: &["vac.policy.approval"],
        expected_validation: &[
            "vac doctor identity",
            "cargo test -p vac-core identity_check",
        ],
        expected_ownership: &["vac-core::identity::check"],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "release",
        capability_id: "vac.release",
        expected_source_roots: &[
            "vac-rs/cli/src/doctor_cli.rs",
            ".vac/workflows/maintenance.release-gate.yaml",
        ],
        expected_surfaces: &["surface.cli"],
        expected_policy: &["vac.policy.approval"],
        expected_validation: &["vac doctor registry", "release-gate workflow run"],
        expected_ownership: &["vac-cli::doctor_cli"],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "tui",
        capability_id: "vac.tui",
        expected_source_roots: &["vac-rs/tui/src/"],
        expected_surfaces: &["surface.tui"],
        expected_policy: &["vac.default-local"],
        expected_validation: &["cargo test -p vac-surface-tui", "vac doctor surfaces"],
        expected_ownership: &["vac-tui"],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "ownership",
        capability_id: "vac.ownership",
        expected_source_roots: &[
            "vac-rs/core/src/control_plane/ownership_scan.rs",
            "vac-rs/cli/src/doctor_cli.rs",
        ],
        expected_surfaces: &["surface.cli", "surface.tui"],
        expected_policy: &["vac.default-local"],
        expected_validation: &[
            "vac doctor ownership",
            "cargo test -p vac-core ownership_scan",
        ],
        expected_ownership: &["vac-core::control_plane::ownership_scan"],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "architecture",
        capability_id: "vac.architecture",
        expected_source_roots: &[
            "vac-rs/core/src/control_plane/architecture_invariants.rs",
            "docs/workflow-control-plane/",
        ],
        expected_surfaces: &["surface.cli"],
        expected_policy: &["vac.default-local"],
        expected_validation: &[
            "vac doctor architecture",
            "cargo test -p vac-core architecture",
        ],
        expected_ownership: &["vac-core::control_plane::architecture"],
        current_status: "ready",
        target_status: "ready",
    },
];

pub const ROOT_SEED_POLICY_IDS: &[&str] = &[
    "vac.policy.approval",
    "vac.default-local",
    "vac.policy.filesystem",
    "vac.policy.network",
    "vac.policy.sandbox",
    "vac.policy.tools",
];

pub const ROOT_SEED_SURFACE_IDS: &[&str] = &[
    "surface.tui",
    "surface.slash",
    "surface.palette",
    "surface.cli",
];

pub const ROOT_SEED_WORKFLOW_IDS: &[&str] = &[
    "maintenance.build-check",
    "maintenance.identity-check",
    "maintenance.release-gate",
    "maintenance.ownership-scan",
    "maintenance.no-duplicate-tui",
    "maintenance.donor-migration-gate",
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn root_seed_catalog_entries_are_machine_readable() {
        let mut features = HashSet::new();
        let mut capabilities = HashSet::new();
        for entry in ROOT_SEED_CAPABILITY_REQUIREMENTS {
            assert!(!entry.feature.trim().is_empty());
            assert!(entry.capability_id.starts_with("vac."));
            assert!(
                !entry.expected_source_roots.is_empty(),
                "{} source roots",
                entry.feature
            );
            assert!(
                !entry.expected_surfaces.is_empty(),
                "{} surfaces",
                entry.feature
            );
            assert!(
                !entry.expected_policy.is_empty(),
                "{} policy",
                entry.feature
            );
            assert!(
                !entry.expected_validation.is_empty(),
                "{} validation",
                entry.feature
            );
            assert!(
                !entry.expected_ownership.is_empty(),
                "{} ownership",
                entry.feature
            );
            assert!(!entry.current_status.trim().is_empty());
            assert!(!entry.target_status.trim().is_empty());
            assert!(features.insert(entry.feature));
            assert!(capabilities.insert(entry.capability_id));
        }
    }

    #[test]
    fn root_seed_catalog_contains_plan19_canonical_count() {
        assert_eq!(ROOT_SEED_CAPABILITY_REQUIREMENTS.len(), 13);
        assert_eq!(ROOT_SEED_SURFACE_IDS.len(), 4);
        assert_eq!(ROOT_SEED_POLICY_IDS.len(), 6);
        assert_eq!(ROOT_SEED_WORKFLOW_IDS.len(), 6);
    }
}
