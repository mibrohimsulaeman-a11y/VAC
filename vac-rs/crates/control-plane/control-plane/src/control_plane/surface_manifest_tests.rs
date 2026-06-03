use super::SurfaceManifest;
use super::SurfaceManifestKind;
use super::SurfaceRouteKind;
use super::SurfaceRouteStatus;
use super::parse_surface_manifest;
use super::surface_route_dedup_key;
use super::surface_route_kind_label;
use super::surface_route_target;
use super::validate_surface_manifest;
use super::validate_surface_manifest_against_known_capabilities;
use std::collections::HashSet;
use std::path::Path;

fn sample_manifest_yaml() -> &'static str {
    r#"
schema_version: 1
kind: surface
id: surface.activity
title: Activity
routes:
  - kind: tui
    path: /activity
    capability: vac.activity
    owner: vac-rs/tui
    visible: true
    status: partial
    reason: partial route documents degraded guidance for static validation
  - kind: slash
    command: /activity
    capability: vac.activity
    owner: vac-rs/tui
    visible: true
    status: ready
  - kind: palette
    action: open_activity
    capability: vac.activity
    owner: vac-rs/tui
    visible: true
    status: planned
  - kind: cli
    command: vac activity
    capability: vac.activity
    visible: false
    status: cli_only
    reason: CLI mirrors slash entry
capabilities:
  - vac.activity
"#
}

#[test]
fn parse_and_validate_surface_manifest() {
    let manifest = parse_surface_manifest(Path::new("surface.yaml"), sample_manifest_yaml())
        .expect("surface manifest should parse");

    assert_eq!(manifest.schema_version, 1);
    assert_eq!(manifest.kind, SurfaceManifestKind::Surface);
    assert_eq!(manifest.id, "surface.activity");
    assert_eq!(manifest.title, "Activity");
    assert_eq!(manifest.capabilities, vec!["vac.activity"]);
    assert_eq!(manifest.routes.len(), 4);
    assert_eq!(manifest.routes[0].kind, SurfaceRouteKind::Tui);
    assert_eq!(manifest.routes[0].status, SurfaceRouteStatus::Partial);
    assert_eq!(manifest.routes[3].status, SurfaceRouteStatus::CliOnly);

    validate_surface_manifest(Path::new("surface.yaml"), &manifest)
        .expect("surface manifest should validate");
}

#[test]
fn surface_manifest_rejects_visible_routes_without_owner() {
    let manifest = SurfaceManifest {
        schema_version: 1,
        kind: SurfaceManifestKind::Surface,
        id: "surface.activity".to_string(),
        title: "Activity".to_string(),
        capabilities: vec!["vac.activity".to_string()],
        routes: vec![super::SurfaceRoute {
            kind: SurfaceRouteKind::Tui,
            path: Some("/activity".to_string()),
            command: None,
            action: None,
            label: None,
            capability: "vac.activity".to_string(),
            owner: None,
            visible: true,
            status: SurfaceRouteStatus::Ready,
            reason: None,
        }],
    };

    let error = validate_surface_manifest(Path::new("surface.yaml"), &manifest)
        .expect_err("visible routes require an owner");
    assert_eq!(error.field_path(), "routes[0].owner");
    assert_eq!(
        error.message(),
        "visible routes must declare a capability owner"
    );
}

#[test]
fn surface_manifest_rejects_visible_route_missing_surface_capability() {
    let manifest = SurfaceManifest {
        schema_version: 1,
        kind: SurfaceManifestKind::Surface,
        id: "surface.activity".to_string(),
        title: "Activity".to_string(),
        capabilities: vec!["vac.other".to_string()],
        routes: vec![super::SurfaceRoute {
            kind: SurfaceRouteKind::Tui,
            path: Some("/activity".to_string()),
            command: None,
            action: None,
            label: None,
            capability: "vac.activity".to_string(),
            owner: Some("vac-rs/tui".to_string()),
            visible: true,
            status: SurfaceRouteStatus::Ready,
            reason: None,
        }],
    };

    let error = validate_surface_manifest(Path::new("surface.yaml"), &manifest)
        .expect_err("visible routes must be backed by a listed surface capability");
    assert_eq!(error.field_path(), "routes[0].capability");
    assert_eq!(
        error.message(),
        "visible route capability must be listed in surface capabilities"
    );
}

#[test]
fn surface_manifest_rejects_unknown_known_capability() {
    let manifest = parse_surface_manifest(Path::new("surface.yaml"), sample_manifest_yaml())
        .expect("surface manifest should parse");

    let known_capabilities = HashSet::from([String::from("vac.other")]);
    let error = validate_surface_manifest_against_known_capabilities(
        Path::new("surface.yaml"),
        &manifest,
        &known_capabilities,
    )
    .expect_err("unknown capabilities should fail the known-capability gate");

    assert_eq!(error.field_path(), "routes[0].capability");
    assert_eq!(error.message(), "surface references an unknown capability");
}

