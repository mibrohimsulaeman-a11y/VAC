#![allow(dead_code)]
//! VAC-Init runtime gate enforcement contract.
//!
//! Production Hardening C makes the control plane binding explicit at each
//! runtime boundary: agent intake (pre-plan), patch executor (pre-patch),
//! command runner (pre-command), and task completion (evidence gate). This
//! module is dependency-free so sandbox validation can run with targeted
//! `rustc --test` instead of a full workspace build.

use std::fmt;

pub const RUNTIME_ENFORCEMENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RuntimeGateKind {
    PrePlan,
    PrePatch,
    PreCommand,
    EvidenceCompletion,
}

impl RuntimeGateKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PrePlan => "pre_plan",
            Self::PrePatch => "pre_patch",
            Self::PreCommand => "pre_command",
            Self::EvidenceCompletion => "evidence_completion",
        }
    }
}

impl fmt::Display for RuntimeGateKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RuntimeGateSeverity {
    Info,
    Warning,
    Error,
    Blocked,
}

impl RuntimeGateSeverity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Blocked => "blocked",
        }
    }

    pub const fn is_failure(self) -> bool {
        matches!(self, Self::Error | Self::Blocked)
    }
}

impl fmt::Display for RuntimeGateSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RuntimeGateDecision {
    Allow,
    ApprovalRequired,
    Block,
}

impl RuntimeGateDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::ApprovalRequired => "approval_required",
            Self::Block => "block",
        }
    }

    pub const fn exit_code(self) -> i32 {
        match self {
            Self::Allow => 0,
            Self::ApprovalRequired | Self::Block => 1,
        }
    }
}

impl fmt::Display for RuntimeGateDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeGateIssue {
    pub severity: RuntimeGateSeverity,
    pub code: &'static str,
    pub message: String,
    pub remediation: &'static str,
}

impl RuntimeGateIssue {
    pub fn blocked(
        code: &'static str,
        message: impl Into<String>,
        remediation: &'static str,
    ) -> Self {
        Self {
            severity: RuntimeGateSeverity::Blocked,
            code,
            message: message.into(),
            remediation,
        }
    }

    pub fn warning(
        code: &'static str,
        message: impl Into<String>,
        remediation: &'static str,
    ) -> Self {
        Self {
            severity: RuntimeGateSeverity::Warning,
            code,
            message: message.into(),
            remediation,
        }
    }

