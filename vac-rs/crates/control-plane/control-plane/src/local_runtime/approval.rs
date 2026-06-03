use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use super::ApprovalId;
use super::TaskId;
use super::impl_display_as_str;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalAction {
    WriteFiles,
    ExecuteProcess,
    NetworkAccess,
    ConnectorCall,
    Restore,
    Other,
}

impl_display_as_str!(ApprovalAction {
    WriteFiles => "write_files",
    ExecuteProcess => "execute_process",
    NetworkAccess => "network_access",
    ConnectorCall => "connector_call",
    Restore => "restore",
    Other => "other",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    ReadOnly,
    SafeEdit,
    BroadEdit,
    Destructive,
    Execute,
    Network,
    Credential,
    Unknown,
}

impl_display_as_str!(RiskLevel {
    ReadOnly => "read_only",
    SafeEdit => "safe_edit",
    BroadEdit => "broad_edit",
    Destructive => "destructive",
    Execute => "execute",
    Network => "network",
    Credential => "credential",
    Unknown => "unknown",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewKind {
    None,
    Text,
    Diff,
    Command,
    FileList,
}

impl_display_as_str!(PreviewKind {
    None => "none",
    Text => "text",
    Diff => "diff",
    Command => "command",
    FileList => "file_list",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalPreview {
    None,
    Text(String),
    Diff(String),
    Command(String),
    FileList(Vec<PathBuf>),
}

impl ApprovalPreview {
    pub fn kind(&self) -> PreviewKind {
        match self {
            Self::None => PreviewKind::None,
            Self::Text(_) => PreviewKind::Text,
            Self::Diff(_) => PreviewKind::Diff,
            Self::Command(_) => PreviewKind::Command,
            Self::FileList(_) => PreviewKind::FileList,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approved,
    Rejected,
    Cancelled,
    Timeout,
}

impl_display_as_str!(ApprovalDecision {
    Approved => "approved",
    Rejected => "rejected",
    Cancelled => "cancelled",
    Timeout => "timeout",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalResource {
    File(PathBuf),
    Command(String),
    Network(String),
    Connector(String),
    Config(String),
    Other(String),
}

impl std::fmt::Display for ApprovalResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File(p) => write!(f, "file:{}", p.display()),
            Self::Command(s) => write!(f, "command:{s}"),
            Self::Network(s) => write!(f, "network:{s}"),
            Self::Connector(s) => write!(f, "connector:{s}"),
            Self::Config(s) => write!(f, "config:{s}"),
            Self::Other(s) => write!(f, "other:{s}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: ApprovalId,
    pub task_id: TaskId,
    pub action: ApprovalAction,
    pub risk: RiskLevel,
    pub reason: String,
    pub resources: Vec<ApprovalResource>,
    pub preview: ApprovalPreview,
    pub validation_after: Vec<String>,
}

impl ApprovalRequest {
    pub fn safe_edit(
        id: ApprovalId,
        task_id: TaskId,
        reason: impl Into<String>,
        resources: Vec<ApprovalResource>,
        preview: ApprovalPreview,
        validation_after: Vec<String>,
    ) -> Self {
        Self {
            id,
            task_id,
            action: ApprovalAction::WriteFiles,
            risk: RiskLevel::SafeEdit,
            reason: reason.into(),
            resources,
            preview,
            validation_after,
        }
    }
}
