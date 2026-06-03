//! Root feature conversion report for Plan 19 hardening (Phase 4).
//!
//! Tracks whether each canonical root feature has been fully promoted from
//! seed planning into a manifest-backed capability, and whether manifest
//! ownership still maps to source domains that exist on disk.
//!
//! Phase 4 additions (Plan 19 audit P1):
//! 1. Hidden-domain entries are surfaced at `Blocked` severity via
//!    [`entry_severity`] so `registry_diagnostics` (lane 2.6) can promote
//!    unowned source-domain rows from warning into a hard failure.
//! 2. Overclaimed detection now spans `crates`, `expected_source_roots`,
//!    `expected_surfaces`, `expected_policy`, `expected_validation`, and
//!    `expected_ownership` — not just `modules`.
//! 3. The typed `RootSeedCapabilityRequirement` fields (`expected_*`) are
//!    enforced by [`classify_root_feature`] instead of staying informational.

use std::fmt;
use std::fs;
use std::path::Path;

use super::capability_manifest::CapabilityManifest;
use super::capability_manifest::CapabilityOwnership;
use super::capability_manifest::CapabilityStatus;
use super::ownership_scan::OwnershipScanReport;
use super::registry::ControlPlaneRegistry;
use super::root_feature_catalog::ROOT_SEED_CAPABILITY_REQUIREMENTS;
use super::root_feature_catalog::RootSeedCapabilityRequirement;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionState {
    Complete,
    Partial,
    Hidden,
    Overclaimed,
}

impl ConversionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConversionState::Complete => "complete",
            ConversionState::Partial => "partial",
            ConversionState::Hidden => "hidden",
            ConversionState::Overclaimed => "overclaimed",
        }
    }
}

impl fmt::Display for ConversionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Phase 4 severity classification for conversion entries.
///
/// `registry_diagnostics` (lane 2.6) maps this onto `RegistryDiagnosticSeverity`
/// so hidden-domain rows surface as `Blocked` instead of `Warning`. Keeping
/// the mapping inside lane 2.15 lets the audit gate evolve without touching
/// the diagnostics emitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionSeverity {
    Info,
    Warning,
    Blocked,
}

impl ConversionSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConversionSeverity::Info => "info",
            ConversionSeverity::Warning => "warning",
            ConversionSeverity::Blocked => "blocked",
        }
    }
}

