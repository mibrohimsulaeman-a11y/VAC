use serde_yaml::Value;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

const RUNTIME_OWNER_SUBCOMMAND: &str = "runtime-owner-gates";

const MANIFEST_SPECS: &[ManifestSpec] = &[
    ManifestSpec {
        path: ".vac/capabilities/local_runtime_owner.yaml",
        required: true,
    },
    ManifestSpec {
        path: ".vac/capabilities/tui_session_runtime.yaml",
        required: false,
    },
    ManifestSpec {
        path: ".vac/capabilities/runtime_approval_bridge.yaml",
        required: false,
    },
    ManifestSpec {
        path: ".vac/workflows/maintenance.runtime-owner-gate.yaml",
        required: false,
    },
    ManifestSpec {
        path: ".vac/workflows/maintenance.no-app-server-local-path.yaml",
        required: false,
    },
    ManifestSpec {
        path: ".vac/policies/runtime-owner-replacement.yaml",
        required: false,
    },
];

const REQUIRED_FIELDS: &[&str] = &[
    "schema_version",
    "kind",
    "id",
    "title",
    "status",
    "owner",
    "ownership",
    "policy",
    "validation",
];

const OWNER_NATIVE_SUPPORT_MANIFEST: &str = ".vac/registry/runtime/owner-native-support.yaml";

const CRITICAL_OWNER_NATIVE_METHODS: &[&str] = &[
    "start_thread_with_session_start_source",
    "turn_start",
    "turn_steer",
    "turn_interrupt",
    "startup_interrupt",
    "thread_shell_command",
    "thread_list",
    "thread_read",
    "resume_thread",
    "branch_thread",
    "read_account",
];

const APP_SERVER_DEPENDENCY_CRATES: &[&str] = &[
    "vac-app-server",
    "vac-app-server-client",
    "vac-app-server-protocol",
    "vac-app-server-transport",
];

const APP_SERVER_WATCHED_MANIFESTS: &[&str] = &[
    "vac-rs/cli/Cargo.toml",
    "vac-rs/core/Cargo.toml",
    "vac-rs/exec/Cargo.toml",
    "vac-rs/local-runtime-owner/Cargo.toml",
    "vac-rs/tui/Cargo.toml",
];

const APP_SERVER_SOURCE_SCAN_PATHS: &[&str] = &[
    "vac-rs/local-runtime-owner/src",
    "vac-rs/tui/src/local_runtime_session.rs",
    "vac-rs/tui/src/session_protocol.rs",
    "vac-rs/tui/src/runtime_owner_session.rs",
    "vac-rs/exec/src/runtime_adapter.rs",
];

const APP_SERVER_IMPORT_PATTERNS: &[&str] = &[
    "vac_app_server",
    "vac_app_server_client",
    "AppServerSession",
];

const MESSAGE_PROCESSOR_SCAN_PATHS: &[&str] = &[
    "vac-rs/local-runtime-owner/src",
    "vac-rs/exec/src/runtime_adapter.rs",
    "vac-rs/tui/src/local_runtime_session.rs",
];

const MESSAGE_PROCESSOR_COPY_PATTERNS: &[&str] = &[
    "struct MessageProcessor",
    "enum MessageProcessor",
    "type MessageProcessor",
    "impl MessageProcessor",
    "MessageProcessor::",
];

const PTY_EVIDENCE_DIRS: &[&str] = &[
    "docs/workflow-control-plane/plans/23-evidence",
    "docs/workflow-control-plane/plans/30-evidence",
];

const UNSUPPORTED_CONTROL_SCAN_PATHS: &[&str] = &[
    "vac-rs/local-runtime-owner/src/command_bus.rs",
    "vac-rs/tui/src/local_runtime_session.rs",
];

const UNSUPPORTED_CONTROL_NONDEFAULT_DEFER_MARKER: &str =
    "VAC_RUNTIME_OWNER_NONDEFAULT_DEFER_ACCEPTED: plan30-owner-native-default-parity";

const UNSUPPORTED_CONTROL_PATTERNS: &[(&str, &str)] = &[
    (
        "PluginSurfaceOwnerProviderRequired",
        "plugin owner provider still fails closed instead of completing default local runtime parity",
    ),
    (
        "ExternalAgentConfigBackgroundImportRequired",
        "external-agent import still has explicit background completion blocker semantics",
    ),
    (
        "UnsupportedControl",
        "owner path still exposes an unsupported control sentinel",
    ),
];

