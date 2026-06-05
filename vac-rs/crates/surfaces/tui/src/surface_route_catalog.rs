use std::collections::HashMap;
use std::path::Path;
use vac_core::control_plane::load_control_plane_registry_report;
use vac_core::control_plane::surface_manifest::SurfaceRoute;
use vac_core::control_plane::surface_manifest::SurfaceRouteKind;
use vac_core::control_plane::surface_manifest::SurfaceRouteStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SurfaceRouteMetadata {
    pub(crate) capability: String,
    pub(crate) status: String,
    pub(crate) visible: bool,
    pub(crate) reason: Option<String>,
    pub(crate) owner: Option<String>,
}

pub(crate) type SlashRouteMetadata = SurfaceRouteMetadata;
pub(crate) type PaletteRouteMetadata = SurfaceRouteMetadata;
pub(crate) type CliRouteMetadata = SurfaceRouteMetadata;
pub(crate) type TuiRouteMetadata = SurfaceRouteMetadata;
pub(crate) type StatuslineRouteMetadata = SurfaceRouteMetadata;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SurfaceRouteCatalog {
    slash_routes: HashMap<String, SurfaceRouteMetadata>,
    palette_routes: HashMap<String, SurfaceRouteMetadata>,
    cli_routes: HashMap<String, SurfaceRouteMetadata>,
    tui_routes: HashMap<String, SurfaceRouteMetadata>,
    statusline_routes: HashMap<String, SurfaceRouteMetadata>,
}

impl SurfaceRouteCatalog {
    pub(crate) fn load(start: impl AsRef<Path>) -> Self {
        let report = load_control_plane_registry_report(start);
        if report.state() == vac_core::control_plane::RegistryLoadState::Failure {
            return Self::default();
        }
        let mut slash_routes = HashMap::new();
        let mut palette_routes = HashMap::new();
        let mut cli_routes = HashMap::new();
        let mut tui_routes = HashMap::new();
        let mut statusline_routes = HashMap::new();

        let Some(registry) = report.registry() else {
            return Self {
                slash_routes,
                palette_routes,
                cli_routes,
                tui_routes,
                statusline_routes,
            };
        };

        for surface in &registry.surfaces.manifests {
            for route in &surface.manifest.routes {
                let metadata = SurfaceRouteMetadata {
                    capability: route.capability.clone(),
                    status: format_surface_route_status(route.status),
                    visible: route.visible,
                    reason: route.reason.clone(),
                    owner: route.owner.clone(),
                };

                match route.kind {
                    SurfaceRouteKind::Slash => {
                        if let Some(command) = normalize_command(route) {
                            slash_routes
                                .entry(command)
                                .or_insert_with(|| metadata.clone());
                        }
                    }
                    SurfaceRouteKind::Palette => {
                        if let Some(action) = normalize_action(route) {
                            palette_routes
                                .entry(action)
                                .or_insert_with(|| metadata.clone());
                        }
                    }
                    SurfaceRouteKind::Cli => {
                        if let Some(command) = normalize_cli_command(route) {
                            cli_routes
                                .entry(command)
                                .or_insert_with(|| metadata.clone());
                        }
                    }
                    SurfaceRouteKind::Tui => {
                        if let Some(path) = normalize_path(route) {
                            tui_routes.entry(path).or_insert_with(|| metadata.clone());
                        }
                    }
                    SurfaceRouteKind::Statusline => {
                        if let Some(label) = normalize_label(route) {
                            statusline_routes
                                .entry(label)
                                .or_insert_with(|| metadata.clone());
                        }
                    }
                }
            }
        }

        Self {
            slash_routes,
            palette_routes,
            cli_routes,
            tui_routes,
            statusline_routes,
        }
    }

