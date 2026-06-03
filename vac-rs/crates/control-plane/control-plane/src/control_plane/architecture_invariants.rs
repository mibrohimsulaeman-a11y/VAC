use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use super::capability_manifest::CapabilityCompatibilityTransport;
use super::capability_manifest::CapabilityManifest;
use super::capability_manifest::CapabilityStatus;
use super::docs_maintenance::DocsMaintenanceReport;
use super::docs_maintenance::load_docs_maintenance_report;
use super::ownership_scan::OwnershipScanReport;
use super::ownership_scan::load_ownership_scan_report;
use super::registry::find_vac_root;
use super::registry::load_control_plane_registry_report;
use super::registry_diagnostics::RegistryLoadReport;
use super::workflow_runner::evaluate_workflow_approval_policy;
use super::workflow_runner::preview_workflow_manifest;
use super::workflow_runner::resolve_workflow_step_handler;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchitectureInvariantStatus {
    Pass,
    Warn,
    Fail,
}

impl fmt::Display for ArchitectureInvariantStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Pass => "pass",
            Self::Warn => "warn",
            Self::Fail => "fail",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchitectureInvariantCheck {
    pub name: &'static str,
    pub status: ArchitectureInvariantStatus,
    pub summary: String,
    pub details: Vec<String>,
}

impl ArchitectureInvariantCheck {
    fn pass(name: &'static str, summary: impl Into<String>) -> Self {
        Self {
            name,
            status: ArchitectureInvariantStatus::Pass,
            summary: summary.into(),
            details: Vec::new(),
        }
    }

    fn warn(name: &'static str, summary: impl Into<String>) -> Self {
        Self {
            name,
            status: ArchitectureInvariantStatus::Warn,
            summary: summary.into(),
            details: Vec::new(),
        }
    }

    fn fail(name: &'static str, summary: impl Into<String>) -> Self {
        Self {
            name,
            status: ArchitectureInvariantStatus::Fail,
            summary: summary.into(),
            details: Vec::new(),
        }
    }

    fn with_details(mut self, details: Vec<String>) -> Self {
        self.details = details;
        self
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = vec![format!("{}: {} - {}", self.name, self.status, self.summary)];
        lines.extend(self.details.iter().map(|detail| format!("  {detail}")));
        lines
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArchitectureInvariantReport {
    repo_root: Option<PathBuf>,
    registry: RegistryLoadReport,
    ownership: OwnershipScanReport,
    docs: DocsMaintenanceReport,
    checks: Vec<ArchitectureInvariantCheck>,
}

impl ArchitectureInvariantReport {
    pub fn repo_root(&self) -> Option<&Path> {
        self.repo_root.as_deref()
    }

    pub fn registry(&self) -> &RegistryLoadReport {
        &self.registry
    }

    pub fn ownership(&self) -> &OwnershipScanReport {
        &self.ownership
    }

    pub fn docs(&self) -> &DocsMaintenanceReport {
        &self.docs
    }

    pub fn checks(&self) -> &[ArchitectureInvariantCheck] {
        &self.checks
    }

    pub fn pass_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|check| matches!(check.status, ArchitectureInvariantStatus::Pass))
            .count()
    }