#[derive(Debug, Clone, Copy)]
struct ManifestSpec {
    path: &'static str,
    required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum FindingLevel {
    Warning,
    Error,
}

impl FindingLevel {
    fn as_str(self) -> &'static str {
        match self {
            FindingLevel::Warning => "warning",
            FindingLevel::Error => "error",
        }
    }
}

#[derive(Debug, Clone)]
struct RuntimeOwnerFinding {
    level: FindingLevel,
    code: &'static str,
    manifest: Option<String>,
    detail: String,
}

#[derive(Debug, Default)]
struct RuntimeOwnerGateReport {
    root: PathBuf,
    checked_manifests: Vec<String>,
    findings: Vec<RuntimeOwnerFinding>,
}

impl RuntimeOwnerGateReport {
    fn push(
        &mut self,
        level: FindingLevel,
        code: &'static str,
        manifest: Option<&str>,
        detail: impl Into<String>,
    ) {
        self.findings.push(RuntimeOwnerFinding {
            level,
            code,
            manifest: manifest.map(str::to_owned),
            detail: detail.into(),
        });
    }

    fn warning_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.level == FindingLevel::Warning)
            .count()
    }

    fn error_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.level == FindingLevel::Error)
            .count()
    }

    fn status_label(&self) -> &'static str {
        if self.error_count() > 0 {
            "fail"
        } else if self.warning_count() > 0 {
            "warning"
        } else {
            "green"
        }
    }

    fn cli_exit_code(&self) -> i32 {
        if self.error_count() > 0 { 1 } else { 0 }
    }

    fn render_text(&self) -> String {
        let mut out = String::new();
        out.push_str("VAC Runtime Owner Gates Diagnostics\n");
        out.push_str("===================================\n");
        out.push_str(&format!("root: {}\n", self.root.display()));
        out.push_str(&format!("status: {}\n", self.status_label()));
        out.push_str(&format!(
            "summary: manifests_checked={} warnings={} errors={}\n",
            self.checked_manifests.len(),
            self.warning_count(),
            self.error_count()
        ));
        out.push_str("mode: hard-gated (missing fields/source domains, retired app-server regressions, false-green PTY, unsupported default controls)\n");
        out.push_str("manifests:\n");
        if self.checked_manifests.is_empty() {
            out.push_str("  - (none)\n");
        } else {
            for manifest in &self.checked_manifests {
                out.push_str(&format!("  - {}\n", manifest));
            }
        }
        out.push_str("findings:\n");
        if self.findings.is_empty() {
            out.push_str("  - level: info\n");
            out.push_str("    code: runtime_owner_gates_green\n");
            out.push_str("    detail: Plan 32 runtime-owner manifests loaded without warnings.\n");
        } else {
            for finding in &self.findings {
                out.push_str(&format!("  - level: {}\n", finding.level.as_str()));
                out.push_str(&format!("    code: {}\n", finding.code));
                if let Some(manifest) = &finding.manifest {
                    out.push_str(&format!("    manifest: {}\n", manifest));
                }
                out.push_str(&format!("    detail: {}\n", finding.detail));
            }
        }
        out
    }
}

pub(super) fn run_external_subcommand(args: Vec<OsString>) -> anyhow::Result<i32> {
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        eprintln!("missing doctor subcommand");
        return Ok(2);
    };

    if command.to_string_lossy() != RUNTIME_OWNER_SUBCOMMAND {
        eprintln!("unknown doctor subcommand `{}`", command.to_string_lossy());
        return Ok(2);
    }

    let root = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    if let Some(extra) = args.next() {
        eprintln!(
            "unexpected extra argument for `vac doctor runtime-owner-gates`: {}",
            extra.to_string_lossy()
        );
        return Ok(2);
    }

    let report = load_runtime_owner_gate_report(&root);
    println!("{}", report.render_text());
    Ok(report.cli_exit_code())
}

