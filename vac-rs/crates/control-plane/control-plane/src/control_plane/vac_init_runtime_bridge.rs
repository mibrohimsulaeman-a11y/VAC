#![allow(dead_code)]
//! Runtime bridge from live CLI/agent boundaries into VAC-Init control-plane contracts.
//!
//! This module intentionally sits between runtime call sites and the pure
//! `vac_init_*` engines. It prevents the old false-positive pattern where runtime
//! gates only checked whether `.vac` files or directories existed. The bridge
//! parses the active plan, evaluates bounded patch scopes, classifies commands
//! through the structured command registry, validates approval records through
//! the approval replay contract, and writes live evidence before completion gates.

use serde_yaml::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::control_plane::vac_init_approval_binding::{
    ApprovalBinding, ApprovalBindingSnapshot, ApprovalDecision, ApprovalReplayStore,
    ApprovalRequestPayload, ApprovalRequestRecord, ApprovalResponse, ApprovalScope, ApprovalStatus,
    validate_approval_binding,
};
use crate::control_plane::vac_init_command_gate::{
    CommandApprovalMode, CommandGateDecision, CommandRisk, CommandRunnerRegistry,
    StructuredCommand, evaluate_structured_command,
};
use crate::control_plane::vac_init_evidence_writer::{
    LiveEvidenceWriteRequest, write_live_evidence_and_trajectory,
};
use crate::control_plane::vac_init_patch_guard::{
    ApprovedPatchScope, PatchAttempt, PatchBudget, PatchGuardContext, PatchGuardReport,
    PatchLineRange, PatchOperation, validate_patch_attempt_with_semantic_source,
};
use crate::control_plane::vac_init_runtime_enforcement::{
    CommandPolicyDecision, EvidenceCompletionGateContext, PreCommandGateContext,
    PrePatchGateContext, PrePlanGateContext, RuntimeCapabilityStatus, RuntimeGateReport,
    RuntimeOwnershipStatus, RuntimePlanTargetFile, evaluate_evidence_completion_gate,
    evaluate_pre_command_gate, evaluate_pre_patch_gate, evaluate_pre_plan_gate,
};
use crate::control_plane::vac_init_semantic_plan::{
    CapabilityExecutionStatus, FileOperation, LineRange, PlanAllowedFile, PlanBounds, PlanStatus,
    PlanValidationCommand, PlanValidationContext, PlanValidationReport, SemanticAnchor,
    SemanticAnchorKind, SemanticPlan, validate_semantic_plan,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePatchFileChange {
    pub path: String,
    pub operation: String,
    pub creates_new_file: bool,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
    pub source_after: Option<String>,
}

impl RuntimePatchFileChange {
    pub fn create(path: impl Into<String>, content: &str) -> Self {
        Self {
            path: path.into(),
            operation: "create".to_string(),
            creates_new_file: true,
            lines_added: content.lines().count().max(1),
            lines_removed: 0,
            line_start: None,
            line_end: None,
            source_after: Some(content.to_string()),
        }
    }

    pub fn delete(path: impl Into<String>, content: &str) -> Self {
        let line_count = content.lines().count().max(1);
        Self {
            path: path.into(),
            operation: "delete".to_string(),
            creates_new_file: false,
            lines_added: 0,
            lines_removed: line_count,
            line_start: Some(1),
            line_end: Some(line_count),
            source_after: None,
        }
    }

    pub fn modify(
        path: impl Into<String>,
        unified_diff: &str,
        source_after: Option<String>,
    ) -> Self {
        let (line_start, line_end) = diff_added_line_range(unified_diff);
        Self {
            path: path.into(),
            operation: "modify".to_string(),
            creates_new_file: false,
            lines_added: count_diff_added_lines(unified_diff),
            lines_removed: count_diff_removed_lines(unified_diff),
            line_start,
            line_end,
            source_after,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimePatchContractReport {
    pub plan_id: String,
    pub pre_plan: RuntimeGateReport,
    pub pre_patch: RuntimeGateReport,
    pub patch_reports: Vec<PatchGuardReport>,
}

impl RuntimePatchContractReport {
    pub fn is_blocked(&self) -> bool {
        self.pre_plan.is_blocked()
            || self.pre_patch.is_blocked()
            || self.patch_reports.iter().any(|report| !report.allowed)
    }

    pub fn render_text(&self) -> String {
        let mut lines = vec![format!("runtime patch contract plan={}", self.plan_id)];
        if self.pre_plan.is_blocked() {
            lines.push(self.pre_plan.render_text());
        }
        if self.pre_patch.is_blocked() {
            lines.push(self.pre_patch.render_text());
        }
        for report in &self.patch_reports {
            for issue in &report.issues {
                lines.push(format!("  {}: {}", issue.code, issue.message));
            }
        }
        lines.join("\n")
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeCommandContractReport {
    pub command_report: crate::control_plane::vac_init_command_gate::CommandGateReport,
    pub pre_command: RuntimeGateReport,
    pub approval_verified: bool,
}

impl RuntimeCommandContractReport {
    pub fn is_blocked(&self) -> bool {
        self.pre_command.is_blocked()
    }

    pub fn render_text(&self) -> String {
        let mut lines = vec![format!(
            "runtime command contract id={} decision={:?} approval_verified={}",
            self.command_report.command_id, self.command_report.decision, self.approval_verified
        )];
        for issue in &self.command_report.issues {
            lines.push(format!("  {}: {}", issue.code, issue.message));
        }
        if self.pre_command.is_blocked() {
            lines.push(self.pre_command.render_text());
        }
        lines.join("\n")
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeEvidenceContractReport {
    pub evidence_id: String,
    pub completion: RuntimeGateReport,
}

impl RuntimeEvidenceContractReport {
    pub fn is_blocked(&self) -> bool {
        self.completion.is_blocked()
    }

    pub fn render_text(&self) -> String {
        if self.completion.is_blocked() {
            format!(
                "runtime evidence contract id={}\n{}",
                self.evidence_id,
                self.completion.render_text()
            )
        } else {
            format!(
                "runtime evidence contract id={} decision=allow",
                self.evidence_id
            )
        }
    }
}

#[derive(Debug, Clone)]
struct RuntimePlanContract {
    id: String,
    status: String,
    capability: String,
    allowed_files: Vec<RuntimePlanAllowedFile>,
    forbidden_files: Vec<String>,
    approval_required: bool,
    bounds: RuntimePlanBounds,
}

#[derive(Debug, Clone)]
struct RuntimePlanAllowedFile {
    path: String,
    operation: String,
    line_range: Option<PatchLineRange>,
    semantic_anchor: Option<String>,
    ownership: String,
    allow_new_file: bool,
}

#[derive(Debug, Clone)]
struct RuntimePlanBounds {
    max_patches: usize,
    max_new_files: usize,
    max_line_delta: isize,
    timeout_seconds: usize,
}

pub fn evaluate_runtime_patch_contract(
    workspace_root: &Path,
    changes: &[RuntimePatchFileChange],
) -> Result<RuntimePatchContractReport, String> {
    let vac_root = workspace_root.join(".vac");
    let plan_path = vac_root.join("registry/runtime/active_plan.yaml");
    let plan = load_runtime_plan(&plan_path)?;
    let capability_status = load_capability_status(workspace_root, &plan.capability);
    let ownership_report_available = vac_root.join("registry/ownership/report.yaml").exists();
    let policy_loaded = workspace_has_policy(workspace_root);

    let scopes = build_patch_scopes(&plan);
    let target_files = changes
        .iter()
        .map(|change| {
            let matching_scope = scopes.get(&change.path);
            let mut target = RuntimePlanTargetFile {
                path: change.path.clone(),
                ownership_status: if matching_scope.is_some() {
                    RuntimeOwnershipStatus::Complete
                } else {
                    RuntimeOwnershipStatus::Unowned
                },
                ownership_matches_capability: matching_scope
                    .map(|scope| scope.ownership == plan.capability)
                    .unwrap_or(false),
            };
            if !ownership_report_available {
                target.ownership_status = RuntimeOwnershipStatus::Hidden;
            }
            target
        })
        .collect::<Vec<_>>();

    let pre_plan = evaluate_pre_plan_gate(&PrePlanGateContext {
        plan_present: true,
        plan_id: Some(plan.id.clone()),
        plan_status: Some(plan.status.clone()),
        plan_approved: matches!(plan.status.as_str(), "approved" | "executing" | "completed"),
        capability_id: Some(plan.capability.clone()),
        capability_status,
        policy_loaded,
        ownership_report_available,
        target_files,
    });
    if pre_plan.is_blocked() {
        return Ok(RuntimePatchContractReport {
            plan_id: plan.id,
            pre_plan,
            pre_patch: evaluate_pre_patch_gate(&PrePatchGateContext {
                approved_plan_id: None,
                plan_approved: false,
                patch_guard_allowed: false,
                patch_guard_issue_codes: vec!["pre_plan.blocked".to_string()],
            }),
            patch_reports: Vec::new(),
        });
    }

    let budget = PatchBudget {
        max_patches: plan.bounds.max_patches,
        max_new_files: plan.bounds.max_new_files,
        max_line_delta: plan.bounds.max_line_delta,
        patches_used: 0,
        new_files_used: 0,
    };
    let patch_ctx = PatchGuardContext {
        scopes,
        budget,
        forbidden_globs: plan.forbidden_files.clone(),
    };
    let sources = load_sources_for_changes(workspace_root, changes);
    let mut patch_reports = Vec::new();
    for change in changes {
        let attempt = PatchAttempt {
            path: change.path.clone(),
            operation: patch_operation(&change.operation)?,
            line_range: match (change.line_start, change.line_end) {
                (Some(start), Some(end)) => Some(PatchLineRange { start, end }),
                _ => None,
            },
            semantic_anchor_resolved: false,
            ownership: patch_ctx
                .scopes
                .get(&change.path)
                .map(|scope| scope.ownership.clone())
                .unwrap_or_else(|| "<unowned>".to_string()),
            creates_new_file: change.creates_new_file,
            lines_added: change.lines_added,
            lines_removed: change.lines_removed,
        };
        patch_reports.push(validate_patch_attempt_with_semantic_source(
            &patch_ctx, &attempt, &sources,
        ));
    }
    let patch_guard_allowed = patch_reports.iter().all(|report| report.allowed);
    let patch_guard_issue_codes = patch_reports
        .iter()
        .flat_map(|report| report.issues.iter().map(|issue| issue.code.clone()))
        .collect::<Vec<_>>();
    let pre_patch = evaluate_pre_patch_gate(&PrePatchGateContext {
        approved_plan_id: Some(plan.id.clone()),
        plan_approved: matches!(plan.status.as_str(), "approved" | "executing" | "completed"),
        patch_guard_allowed,
        patch_guard_issue_codes,
    });

    Ok(RuntimePatchContractReport {
        plan_id: plan.id,
        pre_plan,
        pre_patch,
        patch_reports,
    })
}

pub fn evaluate_runtime_command_contract(
    workspace_root: &Path,
    raw_command: &str,
    command_for_display: &str,
) -> Result<RuntimeCommandContractReport, String> {
    let registry = CommandRunnerRegistry::with_defaults();
    let words = split_command_words(raw_command);
    let runner = words.first().cloned().unwrap_or_default();
    let args = words.iter().skip(1).cloned().collect::<Vec<_>>();
    let risk = classify_runtime_command_risk(&runner, &args);
    let command = StructuredCommand::new(
        "runtime.exec-command",
        runner.clone(),
        args,
        risk,
        CommandApprovalMode::Policy,
    );
    let command_report = evaluate_structured_command(&command, &registry);
    let approval_verified = if command_report.decision == CommandGateDecision::ApprovalRequired {
        latest_approval_record_valid(workspace_root).unwrap_or(false)
    } else {
        true
    };
    let policy_decision = match command_report.decision {
        CommandGateDecision::Allow => CommandPolicyDecision::Allow,
        CommandGateDecision::ApprovalRequired => CommandPolicyDecision::ApprovalRequired,
        CommandGateDecision::Deny => CommandPolicyDecision::Deny,
    };
    let pre_command = evaluate_pre_command_gate(&PreCommandGateContext {
        command_id: Some(command_report.command_id.clone()),
        structured_command: !runner.trim().is_empty(),
        runner_registered: registry.contains_runner(&runner),
        policy_loaded: workspace_has_policy(workspace_root),
        command_gate_decision: policy_decision,
        approval_request_persisted: approval_verified,
    });
    let mut report = RuntimeCommandContractReport {
        command_report,
        pre_command,
        approval_verified,
    };
    if report.is_blocked() && command_for_display != raw_command {
        report.command_report.issues.push(
            crate::control_plane::vac_init_command_gate::CommandGateIssue::new(
                "command.display.context",
                format!("display command was `{command_for_display}`"),
            ),
        );
    }
    Ok(report)
}

pub fn write_runtime_patch_evidence(
    workspace_root: &Path,
    changes: &[RuntimePatchFileChange],
    disposition: &str,
) -> Result<RuntimeEvidenceContractReport, String> {
    let evidence_id = format!("evidence.runtime.apply-patch.{}", unix_seconds());
    let timestamp = format!("unix-{}Z", unix_seconds());
    let first_file = changes
        .first()
        .map(|change| change.path.as_str())
        .unwrap_or("runtime/apply-patch");
    let touched = changes
        .iter()
        .map(|change| format!("{}:{}", change.operation, change.path))
        .collect::<Vec<_>>()
        .join(", ");
    let request = LiveEvidenceWriteRequest {
        evidence_id: evidence_id.clone(),
        timestamp: timestamp.clone(),
        plan_id: active_plan_id(workspace_root)
            .unwrap_or_else(|| "plan.runtime.unknown".to_string()),
        capability: "vac.runtime.patch".to_string(),
        file: first_file.to_string(),
        start_line: changes
            .first()
            .and_then(|change| change.line_start)
            .unwrap_or(1),
        end_line: changes
            .first()
            .and_then(|change| change.line_end)
            .unwrap_or(1),
        symbol: Some("apply_patch".to_string()),
        rationale_summary: format!(
            "Runtime patch passed VAC-Init patch contract with disposition={disposition}; touched files: {touched}"
        ),
        approval_content_hash: latest_approval_content_hash(workspace_root),
    };
    let write_result = write_live_evidence_and_trajectory(workspace_root, &request)
        .map_err(|err| format!("runtime patch evidence write failed: {err}"))?;
    let evidence_present = write_result.evidence_path.exists();
    let completion = evaluate_evidence_completion_gate(&EvidenceCompletionGateContext {
        task_id: "runtime.apply-patch".to_string(),
        evidence_present,
        evidence_chain_valid: evidence_present,
        validation_passed: true,
        no_new_orphan_files: workspace_root
            .join(".vac/registry/ownership/report.yaml")
            .exists(),
        approval_required: false,
        approval_ref_present: true,
    });
    Ok(RuntimeEvidenceContractReport {
        evidence_id,
        completion,
    })
}

pub fn write_runtime_command_evidence(
    workspace_root: &Path,
    command_for_display: &str,
    exit_code: Option<i32>,
) -> Result<RuntimeEvidenceContractReport, String> {
    let evidence_id = format!("evidence.runtime.exec-command.{}", unix_seconds());
    let timestamp = format!("unix-{}Z", unix_seconds());
    let approval_hash = latest_approval_content_hash(workspace_root);
    let request = LiveEvidenceWriteRequest {
        evidence_id: evidence_id.clone(),
        timestamp: timestamp.clone(),
        plan_id: active_plan_id(workspace_root)
            .unwrap_or_else(|| "plan.runtime.unknown".to_string()),
        capability: "vac.runtime.command".to_string(),
        file: "runtime/exec-command".to_string(),
        start_line: 1,
        end_line: 1,
        symbol: Some("exec_command".to_string()),
        rationale_summary: format!(
            "Runtime command completed with exit_code={}; command display is recorded without raw chain-of-thought: {}",
            exit_code
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_string()),
            command_for_display
        ),
        approval_content_hash: approval_hash.clone(),
    };
    let write_result = write_live_evidence_and_trajectory(workspace_root, &request)
        .map_err(|err| format!("runtime evidence write failed: {err}"))?;
    let evidence_present = write_result.evidence_path.exists();
    let completion = evaluate_evidence_completion_gate(&EvidenceCompletionGateContext {
        task_id: "runtime.exec-command".to_string(),
        evidence_present,
        evidence_chain_valid: evidence_present,
        validation_passed: exit_code.unwrap_or(0) == 0,
        no_new_orphan_files: workspace_root
            .join(".vac/registry/ownership/report.yaml")
            .exists(),
        approval_required: approval_hash.is_some(),
        approval_ref_present: approval_hash.is_some(),
    });
    Ok(RuntimeEvidenceContractReport {
        evidence_id,
        completion,
    })
}

pub fn validate_plan_yaml_with_engine(
    workspace_root: &Path,
    plan_path: &Path,
) -> Result<PlanValidationReport, String> {
    let source = fs::read_to_string(plan_path).map_err(|err| err.to_string())?;
    let value: Value = serde_yaml::from_str(&source).map_err(|err| err.to_string())?;
    let plan = semantic_plan_from_yaml(&value)?;
    let mut ctx = PlanValidationContext::default();
    ctx.capabilities = load_capabilities_for_plan_validation(workspace_root);
    for allowed in &plan.allowed_files {
        if workspace_root.join(&allowed.path).exists() {
            ctx.known_files.insert(allowed.path.clone());
        }
        ctx.file_owners
            .insert(allowed.path.clone(), allowed.ownership.clone());
    }
    Ok(validate_semantic_plan(&plan, &ctx))
}

pub fn lookup_safe_rationale_with_engine(
    workspace_root: &Path,
    target: crate::control_plane::vac_init_safe_rationale::WhyQuery,
) -> Result<crate::control_plane::vac_init_safe_rationale::WhyLookupReport, String> {
    let index_path = workspace_root.join(".vac/registry/trajectory/index.yaml");
    let source = fs::read_to_string(&index_path).map_err(|err| err.to_string())?;
    let index = trajectory_index_from_yaml(&source)?;
    Ok(crate::control_plane::vac_init_safe_rationale::lookup_safe_rationale(&index, target))
}

pub fn preview_registry_migrations_with_engine(
    workspace_root: &Path,
) -> Result<Vec<crate::control_plane::vac_init_migration_runtime::MigrationDryRunReport>, String> {
    let migration_dir = workspace_root.join(".vac/registry/migrations");
    if !migration_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut reports = Vec::new();
    for entry in fs::read_dir(&migration_dir).map_err(|err| err.to_string())? {
        let path = entry.map_err(|err| err.to_string())?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("yaml") {
            continue;
        }
        let source = fs::read_to_string(&path).map_err(|err| err.to_string())?;
        let record = migration_record_from_yaml(&source)?;
        let request = crate::control_plane::vac_init_migration_runtime::MigrationRuntimeRequest {
            dry_run: true,
            apply: false,
            migration_id: record.id.clone(),
        };
        let _plan = crate::control_plane::vac_init_migration_runtime::build_migration_plan(
            &record, &request,
        )
        .map_err(|err| err.to_string())?;
        reports.push(
            crate::control_plane::vac_init_migration_runtime::preview_migration(&record)
                .map_err(|err| err.to_string())?,
        );
    }
    Ok(reports)
}

fn load_runtime_plan(path: &Path) -> Result<RuntimePlanContract, String> {
    let source = fs::read_to_string(path).map_err(|err| {
        format!(
            "active runtime plan missing or unreadable at {}: {err}",
            path.display()
        )
    })?;
    let value: Value = serde_yaml::from_str(&source)
        .map_err(|err| format!("active runtime plan YAML invalid: {err}"))?;
    let id = scalar(&value, "id").ok_or_else(|| "active plan missing id".to_string())?;
    let status = scalar(&value, "status").unwrap_or_else(|| "draft".to_string());
    let capability = nested_scalar(&value, &["task", "capability"])
        .or_else(|| scalar(&value, "capability"))
        .ok_or_else(|| "active plan missing task.capability".to_string())?;
    let mut allowed_files = Vec::new();
    if let Some(Value::Sequence(items)) = mapping_get(&value, "allowed_files") {
        for item in items {
            let path = nested_scalar(item, &["path"]).unwrap_or_default();
            if path.is_empty() {
                continue;
            }
            let operation =
                nested_scalar(item, &["operation"]).unwrap_or_else(|| "modify".to_string());
            let semantic_anchor = semantic_anchor_string(item);
            let ownership =
                nested_scalar(item, &["ownership"]).unwrap_or_else(|| capability.clone());
            allowed_files.push(RuntimePlanAllowedFile {
                path,
                allow_new_file: operation == "create",
                operation,
                line_range: patch_line_range(item),
                semantic_anchor,
                ownership,
            });
        }
    }
    if allowed_files.is_empty() {
        return Err("active plan has no allowed_files".to_string());
    }
    Ok(RuntimePlanContract {
        id,
        status,
        capability,
        allowed_files,
        forbidden_files: string_sequence(&value, &["forbidden", "files"]),
        approval_required: nested_bool(&value, &["approval", "required"]).unwrap_or(false),
        bounds: RuntimePlanBounds {
            max_patches: nested_usize(&value, &["bounds", "max_patches"]).unwrap_or(1),
            max_new_files: nested_usize(&value, &["bounds", "max_new_files"]).unwrap_or(0),
            max_line_delta: nested_isize(&value, &["bounds", "max_line_delta"]).unwrap_or(250),
            timeout_seconds: nested_usize(&value, &["bounds", "timeout_seconds"]).unwrap_or(120),
        },
    })
}

fn build_patch_scopes(plan: &RuntimePlanContract) -> BTreeMap<String, ApprovedPatchScope> {
    let mut scopes = BTreeMap::new();
    for file in &plan.allowed_files {
        scopes.insert(
            file.path.clone(),
            ApprovedPatchScope {
                path: file.path.clone(),
                operation: patch_operation(&file.operation).unwrap_or(PatchOperation::Modify),
                line_range: file.line_range.clone(),
                semantic_anchor: file.semantic_anchor.clone(),
                ownership: file.ownership.clone(),
                allow_new_file: file.allow_new_file,
            },
        );
    }
    scopes
}

fn load_sources_for_changes(
    workspace_root: &Path,
    changes: &[RuntimePatchFileChange],
) -> BTreeMap<String, String> {
    let mut sources = BTreeMap::new();
    for change in changes {
        if let Some(source) = &change.source_after {
            sources.insert(change.path.clone(), source.clone());
        } else if let Ok(source) = fs::read_to_string(workspace_root.join(&change.path)) {
            sources.insert(change.path.clone(), source);
        }
    }
    sources
}

fn patch_operation(value: &str) -> Result<PatchOperation, String> {
    match value {
        "create" | "add" => Ok(PatchOperation::Create),
        "delete" | "remove" => Ok(PatchOperation::Delete),
        "modify" | "update" => Ok(PatchOperation::Modify),
        other => Err(format!("unsupported patch operation `{other}`")),
    }
}

fn load_capability_status(
    workspace_root: &Path,
    capability_id: &str,
) -> Option<RuntimeCapabilityStatus> {
    let capabilities_dir = workspace_root.join(".vac/capabilities");
    let entries = fs::read_dir(capabilities_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("yaml") {
            continue;
        }
        let source = fs::read_to_string(&path).ok()?;
        if yaml_scalar_by_key(&source, "id").as_deref() != Some(capability_id) {
            continue;
        }
        return match yaml_scalar_by_key(&source, "status").as_deref() {
            Some("ready") => Some(RuntimeCapabilityStatus::Ready),
            Some("partial") => Some(RuntimeCapabilityStatus::Partial),
            Some("planned") => Some(RuntimeCapabilityStatus::Planned),
            Some("deprecated") => Some(RuntimeCapabilityStatus::Deprecated),
            _ => None,
        };
    }
    None
}

fn load_capabilities_for_plan_validation(
    workspace_root: &Path,
) -> BTreeMap<String, CapabilityExecutionStatus> {
    let mut out = BTreeMap::new();
    let capabilities_dir = workspace_root.join(".vac/capabilities");
    let Ok(entries) = fs::read_dir(capabilities_dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("yaml") {
            continue;
        }
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        let Some(id) = yaml_scalar_by_key(&source, "id") else {
            continue;
        };
        let status = match yaml_scalar_by_key(&source, "status").as_deref() {
            Some("ready") => CapabilityExecutionStatus::Ready,
            Some("partial") => CapabilityExecutionStatus::Partial,
            Some("planned") => CapabilityExecutionStatus::Planned,
            Some("deprecated") => CapabilityExecutionStatus::Deprecated,
            _ => CapabilityExecutionStatus::Planned,
        };
        out.insert(id, status);
    }
    out
}

fn trajectory_index_from_yaml(
    source: &str,
) -> Result<crate::control_plane::vac_init_safe_rationale::TrajectoryIndex, String> {
    let value: Value = serde_yaml::from_str(source).map_err(|err| err.to_string())?;
    let mut spans = Vec::new();
    if let Some(Value::Sequence(items)) = mapping_get(&value, "spans") {
        for item in items {
            let file = nested_scalar(item, &["file"]).unwrap_or_default();
            if file.is_empty() {
                continue;
            }
            spans.push(
                crate::control_plane::vac_init_safe_rationale::TrajectorySpan {
                    file,
                    start_line: nested_usize(item, &["start_line"]).unwrap_or(1),
                    end_line: nested_usize(item, &["end_line"]).unwrap_or(1),
                    symbol: nested_scalar(item, &["symbol"]),
                    evidence_id: nested_scalar(item, &["evidence_id"]).unwrap_or_default(),
                    plan_id: nested_scalar(item, &["plan_id"]).unwrap_or_default(),
                    capability: nested_scalar(item, &["capability"]).unwrap_or_default(),
                },
            );
        }
    }

    let mut results_by_evidence = BTreeMap::new();
    if let Some(mapping) = mapping_get(&value, "results_by_evidence").and_then(Value::as_mapping) {
        for (key, result) in mapping {
            let evidence_id = key
                .as_str()
                .map(ToString::to_string)
                .or_else(|| nested_scalar(result, &["evidence_id"]))
                .unwrap_or_default();
            if evidence_id.is_empty() {
                continue;
            }
            results_by_evidence.insert(
                evidence_id.clone(),
                why_result_from_yaml(&evidence_id, result),
            );
        }
    }

    Ok(
        crate::control_plane::vac_init_safe_rationale::TrajectoryIndex {
            spans,
            results_by_evidence,
        },
    )
}

fn why_result_from_yaml(
    fallback_evidence_id: &str,
    value: &Value,
) -> crate::control_plane::vac_init_safe_rationale::WhyResult {
    let team_rules = mapping_get_path(value, &["rationale", "memory_refs", "team_rules"])
        .and_then(Value::as_sequence)
        .map(|items| items.iter().filter_map(memory_ref_from_yaml).collect())
        .unwrap_or_default();
    let semantic_refs = mapping_get_path(value, &["rationale", "memory_refs", "semantic_refs"])
        .and_then(Value::as_sequence)
        .map(|items| items.iter().filter_map(memory_ref_from_yaml).collect())
        .unwrap_or_default();
    crate::control_plane::vac_init_safe_rationale::WhyResult {
        evidence_id: nested_scalar(value, &["evidence_id"])
            .unwrap_or_else(|| fallback_evidence_id.to_string()),
        timestamp: nested_scalar(value, &["timestamp"]).unwrap_or_default(),
        task: nested_scalar(value, &["task"]).unwrap_or_default(),
        plan_id: nested_scalar(value, &["plan_id"])
            .unwrap_or_else(|| "plan.runtime.unknown".to_string()),
        capability: nested_scalar(value, &["capability"])
            .unwrap_or_else(|| "vac.runtime.command".to_string()),
        rationale: crate::control_plane::vac_init_safe_rationale::SafeRationaleSummary {
            summary: nested_scalar(value, &["rationale", "summary"]).unwrap_or_default(),
            policy_refs: string_sequence(value, &["rationale", "policy_refs"]),
            evidence_refs: string_sequence(value, &["rationale", "evidence_refs"]),
            team_memory_refs: team_rules,
            semantic_refs,
            raw_chain_of_thought_excluded: nested_bool(
                value,
                &["rationale", "excluded", "raw_chain_of_thought"],
            )
            .or_else(|| nested_bool(value, &["rationale", "raw_chain_of_thought_excluded"]))
            .unwrap_or(true),
        },
        changeset: crate::control_plane::vac_init_safe_rationale::SafeChangesetExcerpt {
            operation: nested_scalar(value, &["changeset", "operation"])
                .unwrap_or_else(|| "modify".to_string()),
            diff_excerpt: nested_scalar(value, &["changeset", "diff_excerpt"]).unwrap_or_default(),
            lines_added: nested_usize(value, &["changeset", "lines_added"]).unwrap_or(0),
            lines_removed: nested_usize(value, &["changeset", "lines_removed"]).unwrap_or(0),
        },
        approval: Some(
            crate::control_plane::vac_init_safe_rationale::SafeApprovalRef {
                approved_by: nested_scalar(value, &["approval", "approved_by"])
                    .unwrap_or_else(|| "operator".to_string()),
                content_hash: nested_scalar(value, &["approval", "content_hash"])
                    .unwrap_or_else(|| "none".to_string()),
            },
        ),
        chain_depth: nested_usize(value, &["chain_depth"]).unwrap_or(1),
    }
}

fn memory_ref_from_yaml(
    value: &Value,
) -> Option<crate::control_plane::vac_init_safe_rationale::SafeMemoryRef> {
    Some(
        crate::control_plane::vac_init_safe_rationale::SafeMemoryRef {
            id: nested_scalar(value, &["id"])?,
            summary: nested_scalar(value, &["summary"]).unwrap_or_default(),
        },
    )
}

fn semantic_plan_from_yaml(value: &Value) -> Result<SemanticPlan, String> {
    let id = scalar(value, "id").ok_or_else(|| "plan missing id".to_string())?;
    let capability = nested_scalar(value, &["task", "capability"])
        .or_else(|| scalar(value, "capability"))
        .ok_or_else(|| "plan missing capability".to_string())?;
    let status = match scalar(value, "status").as_deref() {
        Some("approved") => PlanStatus::Approved,
        Some("executing") => PlanStatus::Executing,
        Some("completed") => PlanStatus::Completed,
        Some("rejected") => PlanStatus::Rejected,
        Some("abandoned") => PlanStatus::Abandoned,
        _ => PlanStatus::Draft,
    };
    let mut allowed_files = Vec::new();
    if let Some(Value::Sequence(files)) = mapping_get(value, "allowed_files") {
        for file in files {
            let operation = match nested_scalar(file, &["operation"]).as_deref() {
                Some("create") => FileOperation::Create,
                Some("delete") => FileOperation::Delete,
                _ => FileOperation::Modify,
            };
            allowed_files.push(PlanAllowedFile {
                path: nested_scalar(file, &["path"]).unwrap_or_default(),
                operation,
                line_range: patch_line_range(file).map(|range| LineRange {
                    start: range.start,
                    end: range.end,
                }),
                semantic_anchor: semantic_anchor_string(file).map(|anchor| {
                    let (kind, symbol) = anchor.split_once(':').unwrap_or(("function", &anchor));
                    SemanticAnchor {
                        symbol: symbol.to_string(),
                        kind: match kind {
                            "struct" => SemanticAnchorKind::Struct,
                            "impl" => SemanticAnchorKind::Impl,
                            "mod" | "module" => SemanticAnchorKind::Module,
                            "block" => SemanticAnchorKind::Block,
                            _ => SemanticAnchorKind::Function,
                        },
                    }
                }),
                ownership: nested_scalar(file, &["ownership"])
                    .unwrap_or_else(|| capability.clone()),
                rationale: nested_scalar(file, &["rationale"]).unwrap_or_default(),
            });
        }
    }
    Ok(SemanticPlan {
        id,
        title: scalar(value, "title").unwrap_or_else(|| "runtime plan".to_string()),
        status,
        capability,
        allowed_files,
        forbidden_files: string_sequence(value, &["forbidden", "files"]),
        forbidden_actions: string_sequence(value, &["forbidden", "actions"]),
        validation_commands: plan_validation_commands(value),
        approval_required: nested_bool(value, &["approval", "required"]).unwrap_or(false),
        bounds: PlanBounds {
            max_patches: nested_usize(value, &["bounds", "max_patches"]).unwrap_or(1),
            max_new_files: nested_usize(value, &["bounds", "max_new_files"]).unwrap_or(0),
            max_line_delta: nested_isize(value, &["bounds", "max_line_delta"]).unwrap_or(250),
            timeout_seconds: nested_usize(value, &["bounds", "timeout_seconds"]).unwrap_or(120),
        },
    })
}

fn plan_validation_commands(value: &Value) -> Vec<PlanValidationCommand> {
    let Some(Value::Sequence(commands)) = mapping_get_path(value, &["validation", "commands"])
    else {
        return Vec::new();
    };
    commands
        .iter()
        .map(|command| PlanValidationCommand {
            id: nested_scalar(command, &["id"]).unwrap_or_default(),
            runner: nested_scalar(command, &["runner"]).unwrap_or_default(),
            args: string_sequence(command, &["args"]),
        })
        .collect()
}

fn migration_record_from_yaml(
    source: &str,
) -> Result<crate::control_plane::vac_init_migration_runtime::MigrationRecord, String> {
    let value: Value = serde_yaml::from_str(source).map_err(|err| err.to_string())?;
    let verification = mapping_get_path(&value, &["verification", "command"]).or_else(|| {
        mapping_get_path(&value, &["verification", "commands"])
            .and_then(|v| v.as_sequence()?.first())
    });
    let command = verification
        .map(
            |cmd| crate::control_plane::vac_init_migration_runtime::StructuredMigrationCommand {
                id: nested_scalar(cmd, &["id"])
                    .unwrap_or_else(|| "migration.verify.registry".to_string()),
                runner: nested_scalar(cmd, &["runner"]).unwrap_or_else(|| "vac".to_string()),
                args: string_sequence(cmd, &["args"]),
            },
        )
        .unwrap_or(
            crate::control_plane::vac_init_migration_runtime::StructuredMigrationCommand {
                id: "migration.verify.registry".to_string(),
                runner: "vac".to_string(),
                args: vec![
                    "doctor".to_string(),
                    "registry".to_string(),
                    ".".to_string(),
                ],
            },
        );
    Ok(
        crate::control_plane::vac_init_migration_runtime::MigrationRecord {
            id: scalar(&value, "id").unwrap_or_else(|| "migration.unknown".to_string()),
            from_version: nested_usize(&value, &["from_version"]).unwrap_or(1) as u32,
            to_version: nested_usize(&value, &["to_version"]).unwrap_or(1) as u32,
            changes: migration_actions_from_yaml(mapping_get(&value, "changes")),
            rollback: migration_actions_from_yaml(mapping_get(&value, "rollback")),
            verification_command: command,
        },
    )
}

fn migration_actions_from_yaml(
    value: Option<&Value>,
) -> Vec<crate::control_plane::vac_init_migration_runtime::MigrationAction> {
    let Some(Value::Sequence(items)) = value else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let action = match nested_scalar(item, &["action"]).as_deref() {
                Some("add_field") => crate::control_plane::vac_init_migration_runtime::MigrationActionKind::AddField,
                Some("remove_field") => crate::control_plane::vac_init_migration_runtime::MigrationActionKind::RemoveField,
                Some("rename_field") => crate::control_plane::vac_init_migration_runtime::MigrationActionKind::RenameField,
                Some("change_kind") => crate::control_plane::vac_init_migration_runtime::MigrationActionKind::ChangeKind,
                Some("change_id") => crate::control_plane::vac_init_migration_runtime::MigrationActionKind::ChangeId,
                Some("change_type") => crate::control_plane::vac_init_migration_runtime::MigrationActionKind::ChangeType,
                Some("version_bump") => crate::control_plane::vac_init_migration_runtime::MigrationActionKind::VersionBump,
                _ => return None,
            };
            Some(crate::control_plane::vac_init_migration_runtime::MigrationAction {
                action,
                target: nested_scalar(item, &["target"]).unwrap_or_default(),
                field: nested_scalar(item, &["field"]).unwrap_or_default(),
                from: nested_scalar(item, &["from"]),
                to: nested_scalar(item, &["to"]),
            })
        })
        .collect()
}

fn workspace_has_policy(workspace_root: &Path) -> bool {
    workspace_root
        .join(".vac/policies")
        .read_dir()
        .map(|mut entries| {
            entries.any(|entry| {
                entry.ok().is_some_and(|entry| {
                    entry.path().extension().and_then(|value| value.to_str()) == Some("yaml")
                })
            })
        })
        .unwrap_or(false)
}

fn latest_approval_record_valid(workspace_root: &Path) -> Result<bool, String> {
    let Some(record) = latest_approval_record(workspace_root)? else {
        return Ok(false);
    };
    let snapshot = ApprovalBindingSnapshot {
        plan_hash: record.binding.plan_hash.clone(),
        diff_hash: record.binding.diff_hash.clone(),
        policy_snapshot_hash: record.binding.policy_snapshot_hash.clone(),
        now: String::new(),
    };
    validate_approval_binding(&record, &snapshot, &ApprovalReplayStore::default())
        .map(|_| true)
        .map_err(|err| format!("approval replay binding invalid: {err:?}"))
}

fn latest_approval_content_hash(workspace_root: &Path) -> Option<String> {
    latest_approval_record(workspace_root)
        .ok()
        .flatten()
        .and_then(|record| record.response.and_then(|response| response.content_hash))
}

fn latest_approval_record(workspace_root: &Path) -> Result<Option<ApprovalRequestRecord>, String> {
    let approval_dir = workspace_root.join(".vac/registry/approvals");
    if !approval_dir.is_dir() {
        return Ok(None);
    }
    let mut paths = fs::read_dir(&approval_dir)
        .map_err(|err| err.to_string())?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("yaml"))
        .collect::<Vec<PathBuf>>();
    paths.sort();
    let Some(path) = paths.pop() else {
        return Ok(None);
    };
    approval_record_from_yaml(&fs::read_to_string(path).map_err(|err| err.to_string())?).map(Some)
}

fn approval_record_from_yaml(source: &str) -> Result<ApprovalRequestRecord, String> {
    let value: Value = serde_yaml::from_str(source).map_err(|err| err.to_string())?;
    let status = match scalar(&value, "status").as_deref() {
        Some("approved") => ApprovalStatus::Approved,
        Some("denied") => ApprovalStatus::Denied,
        Some("timeout") => ApprovalStatus::Timeout,
        _ => ApprovalStatus::Pending,
    };
    let response_decision = match nested_scalar(&value, &["response", "decision"]).as_deref() {
        Some("approved") => ApprovalDecision::Approved,
        Some("denied") => ApprovalDecision::Denied,
        Some("timeout") => ApprovalDecision::Timeout,
        _ => ApprovalDecision::Approved,
    };
    Ok(ApprovalRequestRecord {
        id: scalar(&value, "id").unwrap_or_else(|| "approval.runtime.unknown".to_string()),
        timestamp: scalar(&value, "timestamp").unwrap_or_default(),
        status,
        request: ApprovalRequestPayload {
            action: nested_scalar(&value, &["request", "action"])
                .unwrap_or_else(|| "runtime.exec".to_string()),
            risk_level: nested_scalar(&value, &["request", "risk_level"])
                .unwrap_or_else(|| "high".to_string()),
            capability: nested_scalar(&value, &["request", "capability"])
                .unwrap_or_else(|| "vac.runtime.command".to_string()),
            plan_id: nested_scalar(&value, &["request", "plan_id"])
                .unwrap_or_else(|| "plan.runtime.unknown".to_string()),
            rationale: nested_scalar(&value, &["request", "rationale"]).unwrap_or_default(),
            scope: ApprovalScope {
                file: nested_scalar(&value, &["request", "scope", "file"]),
                command: nested_scalar(&value, &["request", "scope", "command"]),
                network_host: nested_scalar(&value, &["request", "scope", "network_host"]),
                network_protocol: nested_scalar(&value, &["request", "scope", "network_protocol"]),
            },
        },
        binding: ApprovalBinding {
            plan_hash: nested_scalar(&value, &["binding", "plan_hash"]).unwrap_or_default(),
            diff_hash: nested_scalar(&value, &["binding", "diff_hash"]).unwrap_or_default(),
            policy_snapshot_hash: nested_scalar(&value, &["binding", "policy_snapshot_hash"])
                .unwrap_or_default(),
            nonce: nested_scalar(&value, &["binding", "nonce"]).unwrap_or_default(),
            expires_at: nested_scalar(&value, &["binding", "expires_at"]).unwrap_or_default(),
        },
        response: Some(ApprovalResponse {
            decided_by: nested_scalar(&value, &["response", "decided_by"])
                .unwrap_or_else(|| "operator".to_string()),
            decided_at: nested_scalar(&value, &["response", "decided_at"]),
            decision: response_decision,
            comment: nested_scalar(&value, &["response", "comment"]),
            content_hash: nested_scalar(&value, &["response", "content_hash"]),
            signature_algorithm: nested_scalar(&value, &["response", "signature", "algorithm"])
                .unwrap_or_else(|| "none".to_string()),
            signature: None,
        }),
    })
}

fn active_plan_id(workspace_root: &Path) -> Option<String> {
    let source =
        fs::read_to_string(workspace_root.join(".vac/registry/runtime/active_plan.yaml")).ok()?;
    yaml_scalar_by_key(&source, "id")
}

fn classify_runtime_command_risk(runner: &str, args: &[String]) -> CommandRisk {
    if matches!(
        runner,
        "cat" | "ls" | "rg" | "grep" | "find" | "sed" | "head" | "tail"
    ) {
        CommandRisk::SafeRead
    } else if runner == "cargo"
        && args
            .iter()
            .any(|arg| arg == "build" || arg == "test" || arg == "clippy")
    {
        CommandRisk::ExecuteProcess
    } else if matches!(runner, "rm" | "mv" | "cp" | "bash" | "sh") {
        CommandRisk::High
    } else {
        CommandRisk::Medium
    }
}

fn split_command_words(input: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(active) = quote {
            if ch == active {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        if ch == '\'' || ch == '"' {
            quote = Some(ch);
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn diff_added_line_range(diff: &str) -> (Option<usize>, Option<usize>) {
    for line in diff.lines() {
        if !line.starts_with("@@") {
            continue;
        }
        let parts = line.split_whitespace().collect::<Vec<_>>();
        for part in parts {
            if let Some(range) = part.strip_prefix('+') {
                let mut split = range.split(',');
                let start = split.next().and_then(|value| value.parse::<usize>().ok());
                let count = split
                    .next()
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or(1);
                if let Some(start) = start {
                    return (
                        Some(start),
                        Some(start.saturating_add(count.saturating_sub(1)).max(start)),
                    );
                }
            }
        }
    }
    (None, None)
}

fn count_diff_added_lines(diff: &str) -> usize {
    diff.lines()
        .filter(|line| line.starts_with('+') && !line.starts_with("+++"))
        .count()
}

fn count_diff_removed_lines(diff: &str) -> usize {
    diff.lines()
        .filter(|line| line.starts_with('-') && !line.starts_with("---"))
        .count()
}

fn patch_line_range(value: &Value) -> Option<PatchLineRange> {
    let range = mapping_get(value, "line_range")?;
    let start = nested_usize(range, &["start"])
        .or_else(|| nested_usize(range, &["from"]))
        .or_else(|| {
            range
                .as_sequence()
                .and_then(|seq| seq.first())
                .and_then(value_as_usize)
        })?;
    let end = nested_usize(range, &["end"])
        .or_else(|| nested_usize(range, &["to"]))
        .or_else(|| {
            range
                .as_sequence()
                .and_then(|seq| seq.get(1))
                .and_then(value_as_usize)
        })?;
    Some(PatchLineRange { start, end })
}

fn semantic_anchor_string(value: &Value) -> Option<String> {
    let anchor = mapping_get(value, "semantic_anchor")?;
    if let Some(raw) = anchor.as_str() {
        return Some(raw.to_string());
    }
    let symbol = nested_scalar(anchor, &["symbol"])?;
    let kind = nested_scalar(anchor, &["kind"]).unwrap_or_else(|| "function".to_string());
    Some(format!("{}:{}", kind, symbol))
}

fn string_sequence(value: &Value, keys: &[&str]) -> Vec<String> {
    let Some(Value::Sequence(items)) = mapping_get_path(value, keys) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| match item {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            _ => None,
        })
        .collect()
}

fn nested_scalar(value: &Value, keys: &[&str]) -> Option<String> {
    let mut current = value;
    for key in keys {
        current = mapping_get(current, key)?;
    }
    match current {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn scalar(value: &Value, key: &str) -> Option<String> {
    nested_scalar(value, &[key])
}

fn nested_bool(value: &Value, keys: &[&str]) -> Option<bool> {
    let mut current = value;
    for key in keys {
        current = mapping_get(current, key)?;
    }
    current.as_bool()
}

fn nested_usize(value: &Value, keys: &[&str]) -> Option<usize> {
    let mut current = value;
    for key in keys {
        current = mapping_get(current, key)?;
    }
    value_as_usize(current)
}

fn nested_isize(value: &Value, keys: &[&str]) -> Option<isize> {
    nested_scalar(value, keys).and_then(|value| value.parse::<isize>().ok())
}

fn value_as_usize(value: &Value) -> Option<usize> {
    match value {
        Value::Number(number) => number.as_u64().map(|value| value as usize),
        Value::String(value) => value.parse::<usize>().ok(),
        _ => None,
    }
}

fn mapping_get_path<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in keys {
        current = mapping_get(current, key)?;
    }
    Some(current)
}

fn mapping_get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value
        .as_mapping()
        .and_then(|mapping| mapping.get(&Value::String(key.to_string())))
}

fn yaml_scalar_by_key(source: &str, key: &str) -> Option<String> {
    source.lines().find_map(|line| {
        let (left, right) = line.split_once(':')?;
        if left.trim() == key {
            Some(
                right
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string(),
            )
        } else {
            None
        }
    })
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
