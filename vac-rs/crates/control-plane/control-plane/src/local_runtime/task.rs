use serde::Deserialize;
use serde::Serialize;

use super::InvalidTransition;
use super::SessionId;
use super::TaskId;
use super::impl_display_as_str;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTaskKind {
    SemanticCoding,
    Workflow,
    Review,
    Apply,
    Maintenance,
}

impl_display_as_str!(RuntimeTaskKind {
    SemanticCoding => "semantic_coding",
    Workflow => "workflow",
    Review => "review",
    Apply => "apply",
    Maintenance => "maintenance",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTaskStatus {
    Pending,
    Running,
    WaitingApproval,
    Completed,
    Failed,
    Cancelled,
}

impl_display_as_str!(RuntimeTaskStatus {
    Pending => "pending",
    Running => "running",
    WaitingApproval => "waiting_approval",
    Completed => "completed",
    Failed => "failed",
    Cancelled => "cancelled",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeTask {
    pub id: TaskId,
    pub session_id: SessionId,
    pub kind: RuntimeTaskKind,
    pub prompt: String,
    pub status: RuntimeTaskStatus,
}

impl RuntimeTask {
    pub fn new(session_id: SessionId, kind: RuntimeTaskKind, prompt: impl Into<String>) -> Self {
        Self::with_id(TaskId::new(), session_id, kind, prompt)
    }

    pub fn with_id(
        id: TaskId,
        session_id: SessionId,
        kind: RuntimeTaskKind,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            id,
            session_id,
            kind,
            prompt: prompt.into(),
            status: RuntimeTaskStatus::Pending,
        }
    }

    pub fn start(&mut self) -> Result<(), InvalidTransition> {
        self.transition_to(RuntimeTaskStatus::Running, &[RuntimeTaskStatus::Pending])
    }

    pub fn await_approval(&mut self) -> Result<(), InvalidTransition> {
        self.transition_to(
            RuntimeTaskStatus::WaitingApproval,
            &[RuntimeTaskStatus::Running],
        )
    }

    pub fn resume_from_approval(&mut self) -> Result<(), InvalidTransition> {
        self.transition_to(
            RuntimeTaskStatus::Running,
            &[RuntimeTaskStatus::WaitingApproval],
        )
    }

    pub fn complete(&mut self) -> Result<(), InvalidTransition> {
        self.transition_to(RuntimeTaskStatus::Completed, &[RuntimeTaskStatus::Running])
    }

    pub fn fail(&mut self) -> Result<(), InvalidTransition> {
        self.transition_to(
            RuntimeTaskStatus::Failed,
            &[
                RuntimeTaskStatus::Running,
                RuntimeTaskStatus::WaitingApproval,
            ],
        )
    }

    pub fn cancel(&mut self) -> Result<(), InvalidTransition> {
        self.transition_to(
            RuntimeTaskStatus::Cancelled,
            &[
                RuntimeTaskStatus::Pending,
                RuntimeTaskStatus::Running,
                RuntimeTaskStatus::WaitingApproval,
            ],
        )
    }

    fn transition_to(
        &mut self,
        next: RuntimeTaskStatus,
        allowed_from: &[RuntimeTaskStatus],
    ) -> Result<(), InvalidTransition> {
        if allowed_from.contains(&self.status) {
            self.status = next;
            Ok(())
        } else {
            Err(InvalidTransition::new(
                "task",
                self.status.as_str(),
                next.as_str(),
            ))
        }
    }
}