    pub fn warn_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|check| matches!(check.status, ArchitectureInvariantStatus::Warn))
            .count()
    }

    pub fn fail_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|check| matches!(check.status, ArchitectureInvariantStatus::Fail))
            .count()
    }

    pub fn is_failure(&self) -> bool {
        self.fail_count() > 0
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(if self.is_failure() {
            "architecture invariants: failure".to_string()
        } else if self.warn_count() > 0 {
            "architecture invariants: warning".to_string()
        } else {
            "architecture invariants: ok".to_string()
        });
        lines.push(format!(
            "architecture summary: pass={} warn={} fail={}",
            self.pass_count(),
            self.warn_count(),
            self.fail_count()
        ));
        match self.repo_root() {
            Some(repo_root) => lines.push(format!("repo root: {}", repo_root.display())),
            None => lines.push("repo root: <unknown>".to_string()),
        }

        if self.checks.is_empty() {
            lines.push("architecture checks: <none>".to_string());
            return lines;
        }

        lines.push("architecture checks:".to_string());
        for (index, check) in self.checks.iter().enumerate() {
            lines.push(format!("  {}. {}", index + 1, check.render_lines()[0]));
            for detail in check.render_lines().into_iter().skip(1) {
                lines.push(format!("     {detail}"));
            }
        }
        lines
    }

    pub fn render_text(&self) -> String {
        self.render_lines().join("\n")
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductManifest {
    schema_version: u32,
    kind: String,
    id: String,
    title: String,
    command: String,
    status: String,
    control_plane: String,
    runtime: String,
    launcher: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusManifest {
    schema_version: u32,
    kind: String,
    id: String,
    status: String,
    #[serde(default)]
    notes: Vec<String>,
}

fn load_yaml_manifest<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let contents = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let deserializer = serde_yaml::Deserializer::from_str(&contents);
    serde_path_to_error::deserialize(deserializer).map_err(|error| error.into_inner().to_string())
}

pub fn load_architecture_invariant_report(start: impl AsRef<Path>) -> ArchitectureInvariantReport {
    let registry = load_control_plane_registry_report(start.as_ref());
    let ownership = load_ownership_scan_report(start.as_ref());
    let docs = load_docs_maintenance_report(start.as_ref());

    let repo_root = find_vac_root(start.as_ref())
        .ok()
        .and_then(|vac_root| vac_root.parent().map(Path::to_path_buf));

    let checks = match repo_root.as_ref() {
        Some(repo_root) => build_checks(repo_root, &registry, &ownership, &docs),
        None => vec![
            ArchitectureInvariantCheck::fail("source-of-truth", "unable to locate `.vac` root")
                .with_details(vec![
                    "run from the repository root or a directory inside it".to_string(),
                ]),
        ],
    };

    ArchitectureInvariantReport {
        repo_root,
        registry,
        ownership,
        docs,
        checks,
    }
}

fn build_checks(
    repo_root: &Path,
    registry: &RegistryLoadReport,
    ownership: &OwnershipScanReport,
    docs: &DocsMaintenanceReport,
) -> Vec<ArchitectureInvariantCheck> {
    vec![
        check_product_manifest(repo_root),
        check_registry_inventory(registry),
        check_tui_observability(registry),
        check_policy_gates(registry),
        check_workflow_execution(registry),
        check_approval_protects(registry),
        check_ownership_coverage(ownership),
        check_legacy_transport_quarantine(repo_root, registry),
        check_tui_direct_app_server_dependency(repo_root),
        check_docs_alignment(docs),
    ]
}

fn check_product_manifest(repo_root: &Path) -> ArchitectureInvariantCheck {
    let product_path = repo_root.join(".vac/registry/product.yaml");
    let status_path = repo_root.join(".vac/registry/status.yaml");

    let product = load_yaml_manifest::<ProductManifest>(&product_path);
    let status = load_yaml_manifest::<StatusManifest>(&status_path);

    match (product, status) {
        (Ok(product), Ok(status)) => {
            let mut details = vec![
                format!(
                    "product: kind={} id={} title={} command={} status={}",
                    product.kind, product.id, product.title, product.command, product.status
                ),
                format!(
                    "registry root: control_plane={} runtime={} launcher={}",
                    product.control_plane, product.runtime, product.launcher
                ),
                format!(
                    "status: kind={} id={} status={} notes={}",
                    status.kind,
                    status.id,
                    status.status,
                    status.notes.len()
                ),
            ];
            if product.schema_version != 1 {
                return ArchitectureInvariantCheck::fail(
                    "source-of-truth",
                    format!(
                        "product manifest schema version {} is unsupported",
                        product.schema_version
                    ),
                )
                .with_details(details);
            }
            if status.schema_version != 1 {
                details.push(format!(
                    "warning: status manifest schema version {} is unexpected",
                    status.schema_version
                ));
                return ArchitectureInvariantCheck::warn(
                    "source-of-truth",
                    "product manifest is present but status manifest schema version is unexpected",
                )
                .with_details(details);
            }

            let mut status_ok = true;
            if product.kind != "product" || product.id != "vac" || product.command != "vac" {
                status_ok = false;
            }
            if product.control_plane != ".vac" || product.runtime != "vac-rs" {
                status_ok = false;
            }
            if status.kind != "status" || status.id != "vac.migration" {
                status_ok = false;
            }
            if status.status != "ready" {
                details.push(format!(
                    "warning: status manifest reports `{}` instead of `ready`",
                    status.status
                ));
            }

            if status_ok {
                ArchitectureInvariantCheck::pass(
                    "source-of-truth",
                    "product registry and migration status manifests are present",
                )
                .with_details(details)
            } else {
                ArchitectureInvariantCheck::fail(
                    "source-of-truth",
                    "product registry manifest or status manifest is inconsistent",
                )
                .with_details(details)
            }
        }
        (product, status) => {
            let mut details = Vec::new();
            if let Err(error) = product {
                details.push(format!(
                    "{}: unable to read product manifest: {error}",
                    product_path.display()
                ));
            }
            if let Err(error) = status {
                details.push(format!(
                    "{}: unable to read status manifest: {error}",
                    status_path.display()
                ));
            }
            ArchitectureInvariantCheck::fail(
                "source-of-truth",
                "product registry and status manifests must exist",
            )
            .with_details(details)
        }
    }
}

fn registry_capability_ids(registry: &super::registry::ControlPlaneRegistry) -> BTreeSet<String> {
    registry
        .capabilities
        .manifests
        .iter()
        .map(|entry| entry.manifest.id.clone())
        .collect()
}

fn find_capability<'a>(
    registry: &'a super::registry::ControlPlaneRegistry,
    id: &str,
) -> Option<&'a CapabilityManifest> {
    registry
        .capabilities
        .manifests
        .iter()
        .find(|entry| entry.manifest.id == id)
        .map(|entry| &entry.manifest)
}

