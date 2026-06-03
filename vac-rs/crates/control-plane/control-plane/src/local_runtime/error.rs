use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeErrorOwner {
    User,
    Provider,
    Sandbox,
    Policy,
    Bridge,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", content = "details", rename_all = "snake_case")]
pub enum RuntimeErrorCode {
    BudgetLimited,
    ApprovalRequired,
    ValidationFailed,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeError {
    #[serde(flatten)]
    pub code: RuntimeErrorCode,
    pub message: String,
    pub recovery_hint: Option<String>,
    pub retry_safe: bool,
    pub owner: RuntimeErrorOwner,
}

impl RuntimeError {
    pub fn new(
        code: RuntimeErrorCode,
        message: impl Into<String>,
        recovery_hint: impl Into<Option<String>>,
        retry_safe: bool,
        owner: RuntimeErrorOwner,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            recovery_hint: recovery_hint.into(),
            retry_safe,
            owner,
        }
    }
}

pub type TaskFailure = RuntimeError;
