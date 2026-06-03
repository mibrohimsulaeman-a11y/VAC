#![allow(dead_code)]
//! Dependency-free VAC-Init manifest contracts for capability, policy, surface,
//! and workflow manifests.
//!
//! The existing serde-backed loaders remain the runtime YAML implementation.
//! This module captures the v1-alpha refined spec as stable Rust contracts that
//! can be validated with direct `rustc --test` in sandboxed environments.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ManifestValidationSeverity {
    Info,
    Warning,
    Error,
    Blocked,
}

impl ManifestValidationSeverity {
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

impl fmt::Display for ManifestValidationSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestValidationIssue {
    pub severity: ManifestValidationSeverity,
    pub manifest_id: String,
    pub field_path: String,
    pub message: String,
    pub hint: Option<String>,
}

impl ManifestValidationIssue {
    pub fn new(
        severity: ManifestValidationSeverity,
        manifest_id: impl Into<String>,
        field_path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            manifest_id: manifest_id.into(),
            field_path: field_path.into(),
            message: message.into(),
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn render_line(&self) -> String {
        match &self.hint {
            Some(hint) => format!(
                "{}: {}:{}: {} (hint: {})",
                self.severity, self.manifest_id, self.field_path, self.message, hint
            ),
            None => format!(
                "{}: {}:{}: {}",
                self.severity, self.manifest_id, self.field_path, self.message
            ),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManifestValidationReport {
    pub issues: Vec<ManifestValidationIssue>,
}

impl ManifestValidationReport {
    pub fn new() -> Self {
        Self { issues: Vec::new() }
    }

    pub fn push(&mut self, issue: ManifestValidationIssue) {
        self.issues.push(issue);
    }

    pub fn extend(&mut self, issues: Vec<ManifestValidationIssue>) {
        self.issues.extend(issues);
    }

    pub fn is_failure(&self) -> bool {
        self.issues.iter().any(|issue| issue.severity.is_failure())
    }

    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity.is_failure())
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == ManifestValidationSeverity::Warning)
            .count()
    }

    pub fn render_text(&self) -> String {
        if self.issues.is_empty() {
            return "manifest contract: pass".to_string();
        }
        self.issues
            .iter()
            .map(ManifestValidationIssue::render_line)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CapabilityLifecycleStatus {
    Planned,
    Partial,
    Ready,
    Deprecated,
}

impl CapabilityLifecycleStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Partial => "partial",
            Self::Ready => "ready",
            Self::Deprecated => "deprecated",
        }
    }

    pub const fn allows_executable_surface(self) -> bool {
        matches!(self, Self::Partial | Self::Ready)
    }
}

impl fmt::Display for CapabilityLifecycleStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RiskLevel {
    SafeRead,
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SafeRead => "safe_read",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub const fn requires_approval_by_default(self) -> bool {
        matches!(self, Self::High | Self::Critical)
    }
}

impl fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PolicyDecision {
    Allow,
    ApprovalRequired,
    Deny,
}

impl PolicyDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::ApprovalRequired => "approval_required",
            Self::Deny => "deny",
        }
    }

    pub const fn precedence(self) -> u8 {
        match self {
            Self::Deny => 3,
            Self::ApprovalRequired => 2,
            Self::Allow => 1,
        }
    }
}

impl fmt::Display for PolicyDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PolicyAction {
    FilesystemRead,
    FilesystemWrite,
    FilesystemDelete,
    ExecuteProcess,
    NetworkAccess,
    SessionWrite,
    CheckpointWrite,
    CredentialRead,
    MemoryWrite,
    PlanModify,
    CapabilityRegister,
}

impl PolicyAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FilesystemRead => "filesystem_read",
            Self::FilesystemWrite => "filesystem_write",
            Self::FilesystemDelete => "filesystem_delete",
            Self::ExecuteProcess => "execute_process",
            Self::NetworkAccess => "network_access",
            Self::SessionWrite => "session_write",
            Self::CheckpointWrite => "checkpoint_write",
            Self::CredentialRead => "credential_read",
            Self::MemoryWrite => "memory_write",
            Self::PlanModify => "plan_modify",
            Self::CapabilityRegister => "capability_register",
        }
    }

    pub const fn is_mutating_or_sensitive(self) -> bool {
        !matches!(self, Self::FilesystemRead)
    }
}

