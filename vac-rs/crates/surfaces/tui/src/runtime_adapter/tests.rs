// Runtime adapter test suite (split from runtime_adapter.rs).

#![allow(unused_imports)]
#![allow(dead_code)]

use std::path::PathBuf;

use vac_core::local_runtime::ApprovalAction;
use vac_core::local_runtime::ApprovalDecision;
use vac_core::local_runtime::ApprovalId;
use vac_core::local_runtime::ApprovalPreview;
use vac_core::local_runtime::ApprovalResolved;
use vac_core::local_runtime::AutonomyMode;
use vac_core::local_runtime::RuntimeCommand;
use vac_core::local_runtime::RuntimeEntrypoint;
use vac_core::local_runtime::RuntimeEvent;
use vac_core::local_runtime::RuntimeTaskKind;
use vac_core::local_runtime::SessionId;
use vac_core::local_runtime::TaskId;
use vac_core::local_runtime::ValidationStatus;

use crate::session_protocol::CommandExecutionApprovalDecision;
use crate::session_protocol::FileChangeApprovalDecision;

use super::approval::approval_registry;
use super::*;

#[cfg(test)]
fn cwd_is_usable(cwd: &std::path::Path) -> bool {
    !cwd.as_os_str().is_empty()
}

fn fresh_bridge() -> RuntimeBridge {
    let cmd = mint_start_task(
        "do the thing",
        PathBuf::from("/tmp"),
        default_autonomy_mode(),
    );
    let RuntimeCommand::StartTask(start) = cmd else {
        panic!("expected StartTask");
    };
    let (session, task) = open_session_and_task(&start);
    RuntimeBridge::new(session, task)
}

#[test]
fn mint_start_task_uses_tui_entrypoint() {
    let cmd = mint_start_task("hello", PathBuf::from("/repo"), AutonomyMode::Assist);
    match cmd {
        RuntimeCommand::StartTask(start) => {
            assert_eq!(start.prompt, "hello");
            assert_eq!(start.entrypoint, RuntimeEntrypoint::Tui);
            assert_eq!(start.autonomy_mode, AutonomyMode::Assist);
            assert_eq!(start.cwd, PathBuf::from("/repo"));
        }
        other => panic!("expected StartTask, got {other:?}"),
    }
}

#[test]
fn open_session_and_task_links_task_to_session() {
    let cmd = mint_start_task(
        "do the thing",
        PathBuf::from("/tmp"),
        default_autonomy_mode(),
    );
    let RuntimeCommand::StartTask(start) = cmd else {
        panic!("expected StartTask");
    };
    assert!(cwd_is_usable(&start.cwd));
    let (session, task) = open_session_and_task(&start);
    assert_eq!(task.session_id, session.id);
    assert_eq!(session.entrypoint, RuntimeEntrypoint::Tui);
    assert_eq!(task.kind, RuntimeTaskKind::SemanticCoding);
    assert_eq!(task.prompt, "do the thing");
    let trace = start_trace(&session, &task, &start.prompt);
    assert!(trace.contains("entrypoint=tui"));
    assert!(trace.contains("prompt_len=12"));
}

#[test]
fn approval_decision_command_maps_decision_to_command() {
    let approval_id = ApprovalId::new();
    let approve = approval_decision_command(approval_id, ApprovalDecision::Approved);
    let reject = approval_decision_command(approval_id, ApprovalDecision::Rejected);
    assert!(matches!(
        approve,
        RuntimeCommand::Approve { approval_id: id } if id == approval_id
    ));
    assert!(matches!(
        reject,
        RuntimeCommand::Reject { approval_id: id } if id == approval_id
    ));
}

#[test]
fn cancel_and_resume_commands_round_trip_ids() {
    let task_id = TaskId::new();
    let session_id = SessionId::new();
    assert!(matches!(
        cancel_task_command(task_id),
        RuntimeCommand::CancelTask { task_id: id } if id == task_id
    ));
    assert!(matches!(
        resume_session_command(session_id),
        RuntimeCommand::ResumeSession { session_id: id } if id == session_id
    ));
}

#[test]
fn runtime_submission_plan_uses_start_task_as_canonical_input() {
    // The chatwidget bottom-input submit path mints a `StartTask` and
    // hands it to `RuntimeSubmitPlan::from_runtime_command`. The plan
    // must accept that canonical input and surface the prompt/cwd it
    // will route through the legacy compat transport.
    let cmd = mint_start_task("hello", PathBuf::from("/repo"), AutonomyMode::Assist);
    let plan = RuntimeSubmitPlan::from_runtime_command(cmd)
        .expect("mint_start_task always yields RuntimeCommand::StartTask");
    assert_eq!(plan.prompt(), "hello");
    assert_eq!(plan.cwd(), std::path::Path::new("/repo"));
    assert_eq!(plan.start_task.entrypoint, RuntimeEntrypoint::Tui);
    assert_eq!(plan.start_task.autonomy_mode, AutonomyMode::Assist);
}

#[test]
fn runtime_submission_plan_rejects_non_start_task_commands() {
    // The plan is the runtime-first seam for *fresh prompt* submission.
    // Any other command variant must be rejected so the chatwidget does
    // not silently fall back to the legacy transport without a
    // canonical `StartTask` input.
    let cancel = RuntimeCommand::CancelTask {
        task_id: TaskId::new(),
    };
    assert!(RuntimeSubmitPlan::from_runtime_command(cancel).is_none());
    let approve = RuntimeCommand::Approve {
        approval_id: ApprovalId::new(),
    };
    assert!(RuntimeSubmitPlan::from_runtime_command(approve).is_none());
}

