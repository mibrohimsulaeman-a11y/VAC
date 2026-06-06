use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::path::Path;
use std::path::PathBuf;

use super::capability_manifest::CapabilityManifest;
use super::capability_manifest::CapabilityOwner;
use super::capability_manifest::CapabilityPolicy;
use super::capability_manifest::CapabilityStatus;
use super::capability_manifest::CapabilitySurfaces;
use super::ownership_scan::build_ownership_scan_report_for_registry;
use super::registry::ControlPlaneRegistry;
use super::registry::RegistryLoadError;
use super::root_feature_catalog::ROOT_SEED_CAPABILITY_REQUIREMENTS;
use super::root_feature_catalog::ROOT_SEED_POLICY_IDS;
use super::root_feature_catalog::ROOT_SEED_SURFACE_IDS;
use super::root_feature_catalog::ROOT_SEED_WORKFLOW_IDS;
use super::root_feature_conversion::ConversionState;
use super::root_feature_conversion::build_root_feature_conversion_report;
use super::surface_manifest::SurfaceRouteKind;
use super::workflow_manifest::WorkflowStatus;
use super::workflow_manifest::workflow_step_use_resolves;
use super::workflow_runner::preview_workflow_manifest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryDiagnosticSeverity {
    Info,
    Warning,
    Error,
    Blocked,
}

impl fmt::Display for RegistryDiagnosticSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Blocked => "blocked",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryLoadState {
    Loading,
    Empty,
    Success,
    Failure,
}

impl fmt::Display for RegistryLoadState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Loading => "loading",
            Self::Empty => "empty",
            Self::Success => "success",
            Self::Failure => "failure",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryDiagnostic {
    severity: RegistryDiagnosticSeverity,
    file: PathBuf,
    field_path: Option<String>,
    message: String,
    hint: Option<String>,
}

