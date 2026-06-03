use std::path::PathBuf;

use anyhow::bail;
use clap::Parser;
use vac_utils_cli::CliConfigOverrides;

/// Apply a diff from a local source.
///
/// The former cloud-task fetch path was retired for the local TUI+CLI coding
/// tool build. Keeping the command as a fail-closed compatibility surface avoids
/// silently reaching ChatGPT backend task APIs while preserving CLI parse
/// compatibility for scripts that still probe `vac apply --help`.
#[derive(Debug, Parser)]
pub struct ApplyCommand {
    /// Deprecated cloud task id. Cloud task retrieval is disabled in this build.
    pub task_id: String,

    #[clap(flatten)]
    pub config_overrides: CliConfigOverrides,
}

pub async fn run_apply_command(
    _apply_cli: ApplyCommand,
    _cwd: Option<PathBuf>,
) -> anyhow::Result<()> {
    bail!(
        "vac apply cloud task retrieval was removed from the local coding-agent build; use the TUI patch approval flow or apply a local diff with git/apply-patch instead"
    )
}