impl fmt::Display for ConversionSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Severity for a specific conversion entry.
///
/// Hidden source domains and capabilities missing ownership both promote to
/// `Blocked` per Plan 19 audit P1 (hidden-domain → Blocked, not Warning).
/// Overclaimed entries (including the new missing-expected-* signals) also
/// stay at `Blocked`.
pub fn entry_severity(entry: &RootFeatureConversionEntry) -> ConversionSeverity {
    match entry.state {
        ConversionState::Complete => ConversionSeverity::Info,
        ConversionState::Partial => ConversionSeverity::Warning,
        ConversionState::Hidden | ConversionState::Overclaimed => ConversionSeverity::Blocked,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootFeatureConversionEntry {
    pub feature: String,
    pub capability_id: String,
    pub state: ConversionState,
    pub status: Option<CapabilityStatus>,
    pub reason: Option<String>,
    pub claimed_modules: Vec<String>,
    pub overclaimed_modules: Vec<String>,
    /// Capability ownership crates that have no matching `Cargo.toml` in the
    /// source inventory. Phase 4 audit P1: overclaimed must cover crates,
    /// not just modules.
    pub overclaimed_crates: Vec<String>,
    /// `expected_source_roots` entries that did not resolve to a file or
    /// directory on disk.
    pub missing_expected_source_roots: Vec<String>,
    /// `expected_surfaces` IDs that the capability manifest does not declare.
    pub missing_expected_surfaces: Vec<String>,
    /// `expected_policy` IDs not satisfied by the capability policy block.
    pub missing_expected_policy: Vec<String>,
    /// `expected_validation` entries not present in `validation.commands`
    /// or `validation.gates`.
    pub missing_expected_validation: Vec<String>,
    /// `expected_ownership` entries (e.g. `vac-core::chat`) not satisfied by
    /// the capability ownership block.
    pub missing_expected_ownership: Vec<String>,
}

impl RootFeatureConversionEntry {
    /// True when any Phase-4 enforcement signal is non-empty.
    pub fn has_expected_drift(&self) -> bool {
        !self.overclaimed_crates.is_empty()
            || !self.missing_expected_source_roots.is_empty()
            || !self.missing_expected_surfaces.is_empty()
            || !self.missing_expected_policy.is_empty()
            || !self.missing_expected_validation.is_empty()
            || !self.missing_expected_ownership.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RootFeatureConversionReport {
    entries: Vec<RootFeatureConversionEntry>,
}

impl RootFeatureConversionReport {
    pub fn entries(&self) -> &[RootFeatureConversionEntry] {
        &self.entries
    }

    pub fn complete_count(&self) -> usize {
        self.count_state(ConversionState::Complete)
    }

    pub fn partial_count(&self) -> usize {
        self.count_state(ConversionState::Partial)
    }

    pub fn hidden_count(&self) -> usize {
        self.count_state(ConversionState::Hidden)
    }

    pub fn overclaimed_count(&self) -> usize {
        self.count_state(ConversionState::Overclaimed)
    }

    fn count_state(&self, state: ConversionState) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.state == state)
            .count()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "root feature conversion: complete={} partial={} hidden={} overclaimed={}",
            self.complete_count(),
            self.partial_count(),
            self.hidden_count(),
            self.overclaimed_count(),
        )
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = Vec::with_capacity(self.entries.len() + 1);
        lines.push(self.summary_line());
        for entry in &self.entries {
            let status = entry.status.map(status_str).unwrap_or("missing");
            let mut line = if let Some(source_domain) = entry.capability_id.strip_prefix("source:")
            {
                format!("  source:{source_domain} {} status={status}", entry.state)
            } else {
                format!(
                    "  {:<14} -> {:<22} [{}] status={}",
                    entry.feature, entry.capability_id, entry.state, status
                )
            };
            if let Some(reason) = &entry.reason {
                line.push_str(&format!(" reason={reason:?}"));
            }
            if !entry.claimed_modules.is_empty() {
                line.push_str(&format!(
                    " claimed_modules=[{}]",
                    entry.claimed_modules.join(", ")
                ));
            }
            if !entry.overclaimed_modules.is_empty() {
                line.push_str(&format!(
                    " overclaimed_modules=[{}]",
                    entry.overclaimed_modules.join(", ")
                ));
            }
            if !entry.overclaimed_crates.is_empty() {
                line.push_str(&format!(
                    " overclaimed_crates=[{}]",
                    entry.overclaimed_crates.join(", ")
                ));
            }
            if !entry.missing_expected_source_roots.is_empty() {
                line.push_str(&format!(
                    " missing_expected_source_roots=[{}]",
                    entry.missing_expected_source_roots.join(", ")
                ));
            }
            if !entry.missing_expected_surfaces.is_empty() {
                line.push_str(&format!(
                    " missing_expected_surfaces=[{}]",
                    entry.missing_expected_surfaces.join(", ")
                ));
            }
            if !entry.missing_expected_policy.is_empty() {
                line.push_str(&format!(
                    " missing_expected_policy=[{}]",
                    entry.missing_expected_policy.join(", ")
                ));
            }
            if !entry.missing_expected_validation.is_empty() {
                line.push_str(&format!(
                    " missing_expected_validation=[{}]",
                    entry.missing_expected_validation.join(", ")
                ));
            }
            if !entry.missing_expected_ownership.is_empty() {
                line.push_str(&format!(
                    " missing_expected_ownership=[{}]",
                    entry.missing_expected_ownership.join(", ")
                ));
            }
            lines.push(line);
        }
        lines
    }

    pub fn render_text(&self) -> String {
        self.render_lines().join("\n")
    }
}

pub fn status_str(status: CapabilityStatus) -> &'static str {
    match status {
        CapabilityStatus::Planned => "planned",
        CapabilityStatus::Partial => "partial",
        CapabilityStatus::Ready => "ready",
        CapabilityStatus::Deprecated => "deprecated",
        CapabilityStatus::Disabled => "disabled",
    }
}

