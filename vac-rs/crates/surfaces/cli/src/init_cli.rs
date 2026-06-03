use clap::Parser;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Bootstrap or inspect the VAC-Init workflow control-plane state.
#[derive(Debug, Parser)]
pub struct InitCommand {
    /// Render the planned init artifacts without mutating the workspace.
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Resume a non-ready .vac/.init/state.yaml lifecycle state.
    #[arg(long, default_value_t = false)]
    resume: bool,

    /// Show the current init lifecycle status.
    #[arg(long, default_value_t = false)]
    status: bool,

    /// Refresh the scan report and leave the init lifecycle at discovered.
    #[arg(long, default_value_t = false)]
    scan: bool,

    /// Refresh the risk scanner output and leave lifecycle at policy_inferred.
    #[arg(long = "rescan-ast", default_value_t = false)]
    rescan_ast: bool,

    /// Run the operator-guided init UX (partition strategy, scan preview, synthesize/verify).
    #[arg(long, default_value_t = false)]
    interactive: bool,

    /// Workspace root.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,
}

impl InitCommand {
    pub fn run(self) -> anyhow::Result<()> {
        let mode = vac_core::control_plane::resolve_vac_init_mode(
            self.dry_run,
            self.resume,
            self.status,
            self.scan,
            self.rescan_ast,
        )
        .map_err(anyhow::Error::msg)?;

        let root = normalize_root(&self.path)?;
        if self.interactive {
            write_interactive_init_choices(&root)?;
        }
        let init_dir = root.join(".vac/.init");
        let state_path = init_dir.join("state.yaml");

        if matches!(mode, vac_core::control_plane::VacInitCliMode::Status) {
            print_init_status(&state_path)?;
            return Ok(());
        }

        let timestamp = utc_timestamp_string();
        let previous_state = if state_path.exists() {
            read_scalar(&state_path, "current_state").unwrap_or_else(|| "uninitialized".to_string())
        } else {
            "uninitialized".to_string()
        };
        let doctor_report = build_init_doctor_report(&root, &timestamp);
        let state_record = build_lifecycle_state_record(
            mode,
            &previous_state,
            &timestamp,
            doctor_report.passed,
        )
        .map_err(anyhow::Error::msg)?;
        let current_state = state_record.current_state.as_str();

        let state_yaml = state_record.render_yaml();
        let scanner_report_files =
            vac_core::control_plane::build_vac_init_live_scanner_report_files(&root)
                .map_err(anyhow::Error::msg)?;
        let strategy_yaml = render_strategy_yaml(&root, &timestamp)?;
        let doctor_report_yaml = doctor_report.render_yaml();

        if matches!(mode, vac_core::control_plane::VacInitCliMode::DryRun) {
            println!("vac init dry-run");
            println!("workspace: {}", root.display());
            println!("would_write:");
            println!("  - .vac/.init/state.yaml");
            println!("  - .vac/.init/scan_report.yaml");
            println!("  - .vac/.init/source_inventory.yaml");
            println!("  - .vac/.init/risk_findings.yaml");
            println!("  - .vac/.init/risk_findings/index.yaml");
            println!("  - .vac/.init/risk_findings/full.yaml");
            println!("  - .vac/.init/risk_findings/by-risk/*.yaml");
            println!("  - .vac/.init/risk_findings/by-scope/*.yaml");
            println!("  - .vac/.init/policy_inference_report.yaml");
            println!("  - .vac/.init/scanner_doctor_report.yaml");
            println!("  - .vac/.init/source_inventory/by-class/*.yaml");
            println!("  - .vac/.init/strategy.yaml");
            println!("  - .vac/.init/doctor_report.yaml");
            println!("state_preview:");
            for line in state_yaml.lines() {
                println!("  {line}");
            }
            return Ok(());
        }

        fs::create_dir_all(&init_dir)?;
        write_store(&root, ".vac/.init/state.yaml", &state_yaml)?;
        for (relative_path, content) in scanner_report_files {
            write_store(&root, &relative_path, &content)?;
        }
        write_store(&root, ".vac/.init/strategy.yaml", &strategy_yaml)?;
        write_store(&root, ".vac/.init/doctor_report.yaml", &doctor_report_yaml)?;
        write_init_lifecycle_evidence(&root, current_state, &timestamp)?;

        println!("vac init: {}", current_state);
        println!("workspace: {}", root.display());
        println!("wrote: {}", state_path.display());
        Ok(())
    }
}


