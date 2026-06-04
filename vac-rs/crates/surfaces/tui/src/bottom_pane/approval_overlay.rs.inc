// Approval modal rendering and decision routing for high-risk operations.
//
// This module converts agent approval requests (exec/apply-patch/MCP
// elicitation) into a list-selection view with action-specific options and
// shortcuts. It owns two important contracts:
//
// 1. Selection always emits an explicit decision event back to the app.
// 2. MCP elicitation keeps `Esc` mapped to `Cancel`, even with custom
//    keybindings, so dismissal never silently becomes "continue without info".
//
// This module does not evaluate whether an action is safe to run; it only
// presents choices and routes user decisions.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::app::app_server_requests::ResolvedAppServerRequest;
#[cfg(test)]
use crate::app_command::AppCommand as Op;
use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;
use crate::bottom_pane::BottomPaneView;
use crate::bottom_pane::CancellationEvent;
use crate::bottom_pane::list_selection_view::ListSelectionView;
use crate::bottom_pane::list_selection_view::SelectionItem;
use crate::bottom_pane::list_selection_view::SelectionViewParams;
use crate::bottom_pane::popup_consts::accept_cancel_hint_line;
use crate::diff_model::FileChange;
use crate::exec_command::strip_bash_lc_and_escape;
use crate::history_cell;
use crate::history_cell::ReviewDecision;
use crate::key_hint;
use crate::key_hint::KeyBinding;
use crate::key_hint::KeyBindingListExt;
use crate::keymap::ApprovalKeymap;
use crate::keymap::ListKeymap;
use crate::keymap::primary_binding;
use crate::render::highlight::highlight_bash_to_lines;
use crate::render::renderable::ColumnRenderable;
use crate::render::renderable::Renderable;
use crate::session_protocol::AdditionalPermissionProfile;
use crate::session_protocol::CommandExecutionApprovalDecision;
use crate::session_protocol::FileChangeApprovalDecision;
use crate::session_protocol::McpServerElicitationAction;
use crate::session_protocol::NetworkApprovalContext;
use crate::session_protocol::NetworkPolicyRuleAction;
use crate::session_protocol::RequestId;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;
use vac_features::Features;
use vac_protocol::ThreadId;
use vac_protocol::permissions::FileSystemAccessMode;
use vac_protocol::permissions::FileSystemPath;
use vac_protocol::permissions::FileSystemSandboxEntry;
use vac_protocol::permissions::FileSystemSpecialPath;
use vac_protocol::request_permissions::PermissionGrantScope;
use vac_protocol::request_permissions::RequestPermissionProfile;
use vac_utils_absolute_path::AbsolutePathBuf;

/// Request coming from the agent that needs user approval.
#[derive(Clone, Debug)]
pub(crate) enum ApprovalRequest {
    Exec {
        thread_id: ThreadId,
        thread_label: Option<String>,
        id: String,
        command: Vec<String>,
        cwd: AbsolutePathBuf,
        reason: Option<String>,
        available_decisions: Vec<CommandExecutionApprovalDecision>,
        network_approval_context: Option<NetworkApprovalContext>,
        additional_permissions: Option<AdditionalPermissionProfile>,
    },
    Permissions {
        thread_id: ThreadId,
        thread_label: Option<String>,
        call_id: String,
        reason: Option<String>,
        permissions: RequestPermissionProfile,
    },
    ApplyPatch {
        thread_id: ThreadId,
        thread_label: Option<String>,
        id: String,
        reason: Option<String>,
        cwd: AbsolutePathBuf,
        changes: HashMap<PathBuf, FileChange>,
    },
    McpElicitation {
        thread_id: ThreadId,
        thread_label: Option<String>,
        server_name: String,
        request_id: RequestId,
        message: String,
    },
}

impl ApprovalRequest {
    fn thread_id(&self) -> ThreadId {
        match self {
            ApprovalRequest::Exec { thread_id, .. }
            | ApprovalRequest::Permissions { thread_id, .. }
            | ApprovalRequest::ApplyPatch { thread_id, .. }
            | ApprovalRequest::McpElicitation { thread_id, .. } => *thread_id,
        }
    }

    fn thread_label(&self) -> Option<&str> {
        match self {
            ApprovalRequest::Exec { thread_label, .. }
            | ApprovalRequest::Permissions { thread_label, .. }
            | ApprovalRequest::ApplyPatch { thread_label, .. }
            | ApprovalRequest::McpElicitation { thread_label, .. } => thread_label.as_deref(),
        }
    }

    pub(super) fn matches_resolved_request(&self, request: &ResolvedAppServerRequest) -> bool {
        match (self, request) {
            (
                ApprovalRequest::Exec { id, .. },
                ResolvedAppServerRequest::ExecApproval { id: resolved_id },
            ) => id == resolved_id,
            (
                ApprovalRequest::Permissions { call_id, .. },
                ResolvedAppServerRequest::PermissionsApproval { id },
            ) => call_id == id,
            (
                ApprovalRequest::ApplyPatch { id, .. },
                ResolvedAppServerRequest::FileChangeApproval { id: resolved_id },
            ) => id == resolved_id,
            (
                ApprovalRequest::McpElicitation {
                    server_name,
                    request_id,
                    ..
                },
                ResolvedAppServerRequest::McpElicitation {
                    server_name: resolved_server_name,
                    request_id: resolved_request_id,
                },
            ) => server_name == resolved_server_name && request_id == resolved_request_id,
            _ => false,
        }
    }
}

/// Modal overlay asking the user to approve or deny one or more requests.
pub(crate) struct ApprovalOverlay {
    current_request: Option<ApprovalRequest>,
    queue: Vec<ApprovalRequest>,
    app_event_tx: AppEventSender,
    list: ListSelectionView,
    options: Vec<ApprovalOption>,
    current_complete: bool,
    done: bool,
    features: Features,
    approval_keymap: ApprovalKeymap,
    list_keymap: ListKeymap,
}

impl ApprovalOverlay {
    pub fn new(
        request: ApprovalRequest,
        app_event_tx: AppEventSender,
        features: Features,
        approval_keymap: ApprovalKeymap,
        list_keymap: ListKeymap,
    ) -> Self {
        let mut view = Self {
            current_request: None,
            queue: Vec::new(),
            app_event_tx: app_event_tx.clone(),
            list: ListSelectionView::new(Default::default(), app_event_tx, list_keymap.clone()),
            options: Vec::new(),
            current_complete: false,
            done: false,
            features,
            approval_keymap,
            list_keymap,
        };
        view.set_current(request);
        view
    }

    pub fn enqueue_request(&mut self, req: ApprovalRequest) {
        self.queue.push(req);
    }

    fn dismiss_resolved_request(&mut self, request: &ResolvedAppServerRequest) -> bool {
        let queue_len = self.queue.len();
        self.queue
            .retain(|queued_request| !queued_request.matches_resolved_request(request));
        if self
            .current_request
            .as_ref()
            .is_some_and(|current_request| current_request.matches_resolved_request(request))
        {
            self.current_complete = true;
            self.advance_queue();
            return true;
        }

        self.queue.len() != queue_len
    }

    fn set_current(&mut self, request: ApprovalRequest) {
        self.current_complete = false;
        let header = build_header(&request);
        let (options, params) = Self::build_options(
            &request,
            header,
            &self.features,
            &self.approval_keymap,
            &self.list_keymap,
        );
        self.current_request = Some(request);
        self.options = options;
        self.list =
            ListSelectionView::new(params, self.app_event_tx.clone(), self.list_keymap.clone());
    }

    fn build_options(
        request: &ApprovalRequest,
        header: Box<dyn Renderable>,
        _features: &Features,
        approval_keymap: &ApprovalKeymap,
        list_keymap: &ListKeymap,
    ) -> (Vec<ApprovalOption>, SelectionViewParams) {
        let (options, title) = match request {
            ApprovalRequest::Exec {
                available_decisions,
                network_approval_context,
                additional_permissions,
                ..
            } => (
                exec_options(
                    available_decisions,
                    network_approval_context.as_ref(),
                    additional_permissions.as_ref(),
                    approval_keymap,
                ),
                "approval required".to_string(),
            ),
            ApprovalRequest::Permissions { .. } => (
                permissions_options(approval_keymap),
                "approval required".to_string(),
            ),
            ApprovalRequest::ApplyPatch { .. } => (
                patch_options(approval_keymap),
                "approval required".to_string(),
            ),
            ApprovalRequest::McpElicitation { server_name, .. } => (
                elicitation_options(approval_keymap),
                format!("approval required · {server_name}"),
            ),
        };

        let header = Box::new(ColumnRenderable::with([
            Line::from(title.bold()).into(),
            Line::from("").into(),
            header,
        ]));

        let items = options
            .iter()
            .map(|opt| SelectionItem {
                name: opt.label.clone(),
                display_shortcut: opt.shortcuts.first().copied(),
                dismiss_on_select: false,
                ..Default::default()
            })
            .collect();

        let params = SelectionViewParams {
            footer_hint: Some(approval_footer_hint(request, approval_keymap, list_keymap)),
            items,
            header,
            ..Default::default()
        };

        (options, params)
    }