#[test]
fn runtime_submission_plan_creates_session_and_task_from_start_task() {
    // Session/task derived from the runtime-first input must reuse the
    // canonical `StartTask` fields verbatim so activity, approval,
    // validation and evidence projections share a single stable id.
    let cmd = mint_start_task("hi", PathBuf::from("/repo"), AutonomyMode::Assist);
    let plan = RuntimeSubmitPlan::from_runtime_command(cmd).unwrap();
    assert_eq!(plan.session.cwd, plan.start_task.cwd);
    assert_eq!(plan.session.entrypoint, plan.start_task.entrypoint);
    assert_eq!(plan.session.autonomy_mode, plan.start_task.autonomy_mode);
    assert_eq!(plan.task.session_id, plan.session.id);
    assert_eq!(plan.task.kind, RuntimeTaskKind::SemanticCoding);
    assert_eq!(plan.task.prompt, plan.start_task.prompt);
}

#[test]
fn runtime_submission_plan_preserves_prompt_and_cwd() {
    // Chatwidget reuses `runtime_submission.start_task.cwd` when it
    // builds the legacy `AppCommand::UserTurn` compat transport. Lock
    // that round-trip down here so a future refactor cannot quietly
    // re-derive cwd from `self.config` and break the canonical-input
    // contract.
    let cmd = mint_start_task(
        "preserve me",
        PathBuf::from("/foo/bar"),
        AutonomyMode::Assist,
    );
    let plan = RuntimeSubmitPlan::from_runtime_command(cmd).unwrap();
    assert_eq!(plan.start_task.prompt, "preserve me");
    assert_eq!(plan.start_task.cwd, PathBuf::from("/foo/bar"));
    let trace = plan.trace();
    assert!(trace.contains("entrypoint=tui"));
    assert!(trace.contains("autonomy=assist"));
    assert!(trace.contains("prompt_len=11"));
}

#[test]
fn runtime_submission_plan_marks_legacy_userturn_as_compat_transport() {
    // The plan must explicitly tag the legacy `AppCommand::UserTurn`
    // path as a compatibility transport, not as the canonical
    // execution source-of-truth. 00E retires this marker by removing
    // the legacy reachability entirely.
    let cmd = mint_start_task("hello", PathBuf::from("/repo"), AutonomyMode::Assist);
    let plan = RuntimeSubmitPlan::from_runtime_command(cmd).unwrap();
    assert_eq!(
        plan.legacy_compat_transport,
        LegacyCompatTransport::UserTurn
    );
}

#[test]
fn opening_events_include_session_and_task_started_once() {
    let mut bridge = fresh_bridge();
    let first = bridge.opening_events();
    assert_eq!(first.len(), 2);
    assert!(matches!(first[0], RuntimeEvent::SessionStarted(_)));
    assert!(matches!(first[1], RuntimeEvent::TaskStarted(_)));
    // Idempotent: do not emit duplicates if asked again.
    let second = bridge.opening_events();
    assert!(second.is_empty());
}

#[test]
fn record_approval_decision_returns_command_and_resolved_event() {
    let mut bridge = fresh_bridge();
    let approval_id = ApprovalId::new();
    let (cmd, ev) = bridge.record_approval_decision(
        approval_id,
        ApprovalDecision::Approved,
        Some("operator approved".to_string()),
    );
    assert!(matches!(
        cmd,
        RuntimeCommand::Approve { approval_id: id } if id == approval_id
    ));
    assert!(matches!(ev, RuntimeEvent::ApprovalResolved(_)));
}

#[test]
fn runtime_history_label_renders_each_event_kind() {
    let mut bridge = fresh_bridge();
    let opens = bridge.opening_events();
    for event in opens {
        let label = verbose_runtime_history_label(&event);
        assert!(
            label.starts_with("vac runtime:"),
            "label should be operator-friendly: {label}"
        );
    }
}

#[test]
fn project_exec_approval_request_emits_approval_requested_with_command_preview() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let events = bridge.project_exec_approval_request(
        "call-123",
        &["ls".to_string(), "-la".to_string()],
        Some("operator review"),
    );
    assert_eq!(events.len(), 1);
    match &events[0] {
        RuntimeEvent::ApprovalRequested(req) => {
            assert_eq!(req.task_id, bridge.task_id());
            assert!(matches!(req.action, ApprovalAction::ExecuteProcess));
            assert!(matches!(req.preview, ApprovalPreview::Command(_)));
        }
        other => panic!("expected ApprovalRequested, got {other:?}"),
    }
}

#[test]
fn project_apply_patch_approval_request_emits_file_list_preview() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let files = vec![PathBuf::from("src/lib.rs"), PathBuf::from("README.md")];
    let events = bridge.project_apply_patch_approval_request("call-00D3-patch-legacy", files, None);
    assert_eq!(events.len(), 1);
    match &events[0] {
        RuntimeEvent::ApprovalRequested(req) => {
            assert!(matches!(req.preview, ApprovalPreview::FileList(_)));
        }
        other => panic!("expected ApprovalRequested, got {other:?}"),
    }
}

#[test]
fn project_task_completed_emits_label_with_runtime_prefix() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let events = bridge.project_task_completed();
    assert_eq!(events.len(), 1);
    let label = verbose_runtime_history_label(&events[0]);
    assert_eq!(label, "vac runtime: task completed");
}

