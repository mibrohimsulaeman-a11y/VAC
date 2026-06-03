#![allow(dead_code)]
//! Dependency-free VAC-Init registry validator foundation.
//!
//! This module performs first-pass `.vac/` manifest checks that do not require
//! serde: envelope presence, kind allowlist, id uniqueness, pillar-specific
//! required fields, and fail-closed diagnostics. It is intentionally small and
//! deterministic so `vac doctor registry .` can later delegate to the same
//! diagnostic vocabulary.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub const VALIDATOR_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VacInitRegistryKind {
    Capability,
    Policy,
    Workflow,
    WorkflowStep,
    Surface,
    RegistryStatus,
    Domains,
    InitState,
    Evidence,
    Plan,
    ApprovalRequest,
    OwnershipReport,
    MemoryRecord,
    RiskFinding,
    Migration,
    Trajectory,
    TestAssertion,
    Product,
    Status,
    DonorInventory,
}

impl VacInitRegistryKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Capability => "capability",
            Self::Policy => "policy",
            Self::Workflow => "workflow",
            Self::WorkflowStep => "workflow_step",
            Self::Surface => "surface",
            Self::RegistryStatus => "registry_status",
            Self::Domains => "domains",
            Self::InitState => "init_state",
            Self::Evidence => "evidence",
            Self::Plan => "plan",
            Self::ApprovalRequest => "approval_request",
            Self::OwnershipReport => "ownership_report",
            Self::MemoryRecord => "memory_record",
            Self::RiskFinding => "risk_finding",
            Self::Migration => "migration",
            Self::Trajectory => "trajectory",
            Self::TestAssertion => "test_assertion",
            Self::Product => "product",
            Self::Status => "status",
            Self::DonorInventory => "donor_inventory",
        }
    }

    pub const fn is_spec_kind(self) -> bool {
        !matches!(self, Self::Product | Self::Status | Self::DonorInventory)
    }

    pub const fn is_current_workspace_compatibility(self) -> bool {
        !self.is_spec_kind()
    }
}

impl fmt::Display for VacInitRegistryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for VacInitRegistryKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_kind(value).ok_or_else(|| format!("unknown VAC manifest kind `{value}`"))
    }
}

