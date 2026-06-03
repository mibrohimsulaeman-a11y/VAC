//! Projection of `vac_protocol::Event` into `RuntimeEvent` for the local runtime.
//!
//! Lifted from `vac-rs/exec/src/local_runtime.rs` (P1.2 of code-review plan).
//! Owns `LocalRuntimeBridge`, `BridgeOutput`, `runtime_approval_events_for_*`,
//! and helper functions for event/turn/tool-call/preview formatting.

use std::collections::HashMap;
use std::fmt::Write as _;

use vac_protocol::approvals::ApplyPatchApprovalRequestEvent;
use vac_protocol::approvals::ElicitationRequestEvent;
use vac_protocol::approvals::ExecApprovalRequestEvent;
use vac_protocol::items::TurnItem;
use vac_protocol::models::ContentItem;
use vac_protocol::protocol::DeprecationNoticeEvent;
use vac_protocol::protocol::Event;
use vac_protocol::protocol::EventMsg;
use vac_protocol::protocol::ExecCommandStatus;
use vac_protocol::protocol::ItemCompletedEvent;
use vac_protocol::protocol::ItemStartedEvent;
use vac_protocol::protocol::ModelRerouteEvent;
use vac_protocol::protocol::ModelVerificationEvent;
use vac_protocol::protocol::RawResponseItemEvent;
use vac_protocol::protocol::StreamErrorEvent;
use vac_protocol::protocol::TurnAbortReason;
use vac_protocol::protocol::TurnCompleteEvent;
use vac_protocol::protocol::TurnStartedEvent;
use vac_protocol::protocol::UserMessageEvent;
use vac_protocol::protocol::WarningEvent;
use vac_protocol::request_permissions::RequestPermissionsEvent;

use super::*;

const DEFAULT_APPROVAL_HINT: &str = "approval is not available in exec mode";

/// Soft cap on `LocalRuntimeBridge::assistant_buffer` in bytes. Long-running
/// sessions do not retain unbounded transcript content; oldest content is
/// trimmed at a UTF-8 boundary when the cap is exceeded.
const ASSISTANT_BUFFER_SOFT_CAP_BYTES: usize = 256 * 1024;

/// Soft cap on the number of evidence entries collected by
/// `LocalRuntimeBridge`. Entries beyond the cap are dropped and the dropped
/// count is surfaced in the final `TaskCompleted` evidence list.
const EVIDENCE_SOFT_CAP_ENTRIES: usize = 1024;

pub struct LocalRuntimeBridge {
    session_id: SessionId,
    task_id: TaskId,
    assistant_buffer: String,
    /// Total bytes dropped from `assistant_buffer` due to the soft cap.
    /// Surfaced in `TaskCompleted` evidence when non-zero so operators can
    /// spot truncated transcripts.
    assistant_buffer_overflowed_bytes: usize,
    evidence: Vec<String>,
    /// Number of evidence entries dropped due to `EVIDENCE_SOFT_CAP_ENTRIES`.
    /// Surfaced in `TaskCompleted` evidence when non-zero.
    evidence_overflowed: usize,
    final_message: Option<String>,
    validation_calls: HashMap<String, String>,
    current_turn_id: Option<String>,
    task_prompt: String,
}

pub struct BridgeOutput {
    pub events: Vec<RuntimeEvent>,
    pub error_seen: bool,
    pub terminate: bool,
}

impl LocalRuntimeBridge {
    pub fn new(session_id: SessionId, task_id: TaskId, task_prompt: String) -> Self {
        Self {
            session_id,
            task_id,
            assistant_buffer: String::new(),
            assistant_buffer_overflowed_bytes: 0,
            evidence: Vec::new(),
            evidence_overflowed: 0,
            final_message: None,
            validation_calls: HashMap::new(),
            current_turn_id: None,
            task_prompt,
        }
    }

    /// Append to `assistant_buffer` while honoring
    /// `ASSISTANT_BUFFER_SOFT_CAP_BYTES`. Drops the oldest content at a
    /// UTF-8 boundary when the cap would be exceeded and bumps
    /// `assistant_buffer_overflowed_bytes`.
    fn append_assistant_text(&mut self, text: &str) {
        self.assistant_buffer.push_str(text);
        if self.assistant_buffer.len() <= ASSISTANT_BUFFER_SOFT_CAP_BYTES {
            return;
        }

        let drop_bytes = self.assistant_buffer.len() - ASSISTANT_BUFFER_SOFT_CAP_BYTES;
        let mut cut = drop_bytes;
        while cut < self.assistant_buffer.len() && !self.assistant_buffer.is_char_boundary(cut) {
            cut += 1;
        }
        self.assistant_buffer_overflowed_bytes =
            self.assistant_buffer_overflowed_bytes.saturating_add(cut);
        self.assistant_buffer.drain(..cut);
    }