#[test]
fn project_task_failed_emits_failed_label_with_message() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let events = bridge.project_task_failed("boom".to_string());
    assert_eq!(events.len(), 1);
    let label = verbose_runtime_history_label(&events[0]);
    assert!(label.contains("task failed"), "got: {label}");
    assert!(label.contains("boom"), "got: {label}");
}

#[test]
fn project_task_cancelled_emits_label_with_reason() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    // Non-"interrupted" reasons render as "task cancelled — {reason}".
    let events = bridge.project_task_cancelled(Some("timeout".to_string()));
    let label = verbose_runtime_history_label(&events[0]);
    assert_eq!(
        label, "vac runtime: task cancelled — timeout",
        "got: {label}"
    );
}

#[test]
fn approval_resolved_for_exec_maps_accept_to_approve_command() {
    let (cmd, ev, _correlation) =
        approval_resolved_for_exec_decision("call-1", &CommandExecutionApprovalDecision::Accept);
    assert!(matches!(cmd, RuntimeCommand::Approve { .. }));
    assert!(matches!(ev, RuntimeEvent::ApprovalResolved(_)));
}

#[test]
fn approval_resolved_for_exec_maps_cancel_to_reject_command() {
    let (cmd, _ev, _correlation) =
        approval_resolved_for_exec_decision("call-1", &CommandExecutionApprovalDecision::Cancel);
    assert!(matches!(cmd, RuntimeCommand::Reject { .. }));
}

#[test]
fn approval_resolved_for_patch_maps_accept_and_cancel() {
    let (cmd_a, _, _correlation) =
        approval_resolved_for_patch_decision("call-1", &FileChangeApprovalDecision::Accept);
    assert!(matches!(cmd_a, RuntimeCommand::Approve { .. }));
    let (cmd_c, _, _correlation) =
        approval_resolved_for_patch_decision("call-1", &FileChangeApprovalDecision::Cancel);
    assert!(matches!(cmd_c, RuntimeCommand::Reject { .. }));
}

#[test]
fn project_assistant_delta_emits_only_first_non_empty_delta_per_turn() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let first = bridge.project_assistant_delta("Hello");
    assert_eq!(first.len(), 1);
    assert!(matches!(first[0], RuntimeEvent::AssistantDelta(_)));
    let again = bridge.project_assistant_delta(" world");
    assert!(
        again.is_empty(),
        "second delta in same turn must be deduped"
    );
    let empty = bridge.project_assistant_delta("   ");
    assert!(empty.is_empty(), "empty delta must not emit");
    // After task complete the flag resets, so a fresh delta would emit again.
    let _ = bridge.project_task_completed();
    let next_turn = bridge.project_assistant_delta("new turn");
    assert_eq!(next_turn.len(), 1);
}

#[test]
fn project_exec_command_started_emits_tool_started_for_non_validation() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let events =
        bridge.project_exec_command_started("call-1", &["ls".to_string(), "-la".to_string()]);
    assert_eq!(events.len(), 1);
    match &events[0] {
        RuntimeEvent::ToolCallStarted(t) => {
            assert_eq!(t.tool_name, "exec");
            assert_eq!(t.input_preview.as_deref(), Some("ls -la"));
        }
        other => panic!("expected ToolCallStarted, got {other:?}"),
    }
}

#[test]
fn project_exec_command_started_emits_validation_started_for_cargo_check() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let events = bridge.project_exec_command_started(
        "call-2",
        &[
            "cargo".to_string(),
            "check".to_string(),
            "-p".to_string(),
            "vac-tui".to_string(),
        ],
    );
    assert_eq!(events.len(), 1);
    match &events[0] {
        RuntimeEvent::ValidationStarted(v) => {
            assert!(v.command_display.contains("cargo check"));
        }
        other => panic!("expected ValidationStarted, got {other:?}"),
    }
}

#[test]
fn project_exec_command_finished_emits_tool_finished_with_success_from_exit_code() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let _ = bridge.project_exec_command_started("call-3", &["ls".to_string()]);
    let ok_events = bridge.project_exec_command_finished("call-3", &["ls".to_string()], 0);
    match &ok_events[0] {
        RuntimeEvent::ToolCallFinished(t) => {
            assert!(t.success);
            assert_eq!(t.tool_name, "exec");
        }
        other => panic!("expected ToolCallFinished, got {other:?}"),
    }
    let fail_events = bridge.project_exec_command_finished("call-4", &["false".to_string()], 1);
    match &fail_events[0] {
        RuntimeEvent::ToolCallFinished(t) => assert!(!t.success),
        other => panic!("expected ToolCallFinished, got {other:?}"),
    }
}

#[test]
fn project_exec_command_finished_emits_validation_finished_for_validation_call() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let _ = bridge.project_exec_command_started("v-1", &["cargo".to_string(), "test".to_string()]);
    let events =
        bridge.project_exec_command_finished("v-1", &["cargo".to_string(), "test".to_string()], 0);
    match &events[0] {
        RuntimeEvent::ValidationFinished(v) => {
            assert_eq!(v.status, ValidationStatus::Passed);
        }
        other => panic!("expected ValidationFinished, got {other:?}"),
    }
}

#[test]
fn project_mcp_tool_started_and_finished_emit_tool_events() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let s_events = bridge.project_mcp_tool_started("m-1", "workflow", "bash");
    match &s_events[0] {
        RuntimeEvent::ToolCallStarted(t) => {
            assert_eq!(t.tool_name, "mcp:workflow/bash");
            assert!(t.input_preview.is_none());
        }
        other => panic!("expected ToolCallStarted, got {other:?}"),
    }
    let f_events = bridge.project_mcp_tool_finished("m-1", "workflow", "bash", true);
    match &f_events[0] {
        RuntimeEvent::ToolCallFinished(t) => {
            assert_eq!(t.tool_name, "mcp:workflow/bash");
            assert!(t.success);
        }
        other => panic!("expected ToolCallFinished, got {other:?}"),
    }
}

