//! Round-trip serde tests + insta golden snapshots for the Local Runtime
//! Contract DTOs (`RuntimeCommand`, `RuntimeEvent`, `ApprovalRequest`).
//!
//! These tests guard the wire format used by `vac-exec` and `vac-tui` to
//! observe the local runtime. Adding a new variant should add a matching
//! round-trip case here so we catch accidental schema breaks.

use std::path::PathBuf;

use pretty_assertions::assert_eq;
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::ApprovalDecision;
use super::ApprovalId;
use super::ApprovalPreview;
use super::ApprovalRequest;
use super::ApprovalResolved;
use super::ApprovalResource;
use super::AssistantDelta;
use super::AutonomyMode;
use super::EnteredReviewMode;
use super::ExitedReviewMode;
use super::ReviewTarget;
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
use super::SessionEnded;
use super::SessionId;
use super::SessionStarted;
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
use super::WorkflowId;

fn fixed_session_id() -> SessionId {
    SessionId::from_string("11111111-1111-1111-1111-111111111111").expect("valid session id")
}

fn fixed_task_id() -> TaskId {
    TaskId::from_string("22222222-2222-2222-2222-222222222222").expect("valid task id")
}

fn fixed_approval_id() -> ApprovalId {
    ApprovalId::from_string("33333333-3333-3333-3333-333333333333").expect("valid approval id")
}

fn fixed_tool_call_id() -> ToolCallId {
    ToolCallId::from_string("44444444-4444-4444-4444-444444444444").expect("valid tool call id")
}

fn fixed_workflow_id() -> WorkflowId {
    WorkflowId::from_string("55555555-5555-5555-5555-555555555555").expect("valid workflow id")
}

fn fixed_session() -> RuntimeSession {
    RuntimeSession {
        id: fixed_session_id(),
        created_at: chrono::DateTime::<chrono::Utc>::from_timestamp(1_700_000_000, 0)
            .expect("timestamp should be valid"),
        cwd: PathBuf::from("/repo"),
        entrypoint: RuntimeEntrypoint::Tui,
        autonomy_mode: AutonomyMode::Assist,
        status: RuntimeSessionStatus::Active,
    }
}

fn fixed_task() -> RuntimeTask {
    RuntimeTask {
        id: fixed_task_id(),
        session_id: fixed_session_id(),
        kind: RuntimeTaskKind::SemanticCoding,
        prompt: "Fix the auth module".to_string(),
        status: RuntimeTaskStatus::Running,
    }
}

fn fixed_approval_request() -> ApprovalRequest {
    ApprovalRequest::safe_edit(
        fixed_approval_id(),
        fixed_task_id(),
        "VAC needs to update two files",
        vec![
            ApprovalResource::File(PathBuf::from("src/auth.rs")),
            ApprovalResource::File(PathBuf::from("tests/auth.rs")),
        ],
        ApprovalPreview::Diff("diff --git a/src/auth.rs b/src/auth.rs".to_string()),
        vec!["cargo test -p auth".to_string()],
    )
}

fn fixed_runtime_error() -> RuntimeError {
    RuntimeError::new(
        RuntimeErrorCode::ValidationFailed,
        "validation failed",
        Some("re-run the test command".to_string()),
        true,
        RuntimeErrorOwner::Internal,
    )
}

fn round_trip<T>(value: &T) -> T
where
    T: Serialize + DeserializeOwned,
{
    let json = serde_json::to_string(value).expect("serialize");
    serde_json::from_str(&json).expect("deserialize")
}

