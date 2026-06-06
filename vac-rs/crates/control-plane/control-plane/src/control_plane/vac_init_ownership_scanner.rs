#![allow(dead_code)]
//! VAC-Init source inventory and ownership scanner foundation.
//!
//! This module implements the Batch 5 contract without external crates: source
//! classification, ownership target matching, unowned/overclaimed detection,
//! and quarantine action suggestions. It is intentionally deterministic so it
//! can be validated with direct `rustc --test`.

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SourceFileCategory {
    RustSource,
    Test,
    Fixture,
    Manifest,
    Documentation,
    Script,
    Generated,
    Vendor,
    BuildOutput,
    Other,
}

impl SourceFileCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RustSource => "rust_source",
            Self::Test => "test",
            Self::Fixture => "fixture",
            Self::Manifest => "manifest",
            Self::Documentation => "documentation",
            Self::Script => "script",
            Self::Generated => "generated",
            Self::Vendor => "vendor",
            Self::BuildOutput => "build_output",
            Self::Other => "other",
        }
    }

    pub const fn should_scan_for_ownership(self) -> bool {
        matches!(
            self,
            Self::RustSource
                | Self::Test
                | Self::Fixture
                | Self::Manifest
                | Self::Documentation
                | Self::Script
        )
    }
}

impl fmt::Display for SourceFileCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OwnershipCoverageStatus {
    Complete,
    Partial,
    Hidden,
    Overclaimed,
    Unowned,
}

impl OwnershipCoverageStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Hidden => "hidden",
            Self::Overclaimed => "overclaimed",
            Self::Unowned => "unowned",
        }
    }

    pub const fn blocks_agent_write(self) -> bool {
        matches!(self, Self::Hidden | Self::Overclaimed | Self::Unowned)
    }
}

impl fmt::Display for OwnershipCoverageStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum QuarantineLevel {
    None,
    SoftQuarantine,
    HardQuarantine,
    AutoQuarantine,
}

impl QuarantineLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::SoftQuarantine => "soft_quarantine",
            Self::HardQuarantine => "hard_quarantine",
            Self::AutoQuarantine => "auto_quarantine",
        }
    }
}

impl fmt::Display for QuarantineLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OwnershipActionType {
    AssignOwner,
    ResolveOverclaim,
    RegisterCapability,
    Quarantine,
}

impl OwnershipActionType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AssignOwner => "assign_owner",
            Self::ResolveOverclaim => "resolve_overclaim",
            Self::RegisterCapability => "register_capability",
            Self::Quarantine => "quarantine",
        }
    }
}

