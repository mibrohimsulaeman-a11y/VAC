// Typed activity items for the runtime sidebar (Step 00D-8 foundation).

#![allow(unused_imports)]

use vac_core::local_runtime::RuntimeEvent;
use vac_core::local_runtime::ValidationStatus;

use super::labels::approval_decision_label;
use super::labels::approval_kind_label;
use super::labels::preview_inline;
use super::labels::task_kind_label;
use super::labels::validation_status_label;

// =============================================================================
// 00D-8 — Activity Sidebar Foundation
// =============================================================================
//
// Typed activity items projected from RuntimeEvent. The sidebar reads these
// items directly; previously the chatwidget transcript inserted "VAC activity:"
// rows as part of `project_runtime_events`. Activity now lives in a sidebar
// state owned by ChatWidget. Verbose mode (VAC_RUNTIME_VERBOSE=1) still
// preserves the audit-style "vac runtime:" rows in the transcript via
// `verbose_runtime_history_label`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeActivityItem {
    pub kind: RuntimeActivityKind,
    pub status: RuntimeActivityStatus,
    pub label: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeActivityKind {
    Task,
    Command,
    Check,
    Approval,
    Mcp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeActivityStatus {
    Started,
    Completed,
    Failed,
    Waiting,
    Interrupted,
    Info,
}

#[derive(Debug, Clone)]
#[derive(Default)]
pub(crate) struct RuntimeActivityState {
    items: Vec<RuntimeActivityItem>,
    expanded: bool,
}


impl RuntimeActivityState {
    pub(crate) fn push(&mut self, item: RuntimeActivityItem) {
        self.items.push(item);
    }

    pub(crate) fn last_visible(&self, limit: usize) -> &[RuntimeActivityItem] {
        let len = self.items.len();
        let start = len.saturating_sub(limit);
        &self.items[start..]
    }

    pub(crate) fn all(&self) -> &[RuntimeActivityItem] {
        &self.items
    }

    pub(crate) fn is_expanded(&self) -> bool {
        self.expanded
    }

    #[allow(dead_code)]
    pub(crate) fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
    }

    pub(crate) fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
    }

    pub(crate) fn visible_items(&self, limit: usize) -> &[RuntimeActivityItem] {
        if self.expanded {
            self.all()
        } else {
            self.last_visible(limit)
        }
    }
}

pub(crate) const RUNTIME_ACTIVITY_DEFAULT_LIMIT: usize = 5;

/// Whether the runtime audit/verbose mode is enabled (VAC_RUNTIME_VERBOSE).
/// Used by chatwidget to decide whether to also append `vac runtime:` rows
/// in addition to feeding the activity sidebar.
pub(crate) fn runtime_verbose_enabled() -> bool {
    std::env::var_os("VAC_RUNTIME_VERBOSE").is_some()
}

