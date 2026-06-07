use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ApprovalRequestId(String);

impl ApprovalRequestId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn for_workflow_step(workflow_id: &str, step_id: &str) -> Self {
        Self(format!("approval:{workflow_id}:{step_id}"))
    }

    pub fn for_workflow_step_run(workflow_id: &str, run_id: &str, step_id: &str) -> Self {
        Self::for_workflow_step_attempt(workflow_id, run_id, "attempt-1", step_id)
    }

    pub fn for_workflow_step_attempt(
        workflow_id: &str,
        run_id: &str,
        attempt_id: &str,
        step_id: &str,
    ) -> Self {
        Self(format!(
            "approval:{workflow_id}:run:{run_id}:attempt:{attempt_id}:step:{step_id}"
        ))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ApprovalRequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalAction {
    FilesystemWrite,
    FilesystemDelete,
    ProcessExecute,
    NetworkAccess,
    ToolCall,
    SessionWrite,
    Other,
}

impl ApprovalAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FilesystemWrite => "filesystem_write",
            Self::FilesystemDelete => "filesystem_delete",
            Self::ProcessExecute => "process_execute",
            Self::NetworkAccess => "network_access",
            Self::ToolCall => "tool_call",
            Self::SessionWrite => "session_write",
            Self::Other => "other",
        }
    }
}

impl fmt::Display for ApprovalAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalRisk {
    SafeRead,
    SafeWrite,
    Execute,
    Network,
    Destructive,
    Unknown,
}

impl ApprovalRisk {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SafeRead => "safe_read",
            Self::SafeWrite => "safe_write",
            Self::Execute => "execute",
            Self::Network => "network",
            Self::Destructive => "destructive",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for ApprovalRisk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

impl ApprovalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Expired => "expired",
        }
    }
}

impl fmt::Display for ApprovalStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: ApprovalRequestId,
    pub workflow_id: String,
    pub step_id: String,
    pub capability_id: String,
    pub action: ApprovalAction,
    pub reasons: Vec<String>,
    pub risk: ApprovalRisk,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_role: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub status: ApprovalStatus,
    pub decided_by: Option<String>,
    pub decided_at: Option<DateTime<Utc>>,
    pub decision_reason: Option<String>,
}

