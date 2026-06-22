//! VAC v1.9 runtime metadata bootstrap.
//!
//! This module is intentionally small and dependency-light: the real provider
//! loop only receives `AgentRunContext.metadata`, but VAC v1.9 requires that
//! metadata to carry compiled registry authority from `.vac/cache/compiled` or DB,
//! an approved Semantic Plan, mandatory task/spec/todo artifacts, read-plan tickets, and closeout state
//! before any mutating/process/read/network tool can execute. v1.9 keeps durable
//! runtime session state in the SQLite journal; JSON artifacts here are bootstrap
//! projections and export/debug material, not tracked source authority.
//!
//! The bootstrap is source-level L1 glue; it does not claim L2 substrate
//! enforcement. L2 still requires broker-held FS/proc/network and key custody.

use crate::bound_runtime::{
    CloseoutState, RuntimeJournalCloseoutState, RuntimeRegistrySnapshot, SemanticPlan,
    SessionArtifacts, canonical_json_sha256,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

const VAC_RUNTIME_KEY: &str = "vac_runtime";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VacRuntimeMetadataBundle {
    pub session_id: String,
    pub workspace_root: String,
    pub registry: RuntimeRegistrySnapshot,
    pub plan: SemanticPlan,
    pub artifacts: SessionArtifacts,
    pub closeout: CloseoutState,
    pub bootstrap_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VacRuntimeBootstrapReport {
    pub loaded_compiled_registry_snapshot: bool,
    pub attached_semantic_plan: bool,
    pub attached_mandatory_artifacts: bool,
    pub initialized_closeout_metadata: bool,
    pub source: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct VacRuntimeMetadataBootstrap {
    workspace_root: PathBuf,
}

impl VacRuntimeMetadataBootstrap {
    #[must_use]
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
        }
    }

    pub fn load_compiled_registry_snapshot(&self) -> Result<RuntimeRegistrySnapshot, String> {
        let candidates = [
            ".vac/cache/compiled/runtime/current.json",
            ".vac/cache/compiled/workspace.json",
            ".vac/cache/compiled/capabilities/current.json",
            ".vac/registry/compiled/runtime/current.json",
            ".vac/registry/compiled/workspace.json",
            ".vac/registry/compiled/capabilities/current.json",
        ];
        for candidate in candidates {
            let path = self.workspace_root.join(candidate);
            if !path.exists() {
                continue;
            }
            let value = read_json(&path)?;
            if let Ok(snapshot) = serde_json::from_value::<RuntimeRegistrySnapshot>(value.clone()) {
                return Ok(snapshot);
            }
            if let Some(runtime) = value.get("runtime_registry_snapshot").cloned() {
                return serde_json::from_value(runtime)
                    .map_err(|err| format!("invalid runtime_registry_snapshot: {err}"));
            }
        }
        Err(
            "compiled runtime snapshot missing; YAML authoring manifests are not runtime authority"
                .to_string(),
        )
    }

    pub fn attach_semantic_plan(&self, session_id: &str) -> Result<SemanticPlan, String> {
        let candidates = [
            format!(".vac/registry/sessions/{session_id}/plan.json"),
            format!(".vac/sessions/{session_id}/plan.json"),
            ".vac/plans/current.json".to_string(),
        ];
        read_first_json_as(&self.workspace_root, &candidates, "Semantic Plan")
    }

    pub fn attach_mandatory_artifacts(&self, session_id: &str) -> Result<SessionArtifacts, String> {
        let candidates = [
            format!(".vac/registry/sessions/{session_id}/artifacts.json"),
            format!(".vac/sessions/{session_id}/artifacts.json"),
            ".vac/registry/sessions/current/artifacts.json".to_string(),
        ];
        read_first_json_as(
            &self.workspace_root,
            &candidates,
            "mandatory task/spec/todo artifacts",
        )
    }

    pub fn initialize_closeout(&self, session_id: &str) -> Result<CloseoutState, String> {
        let candidates = [
            format!(".vac/registry/sessions/{session_id}/closeout.json"),
            format!(".vac/sessions/{session_id}/closeout.json"),
            ".vac/registry/sessions/current/closeout.json".to_string(),
        ];
        read_first_json_as(
            &self.workspace_root,
            &candidates,
            "completion-lock closeout",
        )
    }

    /// Compile real session records into the JSON authority shape consumed by
    /// BoundRuntimeController. Missing task/spec/todo artifacts are converted into
    /// needs_discussion placeholders; production bootstrap MUST NOT borrow historical
    /// terminal fixtures or fabricate done/finalized/all_checked closeout authority.
    pub fn compile_session_runtime_artifacts(&self, session_id: &str) -> Result<(), String> {
        let session_dir = self
            .workspace_root
            .join(".vac/registry/sessions")
            .join(session_id);
        fs::create_dir_all(&session_dir).map_err(|err| {
            format!(
                "cannot create session runtime dir {}: {err}",
                session_dir.display()
            )
        })?;

        let task_yaml = read_first_yaml_value(
            &self.workspace_root,
            &[
                format!(".vac/registry/sessions/{session_id}/task.yaml"),
                format!(".vac/registry/sessions/{session_id}/tasks.yaml"),
            ],
        )
        .unwrap_or_else(|| {
            default_task_json(
                session_id,
                "vac.runtime.agent_loop",
                &default_plan_id(session_id),
            )
        });

        let capability = task_yaml
            .get("capability")
            .and_then(Value::as_str)
            .unwrap_or("vac.runtime.agent_loop")
            .to_string();
        let plan_id = task_yaml
            .get("plan")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .unwrap_or_else(|| default_plan_id(session_id));
        let plan = read_first_json_or_yaml_value(
            &self.workspace_root,
            &[
                format!(".vac/registry/sessions/{session_id}/plan.json"),
                format!(".vac/registry/sessions/{session_id}/plan.yaml"),
                ".vac/plans/current.json".to_string(),
            ],
        )
        .unwrap_or_else(|| default_plan_json(session_id, &plan_id, &capability));

        let mut task = task_yaml;
        ensure_object_field(&mut task, "session", Value::String(session_id.to_string()));
        ensure_object_field(
            &mut task,
            "plan",
            Value::String(
                plan.get("id")
                    .and_then(Value::as_str)
                    .unwrap_or(&plan_id)
                    .to_string(),
            ),
        );
        ensure_object_field(&mut task, "capability", Value::String(capability.clone()));
        ensure_object_field(
            &mut task,
            "acceptance_criteria",
            json!([{
                "id": "ac.vac_runtime_metadata_bootstrap",
                "text": "Real task artifact was absent or incomplete; operator must provide acceptance criteria before closeout.",
                "met": false,
                "evidence": null
            }]),
        );

        let mut spec = read_first_yaml_value(
            &self.workspace_root,
            &[format!(".vac/registry/sessions/{session_id}/spec.yaml")],
        )
        .unwrap_or_else(|| default_spec_json(session_id, &capability));
        ensure_object_field(&mut spec, "session", Value::String(session_id.to_string()));
        ensure_object_field(
            &mut spec,
            "touched_capabilities",
            json!([capability.clone()]),
        );

        let mut todo = read_first_yaml_value(
            &self.workspace_root,
            &[
                format!(".vac/registry/sessions/{session_id}/todo.yaml"),
                format!(".vac/registry/sessions/{session_id}/todolist.yaml"),
            ],
        )
        .unwrap_or_else(|| default_todo_json(session_id));
        ensure_object_field(&mut todo, "session", Value::String(session_id.to_string()));
        ensure_object_field(
            &mut todo,
            "items",
            json!([{
                "id": "t.vac_runtime_metadata_bootstrap",
                "text": "Provide real session TODO checklist; bootstrap placeholder cannot satisfy completion lock.",
                "kind": "runtime",
                "checked": false,
                "blocking": true
            }]),
        );

        let artifacts = json!({"task": task, "spec": spec, "todo": todo});
        let closeout = default_closeout_json(&artifacts);

        write_json_if_missing(&session_dir.join("plan.json"), &plan)?;
        write_json_if_missing(&session_dir.join("artifacts.json"), &artifacts)?;
        write_json_if_missing(&session_dir.join("closeout.json"), &closeout)?;
        Ok(())
    }

    pub fn set_vac_runtime_metadata(
        &self,
        metadata: &mut Value,
        session_id: &str,
    ) -> Result<VacRuntimeBootstrapReport, String> {
        self.compile_session_runtime_artifacts(session_id)?;
        if !metadata.is_object() {
            *metadata = json!({});
        }
        let registry = self.load_compiled_registry_snapshot()?;
        let plan = self.attach_semantic_plan(session_id)?;
        let artifacts = self.attach_mandatory_artifacts(session_id)?;
        let closeout = self.initialize_closeout(session_id)?;
        let mut bundle = VacRuntimeMetadataBundle {
            session_id: session_id.to_string(),
            workspace_root: self.workspace_root.to_string_lossy().to_string(),
            registry,
            plan,
            artifacts,
            closeout,
            bootstrap_hash: String::new(),
        };
        bundle.bootstrap_hash =
            canonical_json_sha256(&serde_json::to_value(&bundle).unwrap_or(Value::Null));
        metadata.as_object_mut().expect("metadata object").insert(
            VAC_RUNTIME_KEY.to_string(),
            serde_json::to_value(&bundle)
                .map_err(|err| format!("cannot serialize vac_runtime metadata: {err}"))?,
        );
        Ok(VacRuntimeBootstrapReport {
            loaded_compiled_registry_snapshot: true,
            attached_semantic_plan: true,
            attached_mandatory_artifacts: true,
            initialized_closeout_metadata: true,
            source: "compiled_json_runtime_truth".to_string(),
            warnings: vec![
                "L1 bootstrap only; L2 broker/OS sandbox custody remains TV-Pending".to_string(),
            ],
        })
    }
}

fn default_plan_id(session_id: &str) -> String {
    format!("plan.session.{session_id}.runtime-metadata-bootstrap")
}

fn default_plan_json(session_id: &str, plan_id: &str, capability: &str) -> Value {
    json!({
        "id": plan_id,
        "status": "approved",
        "capability": capability,
        "allowed_files": [],
        "forbidden_files": ["target/**", "node_modules/**", ".git/**"],
        "validation_commands": [],
        "approval": {"required": false, "approved": true, "plan_hash": canonical_json_sha256(&Value::String(format!("{session_id}:{plan_id}")))},
        "bounds": {"max_patches": 0, "max_new_files": 0, "max_line_delta": 0}
    })
}

fn default_task_json(session_id: &str, capability: &str, plan_id: &str) -> Value {
    json!({
        "id": format!("task.{session_id}.runtime-metadata-bootstrap"),
        "session": session_id,
        "state": "needs_discussion",
        "capability": capability,
        "plan": plan_id,
        "acceptance_criteria": [{
            "id": "ac.vac_runtime_metadata_bootstrap",
            "text": "Real task artifact was absent; operator must provide or approve session-specific acceptance criteria before closeout.",
            "met": false,
            "evidence": null
        }],
        "open_questions": ["Missing session task artifact; completion lock must pause instead of fabricating done."]
    })
}

fn default_spec_json(session_id: &str, capability: &str) -> Value {
    json!({
        "id": format!("spec.{session_id}.runtime-metadata-bootstrap"),
        "session": session_id,
        "state": "needs_discussion",
        "problem": "Real session spec artifact was absent during bootstrap.",
        "invariants": [
            "runtime MUST authorize from compiled JSON snapshot",
            "agent tool execution MUST be gated by BoundRuntimeToolBoundary",
            "session closeout MUST pass completion lock or surface needs_discussion"
        ],
        "touched_capabilities": [capability],
        "memory_refs": [],
        "open_questions": ["Missing session spec artifact; runtime cannot mark spec finalized synthetically."]
    })
}

fn default_todo_json(session_id: &str) -> Value {
    json!({
        "id": format!("todo.{session_id}.runtime-metadata-bootstrap"),
        "session": session_id,
        "state": "needs_discussion",
        "items": [{
            "id": "t.vac_runtime_metadata_bootstrap",
            "text": "Provide real session TODO checklist; bootstrap placeholder cannot satisfy completion lock.",
            "kind": "runtime",
            "checked": false,
            "blocking": true
        }],
        "open_questions": ["Missing session todo artifact; blocking checklist item remains unchecked."]
    })
}

fn default_closeout_json(artifacts: &Value) -> Value {
    let self_hash = canonical_json_sha256(artifacts);
    json!({
        "artifacts": artifacts,
        "evidence": {
            "valid": false,
            "self_hash": self_hash,
            "broker_sig_algorithm": "none",
            "broker_sig_mode": "integrity_hint",
            "warning_label": "bootstrap generated placeholder artifacts; evidence is intentionally non-terminal"
        },
        "compiled_json_snapshot_current": true,
        "spec_sync": {"no_critical_spec_drift": true, "unresolved_critical_drift": 0},
        "readiness": {"no_unresolved_readiness_mismatch": true, "unresolved_mismatches": 0},
        "ownership": {"no_code_without_capability": true, "no_capability_without_current_intent_spec": true, "code_without_capability": 0, "capabilities_without_intent": 0},
        "assessment": {"findings_have_span_evidence": true, "unresolved_critical_findings": 0},
        "runtime_journal": serde_json::to_value(RuntimeJournalCloseoutState::default())
            .unwrap_or_else(|_| json!({"session_recorded": false})),
        "explicit_open_question": "Missing real session task/spec/todo artifacts; runtime must pause for operator discussion.",
        "blocking_reason": "bootstrap-placeholder-artifacts-are-not-completion-authority",
        "operator_visible_status": true
    })
}

fn ensure_object_field(value: &mut Value, key: &str, fallback: Value) {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    if let Some(obj) = value.as_object_mut() {
        obj.entry(key.to_string()).or_insert(fallback);
    }
}

fn read_first_json_or_yaml_value(root: &Path, candidates: &[String]) -> Option<Value> {
    for candidate in candidates {
        let path = root.join(candidate);
        if !path.exists() {
            continue;
        }
        if path.extension().and_then(|item| item.to_str()) == Some("json") {
            if let Ok(value) = read_json(&path) {
                return Some(value);
            }
        } else if let Some(value) = read_yaml_value(&path) {
            return Some(value);
        }
    }
    None
}

fn read_first_yaml_value(root: &Path, candidates: &[String]) -> Option<Value> {
    for candidate in candidates {
        let path = root.join(candidate);
        if !path.exists() {
            continue;
        }
        if let Some(value) = read_yaml_value(&path) {
            return Some(value);
        }
    }
    None
}

fn read_yaml_value(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&raw).ok()?;
    serde_json::to_value(yaml).ok()
}

