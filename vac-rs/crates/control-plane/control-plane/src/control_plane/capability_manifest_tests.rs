use super::CapabilityCliSurface;
use super::CapabilityCommandCollection;
use super::CapabilityCompatibilityTransport;
use super::CapabilityCompatibilityTransportStatus;
use super::CapabilityDonorCleanupStatus;
use super::CapabilityDonorSource;
use super::CapabilityManifestKind;
use super::CapabilityOwner;
use super::CapabilityOwnership;
use super::CapabilityPolicyValue;
use super::CapabilityStates;
use super::CapabilityStatus;
use super::CapabilitySurfaces;
use super::CapabilityValidation;
use super::parse_capability_manifest;
use super::validate_capability_manifest;

fn parse_ok(contents: &str) -> super::CapabilityManifest {
    parse_capability_manifest("test.yaml", contents).expect("manifest should parse")
}

fn parse_err(contents: &str) -> String {
    parse_capability_manifest("test.yaml", contents)
        .expect_err("manifest should fail")
        .to_string()
}

fn validate_err(manifest: &super::CapabilityManifest) -> String {
    validate_capability_manifest(manifest)
        .expect_err("manifest validation should fail")
        .to_string()
}

#[test]
fn parses_canonical_capability_manifest() {
    let manifest = parse_ok(
        r#"
schema_version: 1
kind: capability
id: vac.semantic_loop
title: Semantic Coding Loop
status: planned
reason: "Plan 02"
owner: vac-rs/core/local_runtime
description: Converts prompts into bounded semantic execution.
depends_on:
  - vac.chat
optional_depends_on: []
runtime_events:
  - turn.started
  - turn.completed
states:
  - empty
  - loading
  - success
  - failure
surfaces:
  tui:
    routes:
      - /activity
      - /workflow
  slash:
    commands:
      - /workflow
  palette: true
  cli:
    commands:
      - vac exec
policy:
  risk: safe_edit
  mutates_files: false
  approval_required_for:
    - write_files
validation:
  commands:
    - cargo check -p vac-surface-cli
"#,
    );

    assert_eq!(manifest.schema_version, 1);
    assert_eq!(manifest.kind, CapabilityManifestKind::Capability);
    assert_eq!(manifest.id, "vac.semantic_loop");
    assert_eq!(manifest.status, CapabilityStatus::Planned);
    assert_eq!(
        manifest.owner,
        CapabilityOwner::Path("vac-rs/core/local_runtime".to_string())
    );
    assert_eq!(
        manifest.states,
        Some(CapabilityStates::List(vec![
            "empty".to_string(),
            "loading".to_string(),
            "success".to_string(),
            "failure".to_string(),
        ]))
    );

    assert_eq!(
        manifest.surfaces,
        CapabilitySurfaces {
            tui: Some(super::CapabilityRouteCollection {
                routes: vec!["/activity".to_string(), "/workflow".to_string()],
            }),
            slash: Some(CapabilityCommandCollection {
                commands: vec!["/workflow".to_string()],
            }),
            palette: true,
            cli: Some(CapabilityCliSurface {
                enabled: true,
                commands: vec!["vac exec".to_string()],
            }),
        }
    );
    assert_eq!(manifest.policy.risk.as_deref(), Some("safe_edit"));
    assert_eq!(
        manifest.policy.mutates_files,
        Some(CapabilityPolicyValue::Bool(false))
    );
    assert_eq!(
        manifest.validation,
        CapabilityValidation {
            commands: vec!["cargo check -p vac-surface-cli".to_string()],
            gates: Vec::new(),
        }
    );
}

#[test]
fn parses_structured_validation_metadata_without_changing_command_text() {
    let manifest = parse_ok(
        r#"
schema_version: 1
kind: capability
id: vac.validation_metadata
title: Validation Metadata
status: planned
reason: "Plan metadata"
owner: vac-rs/control-plane
states:
  - ready
surfaces:
  palette: true
policy:
  risk: safe_read
validation:
  commands:
    - id: cargo.check
      runner: cargo
      args: ["check", "-p", "vac-control-plane"]
      risk: safe_read
      approval: none
      evidence: evidence.validation.metadata
      note: "metadata accepted"
      env_required: ["RUST_BACKTRACE"]
      network: false
"#,
    );

    assert_eq!(
        manifest.validation.commands,
        vec!["cargo check -p vac-control-plane".to_string()]
    );
}