    /// Append to `evidence` while honoring `EVIDENCE_SOFT_CAP_ENTRIES`.
    /// Drops new entries silently past the cap and bumps
    /// `evidence_overflowed` so the count can be surfaced at TaskCompleted.
    fn push_evidence(&mut self, entry: String) {
        if self.evidence.len() < EVIDENCE_SOFT_CAP_ENTRIES {
            self.evidence.push(entry);
        } else {
            self.evidence_overflowed = self.evidence_overflowed.saturating_add(1);
        }
    }

    pub fn warnings_for_event(&mut self, event: &Event) -> Vec<String> {
        match &event.msg {
            EventMsg::Warning(WarningEvent { message }) => vec![format!("warning: {message}")],
            EventMsg::DeprecationNotice(DeprecationNoticeEvent { summary, details }) => {
                let mut message = format!("deprecated: {summary}");
                if let Some(details) = details {
                    let _ = write!(message, " ({details})");
                }
                vec![message]
            }
            EventMsg::ModelReroute(ModelRerouteEvent {
                from_model,
                to_model,
                ..
            }) => vec![format!("model rerouted: {from_model} -> {to_model}")],
            EventMsg::ModelVerification(ModelVerificationEvent { verifications }) => {
                vec![format!("model verification: {verifications:?}")]
            }
            EventMsg::StreamError(StreamErrorEvent {
                message,
                additional_details,
                ..
            }) => {
                let mut warning = message.clone();
                if let Some(details) = additional_details {
                    let _ = write!(warning, " ({details})");
                }
                vec![warning]
            }
            EventMsg::Error(error) => vec![format!("error: {}", error.message)],
            _ => Vec::new(),
        }
    }