fn load_runtime_owner_gate_report(root: &Path) -> RuntimeOwnerGateReport {
    let root = root.to_path_buf();
    let crate_paths = load_workspace_crate_paths(&root);
    let mut report = RuntimeOwnerGateReport {
        root: root.clone(),
        checked_manifests: Vec::new(),
        findings: Vec::new(),
    };

    for spec in MANIFEST_SPECS {
        let manifest_path = root.join(spec.path);
        if !manifest_path.exists() {
            if spec.required {
                report.push(
                    FindingLevel::Error,
                    "missing_required_manifest",
                    Some(spec.path),
                    format!("required Plan 32 manifest `{}` does not exist", spec.path),
                );
            }
            continue;
        }

        report.checked_manifests.push(spec.path.to_owned());
        let Ok(raw) = fs::read_to_string(&manifest_path) else {
            report.push(
                FindingLevel::Error,
                "manifest_read_failed",
                Some(spec.path),
                format!("failed to read `{}`", spec.path),
            );
            continue;
        };

        let manifest = match serde_yaml::from_str::<Value>(&raw) {
            Ok(value) => value,
            Err(err) => {
                report.push(
                    FindingLevel::Error,
                    "manifest_yaml_invalid",
                    Some(spec.path),
                    format!("failed to parse YAML: {err}"),
                );
                continue;
            }
        };

        validate_required_fields(&mut report, spec.path, &manifest);
        validate_manifest_source_domains(&mut report, spec.path, &manifest, &crate_paths);
    }

    validate_app_server_dependency_regressions(&mut report);
    validate_app_server_source_regressions(&mut report);
    validate_message_processor_copy_regressions(&mut report);
    validate_pty_false_green_evidence(&mut report);
    validate_unsupported_control_default_defers(&mut report);
    validate_owner_native_support_manifest(&mut report);

    report
}