impl ApprovalRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn pending(
        id: ApprovalRequestId,
        workflow_id: impl Into<String>,
        step_id: impl Into<String>,
        capability_id: impl Into<String>,
        action: ApprovalAction,
        risk: ApprovalRisk,
        reasons: Vec<String>,
        requested_at: DateTime<Utc>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            workflow_id: workflow_id.into(),
            step_id: step_id.into(),
            capability_id: capability_id.into(),
            action,
            reasons,
            risk,
            required_role: None,
            requested_at,
            expires_at,
            status: ApprovalStatus::Pending,
            decided_by: None,
            decided_at: None,
            decision_reason: None,
        }
    }

    pub fn with_required_role(mut self, role: impl Into<String>) -> Self {
        let role = role.into();
        self.required_role = (!role.trim().is_empty()).then_some(role);
        self
    }

    pub fn requires_role(&self, role: &str) -> bool {
        self.required_role.as_deref() == Some(role)
    }

    pub fn is_pending(&self) -> bool {
        self.status == ApprovalStatus::Pending
    }

    pub fn is_expired_at(&self, now: DateTime<Utc>) -> bool {
        self.is_pending() && self.expires_at.is_some_and(|expires_at| expires_at <= now)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalDecision {
    Approved {
        actor: String,
        actor_role: Option<String>,
        reason: String,
        decided_at: DateTime<Utc>,
    },
    Rejected {
        actor: String,
        actor_role: Option<String>,
        reason: String,
        decided_at: DateTime<Utc>,
    },
    Expired {
        reason: String,
        decided_at: DateTime<Utc>,
    },
}

impl ApprovalDecision {
    pub fn approved(
        actor: impl Into<String>,
        reason: impl Into<String>,
        decided_at: DateTime<Utc>,
    ) -> Self {
        Self::Approved {
            actor: actor.into(),
            actor_role: None,
            reason: reason.into(),
            decided_at,
        }
    }

    pub fn approved_with_role(
        actor: impl Into<String>,
        actor_role: impl Into<String>,
        reason: impl Into<String>,
        decided_at: DateTime<Utc>,
    ) -> Self {
        Self::Approved {
            actor: actor.into(),
            actor_role: Some(actor_role.into()),
            reason: reason.into(),
            decided_at,
        }
    }

    pub fn rejected(
        actor: impl Into<String>,
        reason: impl Into<String>,
        decided_at: DateTime<Utc>,
    ) -> Self {
        Self::Rejected {
            actor: actor.into(),
            actor_role: None,
            reason: reason.into(),
            decided_at,
        }
    }

    pub fn rejected_with_role(
        actor: impl Into<String>,
        actor_role: impl Into<String>,
        reason: impl Into<String>,
        decided_at: DateTime<Utc>,
    ) -> Self {
        Self::Rejected {
            actor: actor.into(),
            actor_role: Some(actor_role.into()),
            reason: reason.into(),
            decided_at,
        }
    }

    pub fn expired(reason: impl Into<String>, decided_at: DateTime<Utc>) -> Self {
        Self::Expired {
            reason: reason.into(),
            decided_at,
        }
    }

    fn status(&self) -> ApprovalStatus {
        match self {
            Self::Approved { .. } => ApprovalStatus::Approved,
            Self::Rejected { .. } => ApprovalStatus::Rejected,
            Self::Expired { .. } => ApprovalStatus::Expired,
        }
    }

    fn actor(&self) -> Option<&str> {
        match self {
            Self::Approved { actor, .. } | Self::Rejected { actor, .. } => Some(actor.as_str()),
            Self::Expired { .. } => None,
        }
    }

    fn actor_role(&self) -> Option<&str> {
        match self {
            Self::Approved { actor_role, .. } | Self::Rejected { actor_role, .. } => {
                actor_role.as_deref()
            }
            Self::Expired { .. } => None,
        }
    }

    fn reason(&self) -> &str {
        match self {
            Self::Approved { reason, .. }
            | Self::Rejected { reason, .. }
            | Self::Expired { reason, .. } => reason,
        }
    }

    fn decided_at(&self) -> DateTime<Utc> {
        match self {
            Self::Approved { decided_at, .. }
            | Self::Rejected { decided_at, .. }
            | Self::Expired { decided_at, .. } => *decided_at,
        }
    }
}

impl ApprovalRequest {
    pub fn local_runtime_approval_id(&self) -> crate::local_runtime::ApprovalId {
        crate::local_runtime::ApprovalId::from_uuid(Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            format!("vac:workflow-approval:{}", self.id).as_bytes(),
        ))
    }

    pub fn local_runtime_task_id(&self) -> crate::local_runtime::TaskId {
        crate::local_runtime::TaskId::from_uuid(Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            format!("vac:workflow:{}", self.workflow_id).as_bytes(),
        ))
    }

    pub fn to_local_runtime_approval_request(&self) -> crate::local_runtime::ApprovalRequest {
        crate::local_runtime::ApprovalRequest {
            id: self.local_runtime_approval_id(),
            task_id: self.local_runtime_task_id(),
            action: local_runtime_action(self.action),
            risk: local_runtime_risk(self.risk),
            reason: local_runtime_reason(self),
            resources: vec![local_runtime_resource(self)],
            preview: crate::local_runtime::ApprovalPreview::Text(local_runtime_preview(self)),
            validation_after: Vec::new(),
        }
    }

    pub fn to_local_runtime_event(&self) -> crate::local_runtime::RuntimeEvent {
        crate::local_runtime::RuntimeEvent::from(self.to_local_runtime_approval_request())
    }
}

fn local_runtime_action(action: ApprovalAction) -> crate::local_runtime::ApprovalAction {
    match action {
        ApprovalAction::FilesystemWrite | ApprovalAction::FilesystemDelete => {
            crate::local_runtime::ApprovalAction::WriteFiles
        }
        ApprovalAction::ProcessExecute => crate::local_runtime::ApprovalAction::ExecuteProcess,
        ApprovalAction::NetworkAccess => crate::local_runtime::ApprovalAction::NetworkAccess,
        ApprovalAction::ToolCall => crate::local_runtime::ApprovalAction::ConnectorCall,
        ApprovalAction::SessionWrite | ApprovalAction::Other => {
            crate::local_runtime::ApprovalAction::Other
        }
    }
}

