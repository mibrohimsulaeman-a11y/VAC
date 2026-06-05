#![cfg(feature = "full-cli")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use vac_core::control_plane::{
    AcceptanceCriterion, SpecArtifactState, TaskArtifactState, TodoArtifactState,
    load_session_bundle, load_session_doctor_report, write_session_artifacts,
};

fn vac_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_vac")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/vac"))
}

fn make_temp_root(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vac-session-artifacts-{name}-{unique}"));
    fs::create_dir_all(&root).expect("create temp root");
    root
}

fn run_vac(args: &[&str]) -> std::process::Output {
    Command::new(vac_bin())
        .args(args)
        .output()
        .expect("run vac command")
}

#[test]
fn session_status_reports_close_state_after_discussion_closeout() {
    let root = make_temp_root("discussion-closeout");

    let start = run_vac(&[
        "session",
        "start",
        "session-700",
        "--workspace",
        root.to_str().expect("root path"),
        "--force",
        "--problem",
        "status closeout",
    ]);
    let start_stdout = String::from_utf8_lossy(&start.stdout);
    let start_stderr = String::from_utf8_lossy(&start.stderr);
    assert!(
        start.status.success(),
        "stdout:\n{start_stdout}\nstderr:\n{start_stderr}"
    );

    let mut bundle = load_session_bundle(&root, "session-700").expect("bundle");
    bundle.task.state = TaskArtifactState::NeedsDiscussion;
    bundle.task.acceptance_criteria.push(AcceptanceCriterion {
        id: "ac.1".to_string(),
        text: "needs evidence".to_string(),
        met: true,
        evidence: None,
    });
    bundle.spec.state = SpecArtifactState::NeedsDiscussion;
    bundle.todo.state = TodoArtifactState::NeedsDiscussion;
    write_session_artifacts(&root, &bundle).expect("rewrite bundle");

    let close = run_vac(&[
        "session",
        "close",
        "session-700",
        "--workspace",
        root.to_str().expect("root path"),
    ]);
    let close_stdout = String::from_utf8_lossy(&close.stdout);
    let close_stderr = String::from_utf8_lossy(&close.stderr);
    assert!(
        !close.status.success(),
        "stdout:\n{close_stdout}\nstderr:\n{close_stderr}"
    );
    assert!(
        close_stdout.contains("close_state: paused_for_discussion"),
        "stdout:\n{close_stdout}"
    );

    let status = run_vac(&[
        "session",
        "status",
        "session-700",
        "--workspace",
        root.to_str().expect("root path"),
    ]);
    let status_stdout = String::from_utf8_lossy(&status.stdout);
    let status_stderr = String::from_utf8_lossy(&status.stderr);
    assert!(
        status.status.success(),
        "stdout:\n{status_stdout}\nstderr:\n{status_stderr}"
    );
    assert!(
        status_stdout.contains("vac session status: PASS"),
        "stdout:\n{status_stdout}"
    );
    assert!(
        status_stdout.contains("close_state: paused_for_discussion"),
        "stdout:\n{status_stdout}"
    );

    let report = load_session_doctor_report(&root);
    assert_eq!(report.cli_exit_code(), 0);
    assert!(
        report
            .render_text()
            .contains("close: paused_for_discussion"),
        "{}",
        report.render_text()
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn doctor_sessions_reports_close_state_for_discussion_closeout() {
    let root = make_temp_root("discussion-doctor");

    let start = run_vac(&[
        "session",
        "start",
        "session-701",
        "--workspace",
        root.to_str().expect("root path"),
        "--force",
        "--problem",
        "doctor closeout",
    ]);
    let start_stdout = String::from_utf8_lossy(&start.stdout);
    let start_stderr = String::from_utf8_lossy(&start.stderr);
    assert!(
        start.status.success(),
        "stdout:\n{start_stdout}\nstderr:\n{start_stderr}"
    );

    let mut bundle = load_session_bundle(&root, "session-701").expect("bundle");
    bundle.task.state = TaskArtifactState::NeedsDiscussion;
    bundle.task.acceptance_criteria.push(AcceptanceCriterion {
        id: "ac.1".to_string(),
        text: "needs evidence".to_string(),
        met: true,
        evidence: None,
    });
    bundle.spec.state = SpecArtifactState::NeedsDiscussion;
    bundle.todo.state = TodoArtifactState::NeedsDiscussion;
    write_session_artifacts(&root, &bundle).expect("rewrite bundle");

    let close = run_vac(&[
        "session",
        "close",
        "session-701",
        "--workspace",
        root.to_str().expect("root path"),
    ]);
    let close_stdout = String::from_utf8_lossy(&close.stdout);
    let close_stderr = String::from_utf8_lossy(&close.stderr);
    assert!(
        !close.status.success(),
        "stdout:\n{close_stdout}\nstderr:\n{close_stderr}"
    );

    let doctor = run_vac(&["doctor", "sessions", root.to_str().expect("root path")]);
    let doctor_stdout = String::from_utf8_lossy(&doctor.stdout);
    let doctor_stderr = String::from_utf8_lossy(&doctor.stderr);
    assert!(
        doctor.status.success(),
        "stdout:\n{doctor_stdout}\nstderr:\n{doctor_stderr}"
    );
    assert!(
        doctor_stdout.contains("vac doctor sessions: PASS"),
        "stdout:\n{doctor_stdout}"
    );
    assert!(
        doctor_stdout.contains("close: paused_for_discussion"),
        "stdout:\n{doctor_stdout}"
    );
    assert!(
        doctor_stdout.contains("- session-701 [paused_for_discussion]"),
        "stdout:\n{doctor_stdout}"
    );

    let _ = fs::remove_dir_all(root);
}