    pub fn map_core_event(&mut self, event: Event) -> BridgeOutput {
        if let Some(turn_id) = event_turn_id(&event.msg)
            && let Some(active_turn_id) = self.current_turn_id.as_deref()
            && active_turn_id != turn_id
        {
            tracing::debug!(
                turn_id = %turn_id,
                active_turn_id = %active_turn_id,
                "dropping local-runtime event from a stale turn"
            );
            return BridgeOutput {
                events: Vec::new(),
                error_seen: false,
                terminate: false,
            };
        }

        let mut events = Vec::new();
        let mut error_seen = false;
        let mut terminate = false;

        match event.msg {
            EventMsg::TurnStarted(TurnStartedEvent { turn_id, .. }) => {
                self.current_turn_id = Some(turn_id);
                let mut task = RuntimeTask::with_id(
                    self.task_id,
                    self.session_id,
                    RuntimeTaskKind::SemanticCoding,
                    self.task_prompt.clone(),
                );
                task.start().expect("task should be Pending before start");
                events.push(RuntimeEvent::TaskStarted(TaskStarted::new(task)));
            }
            EventMsg::AgentMessageContentDelta(delta) => {
                self.append_assistant_text(&delta.delta);
                events.push(RuntimeEvent::AssistantDelta(AssistantDelta::new(
                    self.task_id,
                    delta.delta,
                )));
            }
            EventMsg::PlanDelta(delta) => {
                self.append_assistant_text(&delta.delta);
                events.push(RuntimeEvent::AssistantDelta(AssistantDelta::new(
                    self.task_id,
                    delta.delta,
                )));
            }
            EventMsg::ReasoningContentDelta(delta) => {
                self.append_assistant_text(&delta.delta);
                events.push(RuntimeEvent::AssistantDelta(AssistantDelta::new(
                    self.task_id,
                    delta.delta,
                )));
            }
            EventMsg::ReasoningRawContentDelta(delta) => {
                self.append_assistant_text(&delta.delta);
                events.push(RuntimeEvent::AssistantDelta(AssistantDelta::new(
                    self.task_id,
                    delta.delta,
                )));
            }
            EventMsg::ExecCommandBegin(begin) => {
                let tool_call_id = tool_call_id_from_string(&begin.call_id);
                let command_display = command_display(&begin.command);
                events.push(RuntimeEvent::ToolCallStarted(ToolCallStarted::new(
                    self.task_id,
                    tool_call_id,
                    "exec",
                    Some(command_display.clone()),
                )));
                if is_validation_command(&begin.command) {
                    self.validation_calls
                        .insert(begin.call_id.clone(), command_display.clone());
                    events.push(RuntimeEvent::ValidationStarted(ValidationStarted::new(
                        self.task_id,
                        command_display,
                    )));
                }
            }
            EventMsg::ExecCommandEnd(end) => {
                let tool_call_id = tool_call_id_from_string(&end.call_id);
                let command_display = command_display(&end.command);
                let output_preview = preview_text(&end.formatted_output);
                let status = end.status;
                let success = matches!(&status, ExecCommandStatus::Completed);
                events.push(RuntimeEvent::ToolCallFinished(ToolCallFinished::new(
                    self.task_id,
                    tool_call_id,
                    "exec",
                    output_preview.clone(),
                    success,
                )));
                self.push_evidence(format!(
                    "exec: {command_display} -> {}",
                    exec_status_label(&status)
                ));
                if let Some(validation_command) = self.validation_calls.remove(&end.call_id) {
                    events.push(RuntimeEvent::ValidationFinished(ValidationFinished::new(
                        self.task_id,
                        validation_command,
                        validation_status_from_exec_status(&status),
                        output_preview,
                    )));
                }
            }
            EventMsg::McpToolCallBegin(begin) => {
                let tool_call_id = tool_call_id_from_string(&begin.call_id);
                let preview = begin
                    .invocation
                    .arguments
                    .as_ref()
                    .and_then(|value| serde_json::to_string(value).ok())
                    .and_then(|text| preview_text(&text));
                events.push(RuntimeEvent::ToolCallStarted(ToolCallStarted::new(
                    self.task_id,
                    tool_call_id,
                    format!("{}/{}", begin.invocation.server, begin.invocation.tool),
                    preview,
                )));
            }
            EventMsg::McpToolCallEnd(end) => {
                let tool_call_id = tool_call_id_from_string(&end.call_id);
                let output_preview = end
                    .result
                    .as_ref()
                    .ok()
                    .and_then(|result| serde_json::to_string(result).ok())
                    .and_then(|text| preview_text(&text));
                let success = end.is_success();
                events.push(RuntimeEvent::ToolCallFinished(ToolCallFinished::new(
                    self.task_id,
                    tool_call_id,
                    format!("{}/{}", end.invocation.server, end.invocation.tool),
                    output_preview,
                    success,
                )));
                self.push_evidence(format!(
                    "mcp: {}/{} -> {}",
                    end.invocation.server,
                    end.invocation.tool,
                    if success { "completed" } else { "failed" }
                ));
            }
            EventMsg::WebSearchBegin(begin) => {
                let tool_call_id = tool_call_id_from_string(&begin.call_id);
                events.push(RuntimeEvent::ToolCallStarted(ToolCallStarted::new(
                    self.task_id,
                    tool_call_id,
                    "web_search",
                    None::<String>,
                )));
            }
            EventMsg::WebSearchEnd(end) => {
                let tool_call_id = tool_call_id_from_string(&end.call_id);
                let preview = Some(format!("{} [{:?}]", end.query, end.action));
                events.push(RuntimeEvent::ToolCallFinished(ToolCallFinished::new(
                    self.task_id,
                    tool_call_id,
                    "web_search",
                    preview,
                    true,
                )));
                self.push_evidence(format!("web search: {} [{:?}]", end.query, end.action));
            }
            EventMsg::ImageGenerationBegin(begin) => {
                let tool_call_id = tool_call_id_from_string(&begin.call_id);
                events.push(RuntimeEvent::ToolCallStarted(ToolCallStarted::new(
                    self.task_id,
                    tool_call_id,
                    "image_generation",
                    None::<String>,
                )));
            }
            EventMsg::ImageGenerationEnd(end) => {
                let tool_call_id = tool_call_id_from_string(&end.call_id);
                let preview = preview_text(&end.result);
                events.push(RuntimeEvent::ToolCallFinished(ToolCallFinished::new(
                    self.task_id,
                    tool_call_id,
                    "image_generation",
                    preview,
                    true,
                )));
                self.push_evidence(format!("image generation: {}", end.status));
            }
            EventMsg::PatchApplyBegin(begin) => {
                let tool_call_id = tool_call_id_from_string(&begin.call_id);
                let preview = Some(format!("{} file(s)", begin.changes.len()));
                events.push(RuntimeEvent::ToolCallStarted(ToolCallStarted::new(
                    self.task_id,
                    tool_call_id,
                    "apply_patch",
                    preview,
                )));
            }
            EventMsg::PatchApplyEnd(end) => {
                let tool_call_id = tool_call_id_from_string(&end.call_id);
                let preview = preview_text(&end.stdout).or_else(|| preview_text(&end.stderr));
                events.push(RuntimeEvent::ToolCallFinished(ToolCallFinished::new(
                    self.task_id,
                    tool_call_id,
                    "apply_patch",
                    preview,
                    end.success,
                )));
                self.push_evidence(format!(
                    "apply_patch: {}",
                    if end.success { "completed" } else { "failed" }
                ));
            }
            EventMsg::ExecApprovalRequest(request) => {
                events.extend(runtime_approval_events_for_exec_approval(
                    self.task_id,
                    request,
                ));
                terminate = true;
            }
            EventMsg::ApplyPatchApprovalRequest(request) => {
                events.extend(runtime_approval_events_for_patch(self.task_id, request));
                terminate = true;
            }
            EventMsg::RequestPermissions(request) => {
                events.extend(runtime_approval_events_for_permissions(
                    self.task_id,
                    request,
                ));
                terminate = true;
            }
            EventMsg::ElicitationRequest(request) => {
                events.extend(runtime_approval_events_for_elicitation(
                    self.task_id,
                    request,
                ));
                terminate = true;
            }
            EventMsg::TurnComplete(TurnCompleteEvent {
                last_agent_message, ..
            }) => {
                let summary = last_agent_message.clone().or_else(|| {
                    (!self.assistant_buffer.trim().is_empty())
                        .then(|| self.assistant_buffer.trim().to_string())
                });
                if !self.validation_calls.is_empty() {
                    let pending = std::mem::take(&mut self.validation_calls);
                    for command_display in pending.into_values() {
                        events.push(RuntimeEvent::ValidationFinished(ValidationFinished::new(
                            self.task_id,
                            command_display,
                            ValidationStatus::Cancelled,
                            Some(
                                "validation was not completed before the task finished".to_string(),
                            ),
                        )));
                    }
                }
                let mut completed_evidence = self.evidence.clone();
                if self.assistant_buffer_overflowed_bytes > 0 {
                    completed_evidence.push(format!(
                        "note: assistant_buffer truncated, {} bytes dropped from the head due to the {ASSISTANT_BUFFER_SOFT_CAP_BYTES}-byte soft cap",
                        self.assistant_buffer_overflowed_bytes
                    ));
                }
                if self.evidence_overflowed > 0 {
                    completed_evidence.push(format!(
                        "note: {} additional evidence entries dropped due to the {EVIDENCE_SOFT_CAP_ENTRIES}-entry soft cap",
                        self.evidence_overflowed
                    ));
                }
                events.push(RuntimeEvent::TaskCompleted(TaskCompleted::new(
                    self.task_id,
                    summary,
                    completed_evidence,
                )));
                self.final_message = events.iter().rev().find_map(|event| match event {
                    RuntimeEvent::TaskCompleted(completed) => completed.summary.clone(),
                    _ => None,
                });
                terminate = true;
            }
            EventMsg::TurnAborted(event) => {
                if !self.validation_calls.is_empty() {
                    let pending = std::mem::take(&mut self.validation_calls);
                    for command_display in pending.into_values() {
                        events.push(RuntimeEvent::ValidationFinished(ValidationFinished::new(
                            self.task_id,
                            command_display,
                            ValidationStatus::Cancelled,
                            Some("validation was cancelled".to_string()),
                        )));
                    }
                }
                match event.reason {
                    TurnAbortReason::Interrupted | TurnAbortReason::Replaced => {
                        events.push(RuntimeEvent::TaskCancelled(TaskCancelled::new(
                            self.task_id,
                            Some(format!("task {}", turn_abort_reason_label(event.reason))),
                        )));
                    }
                    TurnAbortReason::ReviewEnded => {
                        events.push(RuntimeEvent::TaskCancelled(TaskCancelled::new(
                            self.task_id,
                            Some("review ended".to_string()),
                        )));
                    }
                    TurnAbortReason::BudgetLimited => {
                        events.push(RuntimeEvent::TaskFailed(TaskFailed::new(
                            self.task_id,
                            RuntimeError::new(
                                RuntimeErrorCode::BudgetLimited,
                                "task stopped because the budget was exhausted",
                                Some("reduce prompt size or restart the task".to_string()),
                                false,
                                RuntimeErrorOwner::Provider,
                            ),
                        )));
                    }
                }
                terminate = true;
            }
            EventMsg::Error(error) => {
                let affects_turn_status = error
                    .vac_error_info
                    .as_ref()
                    .is_none_or(|info| info.affects_turn_status());
                if affects_turn_status {
                    self.push_evidence(format!("error: {}", error.message));
                }
                error_seen = affects_turn_status;
            }
            EventMsg::StreamError(StreamErrorEvent {
                message,
                vac_error_info,
                additional_details,
            }) => {
                let affects_turn_status = vac_error_info
                    .as_ref()
                    .is_none_or(|info| info.affects_turn_status());
                if affects_turn_status {
                    self.push_evidence(format!("stream error: {message}"));
                }
                error_seen |= affects_turn_status;
                if let Some(details) = additional_details {
                    self.push_evidence(details);
                }
            }
            EventMsg::UserMessage(UserMessageEvent { message, .. }) => {
                if self.final_message.is_none() && !message.trim().is_empty() {
                    self.final_message = Some(message);
                }
            }
            EventMsg::ItemStarted(ItemStartedEvent { item, .. }) => {
                if let TurnItem::Plan(plan) = item {
                    if !plan.text.trim().is_empty() {
                        self.append_assistant_text(&plan.text);
                        events.push(RuntimeEvent::AssistantDelta(AssistantDelta::new(
                            self.task_id,
                            plan.text,
                        )));
                    }
                }
            }
            EventMsg::ItemCompleted(ItemCompletedEvent { item, .. }) => {
                if let TurnItem::AgentMessage(message) = item
                    && let Some(text) = message
                        .content
                        .iter()
                        .filter_map(|content| match content {
                            vac_protocol::items::AgentMessageContent::Text { text } => {
                                Some(text.clone())
                            }
                        })
                        .next()
                    && !text.trim().is_empty()
                {
                    self.final_message = Some(text);
                }
            }
            EventMsg::RawResponseItem(RawResponseItemEvent { item }) => {
                if let vac_protocol::models::ResponseItem::Message { role, content, .. } = item
                    && role == "assistant"
                {
                    let text = content
                        .iter()
                        .filter_map(|content| match content {
                            ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                                Some(text.clone())
                            }
                            ContentItem::InputImage { .. } => None,
                        })
                        .collect::<Vec<_>>()
                        .join("");
                    if !text.trim().is_empty() {
                        self.final_message = Some(text);
                    }
                }
            }
            EventMsg::ShutdownComplete => {
                terminate = true;
            }
            EventMsg::EnteredReviewMode(review_request) => {
                let target = convert_protocol_review_target(review_request.target);
                events.push(RuntimeEvent::EnteredReviewMode(
                    super::EnteredReviewMode::new(target, review_request.user_facing_hint),
                ));
            }
            EventMsg::ExitedReviewMode(_) => {
                events.push(RuntimeEvent::ExitedReviewMode(
                    super::ExitedReviewMode::new(None::<String>),
                ));
            }
            _ => {}
        }

