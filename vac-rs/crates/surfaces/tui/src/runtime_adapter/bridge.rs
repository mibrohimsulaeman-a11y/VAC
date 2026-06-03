// `RuntimeBridge` projects in-process protocol events into `RuntimeEvent`s.

#![allow(unused_imports)]

use std::collections::HashMap;
use std::path::PathBuf;
use tracing::warn;

use vac_core::local_runtime::ApprovalAction;
use vac_core::local_runtime::ApprovalDecision;
use vac_core::local_runtime::ApprovalId;
use vac_core::local_runtime::ApprovalPreview;
use vac_core::local_runtime::ApprovalRequest;
use vac_core::local_runtime::ApprovalResolved;
use vac_core::local_runtime::ApprovalResource;
use vac_core::local_runtime::AssistantDelta;
use vac_core::local_runtime::RiskLevel;
use vac_core::local_runtime::RuntimeCommand;
use vac_core::local_runtime::RuntimeError;
use vac_core::local_runtime::RuntimeEvent;
use vac_core::local_runtime::RuntimeSession;
use vac_core::local_runtime::RuntimeTask;
use vac_core::local_runtime::SessionId;
use vac_core::local_runtime::SessionStarted;
use vac_core::local_runtime::TaskCancelled;
use vac_core::local_runtime::TaskCompleted;
use vac_core::local_runtime::TaskFailed;
use vac_core::local_runtime::TaskId;
use vac_core::local_runtime::TaskStarted;
use vac_core::local_runtime::ToolCallFinished;
use vac_core::local_runtime::ToolCallStarted;
use vac_core::local_runtime::ValidationFinished;
use vac_core::local_runtime::ValidationStarted;
use vac_core::local_runtime::ValidationStatus;

use super::activity::RuntimeActivityItem;
use super::activity::RuntimeActivityKind;
use super::activity::RuntimeActivityStatus;
use super::activity::runtime_activity_item;
use super::approval::approval_decision_command;
use super::approval::approval_registry;
use super::labels::RuntimeLabelMode;
use super::labels::activity_history_label;
use super::labels::approval_decision_label;
use super::labels::command_display;
use super::labels::is_validation_command;
use super::labels::preview_text;
use super::labels::runtime_label_mode;
use super::labels::tool_call_id_from_string;
use super::labels::verbose_runtime_history_label;

/// Stateful projector that turns the legacy in-process [`EventMsg`] stream
/// into [`RuntimeEvent`]s for the existing root TUI surfaces.
///
/// The bridge is intentionally minimal — it owns just enough state to know
/// which [`SessionId`] / [`TaskId`] every event belongs to, plus the
/// validation-call bookkeeping needed to emit `ValidationFinished` when a
/// validation `cargo test` / `pytest` / etc. command completes.
pub(crate) struct RuntimeBridge {
    session: RuntimeSession,
    task: RuntimeTask,
    validation_calls: std::collections::HashMap<String, String>,
    /// Call-ids whose approval id was minted by this bridge and inserted into
    /// [`approval_registry`]. Drained on terminal task projection so a
    /// cancelled or failed turn does not leak entries across tasks.
    tracked_approval_call_ids: Vec<String>,
    /// `call_id` -> exec command preview captured at `ToolCallStarted`. Used
    /// so the matching `ToolCallFinished` row reuses the *same* preview the
    /// operator already saw when the command started, even if the caller
    /// passes a different (or empty) command slice on finish.
    exec_calls: std::collections::HashMap<String, String>,
    /// `call_id` -> `"server/tool"` label captured at `ToolCallStarted` for
    /// MCP tool calls. Same lifecycle-pairing intent as `exec_calls`.
    mcp_calls: std::collections::HashMap<String, String>,
    /// `ApprovalId` -> kind label ("exec", "patch", "permissions",
    /// "mcp elicitation") captured when an `ApprovalRequested` is projected.
    /// Looked up at `ApprovalResolved` rendering time so the resolved row
    /// agrees with the requested row on what was being approved.
    approval_kinds: std::collections::HashMap<ApprovalId, &'static str>,
    seen_session_started: bool,
    seen_task_started: bool,
    assistant_delta_seen_this_turn: bool,
}

