//! Session doctor report for the Part V session registry.

use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use super::completion_lock::evaluate_completion_lock;
use super::completion_lock::session_close_state;
use super::session_artifacts::SessionArtifactState;
use super::session_artifacts::load_session_artifact_record;
use super::session_artifacts::load_session_bundle;
use super::session_artifacts::load_session_manifest;
use super::session_artifacts::session_artifact_paths;
use super::session_artifacts::session_registry_dir;
use super::session_closeout::SessionCloseoutRecord;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionDoctorSessionReport {
    pub session_id: String,
    pub manifest_state: Option<SessionArtifactState>,
    pub lock_state: Option<SessionArtifactState>,
    pub close_state: Option<SessionArtifactState>,
    pub lock_outcome: Option<super::completion_lock::CompletionLockOutcome>,
    pub has_manifest: bool,
    pub has_task: bool,
    pub has_spec: bool,
    pub has_todo: bool,
    pub has_close: bool,
    pub problems: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionDoctorReport {
    pub workspace_root: PathBuf,
    pub sessions: Vec<SessionDoctorSessionReport>,
    pub problems: Vec<String>,
}

impl SessionDoctorReport {
    pub fn cli_exit_code(&self) -> i32 {
        if self.problems.is_empty()
            && self
                .sessions
                .iter()
                .all(|session| session.problems.is_empty())
        {
            0
        } else {
            1
        }
    }

    pub fn is_failure(&self) -> bool {
        self.cli_exit_code() != 0
    }

    pub fn render_text(&self) -> String {
        let mut out = String::new();
        let status = if self.is_failure() { "FAIL" } else { "PASS" };
        let _ = writeln!(out, "vac doctor sessions: {status}");
        let _ = writeln!(out, "workspace: {}", self.workspace_root.display());
        let _ = writeln!(out, "sessions: {}", self.sessions.len());
        if self.sessions.is_empty() {
            let _ = writeln!(
                out,
                "tip: no sessions registered yet; run `vac session start <SESSION_ID>`"
            );
        }
        for session in &self.sessions {
            let status = session
                .close_state
                .or(session.lock_state)
                .or(session.manifest_state)
                .map(format_session_state)
                .unwrap_or("unknown");
            let _ = writeln!(out, "- {} [{}]", session.session_id, status);
            let _ = writeln!(
                out,
                "  files: manifest={} task={} spec={} todo={} close={}",
                yes_no(session.has_manifest),
                yes_no(session.has_task),
                yes_no(session.has_spec),
                yes_no(session.has_todo),
                yes_no(session.has_close),
            );
            if let Some(outcome) = session.lock_outcome {
                let _ = writeln!(out, "  lock: {}", format_lock_outcome(outcome));
            }
            if let Some(close_state) = session.close_state {
                let _ = writeln!(out, "  close: {}", format_session_state(close_state));
            }
            if let Some(manifest_state) = session.manifest_state {
                let _ = writeln!(out, "  manifest: {}", format_session_state(manifest_state));
            }
            for problem in &session.problems {
                let _ = writeln!(out, "  problem: {problem}");
            }
        }
        for problem in &self.problems {
            let _ = writeln!(out, "problem: {problem}");
        }
        out
    }
}

pub fn load_session_doctor_report(workspace_root: impl AsRef<Path>) -> SessionDoctorReport {
    let workspace_root = workspace_root.as_ref().to_path_buf();
    let sessions_root = session_registry_dir(&workspace_root);
    let mut report = SessionDoctorReport {
        workspace_root: workspace_root.clone(),
        sessions: Vec::new(),
        problems: Vec::new(),
    };

    if !sessions_root.is_dir() {
        return report;
    }

    let read_dir = match fs::read_dir(&sessions_root) {
        Ok(read_dir) => read_dir,
        Err(err) => {
            report
                .problems
                .push(format!("failed to read {}: {err}", sessions_root.display()));
            return report;
        }
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let session_id = entry.file_name().to_string_lossy().to_string();
        report
            .sessions
            .push(build_session_report(&workspace_root, &session_id));
    }
    report
        .sessions
        .sort_by(|left, right| left.session_id.cmp(&right.session_id));
    report
}

fn build_session_report(workspace_root: &Path, session_id: &str) -> SessionDoctorSessionReport {
    let mut report = SessionDoctorSessionReport {
        session_id: session_id.to_string(),
        manifest_state: None,
        lock_state: None,
        close_state: None,
        lock_outcome: None,
        has_manifest: false,
        has_task: false,
        has_spec: false,
        has_todo: false,
        has_close: false,
        problems: Vec::new(),
    };

    let paths = match session_artifact_paths(workspace_root, session_id) {
        Ok(paths) => paths,
        Err(err) => {
            report
                .problems
                .push(format!("invalid session path `{session_id}`: {err}"));
            return report;
        }
    };

    report.has_manifest = paths.session_manifest.exists();
    report.has_task = paths.task.exists();
    report.has_spec = paths.spec.exists();
    report.has_todo = paths.todo.exists();
    report.has_close = paths.close.exists();

    if report.has_manifest {
        match load_session_manifest(workspace_root, session_id) {
            Ok(manifest) => {
                report.manifest_state = Some(manifest.state);
            }
            Err(err) => report.problems.push(err),
        }
    } else {
        report.problems.push("missing session.yaml".to_string());
    }

    if report.has_task && report.has_spec && report.has_todo {
        match load_session_bundle(workspace_root, session_id) {
            Ok(bundle) => {
                let decision = evaluate_completion_lock(&bundle.task, &bundle.spec, &bundle.todo);
                report.lock_state = Some(session_close_state(&decision));
                report.lock_outcome = Some(decision.outcome);
                if report.manifest_state.is_some() && report.manifest_state != report.lock_state {
                    report.problems.push(format!(
                        "manifest state does not match lock evaluation: {:?} vs {:?}",
                        report.manifest_state, report.lock_state
                    ));
                }
            }
            Err(err) => report.problems.push(err),
        }
    } else {
        if !report.has_task {
            report.problems.push("missing task.yaml".to_string());
        }
        if !report.has_spec {
            report.problems.push("missing spec.yaml".to_string());
        }
        if !report.has_todo {
            report.problems.push("missing todo.yaml".to_string());
        }
    }

    if report.has_close {
        match load_session_artifact_record::<SessionCloseoutRecord>(workspace_root, &paths.close) {
            Ok(closeout) => {
                report.close_state = Some(closeout.state);
                if let Some(manifest_state) = report.manifest_state
                    && manifest_state != closeout.state
                {
                    report.problems.push(format!(
                        "closeout state does not match manifest state: {:?} vs {:?}",
                        closeout.state, manifest_state
                    ));
                }
                if let Some(lock_state) = report.lock_state
                    && lock_state != closeout.state
                {
                    report.problems.push(format!(
                        "closeout state does not match lock evaluation: {:?} vs {:?}",
                        closeout.state, lock_state
                    ));
                }
            }
            Err(err) => report.problems.push(err),
        }
    } else if matches!(
        report.manifest_state,
        Some(SessionArtifactState::PausedForDiscussion | SessionArtifactState::Done)
    ) {
        report
            .problems
            .push("missing close.yaml for a finalized session".to_string());
    }

    report
}

fn format_session_state(state: SessionArtifactState) -> &'static str {
    match state {
        SessionArtifactState::Open => "open",
        SessionArtifactState::PausedForDiscussion => "paused_for_discussion",
        SessionArtifactState::Done => "done",
    }
}