pub fn build_root_feature_conversion_report(
    registry: &ControlPlaneRegistry,
    ownership_scan: &OwnershipScanReport,
) -> RootFeatureConversionReport {
    // Phase 4: derive the repo root so `expected_source_roots` can be checked
    // against the actual workspace tree. `.vac/` lives one level below the
    // repo root by convention.
    let repo_root = registry
        .vac_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| registry.vac_root.clone());
    let mut entries = ROOT_SEED_CAPABILITY_REQUIREMENTS
        .iter()
        .map(|req| classify_root_feature(registry, ownership_scan, &repo_root, req))
        .collect::<Vec<_>>();
    entries.extend(
        ownership_scan
            .unowned_source_domains()
            .iter()
            .map(|source| RootFeatureConversionEntry {
                feature: format!("source:{}:{}", source.crate_name, source.module),
                capability_id: format!("source:{}/{}", source.crate_name, source.module),
                state: ConversionState::Hidden,
                status: None,
                reason: Some("unowned source domain".to_string()),
                claimed_modules: Vec::new(),
                overclaimed_modules: Vec::new(),
                overclaimed_crates: Vec::new(),
                missing_expected_source_roots: Vec::new(),
                missing_expected_surfaces: Vec::new(),
                missing_expected_policy: Vec::new(),
                missing_expected_validation: Vec::new(),
                missing_expected_ownership: Vec::new(),
            }),
    );
    RootFeatureConversionReport { entries }
}

fn classify_root_feature(
    registry: &ControlPlaneRegistry,
    ownership_scan: &OwnershipScanReport,
    repo_root: &Path,
    req: &RootSeedCapabilityRequirement,
) -> RootFeatureConversionEntry {
    let manifest = match find_capability(registry, req.capability_id) {
        None => {
            return RootFeatureConversionEntry {
                feature: req.feature.to_string(),
                capability_id: req.capability_id.to_string(),
                state: ConversionState::Hidden,
                status: None,
                reason: None,
                claimed_modules: Vec::new(),
                overclaimed_modules: Vec::new(),
                overclaimed_crates: Vec::new(),
                missing_expected_source_roots: Vec::new(),
                missing_expected_surfaces: Vec::new(),
                missing_expected_policy: Vec::new(),
                missing_expected_validation: Vec::new(),
                missing_expected_ownership: Vec::new(),
            };
        }
        Some(manifest) => manifest,
    };

    let claimed_modules = manifest
        .ownership
        .as_ref()
        .map(|ownership| ownership.modules.clone())
        .unwrap_or_default();
    let overclaimed_modules = manifest
        .ownership
        .as_ref()
        .map(|ownership| ownership_scan.missing_claimed_modules(ownership))
        .unwrap_or_default();
    let overclaimed_crates = manifest
        .ownership
        .as_ref()
        .map(|ownership| missing_claimed_crates(ownership_scan, ownership))
        .unwrap_or_default();

    // Phase 4 enforcement is only meaningful when the capability actually
    // carries ownership metadata. Hidden capabilities are surfaced through
    // their own diagnostic, so we keep their drift fields empty for clarity.
    let has_ownership_metadata = manifest
        .ownership
        .as_ref()
        .map(|o| !o.crates.is_empty() || !o.modules.is_empty() || !o.targets.is_empty())
        .unwrap_or(false);

    let (
        missing_expected_source_roots,
        missing_expected_surfaces,
        missing_expected_policy,
        missing_expected_validation,
        missing_expected_ownership,
    ) = if has_ownership_metadata {
        (
            req.expected_source_roots
                .iter()
                .filter(|root| !expected_source_root_satisfied(repo_root, root))
                .map(|root| root.to_string())
                .collect::<Vec<_>>(),
            req.expected_surfaces
                .iter()
                .filter(|surface_id| !capability_satisfies_expected_surface(manifest, surface_id))
                .map(|surface_id| surface_id.to_string())
                .collect::<Vec<_>>(),
            req.expected_policy
                .iter()
                .filter(|policy_id| !capability_satisfies_expected_policy(manifest, policy_id))
                .map(|policy_id| policy_id.to_string())
                .collect::<Vec<_>>(),
            req.expected_validation
                .iter()
                .filter(|cmd| !capability_satisfies_expected_validation(manifest, cmd))
                .map(|cmd| cmd.to_string())
                .collect::<Vec<_>>(),
            req.expected_ownership
                .iter()
                .filter(|expected| !capability_satisfies_expected_ownership(manifest, expected))
                .map(|expected| expected.to_string())
                .collect::<Vec<_>>(),
        )
    } else {
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new())
    };

    let any_expected_drift = !overclaimed_crates.is_empty()
        || !missing_expected_source_roots.is_empty()
        || !missing_expected_surfaces.is_empty()
        || !missing_expected_policy.is_empty()
        || !missing_expected_validation.is_empty()
        || !missing_expected_ownership.is_empty();

    let state = if !has_ownership_metadata {
        // Capability without ownership metadata -> Hidden.
        ConversionState::Hidden
    } else if manifest.status != CapabilityStatus::Ready {
        // Planned/Partial capability stays Partial regardless of drift; the
        // partial label already signals "evidence not complete".
        ConversionState::Partial
    } else if !overclaimed_modules.is_empty() || any_expected_drift {
        // Ready capability that overclaims modules/crates or fails any of the
        // canonical Expected_* checks -> Overclaimed (Phase 4 audit P1).
        ConversionState::Overclaimed
    } else {
        ConversionState::Complete
    };

    RootFeatureConversionEntry {
        feature: req.feature.to_string(),
        capability_id: req.capability_id.to_string(),
        state,
        status: Some(manifest.status),
        reason: manifest.reason.clone(),
        claimed_modules,
        overclaimed_modules,
        overclaimed_crates,
        missing_expected_source_roots,
        missing_expected_surfaces,
        missing_expected_policy,
        missing_expected_validation,
        missing_expected_ownership,
    }
}

