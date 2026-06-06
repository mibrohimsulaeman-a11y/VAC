use super::load_policy_registry;
use std::fs;
use tempfile::tempdir;

fn policy_manifest_yaml() -> &'static str {
    r#"
schema_version: 1
kind: policy
id: vac.default-local
title: Default Local Policy
default_decision: deny
rules:
  - id: block-git
    match:
      tool: git
    decision: deny
    reason: Block git calls in the loader fixture
"#
}

#[test]
fn loads_policy_registry_from_vac_root() {
    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    let policies_dir = vac_root.join("policies");
    fs::create_dir_all(&policies_dir).expect("create policies dir");
    fs::write(
        policies_dir.join("default-local.yaml"),
        policy_manifest_yaml(),
    )
    .expect("write policy manifest");

    let registry = load_policy_registry(tempdir.path()).expect("load policy registry");
    assert_eq!(registry.vac_root, vac_root);
    assert_eq!(registry.manifest_dir, policies_dir);
    assert_eq!(registry.manifests.len(), 1);
    assert_eq!(registry.manifests[0].manifest.id, "vac.default-local");
}

#[test]
fn policy_registry_reports_cross_family_collisions() {
    use super::super::registry::RegistryLoadError;
    use super::super::registry::load_control_plane_registry;
    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    let policies_dir = vac_root.join("policies");
    let capabilities_dir = vac_root.join("capabilities");
    fs::create_dir_all(&policies_dir).expect("create policies dir");
    fs::create_dir_all(&capabilities_dir).expect("create capabilities dir");

    // Write a policy manifest
    fs::write(
        policies_dir.join("default-local.yaml"),
        policy_manifest_yaml(), // id: vac.default-local
    )
    .expect("write policy manifest");

    // Write a capability manifest with the same ID
    let capability_yaml = r#"
schema_version: 1
kind: capability
id: vac.default-local
title: Default Local Policy
status: partial
reason: Registry collision fixture
states:
  - empty
owner: vac-rs/tui
surfaces:
  tui: /activity
  slash: /activity
  palette: true
  cli: false
policy:
  mutates_files: false
validation:
  commands: []
"#;
    fs::write(capabilities_dir.join("default-local.yaml"), capability_yaml)
        .expect("write capability manifest");

    let error = load_control_plane_registry(tempdir.path()).expect_err("expected load collision");
    match error {
        RegistryLoadError::Aggregate { errors, .. } => {
            let has_duplicate = errors.iter().any(|e| matches!(e, RegistryLoadError::DuplicateManifestId { id, .. } if id == "vac.default-local"));
            assert!(
                has_duplicate,
                "expected duplicate error for vac.default-local, got: {errors:?}"
            );
        }
        RegistryLoadError::DuplicateManifestId { id, .. } => {
            assert_eq!(id, "vac.default-local");
        }
        other => panic!("expected aggregate or duplicate error, got: {other:?}"),
    }
}

#[test]
fn evaluate_multi_manifest_policy_precedence_global_fallback() {
    use super::super::policy_manifest::PolicyAction;
    use super::super::policy_manifest::PolicyManifest;
    use super::super::policy_manifest::PolicyPathScope;
    use super::super::policy_manifest::WorkflowStepExecutionIntent;
    use super::super::policy_manifest::evaluate_step_policy;

    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    let policies_dir = vac_root.join("policies");
    fs::create_dir_all(&policies_dir).expect("create policies dir");

    // Write manifest_A (approval fallback)
    let yaml_a = r#"
schema_version: 1
kind: policy
id: vac.approval-policy
title: Approval Policy
default_decision: approval_required
rules: []
"#;
    fs::write(policies_dir.join("approval-policy.yaml"), yaml_a).expect("write approval policy");

    // Write manifest_B (default deny)
    let yaml_b = r#"
schema_version: 1
kind: policy
id: vac.deny-policy
title: Deny Policy
default_decision: deny
rules: []
"#;
    fs::write(policies_dir.join("deny-policy.yaml"), yaml_b).expect("write deny policy");

    let registry = load_policy_registry(tempdir.path()).expect("load policy registry");
    let manifests: Vec<PolicyManifest> = registry
        .manifests
        .iter()
        .map(|e| e.manifest.clone())
        .collect();
    assert_eq!(manifests.len(), 2);

    let intent = WorkflowStepExecutionIntent {
        action: PolicyAction::FilesystemRead,
        path_scope: Some(PolicyPathScope::Project),
        data_class: None,
        tool: None,
        network_scope: None,
        requires_approval_inherently: false,
        step_uses: "capability.docs.read".to_string(),
        capability_id: None,
    };

    let report = evaluate_step_policy(&intent, &manifests);
    // Since no rules matched in either manifest, the default decisions are gathered.
    // ApprovalRequired and Deny are gathered. Deny must take precedence globally.
    assert!(
        report.is_blocked(),
        "expected block report due to global-fallback deny from deny-policy, got: {report:?}"
    );
}

#[test]
fn evaluate_multi_manifest_policy_applies_per_manifest_fallback() {
    use super::super::policy_manifest::PolicyAction;
    use super::super::policy_manifest::PolicyDecisionReport;
    use super::super::policy_manifest::PolicyManifest;
    use super::super::policy_manifest::PolicyPathScope;
    use super::super::policy_manifest::WorkflowStepExecutionIntent;
    use super::super::policy_manifest::evaluate_step_policy;

    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    let policies_dir = vac_root.join("policies");
    fs::create_dir_all(&policies_dir).expect("create policies dir");

    let default_deny_yaml = r#"
schema_version: 1
kind: policy
id: vac.default-deny
title: Default Deny
default_decision: deny
rules: []
"#;
    fs::write(policies_dir.join("default-deny.yaml"), default_deny_yaml)
        .expect("write default deny policy");

    let scoped_allow_yaml = r#"
schema_version: 1
kind: policy
id: vac.scoped-allow
title: Scoped Allow
default_decision: deny
rules:
  - id: allow-project-read
    match:
      action: filesystem_read
      path: project
    decision: allow
    reason: Allow project-scoped read fixture
"#;
    fs::write(policies_dir.join("scoped-allow.yaml"), scoped_allow_yaml)
        .expect("write scoped allow policy");

    let registry = load_policy_registry(tempdir.path()).expect("load policy registry");
    let manifests: Vec<PolicyManifest> = registry
        .manifests
        .iter()
        .map(|e| e.manifest.clone())
        .collect();
    assert_eq!(manifests.len(), 2);

    let intent = WorkflowStepExecutionIntent {
        action: PolicyAction::FilesystemRead,
        path_scope: Some(PolicyPathScope::Project),
        data_class: None,
        tool: None,
        network_scope: None,
        requires_approval_inherently: false,
        step_uses: "capability.docs.read".to_string(),
        capability_id: None,
    };

    let report = evaluate_step_policy(&intent, &manifests);
    match report {
        PolicyDecisionReport::Block { reasons } => {
            assert!(
                reasons
                    .iter()
                    .any(|reason| reason.contains("[vac.default-deny] default")),
                "expected default deny to contribute across manifests, got: {reasons:?}"
            );
        }
        other => panic!("expected block from per-manifest default deny fallback, got: {other:?}"),
    }
}