fn find_workflow<'a>(
    registry: &'a super::registry::ControlPlaneRegistry,
    id: &str,
) -> Option<&'a super::workflow_manifest::WorkflowManifest> {
    registry
        .workflows
        .manifests
        .iter()
        .find(|entry| entry.manifest.id == id)
        .map(|entry| &entry.manifest)
}

fn check_registry_inventory(registry: &RegistryLoadReport) -> ArchitectureInvariantCheck {
    let Some(control_plane) = registry.registry() else {
        return ArchitectureInvariantCheck::fail(
            "registry-inventory",
            "control-plane registry did not load",
        )
        .with_details(registry.render_lines());
    };

    let required_ids = [
        "vac.approvals",
        "vac.architecture",
        "vac.build",
        "vac.chat",
        "vac.identity",
        "vac.identity.check",
        "vac.ownership",
        "vac.release",
        "vac.sandbox",
        "vac.sessions",
        "vac.tools",
        "vac.tui",
        "vac.workflow",
    ];
    let ids = registry_capability_ids(control_plane);
    let missing = required_ids
        .iter()
        .filter(|id| !ids.contains(**id))
        .copied()
        .collect::<Vec<_>>();
    let mut details = vec![format!(
        "capabilities: loaded={} required={}",
        ids.len(),
        required_ids.len()
    )];
    if missing.is_empty() {
        ArchitectureInvariantCheck::pass(
            "registry-inventory",
            "required root capability manifests are present",
        )
        .with_details(details)
    } else {
        details.push(format!(
            "missing capability manifests: {}",
            missing.join(", ")
        ));
        ArchitectureInvariantCheck::fail(
            "registry-inventory",
            "one or more required root capability manifests are missing",
        )
        .with_details(details)
    }
}

fn check_tui_observability(registry: &RegistryLoadReport) -> ArchitectureInvariantCheck {
    let Some(control_plane) = registry.registry() else {
        return ArchitectureInvariantCheck::fail(
            "tui-observability",
            "control-plane registry did not load",
        );
    };

    let Some(tui_capability) = find_capability(control_plane, "vac.tui") else {
        return ArchitectureInvariantCheck::fail(
            "tui-observability",
            "vac.tui capability manifest is missing",
        );
    };

    let mut details = vec![format!("tui capability: {}", tui_capability.title)];
    let mut missing_routes = Vec::new();
    if let Some(tui_surface) = tui_capability.surfaces.tui.as_ref() {
        for route in ["/", "/capabilities", "/workflow", "/debug-config"] {
            if !tui_surface.routes.iter().any(|value| value == route) {
                missing_routes.push(route);
            }
        }
    } else {
        missing_routes.extend(["/", "/capabilities", "/workflow", "/debug-config"]);
    }

    let mut missing_surface_rows = Vec::new();
    let surface_ids = control_plane
        .surfaces
        .manifests
        .iter()
        .map(|entry| entry.manifest.id.as_str())
        .collect::<BTreeSet<_>>();
    for surface_id in ["surface.slash", "surface.palette"] {
        if !surface_ids.contains(surface_id) {
            missing_surface_rows.push(surface_id);
        }
    }

    if missing_routes.is_empty() && missing_surface_rows.is_empty() {
        ArchitectureInvariantCheck::pass(
            "tui-observability",
            "capabilities and workflows are visible in the root TUI surface catalog",
        )
        .with_details(details)
    } else {
        if !missing_routes.is_empty() {
            details.push(format!(
                "missing TUI routes in vac.tui manifest: {}",
                missing_routes.join(", ")
            ));
        }
        if !missing_surface_rows.is_empty() {
            details.push(format!(
                "missing surface registry rows: {}",
                missing_surface_rows.join(", ")
            ));
        }
        ArchitectureInvariantCheck::fail(
            "tui-observability",
            "root TUI does not observe all required capability and workflow surfaces",
        )
        .with_details(details)
    }
}

