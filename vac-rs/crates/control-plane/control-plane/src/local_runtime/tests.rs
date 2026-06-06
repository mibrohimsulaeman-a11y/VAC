use std::path::PathBuf;

use pretty_assertions::assert_eq;

use super::ApprovalAction;
use super::ApprovalId;
use super::ApprovalPreview;
use super::ApprovalRequest;
use super::ApprovalResource;
use super::AssistantDelta;
use super::AutonomyMode;
use super::PreviewKind;
use super::RiskLevel;
use super::RuntimeCommand;
use super::RuntimeEntrypoint;
use super::RuntimeError;
use super::RuntimeErrorCode;
use super::RuntimeErrorOwner;
use super::RuntimeEvent;
use super::RuntimeSession;
use super::RuntimeSessionStatus;
use super::RuntimeTask;
use super::RuntimeTaskKind;
use super::RuntimeTaskStatus;
use super::SessionId;
use super::StartTask;
use super::TaskCancelled;
use super::TaskCompleted;
use super::TaskFailed;
use super::TaskId;
use super::TaskStarted;
use super::ToolCallFinished;
use super::ToolCallId;
use super::ToolCallStarted;
use super::ValidationFinished;
use super::ValidationStarted;
use super::ValidationStatus;

#[test]
fn local_runtime_start_task_command_builds_expected_payload() {
    let command = RuntimeCommand::start_task(
        "Fix the failing tests",
        AutonomyMode::Autopilot,
        RuntimeEntrypoint::Exec,
        PathBuf::from("/repo"),
    );

    let RuntimeCommand::StartTask(start_task) = command else {
        panic!("expected start task command");
    };

    let start_task_for_debug = start_task.clone();
    assert_eq!(
        start_task,
        StartTask::new(
            "Fix the failing tests",
            AutonomyMode::Autopilot,
            RuntimeEntrypoint::Exec,
            PathBuf::from("/repo"),
        )
    );
    assert!(format!("{:?}", RuntimeCommand::StartTask(start_task_for_debug)).contains("StartTask"));
    assert_eq!(start_task.entrypoint.to_string(), "exec");
    assert_eq!(start_task.autonomy_mode.to_string(), "autopilot");
}

#[test]
fn local_runtime_task_started_event_keeps_task_snapshot() {
    let session_id =
        SessionId::from_string("11111111-1111-1111-1111-111111111111").expect("valid session id");
    let task = RuntimeTask {
        id: TaskId::from_string("22222222-2222-2222-2222-222222222222").expect("valid task id"),
        session_id,
        kind: RuntimeTaskKind::SemanticCoding,
        prompt: "Fix the auth module".to_string(),
        status: RuntimeTaskStatus::Running,
    };
    let event = RuntimeEvent::TaskStarted(TaskStarted::new(task.clone()));

    assert_eq!(event, RuntimeEvent::TaskStarted(TaskStarted { task }));
    assert!(format!("{event:?}").contains("TaskStarted"));

    let session = RuntimeSession {
        id: session_id,
        created_at: chrono::DateTime::<chrono::Utc>::from_timestamp(1_700_000_000, 0)
            .expect("timestamp should be valid"),
        cwd: PathBuf::from("/repo"),
        entrypoint: RuntimeEntrypoint::Tui,
        autonomy_mode: AutonomyMode::Assist,
        status: RuntimeSessionStatus::Active,
    };
    let session_event = RuntimeEvent::SessionStarted(super::SessionStarted::new(session.clone()));
    assert!(matches!(session_event, RuntimeEvent::SessionStarted(_)));
    assert_eq!(session.status, RuntimeSessionStatus::Active);
}

