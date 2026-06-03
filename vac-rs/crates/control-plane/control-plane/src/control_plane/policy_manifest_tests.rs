use super::PolicyAction;
use super::PolicyDecision;
use super::PolicyManifestError;
use super::PolicyManifestKind;
use super::PolicyPathScope;
use super::parse_policy_manifest;
use super::validate_policy_manifest;
use vac_protocol::approvals::NetworkApprovalProtocol;

fn parse_ok(contents: &str) -> super::PolicyManifest {
    parse_policy_manifest("test.yaml", contents).expect("policy manifest should parse")
}

fn parse_err(contents: &str) -> PolicyManifestError {
    parse_policy_manifest("test.yaml", contents).expect_err("policy manifest should fail")
}

#[test]
fn parses_typed_policy_manifest() {
    let manifest = parse_ok(
        r#"
schema_version: 1
kind: policy
id: vac.default-local
title: Default local policy
default_decision: approval_required
rules:
  - id: read-project
    match:
      action: filesystem_read
      path: project
    decision: allow
    reason: Read-only project inspection is allowed.
  - id: write-project
    match:
      action: filesystem_write
      path: project
    decision: approval_required
    reason: File writes require operator approval.
  - id: deny-secrets
    match:
      data_class: secret_like
    decision: deny
    reason: Secret-like values cannot be exposed.
"#,
    );

    assert_eq!(manifest.kind, PolicyManifestKind::Policy);
    assert_eq!(manifest.default_decision, PolicyDecision::ApprovalRequired);
    assert_eq!(manifest.rules.len(), 3);
    assert_eq!(
        manifest.rules[0].match_.action,
        Some(PolicyAction::FilesystemRead)
    );
    assert_eq!(
        manifest.rules[0].match_.path,
        Some(PolicyPathScope::Project)
    );
    assert_eq!(manifest.rules[2].decision, PolicyDecision::Deny);
}

#[test]
fn rejects_unknown_kind() {
    let err = parse_err(
        r#"
schema_version: 1
kind: capability
id: vac.default-local
title: Default local policy
default_decision: approval_required
rules: []
"#,
    );

    assert!(err.to_string().contains("kind"));
}

#[test]
fn rejects_unsupported_schema_version() {
    let err = parse_err(
        r#"
schema_version: 2
kind: policy
id: vac.default-local
title: Default local policy
default_decision: approval_required
rules: []
"#,
    );

    assert!(err.to_string().contains("schema_version"));
}

#[test]
fn rejects_missing_rule_reason() {
    let err = parse_err(
        r#"
schema_version: 1
kind: policy
id: vac.default-local
title: Default local policy
default_decision: approval_required
rules:
  - id: read-project
    match:
      action: filesystem_read
      path: project
    decision: allow
"#,
    );

    assert!(err.to_string().contains("reason"));
}

#[test]
fn rejects_invalid_decision() {
    let err = parse_err(
        r#"
schema_version: 1
kind: policy
id: vac.default-local
title: Default local policy
default_decision: approval_required
rules:
  - id: read-project
    match:
      action: filesystem_read
      path: project
    decision: maybe
    reason: Read-only project inspection is allowed.
"#,
    );

    assert!(err.to_string().contains("decision"));
}

#[test]
fn rejects_default_decision_allow() {
    let err = parse_err(
        r#"
schema_version: 1
kind: policy
id: vac.default-local
title: Default local policy
default_decision: allow
rules: []
"#,
    );

    assert!(err.to_string().contains("default_decision"));
    assert!(err.to_string().contains("cannot be `allow`"));
}

#[test]
fn rejects_broad_allow_without_scope() {
    let err = parse_err(
        r#"
schema_version: 1
kind: policy
id: vac.default-local
title: Default local policy
default_decision: approval_required
rules:
  - id: read-workspace
    match:
      action: filesystem_read
      path: workspace
    decision: allow
    reason: Broad read access should not be auto-allowed.
"#,
    );

    assert!(err.to_string().contains("unsafe broad allow"));
}

#[test]
fn rejects_allow_for_mutating_action() {
    let err = parse_err(
        r#"
schema_version: 1
kind: policy
id: vac.default-local
title: Default local policy
default_decision: approval_required
rules:
  - id: write-project
    match:
      action: filesystem_write
      path: project
    decision: allow
    reason: File writes should never be auto-allowed.
"#,
    );

    assert!(err.to_string().contains("unsafe broad allow"));
}

