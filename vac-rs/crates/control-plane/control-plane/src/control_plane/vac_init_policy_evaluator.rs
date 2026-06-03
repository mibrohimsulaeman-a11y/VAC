#![allow(dead_code)]
//! Fail-closed VAC-Init policy evaluator and multi-policy merge contract.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PolicyAction {
    FilesystemRead,
    FilesystemWrite,
    FilesystemDelete,
    ExecuteProcess,
    NetworkAccess,
    SessionWrite,
    CheckpointWrite,
    CredentialRead,
    MemoryWrite,
    PlanModify,
    CapabilityRegister,
}

impl PolicyAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FilesystemRead => "filesystem_read",
            Self::FilesystemWrite => "filesystem_write",
            Self::FilesystemDelete => "filesystem_delete",
            Self::ExecuteProcess => "execute_process",
            Self::NetworkAccess => "network_access",
            Self::SessionWrite => "session_write",
            Self::CheckpointWrite => "checkpoint_write",
            Self::CredentialRead => "credential_read",
            Self::MemoryWrite => "memory_write",
            Self::PlanModify => "plan_modify",
            Self::CapabilityRegister => "capability_register",
        }
    }
}

impl fmt::Display for PolicyAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PolicyDecision {
    Allow,
    ApprovalRequired,
    Deny,
}

impl PolicyDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::ApprovalRequired => "approval_required",
            Self::Deny => "deny",
        }
    }

    pub const fn precedence(self) -> u8 {
        match self {
            Self::Deny => 3,
            Self::ApprovalRequired => 2,
            Self::Allow => 1,
        }
    }

    pub fn most_restrictive(self, other: Self) -> Self {
        if self.precedence() >= other.precedence() {
            self
        } else {
            other
        }
    }
}

impl fmt::Display for PolicyDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PolicyLayerKind {
    HardcodedSafety,
    Workspace,
    Capability,
    Workflow,
    Plan,
    ApprovalSession,
}

impl PolicyLayerKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HardcodedSafety => "hardcoded_safety",
            Self::Workspace => "workspace",
            Self::Capability => "capability",
            Self::Workflow => "workflow",
            Self::Plan => "plan",
            Self::ApprovalSession => "approval_session",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyMatch {
    pub action: PolicyAction,
    pub path_prefix: Option<String>,
    pub data_class: Option<String>,
    pub network_host: Option<String>,
}

impl PolicyMatch {
    pub fn action(action: PolicyAction) -> Self {
        Self {
            action,
            path_prefix: None,
            data_class: None,
            network_host: None,
        }
    }

