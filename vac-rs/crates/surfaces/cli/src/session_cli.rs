use anyhow::{Result, bail};
use clap::Parser;
use clap::Subcommand;
use std::path::{Path, PathBuf};
use vac_core::control_plane::SessionArtifactBundle;
use vac_core::control_plane::SessionArtifactState;
use vac_core::control_plane::SpecArtifactState;
use vac_core::control_plane::TaskArtifactState;
use vac_core::control_plane::TodoArtifactState;
use vac_core::control_plane::close_session_artifacts;
use vac_core::control_plane::evaluate_completion_lock;
use vac_core::control_plane::load_session_artifact_record;
use vac_core::control_plane::load_session_bundle;
use vac_core::control_plane::load_session_manifest;
use vac_core::control_plane::session_artifact_paths;
use vac_core::control_plane::write_session_artifacts;

/// Manage VAC Part V session artifacts from the CLI.
#[derive(Debug, Parser)]
pub struct SessionCommand {
    #[command(subcommand)]
    command: SessionSubcommand,
}

impl SessionCommand {
    pub fn run(self) -> Result<()> {
        match self.command {
            SessionSubcommand::Start(command) => command.run(),
            SessionSubcommand::Status(command) => command.run(),
            SessionSubcommand::Lockcheck(command) => command.run(),
            SessionSubcommand::Close(command) => command.run(),
        }
    }
}

#[derive(Debug, Subcommand)]
enum SessionSubcommand {
    /// Create a new open session artifact bundle.
    Start(StartSessionCommand),
    /// Render the current status of a session bundle.
    Status(SessionStatusCommand),
    /// Evaluate whether a session can be closed cleanly.
    Lockcheck(SessionLockcheckCommand),
    /// Write the closeout artifact and finalize the session manifest.
    Close(SessionCloseCommand),
}

#[derive(Debug, Parser)]
struct StartSessionCommand {
    /// Session identifier.
    #[arg(value_name = "SESSION_ID")]
    session_id: String,

    /// Slug used to derive task/spec/todo ids.
    #[arg(long, default_value = "session")]
    slug: String,

    /// Human-readable problem statement for the session.
    #[arg(long, value_name = "TEXT", default_value = "VAC session initialized")]
    problem: String,

    /// Workspace root.
    #[arg(long, value_name = "PATH", default_value = ".")]
    workspace: PathBuf,

    /// Allow overwriting an existing session bundle.
    #[arg(long, default_value_t = false)]
    force: bool,
}

#[derive(Debug, Parser)]
struct SessionStatusCommand {
    /// Session identifier.
    #[arg(value_name = "SESSION_ID")]
    session_id: String,

    /// Workspace root.
    #[arg(long, value_name = "PATH", default_value = ".")]
    workspace: PathBuf,
}

#[derive(Debug, Parser)]
struct SessionLockcheckCommand {
    /// Session identifier.
    #[arg(value_name = "SESSION_ID")]
    session_id: String,

    /// Workspace root.
    #[arg(long, value_name = "PATH", default_value = ".")]
    workspace: PathBuf,
}

#[derive(Debug, Parser)]
struct SessionCloseCommand {
    /// Session identifier.
    #[arg(value_name = "SESSION_ID")]
    session_id: String,

    /// Workspace root.
    #[arg(long, value_name = "PATH", default_value = ".")]
    workspace: PathBuf,
}

impl StartSessionCommand {
    fn run(self) -> Result<()> {
        let root = normalize_root(&self.workspace)?;
        let paths = session_artifact_paths(&root, &self.session_id).map_err(anyhow::Error::msg)?;
        if !self.force
            && (paths.session_manifest.exists()
                || paths.task.exists()
                || paths.spec.exists()
                || paths.todo.exists())
        {
            bail!("session `{}` already exists", self.session_id);
        }

        let bundle = SessionArtifactBundle::new(
            self.session_id.clone(),
            self.slug,
            TaskArtifactState::Open,
            SpecArtifactState::Draft,
            TodoArtifactState::Open,
            self.problem,
        );
        let result = write_session_artifacts(&root, &bundle).map_err(anyhow::Error::msg)?;

        println!("vac session start: PASS");
        println!("session: {}", self.session_id);
        println!("workspace: {}", root.display());
        println!("manifest: {}", result.manifest.display());
        println!("task: {}", result.task.display());
        println!("spec: {}", result.spec.display());
        println!("todo: {}", result.todo.display());
        Ok(())
    }
}

