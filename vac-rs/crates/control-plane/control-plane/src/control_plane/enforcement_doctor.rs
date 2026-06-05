use super::enforcement_level::EnforcementStatusReport;
use super::enforcement_level::load_enforcement_status_report;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnforcementDoctorReport {
    pub status: EnforcementStatusReport,
}

impl EnforcementDoctorReport {
    pub fn exit_code(&self) -> i32 {
        self.status.cli_exit_code()
    }

    pub fn render_text(&self) -> String {
        self.status.render_text()
    }
}

pub fn load_enforcement_doctor_report(workspace_root: impl AsRef<Path>) -> EnforcementDoctorReport {
    EnforcementDoctorReport {
        status: load_enforcement_status_report(workspace_root),
    }
}

#[cfg(test)]
mod tests {
    use super::load_enforcement_doctor_report;
    use std::fs;

    #[test]
    fn doctor_report_defaults_to_l1_surface() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let report = load_enforcement_doctor_report(tempdir.path());
        assert_eq!(report.exit_code(), 0);
        assert!(
            report
                .render_text()
                .contains("L1 — advisory/cooperative mode")
        );
    }

    #[test]
    fn doctor_report_flags_l2_overclaim() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let registry = tempdir.path().join(".vac/registry");
        fs::create_dir_all(&registry).expect("create registry");
        fs::write(
            registry.join("init_state.yaml"),
            "schema_version: 1\nkind: init_state\nid: init.state\nenforcement_level: L2\n",
        )
        .expect("write init state");

        let report = load_enforcement_doctor_report(tempdir.path());
        assert_eq!(report.exit_code(), 1);
        assert!(report.render_text().contains("registry claims L2"));
    }
}
