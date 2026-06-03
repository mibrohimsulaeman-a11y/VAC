use clap::Parser;
use clap::Subcommand;
use serde_yaml::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};


#[derive(Debug, Clone)]
struct ExecutionSandboxProfile {
    cwd: PathBuf,
    env_allowlist: Vec<String>,
    timeout: Duration,
    max_stdout_bytes: usize,
    max_stderr_bytes: usize,
    allow_network: bool,
}

impl ExecutionSandboxProfile {
    fn local_plan(workspace: &Path) -> Self {
        Self {
            cwd: workspace.to_path_buf(),
            env_allowlist: vec!["PATH".to_string(), "HOME".to_string(), "RUST_LOG".to_string()],
            timeout: Duration::from_secs(30),
            max_stdout_bytes: 256 * 1024,
            max_stderr_bytes: 128 * 1024,
            allow_network: false,
        }
    }

    fn profile_hash(&self) -> String {
        vac_core::control_plane::vac_init_evidence_chain::sha256_hex(format!(
            "cwd={}\nenv={:?}\ntimeout_ms={}\nstdout={}\nstderr={}\nallow_network={}\n",
            self.cwd.display(), self.env_allowlist, self.timeout.as_millis(), self.max_stdout_bytes, self.max_stderr_bytes, self.allow_network
        ).as_bytes())
    }
}

#[derive(Debug, Clone)]
struct SandboxedCommandOutput { exit_code: Option<i32>, stdout: String, stderr: String, timed_out: bool }