#[test]
fn rejects_blank_structured_validation_metadata() {
    let err = parse_err(
        r#"
schema_version: 1
kind: capability
id: vac.validation_metadata
title: Validation Metadata
status: planned
reason: "Plan metadata"
owner: vac-rs/control-plane
states:
  - ready
surfaces:
  palette: true
policy:
  risk: safe_read
validation:
  commands:
    - runner: cargo
      args: ["check"]
      evidence: " "
"#,
    );

    assert!(err.contains("validation.commands[0].evidence"));
}

#[test]
fn parses_legacy_owner_object_and_surface_shorthands() {
    let manifest = parse_ok(
        r#"
schema_version: 1
kind: capability
id: vac.tui
title: Root TUI
status: partial
reason: "Plan 02"
states:
  - empty
owner:
  crate: vac-rs/core
  module: local_runtime
surfaces:
  tui: /activity
  slash: /workflow
  palette: false
  cli: false
policy:
  mutates_files: possible
validation:
  commands: []
"#,
    );

    assert_eq!(
        manifest.owner,
        CapabilityOwner::Structured(super::CapabilityOwnerObject {
            crate_name: "vac-rs/core".to_string(),
            module: "local_runtime".to_string(),
            team: None,
        })
    );
    assert_eq!(
        manifest.surfaces,
        CapabilitySurfaces {
            tui: Some(super::CapabilityRouteCollection {
                routes: vec!["/activity".to_string()],
            }),
            slash: Some(CapabilityCommandCollection {
                commands: vec!["/workflow".to_string()],
            }),
            palette: false,
            cli: Some(CapabilityCliSurface {
                enabled: false,
                commands: Vec::new(),
            }),
        }
    );
    assert_eq!(
        manifest.policy.mutates_files,
        Some(CapabilityPolicyValue::Text("possible".to_string()))
    );
}

#[test]
fn parses_capability_ownership_metadata() {
    let manifest = parse_ok(
        r#"
schema_version: 1
kind: capability
id: vac.ownership
title: Ownership
status: partial
reason: "Plan 02"
states:
  - empty
owner:
  crate: vac-core
  module: control_plane
ownership:
  crates:
    - vac-core
    - vac-cli
  modules:
    - control_plane
    - doctor_cli
  test_only: false
  retired: false
surfaces:
  tui:
    routes:
      - /capabilities
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
    );

    assert_eq!(
        manifest.ownership,
        Some(CapabilityOwnership {
            crates: vec!["vac-core".to_string(), "vac-cli".to_string()],
            modules: vec!["control_plane".to_string(), "doctor_cli".to_string()],
            test_only: false,
            retired: false,
            deletion_plan: None,
            targets: Vec::new(),
        })
    );
}

#[test]
fn rejects_empty_capability_ownership() {
    let err = parse_err(
        r#"
schema_version: 1
kind: capability
id: vac.ownership
title: Ownership
status: partial
reason: "Plan 02"
states:
  - empty
owner:
  crate: vac-core
  module: control_plane
ownership:
  crates: []
  modules: []
surfaces:
  tui:
    routes:
      - /capabilities
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
    );

    assert!(err.contains("ownership"));
    assert!(err.contains("at least one classic crate/module or granular target must be declared"));
}

#[test]
fn parses_donor_source_with_required_metadata() {
    let manifest = parse_ok(
        r#"
schema_version: 1
kind: capability
id: vac.release
title: Release
status: planned
reason: "Plan 02"
states:
  - empty
owner: vac-cli/doctor_cli
surfaces:
  tui:
    routes:
      - /workflow
  slash:
    commands:
      - /workflow
  palette: true
policy:
  mutates_files: false
validation:
  commands:
    - cargo check -p vac-surface-cli
donor_source:
  source: donor/vac/crates/vac_release
  root_target: vac-rs/cli/src/doctor_cli.rs
  cleanup_status: migrated
"#,
    );

    assert_eq!(
        manifest.donor_source,
        Some(CapabilityDonorSource {
            source: "donor/vac/crates/vac_release".to_string(),
            root_target: "vac-rs/cli/src/doctor_cli.rs".to_string(),
            cleanup_status: CapabilityDonorCleanupStatus::Migrated,
        })
    );
}