fn validate_owner_native_support_manifest(report: &mut RuntimeOwnerGateReport) {
    let path = report.root.join(OWNER_NATIVE_SUPPORT_MANIFEST);
    if !path.exists() {
        report.push(
            FindingLevel::Error,
            "missing_owner_native_support_manifest",
            Some(OWNER_NATIVE_SUPPORT_MANIFEST),
            "runtime owner support manifest is required so init/doctor readiness does not depend on brittle source text scans",
        );
        return;
    }
    report
        .checked_manifests
        .push(OWNER_NATIVE_SUPPORT_MANIFEST.to_string());
    let Ok(raw) = fs::read_to_string(&path) else {
        report.push(
            FindingLevel::Error,
            "owner_native_support_read_failed",
            Some(OWNER_NATIVE_SUPPORT_MANIFEST),
            "failed to read owner-native support manifest",
        );
        return;
    };
    let manifest = match serde_yaml::from_str::<Value>(&raw) {
        Ok(value) => value,
        Err(err) => {
            report.push(
                FindingLevel::Error,
                "owner_native_support_yaml_invalid",
                Some(OWNER_NATIVE_SUPPORT_MANIFEST),
                format!("failed to parse YAML: {err}"),
            );
            return;
        }
    };
    if map_get(&manifest, "kind").and_then(Value::as_str) != Some("runtime_owner_support") {
        report.push(
            FindingLevel::Error,
            "owner_native_support_kind_invalid",
            Some(OWNER_NATIVE_SUPPORT_MANIFEST),
            "expected kind: runtime_owner_support",
        );
    }
    if map_get(&manifest, "status").and_then(Value::as_str) != Some("ready") {
        report.push(
            FindingLevel::Error,
            "owner_native_support_not_ready",
            Some(OWNER_NATIVE_SUPPORT_MANIFEST),
            "local_runtime_owner cannot be Ready unless owner-native support status is ready",
        );
    }
    let methods = map_get(&manifest, "critical_methods")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();
    for method in CRITICAL_OWNER_NATIVE_METHODS {
        let entry = methods
            .iter()
            .find(|entry| map_get(entry, "name").and_then(Value::as_str) == Some(*method));
        match entry {
            Some(entry) => {
                if map_get(entry, "status").and_then(Value::as_str) != Some("implemented") {
                    report.push(
                        FindingLevel::Error,
                        "owner_native_method_not_implemented",
                        Some(OWNER_NATIVE_SUPPORT_MANIFEST),
                        format!(
                            "critical owner-native method `{method}` must be status: implemented"
                        ),
                    );
                }
                if map_get(entry, "release_blocking").and_then(Value::as_bool) != Some(true) {
                    report.push(
                        FindingLevel::Error,
                        "owner_native_method_not_release_blocking",
                        Some(OWNER_NATIVE_SUPPORT_MANIFEST),
                        format!(
                            "critical owner-native method `{method}` must be release_blocking: true"
                        ),
                    );
                }
            }
            None => report.push(
                FindingLevel::Error,
                "owner_native_method_missing",
                Some(OWNER_NATIVE_SUPPORT_MANIFEST),
                format!(
                    "critical owner-native method `{method}` is missing from the support manifest"
                ),
            ),
        }
    }

    let source_contract = map_get(&manifest, "source_contract")
        .and_then(Value::as_str)
        .unwrap_or("");
    let source_contract_path = report.root.join(source_contract);
    if source_contract.is_empty() || !source_contract_path.exists() {
        report.push(
            FindingLevel::Error,
            "owner_native_support_contract_missing",
            Some(OWNER_NATIVE_SUPPORT_MANIFEST),
            "runtime support manifest must point at vac-rs/tui/src/owner_native_runtime_support.rs",
        );
    } else if let Ok(contract) = fs::read_to_string(&source_contract_path) {
        for method in CRITICAL_OWNER_NATIVE_METHODS {
            if !contract.contains(&format!("\"{method}\"")) {
                report.push(
                    FindingLevel::Error,
                    "owner_native_contract_method_missing",
                    Some(source_contract),
                    format!("critical owner-native method `{method}` is absent from the code-owned support contract"),
                );
            }
        }
        for required in [
            "OWNER_RUNTIME_METHOD_SUPPORT",
            "release_blocking_owner_runtime_methods",
            "OwnerRuntimeMethodStatus::Implemented",
            "OwnerRuntimeMethodSupport::fail_closed(\"thread_rollback\"",
            "OwnerRuntimeMethodSupport::fail_closed(\"resolve_server_request\"",
            "OwnerRuntimeMethodSupport::fail_closed(\"reject_server_request\"",
        ] {
            if !contract.contains(required) {
                report.push(
                    FindingLevel::Error,
                    "owner_native_support_contract_anchor_missing",
                    Some(source_contract),
                    format!("owner-native support contract anchor `{required}` is absent"),
                );
            }
        }
    }

    let parity_registry = map_get(&manifest, "parity_registry")
        .and_then(Value::as_str)
        .unwrap_or("");
    let parity_path = report.root.join(parity_registry);
    if parity_registry.is_empty() || !parity_path.exists() {
        report.push(
            FindingLevel::Error,
            "owner_native_parity_registry_missing",
            Some(OWNER_NATIVE_SUPPORT_MANIFEST),
            "runtime support manifest must point at vac-rs/tui/src/owner_native_operation_parity.rs",
        );
    } else if let Ok(parity) = fs::read_to_string(&parity_path) {
        for required in [
            "OwnerNativeOperationStatus::NonDefaultFailClosed",
            "plan30_non_default_fail_closed_is_limited_to_noncritical_controls",
            "thread_rollback",
            "resolve_server_request",
            "reject_server_request",
        ] {
            if !parity.contains(required) {
                report.push(
                    FindingLevel::Error,
                    "owner_native_parity_anchor_missing",
                    Some(parity_registry),
                    format!("owner-native parity anchor `{required}` is absent"),
                );
            }
        }
    }
}

fn validate_app_server_dependency_regressions(report: &mut RuntimeOwnerGateReport) {
    let root = report.root.clone();
    for manifest in APP_SERVER_WATCHED_MANIFESTS {
        let path = root.join(manifest);
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        for crate_name in APP_SERVER_DEPENDENCY_CRATES {
            if manifest_mentions_default_dependency(&raw, crate_name) {
                report.push(
                    FindingLevel::Error,
                    "app_server_dependency_present",
                    None,
                    format!(
                        "watched runtime-owner manifest `{manifest}` still has retired dependency `{crate_name}`; Epic A removes app-server compatibility from the product path"
                    ),
                );
            } else if manifest_mentions_optional_dependency(&raw, crate_name) {
                report.push(
                    FindingLevel::Error,
                    "app_server_dependency_present",
                    None,
                    format!(
                        "watched runtime-owner manifest `{manifest}` still has optional retired dependency `{crate_name}`; Epic A retires app-server compatibility entirely for the product path"
                    ),
                );
            }
        }
    }
}

fn validate_app_server_source_regressions(report: &mut RuntimeOwnerGateReport) {
    let root = report.root.clone();
    for scan_path in APP_SERVER_SOURCE_SCAN_PATHS {
        let path = root.join(scan_path);
        if path.is_dir() {
            visit_rs_files(&path, &mut |file| {
                scan_app_server_source_file(report, scan_path, file);
            });
        } else if path.is_file() {
            scan_app_server_source_file(report, scan_path, &path);
        }
    }
}

