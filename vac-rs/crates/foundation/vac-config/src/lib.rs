//! VAC workspace config model. This is the typed view of `.vac/vac.toml`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub schema_version: u64,
    pub workspace: WorkspaceIdentity,
    pub runtime: RuntimeConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceIdentity {
    pub id: String,
    pub product: String,
    pub rust_workspace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub enforcement_level: String,
    pub control_plane_authority: String,
    pub default_profile: String,
}

impl WorkspaceConfig {
    #[must_use]
    pub fn local_l1() -> Self {
        Self {
            schema_version: 1,
            workspace: WorkspaceIdentity {
                id: "vac.workspace".to_string(),
                product: "VAC".to_string(),
                rust_workspace: "vac-rs".to_string(),
            },
            runtime: RuntimeConfig {
                enforcement_level: "L1".to_string(),
                control_plane_authority: "compiled_json".to_string(),
                default_profile: "default".to_string(),
            },
        }
    }
}