impl RegistryDiagnostic {
    pub fn new(
        severity: RegistryDiagnosticSeverity,
        file: impl Into<PathBuf>,
        field_path: Option<impl Into<String>>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            file: file.into(),
            field_path: field_path.map(Into::into),
            message: message.into(),
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn severity(&self) -> RegistryDiagnosticSeverity {
        self.severity
    }

    pub fn file(&self) -> &Path {
        &self.file
    }

    pub fn field_path(&self) -> Option<&str> {
        self.field_path.as_deref()
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn hint(&self) -> Option<&str> {
        self.hint.as_deref()
    }

    pub fn from_error(error: &RegistryLoadError) -> Self {
        match error {
            RegistryLoadError::VacRootNotFound { path } => Self::new(
                RegistryDiagnosticSeverity::Warning,
                path,
                None::<String>,
                error.message(),
            )
            .with_hint("create a `.vac/registry/product.yaml` file to enable the control plane"),
            RegistryLoadError::Directory { path, .. } => Self::new(
                RegistryDiagnosticSeverity::Error,
                path,
                None::<String>,
                error.message(),
            )
            .with_hint("create the directory or remove the file at this path"),
            RegistryLoadError::DuplicateManifestId {
                path,
                id,
                previous_path,
            } => Self::new(
                RegistryDiagnosticSeverity::Blocked,
                path,
                None::<String>,
                format!(
                    "duplicate manifest id `{id}` (previously loaded from `{}`)",
                    previous_path.display()
                ),
            )
            .with_hint("rename one of the manifests so each `id` stays unique"),
            RegistryLoadError::Aggregate { path, .. } => Self::new(
                RegistryDiagnosticSeverity::Error,
                path,
                None::<String>,
                error.message(),
            )
            .with_hint("fix every reported registry error and rerun `vac doctor registry`"),
            RegistryLoadError::Manifest {
                path, field_path, ..
            } => Self::new(
                RegistryDiagnosticSeverity::Error,
                path,
                Some(field_path.as_str()),
                error.message(),
            )
            .with_hint("fix the reported field or YAML syntax and rerun `vac doctor registry`"),
        }
    }

    pub fn render_line(&self) -> String {
        match self.field_path() {
            Some(field_path) => {
                format!(
                    "{}: {}:{}: {}",
                    self.severity,
                    self.file.display(),
                    field_path,
                    self.message
                )
            }
            None => format!(
                "{}: {}: {}",
                self.severity,
                self.file.display(),
                self.message
            ),
        }
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = Vec::with_capacity(2);
        lines.push(self.render_line());
        if let Some(hint) = self.hint() {
            lines.push(format!("  hint: {hint}"));
        }
        lines
    }

    pub fn render_text(&self) -> String {
        self.render_lines().join("\n")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegistryLoadReport {
    state: RegistryLoadState,
    vac_root: Option<PathBuf>,
    registry: Option<ControlPlaneRegistry>,
    diagnostics: Vec<RegistryDiagnostic>,
}

impl RegistryLoadReport {
    pub fn loading() -> Self {
        Self {
            state: RegistryLoadState::Loading,
            vac_root: None,
            registry: None,
            diagnostics: Vec::new(),
        }
    }

    pub fn empty() -> Self {
        Self {
            state: RegistryLoadState::Empty,
            vac_root: None,
            registry: None,
            diagnostics: Vec::new(),
        }
    }

    pub fn success(registry: ControlPlaneRegistry) -> Self {
        let state = if registry.is_empty() {
            RegistryLoadState::Empty
        } else {
            RegistryLoadState::Success
        };
        let vac_root = Some(registry.vac_root.clone());
        let diagnostics = root_seed_coverage_diagnostics(&registry);
        Self {
            state,
            vac_root,
            registry: Some(registry),
            diagnostics,
        }
    }

    pub fn from_error(error: RegistryLoadError) -> Self {
        Self::from_error_at_root(None, error)
    }

    pub fn from_error_at_root(vac_root: Option<PathBuf>, error: RegistryLoadError) -> Self {
        let state = match &error {
            RegistryLoadError::VacRootNotFound { .. } => RegistryLoadState::Empty,
            RegistryLoadError::Directory { .. }
            | RegistryLoadError::DuplicateManifestId { .. }
            | RegistryLoadError::Aggregate { .. }
            | RegistryLoadError::Manifest { .. } => RegistryLoadState::Failure,
        };
        let diagnostics = diagnostics_from_error(&error);
        let vac_root = vac_root.or_else(|| match &error {
            RegistryLoadError::VacRootNotFound { .. } => None,
            _ => Some(error.path().to_path_buf()),
        });
        Self {
            state,
            vac_root,
            registry: None,
            diagnostics,
        }
    }

    pub fn state(&self) -> RegistryLoadState {
        self.state
    }

    pub fn vac_root(&self) -> Option<&Path> {
        self.vac_root.as_deref()
    }

    pub fn registry(&self) -> Option<&ControlPlaneRegistry> {
        self.registry.as_ref()
    }

    pub fn diagnostics(&self) -> &[RegistryDiagnostic] {
        &self.diagnostics
    }

    pub fn is_failure(&self) -> bool {
        matches!(self.state, RegistryLoadState::Failure)
            || self.diagnostics.iter().any(|diagnostic| {
                matches!(
                    diagnostic.severity(),
                    RegistryDiagnosticSeverity::Error | RegistryDiagnosticSeverity::Blocked
                )
            })
    }

    pub fn cli_exit_code(&self) -> i32 {
        if self.is_failure() { 1 } else { 0 }
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        let summary = match self.state {
            RegistryLoadState::Loading => "registry: loading".to_string(),
            RegistryLoadState::Empty => "registry: empty".to_string(),
            RegistryLoadState::Success => {
                let manifest_count = self
                    .registry
                    .as_ref()
                    .map(ControlPlaneRegistry::manifest_count)
                    .unwrap_or(0);
                format!("registry: loaded {manifest_count} manifests")
            }
            RegistryLoadState::Failure => "registry: failure".to_string(),
        };
        lines.push(summary);
        if let Some(registry) = self.registry.as_ref() {
            lines.extend(render_root_seed_coverage_lines(registry, &self.diagnostics));
        }
        for diagnostic in &self.diagnostics {
            lines.extend(diagnostic.render_lines());
        }
        lines
    }

    pub fn render_text(&self) -> String {
        self.render_lines().join("\n")
    }

    pub fn render_tui_lines(&self) -> Vec<String> {
        self.render_lines()
    }

    pub fn render_tui_text(&self) -> String {
        self.render_text()
    }
}

fn diagnostics_from_error(error: &RegistryLoadError) -> Vec<RegistryDiagnostic> {
    match error {
        RegistryLoadError::Aggregate { errors, .. } => errors
            .iter()
            .flat_map(diagnostics_from_error)
            .collect::<Vec<_>>(),
        error => vec![RegistryDiagnostic::from_error(error)],
    }
}

fn render_root_seed_coverage_lines(
    registry: &ControlPlaneRegistry,
    diagnostics: &[RegistryDiagnostic],
) -> Vec<String> {
    let covered_capabilities = ROOT_SEED_CAPABILITY_REQUIREMENTS
        .iter()
        .filter(|required| find_capability(registry, required.capability_id).is_some())
        .count();
    let covered_policies = ROOT_SEED_POLICY_IDS
        .iter()
        .filter(|policy_id| {
            registry
                .policies
                .manifests
                .iter()
                .any(|entry| entry.manifest.id == **policy_id)
        })
        .count();
    let covered_surfaces = ROOT_SEED_SURFACE_IDS
        .iter()
        .filter(|surface_id| {
            registry
                .surfaces
                .manifests
                .iter()
                .any(|entry| entry.manifest.id == **surface_id)
        })
        .count();
    let covered_workflows = ROOT_SEED_WORKFLOW_IDS
        .iter()
        .filter(|workflow_id| {
            registry
                .workflows
                .manifests
                .iter()
                .any(|entry| entry.manifest.id == **workflow_id)
        })
        .count();
    let blocking_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.severity(),
                RegistryDiagnosticSeverity::Error | RegistryDiagnosticSeverity::Blocked
            )
        })
        .count();
    let coverage_state = if blocking_diagnostics == 0 {
        "pass"
    } else {
        "blocked"
    };

    vec![format!(
        "root seed coverage: {coverage_state} capabilities={}/{} policies={}/{} surfaces={}/{} workflows={}/{} diagnostics={}",
        covered_capabilities,
        ROOT_SEED_CAPABILITY_REQUIREMENTS.len(),
        covered_policies,
        ROOT_SEED_POLICY_IDS.len(),
        covered_surfaces,
        ROOT_SEED_SURFACE_IDS.len(),
        covered_workflows,
        ROOT_SEED_WORKFLOW_IDS.len(),
        diagnostics.len()
    )]
}

fn root_seed_coverage_diagnostics(registry: &ControlPlaneRegistry) -> Vec<RegistryDiagnostic> {
    let mut diagnostics = Vec::new();

    for required in ROOT_SEED_CAPABILITY_REQUIREMENTS {
        match find_capability(registry, required.capability_id) {
            Some(entry) => diagnostics.extend(validate_root_seed_capability(
                registry,
                required.feature,
                required.capability_id,
                &entry.path,
                &entry.manifest,
            )),
            None => diagnostics.push(
                RegistryDiagnostic::new(
                    RegistryDiagnosticSeverity::Blocked,
                    capability_manifest_path(registry, required.capability_id),
                    Some(format!("root_seed.{}", required.feature)),
                    format!(
                        "missing root seed capability `{}` for feature `{}`",
                        required.capability_id, required.feature
                    ),
                )
                .with_hint("seed the canonical root capability before accepting donor-backed work"),
            ),
        }
    }

    if registry.policies.manifests.is_empty() {
        diagnostics.push(
            RegistryDiagnostic::new(
                RegistryDiagnosticSeverity::Blocked,
                registry.vac_root.join("policies"),
                Some("root_seed.policy"),
                "root seed policy registry is empty",
            )
            .with_hint("seed baseline policy manifests under `.vac/policies/`"),
        );
    }

    for policy_id in ROOT_SEED_POLICY_IDS {
        if !registry
            .policies
            .manifests
            .iter()
            .any(|entry| entry.manifest.id == *policy_id)
        {
            diagnostics.push(
                RegistryDiagnostic::new(
                    RegistryDiagnosticSeverity::Blocked,
                    policy_manifest_path(registry, policy_id),
                    Some(format!("root_seed.policy.{policy_id}")),
                    format!("missing root seed policy manifest `{policy_id}`"),
                )
                .with_hint("seed the canonical baseline policy set before donor-backed work"),
            );
        }
    }

    for surface_id in ROOT_SEED_SURFACE_IDS {
        if !registry
            .surfaces
            .manifests
            .iter()
            .any(|entry| entry.manifest.id == *surface_id)
        {
            diagnostics.push(
                RegistryDiagnostic::new(
                    RegistryDiagnosticSeverity::Blocked,
                    surface_manifest_path(registry, surface_id),
                    Some(format!("root_seed.surface.{surface_id}")),
                    format!("missing root seed surface manifest `{surface_id}`"),
                )
                .with_hint("seed TUI, slash, palette, and CLI surface manifests before donor work"),
            );
        }
    }

    for workflow_id in ROOT_SEED_WORKFLOW_IDS {
        if !registry
            .workflows
            .manifests
            .iter()
            .any(|entry| entry.manifest.id == *workflow_id)
        {
            diagnostics.push(
                RegistryDiagnostic::new(
                    RegistryDiagnosticSeverity::Blocked,
                    workflow_manifest_path(registry, workflow_id),
                    Some(format!("root_seed.workflow.{workflow_id}")),
                    format!("missing root maintenance workflow `{workflow_id}`"),
                )
                .with_hint("seed required maintenance workflows before donor-backed work"),
            );
        }
    }

    diagnostics.extend(root_feature_conversion_diagnostics(registry));

    for entry in &registry.workflows.manifests {
        diagnostics.extend(validate_ready_workflow_runner_support(
            &entry.path,
            &entry.manifest,
            registry,
        ));
    }

    diagnostics.extend(cross_surface_duplicate_route_diagnostics(registry));
    diagnostics.extend(surface_capability_drift_diagnostics(registry));

    diagnostics
}

fn cross_surface_duplicate_route_diagnostics(
    registry: &ControlPlaneRegistry,
) -> Vec<RegistryDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen_routes: HashMap<String, (PathBuf, usize, String)> = HashMap::new();

