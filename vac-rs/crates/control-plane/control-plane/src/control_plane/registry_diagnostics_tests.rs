use super::super::registry::load_control_plane_registry_report;
use super::RegistryDiagnosticSeverity;
use super::RegistryLoadReport;
use super::RegistryLoadState;
use std::fs;
use tempfile::tempdir;
use tempfile::tempdir_in;

#[test]
fn loading_report_renders_loading_state() {
    let report = RegistryLoadReport::loading();
    assert_eq!(report.state(), RegistryLoadState::Loading);
    assert_eq!(report.render_lines(), vec!["registry: loading".to_string()]);
}

#[test]
fn missing_vac_root_renders_empty_warning_report() {
    let tempdir = tempdir_in("/var/tmp").expect("tempdir");
    let report = load_control_plane_registry_report(tempdir.path());

    assert_eq!(report.state(), RegistryLoadState::Empty);
    assert_eq!(report.diagnostics().len(), 1);
    assert!(report.vac_root().is_none());

    let diagnostic = &report.diagnostics()[0];
    assert_eq!(diagnostic.severity(), RegistryDiagnosticSeverity::Warning);
    assert!(
        diagnostic
            .render_line()
            .contains("unable to locate `.vac` root")
    );
    assert!(report.render_text().contains("registry: empty"));
    assert!(
        report.render_text().contains(
            "hint: create a `.vac/registry/product.yaml` file to enable the control plane"
        )
    );
}

#[test]
fn invalid_manifest_renders_file_and_field_path() {
    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    let workflows_dir = vac_root.join("workflows");
    fs::create_dir_all(&workflows_dir).expect("create workflows dir");
    fs::write(
        workflows_dir.join("broken.yaml"),
        r#"
schema_version: 1
kind: workflow
id: product.semantic-coding-loop
title: Semantic coding loop
status: planned
inputs: {}
steps:
  - id: intent
    uses: capability.intent.parse
ui:
  surface: workflow
  progress_panel: true
  activity_log: true
policy:
  default_risk: safe_edit
validation:
  commands: []
"#,
    )
    .expect("write workflow manifest");

    let report = load_control_plane_registry_report(tempdir.path());

    assert_eq!(report.state(), RegistryLoadState::Failure);
    assert_eq!(report.vac_root(), Some(vac_root.as_path()));
    assert_eq!(report.diagnostics().len(), 1);

    let diagnostic = &report.diagnostics()[0];
    assert_eq!(diagnostic.severity(), RegistryDiagnosticSeverity::Error);
    assert_eq!(
        diagnostic.file(),
        workflows_dir.join("broken.yaml").as_path()
    );
    assert_eq!(diagnostic.field_path(), Some("ui.surface"));
    assert!(report.render_text().contains("registry: failure"));
    assert!(
        report
            .render_text()
            .contains("hint: fix the reported field or YAML syntax")
    );
}