        BridgeOutput {
            events,
            error_seen,
            terminate,
        }
    }
}

pub fn runtime_approval_events_for_exec_approval(
    task_id: TaskId,
    request: ExecApprovalRequestEvent,
) -> Vec<RuntimeEvent> {
    let reason = request
        .reason
        .unwrap_or_else(|| DEFAULT_APPROVAL_HINT.to_string());
    let command_display = command_display(&request.command);
    let approval = ApprovalRequest {
        id: ApprovalId::new(),
        task_id,
        action: ApprovalAction::ExecuteProcess,
        risk: RiskLevel::Execute,
        reason,
        resources: vec![ApprovalResource::Command(command_display.clone())],
        preview: ApprovalPreview::Command(command_display),
        validation_after: Vec::new(),
    };
    let failure = TaskFailed::new(
        task_id,
        RuntimeError::new(
            RuntimeErrorCode::ApprovalRequired,
            "approval is required in exec mode",
            Some("run with bypass approvals only if you intentionally want to skip the approval gate".to_string()),
            false,
            RuntimeErrorOwner::Policy,
        ),
    );
    vec![
        RuntimeEvent::ApprovalRequested(Box::new(approval)),
        RuntimeEvent::TaskFailed(failure),
    ]
}

pub fn runtime_approval_events_for_patch(
    task_id: TaskId,
    request: ApplyPatchApprovalRequestEvent,
) -> Vec<RuntimeEvent> {
    let files = request.changes.keys().cloned().collect::<Vec<_>>();
    let preview = if files.is_empty() {
        ApprovalPreview::None
    } else {
        ApprovalPreview::FileList(files.clone())
    };
    let approval = ApprovalRequest::safe_edit(
        ApprovalId::new(),
        task_id,
        request
            .reason
            .unwrap_or_else(|| "patch approval required".to_string()),
        files.into_iter().map(ApprovalResource::File).collect(),
        preview,
        Vec::new(),
    );
    vec![
        RuntimeEvent::ApprovalRequested(Box::new(approval)),
        RuntimeEvent::TaskFailed(TaskFailed::new(
            task_id,
            RuntimeError::new(
                RuntimeErrorCode::ApprovalRequired,
                "patch approval is required in exec mode",
                Some("run with bypass approvals only if you intentionally want to skip the approval gate".to_string()),
                false,
                RuntimeErrorOwner::Policy,
            ),
        )),
    ]
}

