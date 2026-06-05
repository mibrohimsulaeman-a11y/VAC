// Local-tool stub for removed multi-agent collaboration UI.
//
// External-agent import/migration remains in the codebase; this module only disables the
// interactive multi-agent collaboration surface that is outside the local TUI+CLI coding scope.

use crate::history_cell::PlainHistoryCell;
use crate::session_protocol::{
    CollabAgentStatus, CollabAgentTool, CollabAgentToolCallStatus, ThreadItem,
};
use crossterm::event::KeyEvent;
use ratatui::style::Stylize;
use ratatui::text::Span;
use vac_protocol::ThreadId;
use vac_protocol::vastar_models::ReasoningEffort as ReasoningEffortConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentPickerThreadEntry {
    pub(crate) agent_nickname: Option<String>,
    pub(crate) agent_role: Option<String>,
    pub(crate) is_closed: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AgentMetadata {
    pub(crate) agent_nickname: Option<String>,
    pub(crate) agent_role: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpawnRequestSummary {
    pub(crate) model: String,
    pub(crate) reasoning_effort: ReasoningEffortConfig,
}

pub(crate) fn agent_picker_status_dot_spans(is_closed: bool) -> Vec<Span<'static>> {
    let dot = if is_closed {
        "•".dim()
    } else {
        "•".green()
    };
    vec![dot, " ".into()]
}

pub(crate) fn format_agent_picker_item_name(
    agent_nickname: Option<&str>,
    agent_role: Option<&str>,
    is_primary: bool,
) -> String {
    if is_primary {
        return "Main [default]".to_string();
    }
    match (agent_nickname, agent_role) {
        (Some(name), Some(role)) if !name.trim().is_empty() && !role.trim().is_empty() => {
            format!("{} [{}]", name.trim(), role.trim())
        }
        (Some(name), _) if !name.trim().is_empty() => name.trim().to_string(),
        (_, Some(role)) if !role.trim().is_empty() => format!("[{}]", role.trim()),
        _ => "Agent".to_string(),
    }
}

pub(crate) fn previous_agent_shortcut() -> crate::key_hint::KeyBinding {
    crate::key_hint::alt(crossterm::event::KeyCode::Left)
}
pub(crate) fn next_agent_shortcut() -> crate::key_hint::KeyBinding {
    crate::key_hint::alt(crossterm::event::KeyCode::Right)
}
pub(crate) fn previous_agent_shortcut_matches(
    _key_event: KeyEvent,
    _allow_word_motion_fallback: bool,
) -> bool {
    false
}
pub(crate) fn next_agent_shortcut_matches(
    _key_event: KeyEvent,
    _allow_word_motion_fallback: bool,
) -> bool {
    false
}

pub(crate) fn spawn_request_summary(item: &ThreadItem) -> Option<SpawnRequestSummary> {
    if let ThreadItem::CollabAgentToolCall {
        tool: CollabAgentTool::SpawnAgent,
        model: Some(model),
        reasoning_effort: Some(reasoning_effort),
        ..
    } = item
    {
        Some(SpawnRequestSummary {
            model: model.clone(),
            reasoning_effort: *reasoning_effort,
        })
    } else {
        None
    }
}

pub(crate) fn tool_call_history_cell(
    item: &ThreadItem,
    cached_spawn_request: Option<&SpawnRequestSummary>,
    mut agent_metadata: impl FnMut(ThreadId) -> AgentMetadata,
) -> Option<PlainHistoryCell> {
    if let ThreadItem::CollabAgentToolCall {
        tool,
        status,
        receiver_thread_ids,
        model,
        reasoning_effort,
        agents_states,
        ..
    } = item
    {
        match tool {
            CollabAgentTool::SpawnAgent => {
                if matches!(status, CollabAgentToolCallStatus::InProgress) {
                    Some(PlainHistoryCell::new(vec![
                        vec!["• ".dim(), "Waiting for agents...".into()].into(),
                    ]))
                } else if matches!(status, CollabAgentToolCallStatus::Completed) {
                    let mut name = "Agent".to_string();
                    if let Some(first_receiver) = receiver_thread_ids.first() {
                        if let Ok(tid) = ThreadId::from_string(first_receiver) {
                            let meta = agent_metadata(tid);
                            name = format_agent_picker_item_name(
                                meta.agent_nickname.as_deref(),
                                meta.agent_role.as_deref(),
                                false,
                            );
                        }
                    }

                    let final_model = model.as_ref().or(cached_spawn_request.map(|r| &r.model));
                    let final_effort =
                        reasoning_effort.or(cached_spawn_request.map(|r| r.reasoning_effort));

                    let details = match (final_model, final_effort) {
                        (Some(m), Some(e)) => format!(" ({} {})", m, format!("{e}").to_lowercase()),
                        (Some(m), None) => format!(" ({})", m),
                        (None, Some(e)) => format!(" ({})", format!("{e}").to_lowercase()),
                        (None, None) => String::new(),
                    };

                    let line = format!("Spawned {name}{details}");
                    Some(PlainHistoryCell::new(vec![
                        vec!["• ".dim(), line.into()].into(),
                    ]))
                } else {
                    Some(PlainHistoryCell::new(vec![
                        vec!["■ ".red(), "Failed to spawn agent".into()].into(),
                    ]))
                }
            }
            CollabAgentTool::Wait => {
                let mut lines = Vec::new();
                lines.push(vec!["• ".dim(), "Waiting for agents...".into()].into());
                for rid in receiver_thread_ids {
                    if let Ok(tid) = ThreadId::from_string(rid) {
                        let metadata = agent_metadata(tid);
                        let name = format_agent_picker_item_name(
                            metadata.agent_nickname.as_deref(),
                            metadata.agent_role.as_deref(),
                            false,
                        );
                        let state_str = if let Some(state) = agents_states.get(rid) {
                            let status_str = match state.status {
                                CollabAgentStatus::PendingInit => "Pending",
                                CollabAgentStatus::Running => "Running",
                                CollabAgentStatus::Interrupted => "Interrupted",
                                CollabAgentStatus::Completed => "Completed",
                                CollabAgentStatus::Errored => "Errored",
                                CollabAgentStatus::Shutdown => "Shutdown",
                                CollabAgentStatus::NotFound => "NotFound",
                            };
                            let mut state_str = status_str.to_string();
                            if let Some(msg) = &state.message {
                                state_str.push_str(": ");
                                state_str.push_str(msg);
                            }
                            state_str
                        } else {
                            "Unknown".to_string()
                        };
                        lines.push(
                            vec!["  - ".dim(), name.into(), format!(" ({})", state_str).dim()]
                                .into(),
                        );
                    }
                }
                Some(PlainHistoryCell::new(lines))
            }
            _ => None,
        }
    } else {
        None
    }
}
