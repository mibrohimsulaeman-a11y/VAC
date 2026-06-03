use std::path::Path;

use super::registry::load_control_plane_registry_report;
use super::registry_diagnostics::RegistryLoadReport;
use super::surface_manifest::SurfaceRouteKind;
use super::surface_readiness::SurfaceRouteReadinessReport;
use super::surface_readiness::load_surface_route_readiness_report;
use super::surface_registry::SurfaceCrossDiagnostic;
use super::surface_registry::SurfaceCrossSeverity;
use super::surface_registry::validate_surface_cross;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceDoctorManifestEntry {
    pub id: String,
    pub path: String,
    pub routes: usize,
    pub capabilities: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceDoctorReport {
    registry: RegistryLoadReport,
    manifests: Vec<SurfaceDoctorManifestEntry>,
    route_count: usize,
    tui_routes: usize,
    slash_routes: usize,
    palette_routes: usize,
    cli_routes: usize,
    statusline_routes: usize,
    surface_diagnostics: Vec<SurfaceCrossDiagnostic>,
    route_readiness: SurfaceRouteReadinessReport,
}

pub fn load_surface_doctor_report(path: impl AsRef<Path>) -> SurfaceDoctorReport {
    let root = path.as_ref();
    let registry = load_control_plane_registry_report(root);
    let mut report = SurfaceDoctorReport {
        registry,
        manifests: Vec::new(),
        route_count: 0,
        tui_routes: 0,
        slash_routes: 0,
        palette_routes: 0,
        cli_routes: 0,
        statusline_routes: 0,
        surface_diagnostics: Vec::new(),
        route_readiness: load_surface_route_readiness_report(root),
    };

    let Some(registry) = report.registry.registry() else {
        return report;
    };

    report.surface_diagnostics =
        validate_surface_cross(&registry.surfaces, Some(&registry.capabilities));

    for entry in &registry.surfaces.manifests {
        report.route_count += entry.manifest.routes.len();
        for route in &entry.manifest.routes {
            match route.kind {
                SurfaceRouteKind::Tui => report.tui_routes += 1,
                SurfaceRouteKind::Slash => report.slash_routes += 1,
                SurfaceRouteKind::Palette => report.palette_routes += 1,
                SurfaceRouteKind::Cli => report.cli_routes += 1,
                SurfaceRouteKind::Statusline => report.statusline_routes += 1,
            }
        }
        report.manifests.push(SurfaceDoctorManifestEntry {
            id: entry.manifest.id.clone(),
            path: entry.path.display().to_string(),
            routes: entry.manifest.routes.len(),
            capabilities: entry.manifest.capabilities.len(),
        });
    }
    report
        .manifests
        .sort_by(|left, right| left.id.cmp(&right.id));
    report
}

impl SurfaceDoctorReport {
    pub fn registry(&self) -> &RegistryLoadReport {
        &self.registry
    }

    pub fn surface_diagnostics(&self) -> &[SurfaceCrossDiagnostic] {
        &self.surface_diagnostics
    }

    pub fn route_readiness(&self) -> &SurfaceRouteReadinessReport {
        &self.route_readiness
    }

    pub fn has_critical_cross_diagnostics(&self) -> bool {
        self.surface_diagnostics
            .iter()
            .any(|d| matches!(d.severity(), SurfaceCrossSeverity::Failure))
    }

    pub fn is_failure(&self) -> bool {
        self.registry.is_failure()
            || self.has_critical_cross_diagnostics()
            || !self.route_readiness.is_ready()
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = self.registry.render_lines();
        if self.registry.registry().is_none() {
            if self.is_failure() && !lines.iter().any(|l| l.contains("surfaces: failure")) {
                lines.push("surfaces: failure".to_string());
            }
            return lines;
        }

        lines.push(format!(
            "surfaces: loaded {} manifests routes={}",
            self.manifests.len(),
            self.route_count
        ));
        lines.push(format!(
            "surface route kinds: tui={} slash={} palette={} cli={} statusline={}",
            self.tui_routes,
            self.slash_routes,
            self.palette_routes,
            self.cli_routes,
            self.statusline_routes
        ));
        for manifest in &self.manifests {
            lines.push(format!(
                "{}: routes={} capabilities={} path={}",
                manifest.id, manifest.routes, manifest.capabilities, manifest.path
            ));
        }

        let dup_count = self
            .surface_diagnostics
            .iter()
            .filter(|d| matches!(d, SurfaceCrossDiagnostic::DuplicateRoute { .. }))
            .count();
        let owner_count = self
            .surface_diagnostics
            .iter()
            .filter(|d| matches!(d, SurfaceCrossDiagnostic::OwnerConflict { .. }))
            .count();
        let palette_count = self
            .surface_diagnostics
            .iter()
            .filter(|d| matches!(d, SurfaceCrossDiagnostic::PaletteDrift { .. }))
            .count();
        let route_count = self
            .surface_diagnostics
            .iter()
            .filter(|d| matches!(d, SurfaceCrossDiagnostic::SurfaceRouteDrift { .. }))
            .count();
        lines.push(format!(
            "surface diagnostics: duplicates={dup_count} owner_conflicts={owner_count} palette_drift={palette_count} route_drift={route_count}"
        ));
        lines.extend(self.route_readiness.render_lines());
        for diag in &self.surface_diagnostics {
            lines.push(format!("surface diagnostic: {}", diag.render()));
        }

        if self.is_failure() && !lines.iter().any(|l| l.contains("surfaces: failure")) {
            lines.push("surfaces: failure".to_string());
        }

        lines
    }

    pub fn render_text(&self) -> String {
        self.render_lines().join("\n")
    }
}

#[cfg(test)]
#[path = "surface_doctor_tests.rs"]
mod tests;
