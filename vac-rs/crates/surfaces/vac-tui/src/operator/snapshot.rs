use crate::operator::mode::OperatorMode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrandSnapshot {
    pub product: String,
    pub binary: String,
    pub rulebook: String,
}

impl Default for BrandSnapshot {
    fn default() -> Self {
        Self {
            product: "VAC".to_string(),
            binary: "vac".to_string(),
            rulebook: "vac.core".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelSnapshot {
    pub provider: Option<String>,
    pub active_model: Option<String>,
    pub context_limit: Option<u64>,
}

impl ModelSnapshot {
    pub fn display_model(&self) -> &str {
        self.active_model.as_deref().unwrap_or("unknown")
    }
    pub fn display_provider(&self) -> &str {
        self.provider.as_deref().unwrap_or("not configured")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageSnapshot {
    pub tokens_used: u64,
    pub context_limit: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlPlaneSnapshot {
    pub status: String,
    pub valid_percent: Option<u64>,
    pub compiled_snapshot: Option<String>,
    pub unresolved_critical_drift: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnforcementSnapshot {
    pub level: String,
    pub isolation: String,
    pub network: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub id: Option<String>,
    pub recent: Vec<RecentTaskSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentTaskSnapshot {
    pub title: String,
    pub when: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolTimelineItem {
    pub name: String,
    pub target: String,
    pub state: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalSnapshot {
    pub kind: String,
    pub command: String,
    pub cwd: String,
    pub risk: String,
    pub policy: String,
    pub sandbox: String,
    pub network: String,
    pub writes: String,
    pub batch_position: Option<(usize, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeJobSnapshot {
    pub id: String,
    pub state: String,
    pub kind: String,
    pub trigger: String,
    pub title: String,
    pub age: String,
    pub next_run: Option<String>,
    pub retry_count: u64,
    pub token_usage: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeJobsSnapshot {
    pub records: Vec<RuntimeJobSnapshot>,
    pub queued: usize,
    pub running: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorSnapshot {
    pub brand: BrandSnapshot,
    pub active_mode: OperatorMode,
    pub cwd: String,
    pub profile: String,
    pub version: String,
    pub tabs: Vec<String>,
    pub model: ModelSnapshot,
    pub usage: UsageSnapshot,
    pub session: SessionSnapshot,
    pub control_plane: ControlPlaneSnapshot,
    pub enforcement: EnforcementSnapshot,
    pub tool_timeline: Vec<ToolTimelineItem>,
    pub approval: Option<ApprovalSnapshot>,
    pub runtime_jobs: RuntimeJobsSnapshot,
}

impl OperatorSnapshot {
    pub fn from_workspace(workspace_root: impl AsRef<Path>, active_mode: OperatorMode) -> Self {
        let root = workspace_root.as_ref();
        let status = read_json(root.join(".vac/registry/status.json"));
        let jobs = read_json(root.join(".vac/registry/runtime/jobs.json"));
        let model = model_from_status(&status);
        let profile = env::var("VAC_PROFILE").unwrap_or_else(|_| "default".to_string());
        let cwd = env::current_dir()
            .ok()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| root.display().to_string());
        let usage = UsageSnapshot {
            tokens_used: env::var("VAC_TOKEN_USAGE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            context_limit: model.context_limit,
        };
        let control_plane = control_plane_from_status(&status);
        let enforcement = enforcement_from_status(&status);
        let session = session_from_status(&status);
        let runtime_jobs = runtime_jobs_from_registry(&jobs);
        Self {
            brand: BrandSnapshot::default(),
            active_mode,
            cwd,
            profile,
            version: env!("CARGO_PKG_VERSION").to_string(),
            tabs: vec![
                "chat".into(),
                "runtime".into(),
                "review".into(),
                "workbench".into(),
                "mcp".into(),
            ],
            model,
            usage,
            session,
            control_plane,
            enforcement,
            tool_timeline: Vec::new(),
            approval: None,
            runtime_jobs,
        }
    }

    pub fn with_tool_timeline(mut self, items: Vec<ToolTimelineItem>) -> Self {
        let keep_from = items.len().saturating_sub(5);
        self.tool_timeline = items.into_iter().skip(keep_from).collect();
        self
    }

    pub fn with_approval(mut self, approval: ApprovalSnapshot) -> Self {
        self.approval = Some(approval);
        self.active_mode = OperatorMode::ApprovalRequired;
        self
    }
}

fn read_json(path: PathBuf) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

fn model_from_status(status: &Option<Value>) -> ModelSnapshot {
    let provider = env::var("VAC_PROVIDER").ok().or_else(|| {
        status
            .as_ref()?
            .pointer("/model/provider")?
            .as_str()
            .map(ToOwned::to_owned)
    });
    let active_model = env::var("VAC_MODEL").ok().or_else(|| {
        status
            .as_ref()?
            .pointer("/model/active_model")?
            .as_str()
            .map(ToOwned::to_owned)
    });
    let context_limit = env::var("VAC_CONTEXT_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .or_else(|| status.as_ref()?.pointer("/model/context_limit")?.as_u64());
    ModelSnapshot {
        provider,
        active_model,
        context_limit,
    }
}

fn control_plane_from_status(status: &Option<Value>) -> ControlPlaneSnapshot {
    let compiled_snapshot = status
        .as_ref()
        .and_then(|v| v.pointer("/control_plane/compiled_snapshot"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let valid_percent = status
        .as_ref()
        .and_then(|v| v.pointer("/readiness/valid_percent"))
        .and_then(Value::as_u64);
    let unresolved_critical_drift = status
        .as_ref()
        .and_then(|v| v.pointer("/spec_sync/unresolved_critical_drift"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let status_label = if status.is_some() {
        "compiled"
    } else {
        "uninitialized"
    }
    .to_string();
    ControlPlaneSnapshot {
        status: status_label,
        valid_percent,
        compiled_snapshot,
        unresolved_critical_drift,
    }
}

fn enforcement_from_status(status: &Option<Value>) -> EnforcementSnapshot {
    let level = status
        .as_ref()
        .and_then(|v| v.pointer("/workspace/enforcement_level"))
        .and_then(Value::as_str)
        .unwrap_or("L1")
        .to_string();
    EnforcementSnapshot {
        level,
        isolation: "off".to_string(),
        network: "policy".to_string(),
    }
}

fn session_from_status(status: &Option<Value>) -> SessionSnapshot {
    let id = env::var("VAC_SESSION_ID").ok().or_else(|| {
        status
            .as_ref()?
            .pointer("/session/current")?
            .as_str()
            .map(ToOwned::to_owned)
    });
    let recent = status
        .as_ref()
        .and_then(|v| v.pointer("/session/recent"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|r| {
                    Some(RecentTaskSnapshot {
                        title: r.get("title")?.as_str()?.to_string(),
                        when: r
                            .get("when")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown")
                            .to_string(),
                        status: r
                            .get("status")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown")
                            .to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    SessionSnapshot { id, recent }
}

fn runtime_jobs_from_registry(registry: &Option<Value>) -> RuntimeJobsSnapshot {
    let records: Vec<RuntimeJobSnapshot> = registry
        .as_ref()
        .and_then(|v| v.get("records"))
        .and_then(Value::as_array)
        .map(|rows| rows.iter().filter_map(job_from_json).collect())
        .unwrap_or_default();
    let queued = records.iter().filter(|r| r.state == "queued").count();
    let running = records.iter().filter(|r| r.state == "running").count();
    RuntimeJobsSnapshot {
        records,
        queued,
        running,
    }
}

fn job_from_json(v: &Value) -> Option<RuntimeJobSnapshot> {
    Some(RuntimeJobSnapshot {
        id: v.get("id")?.as_str()?.to_string(),
        state: v
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("queued")
            .to_string(),
        kind: v
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("one_shot")
            .to_string(),
        trigger: v
            .get("trigger")
            .and_then(Value::as_str)
            .unwrap_or("manual")
            .to_string(),
        title: v
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("untitled runtime job")
            .to_string(),
        age: v
            .get("age")
            .and_then(Value::as_str)
            .unwrap_or("0s")
            .to_string(),
        next_run: v
            .get("next_run")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        retry_count: v
            .pointer("/inspect/retry_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        token_usage: v.pointer("/inspect/token_usage").and_then(Value::as_u64),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn agent_timeline_keeps_last_five() {
        let s = OperatorSnapshot::from_workspace(".", OperatorMode::AgentWorking)
            .with_tool_timeline(
                (0..7)
                    .map(|i| ToolTimelineItem {
                        name: format!("tool{i}"),
                        target: "file".into(),
                        state: "ok".into(),
                        detail: None,
                    })
                    .collect(),
            );
        assert_eq!(s.tool_timeline.len(), 5);
        assert_eq!(s.tool_timeline[0].name, "tool2");
    }

    #[test]
    fn runtime_jobs_empty_state_no_mock_rows() {
        let s = OperatorSnapshot::from_workspace("/definitely/missing", OperatorMode::RuntimeJobs);
        assert!(s.runtime_jobs.records.is_empty());
    }
}