fn write_interactive_init_choices(root: &Path) -> anyhow::Result<()> {
    let mut stdout = io::stdout();
    let strategy = prompt_with_default(
        &mut stdout,
        "VAC init partition strategy [current_registry/layered_control_plane]",
        "layered_control_plane",
    )?;
    let synthesis = prompt_with_default(
        &mut stdout,
        "Synthesize missing manifests after scan? [yes/no]",
        "yes",
    )?;
    let verify = prompt_with_default(
        &mut stdout,
        "Run doctor verification after synthesis? [yes/no]",
        "yes",
    )?;
    let choices = format!(
        "schema_version: 1\nkind: init.operator_choices\nid: init.operator_choices\npartition_strategy: {}\nsynthesize_missing_manifests: {}\nverify_after_synthesis: {}\n",
        yaml_scalar(&strategy),
        yaml_scalar(&synthesis),
        yaml_scalar(&verify)
    );
    write_store(root, ".vac/.init/operator_choices.yaml", &choices)
}

fn prompt_with_default(stdout: &mut io::Stdout, prompt: &str, default: &str) -> anyhow::Result<String> {
    write!(stdout, "{prompt} ({default}): ")?;
    stdout.flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let value = input.trim();
    if value.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(value.to_string())
    }
}

fn print_init_status(state_path: &Path) -> anyhow::Result<()> {
    if !state_path.exists() {
        println!("vac init status: uninitialized");
        println!("state: missing .vac/.init/state.yaml");
        return Ok(());
    }
    let state = fs::read_to_string(state_path)?;
    let current = read_scalar(state_path, "current_state").unwrap_or_else(|| "unknown".to_string());
    println!("vac init status: {current}");
    println!("state_path: {}", state_path.display());
    for line in state.lines() {
        if line.starts_with("current_state:")
            || line.starts_with("previous_state:")
            || line.starts_with("timestamp:")
            || line.starts_with("error:")
        {
            println!("{line}");
        }
    }
    Ok(())
}

fn normalize_root(path: &Path) -> anyhow::Result<PathBuf> {
    if path.exists() {
        Ok(path.canonicalize()?)
    } else {
        Ok(path.to_path_buf())
    }
}

fn build_lifecycle_state_record(
    mode: vac_core::control_plane::VacInitCliMode,
    previous_state: &str,
    timestamp: &str,
    doctor_passed: bool,
) -> Result<vac_core::control_plane::VacInitStateRecord, String> {
    use std::str::FromStr;
    let previous = vac_core::control_plane::VacInitLifecycleState::from_str(previous_state)
        .unwrap_or(vac_core::control_plane::VacInitLifecycleState::Uninitialized);
    let mut record = vac_core::control_plane::VacInitStateRecord::new(timestamp.to_string());
    record.current_state = previous;
    attach_lifecycle_artifacts(&mut record);

    let next = next_lifecycle_state(mode, previous, doctor_passed);
    let mut updated = if next == previous {
        record
    } else {
        vac_core::control_plane::advance_init_state(&record, next, timestamp.to_string())
            .map_err(|err| err.to_string())?
    };
    attach_lifecycle_artifacts(&mut updated);
    updated.validate().map_err(|err| err.to_string())?;
    Ok(updated)
}