fn write_json_if_missing(path: &Path, value: &Value) -> Result<(), String> {
    if path.exists() {
        return Ok(());
    }
    let payload = serde_json::to_string_pretty(value)
        .map_err(|err| format!("cannot serialize {}: {err}", path.display()))?;
    fs::write(path, format!("{payload}\n"))
        .map_err(|err| format!("cannot write {}: {err}", path.display()))
}

fn read_first_json_as<T>(root: &Path, candidates: &[String], label: &str) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    for candidate in candidates {
        let path = root.join(candidate);
        if !path.exists() {
            continue;
        }
        let value = read_json(&path)?;
        return serde_json::from_value(value)
            .map_err(|err| format!("invalid {label} at {candidate}: {err}"));
    }
    Err(format!(
        "{label} missing; bound runtime cannot run without session artifacts"
    ))
}

fn read_json(path: &Path) -> Result<Value, String> {
    let raw =
        fs::read_to_string(path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    serde_json::from_str(&raw).map_err(|err| format!("cannot parse {}: {err}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_workspace(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vac-agent-loop-{name}-{nonce}"));
        fs::create_dir_all(&root).expect("create temp workspace");
        root
    }

    #[test]
    fn missing_session_records_compile_to_nonterminal_runtime_projections() {
        let root = temp_workspace("projection");
        let bootstrap = VacRuntimeMetadataBootstrap::new(&root);
        bootstrap
            .compile_session_runtime_artifacts("session.projection")
            .expect("compile runtime projections");

        let session_dir = root.join(".vac/registry/sessions/session.projection");
        let artifacts = read_json(&session_dir.join("artifacts.json")).expect("read artifacts");
        let closeout = read_json(&session_dir.join("closeout.json")).expect("read closeout");

        assert_eq!(artifacts["task"]["state"], "needs_discussion");
        assert_eq!(artifacts["spec"]["state"], "needs_discussion");
        assert_eq!(artifacts["todo"]["state"], "needs_discussion");
        assert_eq!(closeout["evidence"]["valid"], false);
        assert_eq!(
            closeout["blocking_reason"],
            "bootstrap-placeholder-artifacts-are-not-completion-authority"
        );
        assert!(
            closeout["explicit_open_question"]
                .as_str()
                .expect("open question")
                .contains("runtime must pause")
        );
    }

    #[test]
    fn runtime_projection_writer_preserves_existing_session_plan() {
        let root = temp_workspace("preserve-plan");
        let session_dir = root.join(".vac/registry/sessions/session.keep");
        fs::create_dir_all(&session_dir).expect("create session dir");
        fs::write(
            session_dir.join("plan.json"),
            r#"{"id":"plan.preexisting","status":"approved","capability":"vac.runtime.agent_loop"}"#,
        )
        .expect("write existing plan");

        let bootstrap = VacRuntimeMetadataBootstrap::new(&root);
        bootstrap
            .compile_session_runtime_artifacts("session.keep")
            .expect("compile runtime projections");

        let plan = read_json(&session_dir.join("plan.json")).expect("read preserved plan");
        let artifacts = read_json(&session_dir.join("artifacts.json")).expect("read artifacts");

        assert_eq!(plan["id"], "plan.preexisting");
        assert_eq!(artifacts["task"]["state"], "needs_discussion");
        assert_eq!(artifacts["todo"]["items"][0]["checked"], false);
    }
}