    pub fn matches(&self, request: &ActionContext) -> bool {
        if self.action != request.action {
            return false;
        }
        if let Some(prefix) = &self.path_prefix {
            match &request.path {
                Some(path) if path.starts_with(prefix) => {}
                _ => return false,
            }
        }
        if let Some(data_class) = &self.data_class {
            match &request.data_class {
                Some(request_class) if request_class == data_class => {}
                _ => return false,
            }
        }
        if let Some(host) = &self.network_host {
            match &request.network_host {
                Some(request_host) if glob_like_match(host, request_host) => {}
                _ => return false,
            }
        }
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyRule {
    pub id: String,
    pub matcher: PolicyMatch,
    pub decision: PolicyDecision,
    pub reason: String,
}

impl PolicyRule {
    pub fn new(
        id: impl Into<String>,
        action: PolicyAction,
        decision: PolicyDecision,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            matcher: PolicyMatch::action(action),
            decision,
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyLayer {
    pub id: String,
    pub kind: PolicyLayerKind,
    pub default_decision: PolicyDecision,
    pub rules: Vec<PolicyRule>,
}

impl PolicyLayer {
    pub fn new(
        id: impl Into<String>,
        kind: PolicyLayerKind,
        default_decision: PolicyDecision,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            default_decision,
            rules: Vec::new(),
        }
    }

    pub fn with_rule(mut self, rule: PolicyRule) -> Self {
        self.rules.push(rule);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionContext {
    pub action: PolicyAction,
    pub path: Option<String>,
    pub data_class: Option<String>,
    pub network_host: Option<String>,
    pub capability: Option<String>,
    pub plan_id: Option<String>,
}

impl ActionContext {
    pub fn new(action: PolicyAction) -> Self {
        Self {
            action,
            path: None,
            data_class: None,
            network_host: None,
            capability: None,
            plan_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedPolicyDecision {
    pub layer_id: String,
    pub layer_kind: PolicyLayerKind,
    pub rule_id: Option<String>,
    pub decision: PolicyDecision,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyEvaluationReport {
    pub final_decision: PolicyDecision,
    pub matched: Vec<MatchedPolicyDecision>,
    pub fail_closed: bool,
}

impl PolicyEvaluationReport {
    pub fn is_blocked(&self) -> bool {
        self.final_decision == PolicyDecision::Deny
    }
}

pub fn evaluate_policies(
    layers: &[PolicyLayer],
    request: &ActionContext,
) -> PolicyEvaluationReport {
    if let Some(hardcoded) = hardcoded_safety_decision(request) {
        return PolicyEvaluationReport {
            final_decision: PolicyDecision::Deny,
            matched: vec![hardcoded],
            fail_closed: false,
        };
    }

    if layers.is_empty() {
        return PolicyEvaluationReport {
            final_decision: PolicyDecision::Deny,
            matched: Vec::new(),
            fail_closed: true,
        };
    }

    let mut final_decision = PolicyDecision::Allow;
    let mut matched = Vec::new();
    for layer in layers {
        let mut layer_matched = false;
        for rule in &layer.rules {
            if rule.matcher.matches(request) {
                layer_matched = true;
                final_decision = final_decision.most_restrictive(rule.decision);
                matched.push(MatchedPolicyDecision {
                    layer_id: layer.id.clone(),
                    layer_kind: layer.kind,
                    rule_id: Some(rule.id.clone()),
                    decision: rule.decision,
                    reason: rule.reason.clone(),
                });
            }
        }
        if !layer_matched {
            final_decision = final_decision.most_restrictive(layer.default_decision);
            matched.push(MatchedPolicyDecision {
                layer_id: layer.id.clone(),
                layer_kind: layer.kind,
                rule_id: None,
                decision: layer.default_decision,
                reason: "layer default decision applied".to_string(),
            });
        }
    }

    PolicyEvaluationReport {
        final_decision,
        matched,
        fail_closed: false,
    }
}

fn hardcoded_safety_decision(request: &ActionContext) -> Option<MatchedPolicyDecision> {
    if request.action == PolicyAction::CredentialRead
        || request.data_class.as_deref() == Some("credential")
        || request.data_class.as_deref() == Some("secret_like")
    {
        return Some(MatchedPolicyDecision {
            layer_id: "hardcoded.safety.credentials".to_string(),
            layer_kind: PolicyLayerKind::HardcodedSafety,
            rule_id: Some("hardcoded.deny.credential_read".to_string()),
            decision: PolicyDecision::Deny,
            reason: "credential and secret-like data access is denied unless a future explicit secret broker is used".to_string(),
        });
    }
    None
}

fn glob_like_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return value.starts_with(prefix);
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return value.ends_with(suffix);
    }
    pattern == value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_policy_loaded_is_fail_closed_deny() {
        let report = evaluate_policies(&[], &ActionContext::new(PolicyAction::FilesystemRead));
        assert_eq!(report.final_decision, PolicyDecision::Deny);
        assert!(report.fail_closed);
    }

    #[test]
    fn explicit_deny_wins_over_lower_allow() {
        let workspace = PolicyLayer::new(
            "workspace",
            PolicyLayerKind::Workspace,
            PolicyDecision::Allow,
        )
        .with_rule(PolicyRule::new(
            "deny.delete",
            PolicyAction::FilesystemDelete,
            PolicyDecision::Deny,
            "no delete",
        ));
        let plan = PolicyLayer::new("plan", PolicyLayerKind::Plan, PolicyDecision::Allow)
            .with_rule(PolicyRule::new(
                "allow.delete",
                PolicyAction::FilesystemDelete,
                PolicyDecision::Allow,
                "plan wants delete",
            ));
        let report = evaluate_policies(
            &[workspace, plan],
            &ActionContext::new(PolicyAction::FilesystemDelete),
        );
        assert_eq!(report.final_decision, PolicyDecision::Deny);
    }

    #[test]
    fn approval_required_cannot_be_downgraded_by_allow() {
        let capability = PolicyLayer::new(
            "cap",
            PolicyLayerKind::Capability,
            PolicyDecision::ApprovalRequired,
        );
        let plan = PolicyLayer::new("plan", PolicyLayerKind::Plan, PolicyDecision::Allow);
        let report = evaluate_policies(
            &[capability, plan],
            &ActionContext::new(PolicyAction::ExecuteProcess),
        );
        assert_eq!(report.final_decision, PolicyDecision::ApprovalRequired);
    }

    #[test]
    fn hardcoded_credential_deny_precedes_everything() {
        let workspace = PolicyLayer::new(
            "workspace",
            PolicyLayerKind::Workspace,
            PolicyDecision::Allow,
        )
        .with_rule(PolicyRule::new(
            "allow.creds",
            PolicyAction::CredentialRead,
            PolicyDecision::Allow,
            "bad",
        ));
        let report = evaluate_policies(
            &[workspace],
            &ActionContext::new(PolicyAction::CredentialRead),
        );
        assert_eq!(report.final_decision, PolicyDecision::Deny);
        assert_eq!(
            report.matched[0].layer_kind,
            PolicyLayerKind::HardcodedSafety
        );
    }

    #[test]
    fn network_host_match_uses_glob_like_prefix() {
        let mut rule = PolicyRule::new(
            "allow.github",
            PolicyAction::NetworkAccess,
            PolicyDecision::Allow,
            "github",
        );
        rule.matcher.network_host = Some("api.github.*".to_string());
        let layer = PolicyLayer::new(
            "workspace",
            PolicyLayerKind::Workspace,
            PolicyDecision::Deny,
        )
        .with_rule(rule);
        let mut ctx = ActionContext::new(PolicyAction::NetworkAccess);
        ctx.network_host = Some("api.github.com".to_string());
        let report = evaluate_policies(&[layer], &ctx);
        assert_eq!(report.final_decision, PolicyDecision::Allow);
    }

    #[test]
    fn unmatched_layer_default_decision_participates_in_merge() {
        let workspace = PolicyLayer::new(
            "workspace",
            PolicyLayerKind::Workspace,
            PolicyDecision::Deny,
        );
        let report = evaluate_policies(
            &[workspace],
            &ActionContext::new(PolicyAction::FilesystemRead),
        );
        assert_eq!(report.final_decision, PolicyDecision::Deny);
        assert_eq!(report.matched[0].rule_id, None);
    }

    #[test]
    fn six_policy_layers_most_restrictive_wins() {
        let layers = vec![
            PolicyLayer::new(
                "hardcoded.safety.fixture",
                PolicyLayerKind::HardcodedSafety,
                PolicyDecision::Allow,
            ),
            PolicyLayer::new(
                "workspace.fixture",
                PolicyLayerKind::Workspace,
                PolicyDecision::Allow,
            ),
            PolicyLayer::new(
                "capability.fixture",
                PolicyLayerKind::Capability,
                PolicyDecision::Allow,
            )
            .with_rule(PolicyRule::new(
                "capability.requires.approval",
                PolicyAction::FilesystemWrite,
                PolicyDecision::ApprovalRequired,
                "capability write requires operator approval",
            )),
            PolicyLayer::new(
                "workflow.fixture",
                PolicyLayerKind::Workflow,
                PolicyDecision::Allow,
            ),
            PolicyLayer::new("plan.fixture", PolicyLayerKind::Plan, PolicyDecision::Allow)
                .with_rule(PolicyRule::new(
                    "plan.allows.write",
                    PolicyAction::FilesystemWrite,
                    PolicyDecision::Allow,
                    "plan is bounded",
                )),
            PolicyLayer::new(
                "approval_session.fixture",
                PolicyLayerKind::ApprovalSession,
                PolicyDecision::Deny,
            ),
        ];
        let report = evaluate_policies(&layers, &ActionContext::new(PolicyAction::FilesystemWrite));
        assert_eq!(report.final_decision, PolicyDecision::Deny);
        assert_eq!(report.matched.len(), 6);
        assert!(
            report
                .matched
                .iter()
                .any(|decision| decision.layer_kind == PolicyLayerKind::ApprovalSession)
        );
    }

    #[test]
    fn path_specific_rule_participates_in_precedence_merge() {
        let mut scoped_rule = PolicyRule::new(
            "workspace.src.write.approval",
            PolicyAction::FilesystemWrite,
            PolicyDecision::ApprovalRequired,
            "source edits need approval",
        );
        scoped_rule.matcher.path_prefix = Some("vac-rs/core/src/".to_string());
        let workspace = PolicyLayer::new(
            "workspace.fixture",
            PolicyLayerKind::Workspace,
            PolicyDecision::Allow,
        )
        .with_rule(scoped_rule);
        let plan = PolicyLayer::new("plan.fixture", PolicyLayerKind::Plan, PolicyDecision::Allow);
        let mut ctx = ActionContext::new(PolicyAction::FilesystemWrite);
        ctx.path = Some("vac-rs/core/src/control_plane/mod.rs".to_string());
        let report = evaluate_policies(&[workspace, plan], &ctx);
        assert_eq!(report.final_decision, PolicyDecision::ApprovalRequired);
    }
}
