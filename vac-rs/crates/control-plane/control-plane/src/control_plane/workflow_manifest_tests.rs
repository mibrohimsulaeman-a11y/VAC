use std::collections::HashSet;

use super::*;

const SAMPLE_PATH: &str = "fixtures/sample.workflow.yaml";

fn sample_manifest_yaml() -> String {
    r#"schema_version: 1
kind: workflow
id: product.sample
title: Sample workflow
status: ready
inputs:
  prompt:
    type: string
    required: true
  autopilot:
    type: boolean
    required: false
    default: false
steps:
  - id: parse_intent
    uses: capability.intent.parse
    when: prompt and not autopilot
    policy:
      default_risk: low
    ui:
      surface: /workflow/intent
  - id: orient_repo
    uses: capability.repo.orient
    policy:
      mutates_files: false
ui:
  surface: /workflow/sample
  progress_panel: true
  activity_log: true
policy:
  default_risk: low
  mutates_files: false
  approval_required_for:
    - sensitive
validation:
  commands:
    - cargo nextest run -p vac-core --lib
  gates:
    - parse_intent
"#
    .to_string()
}

#[test]
fn parses_typed_workflow_manifest() {
    let manifest = parse_workflow_manifest(SAMPLE_PATH, &sample_manifest_yaml()).expect("parse");
    assert_eq!(manifest.id, "product.sample");
    assert_eq!(manifest.steps[0].uses, "capability.intent.parse");
    assert_eq!(
        manifest.steps[0].when.as_deref(),
        Some("prompt and not autopilot")
    );
    assert_eq!(manifest.validation.gates, vec!["parse_intent".to_string()]);
}

#[test]
fn rejects_unknown_kind() {
    let yaml = sample_manifest_yaml().replace("kind: workflow", "kind: garbage");
    parse_workflow_manifest(SAMPLE_PATH, &yaml).expect_err("kind");
}

#[test]
fn rejects_unsupported_schema_version() {
    let yaml = sample_manifest_yaml().replace("schema_version: 1", "schema_version: 99");
    let err = parse_workflow_manifest(SAMPLE_PATH, &yaml).expect_err("schema");
    assert!(err.message().contains("unsupported schema"));
}

#[test]
fn rejects_missing_ui() {
    let yaml = sample_manifest_yaml().replace(
        "ui:\n  surface: /workflow/sample\n  progress_panel: true\n  activity_log: true\n",
        "",
    );
    parse_workflow_manifest(SAMPLE_PATH, &yaml).expect_err("missing ui");
}

#[test]
fn rejects_invalid_step_ui_surface() {
    let yaml = sample_manifest_yaml().replace("/workflow/intent", "workflow/intent");
    let err = parse_workflow_manifest(SAMPLE_PATH, &yaml).expect_err("surface");
    assert!(err.message().contains("must start with"));
}

#[test]
fn rejects_unknown_step_field_command() {
    let yaml = sample_manifest_yaml().replace(
        "uses: capability.intent.parse",
        "uses: capability.intent.parse\n    command: echo hi",
    );
    parse_workflow_manifest(SAMPLE_PATH, &yaml).expect_err("unknown field");
}

#[test]
fn rejects_invalid_when_expression() {
    let yaml = sample_manifest_yaml().replace(
        "when: prompt and not autopilot",
        "when: prompt && autopilot",
    );
    let err = parse_workflow_manifest(SAMPLE_PATH, &yaml).expect_err("when");
    assert!(err.message().contains("invalid condition"));
}

#[test]
fn rejects_unknown_capability_when_resolved_against_registry() {
    let manifest = parse_workflow_manifest(SAMPLE_PATH, &sample_manifest_yaml()).expect("parse");
    let known = HashSet::new();
    let err = validate_workflow_manifest_against_known_capabilities(SAMPLE_PATH, &manifest, &known)
        .expect_err("unknown capability");
    assert!(err.message().contains("unknown capability"));
}