#[test]
fn approval_id_correlates_request_and_resolution_for_exec() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let call_id = "call-00D3-exec-correlate";
    let events = bridge.project_exec_approval_request(call_id, &["ls".to_string()], None);
    let request_id = match &events[0] {
        RuntimeEvent::ApprovalRequested(req) => req.id,
        other => panic!("expected ApprovalRequested, got {other:?}"),
    };
    let (cmd, resolved, _correlation) =
        approval_resolved_for_exec_decision(call_id, &CommandExecutionApprovalDecision::Accept);
    match cmd {
        RuntimeCommand::Approve { approval_id } => {
            assert_eq!(
                approval_id, request_id,
                "resolved approval id must match the requested approval id"
            );
        }
        other => panic!("expected Approve command, got {other:?}"),
    }
    match resolved {
        RuntimeEvent::ApprovalResolved(ev) => {
            assert_eq!(ev.approval_id, request_id);
        }
        other => panic!("expected ApprovalResolved, got {other:?}"),
    }
}

#[test]
fn approval_id_correlates_request_and_resolution_for_patch() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let call_id = "call-00D3-patch-correlate";
    let events = bridge.project_apply_patch_approval_request(
        call_id,
        vec![PathBuf::from("src/main.rs")],
        None,
    );
    let request_id = match &events[0] {
        RuntimeEvent::ApprovalRequested(req) => req.id,
        other => panic!("expected ApprovalRequested, got {other:?}"),
    };
    let (cmd, resolved, _correlation) =
        approval_resolved_for_patch_decision(call_id, &FileChangeApprovalDecision::Cancel);
    match cmd {
        RuntimeCommand::Reject { approval_id } => {
            assert_eq!(approval_id, request_id);
        }
        other => panic!("expected Reject command, got {other:?}"),
    }
    match resolved {
        RuntimeEvent::ApprovalResolved(ev) => assert_eq!(ev.approval_id, request_id),
        other => panic!("expected ApprovalResolved, got {other:?}"),
    }
}

#[test]
fn record_call_approval_decision_returns_correlated_pair_when_pending() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let call_id = "call-00D3-record-pending";
    let events = bridge.project_exec_approval_request(call_id, &["true".to_string()], None);
    let request_id = match &events[0] {
        RuntimeEvent::ApprovalRequested(req) => req.id,
        other => panic!("expected ApprovalRequested, got {other:?}"),
    };
    let pair = bridge.record_call_approval_decision(
        call_id,
        ApprovalDecision::Approved,
        Some("operator approved".to_string()),
    );
    let (cmd, ev) = pair.expect("pending entry should yield a correlated pair");
    match cmd {
        RuntimeCommand::Approve { approval_id } => assert_eq!(approval_id, request_id),
        other => panic!("expected Approve, got {other:?}"),
    }
    match ev {
        RuntimeEvent::ApprovalResolved(ev) => assert_eq!(ev.approval_id, request_id),
        other => panic!("expected ApprovalResolved, got {other:?}"),
    }
    // Second call must return None: registry entry consumed.
    assert!(
        bridge
            .record_call_approval_decision(call_id, ApprovalDecision::Approved, None)
            .is_none(),
        "second resolution for the same call_id must not yield a correlated pair"
    );
}

#[test]
fn record_call_approval_decision_returns_none_when_no_pending_entry() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let pair = bridge.record_call_approval_decision(
        "call-00D3-record-nopending",
        ApprovalDecision::Rejected,
        None,
    );
    assert!(pair.is_none());
}

#[test]
fn project_task_completed_clears_tracked_pending_approvals() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let call_id = "call-00D3-cleanup";
    let _ = bridge.project_exec_approval_request(call_id, &["ls".to_string()], None);
    assert!(
        approval_registry::contains(call_id),
        "registry must contain the freshly minted call_id"
    );
    let _ = bridge.project_task_completed();
    assert!(
        !approval_registry::contains(call_id),
        "task complete must drain tracked call_ids from the registry"
    );
}

#[test]
fn approval_id_correlates_request_and_resolution_for_permissions() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let call_id = "call-00D4-perm-correlate";
    let events = bridge.project_permissions_approval_request(
        call_id,
        "additional permissions requested",
        Some("network access"),
    );
    let request_id = match &events[0] {
        RuntimeEvent::ApprovalRequested(req) => req.id,
        other => panic!("expected ApprovalRequested, got {other:?}"),
    };
    let (cmd, resolved, correlation) =
        approval_resolved_for_permissions_decision(call_id, ApprovalDecision::Approved);
    assert_eq!(correlation, ApprovalCorrelation::Correlated);
    match cmd {
        RuntimeCommand::Approve { approval_id } => {
            assert_eq!(
                approval_id, request_id,
                "permissions resolution must reuse the requested ApprovalId"
            );
        }
        other => panic!("expected Approve command, got {other:?}"),
    }
    match resolved {
        RuntimeEvent::ApprovalResolved(ev) => assert_eq!(ev.approval_id, request_id),
        other => panic!("expected ApprovalResolved, got {other:?}"),
    }
}