    let mut sorted_surfaces: Vec<_> = registry.surfaces.manifests.iter().collect();
    sorted_surfaces.sort_by_key(|s| &s.path);

    for surface in sorted_surfaces {
        for (route_index, route) in surface.manifest.routes.iter().enumerate() {
            let target = route_target(route).to_string();
            let key = format!("{}:{target}", surface_route_kind_label(route.kind));
            if let Some((first_path, first_index, first_capability)) = seen_routes.get(&key) {
                diagnostics.push(
                    RegistryDiagnostic::new(
                        RegistryDiagnosticSeverity::Blocked,
                        &surface.path,
                        Some(format!("routes[{route_index}]")),
                        format!(
                            "duplicate {} surface route `{target}` for `{}` also appears in {}:routes[{first_index}] for `{first_capability}`",
                            surface_route_kind_label(route.kind),
                            route.capability,
                            first_path.display()
                        ),
                    )
                    .with_hint(
                        "keep each surface route target unique across `.vac/surfaces/` or retire the stale duplicate route",
                    ),
                );
                continue;
            }
            seen_routes.insert(
                key,
                (surface.path.clone(), route_index, route.capability.clone()),
            );
        }
    }

    diagnostics
}

fn surface_capability_drift_diagnostics(
    registry: &ControlPlaneRegistry,
) -> Vec<RegistryDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut capability_owners: HashMap<String, (String, PathBuf, usize)> = HashMap::new();

    let mut sorted_surfaces: Vec<_> = registry.surfaces.manifests.iter().collect();
    sorted_surfaces.sort_by_key(|s| &s.path);

    for surface in sorted_surfaces {
        for (route_index, route) in surface.manifest.routes.iter().enumerate() {
            let Some(capability) = find_capability(registry, &route.capability) else {
                continue;
            };

            // P2 owner consistency lintas surface
            if let Some(route_owner) = route.owner.as_deref() {
                if let Some((first_owner, first_path, first_index)) =
                    capability_owners.get(&route.capability)
                {
                    if first_owner != route_owner {
                        diagnostics.push(
                            RegistryDiagnostic::new(
                                RegistryDiagnosticSeverity::Warning,
                                &surface.path,
                                Some(format!("routes[{route_index}].owner")),
                                format!(
                                    "inconsistent owner `{route_owner}` for capability `{}` (previously declared as `{first_owner}` in {}:routes[{first_index}])",
                                    route.capability,
                                    first_path.display()
                                ),
                            )
                            .with_hint("keep the route owner consistent across all surfaces for the same capability"),
                        );
                    }
                } else {
                    capability_owners.insert(
                        route.capability.clone(),
                        (route_owner.to_string(), surface.path.clone(), route_index),
                    );
                }
            }

            if route.visible
                && !capability_declares_surface_route(
                    &capability.manifest.surfaces,
                    route.kind,
                    route_target(route),
                )
            {
                diagnostics.push(
                    RegistryDiagnostic::new(
                        RegistryDiagnosticSeverity::Warning,
                        &surface.path,
                        Some(format!("routes[{route_index}].capability")),
                        format!(
                            "visible surface route `{}` for `{}` is not declared by the capability manifest",
                            route_target(route), route.capability
                        ),
                    )
                    .with_hint("update the capability `surfaces` block or remove the stale route from the surface manifest"),
                );
            }

            if route.visible
                && let Some(route_owner) = route.owner.as_deref()
            {
                let capability_owner = capability_owner_label(&capability.manifest.owner);
                if !route_owner_matches_capability_owner(route_owner, &capability_owner) {
                    diagnostics.push(
                            RegistryDiagnostic::new(
                                RegistryDiagnosticSeverity::Warning,
                                &surface.path,
                                Some(format!("routes[{route_index}].owner")),
                                format!(
                                    "surface route owner `{route_owner}` differs from capability owner `{capability_owner}` for `{}`",
                                    route.capability
                                ),
                            )
                            .with_hint("keep route owner and capability owner aligned, or move implementation ownership into the capability manifest"),
                        );
                }
            }
        }
    }

    for capability in &registry.capabilities.manifests {
        for (kind, target) in declared_capability_surface_targets(&capability.manifest.surfaces) {
            if !surface_registry_declares_route(registry, &capability.manifest.id, kind, &target) {
                diagnostics.push(
                    RegistryDiagnostic::new(
                        RegistryDiagnosticSeverity::Warning,
                        &capability.path,
                        Some("surfaces"),
                        format!(
                            "capability `{}` declares {} surface `{target}` but no matching surface manifest route exists",
                            capability.manifest.id,
                            surface_route_kind_label(kind)
                        ),
                    )
                    .with_hint("add the route to `.vac/surfaces/` or remove the stale capability surface declaration"),
                );
            }
        }

        // P1 palette: true ambiguity
        if capability.manifest.surfaces.palette {
            let has_palette_route = registry.surfaces.manifests.iter().any(|surface| {
                surface.manifest.routes.iter().any(|route| {
                    route.capability == capability.manifest.id
                        && route.kind == SurfaceRouteKind::Palette
                })
            });
            if !has_palette_route {
                diagnostics.push(
                    RegistryDiagnostic::new(
                        RegistryDiagnosticSeverity::Warning,
                        &capability.path,
                        Some("surfaces.palette"),
                        format!(
                            "capability `{}` declares palette surface but no matching surface manifest route exists",
                            capability.manifest.id
                        ),
                    )
                    .with_hint("add a palette route to .vac/surfaces/ or set palette: false in the capability manifest"),
                );
            }
        }
    }

    diagnostics
}

