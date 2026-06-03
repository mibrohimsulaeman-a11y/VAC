// Local-tool stub for removed multi-agent collaboration UI.
//
// External-agent import/migration remains in the codebase; this module only disables the
// interactive multi-agent collaboration surface that is outside the local TUI+CLI coding scope.

use crate::history_cell::PlainHistoryCell;
use crate::session_protocol::ThreadItem;
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
    let dot = if is_closed { "•".dim() } else { "•".green() };
    vec![dot, " ".into()]
}

pub(crate) fn format_agent_picker_item_name(
    agent_nickname: Option<&str>,
    agent_role: Option<&str>,
    is_primary: bool,
) -> String {
    if is_primary { return "Main [default]".to_string(); }
    match (agent_nickname, agent_role) {
        (Some(name), Some(role)) if !name.trim().is_empty() && !role.trim().is_empty() => {
            format!("{} [{}]", name.trim(), role.trim())
        }
        (Some(name), _) if !name.trim().is_empty() => name.trim().to_string(),
        (_, Some(role)) if !role.trim().is_empty() => format!("[{}]", role.trim()),
        _ => "Agent".to_string(),
    }
}

pub(crate) fn previous_agent_shortcut() -> crate::key_hint::KeyBinding { crate::key_hint::alt(crossterm::event::KeyCode::Left) }
pub(crate) fn next_agent_shortcut() -> crate::key_hint::KeyBinding { crate::key_hint::alt(crossterm::event::KeyCode::Right) }
pub(crate) fn previous_agent_shortcut_matches(_key_event: KeyEvent, _allow_word_motion_fallback: bool) -> bool { false }
pub(crate) fn next_agent_shortcut_matches(_key_event: KeyEvent, _allow_word_motion_fallback: bool) -> bool { false }

pub(crate) fn spawn_request_summary(_item: &ThreadItem) -> Option<SpawnRequestSummary> { None }

pub(crate) fn tool_call_history_cell(
    _item: &ThreadItem,
    _cached_spawn_request: Option<&SpawnRequestSummary>,
    _agent_metadata: impl FnMut(ThreadId) -> AgentMetadata,
) -> Option<PlainHistoryCell> {
    None
}