impl fmt::Display for PolicyAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityOwnerContract {
    pub crate_name: String,
    pub module: String,
    pub team: Option<String>,
}

impl CapabilityOwnerContract {
    pub fn new(crate_name: impl Into<String>, module: impl Into<String>) -> Self {
        Self {
            crate_name: crate_name.into(),
            module: module.into(),
            team: None,
        }
    }

    pub fn display_name(&self) -> String {
        format!("{}::{}", self.crate_name, self.module)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OwnershipTargetKind {
    Crate,
    Module,
    Path,
}

impl OwnershipTargetKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Crate => "crate",
            Self::Module => "module",
            Self::Path => "path",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipTargetContract {
    pub kind: OwnershipTargetKind,
    pub crate_name: Option<String>,
    pub module: Option<String>,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

impl OwnershipTargetContract {
    pub fn crate_target(crate_name: impl Into<String>) -> Self {
        Self {
            kind: OwnershipTargetKind::Crate,
            crate_name: Some(crate_name.into()),
            module: None,
            include: Vec::new(),
            exclude: Vec::new(),
        }
    }

    pub fn module_target(crate_name: impl Into<String>, module: impl Into<String>) -> Self {
        Self {
            kind: OwnershipTargetKind::Module,
            crate_name: Some(crate_name.into()),
            module: Some(module.into()),
            include: Vec::new(),
            exclude: Vec::new(),
        }
    }

    pub fn path_target(include: Vec<String>, exclude: Vec<String>) -> Self {
        Self {
            kind: OwnershipTargetKind::Path,
            crate_name: None,
            module: None,
            include,
            exclude,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilityOwnershipContract {
    pub crates: Vec<String>,
    pub modules: Vec<String>,
    pub targets: Vec<OwnershipTargetContract>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityPolicyContract {
    pub risk: RiskLevel,
    pub mutates_files: bool,
    pub network: bool,
    pub redaction: bool,
    pub approval_required_for: Vec<PolicyAction>,
}

impl CapabilityPolicyContract {
    pub fn safe_read() -> Self {
        Self {
            risk: RiskLevel::SafeRead,
            mutates_files: false,
            network: false,
            redaction: false,
            approval_required_for: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilitySurfaceContract {
    pub cli_commands: Vec<String>,
    pub palette: bool,
    pub tui_route: Option<String>,
}

impl CapabilitySurfaceContract {
    pub fn is_empty(&self) -> bool {
        self.cli_commands.is_empty() && !self.palette && self.tui_route.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ValidationApprovalMode {
    Policy,
    Always,
    Never,
}

impl ValidationApprovalMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Policy => "policy",
            Self::Always => "always",
            Self::Never => "never",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredValidationCommand {
    pub id: String,
    pub runner: String,
    pub args: Vec<String>,
    pub risk: RiskLevel,
    pub approval: ValidationApprovalMode,
}

impl StructuredValidationCommand {
    pub fn new(
        id: impl Into<String>,
        runner: impl Into<String>,
        args: Vec<String>,
        risk: RiskLevel,
        approval: ValidationApprovalMode,
    ) -> Self {
        Self {
            id: id.into(),
            runner: runner.into(),
            args,
            risk,
            approval,
        }
    }

    pub fn is_structured(&self) -> bool {
        !self.id.trim().is_empty()
            && !self.runner.trim().is_empty()
            && !self.runner.contains(' ')
            && !self.runner.contains('|')
            && !self.runner.contains('>')
            && !self.runner.contains('<')
            && self
                .args
                .iter()
                .all(|arg| !arg.contains('|') && !arg.contains('>') && !arg.contains('<'))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationContract {
    pub gates: Vec<String>,
    pub commands: Vec<StructuredValidationCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityManifestContract {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub title: String,
    pub status: CapabilityLifecycleStatus,
    pub owner: Option<CapabilityOwnerContract>,
    pub ownership: CapabilityOwnershipContract,
    pub depends_on: Vec<String>,
    pub policy: Option<CapabilityPolicyContract>,
    pub surfaces: CapabilitySurfaceContract,
    pub states: Vec<String>,
    pub validation: ValidationContract,
    pub docs: Vec<String>,
    pub reason: Option<String>,
    pub next_validation: Option<String>,
    pub deprecation: Option<String>,
}

impl CapabilityManifestContract {
    pub fn minimal_planned(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            schema_version: MANIFEST_SCHEMA_VERSION,
            kind: "capability".to_string(),
            id: id.into(),
            title: title.into(),
            status: CapabilityLifecycleStatus::Planned,
            owner: None,
            ownership: CapabilityOwnershipContract::default(),
            depends_on: Vec::new(),
            policy: None,
            surfaces: CapabilitySurfaceContract {
                cli_commands: Vec::new(),
                palette: false,
                tui_route: None,
            },
            states: Vec::new(),
            validation: ValidationContract::default(),
            docs: Vec::new(),
            reason: None,
            next_validation: None,
            deprecation: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyMatchContract {
    pub action: PolicyAction,
    pub path: Option<String>,
    pub data_class: Option<String>,
    pub network_host: Option<String>,
    pub network_protocol: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyRuleContract {
    pub id: String,
    pub match_: PolicyMatchContract,
    pub decision: PolicyDecision,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyManifestContract {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub title: String,
    pub default_decision: PolicyDecision,
    pub rules: Vec<PolicyRuleContract>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SurfaceRouteKindContract {
    Cli,
    Tui,
    Slash,
    Palette,
}

impl SurfaceRouteKindContract {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Tui => "tui",
            Self::Slash => "slash",
            Self::Palette => "palette",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceRouteContract {
    pub kind: SurfaceRouteKindContract,
    pub command: String,
    pub capability: String,
    pub owner: String,
    pub visible: bool,
    pub status: CapabilityLifecycleStatus,
}

impl SurfaceRouteContract {
    pub fn is_executable(&self) -> bool {
        self.visible
            && matches!(
                self.kind,
                SurfaceRouteKindContract::Cli
                    | SurfaceRouteKindContract::Slash
                    | SurfaceRouteKindContract::Palette
                    | SurfaceRouteKindContract::Tui
            )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceManifestContract {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub title: String,
    pub capabilities: Vec<String>,
    pub routes: Vec<SurfaceRouteContract>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WorkflowStatusContract {
    Planned,
    Ready,
}

impl WorkflowStatusContract {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Ready => "ready",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowStepContract {
    pub id: String,
    pub uses: String,
    pub depends_on: Vec<String>,
    pub ui_surface: Option<String>,
    pub approval_surface: bool,
    pub evidence_surface: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowPolicyContract {
    pub default_risk: RiskLevel,
    pub mutates_files: bool,
    pub network: bool,
    pub redaction: bool,
    pub approval_required_for: Vec<PolicyAction>,
}

impl WorkflowPolicyContract {
    pub fn safe_read() -> Self {
        Self {
            default_risk: RiskLevel::SafeRead,
            mutates_files: false,
            network: false,
            redaction: false,
            approval_required_for: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowManifestContract {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub title: String,
    pub status: WorkflowStatusContract,
    pub inputs: BTreeMap<String, String>,
    pub steps: Vec<WorkflowStepContract>,
    pub policy: WorkflowPolicyContract,
    pub validation: ValidationContract,
    pub ui_surface: String,
}

pub fn validate_capability_contract(
    manifest: &CapabilityManifestContract,
) -> ManifestValidationReport {
    let mut report = ManifestValidationReport::new();
    validate_common_envelope(
        &mut report,
        &manifest.id,
        manifest.schema_version,
        &manifest.kind,
        "capability",
    );
    validate_non_empty(&mut report, &manifest.id, "title", &manifest.title);

    match manifest.status {
        CapabilityLifecycleStatus::Planned => {
            if !manifest.surfaces.is_empty() {
                report.push(ManifestValidationIssue::new(
                    ManifestValidationSeverity::Blocked,
                    &manifest.id,
                    "surfaces",
                    "planned capability must not expose executable surfaces",
                ));
            }
        }
        CapabilityLifecycleStatus::Partial => {
            if manifest.owner.is_none() {
                report.push(ManifestValidationIssue::new(
                    ManifestValidationSeverity::Blocked,
                    &manifest.id,
                    "owner",
                    "partial capability requires owner assignment",
                ));
            }
            if manifest
                .reason
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            {
                report.push(ManifestValidationIssue::new(
                    ManifestValidationSeverity::Warning,
                    &manifest.id,
                    "reason",
                    "partial capability should explain remaining work",
                ));
            }
            if manifest
                .next_validation
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            {
                report.push(ManifestValidationIssue::new(
                    ManifestValidationSeverity::Warning,
                    &manifest.id,
                    "next_validation",
                    "partial capability should name the next validation gate",
                ));
            }
        }
        CapabilityLifecycleStatus::Ready => {
            if manifest.owner.is_none() {
                report.push(ManifestValidationIssue::new(
                    ManifestValidationSeverity::Blocked,
                    &manifest.id,
                    "owner",
                    "ready capability requires owner",
                ));
            }
            if manifest.ownership.targets.is_empty()
                && manifest.ownership.crates.is_empty()
                && manifest.ownership.modules.is_empty()
            {
                report.push(ManifestValidationIssue::new(
                    ManifestValidationSeverity::Blocked,
                    &manifest.id,
                    "ownership",
                    "ready capability requires ownership targets",
                ));
            }
            if manifest.policy.is_none() {
                report.push(ManifestValidationIssue::new(
                    ManifestValidationSeverity::Blocked,
                    &manifest.id,
                    "policy",
                    "ready capability requires policy block",
                ));
            }
            if manifest.surfaces.is_empty() {
                report.push(ManifestValidationIssue::new(
                    ManifestValidationSeverity::Blocked,
                    &manifest.id,
                    "surfaces",
                    "ready capability requires at least one surface",
                ));
            }
            if manifest.validation.commands.is_empty() && manifest.validation.gates.is_empty() {
                report.push(ManifestValidationIssue::new(
                    ManifestValidationSeverity::Blocked,
                    &manifest.id,
                    "validation",
                    "ready capability requires validation commands or gates",
                ));
            }
        }
        CapabilityLifecycleStatus::Deprecated => {
            if manifest
                .deprecation
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            {
                report.push(ManifestValidationIssue::new(
                    ManifestValidationSeverity::Blocked,
                    &manifest.id,
                    "deprecation",
                    "deprecated capability requires deprecation block",
                ));
            }
        }
    }

    for target in &manifest.ownership.targets {
        match target.kind {
            OwnershipTargetKind::Crate => {
                if target.crate_name.as_deref().unwrap_or_default().is_empty() {
                    report.push(ManifestValidationIssue::new(
                        ManifestValidationSeverity::Blocked,
                        &manifest.id,
                        "ownership.targets[].crate",
                        "crate target requires crate name",
                    ));
                }
            }
            OwnershipTargetKind::Module => {
                if target.crate_name.as_deref().unwrap_or_default().is_empty()
                    || target.module.as_deref().unwrap_or_default().is_empty()
                {
                    report.push(ManifestValidationIssue::new(
                        ManifestValidationSeverity::Blocked,
                        &manifest.id,
                        "ownership.targets[].module",
                        "module target requires crate and module",
                    ));
                }
            }
            OwnershipTargetKind::Path => {
                if target.include.is_empty() {
                    report.push(ManifestValidationIssue::new(
                        ManifestValidationSeverity::Blocked,
                        &manifest.id,
                        "ownership.targets[].include",
                        "path target requires include patterns",
                    ));
                }
            }
        }
    }

    for command in &manifest.validation.commands {
        validate_command(&mut report, &manifest.id, command);
    }
    report
}

pub fn validate_policy_contract(manifest: &PolicyManifestContract) -> ManifestValidationReport {
    let mut report = ManifestValidationReport::new();
    validate_common_envelope(
        &mut report,
        &manifest.id,
        manifest.schema_version,
        &manifest.kind,
        "policy",
    );
    validate_non_empty(&mut report, &manifest.id, "title", &manifest.title);
    if manifest.rules.is_empty() && manifest.default_decision != PolicyDecision::Deny {
        report.push(ManifestValidationIssue::new(
            ManifestValidationSeverity::Warning,
            &manifest.id,
            "rules",
            "policy with no rules should usually default to deny",
        ));
    }
    let mut ids = BTreeSet::new();
    for rule in &manifest.rules {
        validate_non_empty(&mut report, &manifest.id, "rules[].id", &rule.id);
        if !ids.insert(rule.id.as_str()) {
            report.push(ManifestValidationIssue::new(
                ManifestValidationSeverity::Blocked,
                &manifest.id,
                "rules[].id",
                format!("duplicate rule id `{}`", rule.id),
            ));
        }
        if rule.reason.trim().is_empty() {
            report.push(ManifestValidationIssue::new(
                ManifestValidationSeverity::Blocked,
                &manifest.id,
                "rules[].reason",
                "policy rule requires human-readable reason",
            ));
        }
        if rule.match_.action == PolicyAction::NetworkAccess
            && rule
                .match_
                .network_host
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            report.push(ManifestValidationIssue::new(
                ManifestValidationSeverity::Blocked,
                &manifest.id,
                "rules[].match.network.host",
                "network_access rule requires host scope",
            ));
        }
    }
    report
}

pub fn validate_surface_contract(
    manifest: &SurfaceManifestContract,
    capabilities: &BTreeMap<String, CapabilityManifestContract>,
) -> ManifestValidationReport {
    let mut report = ManifestValidationReport::new();
    validate_common_envelope(
        &mut report,
        &manifest.id,
        manifest.schema_version,
        &manifest.kind,
        "surface",
    );
    validate_non_empty(&mut report, &manifest.id, "title", &manifest.title);

    for capability in &manifest.capabilities {
        if !capabilities.contains_key(capability) {
            report.push(ManifestValidationIssue::new(
                ManifestValidationSeverity::Blocked,
                &manifest.id,
                "capabilities[]",
                format!("surface references unknown capability `{capability}`"),
            ));
        }
    }

    for route in &manifest.routes {
        if route.command.trim().is_empty() {
            report.push(ManifestValidationIssue::new(
                ManifestValidationSeverity::Blocked,
                &manifest.id,
                "routes[].command",
                "surface route requires command/path/action value",
            ));
        }
        let Some(capability) = capabilities.get(&route.capability) else {
            report.push(ManifestValidationIssue::new(
                ManifestValidationSeverity::Blocked,
                &manifest.id,
                "routes[].capability",
                format!("route references unknown capability `{}`", route.capability),
            ));
            continue;
        };
        if route.visible
            && route.is_executable()
            && capability.status == CapabilityLifecycleStatus::Planned
        {
            report.push(ManifestValidationIssue::new(
                ManifestValidationSeverity::Blocked,
                &manifest.id,
                "routes[].status",
                "planned capability must not be visible as executable route",
            ));
        }
        if route.status == CapabilityLifecycleStatus::Planned && route.visible {
            report.push(ManifestValidationIssue::new(
                ManifestValidationSeverity::Blocked,
                &manifest.id,
                "routes[].visible",
                "planned route must not be visible",
            ));
        }
        if let Some(owner) = &capability.owner {
            let owner_text = owner.display_name();
            let legacy_owner = format!("{}/{}", owner.crate_name, owner.module.replace("::", "/"));
            if !route.owner.is_empty()
                && route.owner != owner_text
                && route.owner != legacy_owner
                && !route.owner.contains(&owner.crate_name)
            {
                report.push(ManifestValidationIssue::new(
                    ManifestValidationSeverity::Warning,
                    &manifest.id,
                    "routes[].owner",
                    format!(
                        "route owner `{}` does not obviously match capability owner `{}`",
                        route.owner, owner_text
                    ),
                ));
            }
        }
    }

    for capability in capabilities.values() {
        if capability.status.allows_executable_surface() {
            let has_surface = manifest
                .routes
                .iter()
                .any(|route| route.capability == capability.id && route.visible);
            if !has_surface && capability.surfaces.is_empty() {
                report.push(ManifestValidationIssue::new(
                    ManifestValidationSeverity::Warning,
                    &manifest.id,
                    "routes",
                    format!("capability `{}` has no visible route", capability.id),
                ));
            }
        }
    }
    report
}

pub fn validate_workflow_contract(manifest: &WorkflowManifestContract) -> ManifestValidationReport {
    let mut report = ManifestValidationReport::new();
    validate_common_envelope(
        &mut report,
        &manifest.id,
        manifest.schema_version,
        &manifest.kind,
        "workflow",
    );
    validate_non_empty(&mut report, &manifest.id, "title", &manifest.title);
    validate_non_empty(
        &mut report,
        &manifest.id,
        "ui.surface",
        &manifest.ui_surface,
    );

    if manifest.steps.is_empty() {
        report.push(ManifestValidationIssue::new(
            ManifestValidationSeverity::Blocked,
            &manifest.id,
            "steps",
            "workflow requires at least one typed step",
        ));
    }

    let mut step_ids = BTreeSet::new();
    for step in &manifest.steps {
        validate_non_empty(&mut report, &manifest.id, "steps[].id", &step.id);
        validate_non_empty(&mut report, &manifest.id, "steps[].uses", &step.uses);
        if !step_ids.insert(step.id.as_str()) {
            report.push(ManifestValidationIssue::new(
                ManifestValidationSeverity::Blocked,
                &manifest.id,
                "steps[].id",
                format!("duplicate workflow step id `{}`", step.id),
            ));
        }
        if !step.uses.starts_with("capability.") && !step.uses.starts_with("step.") {
            report.push(ManifestValidationIssue::new(
                ManifestValidationSeverity::Blocked,
                &manifest.id,
                "steps[].uses",
                "workflow step must use allowed capability.* or step.* vocabulary",
            ));
        }
    }

    for step in &manifest.steps {
        for dep in &step.depends_on {
            if !step_ids.contains(dep.as_str()) {
                report.push(ManifestValidationIssue::new(
                    ManifestValidationSeverity::Blocked,
                    &manifest.id,
                    "steps[].depends_on",
                    format!("step `{}` depends on unknown step `{dep}`", step.id),
                ));
            }
        }
    }

    for command in &manifest.validation.commands {
        validate_command(&mut report, &manifest.id, command);
    }
    report
}

pub fn merge_policy_decisions(decisions: &[PolicyDecision]) -> PolicyDecision {
    decisions
        .iter()
        .copied()
        .max_by_key(|decision| decision.precedence())
        .unwrap_or(PolicyDecision::Deny)
}

fn validate_common_envelope(
    report: &mut ManifestValidationReport,
    id: &str,
    schema_version: u32,
    kind: &str,
    expected_kind: &str,
) {
    if schema_version != MANIFEST_SCHEMA_VERSION {
        report.push(ManifestValidationIssue::new(
            ManifestValidationSeverity::Blocked,
            id,
            "schema_version",
            format!("expected schema_version {MANIFEST_SCHEMA_VERSION}, found {schema_version}"),
        ));
    }
    if kind != expected_kind {
        report.push(ManifestValidationIssue::new(
            ManifestValidationSeverity::Blocked,
            id,
            "kind",
            format!("expected kind `{expected_kind}`, found `{kind}`"),
        ));
    }
    validate_non_empty(report, id, "id", id);
    if !id.contains('.') {
        report.push(ManifestValidationIssue::new(
            ManifestValidationSeverity::Blocked,
            id,
            "id",
            "manifest id must be dotted",
        ));
    }
}

fn validate_non_empty(
    report: &mut ManifestValidationReport,
    manifest_id: &str,
    field_path: &str,
    value: &str,
) {
    if value.trim().is_empty() {
        report.push(ManifestValidationIssue::new(
            ManifestValidationSeverity::Blocked,
            manifest_id,
            field_path,
            "missing required field",
        ));
    }
}

fn validate_command(
    report: &mut ManifestValidationReport,
    manifest_id: &str,
    command: &StructuredValidationCommand,
) {
    if !command.is_structured() {
        report.push(ManifestValidationIssue::new(
            ManifestValidationSeverity::Blocked,
            manifest_id,
            "validation.commands[]",
            format!("command `{}` is not structured", command.id),
        ));
    }
    if command.risk.requires_approval_by_default()
        && command.approval == ValidationApprovalMode::Never
    {
        report.push(ManifestValidationIssue::new(
            ManifestValidationSeverity::Blocked,
            manifest_id,
            "validation.commands[].approval",
            format!(
                "high-risk command `{}` must not use approval=never",
                command.id
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_capability() -> CapabilityManifestContract {
        let mut capability =
            CapabilityManifestContract::minimal_planned("vac.test.ready", "Ready Test Capability");
        capability.status = CapabilityLifecycleStatus::Ready;
        capability.owner = Some(CapabilityOwnerContract::new(
            "vac-core",
            "control_plane::test",
        ));
        capability.ownership.crates.push("vac-core".to_string());
        capability
            .ownership
            .modules
            .push("control_plane::test".to_string());
        capability
            .ownership
            .targets
            .push(OwnershipTargetContract::module_target(
                "vac-core",
                "control_plane::test",
            ));
        capability.policy = Some(CapabilityPolicyContract::safe_read());
        capability.surfaces.palette = true;
        capability.surfaces.tui_route = Some("/capabilities".to_string());
        capability.validation.gates.push("test.gate".to_string());
        capability
    }

    #[test]
    fn ready_capability_requires_owner_policy_surface_and_validation() {
        let mut capability =
            CapabilityManifestContract::minimal_planned("vac.test.missing", "Missing Ready Fields");
        capability.status = CapabilityLifecycleStatus::Ready;
        let report = validate_capability_contract(&capability);
        assert!(report.is_failure());
        assert!(
            report
                .render_text()
                .contains("ready capability requires owner")
        );
        assert!(report.render_text().contains("ownership targets"));
        assert!(report.render_text().contains("policy block"));
    }

    #[test]
    fn ready_capability_contract_passes() {
        let report = validate_capability_contract(&ready_capability());
        assert!(!report.is_failure(), "{}", report.render_text());
    }

    #[test]
    fn planned_capability_must_not_expose_surface() {
        let mut capability =
            CapabilityManifestContract::minimal_planned("vac.test.planned", "Planned Capability");
        capability.surfaces.palette = true;
        let report = validate_capability_contract(&capability);
        assert!(report.is_failure());
        assert!(report.render_text().contains("planned capability"));
    }

    #[test]
    fn deprecated_capability_requires_deprecation_block() {
        let mut capability = CapabilityManifestContract::minimal_planned(
            "vac.test.deprecated",
            "Deprecated Capability",
        );
        capability.status = CapabilityLifecycleStatus::Deprecated;
        let report = validate_capability_contract(&capability);
        assert!(report.is_failure());
        assert!(report.render_text().contains("deprecation"));
    }

    #[test]
    fn policy_merge_is_most_restrictive() {
        assert_eq!(
            merge_policy_decisions(&[PolicyDecision::Allow, PolicyDecision::ApprovalRequired]),
            PolicyDecision::ApprovalRequired
        );
        assert_eq!(
            merge_policy_decisions(&[PolicyDecision::Allow, PolicyDecision::Deny]),
            PolicyDecision::Deny
        );
        assert_eq!(merge_policy_decisions(&[]), PolicyDecision::Deny);
    }

    #[test]
    fn policy_network_rule_requires_host_scope() {
        let policy = PolicyManifestContract {
            schema_version: MANIFEST_SCHEMA_VERSION,
            kind: "policy".to_string(),
            id: "vac.policy.test".to_string(),
            title: "Policy Test".to_string(),
            default_decision: PolicyDecision::Deny,
            rules: vec![PolicyRuleContract {
                id: "rule.network".to_string(),
                match_: PolicyMatchContract {
                    action: PolicyAction::NetworkAccess,
                    path: None,
                    data_class: None,
                    network_host: None,
                    network_protocol: Some("https".to_string()),
                },
                decision: PolicyDecision::ApprovalRequired,
                reason: "network requires review".to_string(),
            }],
        };
        let report = validate_policy_contract(&policy);
        assert!(report.is_failure());
        assert!(report.render_text().contains("network_access"));
    }

    #[test]
    fn surface_rejects_planned_executable_route() {
        let planned =
            CapabilityManifestContract::minimal_planned("vac.test.planned", "Planned Capability");
        let mut capabilities = BTreeMap::new();
        capabilities.insert(planned.id.clone(), planned);
        let surface = SurfaceManifestContract {
            schema_version: MANIFEST_SCHEMA_VERSION,
            kind: "surface".to_string(),
            id: "surface.test".to_string(),
            title: "Surface Test".to_string(),
            capabilities: vec!["vac.test.planned".to_string()],
            routes: vec![SurfaceRouteContract {
                kind: SurfaceRouteKindContract::Cli,
                command: "vac test".to_string(),
                capability: "vac.test.planned".to_string(),
                owner: "vac-core/control_plane::test".to_string(),
                visible: true,
                status: CapabilityLifecycleStatus::Planned,
            }],
        };
        let report = validate_surface_contract(&surface, &capabilities);
        assert!(report.is_failure());
        assert!(report.render_text().contains("planned"));
    }

    #[test]
    fn workflow_depends_on_unknown_step_is_blocked() {
        let workflow = WorkflowManifestContract {
            schema_version: MANIFEST_SCHEMA_VERSION,
            kind: "workflow".to_string(),
            id: "maintenance.test.workflow".to_string(),
            title: "Workflow Test".to_string(),
            status: WorkflowStatusContract::Ready,
            inputs: BTreeMap::new(),
            steps: vec![WorkflowStepContract {
                id: "validate".to_string(),
                uses: "capability.test.check".to_string(),
                depends_on: vec!["missing".to_string()],
                ui_surface: Some("/capabilities".to_string()),
                approval_surface: false,
                evidence_surface: true,
            }],
            policy: WorkflowPolicyContract::safe_read(),
            validation: ValidationContract::default(),
            ui_surface: "/capabilities".to_string(),
        };
        let report = validate_workflow_contract(&workflow);
        assert!(report.is_failure());
        assert!(report.render_text().contains("unknown step"));
    }

    #[test]
    fn high_risk_command_cannot_skip_approval() {
        let mut capability = ready_capability();
        capability
            .validation
            .commands
            .push(StructuredValidationCommand::new(
                "danger.exec",
                "cargo",
                vec!["test".to_string()],
                RiskLevel::High,
                ValidationApprovalMode::Never,
            ));
        let report = validate_capability_contract(&capability);
        assert!(report.is_failure());
        assert!(report.render_text().contains("approval=never"));
    }

    #[test]
    fn free_form_shell_command_is_rejected() {
        let command = StructuredValidationCommand::new(
            "bad.shell",
            "bash -c",
            vec!["cargo test | tee out".to_string()],
            RiskLevel::High,
            ValidationApprovalMode::Policy,
        );
        assert!(!command.is_structured());
    }
}