    pub(crate) fn slash_route(&self, command: &str) -> Option<&SlashRouteMetadata> {
        self.slash_routes.get(command)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn palette_route(&self, action: &str) -> Option<&PaletteRouteMetadata> {
        self.palette_routes.get(action)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn cli_route(&self, command: &str) -> Option<&CliRouteMetadata> {
        self.cli_routes.get(command)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn tui_route(&self, path: &str) -> Option<&TuiRouteMetadata> {
        self.tui_routes.get(path)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn statusline_route(&self, label: &str) -> Option<&StatuslineRouteMetadata> {
        self.statusline_routes.get(label)
    }

    pub(crate) fn slash_routes(&self) -> Vec<(String, SlashRouteMetadata)> {
        sorted_routes(&self.slash_routes)
    }

    pub(crate) fn palette_routes(&self) -> Vec<(String, PaletteRouteMetadata)> {
        sorted_routes(&self.palette_routes)
    }

    pub(crate) fn cli_routes(&self) -> Vec<(String, CliRouteMetadata)> {
        sorted_routes(&self.cli_routes)
    }

    pub(crate) fn tui_routes(&self) -> Vec<(String, TuiRouteMetadata)> {
        sorted_routes(&self.tui_routes)
    }

    pub(crate) fn statusline_routes(&self) -> Vec<(String, StatuslineRouteMetadata)> {
        sorted_routes(&self.statusline_routes)
    }
}

fn normalize_command(route: &SurfaceRoute) -> Option<String> {
    let command = route.command.as_deref()?;
    Some(command.strip_prefix('/').unwrap_or(command).to_string())
}

fn normalize_action(route: &SurfaceRoute) -> Option<String> {
    let action = route.action.as_deref()?;
    Some(action.strip_prefix('/').unwrap_or(action).to_string())
}

fn normalize_cli_command(route: &SurfaceRoute) -> Option<String> {
    let command = route.command.as_deref()?;
    Some(command.trim().to_string())
}

fn normalize_path(route: &SurfaceRoute) -> Option<String> {
    let path = route.path.as_deref()?;
    Some(path.trim().to_string())
}

fn normalize_label(route: &SurfaceRoute) -> Option<String> {
    let label = route.label.as_deref()?;
    Some(label.trim().to_string())
}

fn sorted_routes(
    routes: &HashMap<String, SurfaceRouteMetadata>,
) -> Vec<(String, SurfaceRouteMetadata)> {
    let mut routes = routes
        .iter()
        .map(|(name, metadata)| (name.clone(), metadata.clone()))
        .collect::<Vec<_>>();
    routes.sort_by(|left, right| left.0.cmp(&right.0));
    routes
}

fn format_surface_route_status(status: SurfaceRouteStatus) -> String {
    match status {
        SurfaceRouteStatus::Ready => "ready".to_string(),
        SurfaceRouteStatus::Partial => "partial".to_string(),
        SurfaceRouteStatus::Planned => "planned".to_string(),
        SurfaceRouteStatus::Unavailable => "unavailable".to_string(),
        SurfaceRouteStatus::CliOnly => "cli-only".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::SurfaceRouteCatalog;
    use std::fs;
    use tempfile::tempdir;
    use vac_core::control_plane::load_control_plane_registry_report;

    #[test]
    fn loads_slash_route_catalog() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join(".vac/surfaces")).expect("surfaces dir");
        fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
        fs::write(
            tempdir.path().join(".vac/capabilities/workflow.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: deprecated
owner:
  crate: vac-tui
  module: workflow
surfaces:
  palette: true
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands: []
"#,
        )
        .expect("capability manifest");
        fs::write(
            tempdir.path().join(".vac/surfaces/slash.yaml"),
            r#"
schema_version: 1
kind: surface
id: surface.slash
title: Slash surface
routes:
  - kind: slash
    command: /capabilities
    capability: vac.workflow
    owner: "vac-tui::capability_dashboard"
    visible: true
    status: ready
  - kind: slash
    command: /workflow
    capability: vac.workflow
    owner: "vac-tui::workflow_browser"
    visible: true
    status: partial
    reason: "Under development"
capabilities: [vac.workflow]
"#,
        )
        .expect("surface manifest");

        let catalog = SurfaceRouteCatalog::load(tempdir.path());
        let report = load_control_plane_registry_report(tempdir.path());
        assert!(report.registry().is_some(), "{}", report.render_tui_text());
        let capabilities = catalog
            .slash_route("capabilities")
            .expect("capabilities route");
        assert_eq!(capabilities.capability, "vac.workflow");
        assert_eq!(capabilities.status, "ready");
        assert!(capabilities.visible);
        assert_eq!(
            capabilities.owner.as_deref(),
            Some("vac-tui::capability_dashboard")
        );

        let workflow = catalog.slash_route("workflow").expect("workflow route");
        assert_eq!(workflow.status, "partial");
    }

    #[test]
    fn loads_palette_and_cli_route_catalogs() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join(".vac/surfaces")).expect("surfaces dir");
        fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
        fs::write(
            tempdir.path().join(".vac/capabilities/workflow.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: deprecated
owner:
  crate: vac-tui
  module: workflow
surfaces:
  palette: true
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands: []
"#,
        )
        .expect("capability manifest");
        fs::write(
            tempdir.path().join(".vac/capabilities/tools.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.tools
title: Tools
status: deprecated
owner:
  crate: vac-rs
  module: tools
surfaces:
  palette: true
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands: []
"#,
        )
        .expect("capability manifest");
        fs::write(
            tempdir.path().join(".vac/surfaces/palette.yaml"),
            r#"
schema_version: 1
kind: surface
id: surface.palette
title: Palette surface
routes:
  - kind: palette
    action: open_capabilities
    capability: vac.workflow
    owner: "vac-tui::capability_dashboard"
    visible: true
    status: ready
  - kind: palette
    action: open_workflow
    capability: vac.workflow
    owner: "vac-tui::workflow_browser"
    visible: true
    status: partial
    reason: "Under development"
capabilities: [vac.workflow]
"#,
        )
        .expect("palette manifest");
        fs::write(
            tempdir.path().join(".vac/surfaces/cli.yaml"),
            r#"
schema_version: 1
kind: surface
id: surface.cli
title: CLI surface
routes:
  - kind: cli
    command: vac doctor registry
    capability: vac.workflow
    owner: "vac-rs/cli"
    visible: true
    status: ready
  - kind: cli
    command: vac exec
    capability: vac.tools
    owner: "vac-rs/cli"
    visible: true
    status: partial
    reason: "Under development"
capabilities: [vac.workflow, vac.tools]
"#,
        )
        .expect("cli manifest");

        let catalog = SurfaceRouteCatalog::load(tempdir.path());
        let palette = catalog
            .palette_route("open_capabilities")
            .expect("palette route");
        assert_eq!(palette.capability, "vac.workflow");
        assert_eq!(palette.status, "ready");

        let cli = catalog.cli_route("vac doctor registry").expect("cli route");
        assert_eq!(cli.status, "ready");

        assert_eq!(catalog.palette_routes().len(), 2);
        assert_eq!(catalog.cli_routes().len(), 2);
    }

    #[test]
    fn loads_tui_and_statusline_route_catalogs() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join(".vac/surfaces")).expect("surfaces dir");
        fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
        fs::write(
            tempdir.path().join(".vac/capabilities/workflow.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: deprecated
owner:
  crate: vac-tui
  module: workflow
surfaces:
  palette: true
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands: []
"#,
        )
        .expect("capability manifest");
        fs::write(
            tempdir.path().join(".vac/capabilities/sessions.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.sessions
title: Sessions
status: deprecated
owner:
  crate: vac-core
  module: sessions
surfaces:
  palette: true
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands: []
"#,
        )
        .expect("capability manifest");
        fs::write(
            tempdir.path().join(".vac/surfaces/tui.yaml"),
            r#"
schema_version: 1
kind: surface
id: surface.tui
title: TUI surface
routes:
  - kind: tui
    path: /capabilities
    capability: vac.workflow
    owner: "vac-tui::capability_dashboard"
    visible: true
    status: ready
  - kind: tui
    path: /status
    capability: vac.sessions
    owner: "vac-tui::chatwidget"
    visible: true
    status: partial
    reason: "Under development"
capabilities: [vac.workflow, vac.sessions]
"#,
        )
        .expect("tui manifest");
        fs::write(
            tempdir.path().join(".vac/surfaces/statusline.yaml"),
            r#"
schema_version: 1
kind: surface
id: surface.statusline
title: Statusline surface
routes:
  - kind: statusline
    label: session
    capability: vac.sessions
    owner: "vac-tui::statusline"
    visible: true
    status: ready
capabilities: [vac.sessions]
"#,
        )
        .expect("statusline manifest");

        let catalog = SurfaceRouteCatalog::load(tempdir.path());
        let report = load_control_plane_registry_report(tempdir.path());
        assert!(report.registry().is_some(), "{}", report.render_tui_text());

        let capabilities = catalog
            .tui_route("/capabilities")
            .expect("tui /capabilities route");
        assert_eq!(capabilities.capability, "vac.workflow");
        assert_eq!(capabilities.status, "ready");
        assert!(capabilities.visible);

        let status = catalog.tui_route("/status").expect("tui /status route");
        assert_eq!(status.status, "partial");

        assert_eq!(catalog.tui_routes().len(), 2);

        let session = catalog
            .statusline_route("session")
            .expect("statusline session route");
        assert_eq!(session.capability, "vac.sessions");
        assert_eq!(session.status, "ready");
        assert_eq!(catalog.statusline_routes().len(), 1);
    }
}
