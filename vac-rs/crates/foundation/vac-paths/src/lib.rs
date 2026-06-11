#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePaths {
    pub root: PathBuf,
    pub control_plane: PathBuf,
    pub legacy_control_plane: PathBuf,
}

impl WorkspacePaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            control_plane: root.join(vac_brand::CONFIG_DIR),
            legacy_control_plane: root.join(vac_brand::LEGACY_CONFIG_DIR),
            root,
        }
    }

    pub fn vac_toml(&self) -> PathBuf {
        self.control_plane.join("vac.toml")
    }
    pub fn registry_status(&self) -> PathBuf {
        self.control_plane.join("registry/status.json")
    }
    pub fn runtime_jobs(&self) -> PathBuf {
        self.control_plane.join("registry/runtime/jobs.json")
    }
    pub fn compiled_registry(&self) -> PathBuf {
        self.control_plane.join("registry/compiled")
    }

    pub fn has_control_plane(&self) -> bool {
        self.vac_toml().is_file()
    }

    pub fn legacy_read_only_config(&self) -> Option<PathBuf> {
        let candidate = self.legacy_control_plane.join("config.toml");
        candidate.is_file().then_some(candidate)
    }
}

pub fn discover_workspace_root(start: impl AsRef<Path>) -> PathBuf {
    let mut cursor = start.as_ref().to_path_buf();
    if cursor.is_file() {
        let _ = cursor.pop();
    }
    loop {
        if cursor.join(vac_brand::CONFIG_DIR).exists() || cursor.join("vac-rs/Cargo.toml").exists()
        {
            return cursor;
        }
        if !cursor.pop() {
            return std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        }
    }
}

#[must_use]
pub fn globish_matches(pattern: &str, value: &str) -> bool {
    if pattern == "*" || pattern == value {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        return value.ends_with(suffix);
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return value.starts_with(prefix);
    }
    false
}

#[must_use]
pub fn path_matches(pattern: &str, path: &str) -> bool {
    if pattern == path || matches!(pattern, "**" | "*") {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    if !pattern.contains('*') {
        return false;
    }
    wildcard_match(pattern, path)
}

#[must_use]
pub fn wildcard_match(pattern: &str, path: &str) -> bool {
    let mut remaining = path;
    let parts: Vec<&str> = pattern.split('*').collect();
    if let Some(first) = parts.first() {
        if !first.is_empty() && !remaining.starts_with(first) {
            return false;
        }
        if !first.is_empty() {
            remaining = &remaining[first.len()..];
        }
    }
    for part in parts.iter().skip(1) {
        if part.is_empty() {
            continue;
        }
        let Some(index) = remaining.find(part) else {
            return false;
        };
        let advance = index.saturating_add(part.len());
        remaining = &remaining[advance..];
    }
    if let Some(last) = parts.last() {
        last.is_empty() || path.ends_with(last)
    } else {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_write_to_vac_not_legacy() {
        let paths = WorkspacePaths::new("/tmp/workspace");
        assert!(
            paths
                .registry_status()
                .ends_with(".vac/registry/status.json")
        );
        assert!(paths.legacy_control_plane.ends_with(".vac"));
    }

    #[test]
    fn shared_globish_path_policy_matches() {
        assert!(path_matches("vac-rs/**", "vac-rs/core/src/lib.rs"));
        assert!(path_matches("*.rs", "main.rs"));
        assert!(globish_matches("*.example.com", "api.example.com"));
        assert!(!path_matches("docs/**", "vac-rs/core/src/lib.rs"));
    }
}
