//! End-to-end harness for the VAC v1.9 bounded runtime projection contract.
//!
//! The harness is provider-free. It validates that an agent-like task can only
//! progress when compiled runtime state, a valid Semantic Plan, locked
//! runtime-journal projections, bounded patches, structured commands, validation,
//! evidence, SpecSync/readiness/ownership/assessment, and the Completion Lock all
//! agree. Real model streaming stays outside this module; this is the runtime
//! boundary contract.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::bound_runtime::{
    BoundRuntimeConfig, BoundRuntimeController, BoundRuntimePhase, CloseoutState,
    CompletionDisposition, CompletionLockResult, GateDecision, GateOutcome, PatchAttempt,
    RuntimeGate, RuntimeRegistrySnapshot, SemanticPlan, SessionArtifacts, SessionState,
    StructuredCommand,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundAgentE2EInput {
    pub session_id: String,
    pub user_prompt: String,
    pub registry: RuntimeRegistrySnapshot,
    pub plan: SemanticPlan,
    pub artifacts: SessionArtifacts,
    #[serde(default)]
    pub patch_attempts: Vec<PatchAttempt>,
    #[serde(default)]
    pub validation_commands: Vec<StructuredCommand>,
    #[serde(default)]
    pub operator_approved_command_ids: Vec<String>,
    pub commands_exit_zero: bool,
    pub no_new_orphans: bool,
    pub closeout: CloseoutState,
    #[serde(default)]
    pub requested_l2_claim: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundAgentE2EReport {
    pub session_id: String,
    pub user_prompt: String,
    pub phases: Vec<BoundRuntimePhase>,
    pub gate_outcomes: Vec<GateOutcome>,
    pub completion: Option<CompletionLockResult>,
    pub final_state: SessionState,
    pub disposition: CompletionDisposition,
    pub stopped_at: Option<RuntimeGate>,
    pub operator_visible_summary: String,
}

impl BoundAgentE2EReport {
    #[must_use]
    pub fn is_done(&self) -> bool {
        matches!(self.disposition, CompletionDisposition::Done)
            && matches!(self.final_state, SessionState::Done)
            && self.stopped_at.is_none()
    }

    #[must_use]
    pub fn is_paused_for_operator(&self) -> bool {
        matches!(self.disposition, CompletionDisposition::PausedForDiscussion)
            || self.gate_outcomes.iter().any(|outcome| {
                matches!(
                    outcome.decision,
                    GateDecision::ApprovalRequired | GateDecision::NeedsDiscussion
                )
            })
    }

    #[must_use]
    pub fn blockers(&self) -> Vec<String> {
        let mut out = Vec::new();
        for gate in &self.gate_outcomes {
            out.extend(gate.blockers.iter().cloned());
        }
        if let Some(completion) = &self.completion {
            out.extend(completion.blockers.iter().cloned());
        }
        out.sort();
        out.dedup();
        out
    }
}

#[must_use]
pub fn run_bound_agent_e2e(
    input: BoundAgentE2EInput,
    config: BoundRuntimeConfig,
) -> BoundAgentE2EReport {
    let approved_commands: BTreeSet<String> = input
        .operator_approved_command_ids
        .iter()
        .cloned()
        .collect();
    let mut runtime =
        BoundRuntimeController::new(&input.session_id, input.registry.clone(), config);
    let mut gate_outcomes = Vec::new();
    let mut phases = Vec::new();
    let mut stopped_at = None;
    let mut completion = None;

    let honesty = runtime.enforcement_honesty_gate(input.requested_l2_claim);
    if honesty.is_blocked() {
        stopped_at = Some(RuntimeGate::EnforcementHonesty);
        gate_outcomes.push(honesty);
        return report(
            input,
            runtime.session.state,
            phases,
            gate_outcomes,
            completion,
            stopped_at,
        );
    }
    gate_outcomes.push(honesty);

    let pre_plan = runtime.approve_plan(input.plan.clone());
    phases.push(BoundRuntimePhase::IntentPlan);
    if pre_plan.is_blocked() || matches!(pre_plan.decision, GateDecision::ApprovalRequired) {
        stopped_at = Some(RuntimeGate::PrePlan);
        gate_outcomes.push(pre_plan);
        return report(
            input,
            runtime.session.state,
            phases,
            gate_outcomes,
            completion,
            stopped_at,
        );
    }
    gate_outcomes.push(pre_plan);

    let artifacts = runtime.setup_artifacts(&input.artifacts);
    phases.push(BoundRuntimePhase::SetupArtifacts);
    if artifacts.is_blocked() {
        stopped_at = Some(RuntimeGate::ArtifactLock);
        gate_outcomes.push(artifacts);
        return report(
            input,
            runtime.session.state,
            phases,
            gate_outcomes,
            completion,
            stopped_at,
        );
    }
    gate_outcomes.push(artifacts);

    for attempt in &input.patch_attempts {
        let gate = runtime.pre_patch_gate(attempt);
        phases.push(BoundRuntimePhase::AgentProcess);
        if gate.is_blocked() {
            stopped_at = Some(RuntimeGate::PrePatch);
            gate_outcomes.push(gate);
            return report(
                input,
                runtime.session.state,
                phases,
                gate_outcomes,
                completion,
                stopped_at,
            );
        }
        gate_outcomes.push(gate);
    }

    for command in &input.validation_commands {
        let gate = runtime.pre_command_gate(command);
        if gate.is_blocked() {
            stopped_at = Some(RuntimeGate::PreCommand);
            gate_outcomes.push(gate);
            return report(
                input,
                runtime.session.state,
                phases,
                gate_outcomes,
                completion,
                stopped_at,
            );
        }
        if matches!(gate.decision, GateDecision::ApprovalRequired)
            && !approved_commands.contains(&command.id)
        {
            stopped_at = Some(RuntimeGate::PreCommand);
            gate_outcomes.push(gate);
            return report(
                input,
                runtime.session.state,
                phases,
                gate_outcomes,
                completion,
                stopped_at,
            );
        }
        gate_outcomes.push(gate);
    }

    let post_validation =
        runtime.post_validation_gate(input.commands_exit_zero, input.no_new_orphans);
    phases.push(BoundRuntimePhase::Validate);
    if post_validation.is_blocked() {
        stopped_at = Some(RuntimeGate::PostValidation);
        gate_outcomes.push(post_validation);
        return report(
            input,
            runtime.session.state,
            phases,
            gate_outcomes,
            completion,
            stopped_at,
        );
    }
    gate_outcomes.push(post_validation);

    let close = runtime.complete_session(&input.closeout);
    phases.push(BoundRuntimePhase::UpdateVac);
    phases.push(BoundRuntimePhase::Done);
    if matches!(close.disposition, CompletionDisposition::Blocked) {
        stopped_at = Some(RuntimeGate::CompletionLock);
    }
    completion = Some(close);

    report(
        input,
        runtime.session.state,
        phases,
        gate_outcomes,
        completion,
        stopped_at,
    )
}

fn report(
    input: BoundAgentE2EInput,
    final_state: SessionState,
    phases: Vec<BoundRuntimePhase>,
    gate_outcomes: Vec<GateOutcome>,
    completion: Option<CompletionLockResult>,
    stopped_at: Option<RuntimeGate>,
) -> BoundAgentE2EReport {
    let disposition = completion
        .as_ref()
        .map(|item| item.disposition)
        .unwrap_or_else(|| {
            if gate_outcomes.iter().any(|outcome| {
                matches!(
                    outcome.decision,
                    GateDecision::ApprovalRequired | GateDecision::NeedsDiscussion
                )
            }) {
                CompletionDisposition::PausedForDiscussion
            } else {
                CompletionDisposition::Blocked
            }
        });
    let operator_visible_summary = match disposition {
        CompletionDisposition::Done => {
            "Done — bounded VAC slice completed with artifacts, validation, evidence, and closeout"
                .to_string()
        }
        CompletionDisposition::PausedForDiscussion => {
            "Paused: needs your input — runtime did not silently complete".to_string()
        }
        CompletionDisposition::Blocked => {
            "Blocked — VAC runtime gate prevented an unsafe or incomplete agent slice".to_string()
        }
    };
    BoundAgentE2EReport {
        session_id: input.session_id,
        user_prompt: input.user_prompt,
        phases,
        gate_outcomes,
        completion,
        final_state,
        disposition,
        stopped_at,
        operator_visible_summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bound_runtime::*;

    fn registry(authority: RuntimeAuthority) -> RuntimeRegistrySnapshot {
        RuntimeRegistrySnapshot {
            authority,
            compiled_snapshot_current: matches!(authority, RuntimeAuthority::CompiledJson),
            source_hashes: vec![SourceHash {
                path: ".vac/capabilities/vac-runtime-agent_loop.yaml".to_string(),
                sha256: "sha256:abc".to_string(),
            }],
            capabilities: vec![CapabilityRuntimeRecord {
                id: "vac.runtime.agent_loop".to_string(),
                readiness: ReadinessTriplet {
                    declared: ReadinessLevel::Ready,
                    computed: ReadinessLevel::Ready,
                    effective: ReadinessLevel::Ready,
                    reason: "fixture ready".to_string(),
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

    fn plan() -> SemanticPlan {
        SemanticPlan {
            id: "plan.runtime-e2e".to_string(),
            status: PlanStatus::Approved,
            capability: "vac.runtime.agent_loop".to_string(),
            allowed_files: vec![PlanFileScope {
                path: "vac-rs/crates/runtime/vac-agent-loop/src/runtime_e2e.rs".to_string(),
                operation: FileOperation::Modify,
                line_range: LineRange { start: 1, end: 500 },
                semantic_anchor: SemanticAnchor {
                    symbol: "run_bound_agent_e2e".to_string(),
                    kind: "function".to_string(),
                    ast_path: Some("crate::runtime_e2e::run_bound_agent_e2e".to_string()),
                    normalized_fingerprint: Some("sha256:anchor".to_string()),
                },
                ownership: "vac.runtime.agent_loop".to_string(),
            }],
            forbidden_files: vec!["target/**".to_string(), ".git/**".to_string()],
            validation_commands: vec!["sv.runtime.agent_e2e".to_string()],
            approval: PlanApproval {
                required: false,
                approved: true,
                plan_hash: Some("sha256:plan".to_string()),
            },
            bounds: PatchBudget {
                max_patches: 2,
                max_new_files: 0,
                max_line_delta: 100,
            },
        }
    }

    fn artifacts(session: &str) -> SessionArtifacts {
        completed_artifacts(session, "plan.runtime-e2e", "evidence.runtime-e2e")
    }

    fn patch() -> PatchAttempt {
        PatchAttempt {
            path: "vac-rs/crates/runtime/vac-agent-loop/src/runtime_e2e.rs".to_string(),
            operation: FileOperation::Modify,
            touched_range: LineRange { start: 10, end: 20 },
            semantic_anchor: SemanticAnchor {
                symbol: "run_bound_agent_e2e".to_string(),
                kind: "function".to_string(),
                ast_path: Some("crate::runtime_e2e::run_bound_agent_e2e".to_string()),
                normalized_fingerprint: Some("sha256:anchor".to_string()),
            },
            lines_added: 3,
            lines_removed: 1,
            creates_new_file: false,
            patch_index: 0,
        }
    }

    fn input(authority: RuntimeAuthority) -> BoundAgentE2EInput {
        BoundAgentE2EInput {
            session_id: "session.runtime-e2e".to_string(),
            user_prompt: "validate VAC runtime behavior".to_string(),
            registry: registry(authority),
            plan: plan(),
            artifacts: artifacts("session.runtime-e2e"),
            patch_attempts: vec![patch()],
            validation_commands: vec![StructuredCommand {
                id: "sv.runtime.agent_e2e".to_string(),
                runner: "python".to_string(),
                args: vec!["scripts/vac-runtime-agent-e2e-sv.py".to_string()],
                risk: CommandRisk::SafeRead,
                approval: CommandApproval::Never,
                policy_decision: PolicyDecision::Allow,
            }],
            operator_approved_command_ids: Vec::new(),
            commands_exit_zero: true,
            no_new_orphans: true,
            closeout: successful_closeout(
                "session.runtime-e2e",
                "plan.runtime-e2e",
                "evidence.runtime-e2e",
            ),
            requested_l2_claim: false,
        }
    }

    #[test]
    fn e2e_happy_path_reaches_done() {
        let report = run_bound_agent_e2e(
            input(RuntimeAuthority::CompiledJson),
            BoundRuntimeConfig::default(),
        );
        assert!(report.is_done());
        assert_eq!(report.phases.last(), Some(&BoundRuntimePhase::Done));
    }

    #[test]
    fn e2e_blocks_raw_yaml_authority_before_patch() {
        let report = run_bound_agent_e2e(
            input(RuntimeAuthority::RawYaml),
            BoundRuntimeConfig::default(),
        );
        assert_eq!(report.stopped_at, Some(RuntimeGate::PrePlan));
        assert!(!report.blockers().is_empty());
    }

    #[test]
    fn e2e_pauses_on_unapproved_destructive_command() {
        let mut i = input(RuntimeAuthority::CompiledJson);
        i.validation_commands = vec![StructuredCommand {
            id: "cleanup.target".to_string(),
            runner: "rm".to_string(),
            args: vec!["-rf".to_string(), "target/debug/incremental".to_string()],
            risk: CommandRisk::High,
            approval: CommandApproval::Always,
            policy_decision: PolicyDecision::ApprovalRequired,
        }];
        let report = run_bound_agent_e2e(i, BoundRuntimeConfig::default());
        assert_eq!(report.stopped_at, Some(RuntimeGate::PreCommand));
        assert!(report.is_paused_for_operator());
    }

    #[test]
    fn e2e_completion_lock_blocks_open_runtime_journal_projection_todo() {
        let mut i = input(RuntimeAuthority::CompiledJson);
        i.closeout.artifacts.todo.state = TodoArtifactState::Open;
        i.closeout.artifacts.todo.items[0].checked = false;
        let report = run_bound_agent_e2e(i, BoundRuntimeConfig::default());
        assert_eq!(report.stopped_at, Some(RuntimeGate::CompletionLock));
        assert!(report.blockers().iter().any(|item| item == "todo_terminal"));
    }
}