#[test]
fn rejects_ready_without_validation() {
    let yaml = sample_manifest_yaml().replace(
        "validation:\n  commands:\n    - cargo nextest run -p vac-core --lib\n  gates:\n    - parse_intent\n",
        "validation:\n  commands: []\n  gates: []\n",
    );
    let err = parse_workflow_manifest(SAMPLE_PATH, &yaml).expect_err("validation");
    assert!(err.message().contains("ready workflow"));
}

#[test]
fn rejects_non_capability_step_ref() {
    let yaml =
        sample_manifest_yaml().replace("uses: capability.intent.parse", "uses: tool.intent.parse");
    let err = parse_workflow_manifest(SAMPLE_PATH, &yaml).expect_err("uses prefix");
    assert!(err.message().contains("capability."));
}

#[test]
fn accepts_validation_gate_that_is_not_a_step_id() {
    let yaml = sample_manifest_yaml().replace("- parse_intent", "- ghost_step");
    let manifest = parse_workflow_manifest(SAMPLE_PATH, &yaml).expect("external gate");
    assert_eq!(manifest.validation.gates, vec!["ghost_step".to_string()]);
}

#[test]
fn parses_structured_validation_metadata_without_changing_command_text() {
    let yaml = sample_manifest_yaml().replace(
        "validation:\n  commands:\n    - cargo nextest run -p vac-core --lib\n  gates:\n    - parse_intent\n",
        "validation:\n  commands:\n    - id: cargo.nextest\n      runner: cargo\n      args: [\"nextest\", \"run\", \"-p\", \"vac-core\", \"--lib\"]\n      risk: safe_read\n      approval: none\n      evidence: evidence.workflow.metadata\n      note: metadata accepted\n      env_required: [RUST_BACKTRACE]\n      network: false\n  gates:\n    - parse_intent\n",
    );
    let manifest = parse_workflow_manifest(SAMPLE_PATH, &yaml).expect("structured validation");
    assert_eq!(
        manifest.validation.commands,
        vec!["cargo nextest run -p vac-core --lib".to_string()]
    );
}

#[test]
fn rejects_blank_structured_validation_metadata() {
    let yaml = sample_manifest_yaml().replace(
        "validation:\n  commands:\n    - cargo nextest run -p vac-core --lib\n  gates:\n    - parse_intent\n",
        "validation:\n  commands:\n    - runner: cargo\n      args: [\"check\"]\n      note: \" \"\n  gates:\n    - parse_intent\n",
    );
    let err = parse_workflow_manifest(SAMPLE_PATH, &yaml).expect_err("blank metadata");
    assert!(
        err.message()
            .contains("validation entry 0 must not be empty")
    );
}

// --- Plan 03 additions --------------------------------------------------------

#[test]
fn parses_policy_block_with_all_fields() {
    let yaml = sample_manifest_yaml().replace(
        "policy:\n  default_risk: low\n  mutates_files: false\n  approval_required_for:\n    - sensitive\n",
        "policy:\n  default_risk: medium\n  mutates_files: true\n  network: blocked\n  redaction: false\n  approval_required_for:\n    - sensitive\n    - destructive\n",
    );
    let manifest = parse_workflow_manifest(SAMPLE_PATH, &yaml).expect("policy parse");
    assert_eq!(manifest.policy.default_risk.as_deref(), Some("medium"));
    assert_eq!(
        manifest.policy.mutates_files,
        Some(WorkflowPolicyValue::Bool(true))
    );
    assert_eq!(
        manifest.policy.network,
        Some(WorkflowPolicyValue::Text("blocked".to_string()))
    );
    assert_eq!(
        manifest.policy.redaction,
        Some(WorkflowPolicyValue::Bool(false))
    );
    assert_eq!(
        manifest.policy.approval_required_for,
        vec!["sensitive".to_string(), "destructive".to_string()]
    );
}

#[test]
fn condition_parser_accepts_keyword_operators() {
    let yaml = sample_manifest_yaml().replace(
        "when: prompt and not autopilot",
        "when: prompt and (autopilot or not draft)",
    );
    let manifest = parse_workflow_manifest(SAMPLE_PATH, &yaml).expect("condition");
    assert_eq!(
        manifest.steps[0].when.as_deref(),
        Some("prompt and (autopilot or not draft)")
    );
}