fn normalize_slash_command(cmd: &str) -> &str {
    cmd.strip_prefix('/').unwrap_or(cmd)
}

fn capability_declares_surface_route(
    surfaces: &CapabilitySurfaces,
    kind: SurfaceRouteKind,
    target: &str,
) -> bool {
    match kind {
        SurfaceRouteKind::Tui => surfaces
            .tui
            .as_ref()
            .is_some_and(|routes| routes.routes.iter().any(|route| route == target)),
        SurfaceRouteKind::Slash => surfaces.slash.as_ref().is_some_and(|commands| {
            commands
                .commands
                .iter()
                .any(|command| normalize_slash_command(command) == normalize_slash_command(target))
        }),
        SurfaceRouteKind::Palette => surfaces.palette,
        SurfaceRouteKind::Cli => surfaces
            .cli
            .as_ref()
            .is_some_and(|cli| cli.enabled && cli.commands.iter().any(|command| command == target)),
        SurfaceRouteKind::Statusline => true,
    }
}

fn declared_capability_surface_targets(
    surfaces: &CapabilitySurfaces,
) -> Vec<(SurfaceRouteKind, String)> {
    let mut targets = Vec::new();
    if let Some(routes) = surfaces.tui.as_ref() {
        targets.extend(
            routes
                .routes
                .iter()
                .cloned()
                .map(|route| (SurfaceRouteKind::Tui, route)),
        );
    }
    if let Some(commands) = surfaces.slash.as_ref() {
        targets.extend(
            commands
                .commands
                .iter()
                .cloned()
                .map(|command| (SurfaceRouteKind::Slash, command)),
        );
    }
    if let Some(cli) = surfaces.cli.as_ref()
        && cli.enabled
    {
        targets.extend(
            cli.commands
                .iter()
                .cloned()
                .map(|command| (SurfaceRouteKind::Cli, command)),
        );
    }
    targets
}