#[test]
fn local_runtime_safe_edit_approval_request_uses_product_terms() {
    let approval_id =
        ApprovalId::from_string("33333333-3333-3333-3333-333333333333").expect("valid approval id");
    let task_id =
        TaskId::from_string("44444444-4444-4444-4444-444444444444").expect("valid task id");
    let request = ApprovalRequest::safe_edit(
        approval_id,
        task_id,
        "VAC needs to update two files",
        vec![
            ApprovalResource::File(PathBuf::from("src/auth.rs")),
            ApprovalResource::File(PathBuf::from("tests/auth.rs")),
        ],
        ApprovalPreview::Diff("diff --git a/src/auth.rs b/src/auth.rs".to_string()),
        vec!["cargo test -p auth".to_string()],
    );

    assert_eq!(request.action, ApprovalAction::WriteFiles);
    assert_eq!(request.risk, RiskLevel::SafeEdit);
    assert_eq!(request.preview.kind(), PreviewKind::Diff);
    assert_eq!(
        request.validation_after,
        vec!["cargo test -p auth".to_string()]
    );
    assert_eq!(request.resources.len(), 2);
    assert_eq!(request.action.to_string(), "write_files");
    assert_eq!(request.risk.to_string(), "safe_edit");
}

#[test]
fn local_runtime_validation_finished_reports_pass_and_fail() {
    let task_id =
        TaskId::from_string("55555555-5555-5555-5555-555555555555").expect("valid task id");
    let passed = ValidationFinished::new(
        task_id,
        "cargo test -p vac-core",
        ValidationStatus::Passed,
        Some("all tests passed".to_string()),
    );
    let failed = ValidationFinished::new(
        task_id,
        "cargo test -p vac-core",
        ValidationStatus::Failed,
        Some("one test failed".to_string()),
    );

    assert_eq!(passed.status, ValidationStatus::Passed);
    assert_eq!(failed.status, ValidationStatus::Failed);
    assert_eq!(passed.command_display, "cargo test -p vac-core");
    assert_eq!(passed.status.to_string(), "passed");
    assert_eq!(failed.status.to_string(), "failed");

    let started = ValidationStarted::new(task_id, "cargo test -p vac-core");
    assert_eq!(started.command_display, "cargo test -p vac-core");
}

#[test]
fn local_runtime_error_and_completion_types_are_operator_friendly() {
    let task_id =
        TaskId::from_string("66666666-6666-6666-6666-666666666666").expect("valid task id");
    let error = RuntimeError::new(
        RuntimeErrorCode::ValidationFailed,
        "validation failed",
        Some("re-run the test command".to_string()),
        true,
        RuntimeErrorOwner::Internal,
    );
    let failed = TaskFailed::new(task_id, error.clone());
    let completed = TaskCompleted::new(
        task_id,
        Some("task completed".to_string()),
        vec!["cargo test -p vac-core".to_string()],
    );
    let cancelled = TaskCancelled::new(task_id, Some("user cancelled".to_string()));
    let assistant_delta = AssistantDelta::new(task_id, "working through the failure");
    let tool_call_id = ToolCallId::from_string("77777777-7777-7777-7777-777777777777")
        .expect("valid tool call id");
    let tool_started = ToolCallStarted::new(
        task_id,
        tool_call_id,
        "shell",
        Some("cargo test".to_string()),
    );
    let tool_finished =
        ToolCallFinished::new(task_id, tool_call_id, "shell", Some("ok".to_string()), true);

    assert_eq!(error.owner, RuntimeErrorOwner::Internal);
    assert_eq!(error.retry_safe, true);
    assert!(format!("{failed:?}").contains("TaskFailed"));
    assert!(format!("{completed:?}").contains("TaskCompleted"));
    assert!(format!("{cancelled:?}").contains("TaskCancelled"));
    assert!(format!("{assistant_delta:?}").contains("AssistantDelta"));
    assert!(format!("{tool_started:?}").contains("ToolCallStarted"));
    assert!(format!("{tool_finished:?}").contains("ToolCallFinished"));
    assert_eq!(RuntimeEntrypoint::Workflow.to_string(), "workflow");
    assert_eq!(AutonomyMode::Suggest.to_string(), "suggest");
}

