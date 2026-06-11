//! VAC v1.5 mediation for the real async agent/tool path.
//!
//! `bound_runtime.rs` contains the contract state machine. This module binds
//! that state machine to real `ProposedToolCall` values so the provider loop
//! cannot execute mutating MCP tools before VAC has evaluated plan, artifact,
//! patch, command, and completion-lock gates.

use crate::bound_runtime::{
    BoundRuntimeConfig, BoundRuntimeController, CloseoutState, CommandApproval, CommandRisk,
    FileOperation, GateDecision, GateOutcome, LineRange, NetworkAccess, PatchAttempt,
    PolicyDecision, ReadAccess, RuntimeGate, RuntimeRegistrySnapshot, SemanticAnchor, SemanticPlan,
    SessionArtifacts, StructuredCommand, canonical_json_sha256,
};
use crate::types::{AgentRunContext, ProposedToolCall, ToolDecision};
use serde_json::{Map, Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const VAC_RUNTIME_KEY: &str = "vac_runtime";
const VAC_BOUND_APPROVAL_KEY: &str = "vac_bound_approval";

#[derive(Debug, Clone)]
pub struct BoundToolGate {
    pub outcome: GateOutcome,
    pub stamped_tool_call: Option<ProposedToolCall>,
    pub approval_request: Option<Value>,
}

impl BoundToolGate {
    #[must_use]
    pub fn allowed(tool_call: ProposedToolCall, outcome: GateOutcome) -> Self {
        Self {
            outcome,
            stamped_tool_call: Some(tool_call),
            approval_request: None,
        }
    }

    #[must_use]
    pub fn denied(outcome: GateOutcome) -> Self {
        Self {
            outcome,
            stamped_tool_call: None,
            approval_request: None,
        }
    }

    pub fn approval_required(outcome: GateOutcome, approval_request: Value) -> Self {
        Self {
            outcome,
            stamped_tool_call: None,
            approval_request: Some(approval_request),
        }
    }

    #[must_use]
    pub fn can_execute(&self) -> bool {
        matches!(
            self.outcome.decision,
            GateDecision::Pass | GateDecision::PassWithWarnings
        )
    }

    #[must_use]
    pub fn tool_error_payload(&self) -> String {
        let payload = json!({
            "error": "VAC_BOUND_RUNTIME_GATE_BLOCKED",
            "gate": self.outcome.gate,
            "decision": self.outcome.decision,
            "blockers": self.outcome.blockers,
            "warnings": self.outcome.warnings,
            "operator_action": "approve or revise the Semantic Plan / session artifacts, then retry through VAC runtime",
        });
        payload.to_string()
    }
}

#[derive(Debug)]
pub struct BoundRuntimeToolBoundary {
    controller: Option<BoundRuntimeController>,
    plan: Option<SemanticPlan>,
    workspace_root: PathBuf,
    next_patch_index: u32,
    mutating_tool_seen: bool,
    pending_approval_hashes: BTreeMap<String, String>,
    pending_approval_ids: BTreeMap<String, String>,
    approved_tool_grants: BTreeMap<String, String>,
    trace: Vec<Value>,
}

impl BoundRuntimeToolBoundary {
    #[must_use]
    pub fn from_context_metadata(run: &AgentRunContext, metadata: &mut Value) -> Self {
        ensure_object(metadata);
        ensure_vac_runtime_object(metadata);

        let mut boundary = Self {
            controller: None,
            plan: None,
            workspace_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            next_patch_index: 0,
            mutating_tool_seen: false,
            pending_approval_hashes: BTreeMap::new(),
            pending_approval_ids: BTreeMap::new(),
            approved_tool_grants: BTreeMap::new(),
            trace: Vec::new(),
        };

        let Some(vac_runtime) = metadata.get(VAC_RUNTIME_KEY).cloned() else {
            boundary.push_trace(GateOutcome::block(
                RuntimeGate::PrePlan,
                "vac_runtime metadata missing; mutating tools are fail-closed until Semantic Plan is attached",
            ));
            boundary.flush_trace(metadata);
            return boundary;
        };

        boundary.workspace_root = workspace_root_from_runtime(&vac_runtime);

        let Some(session_id) = vac_runtime
            .get("session_id")
            .and_then(Value::as_str)
            .filter(|item| !item.trim().is_empty())
        else {
            boundary.push_trace(GateOutcome::block(
                RuntimeGate::PrePlan,
                "vac_runtime.session_id missing; mutating tools are fail-closed",
            ));
            boundary.flush_trace(metadata);
            return boundary;
        };

        let registry = match parse_required::<RuntimeRegistrySnapshot>(&vac_runtime, "registry") {
            Ok(registry) => registry,
            Err(outcome) => {
                boundary.push_trace(outcome);
                boundary.flush_trace(metadata);
                return boundary;
            }
        };
        let plan = match parse_required::<SemanticPlan>(&vac_runtime, "plan") {
            Ok(plan) => plan,
            Err(outcome) => {
                boundary.push_trace(outcome);
                boundary.flush_trace(metadata);
                return boundary;
            }
        };
        let artifacts = match parse_required::<SessionArtifacts>(&vac_runtime, "artifacts") {
            Ok(artifacts) => artifacts,
            Err(outcome) => {
                boundary.push_trace(outcome);
                boundary.flush_trace(metadata);
                return boundary;
            }
        };

        let mut controller = BoundRuntimeController::new(
            session_id.to_string(),
            registry,
            BoundRuntimeConfig::default(),
        );
        let requested_l2_claim = vac_runtime
            .get("requested_l2_claim")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let honesty = controller.enforcement_honesty_gate(requested_l2_claim);
        boundary.push_trace(honesty.clone());
        if honesty.is_blocked() {
            boundary.flush_trace(metadata);
            return boundary;
        }

        let plan_gate = controller.approve_plan(plan.clone());
        boundary.push_trace(plan_gate.clone());
        if plan_gate.is_blocked() || matches!(plan_gate.decision, GateDecision::ApprovalRequired) {
            boundary.flush_trace(metadata);
            return boundary;
        }

        let artifact_gate = controller.setup_artifacts(&artifacts);
        boundary.push_trace(artifact_gate.clone());
        if artifact_gate.is_blocked() {
            boundary.flush_trace(metadata);
            return boundary;
        }

        set_vac_runtime_field(
            metadata,
            "active_run_id",
            Value::String(run.run_id.to_string()),
        );
        boundary.plan = Some(plan);
        boundary.controller = Some(controller);
        boundary.flush_trace(metadata);
        boundary
    }

    pub fn gate_tool_call(
        &mut self,
        tool_call: &ProposedToolCall,
        metadata: &mut Value,
    ) -> BoundToolGate {
        ensure_object(metadata);
        ensure_vac_runtime_object(metadata);

        if is_read_tool(&tool_call.name) {
            self.mutating_tool_seen = true;
            let Some(controller) = self.controller.as_mut() else {
                let outcome = GateOutcome::block(
                    RuntimeGate::PrePlan,
                    "read tool attempted without approved VAC Semantic Plan, read-plan ticket, and artifact lock",
                );
                self.push_trace(outcome.clone());
                self.flush_trace(metadata);
                return BoundToolGate::denied(outcome);
            };
            let mut access = match read_access_from_tool_call(tool_call, &self.workspace_root) {
                Ok(access) => access,
                Err(outcome) => {
                    self.push_trace(outcome.clone());
                    self.flush_trace(metadata);
                    return BoundToolGate::denied(outcome);
                }
            };
            access.policy_decision = controller.compute_read_policy_decision(&access);
            let raw_outcome = controller.pre_read_gate(&access);
            let (outcome, approval_request) =
                self.resolve_approval_or_pause(tool_call, raw_outcome, metadata);
            self.push_trace(outcome.clone());
            self.flush_trace(metadata);
            if matches!(
                outcome.decision,
                GateDecision::Pass | GateDecision::PassWithWarnings
            ) {
                return BoundToolGate::allowed(
                    stamp_tool_call(tool_call, &outcome, self.plan.as_ref(), metadata),
                    outcome,
                );
            }
            if let Some(request) = approval_request {
                return BoundToolGate::approval_required(outcome, request);
            }
            return BoundToolGate::denied(outcome);
        }

        if !is_mutating_or_process_tool(&tool_call.name) {
            let mut outcome = GateOutcome::pass(RuntimeGate::PreCommand);
            outcome.push_warning("non-mutating helper remains runtime-observed; governed read/network tools are not allowed to bypass VAC gates");
            self.push_trace(outcome.clone());
            self.flush_trace(metadata);
            return BoundToolGate::allowed(tool_call.clone(), outcome);
        }

        self.mutating_tool_seen = true;
        let Some(controller) = self.controller.as_mut() else {
            let outcome = GateOutcome::block(
                RuntimeGate::PrePlan,
                "mutating/process tool attempted without approved VAC Semantic Plan and artifact lock",
            );
            self.push_trace(outcome.clone());
            self.flush_trace(metadata);
            return BoundToolGate::denied(outcome);
        };

        if is_command_tool(&tool_call.name) {
            let mut command = match command_from_tool_call(tool_call) {
                Ok(command) => command,
                Err(outcome) => {
                    self.push_trace(outcome.clone());
                    self.flush_trace(metadata);
                    return BoundToolGate::denied(outcome);
                }
            };
            command.policy_decision =
                controller.compute_command_policy_decision(self.plan.as_ref(), &command);
            let raw_outcome = controller.pre_command_gate(&command);
            let (outcome, approval_request) =
                self.resolve_approval_or_pause(tool_call, raw_outcome, metadata);
            self.push_trace(outcome.clone());
            self.flush_trace(metadata);
            if matches!(
                outcome.decision,
                GateDecision::Pass | GateDecision::PassWithWarnings
            ) {
                return BoundToolGate::allowed(
                    stamp_tool_call(tool_call, &outcome, self.plan.as_ref(), metadata),
                    outcome,
                );
            }
            if let Some(request) = approval_request {
                return BoundToolGate::approval_required(outcome, request);
            }
            return BoundToolGate::denied(outcome);
        }

        if is_network_tool(&tool_call.name) {
            let mut access = match network_access_from_tool_call(tool_call) {
                Ok(access) => access,
                Err(outcome) => {
                    self.push_trace(outcome.clone());
                    self.flush_trace(metadata);
                    return BoundToolGate::denied(outcome);
                }
            };
            access.policy_decision = controller.compute_network_policy_decision(&access);
            let raw_outcome = controller.pre_network_gate(&access);
            let (outcome, approval_request) =
                self.resolve_approval_or_pause(tool_call, raw_outcome, metadata);
            self.push_trace(outcome.clone());
            self.flush_trace(metadata);
            if matches!(
                outcome.decision,
                GateDecision::Pass | GateDecision::PassWithWarnings
            ) {
                return BoundToolGate::allowed(
                    stamp_tool_call(tool_call, &outcome, self.plan.as_ref(), metadata),
                    outcome,
                );
            }
            if let Some(request) = approval_request {
                return BoundToolGate::approval_required(outcome, request);
            }
            return BoundToolGate::denied(outcome);
        }

        if is_patch_tool(&tool_call.name) {
            let patch = match patch_from_tool_call(
                tool_call,
                self.plan.as_ref(),
                &self.workspace_root,
                self.next_patch_index,
            ) {
                Ok(patch) => patch,
                Err(outcome) => {
                    self.push_trace(outcome.clone());
                    self.flush_trace(metadata);
                    return BoundToolGate::denied(outcome);
                }
            };
            let outcome = controller.pre_patch_gate(&patch);
            self.push_trace(outcome.clone());
            self.flush_trace(metadata);
            if matches!(
                outcome.decision,
                GateDecision::Pass | GateDecision::PassWithWarnings
            ) {
                self.next_patch_index = self.next_patch_index.saturating_add(1);
                return BoundToolGate::allowed(
                    stamp_tool_call(tool_call, &outcome, self.plan.as_ref(), metadata),
                    outcome,
                );
            }
            return BoundToolGate::denied(outcome);
        }

        let outcome = GateOutcome::block(
            RuntimeGate::PreCommand,
            format!("unknown mutating tool is fail-closed: {}", tool_call.name),
        );
        self.push_trace(outcome.clone());
        self.flush_trace(metadata);
        BoundToolGate::denied(outcome)
    }

    fn resolve_approval_or_pause(
        &mut self,
        tool_call: &ProposedToolCall,
        mut outcome: GateOutcome,
        metadata: &mut Value,
    ) -> (GateOutcome, Option<Value>) {
        if !matches!(outcome.decision, GateDecision::ApprovalRequired) {
            return (outcome, None);
        }
        if let Some(binding_hash) = self.approved_tool_grants.remove(&tool_call.id) {
            outcome.decision = GateDecision::PassWithWarnings;
            outcome.push_warning(format!(
                "operator scoped grant applied for VAC approval request; single retry binding={binding_hash}"
            ));
            self.pending_approval_hashes.remove(&tool_call.id);
            self.pending_approval_ids.remove(&tool_call.id);
            return (outcome, None);
        }
        let request = build_approval_request_v2(tool_call, &outcome, self.plan.as_ref(), metadata);
        let binding_hash = request
            .get("binding")
            .and_then(|binding| binding.get("binding_hash"))
            .and_then(Value::as_str)
            .unwrap_or("sha256:unknown")
            .to_string();
        let request_id = request
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("approval.unknown")
            .to_string();
        self.pending_approval_hashes
            .insert(tool_call.id.clone(), binding_hash.clone());
        self.pending_approval_ids
            .insert(tool_call.id.clone(), request_id.clone());
        persist_approval_request(&self.workspace_root, metadata, &request);
        append_runtime_array(metadata, "pending_approval_requests", request.clone());
        outcome.push_warning(format!(
            "operator approval request persisted as {request_id}; tool execution is paused, not denied"
        ));
        (outcome, Some(request))
    }

    pub fn record_operator_tool_decision(
        &mut self,
        tool_call_id: &str,
        decision: &ToolDecision,
        metadata: &mut Value,
    ) {
        ensure_object(metadata);
        ensure_vac_runtime_object(metadata);
        let Some(request_id) = self.pending_approval_ids.get(tool_call_id).cloned() else {
            return;
        };
        let binding_hash = self
            .pending_approval_hashes
            .get(tool_call_id)
            .cloned()
            .unwrap_or_else(|| "sha256:unknown".to_string());
        match decision {
            ToolDecision::Accept => {
                self.approved_tool_grants
                    .insert(tool_call_id.to_string(), binding_hash.clone());
                append_runtime_array(
                    metadata,
                    "approved_scoped_grants",
                    json!({
                        "tool_call_id": tool_call_id,
                        "approval_request_id": request_id,
                        "binding_hash": binding_hash,
                        "grant": "single_retry",
                        "mode": "l1_operator_mediated"
                    }),
                );
                persist_operator_response(
                    &self.workspace_root,
                    metadata,
                    &request_id,
                    tool_call_id,
                    "approved",
                    &binding_hash,
                );
            }
            ToolDecision::Reject | ToolDecision::CustomResult { .. } => {
                self.pending_approval_hashes.remove(tool_call_id);
                self.pending_approval_ids.remove(tool_call_id);
                append_runtime_array(
                    metadata,
                    "denied_scoped_grants",
                    json!({
                        "tool_call_id": tool_call_id,
                        "approval_request_id": request_id,
                        "binding_hash": binding_hash,
                        "decision": "denied_or_custom_result"
                    }),
                );
                persist_operator_response(
                    &self.workspace_root,
                    metadata,
                    &request_id,
                    tool_call_id,
                    "denied",
                    &binding_hash,
                );
            }
        }
        self.flush_trace(metadata);
    }

    pub fn complete_before_run_completed(&mut self, metadata: &mut Value) -> Option<GateOutcome> {
        ensure_object(metadata);
        ensure_vac_runtime_object(metadata);
        let vac_runtime = metadata
            .get(VAC_RUNTIME_KEY)
            .cloned()
            .unwrap_or_else(|| json!({}));
        let Some(controller) = self.controller.as_mut() else {
            if self.mutating_tool_seen {
                let outcome = GateOutcome::block(
                    RuntimeGate::CompletionLock,
                    "mutating tool ran or was attempted without active VAC runtime controller",
                );
                self.push_trace(outcome.clone());
                self.flush_trace(metadata);
                return Some(outcome);
            }
            return None;
        };
        let Some(closeout_value) = vac_runtime.get("closeout").cloned() else {
            if self.mutating_tool_seen {
                let outcome = GateOutcome::block(
                    RuntimeGate::CompletionLock,
                    "completion lock requires closeout state after mutating/process tool activity",
                );
                self.push_trace(outcome.clone());
                self.flush_trace(metadata);
                return Some(outcome);
            }
            return None;
        };
        let closeout = match serde_json::from_value::<CloseoutState>(closeout_value) {
            Ok(closeout) => closeout,
            Err(err) => {
                let outcome = GateOutcome::block(
                    RuntimeGate::CompletionLock,
                    format!("invalid closeout state: {err}"),
                );
                self.push_trace(outcome.clone());
                self.flush_trace(metadata);
                return Some(outcome);
            }
        };
        let result = controller.complete_session(&closeout);
        let mut outcome = GateOutcome::pass(RuntimeGate::CompletionLock);
        outcome.blockers = result.blockers;
        outcome.warnings = result.warnings;
        outcome.decision = match result.disposition {
            crate::bound_runtime::CompletionDisposition::Done => {
                if outcome.warnings.is_empty() {
                    GateDecision::Pass
                } else {
                    GateDecision::PassWithWarnings
                }
            }
            crate::bound_runtime::CompletionDisposition::PausedForDiscussion => {
                GateDecision::NeedsDiscussion
            }
            crate::bound_runtime::CompletionDisposition::Blocked => GateDecision::Block,
        };
        self.push_trace(outcome.clone());
        self.flush_trace(metadata);
        if matches!(
            outcome.decision,
            GateDecision::Block | GateDecision::NeedsDiscussion
        ) {
            Some(outcome)
        } else {
            None
        }
    }

    fn push_trace(&mut self, outcome: GateOutcome) {
        self.trace.push(json!({
            "gate": outcome.gate,
            "decision": outcome.decision,
            "blockers": outcome.blockers,
            "warnings": outcome.warnings,
        }));
    }

    fn flush_trace(&mut self, metadata: &mut Value) {
        ensure_object(metadata);
        ensure_vac_runtime_object(metadata);
        let Some(obj) = metadata
            .get_mut(VAC_RUNTIME_KEY)
            .and_then(Value::as_object_mut)
        else {
            return;
        };
        let trace = obj
            .entry("trace".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        if let Some(items) = trace.as_array_mut() {
            items.append(&mut self.trace);
        }
    }
}

fn parse_required<T>(runtime: &Value, key: &str) -> Result<T, GateOutcome>
where
    T: serde::de::DeserializeOwned,
{
    let Some(value) = runtime.get(key).cloned() else {
        return Err(GateOutcome::block(
            RuntimeGate::PrePlan,
            format!("vac_runtime.{key} missing; runtime is fail-closed"),
        ));
    };
    serde_json::from_value(value).map_err(|err| {
        GateOutcome::block(
            RuntimeGate::PrePlan,
            format!("vac_runtime.{key} invalid: {err}"),
        )
    })
}

fn ensure_object(value: &mut Value) {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
}

fn ensure_vac_runtime_object(value: &mut Value) {
    if let Some(obj) = value.as_object_mut() {
        obj.entry(VAC_RUNTIME_KEY.to_string())
            .or_insert_with(|| Value::Object(Map::new()));
    }
}

fn set_vac_runtime_field(metadata: &mut Value, key: &str, value: Value) {
    ensure_vac_runtime_object(metadata);
    if let Some(obj) = metadata
        .get_mut(VAC_RUNTIME_KEY)
        .and_then(Value::as_object_mut)
    {
        obj.insert(key.to_string(), value);
    }
}

fn is_command_tool(name: &str) -> bool {
    matches!(
        name,
        "run_command" | "run_command_task" | "run_remote_command" | "run_remote_command_task"
    )
}

fn is_patch_tool(name: &str) -> bool {
    matches!(name, "create" | "str_replace" | "remove")
}

fn is_network_tool(name: &str) -> bool {
    matches!(name, "view_web_page")
}

fn is_read_tool(name: &str) -> bool {
    matches!(name, "view")
}

fn is_mutating_or_process_tool(name: &str) -> bool {
    is_command_tool(name)
        || is_patch_tool(name)
        || is_network_tool(name)
        || is_read_tool(name)
        || matches!(
            name,
            "generate_code" | "dynamic_subagent_task" | "cancel_task"
        )
}

fn workspace_root_from_runtime(runtime: &Value) -> PathBuf {
    runtime
        .get("workspace_root")
        .and_then(Value::as_str)
        .filter(|item| !item.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn command_from_tool_call(tool_call: &ProposedToolCall) -> Result<StructuredCommand, GateOutcome> {
    if let Some(value) = tool_call.arguments.get("structured_command") {
        return structured_command_from_value(value, &tool_call.name);
    }
    if tool_call.arguments.get("runner").is_some()
        || tool_call.arguments.get("args").is_some()
        || tool_call.arguments.get("command_id").is_some()
    {
        return structured_command_from_value(&tool_call.arguments, &tool_call.name);
    }
    if tool_call
        .arguments
        .get("command")
        .and_then(Value::as_str)
        .is_some()
    {
        return Err(GateOutcome::block(
            RuntimeGate::PreCommand,
            "free-form command string is not runtime authority; command tools must send structured_command {id, runner, args, risk, approval}",
        ));
    }
    Err(GateOutcome::block(
        RuntimeGate::PreCommand,
        "command tool call missing structured_command object",
    ))
}

fn structured_command_from_value(
    value: &Value,
    tool_name: &str,
) -> Result<StructuredCommand, GateOutcome> {
    let Some(obj) = value.as_object() else {
        return Err(GateOutcome::block(
            RuntimeGate::PreCommand,
            "structured_command must be a JSON object",
        ));
    };
    let id = obj
        .get("id")
        .or_else(|| obj.get("command_id"))
        .and_then(Value::as_str)
        .filter(|item| !item.trim().is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("tool.{tool_name}.structured"));
    let Some(runner) = obj
        .get("runner")
        .and_then(Value::as_str)
        .filter(|item| !item.trim().is_empty())
    else {
        return Err(GateOutcome::block(
            RuntimeGate::PreCommand,
            "structured command missing runner",
        ));
    };
    let args = match obj.get("args") {
        Some(Value::Array(items)) => {
            let mut out = Vec::new();
            for item in items {
                let Some(arg) = item.as_str() else {
                    return Err(GateOutcome::block(
                        RuntimeGate::PreCommand,
                        "structured command args must be string array",
                    ));
                };
                out.push(arg.to_string());
            }
            out
        }
        Some(_) => {
            return Err(GateOutcome::block(
                RuntimeGate::PreCommand,
                "structured command args must be string array",
            ));
        }
        None => Vec::new(),
    };
    if obj.contains_key("policy_decision") || obj.contains_key("policy") {
        return Err(GateOutcome::block(
            RuntimeGate::PreCommand,
            "tool-supplied policy_decision is not authority; VAC computes policy from compiled JSON snapshots",
        ));
    }
    let risk = obj
        .get("risk")
        .and_then(Value::as_str)
        .map(command_risk_from_str)
        .transpose()?
        .unwrap_or_else(|| inferred_command_risk(runner, &args));
    let approval = obj
        .get("approval")
        .and_then(Value::as_str)
        .map(command_approval_from_str)
        .transpose()?
        .unwrap_or(CommandApproval::Policy);

    Ok(StructuredCommand {
        id,
        runner: runner.to_string(),
        args,
        risk,
        approval,
        policy_decision: PolicyDecision::Deny,
    })
}

fn command_risk_from_str(raw: &str) -> Result<CommandRisk, GateOutcome> {
    match raw {
        "safe_read" => Ok(CommandRisk::SafeRead),
        "low" => Ok(CommandRisk::Low),
        "medium" => Ok(CommandRisk::Medium),
        "high" => Ok(CommandRisk::High),
        "critical" | "destructive" => Ok(CommandRisk::Critical),
        "execute_process" => Ok(CommandRisk::ExecuteProcess),
        other => Err(GateOutcome::block(
            RuntimeGate::PreCommand,
            format!("unknown structured command risk: {other}"),
        )),
    }
}

fn command_approval_from_str(raw: &str) -> Result<CommandApproval, GateOutcome> {
    match raw {
        "never" => Ok(CommandApproval::Never),
        "policy" => Ok(CommandApproval::Policy),
        "always" => Ok(CommandApproval::Always),
        other => Err(GateOutcome::block(
            RuntimeGate::PreCommand,
            format!("unknown structured command approval: {other}"),
        )),
    }
}

fn read_access_from_tool_call(
    tool_call: &ProposedToolCall,
    workspace_root: &Path,
) -> Result<ReadAccess, GateOutcome> {
    let Some(path) = tool_call
        .arguments
        .get("path")
        .and_then(Value::as_str)
        .filter(|item| !item.trim().is_empty())
    else {
        return Err(GateOutcome::block(
            RuntimeGate::PreRead,
            "view tool call missing path",
        ));
    };
    if tool_call.arguments.get("policy_decision").is_some()
        || tool_call.arguments.get("policy").is_some()
    {
        return Err(GateOutcome::block(
            RuntimeGate::PreRead,
            "tool-supplied read policy decision is not authority; VAC computes filesystem_read/network_access from compiled JSON",
        ));
    }
    let credential_material_present = tool_call
        .arguments
        .get("password")
        .and_then(Value::as_str)
        .is_some_and(|v| !v.is_empty())
        || tool_call
            .arguments
            .get("private_key_path")
            .and_then(Value::as_str)
            .is_some_and(|v| !v.is_empty());
    let read_plan_ticket = tool_call
        .arguments
        .get("read_plan_ticket")
        .and_then(|value| {
            value.as_str().map(ToString::to_string).or_else(|| {
                value
                    .get("id")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            })
        });
    if let Some(ticket) = read_plan_ticket.as_deref() {
        validate_read_plan_ticket(workspace_root, ticket, path)?;
    }
    let remote = is_remote_path_hint(path);
    let (host, protocol) = if remote {
        (remote_host_hint(path), Some("ssh".to_string()))
    } else {
        (None, Some("local".to_string()))
    };
    Ok(ReadAccess {
        id: read_plan_ticket
            .clone()
            .unwrap_or_else(|| format!("tool.view.read.{}", stable_suffix(path))),
        path: path.to_string(),
        action: if remote {
            "network_access".to_string()
        } else {
            "filesystem_read".to_string()
        },
        host,
        protocol,
        read_plan_ticket,
        credential_material_present,
        grep: tool_call.arguments.get("grep").is_some(),
        glob: tool_call.arguments.get("glob").is_some(),
        remote,
        risk: if remote || credential_material_present {
            CommandRisk::High
        } else {
            CommandRisk::SafeRead
        },
        approval: CommandApproval::Policy,
        policy_decision: PolicyDecision::Deny,
    })
}

fn validate_read_plan_ticket(
    workspace_root: &Path,
    ticket: &str,
    requested_path: &str,
) -> Result<(), GateOutcome> {
    let read_plans = workspace_root.join(".vac/index/read_plans.jsonl");
    let Ok(contents) = fs::read_to_string(&read_plans) else {
        return Err(GateOutcome::block(
            RuntimeGate::PreRead,
            "read_plan_ticket supplied but .vac/index/read_plans.jsonl is unavailable",
        ));
    };
    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(row) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let id_match = row
            .get("ticket_id")
            .or_else(|| row.get("id"))
            .and_then(Value::as_str)
            == Some(ticket);
        let path_match = row
            .get("path")
            .and_then(Value::as_str)
            .is_some_and(|path| path == requested_path);
        if id_match && path_match {
            return Ok(());
        }
    }
    Err(GateOutcome::block(
        RuntimeGate::PreRead,
        "read_plan_ticket does not resolve to the requested path in the deterministic index",
    ))
}

fn is_remote_path_hint(path: &str) -> bool {
    path.starts_with("ssh://") || (path.contains('@') && path.contains(':'))
}

fn remote_host_hint(path: &str) -> Option<String> {
    if let Some(rest) = path.strip_prefix("ssh://") {
        return rest
            .split('/')
            .next()
            .and_then(|auth| auth.rsplit('@').next())
            .map(|host| host.split(':').next().unwrap_or(host).to_string());
    }
    path.split('@')
        .nth(1)
        .map(|rest| rest.split(':').next().unwrap_or(rest).to_string())
}

fn stable_suffix(input: &str) -> String {
    let mut out = input
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .take(24)
        .collect::<String>();
    if out.is_empty() {
        out = "path".to_string();
    }
    out
}

fn network_access_from_tool_call(
    tool_call: &ProposedToolCall,
) -> Result<NetworkAccess, GateOutcome> {
    let Some(url) = tool_call
        .arguments
        .get("url")
        .and_then(Value::as_str)
        .filter(|item| !item.trim().is_empty())
    else {
        return Err(GateOutcome::block(
            RuntimeGate::PreNetwork,
            "network tool call missing url",
        ));
    };
    let (protocol, host) = split_https_url(url)?;
    let policy_source = tool_call
        .arguments
        .get("network_access")
        .unwrap_or(&tool_call.arguments);
    if policy_source.get("policy_decision").is_some() || policy_source.get("policy").is_some() {
        return Err(GateOutcome::block(
            RuntimeGate::PreNetwork,
            "tool-supplied network policy decision is not authority; VAC computes network_access from compiled JSON snapshots",
        ));
    }
    let approval = policy_source
        .get("approval")
        .and_then(Value::as_str)
        .map(command_approval_from_str)
        .transpose()?
        .unwrap_or(CommandApproval::Policy);
    let risk = policy_source
        .get("risk")
        .and_then(Value::as_str)
        .map(command_risk_from_str)
        .transpose()?
        .unwrap_or(CommandRisk::Medium);
    Ok(NetworkAccess {
        id: policy_source
            .get("id")
            .and_then(Value::as_str)
            .filter(|item| !item.trim().is_empty())
            .unwrap_or("tool.view_web_page.network_read")
            .to_string(),
        url: url.to_string(),
        host,
        protocol,
        risk,
        approval,
        policy_decision: PolicyDecision::Deny,
    })
}

fn split_https_url(url: &str) -> Result<(String, String), GateOutcome> {
    let Some(rest) = url.strip_prefix("https://") else {
        return Err(GateOutcome::block(
            RuntimeGate::PreNetwork,
            "network read must use https:// URL",
        ));
    };
    let host = rest
        .split('/')
        .next()
        .unwrap_or("")
        .split('@')
        .next_back()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");
    if host.trim().is_empty() {
        return Err(GateOutcome::block(
            RuntimeGate::PreNetwork,
            "network read URL host is empty",
        ));
    }
    Ok(("https".to_string(), host.to_string()))
}

fn inferred_command_risk(runner: &str, args: &[String]) -> CommandRisk {
    if is_destructive(runner, args) {
        CommandRisk::Critical
    } else if matches!(
        runner,
        "cargo" | "rustc" | "python" | "python3" | "node" | "pnpm" | "npm" | "git" | "vac"
    ) {
        CommandRisk::ExecuteProcess
    } else {
        CommandRisk::Low
    }
}

fn patch_from_tool_call(
    tool_call: &ProposedToolCall,
    plan: Option<&SemanticPlan>,
    workspace_root: &Path,
    patch_index: u32,
) -> Result<PatchAttempt, GateOutcome> {
    let Some(path) = tool_call.arguments.get("path").and_then(Value::as_str) else {
        return Err(GateOutcome::block(
            RuntimeGate::PrePatch,
            "patch tool call missing path",
        ));
    };
    let operation = match tool_call.name.as_str() {
        "create" => FileOperation::Create,
        "str_replace" => FileOperation::Modify,
        "remove" => FileOperation::Delete,
        other => {
            return Err(GateOutcome::block(
                RuntimeGate::PrePatch,
                format!("not a bounded patch tool: {other}"),
            ));
        }
    };
    let Some(scope) = plan.and_then(|plan| plan.allowed_scope(path)) else {
        return Err(GateOutcome::block(
            RuntimeGate::PrePatch,
            format!("file is outside active Semantic Plan: {path}"),
        ));
    };
    let (touched_range, semantic_anchor, lines_added, lines_removed, creates_new_file) =
        match tool_call.name.as_str() {
            "create" => {
                let file_text = tool_call
                    .arguments
                    .get("file_text")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let added = count_lines(Some(file_text));
                let range = LineRange {
                    start: 1,
                    end: added.max(1),
                };
                (
                    range,
                    semantic_anchor_for_new_file(path, file_text, &scope.semantic_anchor),
                    added,
                    0,
                    true,
                )
            }
            "str_replace" => {
                reject_untrusted_patch_preimage_fields(tool_call)?;
                if tool_call
                    .arguments
                    .get("replace_all")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    return Err(GateOutcome::block(
                        RuntimeGate::PrePatch,
                        "replace_all is a separate explicitly bounded operation and cannot share single-range patch approval",
                    ));
                }
                let Some(old_str) = tool_call.arguments.get("old_str").and_then(Value::as_str)
                else {
                    return Err(GateOutcome::block(
                        RuntimeGate::PrePatch,
                        "str_replace missing old_str",
                    ));
                };
                let new_str = tool_call
                    .arguments
                    .get("new_str")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let file_text = load_patch_file_text(workspace_root, path)?;
                let matches = find_all_matches(&file_text, old_str);
                match matches.as_slice() {
                    [] => {
                        return Err(GateOutcome::block(
                            RuntimeGate::PrePatch,
                            "old_str has zero matches in current file; plan refresh required",
                        ));
                    }
                    [_first, _second, ..] => {
                        return Err(GateOutcome::block(
                            RuntimeGate::PrePatch,
                            "old_str has multiple matches in current file; Semantic Plan must disambiguate range/anchor",
                        ));
                    }
                    [byte_start] => {
                        let byte_end = byte_start.saturating_add(old_str.len());
                        let range = byte_range_to_line_range(&file_text, *byte_start, byte_end);
                        let anchor = resolve_actual_semantic_anchor(
                            path,
                            &file_text,
                            range,
                            &scope.semantic_anchor,
                        );
                        (
                            range,
                            anchor,
                            count_lines(Some(new_str)),
                            count_lines(Some(old_str)),
                            false,
                        )
                    }
                }
            }
            "remove" => {
                let file_text = load_patch_file_text(workspace_root, path)?;
                let total = count_lines(Some(&file_text));
                let range = LineRange {
                    start: 1,
                    end: total.max(1),
                };
                let anchor =
                    resolve_actual_semantic_anchor(path, &file_text, range, &scope.semantic_anchor);
                (range, anchor, 0, total, false)
            }
            _ => unreachable!(),
        };
    Ok(PatchAttempt {
        path: path.to_string(),
        operation,
        touched_range,
        semantic_anchor,
        lines_added,
        lines_removed,
        creates_new_file,
        patch_index,
    })
}

fn reject_untrusted_patch_preimage_fields(tool_call: &ProposedToolCall) -> Result<(), GateOutcome> {
    for field in [
        "current_file_text",
        "file_text_before",
        "preimage",
        "file_text_snapshot",
    ] {
        if tool_call.arguments.get(field).is_some() {
            return Err(GateOutcome::block(
                RuntimeGate::PrePatch,
                format!(
                    "tool-supplied patch preimage field `{field}` is not trusted; broker reads actual workspace storage"
                ),
            ));
        }
    }
    Ok(())
}

fn load_patch_file_text(workspace_root: &Path, path: &str) -> Result<String, GateOutcome> {
    let full_path = workspace_root.join(path);
    fs::read_to_string(&full_path).map_err(|err| {
        GateOutcome::block(
            RuntimeGate::PrePatch,
            format!("cannot resolve actual edit range for {path}: {err}; VAC refuses plan-echo patch gating"),
        )
    })
}

fn find_all_matches(haystack: &str, needle: &str) -> Vec<usize> {
    if needle.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut cursor = 0;
    while let Some(idx) = haystack[cursor..].find(needle) {
        let absolute = cursor + idx;
        out.push(absolute);
        cursor = absolute + needle.len();
    }
    out
}

fn byte_range_to_line_range(text: &str, start: usize, end: usize) -> LineRange {
    let mut start_line = 1u32;
    let mut end_line = 1u32;
    for (idx, ch) in text.char_indices() {
        if idx < start && ch == '\n' {
            start_line += 1;
        }
        if idx < end && ch == '\n' {
            end_line += 1;
        }
        if idx >= end {
            break;
        }
    }
    LineRange {
        start: start_line,
        end: end_line.max(start_line),
    }
}

fn semantic_anchor_for_new_file(
    path: &str,
    text: &str,
    expected: &SemanticAnchor,
) -> SemanticAnchor {
    resolve_actual_semantic_anchor(
        path,
        text,
        LineRange {
            start: 1,
            end: count_lines(Some(text)).max(1),
        },
        expected,
    )
}

fn resolve_actual_semantic_anchor(
    path: &str,
    text: &str,
    touched: LineRange,
    expected: &SemanticAnchor,
) -> SemanticAnchor {
    let detected = detect_rust_anchor(text, touched).unwrap_or_else(|| expected.clone());
    let ast_path = detected
        .ast_path
        .or_else(|| Some(format!("file::{path}::{}", detected.symbol)));
    let normalized_fingerprint = Some(normalized_anchor_fingerprint(
        text,
        touched,
        &detected.symbol,
    ));
    SemanticAnchor {
        symbol: detected.symbol,
        kind: detected.kind,
        ast_path,
        normalized_fingerprint,
    }
}

fn detect_rust_anchor(text: &str, touched: LineRange) -> Option<SemanticAnchor> {
    let mut best: Option<(u32, SemanticAnchor)> = None;
    for (idx, line) in text.lines().enumerate() {
        let line_no = idx as u32 + 1;
        if line_no > touched.start {
            break;
        }
        if let Some((kind, symbol)) = parse_rust_symbol(line) {
            best = Some((
                line_no,
                SemanticAnchor {
                    symbol: symbol.to_string(),
                    kind: kind.to_string(),
                    ast_path: Some(format!("rust::{kind}::{symbol}")),
                    normalized_fingerprint: None,
                },
            ));
        }
    }
    best.map(|(_, anchor)| anchor)
}

fn parse_rust_symbol(line: &str) -> Option<(&'static str, &str)> {
    let trimmed = line.trim_start();
    let normalized = trimmed.strip_prefix("pub ").unwrap_or(trimmed);
    for (prefix, kind) in [
        ("async fn ", "function"),
        ("fn ", "function"),
        ("struct ", "struct"),
        ("enum ", "enum"),
        ("trait ", "trait"),
        ("impl ", "impl"),
        ("mod ", "module"),
    ] {
        if let Some(rest) = normalized.strip_prefix(prefix) {
            let symbol = rest
                .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                .next()
                .unwrap_or("");
            if !symbol.is_empty() {
                return Some((kind, symbol));
            }
        }
    }
    None
}

fn normalized_anchor_fingerprint(text: &str, touched: LineRange, symbol: &str) -> String {
    let start = touched.start.saturating_sub(3).max(1);
    let end = touched.end.saturating_add(3);
    let mut normalized = String::new();
    for (idx, line) in text.lines().enumerate() {
        let line_no = idx as u32 + 1;
        if line_no >= start && line_no <= end {
            normalized.push_str(line.trim());
            normalized.push('\n');
        }
    }
    canonical_json_sha256(&json!({"symbol": symbol, "window": normalized}))
}

fn count_lines(text: Option<&str>) -> u32 {
    text.map(|item| item.lines().count().max(1) as u32)
        .unwrap_or(1)
}

fn tool_arguments_without_bound_approval(tool_call: &ProposedToolCall) -> Value {
    let mut arguments = tool_call.arguments.clone();
    if let Some(obj) = arguments.as_object_mut() {
        obj.remove(VAC_BOUND_APPROVAL_KEY);
    }
    arguments
}

fn build_approval_request_v2(
    tool_call: &ProposedToolCall,
    outcome: &GateOutcome,
    plan: Option<&SemanticPlan>,
    metadata: &Value,
) -> Value {
    let (action, target, read_plan_ticket) = approval_binding_scope(tool_call);
    let session_id = session_id_from_metadata(metadata);
    let capability = plan
        .map(|plan| plan.capability.clone())
        .unwrap_or_else(|| "unknown-capability".to_string());
    let plan_id = plan
        .map(|plan| plan.id.clone())
        .unwrap_or_else(|| "unknown-plan".to_string());
    let plan_hash = plan
        .and_then(|plan| serde_json::to_value(plan).ok())
        .map(|value| canonical_json_sha256(&value))
        .unwrap_or_else(|| "sha256:missing-plan".to_string());
    let diff_hash = canonical_json_sha256(&json!({
        "tool_call_id": tool_call.id,
        "tool_name": tool_call.name,
        "action": action,
        "target": target,
        "arguments": tool_arguments_without_bound_approval(tool_call),
        "gate": outcome.gate,
    }));
    let policy_snapshot_hash = metadata
        .get(VAC_RUNTIME_KEY)
        .and_then(|runtime| runtime.get("registry"))
        .map(canonical_json_sha256)
        .unwrap_or_else(|| "sha256:missing-registry".to_string());
    let nonce = canonical_json_sha256(&json!({
        "session_id": session_id,
        "tool_call_id": tool_call.id,
        "diff_hash": diff_hash,
        "policy_snapshot_hash": policy_snapshot_hash,
        "mode": "l1_operator_mediated"
    }));
    let binding_hash = canonical_json_sha256(&json!({
        "plan_hash": plan_hash,
        "diff_hash": diff_hash,
        "policy_snapshot_hash": policy_snapshot_hash,
        "nonce": nonce,
    }));
    let request_id = format!(
        "approval.{}.{}.{}",
        stable_suffix(&session_id),
        stable_suffix(&tool_call.id),
        binding_hash
            .trim_start_matches("sha256:")
            .chars()
            .take(12)
            .collect::<String>()
    );
    json!({
        "schema_version": 2,
        "kind": "approval_request",
        "id": request_id,
        "status": "pending",
        "request": {
            "action": action,
            "scope": {
                "target": target,
                "tool_call_id": tool_call.id,
                "tool_name": tool_call.name,
                "read_plan_ticket": read_plan_ticket,
            },
            "risk_level": "policy_controlled",
            "capability": capability,
            "plan_id": plan_id,
            "rationale": "VAC runtime policy returned ApprovalRequired; execution is paused until an operator grants this exact binding."
        },
        "binding": {
            "plan_hash": plan_hash,
            "diff_hash": diff_hash,
            "policy_snapshot_hash": policy_snapshot_hash,
            "binding_hash": binding_hash,
            "nonce": nonce,
            "expires_at": "l1-session-scoped"
        },
        "response": null,
        "runtime_gate": outcome.gate,
        "warnings": outcome.warnings,
        "blockers": outcome.blockers,
        "mode": "l1_operator_mediated"
    })
}

fn persist_approval_request(workspace_root: &Path, metadata: &Value, request: &Value) {
    let session_id = stable_suffix(&session_id_from_metadata(metadata));
    let dir = workspace_root
        .join(".vac/registry/approvals")
        .join(session_id);
    let _ = fs::create_dir_all(&dir);
    let Some(id) = request.get("id").and_then(Value::as_str) else {
        return;
    };
    if let Ok(payload) = serde_json::to_string_pretty(request) {
        let _ = fs::write(
            dir.join(format!("{id}.json")),
            format!(
                "{payload}
"
            ),
        );
    }
}

fn persist_operator_response(
    workspace_root: &Path,
    metadata: &Value,
    request_id: &str,
    tool_call_id: &str,
    decision: &str,
    binding_hash: &str,
) {
    let session_id = stable_suffix(&session_id_from_metadata(metadata));
    let dir = workspace_root
        .join(".vac/registry/approvals")
        .join(session_id);
    let _ = fs::create_dir_all(&dir);
    let payload = json!({
        "schema_version": 2,
        "kind": "approval_response",
        "approval_request_id": request_id,
        "tool_call_id": tool_call_id,
        "decision": decision,
        "binding_hash": binding_hash,
        "mode": "l1_operator_mediated",
        "grant": if decision == "approved" { "single_retry" } else { "none" },
    });
    if let Ok(raw) = serde_json::to_string_pretty(&payload) {
        let _ = fs::write(
            dir.join(format!("{request_id}.{decision}.json")),
            format!(
                "{raw}
"
            ),
        );
    }
}

fn session_id_from_metadata(metadata: &Value) -> String {
    metadata
        .get(VAC_RUNTIME_KEY)
        .and_then(|runtime| runtime.get("session_id"))
        .and_then(Value::as_str)
        .filter(|item| !item.trim().is_empty())
        .unwrap_or("unknown-session")
        .to_string()
}

fn append_runtime_array(metadata: &mut Value, key: &str, item: Value) {
    ensure_object(metadata);
    ensure_vac_runtime_object(metadata);
    let Some(runtime) = metadata
        .get_mut(VAC_RUNTIME_KEY)
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    let entry = runtime
        .entry(key.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !entry.is_array() {
        *entry = Value::Array(Vec::new());
    }
    if let Some(items) = entry.as_array_mut() {
        items.push(item);
    }
}

fn stamp_tool_call(
    tool_call: &ProposedToolCall,
    outcome: &GateOutcome,
    plan: Option<&SemanticPlan>,
    metadata: &Value,
) -> ProposedToolCall {
    let mut stamped = tool_call.clone();
    let request = build_approval_request_v2(tool_call, outcome, plan, metadata);
    let (action, target, read_plan_ticket) = approval_binding_scope(tool_call);
    let session_id = session_id_from_metadata(metadata);
    let capability = plan
        .map(|plan| plan.capability.clone())
        .unwrap_or_else(|| "unknown-capability".to_string());
    let binding = request.get("binding").and_then(Value::as_object);
    let approval = json!({
        "schema_version": 2,
        "kind": "vac_bound_tool_approval",
        "approval_request_id": request.get("id").and_then(Value::as_str).unwrap_or("approval.unknown"),
        "tool_call_id": tool_call.id,
        "tool_name": tool_call.name,
        "gate": outcome.gate,
        "decision": outcome.decision,
        "mode": "l1_runtime_mediated",
        "action": action,
        "target": target,
        "session_id": session_id,
        "capability": capability,
        "read_plan_ticket": read_plan_ticket,
        "plan_hash": binding.and_then(|map| map.get("plan_hash")).and_then(Value::as_str).unwrap_or("sha256:missing-plan"),
        "diff_hash": binding.and_then(|map| map.get("diff_hash")).and_then(Value::as_str).unwrap_or("sha256:missing-diff"),
        "policy_snapshot_hash": binding.and_then(|map| map.get("policy_snapshot_hash")).and_then(Value::as_str).unwrap_or("sha256:missing-registry"),
        "nonce": binding.and_then(|map| map.get("nonce")).and_then(Value::as_str).unwrap_or("sha256:missing-nonce"),
        "expires_at": binding.and_then(|map| map.get("expires_at")).and_then(Value::as_str).unwrap_or("l1-session-scoped"),
        "binding_hash": binding.and_then(|map| map.get("binding_hash")).and_then(Value::as_str).unwrap_or("sha256:missing-binding"),
        "operator_sig": {"algorithm": "none", "mode": "l1_operator_mediated_integrity_hint"},
        "broker_sig": {"algorithm": "none", "mode": "l1_runtime_mediated_integrity_hint"},
    });
    if let Some(obj) = stamped.arguments.as_object_mut() {
        obj.insert(VAC_BOUND_APPROVAL_KEY.to_string(), approval);
    }
    stamped
}

fn approval_binding_scope(tool_call: &ProposedToolCall) -> (String, String, Option<String>) {
    if is_command_tool(&tool_call.name) {
        let command = tool_call
            .arguments
            .get("command")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .or_else(|| {
                tool_call
                    .arguments
                    .get("structured_command")
                    .map(|value| value.to_string())
            })
            .unwrap_or_else(|| tool_call.name.clone());
        let action = match tool_call.name.as_str() {
            "run_remote_command" => "remote_execute_process",
            "run_command_task" => "execute_process_task",
            "run_remote_command_task" => "remote_execute_process_task",
            _ => "execute_process",
        };
        return (action.to_string(), command, None);
    }
    if is_network_tool(&tool_call.name) {
        let target = tool_call
            .arguments
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        return ("network_access".to_string(), target, None);
    }
    if is_read_tool(&tool_call.name) {
        let target = tool_call
            .arguments
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let ticket = tool_call
            .arguments
            .get("read_plan_ticket")
            .and_then(|value| {
                value.as_str().map(ToString::to_string).or_else(|| {
                    value
                        .get("id")
                        .and_then(Value::as_str)
                        .map(ToString::to_string)
                })
            });
        return ("filesystem_read".to_string(), target, ticket);
    }
    if is_patch_tool(&tool_call.name) {
        let target = tool_call
            .arguments
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let action = if tool_call.name == "remove" {
            "filesystem_delete"
        } else {
            "filesystem_write"
        };
        return (action.to_string(), target, None);
    }
    ("tool_execute".to_string(), tool_call.name.clone(), None)
}

fn is_destructive(runner: &str, args: &[String]) -> bool {
    matches!(runner, "rm" | "del" | "rmdir")
        || (runner == "cargo" && args.iter().any(|arg| arg == "clean"))
}
