use crate::protocol::item_builders::build_command_execution_begin_item;
use crate::protocol::item_builders::build_command_execution_end_item;
use crate::protocol::item_builders::build_file_change_approval_request_item;
use crate::protocol::item_builders::build_file_change_begin_item;
use crate::protocol::item_builders::build_file_change_end_item;
use crate::protocol::item_builders::build_item_from_guardian_event;
use crate::protocol::v2::CollabAgentState;
use crate::protocol::v2::CollabAgentTool;
use crate::protocol::v2::CollabAgentToolCallStatus;
use crate::protocol::v2::CommandExecutionStatus;
use crate::protocol::v2::DynamicToolCallOutputContentItem;
use crate::protocol::v2::DynamicToolCallStatus;
use crate::protocol::v2::McpToolCallError;
use crate::protocol::v2::McpToolCallResult;
use crate::protocol::v2::McpToolCallStatus;
use crate::protocol::v2::ThreadItem;
use crate::protocol::v2::Turn;
use crate::protocol::v2::TurnError as V2TurnError;
use crate::protocol::v2::TurnError;
use crate::protocol::v2::TurnStatus;
use crate::protocol::v2::UserInput;
use crate::protocol::v2::WebSearchAction;
use std::collections::HashMap;
use tracing::warn;
use uuid::Uuid;
use vac_protocol::items::parse_hook_prompt_message;
use vac_protocol::models::MessagePhase;
use vac_protocol::protocol::AgentReasoningEvent;
use vac_protocol::protocol::AgentReasoningRawContentEvent;
use vac_protocol::protocol::AgentStatus;
use vac_protocol::protocol::ApplyPatchApprovalRequestEvent;
use vac_protocol::protocol::CompactedItem;
use vac_protocol::protocol::ContextCompactedEvent;
use vac_protocol::protocol::DynamicToolCallResponseEvent;
use vac_protocol::protocol::ErrorEvent;
use vac_protocol::protocol::EventMsg;
use vac_protocol::protocol::ExecCommandBeginEvent;
use vac_protocol::protocol::ExecCommandEndEvent;
use vac_protocol::protocol::GuardianAssessmentEvent;
use vac_protocol::protocol::GuardianAssessmentStatus;
use vac_protocol::protocol::ImageGenerationBeginEvent;
use vac_protocol::protocol::ImageGenerationEndEvent;
use vac_protocol::protocol::ItemCompletedEvent;
use vac_protocol::protocol::ItemStartedEvent;
use vac_protocol::protocol::McpToolCallBeginEvent;
use vac_protocol::protocol::McpToolCallEndEvent;
use vac_protocol::protocol::PatchApplyBeginEvent;
use vac_protocol::protocol::PatchApplyEndEvent;
use vac_protocol::protocol::ReviewOutputEvent;
use vac_protocol::protocol::RolloutItem;
use vac_protocol::protocol::ThreadRolledBackEvent;
use vac_protocol::protocol::TurnAbortedEvent;
use vac_protocol::protocol::TurnCompleteEvent;
use vac_protocol::protocol::TurnStartedEvent;
use vac_protocol::protocol::UserMessageEvent;
use vac_protocol::protocol::ViewImageToolCallEvent;
use vac_protocol::protocol::WebSearchBeginEvent;
use vac_protocol::protocol::WebSearchEndEvent;

#[cfg(test)]
use crate::protocol::v2::CommandAction;
#[cfg(test)]
use crate::protocol::v2::FileUpdateChange;
#[cfg(test)]
use crate::protocol::v2::PatchApplyStatus;
#[cfg(test)]
use crate::protocol::v2::PatchChangeKind;
#[cfg(test)]
use vac_protocol::protocol::ExecCommandStatus as CoreExecCommandStatus;
#[cfg(test)]
use vac_protocol::protocol::PatchApplyStatus as CorePatchApplyStatus;

/// Convert persisted [`RolloutItem`] entries into a sequence of [`Turn`] values.
///
/// When available, this uses `TurnContext.turn_id` as the canonical turn id so
/// resumed/rebuilt thread history preserves the original turn identifiers.
pub fn build_turns_from_rollout_items(items: &[RolloutItem]) -> Vec<Turn> {
    let mut builder = ThreadHistoryBuilder::new();
    for item in items {
        builder.handle_rollout_item(item);
    }
    builder.finish()
}

pub struct ThreadHistoryBuilder {
    turns: Vec<Turn>,
    current_turn: Option<PendingTurn>,
    next_item_index: i64,
    current_rollout_index: usize,
    next_rollout_index: usize,
}

impl Default for ThreadHistoryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

