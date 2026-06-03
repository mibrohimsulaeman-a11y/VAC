use std::fmt::Display;
use std::fmt::Formatter;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use super::ApprovalId;
use super::SessionId;
use super::TaskId;
use super::WorkflowId;
use super::session::AutonomyMode;
use super::session::RuntimeEntrypoint;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartTask {
    pub prompt: String,
    pub autonomy_mode: AutonomyMode,
    pub entrypoint: RuntimeEntrypoint,
    pub cwd: PathBuf,
}

impl StartTask {
    pub fn new(
        prompt: impl Into<String>,
        autonomy_mode: AutonomyMode,
        entrypoint: RuntimeEntrypoint,
        cwd: impl Into<PathBuf>,
    ) -> Self {
        Self {
            prompt: prompt.into(),
            autonomy_mode,
            entrypoint,
            cwd: cwd.into(),
        }
    }
}

pub use vac_protocol::protocol::ReviewTarget;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartReview {
    #[serde(with = "review_target_serde")]
    pub target: ReviewTarget,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub prompt: Option<String>,
    pub autonomy_mode: AutonomyMode,
    pub entrypoint: RuntimeEntrypoint,
    pub cwd: PathBuf,
}

impl StartReview {
    pub fn new(
        target: ReviewTarget,
        prompt: Option<String>,
        autonomy_mode: AutonomyMode,
        entrypoint: RuntimeEntrypoint,
        cwd: impl Into<PathBuf>,
    ) -> Self {
        Self {
            target,
            prompt,
            autonomy_mode,
            entrypoint,
            cwd: cwd.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuntimeCommand {
    StartTask(StartTask),
    StartReview(StartReview),
    CancelTask {
        task_id: TaskId,
    },
    Approve {
        approval_id: ApprovalId,
    },
    Reject {
        approval_id: ApprovalId,
    },
    ResumeSession {
        session_id: SessionId,
    },
    RunWorkflow {
        workflow_id: WorkflowId,
        input: WorkflowInput,
    },
}

pub type WorkflowInput = String;

impl RuntimeCommand {
    pub fn start_task(
        prompt: impl Into<String>,
        autonomy_mode: AutonomyMode,
        entrypoint: RuntimeEntrypoint,
        cwd: impl Into<PathBuf>,
    ) -> Self {
        Self::StartTask(StartTask::new(prompt, autonomy_mode, entrypoint, cwd))
    }

    pub fn start_review(
        target: ReviewTarget,
        prompt: Option<String>,
        autonomy_mode: AutonomyMode,
        entrypoint: RuntimeEntrypoint,
        cwd: impl Into<PathBuf>,
    ) -> Self {
        Self::StartReview(StartReview::new(
            target,
            prompt,
            autonomy_mode,
            entrypoint,
            cwd,
        ))
    }
}

impl Display for StartTask {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "StartTask(prompt={:?}, autonomy_mode={}, entrypoint={}, cwd={})",
            self.prompt,
            self.autonomy_mode,
            self.entrypoint,
            self.cwd.display()
        )
    }
}

pub(super) mod review_target_serde {
    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serializer;
    use serde::ser::SerializeMap;
    use vac_protocol::protocol::ReviewTarget;

    #[derive(Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum LegacyReviewTarget {
        Uncommitted,
        Base { branch: String },
        Commit { sha: String, title: Option<String> },
        Custom { instructions: String },
    }

    pub(crate) fn serialize<S>(target: &ReviewTarget, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match target {
            ReviewTarget::UncommittedChanges => serializer.serialize_str("uncommitted"),
            ReviewTarget::BaseBranch { branch } => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("base", &serde_json::json!({ "branch": branch }))?;
                map.end()
            }
            ReviewTarget::Commit { sha, title } => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("commit", &serde_json::json!({ "sha": sha, "title": title }))?;
                map.end()
            }
            ReviewTarget::Custom { instructions } => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(
                    "custom",
                    &serde_json::json!({ "instructions": instructions }),
                )?;
                map.end()
            }
        }
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<ReviewTarget, D::Error>
    where
        D: Deserializer<'de>,
    {
        let legacy = LegacyReviewTarget::deserialize(deserializer)?;
        Ok(match legacy {
            LegacyReviewTarget::Uncommitted => ReviewTarget::UncommittedChanges,
            LegacyReviewTarget::Base { branch } => ReviewTarget::BaseBranch { branch },
            LegacyReviewTarget::Commit { sha, title } => ReviewTarget::Commit { sha, title },
            LegacyReviewTarget::Custom { instructions } => ReviewTarget::Custom { instructions },
        })
    }
}

fn review_target_display(target: &ReviewTarget) -> String {
    match target {
        ReviewTarget::UncommittedChanges => "uncommitted".to_string(),
        ReviewTarget::BaseBranch { branch } => format!("base({branch})"),
        ReviewTarget::Commit {
            sha,
            title: Some(t),
        } => format!("commit({sha}, {t:?})"),
        ReviewTarget::Commit { sha, title: None } => format!("commit({sha})"),
        ReviewTarget::Custom { instructions } => format!("custom({instructions:?})"),
    }
}

impl Display for StartReview {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "StartReview(target={}, prompt={:?}, autonomy_mode={}, entrypoint={}, cwd={})",
            review_target_display(&self.target),
            self.prompt,
            self.autonomy_mode,
            self.entrypoint,
            self.cwd.display()
        )
    }
}