#[test]
fn surface_manifest_rejects_cli_only_route_without_reason() {
    let manifest = SurfaceManifest {
        schema_version: 1,
        kind: SurfaceManifestKind::Surface,
        id: "surface.cli".to_string(),
        title: "CLI".to_string(),
        capabilities: vec!["vac.cli".to_string()],
        routes: vec![super::SurfaceRoute {
            kind: SurfaceRouteKind::Cli,
            path: None,
            command: Some("vac status".to_string()),
            action: None,
            label: None,
            capability: "vac.cli".to_string(),
            owner: None,
            visible: false,
            status: SurfaceRouteStatus::CliOnly,
            reason: None,
        }],
    };

    let error = validate_surface_manifest(Path::new("surface.yaml"), &manifest)
        .expect_err("cli-only routes must explain why");
    assert_eq!(error.field_path(), "routes[0].reason");
    assert_eq!(
        error.message(),
        "cli-only or unavailable routes must say why"
    );
}

#[test]
fn surface_manifest_rejects_route_kind_field_mismatch() {
    let error = parse_surface_manifest(
        Path::new("surface.yaml"),
        r#"
schema_version: 1
kind: surface
id: surface.bad
title: Bad
capabilities:
  - vac.bad
routes:
  - kind: slash
    command: /bad
    path: /bad
    capability: vac.bad
    owner: vac-rs/tui
    visible: true
    status: ready
"#,
    )
    .expect_err("slash route must not also declare path");

    assert_eq!(error.field_path(), "routes[0]");
    assert_eq!(error.message(), "slash routes require only `command`");
}

#[test]
fn surface_manifest_rejects_visible_unavailable_route() {
    let error = parse_surface_manifest(
        Path::new("surface.yaml"),
        r#"
schema_version: 1
kind: surface
id: surface.bad
title: Bad
capabilities:
  - vac.bad
routes:
  - kind: tui
    path: /bad
    capability: vac.bad
    owner: vac-rs/tui
    visible: true
    status: unavailable
    reason: not ready
"#,
    )
    .expect_err("visible unavailable routes must be rejected");

    assert_eq!(error.field_path(), "routes[0].status");
    assert_eq!(
        error.message(),
        "visible routes must be ready, partial, or planned"
    );
}

#[test]
fn surface_manifest_rejects_statusline_with_action_field() {
    let error = parse_surface_manifest(
        Path::new("surface.yaml"),
        r#"
schema_version: 1
kind: surface
id: surface.bad
title: Bad
capabilities:
  - vac.bad
routes:
  - kind: statusline
    label: session
    action: open_session
    capability: vac.bad
    owner: vac-rs/tui
    visible: true
    status: ready
"#,
    )
    .expect_err("statusline route must not declare action");

    assert_eq!(error.field_path(), "routes[0]");
    assert_eq!(error.message(), "statusline routes require only `label`");
}

#[test]
fn surface_route_target_returns_canonical_field_per_kind() {
    let manifest = parse_surface_manifest(Path::new("surface.yaml"), sample_manifest_yaml())
        .expect("sample manifest should parse");
    let routes = &manifest.routes;
    assert_eq!(surface_route_target(&routes[0]), Some("/activity"));
    assert_eq!(surface_route_target(&routes[1]), Some("/activity"));
    assert_eq!(surface_route_target(&routes[2]), Some("open_activity"));
    assert_eq!(surface_route_target(&routes[3]), Some("vac activity"));
}

#[test]
fn surface_route_dedup_key_distinguishes_kinds_on_same_target() {
    let manifest = parse_surface_manifest(Path::new("surface.yaml"), sample_manifest_yaml())
        .expect("sample manifest should parse");
    let tui_key = surface_route_dedup_key(&manifest.routes[0]).expect("tui key");
    let slash_key = surface_route_dedup_key(&manifest.routes[1]).expect("slash key");
    assert_eq!(tui_key, (SurfaceRouteKind::Tui, "/activity".to_string()));
    assert_eq!(
        slash_key,
        (SurfaceRouteKind::Slash, "/activity".to_string())
    );
    assert_ne!(tui_key, slash_key);
}

#[test]
fn surface_route_kind_label_covers_all_variants() {
    assert_eq!(surface_route_kind_label(SurfaceRouteKind::Tui), "tui");
    assert_eq!(surface_route_kind_label(SurfaceRouteKind::Slash), "slash");
    assert_eq!(
        surface_route_kind_label(SurfaceRouteKind::Palette),
        "palette"
    );
    assert_eq!(surface_route_kind_label(SurfaceRouteKind::Cli), "cli");
    assert_eq!(
        surface_route_kind_label(SurfaceRouteKind::Statusline),
        "statusline"
    );
}