#[test]
fn runtime_command_round_trips_each_variant() {
    let commands = vec![
        RuntimeCommand::start_task(
            "Fix the failing tests",
            AutonomyMode::Autopilot,
            RuntimeEntrypoint::Exec,
            PathBuf::from("/repo"),
        ),
        RuntimeCommand::start_review(
            ReviewTarget::UncommittedChanges,
            None,
            AutonomyMode::Suggest,
            RuntimeEntrypoint::Tui,
            PathBuf::from("/repo"),
        ),
        RuntimeCommand::start_review(
            ReviewTarget::BaseBranch {
                branch: "main".to_string(),
            },
            Some("review for security issues".to_string()),
            AutonomyMode::Autopilot,
            RuntimeEntrypoint::Exec,
            PathBuf::from("/repo"),
        ),
        RuntimeCommand::start_review(
            ReviewTarget::Commit {
                sha: "deadbeef".to_string(),
                title: Some("fix auth bug".to_string()),
            },
            Some("focus on error handling".to_string()),
            AutonomyMode::Assist,
            RuntimeEntrypoint::Exec,
            PathBuf::from("/repo"),
        ),
        RuntimeCommand::start_review(
            ReviewTarget::Custom {
                instructions: "look for SQL injection".to_string(),
            },
            None,
            AutonomyMode::Suggest,
            RuntimeEntrypoint::Tui,
            PathBuf::from("/repo"),
        ),
        RuntimeCommand::CancelTask {
            task_id: fixed_task_id(),
        },
        RuntimeCommand::Approve {
            approval_id: fixed_approval_id(),
        },
        RuntimeCommand::Reject {
            approval_id: fixed_approval_id(),
        },
        RuntimeCommand::ResumeSession {
            session_id: fixed_session_id(),
        },
        RuntimeCommand::RunWorkflow {
            workflow_id: fixed_workflow_id(),
            input: "{\"prompt\":\"hello\"}".to_string(),
        },
    ];
    for cmd in &commands {
        assert_eq!(*cmd, round_trip(cmd));
    }
}

#[test]
fn runtime_event_round_trips_each_variant() {
    let events = vec![
        RuntimeEvent::SessionStarted(SessionStarted::new(fixed_session())),
        RuntimeEvent::TaskStarted(TaskStarted::new(fixed_task())),
        RuntimeEvent::AssistantDelta(AssistantDelta::new(fixed_task_id(), "working...")),
        RuntimeEvent::ToolCallStarted(ToolCallStarted::new(
            fixed_task_id(),
            fixed_tool_call_id(),
            "shell",
            Some("cargo test".to_string()),
        )),
        RuntimeEvent::ToolCallFinished(ToolCallFinished::new(
            fixed_task_id(),
            fixed_tool_call_id(),
            "shell",
            Some("ok".to_string()),
            true,
        )),
        RuntimeEvent::from(fixed_approval_request()),
        RuntimeEvent::ApprovalResolved(ApprovalResolved::new(
            fixed_approval_id(),
            ApprovalDecision::Approved,
            Some("looks good".to_string()),
        )),
        RuntimeEvent::ValidationStarted(ValidationStarted::new(
            fixed_task_id(),
            "cargo test -p vac-core",
        )),
        RuntimeEvent::ValidationFinished(ValidationFinished::new(
            fixed_task_id(),
            "cargo test -p vac-core",
            ValidationStatus::Passed,
            Some("all tests passed".to_string()),
        )),
        RuntimeEvent::TaskCompleted(TaskCompleted::new(
            fixed_task_id(),
            Some("task completed".to_string()),
            vec!["cargo test -p vac-core".to_string()],
        )),
        RuntimeEvent::TaskFailed(TaskFailed::new(fixed_task_id(), fixed_runtime_error())),
        RuntimeEvent::TaskCancelled(TaskCancelled::new(
            fixed_task_id(),
            Some("user cancelled".to_string()),
        )),
        RuntimeEvent::EnteredReviewMode(EnteredReviewMode::new(
            ReviewTarget::UncommittedChanges,
            Some("review uncommitted changes".to_string()),
        )),
        RuntimeEvent::EnteredReviewMode(EnteredReviewMode::new(
            ReviewTarget::Custom {
                instructions: "audit error paths".to_string(),
            },
            None,
        )),
        RuntimeEvent::ExitedReviewMode(ExitedReviewMode::new(Some("3 findings".to_string()))),
        RuntimeEvent::ExitedReviewMode(ExitedReviewMode::new(None::<String>)),
    ];
    for ev in &events {
        assert_eq!(*ev, round_trip(ev));
    }
}

