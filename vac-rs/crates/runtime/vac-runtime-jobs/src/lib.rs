//! VAC v1.9 control-plane support crate.
//!
//! This crate is intentionally small in the sandbox checkpoint. It provides
//! typed contracts used by static validation and later TV cargo fix loops.
use serde::{Deserialize, Serialize};

use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeJobKind {
    OneShot,
    Cron,
    Filewatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeJobState {
    Running,
    Queued,
    Ok,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeJobRecord {
    pub id: String,
    pub state: RuntimeJobState,
    pub kind: RuntimeJobKind,
    pub trigger: String,
    pub age: Option<String>,
    pub next_run: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeJobsSnapshot {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub mode: String,
    pub jobs: Vec<RuntimeJobRecord>,
}

pub fn load_snapshot(path: impl AsRef<Path>) -> Result<Option<RuntimeJobsSnapshot>, String> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&raw)
        .map(Some)
        .map_err(|err| err.to_string())
}
