//! Session artifact schema and persistence helpers for the Part V control-plane track.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::vac_init_live_stores::read_vac_init_store_record;
use super::vac_init_live_stores::validate_workspace_relative_store_path;
use super::vac_init_live_stores::write_vac_init_store_record_atomic;

pub const SESSION_ARTIFACT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskArtifactState {
    Open,
    Done,
    NeedsDiscussion,
    Dropped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecArtifactState {
    Draft,
    Finalized,
    NeedsDiscussion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoArtifactState {
    Open,
    AllChecked,
    NeedsDiscussion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionArtifactState {
    Open,
    PausedForDiscussion,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptanceCriterion {
    pub id: String,
    pub text: String,
    pub met: bool,
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskArtifact {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub session: String,
    pub state: TaskArtifactState,
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
    pub open_questions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactContract {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub invariants: Vec<String>,
    pub out_of_scope: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecArtifact {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub session: String,
    pub state: SpecArtifactState,
    pub problem: String,
    pub contract: ArtifactContract,
    pub touched_capabilities: Vec<String>,
    pub memory_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub text: String,
    pub kind: String,
    pub checked: bool,
    pub blocking: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoArtifact {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub session: String,
    pub state: TodoArtifactState,
    pub items: Vec<TodoItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionArtifactManifest {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub session: String,
    pub state: SessionArtifactState,
    pub task_id: String,
    pub spec_id: String,
    pub todo_id: String,
    pub close_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionArtifactBundle {
    pub session_id: String,
    pub task: TaskArtifact,
    pub spec: SpecArtifact,
    pub todo: TodoArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionArtifactPaths {
    pub session_dir: PathBuf,
    pub session_manifest: PathBuf,
    pub task: PathBuf,
    pub spec: PathBuf,
    pub todo: PathBuf,
    pub close: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionArtifactWriteResult {
    pub manifest: PathBuf,
    pub task: PathBuf,
    pub spec: PathBuf,
    pub todo: PathBuf,
}

impl TaskArtifact {
    pub fn new(
        session_id: impl Into<String>,
        slug: impl Into<String>,
        state: TaskArtifactState,
    ) -> Self {
        let session = session_id.into();
        let slug = slug.into();
        Self {
            schema_version: SESSION_ARTIFACT_SCHEMA_VERSION,
            kind: "task_artifact".to_string(),
            id: format!("task.{session}.{slug}"),
            session,
            state,
            acceptance_criteria: Vec::new(),
            open_questions: Vec::new(),
        }
    }
}

impl SpecArtifact {
    pub fn new(
        session_id: impl Into<String>,
        slug: impl Into<String>,
        state: SpecArtifactState,
        problem: impl Into<String>,
    ) -> Self {
        let session = session_id.into();
        let slug = slug.into();
        Self {
            schema_version: SESSION_ARTIFACT_SCHEMA_VERSION,
            kind: "spec_artifact".to_string(),
            id: format!("spec.{session}.{slug}"),
            session,
            state,
            problem: problem.into(),
            contract: ArtifactContract {
                inputs: Vec::new(),
                outputs: Vec::new(),
                invariants: Vec::new(),
                out_of_scope: Vec::new(),
            },
            touched_capabilities: Vec::new(),
            memory_refs: Vec::new(),
        }
    }
}

impl TodoArtifact {
    pub fn new(
        session_id: impl Into<String>,
        slug: impl Into<String>,
        state: TodoArtifactState,
    ) -> Self {
        let session = session_id.into();
        let slug = slug.into();
        Self {
            schema_version: SESSION_ARTIFACT_SCHEMA_VERSION,
            kind: "todo_artifact".to_string(),
            id: format!("todo.{session}.{slug}"),
            session,
            state,
            items: Vec::new(),
        }
    }
}

impl SessionArtifactManifest {
    pub fn new(
        session_id: impl Into<String>,
        task_id: impl Into<String>,
        spec_id: impl Into<String>,
        todo_id: impl Into<String>,
        close_id: impl Into<String>,
        state: SessionArtifactState,
    ) -> Self {
        let session = session_id.into();
        Self {
            schema_version: SESSION_ARTIFACT_SCHEMA_VERSION,
            kind: "session_artifact".to_string(),
            id: format!("session.{session}"),
            session,
            state,
            task_id: task_id.into(),
            spec_id: spec_id.into(),
            todo_id: todo_id.into(),
            close_id: close_id.into(),
        }
    }
}

impl SessionArtifactBundle {
    pub fn new(
        session_id: impl Into<String>,
        slug: impl Into<String>,
        task_state: TaskArtifactState,
        spec_state: SpecArtifactState,
        todo_state: TodoArtifactState,
        problem: impl Into<String>,
    ) -> Self {
        let session_id = session_id.into();
        let slug = slug.into();
        Self {
            session_id: session_id.clone(),
            task: TaskArtifact::new(session_id.clone(), slug.clone(), task_state),
            spec: SpecArtifact::new(session_id.clone(), slug.clone(), spec_state, problem),
            todo: TodoArtifact::new(session_id, slug, todo_state),
        }
    }

    pub fn manifest(&self, state: SessionArtifactState) -> SessionArtifactManifest {
        SessionArtifactManifest::new(
            self.session_id.clone(),
            self.task.id.clone(),
            self.spec.id.clone(),
            self.todo.id.clone(),
            format!("close.{}", self.session_id),
            state,
        )
    }
}

pub fn validate_session_id(session_id: &str) -> Result<(), String> {
    let trimmed = session_id.trim();
    if trimmed.is_empty() {
        return Err("session id must not be empty".to_string());
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") {
        return Err("session id must be path-safe".to_string());
    }
    Ok(())
}

pub fn session_registry_dir(workspace_root: impl AsRef<Path>) -> PathBuf {
    workspace_root.as_ref().join(".vac/registry/sessions")
}

pub fn session_dir(workspace_root: impl AsRef<Path>, session_id: &str) -> PathBuf {
    session_registry_dir(workspace_root).join(session_id)
}

pub fn session_artifact_paths(
    workspace_root: impl AsRef<Path>,
    session_id: &str,
) -> Result<SessionArtifactPaths, String> {
    validate_session_id(session_id)?;
    let session_dir = session_dir(workspace_root, session_id);
    Ok(SessionArtifactPaths {
        session_manifest: session_dir.join("session.yaml"),
        task: session_dir.join("task.yaml"),
        spec: session_dir.join("spec.yaml"),
        todo: session_dir.join("todo.yaml"),
        close: session_dir.join("close.yaml"),
        session_dir,
    })
}

pub fn write_session_artifacts(
    workspace_root: impl AsRef<Path>,
    bundle: &SessionArtifactBundle,
) -> Result<SessionArtifactWriteResult, String> {
    let paths = session_artifact_paths(workspace_root.as_ref(), &bundle.session_id)?;
    let manifest = bundle.manifest(SessionArtifactState::Open);
    write_session_artifact_record(workspace_root.as_ref(), &paths.session_manifest, &manifest)?;
    write_session_artifact_record(workspace_root.as_ref(), &paths.task, &bundle.task)?;
    write_session_artifact_record(workspace_root.as_ref(), &paths.spec, &bundle.spec)?;
    write_session_artifact_record(workspace_root.as_ref(), &paths.todo, &bundle.todo)?;
    Ok(SessionArtifactWriteResult {
        manifest: paths.session_manifest,
        task: paths.task,
        spec: paths.spec,
        todo: paths.todo,
    })
}

pub fn read_session_artifact(
    workspace_root: impl AsRef<Path>,
    relative_path: impl AsRef<Path>,
) -> Result<String, String> {
    validate_workspace_relative_store_path(relative_path.as_ref())?;
    read_vac_init_store_record(workspace_root, relative_path).map_err(|err| err.to_string())
}

pub fn load_session_artifact_record<T: DeserializeOwned>(
    workspace_root: impl AsRef<Path>,
    relative_path: impl AsRef<Path>,
) -> Result<T, String> {
    let workspace_root = workspace_root.as_ref();
    let relative_path = relative_path.as_ref();
    let relative_path = if relative_path.is_absolute() {
        relative_path
            .strip_prefix(workspace_root)
            .map_err(|_| "session artifact path must be inside the workspace root".to_string())?
    } else {
        relative_path
    };
    validate_workspace_relative_store_path(relative_path)?;
    let yaml =
        read_vac_init_store_record(workspace_root, relative_path).map_err(|err| err.to_string())?;
    serde_yaml::from_str(&yaml).map_err(|err| err.to_string())
}

pub fn load_session_manifest(
    workspace_root: impl AsRef<Path>,
    session_id: &str,
) -> Result<SessionArtifactManifest, String> {
    let paths = session_artifact_paths(workspace_root.as_ref(), session_id)?;
    let manifest: SessionArtifactManifest =
        load_session_artifact_record(workspace_root.as_ref(), &paths.session_manifest)?;
    if manifest.session != session_id {
        return Err(format!(
            "session manifest `{}` belongs to `{}` not `{session_id}`",
            manifest.id, manifest.session
        ));
    }
    Ok(manifest)
}

pub fn load_session_bundle(
    workspace_root: impl AsRef<Path>,
    session_id: &str,
) -> Result<SessionArtifactBundle, String> {
    let paths = session_artifact_paths(workspace_root.as_ref(), session_id)?;
    let task: TaskArtifact = load_session_artifact_record(workspace_root.as_ref(), &paths.task)?;
    let spec: SpecArtifact = load_session_artifact_record(workspace_root.as_ref(), &paths.spec)?;
    let todo: TodoArtifact = load_session_artifact_record(workspace_root.as_ref(), &paths.todo)?;
    if task.session != session_id {
        return Err(format!(
            "task artifact `{}` belongs to `{}` not `{session_id}`",
            task.id, task.session
        ));
    }
    if spec.session != session_id {
        return Err(format!(
            "spec artifact `{}` belongs to `{}` not `{session_id}`",
            spec.id, spec.session
        ));
    }
    if todo.session != session_id {
        return Err(format!(
            "todo artifact `{}` belongs to `{}` not `{session_id}`",
            todo.id, todo.session
        ));
    }
    Ok(SessionArtifactBundle {
        session_id: session_id.to_string(),
        task,
        spec,
        todo,
    })
}

pub fn write_session_artifact_record<T: Serialize>(
    workspace_root: impl AsRef<Path>,
    record_path: impl AsRef<Path>,
    record: &T,
) -> Result<(), String> {
    let yaml = serde_yaml::to_string(record).map_err(|err| err.to_string())?;
    let workspace_root = workspace_root.as_ref();
    let record_path = record_path.as_ref();
    let relative_path = if record_path.is_absolute() {
        record_path
            .strip_prefix(workspace_root)
            .map_err(|_| "session artifact path must be inside the workspace root".to_string())?
    } else {
        record_path
    };
    validate_workspace_relative_store_path(relative_path)?;
    let _ = write_vac_init_store_record_atomic(workspace_root, relative_path, &yaml)?;
    Ok(())
}

pub fn session_artifact_relative_path(
    session_id: &str,
    file_name: &str,
) -> Result<PathBuf, String> {
    validate_session_id(session_id)?;
    if file_name.trim().is_empty() {
        return Err("artifact file name must not be empty".to_string());
    }
    Ok(PathBuf::from(".vac/registry/sessions")
        .join(session_id)
        .join(file_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn temp_root() -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("vac-session-artifacts-{unique}"))
    }

    #[test]
    fn session_bundle_writes_yaml_artifacts() {
        let root = temp_root();
        let mut bundle = SessionArtifactBundle::new(
            "session-001",
            "local-agent",
            TaskArtifactState::Open,
            SpecArtifactState::Draft,
            TodoArtifactState::Open,
            "wrap the remaining control-plane artifacts",
        );
        bundle.task.acceptance_criteria.push(AcceptanceCriterion {
            id: "ac.1".to_string(),
            text: "bundle writes task.yaml".to_string(),
            met: true,
            evidence: Some("evidence-1".to_string()),
        });
        bundle.todo.items.push(TodoItem {
            id: "t.1".to_string(),
            text: "write the files".to_string(),
            kind: "implement".to_string(),
            checked: true,
            blocking: true,
        });

        let result = write_session_artifacts(&root, &bundle).expect("write session artifacts");
        assert!(result.manifest.exists());
        assert!(result.task.exists());
        assert!(result.spec.exists());
        assert!(result.todo.exists());

        let task = read_session_artifact(&root, ".vac/registry/sessions/session-001/task.yaml")
            .expect("read task artifact");
        assert!(task.contains("task.session-001.local-agent"));
        assert!(task.contains("bundle writes task.yaml"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_invalid_session_ids() {
        assert!(validate_session_id("").is_err());
        assert!(validate_session_id("../escape").is_err());
        assert!(validate_session_id("session/escape").is_err());
        assert!(validate_session_id("session-001").is_ok());
    }

    #[test]
    fn session_artifact_relative_path_builds_workspace_relative_paths() {
        let path = session_artifact_relative_path("session-001", "task.yaml").expect("path");
        assert_eq!(
            path,
            PathBuf::from(".vac/registry/sessions/session-001/task.yaml")
        );
    }
}
