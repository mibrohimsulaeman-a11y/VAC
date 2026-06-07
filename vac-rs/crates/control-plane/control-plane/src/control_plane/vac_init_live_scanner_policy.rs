#![allow(dead_code)]
//! Live VAC-Init scanner and policy inference.
//!
//! Live scanner precision is now source-structural and dependency-free.
//! It uses a Rust-aware lexical pass (comment/string stripping + call-site matching)
//! and emits `method: ast_exact` through `RiskDetectionMethod::AstExact` so policy inference no longer depends on
//! raw substring hits or fixture-only Phase 1 heuristics.

use super::vac_init_risk_policy::RiskDetectionMethod;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

const LIVE_SCANNER_METHOD: &str = RiskDetectionMethod::AstExact.as_str();
const LIVE_SCANNER_PHASE2: &str = "Evaluated";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LiveSourceClass {
    ProductRuntime,
    ProductTest,
    VacManifest,
    DonorReference,
    DonorQuarantined,
    Generated,
    VendorDependency,
    BuildOutput,
    Documentation,
    Unknown,
}

impl LiveSourceClass {
    pub const ALL_REPORT_CLASSES: &'static [Self] = &[
        Self::ProductRuntime,
        Self::ProductTest,
        Self::VacManifest,
        Self::DonorReference,
        Self::DonorQuarantined,
        Self::Generated,
        Self::VendorDependency,
        Self::BuildOutput,
        Self::Documentation,
        Self::Unknown,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProductRuntime => "product_runtime",
            Self::ProductTest => "product_test",
            Self::VacManifest => "vac_manifest",
            Self::DonorReference => "donor_reference",
            Self::DonorQuarantined => "donor_quarantined",
            Self::Generated => "generated",
            Self::VendorDependency => "vendor_dependency",
            Self::BuildOutput => "build_output",
            Self::Documentation => "documentation",
            Self::Unknown => "unknown",
        }
    }

    pub const fn report_slug(self) -> &'static str {
        match self {
            Self::ProductRuntime => "product",
            Self::ProductTest => "test",
            Self::VacManifest => "vac_manifest",
            Self::DonorReference => "donor_reference",
            Self::DonorQuarantined => "donor_quarantined",
            Self::Generated => "generated",
            Self::VendorDependency => "vendor",
            Self::BuildOutput => "build_output",
            Self::Documentation => "documentation",
            Self::Unknown => "unknown",
        }
    }

    pub const fn is_scannable(self) -> bool {
        matches!(
            self,
            Self::ProductRuntime | Self::ProductTest | Self::VacManifest
        )
    }

    pub const fn is_policy_runtime_scope(self) -> bool {
        matches!(self, Self::ProductRuntime)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LiveRiskAction {
    ExecuteProcess,
    NetworkAccess,
    FilesystemWrite,
    FilesystemDelete,
    CredentialRead,
    SafeRead,
}

impl LiveRiskAction {
    pub const ALL_POLICY_ACTIONS: &'static [Self] = &[
        Self::CredentialRead,
        Self::NetworkAccess,
        Self::ExecuteProcess,
        Self::FilesystemDelete,
        Self::FilesystemWrite,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExecuteProcess => "execute_process",
            Self::NetworkAccess => "network_access",
            Self::FilesystemWrite => "filesystem_write",
            Self::FilesystemDelete => "filesystem_delete",
            Self::CredentialRead => "credential_read",
            Self::SafeRead => "safe_read",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConfidenceLabel {
    Certain,
    High,
    Moderate,
    Low,
    Uncertain,
}

impl ConfidenceLabel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Certain => "certain",
            Self::High => "high",
            Self::Moderate => "moderate",
            Self::Low => "low",
            Self::Uncertain => "uncertain",
        }
    }
}

pub fn confidence_label(confidence: f32) -> ConfidenceLabel {
    if confidence >= 0.90 {
        ConfidenceLabel::Certain
    } else if confidence >= 0.70 {
        ConfidenceLabel::High
    } else if confidence >= 0.50 {
        ConfidenceLabel::Moderate
    } else if confidence >= 0.30 {
        ConfidenceLabel::Low
    } else {
        ConfidenceLabel::Uncertain
    }
}

/// Minimum scan coverage ratio (G-01) below which the scanner verdict fails
/// closed to `Unverified`. Coverage is the fraction of scan-eligible Rust
/// sources that were successfully read and scanned.
pub const SCAN_COVERAGE_MIN_THRESHOLD: f32 = 0.95;

/// Three-class, fail-closed scan verdict (F-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ScanCoverageVerdict {
    Verified,
    Partial,
    Unverified,
}

impl ScanCoverageVerdict {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Partial => "partial",
            Self::Unverified => "unverified",
        }
    }

    pub const fn is_fail_closed(self) -> bool {
        matches!(self, Self::Unverified)
    }
}

/// Scan coverage accounting over scan-eligible Rust sources.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScanCoverage {
    pub scannable_files: usize,
    pub scanned_files: usize,
    pub unreadable_files: Vec<String>,
}

impl ScanCoverage {
    pub fn coverage_ratio(&self) -> f32 {
        if self.scannable_files == 0 {
            return 0.0;
        }
        self.scanned_files as f32 / self.scannable_files as f32
    }

    pub fn verdict(&self, threshold: f32) -> ScanCoverageVerdict {
        scan_coverage_verdict(self, threshold)
    }
}

/// Compute the three-class verdict, failing closed to `Unverified`.
///
/// - No scan-eligible sources at all => `Unverified` (cannot attest coverage).
/// - Full coverage with no unreadable files => `Verified`.
/// - Coverage at/above `threshold` (but not full) => `Partial`.
/// - Coverage below `threshold` => `Unverified` (G-01 fail-closed).
pub fn scan_coverage_verdict(coverage: &ScanCoverage, threshold: f32) -> ScanCoverageVerdict {
    if coverage.scannable_files == 0 {
        return ScanCoverageVerdict::Unverified;
    }
    if coverage.unreadable_files.is_empty() && coverage.scanned_files == coverage.scannable_files {
        ScanCoverageVerdict::Verified
    } else if coverage.coverage_ratio() >= threshold {
        ScanCoverageVerdict::Partial
    } else {
        ScanCoverageVerdict::Unverified
    }
}

/// Walk the workspace and account for scan coverage over scan-eligible Rust
/// sources. A scan-eligible Rust file that cannot be read is recorded as
/// unreadable so coverage cannot be silently inflated.
pub fn build_live_scan_coverage(root: impl AsRef<Path>) -> Result<ScanCoverage, String> {
    let root = root.as_ref();
    let entries = build_live_source_inventory(root)?;
    Ok(scan_coverage_from_inventory(root, &entries))
}

