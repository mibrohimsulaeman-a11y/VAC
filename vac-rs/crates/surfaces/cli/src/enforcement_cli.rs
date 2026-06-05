use clap::Parser;
use clap::Subcommand;
use std::path::PathBuf;
use vac_core::control_plane::load_enforcement_status_report;

#[derive(Debug, Parser)]
pub struct EnforcementCommand {
    #[clap(subcommand)]
    command: EnforcementSubcommand,
}

#[derive(Debug, Subcommand)]
enum EnforcementSubcommand {
    /// Render the current enforcement claim/observation status.
    Status(EnforcementStatusCommand),
}

#[derive(Debug, Parser)]
struct EnforcementStatusCommand {
    /// Workspace root.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,
}

impl EnforcementStatusCommand {
    fn run(self) -> anyhow::Result<()> {
        let report = load_enforcement_status_report(&self.path);
        println!("{}", report.render_text());
        let exit_code = report.cli_exit_code();
        if exit_code == 0 {
            Ok(())
        } else {
            anyhow::bail!("enforcement status failed with exit code {exit_code}");
        }
    }
}

impl EnforcementCommand {
    pub fn run(self) -> anyhow::Result<()> {
        match self.command {
            EnforcementSubcommand::Status(command) => command.run(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EnforcementCommand;
    use clap::Parser;

    #[test]
    fn enforcement_command_can_be_parsed_by_clap() {
        let _ = EnforcementCommand::parse_from(["vac", "status"]);
    }
}