fn surface_registry_declares_route(
    registry: &ControlPlaneRegistry,
    capability_id: &str,
    kind: SurfaceRouteKind,
    target: &str,
) -> bool {
    registry.surfaces.manifests.iter().any(|surface| {
        surface.manifest.routes.iter().any(|route| {
            if route.capability != capability_id || route.kind != kind {
                return false;
            }
            let route_tgt = route_target(route);
            if kind == SurfaceRouteKind::Slash {
                normalize_slash_command(route_tgt) == normalize_slash_command(target)
            } else {
                route_tgt == target
            }
        })
    })
}

fn route_target(route: &super::surface_manifest::SurfaceRoute) -> &str {
    route
        .path
        .as_deref()
        .or(route.command.as_deref())
        .or(route.action.as_deref())
        .or(route.label.as_deref())
        .unwrap_or("<missing>")
}

fn surface_route_kind_label(kind: SurfaceRouteKind) -> &'static str {
    match kind {
        SurfaceRouteKind::Tui => "tui",
        SurfaceRouteKind::Slash => "slash",
        SurfaceRouteKind::Palette => "palette",
        SurfaceRouteKind::Cli => "cli",
        SurfaceRouteKind::Statusline => "statusline",
    }
}

fn capability_owner_label(owner: &CapabilityOwner) -> String {
    match owner {
        CapabilityOwner::Path(path) => path.clone(),
        CapabilityOwner::Structured(object) => format!("{}/{}", object.crate_name, object.module),
    }
}

