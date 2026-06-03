use clap::Parser;
use std::path::{Path, PathBuf};
use vac_core::control_plane::VacInitWhyCliTarget;
use vac_core::control_plane::parse_vac_init_why_target;
use vac_core::control_plane::vac_init_safe_rationale::WhyQuery;

/// Explain safe rationale for a file, line, range, or symbol.
#[derive(Debug, Parser)]
pub struct WhyCommand {
    /// Query target: <file>, <file>:<line>, <file>:<start>-<end>, or <file>::<symbol>.
    #[arg(value_name = "TARGET")]
    target: Option<String>,

    /// Maximum evidence chain depth to render.
    #[arg(long, default_value_t = 3)]
    depth: usize,

    /// Workspace root.
    #[arg(long, value_name = "PATH", default_value = ".")]
    workspace: PathBuf,
}

impl WhyCommand {
    pub fn run(self) -> anyhow::Result<()> {
        let Some(target) = self.target else {
            println!("vac why: missing query target");
            println!("usage: vac why <file>:<line> | <file>:<start>-<end> | <file>::<symbol>");
            std::process::exit(1);
        };

        let parsed = match parse_vac_init_why_target(&target) {
            Ok(query) => query,
            Err(error) => {
                println!("vac why: invalid query");
                println!("  - {error}");
                std::process::exit(1);
            }
        };

        let workspace = normalize_root(&self.workspace)?;
        let index_path = workspace.join(".vac/registry/trajectory/index.yaml");
        if !index_path.exists() {
            println!("vac why: no trajectory index");
            println!("index_path: {}", index_path.display());
            println!(
                "diagnostic: run a task that writes safe rationale evidence before querying why"
            );
            std::process::exit(1);
        }

        let engine_query = why_query_from_cli_target(&parsed, self.depth);
        let report = vac_core::control_plane::lookup_vac_init_safe_rationale_with_engine(
            &workspace,
            engine_query,
        )
        .map_err(anyhow::Error::msg)?;

        println!("vac why");
        println!("query: {target}");
        println!("index: {}", index_path.display());
        println!("engine: vac_init_safe_rationale");

        for diagnostic in &report.diagnostics {
            println!("diagnostic: {diagnostic}");
        }

        let mut rendered = 0usize;
        for result in report.results.iter().take(self.depth) {
            rendered += 1;
            println!("result[{rendered}]:");
            println!("  evidence_id: {}", result.evidence_id);
            println!("  timestamp: {}", result.timestamp);
            println!("  task: {}", result.task);
            println!("  plan_id: {}", result.plan_id);
            println!("  capability: {}", result.capability);
            println!("  rationale: {}", result.rationale.summary);
            if !result.rationale.policy_refs.is_empty() {
                println!("  policy_refs: {}", result.rationale.policy_refs.join(", "));
            }
            if !result.rationale.evidence_refs.is_empty() {
                println!("  evidence_refs: {}", result.rationale.evidence_refs.join(", "));
            }
            println!("  raw_chain_of_thought: excluded");
        }

        if rendered == 0 {
            println!("diagnostic: no matching safe rationale found");
            std::process::exit(1);
        }

        Ok(())
    }
}

fn why_query_from_cli_target(target: &VacInitWhyCliTarget, depth: usize) -> WhyQuery {
    match target {
        VacInitWhyCliTarget::File { file } => WhyQuery {
            file: file.clone(),
            line: None,
            range: None,
            symbol: None,
            depth,
        },
        VacInitWhyCliTarget::Line { file, line } => WhyQuery {
            file: file.clone(),
            line: Some(*line),
            range: None,
            symbol: None,
            depth,
        },
        VacInitWhyCliTarget::Range { file, start, end } => WhyQuery {
            file: file.clone(),
            line: None,
            range: Some((*start, *end)),
            symbol: None,
            depth,
        },
        VacInitWhyCliTarget::Symbol { file, symbol } => WhyQuery {
            file: file.clone(),
            line: None,
            range: None,
            symbol: Some(symbol.clone()),
            depth,
        },
    }
}

fn normalize_root(path: &Path) -> anyhow::Result<PathBuf> {
    if path.exists() {
        Ok(path.canonicalize()?)
    } else {
        Ok(path.to_path_buf())
    }
}