fn find_capability<'a>(
    registry: &'a ControlPlaneRegistry,
    capability_id: &str,
) -> Option<&'a CapabilityManifest> {
    registry
        .capabilities
        .manifests
        .iter()
        .map(|located| &located.manifest)
        .find(|manifest| manifest.id == capability_id)
}

/// Crates declared in `ownership.crates` that are not represented in the
/// source inventory (no matching `Cargo.toml`).
fn missing_claimed_crates(
    ownership_scan: &OwnershipScanReport,
    ownership: &CapabilityOwnership,
) -> Vec<String> {
    let inventory = ownership_scan.source_inventory();
    ownership
        .crates
        .iter()
        .filter(|crate_name| {
            !inventory
                .entries()
                .iter()
                .any(|entry| entry.crate_name == **crate_name)
        })
        .cloned()
        .collect()
}

/// Check whether `pattern` resolves to a file or directory on disk relative
/// to the repository root. Supports three pattern flavors:
///
/// 1. Trailing `/` -> must be an existing directory.
/// 2. Single `*` wildcard in the filename -> parent dir must exist and at
///    least one entry must match the prefix/suffix split.
/// 3. Otherwise -> literal path existence (`path.exists()`).
fn expected_source_root_satisfied(repo_root: &Path, pattern: &str) -> bool {
    if pattern.ends_with('/') {
        return repo_root.join(pattern.trim_end_matches('/')).is_dir();
    }
    if !pattern.contains('*') {
        return repo_root.join(pattern).exists();
    }
    let path = Path::new(pattern);
    let Some(parent) = path.parent() else {
        return false;
    };
    let Some(filename) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let dir = repo_root.join(parent);
    if !dir.is_dir() {
        return false;
    }
    let Some((prefix, suffix)) = filename.split_once('*') else {
        return false;
    };
    let Ok(read_dir) = fs::read_dir(&dir) else {
        return false;
    };
    read_dir.flatten().any(|entry| {
        entry
            .file_name()
            .to_str()
            .map(|name| name.starts_with(prefix) && name.ends_with(suffix))
            .unwrap_or(false)
    })
}

fn capability_satisfies_expected_surface(manifest: &CapabilityManifest, surface_id: &str) -> bool {
    let surfaces = &manifest.surfaces;
    match surface_id {
        "surface.tui" => surfaces
            .tui
            .as_ref()
            .is_some_and(|tui| !tui.routes.is_empty()),
        "surface.slash" => surfaces
            .slash
            .as_ref()
            .is_some_and(|slash| !slash.commands.is_empty()),
        "surface.palette" => surfaces.palette,
        "surface.cli" => surfaces.cli.as_ref().is_some_and(|cli| cli.enabled),
        _ => true,
    }
}