pub fn runtime_approval_events_for_permissions(
    task_id: TaskId,
    request: RequestPermissionsEvent,
) -> Vec<RuntimeEvent> {
    let mut resources = Vec::new();
    if let Some(file_system) = request.permissions.file_system.as_ref() {
        resources.push(ApprovalResource::Other(format!(
            "file_system: {file_system:?}"
        )));
    }
    if let Some(network) = request.permissions.network.as_ref() {
        resources.push(ApprovalResource::Other(format!("network: {network:?}")));
    }
    if resources.is_empty() {
        resources.push(ApprovalResource::Other("permissions request".to_string()));
    }
    let approval = ApprovalRequest {
        id: ApprovalId::new(),
        task_id,
        action: ApprovalAction::Other,
        risk: RiskLevel::Unknown,
        reason: request
            .reason
            .unwrap_or_else(|| "permissions request".to_string()),
        resources,
        preview: ApprovalPreview::Text("permissions request".to_string()),
        validation_after: Vec::new(),
    };
    vec![
        RuntimeEvent::ApprovalRequested(Box::new(approval)),
        RuntimeEvent::TaskFailed(TaskFailed::new(
            task_id,
            RuntimeError::new(
                RuntimeErrorCode::ApprovalRequired,
                "permissions approval is required in exec mode",
                Some("run with bypass approvals only if you intentionally want to skip the approval gate".to_string()),
                false,
                RuntimeErrorOwner::Policy,
            ),
        )),
    ]
}