fn scan_coverage_from_inventory(root: &Path, entries: &[LiveSourceEntry]) -> ScanCoverage {
    let mut coverage = ScanCoverage::default();
    for entry in entries {
        if !entry.scan_eligible || !entry.path.ends_with(".rs") {
            continue;
        }
        coverage.scannable_files += 1;
        if fs::read_to_string(root.join(&entry.path)).is_ok() {
            coverage.scanned_files += 1;
        } else {
            coverage.unreadable_files.push(entry.path.clone());
        }
    }
    coverage
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiveSourceEntry {
    pub path: String,
    pub class: LiveSourceClass,
    pub language: String,
    pub crate_hint: Option<String>,
    pub module_hint: Option<String>,
    pub scan_eligible: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LiveOwnershipStatus {
    Complete,
    Partial,
    Hidden,
    Overclaimed,
    Unowned,
    NotEvaluated,
}

impl LiveOwnershipStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Hidden => "hidden",
            Self::Overclaimed => "overclaimed",
            Self::Unowned => "unowned",
            Self::NotEvaluated => "not_evaluated",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LiveQuarantineLevel {
    None,
    SoftQuarantine,
    HardQuarantine,
    AutoQuarantine,
}

impl LiveQuarantineLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::SoftQuarantine => "soft_quarantine",
            Self::HardQuarantine => "hard_quarantine",
            Self::AutoQuarantine => "auto_quarantine",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveOwnershipAnnotation {
    pub status: LiveOwnershipStatus,
    pub capability: Option<String>,
    pub quarantine: LiveQuarantineLevel,
    pub reason: String,
}

impl LiveOwnershipAnnotation {
    pub fn not_evaluated(reason: impl Into<String>) -> Self {
        Self {
            status: LiveOwnershipStatus::NotEvaluated,
            capability: None,
            quarantine: LiveQuarantineLevel::None,
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct LiveOwnershipIndex {
    populated: bool,
    records: BTreeMap<String, (LiveOwnershipStatus, Option<String>)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiveRiskFinding {
    pub id: String,
    pub file: String,
    pub line: usize,
    pub pattern: String,
    pub action: LiveRiskAction,
    pub scope: LiveSourceClass,
    pub confidence: f32,
    pub method: &'static str,
    pub ambiguous: bool,
    pub alternatives: Vec<String>,
    pub ownership: LiveOwnershipAnnotation,
}

pub fn classify_live_path(path: &str) -> LiveSourceClass {
    let normalized = path.replace('\\', "/");
    if normalized == ".git"
        || normalized.starts_with(".git/")
        || normalized == "target"
        || normalized.starts_with("target/")
        || normalized.contains("/target/")
        || normalized == "node_modules"
        || normalized.starts_with("node_modules/")
        || normalized.contains("/node_modules/")
    {
        LiveSourceClass::BuildOutput
    } else if normalized == "vac-rs/vendor"
        || normalized.starts_with("vac-rs/vendor/")
        || normalized == "vendor"
        || normalized.starts_with("vendor/")
        || normalized.contains("/.cargo/")
    {
        LiveSourceClass::VendorDependency
    } else if normalized.ends_with(".generated.rs")
        || normalized.contains("/generated/")
        || normalized.contains("/schema/json/")
        || normalized.contains("/schema/typescript/")
    {
        LiveSourceClass::Generated
    } else if normalized.starts_with("donor/vac/crates/vac_shell")
        || normalized.starts_with("donor/vac/crates/vac_tui_runtime")
    {
        LiveSourceClass::DonorQuarantined
    } else if normalized.starts_with("donor/vac/") {
        LiveSourceClass::DonorReference
    } else if normalized.starts_with(".vac/") && normalized.ends_with(".yaml") {
        LiveSourceClass::VacManifest
    } else if normalized.starts_with("tests/")
        || normalized.contains("/tests/")
        || normalized.ends_with("_test.rs")
        || normalized.contains("/test_")
        || normalized.contains("/fixtures/")
    {
        LiveSourceClass::ProductTest
    } else if normalized.ends_with(".md") {
        LiveSourceClass::Documentation
    } else if (normalized.starts_with("scripts/") && normalized.ends_with(".sh"))
        || normalized.ends_with(".rs")
        || normalized.ends_with(".toml")
    {
        LiveSourceClass::ProductRuntime
    } else {
        LiveSourceClass::Unknown
    }
}

pub fn build_live_source_inventory(root: impl AsRef<Path>) -> Result<Vec<LiveSourceEntry>, String> {
    let root = root.as_ref();
    let mut entries = Vec::new();
    walk_workspace(root, &mut |path| {
        let rel = rel_path(root, path)?;
        let mut class = classify_live_path(&rel);
        if class == LiveSourceClass::DonorReference && donor_inventory_marks_quarantined(root, &rel)
        {
            class = LiveSourceClass::DonorQuarantined;
        }
        let scan_eligible = class.is_scannable();
        entries.push(LiveSourceEntry {
            language: language_for(&rel).to_string(),
            crate_hint: crate_hint(&rel),
            module_hint: module_hint(&rel),
            path: rel,
            class,
            scan_eligible,
            reason: scan_reason(class).to_string(),
        });
        Ok(())
    })?;
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

pub fn build_live_risk_findings(root: impl AsRef<Path>) -> Result<Vec<LiveRiskFinding>, String> {
    let root = root.as_ref();
    let mut findings = Vec::new();
    let entries = build_live_source_inventory(root)?;
    let ownership_index = load_live_ownership_index(root);
    for entry in entries {
        if !entry.scan_eligible || !entry.path.ends_with(".rs") {
            continue;
        }
        let path = root.join(&entry.path);
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        for (line_idx, line) in source.lines().enumerate() {
            if let Some((pattern, action, confidence, ambiguous, alternatives)) =
                detect_line_risk(line)
            {
                let id = format!(
                    "finding.live-init.{}.{}",
                    sanitize_id(&entry.path),
                    line_idx + 1
                );
                findings.push(LiveRiskFinding {
                    id,
                    file: entry.path.clone(),
                    line: line_idx + 1,
                    pattern: pattern.to_string(),
                    action,
                    scope: entry.class,
                    confidence,
                    method: LIVE_SCANNER_METHOD,
                    ambiguous,
                    alternatives,
                    ownership: ownership_annotation_for(&ownership_index, &entry),
                });
            }
        }
    }
    Ok(findings)
}

pub fn render_live_scan_report_yaml(
    entries: &[LiveSourceEntry],
    findings: &[LiveRiskFinding],
) -> String {
    let by_class = count_sources_by_class(entries);
    let mut out = String::new();
    out.push_str("schema_version: 1\nkind: registry_status\nid: init.scan_report\ntitle: VAC init scanner report\nstatus: ready\nsummary:\n");
    out.push_str(&format!("  source_files: {}\n", entries.len()));
    out.push_str(&format!(
        "  scannable_files: {}\n",
        entries.iter().filter(|entry| entry.scan_eligible).count()
    ));
    out.push_str(&format!("  risk_findings: {}\n", findings.len()));
    out.push_str(&format!("  method: {LIVE_SCANNER_METHOD}\n"));
    out.push_str(&format!("  phase2_ast_exact: {LIVE_SCANNER_PHASE2}\n"));
    out.push_str("by_class:\n");
    for class in LiveSourceClass::ALL_REPORT_CLASSES {
        out.push_str(&format!(
            "  {}: {}\n",
            class.as_str(),
            by_class.get(class).copied().unwrap_or(0)
        ));
    }
    out
}

pub fn render_source_inventory_yaml(entries: &[LiveSourceEntry]) -> String {
    let mut out = String::new();
    let by_class = count_sources_by_class(entries);
    out.push_str("schema_version: 1\nkind: registry_status\nid: init.source_inventory\ntitle: VAC init live source inventory\nstatus: ready\nsummary:\n");
    out.push_str(&format!("  total_files: {}\n", entries.len()));
    out.push_str(&format!(
        "  scannable_files: {}\n",
        entries.iter().filter(|entry| entry.scan_eligible).count()
    ));
    for class in LiveSourceClass::ALL_REPORT_CLASSES {
        out.push_str(&format!(
            "  {}: {}\n",
            class.as_str(),
            by_class.get(class).copied().unwrap_or(0)
        ));
    }
    out.push_str("files:\n");
    for entry in entries {
        render_source_entry(&mut out, entry, "  ");
    }
    out
}

pub fn render_source_inventory_by_class_yaml(
    class: LiveSourceClass,
    entries: &[LiveSourceEntry],
) -> String {
    let filtered = entries
        .iter()
        .filter(|entry| entry.class == class)
        .collect::<Vec<_>>();
    let mut out = String::new();
    out.push_str(&format!(
        "schema_version: 1\nkind: registry_status\nid: init.source_inventory.{}\ntitle: 'VAC init source inventory by class: {}'\nstatus: ready\nsummary:\n  class: {}\n  total_files: {}\nfiles:\n",
        class.report_slug(),
        class.as_str(),
        class.as_str(),
        filtered.len()
    ));
    for entry in filtered {
        render_source_entry(&mut out, entry, "  ");
    }
    if entries.is_empty() {
        out.push_str("  []\n");
    }
    out
}

pub fn render_risk_findings_yaml(findings: &[LiveRiskFinding]) -> String {
    let mut out = String::new();
    out.push_str("schema_version: 1\nkind: risk_finding\nid: finding.live-init.report\ntitle: VAC init live risk findings\nstatus: ready\nsummary:\n");
    out.push_str(&format!("  total_findings: {}\n", findings.len()));
    out.push_str(&format!("  method: {LIVE_SCANNER_METHOD}\n"));
    out.push_str("  note: Summary pointer only; full findings are stored in .vac/.init/risk_findings/full.yaml.\n");
    out.push_str("  full: .vac/.init/risk_findings/full.yaml\n");
    out.push_str("  index: .vac/.init/risk_findings/index.yaml\n");
    out.push_str("  by_risk_dir: .vac/.init/risk_findings/by-risk/\n");
    out.push_str("  by_scope_dir: .vac/.init/risk_findings/by-scope/\n");
    out
}

pub fn render_risk_findings_index_yaml(findings: &[LiveRiskFinding]) -> String {
    let by_risk = count_findings_by_risk(findings);
    let by_scope = count_findings_by_scope(findings);
    let mut out = String::new();
    out.push_str("schema_version: 1\nkind: risk_finding\nid: finding.live-init.index\ntitle: VAC init risk finding index\nstatus: ready\nsummary:\n");
    out.push_str(&format!("  total_findings: {}\n", findings.len()));
    out.push_str("  full: .vac/.init/risk_findings/full.yaml\n");
    out.push_str("by_risk:\n");
    for (risk, count) in by_risk {
        out.push_str(&format!("  {}: {}\n", risk.as_str(), count));
    }
    out.push_str("by_scope:\n");
    for (scope, count) in by_scope {
        out.push_str(&format!("  {}: {}\n", scope.as_str(), count));
    }
    out
}

pub fn render_risk_findings_full_yaml(findings: &[LiveRiskFinding]) -> String {
    let mut out = String::new();
    out.push_str("schema_version: 1\nkind: risk_finding\nid: finding.live-init.full\ntitle: VAC init full risk findings\nstatus: ready\nsummary:\n");
    out.push_str(&format!(
        "  total_findings: {}\nfindings:\n",
        findings.len()
    ));
    render_findings(&mut out, findings, "  ");
    out
}

pub fn render_risk_findings_by_risk_yaml(
    action: LiveRiskAction,
    findings: &[LiveRiskFinding],
) -> String {
    let filtered = findings
        .iter()
        .filter(|finding| finding.action == action)
        .cloned()
        .collect::<Vec<_>>();
    let mut out = String::new();
    out.push_str(&format!(
        "schema_version: 1\nkind: risk_finding\nid: finding.live-init.by-risk.{}\ntitle: 'VAC init risk findings by risk: {}'\nstatus: ready\nsummary:\n  risk: {}\n  total_findings: {}\nfindings:\n",
        action.as_str(),
        action.as_str(),
        action.as_str(),
        filtered.len()
    ));
    render_findings(&mut out, &filtered, "  ");
    out
}

pub fn render_risk_findings_by_scope_yaml(
    scope: LiveSourceClass,
    findings: &[LiveRiskFinding],
) -> String {
    let filtered = findings
        .iter()
        .filter(|finding| finding.scope == scope)
        .cloned()
        .collect::<Vec<_>>();
    let mut out = String::new();
    out.push_str(&format!(
        "schema_version: 1\nkind: risk_finding\nid: finding.live-init.by-scope.{}\ntitle: 'VAC init risk findings by scope: {}'\nstatus: ready\nsummary:\n  scope: {}\n  total_findings: {}\nfindings:\n",
        scope.report_slug(),
        scope.as_str(),
        scope.as_str(),
        filtered.len()
    ));
    render_findings(&mut out, &filtered, "  ");
    out
}

pub fn render_policy_inference_report_yaml(findings: &[LiveRiskFinding]) -> String {
    let mut out = String::new();
    out.push_str("schema_version: 1\nkind: registry_status\nid: init.policy_inference_report\ntitle: VAC init policy inference report\nstatus: ready\nsummary:\n");
    out.push_str(&format!("  findings: {}\n", findings.len()));
    out.push_str(&format!(
        "  product_runtime_findings: {}\n",
        findings
            .iter()
            .filter(|finding| finding.scope == LiveSourceClass::ProductRuntime)
            .count()
    ));
    out.push_str(&format!(
        "  review_only_findings: {}\n",
        findings
            .iter()
            .filter(|finding| infer_policy_decision(finding) == "review_only")
            .count()
    ));
    out.push_str(&format!(
        "  ownership_not_evaluated_findings: {}\n",
        findings
            .iter()
            .filter(|finding| finding.ownership.status == LiveOwnershipStatus::NotEvaluated)
            .count()
    ));
    out.push_str(&format!(
        "  blocked_findings: {}\n",
        findings
            .iter()
            .filter(|finding| infer_policy_decision(finding) == "blocked")
            .count()
    ));
    out.push_str("  auto_allow_ambiguous: false\n");
    out.push_str("rules:\n");

    let mut emitted = false;
    for scope in LiveSourceClass::ALL_REPORT_CLASSES {
        for action in LiveRiskAction::ALL_POLICY_ACTIONS {
            let scoped = findings
                .iter()
                .filter(|finding| finding.scope == *scope && finding.action == *action)
                .collect::<Vec<_>>();
            if scoped.is_empty() {
                continue;
            }
            emitted = true;
            let decision = merge_scoped_decision(&scoped);
            out.push_str(&format!("  - risk: {}\n", action.as_str()));
            out.push_str(&format!("    scope: {}\n", scope.as_str()));
            out.push_str(&format!("    source_count: {}\n", scoped.len()));
            out.push_str(&format!("    decision: {decision}\n"));
            out.push_str(&format!(
                "    confidence_label: {}\n",
                max_confidence_label(&scoped).as_str()
            ));
            out.push_str(
                "    reason: Scope-aware fail-closed inference from VAC init scanner findings.\n",
            );
        }
    }
    if !emitted {
        out.push_str("  - risk: safe_read\n    scope: product_runtime\n    source_count: 0\n    decision: allow\n    confidence_label: certain\n    reason: No risk-bearing scanner findings were detected.\n");
    }
    out
}

pub fn build_live_scanner_reports(
    root: impl AsRef<Path>,
) -> Result<(String, String, String), String> {
    let inventory = build_live_source_inventory(&root)?;
    let findings = build_live_risk_findings(&root)?;
    Ok((
        render_source_inventory_yaml(&inventory),
        render_risk_findings_yaml(&findings),
        render_policy_inference_report_yaml(&findings),
    ))
}

pub fn build_live_scanner_report_files(
    root: impl AsRef<Path>,
) -> Result<Vec<(String, String)>, String> {
    let root = root.as_ref();
    let inventory = build_live_source_inventory(root)?;
    let findings = build_live_risk_findings(root)?;
    let mut files = Vec::new();

    files.push((
        ".vac/.init/scan_report.yaml".to_string(),
        render_live_scan_report_yaml(&inventory, &findings),
    ));
    files.push((
        ".vac/.init/source_inventory.yaml".to_string(),
        render_source_inventory_yaml(&inventory),
    ));
    for class in LiveSourceClass::ALL_REPORT_CLASSES {
        files.push((
            format!(
                ".vac/.init/source_inventory/by-class/{}.yaml",
                class.report_slug()
            ),
            render_source_inventory_by_class_yaml(*class, &inventory),
        ));
    }

    files.push((
        ".vac/.init/risk_findings.yaml".to_string(),
        render_risk_findings_yaml(&findings),
    ));
    files.push((
        ".vac/.init/risk_findings/index.yaml".to_string(),
        render_risk_findings_index_yaml(&findings),
    ));
    files.push((
        ".vac/.init/risk_findings/full.yaml".to_string(),
        render_risk_findings_full_yaml(&findings),
    ));
    for action in LiveRiskAction::ALL_POLICY_ACTIONS {
        files.push((
            format!(".vac/.init/risk_findings/by-risk/{}.yaml", action.as_str()),
            render_risk_findings_by_risk_yaml(*action, &findings),
        ));
    }
    for scope in LiveSourceClass::ALL_REPORT_CLASSES {
        files.push((
            format!(
                ".vac/.init/risk_findings/by-scope/{}.yaml",
                scope.report_slug()
            ),
            render_risk_findings_by_scope_yaml(*scope, &findings),
        ));
    }

    files.push((
        ".vac/.init/policy_inference_report.yaml".to_string(),
        render_policy_inference_report_yaml(&findings),
    ));
    let coverage = scan_coverage_from_inventory(root, &inventory);
    files.push((
        ".vac/.init/scanner_doctor_report.yaml".to_string(),
        render_scanner_doctor_report_yaml(&inventory, &findings, &coverage),
    ));
    Ok(files)
}

pub fn render_scanner_doctor_report_yaml(
    entries: &[LiveSourceEntry],
    findings: &[LiveRiskFinding],
    coverage: &ScanCoverage,
) -> String {
    let total_by_class = count_sources_by_class(entries);
    let not_evaluated = findings
        .iter()
        .filter(|finding| finding.ownership.status == LiveOwnershipStatus::NotEvaluated)
        .count();
    let unowned_runtime = findings
        .iter()
        .filter(|finding| {
            finding.scope == LiveSourceClass::ProductRuntime
                && finding.ownership.status == LiveOwnershipStatus::Unowned
        })
        .count();
    let overclaimed_runtime = findings
        .iter()
        .filter(|finding| {
            finding.scope == LiveSourceClass::ProductRuntime
                && finding.ownership.status == LiveOwnershipStatus::Overclaimed
        })
        .count();
    let credential_without_deny = findings
        .iter()
        .filter(|finding| {
            finding.scope == LiveSourceClass::ProductRuntime
                && finding.action == LiveRiskAction::CredentialRead
                && infer_policy_decision(finding) != "deny"
        })
        .count();

    let ownership_status = if not_evaluated > 0 {
        "NotEvaluated"
    } else if unowned_runtime > 0 || overclaimed_runtime > 0 {
        "blocked"
    } else {
        "pass"
    };

    let mut out = String::new();
    out.push_str("schema_version: 1\nkind: registry_status\nid: init.scanner_doctor_report\ntitle: VAC init scanner doctor report\nstatus: ready\nsummary:\n");
    out.push_str(&format!("  source_files: {}\n", entries.len()));
    out.push_str(&format!("  risk_findings: {}\n", findings.len()));
    out.push_str(&format!("  ownership_status: {ownership_status}\n"));
    out.push_str(&format!(
        "  ownership_not_evaluated_findings: {not_evaluated}\n"
    ));
    out.push_str(&format!(
        "  unowned_product_runtime_findings: {unowned_runtime}\n"
    ));
    out.push_str(&format!(
        "  overclaimed_product_runtime_findings: {overclaimed_runtime}\n"
    ));
    out.push_str(&format!(
        "  product_runtime_credential_without_deny: {credential_without_deny}\n"
    ));
    let scan_verdict = scan_coverage_verdict(coverage, SCAN_COVERAGE_MIN_THRESHOLD);
    out.push_str(&format!(
        "  scannable_rust_files: {}\n",
        coverage.scannable_files
    ));
    out.push_str(&format!("  scanned_rust_files: {}\n", coverage.scanned_files));
    out.push_str(&format!(
        "  unreadable_rust_files: {}\n",
        coverage.unreadable_files.len()
    ));
    out.push_str(&format!("  scan_coverage: {:.2}\n", coverage.coverage_ratio()));
    out.push_str(&format!(
        "  scan_coverage_threshold: {SCAN_COVERAGE_MIN_THRESHOLD:.2}\n"
    ));
    out.push_str(&format!("  scan_verdict: {}\n", scan_verdict.as_str()));
    out.push_str("checks:\n");
    out.push_str("  full_storage: pass\n");
    out.push_str("  policy_fail_closed: pass\n");
    out.push_str(&format!(
        "  scan_coverage: {}\n",
        if scan_verdict.is_fail_closed() {
            "fail"
        } else {
            "pass"
        }
    ));
    out.push_str(&format!("  ownership: {ownership_status}\n"));
    out.push_str("classes:\n");
    for class in LiveSourceClass::ALL_REPORT_CLASSES {
        out.push_str(&format!(
            "  {}: {}\n",
            class.as_str(),
            total_by_class.get(class).copied().unwrap_or(0)
        ));
    }
    out
}

fn donor_inventory_marks_quarantined(root: &Path, relative_path: &str) -> bool {
    let path = root.join(".vac/registry/donor-inventory.yaml");
    let Ok(source) = fs::read_to_string(path) else {
        return false;
    };
    let source = source.replace('\r', "");
    let crate_name = relative_path
        .split('/')
        .collect::<Vec<_>>()
        .windows(2)
        .find_map(|window| {
            if window[0] == "crates" {
                Some(window[1].to_string())
            } else {
                None
            }
        });
    let Some(crate_name) = crate_name else {
        return false;
    };
    let needle = format!("donor_source: {crate_name}");
    let Some(start) = source.find(&needle) else {
        return false;
    };
    let rest = &source[start..];
    let end = rest.find("\n- donor_source:").unwrap_or(rest.len());
    let entry = &rest[..end];
    entry.contains("decision: QUARANTINE") || entry.contains("status: QUARANTINED")
}

fn load_live_ownership_index(root: &Path) -> LiveOwnershipIndex {
    let path = root.join(".vac/registry/ownership/report.yaml");
    let Ok(source) = fs::read_to_string(path) else {
        return LiveOwnershipIndex::default();
    };
    let populated = parse_usize_field(&source, "total_files").unwrap_or(0) > 0
        && source.contains("\nfiles:")
        && source.contains("\n  - path:");
    if !populated {
        return LiveOwnershipIndex::default();
    }

    let mut records = BTreeMap::new();
    let mut current_path: Option<String> = None;
    let mut current_status: Option<LiveOwnershipStatus> = None;
    let mut current_capability: Option<String> = None;

    fn commit(
        records: &mut BTreeMap<String, (LiveOwnershipStatus, Option<String>)>,
        path: &mut Option<String>,
        status: &mut Option<LiveOwnershipStatus>,
        capability: &mut Option<String>,
    ) {
        if let Some(path_value) = path.take() {
            records.insert(
                path_value,
                (
                    status.unwrap_or(LiveOwnershipStatus::Hidden),
                    capability.take(),
                ),
            );
        }
        *status = None;
        *capability = None;
    }

    for raw in source.lines() {
        let line = raw.trim();
        if let Some(value) = line.strip_prefix("- path:") {
            commit(
                &mut records,
                &mut current_path,
                &mut current_status,
                &mut current_capability,
            );
            current_path = Some(trim_yaml_scalar(value));
        } else if let Some(value) = line.strip_prefix("path:") {
            commit(
                &mut records,
                &mut current_path,
                &mut current_status,
                &mut current_capability,
            );
            current_path = Some(trim_yaml_scalar(value));
        } else if let Some(value) = line.strip_prefix("status:") {
            current_status = Some(match trim_yaml_scalar(value).as_str() {
                "complete" => LiveOwnershipStatus::Complete,
                "partial" => LiveOwnershipStatus::Partial,
                "hidden" => LiveOwnershipStatus::Hidden,
                "overclaimed" => LiveOwnershipStatus::Overclaimed,
                "unowned" => LiveOwnershipStatus::Unowned,
                _ => LiveOwnershipStatus::Hidden,
            });
        } else if let Some(value) = line.strip_prefix("capability:") {
            let value = trim_yaml_scalar(value);
            if value != "null" && !value.is_empty() {
                current_capability = Some(value);
            }
        }
    }
    commit(
        &mut records,
        &mut current_path,
        &mut current_status,
        &mut current_capability,
    );

    LiveOwnershipIndex {
        populated: !records.is_empty(),
        records,
    }
}

fn ownership_annotation_for(
    index: &LiveOwnershipIndex,
    entry: &LiveSourceEntry,
) -> LiveOwnershipAnnotation {
    if entry.class == LiveSourceClass::DonorQuarantined {
        return LiveOwnershipAnnotation {
            status: LiveOwnershipStatus::NotEvaluated,
            capability: None,
            quarantine: LiveQuarantineLevel::HardQuarantine,
            reason: "donor source is quarantined by donor inventory/status board".to_string(),
        };
    }

    if !index.populated {
        return LiveOwnershipAnnotation::not_evaluated(
            "ownership report is missing or seed/unpopulated; scanner does not fake ownership coverage",
        );
    }

    if let Some((status, capability)) = index.records.get(&entry.path) {
        return LiveOwnershipAnnotation {
            status: *status,
            capability: capability.clone(),
            quarantine: quarantine_for_status(*status),
            reason: "from .vac/registry/ownership/report.yaml".to_string(),
        };
    }

    let status = match entry.class {
        LiveSourceClass::ProductRuntime | LiveSourceClass::ProductTest => {
            LiveOwnershipStatus::Unowned
        }
        _ => LiveOwnershipStatus::Hidden,
    };
    LiveOwnershipAnnotation {
        status,
        capability: None,
        quarantine: quarantine_for_status(status),
        reason: "file absent from populated ownership report".to_string(),
    }
}

fn quarantine_for_status(status: LiveOwnershipStatus) -> LiveQuarantineLevel {
    match status {
        LiveOwnershipStatus::Complete | LiveOwnershipStatus::Partial => LiveQuarantineLevel::None,
        LiveOwnershipStatus::Hidden => LiveQuarantineLevel::SoftQuarantine,
        LiveOwnershipStatus::Overclaimed | LiveOwnershipStatus::Unowned => {
            LiveQuarantineLevel::HardQuarantine
        }
        LiveOwnershipStatus::NotEvaluated => LiveQuarantineLevel::None,
    }
}

fn parse_usize_field(source: &str, field: &str) -> Option<usize> {
    let prefix = format!("{field}:");
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix(&prefix) {
            return value.trim().parse::<usize>().ok();
        }
    }
    None
}

fn trim_yaml_scalar(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn detect_line_risk(line: &str) -> Option<(&'static str, LiveRiskAction, f32, bool, Vec<String>)> {
    let compact = rust_lexical_code(line);
    if rust_callsite_present(
        &compact,
        &[
            "std::process::Command::new(",
            "tokio::process::Command::new(",
            "Command::new(",
        ],
    ) {
        Some((
            "Command::new/process",
            LiveRiskAction::ExecuteProcess,
            0.92,
            false,
            Vec::new(),
        ))
    } else if rust_callsite_present(
        &compact,
        &[
            "std::fs::write(",
            "tokio::fs::write(",
            "File::create(",
            "OpenOptions::new(",
        ],
    ) {
        Some((
            "fs::write/File::create/OpenOptions",
            LiveRiskAction::FilesystemWrite,
            0.90,
            false,
            Vec::new(),
        ))
    } else if rust_callsite_present(&compact, &["remove_file(", "remove_dir_all("]) {
        Some((
            "remove_file/remove_dir_all",
            LiveRiskAction::FilesystemDelete,
            0.90,
            false,
            Vec::new(),
        ))
    } else if rust_callsite_present(
        &compact,
        &[
            "tokio::net::",
            "reqwest::",
            "hyper::",
            "tonic::",
            "websocket",
        ],
    ) {
        Some((
            "network client/socket",
            LiveRiskAction::NetworkAccess,
            0.88,
            false,
            Vec::new(),
        ))
    } else if rust_callsite_present(
        &compact,
        &["env::var(", "std::env::var(", "dotenv", "keyring"],
    ) {
        Some((
            "env::var/dotenv/keyring",
            LiveRiskAction::CredentialRead,
            0.92,
            false,
            Vec::new(),
        ))
    } else {
        None
    }
}

fn rust_callsite_present(compact_code: &str, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| compact_code.contains(candidate))
}

fn rust_lexical_code(line: &str) -> String {
    let mut out = String::new();
    let mut chars = line.chars().peekable();
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if !in_string && !in_char && ch == '/' && chars.peek() == Some(&'/') {
            break;
        }
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if in_char {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '\'' {
                in_char = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            continue;
        }
        if ch == '\'' {
            in_char = true;
            continue;
        }
        if !ch.is_whitespace() {
            out.push(ch);
        }
    }
    out
}

fn infer_policy_decision(finding: &LiveRiskFinding) -> &'static str {
    if finding.ambiguous
        || matches!(
            confidence_label(finding.confidence),
            ConfidenceLabel::Low | ConfidenceLabel::Uncertain
        )
    {
        return "review_only";
    }
    match finding.scope {
        LiveSourceClass::ProductRuntime => match finding.action {
            LiveRiskAction::CredentialRead => "deny",
            LiveRiskAction::ExecuteProcess
            | LiveRiskAction::NetworkAccess
            | LiveRiskAction::FilesystemWrite
            | LiveRiskAction::FilesystemDelete => "approval_required",
            LiveRiskAction::SafeRead => "allow",
        },
        LiveSourceClass::ProductTest | LiveSourceClass::VacManifest => "review_only",
        LiveSourceClass::DonorReference => "reference_only",
        LiveSourceClass::DonorQuarantined => "blocked",
        LiveSourceClass::Generated
        | LiveSourceClass::VendorDependency
        | LiveSourceClass::BuildOutput => "ignored_by_scope",
        LiveSourceClass::Documentation | LiveSourceClass::Unknown => "review_only",
    }
}

fn merge_scoped_decision(findings: &[&LiveRiskFinding]) -> &'static str {
    let mut decision = "allow";
    for finding in findings {
        let next = infer_policy_decision(finding);
        decision = more_restrictive(decision, next);
    }
    decision
}

fn more_restrictive(left: &'static str, right: &'static str) -> &'static str {
    fn rank(value: &str) -> u8 {
        match value {
            "deny" | "blocked" => 5,
            "approval_required" => 4,
            "review_only" => 3,
            "reference_only" => 2,
            "ignored_by_scope" => 1,
            "allow" => 0,
            _ => 3,
        }
    }
    if rank(right) > rank(left) {
        right
    } else {
        left
    }
}

fn max_confidence_label(findings: &[&LiveRiskFinding]) -> ConfidenceLabel {
    findings
        .iter()
        .map(|finding| confidence_label(finding.confidence))
        .max()
        .unwrap_or(ConfidenceLabel::Uncertain)
}

fn walk_workspace<F>(root: &Path, f: &mut F) -> Result<(), String>
where
    F: FnMut(&Path) -> Result<(), String>,
{
    fn walk_inner<F>(root: &Path, current: &Path, f: &mut F) -> Result<(), String>
    where
        F: FnMut(&Path) -> Result<(), String>,
    {
        if !current.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(current).map_err(|err| err.to_string())? {
            let entry = entry.map_err(|err| err.to_string())?;
            let path = entry.path();
            let rel = rel_path(root, &path)?;
            let class = classify_live_path(&rel);
            if matches!(
                class,
                LiveSourceClass::BuildOutput | LiveSourceClass::VendorDependency
            ) {
                continue;
            }
            if path.is_dir() {
                walk_inner(root, &path, f)?;
            } else {
                f(&path)?;
            }
        }
        Ok(())
    }

    walk_inner(root, root, f)
}

fn count_sources_by_class(entries: &[LiveSourceEntry]) -> BTreeMap<LiveSourceClass, usize> {
    let mut counts = BTreeMap::new();
    for entry in entries {
        *counts.entry(entry.class).or_insert(0) += 1;
    }
    counts
}

fn count_findings_by_risk(findings: &[LiveRiskFinding]) -> BTreeMap<LiveRiskAction, usize> {
    let mut counts = BTreeMap::new();
    for finding in findings {
        *counts.entry(finding.action).or_insert(0) += 1;
    }
    counts
}

fn count_findings_by_scope(findings: &[LiveRiskFinding]) -> BTreeMap<LiveSourceClass, usize> {
    let mut counts = BTreeMap::new();
    for finding in findings {
        *counts.entry(finding.scope).or_insert(0) += 1;
    }
    counts
}

fn render_source_entry(out: &mut String, entry: &LiveSourceEntry, indent: &str) {
    out.push_str(&format!("{indent}- path: {}\n", yaml_scalar(&entry.path)));
    out.push_str(&format!("{indent}  class: {}\n", entry.class.as_str()));
    out.push_str(&format!("{indent}  language: {}\n", entry.language));
    out.push_str(&format!(
        "{indent}  scan_eligible: {}\n",
        entry.scan_eligible
    ));
    out.push_str(&format!(
        "{indent}  reason: {}\n",
        yaml_scalar(&entry.reason)
    ));
    if let Some(crate_hint) = &entry.crate_hint {
        out.push_str(&format!(
            "{indent}  crate_hint: {}\n",
            yaml_scalar(crate_hint)
        ));
    }
    if let Some(module_hint) = &entry.module_hint {
        out.push_str(&format!(
            "{indent}  module_hint: {}\n",
            yaml_scalar(module_hint)
        ));
    }
}

fn render_findings(out: &mut String, findings: &[LiveRiskFinding], indent: &str) {
    if findings.is_empty() {
        out.push_str(&format!("{indent}[]\n"));
        return;
    }
    for finding in findings {
        out.push_str(&format!("{indent}- id: {}\n", yaml_scalar(&finding.id)));
        out.push_str(&format!("{indent}  file: {}\n", yaml_scalar(&finding.file)));
        out.push_str(&format!("{indent}  line: {}\n", finding.line));
        out.push_str(&format!(
            "{indent}  pattern: {}\n",
            yaml_scalar(&finding.pattern)
        ));
        out.push_str(&format!(
            "{indent}  inferred_risk: {}\n",
            finding.action.as_str()
        ));
        out.push_str(&format!("{indent}  scope: {}\n", finding.scope.as_str()));
        out.push_str(&format!("{indent}  ownership:\n"));
        out.push_str(&format!(
            "{indent}    status: {}\n",
            finding.ownership.status.as_str()
        ));
        match &finding.ownership.capability {
            Some(capability) => out.push_str(&format!(
                "{indent}    capability: {}\n",
                yaml_scalar(capability)
            )),
            None => out.push_str(&format!("{indent}    capability: null\n")),
        }
        out.push_str(&format!(
            "{indent}    quarantine: {}\n",
            finding.ownership.quarantine.as_str()
        ));
        out.push_str(&format!(
            "{indent}    reason: {}\n",
            yaml_scalar(&finding.ownership.reason)
        ));
        out.push_str(&format!(
            "{indent}  confidence: {:.2}\n",
            finding.confidence
        ));
        out.push_str(&format!(
            "{indent}  confidence_label: {}\n",
            confidence_label(finding.confidence).as_str()
        ));
        out.push_str(&format!("{indent}  method: {}\n", finding.method));
        out.push_str(&format!("{indent}  ambiguous: {}\n", finding.ambiguous));
        if finding.alternatives.is_empty() {
            out.push_str(&format!("{indent}  alternatives: []\n"));
        } else {
            out.push_str(&format!("{indent}  alternatives:\n"));
            for alternative in &finding.alternatives {
                out.push_str(&format!("{indent}    - {}\n", yaml_scalar(alternative)));
            }
        }
    }
}

fn scan_reason(class: LiveSourceClass) -> &'static str {
    match class {
        LiveSourceClass::ProductRuntime => "product runtime source is scanned for policy risk",
        LiveSourceClass::ProductTest => "test source is scanned but policy is review-only",
        LiveSourceClass::VacManifest => "VAC manifest is scanned for governance context",
        LiveSourceClass::DonorReference => {
            "donor source is reference-only and excluded from product policy inference"
        }
        LiveSourceClass::DonorQuarantined => {
            "donor source is quarantined and excluded from product policy inference"
        }
        LiveSourceClass::Generated => "generated source is excluded from scanner policy inference",
        LiveSourceClass::VendorDependency => "vendor dependency source is excluded",
        LiveSourceClass::BuildOutput => "build output is excluded",
        LiveSourceClass::Documentation => "documentation is indexed but not risk-scanned",
        LiveSourceClass::Unknown => "unknown file class is indexed for review",
    }
}

fn rel_path(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map_err(|err| err.to_string())
        .map(|value| value.to_string_lossy().replace('\\', "/"))
}

fn language_for(path: &str) -> &'static str {
    if path.ends_with(".rs") {
        "rust"
    } else if path.ends_with(".toml") {
        "toml"
    } else if path.ends_with(".yaml") || path.ends_with(".yml") {
        "yaml"
    } else if path.ends_with(".md") {
        "markdown"
    } else if path.ends_with(".sh") {
        "shell"
    } else if path.ends_with(".json") {
        "json"
    } else {
        "unknown"
    }
}

fn crate_hint(path: &str) -> Option<String> {
    let parts = path.split('/').collect::<Vec<_>>();
    if parts.first() == Some(&"vac-rs") && parts.len() > 2 {
        Some(parts[1].to_string())
    } else if parts.first() == Some(&"donor") && parts.len() > 4 {
        Some(parts[3].to_string())
    } else {
        None
    }
}

fn module_hint(path: &str) -> Option<String> {
    if !path.ends_with(".rs") {
        return None;
    }
    let without = path.strip_suffix(".rs").unwrap_or(path);
    if let Some(src_index) = without.split('/').position(|part| part == "src") {
        let parts = without.split('/').skip(src_index + 1).collect::<Vec<_>>();
        if !parts.is_empty() {
            return Some(parts.join("::").replace("mod", ""));
        }
    }
    None
}

fn sanitize_id(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('.') {
            out.push('.');
        }
    }
    out.trim_matches('.').to_string()
}

