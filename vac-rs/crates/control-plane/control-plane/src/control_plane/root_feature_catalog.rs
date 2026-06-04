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
        expected_source_roots: &[
            "vac-rs/crates/surfaces/tui/src/chatwidget.rs",
            "vac-rs/crates/surfaces/tui/src/chatwidget/",
        ],
        expected_surfaces: &["surface.tui", "surface.slash"],
        expected_policy: &["vac.policy.approval", "vac.policy.tools"],
        expected_validation: &["cargo +1.93.0 check -p vac-surface-tui --tests"],
        expected_ownership: &["vac-capability-chat", "vac-surface-tui::chatwidget"],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "approvals",
        capability_id: "vac.approvals",
        expected_source_roots: &[
            "vac-rs/crates/control-plane/control-plane/src/control_plane/approval_*.rs",
            "vac-rs/crates/surfaces/tui/src/bottom_pane/approval_overlay.rs",
        ],
        expected_surfaces: &["surface.tui", "surface.slash"],
        expected_policy: &["vac.policy.approval"],
        expected_validation: &[
            "cargo test --manifest-path vac-rs/Cargo.toml -p vac-core approval -- --nocapture",
            "vac doctor policy .",
        ],
        expected_ownership: &[
            "vac-control-plane::control_plane.approval_lifecycle",
            "vac-surface-tui::bottom_pane.approval_overlay",
        ],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "tools",
        capability_id: "vac.tools",
        expected_source_roots: &[
            "vac-rs/crates/capabilities/tools-domain/src/",
            "vac-rs/crates/capabilities/tools/src/",
            "vac-rs/crates/runtime/shell/apply-patch/src/",
        ],
        expected_surfaces: &["surface.tui", "surface.palette", "surface.slash"],
        expected_policy: &["vac.policy.tools", "vac.policy.sandbox"],
        expected_validation: &["cargo +1.93.0 check -p vac-core"],
        expected_ownership: &["vac-capability-tools", "vac-tools"],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "sandbox",
        capability_id: "vac.sandbox",
        expected_source_roots: &[
            "vac-rs/crates/runtime/sandbox/",
            "vac-rs/crates/runtime/exec/execpolicy/src/",
        ],
        expected_surfaces: &["surface.cli", "surface.tui"],
        expected_policy: &["vac.policy.sandbox", "vac.policy.filesystem"],
        expected_validation: &["cargo +1.93.0 check -p vac-core"],
        expected_ownership: &["vac-core::sandboxing", "vac-sandboxing"],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "sessions",
        capability_id: "vac.sessions",
        expected_source_roots: &[
            "vac-rs/crates/capabilities/sessions/src/",
            "vac-rs/crates/foundation/thread-store/src/",
        ],
        expected_surfaces: &["surface.tui", "surface.slash", "surface.palette"],
        expected_policy: &["vac.policy.approval", "vac.default-local"],
        expected_validation: &["cargo +1.93.0 check -p vac-core"],
        expected_ownership: &["vac-capability-sessions", "vac-protocol"],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "workflow",
        capability_id: "vac.workflow",
        expected_source_roots: &[
            "vac-rs/crates/control-plane/control-plane/src/control_plane/workflow_*.rs",
            ".vac/workflows/",
        ],
        expected_surfaces: &["surface.tui", "surface.slash", "surface.palette"],
        expected_policy: &["vac.policy.approval", "vac.policy.tools"],
        expected_validation: &["cargo +1.93.0 check -p vac-surface-cli"],
        expected_ownership: &[
            "vac-control-plane::control_plane",
            "vac-surface-cli::workflow_cli",
        ],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "build",
        capability_id: "vac.build",
        expected_source_roots: &[
            "vac-rs/crates/control-plane/control-plane/src/control_plane/build_check.rs",
            "vac-rs/crates/surfaces/cli/src/doctor_cli.rs",
        ],
        expected_surfaces: &["surface.cli"],
        expected_policy: &["vac.default-local"],
        expected_validation: &[
            "vac doctor build .",
            "cargo +1.93.0 check --manifest-path vac-rs/Cargo.toml -p vac-surface-cli",
        ],
        expected_ownership: &[
            "vac-control-plane::control_plane.build_check",
            "vac-surface-cli::doctor_cli",
        ],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "identity",
        capability_id: "vac.identity",
        expected_source_roots: &[
            "vac-rs/crates/surfaces/cli/src/auth_cli.rs",
            "vac-rs/crates/providers/login/src/auth/",
            "vac-rs/crates/providers/agent-identity/src/lib.rs",
            "vac-rs/crates/providers/provider-http/src/",
        ],
        expected_surfaces: &["surface.tui", "surface.palette"],
        expected_policy: &["vac.policy.approval"],
        expected_validation: &["cargo +1.93.0 check -p vac-surface-cli"],
        expected_ownership: &["vac-capability-identity", "vac-surface-cli::auth_cli"],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "identity-check",
        capability_id: "vac.identity.check",
        expected_source_roots: &[
            "vac-rs/crates/control-plane/control-plane/src/control_plane/identity_check.rs",
            ".vac/workflows/maintenance.identity-check.yaml",
        ],
        expected_surfaces: &["surface.tui", "surface.palette"],
        expected_policy: &["vac.policy.approval"],
        expected_validation: &["cargo +1.93.0 check -p vac-surface-cli"],
        expected_ownership: &["vac-control-plane::control_plane.identity_check"],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "release",
        capability_id: "vac.release",
        expected_source_roots: &[
            "vac-rs/crates/surfaces/cli/src/doctor_cli.rs",
            "vac-rs/crates/capabilities/release/src/",
            ".vac/workflows/maintenance.release-gate.yaml",
        ],
        expected_surfaces: &["surface.cli"],
        expected_policy: &["vac.policy.approval"],
        expected_validation: &["cargo +1.93.0 check -p vac-surface-cli"],
        expected_ownership: &["vac-capability-release", "vac-surface-cli::doctor_cli"],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "tui",
        capability_id: "vac.tui",
        expected_source_roots: &["vac-rs/crates/surfaces/tui/src/"],
        expected_surfaces: &["surface.tui"],
        expected_policy: &["vac.default-local"],
        expected_validation: &["cargo +1.93.0 check -p vac-surface-tui --tests"],
        expected_ownership: &["vac-surface-tui"],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "ownership",
        capability_id: "vac.ownership",
        expected_source_roots: &[
            "vac-rs/crates/control-plane/control-plane/src/control_plane/ownership_scan.rs",
            "vac-rs/crates/surfaces/cli/src/doctor_cli.rs",
            "vac-rs/crates/capabilities/ownership/src/",
        ],
        expected_surfaces: &["surface.cli", "surface.tui"],
        expected_policy: &["vac.default-local"],
        expected_validation: &["cargo +1.93.0 check -p vac-surface-cli"],
        expected_ownership: &[
            "vac-capability-ownership",
            "vac-control-plane::control_plane.ownership_scan",
        ],
        current_status: "ready",
        target_status: "ready",
    },
    RootSeedCapabilityRequirement {
        feature: "architecture",
        capability_id: "vac.architecture",
        expected_source_roots: &[
            "vac-rs/crates/control-plane/control-plane/src/control_plane/architecture_invariants.rs",
        ],
        expected_surfaces: &["surface.cli"],
        expected_policy: &["vac.default-local"],
        expected_validation: &[
            "cargo +1.93.0 test -p vac-core architecture_invariants -- --nocapture",
            "cargo +1.93.0 check -p vac-surface-cli",
        ],
        expected_ownership: &["vac-control-plane::control_plane.architecture_invariants"],
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
