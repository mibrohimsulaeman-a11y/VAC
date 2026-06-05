//! Session closeout helpers for Part V completion lock integration.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::completion_lock::{
    CompletionLockOutcome, evaluate_completion_lock, session_close_state,
};
use super::session_artifacts::{
    SessionArtifactBundle, SessionArtifactManifest, SessionArtifactState,
    write_session_artifact_record,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionCloseoutRecord {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub session: String,
    pub outcome: CompletionLockOutcome,
    pub lock: super::completion_lock::CompletionLockSummary,
    pub state: SessionArtifactState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCloseoutPaths {
    pub session_dir: PathBuf,
    pub close: PathBuf,
    pub manifest: PathBuf,
}

pub fn session_closeout_paths(
    workspace_root: impl AsRef<Path>,
    session_id: &str,
) -> Result<SessionCloseoutPaths, String> {
    let paths = super::session_artifacts::session_artifact_paths(workspace_root, session_id)?;
    Ok(SessionCloseoutPaths {
        session_dir: paths.session_dir,
        close: paths.close,
        manifest: paths.session_manifest,
    })
}

pub fn close_session_artifacts(
    workspace_root: impl AsRef<Path>,
    bundle: &SessionArtifactBundle,
) -> Result<SessionCloseoutRecord, String> {
    super::session_artifacts::write_session_artifacts(workspace_root.as_ref(), bundle)?;
    let decision = evaluate_completion_lock(&bundle.task, &bundle.spec, &bundle.todo);
    let state = session_close_state(&decision);
    let paths = super::session_artifacts::session_artifact_paths(
        workspace_root.as_ref(),
        &bundle.session_id,
    )?;
    let close_record = SessionCloseoutRecord {
        schema_version: 1,
        kind: "session_closeout".to_string(),
        id: format!("close.{}", bundle.session_id),
        session: bundle.session_id.clone(),
        outcome: decision.outcome,
        lock: decision.summary.clone(),
        state,
    };
    write_session_artifact_record(workspace_root.as_ref(), &paths.close, &close_record)?;
    let manifest = SessionArtifactManifest::new(
        bundle.session_id.clone(),
        bundle.task.id.clone(),
        bundle.spec.id.clone(),
        bundle.todo.id.clone(),
        close_record.id.clone(),
        state,
    );
    write_session_artifact_record(workspace_root.as_ref(), &paths.session_manifest, &manifest)?;
    Ok(close_record)
}

#[cfg(test)]
mod tests {
    use super::super::session_artifacts::{
        AcceptanceCriterion, SessionArtifactManifest, SpecArtifactState, TaskArtifactState,
        TodoArtifactState, load_session_artifact_record, load_session_manifest,
    };
    use super::*;
    use pretty_assertions::assert_eq;

    fn temp_root() -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("vac-session-closeout-{unique}"))
    }

    #[test]
    fn close_session_writes_closeout_record_and_manifest() {
        let root = temp_root();
        let mut bundle = SessionArtifactBundle::new(
            "session-002",
            "closeout",
            TaskArtifactState::Done,
            SpecArtifactState::Finalized,
            TodoArtifactState::AllChecked,
            "closeout problem",
        );
        bundle.task.acceptance_criteria.push(
            super::super::session_artifacts::AcceptanceCriterion {
                id: "ac.1".to_string(),
                text: "done".to_string(),
                met: true,
                evidence: Some("evidence-1".to_string()),
            },
        );

        let record = close_session_artifacts(&root, &bundle).expect("close session");
        assert_eq!(record.session, "session-002");
        let paths = session_closeout_paths(&root, "session-002").expect("paths");
        assert_eq!(
            paths.close,
            root.join(".vac/registry/sessions/session-002/close.yaml")
        );
        assert_eq!(
            paths.manifest,
            root.join(".vac/registry/sessions/session-002/session.yaml")
        );
        assert!(
            root.join(".vac/registry/sessions/session-002/close.yaml")
                .exists()
        );
        assert!(
            root.join(".vac/registry/sessions/session-002/session.yaml")
                .exists()
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn close_session_pauses_for_discussion_when_evidence_is_missing() {
        let root = temp_root();
        let mut bundle = SessionArtifactBundle::new(
            "session-003",
            "closeout",
            TaskArtifactState::NeedsDiscussion,
            SpecArtifactState::NeedsDiscussion,
            TodoArtifactState::NeedsDiscussion,
            "discussion closeout",
        );
        bundle.task.acceptance_criteria.push(AcceptanceCriterion {
            id: "ac.1".to_string(),
            text: "needs evidence".to_string(),
            met: true,
            evidence: None,
        });

        let record = close_session_artifacts(&root, &bundle).expect("close session");
        assert_eq!(record.state, SessionArtifactState::PausedForDiscussion);
        assert_eq!(record.outcome, CompletionLockOutcome::NeedsDiscussion);

        let manifest: SessionArtifactManifest =
            load_session_manifest(&root, "session-003").expect("manifest");
        assert_eq!(manifest.state, SessionArtifactState::PausedForDiscussion);

        let closeout: SessionCloseoutRecord = load_session_artifact_record(
            &root,
            root.join(".vac/registry/sessions/session-003/close.yaml"),
        )
        .expect("closeout");
        assert_eq!(closeout.state, SessionArtifactState::PausedForDiscussion);
        let _ = std::fs::remove_dir_all(root);
    }
}