pub fn parse_kind(value: &str) -> Option<VacInitRegistryKind> {
    match value {
        "capability" => Some(VacInitRegistryKind::Capability),
        "policy" => Some(VacInitRegistryKind::Policy),
        "workflow" => Some(VacInitRegistryKind::Workflow),
        "workflow_step" => Some(VacInitRegistryKind::WorkflowStep),
        "surface" => Some(VacInitRegistryKind::Surface),
        "registry_status" => Some(VacInitRegistryKind::RegistryStatus),
        "domains" => Some(VacInitRegistryKind::Domains),
        "init_state" => Some(VacInitRegistryKind::InitState),
        "evidence" => Some(VacInitRegistryKind::Evidence),
        "plan" => Some(VacInitRegistryKind::Plan),
        "approval_request" => Some(VacInitRegistryKind::ApprovalRequest),
        "ownership_report" => Some(VacInitRegistryKind::OwnershipReport),
        "memory_record" => Some(VacInitRegistryKind::MemoryRecord),
        "risk_finding" => Some(VacInitRegistryKind::RiskFinding),
        "migration" => Some(VacInitRegistryKind::Migration),
        "trajectory" => Some(VacInitRegistryKind::Trajectory),
        "test_assertion" => Some(VacInitRegistryKind::TestAssertion),
        "product" => Some(VacInitRegistryKind::Product),
        "status" => Some(VacInitRegistryKind::Status),
        "donor_inventory" => Some(VacInitRegistryKind::DonorInventory),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryManifestRecord {
    pub path: PathBuf,
    pub schema_version: u32,
    pub kind: VacInitRegistryKind,
    pub id: String,
    pub title: Option<String>,
    pub status: Option<String>,
    pub owner_present: bool,
    pub policy_present: bool,
    pub surfaces_present: bool,
    pub ownership_present: bool,
    pub validation_present: bool,
    pub docs_present: bool,
}

impl RegistryManifestRecord {
    pub fn from_yaml_str(
        path: impl Into<PathBuf>,
        source: &str,
    ) -> Result<Self, RegistryParseError> {
        let path = path.into();
        let scalars = collect_top_level_scalars(source);
        let blocks = collect_top_level_block_keys(source);
        let schema_version = required_scalar(&scalars, "schema_version")?
            .parse::<u32>()
            .map_err(|_| RegistryParseError::InvalidSchemaVersion {
                value: required_scalar(&scalars, "schema_version").unwrap_or_default(),
            })?;
        let kind_raw = required_scalar(&scalars, "kind")?;
        let kind = parse_kind(&kind_raw).ok_or(RegistryParseError::UnknownKind(kind_raw))?;
        let id = required_scalar(&scalars, "id")?;
        validate_manifest_id(&id).map_err(RegistryParseError::InvalidId)?;
        Ok(Self {
            path,
            schema_version,
            kind,
            id,
            title: scalars.get("title").cloned(),
            status: scalars.get("status").cloned(),
            owner_present: blocks.contains("owner") || scalars.contains_key("owner"),
            policy_present: blocks.contains("policy") || scalars.contains_key("policy"),
            surfaces_present: blocks.contains("surfaces") || scalars.contains_key("surfaces"),
            ownership_present: blocks.contains("ownership") || scalars.contains_key("ownership"),
            validation_present: blocks.contains("validation") || scalars.contains_key("validation"),
            docs_present: blocks.contains("docs") || scalars.contains_key("docs"),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryParseError {
    MissingField(&'static str),
    InvalidSchemaVersion { value: String },
    UnknownKind(String),
    InvalidId(String),
    Io(String),
}

impl fmt::Display for RegistryParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "missing required field `{field}`"),
            Self::InvalidSchemaVersion { value } => write!(f, "invalid schema_version `{value}`"),
            Self::UnknownKind(kind) => write!(f, "unknown manifest kind `{kind}`"),
            Self::InvalidId(id) => write!(f, "invalid dotted manifest id `{id}`"),
            Self::Io(error) => f.write_str(error),
        }
    }
}

impl std::error::Error for RegistryParseError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RegistryDiagnosticSeverity {
    Info,
    Warning,
    Error,
    Blocked,
}

impl RegistryDiagnosticSeverity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Blocked => "blocked",
        }
    }

    pub const fn is_failure(self) -> bool {
        matches!(self, Self::Error | Self::Blocked)
    }
}

impl fmt::Display for RegistryDiagnosticSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryValidatorDiagnostic {
    pub severity: RegistryDiagnosticSeverity,
    pub code: String,
    pub path: PathBuf,
    pub field_path: Option<String>,
    pub message: String,
    pub hint: Option<String>,
}