#[test]
fn rejects_legacy_donor_alias_field() {
    let err = parse_err(
        r#"
schema_version: 1
kind: capability
id: vac.release
title: Release
status: planned
owner: vac-cli/doctor_cli
surfaces:
  tui:
    routes:
      - /workflow
  slash:
    commands:
      - /workflow
  palette: true
policy:
  mutates_files: false
validation:
  commands:
    - cargo check -p vac-surface-cli
donor:
  source: donor/vac/crates/vac_release
  root_target: vac-rs/cli/src/doctor_cli.rs
  cleanup_status: migrated
"#,
    );

    assert!(err.contains("donor"));
    assert!(err.contains("unknown field"));
}

#[test]
fn parses_compatibility_transport_metadata() {
    let manifest = parse_ok(
        r#"
schema_version: 1
kind: capability
id: vac.tui
title: TUI
status: partial
reason: "Plan 02"
states:
  - empty
owner:
  crate: vac-surface-tui
  module: lib
surfaces:
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
compatibility_transport:
  - dependency: vac-app-server
    status: compatibility_transport
    owner: vac-tui::app_server_session
    deletion_condition: remove when the local runtime session no longer needs the adapter
    replacement: vac-rs/tui/src/local_runtime_session.rs
    target_removal_plan: Plan 00F
"#,
    );

    assert_eq!(
        manifest.compatibility_transport,
        vec![CapabilityCompatibilityTransport {
            dependency: "vac-app-server".to_string(),
            status: CapabilityCompatibilityTransportStatus::CompatibilityTransport,
            owner: "vac-tui::app_server_session".to_string(),
            deletion_condition: "remove when the local runtime session no longer needs the adapter"
                .to_string(),
            replacement: "vac-rs/tui/src/local_runtime_session.rs".to_string(),
            target_removal_plan: "Plan 00F".to_string(),
        }]
    );
}

#[test]
fn rejects_donor_source_without_root_target() {
    let err = parse_err(
        r#"
schema_version: 1
kind: capability
id: vac.release
title: Release
status: planned
reason: "Plan 02"
states:
  - empty
owner: vac-cli/doctor_cli
surfaces:
  tui:
    routes:
      - /workflow
  slash:
    commands:
      - /workflow
  palette: true
policy:
  mutates_files: false
validation:
  commands:
    - cargo check -p vac-surface-cli
donor_source:
  source: donor/vac/crates/vac_release
  cleanup_status: migrated
"#,
    );

    assert!(err.contains("donor_source.root_target"));
    assert!(err.contains("missing required field"));
}

#[test]
fn rejects_donor_source_with_invalid_cleanup_status() {
    let err = parse_err(
        r#"
schema_version: 1
kind: capability
id: vac.release
title: Release
status: planned
reason: "Plan 02"
states:
  - empty
owner: vac-cli/doctor_cli
surfaces:
  tui:
    routes:
      - /workflow
  slash:
    commands:
      - /workflow
  palette: true
policy:
  mutates_files: false
validation:
  commands:
    - cargo check -p vac-surface-cli
donor_source:
  source: donor/vac/crates/vac_release
  root_target: vac-rs/cli/src/doctor_cli.rs
  cleanup_status: archived
"#,
    );

    assert!(err.contains("donor_source.cleanup_status"));
    assert!(err.contains("donor_only, migrated, delete_candidate"));
}

#[test]
fn rejects_unknown_kind() {
    let err = parse_err(
        r#"
schema_version: 1
kind: workflow
id: vac.workflow
title: Workflow
status: planned
owner: vac-rs/core/workflow
surfaces:
  tui: /workflow
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
    );

    assert!(err.contains("kind"));
    assert!(err.contains("capability"));
}

#[test]
fn rejects_unknown_status() {
    let err = parse_err(
        r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: experimental
owner: vac-rs/core/workflow
surfaces:
  tui: /workflow
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
    );

    assert!(err.contains("status"));
    assert!(err.contains("planned"));
}