fn local_runtime_risk(risk: ApprovalRisk) -> crate::local_runtime::RiskLevel {
    match risk {
        ApprovalRisk::SafeRead => crate::local_runtime::RiskLevel::ReadOnly,
        ApprovalRisk::SafeWrite => crate::local_runtime::RiskLevel::SafeEdit,
        ApprovalRisk::Execute => crate::local_runtime::RiskLevel::Execute,
        ApprovalRisk::Network => crate::local_runtime::RiskLevel::Network,
        ApprovalRisk::Destructive => crate::local_runtime::RiskLevel::Destructive,
        ApprovalRisk::Unknown => crate::local_runtime::RiskLevel::Unknown,
    }
}

fn local_runtime_reason(request: &ApprovalRequest) -> String {
    if request.reasons.is_empty() {
        format!(
            "Workflow `{}` step `{}` requires approval for `{}`.",
            request.workflow_id, request.step_id, request.capability_id
        )
    } else {
        request.reasons.join("; ")
    }
}

fn local_runtime_resource(request: &ApprovalRequest) -> crate::local_runtime::ApprovalResource {
    match request.action {
        ApprovalAction::ProcessExecute => crate::local_runtime::ApprovalResource::Command(format!(
            "workflow={} step={} capability={}",
            request.workflow_id, request.step_id, request.capability_id
        )),
        ApprovalAction::NetworkAccess => crate::local_runtime::ApprovalResource::Network(format!(
            "workflow={} step={} capability={}",
            request.workflow_id, request.step_id, request.capability_id
        )),
        ApprovalAction::ToolCall => crate::local_runtime::ApprovalResource::Connector(format!(
            "workflow={} step={} capability={}",
            request.workflow_id, request.step_id, request.capability_id
        )),
        _ => crate::local_runtime::ApprovalResource::Other(format!(
            "workflow={} step={} capability={} action={} risk={}",
            request.workflow_id,
            request.step_id,
            request.capability_id,
            request.action,
            request.risk
        )),
    }
}

