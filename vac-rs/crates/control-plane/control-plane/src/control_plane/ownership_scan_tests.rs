use super::load_ownership_scan_report;
use std::fs;
use tempfile::tempdir;

#[test]
fn renders_ownership_scan_report_with_missing_entries() {
    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    fs::create_dir_all(vac_root.join("capabilities")).expect("capabilities dir");
    fs::write(
        vac_root.join("capabilities/ownership.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.ownership
title: Ownership
status: partial
reason: "Plan 02"
states:
  - empty
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
  mutates_files: false
validation:
  commands: []
"#,
    )
    .expect("ownership manifest");
    fs::write(
        vac_root.join("capabilities/build.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.build
title: Build
status: ready
states:
  - empty
owner:
  crate: vac-surface-cli
  module: main
surfaces:
  palette: true
policy:
  mutates_files: false
validation:
  commands:
    - cargo test
"#,
    )
    .expect("build manifest");

    let report = load_ownership_scan_report(tempdir.path());
    assert_eq!(report.entries().len(), 2);
    assert_eq!(report.owned_count(), 1);
    assert_eq!(report.unowned_count(), 1);
    assert_eq!(report.ready_unowned_count(), 1);
    assert_eq!(report.target_count(), 1);
    assert_eq!(report.test_only_target_count(), 0);
    assert_eq!(report.retired_target_count(), 0);
    let rendered = report.render_text();
    assert!(rendered.contains("ownership scan: total=2 owned=1 unowned=1 ready_unowned=1"));
    assert!(rendered.contains("ownership targets: total=1 test_only=0 retired=0"));
    assert!(rendered.contains("ownership targets matrix:"));
    assert!(rendered.contains("vac-core/control_plane"));
    assert!(rendered.contains("vac.ownership"));
    assert!(rendered.contains("owned"));
    assert!(rendered.contains("ownership matrix:"));
    assert!(rendered.contains("vac.ownership"));
    assert!(rendered.contains("ownership: crates=[vac-core] modules=[control_plane]"));
    assert!(rendered.contains("vac.build"));
    assert!(rendered.contains("warning: capability missing ownership metadata"));
    assert!(rendered.contains("warning: ready capability missing ownership metadata"));
}

#[test]
fn renders_ownership_scan_report_with_test_only_targets() {
    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    fs::create_dir_all(vac_root.join("capabilities")).expect("capabilities dir");
    fs::write(
        vac_root.join("capabilities/test-only.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.test.only
title: Test Only
status: partial
reason: "Plan 02"
states:
  - empty
owner:
  crate: vac-core
  module: control_plane
ownership:
  crates:
    - vac-core
  modules:
    - control_plane
  test_only: true
surfaces:
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
    )
    .expect("test-only manifest");

    let report = load_ownership_scan_report(tempdir.path());
    assert_eq!(report.entries().len(), 1);
    assert_eq!(report.owned_count(), 1);
    assert_eq!(report.unowned_count(), 0);
    assert_eq!(report.target_count(), 1);
    assert_eq!(report.test_only_target_count(), 1);
    assert_eq!(report.retired_target_count(), 0);
    let rendered = report.render_text();
    assert!(rendered.contains("ownership targets: total=1 test_only=1 retired=0"));
    assert!(rendered.contains("test_only"));
    assert!(rendered.contains("vac-core/control_plane"));
}

#[test]
fn renders_ownership_scan_report_with_retired_targets() {
    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    fs::create_dir_all(vac_root.join("capabilities")).expect("capabilities dir");
    fs::write(
        vac_root.join("capabilities/retired.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.retired
title: Retired
status: deprecated
owner:
  crate: vac-core
  module: control_plane
ownership:
  crates:
    - vac-core
  modules:
    - control_plane
  retired: true
  deletion_plan: "Plan 021"
surfaces:
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
    )
    .expect("retired manifest");

    let report = load_ownership_scan_report(tempdir.path());
    assert_eq!(report.entries().len(), 1);
    assert_eq!(report.owned_count(), 1);
    assert_eq!(report.unowned_count(), 0);
    assert_eq!(report.target_count(), 1);
    assert_eq!(report.test_only_target_count(), 0);
    assert_eq!(report.retired_target_count(), 1);
    let rendered = report.render_text();
    assert!(rendered.contains("ownership targets: total=1 test_only=0 retired=1"));
    assert!(rendered.contains("retired"));
    assert!(rendered.contains("vac-core/control_plane"));
}

#[test]
fn detects_unowned_source_domains_from_inventory() {
    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    fs::create_dir_all(vac_root.join("capabilities")).expect("capabilities dir");
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
        "pub mod unowned_domain;\n",
    )
    .expect("lib rs");
    fs::write(tempdir.path().join("vac-rs/core/src/unowned_domain.rs"), "")
        .expect("unowned domain");

    let report = load_ownership_scan_report(tempdir.path());

    assert_eq!(report.source_domain_count(), 2);
    assert_eq!(report.unowned_source_domain_count(), 1);
    let rendered = report.render_text();
    assert!(rendered.contains("source domains: total=2 unowned=1"));
    assert!(rendered.contains("vac-core/unowned_domain"));
}

#[test]
fn source_inventory_detects_dotted_nested_modules() {
    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    fs::create_dir_all(vac_root.join("capabilities")).expect("capabilities dir");
    fs::create_dir_all(tempdir.path().join("vac-rs/tui/src/bottom_pane")).expect("source dir");
    fs::write(
        tempdir.path().join("vac-rs/tui/Cargo.toml"),
        r#"
[package]
name = "vac-tui"
version = "0.0.0"
edition = "2024"
"#,
    )
    .expect("cargo toml");
    fs::write(
        tempdir.path().join("vac-rs/tui/src/lib.rs"),
        "pub mod bottom_pane;\n",
    )
    .expect("lib rs");
    fs::write(
        tempdir.path().join("vac-rs/tui/src/bottom_pane.rs"),
        "pub mod approval_overlay;\n",
    )
    .expect("bottom pane rs");
    fs::write(
        tempdir
            .path()
            .join("vac-rs/tui/src/bottom_pane/approval_overlay.rs"),
        "",
    )
    .expect("approval overlay rs");
    fs::write(
        vac_root.join("capabilities/approvals.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.approvals
title: Approvals
status: partial
reason: "Plan 02"
states:
  - empty
owner:
  crate: vac-surface-tui
  module: bottom_pane.approval_overlay
ownership:
  crates:
    - vac-tui
  modules:
    - bottom_pane.approval_overlay
surfaces:
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
    )
    .expect("approvals manifest");

    let report = load_ownership_scan_report(tempdir.path());

    assert!(
        report
            .source_inventory()
            .contains_module("vac-tui", "bottom_pane.approval_overlay")
    );
    let ownership = report
        .entries()
        .iter()
        .find(|entry| entry.id == "vac.approvals")
        .and_then(|entry| entry.ownership.as_ref())
        .expect("ownership");
    assert!(report.missing_claimed_modules(ownership).is_empty());
}

#[test]
fn parent_module_ownership_covers_dotted_child_modules() {
    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    fs::create_dir_all(vac_root.join("capabilities")).expect("capabilities dir");
    fs::create_dir_all(tempdir.path().join("vac-rs/core/src/control_plane")).expect("source dir");
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
    fs::write(
        tempdir.path().join("vac-rs/core/src/control_plane.rs"),
        "pub mod ownership_scan;\n",
    )
    .expect("control plane rs");
    fs::write(
        tempdir
            .path()
            .join("vac-rs/core/src/control_plane/ownership_scan.rs"),
        "",
    )
    .expect("ownership scan rs");
    fs::write(
        vac_root.join("capabilities/ownership.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.ownership
title: Ownership
status: partial
reason: "Plan 02"
states:
  - empty
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
  mutates_files: false
validation:
  commands: []
"#,
    )
    .expect("ownership manifest");

    let report = load_ownership_scan_report(tempdir.path());
    let rendered = report.render_text();

    assert!(
        report
            .source_inventory()
            .contains_module("vac-core", "control_plane.ownership_scan")
    );
    assert!(
        !rendered.contains("vac-core/control_plane.ownership_scan hidden"),
        "rendered:\n{rendered}"
    );
}

#[test]
fn source_inventory_ignores_bootstrap_and_test_modules_as_hidden_domains() {
    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    fs::create_dir_all(vac_root.join("capabilities")).expect("capabilities dir");
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
        "pub mod tests;\npub mod chat_tests;\npub mod product_domain;\n",
    )
    .expect("lib rs");
    fs::write(tempdir.path().join("vac-rs/core/src/tests.rs"), "").expect("tests rs");
    fs::write(tempdir.path().join("vac-rs/core/src/chat_tests.rs"), "").expect("chat tests rs");
    fs::write(tempdir.path().join("vac-rs/core/src/product_domain.rs"), "")
        .expect("product domain rs");

    let report = load_ownership_scan_report(tempdir.path());
    let rendered = report.render_text();

    assert!(rendered.contains("vac-core/product_domain hidden"));
    assert!(
        !rendered.contains("vac-core/lib hidden")
            && !rendered.contains("vac-core/tests hidden")
            && !rendered.contains("vac-core/chat_tests hidden"),
        "rendered:\n{rendered}"
    );
}

#[test]
fn detects_unowned_source_domains_when_other_crate_has_targets() {
    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    fs::create_dir_all(vac_root.join("capabilities")).expect("capabilities dir");
    fs::create_dir_all(tempdir.path().join("vac-rs/core/src")).expect("core source dir");
    fs::create_dir_all(tempdir.path().join("vac-rs/orphan/src")).expect("orphan source dir");
    fs::write(
        tempdir.path().join("vac-rs/core/Cargo.toml"),
        r#"
[package]
name = "vac-core"
version = "0.0.0"
edition = "2024"
"#,
    )
    .expect("core cargo toml");
    fs::write(
        tempdir.path().join("vac-rs/core/src/lib.rs"),
        "pub mod control_plane;\n",
    )
    .expect("core lib rs");
    fs::write(tempdir.path().join("vac-rs/core/src/control_plane.rs"), "")
        .expect("control plane rs");
    fs::write(
        tempdir.path().join("vac-rs/orphan/Cargo.toml"),
        r#"
[package]
name = "vac-orphan"
version = "0.0.0"
edition = "2024"
"#,
    )
    .expect("orphan cargo toml");
    fs::write(
        tempdir.path().join("vac-rs/orphan/src/lib.rs"),
        "pub mod hidden_runtime;\n",
    )
    .expect("orphan lib rs");
    fs::write(
        tempdir.path().join("vac-rs/orphan/src/hidden_runtime.rs"),
        "",
    )
    .expect("hidden runtime rs");
    fs::write(
        vac_root.join("capabilities/ownership.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.ownership
title: Ownership
status: partial
reason: "Plan 02"
states:
  - empty
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
  mutates_files: false
validation:
  commands: []
"#,
    )
    .expect("ownership manifest");

    let report = load_ownership_scan_report(tempdir.path());
    let rendered = report.render_text();

    assert!(
        rendered.contains("vac-orphan/hidden_runtime"),
        "rendered:\n{rendered}"
    );
    assert!(report.is_failure(), "rendered:\n{rendered}");
}

#[test]
fn processes_granular_ownership_targets_and_prevents_cartesian_overclaim() {
    let tempdir = tempdir().expect("tempdir");
    let vac_root = tempdir.path().join(".vac");
    fs::create_dir_all(vac_root.join("capabilities")).expect("capabilities dir");
    fs::create_dir_all(tempdir.path().join("vac-rs/core/src")).expect("core src dir");
    fs::create_dir_all(tempdir.path().join("vac-rs/cli/src")).expect("cli src dir");

    fs::write(
        tempdir.path().join("vac-rs/core/Cargo.toml"),
        r#"
[package]
name = "vac-core"
version = "0.0.0"
edition = "2024"
"#,
    )
    .expect("core cargo");
    fs::write(
        tempdir.path().join("vac-rs/core/src/lib.rs"),
        "pub mod control_plane;\npub mod doctor_cli;\n",
    )
    .expect("core lib");
    fs::write(tempdir.path().join("vac-rs/core/src/control_plane.rs"), "").expect("cp rs");
    fs::write(tempdir.path().join("vac-rs/core/src/doctor_cli.rs"), "").expect("doctor rs");

    fs::write(
        vac_root.join("capabilities/ownership.yaml"),
        r#"
schema_version: 1
kind: capability
id: vac.ownership
title: Ownership
status: partial
reason: "Plan 02"
states:
  - empty
owner:
  crate: vac-core
  module: control_plane
ownership:
  targets:
    - crate_name: vac-core
      module: control_plane
      test_only: false
      retired: true
  deletion_plan: "Plan 021 cleanup"
surfaces:
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
    )
    .expect("ownership manifest");

    let report = load_ownership_scan_report(tempdir.path());
    let rendered = report.render_text();

    assert_eq!(report.target_count(), 1);
    assert_eq!(report.retired_target_count(), 1);
    assert!(rendered.contains("vac-core/control_plane"));
    assert!(rendered.contains("retired"));
    assert!(rendered.contains("targets=[vac-core::control_plane (retired)]"));
    assert!(rendered.contains("deletion_plan=\"Plan 021 cleanup\""));
    assert!(rendered.contains("vac-core/doctor_cli"));
    assert!(report.is_failure());
}
