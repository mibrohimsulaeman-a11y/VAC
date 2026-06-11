//! VAC policy evaluator.
//!
//! This crate implements the v1.5 fail-closed policy semantics used by patch,
//! command, network, memory, approval, and registry transaction gates. It is a
//! deterministic source-level engine: no matching policy means `Deny`, and all
//! merges are most-restrictive-wins. Equivalent external contract spelling: PolicyDecision::Deny.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use vac_paths::path_matches as shared_path_matches;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Allow,
    ApprovalRequired,
    Deny,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
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
    ReadinessDeclare,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyMatch {
    pub action: PolicyAction,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub data_class: Option<String>,
    #[serde(default)]
    pub network_host: Option<String>,
    #[serde(default)]
    pub network_protocol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyRule {
    pub id: String,
    pub matcher: PolicyMatch,
    pub decision: Decision,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScopedGrant {
    pub id: String,
    pub action: PolicyAction,
    pub path: Option<String>,
    pub expires_at: String,
    pub evidence: String,
    pub decision: Decision,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicySnapshot {
    pub id: String,
    pub default_decision: Decision,
    #[serde(default)]
    pub hardcoded_safety_denials: Vec<PolicyRule>,
    #[serde(default)]
    pub workspace_rules: Vec<PolicyRule>,
    #[serde(default)]
    pub capability_rules: Vec<PolicyRule>,
    #[serde(default)]
    pub workflow_rules: Vec<PolicyRule>,
    #[serde(default)]
    pub plan_rules: Vec<PolicyRule>,
    #[serde(default)]
    pub scoped_grants: Vec<ScopedGrant>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyRequest {
    pub action: PolicyAction,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub data_class: Option<String>,
    #[serde(default)]
    pub network_host: Option<String>,
    #[serde(default)]
    pub network_protocol: Option<String>,
    #[serde(default)]
    pub capability: Option<String>,
    #[serde(default)]
    pub plan_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PolicyFailureReason {
    NoPolicyLoaded,
    NoMatchingRule,
    HardcodedSafetyDenial,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyEvaluation {
    pub decision: Decision,
    pub matched_rules: Vec<String>,
    pub warnings: Vec<String>,
}

#[must_use]
pub fn most_restrictive(a: Decision, b: Decision) -> Decision {
    use Decision::*;
    match (a, b) {
        (Deny, _) | (_, Deny) => Deny,
        (ApprovalRequired, _) | (_, ApprovalRequired) => ApprovalRequired,
        _ => Allow,
    }
}

impl PolicySnapshot {
    #[must_use]
    pub fn evaluate(&self, request: &PolicyRequest) -> PolicyEvaluation {
        let mut decision: Option<Decision> = None;
        let mut matched_rules = Vec::new();
        let mut warnings = Vec::new();

        if self.id.trim().is_empty() {
            return PolicyEvaluation {
                decision: Decision::Deny,
                matched_rules: Vec::new(),
                warnings: vec![
                    "NoPolicyLoaded: missing policy snapshot id; fail-closed".to_string(),
                ],
            };
        }

        // VAC v1.5 decision precedence is match-first:
        //   explicit deny > approval_required > explicit allow > default_decision.
        // The default decision applies only when no rule matches.  The previous
        // implementation started at default=deny and then merged explicit allow
        // through most_restrictive(), which collapsed all allow/approval rules to
        // deny and made the runtime safe-but-non-operational.
        let layers = [
            &self.hardcoded_safety_denials,
            &self.workspace_rules,
            &self.capability_rules,
            &self.workflow_rules,
            &self.plan_rules,
        ];
        for layer in layers {
            for rule in layer.iter().filter(|rule| rule.matches(request)) {
                decision = Some(match decision {
                    Some(current) => most_restrictive(current, rule.decision),
                    None => rule.decision,
                });
                matched_rules.push(rule.id.clone());
            }
        }

        let mut decision = decision.unwrap_or_else(|| {
            warnings.push(format!(
                "no policy rule matched; applying default_decision={:?}",
                self.default_decision
            ));
            self.default_decision
        });

        for grant in &self.scoped_grants {
            if grant.matches(request) {
                if grant.is_expired(Utc::now()) {
                    warnings.push(format!(
                        "scoped grant expired and was ignored: {}",
                        grant.id
                    ));
                    continue;
                }
                // A scoped grant may lower approval_required to allow, but only
                // when a stronger hard deny has not already won. This is the
                // v1.5 C-05 exception to pure most-restrictive deadlock and is
                // always evidence-bound plus time/scope bounded.
                if !matches!(decision, Decision::Deny) {
                    decision = match (decision, grant.decision) {
                        (Decision::ApprovalRequired, Decision::Allow) => Decision::Allow,
                        (current, grant_decision) => most_restrictive(current, grant_decision),
                    };
                    matched_rules.push(grant.id.clone());
                    warnings.push(format!(
                        "scoped grant applied and evidence-bound: {}",
                        grant.evidence
                    ));
                }
            }
        }

        PolicyEvaluation {
            decision,
            matched_rules,
            warnings,
        }
    }
}

impl PolicyRule {
    #[must_use]
    pub fn matches(&self, request: &PolicyRequest) -> bool {
        self.matcher.action == request.action
            && option_pattern_matches(self.matcher.path.as_deref(), request.path.as_deref())
            && option_pattern_matches(
                self.matcher.data_class.as_deref(),
                request.data_class.as_deref(),
            )
            && option_pattern_matches(
                self.matcher.network_host.as_deref(),
                request.network_host.as_deref(),
            )
            && option_pattern_matches(
                self.matcher.network_protocol.as_deref(),
                request.network_protocol.as_deref(),
            )
    }
}

impl ScopedGrant {
    #[must_use]
    pub fn matches(&self, request: &PolicyRequest) -> bool {
        self.action == request.action
            && option_pattern_matches(self.path.as_deref(), request.path.as_deref())
    }

    #[must_use]
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        if self.expires_at.trim().is_empty() {
            return true;
        }
        match DateTime::parse_from_rfc3339(&self.expires_at) {
            Ok(parsed) => parsed.with_timezone(&Utc) <= now,
            Err(_) => true,
        }
    }
}

#[must_use]
pub fn option_pattern_matches(pattern: Option<&str>, value: Option<&str>) -> bool {
    match pattern {
        None => true,
        Some("any" | "*" | "workspace" | "project") => true,
        Some(pattern) => value.is_some_and(|value| path_pattern_matches(pattern, value)),
    }
}

#[must_use]
pub fn path_pattern_matches(pattern: &str, value: &str) -> bool {
    matches!(pattern, "any" | "workspace" | "project") || shared_path_matches(pattern, value)
}