#[test]
fn rejects_missing_network_scope_for_network_action() {
    let err = parse_err(
        r#"
schema_version: 1
kind: policy
id: vac.default-local
title: Default local policy
default_decision: approval_required
rules:
  - id: allow-network
    match:
      action: network_access
    decision: approval_required
    reason: Network access must be reviewed.
"#,
    );

    assert!(err.to_string().contains("network scope"));
}

#[test]
fn validate_round_trip_manifest() {
    let manifest = parse_ok(
        r#"
schema_version: 1
kind: policy
id: vac.default-local
title: Default local policy
default_decision: deny
rules:
  - id: deny-secrets
    match:
      data_class: secret_like
    decision: deny
    reason: Secret-like values cannot be exposed.
"#,
    );

    validate_policy_manifest("test.yaml", &manifest).expect("validation should succeed");
}

#[test]
fn evaluate_step_policy_returns_unknown_when_no_manifests() {
    use super::PolicyDecisionReport;
    use super::WorkflowStepExecutionIntent;
    use super::evaluate_step_policy;

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

    let report = evaluate_step_policy(&intent, &[]);
    // Empty registry -> UnknownPolicy gives effective default-deny posture
    // until policy YAML is authored.
    assert!(matches!(report, PolicyDecisionReport::UnknownPolicy { .. }));
    assert!(report.requires_approval());
}

#[test]
fn evaluate_step_policy_requires_approval_on_manifest_default_approval_when_no_rule_matches() {
    use super::PolicyDecisionReport;
    use super::WorkflowStepExecutionIntent;
    use super::evaluate_step_policy;

    let default_approval = parse_ok(
        r#"
schema_version: 1
kind: policy
id: vac.default-approval
title: Default approval policy
default_decision: approval_required
rules:
  - id: deny-secrets
    match:
      data_class: secret_like
    decision: deny
    reason: Secret-like values cannot be exposed.
"#,
    );

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

    let report = evaluate_step_policy(&intent, std::slice::from_ref(&default_approval));
    // No rule matches (no data_class on intent) -> manifest default_decision
    // approval_required contributes RequireApproval. Locks in safe fallback.
    assert!(matches!(
        report,
        PolicyDecisionReport::RequireApproval { .. }
    ));
}

#[test]
fn evaluate_step_policy_blocks_on_manifest_default_deny() {
    use super::PolicyDecisionReport;
    use super::WorkflowStepExecutionIntent;
    use super::evaluate_step_policy;

    let strict = parse_ok(
        r#"
schema_version: 1
kind: policy
id: vac.strict-default
title: Strict default policy
default_decision: deny
rules:
  - id: read-project
    match:
      action: filesystem_read
      path: project
    decision: allow
    reason: Read-only project inspection is allowed.
"#,
    );

    let unmatched = WorkflowStepExecutionIntent {
        action: PolicyAction::ProcessExecute,
        path_scope: None,
        data_class: None,
        tool: None,
        network_scope: None,
        requires_approval_inherently: false,
        step_uses: "capability.shell.execute".to_string(),
        capability_id: None,
    };

    let report = evaluate_step_policy(&unmatched, std::slice::from_ref(&strict));
    // ProcessExecute does not match the read-project rule -> default_decision
    // deny contributes Block. Locks in default-deny opt-in behavior.
    assert!(matches!(report, PolicyDecisionReport::Block { .. }));
}

#[test]
fn evaluate_step_policy_matches_tool_against_step_uses_and_capability_id() {
    use super::PolicyDecisionReport;
    use super::WorkflowStepExecutionIntent;
    use super::evaluate_step_policy;

    let policy = parse_ok(
        r#"
schema_version: 1
kind: policy
id: vac.tool-policy
title: Tool policy
default_decision: approval_required
rules:
  - id: deny-build-capability
    match:
      tool: vac.build
    decision: deny
    reason: Build capability is blocked for this policy.
"#,
    );

    let intent = WorkflowStepExecutionIntent {
        action: PolicyAction::ProcessExecute,
        path_scope: Some(PolicyPathScope::Project),
        data_class: None,
        tool: Some("capability.build.cargo_check".to_string()),
        network_scope: None,
        requires_approval_inherently: true,
        step_uses: "capability.build.cargo_check".to_string(),
        capability_id: Some("vac.build".to_string()),
    };

    let report = evaluate_step_policy(&intent, std::slice::from_ref(&policy));
    assert!(matches!(report, PolicyDecisionReport::Block { .. }));
}

