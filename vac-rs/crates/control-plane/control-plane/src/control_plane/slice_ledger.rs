//! Slice ledger persistence for Part V session tracking.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::vac_init_live_stores::write_vac_init_store_record_atomic;

pub const SLICE_LEDGER_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicationCheck {
    pub reused_capabilities: Vec<String>,
    pub similar_slices: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SliceLedgerEntry {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub session: String,
    pub capability: String,
    pub spec: String,
    pub plan: String,
    pub evidence: String,
    pub files: Vec<String>,
    pub dependencies_added: Vec<String>,
    pub dependencies_justification: Vec<String>,
    pub duplication_check: DuplicationCheck,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SliceLedgerIndexEntry {
    pub id: String,
    pub capability: String,
    pub spec: String,
    pub plan: String,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SliceLedgerIndex {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub entries: Vec<SliceLedgerIndexEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceLedgerWriteResult {
    pub entry_path: PathBuf,
    pub index_path: PathBuf,
}

pub fn slice_ledger_root(workspace_root: impl AsRef<Path>) -> PathBuf {
    workspace_root.as_ref().join(".vac/registry/slice-ledger")
}

pub fn slice_ledger_entries_dir(workspace_root: impl AsRef<Path>) -> PathBuf {
    slice_ledger_root(workspace_root).join("slices")
}

pub fn slice_ledger_entry_path(
    workspace_root: impl AsRef<Path>,
    slice_id: &str,
) -> Result<PathBuf, String> {
    validate_slice_id(slice_id)?;
    Ok(slice_ledger_entries_dir(workspace_root).join(format!("{slice_id}.yaml")))
}

pub fn slice_ledger_index_path(workspace_root: impl AsRef<Path>) -> PathBuf {
    slice_ledger_root(workspace_root).join("index.yaml")
}

pub fn validate_slice_id(slice_id: &str) -> Result<(), String> {
    let trimmed = slice_id.trim();
    if trimmed.is_empty() {
        return Err("slice id must not be empty".to_string());
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") {
        return Err("slice id must be path-safe".to_string());
    }
    Ok(())
}

pub fn append_slice_ledger_entry(
    workspace_root: impl AsRef<Path>,
    entry: &SliceLedgerEntry,
) -> Result<SliceLedgerWriteResult, String> {
    let root = workspace_root.as_ref();
    let entry_path = slice_ledger_entry_path(root, &entry.id)?;
    let index_path = slice_ledger_index_path(root);
    let index_dir = index_path
        .parent()
        .ok_or_else(|| "slice ledger index path must have a parent".to_string())?;
    std::fs::create_dir_all(index_dir).map_err(|err| err.to_string())?;
    let entries_dir = slice_ledger_entries_dir(root);
    std::fs::create_dir_all(&entries_dir).map_err(|err| err.to_string())?;

    let entry_yaml = serde_yaml::to_string(entry).map_err(|err| err.to_string())?;
    let _ = write_vac_init_store_record_atomic(
        root,
        entry_path
            .strip_prefix(root)
            .map_err(|err| err.to_string())?,
        &entry_yaml,
    )?;

    let mut index = if index_path.exists() {
        serde_yaml::from_str::<SliceLedgerIndex>(
            &std::fs::read_to_string(&index_path).map_err(|err| err.to_string())?,
        )
        .map_err(|err| err.to_string())?
    } else {
        SliceLedgerIndex {
            schema_version: SLICE_LEDGER_SCHEMA_VERSION,
            kind: "slice_ledger_index".to_string(),
            id: "slice-ledger.index".to_string(),
            entries: Vec::new(),
        }
    };
    index.entries.push(SliceLedgerIndexEntry {
        id: entry.id.clone(),
        capability: entry.capability.clone(),
        spec: entry.spec.clone(),
        plan: entry.plan.clone(),
        evidence: entry.evidence.clone(),
    });
    let index_yaml = serde_yaml::to_string(&index).map_err(|err| err.to_string())?;
    let _ = write_vac_init_store_record_atomic(
        root,
        index_path
            .strip_prefix(root)
            .map_err(|err| err.to_string())?,
        &index_yaml,
    )?;

    Ok(SliceLedgerWriteResult {
        entry_path,
        index_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root() -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("vac-slice-ledger-{unique}"))
    }

    #[test]
    fn append_slice_ledger_entry_writes_index_and_entry() {
        let root = temp_root();
        let entry = SliceLedgerEntry {
            schema_version: SLICE_LEDGER_SCHEMA_VERSION,
            kind: "slice_ledger_entry".to_string(),
            id: "slice.2026-06-04.example".to_string(),
            session: "session-001".to_string(),
            capability: "capability.example".to_string(),
            spec: "spec.session-001.example".to_string(),
            plan: "plan-001".to_string(),
            evidence: "evidence-001".to_string(),
            files: vec!["src/lib.rs".to_string()],
            dependencies_added: vec!["serde_yaml".to_string()],
            dependencies_justification: vec!["artifact persistence".to_string()],
            duplication_check: DuplicationCheck {
                reused_capabilities: vec!["capabilities/sessions".to_string()],
                similar_slices: vec!["slice.2026-06-01.example".to_string()],
            },
        };

        let result = append_slice_ledger_entry(&root, &entry).expect("append");
        assert!(result.entry_path.exists());
        assert!(result.index_path.exists());
        let index = std::fs::read_to_string(&result.index_path).expect("read index");
        assert!(index.contains("slice.2026-06-04.example"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn validates_slice_ids() {
        assert!(validate_slice_id("").is_err());
        assert!(validate_slice_id("slice/escape").is_err());
        assert!(validate_slice_id("slice-001").is_ok());
    }
}