    fn apply_selection(&mut self, actual_idx: usize) {
        if self.current_complete {
            return;
        }
        let Some(option) = self.options.get(actual_idx) else {
            return;
        };
        if let Some(request) = self.current_request.as_ref() {
            match (request, &option.decision) {
                (
                    ApprovalRequest::Exec { id, command, .. },
                    ApprovalDecision::Command(decision),
                ) => {
                    self.handle_exec_decision(id, command, decision.clone());
                }
                (
                    ApprovalRequest::Permissions {
                        call_id,
                        permissions,
                        ..
                    },
                    ApprovalDecision::Permissions(decision),
                ) => self.handle_permissions_decision(call_id, permissions, *decision),
                (
                    ApprovalRequest::ApplyPatch { id, .. },
                    ApprovalDecision::FileChange(decision),
                ) => {
                    self.handle_patch_decision(id, decision.clone());
                }
                (
                    ApprovalRequest::McpElicitation {
                        server_name,
                        request_id,
                        ..
                    },
                    ApprovalDecision::McpElicitation(decision),
                ) => {
                    self.handle_elicitation_decision(server_name, request_id, *decision);
                }
                _ => {}
            }
        }

        self.current_complete = true;
        self.advance_queue();
    }

    fn handle_exec_decision(
        &self,
        id: &str,
        command: &[String],
        decision: CommandExecutionApprovalDecision,
    ) {
        let Some(request) = self.current_request.as_ref() else {
            return;
        };
        if request.thread_label().is_none() {
            let cell = history_cell::new_approval_decision_cell(
                command.to_vec(),
                command_decision_to_review_decision(&decision),
                history_cell::ApprovalDecisionActor::User,
            );
            self.app_event_tx.send(AppEvent::InsertHistoryCell(cell));
        }
        // Local Runtime Contract mirror: emit RuntimeCommand::Approve/Reject
        // and a visible `vac runtime: approval ...` history row alongside
        // the existing legacy submit. The legacy `exec_approval` call
        // still drives execution; this is additive evidence so operators
        // can see contract decisions in the root `vac` history.
        let (runtime_cmd, runtime_event, _correlation) =
            crate::runtime_adapter::approval_resolved_for_exec_decision(id, &decision);
        tracing::info!(
            target: "vac::runtime",
            "approval resolved (exec): {:?}",
            runtime_cmd
        );
        // 00D-7: runtime_history_label now returns Option<String> so the
        // activity-mode renderer can suppress noisy events. ApprovalResolved
        // is always Some in both modes; this guard keeps the call shape
        // generic in case suppression rules expand later.
        if let Some(runtime_label) = crate::runtime_adapter::runtime_history_label(&runtime_event) {
            self.app_event_tx.send(AppEvent::InsertHistoryCell(Box::new(
                crate::history_cell::PlainHistoryCell::new(vec![runtime_label.into()]),
            )));
        }
        let thread_id = request.thread_id();
        self.app_event_tx
            .exec_approval(thread_id, id.to_string(), decision);
    }

    fn handle_permissions_decision(
        &self,
        call_id: &str,
        permissions: &RequestPermissionProfile,
        decision: PermissionsDecision,
    ) {
        let Some(request) = self.current_request.as_ref() else {
            return;
        };
        let granted_permissions = match decision {
            PermissionsDecision::GrantForTurn
            | PermissionsDecision::GrantForTurnWithStrictAutoReview
            | PermissionsDecision::GrantForSession => permissions.clone(),
            PermissionsDecision::Deny => Default::default(),
        };
        let scope = if matches!(decision, PermissionsDecision::GrantForSession) {
            PermissionGrantScope::Session
        } else {
            PermissionGrantScope::Turn
        };
        let strict_auto_review = matches!(
            decision,
            PermissionsDecision::GrantForTurnWithStrictAutoReview
        );
        if request.thread_label().is_none() {
            let message = if granted_permissions.is_empty() {
                "You did not grant additional permissions"
            } else if strict_auto_review {
                "You granted additional permissions with strict auto review"
            } else if matches!(scope, PermissionGrantScope::Session) {
                "You granted additional permissions for this session"
            } else {
                "You granted additional permissions"
            };
            self.app_event_tx.send(AppEvent::InsertHistoryCell(Box::new(
                crate::history_cell::PlainHistoryCell::new(vec![message.into()]),
            )));
        }
        let thread_id = request.thread_id();

        // Local Runtime Contract mirror: emit RuntimeCommand::Approve/Reject
        // and a visible `vac runtime: approval ...` history row alongside
        // the existing legacy permissions response. Keyed on `call_id` to
        // correlate with the projected `ApprovalRequested.id`.
        let approval_decision = match decision {
            PermissionsDecision::GrantForTurn
            | PermissionsDecision::GrantForTurnWithStrictAutoReview
            | PermissionsDecision::GrantForSession => {
                vac_core::local_runtime::ApprovalDecision::Approved
            }
            PermissionsDecision::Deny => vac_core::local_runtime::ApprovalDecision::Rejected,
        };
        let (runtime_cmd, runtime_event, _correlation) =
            crate::runtime_adapter::approval_resolved_for_permissions_decision(
                call_id,
                approval_decision,
            );
        tracing::info!(
            target: "vac::runtime",
            "approval resolved (permissions): {:?}",
            runtime_cmd
        );
        // 00D-7: runtime_history_label now returns Option<String> so the
        // activity-mode renderer can suppress noisy events. ApprovalResolved
        // is always Some in both modes; this guard keeps the call shape
        // generic in case suppression rules expand later.
        if let Some(runtime_label) = crate::runtime_adapter::runtime_history_label(&runtime_event) {
            self.app_event_tx.send(AppEvent::InsertHistoryCell(Box::new(
                crate::history_cell::PlainHistoryCell::new(vec![runtime_label.into()]),
            )));
        }

        self.app_event_tx.request_permissions_response(
            thread_id,
            call_id.to_string(),
            vac_protocol::request_permissions::RequestPermissionsResponse {
                permissions: granted_permissions,
                scope,
                strict_auto_review,
            },
        );
    }

    fn handle_patch_decision(&self, id: &str, decision: FileChangeApprovalDecision) {
        let Some(thread_id) = self
            .current_request
            .as_ref()
            .map(ApprovalRequest::thread_id)
        else {
            return;
        };
        // Local Runtime Contract mirror — see `handle_exec_decision`.
        let (runtime_cmd, runtime_event, _correlation) =
            crate::runtime_adapter::approval_resolved_for_patch_decision(id, &decision);
        tracing::info!(
            target: "vac::runtime",
            "approval resolved (patch): {:?}",
            runtime_cmd
        );
        // 00D-7: runtime_history_label now returns Option<String> so the
        // activity-mode renderer can suppress noisy events. ApprovalResolved
        // is always Some in both modes; this guard keeps the call shape
        // generic in case suppression rules expand later.
        if let Some(runtime_label) = crate::runtime_adapter::runtime_history_label(&runtime_event) {
            self.app_event_tx.send(AppEvent::InsertHistoryCell(Box::new(
                crate::history_cell::PlainHistoryCell::new(vec![runtime_label.into()]),
            )));
        }
        self.app_event_tx
            .patch_approval(thread_id, id.to_string(), decision);
    }

    fn handle_elicitation_decision(
        &self,
        server_name: &str,
        request_id: &RequestId,
        decision: McpServerElicitationAction,
    ) {
        let Some(thread_id) = self
            .current_request
            .as_ref()
            .map(ApprovalRequest::thread_id)
        else {
            return;
        };

        // Local Runtime Contract mirror — see `handle_exec_decision`. Key the
        // resolution on the protocol `request_id` stringified via `Display`,
        // which matches the chatwidget-side projection key.
        let approval_decision = match decision {
            McpServerElicitationAction::Accept => {
                vac_core::local_runtime::ApprovalDecision::Approved
            }
            McpServerElicitationAction::Decline | McpServerElicitationAction::Cancel => {
                vac_core::local_runtime::ApprovalDecision::Rejected
            }
        };
        let request_key = request_id.to_string();
        let (runtime_cmd, runtime_event, _correlation) =
            crate::runtime_adapter::approval_resolved_for_elicitation_decision(
                &request_key,
                approval_decision,
            );
        tracing::info!(
            target: "vac::runtime",
            "approval resolved (mcp_elicitation): {:?}",
            runtime_cmd
        );
        // 00D-7: runtime_history_label now returns Option<String> so the
        // activity-mode renderer can suppress noisy events. ApprovalResolved
        // is always Some in both modes; this guard keeps the call shape
        // generic in case suppression rules expand later.
        if let Some(runtime_label) = crate::runtime_adapter::runtime_history_label(&runtime_event) {
            self.app_event_tx.send(AppEvent::InsertHistoryCell(Box::new(
                crate::history_cell::PlainHistoryCell::new(vec![runtime_label.into()]),
            )));
        }

        self.app_event_tx.resolve_elicitation(
            thread_id,
            server_name.to_string(),
            request_id.clone(),
            decision,
            /*content*/ None,
            /*meta*/ None,
        );
    }

    fn advance_queue(&mut self) {
        if let Some(next) = self.queue.pop() {
            self.set_current(next);
        } else {
            self.done = true;
        }
    }

