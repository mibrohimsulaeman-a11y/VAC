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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyPathNormalization {
    Path(String),
    EscapesWorkspace,
}

#[must_use]
pub fn normalize_policy_path(value: &str) -> PolicyPathNormalization {
    let normalized_separators = value.replace('\\', "/");
    if normalized_separators.starts_with('/') || has_windows_drive_prefix(&normalized_separators) {
        return PolicyPathNormalization::EscapesWorkspace;
    }

    let mut parts: Vec<&str> = Vec::new();
    for part in normalized_separators.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return PolicyPathNormalization::EscapesWorkspace;
                }
            }
            segment => parts.push(segment),
        }
    }

    PolicyPathNormalization::Path(parts.join("/"))
}

#[must_use]
pub fn path_matches(pattern: &str, path: &str) -> bool {
    let (PolicyPathNormalization::Path(pattern), PolicyPathNormalization::Path(path)) =
        (normalize_policy_path(pattern), normalize_policy_path(path))
    else {
        return false;
    };

    path_matches_normalized(&pattern, &path)
}

fn path_matches_normalized(pattern: &str, path: &str) -> bool {
    if pattern == path || matches!(pattern, "**" | "*") {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix("/**")
        && !prefix.contains('*')
    {
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    if !pattern.contains('*') {
        return false;
    }
    wildcard_match(pattern, path)
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic()
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

    #[test]
    fn policy_path_normalization_collapses_dot_and_internal_dot_dot() {
        assert_eq!(
            normalize_policy_path("secret/./a"),
            PolicyPathNormalization::Path("secret/a".to_string())
        );
        assert_eq!(
            normalize_policy_path("x/../secret/a"),
            PolicyPathNormalization::Path("secret/a".to_string())
        );
    }

    #[test]
    fn policy_path_matching_normalizes_pattern_and_request_path() {
        assert!(path_matches("secret/**", "secret/./a"));
        assert!(path_matches("secret/**", "x/../secret/a"));
        assert!(path_matches("**/auth/**", "src/./core/../auth/token.rs"));
    }

    #[test]
    fn policy_path_matching_rejects_workspace_escape_and_absolute_paths() {
        assert!(!path_matches("secret/**", "../secret/a"));
        assert!(!path_matches("secret/**", "/secret/a"));
        assert!(!path_matches("secret/**", "C:/workspace/secret/a"));
        assert!(!path_matches("../secret/**", "secret/a"));
    }
}
