use clap::Subcommand;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

#[derive(Subcommand, PartialEq, Debug)]
pub enum CompileCommands {
    /// Compile YAML authoring manifests into .vac/cache/compiled runtime JSON.
    Registry {
        /// Workspace root, defaults to current directory.
        path: Option<String>,
    },
}

pub async fn run_compile(command: CompileCommands) -> Result<(), String> {
    match command {
        CompileCommands::Registry { path } => compile_registry(path.as_deref().unwrap_or(".")),
    }
}

pub async fn run_doctor(gate: String, path: Option<String>) -> Result<(), String> {
    let root = PathBuf::from(path.unwrap_or_else(|| ".".to_string()));
    let result = match gate.as_str() {
        "registry" => doctor_registry(&root),
        "compiled" => doctor_compiled(&root),
        "runtime-db" => doctor_runtime_db(&root),
        "manifest-sync" => doctor_manifest_sync(&root),
        "index" => doctor_index(&root),
        "intent" => doctor_intent(&root),
        "ownership" => doctor_ownership(&root),
        "policy" => doctor_policy(&root),
        "workflow" => doctor_workflow(&root),
        "memory" => doctor_memory(&root),
        "assessment" => doctor_assessment(&root),
        "spec-sync" => doctor_spec_sync(&root),
        "evidence" => doctor_evidence(&root),
        "enforcement" => doctor_enforcement(&root),
        "release" => {
            let gates = [
                doctor_registry(&root),
                doctor_compiled(&root),
                doctor_index(&root),
                doctor_intent(&root),
                doctor_assessment(&root),
                doctor_spec_sync(&root),
                doctor_ownership(&root),
                doctor_policy(&root),
                doctor_evidence(&root),
                doctor_workflow(&root),
                doctor_memory(&root),
                doctor_enforcement(&root),
                doctor_runtime_db(&root),
                doctor_manifest_sync(&root),
            ];
            let failures = gates
                .iter()
                .filter(|item| vac_doctor::release_blocks_on(item))
                .map(|item| item.gate.clone())
                .collect::<Vec<_>>();
            vac_doctor::product_path_gate(
                "release",
                failures.is_empty(),
                failures
                    .iter()
                    .map(|gate| format!("release blocked by {gate}"))
                    .collect(),
            )
        }
        _ => vac_doctor::product_path_gate(&gate, false, vec!["unknown doctor gate".to_string()]),
    };
    if vac_doctor::release_blocks_on(&result) {
        Err(format!(
            "vac doctor {gate} {}: {:?} failures={:?}",
            root.display(),
            result.status,
            result.failures
        ))
    } else {
        println!(
            "vac doctor {gate} {}: {:?} warnings={:?}",
            root.display(),
            result.status,
            result.warnings
        );
        Ok(())
    }
}

pub async fn run_assess(path: Option<String>) -> Result<(), String> {
    let root = PathBuf::from(path.unwrap_or_else(|| ".".to_string()));
    let report = read_json(&root.join(".vac/assessment/gap_report.json"))?;
    let summary = vac_assessment::validate_gap_report_value(&report);
    println!(
        "vac assess {}: parser_mode={} p1={} blocking={} heuristic={} warnings={:?}",
        root.display(),
        summary.parser_mode,
        summary.p1_count,
        summary.blocking_count,
        summary.heuristic_severity,
        summary.warnings
    );
    Ok(())
}

pub async fn run_spec_sync(path: Option<String>) -> Result<(), String> {
    let root = PathBuf::from(path.unwrap_or_else(|| ".".to_string()));
    let report = read_json(&root.join(".vac/registry/spec-sync/bootstrap.json"))?;
    let _semantic_probe = vac_spec_sync::detect_symbol_invariant_drift(&[], &[]);
    println!(
        "vac spec-sync {}: report {} kind={}",
        root.display(),
        root.join(".vac/registry/spec-sync/bootstrap.json")
            .display(),
        report
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
    );
    Ok(())
}

fn doctor_registry(root: &Path) -> vac_doctor::DoctorResult {
    let failures = [
        ".vac/capabilities",
        ".vac/policies",
        ".vac/workflows",
        ".vac/surfaces",
        ".vac/specs/confirmed",
        ".vac/schemas",
        ".vac/migrations/runtime-db",
    ]
    .iter()
    .filter(|path| !root.join(path).is_dir())
    .map(|path| format!("missing v1.9 authority directory {path}"))
    .collect::<Vec<_>>();
    vac_doctor::product_path_gate("registry", failures.is_empty(), failures)
}

