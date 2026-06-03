use super::load_control_plane_registry_report;
use std::fs;
use tempfile::tempdir;

fn capability_manifest_yaml(id: &str) -> String {
    format!(
        r#"
schema_version: 1
kind: capability
id: {id}
title: {id}
status: partial
reason: Registry test fixture
states:
  - empty
owner:
  crate: vac-core
  module: control_plane
surfaces:
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#
    )
}

fn workflow_manifest_yaml(id: &str, uses: &str) -> String {
    format!(
        r#"
schema_version: 1
kind: workflow
id: {id}
title: {id}
status: planned
inputs: {{}}
steps:
  - id: one
    uses: {uses}
ui:
  surface: /workflow
  progress_panel: true
  activity_log: true
policy:
  default_risk: safe_edit
validation:
  commands: []
"#
    )
}

fn policy_manifest_yaml(id: &str) -> String {
    format!(
        r#"
schema_version: 1
kind: policy
id: {id}
title: {id}
default_decision: deny
rules:
  - id: block-shell
    match:
      tool: shell
    decision: deny
    reason: Block shell in registry fixture
"#
    )
}

#[test]
fn full_registry_reports_all_manifest_parse_errors() {
    let tempdir = tempdir().expect("tempdir");
    let capabilities_dir = tempdir.path().join(".vac/capabilities");
    fs::create_dir_all(&capabilities_dir).expect("capabilities dir");
    fs::write(
        capabilities_dir.join("broken-a.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.broken_a
status: partial
owner: vac-rs/core
surfaces:
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
    )
    .expect("broken a");
    fs::write(
        capabilities_dir.join("broken-b.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.broken_b
status: partial
owner: vac-rs/core
surfaces:
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
    )
    .expect("broken b");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();
    assert!(report.is_failure(), "rendered:\n{rendered}");
    assert!(
        rendered.contains("broken-a.yaml:title"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("broken-b.yaml:title"),
        "rendered:\n{rendered}"
    );
}

#[test]
fn full_registry_reports_cross_family_collisions_in_one_pass() {
    let tempdir = tempdir().expect("tempdir");
    fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
    fs::create_dir_all(tempdir.path().join(".vac/workflows")).expect("workflows dir");
    fs::create_dir_all(tempdir.path().join(".vac/policies")).expect("policies dir");
    fs::write(
        tempdir.path().join(".vac/capabilities/shared.yaml"),
        capability_manifest_yaml("vac.shared"),
    )
    .expect("capability");
    fs::write(
        tempdir.path().join(".vac/workflows/shared.yaml"),
        workflow_manifest_yaml("vac.shared", "capability.shared"),
    )
    .expect("workflow");
    fs::write(
        tempdir.path().join(".vac/policies/shared.yaml"),
        policy_manifest_yaml("vac.shared"),
    )
    .expect("policy");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();
    let duplicate_count = rendered.matches("duplicate manifest id").count();
    assert!(report.is_failure(), "rendered:\n{rendered}");
    assert!(duplicate_count >= 2, "rendered:\n{rendered}");
    assert!(
        rendered.contains("workflows/shared.yaml"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("policies/shared.yaml"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("vac.shared") && rendered.contains("previously loaded from"),
        "rendered:\n{rendered}"
    );
}

#[test]
fn full_registry_validates_workflow_step_uses_after_load() {
    let tempdir = tempdir().expect("tempdir");
    fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
    fs::create_dir_all(tempdir.path().join(".vac/workflows")).expect("workflows dir");
    fs::write(
        tempdir.path().join(".vac/capabilities/known.yaml"),
        capability_manifest_yaml("vac.known"),
    )
    .expect("capability");
    fs::write(
        tempdir.path().join(".vac/workflows/typo.yaml"),
        workflow_manifest_yaml("product.typo", "capability.typo"),
    )
    .expect("workflow");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();
    assert!(report.is_failure(), "rendered:\n{rendered}");
    assert!(
        rendered.contains("steps[0].uses")
            && rendered.contains("workflow vocabulary or a declared capability id"),
        "rendered:\n{rendered}"
    );
}

#[test]
fn full_registry_accepts_vocabulary_alias_and_exact_capability_uses() {
    let tempdir = tempdir().expect("tempdir");
    fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
    fs::create_dir_all(tempdir.path().join(".vac/workflows")).expect("workflows dir");
    fs::write(
        tempdir.path().join(".vac/capabilities/known.yaml"),
        capability_manifest_yaml("vac.known"),
    )
    .expect("capability");
    fs::write(
        tempdir.path().join(".vac/workflows/vocabulary.yaml"),
        workflow_manifest_yaml("product.vocabulary", "capability.root_seed.coverage"),
    )
    .expect("vocabulary workflow");
    fs::write(
        tempdir.path().join(".vac/workflows/alias.yaml"),
        workflow_manifest_yaml("product.alias", "capability.known"),
    )
    .expect("alias workflow");
    fs::write(
        tempdir.path().join(".vac/workflows/exact.yaml"),
        workflow_manifest_yaml("product.exact", "vac.known"),
    )
    .expect("exact workflow");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();

    assert!(
        !rendered.contains("workflow vocabulary or a declared capability id"),
        "rendered:\n{rendered}"
    );
    assert!(
        !rendered.contains("step references an unknown capability"),
        "rendered:\n{rendered}"
    );
}

#[test]
fn duplicate_manifest_id_error_includes_details() {
    let tempdir = tempdir().expect("tempdir");
    let capabilities_dir = tempdir.path().join(".vac/capabilities");
    fs::create_dir_all(&capabilities_dir).expect("capabilities dir");
    fs::write(
        capabilities_dir.join("cap-a.yaml"),
        capability_manifest_yaml("vac.dup"),
    )
    .expect("cap a");
    fs::write(
        capabilities_dir.join("cap-b.yaml"),
        capability_manifest_yaml("vac.dup"),
    )
    .expect("cap b");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();
    assert!(report.is_failure(), "rendered:\n{rendered}");
    assert!(
        rendered.contains("duplicate manifest id `vac.dup`"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("previously loaded from"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("cap-a.yaml") && rendered.contains("cap-b.yaml"),
        "rendered:\n{rendered}"
    );
}

#[test]
fn workflow_step_uses_validation_after_load_regression() {
    let tempdir = tempdir().expect("tempdir");
    fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
    fs::create_dir_all(tempdir.path().join(".vac/workflows")).expect("workflows dir");

    fs::write(
        tempdir.path().join(".vac/capabilities/known.yaml"),
        capability_manifest_yaml("vac.known"),
    )
    .expect("capability");

    let workflow_yaml = r#"
schema_version: 1
kind: workflow
id: product.regression
title: Regression Workflow
status: planned
inputs: {}
steps:
  - id: step_ok_cap
    uses: vac.known
  - id: step_ok_vocab
    uses: capability.root_seed.coverage
  - id: step_fail_a
    uses: capability.unknown_a
  - id: step_fail_b
    uses: vac.unknown_b
ui:
  surface: /workflow
  progress_panel: true
  activity_log: true
policy:
  default_risk: safe_edit
validation:
  commands: []
"#;

    fs::write(
        tempdir.path().join(".vac/workflows/regression.yaml"),
        workflow_yaml,
    )
    .expect("workflow");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();
    assert!(report.is_failure(), "rendered:\n{rendered}");

    assert!(
        rendered.contains("steps[2].uses")
            && rendered.contains(
                "step uses must resolve to workflow vocabulary or a declared capability id"
            ),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("steps[3].uses")
            && rendered.contains(
                "step uses must resolve to workflow vocabulary or a declared capability id"
            ),
        "rendered:\n{rendered}"
    );

    assert!(!rendered.contains("steps[0].uses"), "rendered:\n{rendered}");
    assert!(!rendered.contains("steps[1].uses"), "rendered:\n{rendered}");
}
