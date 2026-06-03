use std::path::PathBuf;

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

use super::InvalidTransition;
use super::SessionId;
use super::impl_display_as_str;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeEntrypoint {
    Tui,
    Exec,
    Workflow,
}

impl_display_as_str!(RuntimeEntrypoint {
    Tui => "tui",
    Exec => "exec",
    Workflow => "workflow",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyMode {
    Suggest,
    Assist,
    Autopilot,
}

impl_display_as_str!(AutonomyMode {
    Suggest => "suggest",
    Assist => "assist",
    Autopilot => "autopilot",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSessionStatus {
    Active,
    WaitingApproval,
    Completed,
    Failed,
    Cancelled,
}

impl_display_as_str!(RuntimeSessionStatus {
    Active => "active",
    WaitingApproval => "waiting_approval",
    Completed => "completed",
    Failed => "failed",
    Cancelled => "cancelled",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSession {
    pub id: SessionId,
    pub created_at: DateTime<Utc>,
    pub cwd: PathBuf,
    pub entrypoint: RuntimeEntrypoint,
    pub autonomy_mode: AutonomyMode,
    pub status: RuntimeSessionStatus,
}

impl RuntimeSession {
    pub fn new(cwd: PathBuf, entrypoint: RuntimeEntrypoint, autonomy_mode: AutonomyMode) -> Self {
        Self::with_id(SessionId::new(), Utc::now(), cwd, entrypoint, autonomy_mode)
    }

    pub fn with_id(
        id: SessionId,
        created_at: DateTime<Utc>,
        cwd: PathBuf,
        entrypoint: RuntimeEntrypoint,
        autonomy_mode: AutonomyMode,
    ) -> Self {
        Self {
            id,
            created_at,
            cwd,
            entrypoint,
            autonomy_mode,
            status: RuntimeSessionStatus::Active,
        }
    }

    pub fn await_approval(&mut self) -> Result<(), InvalidTransition> {
        self.transition_to(
            RuntimeSessionStatus::WaitingApproval,
            &[RuntimeSessionStatus::Active],
        )
    }

    pub fn resume_from_approval(&mut self) -> Result<(), InvalidTransition> {
        self.transition_to(
            RuntimeSessionStatus::Active,
            &[RuntimeSessionStatus::WaitingApproval],
        )
    }

    pub fn complete(&mut self) -> Result<(), InvalidTransition> {
        self.transition_to(
            RuntimeSessionStatus::Completed,
            &[
                RuntimeSessionStatus::Active,
                RuntimeSessionStatus::WaitingApproval,
            ],
        )
    }

    pub fn fail(&mut self) -> Result<(), InvalidTransition> {
        self.transition_to(
            RuntimeSessionStatus::Failed,
            &[
                RuntimeSessionStatus::Active,
                RuntimeSessionStatus::WaitingApproval,
            ],
        )
    }

    pub fn cancel(&mut self) -> Result<(), InvalidTransition> {
        self.transition_to(
            RuntimeSessionStatus::Cancelled,
            &[
                RuntimeSessionStatus::Active,
                RuntimeSessionStatus::WaitingApproval,
            ],
        )
    }

    fn transition_to(
        &mut self,
        next: RuntimeSessionStatus,
        allowed_from: &[RuntimeSessionStatus],
    ) -> Result<(), InvalidTransition> {
        if allowed_from.contains(&self.status) {
            self.status = next;
            Ok(())
        } else {
            Err(InvalidTransition::new(
                "session",
                self.status.as_str(),
                next.as_str(),
            ))
        }
    }
}