fn validate_message_processor_copy_regressions(report: &mut RuntimeOwnerGateReport) {
    let root = report.root.clone();
    for scan_path in MESSAGE_PROCESSOR_SCAN_PATHS {
        let path = root.join(scan_path);
        if path.is_dir() {
            visit_rs_files(&path, &mut |file| {
                scan_message_processor_file(report, scan_path, file);
            });
        } else if path.is_file() {
            scan_message_processor_file(report, scan_path, &path);
        }
    }
}

fn validate_pty_false_green_evidence(report: &mut RuntimeOwnerGateReport) {
    let root = report.root.clone();
    for evidence_dir in PTY_EVIDENCE_DIRS {
        let path = root.join(evidence_dir);
        if !path.is_dir() {
            continue;
        }
        visit_markdown_files(&path, &mut |file| {
            let Ok(raw) = fs::read_to_string(file) else {
                return;
            };
            if blocked_operator_claim(&raw) && pass_claim(&raw) {
                report.push(
                    FindingLevel::Error,
                    "pty_false_green",
                    None,
                    format!(
                        "PTY evidence `{}` contains both BLOCKED-OPERATOR and pass/green language",
                        file.strip_prefix(&root).unwrap_or(file).display()
                    ),
                );
            } else if blocked_operator_claim(&raw) && !real_pty_pass_claim(&raw) {
                report.push(
                    FindingLevel::Error,
                    "pty_false_green",
                    None,
                    format!(
                        "PTY evidence `{}` is BLOCKED-OPERATOR; runtime-owner gates must not treat it as release pass evidence",
                        file.strip_prefix(&root)
                            .unwrap_or(file)
                            .display()
                    ),
                );
            }
        });
    }
}

fn validate_unsupported_control_default_defers(report: &mut RuntimeOwnerGateReport) {
    let root = report.root.clone();
    for scan_path in UNSUPPORTED_CONTROL_SCAN_PATHS {
        let path = root.join(scan_path);
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        for (pattern, detail) in UNSUPPORTED_CONTROL_PATTERNS {
            if raw.contains(pattern) {
                if raw.contains(UNSUPPORTED_CONTROL_NONDEFAULT_DEFER_MARKER) {
                    continue;
                }
                report.push(
                    FindingLevel::Error,
                    "unsupported_control_default_defer",
                    None,
                    format!(
                        "{} contains `{pattern}`: {detail}; default owner-native parity must not rely on unsupported controls",
                        scan_path
                    ),
                );
            }
        }
    }
}

fn manifest_dependency_line<'a>(raw: &'a str, crate_name: &str) -> Option<&'a str> {
    raw.lines().map(str::trim).find(|line| {
        !line.starts_with('#')
            && line
                .split_once('=')
                .map(|(name, _)| name.trim().trim_matches('"') == crate_name)
                .unwrap_or(false)
    })
}

fn manifest_mentions_default_dependency(raw: &str, crate_name: &str) -> bool {
    manifest_dependency_line(raw, crate_name)
        .map(|line| !line.contains("optional = true"))
        .unwrap_or(false)
}

fn manifest_mentions_optional_dependency(raw: &str, crate_name: &str) -> bool {
    manifest_dependency_line(raw, crate_name)
        .map(|line| line.contains("optional = true"))
        .unwrap_or(false)
}

fn visit_rs_files(path: &Path, visit: &mut impl FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit_rs_files(&path, visit);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            visit(&path);
        }
    }
}

fn visit_markdown_files(path: &Path, visit: &mut impl FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit_markdown_files(&path, visit);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("md") {
            visit(&path);
        }
    }
}

fn scan_app_server_source_file(report: &mut RuntimeOwnerGateReport, scan_path: &str, file: &Path) {
    let Ok(raw) = fs::read_to_string(file) else {
        return;
    };

    for (line_number, line) in raw.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }
        if let Some(pattern) = APP_SERVER_IMPORT_PATTERNS
            .iter()
            .find(|pattern| trimmed.contains(**pattern))
        {
            report.push(
                FindingLevel::Error,
                "app_server_import_present",
                None,
                format!(
                    "active runtime-owner scan path `{scan_path}` still references `{pattern}` in `{}` line {}; Epic A retires app-server compatibility from the product path",
                    file.strip_prefix(&report.root)
                        .unwrap_or(file)
                        .display(),
                    line_number + 1
                ),
            );
        }
    }
}