fn check_policy_gates(registry: &RegistryLoadReport) -> ArchitectureInvariantCheck {
    let Some(control_plane) = registry.registry() else {
        return ArchitectureInvariantCheck::fail(
            "policy-gates",
            "control-plane registry did not load",
        );
    };
    let Some(release) = find_capability(control_plane, "vac.release") else {
        return ArchitectureInvariantCheck::fail(
            "policy-gates",
            "vac.release capability manifest is missing",
        );
    };

    let mut details = vec![format!("release capability: {}", release.title)];
    let policy_registry_count = control_plane.policies.manifests.len();
    details.push(format!("policy registry: loaded={policy_registry_count}"));
    if policy_registry_count == 0 {
        return ArchitectureInvariantCheck::fail(
            "policy-gates",
            "policy registry must contain at least one baseline policy manifest",
        )
        .with_details(details);
    }

    if release.policy.approval_required_for.is_empty() {
        details
            .push("release capability policy does not require any approval decisions".to_string());
        return ArchitectureInvariantCheck::fail(
            "policy-gates",
            "release capability policy must require approval for execute_process",
        )
        .with_details(details);
    }
    if release.validation.commands.is_empty() {
        details.push("release capability lacks validation commands".to_string());
    }
    details.push(format!(
        "release approval_required_for: {}",
        release.policy.approval_required_for.join(", ")
    ));
    if release.validation.commands.is_empty() {
        ArchitectureInvariantCheck::warn(
            "policy-gates",
            "release capability carries approval policy but lacks validation commands",
        )
        .with_details(details)
    } else {
        ArchitectureInvariantCheck::pass(
            "policy-gates",
            "release capability carries approval and validation policy and the policy registry is populated",
        )
        .with_details(details)
    }
}

fn check_workflow_execution(registry: &RegistryLoadReport) -> ArchitectureInvariantCheck {
    let Some(control_plane) = registry.registry() else {
        return ArchitectureInvariantCheck::fail(
            "rust-executes",
            "control-plane registry did not load",
        );
    };
    let Some(release_workflow) = find_workflow(control_plane, "maintenance.release-gate") else {
        return ArchitectureInvariantCheck::fail(
            "rust-executes",
            "release-gate workflow manifest is missing",
        );
    };

    let mut details = vec![format!("release gate workflow: {}", release_workflow.title)];
    let release_preview = preview_workflow_manifest(release_workflow);

    let architecture_step_supported = release_workflow.steps.iter().any(|step| {
        step.uses == "capability.architecture.invariants"
            && resolve_workflow_step_handler(&step.uses).is_some()
    });
    let architecture_step_present = release_workflow
        .steps
        .iter()
        .any(|step| step.uses == "capability.architecture.invariants");

    details.push(format!(
        "step preview: supported={} blocked={}",
        release_preview.supported,
        release_preview.blocked_step_count()
    ));

    if architecture_step_present && architecture_step_supported && release_preview.supported {
        ArchitectureInvariantCheck::pass(
            "rust-executes",
            "release gate workflow resolves the architecture invariants step",
        )
        .with_details(details)
    } else {
        if !architecture_step_present {
            details.push(
                "missing capability.architecture.invariants step in maintenance.release-gate"
                    .to_string(),
            );
        } else if !architecture_step_supported {
            details.push(
                "capability.architecture.invariants step is not mapped to a workflow handler"
                    .to_string(),
            );
        }
        ArchitectureInvariantCheck::fail(
            "rust-executes",
            "release gate workflow does not resolve the architecture invariants step",
        )
        .with_details(details)
    }
}

fn check_approval_protects(registry: &RegistryLoadReport) -> ArchitectureInvariantCheck {
    let Some(control_plane) = registry.registry() else {
        return ArchitectureInvariantCheck::fail(
            "approval-protects",
            "control-plane registry did not load",
        );
    };
    let Some(release_workflow) = find_workflow(control_plane, "maintenance.release-gate") else {
        return ArchitectureInvariantCheck::fail(
            "approval-protects",
            "release-gate workflow manifest is missing",
        );
    };

    let approval_policy = evaluate_workflow_approval_policy(release_workflow, control_plane);
    let has_approval_step = release_workflow
        .steps
        .iter()
        .any(|step| step.uses == "capability.approval.request");
    let mut details = vec![format!(
        "approval policy: capability registry=loaded {} allowed={} approval_required={} blocked={}",
        approval_policy.capability_registry_manifest_count(),
        approval_policy.allowed_step_count(),
        approval_policy.approval_required_step_count(),
        approval_policy.blocked_step_count()
    )];

    if has_approval_step && approval_policy.approval_required_step_count() > 0 {
        ArchitectureInvariantCheck::pass(
            "approval-protects",
            "release gate workflow requires approval before execution can complete",
        )
        .with_details(details)
    } else {
        if !has_approval_step {
            details.push(
                "missing capability.approval.request step in maintenance.release-gate".to_string(),
            );
        }
        if approval_policy.approval_required_step_count() == 0 {
            details.push("approval policy does not require any approval decisions".to_string());
        }
        ArchitectureInvariantCheck::fail(
            "approval-protects",
            "release gate workflow can complete without approval protection",
        )
        .with_details(details)
    }
}