impl SessionStatusCommand {
    fn run(self) -> Result<()> {
        let root = normalize_root(&self.workspace)?;
        let manifest =
            load_session_manifest(&root, &self.session_id).map_err(anyhow::Error::msg)?;
        let bundle = load_session_bundle(&root, &self.session_id).map_err(anyhow::Error::msg)?;
        let decision = evaluate_completion_lock(&bundle.task, &bundle.spec, &bundle.todo);
        let closeout = load_closeout_if_present(&root, &self.session_id)?;

        println!("vac session status: PASS");
        println!("session: {}", self.session_id);
        println!("workspace: {}", root.display());
        println!("manifest_state: {}", format_session_state(manifest.state));
        println!("task_state: {}", format_task_state(bundle.task.state));
        println!("spec_state: {}", format_spec_state(bundle.spec.state));
        println!("todo_state: {}", format_todo_state(bundle.todo.state));
        println!(
            "lock: {} (open_blocking_todos={}, unmet_acceptance_criteria={}, missing_evidence_refs={})",
            format_lock_outcome(decision.outcome),
            decision.summary.open_blocking_todos,
            decision.summary.unmet_acceptance_criteria,
            decision.summary.missing_evidence_refs
        );
        if let Some(closeout) = closeout {
            println!("close_state: {}", format_session_state(closeout.state));
        }
        Ok(())
    }
}

impl SessionLockcheckCommand {
    fn run(self) -> Result<()> {
        let root = normalize_root(&self.workspace)?;
        let bundle = load_session_bundle(&root, &self.session_id).map_err(anyhow::Error::msg)?;
        let decision = evaluate_completion_lock(&bundle.task, &bundle.spec, &bundle.todo);
        let ready = decision.summary.can_close();
        println!(
            "vac session lockcheck: {}",
            if ready { "PASS" } else { "FAIL" }
        );
        println!("session: {}", self.session_id);
        println!("workspace: {}", root.display());
        println!("outcome: {}", format_lock_outcome(decision.outcome));
        println!("task_ok: {}", yes_no(decision.summary.task_ok));
        println!("spec_ok: {}", yes_no(decision.summary.spec_ok));
        println!("todo_ok: {}", yes_no(decision.summary.todo_ok));
        println!(
            "open_blocking_todos: {}",
            decision.summary.open_blocking_todos
        );
        println!(
            "unmet_acceptance_criteria: {}",
            decision.summary.unmet_acceptance_criteria
        );
        println!(
            "missing_evidence_refs: {}",
            decision.summary.missing_evidence_refs
        );
        if !ready {
            bail!("session `{}` is not ready for closeout", self.session_id);
        }
        Ok(())
    }
}

impl SessionCloseCommand {
    fn run(self) -> Result<()> {
        let root = normalize_root(&self.workspace)?;
        let bundle = load_session_bundle(&root, &self.session_id).map_err(anyhow::Error::msg)?;
        let record = close_session_artifacts(&root, &bundle).map_err(anyhow::Error::msg)?;
        let ready = matches!(record.state, SessionArtifactState::Done);

        println!("vac session close: {}", if ready { "PASS" } else { "FAIL" });
        println!("session: {}", self.session_id);
        println!("workspace: {}", root.display());
        println!("close_state: {}", format_session_state(record.state));
        println!("closeout: close.{}", self.session_id);
        if !ready {
            bail!(
                "session `{}` is not ready for final closeout",
                self.session_id
            );
        }
        Ok(())
    }
}

fn load_closeout_if_present(
    workspace_root: &Path,
    session_id: &str,
) -> Result<Option<vac_core::control_plane::SessionCloseoutRecord>> {
    let paths = session_artifact_paths(workspace_root, session_id).map_err(anyhow::Error::msg)?;
    if !paths.close.exists() {
        return Ok(None);
    }
    let closeout: vac_core::control_plane::SessionCloseoutRecord =
        load_session_artifact_record(workspace_root, &paths.close).map_err(anyhow::Error::msg)?;
    Ok(Some(closeout))
}

fn normalize_root(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        Ok(path.canonicalize()?)
    } else {
        Ok(path.to_path_buf())
    }
}

fn format_session_state(state: SessionArtifactState) -> &'static str {
    match state {
        SessionArtifactState::Open => "open",
        SessionArtifactState::PausedForDiscussion => "paused_for_discussion",
        SessionArtifactState::Done => "done",
    }
}

fn format_task_state(state: TaskArtifactState) -> &'static str {
    match state {
        TaskArtifactState::Open => "open",
        TaskArtifactState::Done => "done",
        TaskArtifactState::NeedsDiscussion => "needs_discussion",
        TaskArtifactState::Dropped => "dropped",
    }
}

fn format_spec_state(state: SpecArtifactState) -> &'static str {
    match state {
        SpecArtifactState::Draft => "draft",
        SpecArtifactState::Finalized => "finalized",
        SpecArtifactState::NeedsDiscussion => "needs_discussion",
    }
}

fn format_todo_state(state: TodoArtifactState) -> &'static str {
    match state {
        TodoArtifactState::Open => "open",
        TodoArtifactState::AllChecked => "all_checked",
        TodoArtifactState::NeedsDiscussion => "needs_discussion",
    }
}

