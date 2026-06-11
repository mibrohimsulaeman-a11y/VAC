use crate::types::{
    AgentCommand, ProposedToolCall, ToolApprovalAction, ToolApprovalPolicy, ToolDecision,
};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ApprovalEntryState {
    PendingUserDecision,
    Ready(ToolDecision),
    Dispatched,
}

#[derive(Debug, Clone, PartialEq)]
struct ApprovalEntry {
    tool_call: ProposedToolCall,
    state: ApprovalEntryState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedToolCall {
    pub tool_call: ProposedToolCall,
    pub decision: ToolDecision,
}

#[derive(Debug, Clone)]
pub struct ApprovalStateMachine {
    entries: Vec<ApprovalEntry>,
    next_index: usize,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ApprovalError {
    #[error("unknown tool_call_id: {tool_call_id}")]
    UnknownToolCallId { tool_call_id: String },

    #[error("tool_call_id {tool_call_id} is already resolved")]
    AlreadyResolved { tool_call_id: String },

    #[error("invalid approval command for state machine")]
    InvalidCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ApprovalBindingV2 {
    pub plan_hash: String,
    pub diff_hash: String,
    pub policy_snapshot_hash: String,
    pub nonce: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalSignatureAlgorithm {
    Ed25519,
    Webauthn,
    Minisign,
    Sigstore,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalSignatureMode {
    Signed,
    IntegrityHint,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ApprovalSignatureV2 {
    pub algorithm: ApprovalSignatureAlgorithm,
    pub key_id: String,
    pub value: Option<String>,
    pub mode: ApprovalSignatureMode,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecisionV2 {
    Approved,
    Denied,
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ApprovalResponseV2 {
    pub decision: ApprovalDecisionV2,
    pub decided_by: String,
    pub decided_at: Option<String>,
    pub content_hash: String,
    pub operator_sig: ApprovalSignatureV2,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BoundApprovalRequestV2 {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub action: String,
    pub capability: String,
    pub plan_id: String,
    pub binding: ApprovalBindingV2,
    pub response: Option<ApprovalResponseV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ApprovalRoleGrant {
    pub role: String,
    pub actions: Vec<String>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ApprovalReplayGuard {
    pub consumed_nonces: Vec<String>,
    pub current_plan_hash: String,
    pub current_diff_hash: String,
    pub current_policy_snapshot_hash: String,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ApprovalV2Error {
    #[error("invalid approval schema/kind")]
    InvalidEnvelope,
    #[error("approval is missing operator response")]
    MissingResponse,
    #[error("approval was not approved")]
    NotApproved,
    #[error("approval nonce was already consumed")]
    Replay,
    #[error("approval plan hash changed")]
    PlanHashChanged,
    #[error("approval diff hash changed")]
    DiffHashChanged,
    #[error("approval policy snapshot hash changed")]
    PolicySnapshotChanged,
    #[error("approval signature is integrity-hint only")]
    IntegrityHintOnly,
    #[error("approval role is not authorized for action/capability")]
    UnauthorizedRole,
}

pub fn validate_v2_approval_binding(
    request: &BoundApprovalRequestV2,
    guard: &ApprovalReplayGuard,
    grants: &[ApprovalRoleGrant],
) -> Result<(), ApprovalV2Error> {
    if request.schema_version != 2 || request.kind != "approval_request" {
        return Err(ApprovalV2Error::InvalidEnvelope);
    }
    if guard
        .consumed_nonces
        .iter()
        .any(|nonce| nonce == &request.binding.nonce)
    {
        return Err(ApprovalV2Error::Replay);
    }
    if request.binding.plan_hash != guard.current_plan_hash {
        return Err(ApprovalV2Error::PlanHashChanged);
    }
    if request.binding.diff_hash != guard.current_diff_hash {
        return Err(ApprovalV2Error::DiffHashChanged);
    }
    if request.binding.policy_snapshot_hash != guard.current_policy_snapshot_hash {
        return Err(ApprovalV2Error::PolicySnapshotChanged);
    }
    let Some(response) = &request.response else {
        return Err(ApprovalV2Error::MissingResponse);
    };
    if response.decision != ApprovalDecisionV2::Approved {
        return Err(ApprovalV2Error::NotApproved);
    }
    if matches!(
        response.operator_sig.mode,
        ApprovalSignatureMode::IntegrityHint
    ) {
        return Err(ApprovalV2Error::IntegrityHintOnly);
    }
    let authorized = grants.iter().any(|grant| {
        grant.role == response.decided_by
            && grant
                .actions
                .iter()
                .any(|action| action == &request.action || action == "*")
            && grant
                .capabilities
                .iter()
                .any(|capability| capability == &request.capability || capability == "*")
    });
    if !authorized {
        return Err(ApprovalV2Error::UnauthorizedRole);
    }
    Ok(())
}

impl BoundApprovalRequestV2 {
    #[must_use]
    pub fn approval_content_projection(&self) -> serde_json::Value {
        serde_json::json!({
            "schema_version": self.schema_version,
            "kind": self.kind,
            "id": self.id,
            "action": self.action,
            "capability": self.capability,
            "plan_id": self.plan_id,
            "binding": self.binding,
            "response": self.response,
        })
    }
}

impl ApprovalStateMachine {
    pub fn new(tool_calls: Vec<ProposedToolCall>, policy: &ToolApprovalPolicy) -> Self {
        let entries = tool_calls
            .into_iter()
            .map(|tool_call| {
                let initial_state = match policy
                    .action_for(&tool_call.name, Some(&tool_call.arguments))
                {
                    ToolApprovalAction::Approve => ApprovalEntryState::Ready(ToolDecision::Accept),
                    ToolApprovalAction::Deny => ApprovalEntryState::Ready(ToolDecision::Reject),
                    ToolApprovalAction::Ask => ApprovalEntryState::PendingUserDecision,
                };

                ApprovalEntry {
                    tool_call,
                    state: initial_state,
                }
            })
            .collect();

        Self {
            entries,
            next_index: 0,
        }
    }

    pub fn pending_tool_call_ids(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter_map(|entry| {
                if matches!(entry.state, ApprovalEntryState::PendingUserDecision) {
                    Some(entry.tool_call.id.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn is_waiting_for_user(&self) -> bool {
        self.entries
            .iter()
            .any(|entry| matches!(entry.state, ApprovalEntryState::PendingUserDecision))
    }

    pub fn is_complete(&self) -> bool {
        self.next_index >= self.entries.len()
    }

    pub fn apply_command(&mut self, command: AgentCommand) -> Result<(), ApprovalError> {
        match command {
            AgentCommand::ResolveTool {
                tool_call_id,
                decision,
            } => self.resolve_tool(&tool_call_id, decision),
            AgentCommand::ResolveTools { decisions } => {
                for (tool_call_id, decision) in decisions {
                    self.resolve_tool(&tool_call_id, decision)?;
                }
                Ok(())
            }
            _ => Err(ApprovalError::InvalidCommand),
        }
    }

    pub fn resolve_tool(
        &mut self,
        tool_call_id: &str,
        decision: ToolDecision,
    ) -> Result<(), ApprovalError> {
        let maybe_entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.tool_call.id == tool_call_id);

        let Some(entry) = maybe_entry else {
            return Err(ApprovalError::UnknownToolCallId {
                tool_call_id: tool_call_id.to_string(),
            });
        };

        match &entry.state {
            ApprovalEntryState::PendingUserDecision => {
                entry.state = ApprovalEntryState::Ready(decision);
                Ok(())
            }
            ApprovalEntryState::Ready(existing) if *existing == decision => Ok(()),
            ApprovalEntryState::Ready(_) | ApprovalEntryState::Dispatched => {
                Err(ApprovalError::AlreadyResolved {
                    tool_call_id: tool_call_id.to_string(),
                })
            }
        }
    }

    pub fn pause_for_vac_runtime_approval(
        &mut self,
        tool_call_id: &str,
    ) -> Result<(), ApprovalError> {
        let Some((idx, entry)) = self
            .entries
            .iter_mut()
            .enumerate()
            .find(|(_, entry)| entry.tool_call.id == tool_call_id)
        else {
            return Err(ApprovalError::UnknownToolCallId {
                tool_call_id: tool_call_id.to_string(),
            });
        };
        entry.state = ApprovalEntryState::PendingUserDecision;
        if idx < self.next_index {
            self.next_index = idx;
        }
        Ok(())
    }

    pub fn next_ready(&mut self) -> Option<ResolvedToolCall> {
        while self.next_index < self.entries.len() {
            let entry = self.entries.get_mut(self.next_index)?;

            match &entry.state {
                ApprovalEntryState::PendingUserDecision => return None,
                ApprovalEntryState::Ready(decision) => {
                    let resolved = ResolvedToolCall {
                        tool_call: entry.tool_call.clone(),
                        decision: decision.clone(),
                    };
                    entry.state = ApprovalEntryState::Dispatched;
                    self.next_index += 1;
                    return Some(resolved);
                }
                ApprovalEntryState::Dispatched => {
                    self.next_index += 1;
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolApprovalAction;
    use serde_json::json;
    use std::collections::HashMap;

    fn tool_call(id: &str, name: &str) -> ProposedToolCall {
        ProposedToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments: json!({"input": id}),
            metadata: None,
        }
    }

    #[test]
    fn incremental_decisions_buffer_until_prior_call_is_resolved() {
        let calls = vec![tool_call("tc_1", "tool_a"), tool_call("tc_2", "tool_b")];
        let mut machine = ApprovalStateMachine::new(calls, &ToolApprovalPolicy::None);

        let out_of_order = machine.resolve_tool("tc_2", ToolDecision::Accept);
        assert!(out_of_order.is_ok());

        assert!(machine.next_ready().is_none());

        let first_resolution = machine.resolve_tool("tc_1", ToolDecision::Reject);
        assert!(first_resolution.is_ok());

        let first = machine.next_ready();
        assert_eq!(
            first,
            Some(ResolvedToolCall {
                tool_call: tool_call("tc_1", "tool_a"),
                decision: ToolDecision::Reject,
            })
        );

        let second = machine.next_ready();
        assert_eq!(
            second,
            Some(ResolvedToolCall {
                tool_call: tool_call("tc_2", "tool_b"),
                decision: ToolDecision::Accept,
            })
        );

        assert!(machine.is_complete());
    }

    #[test]
    fn bulk_resolution_resolves_multiple_calls() {
        let calls = vec![
            tool_call("tc_1", "tool_a"),
            tool_call("tc_2", "tool_b"),
            tool_call("tc_3", "tool_c"),
        ];
        let mut machine = ApprovalStateMachine::new(calls, &ToolApprovalPolicy::None);

        let mut decisions = HashMap::new();
        decisions.insert("tc_1".to_string(), ToolDecision::Accept);
        decisions.insert("tc_2".to_string(), ToolDecision::Reject);

        let command_result = machine.apply_command(AgentCommand::ResolveTools { decisions });
        assert!(command_result.is_ok());

        assert_eq!(
            machine.next_ready(),
            Some(ResolvedToolCall {
                tool_call: tool_call("tc_1", "tool_a"),
                decision: ToolDecision::Accept,
            })
        );

        assert_eq!(
            machine.next_ready(),
            Some(ResolvedToolCall {
                tool_call: tool_call("tc_2", "tool_b"),
                decision: ToolDecision::Reject,
            })
        );

        assert!(machine.next_ready().is_none());
        assert_eq!(machine.pending_tool_call_ids(), vec!["tc_3".to_string()]);
    }

    #[test]
    fn policy_applies_auto_approve_and_auto_deny() {
        let calls = vec![
            tool_call("tc_1", "safe_tool"),
            tool_call("tc_2", "danger_tool"),
            tool_call("tc_3", "unknown_tool"),
        ];

        let mut rules = HashMap::new();
        rules.insert("safe_tool".to_string(), ToolApprovalAction::Approve);
        rules.insert("danger_tool".to_string(), ToolApprovalAction::Deny);

        let policy = ToolApprovalPolicy::Custom {
            rules,
            default: ToolApprovalAction::Ask,
        };

        let mut machine = ApprovalStateMachine::new(calls, &policy);

        assert_eq!(
            machine.next_ready(),
            Some(ResolvedToolCall {
                tool_call: tool_call("tc_1", "safe_tool"),
                decision: ToolDecision::Accept,
            })
        );

        assert_eq!(
            machine.next_ready(),
            Some(ResolvedToolCall {
                tool_call: tool_call("tc_2", "danger_tool"),
                decision: ToolDecision::Reject,
            })
        );

        assert!(machine.next_ready().is_none());
        assert_eq!(machine.pending_tool_call_ids(), vec!["tc_3".to_string()]);
    }

    #[test]
    fn resolve_unknown_tool_call_returns_error() {
        let calls = vec![tool_call("tc_1", "tool_a")];
        let mut machine = ApprovalStateMachine::new(calls, &ToolApprovalPolicy::None);

        let error = machine.resolve_tool("tc_missing", ToolDecision::Accept);

        assert_eq!(
            error,
            Err(ApprovalError::UnknownToolCallId {
                tool_call_id: "tc_missing".to_string(),
            })
        );
    }

    #[test]
    fn resolve_same_decision_is_idempotent() {
        let calls = vec![tool_call("tc_1", "tool_a")];
        let mut machine = ApprovalStateMachine::new(calls, &ToolApprovalPolicy::None);

        assert!(machine.resolve_tool("tc_1", ToolDecision::Accept).is_ok());
        assert!(machine.resolve_tool("tc_1", ToolDecision::Accept).is_ok());

        assert_eq!(
            machine.next_ready(),
            Some(ResolvedToolCall {
                tool_call: tool_call("tc_1", "tool_a"),
                decision: ToolDecision::Accept,
            })
        );
    }
}
