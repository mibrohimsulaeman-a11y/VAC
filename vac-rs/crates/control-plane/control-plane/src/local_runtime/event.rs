use serde::Deserialize;
use serde::Serialize;

use super::ApprovalDecision;
use super::ApprovalId;
use super::ApprovalRequest;
use super::ReviewTarget;
use super::RuntimeError;
use super::RuntimeSession;
use super::RuntimeSessionStatus;
use super::RuntimeTask;
use super::SessionId;
use super::TaskFailure;
use super::TaskId;
use super::ToolCallId;
use super::ValidationFinished;
use super::ValidationStarted;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStarted {
    pub session: RuntimeSession,
}

impl SessionStarted {
    pub fn new(session: RuntimeSession) -> Self {
        Self { session }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEnded {
    pub session_id: SessionId,
    pub final_status: RuntimeSessionStatus,
    pub reason: Option<String>,
}

impl SessionEnded {
    pub fn new(
        session_id: SessionId,
        final_status: RuntimeSessionStatus,
        reason: impl Into<Option<String>>,
    ) -> Self {
        Self {
            session_id,
            final_status,
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskStarted {
    pub task: RuntimeTask,
}

impl TaskStarted {
    pub fn new(task: RuntimeTask) -> Self {
        Self { task }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantDelta {
    pub task_id: TaskId,
    pub text: String,
}

impl AssistantDelta {
    pub fn new(task_id: TaskId, text: impl Into<String>) -> Self {
        Self {
            task_id,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallStarted {
    pub task_id: TaskId,
    pub tool_call_id: ToolCallId,
    pub tool_name: String,
    pub input_preview: Option<String>,
}

impl ToolCallStarted {
    pub fn new(
        task_id: TaskId,
        tool_call_id: ToolCallId,
        tool_name: impl Into<String>,
        input_preview: impl Into<Option<String>>,
    ) -> Self {
        Self {
            task_id,
            tool_call_id,
            tool_name: tool_name.into(),
            input_preview: input_preview.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallFinished {
    pub task_id: TaskId,
    pub tool_call_id: ToolCallId,
    pub tool_name: String,
    pub output_preview: Option<String>,
    pub success: bool,
}

impl ToolCallFinished {
    pub fn new(
        task_id: TaskId,
        tool_call_id: ToolCallId,
        tool_name: impl Into<String>,
        output_preview: impl Into<Option<String>>,
        success: bool,
    ) -> Self {
        Self {
            task_id,
            tool_call_id,
            tool_name: tool_name.into(),
            output_preview: output_preview.into(),
            success,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalResolved {
    pub approval_id: ApprovalId,
    pub decision: ApprovalDecision,
    pub reason: Option<String>,
}

impl ApprovalResolved {
    pub fn new(
        approval_id: ApprovalId,
        decision: ApprovalDecision,
        reason: impl Into<Option<String>>,
    ) -> Self {
        Self {
            approval_id,
            decision,
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCompleted {
    pub task_id: TaskId,
    pub summary: Option<String>,
    pub evidence: Vec<String>,
}

impl TaskCompleted {
    pub fn new(task_id: TaskId, summary: impl Into<Option<String>>, evidence: Vec<String>) -> Self {
        Self {
            task_id,
            summary: summary.into(),
            evidence,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskFailed {
    pub task_id: TaskId,
    pub error: TaskFailure,
}

impl TaskFailed {
    pub fn new(task_id: TaskId, error: RuntimeError) -> Self {
        Self { task_id, error }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCancelled {
    pub task_id: TaskId,
    pub reason: Option<String>,
}

impl TaskCancelled {
    pub fn new(task_id: TaskId, reason: impl Into<Option<String>>) -> Self {
        Self {
            task_id,
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnteredReviewMode {
    #[serde(with = "super::command::review_target_serde")]
    pub target: ReviewTarget,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub user_facing_hint: Option<String>,
}

impl EnteredReviewMode {
    pub fn new(target: ReviewTarget, user_facing_hint: impl Into<Option<String>>) -> Self {
        Self {
            target,
            user_facing_hint: user_facing_hint.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExitedReviewMode {
    /// Optional human-readable summary of the review result.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub summary: Option<String>,
}

impl ExitedReviewMode {
    pub fn new(summary: impl Into<Option<String>>) -> Self {
        Self {
            summary: summary.into(),
        }
    }
}

/// Append-only product-level event surface.
///
/// `ApprovalRequested` is boxed because [`ApprovalRequest`] is significantly
/// larger than the other variants (it holds resources, a preview, and a
/// validation list). Boxing keeps `RuntimeEvent` itself compact in vectors
/// and channels and removes the `clippy::large_enum_variant` warning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuntimeEvent {
    SessionStarted(SessionStarted),
    SessionEnded(SessionEnded),
    TaskStarted(TaskStarted),
    AssistantDelta(AssistantDelta),
    ToolCallStarted(ToolCallStarted),
    ToolCallFinished(ToolCallFinished),
    ApprovalRequested(Box<ApprovalRequest>),
    ApprovalResolved(ApprovalResolved),
    ValidationStarted(ValidationStarted),
    ValidationFinished(ValidationFinished),
    TaskCompleted(TaskCompleted),
    TaskFailed(TaskFailed),
    TaskCancelled(TaskCancelled),
    EnteredReviewMode(EnteredReviewMode),
    ExitedReviewMode(ExitedReviewMode),
}

impl From<SessionStarted> for RuntimeEvent {
    fn from(value: SessionStarted) -> Self {
        Self::SessionStarted(value)
    }
}

impl From<SessionEnded> for RuntimeEvent {
    fn from(value: SessionEnded) -> Self {
        Self::SessionEnded(value)
    }
}

impl From<TaskStarted> for RuntimeEvent {
    fn from(value: TaskStarted) -> Self {
        Self::TaskStarted(value)
    }
}

impl From<AssistantDelta> for RuntimeEvent {
    fn from(value: AssistantDelta) -> Self {
        Self::AssistantDelta(value)
    }
}

impl From<ToolCallStarted> for RuntimeEvent {
    fn from(value: ToolCallStarted) -> Self {
        Self::ToolCallStarted(value)
    }
}

impl From<ToolCallFinished> for RuntimeEvent {
    fn from(value: ToolCallFinished) -> Self {
        Self::ToolCallFinished(value)
    }
}

impl From<ApprovalRequest> for RuntimeEvent {
    fn from(value: ApprovalRequest) -> Self {
        Self::ApprovalRequested(Box::new(value))
    }
}

impl From<Box<ApprovalRequest>> for RuntimeEvent {
    fn from(value: Box<ApprovalRequest>) -> Self {
        Self::ApprovalRequested(value)
    }
}

impl From<ApprovalResolved> for RuntimeEvent {
    fn from(value: ApprovalResolved) -> Self {
        Self::ApprovalResolved(value)
    }
}

impl From<ValidationStarted> for RuntimeEvent {
    fn from(value: ValidationStarted) -> Self {
        Self::ValidationStarted(value)
    }
}

impl From<ValidationFinished> for RuntimeEvent {
    fn from(value: ValidationFinished) -> Self {
        Self::ValidationFinished(value)
    }
}

impl From<TaskCompleted> for RuntimeEvent {
    fn from(value: TaskCompleted) -> Self {
        Self::TaskCompleted(value)
    }
}

impl From<TaskFailed> for RuntimeEvent {
    fn from(value: TaskFailed) -> Self {
        Self::TaskFailed(value)
    }
}

impl From<TaskCancelled> for RuntimeEvent {
    fn from(value: TaskCancelled) -> Self {
        Self::TaskCancelled(value)
    }
}

impl From<EnteredReviewMode> for RuntimeEvent {
    fn from(value: EnteredReviewMode) -> Self {
        Self::EnteredReviewMode(value)
    }
}

impl From<ExitedReviewMode> for RuntimeEvent {
    fn from(value: ExitedReviewMode) -> Self {
        Self::ExitedReviewMode(value)
    }
}