#[test]
fn evaluate_step_policy_matches_network_scope() {
    use super::PolicyDecisionReport;
    use super::PolicyNetworkScope;
    use super::WorkflowStepExecutionIntent;
    use super::evaluate_step_policy;

    let policy = parse_ok(
        r#"
schema_version: 1
kind: policy
id: vac.network-policy
title: Network policy
default_decision: approval_required
rules:
  - id: deny-api
    match:
      action: network_access
      network:
        host: api.example.test
    decision: deny
    reason: API host is blocked.
"#,
    );

    let intent = WorkflowStepExecutionIntent {
        action: PolicyAction::NetworkAccess,
        path_scope: None,
        data_class: None,
        tool: None,
        network_scope: Some(PolicyNetworkScope {
            host: "api.example.test".to_string(),
            protocol: None,
        }),
        requires_approval_inherently: true,
        step_uses: "capability.network.fetch".to_string(),
        capability_id: None,
    };

    let report = evaluate_step_policy(&intent, std::slice::from_ref(&policy));
    assert!(matches!(report, PolicyDecisionReport::Block { .. }));
}

#[test]
fn evaluate_step_policy_applies_each_manifest_default_when_another_manifest_matches() {
    use super::PolicyDecisionReport;
    use super::WorkflowStepExecutionIntent;
    use super::evaluate_step_policy;

    let allow_read = parse_ok(
        r#"
schema_version: 1
kind: policy
id: vac.allow-read
title: Allow read policy
default_decision: approval_required
rules:
  - id: allow-read-project
    match:
      action: filesystem_read
      path: project
    decision: allow
    reason: Project reads are allowed.
"#,
    );

    let deny_fallback = parse_ok(
        r#"
schema_version: 1
kind: policy
id: vac.deny-fallback
title: Deny fallback policy
default_decision: deny
rules:
  - id: deny-network
    match:
      action: network_access
      network:
        host: api.example.test
    decision: deny
    reason: API host is blocked.
"#,
    );

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

    let report = evaluate_step_policy(&intent, &[allow_read, deny_fallback]);
    // One manifest has an allow rule, but the other manifest does not match
    // and contributes its deny fallback. Deny wins across manifests.
    assert!(matches!(report, PolicyDecisionReport::Block { .. }));
}

#[test]
fn evaluate_step_policy_matches_network_wildcard_and_protocol() {
    use super::PolicyDecisionReport;
    use super::PolicyNetworkScope;
    use super::WorkflowStepExecutionIntent;
    use super::evaluate_step_policy;

    let policy = parse_ok(
        r#"
schema_version: 1
kind: policy
id: vac.network-wildcard
title: Network wildcard policy
default_decision: approval_required
rules:
  - id: deny-any-https
    match:
      action: network_access
      network:
        host: "*"
        protocol: https
    decision: deny
    reason: HTTPS network access is blocked.
"#,
    );

    let intent = WorkflowStepExecutionIntent {
        action: PolicyAction::NetworkAccess,
        path_scope: None,
        data_class: None,
        tool: None,
        network_scope: Some(PolicyNetworkScope {
            host: "api.example.test".to_string(),
            protocol: Some(NetworkApprovalProtocol::Https),
        }),
        requires_approval_inherently: true,
        step_uses: "capability.network.fetch".to_string(),
        capability_id: None,
    };

    let report = evaluate_step_policy(&intent, std::slice::from_ref(&policy));
    assert!(matches!(report, PolicyDecisionReport::Block { .. }));

    let http_intent = WorkflowStepExecutionIntent {
        network_scope: Some(PolicyNetworkScope {
            host: "api.example.test".to_string(),
            protocol: Some(NetworkApprovalProtocol::Http),
        }),
        ..intent
    };

    let report = evaluate_step_policy(&http_intent, std::slice::from_ref(&policy));
    assert!(matches!(
        report,
        PolicyDecisionReport::RequireApproval { .. }
    ));
}