fn next_lifecycle_state(
    mode: vac_core::control_plane::VacInitCliMode,
    previous: vac_core::control_plane::VacInitLifecycleState,
    doctor_passed: bool,
) -> vac_core::control_plane::VacInitLifecycleState {
    use vac_core::control_plane::VacInitLifecycleState as S;
    match mode {
        vac_core::control_plane::VacInitCliMode::Scan => S::Discovered,
        vac_core::control_plane::VacInitCliMode::RescanAst => S::PolicyInferred,
        vac_core::control_plane::VacInitCliMode::DryRun
        | vac_core::control_plane::VacInitCliMode::Status => previous,
        vac_core::control_plane::VacInitCliMode::Resume
        | vac_core::control_plane::VacInitCliMode::Apply => match previous {
            S::Uninitialized | S::ScanFailed => S::Discovered,
            S::Discovered => S::PartitionSelected,
            S::PartitionSelected | S::PolicyConflict => S::PolicyInferred,
            S::PolicyInferred => S::ManifestsSynthesized,
            S::ManifestsSynthesized | S::DoctorFailed => {
                if doctor_passed { S::DoctorVerified } else { S::DoctorFailed }
            }
            S::DoctorVerified => {
                if doctor_passed { S::Ready } else { S::DoctorFailed }
            }
            S::OwnershipMissing => S::Discovered,
            S::Ready | S::OperatorCancelled => previous,
        },
    }
}

fn attach_lifecycle_artifacts(record: &mut vac_core::control_plane::VacInitStateRecord) {
    record.scan_report = Some(".vac/.init/scan_report.yaml".to_string());
    if matches!(
        record.current_state,
        vac_core::control_plane::VacInitLifecycleState::PartitionSelected
            | vac_core::control_plane::VacInitLifecycleState::PolicyInferred
            | vac_core::control_plane::VacInitLifecycleState::ManifestsSynthesized
            | vac_core::control_plane::VacInitLifecycleState::DoctorVerified
            | vac_core::control_plane::VacInitLifecycleState::Ready
    ) {
        record.strategy = Some(".vac/.init/strategy.yaml".to_string());
    }
    if matches!(
        record.current_state,
        vac_core::control_plane::VacInitLifecycleState::PolicyInferred
            | vac_core::control_plane::VacInitLifecycleState::ManifestsSynthesized
            | vac_core::control_plane::VacInitLifecycleState::DoctorVerified
            | vac_core::control_plane::VacInitLifecycleState::Ready
    ) {
        record.risk_findings = Some(".vac/.init/risk_findings.yaml".to_string());
    }
    if matches!(
        record.current_state,
        vac_core::control_plane::VacInitLifecycleState::DoctorVerified
            | vac_core::control_plane::VacInitLifecycleState::DoctorFailed
            | vac_core::control_plane::VacInitLifecycleState::Ready
    ) {
        record.doctor_report = Some(".vac/.init/doctor_report.yaml".to_string());
    }
}

fn write_init_lifecycle_evidence(root: &Path, current_state: &str, timestamp: &str) -> anyhow::Result<()> {
    let request = vac_core::control_plane::VacInitLiveEvidenceWriteRequest {
        evidence_id: format!("evidence.init.lifecycle.{current_state}"),
        timestamp: timestamp.to_string(),
        plan_id: "plan.vac-init.lifecycle".to_string(),
        capability: "vac.init.lifecycle".to_string(),
        file: ".vac/.init/state.yaml".to_string(),
        start_line: 1,
        end_line: 12,
        symbol: Some(current_state.to_string()),
        rationale_summary: format!("vac init advanced through lifecycle engine into state {current_state}"),
        approval_content_hash: None,
    };
    vac_core::control_plane::write_vac_init_live_evidence_and_trajectory(root, &request)
        .map(|_| ())
        .map_err(anyhow::Error::msg)
}