fn route_owner_matches_capability_owner(route_owner: &str, capability_owner: &str) -> bool {
    route_owner == capability_owner
        || route_owner.starts_with(capability_owner)
        || route_owner.contains(capability_owner)
}

fn root_feature_conversion_diagnostics(registry: &ControlPlaneRegistry) -> Vec<RegistryDiagnostic> {
    let ownership_scan = build_ownership_scan_report_for_registry(registry);
    let repo_root = registry.vac_root.parent().unwrap_or(&registry.vac_root);
    let source_domain_paths = ownership_scan
        .unowned_source_domains()
        .iter()
        .map(|source| {
            (
                format!("{}/{}", source.crate_name, source.module),
                repo_root.join("vac-rs").join(&source.source_path),
            )
        })
        .collect::<HashMap<_, _>>();
    let conversion_report = build_root_feature_conversion_report(registry, &ownership_scan);
    let mut diagnostics = Vec::new();

    for entry in conversion_report.entries() {
        match entry.state {
            ConversionState::Overclaimed => {
                let drift = root_feature_conversion_drift_summary(entry);
                diagnostics.push(
                    RegistryDiagnostic::new(
                        RegistryDiagnosticSeverity::Blocked,
                        capability_manifest_path(registry, &entry.capability_id),
                        Some(format!("root_feature_conversion.{}.ownership", entry.feature)),
                        format!(
                            "root feature `{}` has root conversion drift: {}",
                            entry.capability_id,
                            drift,
                        ),
                    )
                    .with_hint("update root capability ownership, surfaces, policy, validation, or root catalog expectations before marking conversion complete"),
                );
            }
            ConversionState::Hidden => {
                if entry.capability_id.starts_with("source:") {
                    let source_domain = entry.capability_id.trim_start_matches("source:");
                    let file = source_domain_paths
                        .get(source_domain)
                        .cloned()
                        .unwrap_or_else(|| source_domain_fallback_path(registry, source_domain));
                    diagnostics.push(
                        RegistryDiagnostic::new(
                            RegistryDiagnosticSeverity::Warning,
                            file,
                            Some(source_domain_field_path(source_domain)),
                            format!(
                                "source domain `{source_domain}` is not claimed by any capability ownership target"
                            ),
                        )
                        .with_hint("declare a capability ownership target for this source domain, or mark the owning target test_only/retired if intentional"),
                    );
                    continue;
                }
                if find_capability(registry, &entry.capability_id).is_some() {
                    diagnostics.push(
                        RegistryDiagnostic::new(
                            RegistryDiagnosticSeverity::Warning,
                            capability_manifest_path(registry, &entry.capability_id),
                            Some(format!(
                                "root_feature_conversion.{}.ownership",
                                entry.feature
                            )),
                            format!(
                                "capability `{}` is missing ownership metadata (hidden state)",
                                entry.capability_id
                            ),
                        )
                        .with_hint("declare ownership metadata in the capability manifest"),
                    );
                }
            }
            ConversionState::Partial => {
                if let Some(entry_manifest) = find_capability(registry, &entry.capability_id) {
                    let field_path = format!("root_feature_conversion.{}.reason", entry.feature);
                    let diagnostic = match entry_manifest.manifest.reason.as_deref() {
                        Some(reason) => RegistryDiagnostic::new(
                            RegistryDiagnosticSeverity::Info,
                            &entry_manifest.path,
                            Some(field_path),
                            format!(
                                "partial capability `{}` remains partial: {reason}",
                                entry.capability_id
                            ),
                        )
                        .with_hint("keep the reason current until the capability is ready"),
                        None => RegistryDiagnostic::new(
                            RegistryDiagnosticSeverity::Warning,
                            &entry_manifest.path,
                            Some(field_path),
                            format!(
                                "partial capability `{}` is missing an explicit reason",
                                entry.capability_id
                            ),
                        )
                        .with_hint("declare a `reason` in the capability manifest explaining why it is planned/partial"),
                    };
                    diagnostics.push(diagnostic);
                }
            }
            _ => {}
        }
    }

    diagnostics
}