impl RuntimeBridge {
    /// Construct a fresh bridge for the supplied session/task pair.
    pub(crate) fn new(session: RuntimeSession, task: RuntimeTask) -> Self {
        Self {
            session,
            task,
            validation_calls: std::collections::HashMap::new(),
            tracked_approval_call_ids: Vec::new(),
            exec_calls: std::collections::HashMap::new(),
            mcp_calls: std::collections::HashMap::new(),
            approval_kinds: std::collections::HashMap::new(),
            seen_session_started: false,
            seen_task_started: false,
            assistant_delta_seen_this_turn: false,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn session_id(&self) -> SessionId {
        self.session.id
    }

    #[allow(dead_code)]
    pub(crate) fn task_id(&self) -> TaskId {
        self.task.id
    }

    /// Emit the synthetic `SessionStarted` / `TaskStarted` pair that the TUI
    /// surfaces should display the first time the bridge is asked for the
    /// runtime view. This gives operators an immediate, contract-shaped
    /// confirmation that the bottom-input Enter routed through Local Runtime
    /// Contract — it is not gated on any wire turn message.
    pub(crate) fn opening_events(&mut self) -> Vec<RuntimeEvent> {
        let mut events = Vec::new();
        if !self.seen_session_started {
            self.seen_session_started = true;
            events.push(RuntimeEvent::SessionStarted(SessionStarted::new(
                self.session.clone(),
            )));
        }
        if !self.seen_task_started {
            self.seen_task_started = true;
            let mut task = self.task.clone();
            if let Err(err) = task.start() {
                warn!(error = ?err, "runtime bridge task was not pending before opening event");
            } else {
                self.task = task.clone();
                events.push(RuntimeEvent::TaskStarted(TaskStarted::new(task)));
            }
        }
        events
    }

    /// Record an approval decision originating from the existing root
    /// approval overlay. Returns both the [`RuntimeCommand`] (so the caller
    /// can surface it on the contract control-plane / log it) and the
    /// matching [`RuntimeEvent::ApprovalResolved`] for the activity rows.
    #[allow(dead_code)]
    pub(crate) fn record_approval_decision(
        &mut self,
        approval_id: ApprovalId,
        decision: ApprovalDecision,
        reason: Option<String>,
    ) -> (RuntimeCommand, RuntimeEvent) {
        let command = approval_decision_command(approval_id, decision);
        let resolved =
            RuntimeEvent::ApprovalResolved(ApprovalResolved::new(approval_id, decision, reason));
        (command, resolved)
    }

    /// Record an approval decision keyed by the originating protocol
    /// `call_id` instead of an explicit [`ApprovalId`]. Looks up the id
    /// minted at `project_exec_approval_request` /
    /// `project_apply_patch_approval_request` time so the resulting
    /// `ApprovalResolved.approval_id` correlates with the matching
    /// `ApprovalRequested.approval_id`. Returns `None` when the call_id has
    /// no pending entry — the caller decides whether to mint a fresh id
    /// (paranoia path) or skip the runtime mirror entirely.
    #[allow(dead_code)]
    pub(crate) fn record_call_approval_decision(
        &mut self,
        call_id: &str,
        decision: ApprovalDecision,
        reason: Option<String>,
    ) -> Option<(RuntimeCommand, RuntimeEvent)> {
        let approval_id = approval_registry::take(call_id)?;
        self.tracked_approval_call_ids.retain(|id| id != call_id);
        Some(self.record_approval_decision(approval_id, decision, reason))
    }

    /// Project an exec approval request from the TUI-local approval surface
    /// into the contract `ApprovalRequested` event. Mirrors
    /// `project_exec_approval_request` but accepts the simpler argument
    /// shape that `chatwidget::handle_exec_approval_now` already has.
    ///
    /// Mints a fresh `ApprovalId`, registers `call_id -> ApprovalId` in
    /// `approval_registry`, and tracks the `call_id` on the bridge so it can
    /// be drained by terminal task projections. The overlay-side resolve fns
    /// later `take(call_id)` from the registry to emit `ApprovalResolved` and
    /// `RuntimeCommand::{Approve,Reject}` carrying the **same** `ApprovalId`.
    pub(crate) fn project_exec_approval_request(
        &mut self,
        call_id: &str,
        command: &[String],
        reason: Option<&str>,
    ) -> Vec<RuntimeEvent> {
        let display = command_display(command);
        let approval_id = ApprovalId::new();
        approval_registry::record(call_id, approval_id);
        self.tracked_approval_call_ids.push(call_id.to_string());
        self.approval_kinds.insert(approval_id, "exec");
        let request = ApprovalRequest {
            id: approval_id,
            task_id: self.task.id,
            action: ApprovalAction::ExecuteProcess,
            risk: RiskLevel::Execute,
            reason: reason
                .map(str::to_string)
                .unwrap_or_else(|| "command requires approval".to_string()),
            resources: vec![ApprovalResource::Command(display.clone())],
            preview: ApprovalPreview::Command(display),
            validation_after: Vec::new(),
        };
        vec![RuntimeEvent::ApprovalRequested(Box::new(request))]
    }

    /// Project a patch (apply-patch) approval request into the contract
    /// `ApprovalRequested` event.
    pub(crate) fn project_apply_patch_approval_request(
        &mut self,
        call_id: &str,
        files: Vec<PathBuf>,
        reason: Option<&str>,
    ) -> Vec<RuntimeEvent> {
        let preview = if files.is_empty() {
            ApprovalPreview::None
        } else {
            ApprovalPreview::FileList(files.clone())
        };
        let resources: Vec<ApprovalResource> =
            files.into_iter().map(ApprovalResource::File).collect();
        let approval_id = ApprovalId::new();
        approval_registry::record(call_id, approval_id);
        self.tracked_approval_call_ids.push(call_id.to_string());
        self.approval_kinds.insert(approval_id, "patch");
        let request = ApprovalRequest::safe_edit(
            approval_id,
            self.task.id,
            reason
                .map(str::to_string)
                .unwrap_or_else(|| "patch approval required".to_string()),
            resources,
            preview,
            Vec::new(),
        );
        vec![RuntimeEvent::ApprovalRequested(Box::new(request))]
    }

    /// Project a permissions approval request into the Local Runtime Contract.
    ///
    /// Mints a fresh `ApprovalId`, registers `call_id -> ApprovalId` in the
    /// `approval_registry`, and tracks the `call_id` so terminal task
    /// projections drain stale entries. The overlay-side
    /// [`approval_resolved_for_permissions_decision`] later `take(call_id)`s
    /// from the registry to emit `ApprovalResolved` carrying the same id.
    pub(crate) fn project_permissions_approval_request(
        &mut self,
        call_id: &str,
        summary: &str,
        reason: Option<&str>,
    ) -> Vec<RuntimeEvent> {
        let approval_id = ApprovalId::new();
        approval_registry::record(call_id, approval_id);
        self.tracked_approval_call_ids.push(call_id.to_string());
        self.approval_kinds.insert(approval_id, "permissions");
        let preview_body = if summary.is_empty() {
            "additional permissions requested".to_string()
        } else {
            summary.to_string()
        };
        let request = ApprovalRequest {
            id: approval_id,
            task_id: self.task.id,
            action: ApprovalAction::Other,
            risk: RiskLevel::Credential,
            reason: reason
                .map(str::to_string)
                .unwrap_or_else(|| "additional permissions requested".to_string()),
            resources: Vec::new(),
            preview: ApprovalPreview::Text(preview_body),
            validation_after: Vec::new(),
        };
        vec![RuntimeEvent::ApprovalRequested(Box::new(request))]
    }

    /// Project an MCP server elicitation request into the Local Runtime
    /// Contract. Keyed on the protocol `request_id` so the matching resolve
    /// fn `approval_resolved_for_elicitation_decision` can re-use the same
    /// `ApprovalId` for `ApprovalResolved`.
    pub(crate) fn project_mcp_elicitation_request(
        &mut self,
        request_id: &str,
        server_name: &str,
        message: &str,
    ) -> Vec<RuntimeEvent> {
        let approval_id = ApprovalId::new();
        approval_registry::record(request_id, approval_id);
        self.tracked_approval_call_ids.push(request_id.to_string());
        self.approval_kinds.insert(approval_id, "mcp elicitation");
        let preview_body = if message.is_empty() {
            format!("MCP server `{server_name}` requested elicitation")
        } else {
            format!("{server_name}: {message}")
        };
        let request = ApprovalRequest {
            id: approval_id,
            task_id: self.task.id,
            action: ApprovalAction::ConnectorCall,
            risk: RiskLevel::Network,
            reason: format!("MCP server `{server_name}` requested elicitation"),
            resources: Vec::new(),
            preview: ApprovalPreview::Text(preview_body),
            validation_after: Vec::new(),
        };
        vec![RuntimeEvent::ApprovalRequested(Box::new(request))]
    }

    /// Drop all per-turn tracking state shared by terminal projections.
    /// Used by `project_task_failed` and `project_task_cancelled`. The
    /// `project_task_completed` path inlines the same clears around its
    /// pending-validation cancellation emission so cancelled validation
    /// events fire **before** state is dropped.
    fn clear_turn_tracking(&mut self) {
        self.assistant_delta_seen_this_turn = false;
        let stale = std::mem::take(&mut self.tracked_approval_call_ids);
        approval_registry::drop_all(&stale);
        self.validation_calls.clear();
        self.exec_calls.clear();
        self.mcp_calls.clear();
        self.approval_kinds.clear();
    }

    /// Emit `TaskCompleted` plus any pending `ValidationFinished` cancellations
    /// when the active turn finishes successfully. Pending validation rows
    /// are emitted as `Cancelled` BEFORE state is dropped so the operator
    /// sees them paired with their started rows.
    pub(crate) fn project_task_completed(&mut self) -> Vec<RuntimeEvent> {
        self.assistant_delta_seen_this_turn = false;
        let stale = std::mem::take(&mut self.tracked_approval_call_ids);
        approval_registry::drop_all(&stale);
        let mut events = Vec::new();
        let pending_validations = std::mem::take(&mut self.validation_calls);
        for command_display in pending_validations.into_values() {
            events.push(RuntimeEvent::ValidationFinished(ValidationFinished::new(
                self.task.id,
                command_display,
                ValidationStatus::Cancelled,
                Some("validation was not completed before the task finished".to_string()),
            )));
        }
        self.exec_calls.clear();
        self.mcp_calls.clear();
        self.approval_kinds.clear();
        events.push(RuntimeEvent::TaskCompleted(TaskCompleted::new(
            self.task.id,
            None,
            Vec::new(),
        )));
        events
    }

    /// Emit `TaskFailed` for a non-retry failure surfaced by the TUI.
    pub(crate) fn project_task_failed(&mut self, message: String) -> Vec<RuntimeEvent> {
        self.clear_turn_tracking();
        vec![RuntimeEvent::TaskFailed(TaskFailed::new(
            self.task.id,
            RuntimeError::new(
                vac_core::local_runtime::RuntimeErrorCode::Unknown("error".to_string()),
                message,
                None::<String>,
                false,
                vac_core::local_runtime::RuntimeErrorOwner::Internal,
            ),
        ))]
    }

    /// Emit `TaskCancelled` for an interrupted/aborted turn.
    pub(crate) fn project_task_cancelled(&mut self, reason: Option<String>) -> Vec<RuntimeEvent> {
        self.clear_turn_tracking();
        vec![RuntimeEvent::TaskCancelled(TaskCancelled::new(
            self.task.id,
            reason,
        ))]
    }

    /// Project a streaming assistant text delta. To avoid spamming the
    /// history surface (assistant streaming arrives token-by-token), we emit
    /// a single `AssistantDelta` row per turn — a marker that the model is
    /// streaming — and let the existing answer-stream UI render the actual
    /// text. Subsequent deltas in the same turn return an empty vector. The
    /// flag resets at task complete/fail/cancel.
    pub(crate) fn project_assistant_delta(&mut self, delta: &str) -> Vec<RuntimeEvent> {
        if self.assistant_delta_seen_this_turn {
            return Vec::new();
        }
        if delta.trim().is_empty() {
            return Vec::new();
        }
        self.assistant_delta_seen_this_turn = true;
        vec![RuntimeEvent::AssistantDelta(AssistantDelta::new(
            self.task.id,
            delta.to_string(),
        ))]
    }

    /// Project the start of an exec command. Validation commands
    /// (`cargo test`, `pytest`, etc.) become `ValidationStarted`; everything
    /// else becomes a generic `ToolCallStarted` with tool name `exec` and a
    /// truncated command preview. Tracks the call id so the matching
    /// finish-event can pair correctly.
    pub(crate) fn project_exec_command_started(
        &mut self,
        call_id: &str,
        command: &[String],
    ) -> Vec<RuntimeEvent> {
        let display = command_display(command);
        if is_validation_command(command) {
            self.validation_calls
                .insert(call_id.to_string(), display.clone());
            return vec![RuntimeEvent::ValidationStarted(ValidationStarted::new(
                self.task.id,
                display,
            ))];
        }
        let preview = preview_text(&display);
        if let Some(p) = preview.as_deref() {
            self.exec_calls.insert(call_id.to_string(), p.to_string());
        }
        vec![RuntimeEvent::ToolCallStarted(ToolCallStarted::new(
            self.task.id,
            tool_call_id_from_string(call_id),
            "exec",
            preview,
        ))]
    }

    /// Project the end of an exec command. If the call id was a tracked
    /// validation, emits `ValidationFinished` with status derived from
    /// `exit_code` (0 == Passed). Otherwise emits a generic
    /// `ToolCallFinished` with `success = exit_code == 0`.
    pub(crate) fn project_exec_command_finished(
        &mut self,
        call_id: &str,
        command: &[String],
        exit_code: i32,
    ) -> Vec<RuntimeEvent> {
        let success = exit_code == 0;
        if let Some(stored) = self.validation_calls.remove(call_id) {
            let status = if success {
                ValidationStatus::Passed
            } else {
                ValidationStatus::Failed
            };
            let summary = if success {
                None
            } else {
                Some(format!("exit code {exit_code}"))
            };
            return vec![RuntimeEvent::ValidationFinished(ValidationFinished::new(
                self.task.id,
                stored,
                status,
                summary,
            ))];
        }
        // Lifecycle pairing: prefer the preview captured at started-time so
        // the operator sees the same command on both rows. Falls back to the
        // finish-time slice when no started entry was recorded (tests, replay).
        let preview = match self.exec_calls.remove(call_id) {
            Some(saved) => Some(saved),
            None => preview_text(&command_display(command)),
        };
        vec![RuntimeEvent::ToolCallFinished(ToolCallFinished::new(
            self.task.id,
            tool_call_id_from_string(call_id),
            "exec",
            preview,
            success,
        ))]
    }

    /// Project the start of an MCP tool call as a generic
    /// `ToolCallStarted` row labelled `mcp:<server>/<tool>`.
    pub(crate) fn project_mcp_tool_started(
        &mut self,
        call_id: &str,
        server: &str,
        tool: &str,
    ) -> Vec<RuntimeEvent> {
        let tool_name = format!("mcp:{server}/{tool}");
        self.mcp_calls
            .insert(call_id.to_string(), tool_name.clone());
        vec![RuntimeEvent::ToolCallStarted(ToolCallStarted::new(
            self.task.id,
            tool_call_id_from_string(call_id),
            tool_name,
            None::<String>,
        ))]
    }

    /// Project the end of an MCP tool call.
    pub(crate) fn project_mcp_tool_finished(
        &mut self,
        call_id: &str,
        server: &str,
        tool: &str,
        success: bool,
    ) -> Vec<RuntimeEvent> {
        // Lifecycle pairing: reuse the label captured at start.
        let tool_name = self
            .mcp_calls
            .remove(call_id)
            .unwrap_or_else(|| format!("mcp:{server}/{tool}"));
        vec![RuntimeEvent::ToolCallFinished(ToolCallFinished::new(
            self.task.id,
            tool_call_id_from_string(call_id),
            tool_name,
            None::<String>,
            success,
        ))]
    }

    /// Bridge-aware history label. Dispatches between the user-facing
    /// activity rows (default) and the verbose `vac runtime` trace (opt-in
    /// via `VAC_RUNTIME_VERBOSE`). Returns `None` when the current mode
    /// suppresses the event so chatwidget can skip rendering an empty cell.
    #[allow(dead_code)]
    pub(crate) fn label_for(&self, event: &RuntimeEvent) -> Option<String> {
        match runtime_label_mode() {
            RuntimeLabelMode::Activity => self.activity_label_for(event),
            RuntimeLabelMode::VerboseRuntime => Some(self.verbose_label_for(event)),
        }
    }

    /// Verbose, audit-style label used when `VAC_RUNTIME_VERBOSE` is set.
    /// Falls through to [`verbose_runtime_history_label`]; for
    /// `ApprovalResolved` it consults `approval_kinds` so the resolved row
    /// carries the same kind label as the requested row
    /// ("approved — exec" pairs with "approval requested — exec").
    pub(crate) fn verbose_label_for(&self, event: &RuntimeEvent) -> String {
        match event {
            RuntimeEvent::ApprovalResolved(res) => {
                let outcome = approval_decision_label(res.decision);
                let kind = self
                    .approval_kinds
                    .get(&res.approval_id)
                    .copied()
                    .unwrap_or("request");
                format!("vac runtime: approval {outcome} — {kind}")
            }
            _ => verbose_runtime_history_label(event),
        }
    }

    /// Activity-mode label used in normal user mode. Bridges
    /// `ApprovalResolved` rows to the kind captured at request-time, then
    /// delegates remaining events to [`activity_history_label`] (which
    /// suppresses session-start, raw assistant streaming, and exec-started
    /// to keep the visible work log low-noise).
    #[allow(dead_code)]
    pub(crate) fn activity_label_for(&self, event: &RuntimeEvent) -> Option<String> {
        match event {
            RuntimeEvent::ApprovalResolved(res) => {
                let outcome = approval_decision_label(res.decision);
                let kind = self
                    .approval_kinds
                    .get(&res.approval_id)
                    .copied()
                    .unwrap_or("request");
                Some(format!("VAC activity: approval {outcome} — {kind}"))
            }
            _ => activity_history_label(event),
        }
    }

    /// Bridge-aware mapping from a runtime event to a sidebar activity
    /// item. For [`RuntimeEvent::ApprovalResolved`] it consults
    /// `approval_kinds` so the resolved row carries the same kind label
    /// (e.g. "patch") as the requested row, mirroring the bridge-aware
    /// label semantics. Other events delegate to [`runtime_activity_item`].
    pub(crate) fn activity_item_for(&self, event: &RuntimeEvent) -> Option<RuntimeActivityItem> {
        match event {
            RuntimeEvent::ApprovalResolved(res) => {
                let kind = self
                    .approval_kinds
                    .get(&res.approval_id)
                    .copied()
                    .unwrap_or("request");
                Some(RuntimeActivityItem {
                    kind: RuntimeActivityKind::Approval,
                    status: RuntimeActivityStatus::Completed,
                    label: format!("approval {}", approval_decision_label(res.decision)),
                    detail: Some(kind.to_string()),
                })
            }
            _ => runtime_activity_item(event),
        }
    }

    #[cfg(test)]
    pub(super) fn exec_calls_len(&self) -> usize {
        self.exec_calls.len()
    }

    #[cfg(test)]
    pub(super) fn mcp_calls_len(&self) -> usize {
        self.mcp_calls.len()
    }

    #[cfg(test)]
    pub(super) fn approval_kinds_len(&self) -> usize {
        self.approval_kinds.len()
    }

    #[cfg(test)]
    pub(super) fn validation_calls_len(&self) -> usize {
        self.validation_calls.len()
    }
}