fn check_ownership_coverage(ownership: &OwnershipScanReport) -> ArchitectureInvariantCheck {
    let mut details = vec![format!(
        "ownership matrix: total={} owned={} unowned={} ready_unowned={}",
        ownership.entries().len(),
        ownership.owned_count(),
        ownership.unowned_count(),
        ownership.ready_unowned_count()
    )];

    if ownership.ready_unowned_count() > 0 {
        details.push("ready capability missing ownership metadata".to_string());
        for entry in ownership.entries().iter().filter(|entry| {
            entry.ownership.is_none() && matches!(entry.status, CapabilityStatus::Ready)
        }) {
            details.push(format!("  {}: {}", entry.id, entry.path));
        }
        ArchitectureInvariantCheck::fail(
            "dead-code-quarantine",
            "ready capabilities are missing ownership metadata",
        )
        .with_details(details)
    } else if ownership.unowned_count() > 0 {
        for entry in ownership
            .entries()
            .iter()
            .filter(|entry| entry.ownership.is_none())
        {
            details.push(format!("  unowned: {} ({})", entry.id, entry.path));
        }
        ArchitectureInvariantCheck::warn(
            "dead-code-quarantine",
            "some capabilities still lack ownership metadata",
        )
        .with_details(details)
    } else {
        ArchitectureInvariantCheck::pass(
            "dead-code-quarantine",
            "all loaded capabilities have ownership metadata",
        )
        .with_details(details)
    }
}

fn parse_tui_dependency_manifest(path: &Path) -> Result<BTreeSet<String>, String> {
    let contents = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let value: toml::Value = toml::from_str(&contents).map_err(|err| err.to_string())?;
    let mut dependencies = BTreeSet::new();
    collect_dependency_names(
        value
            .as_table()
            .ok_or_else(|| "root TOML is not a table".to_string())?,
        &mut dependencies,
    );
    Ok(dependencies)
}

fn collect_dependency_names(
    table: &toml::map::Map<String, toml::Value>,
    dependencies: &mut BTreeSet<String>,
) {
    for (key, value) in table {
        if matches!(
            key.as_str(),
            "dependencies" | "dev-dependencies" | "build-dependencies"
        ) {
            if let Some(dep_table) = value.as_table() {
                dependencies.extend(dep_table.keys().cloned());
            }
        }
        if let Some(child) = value.as_table() {
            collect_dependency_names(child, dependencies);
        }
        if let Some(array) = value.as_array() {
            for entry in array {
                if let Some(child) = entry.as_table() {
                    collect_dependency_names(child, dependencies);
                }
            }
        }
    }
}