fn doctor_intent(root: &Path) -> vac_doctor::DoctorResult {
    let confirmed = root.join(".vac/specs/confirmed/workspace-intent.yaml");
    let legacy = root.join(".vac/specs/workspace-intent.yaml");
    if confirmed.is_file() {
        vac_doctor::product_path_gate("intent", true, Vec::new())
    } else if legacy.is_file() {
        vac_doctor::product_path_gate(
            "intent",
            false,
            vec![
                "intent spec is still in legacy .vac/specs root; move to .vac/specs/confirmed/"
                    .to_string(),
            ],
        )
    } else {
        vac_doctor::product_path_gate(
            "intent",
            false,
            vec!["missing .vac/specs/confirmed/workspace-intent.yaml".to_string()],
        )
    }
}

fn doctor_ownership(root: &Path) -> vac_doctor::DoctorResult {
    let caps_dir = root.join(".vac/capabilities");
    let mut failures = Vec::new();
    if !caps_dir.is_dir() {
        failures.push("missing .vac/capabilities".to_string());
    } else if let Ok(entries) = std::fs::read_dir(&caps_dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().and_then(|item| item.to_str()) == Some("yaml") {
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                if !text.contains("ownership:") || !text.contains("targets:") {
                    failures.push(format!(
                        "capability lacks ownership targets: {}",
                        path.display()
                    ));
                }
            }
        }
    }
    vac_doctor::product_path_gate("ownership", failures.is_empty(), failures)
}