fn format_lock_outcome(outcome: vac_core::control_plane::CompletionLockOutcome) -> &'static str {
    match outcome {
        vac_core::control_plane::CompletionLockOutcome::Done => "done",
        vac_core::control_plane::CompletionLockOutcome::NeedsDiscussion => "needs_discussion",
        vac_core::control_plane::CompletionLockOutcome::Open => "open",
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use vac_core::control_plane::{
        AcceptanceCriterion, SessionCloseoutRecord, SpecArtifactState, TaskArtifactState,
        TodoArtifactState, load_session_artifact_record, load_session_doctor_report,
        load_session_manifest, write_session_artifacts,
    };

    fn temp_root() -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("vac-session-cli-{unique}"))
    }

    #[test]
    fn session_start_status_lockcheck_and_close_roundtrip() {
        let root = temp_root();

        StartSessionCommand {
            session_id: "session-004".to_string(),
            slug: "local".to_string(),
            problem: "roundtrip".to_string(),
            workspace: root.clone(),
            force: true,
        }
        .run()
        .expect("start");

        let mut bundle = load_session_bundle(&root, "session-004").expect("bundle");
        bundle.task.state = TaskArtifactState::Done;
        bundle.task.acceptance_criteria.push(AcceptanceCriterion {
            id: "ac.1".to_string(),
            text: "done".to_string(),
            met: true,
            evidence: Some("evidence-1".to_string()),
        });
        bundle.spec.state = SpecArtifactState::Finalized;
        bundle.todo.state = TodoArtifactState::AllChecked;
        write_session_artifacts(&root, &bundle).expect("rewrite");

        SessionStatusCommand {
            session_id: "session-004".to_string(),
            workspace: root.clone(),
        }
        .run()
        .expect("status");
        SessionLockcheckCommand {
            session_id: "session-004".to_string(),
            workspace: root.clone(),
        }
        .run()
        .expect("lockcheck");
        SessionCloseCommand {
            session_id: "session-004".to_string(),
            workspace: root.clone(),
        }
        .run()
        .expect("close");

        let manifest = load_session_manifest(&root, "session-004").expect("manifest");
        assert_eq!(manifest.state, SessionArtifactState::Done);
        let report = load_session_doctor_report(&root);
        assert_eq!(report.cli_exit_code(), 0);
        assert!(report.render_text().contains("vac doctor sessions: PASS"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn session_close_refuses_to_fake_completion() {
        let root = temp_root();

        StartSessionCommand {
            session_id: "session-005".to_string(),
            slug: "local".to_string(),
            problem: "open closeout".to_string(),
            workspace: root.clone(),
            force: true,
        }
        .run()
        .expect("start");

        let err = SessionCloseCommand {
            session_id: "session-005".to_string(),
            workspace: root.clone(),
        }
        .run()
        .expect_err("close should fail when lock is open");
        assert!(
            err.to_string().contains("not ready for final closeout"),
            "{err}"
        );

        let manifest = load_session_manifest(&root, "session-005").expect("manifest");
        assert_eq!(manifest.state, SessionArtifactState::Open);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn session_close_marks_needs_discussion_as_paused_for_discussion() {
        let root = temp_root();

        StartSessionCommand {
            session_id: "session-006".to_string(),
            slug: "local".to_string(),
            problem: "needs discussion closeout".to_string(),
            workspace: root.clone(),
            force: true,
        }
        .run()
        .expect("start");

        let mut bundle = load_session_bundle(&root, "session-006").expect("bundle");
        bundle.task.acceptance_criteria.push(AcceptanceCriterion {
            id: "ac.1".to_string(),
            text: "needs evidence".to_string(),
            met: true,
            evidence: None,
        });
        write_session_artifacts(&root, &bundle).expect("rewrite");

        let err = SessionCloseCommand {
            session_id: "session-006".to_string(),
            workspace: root.clone(),
        }
        .run()
        .expect_err("close should fail when evidence is missing");
        assert!(
            err.to_string().contains("not ready for final closeout"),
            "{err}"
        );

        let manifest = load_session_manifest(&root, "session-006").expect("manifest");
        assert_eq!(manifest.state, SessionArtifactState::PausedForDiscussion);
        let closeout: SessionCloseoutRecord = load_session_artifact_record(
            &root,
            root.join(".vac/registry/sessions/session-006/close.yaml"),
        )
        .expect("closeout");
        assert_eq!(closeout.state, SessionArtifactState::PausedForDiscussion);
        let report = load_session_doctor_report(&root);
        assert_eq!(report.cli_exit_code(), 0);
        assert!(
            report.render_text().contains("paused_for_discussion"),
            "{}",
            report.render_text()
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
