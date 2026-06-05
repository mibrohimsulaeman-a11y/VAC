use clap::Parser;
use std::path::PathBuf;
use vac_core::control_plane::load_enforcement_doctor_report;

/// Validate enforcement claims against the observed substrate.
#[derive(Debug, Parser)]
pub struct EnforcementDoctorCommand {
    /// Workspace root.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,
}

impl EnforcementDoctorCommand {
    pub fn run(self) -> anyhow::Result<i32> {
        let report = load_enforcement_doctor_report(&self.path);
        println!("{}", report.render_text());
        Ok(report.exit_code())
    }
}

#[cfg(test)]
mod tests {
    use super::EnforcementDoctorCommand;
    use std::path::PathBuf;

    #[test]
    fn enforcement_doctor_helper_compiles_for_root_paths() {
        let _ = EnforcementDoctorCommand {
            path: PathBuf::from("."),
        };
    }
}
