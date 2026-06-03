#![cfg(feature = "full-cli")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn vac_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_vac")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/vac"))
}

fn make_temp_root(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vac-runtime-owner-gates-{name}-{unique}"));
    fs::create_dir_all(&root).expect("create temp root");
    root
}

fn write_workspace_manifest(root: &Path) {
    fs::create_dir_all(root.join("vac-rs")).expect("create vac-rs dir");
    fs::write(
        root.join("vac-rs/Cargo.toml"),
        r#"
[workspace]
members = ["local-runtime-owner"]

[workspace.dependencies]
vac-local-runtime-owner = { path = "local-runtime-owner" }
"#,
    )
    .expect("write workspace manifest");
}

fn seed_runtime_owner_sources(root: &Path) {
    fs::create_dir_all(root.join("vac-rs/local-runtime-owner/src")).expect("create source dir");
    fs::write(
        root.join("vac-rs/local-runtime-owner/src/lib.rs"),
        "pub mod command_bus;\n",
    )
    .expect("write lib");
    fs::write(
        root.join("vac-rs/local-runtime-owner/src/command_bus.rs"),
        "pub fn ping() {}\n",
    )
    .expect("write command bus");
}

fn seed_plan_doc(root: &Path) {
    fs::create_dir_all(root.join("docs/workflow-control-plane/plans")).expect("create docs dir");
    fs::write(
        root.join("docs/workflow-control-plane/plans/32-vac-runtime-owner-gates.md"),
        "# Plan 32\n",
    )
    .expect("write plan doc");
}

fn write_local_runtime_owner_manifest(root: &Path, body: &str) {
    fs::create_dir_all(root.join(".vac/capabilities")).expect("create capability dir");
    fs::write(
        root.join(".vac/capabilities/local_runtime_owner.yaml"),
        body,
    )
    .expect("write local runtime owner manifest");
}

fn valid_manifest() -> &'static str {
    r#"
schema_version: 1
kind: capability
id: vac.local_runtime_owner
title: Local Runtime Owner
status: partial
owner: vac-rs/local-runtime-owner
ownership:
  crates:
    - vac-local-runtime-owner
  targets:
    - crate_name: vac-local-runtime-owner
      module: lib
    - crate_name: vac-local-runtime-owner
      module: command_bus
policy:
  risk: safe_read
  mutates_files: false
validation:
  commands: []
docs:
  - docs/workflow-control-plane/plans/32-vac-runtime-owner-gates.md
"#
}

fn run_runtime_owner_gates(root: &Path) -> std::process::Output {
    Command::new(vac_bin())
        .args(["doctor", "runtime-owner-gates"])
        .arg(root)
        .output()
        .expect("run vac doctor runtime-owner-gates")
}

