use super::SurfaceCrossDiagnostic;
use super::SurfaceCrossSeverity;
use super::build_surface_route_catalog;
use super::load_surface_registry;
use super::validate_surface_cross;
use super::validate_surface_owner_consistency;
use super::validate_surface_reverse_drift;
use super::validate_surface_route_catalog_duplicates;
use crate::control_plane::capability_registry::load_capability_registry;
use crate::control_plane::registry::load_control_plane_registry_report;
use crate::control_plane::surface_manifest::SurfaceRouteKind;
use std::fs;
use tempfile::tempdir;

fn surface_manifest_yaml() -> &'static str {
    r#"
schema_version: 1
kind: surface
id: surface.capabilities
title: Capabilities
routes:
  - kind: tui
    path: /capabilities
    capability: vac.capabilities
    owner: vac-rs/tui
    visible: true
    status: ready
capabilities:
  - vac.capabilities
"#
}

#[test]
fn loads_surface_registry_from_vac_root() {
    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    let surfaces_dir = vac_root.join("surfaces");
    fs::create_dir_all(&surfaces_dir).expect("create surfaces dir");
    fs::write(
        surfaces_dir.join("capabilities.yaml"),
        surface_manifest_yaml(),
    )
    .expect("write surface manifest");

    let registry = load_surface_registry(tempdir.path()).expect("load surface registry");
    assert_eq!(registry.vac_root, vac_root);
    assert_eq!(registry.manifest_dir, surfaces_dir);
    assert_eq!(registry.manifests.len(), 1);
    assert_eq!(registry.manifests[0].manifest.id, "surface.capabilities");
}

fn capability_manifest_yaml(id: &str) -> String {
    format!(
        r#"
schema_version: 1
kind: capability
id: {id}
title: {id}
status: ready
owner:
  crate: vac-core
  module: control_plane
surfaces:
  palette: true
policy:
  mutates_files: false
validation:
  commands:
    - "cargo nextest run -p vac-core --lib"
states:
  - ready
"#
    )
}