#[test]
fn approval_request_round_trips_all_preview_kinds() {
    let base = fixed_approval_request();
    let previews = vec![
        ApprovalPreview::None,
        ApprovalPreview::Text("plain explanation".to_string()),
        ApprovalPreview::Diff("diff --git a b".to_string()),
        ApprovalPreview::Command("cargo run --release".to_string()),
        ApprovalPreview::FileList(vec![PathBuf::from("a.rs"), PathBuf::from("b.rs")]),
    ];
    for preview in previews {
        let req = ApprovalRequest {
            preview: preview.clone(),
            ..base.clone()
        };
        assert_eq!(req, round_trip(&req));
    }
}

#[test]
fn approval_request_round_trips_all_resource_kinds() {
    let req = ApprovalRequest {
        resources: vec![
            ApprovalResource::File(PathBuf::from("src/x.rs")),
            ApprovalResource::Command("ls -la".to_string()),
            ApprovalResource::Network("https://example.com".to_string()),
            ApprovalResource::Connector("github".to_string()),
            ApprovalResource::Config("policy.toml".to_string()),
            ApprovalResource::Other("misc".to_string()),
        ],
        ..fixed_approval_request()
    };
    assert_eq!(req, round_trip(&req));
}

// === insta golden snapshots ===
//
// These pin the serialized JSON shape of each contract surface so renaming a
// field or changing a serde tag is caught as a snapshot diff.

#[test]
fn golden_runtime_command_start_task() {
    insta::assert_json_snapshot!(RuntimeCommand::start_task(
        "Fix the failing tests",
        AutonomyMode::Autopilot,
        RuntimeEntrypoint::Exec,
        PathBuf::from("/repo"),
    ));
}

#[test]
fn golden_runtime_command_start_review() {
    insta::assert_json_snapshot!(RuntimeCommand::start_review(
        ReviewTarget::Commit {
            sha: "deadbeef".to_string(),
            title: Some("fix auth bug".to_string()),
        },
        Some("focus on error handling".to_string()),
        AutonomyMode::Assist,
        RuntimeEntrypoint::Exec,
        PathBuf::from("/repo"),
    ));
}

#[test]
fn golden_runtime_command_cancel_task() {
    insta::assert_json_snapshot!(RuntimeCommand::CancelTask {
        task_id: fixed_task_id(),
    });
}

#[test]
fn golden_runtime_command_approve() {
    insta::assert_json_snapshot!(RuntimeCommand::Approve {
        approval_id: fixed_approval_id(),
    });
}

#[test]
fn golden_runtime_command_reject() {
    insta::assert_json_snapshot!(RuntimeCommand::Reject {
        approval_id: fixed_approval_id(),
    });
}

#[test]
fn golden_runtime_command_resume_session() {
    insta::assert_json_snapshot!(RuntimeCommand::ResumeSession {
        session_id: fixed_session_id(),
    });
}

#[test]
fn golden_runtime_command_run_workflow() {
    insta::assert_json_snapshot!(RuntimeCommand::RunWorkflow {
        workflow_id: fixed_workflow_id(),
        input: "{\"prompt\":\"hello\"}".to_string(),
    });
}

#[test]
fn golden_runtime_event_session_started() {
    insta::assert_json_snapshot!(RuntimeEvent::SessionStarted(SessionStarted::new(
        fixed_session()
    )));
}

#[test]
fn golden_runtime_event_task_started() {
    insta::assert_json_snapshot!(RuntimeEvent::TaskStarted(TaskStarted::new(fixed_task())));
}

#[test]
fn golden_runtime_event_assistant_delta() {
    insta::assert_json_snapshot!(RuntimeEvent::AssistantDelta(AssistantDelta::new(
        fixed_task_id(),
        "working..."
    )));
}

#[test]
fn golden_runtime_event_tool_call_started() {
    insta::assert_json_snapshot!(RuntimeEvent::ToolCallStarted(ToolCallStarted::new(
        fixed_task_id(),
        fixed_tool_call_id(),
        "shell",
        Some("cargo test".to_string()),
    )));
}

#[test]
fn golden_runtime_event_tool_call_finished() {
    insta::assert_json_snapshot!(RuntimeEvent::ToolCallFinished(ToolCallFinished::new(
        fixed_task_id(),
        fixed_tool_call_id(),
        "shell",
        Some("ok".to_string()),
        true,
    )));
}