fn root_feature_conversion_drift_summary(
    entry: &super::root_feature_conversion::RootFeatureConversionEntry,
) -> String {
    let mut parts = Vec::new();
    push_drift_part(
        &mut parts,
        "missing_source_modules",
        &entry.overclaimed_modules,
    );
    push_drift_part(&mut parts, "missing_crates", &entry.overclaimed_crates);
    push_drift_part(
        &mut parts,
        "missing_source_roots",
        &entry.missing_expected_source_roots,
    );
    push_drift_part(
        &mut parts,
        "missing_surfaces",
        &entry.missing_expected_surfaces,
    );
    push_drift_part(&mut parts, "missing_policy", &entry.missing_expected_policy);
    push_drift_part(
        &mut parts,
        "missing_validation",
        &entry.missing_expected_validation,
    );
    push_drift_part(
        &mut parts,
        "missing_ownership",
        &entry.missing_expected_ownership,
    );
    if parts.is_empty() {
        "no detailed drift recorded".to_string()
    } else {
        parts.join("; ")
    }
}

fn push_drift_part(parts: &mut Vec<String>, label: &str, values: &[String]) {
    if !values.is_empty() {
        parts.push(format!("{label}=[{}]", values.join(", ")));
    }
}

fn source_domain_field_path(source_domain: &str) -> String {
    format!(
        "root_feature_conversion.source.{}",
        source_domain.replace('/', ".")
    )
}

fn source_domain_fallback_path(registry: &ControlPlaneRegistry, source_domain: &str) -> PathBuf {
    registry
        .vac_root
        .join("source-domains")
        .join(format!("{}.rs", source_domain.replace('/', "-")))
}

fn validate_ready_workflow_runner_support(
    path: &Path,
    manifest: &super::workflow_manifest::WorkflowManifest,
    registry: &ControlPlaneRegistry,
) -> Vec<RegistryDiagnostic> {
    if !matches!(manifest.status, WorkflowStatus::Ready) {
        return Vec::new();
    }

    let preview = preview_workflow_manifest(manifest);
    if preview.supported {
        return Vec::new();
    }

    let known_capabilities = registry
        .capabilities
        .manifests
        .iter()
        .map(|entry| entry.manifest.id.clone())
        .collect::<HashSet<_>>();
    let blocked_steps = preview
        .blocked_steps
        .iter()
        .filter(|step| !workflow_step_use_resolves(&step.uses, &known_capabilities))
        .map(|step| format!("{}={}", step.id, step.uses))
        .collect::<Vec<_>>();
    if blocked_steps.is_empty() {
        return Vec::new();
    }
    let blocked_steps = blocked_steps.join(", ");

    vec![
        RegistryDiagnostic::new(
            RegistryDiagnosticSeverity::Blocked,
            path,
            Some(format!("workflow.{}.status", manifest.id)),
            format!(
                "ready workflow `{}` has unsupported runner steps: [{}]",
                manifest.id, blocked_steps
            ),
        )
        .with_hint("downgrade the workflow to planned/partial or add typed safe-runner handlers before marking it ready"),
    ]
}