    fn cancel_current_request(&mut self) {
        if self.done {
            return;
        }
        if !self.current_complete
            && let Some(request) = self.current_request.as_ref()
        {
            match request {
                ApprovalRequest::Exec { id, command, .. } => {
                    self.handle_exec_decision(
                        id,
                        command,
                        CommandExecutionApprovalDecision::Cancel,
                    );
                }
                ApprovalRequest::Permissions {
                    call_id,
                    permissions,
                    ..
                } => {
                    self.handle_permissions_decision(
                        call_id,
                        permissions,
                        PermissionsDecision::Deny,
                    );
                }
                ApprovalRequest::ApplyPatch { id, .. } => {
                    self.handle_patch_decision(id, FileChangeApprovalDecision::Cancel);
                }
                ApprovalRequest::McpElicitation {
                    server_name,
                    request_id,
                    ..
                } => {
                    self.handle_elicitation_decision(
                        server_name,
                        request_id,
                        McpServerElicitationAction::Cancel,
                    );
                }
            }
        }
        self.queue.clear();
        self.done = true;
    }

    /// Apply approval-specific shortcuts before delegating to list navigation.
    ///
    /// `open_fullscreen` is handled here because it is orthogonal to list item
    /// selection and should work regardless of current highlighted row.
    fn try_handle_shortcut(&mut self, key_event: &KeyEvent) -> bool {
        if key_event.kind == KeyEventKind::Press
            && self.approval_keymap.open_fullscreen.is_pressed(*key_event)
            && let Some(request) = self.current_request.as_ref()
        {
            self.app_event_tx
                .send(AppEvent::FullScreenApprovalRequest(request.clone()));
            return true;
        }

        if key_event.kind == KeyEventKind::Press
            && self.approval_keymap.open_thread.is_pressed(*key_event)
            && let Some(request) = self.current_request.as_ref()
            && request.thread_label().is_some()
        {
            self.app_event_tx
                .send(AppEvent::SelectAgentThread(request.thread_id()));
            return true;
        }

        if self.list_keymap.cancel.is_pressed(*key_event) {
            self.cancel_current_request();
            return true;
        }

        if let Some(idx) = self
            .options
            .iter()
            .position(|opt| opt.shortcuts.iter().any(|s| s.is_press(*key_event)))
        {
            self.apply_selection(idx);
            true
        } else {
            false
        }
    }
}

impl BottomPaneView for ApprovalOverlay {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if self.try_handle_shortcut(&key_event) {
            return;
        }
        self.list.handle_key_event(key_event);
        if let Some(idx) = self.list.take_last_selected_index() {
            self.apply_selection(idx);
        }
    }

    fn on_ctrl_c(&mut self) -> CancellationEvent {
        self.cancel_current_request();
        CancellationEvent::Handled
    }

    fn is_complete(&self) -> bool {
        self.done
    }

    fn try_consume_approval_request(
        &mut self,
        request: ApprovalRequest,
    ) -> Option<ApprovalRequest> {
        self.enqueue_request(request);
        None
    }

    fn dismiss_app_server_request(&mut self, request: &ResolvedAppServerRequest) -> bool {
        self.dismiss_resolved_request(request)
    }

    fn terminal_title_requires_action(&self) -> bool {
        true
    }
}

impl Renderable for ApprovalOverlay {
    fn desired_height(&self, width: u16) -> u16 {
        self.list.desired_height(width)
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.list.render(area, buf);
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.list.cursor_pos(area)
    }
}

fn approval_footer_hint(
    request: &ApprovalRequest,
    approval_keymap: &ApprovalKeymap,
    list_keymap: &ListKeymap,
) -> Line<'static> {
    let mut spans = accept_cancel_hint_line(
        primary_binding(&list_keymap.accept),
        "to confirm",
        primary_binding(&list_keymap.cancel),
        "to cancel",
    )
    .spans;
    if request.thread_label().is_some()
        && let Some(open_thread) = primary_binding(&approval_keymap.open_thread)
    {
        if !spans.is_empty() {
            spans.push(" or ".into());
        } else {
            spans.push("Press ".into());
        }
        spans.extend([open_thread.into(), " to open thread".into()]);
    }
    Line::from(spans)
}

fn build_header(request: &ApprovalRequest) -> Box<dyn Renderable> {
    match request {
        ApprovalRequest::Exec {
            thread_label,
            reason,
            command,
            cwd,
            network_approval_context,
            additional_permissions,
            ..
        } => {
            let mut header: Vec<Line<'static>> = Vec::new();
            if let Some(thread_label) = thread_label {
                header.push(Line::from(vec![
                    "Thread: ".into(),
                    thread_label.clone().bold(),
                ]));
            }
            let full_cmd = strip_bash_lc_and_escape(command);
            let destructive = is_destructive_shell_command(&full_cmd);
            header.push(Line::from(vec![
                "action kind  ".dim(),
                "BASH".bold(),
                "  the agent wants to run a shell command".dim(),
            ]));
            header.push(Line::from(vec![
                "cwd          ".dim(),
                cwd.as_path().display().to_string().into(),
            ]));
            header.push(Line::from(vec![
                "context      ".dim(),
                runtime_context_label().into(),
                " · ".dim(),
                network_context_label(network_approval_context.as_ref()).into(),
                " · ".dim(),
                write_context_label(additional_permissions.as_ref()).into(),
            ]));
            let risk_label = if destructive {
                Line::from(vec![
                    "risk         ".dim(),
                    "DESTRUCTIVE".red().bold(),
                    " recursive delete · no restore snapshot".yellow(),
                ])
            } else {
                Line::from(vec![
                    "risk         ".dim(),
                    command_risk_label(
                        network_approval_context.as_ref(),
                        additional_permissions.as_ref(),
                    )
                    .into(),
                ])
            };
            header.push(risk_label);
            header.push(Line::from(vec![
                "policy       ".dim(),
                reason
                    .clone()
                    .unwrap_or_else(|| {
                        "vil.core policy gate requires explicit approval".to_string()
                    })
                    .into(),
            ]));
            header.push(Line::from(vec![
                "batch        ".dim(),
                "batch 1/1 — policy-gated approval queue".into(),
            ]));
            header.push(Line::from(""));
            let mut full_cmd_lines = highlight_bash_to_lines(&full_cmd);
            if let Some(first) = full_cmd_lines.first_mut() {
                first.spans.insert(0, Span::from("$ "));
            }
            header.extend(full_cmd_lines);
            header.push(Line::from(""));
            header.push(Line::from(
                "hotkeys  y approve once   a approve+remember   n reject   r reject with reason"
                    .dim(),
            ));
            Box::new(Paragraph::new(header).wrap(Wrap { trim: false }))
        }
        ApprovalRequest::Permissions {
            thread_label,
            reason,
            permissions,
            ..
        } => {
            let mut header: Vec<Line<'static>> = Vec::new();
            if let Some(thread_label) = thread_label {
                header.push(Line::from(vec![
                    "Thread: ".into(),
                    thread_label.clone().bold(),
                ]));
                header.push(Line::from(""));
            }
            header.push(Line::from(vec![
                "action kind  ".dim(),
                "PERMISSIONS".bold(),
                " the agent wants expanded permissions".dim(),
            ]));
            if let Some(reason) = reason {
                header.push(Line::from(vec![
                    "policy       ".dim(),
                    reason.clone().into(),
                ]));
            }
            if let Some(rule_line) = format_requested_permissions_rule(permissions) {
                header.push(Line::from(vec!["request      ".dim(), rule_line.cyan()]));
            }
            header.push(Line::from(
                "hotkeys  y approve once   a approve+remember   n reject   r reject with reason"
                    .dim(),
            ));
            Box::new(Paragraph::new(header).wrap(Wrap { trim: false }))
        }
        ApprovalRequest::ApplyPatch {
            thread_label,
            reason,
            cwd,
            ..
        } => {
            let mut header: Vec<Box<dyn Renderable>> = Vec::new();
            if let Some(thread_label) = thread_label {
                header.push(Box::new(Line::from(vec![
                    "Thread: ".into(),
                    thread_label.clone().bold(),
                ])));
            }
            header.push(Box::new(Line::from(vec![
                "action kind  ".dim(),
                "FILE_EDIT".bold(),
                " the agent wants to modify files".dim(),
            ])));
            header.push(Box::new(Line::from(vec![
                "cwd          ".dim(),
                cwd.as_path().display().to_string().into(),
            ])));
            header.push(Box::new(Line::from(vec![
                "risk         ".dim(),
                "write mutation · review diff before applying".yellow(),
            ])));
            header.push(Box::new(Line::from(vec![
                "policy       ".dim(),
                reason
                    .clone()
                    .unwrap_or_else(|| {
                        "vil.core write_guard requires explicit approval".to_string()
                    })
                    .into(),
            ])));
            header.push(Box::new(Line::from(
                "hotkeys  y approve once   a approve+remember   n reject   r reject with reason"
                    .dim(),
            )));
            Box::new(ColumnRenderable::with(header))
        }
        ApprovalRequest::McpElicitation {
            thread_label,
            server_name,
            message,
            ..
        } => {
            let mut lines = Vec::new();
            if let Some(thread_label) = thread_label {
                lines.push(Line::from(vec![
                    "Thread: ".into(),
                    thread_label.clone().bold(),
                ]));
                lines.push(Line::from(""));
            }
            lines.extend([
                Line::from(vec!["action kind  ".dim(), "MCP".bold()]),
                Line::from(vec!["server       ".dim(), server_name.clone().bold()]),
                Line::from(vec![
                    "policy       ".dim(),
                    "external tool elicitation requires approval".into(),
                ]),
                Line::from(""),
                Line::from(message.clone()),
                Line::from(""),
                Line::from("hotkeys  y approve once   n reject   r reject with reason".dim()),
            ]);
            let header = Paragraph::new(lines).wrap(Wrap { trim: false });
            Box::new(header)
        }
    }
}