    pub fn is_failure(&self) -> bool {
        self.severity.is_failure()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeGateReport {
    pub gate: RuntimeGateKind,
    pub decision: RuntimeGateDecision,
    pub issues: Vec<RuntimeGateIssue>,
}

impl RuntimeGateReport {
    pub fn allow(gate: RuntimeGateKind, issues: Vec<RuntimeGateIssue>) -> Self {
        Self {
            gate,
            decision: RuntimeGateDecision::Allow,
            issues,
        }
    }

    pub fn approval_required(gate: RuntimeGateKind, issues: Vec<RuntimeGateIssue>) -> Self {
        Self {
            gate,
            decision: RuntimeGateDecision::ApprovalRequired,
            issues,
        }
    }

    pub fn block(gate: RuntimeGateKind, issues: Vec<RuntimeGateIssue>) -> Self {
        Self {
            gate,
            decision: RuntimeGateDecision::Block,
            issues,
        }
    }

    pub fn is_blocked(&self) -> bool {
        self.decision == RuntimeGateDecision::Block
    }

    pub fn has_failure_issue(&self) -> bool {
        self.issues.iter().any(RuntimeGateIssue::is_failure)
    }

    pub fn render_text(&self) -> String {
        let mut lines = vec![format!(
            "{} gate: decision={} issues={}",
            self.gate,
            self.decision,
            self.issues.len()
        )];
        for issue in &self.issues {
            lines.push(format!(
                "  {} [{}] {} (remediation: {})",
                issue.severity, issue.code, issue.message, issue.remediation
            ));
        }
        lines.join("\n")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RuntimeCapabilityStatus {
    Planned,
    Partial,
    Ready,
    Deprecated,
}

impl RuntimeCapabilityStatus {
    pub const fn can_execute(self) -> bool {
        matches!(self, Self::Partial | Self::Ready)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Partial => "partial",
            Self::Ready => "ready",
            Self::Deprecated => "deprecated",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RuntimeOwnershipStatus {
    Complete,
    Partial,
    Hidden,
    Overclaimed,
    Unowned,
    HardQuarantine,
}

impl RuntimeOwnershipStatus {
    pub const fn blocks_agent_write(self) -> bool {
        matches!(
            self,
            Self::Hidden | Self::Overclaimed | Self::Unowned | Self::HardQuarantine
        )
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Hidden => "hidden",
            Self::Overclaimed => "overclaimed",
            Self::Unowned => "unowned",
            Self::HardQuarantine => "hard_quarantine",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePlanTargetFile {
    pub path: String,
    pub ownership_status: RuntimeOwnershipStatus,
    pub ownership_matches_capability: bool,
}

impl RuntimePlanTargetFile {
    pub fn complete(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            ownership_status: RuntimeOwnershipStatus::Complete,
            ownership_matches_capability: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrePlanGateContext {
    pub plan_present: bool,
    pub plan_id: Option<String>,
    pub plan_status: Option<String>,
    pub plan_approved: bool,
    pub capability_id: Option<String>,
    pub capability_status: Option<RuntimeCapabilityStatus>,
    pub policy_loaded: bool,
    pub ownership_report_available: bool,
    pub target_files: Vec<RuntimePlanTargetFile>,
}

pub fn evaluate_pre_plan_gate(ctx: &PrePlanGateContext) -> RuntimeGateReport {
    let mut issues = Vec::new();

    if !ctx.plan_present {
        issues.push(RuntimeGateIssue::blocked(
            "pre_plan.plan.missing",
            "agent runtime attempted work without a Semantic Plan",
            "create and approve a machine-readable plan before patch/command execution",
        ));
    }

    if ctx.plan_present && !ctx.plan_approved {
        issues.push(RuntimeGateIssue::blocked(
            "pre_plan.plan.not_approved",
            format!(
                "plan `{}` is not approved",
                ctx.plan_id.as_deref().unwrap_or("<unknown>")
            ),
            "operator approval is required before runtime mutation begins",
        ));
    }

    match ctx.capability_status {
        None => issues.push(RuntimeGateIssue::blocked(
            "pre_plan.capability.missing",
            format!(
                "capability `{}` is not registered",
                ctx.capability_id.as_deref().unwrap_or("<missing>")
            ),
            "register the capability or reject the plan",
        )),
        Some(status @ (RuntimeCapabilityStatus::Planned | RuntimeCapabilityStatus::Deprecated)) => {
            issues.push(RuntimeGateIssue::blocked(
                "pre_plan.capability.not_executable",
                format!("capability status `{}` is not executable", status.as_str()),
                "planned/deprecated capabilities cannot execute without migration or readiness evidence",
            ));
        }
        Some(RuntimeCapabilityStatus::Partial) => issues.push(RuntimeGateIssue::warning(
            "pre_plan.capability.partial",
            "partial capability is executable with reduced confidence",
            "surface the warning to the operator and require stronger validation later",
        )),
        Some(RuntimeCapabilityStatus::Ready) => {}
    }

    if !ctx.policy_loaded {
        issues.push(RuntimeGateIssue::blocked(
            "pre_plan.policy.missing",
            "no policy layer was loaded; VAC is fail-closed",
            "load workspace/capability/workflow policy before runtime execution",
        ));
    }

    if !ctx.ownership_report_available {
        issues.push(RuntimeGateIssue::blocked(
            "pre_plan.ownership_report.missing",
            "ownership report is unavailable",
            "run ownership scanner and persist .vac/registry/ownership/report.yaml",
        ));
    }

    for target in &ctx.target_files {
        if target.ownership_status.blocks_agent_write() {
            issues.push(RuntimeGateIssue::blocked(
                "pre_plan.ownership.blocked",
                format!(
                    "target `{}` has ownership status `{}`",
                    target.path,
                    target.ownership_status.as_str()
                ),
                "assign an owner, resolve overclaim, or remove the file from the plan",
            ));
        } else if target.ownership_status == RuntimeOwnershipStatus::Partial {
            issues.push(RuntimeGateIssue::warning(
                "pre_plan.ownership.partial",
                format!("target `{}` is owned by a partial capability", target.path),
                "allow only with visible reduced-confidence warning",
            ));
        }

        if !target.ownership_matches_capability {
            issues.push(RuntimeGateIssue::blocked(
                "pre_plan.ownership.mismatch",
                format!(
                    "target `{}` ownership does not match plan capability",
                    target.path
                ),
                "align allowed_files[].ownership with the active capability",
            ));
        }
    }

    if issues.iter().any(RuntimeGateIssue::is_failure) {
        RuntimeGateReport::block(RuntimeGateKind::PrePlan, issues)
    } else {
        RuntimeGateReport::allow(RuntimeGateKind::PrePlan, issues)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrePatchGateContext {
    pub approved_plan_id: Option<String>,
    pub plan_approved: bool,
    pub patch_guard_allowed: bool,
    pub patch_guard_issue_codes: Vec<String>,
}

pub fn evaluate_pre_patch_gate(ctx: &PrePatchGateContext) -> RuntimeGateReport {
    let mut issues = Vec::new();

    if ctx
        .approved_plan_id
        .as_deref()
        .unwrap_or_default()
        .is_empty()
    {
        issues.push(RuntimeGateIssue::blocked(
            "pre_patch.plan.missing",
            "patch executor was invoked without an approved plan id",
            "bind the patch attempt to an approved Semantic Plan",
        ));
    }

    if !ctx.plan_approved {
        issues.push(RuntimeGateIssue::blocked(
            "pre_patch.plan.not_approved",
            "patch executor cannot mutate before plan approval",
            "pause and request plan approval",
        ));
    }

    if !ctx.patch_guard_allowed {
        issues.push(RuntimeGateIssue::blocked(
            "pre_patch.guard.denied",
            format!(
                "patch guard denied the attempt: [{}]",
                ctx.patch_guard_issue_codes.join(", ")
            ),
            "reject the patch or refresh the plan/range/anchor budget",
        ));
    }

    if issues.iter().any(RuntimeGateIssue::is_failure) {
        RuntimeGateReport::block(RuntimeGateKind::PrePatch, issues)
    } else {
        RuntimeGateReport::allow(RuntimeGateKind::PrePatch, issues)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CommandPolicyDecision {
    Allow,
    ApprovalRequired,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreCommandGateContext {
    pub command_id: Option<String>,
    pub structured_command: bool,
    pub runner_registered: bool,
    pub policy_loaded: bool,
    pub command_gate_decision: CommandPolicyDecision,
    pub approval_request_persisted: bool,
}

pub fn evaluate_pre_command_gate(ctx: &PreCommandGateContext) -> RuntimeGateReport {
    let mut issues = Vec::new();

    if !ctx.structured_command {
        issues.push(RuntimeGateIssue::blocked(
            "pre_command.command.free_form",
            "runtime command is not a structured command object",
            "migrate to id/runner/args/risk/approval and reject free-form shell",
        ));
    }

    if !ctx.runner_registered {
        issues.push(RuntimeGateIssue::blocked(
            "pre_command.runner.unknown",
            "command runner is not registered",
            "use an allowlisted runner or add a policy-reviewed runner registry entry",
        ));
    }

    if !ctx.policy_loaded {
        issues.push(RuntimeGateIssue::blocked(
            "pre_command.policy.missing",
            "no command policy is loaded; VAC is fail-closed",
            "load policy before command execution",
        ));
    }

    match ctx.command_gate_decision {
        CommandPolicyDecision::Deny => issues.push(RuntimeGateIssue::blocked(
            "pre_command.policy.denied",
            format!(
                "command `{}` is denied by policy",
                ctx.command_id.as_deref().unwrap_or("<unknown>")
            ),
            "do not execute the command",
        )),
        CommandPolicyDecision::ApprovalRequired if !ctx.approval_request_persisted => {
            issues.push(RuntimeGateIssue::blocked(
                "pre_command.approval_request.missing",
                "approval-required command did not persist an approval request",
                "create .vac/registry/approvals/<approval-id>.yaml before pausing",
            ));
        }
        CommandPolicyDecision::ApprovalRequired => {
            return RuntimeGateReport::approval_required(RuntimeGateKind::PreCommand, issues);
        }
        CommandPolicyDecision::Allow => {}
    }

    if issues.iter().any(RuntimeGateIssue::is_failure) {
        RuntimeGateReport::block(RuntimeGateKind::PreCommand, issues)
    } else {
        RuntimeGateReport::allow(RuntimeGateKind::PreCommand, issues)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceCompletionGateContext {
    pub task_id: String,
    pub evidence_present: bool,
    pub evidence_chain_valid: bool,
    pub validation_passed: bool,
    pub no_new_orphan_files: bool,
    pub approval_required: bool,
    pub approval_ref_present: bool,
}

pub fn evaluate_evidence_completion_gate(ctx: &EvidenceCompletionGateContext) -> RuntimeGateReport {
    let mut issues = Vec::new();

    if !ctx.evidence_present {
        issues.push(RuntimeGateIssue::blocked(
            "evidence_completion.evidence.missing",
            format!("task `{}` has no evidence record", ctx.task_id),
            "write hash-chained evidence before marking the task complete",
        ));
    }

    if !ctx.evidence_chain_valid {
        issues.push(RuntimeGateIssue::blocked(
            "evidence_completion.chain.invalid",
            "evidence chain verification failed",
            "repair chain integrity or block further completion/evidence writes",
        ));
    }

    if !ctx.validation_passed {
        issues.push(RuntimeGateIssue::blocked(
            "evidence_completion.validation.failed",
            "post-validation did not pass",
            "rerun bounded validation or abandon/escalate the task",
        ));
    }

    if !ctx.no_new_orphan_files {
        issues.push(RuntimeGateIssue::blocked(
            "evidence_completion.orphan.detected",
            "post-validation found new orphan files",
            "assign ownership or roll back undeclared file creation",
        ));
    }

    if ctx.approval_required && !ctx.approval_ref_present {
        issues.push(RuntimeGateIssue::blocked(
            "evidence_completion.approval_ref.missing",
            "approval-required task has no approval reference",
            "bind completion evidence to approval content hash/signature",
        ));
    }

    if issues.iter().any(RuntimeGateIssue::is_failure) {
        RuntimeGateReport::block(RuntimeGateKind::EvidenceCompletion, issues)
    } else {
        RuntimeGateReport::allow(RuntimeGateKind::EvidenceCompletion, issues)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeEnforcementCoverage {
    pub agent_intake_calls_pre_plan: bool,
    pub patch_executor_calls_pre_patch: bool,
    pub command_runner_calls_pre_command: bool,
    pub task_completion_calls_evidence_gate: bool,
}

impl RuntimeEnforcementCoverage {
    pub const fn production_c_required() -> Self {
        Self {
            agent_intake_calls_pre_plan: true,
            patch_executor_calls_pre_patch: true,
            command_runner_calls_pre_command: true,
            task_completion_calls_evidence_gate: true,
        }
    }

    pub fn missing_gate_names(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if !self.agent_intake_calls_pre_plan {
            missing.push("pre_plan");
        }
        if !self.patch_executor_calls_pre_patch {
            missing.push("pre_patch");
        }
        if !self.command_runner_calls_pre_command {
            missing.push("pre_command");
        }
        if !self.task_completion_calls_evidence_gate {
            missing.push("evidence_completion");
        }
        missing
    }

    pub fn is_complete(&self) -> bool {
        self.missing_gate_names().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_pre_plan() -> PrePlanGateContext {
        PrePlanGateContext {
            plan_present: true,
            plan_id: Some("plan.runtime".to_string()),
            plan_status: Some("approved".to_string()),
            plan_approved: true,
            capability_id: Some("vac.test.fixture".to_string()),
            capability_status: Some(RuntimeCapabilityStatus::Ready),
            policy_loaded: true,
            ownership_report_available: true,
            target_files: vec![RuntimePlanTargetFile::complete("src/lib.rs")],
        }
    }

    #[test]
    fn pre_plan_blocks_when_plan_is_missing() {
        let mut ctx = valid_pre_plan();
        ctx.plan_present = false;
        ctx.plan_approved = false;
        let report = evaluate_pre_plan_gate(&ctx);
        assert!(report.is_blocked(), "{}", report.render_text());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "pre_plan.plan.missing")
        );
    }

    #[test]
    fn pre_plan_blocks_planned_or_deprecated_capabilities() {
        for status in [
            RuntimeCapabilityStatus::Planned,
            RuntimeCapabilityStatus::Deprecated,
        ] {
            let mut ctx = valid_pre_plan();
            ctx.capability_status = Some(status);
            let report = evaluate_pre_plan_gate(&ctx);
            assert!(report.is_blocked(), "{}", report.render_text());
            assert!(
                report
                    .issues
                    .iter()
                    .any(|issue| issue.code == "pre_plan.capability.not_executable")
            );
        }
    }

    #[test]
    fn pre_plan_blocks_when_policy_or_ownership_report_missing() {
        let mut ctx = valid_pre_plan();
        ctx.policy_loaded = false;
        ctx.ownership_report_available = false;
        let report = evaluate_pre_plan_gate(&ctx);
        assert!(report.is_blocked(), "{}", report.render_text());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "pre_plan.policy.missing")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "pre_plan.ownership_report.missing")
        );
    }

    #[test]
    fn pre_plan_blocks_unowned_overclaimed_and_mismatched_targets() {
        let mut ctx = valid_pre_plan();
        ctx.target_files = vec![
            RuntimePlanTargetFile {
                path: "src/unowned.rs".to_string(),
                ownership_status: RuntimeOwnershipStatus::Unowned,
                ownership_matches_capability: true,
            },
            RuntimePlanTargetFile {
                path: "src/overclaimed.rs".to_string(),
                ownership_status: RuntimeOwnershipStatus::Overclaimed,
                ownership_matches_capability: true,
            },
            RuntimePlanTargetFile {
                path: "src/mismatch.rs".to_string(),
                ownership_status: RuntimeOwnershipStatus::Complete,
                ownership_matches_capability: false,
            },
        ];
        let report = evaluate_pre_plan_gate(&ctx);
        assert!(report.is_blocked(), "{}", report.render_text());
        assert_eq!(
            report
                .issues
                .iter()
                .filter(|issue| issue.code == "pre_plan.ownership.blocked")
                .count(),
            2
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "pre_plan.ownership.mismatch")
        );
    }

    #[test]
    fn pre_plan_allows_partial_capability_with_warning_only() {
        let mut ctx = valid_pre_plan();
        ctx.capability_status = Some(RuntimeCapabilityStatus::Partial);
        ctx.target_files[0].ownership_status = RuntimeOwnershipStatus::Partial;
        let report = evaluate_pre_plan_gate(&ctx);
        assert_eq!(report.decision, RuntimeGateDecision::Allow);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.severity == RuntimeGateSeverity::Warning)
        );
    }

    #[test]
    fn pre_patch_blocks_without_approved_plan_or_patch_guard_pass() {
        let ctx = PrePatchGateContext {
            approved_plan_id: None,
            plan_approved: false,
            patch_guard_allowed: false,
            patch_guard_issue_codes: vec![
                "patch.file.outside_plan".to_string(),
                "patch.anchor.unresolved".to_string(),
            ],
        };
        let report = evaluate_pre_patch_gate(&ctx);
        assert!(report.is_blocked(), "{}", report.render_text());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "pre_patch.guard.denied")
        );
    }

    #[test]
    fn pre_command_blocks_free_form_unknown_runner_and_missing_policy() {
        let ctx = PreCommandGateContext {
            command_id: Some("bad.command".to_string()),
            structured_command: false,
            runner_registered: false,
            policy_loaded: false,
            command_gate_decision: CommandPolicyDecision::Allow,
            approval_request_persisted: false,
        };
        let report = evaluate_pre_command_gate(&ctx);
        assert!(report.is_blocked(), "{}", report.render_text());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "pre_command.command.free_form")
        );
    }

    #[test]
    fn pre_command_pauses_when_approval_request_is_persisted() {
        let ctx = PreCommandGateContext {
            command_id: Some("cargo.check".to_string()),
            structured_command: true,
            runner_registered: true,
            policy_loaded: true,
            command_gate_decision: CommandPolicyDecision::ApprovalRequired,
            approval_request_persisted: true,
        };
        let report = evaluate_pre_command_gate(&ctx);
        assert_eq!(report.decision, RuntimeGateDecision::ApprovalRequired);
        assert!(!report.has_failure_issue(), "{}", report.render_text());
    }

    #[test]
    fn pre_command_blocks_approval_required_without_request_record() {
        let ctx = PreCommandGateContext {
            command_id: Some("cargo.check".to_string()),
            structured_command: true,
            runner_registered: true,
            policy_loaded: true,
            command_gate_decision: CommandPolicyDecision::ApprovalRequired,
            approval_request_persisted: false,
        };
        let report = evaluate_pre_command_gate(&ctx);
        assert!(report.is_blocked(), "{}", report.render_text());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "pre_command.approval_request.missing")
        );
    }

    #[test]
    fn evidence_completion_blocks_missing_evidence_broken_chain_and_missing_approval() {
        let ctx = EvidenceCompletionGateContext {
            task_id: "task.runtime".to_string(),
            evidence_present: false,
            evidence_chain_valid: false,
            validation_passed: true,
            no_new_orphan_files: true,
            approval_required: true,
            approval_ref_present: false,
        };
        let report = evaluate_evidence_completion_gate(&ctx);
        assert!(report.is_blocked(), "{}", report.render_text());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "evidence_completion.evidence.missing")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "evidence_completion.chain.invalid")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "evidence_completion.approval_ref.missing")
        );
    }

    #[test]
    fn evidence_completion_allows_when_all_completion_invariants_hold() {
        let ctx = EvidenceCompletionGateContext {
            task_id: "task.runtime".to_string(),
            evidence_present: true,
            evidence_chain_valid: true,
            validation_passed: true,
            no_new_orphan_files: true,
            approval_required: true,
            approval_ref_present: true,
        };
        let report = evaluate_evidence_completion_gate(&ctx);
        assert_eq!(report.decision, RuntimeGateDecision::Allow);
    }

    #[test]
    fn production_c_coverage_requires_all_runtime_boundaries() {
        assert!(RuntimeEnforcementCoverage::production_c_required().is_complete());

        let partial = RuntimeEnforcementCoverage {
            agent_intake_calls_pre_plan: true,
            patch_executor_calls_pre_patch: false,
            command_runner_calls_pre_command: true,
            task_completion_calls_evidence_gate: false,
        };
        assert_eq!(
            partial.missing_gate_names(),
            vec!["pre_patch", "evidence_completion"]
        );
    }
}