#[test]
fn golden_runtime_event_approval_requested() {
    insta::assert_json_snapshot!(RuntimeEvent::from(fixed_approval_request()));
}

#[test]
fn golden_runtime_event_approval_resolved() {
    insta::assert_json_snapshot!(RuntimeEvent::ApprovalResolved(ApprovalResolved::new(
        fixed_approval_id(),
        ApprovalDecision::Approved,
        Some("looks good".to_string()),
    )));
}

#[test]
fn golden_runtime_event_validation_started() {
    insta::assert_json_snapshot!(RuntimeEvent::ValidationStarted(ValidationStarted::new(
        fixed_task_id(),
        "cargo test -p vac-core",
    )));
}

#[test]
fn golden_runtime_event_validation_finished() {
    insta::assert_json_snapshot!(RuntimeEvent::ValidationFinished(ValidationFinished::new(
        fixed_task_id(),
        "cargo test -p vac-core",
        ValidationStatus::Passed,
        Some("all tests passed".to_string()),
    )));
}

#[test]
fn golden_runtime_event_task_completed() {
    insta::assert_json_snapshot!(RuntimeEvent::TaskCompleted(TaskCompleted::new(
        fixed_task_id(),
        Some("task completed".to_string()),
        vec!["cargo test -p vac-core".to_string()],
    )));
}

#[test]
fn golden_runtime_event_task_failed() {
    insta::assert_json_snapshot!(RuntimeEvent::TaskFailed(TaskFailed::new(
        fixed_task_id(),
        fixed_runtime_error(),
    )));
}

#[test]
fn golden_runtime_event_task_cancelled() {
    insta::assert_json_snapshot!(RuntimeEvent::TaskCancelled(TaskCancelled::new(
        fixed_task_id(),
        Some("user cancelled".to_string()),
    )));
}

#[test]
fn golden_approval_request() {
    insta::assert_json_snapshot!(fixed_approval_request());
}

#[test]
fn session_ended_round_trips() {
    let ev = RuntimeEvent::SessionEnded(SessionEnded::new(
        fixed_session_id(),
        RuntimeSessionStatus::Completed,
        Some("user finished".to_string()),
    ));
    assert_eq!(ev, round_trip(&ev));
}

#[test]
fn approval_decision_round_trips_all_variants() {
    let decisions = vec![
        ApprovalDecision::Approved,
        ApprovalDecision::Rejected,
        ApprovalDecision::Cancelled,
        ApprovalDecision::Timeout,
    ];
    for decision in decisions {
        assert_eq!(decision, round_trip(&decision));
    }
}

#[test]
fn golden_runtime_event_session_ended() {
    insta::assert_json_snapshot!(RuntimeEvent::SessionEnded(SessionEnded::new(
        fixed_session_id(),
        RuntimeSessionStatus::Completed,
        Some("user finished".to_string()),
    )));
}

#[test]
fn golden_approval_decision_cancelled() {
    insta::assert_json_snapshot!(ApprovalDecision::Cancelled);
}

#[test]
fn golden_approval_decision_timeout() {
    insta::assert_json_snapshot!(ApprovalDecision::Timeout);
}

#[test]
fn runtime_error_owner_round_trips_all_variants() {
    let variants = [
        RuntimeErrorOwner::User,
        RuntimeErrorOwner::Provider,
        RuntimeErrorOwner::Sandbox,
        RuntimeErrorOwner::Policy,
        RuntimeErrorOwner::Bridge,
        RuntimeErrorOwner::Internal,
    ];
    for variant in variants {
        let restored = round_trip(&variant);
        assert_eq!(variant, restored);
    }
}

#[test]
fn runtime_error_code_round_trips_all_variants() {
    let variants = [
        RuntimeErrorCode::BudgetLimited,
        RuntimeErrorCode::ApprovalRequired,
        RuntimeErrorCode::ValidationFailed,
        RuntimeErrorCode::Unknown("legacy_string_code".to_string()),
    ];
    for variant in variants {
        let restored = round_trip(&variant);
        assert_eq!(variant, restored);
    }
}