fn runtime_context_label() -> &'static str {
    "runtime host"
}

fn network_context_label(network_approval_context: Option<&NetworkApprovalContext>) -> String {
    network_approval_context.map_or_else(
        || "network inherit".to_string(),
        |context| format!("network host {}", context.host),
    )
}

fn write_context_label(additional_permissions: Option<&AdditionalPermissionProfile>) -> String {
    additional_permissions
        .and_then(|permissions| format_additional_permissions_rule(permissions))
        .map(|rule| format!("writes requested ({rule})"))
        .unwrap_or_else(|| "writes policy-gated".to_string())
}

fn command_risk_label(
    network_approval_context: Option<&NetworkApprovalContext>,
    additional_permissions: Option<&AdditionalPermissionProfile>,
) -> &'static str {
    if network_approval_context.is_some() {
        "network access · operator approval required"
    } else if additional_permissions.is_some() {
        "expanded permission profile · operator approval required"
    } else {
        "process execution · operator approval required"
    }
}

fn is_destructive_shell_command(command: &str) -> bool {
    let normalized = command.to_ascii_lowercase();
    normalized.contains("rm -rf")
        || normalized.contains("rm -fr")
        || normalized.contains("git clean -fd")
        || normalized.contains("mkfs")
        || normalized.contains("dd if=")
}

#[derive(Clone)]
enum ApprovalDecision {
    Command(CommandExecutionApprovalDecision),
    FileChange(FileChangeApprovalDecision),
    Permissions(PermissionsDecision),
    McpElicitation(McpServerElicitationAction),
}

#[derive(Clone, Copy)]
enum PermissionsDecision {
    GrantForTurn,
    GrantForTurnWithStrictAutoReview,
    GrantForSession,
    Deny,
}

#[derive(Clone)]
struct ApprovalOption {
    label: String,
    decision: ApprovalDecision,
    shortcuts: Vec<KeyBinding>,
}

fn command_decision_to_review_decision(
    decision: &CommandExecutionApprovalDecision,
) -> ReviewDecision {
    match decision {
        CommandExecutionApprovalDecision::Accept => ReviewDecision::Approved,
        CommandExecutionApprovalDecision::AcceptForSession => ReviewDecision::ApprovedForSession,
        CommandExecutionApprovalDecision::AcceptWithExecpolicyAmendment {
            execpolicy_amendment,
        } => ReviewDecision::ApprovedExecpolicyAmendment {
            proposed_execpolicy_amendment: execpolicy_amendment.clone().into_core(),
        },
        CommandExecutionApprovalDecision::ApplyNetworkPolicyAmendment {
            network_policy_amendment,
        } => ReviewDecision::NetworkPolicyAmendment {
            network_policy_amendment: network_policy_amendment.clone().into_core(),
        },
        CommandExecutionApprovalDecision::Decline => ReviewDecision::Denied,
        CommandExecutionApprovalDecision::Cancel => ReviewDecision::Abort,
    }
}

fn exec_options(
    available_decisions: &[CommandExecutionApprovalDecision],
    network_approval_context: Option<&NetworkApprovalContext>,
    additional_permissions: Option<&AdditionalPermissionProfile>,
    keymap: &ApprovalKeymap,
) -> Vec<ApprovalOption> {
    available_decisions
        .iter()
        .filter_map(|decision| match decision {
            CommandExecutionApprovalDecision::Accept => Some(ApprovalOption {
                label: "approve once".to_string(),
                decision: ApprovalDecision::Command(CommandExecutionApprovalDecision::Accept),
                shortcuts: keymap.approve.clone(),
            }),
            CommandExecutionApprovalDecision::AcceptWithExecpolicyAmendment {
                execpolicy_amendment,
            } => {
                let rendered_prefix = strip_bash_lc_and_escape(&execpolicy_amendment.command);
                if rendered_prefix.contains('\n') || rendered_prefix.contains('\r') {
                    return None;
                }

                Some(ApprovalOption {
                    label: format!("approve+remember · command prefix `{rendered_prefix}`"),
                    decision: ApprovalDecision::Command(
                        CommandExecutionApprovalDecision::AcceptWithExecpolicyAmendment {
                            execpolicy_amendment: execpolicy_amendment.clone(),
                        },
                    ),
                    shortcuts: keymap.approve_for_prefix.clone(),
                })
            }
            CommandExecutionApprovalDecision::AcceptForSession => Some(ApprovalOption {
                label: if network_approval_context.is_some() {
                    "approve+remember · host for this conversation".to_string()
                } else if additional_permissions.is_some() {
                    "approve+remember · permissions for this session".to_string()
                } else {
                    "approve+remember · command for this session".to_string()
                },
                decision: ApprovalDecision::Command(
                    CommandExecutionApprovalDecision::AcceptForSession,
                ),
                shortcuts: keymap.approve_for_session.clone(),
            }),
            CommandExecutionApprovalDecision::ApplyNetworkPolicyAmendment {
                network_policy_amendment,
            } => {
                let (label, shortcuts) = match network_policy_amendment.action {
                    NetworkPolicyRuleAction::Allow => (
                        "approve+remember · allow host".to_string(),
                        keymap.approve_for_prefix.clone(),
                    ),
                    NetworkPolicyRuleAction::Deny => (
                        "reject+remember · block host".to_string(),
                        keymap.deny.clone(),
                    ),
                };
                Some(ApprovalOption {
                    label,
                    decision: ApprovalDecision::Command(
                        CommandExecutionApprovalDecision::ApplyNetworkPolicyAmendment {
                            network_policy_amendment: network_policy_amendment.clone(),
                        },
                    ),
                    shortcuts,
                })
            }
            CommandExecutionApprovalDecision::Decline => Some(ApprovalOption {
                label: "reject".to_string(),
                decision: ApprovalDecision::Command(CommandExecutionApprovalDecision::Decline),
                shortcuts: keymap.deny.clone(),
            }),
            CommandExecutionApprovalDecision::Cancel => Some(ApprovalOption {
                label: "reject with reason".to_string(),
                decision: ApprovalDecision::Command(CommandExecutionApprovalDecision::Cancel),
                shortcuts: keymap.decline.clone(),
            }),
        })
        .collect()
}

pub(crate) fn format_additional_permissions_rule(
    additional_permissions: &AdditionalPermissionProfile,
) -> Option<String> {
    let mut parts = Vec::new();
    if additional_permissions
        .network
        .as_ref()
        .and_then(|network| network.enabled)
        .unwrap_or(false)
    {
        parts.push("network".to_string());
    }
    if let Some(file_system) = additional_permissions.file_system.as_ref() {
        let reads = format_file_system_entry_paths(
            file_system
                .entries
                .iter()
                .filter(|entry| entry.access == FileSystemAccessMode::Read),
        );
        if !reads.is_empty() {
            parts.push(format!("read {reads}"));
        }
        let writes = format_file_system_entry_paths(
            file_system
                .entries
                .iter()
                .filter(|entry| entry.access == FileSystemAccessMode::Write),
        );
        if !writes.is_empty() {
            parts.push(format!("write {writes}"));
        }
        let denied_reads = format_file_system_entry_paths(
            file_system
                .entries
                .iter()
                .filter(|entry| entry.access == FileSystemAccessMode::None),
        );
        if !denied_reads.is_empty() {
            parts.push(format!("deny read {denied_reads}"));
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("; "))
    }
}

pub(crate) fn format_requested_permissions_rule(
    permissions: &RequestPermissionProfile,
) -> Option<String> {
    let permissions =
        crate::app_server_approval_conversions::granted_permission_profile_from_request(
            permissions.clone(),
        );
    format_additional_permissions_rule(&AdditionalPermissionProfile {
        network: permissions.network.map(Into::into),
        file_system: permissions.file_system.map(Into::into),
    })
}

