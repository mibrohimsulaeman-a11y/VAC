//! Surface route readiness scanner for Plan 11 closeout.
//!
//! This module intentionally stays lightweight and side-effect free. It does
//! not execute route handlers and it does not attempt to prove runtime UI
//! behaviour. Its contract is narrower: every visible route declared under
//! `.vac/surfaces/*.yaml` must carry a ready route status once the route is
//! covered by the shared surface registry and the owning capability is ready.

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceRouteReadinessIssue {
    pub path: String,
    pub route_index: usize,
    pub target: String,
    pub capability: String,
    pub status: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SurfaceRouteReadinessReport {
    pub surface_files: usize,
    pub visible_routes: usize,
    pub ready_routes: usize,
    pub non_ready_routes: usize,
    pub issues: Vec<SurfaceRouteReadinessIssue>,
}

impl SurfaceRouteReadinessReport {
    pub fn is_ready(&self) -> bool {
        self.visible_routes > 0 && self.non_ready_routes == 0 && self.issues.is_empty()
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "surface route readiness: ready={} files={} visible_routes={} ready_routes={} non_ready_routes={}",
            self.is_ready(),
            self.surface_files,
            self.visible_routes,
            self.ready_routes,
            self.non_ready_routes
        )];
        for issue in &self.issues {
            lines.push(format!(
                "surface route readiness issue: path={} route={} target={} capability={} status={} reason={}",
                issue.path,
                issue.route_index,
                issue.target,
                issue.capability,
                issue.status,
                issue.reason
            ));
        }
        lines
    }

    pub fn render_text(&self) -> String {
        self.render_lines().join("\n")
    }
}

#[derive(Debug, Clone, Default)]
struct RouteBlock {
    path: PathBuf,
    route_index: usize,
    target: String,
    capability: String,
    visible: Option<bool>,
    status: Option<String>,
}

pub fn load_surface_route_readiness_report(
    repo_root: impl AsRef<Path>,
) -> SurfaceRouteReadinessReport {
    let surface_dir = repo_root.as_ref().join(".vac/surfaces");
    let mut report = SurfaceRouteReadinessReport::default();
    let Ok(entries) = fs::read_dir(&surface_dir) else {
        report.issues.push(SurfaceRouteReadinessIssue {
            path: surface_dir.display().to_string(),
            route_index: 0,
            target: "<surface-dir>".to_string(),
            capability: "<unknown>".to_string(),
            status: "missing".to_string(),
            reason: "surface directory is missing or unreadable".to_string(),
        });
        return report;
    };

    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "yaml"))
        .collect();
    files.sort();

    for path in files {
        report.surface_files += 1;
        match fs::read_to_string(&path) {
            Ok(contents) => consume_surface_file(&path, &contents, &mut report),
            Err(err) => report.issues.push(SurfaceRouteReadinessIssue {
                path: path.display().to_string(),
                route_index: 0,
                target: "<file>".to_string(),
                capability: "<unknown>".to_string(),
                status: "unreadable".to_string(),
                reason: err.to_string(),
            }),
        }
    }
    report
}

fn consume_surface_file(path: &Path, contents: &str, report: &mut SurfaceRouteReadinessReport) {
    let mut current: Option<RouteBlock> = None;
    let mut route_index = 0usize;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("- kind:") {
            flush_route(current.take(), report);
            route_index += 1;
            current = Some(RouteBlock {
                path: path.to_path_buf(),
                route_index,
                ..RouteBlock::default()
            });
            continue;
        }
        let Some(route) = current.as_mut() else {
            continue;
        };
        if let Some(value) = yaml_scalar(trimmed, "path") {
            route.target = value;
        } else if let Some(value) = yaml_scalar(trimmed, "command") {
            route.target = value;
        } else if let Some(value) = yaml_scalar(trimmed, "action") {
            route.target = value;
        } else if let Some(value) = yaml_scalar(trimmed, "label") {
            route.target = value;
        } else if let Some(value) = yaml_scalar(trimmed, "capability") {
            route.capability = value;
        } else if let Some(value) = yaml_scalar(trimmed, "visible") {
            route.visible = Some(value == "true");
        } else if let Some(value) = yaml_scalar(trimmed, "status") {
            route.status = Some(value);
        }
    }
    flush_route(current, report);
}

fn flush_route(route: Option<RouteBlock>, report: &mut SurfaceRouteReadinessReport) {
    let Some(route) = route else {
        return;
    };
    if route.visible != Some(true) {
        return;
    }
    report.visible_routes += 1;
    let status = route.status.unwrap_or_else(|| "missing".to_string());
    if status == "ready" {
        report.ready_routes += 1;
        return;
    }
    report.non_ready_routes += 1;
    report.issues.push(SurfaceRouteReadinessIssue {
        path: route.path.display().to_string(),
        route_index: route.route_index,
        target: empty_as_unknown(route.target),
        capability: empty_as_unknown(route.capability),
        status,
        reason: "visible route is not marked ready in the shared surface registry".to_string(),
    });
}

fn yaml_scalar(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    let value = line.strip_prefix(&prefix)?.trim();
    Some(value.trim_matches('"').trim_matches('\'').to_string())
}

fn empty_as_unknown(value: String) -> String {
    if value.trim().is_empty() {
        "<unknown>".to_string()
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vac-surface-readiness-{name}-{nonce}"));
        fs::create_dir_all(root.join(".vac/surfaces")).expect("surface dir");
        root
    }

    #[test]
    fn passes_when_all_visible_routes_are_ready() {
        let root = temp_root("ready");
        fs::write(
            root.join(".vac/surfaces/cli.yaml"),
            r#"schema_version: 1
kind: surface
id: surface.cli
title: CLI
capabilities:
- vac.workflow
routes:
- kind: cli
  command: vac doctor
  capability: vac.workflow
  owner: vac-cli/doctor_cli
  visible: true
  status: ready
"#,
        )
        .expect("write surface");
        let report = load_surface_route_readiness_report(&root);
        assert!(report.is_ready(), "{}", report.render_text());
        assert_eq!(report.visible_routes, 1);
        assert_eq!(report.ready_routes, 1);
    }

    #[test]
    fn reports_visible_non_ready_route() {
        let root = temp_root("partial");
        fs::write(
            root.join(".vac/surfaces/palette.yaml"),
            r#"schema_version: 1
kind: surface
id: surface.palette
title: Palette
capabilities:
- vac.workflow
routes:
- kind: palette
  action: open_workflow
  capability: vac.workflow
  owner: vac-core/control_plane
  visible: true
  status: partial
"#,
        )
        .expect("write surface");
        let report = load_surface_route_readiness_report(&root);
        assert!(!report.is_ready());
        assert_eq!(report.non_ready_routes, 1);
        assert!(report.render_text().contains("open_workflow"));
    }
}