pub fn runtime_approval_events_for_elicitation(
    task_id: TaskId,
    request: ElicitationRequestEvent,
) -> Vec<RuntimeEvent> {
    let approval = ApprovalRequest {
        id: ApprovalId::new(),
        task_id,
        action: ApprovalAction::ConnectorCall,
        risk: RiskLevel::Credential,
        reason: request.request.message().to_string(),
        resources: vec![ApprovalResource::Other(format!(
            "elicitation: {}",
            request.server_name
        ))],
        preview: ApprovalPreview::Text(request.request.message().to_string()),
        validation_after: Vec::new(),
    };
    vec![
        RuntimeEvent::ApprovalRequested(Box::new(approval)),
        RuntimeEvent::TaskFailed(TaskFailed::new(
            task_id,
            RuntimeError::new(
                RuntimeErrorCode::ApprovalRequired,
                "elicitation is required in exec mode",
                Some("run with bypass approvals only if you intentionally want to skip the approval gate".to_string()),
                false,
                RuntimeErrorOwner::Policy,
            ),
        )),
    ]
}

fn event_turn_id(event: &EventMsg) -> Option<&str> {
    match event {
        EventMsg::TurnStarted(TurnStartedEvent { turn_id, .. })
        | EventMsg::TurnComplete(TurnCompleteEvent { turn_id, .. }) => Some(turn_id),
        EventMsg::AgentMessageContentDelta(delta) => Some(delta.turn_id.as_str()),
        EventMsg::PlanDelta(delta) => Some(delta.turn_id.as_str()),
        EventMsg::ReasoningContentDelta(delta) => Some(delta.turn_id.as_str()),
        EventMsg::ReasoningRawContentDelta(delta) => Some(delta.turn_id.as_str()),
        EventMsg::ExecCommandBegin(begin) => Some(begin.turn_id.as_str()),
        EventMsg::ExecCommandEnd(end) => Some(end.turn_id.as_str()),
        EventMsg::PatchApplyBegin(begin) => Some(begin.turn_id.as_str()),
        EventMsg::PatchApplyEnd(end) => Some(end.turn_id.as_str()),
        EventMsg::ExecApprovalRequest(request) => Some(request.turn_id.as_str()),
        EventMsg::ApplyPatchApprovalRequest(request) => Some(request.turn_id.as_str()),
        EventMsg::RequestPermissions(request) => Some(request.turn_id.as_str()),
        EventMsg::ElicitationRequest(request) => request.turn_id.as_deref(),
        EventMsg::ItemStarted(item) => Some(item.turn_id.as_str()),
        EventMsg::ItemCompleted(item) => Some(item.turn_id.as_str()),
        EventMsg::TurnAborted(event) => event.turn_id.as_deref(),
        _ => None,
    }
}

fn tool_call_id_from_string(value: &str) -> ToolCallId {
    ToolCallId::from_string(value).unwrap_or_default()
}

fn command_display(command: &[String]) -> String {
    if command.is_empty() {
        return "<empty command>".to_string();
    }
    command.join(" ")
}

fn preview_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    const MAX_PREVIEW_CHARS: usize = 240;
    if trimmed.chars().count() <= MAX_PREVIEW_CHARS {
        return Some(trimmed.to_string());
    }
    let mut preview = trimmed.chars().take(MAX_PREVIEW_CHARS).collect::<String>();
    preview.push_str("...");
    Some(preview)
}

fn is_validation_command(command: &[String]) -> bool {
    let display = command
        .iter()
        .map(|part| part.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    [
        "cargo test",
        "cargo check",
        "cargo fmt",
        "cargo clippy",
        "npm test",
        "pnpm test",
        "bun test",
        "pytest",
        "go test",
        "just test",
        "make test",
        "deno test",
    ]
    .iter()
    .any(|needle| display.contains(needle))
}

fn validation_status_from_exec_status(status: &ExecCommandStatus) -> ValidationStatus {
    match status {
        ExecCommandStatus::Completed => ValidationStatus::Passed,
        ExecCommandStatus::Failed => ValidationStatus::Failed,
        ExecCommandStatus::Declined => ValidationStatus::Cancelled,
    }
}

fn exec_status_label(status: &ExecCommandStatus) -> &'static str {
    match status {
        ExecCommandStatus::Completed => "completed",
        ExecCommandStatus::Failed => "failed",
        ExecCommandStatus::Declined => "declined",
    }
}