/// Pure mapping from a Local Runtime Contract event to a sidebar item.
/// Returns `None` for events that should not surface in the sidebar
/// (session started, raw assistant streaming deltas, exec started — those
/// are evident from the existing tool cells in the main transcript).
pub(crate) fn runtime_activity_item(event: &RuntimeEvent) -> Option<RuntimeActivityItem> {
    match event {
        RuntimeEvent::SessionStarted(_) => None,

        RuntimeEvent::TaskStarted(t) => Some(RuntimeActivityItem {
            kind: RuntimeActivityKind::Task,
            status: RuntimeActivityStatus::Started,
            label: "task started".to_string(),
            detail: Some(task_kind_label(t.task.kind).to_string()),
        }),

        RuntimeEvent::AssistantDelta(_) => None,

        RuntimeEvent::ToolCallStarted(t) if t.tool_name == "exec" => None,

        RuntimeEvent::ToolCallStarted(t) => {
            if let Some(rest) = t.tool_name.strip_prefix("mcp:") {
                Some(RuntimeActivityItem {
                    kind: RuntimeActivityKind::Mcp,
                    status: RuntimeActivityStatus::Started,
                    label: "mcp started".to_string(),
                    detail: Some(preview_inline(rest)),
                })
            } else {
                Some(RuntimeActivityItem {
                    kind: RuntimeActivityKind::Mcp,
                    status: RuntimeActivityStatus::Started,
                    label: "tool started".to_string(),
                    detail: Some(preview_inline(&t.tool_name)),
                })
            }
        }

        RuntimeEvent::ToolCallFinished(t) => {
            let status = if t.success {
                RuntimeActivityStatus::Completed
            } else {
                RuntimeActivityStatus::Failed
            };

            if t.tool_name == "exec" {
                Some(RuntimeActivityItem {
                    kind: RuntimeActivityKind::Command,
                    status,
                    label: if t.success {
                        "command completed".to_string()
                    } else {
                        "command failed".to_string()
                    },
                    detail: t.output_preview.as_deref().map(preview_inline),
                })
            } else if let Some(rest) = t.tool_name.strip_prefix("mcp:") {
                Some(RuntimeActivityItem {
                    kind: RuntimeActivityKind::Mcp,
                    status,
                    label: if t.success {
                        "mcp completed".to_string()
                    } else {
                        "mcp failed".to_string()
                    },
                    detail: Some(preview_inline(rest)),
                })
            } else {
                Some(RuntimeActivityItem {
                    kind: RuntimeActivityKind::Mcp,
                    status,
                    label: if t.success {
                        "tool completed".to_string()
                    } else {
                        "tool failed".to_string()
                    },
                    detail: Some(preview_inline(&t.tool_name)),
                })
            }
        }

        RuntimeEvent::ApprovalRequested(req) => Some(RuntimeActivityItem {
            kind: RuntimeActivityKind::Approval,
            status: RuntimeActivityStatus::Waiting,
            label: "approval requested".to_string(),
            detail: Some(approval_kind_label(req.action).to_string()),
        }),

        RuntimeEvent::ApprovalResolved(res) => Some(RuntimeActivityItem {
            kind: RuntimeActivityKind::Approval,
            status: RuntimeActivityStatus::Completed,
            label: format!("approval {}", approval_decision_label(res.decision)),
            detail: Some("request".to_string()),
        }),

        RuntimeEvent::ValidationStarted(v) => Some(RuntimeActivityItem {
            kind: RuntimeActivityKind::Check,
            status: RuntimeActivityStatus::Started,
            label: "check started".to_string(),
            detail: Some(preview_inline(&v.command_display)),
        }),

        RuntimeEvent::ValidationFinished(v) => Some(RuntimeActivityItem {
            kind: RuntimeActivityKind::Check,
            status: match v.status {
                ValidationStatus::Passed => RuntimeActivityStatus::Completed,
                ValidationStatus::Failed => RuntimeActivityStatus::Failed,
                ValidationStatus::Skipped => RuntimeActivityStatus::Info,
                ValidationStatus::Cancelled => RuntimeActivityStatus::Interrupted,
            },
            label: format!("check {}", validation_status_label(v.status)),
            detail: Some(preview_inline(&v.command_display)),
        }),

        RuntimeEvent::TaskCompleted(_) => Some(RuntimeActivityItem {
            kind: RuntimeActivityKind::Task,
            status: RuntimeActivityStatus::Completed,
            label: "task completed".to_string(),
            detail: None,
        }),

        RuntimeEvent::TaskFailed(f) => Some(RuntimeActivityItem {
            kind: RuntimeActivityKind::Task,
            status: RuntimeActivityStatus::Failed,
            label: "task failed".to_string(),
            detail: Some(preview_inline(&f.error.message)),
        }),

        RuntimeEvent::TaskCancelled(c) => match c.reason.as_deref() {
            Some("interrupted") => Some(RuntimeActivityItem {
                kind: RuntimeActivityKind::Task,
                status: RuntimeActivityStatus::Interrupted,
                label: "task interrupted".to_string(),
                detail: None,
            }),
            Some(reason) => Some(RuntimeActivityItem {
                kind: RuntimeActivityKind::Task,
                status: RuntimeActivityStatus::Interrupted,
                label: "task cancelled".to_string(),
                detail: Some(preview_inline(reason)),
            }),
            None => Some(RuntimeActivityItem {
                kind: RuntimeActivityKind::Task,
                status: RuntimeActivityStatus::Interrupted,
                label: "task cancelled".to_string(),
                detail: None,
            }),
        },
        RuntimeEvent::SessionEnded(_) => None,
        RuntimeEvent::EnteredReviewMode(_) => None,
        RuntimeEvent::ExitedReviewMode(_) => None,
    }
}
