use clap::Parser;
use std::path::PathBuf;
use vac_core::control_plane::load_session_doctor_report;

/// Validate the session registry and render a live completion-lock summary.
#[derive(Debug, Parser)]
pub struct SessionsDoctorCommand {
    /// Workspace root.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,
}

impl SessionsDoctorCommand {
    pub fn run(self) -> anyhow::Result<i32> {
        let report = load_session_doctor_report(&self.path);
        println!("{}", report.render_text());
        Ok(report.cli_exit_code())
    }
}

#[cfg(test)]
mod tests {
    use super::SessionsDoctorCommand;
    use std::path::PathBuf;

    #[test]
    fn sessions_doctor_helper_compiles_for_root_paths() {
        let _ = SessionsDoctorCommand {
            path: PathBuf::from("."),
        };
    }
}