impl fmt::Display for OwnershipActionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceInventoryEntry {
    pub path: String,
    pub crate_name: Option<String>,
    pub module: Option<String>,
    pub category: SourceFileCategory,
    pub ignored: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceInventoryReport {
    pub total_files: usize,
    pub scanned_files: usize,
    pub ignored_files: usize,
    pub entries: Vec<SourceInventoryEntry>,
}

impl SourceInventoryReport {
    pub fn category_count(&self, category: SourceFileCategory) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.category == category)
            .count()
    }

    pub fn render_text(&self) -> String {
        let mut lines = vec![format!(
            "source inventory: total={} scanned={} ignored={}",
            self.total_files, self.scanned_files, self.ignored_files
        )];
        for category in [
            SourceFileCategory::RustSource,
            SourceFileCategory::Test,
            SourceFileCategory::Fixture,
            SourceFileCategory::Manifest,
            SourceFileCategory::Documentation,
            SourceFileCategory::Script,
            SourceFileCategory::Vendor,
            SourceFileCategory::BuildOutput,
            SourceFileCategory::Generated,
            SourceFileCategory::Other,
        ] {
            let count = self.category_count(category);
            if count > 0 {
                lines.push(format!("  {category}: {count}"));
            }
        }
        lines.join("\n")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipTargetPattern {
    pub capability_id: String,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub partial: bool,
}

impl OwnershipTargetPattern {
    pub fn new(capability_id: impl Into<String>, include: Vec<String>) -> Self {
        Self {
            capability_id: capability_id.into(),
            include,
            exclude: Vec::new(),
            partial: false,
        }
    }

    pub fn matches(&self, path: &str) -> bool {
        let included = self
            .include
            .iter()
            .any(|pattern| simple_glob_match(pattern, path));
        let excluded = self
            .exclude
            .iter()
            .any(|pattern| simple_glob_match(pattern, path));
        included && !excluded
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipFileRecord {
    pub path: String,
    pub category: SourceFileCategory,
    pub status: OwnershipCoverageStatus,
    pub capability: Option<String>,
    pub claimed_by: Vec<String>,
    pub quarantine: QuarantineLevel,
    pub risk_findings: Vec<String>,
}

impl OwnershipFileRecord {
    pub fn blocks_write(&self) -> bool {
        self.status.blocks_agent_write() || self.quarantine != QuarantineLevel::None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipActionRequired {
    pub action_type: OwnershipActionType,
    pub file: String,
    pub suggestion: String,
    pub priority: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OwnershipScanSummary {
    pub total_files: usize,
    pub total_modules: usize,
    pub owned: usize,
    pub partial: usize,
    pub hidden: usize,
    pub overclaimed: usize,
    pub unowned: usize,
}

impl OwnershipScanSummary {
    pub fn coverage_percent(&self) -> f64 {
        if self.total_files == 0 {
            100.0
        } else {
            ((self.owned + self.partial) as f64 / self.total_files as f64) * 100.0
        }
    }

    pub fn is_failure(&self) -> bool {
        self.overclaimed > 0 || self.unowned > 0
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OwnershipReport {
    pub id: String,
    pub timestamp: String,
    pub workspace_root: String,
    pub summary: OwnershipScanSummary,
    pub files: Vec<OwnershipFileRecord>,
    pub actions_required: Vec<OwnershipActionRequired>,
}

impl OwnershipReport {
    pub fn is_failure(&self) -> bool {
        self.summary.is_failure()
    }

    pub fn render_text(&self) -> String {
        let mut lines = vec![format!(
            "ownership report: total={} owned={} partial={} hidden={} overclaimed={} unowned={} coverage={:.1}%",
            self.summary.total_files,
            self.summary.owned,
            self.summary.partial,
            self.summary.hidden,
            self.summary.overclaimed,
            self.summary.unowned,
            self.summary.coverage_percent()
        )];
        for action in &self.actions_required {
            lines.push(format!(
                "  action: {} file={} priority={} suggestion={}",
                action.action_type, action.file, action.priority, action.suggestion
            ));
        }
        lines.join("\n")
    }
}

pub fn classify_source_file(path: &str) -> SourceFileCategory {
    let normalized = normalize_path(path);
    if normalized.contains("/target/") || normalized.starts_with("target/") {
        return SourceFileCategory::BuildOutput;
    }
    if normalized.contains("/vendor/")
        || normalized.starts_with("vendor/")
        || normalized.contains("/node_modules/")
    {
        return SourceFileCategory::Vendor;
    }
    if normalized.contains("/generated/") || normalized.ends_with(".generated.rs") {
        return SourceFileCategory::Generated;
    }
    if normalized.ends_with(".rs") {
        if normalized.contains("/tests/")
            || normalized.ends_with("_tests.rs")
            || normalized.ends_with("test.rs")
        {
            SourceFileCategory::Test
        } else {
            SourceFileCategory::RustSource
        }
    } else if normalized.contains("tests/fixtures/") || normalized.contains("/fixtures/") {
        SourceFileCategory::Fixture
    } else if normalized.ends_with(".yaml")
        || normalized.ends_with(".yml")
        || normalized.ends_with("Cargo.toml")
        || normalized.ends_with(".toml")
    {
        SourceFileCategory::Manifest
    } else if normalized.ends_with(".md")
        || normalized.ends_with(".txt")
        || normalized.ends_with(".docx")
    {
        SourceFileCategory::Documentation
    } else if normalized.ends_with(".sh") {
        SourceFileCategory::Script
    } else {
        SourceFileCategory::Other
    }
}

pub fn build_source_inventory_from_paths(paths: &[String]) -> SourceInventoryReport {
    let mut entries = Vec::with_capacity(paths.len());
    let mut scanned_files = 0;
    let mut ignored_files = 0;
    for path in paths {
        let normalized = normalize_path(path);
        let category = classify_source_file(&normalized);
        let ignored = !category.should_scan_for_ownership();
        if ignored {
            ignored_files += 1;
        } else {
            scanned_files += 1;
        }
        entries.push(SourceInventoryEntry {
            crate_name: infer_crate_name(&normalized),
            module: infer_module_name(&normalized),
            reason: if ignored {
                Some(format!("{category} is ignored for ownership scan"))
            } else {
                None
            },
            path: normalized,
            category,
            ignored,
        });
    }
    SourceInventoryReport {
        total_files: entries.len(),
        scanned_files,
        ignored_files,
        entries,
    }
}

pub fn build_source_inventory_from_root(
    root: impl AsRef<Path>,
) -> std::io::Result<SourceInventoryReport> {
    let mut paths = Vec::new();
    collect_files(root.as_ref(), root.as_ref(), &mut paths)?;
    paths.sort();
    Ok(build_source_inventory_from_paths(&paths))
}

pub fn build_ownership_report(
    inventory: &SourceInventoryReport,
    targets: &[OwnershipTargetPattern],
    timestamp: impl Into<String>,
    workspace_root: impl Into<String>,
) -> OwnershipReport {
    let mut files = Vec::new();
    let mut actions_required = Vec::new();
    let mut summary = OwnershipScanSummary::default();
    let mut modules = BTreeSet::new();

    for entry in inventory.entries.iter().filter(|entry| !entry.ignored) {
        if let Some(module) = &entry.module {
            modules.insert(module.clone());
        }
        let mut claimed_by = targets
            .iter()
            .filter(|target| target.matches(&entry.path))
            .map(|target| target.capability_id.clone())
            .collect::<Vec<_>>();
        claimed_by.sort();
        claimed_by.dedup();

        let partial_claim = targets
            .iter()
            .any(|target| claimed_by.contains(&target.capability_id) && target.partial);

        let status = match claimed_by.len() {
            0 => OwnershipCoverageStatus::Unowned,
            1 if partial_claim => OwnershipCoverageStatus::Partial,
            1 => OwnershipCoverageStatus::Complete,
            _ => OwnershipCoverageStatus::Overclaimed,
        };
        let quarantine = quarantine_for(entry, status);
        let capability = if claimed_by.len() == 1 {
            Some(claimed_by[0].clone())
        } else {
            None
        };

        match status {
            OwnershipCoverageStatus::Complete => summary.owned += 1,
            OwnershipCoverageStatus::Partial => summary.partial += 1,
            OwnershipCoverageStatus::Hidden => summary.hidden += 1,
            OwnershipCoverageStatus::Overclaimed => summary.overclaimed += 1,
            OwnershipCoverageStatus::Unowned => summary.unowned += 1,
        }

        if status == OwnershipCoverageStatus::Unowned {
            actions_required.push(OwnershipActionRequired {
                action_type: OwnershipActionType::AssignOwner,
                file: entry.path.clone(),
                suggestion: "register capability ownership target or move file into owned module"
                    .to_string(),
                priority: if is_source_code(entry.category) {
                    "high"
                } else {
                    "medium"
                }
                .to_string(),
            });
        } else if status == OwnershipCoverageStatus::Overclaimed {
            actions_required.push(OwnershipActionRequired {
                action_type: OwnershipActionType::ResolveOverclaim,
                file: entry.path.clone(),
                suggestion: format!("choose one owner among {}", claimed_by.join(", ")),
                priority: "high".to_string(),
            });
        }

        files.push(OwnershipFileRecord {
            path: entry.path.clone(),
            category: entry.category,
            status,
            capability,
            claimed_by,
            quarantine,
            risk_findings: Vec::new(),
        });
    }

    summary.total_files = files.len();
    summary.total_modules = modules.len();

    OwnershipReport {
        id: format!("report.ownership.{}", sanitize_timestamp(&timestamp.into())),
        timestamp: "2026-05-29T00:00:00Z".to_string(),
        workspace_root: workspace_root.into(),
        summary,
        files,
        actions_required,
    }
}

pub fn ownership_targets_from_capability_paths(
    capability_id: impl Into<String>,
    include: &[&str],
    exclude: &[&str],
) -> OwnershipTargetPattern {
    OwnershipTargetPattern {
        capability_id: capability_id.into(),
        include: include
            .iter()
            .map(std::string::ToString::to_string)
            .collect(),
        exclude: exclude
            .iter()
            .map(std::string::ToString::to_string)
            .collect(),
        partial: false,
    }
}

fn collect_files(root: &Path, current: &Path, paths: &mut Vec<String>) -> std::io::Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if file_name == ".git" || file_name == "target" || file_name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, paths)?;
        } else {
            let relative = path.strip_prefix(root).unwrap_or(&path);
            paths.push(normalize_path(&relative.display().to_string()));
        }
    }
    Ok(())
}

fn quarantine_for(
    entry: &SourceInventoryEntry,
    status: OwnershipCoverageStatus,
) -> QuarantineLevel {
    match status {
        OwnershipCoverageStatus::Complete | OwnershipCoverageStatus::Partial => {
            QuarantineLevel::None
        }
        OwnershipCoverageStatus::Overclaimed => QuarantineLevel::SoftQuarantine,
        OwnershipCoverageStatus::Hidden => QuarantineLevel::SoftQuarantine,
        OwnershipCoverageStatus::Unowned => {
            if is_source_code(entry.category) {
                QuarantineLevel::HardQuarantine
            } else {
                QuarantineLevel::SoftQuarantine
            }
        }
    }
}

fn is_source_code(category: SourceFileCategory) -> bool {
    matches!(
        category,
        SourceFileCategory::RustSource | SourceFileCategory::Test
    )
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_string()
}

fn infer_crate_name(path: &str) -> Option<String> {
    let parts = path.split('/').collect::<Vec<_>>();
    if parts.len() >= 3 && parts[0] == "vac-rs" {
        Some(format!("vac-{}", parts[1].replace('_', "-")))
    } else if parts.len() >= 2 && parts[0].ends_with("-rs") {
        Some(parts[0].to_string())
    } else if path.starts_with("scripts/") {
        Some("scripts".to_string())
    } else if path.starts_with("docs/") {
        Some("docs".to_string())
    } else {
        None
    }
}

fn infer_module_name(path: &str) -> Option<String> {
    let normalized = normalize_path(path);
    if normalized.ends_with(".rs") {
        let without_ext = normalized.trim_end_matches(".rs");
        if let Some(index) = without_ext.find("/src/") {
            let module = without_ext[index + 5..].replace('/', "::");
            return Some(module.trim_end_matches("::mod").to_string());
        }
    }
    None
}

fn simple_glob_match(pattern: &str, path: &str) -> bool {
    let pattern = normalize_path(pattern);
    let path = normalize_path(path);
    if pattern == path || pattern == "**" || pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    if let Some(prefix) = pattern.strip_suffix("/**/*.rs") {
        return path.starts_with(&format!("{prefix}/")) && path.ends_with(".rs");
    }
    if let Some(suffix) = pattern.strip_prefix("**/*") {
        return path.ends_with(suffix);
    }
    if let Some(suffix) = pattern.strip_prefix("*") {
        return path.ends_with(suffix);
    }
    if pattern.contains('*') {
        let parts = pattern
            .split('*')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        let mut cursor = 0;
        for part in parts {
            let Some(found) = path[cursor..].find(part) else {
                return false;
            };
            cursor += found + part.len();
        }
        return true;
    }
    path.starts_with(&pattern)
}

fn sanitize_timestamp(timestamp: &str) -> String {
    timestamp
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_classifier_detects_build_vendor_and_rust() {
        assert_eq!(
            classify_source_file("vac-rs/core/src/lib.rs"),
            SourceFileCategory::RustSource
        );
        assert_eq!(
            classify_source_file("vac-rs/core/tests/foo.rs"),
            SourceFileCategory::Test
        );
        assert_eq!(
            classify_source_file("vac-rs/vendor/serde/src/lib.rs"),
            SourceFileCategory::Vendor
        );
        assert_eq!(
            classify_source_file("target/debug/foo.o"),
            SourceFileCategory::BuildOutput
        );
    }

    #[test]
    fn source_inventory_counts_ignored_files() {
        let paths = vec![
            "vac-rs/core/src/lib.rs".to_string(),
            "target/debug/build.o".to_string(),
            "docs/readme.md".to_string(),
        ];
        let inventory = build_source_inventory_from_paths(&paths);
        assert_eq!(inventory.total_files, 3);
        assert_eq!(inventory.scanned_files, 2);
        assert_eq!(inventory.ignored_files, 1);
    }

    #[test]
    fn crate_and_module_are_inferred_for_vac_rs_source() {
        let paths = vec!["vac-rs/core/src/control_plane/mod.rs".to_string()];
        let inventory = build_source_inventory_from_paths(&paths);
        let entry = &inventory.entries[0];
        assert_eq!(entry.crate_name.as_deref(), Some("vac-core"));
        assert_eq!(entry.module.as_deref(), Some("control_plane"));
    }

    #[test]
    fn ownership_target_matches_simple_globs() {
        let target = ownership_targets_from_capability_paths(
            "vac.core.control_plane",
            &["vac-rs/core/src/control_plane/**/*.rs"],
            &[],
        );
        assert!(target.matches("vac-rs/core/src/control_plane/mod.rs"));
        assert!(target.matches("vac-rs/core/src/control_plane/schema/envelope.rs"));
        assert!(!target.matches("vac-rs/tui/src/lib.rs"));
    }

    #[test]
    fn unowned_source_is_hard_quarantined() {
        let inventory = build_source_inventory_from_paths(&[
            "vac-rs/core/src/control_plane/mod.rs".to_string(),
            "vac-rs/tui/src/lib.rs".to_string(),
        ]);
        let target = ownership_targets_from_capability_paths(
            "vac.core.control_plane",
            &["vac-rs/core/src/control_plane/**/*.rs"],
            &[],
        );
        let report = build_ownership_report(&inventory, &[target], "2026-05-29T00:00:00Z", "/repo");
        assert_eq!(report.summary.owned, 1);
        assert_eq!(report.summary.unowned, 1);
        let unowned = report
            .files
            .iter()
            .find(|file| file.status == OwnershipCoverageStatus::Unowned)
            .unwrap();
        assert_eq!(unowned.quarantine, QuarantineLevel::HardQuarantine);
        assert!(unowned.blocks_write());
    }

    #[test]
    fn overclaimed_source_blocks_write() {
        let inventory =
            build_source_inventory_from_paths(
                &["vac-rs/core/src/control_plane/mod.rs".to_string()],
            );
        let a = ownership_targets_from_capability_paths(
            "vac.core.control_plane",
            &["vac-rs/core/src/control_plane/**/*.rs"],
            &[],
        );
        let b = ownership_targets_from_capability_paths(
            "vac.core.registry",
            &["vac-rs/core/src/control_plane/mod.rs"],
            &[],
        );
        let report = build_ownership_report(&inventory, &[a, b], "2026-05-29T00:00:00Z", "/repo");
        assert_eq!(report.summary.overclaimed, 1);
        assert_eq!(
            report.actions_required[0].action_type,
            OwnershipActionType::ResolveOverclaim
        );
        assert!(report.files[0].blocks_write());
    }

    #[test]
    fn excluded_target_does_not_claim_path() {
        let mut target = ownership_targets_from_capability_paths(
            "vac.core.control_plane",
            &["vac-rs/core/src/control_plane/**/*.rs"],
            &["vac-rs/core/src/control_plane/generated/**"],
        );
        assert!(target.matches("vac-rs/core/src/control_plane/mod.rs"));
        assert!(!target.matches("vac-rs/core/src/control_plane/generated/types.rs"));
        target.partial = true;
        assert!(target.partial);
    }

    #[test]
    fn partial_target_produces_partial_status() {
        let inventory = build_source_inventory_from_paths(&["docs/vac-init/README.md".to_string()]);
        let mut target = ownership_targets_from_capability_paths("vac.docs", &["docs/**"], &[]);
        target.partial = true;
        let report = build_ownership_report(&inventory, &[target], "2026-05-29T00:00:00Z", "/repo");
        assert_eq!(report.summary.partial, 1);
        assert_eq!(report.summary.coverage_percent(), 100.0);
    }

    #[test]
    fn report_render_includes_actions() {
        let inventory =
            build_source_inventory_from_paths(&["vac-rs/core/src/unowned.rs".to_string()]);
        let report = build_ownership_report(&inventory, &[], "2026-05-29T00:00:00Z", "/repo");
        let text = report.render_text();
        assert!(text.contains("ownership report"));
        assert!(text.contains("assign_owner"));
    }
}
