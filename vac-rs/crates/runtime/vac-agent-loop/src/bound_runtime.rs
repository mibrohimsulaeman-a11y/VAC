//! VAC v1.9 bounded runtime projection contract.
//!
//! This module is intentionally independent from provider I/O. It models the
//! runtime behavior that must surround an agent run: compiled-registry intake,
//! pre-plan/pre-patch/pre-command gates, runtime-journal projections, evidence
//! close-out, and the compatibility completion lock. The existing async agent loop can
//! remain provider/tool focused while broker or CLI/TUI layers instantiate this
//! controller as the execution boundary.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

const HASH_PREFIX: &str = "sha256:";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundRuntimePhase {
    IntentPlan,
    SetupArtifacts,
    AgentProcess,
    Validate,
    UpdateVac,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Initializing,
    Planning,
    ArtifactsSetup,
    Executing,
    Validating,
    Closing,
    Done,
    PausedForDiscussion,
    Abandoned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeConformanceLevel {
    L1,
    L2,
}

impl RuntimeConformanceLevel {
    #[must_use]
    pub fn guarantee_label(self) -> &'static str {
        match self {
            Self::L1 => "L1 advisory: discipline + audit only; no out-of-band syscall guarantee",
            Self::L2 => "L2 enforcing: broker-mediated tool API with OS sandbox boundary",
        }
    }

    #[must_use]
    pub fn may_claim_fail_closed_enforcement(self) -> bool {
        matches!(self, Self::L2)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessLevel {
    Deprecated,
    Planned,
    Partial,
    Ready,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadinessTriplet {
    pub declared: ReadinessLevel,
    pub computed: ReadinessLevel,
    pub effective: ReadinessLevel,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub blockers: Vec<String>,
}

impl ReadinessTriplet {
    #[must_use]
    pub fn fail_closed(mut self) -> Self {
        // Keep the runtime semantics byte-for-byte aligned with vac-readiness:
        // effective must be no stronger than the weakest of operator-declared
        // readiness and computed gate readiness.  The duplicate local enum is
        // retained only to avoid a cross-crate type conversion inside the hot
        // agent loop; the rule is intentionally identical to vac-readiness.
        let weakest = self.declared.min(self.computed);
        if self.effective > weakest {
            self.blockers
                .push("effective_lowered_to_declared_computed_weakest".to_string());
            self.effective = weakest;
        }
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeAuthority {
    CompiledJson,
    RawYaml,
    Unknown,
}

impl RuntimeAuthority {
    #[must_use]
    pub fn is_executable(self) -> bool {
        matches!(self, Self::CompiledJson)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRegistrySnapshot {
    pub authority: RuntimeAuthority,
    pub compiled_snapshot_current: bool,
    #[serde(default)]
    pub source_hashes: Vec<SourceHash>,
    #[serde(default)]
    pub capabilities: Vec<CapabilityRuntimeRecord>,
    #[serde(default)]
    pub deterministic_index_current: bool,
    #[serde(default)]
    pub policy_loaded: bool,
    #[serde(default)]
    pub policy_rules: Vec<CompiledPolicyRule>,
    #[serde(default)]
    pub policy_snapshot: Option<vac_policy::PolicySnapshot>,
    #[serde(default)]
    pub command_registry: Vec<CommandRuntimeRecord>,
    #[serde(default)]
    pub read_plan_tickets: Vec<ReadPlanTicketRecord>,
    pub conformance_level: RuntimeConformanceLevel,
}

impl RuntimeRegistrySnapshot {
    #[must_use]
    pub fn capability(&self, id: &str) -> Option<&CapabilityRuntimeRecord> {
        self.capabilities
            .iter()
            .find(|capability| capability.id == id)
    }

    #[must_use]
    pub fn source_hash_map(&self) -> BTreeMap<String, String> {
        self.source_hashes
            .iter()
            .map(|item| (item.path.clone(), item.sha256.clone()))
            .collect()
    }

    /// Compute process authority inside VAC from the compiled registry snapshot.
    /// Tool-call supplied `policy_decision` is never trusted as authority.
    #[must_use]
    pub fn compute_command_policy_decision(
        &self,
        active_plan: Option<&SemanticPlan>,
        command: &StructuredCommand,
    ) -> PolicyDecision {
        if !self.policy_loaded {
            return PolicyDecision::Deny;
        }
        if command.runner.contains('/')
            || command.runner.contains('\\')
            || command.args.iter().any(|arg| contains_shell_metachar(arg))
        {
            return PolicyDecision::Deny;
        }
        if matches!(
            command.runner.as_str(),
            "sh" | "bash" | "zsh" | "fish" | "cmd" | "powershell"
        ) {
            return PolicyDecision::Deny;
        }
        if is_destructive_runner(&command.runner, &command.args) {
            return PolicyDecision::ApprovalRequired;
        }

        let in_plan = active_plan
            .map(|plan| plan.validation_commands.iter().any(|id| id == &command.id))
            .unwrap_or(false);
        let in_capability = active_plan
            .and_then(|plan| self.capability(&plan.capability))
            .map(|capability| {
                capability
                    .validation_commands
                    .iter()
                    .any(|id| id == &command.id)
            })
            .unwrap_or(false);
        let registered = self.command_registry.iter().any(|registered| {
            registered.id == command.id
                && registered.runner == command.runner
                && registered.args == command.args
        });
        if !(in_plan || in_capability || registered) {
            return PolicyDecision::Deny;
        }

        if let Some(decision) = self.evaluate_runtime_policy(
            "execute_process",
            Some("workspace"),
            Some(&command.runner),
            None,
            None,
            None,
        ) {
            return decision;
        }

        if matches!(command.risk, CommandRisk::SafeRead)
            && matches!(command.approval, CommandApproval::Never)
        {
            PolicyDecision::Allow
        } else {
            PolicyDecision::ApprovalRequired
        }
    }

    /// Compute network authority from compiled policy. URL/tool-call policy fields are descriptive only.
    #[must_use]
    pub fn compute_network_policy_decision(&self, access: &NetworkAccess) -> PolicyDecision {
        if !self.policy_loaded {
            return PolicyDecision::Deny;
        }
        self.evaluate_runtime_policy(
            "network_access",
            None,
            None,
            Some(&access.host),
            Some(&access.protocol),
            None,
        )
        .unwrap_or(PolicyDecision::Deny)
    }

    /// Compute read authority from compiled policy plus deterministic read-plan tickets.
    #[must_use]
    pub fn compute_read_policy_decision(&self, access: &ReadAccess) -> PolicyDecision {
        if !self.policy_loaded {
            return PolicyDecision::Deny;
        }
        if access.credential_material_present {
            return self
                .evaluate_runtime_policy(
                    "credential_read",
                    Some(&access.path),
                    None,
                    access.host.as_deref(),
                    access.protocol.as_deref(),
                    Some("secret_like"),
                )
                .unwrap_or(PolicyDecision::Deny);
        }
        if access.remote {
            return self
                .evaluate_runtime_policy(
                    "network_access",
                    Some(&access.path),
                    None,
                    access.host.as_deref(),
                    access.protocol.as_deref(),
                    None,
                )
                .unwrap_or(PolicyDecision::ApprovalRequired);
        }
        let ticket_valid = access.read_plan_ticket.as_ref().is_some_and(|ticket| {
            self.read_plan_tickets.iter().any(|registered| {
                registered.id == *ticket && vac_paths::path_matches(&registered.path, &access.path)
            })
        });
        if !ticket_valid {
            return PolicyDecision::ApprovalRequired;
        }
        self.evaluate_runtime_policy(
            "filesystem_read",
            Some(&access.path),
            None,
            None,
            None,
            None,
        )
        .unwrap_or(PolicyDecision::Deny)
    }

    fn evaluate_runtime_policy(
        &self,
        action: &str,
        path: Option<&str>,
        runner: Option<&str>,
        host: Option<&str>,
        protocol: Option<&str>,
        data_class: Option<&str>,
    ) -> Option<PolicyDecision> {
        if let Some(snapshot) = &self.policy_snapshot {
            let action = vac_policy_action_from_str(action)?;
            let evaluation = snapshot.evaluate(&vac_policy::PolicyRequest {
                action,
                path: path.map(ToString::to_string),
                data_class: data_class.map(ToString::to_string),
                network_host: host.map(ToString::to_string),
                network_protocol: protocol.map(ToString::to_string),
                capability: None,
                plan_id: None,
                session_id: None,
            });
            return Some(policy_decision_from_vac_policy(evaluation.decision));
        }
        self.matching_policy_decision(action, path, runner, host, protocol, data_class)
    }

    fn matching_policy_decision(
        &self,
        action: &str,
        path: Option<&str>,
        runner: Option<&str>,
        host: Option<&str>,
        protocol: Option<&str>,
        data_class: Option<&str>,
    ) -> Option<PolicyDecision> {
        let mut decision: Option<PolicyDecision> = None;
        for rule in &self.policy_rules {
            if !rule.matches(action, path, runner, host, protocol, data_class) {
                continue;
            }
            decision = Some(match decision {
                Some(existing) => most_restrictive_decision(existing, rule.decision),
                None => rule.decision,
            });
        }
        decision
    }
}

fn vac_policy_action_from_str(action: &str) -> Option<vac_policy::PolicyAction> {
    match action {
        "filesystem_read" => Some(vac_policy::PolicyAction::FilesystemRead),
        "filesystem_write" => Some(vac_policy::PolicyAction::FilesystemWrite),
        "filesystem_delete" => Some(vac_policy::PolicyAction::FilesystemDelete),
        "execute_process" => Some(vac_policy::PolicyAction::ExecuteProcess),
        "network_access" => Some(vac_policy::PolicyAction::NetworkAccess),
        "session_write" => Some(vac_policy::PolicyAction::SessionWrite),
        "checkpoint_write" => Some(vac_policy::PolicyAction::CheckpointWrite),
        "credential_read" => Some(vac_policy::PolicyAction::CredentialRead),
        "memory_write" => Some(vac_policy::PolicyAction::MemoryWrite),
        "plan_modify" => Some(vac_policy::PolicyAction::PlanModify),
        "capability_register" => Some(vac_policy::PolicyAction::CapabilityRegister),
        "readiness_declare" => Some(vac_policy::PolicyAction::ReadinessDeclare),
        _ => None,
    }
}

fn policy_decision_from_vac_policy(decision: vac_policy::Decision) -> PolicyDecision {
    match decision {
        vac_policy::Decision::Allow => PolicyDecision::Allow,
        vac_policy::Decision::ApprovalRequired => PolicyDecision::ApprovalRequired,
        vac_policy::Decision::Deny => PolicyDecision::Deny,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledPolicyRule {
    pub id: String,
    pub action: String,
    pub decision: PolicyDecision,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub runner: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub data_class: Option<String>,
}

impl CompiledPolicyRule {
    #[must_use]
    pub fn matches(
        &self,
        action: &str,
        path: Option<&str>,
        runner: Option<&str>,
        host: Option<&str>,
        protocol: Option<&str>,
        data_class: Option<&str>,
    ) -> bool {
        self.action == action
            && self.path.as_ref().is_none_or(|pattern| {
                path.is_some_and(|value| vac_paths::path_matches(pattern, value))
                    || pattern == "any"
                    || pattern == "workspace"
            })
            && self
                .runner
                .as_ref()
                .is_none_or(|expected| runner == Some(expected.as_str()))
            && self.host.as_ref().is_none_or(|pattern| {
                host.is_some_and(|value| vac_paths::globish_matches(pattern, value))
            })
            && self
                .protocol
                .as_ref()
                .is_none_or(|expected| protocol == Some(expected.as_str()))
            && self
                .data_class
                .as_ref()
                .is_none_or(|expected| data_class == Some(expected.as_str()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRuntimeRecord {
    pub id: String,
    pub runner: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadPlanTicketRecord {
    pub id: String,
    pub path: String,
    #[serde(default)]
    pub spans: Vec<String>,
    #[serde(default)]
    pub capability: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceHash {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRuntimeRecord {
    pub id: String,
    pub readiness: ReadinessTriplet,
    #[serde(default)]
    pub owned_paths: Vec<String>,
    #[serde(default)]
    pub validation_commands: Vec<String>,
    #[serde(default)]
    pub surfaces: Vec<String>,
}

impl CapabilityRuntimeRecord {
    #[must_use]
    pub fn owns_path(&self, path: &str) -> bool {
        self.owned_paths
            .iter()
            .any(|pattern| vac_paths::path_matches(pattern, path))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Draft,
    Approved,
    Executing,
    Completed,
    Rejected,
    Abandoned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticPlan {
    pub id: String,
    pub status: PlanStatus,
    pub capability: String,
    #[serde(default)]
    pub allowed_files: Vec<PlanFileScope>,
    #[serde(default)]
    pub forbidden_files: Vec<String>,
    #[serde(default)]
    pub validation_commands: Vec<String>,
    pub approval: PlanApproval,
    pub bounds: PatchBudget,
}

impl SemanticPlan {
    #[must_use]
    pub fn executable(&self) -> bool {
        matches!(self.status, PlanStatus::Approved | PlanStatus::Executing)
            && (!self.approval.required || self.approval.approved)
    }

    #[must_use]
    pub fn allowed_scope(&self, path: &str) -> Option<&PlanFileScope> {
        self.allowed_files.iter().find(|scope| scope.path == path)
    }

    #[must_use]
    pub fn forbidden_matches(&self, path: &str) -> bool {
        self.forbidden_files
            .iter()
            .any(|pattern| vac_paths::path_matches(pattern, path))
    }
}

const HARD_DENY_PATCH_PATHS: &[&str] = &[".vac/db/**", ".vac/cache/**"];

#[must_use]
fn hard_deny_patch_path(path: &str) -> bool {
    HARD_DENY_PATCH_PATHS
        .iter()
        .any(|pattern| vac_paths::path_matches(pattern, path))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanApproval {
    pub required: bool,
    pub approved: bool,
    #[serde(default)]
    pub plan_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchBudget {
    pub max_patches: u32,
    pub max_new_files: u32,
    pub max_line_delta: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileOperation {
    Create,
    Modify,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanFileScope {
    pub path: String,
    pub operation: FileOperation,
    pub line_range: LineRange,
    pub semantic_anchor: SemanticAnchor,
    pub ownership: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineRange {
    pub start: u32,
    pub end: u32,
}

impl LineRange {
    #[must_use]
    pub fn contains(self, other: Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticAnchor {
    pub symbol: String,
    pub kind: String,
    #[serde(default)]
    pub ast_path: Option<String>,
    #[serde(default)]
    pub normalized_fingerprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchAttempt {
    pub path: String,
    pub operation: FileOperation,
    pub touched_range: LineRange,
    pub semantic_anchor: SemanticAnchor,
    pub lines_added: u32,
    pub lines_removed: u32,
    pub creates_new_file: bool,
    pub patch_index: u32,
}

impl PatchAttempt {
    #[must_use]
    pub fn line_delta(&self) -> u32 {
        self.lines_added.saturating_add(self.lines_removed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadAccess {
    pub id: String,
    pub path: String,
    pub action: String,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub read_plan_ticket: Option<String>,
    pub credential_material_present: bool,
    pub grep: bool,
    pub glob: bool,
    pub remote: bool,
    pub risk: CommandRisk,
    pub approval: CommandApproval,
    pub policy_decision: PolicyDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredCommand {
    pub id: String,
    pub runner: String,
    pub args: Vec<String>,
    pub risk: CommandRisk,
    pub approval: CommandApproval,
    pub policy_decision: PolicyDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkAccess {
    pub id: String,
    pub url: String,
    pub host: String,
    pub protocol: String,
    pub risk: CommandRisk,
    pub approval: CommandApproval,
    pub policy_decision: PolicyDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandRisk {
    SafeRead,
    Low,
    Medium,
    High,
    Critical,
    ExecuteProcess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandApproval {
    Never,
    Policy,
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecision {
    Allow,
    ApprovalRequired,
    Deny,
}

#[must_use]
pub fn most_restrictive_decision(left: PolicyDecision, right: PolicyDecision) -> PolicyDecision {
    match (left, right) {
        (PolicyDecision::Deny, _) | (_, PolicyDecision::Deny) => PolicyDecision::Deny,
        (PolicyDecision::ApprovalRequired, _) | (_, PolicyDecision::ApprovalRequired) => {
            PolicyDecision::ApprovalRequired
        }
        _ => PolicyDecision::Allow,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeGate {
    PrePlan,
    ArtifactLock,
    PrePatch,
    PreRead,
    PreCommand,
    PreNetwork,
    PostValidation,
    Evidence,
    SessionCloseout,
    CompletionLock,
    EnforcementHonesty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateDecision {
    Pass,
    PassWithWarnings,
    ApprovalRequired,
    NeedsDiscussion,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateOutcome {
    pub gate: RuntimeGate,
    pub decision: GateDecision,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl GateOutcome {
    #[must_use]
    pub fn pass(gate: RuntimeGate) -> Self {
        Self {
            gate,
            decision: GateDecision::Pass,
            blockers: Vec::new(),
            warnings: Vec::new(),
        }
    }

    #[must_use]
    pub fn block(gate: RuntimeGate, blocker: impl Into<String>) -> Self {
        Self {
            gate,
            decision: GateDecision::Block,
            blockers: vec![blocker.into()],
            warnings: Vec::new(),
        }
    }

    #[must_use]
    pub fn approval_required(gate: RuntimeGate, reason: impl Into<String>) -> Self {
        Self {
            gate,
            decision: GateDecision::ApprovalRequired,
            blockers: Vec::new(),
            warnings: vec![reason.into()],
        }
    }

    #[must_use]
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        if matches!(self.decision, GateDecision::Pass) {
            self.decision = GateDecision::PassWithWarnings;
        }
        self
    }

    pub fn push_blocker(&mut self, blocker: impl Into<String>) {
        self.blockers.push(blocker.into());
        self.decision = GateDecision::Block;
    }

    pub fn push_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
        if matches!(self.decision, GateDecision::Pass) {
            self.decision = GateDecision::PassWithWarnings;
        }
    }

    #[must_use]
    pub fn is_blocked(&self) -> bool {
        matches!(self.decision, GateDecision::Block)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskArtifactState {
    Open,
    Done,
    NeedsDiscussion,
    Dropped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecArtifactState {
    Draft,
    Finalized,
    NeedsDiscussion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoArtifactState {
    Open,
    AllChecked,
    NeedsDiscussion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskArtifact {
    pub id: String,
    pub session: String,
    pub state: TaskArtifactState,
    pub capability: String,
    pub plan: String,
    #[serde(default)]
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
    #[serde(default)]
    pub open_questions: Vec<String>,
}

impl TaskArtifact {
    #[must_use]
    pub fn terminal(&self) -> bool {
        matches!(
            self.state,
            TaskArtifactState::Done
                | TaskArtifactState::Dropped
                | TaskArtifactState::NeedsDiscussion
        )
    }

    #[must_use]
    pub fn complete(&self) -> bool {
        matches!(self.state, TaskArtifactState::Done)
            && self.acceptance_criteria.iter().all(|item| item.met)
    }

    #[must_use]
    pub fn discussion_ready(&self) -> bool {
        matches!(self.state, TaskArtifactState::NeedsDiscussion) && !self.open_questions.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptanceCriterion {
    pub id: String,
    pub text: String,
    pub met: bool,
    #[serde(default)]
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecArtifact {
    pub id: String,
    pub session: String,
    pub state: SpecArtifactState,
    #[serde(default)]
    pub invariants: Vec<String>,
    #[serde(default)]
    pub touched_capabilities: Vec<String>,
    #[serde(default)]
    pub memory_refs: Vec<String>,
    #[serde(default)]
    pub open_questions: Vec<String>,
}

impl SpecArtifact {
    #[must_use]
    pub fn terminal(&self) -> bool {
        matches!(
            self.state,
            SpecArtifactState::Finalized | SpecArtifactState::NeedsDiscussion
        )
    }

    #[must_use]
    pub fn complete(&self) -> bool {
        matches!(self.state, SpecArtifactState::Finalized)
    }

    #[must_use]
    pub fn discussion_ready(&self) -> bool {
        matches!(self.state, SpecArtifactState::NeedsDiscussion) && !self.open_questions.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoArtifact {
    pub id: String,
    pub session: String,
    pub state: TodoArtifactState,
    #[serde(default)]
    pub items: Vec<TodoItem>,
    #[serde(default)]
    pub open_questions: Vec<String>,
}

impl TodoArtifact {
    #[must_use]
    pub fn terminal(&self) -> bool {
        matches!(
            self.state,
            TodoArtifactState::AllChecked | TodoArtifactState::NeedsDiscussion
        )
    }

    #[must_use]
    pub fn complete(&self) -> bool {
        matches!(self.state, TodoArtifactState::AllChecked)
            && self.items.iter().all(|item| item.checked || !item.blocking)
    }

    #[must_use]
    pub fn discussion_ready(&self) -> bool {
        matches!(self.state, TodoArtifactState::NeedsDiscussion) && !self.open_questions.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub text: String,
    pub kind: String,
    pub checked: bool,
    pub blocking: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionArtifacts {
    pub task: TaskArtifact,
    pub spec: SpecArtifact,
    pub todo: TodoArtifact,
}

impl SessionArtifacts {
    #[must_use]
    pub fn all_terminal(&self) -> bool {
        self.task.terminal() && self.spec.terminal() && self.todo.terminal()
    }

    #[must_use]
    pub fn all_complete(&self) -> bool {
        self.task.complete() && self.spec.complete() && self.todo.complete()
    }

    #[must_use]
    pub fn has_valid_discussion_escape(&self) -> bool {
        self.task.discussion_ready() || self.spec.discussion_ready() || self.todo.discussion_ready()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureAlgorithm {
    None,
    Ed25519,
    Minisign,
    Webauthn,
    Sigstore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureMode {
    Signed,
    IntegrityHint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceState {
    pub valid: bool,
    pub self_hash: String,
    pub broker_sig_algorithm: SignatureAlgorithm,
    pub broker_sig_mode: SignatureMode,
    #[serde(default)]
    pub warning_label: Option<String>,
}

impl EvidenceState {
    #[must_use]
    pub fn integrity_hint(hash: impl Into<String>) -> Self {
        Self {
            valid: true,
            self_hash: hash.into(),
            broker_sig_algorithm: SignatureAlgorithm::None,
            broker_sig_mode: SignatureMode::IntegrityHint,
            warning_label: Some("integrity-hint, NOT tamper-evident".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecSyncState {
    pub no_critical_spec_drift: bool,
    pub unresolved_critical_drift: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadinessState {
    pub no_unresolved_readiness_mismatch: bool,
    pub unresolved_mismatches: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnershipState {
    pub no_code_without_capability: bool,
    pub no_capability_without_current_intent_spec: bool,
    pub code_without_capability: u32,
    pub capabilities_without_intent: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssessmentState {
    pub findings_have_span_evidence: bool,
    pub unresolved_critical_findings: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ManifestSyncCloseoutState {
    pub no_ghost_state: bool,
    pub ghost_state: u32,
    pub no_orphan_state: bool,
    pub orphan_state: u32,
    pub no_stale_authorization: bool,
    pub stale_authorization: u32,
}

impl ManifestSyncCloseoutState {
    #[must_use]
    pub fn current() -> Self {
        Self {
            no_ghost_state: true,
            ghost_state: 0,
            no_orphan_state: true,
            orphan_state: 0,
            no_stale_authorization: true,
            stale_authorization: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RuntimeJournalCloseoutState {
    pub session_recorded: bool,
    pub event_count: u32,
    pub plan_terminal: bool,
    pub plan_manifest_current: bool,
    pub validation_terminal: bool,
    pub validation_manifest_current: bool,
    pub decisions_locked_and_current: bool,
    pub manifest_sync: ManifestSyncCloseoutState,
    pub evidence_summary_present: bool,
    #[serde(default)]
    pub evidence_summary_trust_wording: Option<String>,
}

impl RuntimeJournalCloseoutState {
    #[must_use]
    pub fn product_path_complete(trust_wording: impl Into<String>) -> Self {
        Self {
            session_recorded: true,
            event_count: 1,
            plan_terminal: true,
            plan_manifest_current: true,
            validation_terminal: true,
            validation_manifest_current: true,
            decisions_locked_and_current: true,
            manifest_sync: ManifestSyncCloseoutState::current(),
            evidence_summary_present: true,
            evidence_summary_trust_wording: Some(trust_wording.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloseoutState {
    pub artifacts: SessionArtifacts,
    pub evidence: EvidenceState,
    pub compiled_json_snapshot_current: bool,
    pub spec_sync: SpecSyncState,
    pub readiness: ReadinessState,
    pub ownership: OwnershipState,
    pub assessment: AssessmentState,
    #[serde(default)]
    pub runtime_journal: RuntimeJournalCloseoutState,
    #[serde(default)]
    pub explicit_open_question: Option<String>,
    #[serde(default)]
    pub blocking_reason: Option<String>,
    pub operator_visible_status: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionDisposition {
    Done,
    PausedForDiscussion,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionLockResult {
    pub gate: RuntimeGate,
    pub disposition: CompletionDisposition,
    pub session_state: SessionState,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub required: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSessionRecord {
    pub session_id: String,
    pub state: SessionState,
    pub phase: BoundRuntimePhase,
    pub conformance_level: RuntimeConformanceLevel,
    pub guarantee_label: String,
    pub started_at: DateTime<Utc>,
    #[serde(default)]
    pub plan_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundRuntimeTraceEvent {
    pub phase: BoundRuntimePhase,
    pub gate: RuntimeGate,
    pub decision: GateDecision,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl BoundRuntimeTraceEvent {
    #[must_use]
    pub fn from_outcome(phase: BoundRuntimePhase, outcome: GateOutcome) -> Self {
        Self {
            phase,
            gate: outcome.gate,
            decision: outcome.decision,
            blockers: outcome.blockers,
            warnings: outcome.warnings,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationState {
    pub commands_exit_zero: bool,
    pub no_new_orphans: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundRuntimeE2eInput {
    pub plan: SemanticPlan,
    pub artifacts: SessionArtifacts,
    #[serde(default)]
    pub patches: Vec<PatchAttempt>,
    #[serde(default)]
    pub commands: Vec<StructuredCommand>,
    pub validation: ValidationState,
    pub closeout: CloseoutState,
    #[serde(default)]
    pub requested_l2_claim: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundRuntimeE2eResult {
    pub trace: Vec<BoundRuntimeTraceEvent>,
    pub completion: Option<CompletionLockResult>,
    pub final_session_state: SessionState,
    pub blocked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundRuntimeConfig {
    pub require_compiled_json: bool,
    pub require_deterministic_index: bool,
    pub require_policy: bool,
    pub conformance_level: RuntimeConformanceLevel,
}

impl Default for BoundRuntimeConfig {
    fn default() -> Self {
        Self {
            require_compiled_json: true,
            require_deterministic_index: true,
            require_policy: true,
            conformance_level: RuntimeConformanceLevel::L1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BoundRuntimeController {
    pub config: BoundRuntimeConfig,
    pub session: RuntimeSessionRecord,
    pub registry: RuntimeRegistrySnapshot,
    active_plan: Option<SemanticPlan>,
    patch_count: u32,
    new_file_count: u32,
    line_delta: u32,
}

impl BoundRuntimeController {
    #[must_use]
    pub fn new(
        session_id: impl Into<String>,
        registry: RuntimeRegistrySnapshot,
        config: BoundRuntimeConfig,
    ) -> Self {
        let conformance_level = config.conformance_level;
        Self {
            session: RuntimeSessionRecord {
                session_id: session_id.into(),
                state: SessionState::Initializing,
                phase: BoundRuntimePhase::IntentPlan,
                conformance_level,
                guarantee_label: conformance_level.guarantee_label().to_string(),
                started_at: Utc::now(),
                plan_id: None,
            },
            config,
            registry,
            active_plan: None,
            patch_count: 0,
            new_file_count: 0,
            line_delta: 0,
        }
    }

    #[must_use]
    pub fn compute_command_policy_decision(
        &self,
        active_plan: Option<&SemanticPlan>,
        command: &StructuredCommand,
    ) -> PolicyDecision {
        self.registry
            .compute_command_policy_decision(active_plan, command)
    }

    #[must_use]
    pub fn compute_network_policy_decision(&self, access: &NetworkAccess) -> PolicyDecision {
        self.registry.compute_network_policy_decision(access)
    }

    #[must_use]
    pub fn compute_read_policy_decision(&self, access: &ReadAccess) -> PolicyDecision {
        self.registry.compute_read_policy_decision(access)
    }

    #[must_use]
    pub fn enforcement_honesty_gate(&self, requested_l2_claim: bool) -> GateOutcome {
        if requested_l2_claim
            && !self
                .config
                .conformance_level
                .may_claim_fail_closed_enforcement()
        {
            return GateOutcome::block(
                RuntimeGate::EnforcementHonesty,
                "L2 fail-closed claim is invalid while runtime is L1 advisory",
            );
        }
        let mut outcome = GateOutcome::pass(RuntimeGate::EnforcementHonesty);
        if matches!(self.config.conformance_level, RuntimeConformanceLevel::L1) {
            outcome.push_warning(self.config.conformance_level.guarantee_label());
        }
        outcome
    }

    #[must_use]
    pub fn pre_plan_gate(&self, plan: &SemanticPlan) -> GateOutcome {
        let mut outcome = GateOutcome::pass(RuntimeGate::PrePlan);
        if self.config.require_compiled_json && !self.registry.authority.is_executable() {
            outcome.push_blocker("runtime authorization attempted without compiled JSON snapshot");
        }
        if self.config.require_compiled_json && !self.registry.compiled_snapshot_current {
            outcome.push_blocker("compiled JSON snapshot is stale or missing");
        }
        if self.config.require_deterministic_index && !self.registry.deterministic_index_current {
            outcome.push_blocker("deterministic index/read-plan tickets are stale or missing");
        }
        if self.config.require_policy && !self.registry.policy_loaded {
            outcome.push_blocker("policy snapshot missing; VAC is fail-closed");
        }

        let Some(capability) = self.registry.capability(&plan.capability) else {
            outcome.push_blocker(format!(
                "capability not present in compiled registry: {}",
                plan.capability
            ));
            return outcome;
        };

        let readiness = capability.readiness.clone().fail_closed();
        match readiness.effective {
            ReadinessLevel::Ready => {}
            ReadinessLevel::Partial => outcome.push_warning(format!(
                "capability {} is partial; execution remains bounded and warning-visible",
                capability.id
            )),
            ReadinessLevel::Planned | ReadinessLevel::Deprecated => outcome.push_blocker(format!(
                "capability {} is not executable: {:?}",
                capability.id, readiness.effective
            )),
        }

        for file in &plan.allowed_files {
            if hard_deny_patch_path(&file.path) {
                outcome.push_blocker(format!(
                    "hard_deny_patch path cannot be listed in a Semantic Plan: {}",
                    file.path
                ));
            }
            if file.ownership != capability.id {
                outcome.push_blocker(format!(
                    "plan file {} has ownership {} but active capability is {}",
                    file.path, file.ownership, capability.id
                ));
            }
            if !capability.owns_path(&file.path) {
                outcome.push_blocker(format!(
                    "file is not owned by capability {}: {}",
                    capability.id, file.path
                ));
            }
        }

        if plan.approval.required && !plan.approval.approved {
            if !outcome.is_blocked() {
                outcome.decision = GateDecision::ApprovalRequired;
            }
            outcome.push_warning("operator approval is required before bounded patch execution");
        }

        outcome
    }

    pub fn approve_plan(&mut self, plan: SemanticPlan) -> GateOutcome {
        let mut outcome = self.pre_plan_gate(&plan);
        if !plan.executable() {
            outcome.push_blocker(
                "Semantic Plan must be approved and operator-bound before agent processing",
            );
        }
        if !outcome.is_blocked()
            && matches!(
                outcome.decision,
                GateDecision::Pass | GateDecision::PassWithWarnings
            )
        {
            self.session.state = SessionState::Planning;
            self.session.phase = BoundRuntimePhase::IntentPlan;
            self.session.plan_id = Some(plan.id.clone());
            self.active_plan = Some(plan);
        }
        outcome
    }

    pub fn setup_artifacts(&mut self, artifacts: &SessionArtifacts) -> GateOutcome {
        let mut outcome = GateOutcome::pass(RuntimeGate::ArtifactLock);
        let Some(plan) = &self.active_plan else {
            return GateOutcome::block(
                RuntimeGate::ArtifactLock,
                "mandatory artifacts cannot lock before approved Semantic Plan",
            );
        };
        if artifacts.task.session != self.session.session_id
            || artifacts.spec.session != self.session.session_id
            || artifacts.todo.session != self.session.session_id
        {
            outcome.push_blocker("mandatory artifacts are not bound to the active session");
        }
        if artifacts.task.id.is_empty()
            || artifacts.spec.id.is_empty()
            || artifacts.todo.id.is_empty()
        {
            outcome.push_blocker("mandatory task/spec/todo artifacts must have stable ids");
        }
        if artifacts.task.plan != plan.id {
            outcome.push_blocker("task artifact must reference the active Semantic Plan");
        }
        if artifacts.task.capability != plan.capability {
            outcome.push_blocker(
                "task artifact capability must match the active Semantic Plan capability",
            );
        }
        if !artifacts
            .spec
            .touched_capabilities
            .iter()
            .any(|capability| capability == &plan.capability)
        {
            outcome.push_blocker(
                "spec artifact must list the active capability in touched_capabilities",
            );
        }
        if artifacts.task.acceptance_criteria.is_empty() {
            outcome.push_blocker("task artifact must include verifiable acceptance criteria");
        }
        if artifacts.todo.items.iter().all(|item| !item.blocking) {
            outcome.push_blocker("todo artifact must include at least one blocking closeout item");
        }
        if !outcome.is_blocked() {
            self.session.state = SessionState::ArtifactsSetup;
            self.session.phase = BoundRuntimePhase::SetupArtifacts;
        }
        outcome
    }

    pub fn pre_patch_gate(&mut self, attempt: &PatchAttempt) -> GateOutcome {
        let Some(plan) = &self.active_plan else {
            return GateOutcome::block(
                RuntimeGate::PrePatch,
                "agent attempted patch before approved Semantic Plan",
            );
        };
        if !plan.executable() {
            return GateOutcome::block(
                RuntimeGate::PrePatch,
                "Semantic Plan is not approved/executable",
            );
        }
        if hard_deny_patch_path(&attempt.path) {
            return GateOutcome::block(
                RuntimeGate::PrePatch,
                format!(
                    "hard_deny_patch path is runtime-owned or generated: {}",
                    attempt.path
                ),
            );
        }
        if plan.forbidden_matches(&attempt.path) {
            return GateOutcome::block(
                RuntimeGate::PrePatch,
                "forbidden.files wins over allowed_files",
            );
        }
        let Some(scope) = plan.allowed_scope(&attempt.path) else {
            return GateOutcome::block(
                RuntimeGate::PrePatch,
                format!("file is outside Semantic Plan: {}", attempt.path),
            );
        };
        let mut outcome = GateOutcome::pass(RuntimeGate::PrePatch);
        if scope.operation != attempt.operation {
            outcome.push_blocker("patch operation does not match plan operation");
        }
        if !scope.line_range.contains(attempt.touched_range) {
            outcome.push_blocker("patch range is outside approved line range");
        }
        if scope.semantic_anchor.symbol != attempt.semantic_anchor.symbol
            || scope.semantic_anchor.kind != attempt.semantic_anchor.kind
        {
            outcome.push_blocker("semantic anchor mismatch; plan refresh required");
        }
        if let Some(expected) = &scope.semantic_anchor.normalized_fingerprint
            && attempt.semantic_anchor.normalized_fingerprint.as_ref() != Some(expected)
        {
            outcome.push_blocker("semantic anchor fingerprint drifted; plan refresh required");
        }
        if attempt.creates_new_file && !matches!(scope.operation, FileOperation::Create) {
            outcome.push_blocker("new file creation was not declared in the plan");
        }
        if attempt.patch_index != self.patch_count {
            outcome.push_blocker("patch attempt index is not monotonic for the active session");
        }
        if self.patch_count >= plan.bounds.max_patches {
            outcome.push_blocker("patch budget exceeded");
        }
        if self
            .new_file_count
            .saturating_add(u32::from(attempt.creates_new_file))
            > plan.bounds.max_new_files
        {
            outcome.push_blocker("new file budget exceeded");
        }
        if self.line_delta.saturating_add(attempt.line_delta()) > plan.bounds.max_line_delta {
            outcome.push_blocker("line delta budget exceeded");
        }
        if !outcome.is_blocked() {
            self.patch_count = self.patch_count.saturating_add(1);
            if attempt.creates_new_file {
                self.new_file_count = self.new_file_count.saturating_add(1);
            }
            self.line_delta = self.line_delta.saturating_add(attempt.line_delta());
            self.session.state = SessionState::Executing;
            self.session.phase = BoundRuntimePhase::AgentProcess;
        }
        outcome
    }

    #[must_use]
    pub fn pre_read_gate(&self, access: &ReadAccess) -> GateOutcome {
        if access.id.trim().is_empty() {
            return GateOutcome::block(RuntimeGate::PreRead, "read access id is required");
        }
        if access.path.trim().is_empty() {
            return GateOutcome::block(RuntimeGate::PreRead, "read access path is required");
        }
        if access.credential_material_present
            && !matches!(access.policy_decision, PolicyDecision::Allow)
        {
            return GateOutcome::block(
                RuntimeGate::PreRead,
                "credential-bearing read requires explicit compiled policy allow and redaction boundary",
            );
        }
        if matches!(access.policy_decision, PolicyDecision::Deny) {
            return GateOutcome::block(
                RuntimeGate::PreRead,
                "compiled policy denies this read action",
            );
        }
        if access.remote
            || matches!(access.policy_decision, PolicyDecision::ApprovalRequired)
            || matches!(access.risk, CommandRisk::High | CommandRisk::Critical)
        {
            return GateOutcome::approval_required(
                RuntimeGate::PreRead,
                "read/remote access must pause for read-plan or operator approval",
            );
        }
        if access.read_plan_ticket.is_none() {
            return GateOutcome::approval_required(
                RuntimeGate::PreRead,
                "local view requires a deterministic read_plan_ticket or operator approval",
            );
        }
        let mut outcome = GateOutcome::pass(RuntimeGate::PreRead);
        outcome.push_warning("read access is constrained by compiled policy and read-plan ticket");
        outcome
    }

    #[must_use]
    pub fn pre_network_gate(&self, access: &NetworkAccess) -> GateOutcome {
        if access.id.trim().is_empty() {
            return GateOutcome::block(RuntimeGate::PreNetwork, "network access id is required");
        }
        if access.host.trim().is_empty() {
            return GateOutcome::block(RuntimeGate::PreNetwork, "network access host is required");
        }
        if access.protocol != "https" {
            return GateOutcome::block(
                RuntimeGate::PreNetwork,
                "network access requires explicit https protocol",
            );
        }
        if !access.url.starts_with("https://") {
            return GateOutcome::block(
                RuntimeGate::PreNetwork,
                "network URL is not https authority",
            );
        }
        if matches!(access.policy_decision, PolicyDecision::Deny) {
            return GateOutcome::block(
                RuntimeGate::PreNetwork,
                "compiled policy denies this network access",
            );
        }
        if matches!(access.approval, CommandApproval::Always)
            || matches!(access.policy_decision, PolicyDecision::ApprovalRequired)
            || matches!(access.risk, CommandRisk::High | CommandRisk::Critical)
        {
            return GateOutcome::approval_required(
                RuntimeGate::PreNetwork,
                "network access must pause for policy/operator approval",
            );
        }
        let mut outcome = GateOutcome::pass(RuntimeGate::PreNetwork);
        outcome.push_warning("network read is governed by compiled policy and VAC bound approval");
        outcome
    }

    #[must_use]
    pub fn pre_command_gate(&self, command: &StructuredCommand) -> GateOutcome {
        if command.id.trim().is_empty() {
            return GateOutcome::block(
                RuntimeGate::PreCommand,
                "structured command id is required",
            );
        }
        if command.runner.contains('/')
            || command.runner.contains(' ')
            || command.runner.trim().is_empty()
        {
            return GateOutcome::block(
                RuntimeGate::PreCommand,
                "runner must be a registry executable name, not a shell/path string",
            );
        }
        if matches!(
            command.runner.as_str(),
            "sh" | "bash" | "zsh" | "fish" | "cmd" | "powershell"
        ) {
            return GateOutcome::block(
                RuntimeGate::PreCommand,
                "free-form shell runner is forbidden by structured command contract",
            );
        }
        if command.args.iter().any(|arg| contains_shell_metachar(arg)) {
            return GateOutcome::block(
                RuntimeGate::PreCommand,
                "shell interpolation, wildcard, pipe, or redirection detected in structured args",
            );
        }
        if matches!(command.policy_decision, PolicyDecision::Deny) {
            return GateOutcome::block(
                RuntimeGate::PreCommand,
                "policy snapshot denies this structured command",
            );
        }
        if is_destructive_runner(&command.runner, &command.args) {
            return GateOutcome::approval_required(
                RuntimeGate::PreCommand,
                "destructive filesystem command requires explicit approval",
            );
        }
        if matches!(command.approval, CommandApproval::Always)
            || matches!(command.policy_decision, PolicyDecision::ApprovalRequired)
        {
            return GateOutcome::approval_required(
                RuntimeGate::PreCommand,
                "command must pause for policy/operator approval",
            );
        }
        let mut outcome = GateOutcome::pass(RuntimeGate::PreCommand);
        if matches!(
            command.risk,
            CommandRisk::High | CommandRisk::Critical | CommandRisk::ExecuteProcess
        ) {
            outcome.push_warning(
                "high/process risk command is allowed only because compiled policy allowed it",
            );
        }
        outcome
    }

    pub fn post_validation_gate(
        &mut self,
        commands_exit_zero: bool,
        no_new_orphans: bool,
    ) -> GateOutcome {
        let mut outcome = GateOutcome::pass(RuntimeGate::PostValidation);
        if !commands_exit_zero {
            outcome.push_blocker("one or more validation commands failed");
        }
        if !no_new_orphans {
            outcome.push_blocker("post-validation detected new orphan/unowned files");
        }
        if !outcome.is_blocked() {
            self.session.state = SessionState::Validating;
            self.session.phase = BoundRuntimePhase::Validate;
        }
        outcome
    }

    pub fn execute_contract_e2e(&mut self, input: BoundRuntimeE2eInput) -> BoundRuntimeE2eResult {
        let mut trace = Vec::new();

        let honesty = self.enforcement_honesty_gate(input.requested_l2_claim);
        let honesty_blocked = honesty.is_blocked();
        trace.push(BoundRuntimeTraceEvent::from_outcome(
            self.session.phase,
            honesty,
        ));
        if honesty_blocked {
            return BoundRuntimeE2eResult {
                trace,
                completion: None,
                final_session_state: self.session.state,
                blocked: true,
            };
        }

        let plan_gate = self.approve_plan(input.plan);
        let plan_blocked =
            plan_gate.is_blocked() || matches!(plan_gate.decision, GateDecision::ApprovalRequired);
        trace.push(BoundRuntimeTraceEvent::from_outcome(
            BoundRuntimePhase::IntentPlan,
            plan_gate,
        ));
        if plan_blocked {
            return BoundRuntimeE2eResult {
                trace,
                completion: None,
                final_session_state: self.session.state,
                blocked: true,
            };
        }

        let artifact_gate = self.setup_artifacts(&input.artifacts);
        let artifact_blocked = artifact_gate.is_blocked();
        trace.push(BoundRuntimeTraceEvent::from_outcome(
            BoundRuntimePhase::SetupArtifacts,
            artifact_gate,
        ));
        if artifact_blocked {
            return BoundRuntimeE2eResult {
                trace,
                completion: None,
                final_session_state: self.session.state,
                blocked: true,
            };
        }

        for patch in &input.patches {
            let patch_gate = self.pre_patch_gate(patch);
            let patch_blocked = patch_gate.is_blocked();
            trace.push(BoundRuntimeTraceEvent::from_outcome(
                BoundRuntimePhase::AgentProcess,
                patch_gate,
            ));
            if patch_blocked {
                return BoundRuntimeE2eResult {
                    trace,
                    completion: None,
                    final_session_state: self.session.state,
                    blocked: true,
                };
            }
        }

        for command in &input.commands {
            let command_gate = self.pre_command_gate(command);
            let command_blocked = !matches!(
                command_gate.decision,
                GateDecision::Pass | GateDecision::PassWithWarnings
            );
            trace.push(BoundRuntimeTraceEvent::from_outcome(
                BoundRuntimePhase::Validate,
                command_gate,
            ));
            if command_blocked {
                return BoundRuntimeE2eResult {
                    trace,
                    completion: None,
                    final_session_state: self.session.state,
                    blocked: true,
                };
            }
        }

        let validation_gate = self.post_validation_gate(
            input.validation.commands_exit_zero,
            input.validation.no_new_orphans,
        );
        let validation_blocked = validation_gate.is_blocked();
        trace.push(BoundRuntimeTraceEvent::from_outcome(
            BoundRuntimePhase::Validate,
            validation_gate,
        ));
        if validation_blocked {
            return BoundRuntimeE2eResult {
                trace,
                completion: None,
                final_session_state: self.session.state,
                blocked: true,
            };
        }

        let completion = self.complete_session(&input.closeout);
        let blocked = matches!(completion.disposition, CompletionDisposition::Blocked);
        trace.push(BoundRuntimeTraceEvent {
            phase: BoundRuntimePhase::Done,
            gate: completion.gate,
            decision: if blocked {
                GateDecision::Block
            } else if matches!(
                completion.disposition,
                CompletionDisposition::PausedForDiscussion
            ) {
                GateDecision::NeedsDiscussion
            } else if completion.warnings.is_empty() {
                GateDecision::Pass
            } else {
                GateDecision::PassWithWarnings
            },
            blockers: completion.blockers.clone(),
            warnings: completion.warnings.clone(),
        });
        BoundRuntimeE2eResult {
            trace,
            completion: Some(completion),
            final_session_state: self.session.state,
            blocked,
        }
    }

    pub fn complete_session(&mut self, closeout: &CloseoutState) -> CompletionLockResult {
        let result = evaluate_completion_lock_v1_5(closeout);
        match result.disposition {
            CompletionDisposition::Done => {
                self.session.state = SessionState::Done;
                self.session.phase = BoundRuntimePhase::Done;
            }
            CompletionDisposition::PausedForDiscussion => {
                self.session.state = SessionState::PausedForDiscussion;
                self.session.phase = BoundRuntimePhase::Done;
            }
            CompletionDisposition::Blocked => {
                self.session.state = SessionState::Closing;
                self.session.phase = BoundRuntimePhase::UpdateVac;
            }
        }
        result
    }
}

#[must_use]
pub fn evaluate_completion_lock_v1_5(closeout: &CloseoutState) -> CompletionLockResult {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let required = vec![
        "tasks_terminal".to_string(),
        "spec_terminal".to_string(),
        "todo_terminal".to_string(),
        "evidence_valid".to_string(),
        "compiled_json_snapshot_current".to_string(),
        "no_critical_spec_drift".to_string(),
        "no_unresolved_readiness_mismatch".to_string(),
        "no_code_without_capability".to_string(),
        "no_capability_without_current_intent_spec".to_string(),
        "assessment_findings_have_span_evidence".to_string(),
        "runtime_session_recorded".to_string(),
        "runtime_event_recorded".to_string(),
        "runtime_plan_terminal".to_string(),
        "runtime_plan_manifest_current".to_string(),
        "runtime_validation_terminal".to_string(),
        "runtime_validation_manifest_current".to_string(),
        "runtime_decisions_locked_and_current".to_string(),
        "manifest_sync_no_ghost_state".to_string(),
        "manifest_sync_no_orphan_state".to_string(),
        "manifest_sync_no_stale_authorization".to_string(),
        "evidence_summary_present".to_string(),
        "evidence_summary_true_derived_trust_present".to_string(),
    ];

    let runtime_journal = &closeout.runtime_journal;
    if !runtime_journal.session_recorded {
        blockers.push("runtime_session_recorded".to_string());
    }
    if runtime_journal.event_count == 0 {
        blockers.push("runtime_event_recorded".to_string());
    }
    if !runtime_journal.plan_terminal {
        blockers.push("runtime_plan_terminal".to_string());
    }
    if !runtime_journal.plan_manifest_current {
        blockers.push("runtime_plan_manifest_current".to_string());
    }
    if !runtime_journal.validation_terminal {
        blockers.push("runtime_validation_terminal".to_string());
    }
    if !runtime_journal.validation_manifest_current {
        blockers.push("runtime_validation_manifest_current".to_string());
    }
    if !runtime_journal.decisions_locked_and_current {
        blockers.push("runtime_decisions_locked_and_current".to_string());
    }
    if !runtime_journal.manifest_sync.no_ghost_state
        || runtime_journal.manifest_sync.ghost_state > 0
    {
        blockers.push("manifest_sync_no_ghost_state".to_string());
    }
    if !runtime_journal.manifest_sync.no_orphan_state
        || runtime_journal.manifest_sync.orphan_state > 0
    {
        blockers.push("manifest_sync_no_orphan_state".to_string());
    }
    if !runtime_journal.manifest_sync.no_stale_authorization
        || runtime_journal.manifest_sync.stale_authorization > 0
    {
        blockers.push("manifest_sync_no_stale_authorization".to_string());
    }
    if !runtime_journal.evidence_summary_present {
        blockers.push("evidence_summary_present".to_string());
    }
    if runtime_journal
        .evidence_summary_trust_wording
        .as_ref()
        .is_none_or(|wording| wording.trim().is_empty())
    {
        blockers.push("evidence_summary_true_derived_trust_present".to_string());
    }

    if !closeout.artifacts.task.terminal() {
        blockers.push("tasks_terminal".to_string());
    }
    if !closeout.artifacts.spec.terminal() {
        blockers.push("spec_terminal".to_string());
    }
    if !closeout.artifacts.todo.terminal() {
        blockers.push("todo_terminal".to_string());
    }
    if !closeout.artifacts.all_complete() && closeout.artifacts.all_terminal() {
        if closeout.artifacts.has_valid_discussion_escape() {
            warnings.push("mandatory artifacts use needs_discussion escape hatch".to_string());
        } else {
            blockers.push(
                "mandatory artifacts terminal but not complete and no explicit discussion question"
                    .to_string(),
            );
        }
    }
    if !closeout.evidence.valid {
        blockers.push("evidence_valid".to_string());
    }
    if matches!(
        closeout.evidence.broker_sig_algorithm,
        SignatureAlgorithm::None
    ) {
        warnings.push("evidence is in integrity-hint mode, NOT tamper-evident".to_string());
    }
    if !closeout.compiled_json_snapshot_current {
        blockers.push("compiled_json_snapshot_current".to_string());
    }
    if !closeout.spec_sync.no_critical_spec_drift
        || closeout.spec_sync.unresolved_critical_drift > 0
    {
        blockers.push("no_critical_spec_drift".to_string());
    }
    if !closeout.readiness.no_unresolved_readiness_mismatch
        || closeout.readiness.unresolved_mismatches > 0
    {
        blockers.push("no_unresolved_readiness_mismatch".to_string());
    }
    if !closeout.ownership.no_code_without_capability
        || closeout.ownership.code_without_capability > 0
    {
        blockers.push("no_code_without_capability".to_string());
    }
    if !closeout.ownership.no_capability_without_current_intent_spec
        || closeout.ownership.capabilities_without_intent > 0
    {
        blockers.push("no_capability_without_current_intent_spec".to_string());
    }
    if !closeout.assessment.findings_have_span_evidence
        || closeout.assessment.unresolved_critical_findings > 0
    {
        blockers.push("assessment_findings_have_span_evidence".to_string());
    }

    blockers.sort();
    blockers.dedup();

    let discussion_escape = closeout
        .explicit_open_question
        .as_ref()
        .is_some_and(|item| !item.is_empty())
        && closeout
            .blocking_reason
            .as_ref()
            .is_some_and(|item| !item.is_empty())
        && closeout.operator_visible_status;

    let disposition = if blockers.is_empty() && closeout.artifacts.all_complete() {
        CompletionDisposition::Done
    } else if discussion_escape
        && closeout.artifacts.all_terminal()
        && (closeout.artifacts.has_valid_discussion_escape() || !blockers.is_empty())
    {
        CompletionDisposition::PausedForDiscussion
    } else {
        CompletionDisposition::Blocked
    };

    let session_state = match disposition {
        CompletionDisposition::Done => SessionState::Done,
        CompletionDisposition::PausedForDiscussion => SessionState::PausedForDiscussion,
        CompletionDisposition::Blocked => SessionState::Closing,
    };

    CompletionLockResult {
        gate: RuntimeGate::CompletionLock,
        disposition,
        session_state,
        blockers,
        warnings,
        required,
    }
}

pub use vac_jcs::canonical_json_sha256;

#[must_use]
pub fn approval_binding_hash(
    plan_hash: &str,
    diff_hash: &str,
    policy_snapshot_hash: &str,
    nonce: &str,
) -> String {
    let mut object = Map::new();
    object.insert(
        "diff_hash".to_string(),
        Value::String(diff_hash.to_string()),
    );
    object.insert("nonce".to_string(), Value::String(nonce.to_string()));
    object.insert(
        "plan_hash".to_string(),
        Value::String(plan_hash.to_string()),
    );
    object.insert(
        "policy_snapshot_hash".to_string(),
        Value::String(policy_snapshot_hash.to_string()),
    );
    canonical_json_sha256(&Value::Object(object))
}

fn contains_shell_metachar(arg: &str) -> bool {
    const META: &[char] = &['|', '>', '<', ';', '&', '`', '$'];
    if arg.contains("..") || arg.contains('*') || arg.contains('?') {
        return true;
    }
    arg.chars().any(|ch| META.contains(&ch))
}

fn is_destructive_runner(runner: &str, args: &[String]) -> bool {
    let runner = runner.to_ascii_lowercase();
    if matches!(runner.as_str(), "rm" | "del" | "rmdir") {
        return true;
    }
    if runner == "cargo" && args.iter().any(|arg| arg == "clean") {
        return true;
    }
    false
}

#[must_use]
pub fn completed_artifacts(session: &str, plan_id: &str, evidence_id: &str) -> SessionArtifacts {
    SessionArtifacts {
        task: TaskArtifact {
            id: format!("task.{session}.slice"),
            session: session.to_string(),
            state: TaskArtifactState::Done,
            capability: "vac.runtime.agent_loop".to_string(),
            plan: plan_id.to_string(),
            acceptance_criteria: vec![AcceptanceCriterion {
                id: "ac.1".to_string(),
                text: "agent slice passed bounded runtime gates".to_string(),
                met: true,
                evidence: Some(evidence_id.to_string()),
            }],
            open_questions: Vec::new(),
        },
        spec: SpecArtifact {
            id: format!("spec.{session}.slice"),
            session: session.to_string(),
            state: SpecArtifactState::Finalized,
            invariants: vec![format!("plan {plan_id} remains the execution contract")],
            touched_capabilities: vec!["vac.runtime.agent_loop".to_string()],
            memory_refs: Vec::new(),
            open_questions: Vec::new(),
        },
        todo: TodoArtifact {
            id: format!("todo.{session}.slice"),
            session: session.to_string(),
            state: TodoArtifactState::AllChecked,
            items: vec![TodoItem {
                id: "t.1".to_string(),
                text: "run SV E2E runtime validation".to_string(),
                kind: "test".to_string(),
                checked: true,
                blocking: true,
            }],
            open_questions: Vec::new(),
        },
    }
}

#[must_use]
pub fn successful_closeout(session: &str, plan_id: &str, evidence_id: &str) -> CloseoutState {
    CloseoutState {
        artifacts: completed_artifacts(session, plan_id, evidence_id),
        evidence: EvidenceState::integrity_hint(format!("{HASH_PREFIX}fixture")),
        compiled_json_snapshot_current: true,
        spec_sync: SpecSyncState {
            no_critical_spec_drift: true,
            unresolved_critical_drift: 0,
        },
        readiness: ReadinessState {
            no_unresolved_readiness_mismatch: true,
            unresolved_mismatches: 0,
        },
        ownership: OwnershipState {
            no_code_without_capability: true,
            no_capability_without_current_intent_spec: true,
            code_without_capability: 0,
            capabilities_without_intent: 0,
        },
        assessment: AssessmentState {
            findings_have_span_evidence: true,
            unresolved_critical_findings: 0,
        },
        runtime_journal: RuntimeJournalCloseoutState::product_path_complete(
            "local self-reported trace; integrity hint only",
        ),
        explicit_open_question: None,
        blocking_reason: None,
        operator_visible_status: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> RuntimeRegistrySnapshot {
        RuntimeRegistrySnapshot {
            authority: RuntimeAuthority::CompiledJson,
            compiled_snapshot_current: true,
            source_hashes: vec![SourceHash {
                path: ".vac/capabilities/vac-runtime-agent_loop.yaml".to_string(),
                sha256: "sha256:abc".to_string(),
            }],
            capabilities: vec![CapabilityRuntimeRecord {
                id: "vac.runtime.agent_loop".to_string(),
                readiness: ReadinessTriplet {
                    declared: ReadinessLevel::Partial,
                    computed: ReadinessLevel::Partial,
                    effective: ReadinessLevel::Partial,
                    reason: "TV pending".to_string(),
                    blockers: Vec::new(),
                },
                owned_paths: vec!["vac-rs/crates/runtime/vac-agent-loop/**".to_string()],
                validation_commands: vec!["sv.runtime.agent_e2e".to_string()],
                surfaces: vec!["/runtime".to_string()],
            }],
            deterministic_index_current: true,
            policy_loaded: true,
            policy_rules: Vec::new(),
            policy_snapshot: None,
            command_registry: Vec::new(),
            read_plan_tickets: Vec::new(),
            conformance_level: RuntimeConformanceLevel::L1,
        }
    }

    fn plan(status: PlanStatus) -> SemanticPlan {
        SemanticPlan {
            id: "plan.runtime-agent-bound-loop-sv".to_string(),
            status,
            capability: "vac.runtime.agent_loop".to_string(),
            allowed_files: vec![PlanFileScope {
                path: "vac-rs/crates/runtime/vac-agent-loop/src/bound_runtime.rs".to_string(),
                operation: FileOperation::Modify,
                line_range: LineRange {
                    start: 1,
                    end: 1200,
                },
                semantic_anchor: SemanticAnchor {
                    symbol: "BoundRuntimeController".to_string(),
                    kind: "module".to_string(),
                    ast_path: Some("crate::bound_runtime".to_string()),
                    normalized_fingerprint: Some("sha256:anchor".to_string()),
                },
                ownership: "vac.runtime.agent_loop".to_string(),
            }],
            forbidden_files: vec!["target/**".to_string()],
            validation_commands: vec!["sv.runtime.agent_e2e".to_string()],
            approval: PlanApproval {
                required: false,
                approved: true,
                plan_hash: Some("sha256:plan".to_string()),
            },
            bounds: PatchBudget {
                max_patches: 4,
                max_new_files: 0,
                max_line_delta: 300,
            },
        }
    }

    #[test]
    fn pre_plan_uses_compiled_json_and_warns_on_partial_capability() {
        let runtime = BoundRuntimeController::new("s1", registry(), BoundRuntimeConfig::default());
        let outcome = runtime.pre_plan_gate(&plan(PlanStatus::Approved));
        assert_eq!(outcome.decision, GateDecision::PassWithWarnings);
        assert!(
            outcome
                .warnings
                .iter()
                .any(|warning| warning.contains("partial"))
        );
    }

    #[test]
    fn raw_yaml_authority_is_blocked() {
        let mut snapshot = registry();
        snapshot.authority = RuntimeAuthority::RawYaml;
        let runtime = BoundRuntimeController::new("s1", snapshot, BoundRuntimeConfig::default());
        let outcome = runtime.pre_plan_gate(&plan(PlanStatus::Approved));
        assert!(outcome.is_blocked());
        assert!(
            outcome
                .blockers
                .iter()
                .any(|blocker| blocker.contains("compiled JSON"))
        );
    }

    #[test]
    fn patch_before_approved_plan_is_blocked() {
        let mut runtime =
            BoundRuntimeController::new("s1", registry(), BoundRuntimeConfig::default());
        let attempt = PatchAttempt {
            path: "vac-rs/crates/runtime/vac-agent-loop/src/bound_runtime.rs".to_string(),
            operation: FileOperation::Modify,
            touched_range: LineRange { start: 10, end: 12 },
            semantic_anchor: SemanticAnchor {
                symbol: "BoundRuntimeController".to_string(),
                kind: "module".to_string(),
                ast_path: Some("crate::bound_runtime".to_string()),
                normalized_fingerprint: Some("sha256:anchor".to_string()),
            },
            lines_added: 1,
            lines_removed: 0,
            creates_new_file: false,
            patch_index: 0,
        };
        assert!(runtime.pre_patch_gate(&attempt).is_blocked());
    }

    #[test]
    fn allowed_patch_passes_after_plan_approval() {
        let mut runtime =
            BoundRuntimeController::new("s1", registry(), BoundRuntimeConfig::default());
        let approve = runtime.approve_plan(plan(PlanStatus::Approved));
        assert!(!approve.is_blocked());
        let attempt = PatchAttempt {
            path: "vac-rs/crates/runtime/vac-agent-loop/src/bound_runtime.rs".to_string(),
            operation: FileOperation::Modify,
            touched_range: LineRange { start: 10, end: 12 },
            semantic_anchor: SemanticAnchor {
                symbol: "BoundRuntimeController".to_string(),
                kind: "module".to_string(),
                ast_path: Some("crate::bound_runtime".to_string()),
                normalized_fingerprint: Some("sha256:anchor".to_string()),
            },
            lines_added: 1,
            lines_removed: 0,
            creates_new_file: false,
            patch_index: 0,
        };
        assert_eq!(
            runtime.pre_patch_gate(&attempt).decision,
            GateDecision::Pass
        );
    }

    #[test]
    fn hard_deny_patch_blocks_vac_db_even_if_plan_claims_it() {
        let mut runtime =
            BoundRuntimeController::new("s1", registry(), BoundRuntimeConfig::default());
        let mut p = plan(PlanStatus::Approved);
        p.allowed_files = vec![PlanFileScope {
            path: ".vac/db/runtime.db".to_string(),
            operation: FileOperation::Modify,
            line_range: LineRange { start: 1, end: 1 },
            semantic_anchor: SemanticAnchor {
                symbol: "runtime_db".to_string(),
                kind: "sqlite".to_string(),
                ast_path: None,
                normalized_fingerprint: None,
            },
            ownership: "vac.runtime.agent_loop".to_string(),
        }];
        runtime.active_plan = Some(p);

        let attempt = PatchAttempt {
            path: ".vac/db/runtime.db".to_string(),
            operation: FileOperation::Modify,
            touched_range: LineRange { start: 1, end: 1 },
            semantic_anchor: SemanticAnchor {
                symbol: "runtime_db".to_string(),
                kind: "sqlite".to_string(),
                ast_path: None,
                normalized_fingerprint: None,
            },
            lines_added: 1,
            lines_removed: 0,
            creates_new_file: false,
            patch_index: 0,
        };
        let outcome = runtime.pre_patch_gate(&attempt);
        assert!(outcome.is_blocked());
        assert!(
            outcome
                .blockers
                .iter()
                .any(|blocker| blocker.contains("hard_deny_patch"))
        );
    }

    #[test]
    fn forbidden_file_wins_over_allowed_file() {
        let mut runtime =
            BoundRuntimeController::new("s1", registry(), BoundRuntimeConfig::default());
        let mut p = plan(PlanStatus::Approved);
        p.forbidden_files =
            vec!["vac-rs/crates/runtime/vac-agent-loop/src/bound_runtime.rs".to_string()];
        let approve = runtime.approve_plan(p);
        assert!(!approve.is_blocked());
        let attempt = PatchAttempt {
            path: "vac-rs/crates/runtime/vac-agent-loop/src/bound_runtime.rs".to_string(),
            operation: FileOperation::Modify,
            touched_range: LineRange { start: 1, end: 1 },
            semantic_anchor: SemanticAnchor {
                symbol: "BoundRuntimeController".to_string(),
                kind: "module".to_string(),
                ast_path: None,
                normalized_fingerprint: None,
            },
            lines_added: 1,
            lines_removed: 0,
            creates_new_file: false,
            patch_index: 0,
        };
        let outcome = runtime.pre_patch_gate(&attempt);
        assert!(outcome.is_blocked());
        assert!(
            outcome
                .blockers
                .iter()
                .any(|blocker| blocker.contains("forbidden"))
        );
    }

    #[test]
    fn structured_command_blocks_free_form_shell() {
        let runtime = BoundRuntimeController::new("s1", registry(), BoundRuntimeConfig::default());
        let command = StructuredCommand {
            id: "bad.shell".to_string(),
            runner: "bash".to_string(),
            args: vec!["-lc".to_string(), "cargo check".to_string()],
            risk: CommandRisk::ExecuteProcess,
            approval: CommandApproval::Policy,
            policy_decision: PolicyDecision::ApprovalRequired,
        };
        assert!(runtime.pre_command_gate(&command).is_blocked());
    }

    #[test]
    fn destructive_command_requires_approval() {
        let runtime = BoundRuntimeController::new("s1", registry(), BoundRuntimeConfig::default());
        let command = StructuredCommand {
            id: "cleanup.target".to_string(),
            runner: "rm".to_string(),
            args: vec!["-rf".to_string(), "target/debug/incremental".to_string()],
            risk: CommandRisk::High,
            approval: CommandApproval::Always,
            policy_decision: PolicyDecision::ApprovalRequired,
        };
        assert_eq!(
            runtime.pre_command_gate(&command).decision,
            GateDecision::ApprovalRequired
        );
    }

    #[test]
    fn completion_lock_all_green_done() {
        let closeout = successful_closeout("s1", "plan.ok", "evidence.ok");
        let result = evaluate_completion_lock_v1_5(&closeout);
        assert_eq!(result.disposition, CompletionDisposition::Done);
    }

    #[test]
    fn completion_lock_blocks_nonterminal_runtime_projection_todo() {
        let mut closeout = successful_closeout("s1", "plan.ok", "evidence.ok");
        closeout.artifacts.todo.state = TodoArtifactState::Open;
        closeout.artifacts.todo.items[0].checked = false;
        let result = evaluate_completion_lock_v1_5(&closeout);
        assert_eq!(result.disposition, CompletionDisposition::Blocked);
        assert!(
            result
                .blockers
                .iter()
                .any(|blocker| blocker == "todo_terminal")
        );
    }

    #[test]
    fn completion_lock_pauses_when_escape_hatch_is_operator_visible() {
        let mut closeout = successful_closeout("s1", "plan.ok", "evidence.ok");
        closeout.artifacts.task.state = TaskArtifactState::NeedsDiscussion;
        closeout.artifacts.task.open_questions =
            vec!["Which policy should own this exception?".to_string()];
        closeout.explicit_open_question =
            Some("Which policy should own this exception?".to_string());
        closeout.blocking_reason = Some("operator decision required".to_string());
        closeout.operator_visible_status = true;
        let result = evaluate_completion_lock_v1_5(&closeout);
        assert_eq!(
            result.disposition,
            CompletionDisposition::PausedForDiscussion
        );
    }

    #[test]
    fn completion_lock_needs_discussion_cannot_close_done() {
        let mut closeout = successful_closeout("s1", "plan.ok", "evidence.ok");
        closeout.artifacts.task.state = TaskArtifactState::NeedsDiscussion;
        closeout.artifacts.task.open_questions =
            vec!["Operator must classify this decision.".to_string()];
        closeout.explicit_open_question = Some("Operator must classify this decision.".to_string());
        closeout.blocking_reason = Some("needs discussion is not done-state".to_string());
        closeout.operator_visible_status = true;

        let result = evaluate_completion_lock_v1_5(&closeout);

        assert_ne!(result.disposition, CompletionDisposition::Done);
        assert_eq!(
            result.disposition,
            CompletionDisposition::PausedForDiscussion
        );
    }

    #[test]
    fn completion_lock_blocks_stale_validation_projection() {
        let mut closeout = successful_closeout("s1", "plan.ok", "evidence.ok");
        closeout.runtime_journal.validation_manifest_current = false;

        let result = evaluate_completion_lock_v1_5(&closeout);

        assert_eq!(result.disposition, CompletionDisposition::Blocked);
        assert!(
            result
                .blockers
                .contains(&"runtime_validation_manifest_current".to_string())
        );
    }

    #[test]
    fn completion_lock_blocks_ghost_manifest_state_projection() {
        let mut closeout = successful_closeout("s1", "plan.ok", "evidence.ok");
        closeout.runtime_journal.manifest_sync.no_ghost_state = false;
        closeout.runtime_journal.manifest_sync.ghost_state = 1;

        let result = evaluate_completion_lock_v1_5(&closeout);

        assert_eq!(result.disposition, CompletionDisposition::Blocked);
        assert!(
            result
                .blockers
                .contains(&"manifest_sync_no_ghost_state".to_string())
        );
    }

    #[test]
    fn completion_lock_blocks_missing_evidence_summary_projection() {
        let mut closeout = successful_closeout("s1", "plan.ok", "evidence.ok");
        closeout.runtime_journal.evidence_summary_present = false;
        closeout.runtime_journal.evidence_summary_trust_wording = None;

        let result = evaluate_completion_lock_v1_5(&closeout);

        assert_eq!(result.disposition, CompletionDisposition::Blocked);
        assert!(
            result
                .blockers
                .contains(&"evidence_summary_present".to_string())
        );
        assert!(
            result
                .blockers
                .contains(&"evidence_summary_true_derived_trust_present".to_string())
        );
    }

    #[test]
    fn l1_cannot_claim_l2_fail_closed_boundary() {
        let runtime = BoundRuntimeController::new("s1", registry(), BoundRuntimeConfig::default());
        assert!(runtime.enforcement_honesty_gate(true).is_blocked());
    }

    #[test]
    fn canonical_hash_is_order_stable() {
        let left = serde_json::json!({"b": 2, "a": 1});
        let right = serde_json::json!({"a": 1, "b": 2});
        assert_eq!(canonical_json_sha256(&left), canonical_json_sha256(&right));
    }

    #[test]
    fn e2e_contract_reaches_done_when_every_gate_passes() {
        let mut runtime =
            BoundRuntimeController::new("s1", registry(), BoundRuntimeConfig::default());
        let patch = PatchAttempt {
            path: "vac-rs/crates/runtime/vac-agent-loop/src/bound_runtime.rs".to_string(),
            operation: FileOperation::Modify,
            touched_range: LineRange { start: 20, end: 22 },
            semantic_anchor: SemanticAnchor {
                symbol: "BoundRuntimeController".to_string(),
                kind: "module".to_string(),
                ast_path: Some("crate::bound_runtime".to_string()),
                normalized_fingerprint: Some("sha256:anchor".to_string()),
            },
            lines_added: 2,
            lines_removed: 1,
            creates_new_file: false,
            patch_index: 0,
        };
        let command = StructuredCommand {
            id: "sv.runtime.agent_e2e".to_string(),
            runner: "python3".to_string(),
            args: vec!["scripts/vac-runtime-agent-e2e-sv.py".to_string()],
            risk: CommandRisk::SafeRead,
            approval: CommandApproval::Never,
            policy_decision: PolicyDecision::Allow,
        };
        let input = BoundRuntimeE2eInput {
            plan: plan(PlanStatus::Approved),
            artifacts: completed_artifacts("s1", "plan.runtime-agent-bound-loop-sv", "evidence.ok"),
            patches: vec![patch],
            commands: vec![command],
            validation: ValidationState {
                commands_exit_zero: true,
                no_new_orphans: true,
            },
            closeout: successful_closeout("s1", "plan.runtime-agent-bound-loop-sv", "evidence.ok"),
            requested_l2_claim: false,
        };
        let result = runtime.execute_contract_e2e(input);
        assert!(!result.blocked);
        assert_eq!(result.final_session_state, SessionState::Done);
        assert_eq!(
            result.completion.as_ref().map(|item| item.disposition),
            Some(CompletionDisposition::Done)
        );
        assert!(
            result
                .trace
                .iter()
                .any(|item| item.gate == RuntimeGate::PrePatch)
        );
        assert!(
            result
                .trace
                .iter()
                .any(|item| item.gate == RuntimeGate::CompletionLock)
        );
    }

    #[test]
    fn e2e_contract_blocks_when_command_needs_approval() {
        let mut runtime =
            BoundRuntimeController::new("s1", registry(), BoundRuntimeConfig::default());
        let command = StructuredCommand {
            id: "cargo.check".to_string(),
            runner: "cargo".to_string(),
            args: vec!["check".to_string()],
            risk: CommandRisk::ExecuteProcess,
            approval: CommandApproval::Policy,
            policy_decision: PolicyDecision::ApprovalRequired,
        };
        let input = BoundRuntimeE2eInput {
            plan: plan(PlanStatus::Approved),
            artifacts: completed_artifacts("s1", "plan.runtime-agent-bound-loop-sv", "evidence.ok"),
            patches: Vec::new(),
            commands: vec![command],
            validation: ValidationState {
                commands_exit_zero: true,
                no_new_orphans: true,
            },
            closeout: successful_closeout("s1", "plan.runtime-agent-bound-loop-sv", "evidence.ok"),
            requested_l2_claim: false,
        };
        let result = runtime.execute_contract_e2e(input);
        assert!(result.blocked);
        assert!(result.completion.is_none());
        assert!(
            result
                .trace
                .iter()
                .any(|item| item.gate == RuntimeGate::PreCommand
                    && item.decision == GateDecision::ApprovalRequired)
        );
    }
}
