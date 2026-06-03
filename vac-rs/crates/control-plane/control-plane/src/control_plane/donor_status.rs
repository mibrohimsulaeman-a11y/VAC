//! Donor migration status doctor.
//!
//! Renders a fail-closed view of the donor-backed capabilities discovered under
//! `.vac/capabilities/`. The doctor surfaces the donor metadata declared on
//! each manifest (source path, root target, cleanup status) and refuses to pass
//! when any donor-backed capability is missing the canonical `donor_source`
//! block, its root surface declaration, or carries an unrecognized cleanup
//! status.
//!
//! This is the structured replacement for the historical `grep -l donor_source`
//! heuristic in `scripts/check-donor-status.sh`: the manifest validator already
//! enforces the schema, and this report wires the result into the root `vac`
//! command surface so donor migration is visible (and gated) from the same
//! tooling that runs the rest of the workflow control plane.

use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;

use super::capability_manifest::CapabilityManifest;
use super::capability_manifest::CapabilityManifestError;
use super::capability_manifest::load_capability_manifest;

/// A donor-backed capability that surfaced during the scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DonorStatusEntry {
    pub manifest_path: PathBuf,
    pub capability_id: String,
    pub donor_source: String,
    pub root_target: String,
    pub cleanup_status: String,
    pub has_root_surface: bool,
}

/// A manifest that failed to parse or validate during the scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DonorStatusFailure {
    pub manifest_path: PathBuf,
    pub message: String,
}

/// Structured donor migration report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DonorStatusReport {
    pub repo_root: PathBuf,
    pub capabilities_dir: PathBuf,
    pub entries: Vec<DonorStatusEntry>,
    pub failures: Vec<DonorStatusFailure>,
}

impl DonorStatusReport {
    pub fn is_failure(&self) -> bool {
        !self.failures.is_empty()
    }

    pub fn render_text(&self) -> String {
        self.render_lines().join("\n")
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!(
            "donor status: capabilities_dir={} donor_backed={} failures={}",
            self.capabilities_dir.display(),
            self.entries.len(),
            self.failures.len()
        ));
        if self.entries.is_empty() && self.failures.is_empty() {
            lines.push(
                "donor status: no donor-backed capabilities declared yet (no donor_source blocks found)"
                    .to_string(),
            );
        }
        for entry in &self.entries {
            lines.push(format!(
                "  {} -> source={} root_target={} cleanup_status={} root_surface={}",
                entry.capability_id,
                entry.donor_source,
                entry.root_target,
                entry.cleanup_status,
                if entry.has_root_surface {
                    "yes"
                } else {
                    "missing"
                },
            ));
        }
        for failure in &self.failures {
            lines.push(format!(
                "  FAIL {}: {}",
                failure.manifest_path.display(),
                failure.message
            ));
        }
        if self.is_failure() {
            lines.push("donor status: FAIL — fix manifest validation errors above".to_string());
        } else if !self.entries.is_empty() {
            lines.push("donor status: ok".to_string());
        }
        lines
    }
}

/// Run the donor status doctor against the repo rooted at `path`.
///
/// Scans `.vac/capabilities/*.yaml`, parses each manifest, and records every
/// capability that declares a `donor_source` block. Parse or validation errors
/// are recorded as failures so the gate fails closed rather than silently
/// passing over a malformed manifest.
pub fn load_donor_status_report(path: impl AsRef<Path>) -> DonorStatusReport {
    let repo_root = path.as_ref().to_path_buf();
    let capabilities_dir = repo_root.join(".vac/capabilities");

    let mut entries = Vec::new();
    let mut failures = Vec::new();

    let read_dir = match std::fs::read_dir(&capabilities_dir) {
        Ok(read_dir) => read_dir,
        Err(err) => {
            failures.push(DonorStatusFailure {
                manifest_path: capabilities_dir.clone(),
                message: format!("read capabilities dir: {err}"),
            });
            return DonorStatusReport {
                repo_root,
                capabilities_dir,
                entries,
                failures,
            };
        }
    };

    let mut manifest_paths: Vec<PathBuf> = read_dir
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(OsStr::to_str) == Some("yaml"))
        .collect();
    manifest_paths.sort();

    for manifest_path in manifest_paths {
        match load_capability_manifest(&manifest_path) {
            Ok(manifest) => {
                if let Some(entry) = donor_entry_from_manifest(&manifest_path, &manifest) {
                    entries.push(entry);
                }
            }
            Err(err) => {
                if manifest_mentions_donor_source(&manifest_path) {
                    failures.push(DonorStatusFailure {
                        manifest_path,
                        message: failure_message(&err),
                    });
                }
            }
        }
    }

    DonorStatusReport {
        repo_root,
        capabilities_dir,
        entries,
        failures,
    }
}