fn format_lock_outcome(outcome: super::completion_lock::CompletionLockOutcome) -> &'static str {
    match outcome {
        super::completion_lock::CompletionLockOutcome::Done => "done",
        super::completion_lock::CompletionLockOutcome::NeedsDiscussion => "needs_discussion",
        super::completion_lock::CompletionLockOutcome::Open => "open",
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control_plane::session_artifacts::{
        AcceptanceCriterion, SessionArtifactBundle, SpecArtifactState, TaskArtifactState,
        TodoArtifactState, write_session_artifacts,
    };
    use crate::control_plane::session_closeout::close_session_artifacts;
    use pretty_assertions::assert_eq;

    fn temp_root() -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("vac-session-doctor-{unique}"))
    }

    #[test]
    fn report_passes_for_open_session_with_complete_artifacts() {
        let root = temp_root();
        let bundle = SessionArtifactBundle::new(
            "session-003",
            "doctor",
            TaskArtifactState::Open,
            SpecArtifactState::Draft,
            TodoArtifactState::Open,
            "doctor problem",
        );
        write_session_artifacts(&root, &bundle).expect("write");

        let report = load_session_doctor_report(&root);
        assert_eq!(report.cli_exit_code(), 0);
        assert!(report.render_text().contains("vac doctor sessions: PASS"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn report_flags_missing_session_artifacts() {
        let root = temp_root();
        let session_dir = session_registry_dir(&root).join("broken-session");
        std::fs::create_dir_all(&session_dir).expect("session dir");

        let report = load_session_doctor_report(&root);
        assert_eq!(report.cli_exit_code(), 1);
        assert!(report.render_text().contains("missing session.yaml"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn report_shows_close_state_for_discussion_closeout() {
        let root = temp_root();
        let mut bundle = SessionArtifactBundle::new(
            "session-004",
            "doctor",
            TaskArtifactState::NeedsDiscussion,
            SpecArtifactState::NeedsDiscussion,
            TodoArtifactState::NeedsDiscussion,
            "doctor discussion",
        );
        bundle.task.acceptance_criteria.push(AcceptanceCriterion {
            id: "ac.1".to_string(),
            text: "needs evidence".to_string(),
            met: true,
            evidence: None,
        });

        close_session_artifacts(&root, &bundle).expect("close session");

        let report = load_session_doctor_report(&root);
        assert_eq!(report.cli_exit_code(), 0);
        assert!(
            report
                .render_text()
                .contains("close: paused_for_discussion")
        );
        assert!(
            report
                .render_text()
                .contains("- session-004 [paused_for_discussion]")
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
