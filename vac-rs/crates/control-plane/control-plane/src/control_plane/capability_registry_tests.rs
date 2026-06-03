use super::load_capability_registry;
use std::fs;
use tempfile::tempdir;

fn capability_manifest_yaml() -> &'static str {
    r#"
schema_version: 1
kind: capability
id: vac.activity
title: Activity
status: partial
reason: "Plan 02"
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
"#
}

#[test]
fn loads_capability_registry_from_vac_root() {
    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    let capabilities_dir = vac_root.join("capabilities");
    fs::create_dir_all(&capabilities_dir).expect("create capabilities dir");
    fs::write(
        capabilities_dir.join("activity.yaml"),
        capability_manifest_yaml(),
    )
    .expect("write capability manifest");
    fs::write(capabilities_dir.join("README.md"), "ignored").expect("write readme");

    let registry = load_capability_registry(tempdir.path()).expect("load capability registry");
    assert_eq!(registry.vac_root, vac_root);
    assert_eq!(registry.manifest_dir, capabilities_dir);
    assert_eq!(registry.manifests.len(), 1);
    assert_eq!(registry.manifests[0].manifest.id, "vac.activity");
}

#[test]
fn reports_capability_manifest_errors_with_field_paths() {
    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    let capabilities_dir = vac_root.join("capabilities");
    fs::create_dir_all(&capabilities_dir).expect("create capabilities dir");
    fs::write(
        capabilities_dir.join("broken.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.activity
status: partial
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
"#,
    )
    .expect("write invalid manifest");

    let error = load_capability_registry(tempdir.path()).expect_err("expected registry error");
    assert_eq!(error.path(), capabilities_dir.join("broken.yaml").as_path());
    assert_eq!(error.field_path(), Some("title"));
}

#[test]
fn capability_registry_reports_cross_family_collisions() {
    use super::super::registry::RegistryLoadError;
    use super::super::registry::load_control_plane_registry;
    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    let capabilities_dir = vac_root.join("capabilities");
    let workflows_dir = vac_root.join("workflows");
    fs::create_dir_all(&capabilities_dir).expect("create capabilities dir");
    fs::create_dir_all(&workflows_dir).expect("create workflows dir");

    // Write a capability manifest
    fs::write(
        capabilities_dir.join("activity.yaml"),
        capability_manifest_yaml(), // id: vac.activity
    )
    .expect("write capability manifest");

    // Write a workflow manifest with the same ID
    let workflow_yaml = r#"
schema_version: 1
kind: workflow
id: vac.activity
title: Activity
status: planned
inputs: {}
steps:
  - id: one
    uses: vac.activity
ui:
  surface: /activity
  progress_panel: false
  activity_log: false
policy:
  default_risk: safe_edit
validation:
  commands: []
"#;
    fs::write(workflows_dir.join("activity.yaml"), workflow_yaml).expect("write workflow manifest");

    let error = load_control_plane_registry(tempdir.path()).expect_err("expected load collision");
    match error {
        RegistryLoadError::Aggregate { errors, .. } => {
            let has_duplicate = errors.iter().any(|e| matches!(e, RegistryLoadError::DuplicateManifestId { id, .. } if id == "vac.activity"));
            assert!(
                has_duplicate,
                "expected duplicate error for vac.activity, got: {:?}",
                errors
            );
        }
        RegistryLoadError::DuplicateManifestId { id, .. } => {
            assert_eq!(id, "vac.activity");
        }
        other => panic!("expected aggregate or duplicate error, got: {:?}", other),
    }
}

#[test]
fn capability_registry_reports_cross_family_collision_with_policy_manifest() {
    use super::super::registry::RegistryLoadError;
    use super::super::registry::load_control_plane_registry;
    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    let capabilities_dir = vac_root.join("capabilities");
    let policies_dir = vac_root.join("policies");
    fs::create_dir_all(&capabilities_dir).expect("create capabilities dir");
    fs::create_dir_all(&policies_dir).expect("create policies dir");

    fs::write(
        capabilities_dir.join("activity.yaml"),
        capability_manifest_yaml(), // id: vac.activity
    )
    .expect("write capability manifest");

    let policy_yaml = r#"
schema_version: 1
kind: policy
id: vac.activity
title: Activity Policy
default_decision: deny
rules: []
"#;
    fs::write(policies_dir.join("activity.yaml"), policy_yaml).expect("write policy manifest");

    let error = load_control_plane_registry(tempdir.path()).expect_err("expected load collision");
    match error {
        RegistryLoadError::Aggregate { errors, .. } => {
            let has_duplicate = errors.iter().any(|e| matches!(e, RegistryLoadError::DuplicateManifestId { id, .. } if id == "vac.activity"));
            assert!(
                has_duplicate,
                "expected duplicate error for vac.activity, got: {:?}",
                errors
            );
        }
        RegistryLoadError::DuplicateManifestId { id, .. } => {
            assert_eq!(id, "vac.activity");
        }
        other => panic!("expected aggregate or duplicate error, got: {:?}", other),
    }
}