#[test]
fn approval_id_correlates_request_and_resolution_for_mcp_elicitation() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let request_key = "req-00D4-elic-correlate";
    let events = bridge.project_mcp_elicitation_request(
        request_key,
        "weather-server",
        "Allow weather-server to read your location?",
    );
    let request_id = match &events[0] {
        RuntimeEvent::ApprovalRequested(req) => req.id,
        other => panic!("expected ApprovalRequested, got {other:?}"),
    };
    let (cmd, resolved, correlation) =
        approval_resolved_for_elicitation_decision(request_key, ApprovalDecision::Rejected);
    assert_eq!(correlation, ApprovalCorrelation::Correlated);
    match cmd {
        RuntimeCommand::Reject { approval_id } => {
            assert_eq!(
                approval_id, request_id,
                "elicitation resolution must reuse the requested ApprovalId"
            );
        }
        other => panic!("expected Reject command, got {other:?}"),
    }
    match resolved {
        RuntimeEvent::ApprovalResolved(ev) => assert_eq!(ev.approval_id, request_id),
        other => panic!("expected ApprovalResolved, got {other:?}"),
    }
}

#[test]
fn permissions_approval_resolution_uncorrelated_fallback_emits_valid_pair() {
    // No prior projection → registry has no entry for this key. The
    // resolve fn must mint a fresh ApprovalId and return
    // ApprovalCorrelation::Fallback so callers can label the evidence as
    // uncorrelated.
    let (cmd, resolved, correlation) = approval_resolved_for_permissions_decision(
        "call-00D4-perm-fallback",
        ApprovalDecision::Approved,
    );
    assert_eq!(correlation, ApprovalCorrelation::Fallback);
    let cmd_id = match cmd {
        RuntimeCommand::Approve { approval_id } => approval_id,
        other => panic!("expected Approve, got {other:?}"),
    };
    match resolved {
        RuntimeEvent::ApprovalResolved(ev) => {
            assert_eq!(ev.approval_id, cmd_id, "command and event ids must agree");
        }
        other => panic!("expected ApprovalResolved, got {other:?}"),
    }
}

#[test]
fn mcp_elicitation_resolution_uncorrelated_fallback_emits_valid_pair() {
    let (cmd, resolved, correlation) = approval_resolved_for_elicitation_decision(
        "req-00D4-elic-fallback",
        ApprovalDecision::Rejected,
    );
    assert_eq!(correlation, ApprovalCorrelation::Fallback);
    let cmd_id = match cmd {
        RuntimeCommand::Reject { approval_id } => approval_id,
        other => panic!("expected Reject, got {other:?}"),
    };
    match resolved {
        RuntimeEvent::ApprovalResolved(ev) => {
            assert_eq!(ev.approval_id, cmd_id);
        }
        other => panic!("expected ApprovalResolved, got {other:?}"),
    }
}

#[test]
fn project_task_completed_clears_permissions_and_elicitation_tracked_approvals() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let perm_key = "call-00D4-perm-cleanup";
    let elic_key = "req-00D4-elic-cleanup";
    let _ = bridge.project_permissions_approval_request(perm_key, "summary", None);
    let _ = bridge.project_mcp_elicitation_request(elic_key, "server-x", "please confirm");
    assert!(approval_registry::contains(perm_key));
    assert!(approval_registry::contains(elic_key));
    let _ = bridge.project_task_completed();
    assert!(
        !approval_registry::contains(perm_key),
        "task complete must drain permissions call_ids"
    );
    assert!(
        !approval_registry::contains(elic_key),
        "task complete must drain elicitation request keys"
    );
}

#[test]
fn runtime_history_label_uses_compact_tool_labels() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let started =
        bridge.project_exec_command_started("call-compact", &["ls".to_string(), "-la".to_string()]);
    let label = verbose_runtime_history_label(&started[0]);
    assert_eq!(label, "vac runtime: exec started — ls -la", "got: {label}");
    let finished = bridge.project_exec_command_finished("call-compact", &[], 0);
    let label_done = verbose_runtime_history_label(&finished[0]);
    assert_eq!(
        label_done, "vac runtime: exec done — ls -la",
        "finish must reuse started preview, got: {label_done}"
    );
}

#[test]
fn runtime_history_label_compact_for_mcp_tool() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let started = bridge.project_mcp_tool_started("m-compact", "workflown", "workflow");
    let label = verbose_runtime_history_label(&started[0]);
    assert_eq!(
        label, "vac runtime: mcp started — workflown/workflow",
        "got: {label}"
    );
    let finished = bridge.project_mcp_tool_finished("m-compact", "workflown", "workflow", true);
    let label_done = verbose_runtime_history_label(&finished[0]);
    assert_eq!(label_done, "vac runtime: mcp done — workflown/workflow");
}

#[test]
fn exec_finish_uses_started_command_preview() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let _ = bridge
        .project_exec_command_started("call-pair", &["echo".to_string(), "hi there".to_string()]);
    // Caller passes an empty slice on finish — the bridge must still
    // surface the preview captured at start time. The new safe-preview
    // pipeline routes through strip_bash_lc_and_escape, which preserves
    // arg boundaries by quoting whitespace-bearing args.
    let finished = bridge.project_exec_command_finished("call-pair", &[], 0);
    match &finished[0] {
        RuntimeEvent::ToolCallFinished(t) => {
            assert_eq!(t.output_preview.as_deref(), Some("echo 'hi there'"));
        }
        other => panic!("expected ToolCallFinished, got {other:?}"),
    }
}

