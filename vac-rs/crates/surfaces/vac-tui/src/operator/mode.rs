use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorMode {
    FirstLaunch,
    Idle,
    AgentWorking,
    ApprovalRequired,
    RuntimeJobs,
    Capabilities,
    Assessment,
    SpecSync,
}

impl OperatorMode {
    pub fn tab_label(self) -> &'static str {
        match self {
            Self::FirstLaunch | Self::Idle | Self::AgentWorking | Self::ApprovalRequired => "chat",
            Self::RuntimeJobs => "runtime",
            Self::Capabilities => "review",
            Self::Assessment | Self::SpecSync => "workbench",
        }
    }
}