fn run_sandboxed_command(step: &PlanExecutionStep, profile: &ExecutionSandboxProfile) -> anyhow::Result<SandboxedCommandOutput> {
    let mut command = Command::new(&step.runner);
    command.args(&step.args).current_dir(&profile.cwd).env_clear();
    for key in &profile.env_allowlist {
        if let Some(value) = std::env::var_os(key) { command.env(key, value); }
    }
    if !profile.allow_network { command.env("VAC_NETWORK_DISABLED", "1"); }
    let mut child = command.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn()?;
    let started = Instant::now();
    loop {
        if started.elapsed() > profile.timeout {
            let _ = child.kill();
            let output = child.wait_with_output()?;
            return Ok(SandboxedCommandOutput { exit_code: output.status.code(), stdout: truncate_bytes(&output.stdout, profile.max_stdout_bytes), stderr: truncate_bytes(&output.stderr, profile.max_stderr_bytes), timed_out: true });
        }
        if child.try_wait()?.is_some() {
            let output = child.wait_with_output()?;
            return Ok(SandboxedCommandOutput { exit_code: output.status.code(), stdout: truncate_bytes(&output.stdout, profile.max_stdout_bytes), stderr: truncate_bytes(&output.stderr, profile.max_stderr_bytes), timed_out: false });
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn truncate_bytes(bytes: &[u8], max: usize) -> String {
    let end = bytes.len().min(max);
    String::from_utf8_lossy(&bytes[..end]).to_string()
}

/// Work with VAC semantic plans.
#[derive(Debug, Parser)]
pub struct PlanCommand {
    #[command(subcommand)]
    command: PlanSubcommand,
}

impl PlanCommand {
    pub fn run(self) -> anyhow::Result<()> {
        match self.command {
            PlanSubcommand::Create(command) => command.run(),
            PlanSubcommand::Validate(command) => command.run(),
            PlanSubcommand::Approve(command) => command.run(),
            PlanSubcommand::Execute(command) => command.run(),
            PlanSubcommand::Abandon(command) => command.run(),
        }
    }
}

#[derive(Debug, Subcommand)]
enum PlanSubcommand {
    /// Create a draft bounded semantic plan scaffold.
    Create(PlanCreateCommand),
    /// Validate a bounded semantic plan against the local .vac control plane.
    Validate(PlanValidateCommand),
    /// Mark a validated semantic plan as approved by an operator.
    Approve(PlanApproveCommand),
    /// Execute the plan validation commands and persist a local execution record.
    Execute(PlanExecuteCommand),
    /// Mark a semantic plan as abandoned with a reason.
    Abandon(PlanAbandonCommand),
}


#[derive(Debug, Parser)]
struct PlanCreateCommand {
    /// New dotted plan id, for example `plan.vac.feature`.
    #[arg(long)]
    id: String,

    /// Capability id owned by the plan.
    #[arg(long)]
    capability: String,

    /// Workspace-relative output path.
    #[arg(long, value_name = "FILE")]
    output: PathBuf,

    /// Workspace-relative file allowed by the plan. Repeat for multiple files.
    #[arg(long = "file", value_name = "PATH")]
    files: Vec<String>,

    /// Workspace root used for path normalization.
    #[arg(long, value_name = "PATH", default_value = ".")]
    workspace: PathBuf,
}

#[derive(Debug, Parser)]
struct PlanApproveCommand {
    /// Plan YAML file.
    #[arg(value_name = "FILE")]
    plan: PathBuf,

    /// Operator id written into the approval section.
    #[arg(long, default_value = "operator")]
    approved_by: String,

    /// Workspace root used for .vac registry, policy, and ownership lookup.
    #[arg(long, value_name = "PATH", default_value = ".")]
    workspace: PathBuf,
}

#[derive(Debug, Parser)]
struct PlanExecuteCommand {
    /// Plan YAML file.
    #[arg(value_name = "FILE")]
    plan: PathBuf,

    /// Workspace root used for .vac registry and execution report output.
    #[arg(long, value_name = "PATH", default_value = ".")]
    workspace: PathBuf,

    /// Validate and render the execution record without writing it.
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Actually execute allowed structured commands. Without this flag execution is a dry-run.
    #[arg(long, default_value_t = false)]
    execute: bool,
}

#[derive(Debug, Parser)]
struct PlanAbandonCommand {
    /// Plan YAML file.
    #[arg(value_name = "FILE")]
    plan: PathBuf,

    /// Reason recorded in the plan status metadata.
    #[arg(long, default_value = "abandoned by operator")]
    reason: String,

    /// Workspace root used for path normalization.
    #[arg(long, value_name = "PATH", default_value = ".")]
    workspace: PathBuf,
}

#[derive(Debug, Parser)]
struct PlanValidateCommand {
    /// Plan YAML file.
    #[arg(value_name = "FILE")]
    plan: PathBuf,

    /// Workspace root used for .vac registry, policy, and ownership lookup.
    #[arg(long, value_name = "PATH", default_value = ".")]
    workspace: PathBuf,
}

impl PlanCreateCommand {
    fn run(self) -> anyhow::Result<()> {
        let workspace = normalize_root(&self.workspace)?;
        let output = if self.output.is_absolute() {
            self.output
        } else {
            workspace.join(self.output)
        };
        let files = if self.files.is_empty() {
            vec!["README.md".to_string()]
        } else {
            self.files
        };
        let mut allowed = String::new();
        for file in files {
            allowed.push_str(&format!(
                "  - path: {}\n    operation: modify\n    ownership: {}\n    line_range:\n      start: 1\n      end: 1\n",
                yaml_scalar(&file),
                yaml_scalar(&self.capability)
            ));
        }
        let contents = format!(
            "schema_version: 1\nkind: plan\nid: {}\nstatus: draft\ntask:\n  capability: {}\nallowed_files:\n{}validation:\n  commands:\n    - id: vac.static.check\n      runner: bash\n      args:\n        - scripts/check-vac-source-artifact-packaging-gate.sh\n      risk: low\n      approval: not_required\napproval:\n  state: draft\n",
            yaml_scalar(&self.id),
            yaml_scalar(&self.capability),
            allowed
        );
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&output, contents)?;
        println!("vac plan create: PASS");
        println!("plan: {}", output.display());
        Ok(())
    }
}

impl PlanApproveCommand {
    fn run(self) -> anyhow::Result<()> {
        let workspace = normalize_root(&self.workspace)?;
        let path = plan_path_from(&workspace, &self.plan)?;
        let mut plan: Value = serde_yaml::from_str(&fs::read_to_string(&path)?)?;
        set_top_level_scalar(&mut plan, "status", "approved");
        set_mapping_scalar(&mut plan, &["approval", "state"], "approved");
        set_mapping_scalar(&mut plan, &["approval", "approved_by"], &self.approved_by);
        fs::write(&path, serde_yaml::to_string(&plan)?)?;
        println!("vac plan approve: PASS");
        println!("plan: {}", path.display());
        println!("approved_by: {}", self.approved_by);
        Ok(())
    }
}

impl PlanExecuteCommand {
    fn run(self) -> anyhow::Result<()> {
        let workspace = normalize_root(&self.workspace)?;
        let path = plan_path_from(&workspace, &self.plan)?;
        let validate = PlanValidateCommand {
            plan: path.clone(),
            workspace: workspace.clone(),
        };
        validate.run()?;
        let plan: Value = serde_yaml::from_str(&fs::read_to_string(&path)?)?;
        let plan_id = scalar(&plan, "id").unwrap_or_else(|| "plan.unknown".to_string());
        let commands = plan_execution_steps(&plan)?;
        let approved = scalar(&plan, "status").as_deref() == Some("approved")
            || nested_scalar(&plan, &["approval", "state"]).as_deref() == Some("approved");
        let should_execute = self.execute && !self.dry_run;
        if should_execute && !approved {
            return Err(anyhow::anyhow!(
                "plan execution denied: plan `{plan_id}` is not approved; run `vac plan approve` and provide approval binding evidence first"
            ));
        }
        let execution = execute_plan_steps(&workspace, &commands, should_execute, approved)?;
        let report = render_plan_execution_report(&plan_id, &path, should_execute, &execution);
        let report_path = workspace
            .join(".vac/registry/plans")
            .join(format!("{}.execution.yaml", sanitize_id(&plan_id)));
        if !should_execute {
            println!("vac plan execute: DRY-RUN");
            print!("{report}");
            return Ok(());
        }
        if let Some(parent) = report_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&report_path, report)?;
        println!("vac plan execute: PASS");
        println!("plan: {}", path.display());
        println!("report: {}", report_path.display());
        Ok(())
    }
}

impl PlanAbandonCommand {
    fn run(self) -> anyhow::Result<()> {
        let workspace = normalize_root(&self.workspace)?;
        let path = plan_path_from(&workspace, &self.plan)?;
        let mut plan: Value = serde_yaml::from_str(&fs::read_to_string(&path)?)?;
        set_top_level_scalar(&mut plan, "status", "abandoned");
        set_mapping_scalar(&mut plan, &["approval", "state"], "abandoned");
        set_mapping_scalar(&mut plan, &["approval", "abandon_reason"], &self.reason);
        fs::write(&path, serde_yaml::to_string(&plan)?)?;
        println!("vac plan abandon: PASS");
        println!("plan: {}", path.display());
        println!("reason: {}", self.reason);
        Ok(())
    }
}

impl PlanValidateCommand {
    fn run(self) -> anyhow::Result<()> {
        let workspace = normalize_root(&self.workspace)?;
        let plan_path = if self.plan.is_absolute() {
            self.plan
        } else {
            std::env::current_dir()?.join(self.plan)
        };
        let plan_source = fs::read_to_string(&plan_path)?;
        let plan: Value = serde_yaml::from_str(&plan_source)?;

        let mut issues = Vec::new();
        let engine_report = vac_core::control_plane::validate_vac_init_plan_yaml_with_engine(
            &workspace,
            &plan_path,
        )
        .map_err(anyhow::Error::msg)?;
        issues.extend(
            engine_report
                .issues
                .into_iter()
                .map(|issue| format!("{}: {}", issue.code, issue.message)),
        );
        validate_plan_shape(&plan, &mut issues);
        let capabilities = load_capabilities(&workspace)?;
        let policy_loaded = has_policy(&workspace)?;

        if !policy_loaded {
            issues.push(
                "policy.missing: fail-closed; no .vac/policies/*.yaml policy loaded".to_string(),
            );
        }

        let capability = plan_capability(&plan).unwrap_or_default();
        match capabilities.get(&capability) {
            None => issues.push(format!(
                "capability.missing: capability `{capability}` is not registered"
            )),
            Some(status) if status == "planned" || status == "deprecated" => issues.push(format!(
                "capability.not_executable: capability `{capability}` has status `{status}`"
            )),
            Some(_) => {}
        }

        validate_allowed_files(&workspace, &plan, &capability, &mut issues);
        validate_plan_commands(&plan, &mut issues);

        if issues.is_empty() {
            println!("vac plan validate: PASS");
            println!("plan: {}", plan_path.display());
            println!("workspace: {}", workspace.display());
            println!("capability: {capability}");
            Ok(())
        } else {
            println!("vac plan validate: FAIL");
            println!("plan: {}", plan_path.display());
            for issue in issues {
                println!("  - {issue}");
            }
            std::process::exit(1);
        }
    }
}

fn validate_plan_shape(plan: &Value, issues: &mut Vec<String>) {
    if scalar(plan, "schema_version") != Some("1".to_string()) {
        issues.push("schema_version: expected 1".to_string());
    }
    if scalar(plan, "kind") != Some("plan".to_string()) {
        issues.push("kind: expected plan".to_string());
    }
    match scalar(plan, "id") {
        Some(id) if id.starts_with("plan.") && id.contains('.') => {}
        Some(id) => issues.push(format!("id.invalid: `{id}` must start with plan.")),
        None => issues.push("id.missing: plan id is required".to_string()),
    }
    if !matches!(mapping_get(plan, "allowed_files"), Some(Value::Sequence(seq)) if !seq.is_empty())
    {
        issues.push("allowed_files.empty: plan must declare bounded file scope".to_string());
    }
}

fn validate_allowed_files(
    workspace: &Path,
    plan: &Value,
    capability: &str,
    issues: &mut Vec<String>,
) {
    let Some(Value::Sequence(files)) = mapping_get(plan, "allowed_files") else {
        return;
    };
    for (index, file) in files.iter().enumerate() {
        let path = nested_scalar(file, &["path"]).unwrap_or_default();
        if path.is_empty() || path.starts_with('/') || path.contains("..") || path.contains('\\') {
            issues.push(format!("allowed_files[{index}].path.invalid: `{path}`"));
            continue;
        }
        let operation = nested_scalar(file, &["operation"]).unwrap_or_else(|| "modify".to_string());
        if operation != "create" && !workspace.join(&path).exists() {
            issues.push(format!(
                "allowed_files[{index}].path.unknown: `{path}` does not exist"
            ));
        }
        let ownership = nested_scalar(file, &["ownership"]).unwrap_or_default();
        if ownership.is_empty() {
            issues.push(format!("allowed_files[{index}].ownership.missing"));
        } else if ownership != capability {
            issues.push(format!(
                "allowed_files[{index}].ownership.mismatch: `{ownership}` != plan capability `{capability}`"
            ));
        }
        let has_line_range = mapping_get(file, "line_range").is_some();
        let has_anchor = mapping_get(file, "semantic_anchor").is_some();
        if operation != "create" && !has_line_range && !has_anchor {
            issues.push(format!(
                "allowed_files[{index}].bounds.missing: modify/delete require line_range or semantic_anchor"
            ));
        }
    }
}

fn validate_plan_commands(plan: &Value, issues: &mut Vec<String>) {
    let commands = plan
        .as_mapping()
        .and_then(|mapping| mapping.get(&Value::String("validation".to_string())))
        .and_then(|validation| validation.as_mapping())
        .and_then(|mapping| mapping.get(&Value::String("commands".to_string())));

    let Some(Value::Sequence(commands)) = commands else {
        return;
    };

    for (index, command) in commands.iter().enumerate() {
        if command.as_str().is_some() {
            issues.push(format!(
                "validation.commands[{index}].free_form: structured command object required"
            ));
            continue;
        }
        let required = ["id", "runner", "args", "risk", "approval"];
        for field in required {
            if mapping_get(command, field).is_none() {
                issues.push(format!("validation.commands[{index}].{field}.missing"));
            }
        }
        if !matches!(mapping_get(command, "args"), Some(Value::Sequence(_))) {
            issues.push(format!(
                "validation.commands[{index}].args.invalid: expected list"
            ));
        }
        if let Some(runner) = nested_scalar(command, &["runner"]) {
            if runner.contains('/') || runner.contains('\\') {
                issues.push(format!(
                    "validation.commands[{index}].runner.path: `{runner}`"
                ));
            }
        }
    }
}

fn load_capabilities(workspace: &Path) -> anyhow::Result<BTreeMap<String, String>> {
    let mut capabilities = BTreeMap::new();
    let dir = workspace.join(".vac/capabilities");
    if !dir.is_dir() {
        return Ok(capabilities);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("yaml") {
            continue;
        }
        let source = fs::read_to_string(&path)?;
        let value: Value = serde_yaml::from_str(&source)?;
        if let (Some(id), Some(status)) = (scalar(&value, "id"), scalar(&value, "status")) {
            capabilities.insert(id, status);
        }
    }
    Ok(capabilities)
}

fn has_policy(workspace: &Path) -> anyhow::Result<bool> {
    let dir = workspace.join(".vac/policies");
    if !dir.is_dir() {
        return Ok(false);
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("yaml") {
            let value: Value = serde_yaml::from_str(&fs::read_to_string(&path)?)?;
            if scalar(&value, "kind").as_deref() == Some("policy") {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn plan_capability(plan: &Value) -> Option<String> {
    plan.as_mapping()
        .and_then(|mapping| mapping.get(&Value::String("task".to_string())))
        .and_then(|task| nested_scalar(task, &["capability"]))
        .or_else(|| scalar(plan, "capability"))
}

fn scalar(value: &Value, key: &str) -> Option<String> {
    nested_scalar(value, &[key])
}

fn nested_scalar(value: &Value, keys: &[&str]) -> Option<String> {
    let mut current = value;
    for key in keys {
        current = mapping_get(current, key)?;
    }
    match current {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn mapping_get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value
        .as_mapping()
        .and_then(|mapping| mapping.get(&Value::String(key.to_string())))
}


fn plan_path_from(workspace: &Path, plan: &Path) -> anyhow::Result<PathBuf> {
    if plan.is_absolute() {
        Ok(plan.to_path_buf())
    } else {
        Ok(workspace.join(plan))
    }
}

#[derive(Debug, Clone)]
struct PlanExecutionStep {
    id: String,
    runner: String,
    args: Vec<String>,
    risk: String,
    approval: String,
}

#[derive(Debug, Clone)]
struct PlanExecutionOutcome {
    step: PlanExecutionStep,
    decision: String,
    status: String,
    exit_code: Option<i32>,
    stdout_hash: String,
    stderr_hash: String,
    duration_ms: u128,
    evidence_hash: String,
    sandbox_profile_hash: String,
    timed_out: bool,
    issues: Vec<String>,
}

fn plan_execution_steps(plan: &Value) -> anyhow::Result<Vec<PlanExecutionStep>> {
    let commands = plan
        .as_mapping()
        .and_then(|mapping| mapping.get(&Value::String("validation".to_string())))
        .and_then(|validation| validation.as_mapping())
        .and_then(|mapping| mapping.get(&Value::String("commands".to_string())));
    let Some(Value::Sequence(commands)) = commands else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(commands.len());
    for (index, command) in commands.iter().enumerate() {
        let args = match mapping_get(command, "args") {
            Some(Value::Sequence(values)) => values
                .iter()
                .filter_map(|value| match value {
                    Value::String(value) => Some(value.clone()),
                    Value::Number(value) => Some(value.to_string()),
                    Value::Bool(value) => Some(value.to_string()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        out.push(PlanExecutionStep {
            id: nested_scalar(command, &["id"]).unwrap_or_else(|| format!("plan.command.{index}")),
            runner: nested_scalar(command, &["runner"]).unwrap_or_default(),
            args,
            risk: nested_scalar(command, &["risk"]).unwrap_or_else(|| "medium".to_string()),
            approval: nested_scalar(command, &["approval"]).unwrap_or_else(|| "policy".to_string()),
        });
    }
    Ok(out)
}

fn execute_plan_steps(
    workspace: &Path,
    steps: &[PlanExecutionStep],
    execute: bool,
    approved: bool,
) -> anyhow::Result<Vec<PlanExecutionOutcome>> {
    let registry = vac_core::control_plane::vac_init_command_gate::CommandRunnerRegistry::with_defaults();
    let sandbox_profile = ExecutionSandboxProfile::local_plan(workspace);
    let sandbox_profile_hash = sandbox_profile.profile_hash();
    let mut out = Vec::with_capacity(steps.len());
    for step in steps {
        let command = vac_core::control_plane::vac_init_command_gate::StructuredCommand::new(
            step.id.clone(),
            step.runner.clone(),
            step.args.clone(),
            parse_command_risk(&step.risk),
            parse_command_approval(&step.approval),
        );
        let gate = vac_core::control_plane::vac_init_command_gate::evaluate_structured_command(
            &command,
            &registry,
        );
        let mut issues = gate
            .issues
            .iter()
            .map(|issue| format!("{}: {}", issue.code, issue.message))
            .collect::<Vec<_>>();
        let decision = format!("{:?}", gate.decision);
        let mut status = if gate.is_allowed() {
            "allowed".to_string()
        } else {
            "blocked".to_string()
        };
        if decision == "ApprovalRequired" && approved {
            status = "approved_by_plan_binding".to_string();
        } else if decision == "ApprovalRequired" {
            issues.push("approval.required: command requires approved plan binding".to_string());
        }
        let can_run = execute && (gate.is_allowed() || (decision == "ApprovalRequired" && approved));
        let start = Instant::now();
        let (exit_code, stdout, stderr, timed_out) = if can_run {
            let output = run_sandboxed_command(step, &sandbox_profile)?;
            (output.exit_code, output.stdout, output.stderr, output.timed_out)
        } else {
            (None, String::new(), String::new(), false)
        };
        let duration_ms = start.elapsed().as_millis();
        if timed_out {
            issues.push("sandbox.timeout: command exceeded sandbox timeout".to_string());
            status = "timed_out".to_string();
        }
        let stdout_hash = vac_core::control_plane::vac_init_evidence_chain::sha256_hex(stdout.as_bytes());
        let stderr_hash = vac_core::control_plane::vac_init_evidence_chain::sha256_hex(stderr.as_bytes());
        let evidence_payload = format!(
            "id={}\nrunner={}\nargs={:?}\ndecision={}\nstatus={}\nexit_code={:?}\nstdout_hash={}\nstderr_hash={}\nduration_ms={}\nsandbox_profile_hash={}\ntimed_out={}\n",
            step.id,
            step.runner,
            step.args,
            decision,
            status,
            exit_code,
            stdout_hash,
            stderr_hash,
            duration_ms,
            sandbox_profile_hash,
            timed_out,
        );
        let evidence_hash = vac_core::control_plane::vac_init_evidence_chain::sha256_hex(evidence_payload.as_bytes());
        out.push(PlanExecutionOutcome {
            step: step.clone(),
            decision,
            status,
            exit_code,
            stdout_hash,
            stderr_hash,
            duration_ms,
            evidence_hash,
            sandbox_profile_hash: sandbox_profile_hash.clone(),
            timed_out,
            issues,
        });
    }
    Ok(out)
}

fn parse_command_risk(value: &str) -> vac_core::control_plane::vac_init_command_gate::CommandRisk {
    match value {
        "safe_read" => vac_core::control_plane::vac_init_command_gate::CommandRisk::SafeRead,
        "low" => vac_core::control_plane::vac_init_command_gate::CommandRisk::Low,
        "high" => vac_core::control_plane::vac_init_command_gate::CommandRisk::High,
        "critical" => vac_core::control_plane::vac_init_command_gate::CommandRisk::Critical,
        "execute_process" => vac_core::control_plane::vac_init_command_gate::CommandRisk::ExecuteProcess,
        _ => vac_core::control_plane::vac_init_command_gate::CommandRisk::Medium,
    }
}

fn parse_command_approval(value: &str) -> vac_core::control_plane::vac_init_command_gate::CommandApprovalMode {
    match value {
        "always" => vac_core::control_plane::vac_init_command_gate::CommandApprovalMode::Always,
        "never" | "not_required" => vac_core::control_plane::vac_init_command_gate::CommandApprovalMode::Never,
        _ => vac_core::control_plane::vac_init_command_gate::CommandApprovalMode::Policy,
    }
}

fn render_plan_execution_report(
    plan_id: &str,
    plan_path: &Path,
    executed: bool,
    outcomes: &[PlanExecutionOutcome],
) -> String {
    let status = if executed { "completed" } else { "dry_run" };
    let mut yaml = format!(
        "schema_version: 1\nkind: plan.execution\nid: plan.execution.{}\nplan_id: {}\nplan_path: {}\nstatus: {}\nexecutor: vac plan execute\nexecuted: {}\nsteps:\n",
        sanitize_id(plan_id),
        yaml_scalar(plan_id),
        yaml_scalar(&plan_path.display().to_string()),
        status,
        executed,
    );
    for outcome in outcomes {
        yaml.push_str(&format!(
            "  - id: {}
    runner: {}
    args_hash: {}
    decision: {}
    status: {}
    exit_code: {}
    stdout_hash: {}
    stderr_hash: {}
    duration_ms: {}
    evidence_hash: {}
    sandbox_profile_hash: {}
    timed_out: {}
",
            yaml_scalar(&outcome.step.id),
            yaml_scalar(&outcome.step.runner),
            yaml_scalar(&vac_core::control_plane::vac_init_evidence_chain::sha256_hex(format!("{:?}", outcome.step.args).as_bytes())),
            yaml_scalar(&outcome.decision),
            yaml_scalar(&outcome.status),
            outcome.exit_code.map(|code| code.to_string()).unwrap_or_else(|| "null".to_string()),
            yaml_scalar(&outcome.stdout_hash),
            yaml_scalar(&outcome.stderr_hash),
            outcome.duration_ms,
            yaml_scalar(&outcome.evidence_hash),
            yaml_scalar(&outcome.sandbox_profile_hash),
            outcome.timed_out,
        ));
        if !outcome.issues.is_empty() {
            yaml.push_str("    issues:\n");
            for issue in &outcome.issues {
                yaml.push_str(&format!("      - {}\n", yaml_scalar(issue)));
            }
        }
    }
    yaml
}

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn yaml_scalar(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | '/' | ':'))
    {
        value.to_string()
    } else {
        format!("{value:?}")
    }
}

fn set_top_level_scalar(value: &mut Value, key: &str, new_value: &str) {
    if let Some(mapping) = value.as_mapping_mut() {
        mapping.insert(
            Value::String(key.to_string()),
            Value::String(new_value.to_string()),
        );
    }
}

fn set_mapping_scalar(value: &mut Value, keys: &[&str], new_value: &str) {
    if keys.is_empty() {
        return;
    }
    if keys.len() == 1 {
        set_top_level_scalar(value, keys[0], new_value);
        return;
    }
    let Some(mapping) = value.as_mapping_mut() else {
        return;
    };
    let entry = mapping
        .entry(Value::String(keys[0].to_string()))
        .or_insert_with(|| Value::Mapping(Default::default()));
    set_mapping_scalar(entry, &keys[1..], new_value);
}

fn normalize_root(path: &Path) -> anyhow::Result<PathBuf> {
    if path.exists() {
        Ok(path.canonicalize()?)
    } else {
        Ok(path.to_path_buf())
    }
}