fn scan_message_processor_file(report: &mut RuntimeOwnerGateReport, scan_path: &str, file: &Path) {
    let Ok(raw) = fs::read_to_string(file) else {
        return;
    };
    for (line_number, line) in raw.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }
        if let Some(pattern) = MESSAGE_PROCESSOR_COPY_PATTERNS
            .iter()
            .find(|pattern| trimmed.contains(**pattern))
        {
            report.push(
                FindingLevel::Error,
                "message_processor_copy",
                None,
                format!(
                    "active runtime-owner scan path `{scan_path}` copies `{pattern}` in `{}` line {}; keep MessageProcessor quarantined instead",
                    file.strip_prefix(&report.root)
                        .unwrap_or(file)
                        .display(),
                    line_number + 1
                ),
            );
        }
    }
}

fn blocked_operator_claim(raw: &str) -> bool {
    raw.contains("BLOCKED-OPERATOR") || raw.contains("blocked_operator")
}

fn pass_claim(raw: &str) -> bool {
    raw.lines().any(|line| {
        let normalized = line.trim().to_ascii_lowercase();
        (normalized.starts_with("result:")
            && (normalized.contains("pass") || normalized.contains("passed"))
            && !normalized.contains('|'))
            || normalized == "status: green"
            || normalized == "result_state: passed"
    })
}

fn real_pty_pass_claim(raw: &str) -> bool {
    raw.lines().any(|line| {
        let normalized = line.trim().to_ascii_lowercase();
        normalized.contains("real")
            && normalized.contains("pty")
            && (normalized.contains("pass") || normalized.contains("passed"))
    })
}

fn validate_required_fields(
    report: &mut RuntimeOwnerGateReport,
    manifest_path: &str,
    manifest: &Value,
) {
    for field in REQUIRED_FIELDS {
        if map_get(manifest, field).is_none() {
            report.push(
                FindingLevel::Error,
                "missing_field",
                Some(manifest_path),
                format!("schema-mandated field `{field}` is absent"),
            );
        }
    }
}

fn validate_manifest_source_domains(
    report: &mut RuntimeOwnerGateReport,
    manifest_path: &str,
    manifest: &Value,
    crate_paths: &BTreeMap<String, PathBuf>,
) {
    if let Some(owner) = map_get(manifest, "owner").and_then(Value::as_str) {
        validate_path_claim(report, manifest_path, "owner", owner);
    }

    if let Some(docs) = map_get(manifest, "docs").and_then(Value::as_sequence) {
        for doc in docs.iter().filter_map(Value::as_str) {
            validate_path_claim(report, manifest_path, "docs", doc);
        }
    }

    if let Some(compatibility) =
        map_get(manifest, "compatibility_transport").and_then(Value::as_sequence)
    {
        for entry in compatibility {
            if let Some(owner) = map_get(entry, "owner").and_then(Value::as_str) {
                validate_path_claim(
                    report,
                    manifest_path,
                    "compatibility_transport.owner",
                    owner,
                );
            }
            if let Some(plan) = map_get(entry, "target_removal_plan").and_then(Value::as_str) {
                validate_path_claim(
                    report,
                    manifest_path,
                    "compatibility_transport.target_removal_plan",
                    plan,
                );
            }
        }
    }

    let Some(ownership) = map_get(manifest, "ownership") else {
        return;
    };

    if let Some(crates) = map_get(ownership, "crates").and_then(Value::as_sequence) {
        for crate_name in crates.iter().filter_map(Value::as_str) {
            if !crate_paths.contains_key(crate_name) {
                report.push(
                    FindingLevel::Error,
                    "missing_source_domain",
                    Some(manifest_path),
                    format!("ownership crate `{crate_name}` is not declared in vac-rs/Cargo.toml"),
                );
            }
        }
    }

    if let Some(targets) = map_get(ownership, "targets").and_then(Value::as_sequence) {
        for target in targets {
            let crate_name = map_get(target, "crate_name").and_then(Value::as_str);
            let module = map_get(target, "module").and_then(Value::as_str);
            match (crate_name, module) {
                (Some(crate_name), Some(module)) => {
                    validate_crate_module_claim(
                        report,
                        manifest_path,
                        crate_name,
                        module,
                        crate_paths,
                    );
                }
                (Some(crate_name), None) => {
                    if !crate_paths.contains_key(crate_name) {
                        report.push(
                            FindingLevel::Error,
                            "missing_source_domain",
                            Some(manifest_path),
                            format!("ownership target crate `{crate_name}` is not declared in vac-rs/Cargo.toml"),
                        );
                    }
                    report.push(
                        FindingLevel::Error,
                        "missing_field",
                        Some(manifest_path),
                        format!("ownership target for crate `{crate_name}` is missing `module`"),
                    );
                }
                (None, _) => report.push(
                    FindingLevel::Error,
                    "missing_field",
                    Some(manifest_path),
                    "ownership target is missing `crate_name`",
                ),
            }
        }
    }
}