#[test]
fn golden_runtime_error_unknown_code() {
    let err = RuntimeError::new(
        RuntimeErrorCode::Unknown("legacy_string_code".to_string()),
        "legacy failure",
        None::<String>,
        false,
        RuntimeErrorOwner::Internal,
    );
    insta::assert_json_snapshot!(err);
}

#[test]
fn risk_level_orders_least_to_most_risky() {
    use super::RiskLevel;
    let ordered = [
        RiskLevel::ReadOnly,
        RiskLevel::SafeEdit,
        RiskLevel::BroadEdit,
        RiskLevel::Destructive,
        RiskLevel::Execute,
        RiskLevel::Network,
        RiskLevel::Credential,
        RiskLevel::Unknown,
    ];
    for w in ordered.windows(2) {
        assert!(w[0] < w[1], "expected {:?} < {:?}", w[0], w[1]);
    }
    let mut shuffled = vec![
        RiskLevel::Unknown,
        RiskLevel::ReadOnly,
        RiskLevel::Destructive,
        RiskLevel::SafeEdit,
        RiskLevel::Network,
        RiskLevel::BroadEdit,
        RiskLevel::Credential,
        RiskLevel::Execute,
    ];
    shuffled.sort();
    assert_eq!(shuffled.as_slice(), ordered.as_slice());
}

#[test]
fn approval_resource_display_format() {
    use std::path::PathBuf;
    assert_eq!(
        ApprovalResource::File(PathBuf::from("src/auth.rs")).to_string(),
        "file:src/auth.rs"
    );
    assert_eq!(
        ApprovalResource::Command("ls -la".to_string()).to_string(),
        "command:ls -la"
    );
    assert_eq!(
        ApprovalResource::Network("https://example.com".to_string()).to_string(),
        "network:https://example.com"
    );
    assert_eq!(
        ApprovalResource::Connector("github".to_string()).to_string(),
        "connector:github"
    );
    assert_eq!(
        ApprovalResource::Config("policy.toml".to_string()).to_string(),
        "config:policy.toml"
    );
    assert_eq!(
        ApprovalResource::Other("misc".to_string()).to_string(),
        "other:misc"
    );
}

#[test]
fn golden_runtime_command_start_review_custom() {
    insta::assert_json_snapshot!(RuntimeCommand::start_review(
        ReviewTarget::Custom {
            instructions: "look for SQL injection".to_string(),
        },
        None,
        AutonomyMode::Suggest,
        RuntimeEntrypoint::Tui,
        PathBuf::from("/repo"),
    ));
}

#[test]
fn golden_runtime_event_entered_review_mode_uncommitted() {
    insta::assert_json_snapshot!(RuntimeEvent::EnteredReviewMode(EnteredReviewMode::new(
        ReviewTarget::UncommittedChanges,
        Some("review uncommitted changes".to_string()),
    ),));
}

#[test]
fn golden_runtime_event_entered_review_mode_base() {
    insta::assert_json_snapshot!(RuntimeEvent::EnteredReviewMode(EnteredReviewMode::new(
        ReviewTarget::BaseBranch {
            branch: "main".to_string(),
        },
        None::<String>,
    ),));
}

#[test]
fn golden_runtime_event_entered_review_mode_commit() {
    insta::assert_json_snapshot!(RuntimeEvent::EnteredReviewMode(EnteredReviewMode::new(
        ReviewTarget::Commit {
            sha: "deadbeef".to_string(),
            title: Some("fix auth bug".to_string()),
        },
        Some("focus on error handling".to_string()),
    ),));
}

#[test]
fn golden_runtime_event_entered_review_mode_custom() {
    insta::assert_json_snapshot!(RuntimeEvent::EnteredReviewMode(EnteredReviewMode::new(
        ReviewTarget::Custom {
            instructions: "audit error paths".to_string(),
        },
        None::<String>,
    ),));
}

#[test]
fn golden_runtime_event_exited_review_mode_with_summary() {
    insta::assert_json_snapshot!(RuntimeEvent::ExitedReviewMode(ExitedReviewMode::new(Some(
        "3 findings".to_string()
    )),));
}

#[test]
fn golden_runtime_event_exited_review_mode_empty() {
    insta::assert_json_snapshot!(RuntimeEvent::ExitedReviewMode(ExitedReviewMode::new(
        None::<String>
    ),));
}