#[test]
fn runtime_owner_gates_happy_path_is_green() {
    let root = make_temp_root("happy");
    write_workspace_manifest(&root);
    seed_runtime_owner_sources(&root);
    seed_plan_doc(&root);
    write_local_runtime_owner_manifest(&root, valid_manifest());

    let output = run_runtime_owner_gates(&root);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(stdout.contains("status: green"), "stdout:\n{stdout}");
    assert!(stdout.contains("manifests_checked=1"), "stdout:\n{stdout}");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn runtime_owner_gates_fails_missing_schema_field() {
    let root = make_temp_root("missing-field");
    write_workspace_manifest(&root);
    seed_runtime_owner_sources(&root);
    seed_plan_doc(&root);
    write_local_runtime_owner_manifest(
        &root,
        &valid_manifest().replace("title: Local Runtime Owner\n", ""),
    );

    let output = run_runtime_owner_gates(&root);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(stdout.contains("status: fail"), "stdout:\n{stdout}");
    assert!(stdout.contains("level: error"), "stdout:\n{stdout}");
    assert!(stdout.contains("code: missing_field"), "stdout:\n{stdout}");
    assert!(stdout.contains("field `title`"), "stdout:\n{stdout}");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn runtime_owner_gates_reports_missing_source_domain_as_warning() {
    let root = make_temp_root("missing-source");
    write_workspace_manifest(&root);
    seed_plan_doc(&root);
    write_local_runtime_owner_manifest(&root, valid_manifest());

    let output = run_runtime_owner_gates(&root);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(stdout.contains("status: fail"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("code: missing_source_domain"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("owner claims `vac-rs/local-runtime-owner`"),
        "stdout:\n{stdout}"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn runtime_owner_gates_fails_when_required_manifest_is_absent() {
    let root = make_temp_root("missing-required");
    write_workspace_manifest(&root);

    let output = run_runtime_owner_gates(&root);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!output.status.success(), "stdout:\n{stdout}");
    assert!(stdout.contains("status: fail"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("code: missing_required_manifest"),
        "stdout:\n{stdout}"
    );

    let _ = fs::remove_dir_all(&root);
}

fn write_tui_manifest_with_app_server_dependency(root: &Path) {
    fs::create_dir_all(root.join("vac-rs/tui")).expect("create tui dir");
    fs::write(
        root.join("vac-rs/tui/Cargo.toml"),
        r#"
[package]
name = "vac-tui"

[dependencies]
vac-app-server-client = { workspace = true }
"#,
    )
    .expect("write tui manifest");
}

fn write_pty_blocked_operator_evidence(root: &Path) {
    fs::create_dir_all(root.join("docs/workflow-control-plane/plans/23-evidence"))
        .expect("create pty evidence dir");
    fs::write(
        root.join("docs/workflow-control-plane/plans/23-evidence/blocked.md"),
        "# PTY evidence\n\nResult: BLOCKED-OPERATOR\n\nNo real TTY was available.\n",
    )
    .expect("write pty evidence");
}

#[test]
fn runtime_owner_gates_fails_on_app_server_dependency_regression() {
    let root = make_temp_root("app-server-dependency");
    write_workspace_manifest(&root);
    seed_runtime_owner_sources(&root);
    seed_plan_doc(&root);
    write_local_runtime_owner_manifest(&root, valid_manifest());
    write_tui_manifest_with_app_server_dependency(&root);

    let output = run_runtime_owner_gates(&root);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(stdout.contains("status: fail"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("code: app_server_dependency_present"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("vac-app-server-client"),
        "stdout:\n{stdout}"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn runtime_owner_gates_fails_on_message_processor_copy() {
    let root = make_temp_root("message-processor-copy");
    write_workspace_manifest(&root);
    seed_runtime_owner_sources(&root);
    seed_plan_doc(&root);
    write_local_runtime_owner_manifest(&root, valid_manifest());
    fs::write(
        root.join("vac-rs/local-runtime-owner/src/command_bus.rs"),
        "pub struct MessageProcessor;\n",
    )
    .expect("write copied message processor");

    let output = run_runtime_owner_gates(&root);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!output.status.success(), "stdout:\n{stdout}");
    assert!(stdout.contains("status: fail"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("code: message_processor_copy"),
        "stdout:\n{stdout}"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn runtime_owner_gates_warns_when_pty_evidence_is_blocked_operator() {
    let root = make_temp_root("pty-blocked-operator");
    write_workspace_manifest(&root);
    seed_runtime_owner_sources(&root);
    seed_plan_doc(&root);
    write_local_runtime_owner_manifest(&root, valid_manifest());
    write_pty_blocked_operator_evidence(&root);

    let output = run_runtime_owner_gates(&root);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(stdout.contains("status: warning"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("code: pty_false_green"),
        "stdout:\n{stdout}"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn runtime_owner_gates_warns_on_unsupported_default_control_defer() {
    let root = make_temp_root("unsupported-control");
    write_workspace_manifest(&root);
    seed_runtime_owner_sources(&root);
    seed_plan_doc(&root);
    write_local_runtime_owner_manifest(&root, valid_manifest());
    fs::write(
        root.join("vac-rs/local-runtime-owner/src/command_bus.rs"),
        "pub enum RuntimeCommandBusError { PluginSurfaceOwnerProviderRequired }\n",
    )
    .expect("write unsupported control sentinel");

    let output = run_runtime_owner_gates(&root);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(stdout.contains("status: warning"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("code: unsupported_control_default_defer"),
        "stdout:\n{stdout}"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn runtime_owner_gates_fails_missing_ownership_target_crate_name() {
    let root = make_temp_root("missing-target-crate-name");
    write_workspace_manifest(&root);
    seed_runtime_owner_sources(&root);
    seed_plan_doc(&root);
    write_local_runtime_owner_manifest(
        &root,
        &valid_manifest().replace(
            "    - crate_name: vac-local-runtime-owner\n      module: command_bus\n",
            "    - module: command_bus\n",
        ),
    );

    let output = run_runtime_owner_gates(&root);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!output.status.success(), "stdout:\n{stdout}");
    assert!(stdout.contains("status: fail"), "stdout:\n{stdout}");
    assert!(stdout.contains("level: error"), "stdout:\n{stdout}");
    assert!(stdout.contains("code: missing_field"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("ownership target is missing `crate_name`"),
        "stdout:\n{stdout}"
    );

    let _ = fs::remove_dir_all(&root);
}

fn write_tui_source_with_app_server_import(root: &Path) {
    fs::create_dir_all(root.join("vac-rs/tui/src")).expect("create tui source dir");
    fs::write(
        root.join("vac-rs/tui/src/local_runtime_session.rs"),
        "use vac_app_server_client::Client;\n",
    )
    .expect("write tui source import");
}

#[test]
fn runtime_owner_gates_fails_on_active_app_server_source_import() {
    let root = make_temp_root("app-server-source-import");
    write_workspace_manifest(&root);
    seed_runtime_owner_sources(&root);
    seed_plan_doc(&root);
    write_local_runtime_owner_manifest(&root, valid_manifest());
    write_tui_source_with_app_server_import(&root);

    let output = run_runtime_owner_gates(&root);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(stdout.contains("status: fail"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("code: app_server_import_present"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("vac_app_server_client"),
        "stdout:\n{stdout}"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn runtime_owner_gates_fails_when_ownership_target_module_is_missing() {
    let root = make_temp_root("missing-target-module");
    write_workspace_manifest(&root);
    seed_runtime_owner_sources(&root);
    seed_plan_doc(&root);
    let manifest = valid_manifest().replace("      module: command_bus\n", "");
    write_local_runtime_owner_manifest(&root, &manifest);

    let output = run_runtime_owner_gates(&root);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(stdout.contains("status: fail"), "stdout:\n{stdout}");
    assert!(stdout.contains("code: missing_field"), "stdout:\n{stdout}");
    assert!(stdout.contains("missing `module`"), "stdout:\n{stdout}");

    let _ = fs::remove_dir_all(&root);
}