fn format_file_system_entry_paths<'a>(
    entries: impl Iterator<Item = &'a FileSystemSandboxEntry>,
) -> String {
    entries
        .map(|entry| match &entry.path {
            FileSystemPath::Path { path } => format!("`{}`", path.display()),
            FileSystemPath::GlobPattern { pattern } => format!("glob `{pattern}`"),
            FileSystemPath::Special { value } => format!("`{}`", special_path_label(value)),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn special_path_label(value: &FileSystemSpecialPath) -> String {
    match value {
        FileSystemSpecialPath::Root => ":root".to_string(),
        FileSystemSpecialPath::Minimal => ":minimal".to_string(),
        FileSystemSpecialPath::ProjectRoots { subpath } => path_label(":project_roots", subpath),
        FileSystemSpecialPath::Tmpdir => ":tmpdir".to_string(),
        FileSystemSpecialPath::SlashTmp => "/tmp".to_string(),
        FileSystemSpecialPath::Unknown { path, subpath } => path_label(path, subpath),
    }
}

fn path_label(base: &str, subpath: &Option<PathBuf>) -> String {
    match subpath {
        Some(subpath) => format!("{base}/{}", subpath.display()),
        None => base.to_string(),
    }
}

fn patch_options(keymap: &ApprovalKeymap) -> Vec<ApprovalOption> {
    vec![
        ApprovalOption {
            label: "approve once".to_string(),
            decision: ApprovalDecision::FileChange(FileChangeApprovalDecision::Accept),
            shortcuts: keymap.approve.clone(),
        },
        ApprovalOption {
            label: "approve+remember · file edits".to_string(),
            decision: ApprovalDecision::FileChange(FileChangeApprovalDecision::AcceptForSession),
            shortcuts: keymap.approve_for_session.clone(),
        },
        ApprovalOption {
            label: "reject with reason".to_string(),
            decision: ApprovalDecision::FileChange(FileChangeApprovalDecision::Cancel),
            shortcuts: keymap.decline.clone(),
        },
    ]
}

fn permissions_options(keymap: &ApprovalKeymap) -> Vec<ApprovalOption> {
    let deny_shortcuts = keymap
        .deny
        .iter()
        .copied()
        .filter(|shortcut| shortcut.parts() != (KeyCode::Esc, KeyModifiers::NONE))
        .collect();

    vec![
        ApprovalOption {
            label: "approve once · permissions for this turn".to_string(),
            decision: ApprovalDecision::Permissions(PermissionsDecision::GrantForTurn),
            shortcuts: keymap.approve.clone(),
        },
        ApprovalOption {
            label: "approve once · strict auto review".to_string(),
            decision: ApprovalDecision::Permissions(
                PermissionsDecision::GrantForTurnWithStrictAutoReview,
            ),
            shortcuts: vec![key_hint::plain(KeyCode::Char('r'))],
        },
        ApprovalOption {
            label: "approve+remember · permissions for this session".to_string(),
            decision: ApprovalDecision::Permissions(PermissionsDecision::GrantForSession),
            shortcuts: keymap.approve_for_session.clone(),
        },
        ApprovalOption {
            label: "reject".to_string(),
            decision: ApprovalDecision::Permissions(PermissionsDecision::Deny),
            shortcuts: deny_shortcuts,
        },
    ]
}

/// Build MCP elicitation options with stable cancellation semantics.
///
/// `Esc` is always treated as cancel for elicitation prompts, even if users
/// customize `decline`/`cancel` bindings. We keep this as a hard contract so
/// dismissal remains a safe abort path and never silently maps to "continue
/// without requested info." Any decline/cancel overlap is removed from the
/// decline option in elicitation mode to preserve this invariant.
fn elicitation_options(keymap: &ApprovalKeymap) -> Vec<ApprovalOption> {
    let mut cancel_shortcuts = vec![key_hint::plain(KeyCode::Esc)];
    for shortcut in &keymap.cancel {
        if !cancel_shortcuts.contains(shortcut) {
            cancel_shortcuts.push(*shortcut);
        }
    }

    let decline_shortcuts: Vec<KeyBinding> = keymap
        .decline
        .iter()
        .copied()
        .filter(|shortcut| !cancel_shortcuts.contains(shortcut))
        .collect();

    vec![
        ApprovalOption {
            label: "approve once · provide requested info".to_string(),
            decision: ApprovalDecision::McpElicitation(McpServerElicitationAction::Accept),
            shortcuts: keymap.approve.clone(),
        },
        ApprovalOption {
            label: "reject".to_string(),
            decision: ApprovalDecision::McpElicitation(McpServerElicitationAction::Decline),
            shortcuts: decline_shortcuts,
        },
        ApprovalOption {
            label: "reject with reason".to_string(),
            decision: ApprovalDecision::McpElicitation(McpServerElicitationAction::Cancel),
            shortcuts: cancel_shortcuts,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_event::AppEvent;
    use crate::session_protocol::ExecPolicyAmendment;
    use crate::session_protocol::NetworkApprovalProtocol;
    use crate::session_protocol::NetworkPolicyAmendment;
    use crossterm::event::KeyModifiers;
    use insta::assert_snapshot;
    use pretty_assertions::assert_eq;
    use tokio::sync::mpsc::unbounded_channel;
    use vac_protocol::models::FileSystemPermissions;
    use vac_protocol::models::NetworkPermissions;
    use vac_utils_absolute_path::AbsolutePathBuf;

    fn absolute_path(path: &str) -> AbsolutePathBuf {
        AbsolutePathBuf::from_absolute_path(path).expect("absolute path")
    }

    fn test_cwd() -> AbsolutePathBuf {
        absolute_path("/workspace")
    }

    fn render_overlay_lines(view: &ApprovalOverlay, width: u16) -> String {
        let height = view.desired_height(width);
        let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
        view.render(Rect::new(0, 0, width, height), &mut buf);
        (0..buf.area.height)
            .map(|row| {
                (0..buf.area.width)
                    .map(|col| buf[(col, row)].symbol().to_string())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn normalize_snapshot_paths(rendered: String) -> String {
        [
            (absolute_path("/tmp/readme.txt"), "/tmp/readme.txt"),
            (absolute_path("/tmp/out.txt"), "/tmp/out.txt"),
        ]
        .into_iter()
        .fold(rendered, |rendered, (path, normalized)| {
            rendered.replace(&path.display().to_string(), normalized)
        })
    }

    fn make_overlay(
        request: ApprovalRequest,
        app_event_tx: AppEventSender,
        features: Features,
    ) -> ApprovalOverlay {
        let keymap = crate::keymap::RuntimeKeymap::defaults();
        make_overlay_with_keymap(
            request,
            app_event_tx,
            features,
            keymap.approval,
            keymap.list,
        )
    }

    fn make_overlay_with_keymap(
        request: ApprovalRequest,
        app_event_tx: AppEventSender,
        features: Features,
        approval_keymap: ApprovalKeymap,
        list_keymap: ListKeymap,
    ) -> ApprovalOverlay {
        ApprovalOverlay::new(
            request,
            app_event_tx,
            features,
            approval_keymap,
            list_keymap,
        )
    }

    fn make_exec_request() -> ApprovalRequest {
        ApprovalRequest::Exec {
            thread_id: ThreadId::new(),
            thread_label: None,
            id: "test".to_string(),
            command: vec!["echo".to_string(), "hi".to_string()],
            cwd: test_cwd(),
            reason: Some("reason".to_string()),
            available_decisions: vec![
                CommandExecutionApprovalDecision::Accept,
                CommandExecutionApprovalDecision::Cancel,
            ],
            network_approval_context: None,
            additional_permissions: None,
        }
    }

    fn make_permissions_request() -> ApprovalRequest {
        ApprovalRequest::Permissions {
            thread_id: ThreadId::new(),
            thread_label: None,
            call_id: "test".to_string(),
            reason: Some("need workspace access".to_string()),
            permissions: RequestPermissionProfile {
                network: Some(NetworkPermissions {
                    enabled: Some(true),
                }),
                file_system: Some(FileSystemPermissions::from_read_write_roots(
                    Some(vec![absolute_path("/tmp/readme.txt")]),
                    Some(vec![absolute_path("/tmp/out.txt")]),
                )),
            },
        }
    }

    fn make_elicitation_request() -> ApprovalRequest {
        ApprovalRequest::McpElicitation {
            thread_id: ThreadId::new(),
            thread_label: None,
            server_name: "test-server".to_string(),
            request_id: RequestId::String("request-1".to_string()),
            message: "Need more information".to_string(),
        }
    }

    #[test]
    fn ctrl_c_aborts_and_clears_queue() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx);
        let mut view = make_overlay(make_exec_request(), tx, Features::with_defaults());
        view.enqueue_request(make_exec_request());
        assert_eq!(CancellationEvent::Handled, view.on_ctrl_c());
        assert!(view.queue.is_empty());
        assert!(view.is_complete());
    }

    #[test]
    fn configured_list_cancel_aborts_exec_approval() {
        let (tx_raw, mut rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx_raw);
        let mut keymap = crate::keymap::RuntimeKeymap::defaults();
        keymap.list.cancel = vec![key_hint::plain(KeyCode::Char('q'))];
        let mut view = make_overlay_with_keymap(
            make_exec_request(),
            tx,
            Features::with_defaults(),
            keymap.approval,
            keymap.list,
        );

        view.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));

        assert!(view.is_complete());
        let mut decision = None;
        while let Ok(ev) = rx.try_recv() {
            if let AppEvent::SubmitThreadOp {
                op: Op::ExecApproval { decision: d, .. },
                ..
            } = ev
            {
                decision = Some(d);
                break;
            }
        }
        assert_eq!(decision, Some(CommandExecutionApprovalDecision::Cancel));
    }

    #[test]
    fn configured_list_cancel_cancels_mcp_elicitation() {
        let (tx_raw, mut rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx_raw);
        let mut keymap = crate::keymap::RuntimeKeymap::defaults();
        keymap.list.cancel = vec![key_hint::plain(KeyCode::Char('q'))];
        let mut view = make_overlay_with_keymap(
            make_elicitation_request(),
            tx,
            Features::with_defaults(),
            keymap.approval,
            keymap.list,
        );

        view.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));

        assert!(view.is_complete());
        let mut decision = None;
        while let Ok(ev) = rx.try_recv() {
            if let AppEvent::SubmitThreadOp {
                op: Op::ResolveElicitation { decision: d, .. },
                ..
            } = ev
            {
                decision = Some(d);
                break;
            }
        }
        assert_eq!(decision, Some(McpServerElicitationAction::Cancel));
    }

    #[test]
    fn shortcut_triggers_selection() {
        let (tx, mut rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx);
        let mut view = make_overlay(make_exec_request(), tx, Features::with_defaults());
        assert!(!view.is_complete());
        view.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        // We expect at least one thread-scoped approval op message in the queue.
        let mut saw_op = false;
        while let Ok(ev) = rx.try_recv() {
            if matches!(ev, AppEvent::SubmitThreadOp { .. }) {
                saw_op = true;
                break;
            }
        }
        assert!(saw_op, "expected approval decision to emit an op");
    }

    #[test]
    fn deny_shortcut_submits_denied_exec_decision() {
        let (tx, mut rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx);
        let mut view = make_overlay(
            ApprovalRequest::Exec {
                thread_id: ThreadId::new(),
                thread_label: None,
                id: "test".to_string(),
                command: vec!["echo".to_string(), "hi".to_string()],
                cwd: test_cwd(),
                reason: None,
                available_decisions: vec![
                    CommandExecutionApprovalDecision::Accept,
                    CommandExecutionApprovalDecision::Decline,
                ],
                network_approval_context: None,
                additional_permissions: None,
            },
            tx,
            Features::with_defaults(),
        );

        view.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));

        let mut saw_denied = false;
        while let Ok(ev) = rx.try_recv() {
            if let AppEvent::SubmitThreadOp {
                op: Op::ExecApproval { decision, .. },
                ..
            } = ev
            {
                assert_eq!(decision, CommandExecutionApprovalDecision::Decline);
                saw_denied = true;
                break;
            }
        }
        assert!(saw_denied, "expected deny shortcut to emit denied decision");
    }

    #[test]
    fn network_deny_shortcut_submits_policy_deny_decision() {
        let (tx, mut rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx);
        let amendment = NetworkPolicyAmendment {
            host: "example.com".to_string(),
            action: NetworkPolicyRuleAction::Deny,
        };
        let mut view = make_overlay(
            ApprovalRequest::Exec {
                thread_id: ThreadId::new(),
                thread_label: None,
                id: "test".to_string(),
                command: vec!["curl".to_string(), "https://example.com".to_string()],
                cwd: test_cwd(),
                reason: None,
                available_decisions: vec![
                    CommandExecutionApprovalDecision::Accept,
                    CommandExecutionApprovalDecision::ApplyNetworkPolicyAmendment {
                        network_policy_amendment: amendment.clone(),
                    },
                ],
                network_approval_context: Some(NetworkApprovalContext {
                    host: "example.com".to_string(),
                    protocol: NetworkApprovalProtocol::Https,
                }),
                additional_permissions: None,
            },
            tx,
            Features::with_defaults(),
        );

        view.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));

        let mut saw_deny = false;
        while let Ok(ev) = rx.try_recv() {
            if let AppEvent::SubmitThreadOp {
                op: Op::ExecApproval { decision, .. },
                ..
            } = ev
            {
                assert_eq!(
                    decision,
                    CommandExecutionApprovalDecision::ApplyNetworkPolicyAmendment {
                        network_policy_amendment: amendment
                    }
                );
                saw_deny = true;
                break;
            }
        }
        assert!(
            saw_deny,
            "expected deny shortcut to emit network policy deny decision"
        );
    }

    #[test]
    fn resolved_request_dismisses_overlay_without_emitting_abort() {
        let (tx, mut rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx);
        let mut view = make_overlay(make_exec_request(), tx, Features::with_defaults());

        assert!(
            view.dismiss_app_server_request(&ResolvedAppServerRequest::ExecApproval {
                id: "test".to_string(),
            })
        );
        assert!(
            view.is_complete(),
            "resolved request should close the overlay"
        );
        assert!(
            rx.try_recv().is_err(),
            "dismissing a stale request should not emit an approval op"
        );
    }

    #[test]
    fn o_opens_source_thread_for_cross_thread_approval() {
        let (tx, mut rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx);
        let thread_id = ThreadId::new();
        let mut view = make_overlay(
            ApprovalRequest::Exec {
                thread_id,
                thread_label: Some("Robie [explorer]".to_string()),
                id: "test".to_string(),
                command: vec!["echo".to_string(), "hi".to_string()],
                cwd: test_cwd(),
                reason: None,
                available_decisions: vec![
                    CommandExecutionApprovalDecision::Accept,
                    CommandExecutionApprovalDecision::Cancel,
                ],
                network_approval_context: None,
                additional_permissions: None,
            },
            tx,
            Features::with_defaults(),
        );

        view.handle_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));

        let event = rx.try_recv().expect("expected select-agent-thread event");
        assert_eq!(
            matches!(event, AppEvent::SelectAgentThread(id) if id == thread_id),
            true
        );
    }

    #[test]
    fn configured_open_thread_shortcut_opens_source_thread() {
        let (tx, mut rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx);
        let thread_id = ThreadId::new();
        let mut keymap = crate::keymap::RuntimeKeymap::defaults();
        keymap.approval.open_thread = vec![key_hint::plain(KeyCode::Char('x'))];
        let mut view = make_overlay_with_keymap(
            ApprovalRequest::Exec {
                thread_id,
                thread_label: Some("Robie [explorer]".to_string()),
                id: "test".to_string(),
                command: vec!["echo".to_string(), "hi".to_string()],
                cwd: test_cwd(),
                reason: None,
                available_decisions: vec![
                    CommandExecutionApprovalDecision::Accept,
                    CommandExecutionApprovalDecision::Cancel,
                ],
                network_approval_context: None,
                additional_permissions: None,
            },
            tx,
            Features::with_defaults(),
            keymap.approval,
            keymap.list,
        );

        view.handle_key_event(KeyEvent::new(
            KeyCode::Char('o'),
            /*modifiers*/ KeyModifiers::NONE,
        ));
        assert!(rx.try_recv().is_err());

        view.handle_key_event(KeyEvent::new(
            KeyCode::Char('x'),
            /*modifiers*/ KeyModifiers::NONE,
        ));
        let event = rx.try_recv().expect("expected select-agent-thread event");
        assert!(matches!(event, AppEvent::SelectAgentThread(id) if id == thread_id));
    }

    #[test]
    fn cross_thread_footer_hint_mentions_o_shortcut() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx);
        let view = make_overlay(
            ApprovalRequest::Exec {
                thread_id: ThreadId::new(),
                thread_label: Some("Robie [explorer]".to_string()),
                id: "test".to_string(),
                command: vec!["echo".to_string(), "hi".to_string()],
                cwd: test_cwd(),
                reason: None,
                available_decisions: vec![
                    CommandExecutionApprovalDecision::Accept,
                    CommandExecutionApprovalDecision::Cancel,
                ],
                network_approval_context: None,
                additional_permissions: None,
            },
            tx,
            Features::with_defaults(),
        );

        assert_snapshot!(
            "approval_overlay_cross_thread_prompt",
            render_overlay_lines(&view, /*width*/ 80)
        );
    }

    #[test]
    fn exec_prefix_option_emits_execpolicy_amendment() {
        let (tx, mut rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx);
        let mut view = make_overlay(
            ApprovalRequest::Exec {
                thread_id: ThreadId::new(),
                thread_label: None,
                id: "test".to_string(),
                command: vec!["echo".to_string()],
                cwd: test_cwd(),
                reason: None,
                available_decisions: vec![
                    CommandExecutionApprovalDecision::Accept,
                    CommandExecutionApprovalDecision::AcceptWithExecpolicyAmendment {
                        execpolicy_amendment: ExecPolicyAmendment {
                            command: vec!["echo".to_string()],
                        },
                    },
                    CommandExecutionApprovalDecision::Cancel,
                ],
                network_approval_context: None,
                additional_permissions: None,
            },
            tx,
            Features::with_defaults(),
        );
        view.handle_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
        let mut saw_op = false;
        while let Ok(ev) = rx.try_recv() {
            if let AppEvent::SubmitThreadOp {
                op: Op::ExecApproval { decision, .. },
                ..
            } = ev
            {
                assert_eq!(
                    decision,
                    CommandExecutionApprovalDecision::AcceptWithExecpolicyAmendment {
                        execpolicy_amendment: ExecPolicyAmendment {
                            command: vec!["echo".to_string()],
                        }
                    }
                );
                saw_op = true;
                break;
            }
        }
        assert!(
            saw_op,
            "expected approval decision to emit an op with command prefix"
        );
    }

    #[test]
    fn network_deny_forever_shortcut_is_not_bound() {
        let (tx, mut rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx);
        let mut view = make_overlay(
            ApprovalRequest::Exec {
                thread_id: ThreadId::new(),
                thread_label: None,
                id: "test".to_string(),
                command: vec!["curl".to_string(), "https://example.com".to_string()],
                cwd: test_cwd(),
                reason: None,
                available_decisions: vec![
                    CommandExecutionApprovalDecision::Accept,
                    CommandExecutionApprovalDecision::AcceptForSession,
                    CommandExecutionApprovalDecision::ApplyNetworkPolicyAmendment {
                        network_policy_amendment: NetworkPolicyAmendment {
                            host: "example.com".to_string(),
                            action: NetworkPolicyRuleAction::Allow,
                        },
                    },
                    CommandExecutionApprovalDecision::Cancel,
                ],
                network_approval_context: Some(NetworkApprovalContext {
                    host: "example.com".to_string(),
                    protocol: NetworkApprovalProtocol::Https,
                }),
                additional_permissions: None,
            },
            tx,
            Features::with_defaults(),
        );
        view.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));

        assert!(
            rx.try_recv().is_err(),
            "unexpected approval event emitted for hidden network deny shortcut"
        );
    }

    #[test]
    fn header_includes_command_snippet() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx);
        let command = vec!["echo".into(), "hello".into(), "world".into()];
        let exec_request = ApprovalRequest::Exec {
            thread_id: ThreadId::new(),
            thread_label: None,
            id: "test".into(),
            command,
            cwd: test_cwd(),
            reason: None,
            available_decisions: vec![
                CommandExecutionApprovalDecision::Accept,
                CommandExecutionApprovalDecision::Cancel,
            ],
            network_approval_context: None,
            additional_permissions: None,
        };

        let view = make_overlay(exec_request, tx, Features::with_defaults());
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, view.desired_height(/*width*/ 80)));
        view.render(
            Rect::new(0, 0, 80, view.desired_height(/*width*/ 80)),
            &mut buf,
        );

        let rendered: Vec<String> = (0..buf.area.height)
            .map(|row| {
                (0..buf.area.width)
                    .map(|col| buf[(col, row)].symbol().to_string())
                    .collect()
            })
            .collect();
        assert!(
            rendered
                .iter()
                .any(|line| line.contains("echo hello world")),
            "expected header to include command snippet, got {rendered:?}"
        );
    }

    #[test]
    fn network_exec_options_use_expected_labels_and_hide_execpolicy_amendment() {
        let network_context = NetworkApprovalContext {
            host: "example.com".to_string(),
            protocol: NetworkApprovalProtocol::Https,
        };
        let keymap = crate::keymap::RuntimeKeymap::defaults();
        let options = exec_options(
            &[
                CommandExecutionApprovalDecision::Accept,
                CommandExecutionApprovalDecision::AcceptForSession,
                CommandExecutionApprovalDecision::ApplyNetworkPolicyAmendment {
                    network_policy_amendment: NetworkPolicyAmendment {
                        host: "example.com".to_string(),
                        action: NetworkPolicyRuleAction::Allow,
                    },
                },
                CommandExecutionApprovalDecision::Cancel,
            ],
            Some(&network_context),
            /*additional_permissions*/ None,
            &keymap.approval,
        );

        let labels: Vec<String> = options.into_iter().map(|option| option.label).collect();
        assert_eq!(
            labels,
            vec![
                "approve once".to_string(),
                "approve+remember · host for this conversation".to_string(),
                "approve+remember · allow host".to_string(),
                "reject with reason".to_string(),
            ]
        );
    }

    #[test]
    fn generic_exec_options_can_offer_allow_for_session() {
        let keymap = crate::keymap::RuntimeKeymap::defaults();
        let options = exec_options(
            &[
                CommandExecutionApprovalDecision::Accept,
                CommandExecutionApprovalDecision::AcceptForSession,
                CommandExecutionApprovalDecision::Cancel,
            ],
            /*network_approval_context*/ None,
            /*additional_permissions*/ None,
            &keymap.approval,
        );

        let labels: Vec<String> = options.into_iter().map(|option| option.label).collect();
        assert_eq!(
            labels,
            vec![
                "approve once".to_string(),
                "approve+remember · command for this session".to_string(),
                "reject with reason".to_string(),
            ]
        );
    }

    #[test]
    fn additional_permissions_exec_options_hide_execpolicy_amendment() {
        let keymap = crate::keymap::RuntimeKeymap::defaults();
        let additional_permissions = AdditionalPermissionProfile {
            network: None,
            file_system: Some(FileSystemPermissions::from_read_write_roots(
                Some(vec![absolute_path("/tmp/readme.txt")]),
                Some(vec![absolute_path("/tmp/out.txt")]),
            )),
        };
        let options = exec_options(
            &[
                CommandExecutionApprovalDecision::Accept,
                CommandExecutionApprovalDecision::Cancel,
            ],
            /*network_approval_context*/ None,
            Some(&additional_permissions),
            &keymap.approval,
        );

        let labels: Vec<String> = options.into_iter().map(|option| option.label).collect();
        assert_eq!(
            labels,
            vec!["approve once".to_string(), "reject with reason".to_string()]
        );
    }

    #[test]
    fn permissions_options_use_expected_labels() {
        let keymap = crate::keymap::RuntimeKeymap::defaults();
        let labels: Vec<String> = permissions_options(&keymap.approval)
            .into_iter()
            .map(|option| option.label)
            .collect();
        assert_eq!(
            labels,
            vec![
                "approve once · permissions for this turn".to_string(),
                "approve once · strict auto review".to_string(),
                "approve+remember · permissions for this session".to_string(),
                "reject".to_string(),
            ]
        );
    }

    #[test]
    fn additional_permissions_rule_shows_non_path_file_system_entries() {
        let additional_permissions = AdditionalPermissionProfile {
            network: None,
            file_system: Some(FileSystemPermissions {
                entries: vec![
                    FileSystemSandboxEntry {
                        path: FileSystemPath::Special {
                            value: FileSystemSpecialPath::Root,
                        },
                        access: FileSystemAccessMode::Write,
                    },
                    FileSystemSandboxEntry {
                        path: FileSystemPath::GlobPattern {
                            pattern: "**/*.env".to_string(),
                        },
                        access: FileSystemAccessMode::None,
                    },
                ],
                glob_scan_max_depth: None,
            }),
        };

        assert_eq!(
            format_additional_permissions_rule(&additional_permissions),
            Some("write `:root`; deny read glob `**/*.env`".to_string())
        );
    }

    #[test]
    fn permissions_session_shortcut_submits_session_scope() {
        let (tx, mut rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx);
        let mut view = make_overlay(make_permissions_request(), tx, Features::with_defaults());

        view.handle_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));

        let mut saw_op = false;
        while let Ok(ev) = rx.try_recv() {
            if let AppEvent::SubmitThreadOp {
                op: Op::RequestPermissionsResponse { response, .. },
                ..
            } = ev
            {
                assert_eq!(response.scope, PermissionGrantScope::Session);
                saw_op = true;
                break;
            }
        }
        assert!(
            saw_op,
            "expected permission approval decision to emit a session-scoped response"
        );
    }

    #[test]
    fn permissions_deny_shortcut_uses_deny_keymap() {
        let (tx, mut rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx);
        let mut keymap = crate::keymap::RuntimeKeymap::defaults();
        keymap.approval.deny = vec![key_hint::plain(KeyCode::Char('x'))];
        keymap.approval.decline = Vec::new();
        let mut view = make_overlay_with_keymap(
            make_permissions_request(),
            tx,
            Features::with_defaults(),
            keymap.approval,
            keymap.list,
        );

        view.handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        let mut saw_op = false;
        while let Ok(ev) = rx.try_recv() {
            if let AppEvent::SubmitThreadOp {
                op: Op::RequestPermissionsResponse { response, .. },
                ..
            } = ev
            {
                assert!(response.permissions.is_empty());
                assert_eq!(response.scope, PermissionGrantScope::Turn);
                assert!(!response.strict_auto_review);
                saw_op = true;
                break;
            }
        }
        assert!(
            saw_op,
            "expected permission deny shortcut to emit an empty permission response"
        );
    }

    #[test]
    fn permissions_strict_auto_review_shortcut_submits_turn_scope_with_strict_review() {
        let (tx, mut rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx);
        let mut view = make_overlay(make_permissions_request(), tx, Features::with_defaults());

        view.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));

        let mut saw_op = false;
        while let Ok(ev) = rx.try_recv() {
            if let AppEvent::SubmitThreadOp {
                op: Op::RequestPermissionsResponse { response, .. },
                ..
            } = ev
            {
                assert_eq!(response.scope, PermissionGrantScope::Turn);
                assert!(response.strict_auto_review);
                saw_op = true;
                break;
            }
        }
        assert!(
            saw_op,
            "expected permission approval decision to emit a strict auto review response"
        );
    }

    #[test]
    fn additional_permissions_prompt_shows_permission_rule_line() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx);
        let exec_request = ApprovalRequest::Exec {
            thread_id: ThreadId::new(),
            thread_label: None,
            id: "test".into(),
            command: vec!["cat".into(), "/tmp/readme.txt".into()],
            cwd: test_cwd(),
            reason: None,
            available_decisions: vec![
                CommandExecutionApprovalDecision::Accept,
                CommandExecutionApprovalDecision::Cancel,
            ],
            network_approval_context: None,
            additional_permissions: Some(AdditionalPermissionProfile {
                network: Some(NetworkPermissions {
                    enabled: Some(true),
                }),
                file_system: Some(FileSystemPermissions::from_read_write_roots(
                    Some(vec![absolute_path("/tmp/readme.txt")]),
                    Some(vec![absolute_path("/tmp/out.txt")]),
                )),
            }),
        };

        let view = make_overlay(exec_request, tx, Features::with_defaults());
        let mut buf = Buffer::empty(Rect::new(0, 0, 100, view.desired_height(/*width*/ 100)));
        view.render(
            Rect::new(0, 0, 100, view.desired_height(/*width*/ 100)),
            &mut buf,
        );

        let rendered: Vec<String> = (0..buf.area.height)
            .map(|row| {
                (0..buf.area.width)
                    .map(|col| buf[(col, row)].symbol().to_string())
                    .collect()
            })
            .collect();

        assert!(
            rendered
                .iter()
                .any(|line| line.contains("writes requested")),
            "expected requested permission context, got {rendered:?}"
        );
        assert!(
            rendered.iter().any(|line| line.contains("network;")),
            "expected network permission text, got {rendered:?}"
        );
    }

    #[test]
    fn additional_permissions_prompt_snapshot() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx);
        let exec_request = ApprovalRequest::Exec {
            thread_id: ThreadId::new(),
            thread_label: None,
            id: "test".into(),
            command: vec!["cat".into(), "/tmp/readme.txt".into()],
            cwd: test_cwd(),
            reason: Some("need filesystem access".into()),
            available_decisions: vec![
                CommandExecutionApprovalDecision::Accept,
                CommandExecutionApprovalDecision::Cancel,
            ],
            network_approval_context: None,
            additional_permissions: Some(AdditionalPermissionProfile {
                network: Some(NetworkPermissions {
                    enabled: Some(true),
                }),
                file_system: Some(FileSystemPermissions::from_read_write_roots(
                    Some(vec![absolute_path("/tmp/readme.txt")]),
                    Some(vec![absolute_path("/tmp/out.txt")]),
                )),
            }),
        };

        let view = make_overlay(exec_request, tx, Features::with_defaults());
        assert_snapshot!(
            "approval_overlay_additional_permissions_prompt",
            normalize_snapshot_paths(render_overlay_lines(&view, /*width*/ 120))
        );
    }

    #[test]
    fn permissions_prompt_snapshot() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx);
        let view = make_overlay(make_permissions_request(), tx, Features::with_defaults());
        assert_snapshot!(
            "approval_overlay_permissions_prompt",
            normalize_snapshot_paths(render_overlay_lines(&view, /*width*/ 120))
        );
    }

    #[test]
    fn apply_patch_prompt_with_thread_label_omits_command_line() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx);
        let mut changes = HashMap::new();
        changes.insert(
            PathBuf::from("bug1.txt"),
            FileChange::Add {
                content: "one\ntwo\nthree\n".to_string(),
            },
        );
        let request = ApprovalRequest::ApplyPatch {
            thread_id: ThreadId::new(),
            thread_label: Some("Banach [worker]".to_string()),
            id: "test".to_string(),
            reason: None,
            cwd: absolute_path("/tmp"),
            changes,
        };
        let keymap = crate::keymap::RuntimeKeymap::defaults();
        let view = ApprovalOverlay::new(
            request,
            tx,
            Features::with_defaults(),
            keymap.approval,
            keymap.list,
        );
        let rendered = render_overlay_lines(&view, /*width*/ 120);
        assert!(rendered.contains("Thread: Banach [worker]"));
        assert!(rendered.contains("o to open thread"));
        assert!(!rendered.contains("$ apply_patch"));
    }

    #[test]
    fn network_exec_prompt_title_includes_host() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx);
        let exec_request = ApprovalRequest::Exec {
            thread_id: ThreadId::new(),
            thread_label: None,
            id: "test".into(),
            command: vec!["curl".into(), "https://example.com".into()],
            cwd: test_cwd(),
            reason: Some("network request blocked".into()),
            available_decisions: vec![
                CommandExecutionApprovalDecision::Accept,
                CommandExecutionApprovalDecision::AcceptForSession,
                CommandExecutionApprovalDecision::ApplyNetworkPolicyAmendment {
                    network_policy_amendment: NetworkPolicyAmendment {
                        host: "example.com".to_string(),
                        action: NetworkPolicyRuleAction::Allow,
                    },
                },
                CommandExecutionApprovalDecision::Cancel,
            ],
            network_approval_context: Some(NetworkApprovalContext {
                host: "example.com".to_string(),
                protocol: NetworkApprovalProtocol::Https,
            }),
            additional_permissions: None,
        };

        let view = make_overlay(exec_request, tx, Features::with_defaults());
        let mut buf = Buffer::empty(Rect::new(0, 0, 100, view.desired_height(/*width*/ 100)));
        view.render(
            Rect::new(0, 0, 100, view.desired_height(/*width*/ 100)),
            &mut buf,
        );
        assert_snapshot!("network_exec_prompt", format!("{buf:?}"));

        let rendered: Vec<String> = (0..buf.area.height)
            .map(|row| {
                (0..buf.area.width)
                    .map(|col| buf[(col, row)].symbol().to_string())
                    .collect()
            })
            .collect();

        assert!(
            rendered
                .iter()
                .any(|line| line.contains("network host example.com")),
            "expected network context to include host, got {rendered:?}"
        );
        assert!(
            rendered.iter().any(|line| line.contains("$ curl")),
            "network prompt should show command line, got {rendered:?}"
        );
        assert!(
            !rendered.iter().any(|line| line.contains("don't ask again")),
            "network prompt should not show execpolicy option, got {rendered:?}"
        );
    }

    #[test]
    fn ctrl_shift_a_opens_fullscreen() {
        let (tx_raw, mut rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx_raw);
        let mut view = make_overlay(make_exec_request(), tx, Features::with_defaults());

        view.handle_key_event(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        ));

        let mut saw_fullscreen = false;
        while let Ok(ev) = rx.try_recv() {
            if matches!(ev, AppEvent::FullScreenApprovalRequest(_)) {
                saw_fullscreen = true;
                break;
            }
        }
        assert!(saw_fullscreen, "expected ctrl+shift+a to open fullscreen");
    }

    #[test]
    fn exec_history_cell_wraps_with_two_space_indent() {
        let command = vec![
            "/bin/zsh".into(),
            "-lc".into(),
            "git add tui/src/render/mod.rs tui/src/render/renderable.rs".into(),
        ];
        let cell = history_cell::new_approval_decision_cell(
            command,
            ReviewDecision::Approved,
            history_cell::ApprovalDecisionActor::User,
        );
        let lines = cell.display_lines(/*width*/ 28);
        let rendered: Vec<String> = lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect();
        let expected = vec![
            "✔ You approved vac to run".to_string(),
            "  git add tui/src/render/".to_string(),
            "  mod.rs tui/src/render/".to_string(),
            "  renderable.rs this time".to_string(),
        ];
        assert_eq!(rendered, expected);
    }

    #[test]
    fn esc_cancels_mcp_elicitation() {
        let (tx_raw, mut rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx_raw);
        let mut view = make_overlay(make_elicitation_request(), tx, Features::with_defaults());

        view.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        let mut decision = None;
        while let Ok(ev) = rx.try_recv() {
            if let AppEvent::SubmitThreadOp {
                op: Op::ResolveElicitation { decision: d, .. },
                ..
            } = ev
            {
                decision = Some(d);
                break;
            }
        }
        assert_eq!(decision, Some(McpServerElicitationAction::Cancel));
    }

    #[test]
    fn esc_still_cancels_elicitation_with_custom_overlap() {
        let (tx_raw, mut rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx_raw);
        let mut keymap = crate::keymap::RuntimeKeymap::defaults();
        keymap.approval.decline = vec![
            key_hint::plain(KeyCode::Esc),
            key_hint::plain(KeyCode::Char('n')),
        ];
        keymap.approval.cancel = vec![key_hint::plain(KeyCode::Char('x'))];

        let mut view = make_overlay_with_keymap(
            make_elicitation_request(),
            tx,
            Features::with_defaults(),
            keymap.approval,
            keymap.list,
        );

        view.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        let mut esc_decision = None;
        while let Ok(ev) = rx.try_recv() {
            if let AppEvent::SubmitThreadOp {
                op: Op::ResolveElicitation { decision, .. },
                ..
            } = ev
            {
                esc_decision = Some(decision);
                break;
            }
        }
        assert_eq!(esc_decision, Some(McpServerElicitationAction::Cancel));

        let (tx_raw, mut rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx_raw);
        let mut keymap = crate::keymap::RuntimeKeymap::defaults();
        keymap.approval.decline = vec![
            key_hint::plain(KeyCode::Esc),
            key_hint::plain(KeyCode::Char('n')),
        ];
        keymap.approval.cancel = vec![key_hint::plain(KeyCode::Char('x'))];

        let mut view = make_overlay_with_keymap(
            make_elicitation_request(),
            tx,
            Features::with_defaults(),
            keymap.approval,
            keymap.list,
        );
        view.handle_key_event(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
        let mut n_decision = None;
        while let Ok(ev) = rx.try_recv() {
            if let AppEvent::SubmitThreadOp {
                op: Op::ResolveElicitation { decision, .. },
                ..
            } = ev
            {
                n_decision = Some(decision);
                break;
            }
        }
        assert_eq!(n_decision, Some(McpServerElicitationAction::Decline));
    }

    #[test]
    fn enter_sets_last_selected_index_without_dismissing() {
        let (tx_raw, mut rx) = unbounded_channel::<AppEvent>();
        let tx = AppEventSender::new(tx_raw);
        let mut view = make_overlay(make_exec_request(), tx, Features::with_defaults());
        view.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(
            view.is_complete(),
            "exec approval should complete without queued requests"
        );

        let mut decision = None;
        while let Ok(ev) = rx.try_recv() {
            if let AppEvent::SubmitThreadOp {
                op: Op::ExecApproval { decision: d, .. },
                ..
            } = ev
            {
                decision = Some(d);
                break;
            }
        }
        assert_eq!(decision, Some(CommandExecutionApprovalDecision::Accept));
    }
}