fn doctor_policy(root: &Path) -> vac_doctor::DoctorResult {
    let dir = root.join(".vac/policies");
    let failures = if dir.is_dir()
        && dir
            .read_dir()
            .map(|mut it| {
                it.any(|entry| {
                    entry
                        .map(|e| e.path().extension().and_then(|x| x.to_str()) == Some("yaml"))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    {
        Vec::new()
    } else {
        vec!["missing policy YAML in .vac/policies".to_string()]
    };
    vac_doctor::product_path_gate("policy", failures.is_empty(), failures)
}

fn doctor_workflow(root: &Path) -> vac_doctor::DoctorResult {
    let dir = root.join(".vac/workflows");
    let failures = if dir.is_dir() {
        Vec::new()
    } else {
        vec!["missing .vac/workflows".to_string()]
    };
    vac_doctor::product_path_gate("workflow", failures.is_empty(), failures)
}

fn doctor_memory(root: &Path) -> vac_doctor::DoctorResult {
    let migration = root.join(".vac/migrations/runtime-db/0001_runtime_journal.sql");
    let mut warnings = vec!["memory records are runtime journal/local-only in v1.9; no source-controlled memory DB is expected".to_string()];
    if !migration.is_file() {
        return vac_doctor::product_path_gate(
            "memory",
            false,
            vec![
                "runtime DB migration missing; memory governance tables cannot be verified"
                    .to_string(),
            ],
        );
    }
    warnings.push("persistent memory FTS/vector implementation remains TV-Pending until cargo/runtime DB tests run".to_string());
    vac_doctor::product_path_gate("memory", true, warnings)
}

fn compile_registry(root: &str) -> Result<(), String> {
    let root = PathBuf::from(root);
    let compiled_dir = root.join(".vac/cache/compiled");
    std::fs::create_dir_all(&compiled_dir).map_err(|e| e.to_string())?;
    let report = vac_registry_compiler::compile_registry_from_disk(&root)
        .map_err(|err| format!("{err:?}"))?;
    let out = serde_json::to_value(&report.snapshot).map_err(|e| e.to_string())?;
    let raw = vac_jcs::to_canonical_pretty_string(&out).map_err(|e| e.to_string())?;
    std::fs::write(compiled_dir.join("workspace.json"), raw).map_err(|e| e.to_string())?;
    println!(
        "wrote {} manifests={} warnings={:?}",
        compiled_dir.join("workspace.json").display(),
        report.manifests.len(),
        report.warnings
    );
    Ok(())
}

fn doctor_compiled(root: &Path) -> vac_doctor::DoctorResult {
    match vac_registry_compiler::compile_registry_from_disk(root) {
        Ok(report) => vac_doctor::product_path_gate(
            "compiled",
            !report.snapshot.source_hashes.is_empty(),
            report.warnings,
        ),
        Err(err) => vac_doctor::DoctorResult {
            gate: "compiled".to_string(),
            status: vac_doctor::DoctorStatus::Fail,
            warnings: Vec::new(),
            failures: vec![format!("{err:?}")],
        },
    }
}

fn doctor_index(root: &Path) -> vac_doctor::DoctorResult {
    let manifest = match read_json(&root.join(".vac/index/index_manifest.json")) {
        Ok(value) => value,
        Err(err) => {
            return vac_doctor::DoctorResult {
                gate: "index".to_string(),
                status: vac_doctor::DoctorStatus::Fail,
                warnings: Vec::new(),
                failures: vec![err],
            };
        }
    };
    let mode = manifest
        .pointer("/coverage/rust_ast_mode")
        .or_else(|| manifest.pointer("/parser_mode"))
        .and_then(Value::as_str)
        .unwrap_or("static_heuristic_fail_closed");
    let truth = vac_index::parser_mode_truth(mode);
    let warnings = truth.warning.into_iter().collect::<Vec<_>>();
    let has_read_plans = vac_index::read_plan_ticket_count(&manifest) > 0
        || root.join(".vac/index/read_plans.jsonl").is_file();
    vac_doctor::product_path_gate("index", has_read_plans, warnings)
}

fn doctor_assessment(root: &Path) -> vac_doctor::DoctorResult {
    let report = match read_json(&root.join(".vac/assessment/gap_report.json")) {
        Ok(value) => value,
        Err(err) => {
            return vac_doctor::DoctorResult {
                gate: "assessment".to_string(),
                status: vac_doctor::DoctorStatus::Fail,
                warnings: Vec::new(),
                failures: vec![err],
            };
        }
    };
    let summary = vac_assessment::validate_gap_report_value(&report);
    let p1_blocks_without_waiver =
        vac_assessment::p1_blocks_without_waiver(&summary, summary.p1_count);
    let mut warnings = summary.warnings;
    if p1_blocks_without_waiver {
        warnings.push("all heuristic P1 findings require explicit waiver for release; this doctor remains warn under L1 scaffold".to_string());
    }
    vac_doctor::product_path_gate("assessment", true, warnings)
}

fn doctor_spec_sync(root: &Path) -> vac_doctor::DoctorResult {
    let report = match read_json(&root.join(".vac/registry/spec-sync/bootstrap.json")) {
        Ok(value) => value,
        Err(err) => {
            return vac_doctor::DoctorResult {
                gate: "spec-sync".to_string(),
                status: vac_doctor::DoctorStatus::Fail,
                warnings: Vec::new(),
                failures: vec![err],
            };
        }
    };
    let critical = report
        .get("unresolved_critical_drift")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let semantic_drift = vac_spec_sync::detect_symbol_invariant_drift(&[], &[]);
    let warnings = if semantic_drift.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "semantic drift proposals pending: {}",
            semantic_drift.len()
        )]
    };
    vac_doctor::product_path_gate("spec-sync", critical == 0, warnings)
}

fn doctor_evidence(root: &Path) -> vac_doctor::DoctorResult {
    let evidence_dir = root.join(".vac/registry/evidence");
    if !evidence_dir.is_dir() {
        return vac_doctor::product_path_gate("evidence", false, Vec::new());
    }
    let mut warnings = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&evidence_dir) {
        for entry in entries.filter_map(Result::ok) {
            if entry.path().extension().and_then(|item| item.to_str()) == Some("json")
                && let Ok(value) = read_json(&entry.path())
            {
                let summary = vac_evidence::classify_evidence_authority(&value);
                if let Some(warning) = summary.warning {
                    warnings.push(format!("{}: {warning}", entry.path().display()));
                }
            }
        }
    }
    vac_doctor::product_path_gate("evidence", true, warnings)
}