fn validate_root_seed_capability(
    registry: &ControlPlaneRegistry,
    feature: &str,
    capability_id: &str,
    path: &Path,
    manifest: &CapabilityManifest,
) -> Vec<RegistryDiagnostic> {
    let mut diagnostics = Vec::new();

    if (matches!(manifest.status, CapabilityStatus::Planned)
        || matches!(manifest.status, CapabilityStatus::Partial))
        && manifest.reason.is_none()
    {
        diagnostics.push(
            RegistryDiagnostic::new(
                RegistryDiagnosticSeverity::Warning,
                path,
                Some(format!("root_seed.{feature}.reason")),
                format!(
                    "partial capability `{capability_id}` is missing an explicit reason",
                ),
            )
            .with_hint("declare a `reason` in the capability manifest explaining why it is planned/partial"),
        );
    }

    if manifest.donor_source.is_some() {
        diagnostics.push(
            RegistryDiagnostic::new(
                RegistryDiagnosticSeverity::Blocked,
                path,
                Some(format!("root_seed.{feature}.donor_source")),
                format!("root seed capability `{capability_id}` must not declare donor_source"),
            )
            .with_hint(
                "keep root seed manifests source-of-truth before donor-backed manifests are added",
            ),
        );
    }

    if manifest.ownership.is_none() {
        diagnostics.push(
            RegistryDiagnostic::new(
                RegistryDiagnosticSeverity::Warning,
                path,
                Some(format!("root_seed.{feature}.ownership")),
                format!("root seed capability `{capability_id}` is missing ownership metadata"),
            )
            .with_hint("declare owning crates or modules so root coverage can be gated"),
        );
    }

    if !capability_policy_has_metadata(&manifest.policy) {
        diagnostics.push(
            RegistryDiagnostic::new(
                RegistryDiagnosticSeverity::Blocked,
                path,
                Some(format!("root_seed.{feature}.policy")),
                format!("root seed capability `{capability_id}` is missing policy metadata"),
            )
            .with_hint("declare at least one risk, filesystem, network, redaction, or approval policy field"),
        );
    }

    if manifest.validation.commands.is_empty() && manifest.validation.gates.is_empty() {
        diagnostics.push(
            RegistryDiagnostic::new(
                RegistryDiagnosticSeverity::Blocked,
                path,
                Some(format!("root_seed.{feature}.validation")),
                format!("root seed capability `{capability_id}` is missing validation evidence"),
            )
            .with_hint("declare validation commands or gates for the root seed capability"),
        );
    }

    if matches!(manifest.status, CapabilityStatus::Ready)
        && (manifest.validation.commands.is_empty() && manifest.validation.gates.is_empty())
    {
        diagnostics.push(
            RegistryDiagnostic::new(
                RegistryDiagnosticSeverity::Blocked,
                path,
                Some(format!("root_seed.{feature}.status")),
                format!("ready root seed capability `{capability_id}` lacks validation evidence"),
            )
            .with_hint("downgrade to partial/planned or add executable validation"),
        );
    }

    if matches!(manifest.status, CapabilityStatus::Ready)
        && registry.surfaces.manifests.iter().all(|surface| {
            surface
                .manifest
                .routes
                .iter()
                .all(|route| route.capability != capability_id)
        })
    {
        diagnostics.push(
            RegistryDiagnostic::new(
                RegistryDiagnosticSeverity::Blocked,
                path,
                Some(format!("root_seed.{feature}.surface")),
                format!("ready root seed capability `{capability_id}` is not represented in any surface manifest"),
            )
            .with_hint("add a surface route or downgrade the capability until it is intentionally surfaced"),
        );
    }

    diagnostics
}

fn capability_policy_has_metadata(policy: &CapabilityPolicy) -> bool {
    policy.risk.is_some()
        || policy.mutates_files.is_some()
        || policy.network.is_some()
        || policy.redaction.is_some()
        || !policy.approval_required_for.is_empty()
}

fn find_capability<'a>(
    registry: &'a ControlPlaneRegistry,
    capability_id: &str,
) -> Option<&'a super::registry::LocatedManifest<CapabilityManifest>> {
    registry
        .capabilities
        .manifests
        .iter()
        .find(|entry| entry.manifest.id == capability_id)
}

fn capability_manifest_path(registry: &ControlPlaneRegistry, capability_id: &str) -> PathBuf {
    registry.vac_root.join("capabilities").join(format!(
        "{}.yaml",
        capability_id.trim_start_matches("vac.").replace('.', "-")
    ))
}

fn policy_manifest_path(registry: &ControlPlaneRegistry, policy_id: &str) -> PathBuf {
    let file_stem = match policy_id {
        "vac.default-local" => "default-local".to_string(),
        other => other.trim_start_matches("vac.policy.").replace('.', "-"),
    };
    registry
        .vac_root
        .join("policies")
        .join(format!("{file_stem}.yaml"))
}

fn surface_manifest_path(registry: &ControlPlaneRegistry, surface_id: &str) -> PathBuf {
    registry.vac_root.join("surfaces").join(format!(
        "{}.yaml",
        surface_id.trim_start_matches("surface.")
    ))
}

fn workflow_manifest_path(registry: &ControlPlaneRegistry, workflow_id: &str) -> PathBuf {
    registry.vac_root.join("workflows").join(format!(
        "{}.yaml",
        workflow_id.trim_start_matches("maintenance.")
    ))
}

#[cfg(test)]
#[path = "registry_diagnostics_tests.rs"]
mod tests;
