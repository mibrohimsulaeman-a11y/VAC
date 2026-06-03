use super::load_surface_doctor_report;
use std::fs;
use tempfile::tempdir;

fn seed_capability(root: &std::path::Path, id: &str) {
    seed_capability_with_surfaces(root, id, "  palette: true");
}

fn seed_root_seed_baseline_except_workflow(root: &std::path::Path) {
    seed_root_seed_policies(root);
    seed_root_seed_workflows(root);
    seed_root_seed_surfaces(root);
    seed_root_seed_source_domains(root);

    seed_capability_with_surfaces(
        root,
        "vac.chat",
        "  tui:\n    routes:\n      - /chat\n  slash:\n    commands:\n      - /chat\n  palette: true",
    );
    seed_capability_with_surfaces(
        root,
        "vac.approvals",
        "  tui:\n    routes:\n      - /approvals\n  slash:\n    commands:\n      - /approvals\n  palette: true",
    );
    seed_capability_with_surfaces(
        root,
        "vac.tools",
        "  tui:\n    routes:\n      - /tools\n  slash:\n    commands:\n      - /tools\n  palette: true",
    );
    seed_capability_with_surfaces(
        root,
        "vac.sandbox",
        "  tui:\n    routes:\n      - /sandbox\n  cli:\n    enabled: true\n    commands:\n      - vac sandbox\n  palette: false",
    );
    seed_capability_with_surfaces(
        root,
        "vac.sessions",
        "  tui:\n    routes:\n      - /sessions\n  slash:\n    commands:\n      - /sessions\n  palette: true",
    );
    seed_capability_with_surfaces(
        root,
        "vac.build",
        "  cli:\n    enabled: true\n    commands:\n      - vac build\n  palette: false",
    );
    seed_capability_with_surfaces(
        root,
        "vac.identity",
        "  tui:\n    routes:\n      - /identity\n  cli:\n    enabled: true\n    commands:\n      - vac identity\n  palette: false",
    );
    seed_capability_with_surfaces(
        root,
        "vac.identity.check",
        "  tui:\n    routes:\n      - /identity-check\n  cli:\n    enabled: true\n    commands:\n      - vac doctor identity\n  palette: false",
    );
    seed_capability_with_surfaces(
        root,
        "vac.release",
        "  cli:\n    enabled: true\n    commands:\n      - vac doctor release\n  palette: false",
    );
    seed_capability_with_surfaces(
        root,
        "vac.tui",
        "  tui:\n    routes:\n      - /tui\n  palette: false",
    );
    seed_capability_with_surfaces(
        root,
        "vac.ownership",
        "  tui:\n    routes:\n      - /ownership\n  cli:\n    enabled: true\n    commands:\n      - vac doctor ownership\n  palette: false",
    );
    seed_capability_with_surfaces(
        root,
        "vac.architecture",
        "  cli:\n    enabled: true\n    commands:\n      - vac doctor architecture\n  palette: false",
    );
}

fn seed_root_seed_policies(root: &std::path::Path) {
    for id in [
        "vac.policy.approval",
        "vac.default-local",
        "vac.policy.filesystem",
        "vac.policy.network",
        "vac.policy.sandbox",
        "vac.policy.tools",
    ] {
        let file_name = id.trim_start_matches("vac.").replace('.', "-");
        fs::create_dir_all(root.join(".vac/policies")).expect("policies dir");
        fs::write(
            root.join(format!(".vac/policies/{file_name}.yaml")),
            format!(
                r#"
schema_version: 1
kind: policy
id: {id}
title: {id}
default_decision: approval_required
rules: []
"#
            ),
        )
        .expect("policy manifest");
    }
}

fn seed_root_seed_workflows(root: &std::path::Path) {
    for id in [
        "maintenance.build-check",
        "maintenance.identity-check",
        "maintenance.release-gate",
        "maintenance.ownership-scan",
        "maintenance.no-duplicate-tui",
        "maintenance.donor-migration-gate",
    ] {
        let file_name = id.trim_start_matches("maintenance.").replace('.', "-");
        fs::create_dir_all(root.join(".vac/workflows")).expect("workflows dir");
        fs::write(
            root.join(format!(".vac/workflows/{file_name}.yaml")),
            format!(
                r#"
schema_version: 1
kind: workflow
id: {id}
title: {id}
status: planned
inputs: {{}}
steps:
  - id: report
    uses: capability.activity.emit
ui:
  surface: /workflow
  progress_panel: true
  activity_log: true
policy:
  default_risk: safe_read
validation:
  commands:
    - cargo check -p vac-core
"#
            ),
        )
        .expect("workflow manifest");
    }
}