/// Heuristic policy satisfaction check. The capability manifest does not
/// carry direct policy-id references, so we match each canonical policy id
/// to the most specific field signal available, and fall back to a generic
/// "policy block has metadata" check for less structured ids.
fn capability_satisfies_expected_policy(manifest: &CapabilityManifest, policy_id: &str) -> bool {
    let policy = &manifest.policy;
    let has_any_metadata = policy.risk.is_some()
        || policy.mutates_files.is_some()
        || policy.network.is_some()
        || policy.redaction.is_some()
        || !policy.approval_required_for.is_empty();
    match policy_id {
        "vac.policy.approval" => !policy.approval_required_for.is_empty() || policy.risk.is_some(),
        "vac.policy.filesystem" => policy.mutates_files.is_some(),
        "vac.policy.network" => policy.network.is_some(),
        "vac.policy.redaction" => policy.redaction.is_some(),
        "vac.default-local" => true,
        "vac.policy.sandbox" | "vac.policy.tools" => has_any_metadata,
        _ => true,
    }
}

fn capability_satisfies_expected_validation(manifest: &CapabilityManifest, expected: &str) -> bool {
    manifest
        .validation
        .commands
        .iter()
        .any(|cmd| cmd == expected)
        || manifest
            .validation
            .gates
            .iter()
            .any(|gate| gate == expected)
}

/// Match an `expected_ownership` entry against the capability ownership
/// block. Accepts `crate` (just the crate name) or `crate::module` form.
/// Module matches accept dotted child paths (so an expected
/// `vac-core::runtime` matches a declared `vac-core::runtime.chat_session`).
fn capability_satisfies_expected_ownership(manifest: &CapabilityManifest, expected: &str) -> bool {
    let Some(ownership) = manifest.ownership.as_ref() else {
        return false;
    };
    let (crate_name, module_opt) = match expected.split_once("::") {
        Some((crate_name, module)) => (crate_name, Some(module)),
        None => (expected, None),
    };
    let Some(module) = module_opt else {
        return ownership.crates.iter().any(|c| c == crate_name);
    };
    let crate_listed = ownership.crates.iter().any(|c| c == crate_name);
    let module_listed = ownership
        .modules
        .iter()
        .any(|m| module_dotted_prefix_match(m, module));
    if crate_listed && module_listed {
        return true;
    }
    ownership.targets.iter().any(|target| {
        target.crate_name == crate_name && module_dotted_prefix_match(&target.module, module)
    })
}