fn doctor_enforcement(root: &Path) -> vac_doctor::DoctorResult {
    let status = match read_json(&root.join(".vac/registry/status.json")) {
        Ok(value) => value,
        Err(err) => {
            return vac_doctor::DoctorResult {
                gate: "enforcement".to_string(),
                status: vac_doctor::DoctorStatus::Fail,
                warnings: Vec::new(),
                failures: vec![err],
            };
        }
    };
    let level = status
        .get("enforcement_level")
        .or_else(|| status.pointer("/runtime/enforcement_level"))
        .and_then(Value::as_str)
        .unwrap_or("L1");
    if level.eq_ignore_ascii_case("L2") {
        let broker = status
            .pointer("/trust_boundary/broker_os_sandbox")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let protected_refs = status
            .pointer("/trust_boundary/protected_refs")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let signatures = status
            .pointer("/trust_boundary/operator_broker_signatures")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        vac_doctor::product_path_gate(
            "enforcement",
            broker && protected_refs && signatures,
            vec!["L2 claim requires broker OS sandbox + protected refs + separated operator/broker signatures".to_string()],
        )
    } else {
        vac_doctor::product_path_gate(
            "enforcement",
            true,
            vec![format!(
                "enforcement_level={level}; VAC remains L1 advisory/cooperative, not untrusted-agent L2"
            )],
        )
    }
}

fn doctor_runtime_db(root: &Path) -> vac_doctor::DoctorResult {
    let migration = root.join(".vac/migrations/runtime-db/0001_runtime_journal.sql");
    let mut failures = Vec::new();
    if !migration.is_file() {
        failures.push("missing .vac/migrations/runtime-db/0001_runtime_journal.sql".to_string());
    } else if let Ok(sql) = std::fs::read_to_string(&migration) {
        for table in vac_state::RUNTIME_DB_REQUIRED_TABLES {
            if !sql.contains(table) {
                failures.push(format!("runtime DB migration missing table {table}"));
            }
        }
        for pragma in vac_state::RUNTIME_DB_REQUIRED_PRAGMAS {
            if !sql.contains(pragma) {
                failures.push(format!("runtime DB migration missing pragma {pragma}"));
            }
        }
        let plan = vac_state::runtime_journal_write_plan();
        if plan.transaction_mode != "BEGIN IMMEDIATE" {
            failures.push("runtime journal writer lease must use BEGIN IMMEDIATE".to_string());
        }
        if !sql.contains("manifest_set_hash") || !sql.contains("git_dirty_tree_hash") {
            failures.push(
                "runtime DB migration must carry manifest and git dirty-tree binding fields"
                    .to_string(),
            );
        }
    }
    if root.join(".vac/db/runtime.db").exists() {
        failures.push(
            ".vac/db/runtime.db is local runtime state and must not be packaged as source"
                .to_string(),
        );
    }
    if failures.is_empty() {
        vac_doctor::product_path_gate("runtime-db", true, vec!["runtime.db may be absent in a clean source checkout; migration is source authority".to_string()])
    } else {
        vac_doctor::DoctorResult {
            gate: "runtime-db".to_string(),
            status: vac_doctor::DoctorStatus::Fail,
            warnings: Vec::new(),
            failures,
        }
    }
}

fn doctor_manifest_sync(root: &Path) -> vac_doctor::DoctorResult {
    let report = match vac_registry_compiler::compile_registry_from_disk(root) {
        Ok(report) => report,
        Err(err) => {
            return vac_doctor::DoctorResult {
                gate: "manifest-sync".to_string(),
                status: vac_doctor::DoctorStatus::Fail,
                warnings: Vec::new(),
                failures: vec![format!("{err:?}")],
            };
        }
    };
    let current_hash = report.snapshot.snapshot_hash.clone();
    let cached = root.join(".vac/cache/compiled/workspace.json");
    if cached.is_file() {
        match read_json(&cached) {
            Ok(value) => {
                let cached_hash = value
                    .get("snapshot_hash")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if cached_hash != current_hash {
                    return vac_doctor::DoctorResult { gate: "manifest-sync".to_string(), status: vac_doctor::DoctorStatus::Fail, warnings: Vec::new(), failures: vec!["cached compiled manifest_set_hash differs from current authority manifests".to_string()] };
                }
            }
            Err(err) => {
                return vac_doctor::DoctorResult {
                    gate: "manifest-sync".to_string(),
                    status: vac_doctor::DoctorStatus::Fail,
                    warnings: Vec::new(),
                    failures: vec![err],
                };
            }
        }
    }
    vac_doctor::product_path_gate(
        "manifest-sync",
        true,
        vec![format!("manifest_set_hash={current_hash}")],
    )
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

#[allow(dead_code)]
fn legacy_static_marker(root: &Path) -> Value {
    json!({
        "root": root.display().to_string(),
        "authority": "rust_product_crates",
        "scripts": "fixtures_and_sandbox_sv_helpers_only"
    })
}