#[test]
fn full_registry_rejects_surface_unknown_capability() {
    let tempdir = tempdir().expect("tempdir");
    fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
    fs::create_dir_all(tempdir.path().join(".vac/surfaces")).expect("surfaces dir");
    fs::write(
        tempdir.path().join(".vac/capabilities/workflow.yaml"),
        capability_manifest_yaml("vac.workflow"),
    )
    .expect("capability manifest");
    fs::write(
        tempdir.path().join(".vac/surfaces/broken.yaml"),
        r#"
schema_version: 1
kind: surface
id: surface.broken
title: Broken
capabilities:
  - vac.typo
routes:
  - kind: slash
    command: /broken
    capability: vac.typo
    owner: "vac-tui::broken"
    visible: true
    status: ready
"#,
    )
    .expect("surface manifest");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();
    assert!(report.is_failure(), "rendered:\n{rendered}");
    assert!(
        rendered.contains("surface references an unknown capability"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("routes[0].capability"),
        "rendered:\n{rendered}"
    );
}

fn write_surface(tempdir_path: &std::path::Path, file: &str, yaml: &str) {
    fs::create_dir_all(tempdir_path.join(".vac/surfaces")).expect("surfaces dir");
    fs::write(tempdir_path.join(format!(".vac/surfaces/{file}")), yaml).expect("surface manifest");
}

fn write_capability(tempdir_path: &std::path::Path, file: &str, yaml: &str) {
    fs::create_dir_all(tempdir_path.join(".vac/capabilities")).expect("capabilities dir");
    fs::write(tempdir_path.join(format!(".vac/capabilities/{file}")), yaml)
        .expect("capability manifest");
}

#[test]
fn surface_route_catalog_detects_duplicate_slash_across_surfaces() {
    let tempdir = tempdir().expect("tempdir");
    write_surface(
        tempdir.path(),
        "a.yaml",
        r#"
schema_version: 1
kind: surface
id: surface.a
title: A
capabilities:
  - vac.shared
routes:
  - kind: slash
    command: /shared
    capability: vac.shared
    owner: "vac-tui::a"
    visible: true
    status: ready
"#,
    );
    write_surface(
        tempdir.path(),
        "b.yaml",
        r#"
schema_version: 1
kind: surface
id: surface.b
title: B
capabilities:
  - vac.shared
routes:
  - kind: slash
    command: /shared
    capability: vac.shared
    owner: "vac-tui::b"
    visible: true
    status: ready
"#,
    );

    let registry = load_surface_registry(tempdir.path()).expect("load surface registry");
    let catalog = build_surface_route_catalog(&registry);
    assert_eq!(catalog.len(), 2);
    assert_eq!(catalog.unique_keys(), 1);

    let diagnostics = validate_surface_route_catalog_duplicates(&catalog);
    assert_eq!(diagnostics.len(), 1);
    match &diagnostics[0] {
        SurfaceCrossDiagnostic::DuplicateRoute {
            kind,
            target,
            surfaces,
        } => {
            assert_eq!(*kind, SurfaceRouteKind::Slash);
            assert_eq!(target, "/shared");
            assert_eq!(
                surfaces,
                &vec!["surface.a".to_string(), "surface.b".to_string()]
            );
        }
        other => panic!("expected duplicate route, got {other:?}"),
    }
    assert_eq!(diagnostics[0].severity(), SurfaceCrossSeverity::Failure);
}

#[test]
fn surface_owner_consistency_detects_conflict_across_surfaces() {
    let tempdir = tempdir().expect("tempdir");
    write_surface(
        tempdir.path(),
        "left.yaml",
        r#"
schema_version: 1
kind: surface
id: surface.left
title: Left
capabilities:
  - vac.shared
routes:
  - kind: tui
    path: /left
    capability: vac.shared
    owner: "vac-tui::left"
    visible: true
    status: ready
"#,
    );
    write_surface(
        tempdir.path(),
        "right.yaml",
        r#"
schema_version: 1
kind: surface
id: surface.right
title: Right
capabilities:
  - vac.shared
routes:
  - kind: tui
    path: /right
    capability: vac.shared
    owner: "vac-tui::right"
    visible: true
    status: ready
"#,
    );

    let registry = load_surface_registry(tempdir.path()).expect("load surface registry");
    let diagnostics = validate_surface_owner_consistency(&registry);
    assert_eq!(diagnostics.len(), 1);
    match &diagnostics[0] {
        SurfaceCrossDiagnostic::OwnerConflict { capability, owners } => {
            assert_eq!(capability, "vac.shared");
            assert!(
                owners
                    .iter()
                    .any(|(s, o)| s == "surface.left" && o == "vac-tui::left")
            );
            assert!(
                owners
                    .iter()
                    .any(|(s, o)| s == "surface.right" && o == "vac-tui::right")
            );
        }
        other => panic!("expected owner conflict, got {other:?}"),
    }
    assert_eq!(diagnostics[0].severity(), SurfaceCrossSeverity::Failure);
}

#[test]
fn surface_reverse_drift_detects_palette_declared_without_route() {
    let tempdir = tempdir().expect("tempdir");
    write_capability(
        tempdir.path(),
        "chat.yaml",
        &capability_manifest_yaml("vac.chat"),
    );
    write_surface(
        tempdir.path(),
        "chat.yaml",
        r#"
schema_version: 1
kind: surface
id: surface.chat
title: Chat
capabilities:
  - vac.chat
routes:
  - kind: tui
    path: /chat
    capability: vac.chat
    owner: "vac-tui::chat"
    visible: true
    status: ready
"#,
    );

    let capabilities = load_capability_registry(tempdir.path()).expect("load capability registry");
    let surfaces = load_surface_registry(tempdir.path()).expect("load surface registry");
    let diagnostics = validate_surface_reverse_drift(&surfaces, &capabilities);
    assert!(
        diagnostics.iter().any(|d| matches!(
            d,
            SurfaceCrossDiagnostic::PaletteDrift {
                capability,
                declared_palette: true,
                palette_route_surfaces,
            } if capability == "vac.chat" && palette_route_surfaces.is_empty()
        )),
        "expected palette drift, got {diagnostics:?}"
    );
}

#[test]
fn surface_validate_cross_returns_empty_when_aligned() {
    let tempdir = tempdir().expect("tempdir");
    // Capability with palette:true backed by a real palette route.
    write_capability(
        tempdir.path(),
        "palette.yaml",
        &capability_manifest_yaml("vac.palette"),
    );
    write_surface(
        tempdir.path(),
        "palette.yaml",
        r#"
schema_version: 1
kind: surface
id: surface.palette
title: Palette
capabilities:
  - vac.palette
routes:
  - kind: palette
    action: open_palette
    capability: vac.palette
    owner: "vac-tui::palette"
    visible: true
    status: ready
"#,
    );

    let capabilities = load_capability_registry(tempdir.path()).expect("load capability registry");
    let surfaces = load_surface_registry(tempdir.path()).expect("load surface registry");
    let diagnostics = validate_surface_cross(&surfaces, Some(&capabilities));
    assert!(
        diagnostics.is_empty(),
        "expected no diagnostics, got {diagnostics:?}"
    );
}