impl RegistryValidatorDiagnostic {
    pub fn new(
        severity: RegistryDiagnosticSeverity,
        code: impl Into<String>,
        path: impl Into<PathBuf>,
        field_path: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            path: path.into(),
            field_path,
            message: message.into(),
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn render_line(&self) -> String {
        let field = self
            .field_path
            .as_deref()
            .map(|field| format!(":{field}"))
            .unwrap_or_default();
        match &self.hint {
            Some(hint) => format!(
                "{} [{}] {}{}: {} (hint: {})",
                self.severity,
                self.code,
                self.path.display(),
                field,
                self.message,
                hint
            ),
            None => format!(
                "{} [{}] {}{}: {}",
                self.severity,
                self.code,
                self.path.display(),
                field,
                self.message
            ),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RegistryValidationReport {
    pub manifest_count: usize,
    pub diagnostics: Vec<RegistryValidatorDiagnostic>,
}

impl RegistryValidationReport {
    pub fn is_failure(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity.is_failure())
    }

    pub fn cli_exit_code(&self) -> i32 {
        if self.is_failure() { 1 } else { 0 }
    }

    pub fn render_text(&self) -> String {
        let mut lines = vec![format!(
            "vac init registry validator: manifests={} failures={}",
            self.manifest_count,
            self.diagnostics
                .iter()
                .filter(|d| d.severity.is_failure())
                .count()
        )];
        if self.diagnostics.is_empty() {
            lines.push("registry diagnostics: pass".to_string());
        } else {
            lines.extend(
                self.diagnostics
                    .iter()
                    .map(RegistryValidatorDiagnostic::render_line),
            );
        }
        lines.join("\n")
    }
}

pub fn validate_registry_records(records: &[RegistryManifestRecord]) -> RegistryValidationReport {
    let mut report = RegistryValidationReport {
        manifest_count: records.len(),
        diagnostics: Vec::new(),
    };
    let mut ids: BTreeMap<&str, &Path> = BTreeMap::new();
    let mut capabilities = BTreeSet::new();
    let mut surfaces = Vec::new();

    for record in records {
        if record.schema_version != VALIDATOR_SCHEMA_VERSION {
            report.diagnostics.push(RegistryValidatorDiagnostic::new(
                RegistryDiagnosticSeverity::Blocked,
                "VAC-REG-001",
                &record.path,
                Some("schema_version".to_string()),
                format!(
                    "expected schema_version {VALIDATOR_SCHEMA_VERSION}, found {}",
                    record.schema_version
                ),
            ));
        }
        if let Some(previous) = ids.insert(&record.id, &record.path) {
            report.diagnostics.push(RegistryValidatorDiagnostic::new(
                RegistryDiagnosticSeverity::Blocked,
                "VAC-REG-002",
                &record.path,
                Some("id".to_string()),
                format!(
                    "duplicate manifest id `{}`; first seen at `{}`",
                    record.id,
                    previous.display()
                ),
            ));
        }
        if record.kind.is_current_workspace_compatibility() {
            report.diagnostics.push(
                RegistryValidatorDiagnostic::new(
                    RegistryDiagnosticSeverity::Blocked,
                    "VAC-REG-003",
                    &record.path,
                    Some("kind".to_string()),
                    format!("kind `{}` is forbidden in strict registry mode", record.kind),
                )
                .with_hint("migrate compatibility descriptors to refined spec kinds such as registry_status"),
            );
        }

        match record.kind {
            VacInitRegistryKind::Capability => {
                capabilities.insert(record.id.clone());
                validate_capability_record(record, &mut report);
            }
            VacInitRegistryKind::Policy => validate_policy_record(record, &mut report),
            VacInitRegistryKind::Surface => surfaces.push(record),
            VacInitRegistryKind::Workflow => validate_workflow_record(record, &mut report),
            VacInitRegistryKind::InitState => validate_init_state_record(record, &mut report),
            VacInitRegistryKind::OwnershipReport => {
                validate_ownership_report_record(record, &mut report)
            }
            _ => {}
        }
    }

    if !capabilities.is_empty() && surfaces.is_empty() {
        report.diagnostics.push(RegistryValidatorDiagnostic::new(
            RegistryDiagnosticSeverity::Warning,
            "VAC-REG-020",
            ".vac/surfaces",
            None,
            "registry has capabilities but no surface manifests in this scan",
        ));
    }
    report
}

pub fn validate_registry_yaml_documents(docs: &[(PathBuf, String)]) -> RegistryValidationReport {
    let mut records = Vec::new();
    let mut report = RegistryValidationReport::default();
    for (path, source) in docs {
        match RegistryManifestRecord::from_yaml_str(path, source) {
            Ok(record) => records.push(record),
            Err(error) => report.diagnostics.push(RegistryValidatorDiagnostic::new(
                RegistryDiagnosticSeverity::Blocked,
                "VAC-REG-000",
                path,
                None,
                error.to_string(),
            )),
        }
    }
    let mut validation = validate_registry_records(&records);
    validation.diagnostics.extend(report.diagnostics);
    validation.manifest_count = records.len();
    validation
}

pub fn load_registry_yaml_documents(
    vac_root: impl AsRef<Path>,
) -> Result<Vec<(PathBuf, String)>, RegistryParseError> {
    let vac_root = vac_root.as_ref();
    let mut docs = Vec::new();
    collect_yaml_documents(vac_root, vac_root, &mut docs)?;
    docs.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(docs)
}

pub fn validate_registry_path(vac_root: impl AsRef<Path>) -> RegistryValidationReport {
    match load_registry_yaml_documents(vac_root) {
        Ok(docs) => validate_registry_yaml_documents(&docs),
        Err(error) => {
            let mut report = RegistryValidationReport::default();
            report.diagnostics.push(RegistryValidatorDiagnostic::new(
                RegistryDiagnosticSeverity::Error,
                "VAC-REG-IO",
                ".vac",
                None,
                error.to_string(),
            ));
            report
        }
    }
}

fn collect_yaml_documents(
    root: &Path,
    current: &Path,
    docs: &mut Vec<(PathBuf, String)>,
) -> Result<(), RegistryParseError> {
    for entry in fs::read_dir(current).map_err(|err| RegistryParseError::Io(err.to_string()))? {
        let entry = entry.map_err(|err| RegistryParseError::Io(err.to_string()))?;
        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if file_name.starts_with('.') && file_name != ".init" {
            continue;
        }
        if path.is_dir() {
            collect_yaml_documents(root, &path, docs)?;
            continue;
        }
        let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        if extension != "yaml" && extension != "yml" {
            continue;
        }
        let source =
            fs::read_to_string(&path).map_err(|err| RegistryParseError::Io(err.to_string()))?;
        let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        docs.push((relative, source));
    }
    Ok(())
}

fn validate_capability_record(
    record: &RegistryManifestRecord,
    report: &mut RegistryValidationReport,
) {
    validate_required_text(record, report, "title", record.title.as_deref());
    let status = record.status.as_deref().unwrap_or_default();
    match status {
        "planned" => {
            if record.surfaces_present {
                report.diagnostics.push(RegistryValidatorDiagnostic::new(
                    RegistryDiagnosticSeverity::Warning,
                    "VAC-CAP-001",
                    &record.path,
                    Some("surfaces".to_string()),
                    "planned capability declares surfaces; executable exposure must remain disabled",
                ));
            }
        }
        "partial" => {
            if !record.owner_present {
                report.diagnostics.push(RegistryValidatorDiagnostic::new(
                    RegistryDiagnosticSeverity::Blocked,
                    "VAC-CAP-002",
                    &record.path,
                    Some("owner".to_string()),
                    "partial capability requires owner",
                ));
            }
        }
        "ready" => {
            for (field, present) in [
                ("owner", record.owner_present),
                ("ownership", record.ownership_present),
                ("policy", record.policy_present),
                ("surfaces", record.surfaces_present),
                ("validation", record.validation_present),
                ("docs", record.docs_present),
            ] {
                if !present {
                    report.diagnostics.push(RegistryValidatorDiagnostic::new(
                        RegistryDiagnosticSeverity::Blocked,
                        format!("VAC-CAP-READY-{field}"),
                        &record.path,
                        Some(field.to_string()),
                        format!("ready capability requires `{field}`"),
                    ));
                }
            }
        }
        "deprecated" => {}
        _ => report.diagnostics.push(RegistryValidatorDiagnostic::new(
            RegistryDiagnosticSeverity::Blocked,
            "VAC-CAP-STATUS",
            &record.path,
            Some("status".to_string()),
            format!("invalid capability status `{status}`"),
        )),
    }
}

fn validate_policy_record(record: &RegistryManifestRecord, report: &mut RegistryValidationReport) {
    validate_required_text(record, report, "title", record.title.as_deref());
    if !record.policy_present {
        // Policy manifests express rules at the top level, not policy block. This
        // diagnostic remains info-level so existing serde loader owns deep rules.
        report.diagnostics.push(RegistryValidatorDiagnostic::new(
            RegistryDiagnosticSeverity::Info,
            "VAC-POL-001",
            &record.path,
            Some("rules".to_string()),
            "policy manifest deep rule validation is handled by typed policy loader",
        ));
    }
}

fn validate_workflow_record(
    record: &RegistryManifestRecord,
    report: &mut RegistryValidationReport,
) {
    validate_required_text(record, report, "title", record.title.as_deref());
    match record.status.as_deref() {
        Some("ready" | "planned" | "partial" | "disabled") => {}
        Some(status) => report.diagnostics.push(RegistryValidatorDiagnostic::new(
            RegistryDiagnosticSeverity::Blocked,
            "VAC-WF-STATUS",
            &record.path,
            Some("status".to_string()),
            format!("invalid workflow status `{status}`"),
        )),
        None => report.diagnostics.push(RegistryValidatorDiagnostic::new(
            RegistryDiagnosticSeverity::Blocked,
            "VAC-WF-STATUS",
            &record.path,
            Some("status".to_string()),
            "workflow requires status",
        )),
    }
}

fn validate_init_state_record(
    record: &RegistryManifestRecord,
    report: &mut RegistryValidationReport,
) {
    if record.id != "init.state" {
        report.diagnostics.push(RegistryValidatorDiagnostic::new(
            RegistryDiagnosticSeverity::Blocked,
            "VAC-INIT-ID",
            &record.path,
            Some("id".to_string()),
            "init_state manifest id must be `init.state`",
        ));
    }
}

fn validate_ownership_report_record(
    record: &RegistryManifestRecord,
    report: &mut RegistryValidationReport,
) {
    if !record.id.starts_with("report.ownership.") {
        report.diagnostics.push(RegistryValidatorDiagnostic::new(
            RegistryDiagnosticSeverity::Blocked,
            "VAC-OWN-ID",
            &record.path,
            Some("id".to_string()),
            "ownership_report id must start with `report.ownership.`",
        ));
    }
}

fn validate_required_text(
    record: &RegistryManifestRecord,
    report: &mut RegistryValidationReport,
    field: &str,
    value: Option<&str>,
) {
    if value.unwrap_or_default().trim().is_empty() {
        report.diagnostics.push(RegistryValidatorDiagnostic::new(
            RegistryDiagnosticSeverity::Blocked,
            format!("VAC-REQ-{field}"),
            &record.path,
            Some(field.to_string()),
            format!("manifest requires `{field}`"),
        ));
    }
}

fn collect_top_level_scalars(source: &str) -> BTreeMap<String, String> {
    let mut scalars = BTreeMap::new();
    for raw_line in source.lines() {
        if raw_line.starts_with(' ') || raw_line.starts_with('\t') {
            continue;
        }
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line == "---" {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = strip_comment(value.trim()).trim();
        if !value.is_empty() {
            scalars.insert(key.to_string(), unquote(value).to_string());
        }
    }
    scalars
}

fn collect_top_level_block_keys(source: &str) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for raw_line in source.lines() {
        if raw_line.starts_with(' ') || raw_line.starts_with('\t') {
            continue;
        }
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line == "---" {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if strip_comment(value.trim()).trim().is_empty() {
            keys.insert(key.trim().to_string());
        }
    }
    keys
}

fn required_scalar(
    scalars: &BTreeMap<String, String>,
    field: &'static str,
) -> Result<String, RegistryParseError> {
    scalars
        .get(field)
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .ok_or(RegistryParseError::MissingField(field))
}

fn validate_manifest_id(id: &str) -> Result<(), String> {
    if id == "vac" {
        return Ok(());
    }
    if id.trim() != id
        || id.chars().any(char::is_whitespace)
        || id.starts_with('.')
        || id.ends_with('.')
        || !id.contains('.')
    {
        return Err(id.to_string());
    }
    for segment in id.split('.') {
        if segment.is_empty()
            || !segment
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        {
            return Err(id.to_string());
        }
    }
    Ok(())
}

fn strip_comment(value: &str) -> &str {
    let mut in_single = false;
    let mut in_double = false;
    for (index, ch) in value.char_indices() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '#' if !in_single && !in_double => return &value[..index],
            _ => {}
        }
    }
    value
}

fn unquote(value: &str) -> &str {
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capability_doc(id: &str, status: &str) -> String {
        format!(
            r#"schema_version: 1
kind: capability
id: {id}
title: Test Capability
status: {status}
owner:
  crate: vac-core
  module: control_plane::test
ownership:
  crates: [vac-core]
policy:
  risk: safe_read
surfaces:
  palette: true
validation:
  gates: [test]
docs:
  - docs/test.md
"#
        )
    }

    #[test]
    fn parses_manifest_record_from_yaml_envelope() {
        let record = RegistryManifestRecord::from_yaml_str(
            "capabilities/test.yaml",
            &capability_doc("vac.test.capability", "ready"),
        )
        .unwrap();
        assert_eq!(record.kind, VacInitRegistryKind::Capability);
        assert_eq!(record.id, "vac.test.capability");
        assert!(record.owner_present);
        assert!(record.policy_present);
    }

    #[test]
    fn unknown_kind_is_parse_error() {
        let error = RegistryManifestRecord::from_yaml_str(
            "bad.yaml",
            "schema_version: 1\nkind: mystery\nid: vac.bad\n",
        )
        .unwrap_err();
        assert!(error.to_string().contains("unknown"));
    }

    #[test]
    fn duplicate_ids_are_blocked() {
        let first =
            RegistryManifestRecord::from_yaml_str("a.yaml", &capability_doc("vac.dup", "ready"))
                .unwrap();
        let second =
            RegistryManifestRecord::from_yaml_str("b.yaml", &capability_doc("vac.dup", "ready"))
                .unwrap();
        let report = validate_registry_records(&[first, second]);
        assert!(report.is_failure());
        assert!(report.render_text().contains("duplicate manifest id"));
    }

    #[test]
    fn ready_capability_requires_required_blocks() {
        let record = RegistryManifestRecord::from_yaml_str(
            "capabilities/min.yaml",
            "schema_version: 1\nkind: capability\nid: vac.min\ntitle: Min\nstatus: ready\n",
        )
        .unwrap();
        let report = validate_registry_records(&[record]);
        assert!(report.is_failure());
        assert!(
            report
                .render_text()
                .contains("ready capability requires `owner`")
        );
        assert!(
            report
                .render_text()
                .contains("ready capability requires `policy`")
        );
    }

    #[test]
    fn planned_capability_is_allowed_without_ready_blocks() {
        let record = RegistryManifestRecord::from_yaml_str(
            "capabilities/planned.yaml",
            "schema_version: 1\nkind: capability\nid: vac.plan\ntitle: Plan\nstatus: planned\n",
        )
        .unwrap();
        let report = validate_registry_records(&[record]);
        assert!(!report.is_failure(), "{}", report.render_text());
    }

    #[test]
    fn invalid_schema_version_is_blocked() {
        let record = RegistryManifestRecord::from_yaml_str(
            "capabilities/test.yaml",
            &capability_doc("vac.test", "ready").replace("schema_version: 1", "schema_version: 2"),
        )
        .unwrap();
        let report = validate_registry_records(&[record]);
        assert!(report.is_failure());
        assert!(report.render_text().contains("schema_version"));
    }

    #[test]
    fn init_state_id_is_fixed() {
        let record = RegistryManifestRecord::from_yaml_str(
            "state.yaml",
            "schema_version: 1\nkind: init_state\nid: init.bad\ncurrent_state: ready\n",
        )
        .unwrap();
        let report = validate_registry_records(&[record]);
        assert!(report.is_failure());
        assert!(report.render_text().contains("init.state"));
    }

    #[test]
    fn compatibility_kinds_are_blocked_in_strict_mode() {
        let record = RegistryManifestRecord::from_yaml_str(
            "registry/product.yaml",
            "schema_version: 1\nkind: product\nid: vac\ntitle: VAC\n",
        )
        .unwrap();
        let report = validate_registry_records(&[record]);
        assert!(report.is_failure(), "{}", report.render_text());
        assert!(
            report
                .render_text()
                .contains("forbidden in strict registry mode")
        );
    }
}