#[test]
fn rejects_missing_required_sections() {
    let err = parse_err(
        r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: planned
owner: vac-rs/core/workflow
policy:
  mutates_files: false
validation:
  commands: []
"#,
    );

    assert!(err.contains("surfaces"));
}

#[test]
fn rejects_empty_policy() {
    let err = parse_err(
        r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: planned
owner: vac-rs/core/workflow
surfaces:
  tui: /workflow
  palette: true
policy: {}
validation:
  commands: []
"#,
    );

    assert!(err.contains("policy"));
}

#[test]
fn rejects_missing_validation() {
    let err = parse_err(
        r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: planned
owner: vac-rs/core/workflow
surfaces:
  tui: /workflow
  palette: true
policy:
  mutates_files: false
"#,
    );

    assert!(err.contains("validation"));
}

#[test]
fn rejects_unknown_nested_tui_route_field() {
    let err = parse_err(
        r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: planned
owner: vac-rs/core/workflow
surfaces:
  tui:
    routes:
      - /workflow
    stray: true
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
    );

    assert!(err.contains("surfaces.tui"));
    assert!(err.contains("did not match any variant") || err.contains("unknown field"));
}

#[test]
fn rejects_unknown_nested_slash_command_field() {
    let err = parse_err(
        r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: planned
owner: vac-rs/core/workflow
surfaces:
  slash:
    commands:
      - /workflow
    stray: true
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
    );

    assert!(err.contains("surfaces.slash"));
    assert!(err.contains("did not match any variant") || err.contains("unknown field"));
}

#[test]
fn rejects_unknown_nested_cli_surface_field() {
    let err = parse_err(
        r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: planned
owner: vac-rs/core/workflow
surfaces:
  cli:
    enabled: true
    commands:
      - vac workflow
    stray: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
    );

    assert!(err.contains("surfaces.cli"));
    assert!(err.contains("did not match any variant") || err.contains("unknown field"));
}

#[test]
fn rejects_unknown_nested_state_label_field() {
    let err = parse_err(
        r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: planned
owner: vac-rs/core/workflow
states:
  empty: Empty
  loading: Loading
  success: Ready
  failure: Failed
  stray: Nope
surfaces:
  tui: /workflow
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
    );

    assert!(err.contains("states"));
    assert!(err.contains("did not match any variant") || err.contains("unknown field"));
}

#[test]
fn rejects_retired_ownership_without_deletion_plan() {
    let err = parse_err(
        r#"
schema_version: 1
kind: capability
id: vac.ownership
title: Ownership
status: partial
reason: "Plan 02"
states:
  - empty
owner:
  crate: vac-core
  module: control_plane
ownership:
  crates:
    - vac-core
  modules:
    - control_plane
  retired: true
surfaces:
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
    );

    assert!(err.contains("ownership"));
    assert!(err.contains("retired capabilities or targets must specify a non-empty deletion_plan"));
}

#[test]
fn parses_ownership_with_deletion_plan() {
    let manifest = parse_ok(
        r#"
schema_version: 1
kind: capability
id: vac.ownership
title: Ownership
status: partial
reason: "Plan 02"
states:
  - empty
owner:
  crate: vac-core
  module: control_plane
ownership:
  crates:
    - vac-core
  modules:
    - control_plane
  retired: true
  deletion_plan: "Plan 021 - cleanup old controller"
surfaces:
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
    );

    let ownership = manifest.ownership.expect("ownership should exist");
    assert!(ownership.retired);
    assert_eq!(
        ownership.deletion_plan.as_deref(),
        Some("Plan 021 - cleanup old controller")
    );
}

#[test]
fn parses_granular_ownership_targets() {
    let manifest = parse_ok(
        r#"
schema_version: 1
kind: capability
id: vac.ownership
title: Ownership
status: partial
reason: "Plan 02"
states:
  - empty
owner:
  crate: vac-core
  module: control_plane
ownership:
  targets:
    - crate_name: vac-core
      module: control_plane
      test_only: false
      retired: true
  deletion_plan: "Sunset checklist"
surfaces:
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
    );

    let ownership = manifest.ownership.expect("ownership should exist");
    assert_eq!(ownership.targets.len(), 1);
    assert_eq!(ownership.targets[0].crate_name.as_deref(), Some("vac-core"));
    assert_eq!(
        ownership.targets[0].module.as_deref(),
        Some("control_plane")
    );
    assert!(!ownership.targets[0].test_only);
    assert!(ownership.targets[0].retired);
    assert_eq!(ownership.deletion_plan.as_deref(), Some("Sunset checklist"));
}