fn seed_root_seed_source_domains(root: &std::path::Path) {
    let files = [
        "vac-rs/core/src/chat/mod.rs",
        "vac-rs/core/src/runtime/mod.rs",
        "vac-rs/core/src/control_plane/approval.rs",
        "vac-rs/core/src/tools/mod.rs",
        "vac-rs/tools/src/tools.rs",
        "vac-rs/sandbox/src/sandbox.rs",
        "vac-rs/core/src/sandbox/mod.rs",
        "vac-rs/core/src/sessions/mod.rs",
        "vac-rs/core/src/control_plane/workflow.rs",
        "vac-rs/cli/src/build_cli.rs",
        "vac-rs/core/src/identity/mod.rs",
        "vac-rs/cli/src/identity_cli.rs",
        "vac-rs/core/src/identity/check.rs",
        "vac-rs/cli/src/doctor_cli.rs",
        "vac-rs/tui/src/tui.rs",
        "vac-rs/core/src/control_plane/ownership_scan.rs",
        "vac-rs/core/src/control_plane/architecture.rs",
    ];
    for file in files {
        let path = root.join(file);
        fs::create_dir_all(path.parent().expect("source parent")).expect("source parent dir");
        fs::write(path, "// test fixture source domain\n").expect("source stub");
    }

    for cargo_toml in [
        "vac-rs/core/Cargo.toml",
        "vac-rs/tools/Cargo.toml",
        "vac-rs/sandbox/Cargo.toml",
        "vac-rs/cli/Cargo.toml",
        "vac-rs/tui/Cargo.toml",
    ] {
        let path = root.join(cargo_toml);
        fs::create_dir_all(path.parent().expect("cargo parent")).expect("cargo parent dir");
        fs::write(
            path,
            "[package]\nname = \"fixture\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
        )
        .expect("cargo stub");
    }
}

fn capability_ownership_block(_id: &str) -> String {
    // These surface-doctor fixtures exercise surface rendering, not root feature
    // ownership conversion. Leaving ownership empty avoids coupling the tests to
    // the full source-inventory fixture required by Plan 19 ownership gates.
    String::new()
}