#[test]
fn condition_parser_rejects_legacy_symbolic_operators() {
    let yaml =
        sample_manifest_yaml().replace("when: prompt and not autopilot", "when: prompt && other");
    let err = parse_workflow_manifest(SAMPLE_PATH, &yaml).expect_err("symbolic");
    assert!(err.message().contains("invalid condition"));
}

#[test]
fn condition_parser_rejects_equality_operators() {
    let yaml =
        sample_manifest_yaml().replace("when: prompt and not autopilot", "when: prompt == \"\"");
    let err = parse_workflow_manifest(SAMPLE_PATH, &yaml).expect_err("equality");
    assert!(err.message().contains("invalid condition"));
}

#[test]
fn validates_uses_against_vocabulary_with_union_known_set() {
    let yaml = sample_manifest_yaml()
        .replace(
            "uses: capability.intent.parse",
            "uses: capability.identity.check",
        )
        .replace(
            "uses: capability.repo.orient",
            "uses: capability.root_seed.coverage",
        );
    let manifest = parse_workflow_manifest(SAMPLE_PATH, &yaml).expect("parse");
    let known: HashSet<String> = HashSet::new();
    validate_workflow_manifest_against_known_capabilities(SAMPLE_PATH, &manifest, &known)
        .expect("vocabulary entries should be accepted even with empty known set");
}

#[test]
fn validates_exact_declared_capability_id_when_known() {
    let yaml =
        sample_manifest_yaml().replace("uses: capability.intent.parse", "uses: vac.intent.parse");
    let manifest = parse_workflow_manifest(SAMPLE_PATH, &yaml).expect("parse exact capability id");
    let known = HashSet::from([
        "vac.intent.parse".to_string(),
        "vac.repo.orient".to_string(),
    ]);

    validate_workflow_manifest_against_known_capabilities(SAMPLE_PATH, &manifest, &known)
        .expect("exact declared capability id should be accepted");
}

#[test]
fn validates_capability_check_suffix_against_declared_capability_id() {
    let yaml = sample_manifest_yaml()
        .replace(
            "uses: capability.intent.parse",
            "uses: capability.vac.intent.parse.check",
        )
        .replace(
            "uses: capability.repo.orient",
            "uses: capability.vac.repo.orient.check",
        );
    let manifest = parse_workflow_manifest(SAMPLE_PATH, &yaml).expect("parse check suffix");
    let known = HashSet::from([
        "vac.intent.parse".to_string(),
        "vac.repo.orient".to_string(),
    ]);

    validate_workflow_manifest_against_known_capabilities(SAMPLE_PATH, &manifest, &known)
        .expect("capability check suffix should resolve to the declared capability id");
}

#[test]
fn validates_action_suffix_and_underscore_against_declared_capability_id() {
    let yaml = sample_manifest_yaml()
        .replace(
            "uses: capability.intent.parse",
            "uses: capability.tui.operator_live_adapter.snapshot",
        )
        .replace(
            "uses: capability.repo.orient",
            "uses: capability.docs.validate_control_plane_projection",
        );
    let manifest = parse_workflow_manifest(SAMPLE_PATH, &yaml).expect("parse action suffix");
    let known = HashSet::from([
        "vac.tui.operator-live-adapter".to_string(),
        "vac.docs".to_string(),
    ]);

    validate_workflow_manifest_against_known_capabilities(SAMPLE_PATH, &manifest, &known)
        .expect("action suffixes should resolve to declared capability ids");
}

#[test]
fn rejects_invalid_step_id_grammar() {
    let yaml = sample_manifest_yaml().replace("id: parse_intent", "id: ParseIntent");
    let err = parse_workflow_manifest(SAMPLE_PATH, &yaml).expect_err("step id grammar");
    assert!(err.message().contains("lowercase ascii letter"));
}

#[test]
fn rejects_invalid_validation_gate_grammar() {
    let yaml = sample_manifest_yaml().replace("- parse_intent", "- parse.intent");
    let err = parse_workflow_manifest(SAMPLE_PATH, &yaml).expect_err("gate grammar");
    assert!(err.message().contains("validation gates may contain only"));
}