fn donor_entry_from_manifest(
    manifest_path: &Path,
    manifest: &CapabilityManifest,
) -> Option<DonorStatusEntry> {
    let donor = manifest.donor_source.as_ref()?;
    Some(DonorStatusEntry {
        manifest_path: manifest_path.to_path_buf(),
        capability_id: manifest.id.clone(),
        donor_source: donor.source.clone(),
        root_target: donor.root_target.clone(),
        cleanup_status: donor.cleanup_status.to_string(),
        has_root_surface: has_root_surface(manifest),
    })
}

fn has_root_surface(manifest: &CapabilityManifest) -> bool {
    let tui_route = manifest
        .surfaces
        .tui
        .as_ref()
        .map(|routes| !routes.routes.is_empty())
        .unwrap_or(false);
    let cli_command = manifest
        .surfaces
        .cli
        .as_ref()
        .map(|cli| cli.enabled && !cli.commands.is_empty())
        .unwrap_or(false);
    tui_route || cli_command
}

fn manifest_mentions_donor_source(manifest_path: &Path) -> bool {
    match std::fs::read_to_string(manifest_path) {
        Ok(contents) => contents.contains("donor_source"),
        Err(_) => false,
    }
}

fn failure_message(error: &CapabilityManifestError) -> String {
    format!("{}: {}", error.field_path(), error.message())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    fn make_temp_root(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("vac-donor-status-{label}-{unique}"));
        fs::create_dir_all(path.join(".vac/capabilities")).expect("capabilities dir");
        path
    }

    #[test]
    fn reports_no_donor_capabilities_when_none_declared() {
        let root = make_temp_root("none");
        fs::write(
            root.join(".vac/capabilities/chat.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.chat
title: Chat
status: ready
owner: vac-rs/cli
states:
  - empty
surfaces:
  palette: true
policy:
  mutates_files: false
validation:
  commands:
    - cargo test
"#,
        )
        .expect("manifest");

        let report = load_donor_status_report(&root);
        assert!(!report.is_failure(), "report:\n{}", report.render_text());
        assert!(report.entries.is_empty());
        assert!(report.render_text().contains("donor_backed=0"));
        assert!(
            report
                .render_text()
                .contains("no donor-backed capabilities")
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn reports_well_formed_donor_capability() {
        let root = make_temp_root("ok");
        fs::write(
            root.join(".vac/capabilities/donor_migration.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.donor_migration
title: Donor migration
status: partial
reason: Plan 20 in progress
owner: vac-cli/doctor_cli
states:
  - empty
surfaces:
  palette: true
  cli:
    enabled: true
    commands:
      - vac doctor donor
donor_source:
  source: donor/vac
  root_target: vac-rs/cli/src/doctor_cli.rs
  cleanup_status: donor_only
policy:
  mutates_files: false
validation:
  commands: []
"#,
        )
        .expect("manifest");

        let report = load_donor_status_report(&root);
        assert!(!report.is_failure(), "report:\n{}", report.render_text());
        assert_eq!(report.entries.len(), 1);
        let entry = &report.entries[0];
        assert_eq!(entry.capability_id, "vac.donor_migration");
        assert_eq!(entry.donor_source, "donor/vac");
        assert_eq!(entry.root_target, "vac-rs/cli/src/doctor_cli.rs");
        assert_eq!(entry.cleanup_status, "donor_only");
        assert!(entry.has_root_surface);
        let rendered = report.render_text();
        assert!(rendered.contains("root_surface=yes"));
        assert!(rendered.contains("donor status: ok"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn fails_when_donor_source_missing_root_target() {
        let root = make_temp_root("missing-root-target");
        fs::write(
            root.join(".vac/capabilities/donor_migration.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.donor_migration
title: Donor migration
status: partial
reason: Plan 20 in progress
owner: vac-cli/doctor_cli
states:
  - empty
surfaces:
  palette: true
  cli:
    enabled: true
    commands:
      - vac doctor donor
donor_source:
  source: donor/vac
  cleanup_status: donor_only
policy:
  mutates_files: false
validation:
  commands: []
"#,
        )
        .expect("manifest");

        let report = load_donor_status_report(&root);
        assert!(report.is_failure(), "report:\n{}", report.render_text());
        let rendered = report.render_text();
        assert!(rendered.contains("donor_source.root_target"));
        assert!(rendered.contains("missing required field"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn fails_when_root_target_points_to_donor_frontend_path() {
        let root = make_temp_root("forbidden-target");
        fs::write(
            root.join(".vac/capabilities/donor_migration.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.donor_migration
title: Donor migration
status: partial
reason: Plan 20 in progress
owner: vac-cli/doctor_cli
states:
  - empty
surfaces:
  palette: true
  cli:
    enabled: true
    commands:
      - vac doctor donor
donor_source:
  source: donor/vac
  root_target: donor/vac/frontend/src/index.tsx
  cleanup_status: donor_only
policy:
  mutates_files: false
validation:
  commands: []
"#,
        )
        .expect("manifest");

        let report = load_donor_status_report(&root);
        assert!(report.is_failure(), "report:\n{}", report.render_text());
        let rendered = report.render_text();
        assert!(
            rendered.contains("donor_source.root_target"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("allowlisted VAC Rust/product path"),
            "rendered:\n{rendered}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn fails_when_donor_capability_has_no_root_surface() {
        let root = make_temp_root("missing-surface");
        // donor_source declared but only `palette: true` — no TUI route or CLI command.
        // The manifest validator already rejects this at parse time; we must
        // record the failure rather than silently skipping the manifest.
        fs::write(
            root.join(".vac/capabilities/donor_migration.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.donor_migration
title: Donor migration
status: partial
reason: Plan 20 in progress
owner: vac-cli/doctor_cli
states:
  - empty
surfaces:
  palette: true
donor_source:
  source: donor/vac
  root_target: vac-rs/cli/src/doctor_cli.rs
  cleanup_status: donor_only
policy:
  mutates_files: false
validation:
  commands: []
"#,
        )
        .expect("manifest");

        let report = load_donor_status_report(&root);
        assert!(report.is_failure(), "report:\n{}", report.render_text());
        let rendered = report.render_text();
        assert!(
            rendered.contains("donor-backed capabilities must declare a TUI route or CLI command"),
            "rendered:\n{rendered}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn fails_when_cleanup_status_is_invalid() {
        let root = make_temp_root("invalid-cleanup");
        fs::write(
            root.join(".vac/capabilities/donor_migration.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.donor_migration
title: Donor migration
status: partial
reason: Plan 20 in progress
owner: vac-cli/doctor_cli
states:
  - empty
surfaces:
  palette: true
  cli:
    enabled: true
    commands:
      - vac doctor donor
donor_source:
  source: donor/vac
  root_target: vac-rs/cli/src/doctor_cli.rs
  cleanup_status: shelved
policy:
  mutates_files: false
validation:
  commands: []
"#,
        )
        .expect("manifest");

        let report = load_donor_status_report(&root);
        assert!(report.is_failure(), "report:\n{}", report.render_text());
        let rendered = report.render_text();
        assert!(
            rendered.contains("donor_source.cleanup_status"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("donor_only, migrated, delete_candidate"),
            "rendered:\n{rendered}"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