#[test]
fn runtime_task_with_id_preserves_id() {
    use super::{RuntimeTask, RuntimeTaskKind, RuntimeTaskStatus, SessionId, TaskId};
    let task_id = TaskId::new();
    let session_id = SessionId::new();
    let task = RuntimeTask::with_id(
        task_id,
        session_id,
        RuntimeTaskKind::SemanticCoding,
        "explore the repo",
    );
    assert_eq!(task.id, task_id);
    assert_eq!(task.session_id, session_id);
    assert_eq!(task.status, RuntimeTaskStatus::Pending);
}

#[test]
fn runtime_task_fsm_happy_path_runs_to_completion() {
    use super::{RuntimeTask, RuntimeTaskKind, RuntimeTaskStatus, SessionId};
    let mut task = RuntimeTask::new(
        SessionId::new(),
        RuntimeTaskKind::SemanticCoding,
        "happy path",
    );
    assert_eq!(task.status, RuntimeTaskStatus::Pending);
    task.start().expect("pending -> running");
    assert_eq!(task.status, RuntimeTaskStatus::Running);
    task.await_approval().expect("running -> waiting_approval");
    assert_eq!(task.status, RuntimeTaskStatus::WaitingApproval);
    task.resume_from_approval()
        .expect("waiting_approval -> running");
    assert_eq!(task.status, RuntimeTaskStatus::Running);
    task.complete().expect("running -> completed");
    assert_eq!(task.status, RuntimeTaskStatus::Completed);
}

#[test]
fn runtime_task_fsm_rejects_invalid_transition() {
    use super::{RuntimeTask, RuntimeTaskKind, RuntimeTaskStatus, SessionId};
    let mut task = RuntimeTask::new(SessionId::new(), RuntimeTaskKind::Apply, "invalid");
    task.start().unwrap();
    task.complete().unwrap();
    let err = task.start().expect_err("cannot restart a completed task");
    assert_eq!(err.kind, "task");
    assert_eq!(err.from, "completed");
    assert_eq!(err.to, "running");
    assert_eq!(task.status, RuntimeTaskStatus::Completed);
}

#[test]
fn runtime_session_with_id_preserves_id_and_created_at() {
    use super::{AutonomyMode, RuntimeEntrypoint, RuntimeSession, RuntimeSessionStatus, SessionId};
    use chrono::TimeZone;
    let session_id = SessionId::new();
    let created_at = chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let session = RuntimeSession::with_id(
        session_id,
        created_at,
        std::path::PathBuf::from("/tmp/repo"),
        RuntimeEntrypoint::Tui,
        AutonomyMode::Suggest,
    );
    assert_eq!(session.id, session_id);
    assert_eq!(session.created_at, created_at);
    assert_eq!(session.status, RuntimeSessionStatus::Active);
}

#[test]
fn runtime_session_fsm_happy_path() {
    use super::{AutonomyMode, RuntimeEntrypoint, RuntimeSession, RuntimeSessionStatus};
    let mut session = RuntimeSession::new(
        std::path::PathBuf::from("/tmp"),
        RuntimeEntrypoint::Exec,
        AutonomyMode::Assist,
    );
    assert_eq!(session.status, RuntimeSessionStatus::Active);
    session
        .await_approval()
        .expect("active -> waiting_approval");
    assert_eq!(session.status, RuntimeSessionStatus::WaitingApproval);
    session
        .resume_from_approval()
        .expect("waiting_approval -> active");
    assert_eq!(session.status, RuntimeSessionStatus::Active);
    session.complete().expect("active -> completed");
    assert_eq!(session.status, RuntimeSessionStatus::Completed);
}

#[test]
fn runtime_session_fsm_rejects_terminal_transition() {
    use super::{AutonomyMode, RuntimeEntrypoint, RuntimeSession};
    let mut session = RuntimeSession::new(
        std::path::PathBuf::from("/tmp"),
        RuntimeEntrypoint::Workflow,
        AutonomyMode::Autopilot,
    );
    session.cancel().unwrap();
    let err = session
        .resume_from_approval()
        .expect_err("cannot resume a cancelled session");
    assert_eq!(err.kind, "session");
    assert_eq!(err.from, "cancelled");
    assert_eq!(err.to, "active");
}