fn seed_root_seed_surfaces(root: &std::path::Path) {
    fs::create_dir_all(root.join(".vac/surfaces")).expect("surfaces dir");
    fs::write(
        root.join(".vac/surfaces/tui.yaml"),
        r#"
schema_version: 1
kind: surface
id: surface.tui
title: TUI
capabilities:
  - vac.chat
  - vac.approvals
  - vac.tools
  - vac.sandbox
  - vac.sessions
  - vac.identity
  - vac.identity.check
  - vac.tui
  - vac.ownership
routes:
  - kind: tui
    path: /chat
    capability: vac.chat
    owner: vac-core/control_plane
    visible: true
    status: ready
  - kind: tui
    path: /approvals
    capability: vac.approvals
    owner: vac-core/control_plane
    visible: true
    status: ready
  - kind: tui
    path: /tools
    capability: vac.tools
    owner: vac-core/control_plane
    visible: true
    status: ready
  - kind: tui
    path: /sandbox
    capability: vac.sandbox
    owner: vac-core/control_plane
    visible: true
    status: ready
  - kind: tui
    path: /sessions
    capability: vac.sessions
    owner: vac-core/control_plane
    visible: true
    status: ready
  - kind: tui
    path: /identity
    capability: vac.identity
    owner: vac-core/control_plane
    visible: true
    status: ready
  - kind: tui
    path: /identity-check
    capability: vac.identity.check
    owner: vac-core/control_plane_check
    visible: true
    status: ready
  - kind: tui
    path: /tui
    capability: vac.tui
    owner: vac-core/control_plane
    visible: true
    status: ready
  - kind: tui
    path: /ownership
    capability: vac.ownership
    owner: vac-core/control_plane
    visible: true
    status: ready
"#,
    )
    .expect("tui surface");
    fs::write(
        root.join(".vac/surfaces/slash.yaml"),
        r#"
schema_version: 1
kind: surface
id: surface.slash
title: Slash
capabilities:
  - vac.chat
  - vac.approvals
  - vac.tools
  - vac.sessions
routes:
  - kind: slash
    command: /chat
    capability: vac.chat
    owner: vac-core/control_plane
    visible: true
    status: ready
  - kind: slash
    command: /approvals
    capability: vac.approvals
    owner: vac-core/control_plane
    visible: true
    status: ready
  - kind: slash
    command: /tools
    capability: vac.tools
    owner: vac-core/control_plane
    visible: true
    status: ready
  - kind: slash
    command: /sessions
    capability: vac.sessions
    owner: vac-core/control_plane
    visible: true
    status: ready
"#,
    )
    .expect("slash surface");
    fs::write(
        root.join(".vac/surfaces/palette.yaml"),
        r#"
schema_version: 1
kind: surface
id: surface.palette
title: Palette
capabilities:
  - vac.chat
  - vac.approvals
  - vac.tools
  - vac.sessions
routes:
  - kind: palette
    action: open_chat
    capability: vac.chat
    owner: vac-core/control_plane
    visible: true
    status: ready
  - kind: palette
    action: open_approvals
    capability: vac.approvals
    owner: vac-core/control_plane
    visible: true
    status: ready
  - kind: palette
    action: open_tools
    capability: vac.tools
    owner: vac-core/control_plane
    visible: true
    status: ready
  - kind: palette
    action: open_sessions
    capability: vac.sessions
    owner: vac-core/control_plane
    visible: true
    status: ready
"#,
    )
    .expect("palette surface");
    fs::write(
        root.join(".vac/surfaces/cli.yaml"),
        r#"
schema_version: 1
kind: surface
id: surface.cli
title: CLI
capabilities:
  - vac.sandbox
  - vac.build
  - vac.identity
  - vac.identity.check
  - vac.release
  - vac.ownership
  - vac.architecture
routes:
  - kind: cli
    command: vac sandbox
    capability: vac.sandbox
    owner: vac-core/control_plane
    visible: true
    status: ready
  - kind: cli
    command: vac build
    capability: vac.build
    owner: vac-core/control_plane
    visible: true
    status: ready
  - kind: cli
    command: vac identity
    capability: vac.identity
    owner: vac-core/control_plane
    visible: true
    status: ready
  - kind: cli
    command: vac doctor identity
    capability: vac.identity.check
    owner: vac-core/control_plane_check
    visible: true
    status: ready
  - kind: cli
    command: vac doctor release
    capability: vac.release
    owner: vac-core/control_plane
    visible: true
    status: ready
  - kind: cli
    command: vac doctor ownership
    capability: vac.ownership
    owner: vac-core/control_plane
    visible: true
    status: ready
  - kind: cli
    command: vac doctor architecture
    capability: vac.architecture
    owner: vac-core/control_plane
    visible: true
    status: ready
"#,
    )
    .expect("cli surface");
}

fn seed_capability_with_surfaces(root: &std::path::Path, id: &str, surfaces_block: &str) {
    let file_name = id.trim_start_matches("vac.").replace('.', "-");
    fs::create_dir_all(root.join(".vac/capabilities")).expect("capabilities dir");
    fs::write(
        root.join(format!(".vac/capabilities/{file_name}.yaml")),
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
{ownership_block}surfaces:
{surfaces_block}
policy:
  mutates_files: false
validation:
  commands:
    - "cargo nextest run -p vac-core --lib"
states:
  - ready
"#,
            ownership_block = capability_ownership_block(id)
        ),
    )
    .expect("capability manifest");
}