#[test]
fn registry_root_seed_coverage_reports_missing_feature() {
    let tempdir = tempdir().expect("tempdir");
    fs::create_dir_all(tempdir.path().join(".vac")).expect("create empty vac root");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();

    assert!(report.is_failure(), "rendered:\n{rendered}");
    assert!(
        rendered.contains("root seed coverage: blocked"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("missing root seed capability `vac.release`"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("root seed policy registry is empty"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("missing root seed policy manifest `vac.policy.network`"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("missing root seed surface manifest `surface.tui`"),
        "rendered:\n{rendered}"
    );
}

#[test]
fn registry_root_seed_coverage_allows_ready_workflow_declarative_capability_runner() {
    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    let workflows_dir = vac_root.join("workflows");
    let capabilities_dir = vac_root.join("capabilities");
    fs::create_dir_all(&workflows_dir).expect("create workflows dir");
    fs::create_dir_all(&capabilities_dir).expect("create capabilities dir");
    fs::write(
        capabilities_dir.join("not-mapped.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.not_mapped.yet
title: Not mapped yet
status: ready
states:
  empty: empty
  loading: loading
  success: success
  failure: failure
owner:
  crate: vac-core
  module: not_mapped
ownership:
  crates:
    - vac-core
  modules:
    - not_mapped
surfaces:
  palette: true
policy:
  risk: safe_read
validation:
  commands:
    - cargo check -p vac-core
"#,
    )
    .expect("capability manifest");
    fs::write(
        workflows_dir.join("unsupported-ready.yaml"),
        r#"
schema_version: 1
kind: workflow
id: maintenance.unsupported-ready
title: Unsupported ready workflow
status: ready
inputs: {}
steps:
  - id: validate
    uses: capability.not_mapped.yet
ui:
  surface: /workflow
  progress_panel: true
  activity_log: true
policy:
  default_risk: safe_read
validation:
  commands:
    - cargo +1.95.0 check -p vac-surface-cli
"#,
    )
    .expect("workflow manifest");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();

    assert!(report.is_failure(), "rendered:\n{rendered}");
    assert!(
        !rendered.contains(
            "ready workflow `maintenance.unsupported-ready` has unsupported runner steps"
        ),
        "rendered:\n{rendered}"
    );
    assert!(!rendered.contains("validate=capability.not_mapped.yet"));
}

#[test]
fn registry_root_seed_coverage_reports_missing_canonical_policy() {
    let tempdir = tempdir().expect("tempdir");
    let policies_dir = tempdir.path().join(".vac/policies");
    fs::create_dir_all(&policies_dir).expect("create policies dir");
    fs::write(
        policies_dir.join("approval.yaml"),
        r#"
schema_version: 1
kind: policy
id: vac.policy.approval
title: Approval request policy
default_decision: approval_required
rules:
  - id: approve-tool-call
    match:
      action: tool_call
      tool: approval.request
    decision: approval_required
    reason: approval requests must remain visible to operators
"#,
    )
    .expect("policy manifest");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();

    assert!(report.is_failure(), "rendered:\n{rendered}");
    assert!(rendered.contains("policies=1/6"), "rendered:\n{rendered}");
    assert!(
        rendered.contains("missing root seed policy manifest `vac.policy.network`"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("missing root seed policy manifest `vac.default-local`"),
        "rendered:\n{rendered}"
    );
}

#[test]
fn registry_root_seed_coverage_blocks_ready_capability_without_surface_route() {
    let tempdir = tempdir().expect("tempdir");
    let capabilities_dir = tempdir.path().join(".vac/capabilities");
    fs::create_dir_all(&capabilities_dir).expect("create capabilities dir");
    fs::write(
        capabilities_dir.join("chat.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.chat
title: Chat
status: ready
states:
  empty: empty
  loading: loading
  success: success
  failure: failure
owner:
  crate: vac-surface-tui
  module: chatwidget
ownership:
  crates:
    - vac-tui
  modules:
    - chatwidget
surfaces:
  palette: true
policy:
  risk: safe_read
  mutates_files: false
validation:
  commands:
    - cargo check -p vac-surface-tui
"#,
    )
    .expect("capability manifest");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();

    assert!(report.is_failure(), "rendered:\n{rendered}");
    assert!(
        rendered.contains(
            "ready root seed capability `vac.chat` is not represented in any surface manifest"
        ),
        "rendered:\n{rendered}"
    );
}

#[test]
fn registry_root_seed_coverage_blocks_root_seed_with_donor_source() {
    let tempdir = tempdir().expect("tempdir");
    let capabilities_dir = tempdir.path().join(".vac/capabilities");
    fs::create_dir_all(&capabilities_dir).expect("create capabilities dir");
    fs::write(
        capabilities_dir.join("chat.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.chat
title: Chat
status: partial
states:
  empty: empty
  loading: loading
  success: success
  failure: failure
reason: test fixture partial
owner:
  crate: vac-surface-tui
  module: chatwidget
ownership:
  crates:
    - vac-tui
  modules:
    - chatwidget
surfaces:
  palette: true
  tui:
    routes:
      - /chat
policy:
  risk: safe_read
  mutates_files: false
validation:
  commands:
    - cargo check -p vac-surface-tui
donor_source:
  source: donor/vac/crates/vac_chat
  root_target: vac-rs/tui/src/chatwidget.rs
  cleanup_status: migrated
"#,
    )
    .expect("capability manifest");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();

    assert!(report.is_failure(), "rendered:\n{rendered}");
    assert!(
        rendered.contains("root seed capability `vac.chat` must not declare donor_source"),
        "rendered:\n{rendered}"
    );
}

#[test]
fn registry_root_feature_conversion_reports_unowned_source_domain_without_targets() {
    let tempdir = tempdir().expect("tempdir");
    fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
    fs::create_dir_all(tempdir.path().join("vac-rs/core/src")).expect("source dir");
    fs::write(
        tempdir.path().join("vac-rs/core/Cargo.toml"),
        r#"
[package]
name = "vac-core"
version = "0.0.0"
edition = "2024"
"#,
    )
    .expect("cargo toml");
    fs::write(
        tempdir.path().join("vac-rs/core/src/lib.rs"),
        "pub mod hidden_domain;\n",
    )
    .expect("lib rs");
    fs::write(tempdir.path().join("vac-rs/core/src/hidden_domain.rs"), "").expect("hidden domain");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();

    assert!(
        rendered.contains("source domain `vac-core/hidden_domain` is not claimed"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("root_feature_conversion.source.vac-core.hidden_domain"),
        "rendered:\n{rendered}"
    );
}

#[test]
fn registry_root_feature_conversion_reports_unowned_source_domain_when_targets_exist() {
    let tempdir = tempdir().expect("tempdir");
    fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
    fs::create_dir_all(tempdir.path().join("vac-rs/core/src")).expect("source dir");
    fs::write(
        tempdir.path().join("vac-rs/core/Cargo.toml"),
        r#"
[package]
name = "vac-core"
version = "0.0.0"
edition = "2024"
"#,
    )
    .expect("cargo toml");
    fs::write(
        tempdir.path().join("vac-rs/core/src/lib.rs"),
        "pub mod control_plane;\npub mod hidden_domain;\n",
    )
    .expect("lib rs");
    fs::write(tempdir.path().join("vac-rs/core/src/control_plane.rs"), "")
        .expect("control plane rs");
    fs::write(tempdir.path().join("vac-rs/core/src/hidden_domain.rs"), "").expect("hidden domain");
    fs::write(
        tempdir.path().join(".vac/capabilities/workflow.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: partial
states:
  empty: empty
  loading: loading
  success: success
  failure: failure
reason: test fixture partial
owner:
  crate: vac-core
  module: control_plane
ownership:
  crates:
    - vac-core
  modules:
    - control_plane
surfaces:
  palette: true
policy:
  risk: safe_read
validation:
  commands:
    - cargo check
"#,
    )
    .expect("workflow capability manifest");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();

    assert!(report.is_failure(), "rendered:\n{rendered}");
    assert!(
        rendered.contains("source domain `vac-core/hidden_domain` is not claimed"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("root_feature_conversion.source.vac-core.hidden_domain"),
        "rendered:\n{rendered}"
    );
}

#[test]
fn registry_reports_surface_route_not_declared_by_capability_manifest() {
    let tempdir = tempdir().expect("tempdir");
    let capabilities_dir = tempdir.path().join(".vac/capabilities");
    let surfaces_dir = tempdir.path().join(".vac/surfaces");
    fs::create_dir_all(&capabilities_dir).expect("capabilities dir");
    fs::create_dir_all(&surfaces_dir).expect("surfaces dir");
    fs::write(
        capabilities_dir.join("chat.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.chat
title: Chat
status: ready
states:
  empty: empty
  loading: loading
  success: success
  failure: failure
owner:
  crate: vac-surface-tui
  module: chatwidget
ownership:
  crates:
    - vac-tui
  modules:
    - chatwidget
surfaces:
  palette: true
policy:
  risk: safe_read
validation:
  commands:
    - cargo check -p vac-surface-tui
"#,
    )
    .expect("capability manifest");
    fs::write(
        surfaces_dir.join("tui.yaml"),
        r#"
schema_version: 1
kind: surface
id: surface.tui
title: TUI
capabilities:
  - vac.chat
routes:
  - kind: tui
    path: /chat
    capability: vac.chat
    owner: vac-tui/chatwidget
    visible: true
    status: ready
"#,
    )
    .expect("surface manifest");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();

    assert!(report.is_failure(), "rendered:\n{rendered}");
    assert!(
        rendered.contains("visible surface route `/chat` for `vac.chat` is not declared"),
        "rendered:\n{rendered}"
    );
    assert!(
        !rendered.contains("surface route owner `vac-tui/chatwidget` differs from capability owner `vac-tui/chatwidget`"),
        "matching owner should not warn:\n{rendered}"
    );
}

#[test]
fn registry_reports_capability_surface_without_surface_manifest_route() {
    let tempdir = tempdir().expect("tempdir");
    let capabilities_dir = tempdir.path().join(".vac/capabilities");
    fs::create_dir_all(&capabilities_dir).expect("capabilities dir");
    fs::write(
        capabilities_dir.join("workflow.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: partial
states:
  empty: empty
  loading: loading
  success: success
  failure: failure
reason: route not wired yet
owner:
  crate: vac-core
  module: control_plane
ownership:
  crates:
    - vac-core
  modules:
    - control_plane
surfaces:
  cli:
    enabled: true
    commands:
      - vac doctor registry
policy:
  risk: safe_read
validation:
  commands:
    - cargo check -p vac-core
"#,
    )
    .expect("capability manifest");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();

    assert!(report.is_failure(), "rendered:\n{rendered}");
    assert!(
        rendered.contains("capability `vac.workflow` declares cli surface `vac doctor registry` but no matching surface manifest route exists"),
        "rendered:\n{rendered}"
    );
}

#[test]
fn registry_reports_cross_surface_duplicate_route_keys() {
    let tempdir = tempdir().expect("tempdir");
    let capabilities_dir = tempdir.path().join(".vac/capabilities");
    let surfaces_dir = tempdir.path().join(".vac/surfaces");
    fs::create_dir_all(&capabilities_dir).expect("capabilities dir");
    fs::create_dir_all(&surfaces_dir).expect("surfaces dir");
    fs::write(
        capabilities_dir.join("chat.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.chat
title: Chat
status: ready
states:
  empty: empty
  loading: loading
  success: success
  failure: failure
owner:
  crate: vac-surface-tui
  module: chatwidget
ownership:
  crates:
    - vac-tui
  modules:
    - chatwidget
surfaces:
  slash:
    commands:
      - /chat
policy:
  risk: safe_read
validation:
  commands:
    - cargo check -p vac-surface-tui
"#,
    )
    .expect("chat capability manifest");
    fs::write(
        capabilities_dir.join("workflow.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: partial
states:
  empty: empty
  loading: loading
  success: success
  failure: failure
reason: test fixture partial
owner:
  crate: vac-core
  module: control_plane
ownership:
  crates:
    - vac-core
  modules:
    - control_plane
surfaces:
  slash:
    commands:
      - /chat
policy:
  risk: safe_read
validation:
  commands:
    - cargo check -p vac-core
"#,
    )
    .expect("workflow capability manifest");
    fs::write(
        surfaces_dir.join("slash.yaml"),
        r#"
schema_version: 1
kind: surface
id: surface.slash
title: Slash
capabilities:
  - vac.chat
routes:
  - kind: slash
    command: /chat
    capability: vac.chat
    owner: vac-tui/chatwidget
    visible: true
    status: ready
"#,
    )
    .expect("slash surface manifest");
    fs::write(
        surfaces_dir.join("z_palette.yaml"),
        r#"
schema_version: 1
kind: surface
id: surface.palette
title: Palette
capabilities:
  - vac.workflow
routes:
  - kind: slash
    command: /chat
    capability: vac.workflow
    owner: vac-core/control_plane
    visible: true
    status: ready
"#,
    )
    .expect("duplicate surface manifest");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();

    assert!(report.is_failure(), "rendered:\n{rendered}");
    assert!(
        rendered.contains("duplicate slash surface route `/chat` for `vac.workflow`"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("slash.yaml:routes[0]"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("retire the stale duplicate route"),
        "rendered:\n{rendered}"
    );
}

#[test]
fn registry_capability_missing_ownership_is_warning() {
    let tempdir = tempdir().expect("tempdir");
    let capabilities_dir = tempdir.path().join(".vac/capabilities");
    fs::create_dir_all(&capabilities_dir).expect("capabilities dir");
    fs::write(
        capabilities_dir.join("chat.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.chat
title: Chat
status: ready
states:
  empty: empty
  loading: loading
  success: success
  failure: failure
owner:
  crate: vac-surface-tui
  module: chatwidget
surfaces:
  palette: true
policy:
  risk: safe_read
validation:
  commands:
    - cargo check -p vac-surface-tui
"#,
    )
    .expect("capability manifest");

    let report = load_control_plane_registry_report(tempdir.path());

    // Find the diagnostic for missing ownership metadata
    let warning = report
        .diagnostics()
        .iter()
        .find(|d| d.message().contains("is missing ownership metadata"))
        .expect("diagnostic for missing ownership exists");

    assert_eq!(warning.severity(), RegistryDiagnosticSeverity::Warning);
}

#[test]
fn registry_partial_capability_missing_reason_is_manifest_error() {
    let tempdir = tempdir().expect("tempdir");
    let capabilities_dir = tempdir.path().join(".vac/capabilities");
    fs::create_dir_all(&capabilities_dir).expect("capabilities dir");
    fs::write(
        capabilities_dir.join("chat.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.chat
title: Chat
status: partial
states:
  empty: empty
  loading: loading
  success: success
  failure: failure
owner:
  crate: vac-surface-tui
  module: chatwidget
ownership:
  crates:
    - vac-tui
  modules:
    - chatwidget
surfaces:
  palette: true
policy:
  risk: safe_read
validation:
  commands:
    - cargo check -p vac-surface-tui
"#,
    )
    .expect("capability manifest");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();

    assert_eq!(report.state(), RegistryLoadState::Failure);
    let diagnostic = report
        .diagnostics()
        .iter()
        .find(|d| d.field_path() == Some("reason"))
        .expect("diagnostic for missing reason exists");

    assert_eq!(diagnostic.severity(), RegistryDiagnosticSeverity::Error);
    assert!(
        rendered.contains("planned or partial capabilities must declare a reason"),
        "rendered:
{rendered}"
    );
}

#[test]
fn registry_partial_capability_with_reason_is_surfaced_as_info() {
    let tempdir = tempdir().expect("tempdir");
    let capabilities_dir = tempdir.path().join(".vac/capabilities");
    fs::create_dir_all(&capabilities_dir).expect("capabilities dir");
    fs::create_dir_all(tempdir.path().join("vac-rs/core/src")).expect("source dir");
    fs::write(
        tempdir.path().join("vac-rs/core/Cargo.toml"),
        r#"
[package]
name = "vac-core"
version = "0.0.0"
edition = "2024"
"#,
    )
    .expect("cargo toml");
    fs::write(
        tempdir.path().join("vac-rs/core/src/lib.rs"),
        "pub mod control_plane;\n",
    )
    .expect("lib rs");
    fs::write(tempdir.path().join("vac-rs/core/src/control_plane.rs"), "")
        .expect("control plane rs");
    fs::write(
        capabilities_dir.join("workflow.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: partial
states:
  empty: empty
  loading: loading
  success: success
  failure: failure
reason: runner support is still being wired
owner:
  crate: vac-core
  module: control_plane
ownership:
  crates:
    - vac-core
  modules:
    - control_plane
surfaces:
  palette: true
policy:
  risk: safe_read
validation:
  commands:
    - cargo check -p vac-core
"#,
    )
    .expect("capability manifest");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();
    let info = report
        .diagnostics()
        .iter()
        .find(|d| {
            d.message()
                .contains("remains partial: runner support is still being wired")
        })
        .expect("partial-with-reason diagnostic exists");

    assert_eq!(info.severity(), RegistryDiagnosticSeverity::Info);
    assert!(
        rendered.contains("root_feature_conversion.workflow.reason"),
        "rendered:\n{rendered}"
    );
}

#[test]
fn registry_report_cli_exit_code_follows_failure_state() {
    let tempdir = tempdir_in("/var/tmp").expect("tempdir");
    let empty_report = load_control_plane_registry_report(tempdir.path());
    assert_eq!(empty_report.state(), RegistryLoadState::Empty);
    assert_eq!(empty_report.cli_exit_code(), 0);

    fs::create_dir_all(tempdir.path().join(".vac")).expect("create empty vac root");
    let failing_report = load_control_plane_registry_report(tempdir.path());
    assert!(failing_report.is_failure());
    assert_eq!(failing_report.cli_exit_code(), 1);
}

#[test]
fn registry_blocks_on_reverse_drift_palette() {
    let tempdir = tempdir().expect("tempdir");
    let capabilities_dir = tempdir.path().join(".vac/capabilities");
    let surfaces_dir = tempdir.path().join(".vac/surfaces");
    fs::create_dir_all(&capabilities_dir).expect("capabilities dir");
    fs::create_dir_all(&surfaces_dir).expect("surfaces dir");
    fs::write(
        capabilities_dir.join("chat.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.chat
title: Chat
status: ready
states:
  empty: empty
  loading: loading
  success: success
  failure: failure
owner:
  crate: vac-surface-tui
  module: chatwidget
ownership:
  crates:
    - vac-tui
  modules:
    - chatwidget
surfaces:
  palette: true
policy:
  risk: safe_read
validation:
  commands:
    - cargo check -p vac-surface-tui
"#,
    )
    .expect("capability manifest");

    fs::write(
        surfaces_dir.join("tui.yaml"),
        r#"
schema_version: 1
kind: surface
id: surface.tui
title: TUI
capabilities:
  - vac.chat
routes:
  - kind: tui
    path: /other
    capability: vac.chat
    owner: vac-tui/chatwidget
    visible: false
    status: planned
  - kind: tui
    path: /chat
    capability: vac.chat
    owner: vac-tui/chatwidget
    visible: true
    status: ready
"#,
    )
    .expect("surface manifest");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();

    assert!(report.is_failure(), "rendered:\n{rendered}");
    assert!(
        rendered.contains("capability `vac.chat` declares palette surface but no matching surface manifest route exists"),
        "rendered:\n{rendered}"
    );
}

#[test]
fn registry_blocks_on_inconsistent_route_owners_across_surfaces() {
    let tempdir = tempdir().expect("tempdir");
    let capabilities_dir = tempdir.path().join(".vac/capabilities");
    let surfaces_dir = tempdir.path().join(".vac/surfaces");
    fs::create_dir_all(&capabilities_dir).expect("capabilities dir");
    fs::create_dir_all(&surfaces_dir).expect("surfaces dir");
    fs::write(
        capabilities_dir.join("chat.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.chat
title: Chat
status: ready
states:
  empty: empty
  loading: loading
  success: success
  failure: failure
owner:
  crate: vac-surface-tui
  module: chatwidget
ownership:
  crates:
    - vac-tui
  modules:
    - chatwidget
surfaces:
  palette: true
policy:
  risk: safe_read
validation:
  commands:
    - cargo check -p vac-surface-tui
"#,
    )
    .expect("capability manifest");

    fs::write(
        surfaces_dir.join("slash.yaml"),
        r#"
schema_version: 1
kind: surface
id: surface.slash
title: Slash
capabilities:
  - vac.chat
routes:
  - kind: slash
    command: /chat
    capability: vac.chat
    owner: vac-tui/chatwidget
    visible: true
    status: ready
"#,
    )
    .expect("slash manifest");

    fs::write(
        surfaces_dir.join("palette.yaml"),
        r#"
schema_version: 1
kind: surface
id: surface.palette
title: Palette
capabilities:
  - vac.chat
routes:
  - kind: palette
    action: open_chat
    capability: vac.chat
    owner: vac-core/control_plane
    visible: true
    status: ready
"#,
    )
    .expect("palette manifest");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();

    assert!(report.is_failure(), "rendered:\n{rendered}");
    assert!(
        rendered.contains("inconsistent owner `"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("vac-core/control_plane"),
        "rendered:\n{rendered}"
    );
    assert!(
        rendered.contains("vac-tui/chatwidget"),
        "rendered:\n{rendered}"
    );
}

#[test]
fn registry_allows_slash_route_normalization() {
    let tempdir = tempdir().expect("tempdir");
    let capabilities_dir = tempdir.path().join(".vac/capabilities");
    let surfaces_dir = tempdir.path().join(".vac/surfaces");
    fs::create_dir_all(&capabilities_dir).expect("capabilities dir");
    fs::create_dir_all(&surfaces_dir).expect("surfaces dir");
    fs::write(
        capabilities_dir.join("chat.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.chat
title: Chat
status: ready
states:
  empty: empty
  loading: loading
  success: success
  failure: failure
owner:
  crate: vac-surface-tui
  module: chatwidget
ownership:
  crates:
    - vac-tui
  modules:
    - chatwidget
surfaces:
  slash:
    commands:
      - chat
policy:
  risk: safe_read
validation:
  commands:
    - cargo check -p vac-surface-tui
"#,
    )
    .expect("capability manifest");

    fs::write(
        surfaces_dir.join("slash.yaml"),
        r#"
schema_version: 1
kind: surface
id: surface.slash
title: Slash
capabilities:
  - vac.chat
routes:
  - kind: slash
    command: /chat
    capability: vac.chat
    owner: vac-tui/chatwidget
    visible: true
    status: ready
"#,
    )
    .expect("slash manifest");

    let report = load_control_plane_registry_report(tempdir.path());
    let rendered = report.render_text();

    assert!(
        !rendered.contains("visible surface route `/chat` for `vac.chat` is not declared"),
        "slash command normalization should avoid false drift:\n{rendered}"
    );
}
