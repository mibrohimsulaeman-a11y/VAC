#![allow(dead_code)]
//! Live file-backed VAC-Init durable store helpers.
//!
//! These helpers are intentionally small and dependency-free. They provide the
//! concrete temp+rename write path used by CLI/runtime code while typed stores
//! can layer richer serde models on top.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveStoreWriteResult {
    pub final_path: PathBuf,
    pub temp_path: PathBuf,
    pub bytes_written: usize,
}

pub fn validate_workspace_relative_store_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("store path must not be empty".to_string());
    }
    if path.is_absolute() {
        return Err("store path must be workspace-relative".to_string());
    }
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err("store path must not contain parent traversal".to_string());
        }
    }
    Ok(())
}

pub fn validate_yaml_envelope(content: &str) -> Result<(), String> {
    let mut schema_version = false;
    let mut kind = false;
    let mut id = false;
    for raw in content.lines() {
        let line = raw.trim();
        if line.starts_with("schema_version:") {
            schema_version = true;
        } else if line.starts_with("kind:") {
            kind = true;
        } else if line.starts_with("id:") {
            id = true;
        }
    }
    if schema_version && kind && id {
        Ok(())
    } else {
        Err("store record requires schema_version/kind/id envelope".to_string())
    }
}

pub fn write_vac_init_store_record_atomic(
    workspace_root: impl AsRef<Path>,
    relative_path: impl AsRef<Path>,
    content: &str,
) -> Result<LiveStoreWriteResult, String> {
    let relative_path = relative_path.as_ref();
    validate_workspace_relative_store_path(relative_path)?;
    validate_yaml_envelope(content)?;
    let final_path = workspace_root.as_ref().join(relative_path);
    let parent = final_path
        .parent()
        .ok_or_else(|| "store path must have a parent directory".to_string())?;
    fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    let temp_path = final_path.with_extension(format!(
        "{}.tmp",
        final_path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("yaml")
    ));
    fs::write(&temp_path, content).map_err(|err| err.to_string())?;
    fs::rename(&temp_path, &final_path).map_err(|err| err.to_string())?;
    Ok(LiveStoreWriteResult {
        final_path,
        temp_path,
        bytes_written: content.len(),
    })
}

pub fn read_vac_init_store_record(
    workspace_root: impl AsRef<Path>,
    relative_path: impl AsRef<Path>,
) -> io::Result<String> {
    fs::read_to_string(workspace_root.as_ref().join(relative_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root() -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("vac-live-store-{unique}"))
    }

    #[test]
    fn rejects_absolute_and_parent_traversal_paths() {
        assert!(validate_workspace_relative_store_path(Path::new("/tmp/state.yaml")).is_err());
        assert!(validate_workspace_relative_store_path(Path::new(".vac/../secret.yaml")).is_err());
        assert!(validate_workspace_relative_store_path(Path::new(".vac/.init/state.yaml")).is_ok());
    }

    #[test]
    fn atomic_write_requires_schema_envelope() {
        let root = temp_root();
        let err = write_vac_init_store_record_atomic(
            &root,
            ".vac/.init/state.yaml",
            "kind: init_state\n",
        )
        .unwrap_err();
        assert!(err.contains("schema_version"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn atomic_write_roundtrips_store_record() {
        let root = temp_root();
        let content = "schema_version: 1\nkind: init_state\nid: init.state\ncurrent_state: ready\n";
        let result =
            write_vac_init_store_record_atomic(&root, ".vac/.init/state.yaml", content).unwrap();
        assert_eq!(result.bytes_written, content.len());
        let stored = read_vac_init_store_record(&root, ".vac/.init/state.yaml").unwrap();
        assert_eq!(stored, content);
        assert!(result.final_path.exists());
        assert!(!result.temp_path.exists());
        let _ = fs::remove_dir_all(root);
    }
}