#[test]
fn rejects_granular_ownership_target_with_invalid_retired_flags() {
    let err = parse_err(
        r#"
schema_version: 1
kind: capability
id: vac.ownership
title: Ownership
status: partial
reason: "Plan 02"
states:
  - empty
owner:
  crate: vac-core
  module: control_plane
ownership:
  targets:
    - crate_name: vac-core
      module: control_plane
      test_only: true
      retired: true
  deletion_plan: "Plan 021"
surfaces:
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
    );

    assert!(err.contains("ownership.targets[0]"));
    assert!(err.contains("test_only and retired cannot both be true"));
}

#[test]
fn parse_rejects_ready_capability_without_validation_evidence() {
    let err = parse_err(
        r#"
schema_version: 1
kind: capability
id: vac.ready
title: Ready Capability
status: ready
owner: vac-rs/core/control_plane
states:
  - empty
  - loading
  - success
  - failure
surfaces:
  tui: /capabilities
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
    );

    assert!(err.contains("validation"));
    assert!(
        err.contains("ready capabilities must declare at least one validation command or gate")
    );
}

#[test]
fn parse_rejects_non_deprecated_capability_without_states() {
    let err = parse_err(
        r#"
schema_version: 1
kind: capability
id: vac.states
title: States Capability
status: planned
reason: "Plan 02"
owner: vac-rs/core/control_plane
surfaces:
  tui: /capabilities
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
    );

    assert!(err.contains("states"));
    assert!(err.contains("non-deprecated capabilities must declare states"));
}

#[test]
fn validate_rejects_ready_capability_without_validation_evidence() {
    let mut manifest = parse_ok(
        r#"
schema_version: 1
kind: capability
id: vac.ready
title: Ready Capability
status: ready
owner: vac-rs/core/control_plane
states:
  - empty
  - loading
  - success
  - failure
surfaces:
  tui: /capabilities
  palette: true
policy:
  mutates_files: false
validation:
  commands:
    - cargo check -p vac-core
"#,
    );

    manifest.validation = CapabilityValidation {
        commands: Vec::new(),
        gates: Vec::new(),
    };

    let err = validate_err(&manifest);
    assert!(err.contains("validation"));
    assert!(
        err.contains("ready capabilities must declare at least one validation command or gate")
    );
}

#[test]
fn validate_rejects_non_deprecated_capability_without_states() {
    let mut manifest = parse_ok(
        r#"
schema_version: 1
kind: capability
id: vac.states
title: States Capability
status: ready
owner: vac-rs/core/control_plane
states:
  - empty
  - loading
  - success
  - failure
surfaces:
  tui: /capabilities
  palette: true
policy:
  mutates_files: false
validation:
  commands:
    - cargo check -p vac-core
"#,
    );

    manifest.states = None;

    let err = validate_err(&manifest);
    assert!(err.contains("states"));
    assert!(err.contains("non-deprecated capabilities must declare states"));
}

#[test]
fn validate_rejects_memory_donor_capability_without_tui_or_cli_surface() {
    let mut manifest = parse_ok(
        r#"
schema_version: 1
kind: capability
id: vac.donor
title: Donor Capability
status: ready
owner: vac-rs/core/control_plane
states:
  - empty
  - loading
  - success
  - failure
surfaces:
  tui: /capabilities
  palette: true
policy:
  mutates_files: false
validation:
  commands:
    - cargo check -p vac-core
donor_source:
  source: donor/vac/crates/vac_capability
  root_target: vac-rs/core/src/control_plane/capability_manifest.rs
  cleanup_status: migrated
"#,
    );

    manifest.surfaces = CapabilitySurfaces {
        tui: None,
        slash: Some(CapabilityCommandCollection {
            commands: vec!["/capabilities".to_string()],
        }),
        palette: true,
        cli: None,
    };

    let err = validate_err(&manifest);
    assert!(err.contains("surfaces"));
    assert!(err.contains("donor-backed capabilities must declare a TUI route or CLI command"));
}
