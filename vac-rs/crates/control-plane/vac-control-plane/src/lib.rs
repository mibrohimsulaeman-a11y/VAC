//! VAC v1.9 control-plane support crate.
//!
//! This crate is intentionally small in the sandbox checkpoint. It provides
//! typed contracts used by static validation and later TV cargo fix loops.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SchemaEnvelope {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompiledManifestRef {
    pub id: String,
    pub source_hash: String,
    pub jcs_hash: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompiledCapability {
    pub id: String,
    pub declared: String,
    pub computed: String,
    pub effective: String,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub source_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceStatus {
    pub product: String,
    pub control_plane_version: String,
    pub runtime_authority: String,
    pub enforcement_level: String,
    #[serde(default)]
    pub compiled_snapshot_hash: Option<String>,
    #[serde(default)]
    pub readiness_summary: Option<serde_json::Value>,
    #[serde(default)]
    pub tv_pending: Vec<String>,
}