#[allow(dead_code)]
fn render_scan_report_yaml(root: &Path, timestamp: &str) -> anyhow::Result<String> {
    let mut source_files = 0usize;
    let mut manifest_files = 0usize;
    count_workspace_files(root, &mut source_files, &mut manifest_files)?;
    Ok(format!(
        "schema_version: 1\nkind: registry_status\nid: init.scan_report\ntitle: VAC init scan report\nstatus: ready\ntimestamp: {timestamp}\nsummary:\n  source_files: {source_files}\n  vac_manifest_files: {manifest_files}\n"
    ))
}

fn render_strategy_yaml(root: &Path, timestamp: &str) -> anyhow::Result<String> {
    let choices = load_operator_choices(root);
    let selected = choices
        .get("partition_strategy")
        .cloned()
        .unwrap_or_else(|| "current_registry".to_string());
    let synthesize = choices
        .get("synthesize_missing_manifests")
        .cloned()
        .unwrap_or_else(|| "no".to_string());
    let verify = choices
        .get("verify_after_synthesis")
        .cloned()
        .unwrap_or_else(|| "yes".to_string());
    let source = choices
        .get("source")
        .cloned()
        .unwrap_or_else(|| "cli_non_interactive_default".to_string());
    let rationale = if selected == "layered_control_plane" {
        "Operator selected layered control-plane partitioning; scan output should be reconciled with .vac ownership before synthesis."
    } else {
        "Existing .vac control-plane manifests are preserved and validated before later runtime hardening."
    };
    Ok(format!(
        "schema_version: 1\nkind: registry_status\nid: init.strategy\ntitle: VAC init partition strategy\nstatus: ready\ntimestamp: {timestamp}\nstrategy:\n  selected: {}\n  partition_mode: {}\n  synthesize_missing_manifests: {}\n  verify_after_synthesis: {}\n  source: {}\n  rationale: {}\n",
        yaml_scalar(&selected),
        yaml_scalar(&selected),
        yaml_scalar(&synthesize),
        yaml_scalar(&verify),
        yaml_scalar(&source),
        yaml_scalar(rationale),
    ))
}

fn load_operator_choices(root: &Path) -> std::collections::BTreeMap<String, String> {
    let path = root.join(".vac/.init/operator_choices.yaml");
    let mut choices = std::collections::BTreeMap::new();
    let Ok(source) = fs::read_to_string(path) else {
        return choices;
    };
    for line in source.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if matches!(
            key,
            "partition_strategy" | "synthesize_missing_manifests" | "verify_after_synthesis" | "source"
        ) {
            choices.insert(key.to_string(), value.trim().trim_matches('"').to_string());
        }
    }
    choices
}

#[derive(Debug, Clone)]
struct InitDoctorCheck {
    id: &'static str,
    passed: bool,
    detail: String,
}

#[derive(Debug, Clone)]
struct InitDoctorReport {
    timestamp: String,
    passed: bool,
    checks: Vec<InitDoctorCheck>,
}

impl InitDoctorReport {
    fn render_yaml(&self) -> String {
        let status = if self.passed { "ready" } else { "blocked" };
        let mut yaml = format!(
            "schema_version: 1\nkind: registry_status\nid: init.doctor_report\ntitle: VAC init doctor bootstrap report\nstatus: {status}\ntimestamp: {}\ndoctors:\n",
            self.timestamp
        );
        for check in &self.checks {
            let state = if check.passed { "pass" } else { "fail" };
            yaml.push_str(&format!(
                "  {}:\n    status: {}\n    detail: {}\n",
                check.id,
                state,
                yaml_scalar(&check.detail)
            ));
        }
        yaml
    }
}