fn turn_abort_reason_label(reason: TurnAbortReason) -> &'static str {
    match reason {
        TurnAbortReason::Interrupted => "interrupted",
        TurnAbortReason::Replaced => "replaced",
        TurnAbortReason::ReviewEnded => "review ended",
        TurnAbortReason::BudgetLimited => "budget limited",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use vac_protocol::parse_command::ParsedCommand;
    use vac_protocol::protocol::AgentMessageContentDeltaEvent;
    use vac_protocol::protocol::Event;
    use vac_protocol::protocol::EventMsg;
    use vac_protocol::protocol::ExecCommandBeginEvent;
    use vac_protocol::protocol::ExecCommandEndEvent;
    use vac_protocol::protocol::ExecCommandSource;
    use vac_protocol::protocol::PlanDeltaEvent;
    use vac_protocol::protocol::ReasoningContentDeltaEvent;
    use vac_protocol::protocol::ReasoningRawContentDeltaEvent;
    use vac_protocol::protocol::TurnCompleteEvent;
    use vac_protocol::protocol::TurnStartedEvent;
    use vac_utils_absolute_path::AbsolutePathBuf;

    fn bridge() -> LocalRuntimeBridge {
        LocalRuntimeBridge::new(
            SessionId::from_string("11111111-1111-1111-1111-111111111111")
                .expect("valid session id"),
            TaskId::from_string("22222222-2222-2222-2222-222222222222").expect("valid task id"),
            "implement the change".to_string(),
        )
    }

    fn event(msg: EventMsg) -> Event {
        Event {
            id: "submission-1".to_string(),
            msg,
        }
    }

    fn start_turn(bridge: &mut LocalRuntimeBridge, turn_id: &str) -> BridgeOutput {
        bridge.map_core_event(event(EventMsg::TurnStarted(TurnStartedEvent {
            turn_id: turn_id.to_string(),
            started_at: None,
            model_context_window: None,
            collaboration_mode_kind: Default::default(),
        })))
    }

    fn cwd() -> AbsolutePathBuf {
        AbsolutePathBuf::from_absolute_path("/tmp").expect("absolute cwd")
    }

    fn exec_begin(call_id: &str, turn_id: &str, command: Vec<&str>) -> ExecCommandBeginEvent {
        ExecCommandBeginEvent {
            call_id: call_id.to_string(),
            process_id: None,
            turn_id: turn_id.to_string(),
            command: command.into_iter().map(str::to_string).collect(),
            cwd: cwd(),
            parsed_cmd: Vec::<ParsedCommand>::new(),
            source: ExecCommandSource::Agent,
            interaction_input: None,
        }
    }

    fn exec_end(
        call_id: &str,
        turn_id: &str,
        command: Vec<&str>,
        status: ExecCommandStatus,
        formatted_output: &str,
    ) -> ExecCommandEndEvent {
        ExecCommandEndEvent {
            call_id: call_id.to_string(),
            process_id: None,
            turn_id: turn_id.to_string(),
            command: command.into_iter().map(str::to_string).collect(),
            cwd: cwd(),
            parsed_cmd: Vec::<ParsedCommand>::new(),
            source: ExecCommandSource::Agent,
            interaction_input: None,
            stdout: String::new(),
            stderr: String::new(),
            aggregated_output: String::new(),
            exit_code: 0,
            duration: Duration::from_millis(7),
            formatted_output: formatted_output.to_string(),
            status,
        }
    }

    #[test]
    fn validation_command_detection_matches_common_toolchains() {
        assert!(is_validation_command(&[
            "cargo".to_string(),
            "test".to_string()
        ]));
        assert!(is_validation_command(&[
            "cargo".to_string(),
            "check".to_string()
        ]));
        assert!(is_validation_command(&["pytest".to_string()]));
        assert!(!is_validation_command(&[
            "cargo".to_string(),
            "run".to_string()
        ]));
    }

    #[test]
    fn preview_text_truncates_long_output() {
        let preview = preview_text(&"a".repeat(300)).expect("preview should exist");
        assert!(preview.ends_with("..."));
    }

    #[test]
    fn bridge_projects_assistant_plan_and_reasoning_deltas() {
        let mut bridge = bridge();
        start_turn(&mut bridge, "turn-1");

        let cases = [
            EventMsg::AgentMessageContentDelta(AgentMessageContentDeltaEvent {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "item-agent".to_string(),
                delta: "assistant ".to_string(),
            }),
            EventMsg::PlanDelta(PlanDeltaEvent {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "item-plan".to_string(),
                delta: "plan ".to_string(),
            }),
            EventMsg::ReasoningContentDelta(ReasoningContentDeltaEvent {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "item-reasoning".to_string(),
                delta: "reasoning ".to_string(),
                summary_index: 0,
            }),
            EventMsg::ReasoningRawContentDelta(ReasoningRawContentDeltaEvent {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "item-raw-reasoning".to_string(),
                delta: "raw".to_string(),
                content_index: 0,
            }),
        ];

        let projected = cases
            .into_iter()
            .flat_map(|msg| bridge.map_core_event(event(msg)).events)
            .filter_map(|runtime_event| match runtime_event {
                RuntimeEvent::AssistantDelta(delta) => Some(delta.text),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(projected, vec!["assistant ", "plan ", "reasoning ", "raw"]);
    }

    #[test]
    fn bridge_projects_tool_validation_completion_and_failure_semantics() {
        let mut bridge = bridge();
        start_turn(&mut bridge, "turn-1");

        let begin = bridge.map_core_event(event(EventMsg::ExecCommandBegin(exec_begin(
            "33333333-3333-3333-3333-333333333333",
            "turn-1",
            vec!["cargo", "test"],
        ))));
        assert!(matches!(begin.events.as_slice(), [
            RuntimeEvent::ToolCallStarted(ToolCallStarted { tool_name, .. }),
            RuntimeEvent::ValidationStarted(ValidationStarted { command_display, .. })
        ] if tool_name == "exec" && command_display == "cargo test"));

        let end = bridge.map_core_event(event(EventMsg::ExecCommandEnd(exec_end(
            "33333333-3333-3333-3333-333333333333",
            "turn-1",
            vec!["cargo", "test"],
            ExecCommandStatus::Failed,
            "test failed",
        ))));
        assert!(matches!(end.events.as_slice(), [
            RuntimeEvent::ToolCallFinished(ToolCallFinished { success: false, output_preview, .. }),
            RuntimeEvent::ValidationFinished(ValidationFinished { status: ValidationStatus::Failed, summary, .. })
        ] if output_preview.as_deref() == Some("test failed") && summary.as_deref() == Some("test failed")));
    }

    #[test]
    fn bridge_filters_stale_turn_events_after_turn_start() {
        let mut bridge = bridge();
        start_turn(&mut bridge, "turn-1");

        let output = bridge.map_core_event(event(EventMsg::AgentMessageContentDelta(
            AgentMessageContentDeltaEvent {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-2".to_string(),
                item_id: "stale-item".to_string(),
                delta: "must not leak".to_string(),
            },
        )));

        assert!(output.events.is_empty());
        assert!(!output.error_seen);
        assert!(!output.terminate);
    }

    #[test]
    fn bridge_caps_assistant_buffer_and_evidence() {
        let mut bridge = bridge();
        start_turn(&mut bridge, "turn-1");

        let oversized = format!("{}tail", "x".repeat(ASSISTANT_BUFFER_SOFT_CAP_BYTES + 128));
        bridge.map_core_event(event(EventMsg::AgentMessageContentDelta(
            AgentMessageContentDeltaEvent {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "large-assistant".to_string(),
                delta: oversized,
            },
        )));
        for index in 0..(EVIDENCE_SOFT_CAP_ENTRIES + 3) {
            bridge.map_core_event(event(EventMsg::ExecCommandEnd(exec_end(
                &format!("call-{index}"),
                "turn-1",
                vec!["echo", "ok"],
                ExecCommandStatus::Completed,
                "ok",
            ))));
        }

        let completed = bridge.map_core_event(event(EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id: "turn-1".to_string(),
            last_agent_message: None,
            completed_at: None,
            duration_ms: None,
            time_to_first_token_ms: None,
        })));

        let RuntimeEvent::TaskCompleted(task_completed) = completed
            .events
            .iter()
            .find(|event| matches!(event, RuntimeEvent::TaskCompleted(_)))
            .expect("task should complete")
        else {
            panic!("expected task completed event");
        };
        assert!(
            task_completed
                .summary
                .as_deref()
                .is_some_and(|text| text.ends_with("tail"))
        );
        assert!(
            task_completed
                .evidence
                .iter()
                .any(|entry| entry.contains("assistant_buffer truncated"))
        );
        assert!(
            task_completed
                .evidence
                .iter()
                .any(|entry| entry.contains("additional evidence entries dropped"))
        );
    }
}

fn convert_protocol_review_target(
    target: vac_protocol::protocol::ReviewTarget,
) -> super::ReviewTarget {
    target
}