fn validate_path_claim(
    report: &mut RuntimeOwnerGateReport,
    manifest_path: &str,
    field: &str,
    raw: &str,
) {
    if !looks_like_repo_path(raw) {
        return;
    }

    let root = report.root.clone();
    let candidate = root.join(raw);
    if !candidate.exists() {
        report.push(
            FindingLevel::Error,
            "missing_source_domain",
            Some(manifest_path),
            format!("{field} claims `{raw}`, but that path does not exist on disk"),
        );
    }
}

fn validate_crate_module_claim(
    report: &mut RuntimeOwnerGateReport,
    manifest_path: &str,
    crate_name: &str,
    module: &str,
    crate_paths: &BTreeMap<String, PathBuf>,
) {
    let Some(crate_path) = crate_paths.get(crate_name) else {
        report.push(
            FindingLevel::Error,
            "missing_source_domain",
            Some(manifest_path),
            format!("ownership target crate `{crate_name}` is not declared in vac-rs/Cargo.toml"),
        );
        return;
    };

    let candidates = module_file_candidates(crate_path, module);
    if candidates.iter().any(|path| path.exists()) {
        return;
    }

    let rendered_candidates = candidates
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    report.push(
        FindingLevel::Error,
        "missing_source_domain",
        Some(manifest_path),
        format!(
            "ownership target `{crate_name}::{module}` did not match an on-disk module ({rendered_candidates})"
        ),
    );
}

fn module_file_candidates(crate_path: &Path, module: &str) -> Vec<PathBuf> {
    if module == "lib" {
        return vec![
            crate_path.join("src/lib.rs"),
            crate_path.join("src/main.rs"),
        ];
    }

    let module_path = module.replace('.', "/");
    vec![
        crate_path.join("src").join(format!("{module_path}.rs")),
        crate_path.join("src").join(&module_path).join("mod.rs"),
    ]
}

fn looks_like_repo_path(raw: &str) -> bool {
    if raw.contains(' ') || raw.is_empty() {
        return false;
    }

    raw.contains('/')
        || raw.starts_with('.')
        || raw.ends_with(".rs")
        || raw.ends_with(".md")
        || raw.ends_with(".yaml")
        || raw.ends_with(".toml")
}

fn map_get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let Value::Mapping(mapping) = value else {
        return None;
    };
    mapping.get(Value::String(key.to_owned()))
}

fn load_workspace_crate_paths(root: &Path) -> BTreeMap<String, PathBuf> {
    let mut crate_paths = BTreeMap::new();
    let manifest = root.join("vac-rs/Cargo.toml");
    let Ok(raw) = fs::read_to_string(&manifest) else {
        return crate_paths;
    };

    for line in raw.lines().map(str::trim) {
        let Some((name, rest)) = line.split_once('=') else {
            continue;
        };
        if !rest.contains("path") {
            continue;
        }
        let Some(path) = extract_path_value(rest) else {
            continue;
        };
        let crate_name = name.trim().trim_matches('"');
        if crate_name.starts_with("vac-") {
            crate_paths.insert(crate_name.to_owned(), root.join("vac-rs").join(path));
        }
    }

    crate_paths
}

fn extract_path_value(raw: &str) -> Option<&str> {
    let path_index = raw.find("path")?;
    let after_path = &raw[path_index..];
    let first_quote = after_path.find('"')?;
    let after_first_quote = &after_path[first_quote + 1..];
    let second_quote = after_first_quote.find('"')?;
    Some(&after_first_quote[..second_quote])
}
