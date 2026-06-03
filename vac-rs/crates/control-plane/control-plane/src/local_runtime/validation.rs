use serde::Deserialize;
use serde::Serialize;

use super::TaskId;
use super::impl_display_as_str;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Passed,
    Failed,
    Skipped,
    Cancelled,
}

impl_display_as_str!(ValidationStatus {
    Passed => "passed",
    Failed => "failed",
    Skipped => "skipped",
    Cancelled => "cancelled",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationStarted {
    pub task_id: TaskId,
    pub command_display: String,
}

impl ValidationStarted {
    pub fn new(task_id: TaskId, command_display: impl Into<String>) -> Self {
        Self {
            task_id,
            command_display: command_display.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationFinished {
    pub task_id: TaskId,
    pub command_display: String,
    pub status: ValidationStatus,
    pub summary: Option<String>,
}

impl ValidationFinished {
    pub fn new(
        task_id: TaskId,
        command_display: impl Into<String>,
        status: ValidationStatus,
        summary: impl Into<Option<String>>,
    ) -> Self {
        Self {
            task_id,
            command_display: command_display.into(),
            status,
            summary: summary.into(),
        }
    }
}