fn local_runtime_preview(request: &ApprovalRequest) -> String {
    let mut lines = vec![
        format!("workflow: {}", request.workflow_id),
        format!("step: {}", request.step_id),
        format!("capability: {}", request.capability_id),
        format!("action: {}", request.action),
        format!("risk: {}", request.risk),
        format!("status: {}", request.status),
    ];
    if let Some(required_role) = &request.required_role {
        lines.push(format!("required_role: {required_role}"));
    }
    if !request.reasons.is_empty() {
        lines.push(format!("reasons: {}", request.reasons.join("; ")));
    }
    lines.join("\n")
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ApprovalStoreError {
    #[error("approval request `{0}` not found")]
    NotFound(ApprovalRequestId),
    #[error("approval request `{id}` already resolved with status `{status}`")]
    AlreadyResolved {
        id: ApprovalRequestId,
        status: ApprovalStatus,
    },
    #[error("approval request requires role `{required_role}` but decision did not carry an actor role")]
    RoleRequired { required_role: String },
    #[error("approval request requires role `{required_role}` but actor role `{actual_role}` was provided")]
    RoleMismatch {
        required_role: String,
        actual_role: String,
    },
    #[error("failed to load approval store `{path}`: {message}")]
    Load { path: String, message: String },
    #[error("failed to persist approval store `{path}`: {message}")]
    Persist { path: String, message: String },
}

pub trait ApprovalStore {
    fn request(&mut self, request: ApprovalRequest) -> ApprovalRequestId;

    fn resolve(
        &mut self,
        id: &ApprovalRequestId,
        decision: ApprovalDecision,
    ) -> Result<ApprovalRequest, ApprovalStoreError>;

    fn get(&self, id: &ApprovalRequestId) -> Option<&ApprovalRequest>;
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct InMemoryApprovalStore {
    pub(crate) requests: BTreeMap<ApprovalRequestId, ApprovalRequest>,
}

impl InMemoryApprovalStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn from_requests(requests: BTreeMap<ApprovalRequestId, ApprovalRequest>) -> Self {
        Self { requests }
    }

    pub(crate) fn requests(&self) -> &BTreeMap<ApprovalRequestId, ApprovalRequest> {
        &self.requests
    }

    pub fn pending_count(&self) -> usize {
        self.requests
            .values()
            .filter(|request| request.is_pending())
            .count()
    }

    pub fn expire_due(
        &mut self,
        now: DateTime<Utc>,
        reason: impl Into<String>,
    ) -> Vec<ApprovalRequest> {
        let reason = reason.into();
        // `resolve` below needs `&mut self`, so collect ids first to end the `&self` borrow.
        #[allow(clippy::needless_collect)]
        let expired_ids = self
            .requests
            .values()
            .filter(|request| request.is_expired_at(now))
            .map(|request| request.id.clone())
            .collect::<Vec<_>>();

        expired_ids
            .into_iter()
            .filter_map(|id| {
                self.resolve(&id, ApprovalDecision::expired(reason.clone(), now))
                    .ok()
            })
            .collect()
    }
}

fn validate_actor_role(
    request: &ApprovalRequest,
    decision: &ApprovalDecision,
) -> Result<(), ApprovalStoreError> {
    let Some(required_role) = &request.required_role else {
        return Ok(());
    };
    let Some(actor_role) = decision.actor_role() else {
        if matches!(decision, ApprovalDecision::Expired { .. }) {
            return Ok(());
        }
        return Err(ApprovalStoreError::RoleRequired {
            required_role: required_role.clone(),
        });
    };
    if actor_role == required_role {
        Ok(())
    } else {
        Err(ApprovalStoreError::RoleMismatch {
            required_role: required_role.clone(),
            actual_role: actor_role.to_string(),
        })
    }
}

impl ApprovalStore for InMemoryApprovalStore {
    fn request(&mut self, mut request: ApprovalRequest) -> ApprovalRequestId {
        request.status = ApprovalStatus::Pending;
        request.decided_by = None;
        request.decided_at = None;
        request.decision_reason = None;
        let id = request.id.clone();
        self.requests.insert(id.clone(), request);
        id
    }

    fn resolve(
        &mut self,
        id: &ApprovalRequestId,
        decision: ApprovalDecision,
    ) -> Result<ApprovalRequest, ApprovalStoreError> {
        let request = self
            .requests
            .get_mut(id)
            .ok_or_else(|| ApprovalStoreError::NotFound(id.clone()))?;
        if !request.is_pending() {
            return Err(ApprovalStoreError::AlreadyResolved {
                id: id.clone(),
                status: request.status,
            });
        }

        validate_actor_role(request, &decision)?;
        request.status = decision.status();
        request.decided_by = decision.actor().map(str::to_string);
        request.decided_at = Some(decision.decided_at());
        request.decision_reason = Some(decision.reason().to_string());
        Ok(request.clone())
    }

    fn get(&self, id: &ApprovalRequestId) -> Option<&ApprovalRequest> {
        self.requests.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(seconds, 0).expect("valid fixture timestamp")
    }

    fn request() -> ApprovalRequest {
        ApprovalRequest::pending(
            ApprovalRequestId::for_workflow_step("maintenance.build-check", "validate"),
            "maintenance.build-check",
            "validate",
            "vac.build",
            ApprovalAction::ProcessExecute,
            ApprovalRisk::Execute,
            vec!["policy requires approval for process_execute".to_string()],
            ts(1),
            Some(ts(61)),
        )
    }

    #[test]
    fn workflow_approval_request_id_includes_run_and_attempt_for_parallel_stability() {
        let id = ApprovalRequestId::for_workflow_step_attempt(
            "maintenance.build-check",
            "run-42",
            "attempt-7",
            "validate",
        );

        assert_eq!(
            id.as_str(),
            "approval:maintenance.build-check:run:run-42:attempt:attempt-7:step:validate"
        );
    }

    #[test]
    fn workflow_approval_lifecycle_requests_start_pending() {
        let mut store = InMemoryApprovalStore::new();

        let id = store.request(request());
        let stored = store.get(&id).expect("stored request");

        assert_eq!(id.as_str(), "approval:maintenance.build-check:validate");
        assert_eq!(store.pending_count(), 1);
        assert_eq!(stored.workflow_id, "maintenance.build-check");
        assert_eq!(stored.step_id, "validate");
        assert_eq!(stored.capability_id, "vac.build");
        assert_eq!(stored.action, ApprovalAction::ProcessExecute);
        assert_eq!(stored.risk, ApprovalRisk::Execute);
        assert_eq!(stored.status, ApprovalStatus::Pending);
        assert_eq!(stored.decided_by, None);
        assert_eq!(stored.decided_at, None);
        assert_eq!(stored.decision_reason, None);
    }

    #[test]
    fn workflow_approval_lifecycle_exports_local_runtime_request_contract() {
        let request = request();

        let local = request.to_local_runtime_approval_request();
        let second = request.to_local_runtime_approval_request();

        assert_eq!(local.id, second.id);
        assert_eq!(local.task_id, second.task_id);
        assert_eq!(
            local.action,
            crate::local_runtime::ApprovalAction::ExecuteProcess
        );
        assert_eq!(local.risk, crate::local_runtime::RiskLevel::Execute);
        assert_eq!(local.reason, "policy requires approval for process_execute");
        assert_eq!(local.validation_after, Vec::<String>::new());
        assert!(matches!(
            local.resources.first(),
            Some(crate::local_runtime::ApprovalResource::Command(resource))
                if resource.contains("workflow=maintenance.build-check")
                    && resource.contains("step=validate")
                    && resource.contains("capability=vac.build")
        ));
        assert!(matches!(
            &local.preview,
            crate::local_runtime::ApprovalPreview::Text(preview)
                if preview.contains("workflow: maintenance.build-check")
                    && preview.contains("step: validate")
                    && preview.contains("action: process_execute")
                    && preview.contains("risk: execute")
        ));
        assert!(matches!(
            request.to_local_runtime_event(),
            crate::local_runtime::RuntimeEvent::ApprovalRequested(_)
        ));
    }

    #[test]
    fn workflow_approval_requires_matching_actor_role() {
        let mut store = InMemoryApprovalStore::new();
        let request = request().with_required_role("security_reviewer");
        let id = store.request(request);

        let err = store
            .resolve(
                &id,
                ApprovalDecision::approved("operator", "reviewed", ts(10)),
            )
            .expect_err("approval without role should fail closed");
        assert_eq!(
            err,
            ApprovalStoreError::RoleRequired {
                required_role: "security_reviewer".to_string(),
            }
        );

        let err = store
            .resolve(
                &id,
                ApprovalDecision::approved_with_role(
                    "operator",
                    "release_manager",
                    "reviewed",
                    ts(11),
                ),
            )
            .expect_err("wrong role should fail closed");
        assert_eq!(
            err,
            ApprovalStoreError::RoleMismatch {
                required_role: "security_reviewer".to_string(),
                actual_role: "release_manager".to_string(),
            }
        );

        let resolved = store
            .resolve(
                &id,
                ApprovalDecision::approved_with_role(
                    "operator",
                    "security_reviewer",
                    "reviewed",
                    ts(12),
                ),
            )
            .expect("matching role resolves approval");
        assert_eq!(resolved.status, ApprovalStatus::Approved);
        assert_eq!(resolved.decided_by.as_deref(), Some("operator"));
    }

    #[test]
    fn workflow_approval_lifecycle_resolves_rejection_with_audit_fields() {
        let mut store = InMemoryApprovalStore::new();
        let id = store.request(request());

        let resolved = store
            .resolve(
                &id,
                ApprovalDecision::rejected("operator", "unsafe command", ts(10)),
            )
            .expect("request resolved");

        assert_eq!(resolved.status, ApprovalStatus::Rejected);
        assert_eq!(resolved.decided_by.as_deref(), Some("operator"));
        assert_eq!(resolved.decided_at, Some(ts(10)));
        assert_eq!(resolved.decision_reason.as_deref(), Some("unsafe command"));
        assert_eq!(store.pending_count(), 0);
        assert_eq!(store.get(&id), Some(&resolved));
    }

    #[test]
    fn workflow_approval_lifecycle_expires_due_pending_requests() {
        let mut store = InMemoryApprovalStore::new();
        let id = store.request(request());

        let early = store.expire_due(ts(60), "ttl elapsed");
        assert!(early.is_empty());
        assert_eq!(store.pending_count(), 1);

        let expired = store.expire_due(ts(61), "ttl elapsed");
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].id, id);
        assert_eq!(expired[0].status, ApprovalStatus::Expired);
        assert_eq!(expired[0].decided_at, Some(ts(61)));
        assert_eq!(expired[0].decision_reason.as_deref(), Some("ttl elapsed"));
        assert_eq!(store.pending_count(), 0);
    }

    #[test]
    fn workflow_approval_lifecycle_rejects_second_resolution() {
        let mut store = InMemoryApprovalStore::new();
        let id = store.request(request());
        store
            .resolve(
                &id,
                ApprovalDecision::approved("operator", "reviewed", ts(10)),
            )
            .expect("first resolution");

        let error = store
            .resolve(
                &id,
                ApprovalDecision::rejected("operator", "second decision", ts(11)),
            )
            .expect_err("second resolution should fail");

        assert_eq!(
            error,
            ApprovalStoreError::AlreadyResolved {
                id,
                status: ApprovalStatus::Approved,
            }
        );
    }
}