fn module_dotted_prefix_match(declared: &str, expected: &str) -> bool {
    if declared == expected {
        return true;
    }
    if let Some(suffix) = expected.strip_prefix(declared) {
        return suffix.starts_with('.');
    }
    if let Some(suffix) = declared.strip_prefix(expected) {
        return suffix.starts_with('.');
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control_plane::load_control_plane_registry_report;
    use crate::control_plane::load_ownership_scan_report;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn conversion_state_display_matches_str() {
        assert_eq!(ConversionState::Complete.as_str(), "complete");
        assert_eq!(ConversionState::Partial.as_str(), "partial");
        assert_eq!(ConversionState::Hidden.as_str(), "hidden");
        assert_eq!(ConversionState::Overclaimed.as_str(), "overclaimed");
        assert_eq!(format!("{}", ConversionState::Complete), "complete");
    }

    #[test]
    fn empty_report_summary_is_zero() {
        let report = RootFeatureConversionReport::default();
        assert_eq!(report.complete_count(), 0);
        assert_eq!(report.partial_count(), 0);
        assert_eq!(report.hidden_count(), 0);
        assert_eq!(report.overclaimed_count(), 0);
        assert_eq!(
            report.summary_line(),
            "root feature conversion: complete=0 partial=0 hidden=0 overclaimed=0"
        );
    }

    #[test]
    fn missing_root_capabilities_are_hidden() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join(".vac")).expect("vac root");
        let registry_report = load_control_plane_registry_report(tempdir.path());
        let registry = registry_report.registry().expect("registry");
        let ownership_scan = load_ownership_scan_report(tempdir.path());

        let report = build_root_feature_conversion_report(registry, &ownership_scan);

        assert_eq!(
            report.hidden_count(),
            ROOT_SEED_CAPABILITY_REQUIREMENTS.len()
        );
        assert!(report.render_text().contains("[hidden] status=missing"));
    }

    #[test]
    fn capability_without_ownership_is_hidden() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
        fs::create_dir_all(tempdir.path().join("vac-rs/core/src")).expect("source dir");
        fs::write(
            tempdir.path().join(".vac/capabilities/chat.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.chat
title: Chat
status: ready
owner:
  crate: vac-core
  module: chatwidget
states:
  - active
surfaces:
  palette: true
policy:
  risk: safe_read
validation:
  commands:
    - cargo check
"#,
        )
        .expect("capability manifest");
        fs::write(
            tempdir.path().join("vac-rs/core/Cargo.toml"),
            r#"
[package]
name = "vac-core"
version = "0.0.0"
edition = "2024"
"#,
        )
        .expect("cargo toml");
        fs::write(
            tempdir.path().join("vac-rs/core/src/lib.rs"),
            "pub mod chatwidget;\n",
        )
        .expect("lib rs");
        fs::write(tempdir.path().join("vac-rs/core/src/chatwidget.rs"), "").expect("chatwidget rs");

        let registry_report = load_control_plane_registry_report(tempdir.path());
        let registry = registry_report.registry().expect("registry");
        let ownership_scan = load_ownership_scan_report(tempdir.path());
        let report = build_root_feature_conversion_report(registry, &ownership_scan);
        let chat = report
            .entries()
            .iter()
            .find(|entry| entry.capability_id == "vac.chat")
            .expect("chat entry");

        assert_eq!(chat.state, ConversionState::Hidden);
        assert!(chat.claimed_modules.is_empty());
        assert!(report.render_text().contains("[hidden] status=ready"));
    }

    #[test]
    fn ready_capability_with_missing_claimed_module_is_overclaimed() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
        fs::create_dir_all(tempdir.path().join("vac-rs/core/src")).expect("source dir");
        fs::write(
            tempdir.path().join(".vac/capabilities/chat.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.chat
title: Chat
status: ready
owner:
  crate: vac-core
  module: missing_chat
states:
  - active
ownership:
  crates:
    - vac-core
  modules:
    - missing_chat
surfaces:
  palette: true
policy:
  risk: safe_read
validation:
  commands:
    - cargo check
"#,
        )
        .expect("capability manifest");
        fs::write(
            tempdir.path().join("vac-rs/core/Cargo.toml"),
            r#"
[package]
name = "vac-core"
version = "0.0.0"
edition = "2024"
"#,
        )
        .expect("cargo toml");
        fs::write(
            tempdir.path().join("vac-rs/core/src/lib.rs"),
            "pub mod chatwidget;\n",
        )
        .expect("lib rs");
        fs::write(tempdir.path().join("vac-rs/core/src/chatwidget.rs"), "").expect("chatwidget rs");

        let registry_report = load_control_plane_registry_report(tempdir.path());
        let registry = registry_report.registry().expect("registry");
        let ownership_scan = load_ownership_scan_report(tempdir.path());
        let report = build_root_feature_conversion_report(registry, &ownership_scan);
        let chat = report
            .entries()
            .iter()
            .find(|entry| entry.capability_id == "vac.chat")
            .expect("chat entry");

        assert_eq!(chat.state, ConversionState::Overclaimed);
        assert_eq!(chat.overclaimed_modules, vec!["missing_chat".to_string()]);
        assert!(
            report
                .render_text()
                .contains("overclaimed_modules=[missing_chat]")
        );
    }

    #[test]
    fn conversion_report_includes_hidden_source_domain_entries() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
        fs::create_dir_all(tempdir.path().join("vac-rs/core/src")).expect("source dir");
        fs::write(
            tempdir.path().join("vac-rs/core/Cargo.toml"),
            r#"
[package]
name = "vac-core"
version = "0.0.0"
edition = "2024"
"#,
        )
        .expect("cargo toml");
        fs::write(
            tempdir.path().join("vac-rs/core/src/lib.rs"),
            "pub mod hidden_domain;\n",
        )
        .expect("lib rs");
        fs::write(tempdir.path().join("vac-rs/core/src/hidden_domain.rs"), "")
            .expect("hidden domain");

        let registry_report = load_control_plane_registry_report(tempdir.path());
        let registry = registry_report.registry().expect("registry");
        let ownership_scan = load_ownership_scan_report(tempdir.path());
        let report = build_root_feature_conversion_report(registry, &ownership_scan);

        let entry = report
            .entries()
            .iter()
            .find(|entry| entry.capability_id == "source:vac-core/hidden_domain")
            .expect("hidden source entry");
        assert_eq!(entry.state, ConversionState::Hidden);
        assert_eq!(entry.reason.as_deref(), Some("unowned source domain"));
        assert!(
            report
                .render_text()
                .contains("source:vac-core/hidden_domain")
        );
        // Phase 4 audit P1: hidden source-domain entries must surface at
        // Blocked severity (was Warning).
        assert_eq!(entry_severity(entry), ConversionSeverity::Blocked);
    }

    // ---- Phase 4 (Plan 19 audit P1) additions ----

    fn write_vac_tui_minimal_workspace(temp_root: &Path) {
        fs::create_dir_all(temp_root.join(".vac/capabilities")).expect("capabilities dir");
        fs::create_dir_all(temp_root.join("vac-rs/tui/src")).expect("tui src");
        fs::write(
            temp_root.join("vac-rs/tui/Cargo.toml"),
            r#"
[package]
name = "vac-tui"
version = "0.0.0"
edition = "2024"
"#,
        )
        .expect("cargo toml");
        fs::write(temp_root.join("vac-rs/tui/src/lib.rs"), "// vac-tui\n").expect("lib rs");
    }

    fn write_capability_yaml(temp_root: &Path, file: &str, body: &str) {
        fs::write(temp_root.join(".vac/capabilities").join(file), body)
            .expect("capability manifest");
    }

    #[test]
    fn entry_severity_promotes_hidden_and_overclaimed_to_blocked() {
        let base = RootFeatureConversionEntry {
            feature: "chat".into(),
            capability_id: "vac.chat".into(),
            state: ConversionState::Hidden,
            status: None,
            reason: None,
            claimed_modules: vec![],
            overclaimed_modules: vec![],
            overclaimed_crates: vec![],
            missing_expected_source_roots: vec![],
            missing_expected_surfaces: vec![],
            missing_expected_policy: vec![],
            missing_expected_validation: vec![],
            missing_expected_ownership: vec![],
        };
        assert_eq!(entry_severity(&base), ConversionSeverity::Blocked);

        let mut overclaimed = base.clone();
        overclaimed.state = ConversionState::Overclaimed;
        assert_eq!(entry_severity(&overclaimed), ConversionSeverity::Blocked);

        let mut partial = base.clone();
        partial.state = ConversionState::Partial;
        assert_eq!(entry_severity(&partial), ConversionSeverity::Warning);

        let mut complete = base.clone();
        complete.state = ConversionState::Complete;
        assert_eq!(entry_severity(&complete), ConversionSeverity::Info);
    }

    #[test]
    fn expected_source_root_satisfied_handles_directory_and_glob_patterns() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("vac-rs/core/src/runtime")).expect("runtime dir");
        fs::write(
            tempdir
                .path()
                .join("vac-rs/core/src/runtime/chat_session.rs"),
            "",
        )
        .expect("chat_session rs");
        fs::create_dir_all(tempdir.path().join("vac-rs/core/src/chat")).expect("chat dir");

        let repo_root = tempdir.path();
        assert!(expected_source_root_satisfied(
            repo_root,
            "vac-rs/core/src/chat/"
        ));
        assert!(expected_source_root_satisfied(
            repo_root,
            "vac-rs/core/src/runtime/chat_*.rs"
        ));
        assert!(!expected_source_root_satisfied(
            repo_root,
            "vac-rs/core/src/missing/"
        ));
        assert!(!expected_source_root_satisfied(
            repo_root,
            "vac-rs/core/src/runtime/missing_*.rs"
        ));
        assert!(expected_source_root_satisfied(
            repo_root,
            "vac-rs/core/src/runtime/chat_session.rs"
        ));
    }

    #[test]
    fn ready_capability_missing_expected_surface_is_overclaimed() {
        let tempdir = tempdir().expect("tempdir");
        write_vac_tui_minimal_workspace(tempdir.path());
        // vac.tui catalog: expected_surfaces=[surface.tui]. Manifest declares
        // only `palette: true` so surface.tui is missing.
        write_capability_yaml(
            tempdir.path(),
            "tui.yaml",
            r#"
schema_version: 1
kind: capability
id: vac.tui
title: TUI
status: ready
owner:
  crate: vac-surface-tui
  module: lib
ownership:
  crates:
    - vac-tui
  modules:
    - lib
states:
  - active
surfaces:
  palette: true
policy:
  risk: safe_read
validation:
  commands:
    - cargo test -p vac-surface-tui
    - vac doctor surfaces
"#,
        );

        let registry_report = load_control_plane_registry_report(tempdir.path());
        let registry = registry_report.registry().expect("registry");
        let ownership_scan = load_ownership_scan_report(tempdir.path());
        let report = build_root_feature_conversion_report(registry, &ownership_scan);

        let tui = report
            .entries()
            .iter()
            .find(|entry| entry.capability_id == "vac.tui")
            .expect("tui entry");
        assert_eq!(tui.state, ConversionState::Overclaimed);
        assert!(
            tui.missing_expected_surfaces
                .contains(&"surface.tui".to_string()),
            "expected surface.tui to be reported missing, got: {:?}",
            tui.missing_expected_surfaces
        );
        assert!(tui.missing_expected_validation.is_empty());
        assert_eq!(entry_severity(tui), ConversionSeverity::Blocked);
    }

    #[test]
    fn ready_capability_with_orphan_crate_is_overclaimed() {
        let tempdir = tempdir().expect("tempdir");
        write_vac_tui_minimal_workspace(tempdir.path());
        // Claim an extra crate that does not exist on disk.
        write_capability_yaml(
            tempdir.path(),
            "tui.yaml",
            r#"
schema_version: 1
kind: capability
id: vac.tui
title: TUI
status: ready
owner:
  crate: vac-surface-tui
  module: lib
ownership:
  crates:
    - vac-tui
    - vac-ghost
  modules:
    - lib
states:
  - active
surfaces:
  tui:
    routes:
      - /tui
policy:
  risk: safe_read
validation:
  commands:
    - cargo test -p vac-surface-tui
    - vac doctor surfaces
"#,
        );

        let registry_report = load_control_plane_registry_report(tempdir.path());
        let registry = registry_report.registry().expect("registry");
        let ownership_scan = load_ownership_scan_report(tempdir.path());
        let report = build_root_feature_conversion_report(registry, &ownership_scan);

        let tui = report
            .entries()
            .iter()
            .find(|entry| entry.capability_id == "vac.tui")
            .expect("tui entry");
        assert_eq!(tui.state, ConversionState::Overclaimed);
        assert_eq!(tui.overclaimed_crates, vec!["vac-ghost".to_string()]);
        assert!(
            report
                .render_text()
                .contains("overclaimed_crates=[vac-ghost]")
        );
    }

    #[test]
    fn capability_satisfies_expected_ownership_matches_crate_and_dotted_module() {
        // Unit-style check for the helper without spinning up a registry.
        use crate::control_plane::capability_manifest::{
            CapabilityManifest, CapabilityOwnership, CapabilityOwnershipTarget,
        };
        let manifest_with_target = CapabilityManifest {
            ownership: Some(CapabilityOwnership {
                crates: vec!["vac-core".into()],
                modules: vec!["runtime".into()],
                test_only: false,
                retired: false,
                deletion_plan: None,
                targets: vec![CapabilityOwnershipTarget {
                    crate_name: "vac-core".into(),
                    module: "runtime.chat_session".into(),
                    test_only: false,
                    retired: false,
                }],
            }),
            ..CapabilityManifest::test_stub("vac.chat")
        };

        assert!(capability_satisfies_expected_ownership(
            &manifest_with_target,
            "vac-core"
        ));
        assert!(capability_satisfies_expected_ownership(
            &manifest_with_target,
            "vac-core::runtime"
        ));
        assert!(capability_satisfies_expected_ownership(
            &manifest_with_target,
            "vac-core::runtime.chat_session"
        ));
        assert!(!capability_satisfies_expected_ownership(
            &manifest_with_target,
            "vac-cli"
        ));
        assert!(!capability_satisfies_expected_ownership(
            &manifest_with_target,
            "vac-core::sessions"
        ));
    }
}