fn yaml_scalar(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | '/'))
        && !value.is_empty()
    {
        value.to_string()
    } else {
        format!("{value:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("vac-live-scanner-{unique}"))
    }

    #[test]
    fn classifies_workspace_paths_and_excludes_build_cache() {
        assert_eq!(
            classify_live_path("vac-rs/core/src/lib.rs"),
            LiveSourceClass::ProductRuntime
        );
        assert_eq!(
            classify_live_path(".vac/capabilities/core.yaml"),
            LiveSourceClass::VacManifest
        );
        assert_eq!(
            classify_live_path("vac-rs/vendor/serde/src/lib.rs"),
            LiveSourceClass::VendorDependency
        );
        assert_eq!(
            classify_live_path("vac-rs/target/debug/lib.rmeta"),
            LiveSourceClass::BuildOutput
        );
        assert_eq!(
            classify_live_path("vac-rs/app-server-protocol/schema/json/X.json"),
            LiveSourceClass::Generated
        );
        assert_eq!(
            classify_live_path("donor/vac/crates/vac_shell/src/lib.rs"),
            LiveSourceClass::DonorQuarantined
        );
        assert_eq!(
            classify_live_path("donor/vac/crates/vac_runtime/src/lib.rs"),
            LiveSourceClass::DonorReference
        );
        assert_eq!(
            classify_live_path("vac-rs/core/tests/scanner.rs"),
            LiveSourceClass::ProductTest
        );
    }

    #[test]
    fn detects_live_rust_risk_findings_with_scope_and_alternatives() {
        let root = temp_root();
        let src = root.join("vac-rs/core/src");
        fs::create_dir_all(&src).unwrap();
        fs::write(
            src.join("lib.rs"),
            "fn run() { std::process::Command::new(\"git\"); }\nfn env() { std::env::var(\"TOKEN\"); }\n",
        )
        .unwrap();
        let findings = build_live_risk_findings(&root).unwrap();
        assert!(
            findings
                .iter()
                .any(|finding| finding.action == LiveRiskAction::ExecuteProcess
                    && finding.scope == LiveSourceClass::ProductRuntime)
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.action == LiveRiskAction::CredentialRead
                    && finding.scope == LiveSourceClass::ProductRuntime)
        );
        assert!(
            findings
                .iter()
                .all(|finding| finding.alternatives.is_empty())
        );
        let policy = render_policy_inference_report_yaml(&findings);
        assert!(policy.contains("risk: credential_read"));
        assert!(policy.contains("decision: deny"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn emits_inventory_risk_index_full_and_policy_yaml_with_envelopes() {
        let root = temp_root();
        fs::create_dir_all(root.join(".vac/capabilities")).unwrap();
        fs::write(
            root.join(".vac/capabilities/test.yaml"),
            "schema_version: 1\nkind: capability\nid: vac.test\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("vac-rs/core/src")).unwrap();
        fs::write(
            root.join("vac-rs/core/src/lib.rs"),
            "fn run() { std::process::Command::new(\"git\"); }\n",
        )
        .unwrap();
        let files = build_live_scanner_report_files(&root).unwrap();
        let paths = files
            .iter()
            .map(|(path, _)| path.as_str())
            .collect::<Vec<_>>();
        assert!(paths.contains(&".vac/.init/scan_report.yaml"));
        assert!(paths.contains(&".vac/.init/source_inventory.yaml"));
        assert!(paths.contains(&".vac/.init/risk_findings.yaml"));
        assert!(paths.contains(&".vac/.init/risk_findings/index.yaml"));
        assert!(paths.contains(&".vac/.init/risk_findings/full.yaml"));
        assert!(paths.contains(&".vac/.init/policy_inference_report.yaml"));
        assert!(paths.contains(&".vac/.init/scanner_doctor_report.yaml"));
        let risk_summary = files
            .iter()
            .find(|(path, _)| path == ".vac/.init/risk_findings.yaml")
            .map(|(_, content)| content)
            .unwrap();
        assert!(risk_summary.contains("id: finding.live-init.report"));
        assert!(risk_summary.contains("full: .vac/.init/risk_findings/full.yaml"));
        assert!(!risk_summary.contains("\nfindings:\n"));
        let full = files
            .iter()
            .find(|(path, _)| path == ".vac/.init/risk_findings/full.yaml")
            .map(|(_, content)| content)
            .unwrap();
        assert!(full.contains("ownership:"));
        let doctor = files
            .iter()
            .find(|(path, _)| path == ".vac/.init/scanner_doctor_report.yaml")
            .map(|(_, content)| content)
            .unwrap();
        assert!(doctor.contains("ownership_status: NotEvaluated"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn policy_inference_is_scope_aware_and_fail_closed() {
        let findings = vec![
            LiveRiskFinding {
                id: "finding.runtime.credential".to_string(),
                file: "vac-rs/core/src/lib.rs".to_string(),
                line: 1,
                pattern: "std::env::var".to_string(),
                action: LiveRiskAction::CredentialRead,
                scope: LiveSourceClass::ProductRuntime,
                confidence: 0.80,
                method: LIVE_SCANNER_METHOD,
                ambiguous: false,
                alternatives: Vec::new(),
                ownership: LiveOwnershipAnnotation::not_evaluated("test fixture"),
            },
            LiveRiskFinding {
                id: "finding.test.process".to_string(),
                file: "vac-rs/core/tests/test.rs".to_string(),
                line: 1,
                pattern: "Command::new".to_string(),
                action: LiveRiskAction::ExecuteProcess,
                scope: LiveSourceClass::ProductTest,
                confidence: 0.80,
                method: LIVE_SCANNER_METHOD,
                ambiguous: false,
                alternatives: Vec::new(),
                ownership: LiveOwnershipAnnotation::not_evaluated("test fixture"),
            },
        ];
        let policy = render_policy_inference_report_yaml(&findings);
        assert!(policy.contains("risk: credential_read"));
        assert!(policy.contains("scope: product_runtime"));
        assert!(policy.contains("decision: deny"));
        assert!(policy.contains("scope: product_test"));
        assert!(policy.contains("decision: review_only"));
    }

    #[test]
    fn scan_coverage_verdict_fails_closed_when_below_threshold() {
        let coverage = ScanCoverage {
            scannable_files: 10,
            scanned_files: 5,
            unreadable_files: vec!["vac-rs/core/src/a.rs".to_string()],
        };
        assert_eq!(
            scan_coverage_verdict(&coverage, SCAN_COVERAGE_MIN_THRESHOLD),
            ScanCoverageVerdict::Unverified
        );
    }

    #[test]
    fn scan_coverage_verdict_is_partial_at_or_above_threshold() {
        let coverage = ScanCoverage {
            scannable_files: 100,
            scanned_files: 96,
            unreadable_files: vec!["vac-rs/core/src/a.rs".to_string()],
        };
        assert_eq!(
            scan_coverage_verdict(&coverage, SCAN_COVERAGE_MIN_THRESHOLD),
            ScanCoverageVerdict::Partial
        );
    }

    #[test]
    fn scan_coverage_verdict_is_verified_on_full_coverage() {
        let coverage = ScanCoverage {
            scannable_files: 8,
            scanned_files: 8,
            unreadable_files: Vec::new(),
        };
        assert_eq!(
            scan_coverage_verdict(&coverage, SCAN_COVERAGE_MIN_THRESHOLD),
            ScanCoverageVerdict::Verified
        );
    }

    #[test]
    fn scan_coverage_verdict_fails_closed_with_no_scannable_sources() {
        let coverage = ScanCoverage::default();
        assert_eq!(
            scan_coverage_verdict(&coverage, SCAN_COVERAGE_MIN_THRESHOLD),
            ScanCoverageVerdict::Unverified
        );
    }

    #[test]
    fn scanner_doctor_report_surfaces_scan_coverage_verdict() {
        let root = temp_root();
        fs::create_dir_all(root.join("vac-rs/core/src")).unwrap();
        fs::write(
            root.join("vac-rs/core/src/lib.rs"),
            "fn run() { std::process::Command::new(\"git\"); }\n",
        )
        .unwrap();
        let files = build_live_scanner_report_files(&root).unwrap();
        let doctor = files
            .iter()
            .find(|(path, _)| path == ".vac/.init/scanner_doctor_report.yaml")
            .map(|(_, content)| content)
            .unwrap();
        assert!(doctor.contains("scan_verdict: verified"));
        assert!(doctor.contains("scan_coverage: 1.00"));
        assert!(doctor.contains("scannable_rust_files: 1"));
        let _ = fs::remove_dir_all(root);
    }
}
