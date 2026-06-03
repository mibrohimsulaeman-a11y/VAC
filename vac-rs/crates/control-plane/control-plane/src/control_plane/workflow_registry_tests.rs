use super::load_workflow_registry;
use super::load_workflow_registry_with_known_capabilities;
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

fn workflow_manifest_yaml() -> &'static str {
    r#"
schema_version: 1
kind: workflow
id: product.semantic-coding-loop
title: Semantic coding loop
status: planned
inputs: {}
steps:
  - id: intent
    uses: capability.intent.parse
ui:
  surface: /activity
  progress_panel: true
  activity_log: true
policy:
  default_risk: safe_edit
validation:
  commands: []
"#
}

#[test]
fn loads_workflow_registry_from_vac_root() {
    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    let workflows_dir = vac_root.join("workflows");
    fs::create_dir_all(&workflows_dir).expect("create workflows dir");
    fs::write(
        workflows_dir.join("semantic-coding-loop.yaml"),
        workflow_manifest_yaml(),
    )
    .expect("write workflow manifest");
    fs::write(workflows_dir.join("README.md"), "ignored").expect("write readme");

    let registry = load_workflow_registry(tempdir.path()).expect("load workflow registry");
    assert_eq!(registry.vac_root, vac_root);
    assert_eq!(registry.manifest_dir, workflows_dir);
    assert_eq!(registry.manifests.len(), 1);
    assert_eq!(
        registry.manifests[0].manifest.id,
        "product.semantic-coding-loop"
    );
}

#[test]
fn reports_workflow_manifest_errors_with_field_paths() {
    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    let workflows_dir = vac_root.join("workflows");
    fs::create_dir_all(&workflows_dir).expect("create workflows dir");
    fs::write(
        workflows_dir.join("broken.yaml"),
        r#"
schema_version: 1
kind: workflow
id: product.semantic-coding-loop
title: Semantic coding loop
status: planned
inputs: {}
steps:
  - id: intent
    uses: capability.intent.parse
ui:
  surface: workflow
  progress_panel: true
  activity_log: true
policy:
  default_risk: safe_edit
validation:
  commands: []
"#,
    )
    .expect("write invalid workflow manifest");

    let error = load_workflow_registry(tempdir.path()).expect_err("expected registry error");
    assert_eq!(error.path(), workflows_dir.join("broken.yaml").as_path());
    assert_eq!(error.field_path(), Some("ui.surface"));
}

#[test]
fn known_capability_loader_rejects_unknown_step_uses() {
    let tempdir = tempdir().expect("tempdir");
    let workflows_dir = tempdir.path().join(".vac/workflows");
    fs::create_dir_all(&workflows_dir).expect("create workflows dir");
    fs::write(
        workflows_dir.join("workflow.yaml"),
        workflow_manifest_yaml(),
    )
    .expect("write workflow manifest");

    let known = HashSet::new();
    let error = load_workflow_registry_with_known_capabilities(tempdir.path(), &known)
        .expect_err("unknown capability should fail");

    assert_eq!(error.path(), workflows_dir.join("workflow.yaml").as_path());
    assert_eq!(error.field_path(), Some("steps[0].uses"));
    assert!(error.message().contains("unknown capability"));
}

#[test]
fn known_capability_loader_accepts_alias_to_declared_capability() {
    let tempdir = tempdir().expect("tempdir");
    let workflows_dir = tempdir.path().join(".vac/workflows");
    fs::create_dir_all(&workflows_dir).expect("create workflows dir");
    fs::write(
        workflows_dir.join("workflow.yaml"),
        workflow_manifest_yaml(),
    )
    .expect("write workflow manifest");

    let known = HashSet::from(["vac.intent.parse".to_string()]);
    let registry = load_workflow_registry_with_known_capabilities(tempdir.path(), &known)
        .expect("alias should resolve to known capability");

    assert_eq!(registry.manifests.len(), 1);
}