fn compatibility_transport_entries(
    registry: &super::registry::ControlPlaneRegistry,
) -> BTreeMap<String, CapabilityCompatibilityTransport> {
    find_capability(registry, "vac.tui")
        .map(|manifest| {
            manifest
                .compatibility_transport
                .iter()
                .cloned()
                .map(|entry| (entry.dependency.clone(), entry))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default()
}

fn check_legacy_transport_quarantine(
    repo_root: &Path,
    registry: &RegistryLoadReport,
) -> ArchitectureInvariantCheck {
    let Some(control_plane) = registry.registry() else {
        return ArchitectureInvariantCheck::fail(
            "legacy-transport",
            "control-plane registry did not load",
        );
    };

    let tui_cargo_toml = repo_root.join("vac-rs/tui/Cargo.toml");
    let dependencies = match parse_tui_dependency_manifest(&tui_cargo_toml) {
        Ok(dependencies) => dependencies,
        Err(error) => {
            return ArchitectureInvariantCheck::fail(
                "legacy-transport",
                format!("unable to read {}", tui_cargo_toml.display()),
            )
            .with_details(vec![error]);
        }
    };

    let direct_legacy_deps = [
        "vac-app-server",
        "vac-app-server-client",
        "vac-app-server-protocol",
    ]
    .into_iter()
    .filter(|dependency| dependencies.contains(*dependency))
    .collect::<Vec<_>>();

    let compatibility_entries = compatibility_transport_entries(control_plane);
    let mut details = vec![format!(
        "compatibility manifest entries: {}",
        if compatibility_entries.is_empty() {
            "<none>".to_string()
        } else {
            compatibility_entries
                .values()
                .map(|entry| {
                    format!(
                        "{} [{}] owner={} replacement={} target_removal_plan={}",
                        entry.dependency,
                        entry.status,
                        entry.owner,
                        entry.replacement,
                        entry.target_removal_plan
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        }
    )];

    if direct_legacy_deps.is_empty() {
        return ArchitectureInvariantCheck::pass(
            "legacy-transport",
            "TUI Cargo manifest has no direct app-server transport dependency",
        )
        .with_details(details);
    }

    let missing = direct_legacy_deps
        .iter()
        .filter(|dependency| {
            !compatibility_entries
                .values()
                .any(|entry| entry.dependency == **dependency)
        })
        .copied()
        .collect::<Vec<_>>();

    for dependency in direct_legacy_deps.iter().copied() {
        if let Some(entry) = compatibility_entries
            .values()
            .find(|entry| entry.dependency == dependency)
        {
            details.push(format!(
                "classified legacy transport: {} [{}] owner={} deletion_condition={} replacement={} target_removal_plan={}",
                entry.dependency,
                entry.status,
                entry.owner,
                entry.deletion_condition,
                entry.replacement,
                entry.target_removal_plan
            ));
        } else {
            details.push(format!("unclassified legacy transport: {dependency}"));
        }
    }

    if missing.is_empty() {
        ArchitectureInvariantCheck::warn(
            "legacy-transport",
            "TUI still depends on app-server crates but they are explicitly classified",
        )
        .with_details(details)
    } else {
        details.push(format!(
            "missing compatibility classification for: {}",
            missing.join(", ")
        ));
        ArchitectureInvariantCheck::fail(
            "legacy-transport",
            "TUI Cargo manifest still has unclassified app-server transport dependencies",
        )
        .with_details(details)
    }
}

fn check_tui_direct_app_server_dependency(repo_root: &Path) -> ArchitectureInvariantCheck {
    let tui_cargo_toml = repo_root.join("vac-rs/tui/Cargo.toml");
    let dependencies = match parse_tui_dependency_manifest(&tui_cargo_toml) {
        Ok(dependencies) => dependencies,
        Err(error) => {
            return ArchitectureInvariantCheck::fail(
                "tui-direct-app-server-dep",
                format!("unable to read {}", tui_cargo_toml.display()),
            )
            .with_details(vec![error]);
        }
    };

    let direct_legacy_deps = [
        "vac-app-server",
        "vac-app-server-client",
        "vac-app-server-protocol",
    ]
    .into_iter()
    .filter(|dependency| dependencies.contains(*dependency))
    .collect::<Vec<_>>();

    let details = vec![format!(
        "direct legacy dependencies: {}",
        if direct_legacy_deps.is_empty() {
            "none".to_string()
        } else {
            direct_legacy_deps.join(", ")
        }
    )];

    if direct_legacy_deps.is_empty() {
        ArchitectureInvariantCheck::pass(
            "tui-direct-app-server-dep",
            "TUI Cargo manifest has no direct app-server transport dependency",
        )
        .with_details(details)
    } else {
        ArchitectureInvariantCheck::warn(
            "tui-direct-app-server-dep",
            "TUI Cargo manifest still has direct app-server transport dependencies",
        )
        .with_details(details)
    }
}

fn check_docs_alignment(docs: &DocsMaintenanceReport) -> ArchitectureInvariantCheck {
    let mut details = vec![format!(
        "docs maintenance: docs_checked={} links_checked={} manifest_dirs_present={}/{}",
        docs.docs_checked(),
        docs.links_checked(),
        docs.manifest_dirs_present(),
        docs.manifest_dirs_checked()
    )];
    if docs.is_failure() {
        details.push("docs control-plane index or manifest directories are stale".to_string());
        ArchitectureInvariantCheck::fail(
            "docs-alignment",
            "control-plane docs are not aligned with manifest layout",
        )
        .with_details(details)
    } else {
        ArchitectureInvariantCheck::pass(
            "docs-alignment",
            "control-plane docs and manifest layout are aligned",
        )
        .with_details(details)
    }
}

impl fmt::Display for ArchitectureInvariantReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render_text())
    }
}

#[cfg(test)]
mod tests {
    use super::load_architecture_invariant_report;
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    fn make_temp_repo() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vac-architecture-{unique}"));
        fs::create_dir_all(root.join(".vac/registry")).expect("registry dir");
        fs::create_dir_all(root.join(".vac/capabilities")).expect("capabilities dir");
        fs::create_dir_all(root.join(".vac/workflows")).expect("workflows dir");
        fs::create_dir_all(root.join(".vac/policies")).expect("policies dir");
        fs::create_dir_all(root.join(".vac/surfaces")).expect("surfaces dir");
        fs::create_dir_all(root.join("vac-rs/tui")).expect("tui dir");
        fs::create_dir_all(root.join("docs/workflow-control-plane/plans")).expect("plans dir");
        fs::create_dir_all(root.join("docs/workflow-control-plane/schema")).expect("schema dir");
        root
    }

    fn write_root_manifests(root: &Path, include_compatibility: bool) {
        fs::write(
            root.join(".vac/registry/product.yaml"),
            r#"
schema_version: 1
kind: product
id: vac
title: VAC
command: vac
status: ready
control_plane: .vac
runtime: vac-rs
launcher: vac-cli/bin/vac.js
"#,
        )
        .expect("product manifest");
        fs::write(
            root.join(".vac/registry/status.yaml"),
            r#"
schema_version: 1
kind: status
id: vac.migration
status: ready
notes:
  - Root workflow-control-plane migration is ready for the default local product path.
"#,
        )
        .expect("status manifest");

        let mut tui_compatibility = String::new();
        if include_compatibility {
            tui_compatibility.push_str(
                r#"
compatibility_transport:
  - dependency: vac-app-server
    status: compatibility_transport
    owner: vac-core::control_plane
    deletion_condition: remove when local runtime adapter no longer needs the legacy transport
    replacement: vac-rs/tui/src/local_runtime_session.rs
    target_removal_plan: Plan 00F
  - dependency: vac-app-server-client
    status: compatibility_transport
    owner: vac-core::control_plane
    deletion_condition: remove when TUI no longer needs the client bridge
    replacement: vac-rs/tui/src/local_runtime_session.rs
    target_removal_plan: Plan 00F
  - dependency: vac-app-server-protocol
    status: compatibility_transport
    owner: vac-core::control_plane
    deletion_condition: remove when session DTOs are fully owned by vac-protocol
    replacement: vac-rs/tui/src/session_protocol.rs
    target_removal_plan: Plan 00F
"#,
            );
        }

        let capability_template = |id: &str, title: &str, owner_crate: &str, owner_module: &str| {
            format!(
                r#"
schema_version: 1
kind: capability
id: {id}
title: {title}
status: ready
states:
  - empty
owner:
  crate: {owner_crate}
  module: {owner_module}
ownership:
  crates:
    - {owner_crate}
  modules:
    - {owner_module}
surfaces:
  palette: true
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands:
    - cargo +1.93.0 check -p vac-surface-cli
{tui_compatibility}
"#,
                id = id,
                title = title,
                owner_crate = owner_crate,
                owner_module = owner_module,
                tui_compatibility = tui_compatibility
            )
        };

        for (id, title, owner_crate, owner_module) in [
            (
                "vac.approvals",
                "Approvals",
                "vac-tui",
                "bottom_pane.approval_overlay",
            ),
            (
                "vac.architecture",
                "Architecture",
                "vac-core",
                "control_plane",
            ),
            ("vac.build", "Build", "vac-cli", "main"),
            ("vac.chat", "Chat", "vac-tui", "chatwidget"),
            ("vac.identity", "Identity", "vac-core", "control_plane"),
            (
                "vac.identity.check",
                "Identity Check",
                "vac-core",
                "control_plane",
            ),
            ("vac.ownership", "Ownership", "vac-core", "control_plane"),
            ("vac.release", "Release", "vac-cli", "doctor_cli"),
            ("vac.sandbox", "Sandbox", "vac-core", "sandboxing"),
            ("vac.sessions", "Sessions", "vac-core", "session"),
            ("vac.tools", "Tools", "vac-core", "tools"),
            ("vac.tui", "TUI", "vac-tui", "lib"),
            ("vac.workflow", "Workflow", "vac-core", "control_plane"),
        ] {
            let file_name = id.trim_start_matches("vac.").replace('.', "-");
            fs::write(
                root.join(format!(".vac/capabilities/{file_name}.yaml")),
                capability_template(id, title, owner_crate, owner_module),
            )
            .expect("capability manifest");
        }

        fs::write(
            root.join(".vac/workflows/maintenance.release-gate.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.release-gate
title: Release gate
status: planned
inputs: {}
ui:
  surface: /workflow
  inspect_surface: /debug-config
  progress_panel: true
  activity_log: true
  approval_surface: true
  evidence_surface: true
policy:
  default_risk: safe_read
  approval_required_for:
    - execute_process
steps:
  - id: orient
    uses: capability.approval.request
  - id: validate
    uses: capability.architecture.invariants
  - id: finalize
    uses: capability.tui.pty_gate
  - id: report
    uses: capability.activity.emit
validation:
  commands:
    - cargo +1.93.0 run -p vac-surface-cli -- doctor registry .
"#,
        )
        .expect("release gate workflow");

        fs::write(
            root.join(".vac/policies/default-local.yaml"),
            r#"
schema_version: 1
kind: policy
id: vac.default-local
title: Default local policy
default_decision: approval_required
rules:
  - id: read-project
    match:
      action: filesystem_read
      path: project
    decision: allow
    reason: Read-only project inspection is allowed.
  - id: write-project
    match:
      action: filesystem_write
      path: project
    decision: approval_required
    reason: File writes require operator approval.
"#,
        )
        .expect("default local policy manifest");

        fs::write(
            root.join(".vac/surfaces/slash.yaml"),
            r#"
schema_version: 1
kind: surface
id: surface.slash
title: Slash surface
capabilities:
  - vac.workflow
routes:
  - kind: slash
    command: /workflow
    capability: vac.workflow
    owner: "vac-tui::workflow_browser"
    visible: true
    status: ready
"#,
        )
        .expect("slash surface manifest");
        fs::write(
            root.join(".vac/surfaces/palette.yaml"),
            r#"
schema_version: 1
kind: surface
id: surface.palette
title: Palette surface
capabilities:
  - vac.workflow
routes:
  - kind: palette
    action: open_workflow
    capability: vac.workflow
    owner: "vac-tui::workflow_browser"
    visible: true
    status: ready
"#,
        )
        .expect("palette surface manifest");

        fs::write(
            root.join("vac-rs/tui/Cargo.toml"),
            r#"
[package]
name = "vac-tui"
version = "0.0.0"

[dependencies]
vac-app-server = { workspace = true }
vac-app-server-client = { workspace = true }
vac-app-server-protocol = { workspace = true }
"#,
        )
        .expect("tui cargo");

        fs::write(
            root.join("README.md"),
            "[Workflow control plane](docs/workflow-control-plane/INDEX.md)\n",
        )
        .expect("readme");
        fs::write(
            root.join("AGENTS.md"),
            "[Workflow control plane](docs/workflow-control-plane/INDEX.md)\n",
        )
        .expect("agents");
        fs::write(
            root.join("docs/workflow-control-plane/INDEX.md"),
            "[Plans](plans/INDEX.md)\n[Schema](schema/INDEX.md)\n[Implementation](IMPLEMENTATION_PLAN.md)\n",
        )
        .expect("docs index");
        fs::write(
            root.join("docs/workflow-control-plane/IMPLEMENTATION_PLAN.md"),
            "# Plan\n",
        )
        .expect("implementation plan");
        fs::write(
            root.join("docs/workflow-control-plane/plans/INDEX.md"),
            "[Plan 22](22-enterprise-hygiene.md)\n",
        )
        .expect("plan index");
        fs::write(
            root.join("docs/workflow-control-plane/plans/22-enterprise-hygiene.md"),
            "# Plan 22\n",
        )
        .expect("plan 22");
        fs::write(
            root.join("docs/workflow-control-plane/schema/INDEX.md"),
            "[Capability](capability-manifest.schema.md)\n",
        )
        .expect("schema index");
        fs::write(
            root.join("docs/workflow-control-plane/schema/capability-manifest.schema.md"),
            "# schema\n",
        )
        .expect("schema doc");
    }

    #[test]
    fn architecture_report_flags_unclassified_legacy_transport() {
        let root = make_temp_repo();
        write_root_manifests(&root, false);
        let report = load_architecture_invariant_report(&root);
        let rendered = report.render_text();
        assert!(report.is_failure(), "rendered:\n{rendered}");
        assert!(
            rendered.contains("legacy-transport: fail"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("unclassified legacy transport: vac-app-server"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("tui-direct-app-server-dep: warn"),
            "rendered:\n{rendered}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn architecture_report_warns_on_classified_legacy_transport() {
        let root = make_temp_repo();
        write_root_manifests(&root, true);
        let report = load_architecture_invariant_report(&root);
        let rendered = report.render_text();
        assert!(
            rendered.contains("legacy-transport: warn"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("classified legacy transport: vac-app-server"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("tui-direct-app-server-dep: warn"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("approval-protects: pass"),
            "rendered:\n{rendered}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn architecture_report_passes_on_no_app_server_dependencies() {
        let root = make_temp_repo();
        write_root_manifests(&root, true);
        // Overwrite the Cargo.toml with no legacy dependencies
        fs::write(
            root.join("vac-rs/tui/Cargo.toml"),
            r#"
[package]
name = "vac-tui"
version = "0.0.0"

[dependencies]
tokio = "1.0"
"#,
        )
        .expect("overwrite tui cargo");

        let report = load_architecture_invariant_report(&root);
        let rendered = report.render_text();
        assert!(
            rendered.contains("legacy-transport: pass"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("tui-direct-app-server-dep: pass"),
            "rendered:\n{rendered}"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
