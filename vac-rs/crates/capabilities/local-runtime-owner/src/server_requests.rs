//! Pending server-request registry seam.
//!
//! The registry is the local runtime owner's deterministic owner for
//! operator-facing pending requests. It deliberately models product/runtime
//! semantics instead of app-server DTO ownership: callers insert a pending
//! request, then later resolve, reject, cancel, or drain it during shutdown.

use std::collections::BTreeMap;
use std::fmt;

use vac_core::local_runtime::ApprovalDecision;
use vac_core::local_runtime::TaskId;

const DEFAULT_MAX_PENDING_REQUESTS: usize = 4_096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServerRequestRegistry {
    pending: BTreeMap<ServerRequestId, PendingServerRequest>,
    max_pending: usize,
    next_sequence: u64,
}

impl Default for ServerRequestRegistry {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_MAX_PENDING_REQUESTS)
    }
}

impl ServerRequestRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_capacity(max_pending: usize) -> Self {
        Self {
            pending: BTreeMap::new(),
            max_pending,
            next_sequence: 0,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    #[must_use]
    pub fn contains(&self, request_id: &ServerRequestId) -> bool {
        self.pending.contains_key(request_id)
    }

    #[must_use]
    pub fn get(&self, request_id: &ServerRequestId) -> Option<&PendingServerRequest> {
        self.pending.get(request_id)
    }

    pub fn insert_generated(
        &mut self,
        kind: PendingServerRequestKind,
    ) -> Result<ServerRequestId, ServerRequestRegistryError> {
        let request_id = self.next_generated_request_id();
        self.insert(PendingServerRequest::new(request_id.clone(), kind))?;
        Ok(request_id)
    }

    pub fn insert(
        &mut self,
        request: PendingServerRequest,
    ) -> Result<(), ServerRequestRegistryError> {
        if self.pending.contains_key(&request.id) {
            return Err(ServerRequestRegistryError::DuplicateId {
                request_id: request.id,
            });
        }
        if self.pending.len() >= self.max_pending {
            return Err(ServerRequestRegistryError::Saturated {
                request_id: request.id,
                capacity: self.max_pending,
            });
        }
        self.pending.insert(request.id.clone(), request);
        Ok(())
    }

    pub fn resolve(
        &mut self,
        request_id: &ServerRequestId,
        response: ServerRequestResolution,
    ) -> Result<RuntimeRequestDecision, ServerRequestRegistryError> {
        let pending = self.take(request_id)?;
        RuntimeRequestDecision::resolved(pending, response)
    }

    pub fn reject(
        &mut self,
        request_id: &ServerRequestId,
        reason: impl Into<String>,
    ) -> Result<RuntimeRequestDecision, ServerRequestRegistryError> {
        let pending = self.take(request_id)?;
        Ok(RuntimeRequestDecision::rejected(pending, reason.into()))
    }

    pub fn cancel(
        &mut self,
        request_id: &ServerRequestId,
        reason: impl Into<String>,
    ) -> Result<RuntimeRequestDecision, ServerRequestRegistryError> {
        let pending = self.take(request_id)?;
        Ok(RuntimeRequestDecision::cancelled(pending, reason.into()))
    }

    pub fn shutdown(&mut self, reason: impl Into<String>) -> Vec<RuntimeRequestDecision> {
        let reason = reason.into();
        self.pending
            .split_off(&ServerRequestId::min_value())
            .into_values()
            .map(|pending| RuntimeRequestDecision::cancelled(pending, reason.clone()))
            .collect()
    }

    fn take(
        &mut self,
        request_id: &ServerRequestId,
    ) -> Result<PendingServerRequest, ServerRequestRegistryError> {
        self.pending
            .remove(request_id)
            .ok_or_else(|| ServerRequestRegistryError::UnknownId {
                request_id: request_id.clone(),
            })
    }

    fn next_generated_request_id(&mut self) -> ServerRequestId {
        loop {
            self.next_sequence = self.next_sequence.saturating_add(1);
            let request_id = ServerRequestId(format!("server-request:{:016}", self.next_sequence));
            if !self.pending.contains_key(&request_id) {
                return request_id;
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingServerRequest {
    pub id: ServerRequestId,
    pub task_id: Option<TaskId>,
    pub kind: PendingServerRequestKind,
    pub evidence: Vec<String>,
}

impl PendingServerRequest {
    #[must_use]
    pub fn new(id: ServerRequestId, kind: PendingServerRequestKind) -> Self {
        Self {
            id,
            task_id: None,
            kind,
            evidence: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_task_id(mut self, task_id: TaskId) -> Self {
        self.task_id = Some(task_id);
        self
    }

    #[must_use]
    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence.push(evidence.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PendingServerRequestKind {
    ExecApproval {
        approval_id: String,
        turn_id: Option<String>,
    },
    PatchApproval {
        approval_id: String,
    },
    UserInput {
        turn_id: String,
    },
    Permissions {
        approval_id: String,
    },
    McpElicitation {
        server_name: String,
        request_id: String,
    },
}

impl PendingServerRequestKind {
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::ExecApproval { .. } => "exec_approval",
            Self::PatchApproval { .. } => "patch_approval",
            Self::UserInput { .. } => "user_input",
            Self::Permissions { .. } => "permissions",
            Self::McpElicitation { .. } => "mcp_elicitation",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ServerRequestId(String);

impl ServerRequestId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn min_value() -> Self {
        Self(String::new())
    }
}

impl fmt::Display for ServerRequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for ServerRequestId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for ServerRequestId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServerRequestResolution {
    Approval { decision: ApprovalDecision },
    UserInput { answers: Vec<UserInputAnswer> },
    Permissions(PermissionsDecision),
    McpElicitation(McpElicitationDecision),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserInputAnswer {
    pub question_id: String,
    pub answer: String,
}

impl UserInputAnswer {
    #[must_use]
    pub fn new(question_id: impl Into<String>, answer: impl Into<String>) -> Self {
        Self {
            question_id: question_id.into(),
            answer: answer.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PermissionsDecision {
    pub granted: bool,
    pub scope: PermissionScope,
    pub strict_auto_review: bool,
}

impl PermissionsDecision {
    #[must_use]
    pub fn denied() -> Self {
        Self {
            granted: false,
            scope: PermissionScope::None,
            strict_auto_review: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PermissionScope {
    None,
    Request,
    Turn,
    Session,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct McpElicitationDecision {
    pub action: McpElicitationAction,
    pub content: Option<String>,
}

impl McpElicitationDecision {
    #[must_use]
    pub fn rejected() -> Self {
        Self {
            action: McpElicitationAction::Reject,
            content: None,
        }
    }

    #[must_use]
    pub fn cancelled() -> Self {
        Self {
            action: McpElicitationAction::Cancel,
            content: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum McpElicitationAction {
    Accept,
    Reject,
    Cancel,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeRequestDecision {
    ExecApproval {
        request_id: ServerRequestId,
        approval_id: String,
        turn_id: Option<String>,
        decision: ApprovalDecision,
    },
    PatchApproval {
        request_id: ServerRequestId,
        approval_id: String,
        decision: ApprovalDecision,
    },
    UserInputAnswer {
        request_id: ServerRequestId,
        turn_id: String,
        answers: Vec<UserInputAnswer>,
        cancelled: bool,
    },
    PermissionsResponse {
        request_id: ServerRequestId,
        approval_id: String,
        decision: PermissionsDecision,
    },
    McpElicitationResolution {
        request_id: ServerRequestId,
        server_name: String,
        elicitation_request_id: String,
        decision: McpElicitationDecision,
    },
}

impl RuntimeRequestDecision {
    fn resolved(
        pending: PendingServerRequest,
        response: ServerRequestResolution,
    ) -> Result<Self, ServerRequestRegistryError> {
        let request_id = pending.id;
        match (pending.kind, response) {
            (
                PendingServerRequestKind::ExecApproval {
                    approval_id,
                    turn_id,
                },
                ServerRequestResolution::Approval { decision },
            ) => Ok(Self::ExecApproval {
                request_id,
                approval_id,
                turn_id,
                decision,
            }),
            (
                PendingServerRequestKind::PatchApproval { approval_id },
                ServerRequestResolution::Approval { decision },
            ) => Ok(Self::PatchApproval {
                request_id,
                approval_id,
                decision,
            }),
            (
                PendingServerRequestKind::UserInput { turn_id },
                ServerRequestResolution::UserInput { answers },
            ) => Ok(Self::UserInputAnswer {
                request_id,
                turn_id,
                answers,
                cancelled: false,
            }),
            (
                PendingServerRequestKind::Permissions { approval_id },
                ServerRequestResolution::Permissions(decision),
            ) => Ok(Self::PermissionsResponse {
                request_id,
                approval_id,
                decision,
            }),
            (
                PendingServerRequestKind::McpElicitation {
                    server_name,
                    request_id: elicitation_request_id,
                },
                ServerRequestResolution::McpElicitation(decision),
            ) => Ok(Self::McpElicitationResolution {
                request_id,
                server_name,
                elicitation_request_id,
                decision,
            }),
            (kind, resolution) => Err(ServerRequestRegistryError::KindMismatch {
                request_kind: kind.label(),
                resolution_kind: resolution.label(),
                request_id,
            }),
        }
    }

    fn rejected(pending: PendingServerRequest, _reason: String) -> Self {
        Self::abort_or_cancel(pending, AbortStyle::Reject)
    }

    fn cancelled(pending: PendingServerRequest, _reason: String) -> Self {
        Self::abort_or_cancel(pending, AbortStyle::Cancel)
    }

    fn abort_or_cancel(pending: PendingServerRequest, style: AbortStyle) -> Self {
        let request_id = pending.id;
        match pending.kind {
            PendingServerRequestKind::ExecApproval {
                approval_id,
                turn_id,
            } => Self::ExecApproval {
                request_id,
                approval_id,
                turn_id,
                decision: style.approval_decision(),
            },
            PendingServerRequestKind::PatchApproval { approval_id } => Self::PatchApproval {
                request_id,
                approval_id,
                decision: style.approval_decision(),
            },
            PendingServerRequestKind::UserInput { turn_id } => Self::UserInputAnswer {
                request_id,
                turn_id,
                answers: Vec::new(),
                cancelled: true,
            },
            PendingServerRequestKind::Permissions { approval_id } => Self::PermissionsResponse {
                request_id,
                approval_id,
                decision: PermissionsDecision::denied(),
            },
            PendingServerRequestKind::McpElicitation {
                server_name,
                request_id: elicitation_request_id,
            } => Self::McpElicitationResolution {
                request_id,
                server_name,
                elicitation_request_id,
                decision: match style {
                    AbortStyle::Reject => McpElicitationDecision::rejected(),
                    AbortStyle::Cancel => McpElicitationDecision::cancelled(),
                },
            },
        }
    }
}

impl ServerRequestResolution {
    fn label(&self) -> &'static str {
        match self {
            Self::Approval { .. } => "approval",
            Self::UserInput { .. } => "user_input",
            Self::Permissions(_) => "permissions",
            Self::McpElicitation(_) => "mcp_elicitation",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AbortStyle {
    Reject,
    Cancel,
}

impl AbortStyle {
    fn approval_decision(self) -> ApprovalDecision {
        match self {
            Self::Reject => ApprovalDecision::Rejected,
            Self::Cancel => ApprovalDecision::Cancelled,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServerRequestRegistryError {
    DuplicateId {
        request_id: ServerRequestId,
    },
    UnknownId {
        request_id: ServerRequestId,
    },
    Saturated {
        request_id: ServerRequestId,
        capacity: usize,
    },
    KindMismatch {
        request_id: ServerRequestId,
        request_kind: &'static str,
        resolution_kind: &'static str,
    },
}

impl fmt::Display for ServerRequestRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateId { request_id } => {
                write!(f, "duplicate server request id: {request_id}")
            }
            Self::UnknownId { request_id } => write!(f, "unknown server request id: {request_id}"),
            Self::Saturated {
                request_id,
                capacity,
            } => write!(
                f,
                "server request registry is saturated at {capacity} pending requests; rejected {request_id}"
            ),
            Self::KindMismatch {
                request_id,
                request_kind,
                resolution_kind,
            } => write!(
                f,
                "server request {request_id} is {request_kind}, cannot resolve with {resolution_kind}"
            ),
        }
    }
}

impl std::error::Error for ServerRequestRegistryError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_request(
        registry: &mut ServerRequestRegistry,
        request_id: &str,
        kind: PendingServerRequestKind,
    ) -> ServerRequestId {
        let request_id = ServerRequestId::new(request_id);
        let result = registry.insert(PendingServerRequest::new(request_id.clone(), kind));
        assert!(result.is_ok(), "insert should succeed: {result:?}");
        request_id
    }

    #[test]
    fn generated_ids_are_unique_and_stable() {
        let mut registry = ServerRequestRegistry::new();
        let first = registry.insert_generated(PendingServerRequestKind::PatchApproval {
            approval_id: "patch-1".to_string(),
        });
        let second = registry.insert_generated(PendingServerRequestKind::PatchApproval {
            approval_id: "patch-2".to_string(),
        });

        let (first, second) = match (first, second) {
            (Ok(first), Ok(second)) => (first, second),
            other => panic!("generated insertions should succeed: {other:?}"),
        };
        assert_ne!(first, second);
        assert_eq!(first.as_str(), "server-request:0000000000000001");
        assert_eq!(second.as_str(), "server-request:0000000000000002");
    }

    #[test]
    fn duplicate_ids_are_rejected_without_replacing_original() {
        let mut registry = ServerRequestRegistry::new();
        let request_id = insert_request(
            &mut registry,
            "req-1",
            PendingServerRequestKind::PatchApproval {
                approval_id: "patch-1".to_string(),
            },
        );
        let duplicate = registry.insert(PendingServerRequest::new(
            request_id.clone(),
            PendingServerRequestKind::PatchApproval {
                approval_id: "patch-2".to_string(),
            },
        ));

        assert_eq!(
            duplicate,
            Err(ServerRequestRegistryError::DuplicateId {
                request_id: request_id.clone(),
            })
        );
        assert_eq!(registry.len(), 1);
        assert_eq!(
            registry.get(&request_id).map(|request| &request.kind),
            Some(&PendingServerRequestKind::PatchApproval {
                approval_id: "patch-1".to_string(),
            })
        );
    }

    #[test]
    fn unknown_ids_return_structured_errors() {
        let mut registry = ServerRequestRegistry::new();
        let request_id = ServerRequestId::new("missing");
        let result = registry.reject(&request_id, "operator rejected stale prompt");

        assert_eq!(
            result,
            Err(ServerRequestRegistryError::UnknownId { request_id })
        );
    }

    #[test]
    fn resolve_exec_approval_maps_to_runtime_decision() {
        let mut registry = ServerRequestRegistry::new();
        let request_id = insert_request(
            &mut registry,
            "exec-req",
            PendingServerRequestKind::ExecApproval {
                approval_id: "exec-approval".to_string(),
                turn_id: Some("turn-a".to_string()),
            },
        );

        let decision = registry.resolve(
            &request_id,
            ServerRequestResolution::Approval {
                decision: ApprovalDecision::Approved,
            },
        );

        assert_eq!(
            decision,
            Ok(RuntimeRequestDecision::ExecApproval {
                request_id,
                approval_id: "exec-approval".to_string(),
                turn_id: Some("turn-a".to_string()),
                decision: ApprovalDecision::Approved,
            })
        );
        assert!(registry.is_empty());
    }

    #[test]
    fn reject_exec_approval_maps_to_safe_abort() {
        let mut registry = ServerRequestRegistry::new();
        let request_id = insert_request(
            &mut registry,
            "exec-req",
            PendingServerRequestKind::ExecApproval {
                approval_id: "exec-approval".to_string(),
                turn_id: None,
            },
        );

        let decision = registry.reject(&request_id, "operator rejected");

        assert_eq!(
            decision,
            Ok(RuntimeRequestDecision::ExecApproval {
                request_id,
                approval_id: "exec-approval".to_string(),
                turn_id: None,
                decision: ApprovalDecision::Rejected,
            })
        );
    }

    #[test]
    fn resolve_patch_approval_maps_to_runtime_decision() {
        let mut registry = ServerRequestRegistry::new();
        let request_id = insert_request(
            &mut registry,
            "patch-req",
            PendingServerRequestKind::PatchApproval {
                approval_id: "patch-call".to_string(),
            },
        );

        let decision = registry.resolve(
            &request_id,
            ServerRequestResolution::Approval {
                decision: ApprovalDecision::Approved,
            },
        );

        assert_eq!(
            decision,
            Ok(RuntimeRequestDecision::PatchApproval {
                request_id,
                approval_id: "patch-call".to_string(),
                decision: ApprovalDecision::Approved,
            })
        );
    }

    #[test]
    fn user_input_round_trips_answers_and_rejects_to_cancelled_empty_answer() {
        let mut registry = ServerRequestRegistry::new();
        let request_id = insert_request(
            &mut registry,
            "input-req",
            PendingServerRequestKind::UserInput {
                turn_id: "turn-input".to_string(),
            },
        );

        let decision = registry.resolve(
            &request_id,
            ServerRequestResolution::UserInput {
                answers: vec![UserInputAnswer::new("question-1", "answer")],
            },
        );

        assert_eq!(
            decision,
            Ok(RuntimeRequestDecision::UserInputAnswer {
                request_id: request_id.clone(),
                turn_id: "turn-input".to_string(),
                answers: vec![UserInputAnswer::new("question-1", "answer")],
                cancelled: false,
            })
        );

        let request_id = insert_request(
            &mut registry,
            "input-req-2",
            PendingServerRequestKind::UserInput {
                turn_id: "turn-input-2".to_string(),
            },
        );
        let rejected = registry.reject(&request_id, "operator skipped input");
        assert_eq!(
            rejected,
            Ok(RuntimeRequestDecision::UserInputAnswer {
                request_id,
                turn_id: "turn-input-2".to_string(),
                answers: Vec::new(),
                cancelled: true,
            })
        );
    }

    #[test]
    fn permissions_round_trip_and_reject_denies() {
        let mut registry = ServerRequestRegistry::new();
        let request_id = insert_request(
            &mut registry,
            "permissions-req",
            PendingServerRequestKind::Permissions {
                approval_id: "permissions-call".to_string(),
            },
        );

        let granted = PermissionsDecision {
            granted: true,
            scope: PermissionScope::Turn,
            strict_auto_review: true,
        };
        let decision = registry.resolve(
            &request_id,
            ServerRequestResolution::Permissions(granted.clone()),
        );

        assert_eq!(
            decision,
            Ok(RuntimeRequestDecision::PermissionsResponse {
                request_id: request_id.clone(),
                approval_id: "permissions-call".to_string(),
                decision: granted,
            })
        );

        let request_id = insert_request(
            &mut registry,
            "permissions-req-2",
            PendingServerRequestKind::Permissions {
                approval_id: "permissions-call-2".to_string(),
            },
        );
        let rejected = registry.reject(&request_id, "operator denied permissions");
        assert_eq!(
            rejected,
            Ok(RuntimeRequestDecision::PermissionsResponse {
                request_id,
                approval_id: "permissions-call-2".to_string(),
                decision: PermissionsDecision::denied(),
            })
        );
    }

    #[test]
    fn mcp_elicitation_round_trip_rejects_and_cancels_explicitly() {
        let mut registry = ServerRequestRegistry::new();
        let request_id = insert_request(
            &mut registry,
            "mcp-req",
            PendingServerRequestKind::McpElicitation {
                server_name: "filesystem".to_string(),
                request_id: "mcp-inner-1".to_string(),
            },
        );

        let accepted = McpElicitationDecision {
            action: McpElicitationAction::Accept,
            content: Some("ok".to_string()),
        };
        let decision = registry.resolve(
            &request_id,
            ServerRequestResolution::McpElicitation(accepted.clone()),
        );
        assert_eq!(
            decision,
            Ok(RuntimeRequestDecision::McpElicitationResolution {
                request_id: request_id.clone(),
                server_name: "filesystem".to_string(),
                elicitation_request_id: "mcp-inner-1".to_string(),
                decision: accepted,
            })
        );

        let request_id = insert_request(
            &mut registry,
            "mcp-req-2",
            PendingServerRequestKind::McpElicitation {
                server_name: "github".to_string(),
                request_id: "mcp-inner-2".to_string(),
            },
        );
        let rejected = registry.reject(&request_id, "operator rejected elicitation");
        assert_eq!(
            rejected,
            Ok(RuntimeRequestDecision::McpElicitationResolution {
                request_id: request_id.clone(),
                server_name: "github".to_string(),
                elicitation_request_id: "mcp-inner-2".to_string(),
                decision: McpElicitationDecision::rejected(),
            })
        );

        let request_id = insert_request(
            &mut registry,
            "mcp-req-3",
            PendingServerRequestKind::McpElicitation {
                server_name: "github".to_string(),
                request_id: "mcp-inner-3".to_string(),
            },
        );
        let cancelled = registry.cancel(&request_id, "session shutdown");
        assert_eq!(
            cancelled,
            Ok(RuntimeRequestDecision::McpElicitationResolution {
                request_id,
                server_name: "github".to_string(),
                elicitation_request_id: "mcp-inner-3".to_string(),
                decision: McpElicitationDecision::cancelled(),
            })
        );
    }

    #[test]
    fn mismatched_resolution_fails_structurally_and_removes_request() {
        let mut registry = ServerRequestRegistry::new();
        let request_id = insert_request(
            &mut registry,
            "patch-req",
            PendingServerRequestKind::PatchApproval {
                approval_id: "patch-call".to_string(),
            },
        );

        let result = registry.resolve(
            &request_id,
            ServerRequestResolution::UserInput {
                answers: Vec::new(),
            },
        );

        assert_eq!(
            result,
            Err(ServerRequestRegistryError::KindMismatch {
                request_id,
                request_kind: "patch_approval",
                resolution_kind: "user_input",
            })
        );
        assert!(registry.is_empty());
    }

    #[test]
    fn saturation_rejects_new_request_without_hanging_existing_request() {
        let mut registry = ServerRequestRegistry::with_capacity(1);
        let existing_id = insert_request(
            &mut registry,
            "req-1",
            PendingServerRequestKind::PatchApproval {
                approval_id: "patch-1".to_string(),
            },
        );
        let overflow_id = ServerRequestId::new("req-2");

        let result = registry.insert(PendingServerRequest::new(
            overflow_id.clone(),
            PendingServerRequestKind::PatchApproval {
                approval_id: "patch-2".to_string(),
            },
        ));

        assert_eq!(
            result,
            Err(ServerRequestRegistryError::Saturated {
                request_id: overflow_id,
                capacity: 1,
            })
        );
        assert!(registry.contains(&existing_id));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn shutdown_cancels_all_pending_requests_deterministically() {
        let mut registry = ServerRequestRegistry::new();
        let exec_id = insert_request(
            &mut registry,
            "a-exec",
            PendingServerRequestKind::ExecApproval {
                approval_id: "exec".to_string(),
                turn_id: None,
            },
        );
        let patch_id = insert_request(
            &mut registry,
            "b-patch",
            PendingServerRequestKind::PatchApproval {
                approval_id: "patch".to_string(),
            },
        );

        let decisions = registry.shutdown("owner shutdown");

        assert_eq!(
            decisions,
            vec![
                RuntimeRequestDecision::ExecApproval {
                    request_id: exec_id,
                    approval_id: "exec".to_string(),
                    turn_id: None,
                    decision: ApprovalDecision::Cancelled,
                },
                RuntimeRequestDecision::PatchApproval {
                    request_id: patch_id,
                    approval_id: "patch".to_string(),
                    decision: ApprovalDecision::Cancelled,
                },
            ]
        );
        assert!(registry.is_empty());
    }
}