#[test]
fn mcp_finish_uses_started_tool_label() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let _ = bridge.project_mcp_tool_started("mcp-pair", "alpha", "do_thing");
    // Caller passes mismatched server/tool on finish — bridge must
    // re-use the label captured at start so the activity rows agree.
    let finished = bridge.project_mcp_tool_finished("mcp-pair", "WRONG", "WRONG", true);
    match &finished[0] {
        RuntimeEvent::ToolCallFinished(t) => {
            assert_eq!(t.tool_name, "mcp:alpha/do_thing");
        }
        other => panic!("expected ToolCallFinished, got {other:?}"),
    }
}

#[test]
fn validation_finish_uses_started_command_preview() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let started = bridge.project_exec_command_started(
        "v-pair",
        &[
            "cargo".to_string(),
            "test".to_string(),
            "--all-features".to_string(),
        ],
    );
    assert_eq!(
        verbose_runtime_history_label(&started[0]),
        "vac runtime: validation started — cargo test --all-features",
    );
    // Empty command slice on finish — validation_finished must reuse
    // the command_display captured at start.
    let finished = bridge.project_exec_command_finished("v-pair", &[], 0);
    match &finished[0] {
        RuntimeEvent::ValidationFinished(v) => {
            assert_eq!(v.command_display, "cargo test --all-features");
            assert_eq!(v.status, ValidationStatus::Passed);
        }
        other => panic!("expected ValidationFinished, got {other:?}"),
    }
    assert_eq!(
        verbose_runtime_history_label(&finished[0]),
        "vac runtime: validation passed — cargo test --all-features",
    );
}

#[test]
fn assistant_streaming_label_is_deduped_per_turn() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let first = bridge.project_assistant_delta("hello");
    assert_eq!(first.len(), 1);
    let label = verbose_runtime_history_label(&first[0]);
    assert_eq!(label, "vac runtime: assistant streaming");
    let dup = bridge.project_assistant_delta(" world");
    assert!(dup.is_empty(), "second delta in same turn must be deduped");
    // Terminal event must reset the dedupe flag so the next turn emits
    // an assistant streaming row again.
    let _ = bridge.project_task_completed();
    let next = bridge.project_assistant_delta("new");
    assert_eq!(
        next.len(),
        1,
        "dedupe flag must reset on task terminal events"
    );
}

#[test]
fn task_terminal_events_clear_runtime_tracking_maps() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let _ = bridge.project_exec_command_started("e-clear", &["ls".to_string()]);
    let _ = bridge.project_mcp_tool_started("m-clear", "srv", "tool");
    let _ =
        bridge.project_exec_command_started("v-clear", &["cargo".to_string(), "test".to_string()]);
    let _ = bridge.project_exec_approval_request("a-clear", &["true".to_string()], None);
    assert_eq!(bridge.exec_calls_len(), 1);
    assert_eq!(bridge.mcp_calls_len(), 1);
    assert_eq!(bridge.validation_calls_len(), 1);
    assert_eq!(bridge.approval_kinds_len(), 1);

    // Completed: cancels pending validations FIRST, then clears all maps.
    let _ = bridge.project_task_completed();
    assert_eq!(
        bridge.exec_calls_len(),
        0,
        "task completed must clear exec_calls"
    );
    assert_eq!(
        bridge.mcp_calls_len(),
        0,
        "task completed must clear mcp_calls"
    );
    assert_eq!(
        bridge.validation_calls_len(),
        0,
        "task completed must drain validation_calls after emitting cancelled events"
    );
    assert_eq!(
        bridge.approval_kinds_len(),
        0,
        "task completed must clear approval_kinds"
    );

    // Failed path: all maps cleared.
    let _ = bridge.project_exec_command_started("e-clear-2", &["ls".to_string()]);
    let _ = bridge.project_mcp_tool_started("m-clear-2", "srv", "tool");
    let _ = bridge
        .project_exec_command_started("v-clear-2", &["cargo".to_string(), "check".to_string()]);
    let _ = bridge.project_exec_approval_request("a-clear-2", &["true".to_string()], None);
    let _ = bridge.project_task_failed("boom".to_string());
    assert_eq!(bridge.exec_calls_len(), 0);
    assert_eq!(bridge.mcp_calls_len(), 0);
    assert_eq!(
        bridge.validation_calls_len(),
        0,
        "task failed must clear validation_calls"
    );
    assert_eq!(bridge.approval_kinds_len(), 0);

    // Cancelled path: all maps cleared.
    let _ = bridge.project_exec_command_started("e-clear-3", &["ls".to_string()]);
    let _ = bridge.project_mcp_tool_started("m-clear-3", "srv", "tool");
    let _ = bridge
        .project_exec_command_started("v-clear-3", &["cargo".to_string(), "clippy".to_string()]);
    let _ = bridge.project_exec_approval_request("a-clear-3", &["true".to_string()], None);
    let _ = bridge.project_task_cancelled(Some("interrupted".to_string()));
    assert_eq!(bridge.exec_calls_len(), 0);
    assert_eq!(bridge.mcp_calls_len(), 0);
    assert_eq!(
        bridge.validation_calls_len(),
        0,
        "task cancelled must clear validation_calls"
    );
    assert_eq!(bridge.approval_kinds_len(), 0);
}