fn build_init_doctor_report(root: &Path, timestamp: &str) -> InitDoctorReport {
    let owner_support = root.join(".vac/registry/runtime/owner-native-support.yaml");
    let owner_support_text = fs::read_to_string(&owner_support).unwrap_or_default();
    let active_plan = root.join(".vac/registry/runtime/active_plan.yaml");
    let gate_anchor = root.join("vac-rs/core/src/runtime_gate_callsite_integration.rs");
    let registry_dir = root.join(".vac/registry");
    let policies_dir = root.join(".vac/policies");
    let ownership_report = root.join(".vac/registry/ownership/report.yaml");
    let critical_methods = [
        "name: start_thread_with_session_start_source",
        "name: turn_start",
        "name: turn_steer",
        "name: turn_interrupt",
        "name: startup_interrupt",
        "name: thread_shell_command",
        "name: thread_list",
        "name: thread_read",
        "name: resume_thread",
        "name: branch_thread",
        "name: read_account",
    ];
    let owner_runtime_supported = owner_support.exists()
        && critical_methods
            .iter()
            .all(|needle| owner_support_text.contains(needle))
        && owner_support_text.contains("status: ready")
        && owner_support_text.matches("status: implemented").count() >= critical_methods.len();
    let plan_mode_integrated = active_plan.exists()
        && owner_support_text.contains("pre_gate: implemented")
        && owner_support_text.contains("post_gate: implemented_static")
        && owner_support_text.contains("active_plan: .vac/registry/runtime/active_plan.yaml");
    let command_gate_wired = gate_anchor.exists()
        && owner_support_text.contains("pre_gate: implemented")
        && owner_support_text.contains("evidence_completion_gate: implemented");
    let mut checks = vec![
        InitDoctorCheck {
            id: "registry",
            passed: registry_dir.is_dir(),
            detail: format!("registry_dir={}", registry_dir.display()),
        },
        InitDoctorCheck {
            id: "policy",
            passed: policies_dir.is_dir(),
            detail: format!("policy_dir={}", policies_dir.display()),
        },
        InitDoctorCheck {
            id: "ownership",
            passed: ownership_report.exists(),
            detail: format!("ownership_report={}", ownership_report.display()),
        },
        InitDoctorCheck {
            id: "runtime_owner",
            passed: owner_runtime_supported,
            detail: format!("owner_runtime_support_manifest={}", owner_support.display()),
        },
        InitDoctorCheck {
            id: "command_gate_evidence",
            passed: command_gate_wired,
            detail: format!("owner support manifest declares command pre/evidence gates; gate_anchor={}", gate_anchor.display()),
        },
        InitDoctorCheck {
            id: "plan_mode_runtime_gate",
            passed: plan_mode_integrated,
            detail: format!("runtime Plan Mode declares pre/post semantic gates; active_plan={}", active_plan.display()),
        },
    ];
    let passed = checks.iter().all(|check| check.passed);
    checks.sort_by_key(|check| check.id);
    InitDoctorReport {
        timestamp: timestamp.to_string(),
        passed,
        checks,
    }
}

fn yaml_scalar(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('\"', "\\\""))
}

#[allow(dead_code)]
fn count_workspace_files(
    root: &Path,
    source_files: &mut usize,
    manifest_files: &mut usize,
) -> anyhow::Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if file_name == ".git" || file_name == "target" || file_name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            count_workspace_files(&path, source_files, manifest_files)?;
        } else {
            match path.extension().and_then(|value| value.to_str()) {
                Some("rs" | "toml" | "md") => *source_files += 1,
                Some("yaml" | "yml") if path.components().any(|c| c.as_os_str() == ".vac") => {
                    *manifest_files += 1
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn read_scalar(path: &Path, field: &str) -> Option<String> {
    let source = fs::read_to_string(path).ok()?;
    source.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if key.trim() == field {
            Some(value.trim().trim_matches('"').to_string())
        } else {
            None
        }
    })
}

fn write_store(root: &Path, relative_path: &str, content: &str) -> anyhow::Result<()> {
    vac_core::control_plane::write_vac_init_store_record_atomic(root, relative_path, content)
        .map(|_| ())
        .map_err(anyhow::Error::msg)
}

fn utc_timestamp_string() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("unix-{seconds}Z")
}
