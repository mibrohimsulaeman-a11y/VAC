//! Local Runtime Contract for VAC's local product path.
//!
//! These types are the product-level session/task/event model used by root VAC
//! code. They deliberately avoid app-server or thread terminology so the local
//! runtime can stand on its own.

mod macros;
pub(crate) use macros::impl_display_as_str;

mod approval;
mod command;
mod error;
mod event;
mod ids;
mod projection;
mod session;
mod task;
mod transition;
mod validation;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_serde;

pub use approval::ApprovalAction;
pub use approval::ApprovalDecision;
pub use approval::ApprovalPreview;
pub use approval::ApprovalRequest;
pub use approval::ApprovalResource;
pub use approval::PreviewKind;
pub use approval::RiskLevel;
pub use command::ReviewTarget;
pub use command::RuntimeCommand;
pub use command::StartReview;
pub use command::StartTask;
pub use command::WorkflowInput;
pub use error::RuntimeError;
pub use error::RuntimeErrorCode;
pub use error::RuntimeErrorOwner;
pub use error::TaskFailure;
pub use event::ApprovalResolved;
pub use event::AssistantDelta;
pub use event::EnteredReviewMode;
pub use event::ExitedReviewMode;
pub use event::RuntimeEvent;
pub use event::SessionEnded;
pub use event::SessionStarted;
pub use event::TaskCancelled;
pub use event::TaskCompleted;
pub use event::TaskFailed;
pub use event::TaskStarted;
pub use event::ToolCallFinished;
pub use event::ToolCallStarted;
pub use ids::ApprovalId;
pub use ids::SessionId;
pub use ids::TaskId;
pub use ids::ToolCallId;
pub use ids::WorkflowId;
pub use projection::BridgeOutput;
pub use projection::LocalRuntimeBridge;
pub use session::AutonomyMode;
pub use session::RuntimeEntrypoint;
pub use session::RuntimeSession;
pub use session::RuntimeSessionStatus;
pub use task::RuntimeTask;
pub use task::RuntimeTaskKind;
pub use task::RuntimeTaskStatus;
pub use transition::InvalidTransition;
pub use validation::ValidationFinished;
pub use validation::ValidationStarted;
pub use validation::ValidationStatus;