#[test]
fn approval_labels_are_kind_first() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let exec_req = bridge.project_exec_approval_request("a-exec", &["ls".to_string()], None);
    assert_eq!(
        verbose_runtime_history_label(&exec_req[0]),
        "vac runtime: approval requested — exec"
    );
    let patch_req = bridge.project_apply_patch_approval_request(
        "a-patch",
        vec![PathBuf::from("src/lib.rs")],
        None,
    );
    assert_eq!(
        verbose_runtime_history_label(&patch_req[0]),
        "vac runtime: approval requested — patch"
    );
    let perm_req = bridge.project_permissions_approval_request("a-perm", "summary", None);
    assert_eq!(
        verbose_runtime_history_label(&perm_req[0]),
        "vac runtime: approval requested — permissions"
    );
    let elic_req = bridge.project_mcp_elicitation_request("a-elic", "server-x", "please confirm");
    assert_eq!(
        verbose_runtime_history_label(&elic_req[0]),
        "vac runtime: approval requested — mcp elicitation"
    );
    // Bridge-aware resolved label pairs the requested kind with the decision.
    let request_id = match &exec_req[0] {
        RuntimeEvent::ApprovalRequested(r) => r.id,
        other => panic!("expected ApprovalRequested, got {other:?}"),
    };
    let resolved = RuntimeEvent::ApprovalResolved(ApprovalResolved::new(
        request_id,
        ApprovalDecision::Approved,
        None,
    ));
    assert_eq!(
        bridge.verbose_label_for(&resolved),
        "vac runtime: approval approved — exec"
    );
}

#[test]
fn approval_resolved_pairs_with_recorded_kind() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let req =
        bridge.project_exec_approval_request("a-pair", &["ls".to_string()], Some("need approval"));
    let request_id = match &req[0] {
        RuntimeEvent::ApprovalRequested(r) => r.id,
        other => panic!("expected ApprovalRequested, got {other:?}"),
    };
    // bridge-less label has no kind context; falls back to "— request".
    let resolved_event = RuntimeEvent::ApprovalResolved(ApprovalResolved::new(
        request_id,
        ApprovalDecision::Approved,
        None,
    ));
    assert_eq!(
        verbose_runtime_history_label(&resolved_event),
        "vac runtime: approval approved — request"
    );
    // bridge-aware label keeps the kind so the resolved row pairs with the
    // requested row using the same em-dash grammar.
    assert_eq!(
        bridge.verbose_label_for(&resolved_event),
        "vac runtime: approval approved — exec"
    );
}

#[test]
fn runtime_history_label_uses_compact_session_and_task_labels() {
    let mut bridge = fresh_bridge();
    let events = bridge.opening_events();

    let session_label = verbose_runtime_history_label(&events[0]);
    assert!(
        session_label.starts_with("vac runtime: session started — "),
        "got: {session_label}"
    );
    let id_part = session_label
        .strip_prefix("vac runtime: session started — ")
        .expect("prefix must match")
        .split(" · ")
        .next()
        .expect("split must yield short id");
    assert_eq!(
        id_part.chars().count(),
        8,
        "session label must use 8-char short id, got: {id_part}"
    );
    assert!(
        session_label.contains(" · tui · assist"),
        "got: {session_label}"
    );

    let task_label = verbose_runtime_history_label(&events[1]);
    assert_eq!(task_label, "vac runtime: task started — semantic coding");
}

#[test]
fn exec_failed_label_uses_started_command_preview() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let _ = bridge.project_exec_command_started("call-fail", &["false".to_string()]);
    let finished = bridge.project_exec_command_finished("call-fail", &[], 1);
    assert_eq!(
        verbose_runtime_history_label(&finished[0]),
        "vac runtime: exec failed — false",
    );
}

#[test]
fn command_preview_strips_shell_wrapper() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let started = bridge.project_exec_command_started(
        "shell-wrapper",
        &[
            "/bin/bash".to_string(),
            "-lc".to_string(),
            "ls docs/".to_string(),
        ],
    );
    assert_eq!(
        verbose_runtime_history_label(&started[0]),
        "vac runtime: exec started — ls docs/",
    );
}

#[test]
fn command_preview_redacts_obvious_secrets() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let started = bridge.project_exec_command_started(
        "secret",
        &[
            "curl".to_string(),
            "--token".to_string(),
            "abc123".to_string(),
            "API_KEY=xyz".to_string(),
        ],
    );
    let label = verbose_runtime_history_label(&started[0]);
    assert!(
        label.contains("--token [redacted]"),
        "flag-arg pair must redact, got: {label}"
    );
    assert!(
        label.contains("API_KEY=[redacted]"),
        "assignment must redact, got: {label}"
    );
    assert!(
        !label.contains("abc123"),
        "raw token must not leak: {label}"
    );
    assert!(!label.contains("xyz"), "raw secret must not leak: {label}");
}

#[test]
fn validation_detector_handles_shell_wrapped_commands() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let started = bridge.project_exec_command_started(
        "v-shell",
        &[
            "/bin/bash".to_string(),
            "-lc".to_string(),
            "cargo check -p vac-surface-tui".to_string(),
        ],
    );
    assert!(
        matches!(started[0], RuntimeEvent::ValidationStarted(_)),
        "shell-wrapped cargo check must classify as validation"
    );
    assert_eq!(
        verbose_runtime_history_label(&started[0]),
        "vac runtime: validation started — cargo check -p vac-surface-tui",
    );
}

#[test]
fn task_interrupted_label_is_compact() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let events = bridge.project_task_cancelled(Some("interrupted".to_string()));
    assert_eq!(
        verbose_runtime_history_label(&events[0]),
        "vac runtime: task interrupted",
    );
}

// ----- 00D-7 activity-mode tests -----
//
// These tests pin the user-facing "VAC activity:" rows that show in the
// root TUI by default, plus the suppression of noisy events
// (SessionStarted, AssistantDelta, ToolCallStarted(exec)). They call the
// pure helpers `activity_history_label` / `activity_label_for` directly
// so they are independent of the `VAC_RUNTIME_VERBOSE` env variable and
// safe to run in parallel with the verbose suite.