#[test]
fn surface_doctor_renders_surface_summary() {
    let tempdir = tempdir().expect("tempdir");
    seed_root_seed_baseline_except_workflow(tempdir.path());
    seed_capability_with_surfaces(
        tempdir.path(),
        "vac.workflow",
        "  tui:\n    routes:\n      - /workflow\n  slash:\n    commands:\n      - /workflow\n  palette: false",
    );
    fs::create_dir_all(tempdir.path().join(".vac/surfaces")).expect("surfaces dir");
    fs::write(
        tempdir.path().join(".vac/surfaces/workflow.yaml"),
        r#"
schema_version: 1
kind: surface
id: surface.workflow
title: Workflow surface
capabilities:
  - vac.workflow
routes:
  - kind: slash
    command: /workflow
    capability: vac.workflow
    owner: "vac-tui::workflow_browser"
    visible: true
    status: ready
  - kind: tui
    path: /workflow
    capability: vac.workflow
    owner: "vac-tui::workflow_browser"
    visible: true
    status: ready
"#,
    )
    .expect("surface manifest");

    let report = load_surface_doctor_report(tempdir.path());
    let rendered = report.render_text();
    assert!(!report.is_failure(), "rendered:\n{rendered}");
    assert!(
        rendered.contains("surfaces: loaded 5 manifests routes=26"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("surface route kinds: tui=10 slash=5 palette=4 cli=7 statusline=0"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("surface.workflow: routes=2 capabilities=1"),
        "rendered:\n{rendered}"
    );
}

#[test]
fn surface_doctor_renders_unknown_capability_diagnostic() {
    let tempdir = tempdir().expect("tempdir");
    seed_capability(tempdir.path(), "vac.workflow");
    fs::create_dir_all(tempdir.path().join(".vac/surfaces")).expect("surfaces dir");
    fs::write(
        tempdir.path().join(".vac/surfaces/broken.yaml"),
        r#"
schema_version: 1
kind: surface
id: surface.broken
title: Broken surface
capabilities:
  - vac.missing
routes:
  - kind: slash
    command: /broken
    capability: vac.missing
    owner: "vac-tui::broken"
    visible: true
    status: ready
"#,
    )
    .expect("surface manifest");

    let report = load_surface_doctor_report(tempdir.path());
    let rendered = report.render_text();
    assert!(report.is_failure(), "rendered:\n{rendered}");
    assert!(
        rendered.contains("registry: failure"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("surface references an unknown capability"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("routes[0].capability"),
        "rendered:\n{rendered}"
    );
}

#[test]
fn surface_doctor_renders_palette_drift_warning() {
    let tempdir = tempdir().expect("tempdir");
    seed_root_seed_baseline_except_workflow(tempdir.path());
    seed_capability_with_surfaces(
        tempdir.path(),
        "vac.workflow",
        "  palette: true\n  tui:\n    routes:\n      - /workflow",
    );
    fs::create_dir_all(tempdir.path().join(".vac/surfaces")).expect("surfaces dir");
    fs::write(
        tempdir.path().join(".vac/surfaces/workflow.yaml"),
        r#"
schema_version: 1
kind: surface
id: surface.workflow
title: Workflow
capabilities:
  - vac.workflow
routes:
  - kind: tui
    path: /workflow
    capability: vac.workflow
    owner: "vac-tui::workflow"
    visible: true
    status: ready
"#,
    )
    .expect("surface manifest");

    let report = load_surface_doctor_report(tempdir.path());
    let rendered = report.render_text();
    assert!(report.is_failure(), "rendered:\n{rendered}");
    assert!(
        rendered.contains(
            "surface diagnostics: duplicates=0 owner_conflicts=0 palette_drift=1 route_drift=0"
        ),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains(
            "capability `vac.workflow` declares palette=true but no surface exposes a palette route"
        ),
        "rendered:\n{rendered}"
    );
}

#[test]
fn surface_doctor_fails_on_duplicate_routes_across_surfaces() {
    let tempdir = tempdir().expect("tempdir");
    seed_capability(tempdir.path(), "vac.shared");
    fs::create_dir_all(tempdir.path().join(".vac/surfaces")).expect("surfaces dir");
    fs::write(
        tempdir.path().join(".vac/surfaces/a.yaml"),
        r#"
schema_version: 1
kind: surface
id: surface.a
title: A
capabilities:
  - vac.shared
routes:
  - kind: palette
    action: open_shared
    capability: vac.shared
    owner: "vac-tui::a"
    visible: true
    status: ready
"#,
    )
    .expect("surface manifest a");
    fs::write(
        tempdir.path().join(".vac/surfaces/b.yaml"),
        r#"
schema_version: 1
kind: surface
id: surface.b
title: B
capabilities:
  - vac.shared
routes:
  - kind: palette
    action: open_shared
    capability: vac.shared
    owner: "vac-tui::b"
    visible: true
    status: ready
"#,
    )
    .expect("surface manifest b");

    let report = load_surface_doctor_report(tempdir.path());
    let rendered = report.render_text();
    assert!(report.is_failure(), "rendered:\n{rendered}");
    assert!(
        rendered.contains("surface diagnostic: duplicate palette route `open_shared` declared by surfaces: surface.a, surface.b"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains(
            "surface diagnostic: capability `vac.shared` has inconsistent owners across surfaces:"
        ),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("surfaces: failure"),
        "rendered:\n{rendered}"
    );
}
