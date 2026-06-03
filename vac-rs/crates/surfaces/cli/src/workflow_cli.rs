use clap::Parser;
use clap::Subcommand;
use std::fs;
use std::path::{Path, PathBuf};

/// Inspect or run VAC control-plane workflows from the CLI.
#[derive(Debug, Parser)]
pub struct WorkflowCommand {
    #[command(subcommand)]
    command: WorkflowSubcommand,
}

impl WorkflowCommand {
    pub fn run(self) -> anyhow::Result<()> {
        match self.command {
            WorkflowSubcommand::List(command) => command.run(),
            WorkflowSubcommand::Inspect(command) => command.run(),
            WorkflowSubcommand::Run(command) => command.run(),
        }
    }
}

#[derive(Debug, Subcommand)]
enum WorkflowSubcommand {
    /// List registered workflow manifests.
    List(WorkflowListCommand),
    /// Inspect one workflow manifest and runner readiness.
    Inspect(WorkflowInspectCommand),
    /// Execute a workflow manifest through the local dry-run-safe runner.
    Run(WorkflowRunCommand),
}

#[derive(Debug, Parser)]
struct WorkflowListCommand {
    #[arg(long, value_name = "PATH", default_value = ".")]
    workspace: PathBuf,
}

#[derive(Debug, Parser)]
struct WorkflowInspectCommand {
    #[arg(value_name = "WORKFLOW_ID")]
    workflow_id: String,
    #[arg(long, value_name = "PATH", default_value = ".")]
    workspace: PathBuf,
}

#[derive(Debug, Parser)]
struct WorkflowRunCommand {
    #[arg(value_name = "WORKFLOW_ID")]
    workflow_id: String,
    #[arg(long, value_name = "PATH", default_value = ".")]
    workspace: PathBuf,
    /// Persist a run report under `.vac/registry/runtime/workflows`.
    #[arg(long, default_value_t = true)]
    write_report: bool,
}

impl WorkflowListCommand {
    fn run(self) -> anyhow::Result<()> {
        let workspace = normalize_root(&self.workspace)?;
        let manifests = workflow_manifest_paths(&workspace)?;
        println!("vac workflow list: {} workflow(s)", manifests.len());
        for path in manifests {
            let manifest = vac_core::control_plane::workflow_manifest::load_workflow_manifest(&path)
                .map_err(|err| anyhow::anyhow!(err.to_string()))?;
            println!("- {} — {} ({})", manifest.id, manifest.title, path.display());
        }
        Ok(())
    }
}


fn render_workflow_preview(preview: &vac_core::control_plane::workflow_runner::WorkflowRunPreview) -> String {
    if preview.supported {
        return "supported by initial safe runner".to_string();
    }
    let blocked_steps = preview
        .blocked_steps
        .iter()
        .map(|step| format!("{}={}", step.id, step.uses))
        .collect::<Vec<_>>()
        .join(", ");
    format!("blocked: unsupported steps=[{}]", blocked_steps)
}

impl WorkflowInspectCommand {
    fn run(self) -> anyhow::Result<()> {
        let workspace = normalize_root(&self.workspace)?;
        let path = find_workflow_manifest(&workspace, &self.workflow_id)?;
        let manifest = vac_core::control_plane::workflow_manifest::load_workflow_manifest(&path)
            .map_err(|err| anyhow::anyhow!(err.to_string()))?;
        let preview = vac_core::control_plane::workflow_runner::preview_workflow_manifest(&manifest);
        let dry_run = vac_core::control_plane::workflow_runner::dry_run_workflow_manifest(&manifest);
        println!("vac workflow inspect: PASS");
        println!("id: {}", manifest.id);
        println!("title: {}", manifest.title);
        println!("path: {}", path.display());
        println!("preview:\n{}", render_workflow_preview(&preview));
        println!("dry_run:\n{}", dry_run.render_text());
        Ok(())
    }
}

impl WorkflowRunCommand {
    fn run(self) -> anyhow::Result<()> {
        let workspace = normalize_root(&self.workspace)?;
        let path = find_workflow_manifest(&workspace, &self.workflow_id)?;
        let manifest = vac_core::control_plane::workflow_manifest::load_workflow_manifest(&path)
            .map_err(|err| anyhow::anyhow!(err.to_string()))?;
        let report = vac_core::control_plane::workflow_runner::execute_workflow_manifest(&manifest);
        println!("vac workflow run: PASS");
        println!("id: {}", manifest.id);
        println!("path: {}", path.display());
        println!("{}", report.render_text());
        if self.write_report {
            let report_path = workspace
                .join(".vac/registry/runtime/workflows")
                .join(format!("{}.run.yaml", sanitize_id(&manifest.id)));
            if let Some(parent) = report_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&report_path, render_workflow_run_yaml(&manifest.id, &report.render_text()))?;
            println!("report: {}", report_path.display());
        }
        Ok(())
    }
}

fn workflow_manifest_paths(workspace: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let dir = workspace.join(".vac/workflows");
    let mut paths = Vec::new();
    if !dir.is_dir() {
        return Ok(paths);
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("yaml") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn find_workflow_manifest(workspace: &Path, workflow_id: &str) -> anyhow::Result<PathBuf> {
    for path in workflow_manifest_paths(workspace)? {
        let manifest = vac_core::control_plane::workflow_manifest::load_workflow_manifest(&path)
            .map_err(|err| anyhow::anyhow!(err.to_string()))?;
        if manifest.id == workflow_id {
            return Ok(path);
        }
    }
    Err(anyhow::anyhow!("workflow `{workflow_id}` not found"))
}

fn render_workflow_run_yaml(workflow_id: &str, report: &str) -> String {
    format!(
        "schema_version: 1\nkind: workflow.run\nid: workflow.run.{}\nworkflow_id: {}\nstatus: completed\nreport: |\n{}\n",
        sanitize_id(workflow_id),
        yaml_scalar(workflow_id),
        indent_block(report, 2)
    )
}

fn indent_block(value: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    value
        .lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_root(path: &Path) -> anyhow::Result<PathBuf> {
    if path.exists() {
        Ok(path.canonicalize()?)
    } else {
        Ok(path.to_path_buf())
    }
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