#[test]
fn activity_mode_hides_session_started_and_assistant_streaming() {
    let mut bridge = fresh_bridge();
    let opening = bridge.opening_events();
    // First opening event is SessionStarted, which must be suppressed.
    assert!(matches!(opening[0], RuntimeEvent::SessionStarted(_)));
    assert_eq!(activity_history_label(&opening[0]), None);

    // Second opening event is TaskStarted, which must show up.
    assert!(matches!(opening[1], RuntimeEvent::TaskStarted(_)));
    assert_eq!(
        activity_history_label(&opening[1]).as_deref(),
        Some("VAC activity: task started — semantic coding"),
    );

    // AssistantDelta is suppressed in activity mode (the assistant body
    // is rendered by the existing chat surface; we don't double up here).
    let delta = bridge.project_assistant_delta("hello");
    assert_eq!(delta.len(), 1);
    assert_eq!(activity_history_label(&delta[0]), None);
}

#[test]
fn activity_mode_hides_exec_started_and_shows_command_completed() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();

    let started = bridge
        .project_exec_command_started("call-activity", &["ls".to_string(), "-la".to_string()]);
    // exec started is suppressed: the existing `Explored`/tool cell is
    // already showing this; an extra row would be redundant.
    assert!(matches!(started[0], RuntimeEvent::ToolCallStarted(_)));
    assert_eq!(activity_history_label(&started[0]), None);

    let finished = bridge.project_exec_command_finished("call-activity", &[], 0);
    assert_eq!(
        activity_history_label(&finished[0]).as_deref(),
        Some("VAC activity: command completed — ls -la"),
    );
}

#[test]
fn activity_mode_command_failed_uses_started_preview() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();

    let _ = bridge.project_exec_command_started("call-fail", &["false".to_string()]);
    let finished = bridge.project_exec_command_finished("call-fail", &[], 1);
    assert_eq!(
        activity_history_label(&finished[0]).as_deref(),
        Some("VAC activity: command failed — false"),
    );
}

#[test]
fn activity_mode_check_lifecycle_uses_check_grammar() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();

    let started = bridge.project_exec_command_started(
        "v-activity",
        &[
            "cargo".to_string(),
            "check".to_string(),
            "-p".to_string(),
            "vac-tui".to_string(),
        ],
    );
    assert!(matches!(started[0], RuntimeEvent::ValidationStarted(_)));
    assert_eq!(
        activity_history_label(&started[0]).as_deref(),
        Some("VAC activity: check started — cargo check -p vac-surface-tui"),
    );

    let finished = bridge.project_exec_command_finished("v-activity", &[], 0);
    assert!(matches!(finished[0], RuntimeEvent::ValidationFinished(_)));
    assert_eq!(
        activity_history_label(&finished[0]).as_deref(),
        Some("VAC activity: check passed — cargo check -p vac-surface-tui"),
    );
}

#[test]
fn activity_mode_approval_requested_uses_kind_first_grammar() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();

    let exec_req = bridge.project_exec_approval_request("a-activity", &["ls".to_string()], None);
    assert_eq!(
        activity_history_label(&exec_req[0]).as_deref(),
        Some("VAC activity: approval requested — exec"),
    );
}

#[test]
fn activity_mode_approval_resolved_pairs_with_recorded_kind() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();

    let exec_req = bridge.project_exec_approval_request("a-pair", &["ls".to_string()], None);
    let request_id = match &exec_req[0] {
        RuntimeEvent::ApprovalRequested(r) => r.id,
        other => panic!("expected ApprovalRequested, got {other:?}"),
    };
    let resolved = RuntimeEvent::ApprovalResolved(ApprovalResolved::new(
        request_id,
        ApprovalDecision::Approved,
        None,
    ));

    // Bridge-aware activity label keeps the kind context so the resolved
    // row reads "approved — exec", not "approved — request".
    assert_eq!(
        bridge.activity_label_for(&resolved).as_deref(),
        Some("VAC activity: approval approved — exec"),
    );

    // Without the bridge context, the bare helper falls back to
    // "— request" (the kind is unknown).
    assert_eq!(
        activity_history_label(&resolved).as_deref(),
        Some("VAC activity: approval approved — request"),
    );
}

#[test]
fn activity_mode_task_completed_and_interrupted_use_activity_prefix() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();
    let completed = bridge.project_task_completed();
    assert_eq!(
        activity_history_label(&completed[0]).as_deref(),
        Some("VAC activity: task completed"),
    );

    let mut bridge2 = fresh_bridge();
    let _ = bridge2.opening_events();
    let interrupted = bridge2.project_task_cancelled(Some("interrupted".to_string()));
    assert_eq!(
        activity_history_label(&interrupted[0]).as_deref(),
        Some("VAC activity: task interrupted"),
    );
}

#[test]
fn activity_mode_mcp_lifecycle_uses_mcp_grammar() {
    let mut bridge = fresh_bridge();
    let _ = bridge.opening_events();

    let started = bridge.project_mcp_tool_started("m-act", "ramp", "list_invoices");
    assert_eq!(
        activity_history_label(&started[0]).as_deref(),
        Some("VAC activity: mcp started — ramp/list_invoices"),
    );

    let finished = bridge.project_mcp_tool_finished("m-act", "ramp", "list_invoices", true);
    assert_eq!(
        activity_history_label(&finished[0]).as_deref(),
        Some("VAC activity: mcp completed — ramp/list_invoices"),
    );
}
