use super::build_check::BuildCheckRequest;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildReadinessReport {
    pub capability_manifest_ready: bool,
    pub workflow_ready: bool,
    pub doctor_build_validation_declared: bool,
    pub targeted_command_is_allowlisted: bool,
    pub full_workspace_policy_recorded: bool,
    pub diagnostics: Vec<String>,
}

impl BuildReadinessReport {
    pub fn is_ready(&self) -> bool {
        self.capability_manifest_ready
            && self.workflow_ready
            && self.doctor_build_validation_declared
            && self.targeted_command_is_allowlisted
            && self.full_workspace_policy_recorded
            && self.diagnostics.is_empty()
    }

    pub fn render_text(&self) -> String {
        let mut lines = vec![format!(
            "build readiness: ready={} capability_manifest_ready={} workflow_ready={} doctor_build_validation_declared={} targeted_command_is_allowlisted={} full_workspace_policy_recorded={}",
            self.is_ready(),
            self.capability_manifest_ready,
            self.workflow_ready,
            self.doctor_build_validation_declared,
            self.targeted_command_is_allowlisted,
            self.full_workspace_policy_recorded,
        )];
        if !self.diagnostics.is_empty() {
            lines.push("diagnostics:".to_string());
            lines.extend(self.diagnostics.iter().map(|line| format!("  {line}")));
        }
        lines.join("\n")
    }
}

pub fn load_build_readiness_report(repo_root: impl AsRef<Path>) -> BuildReadinessReport {
    let repo_root = repo_root.as_ref();
    let capability = read_to_string(repo_root.join(".vac/capabilities/build.yaml"));
    let workflow = read_to_string(repo_root.join(".vac/workflows/maintenance.build-check.yaml"));
    let request = BuildCheckRequest::for_repo_root(repo_root);
    let command = request.command_display();

    let capability_manifest_ready = capability
        .as_deref()
        .map(|text| {
            has_yaml_field(text, "id", "vac.build") && has_yaml_field(text, "status", "ready")
        })
        .unwrap_or(false);
    let doctor_build_validation_declared = capability
        .as_deref()
        .map(|text| text.contains("vac doctor build .") || text.contains("vac doctor build"))
        .unwrap_or(false);
    let full_workspace_policy_recorded = capability
        .as_deref()
        .map(|text| text.contains("operator-gated") && text.contains("full workspace"))
        .unwrap_or(false);
    let workflow_ready = workflow
        .as_deref()
        .map(|text| {
            has_yaml_field(text, "status", "ready")
                && text.contains("uses: capability.build.cargo_check")
                && text.contains("approval_required_for:")
                && text.contains("execute_process")
        })
        .unwrap_or(false);
    let targeted_command_is_allowlisted = command.contains("cargo +")
        && command.contains("check --manifest-path vac-rs/Cargo.toml")
        && command.contains("-p vac-surface-cli")
        && request.jobs == 1
        && !request.incremental;

    let mut diagnostics = Vec::new();
    if !capability_manifest_ready {
        diagnostics.push("build capability manifest is missing or not ready".to_string());
    }
    if !workflow_ready {
        diagnostics.push(
            "maintenance.build-check workflow is missing readiness, build step, or execute_process approval"
                .to_string(),
        );
    }
    if !doctor_build_validation_declared {
        diagnostics
            .push("build capability validation does not declare vac doctor build".to_string());
    }
    if !targeted_command_is_allowlisted {
        diagnostics.push(format!(
            "build command is not the expected serial targeted check: {command}"
        ));
    }
    if !full_workspace_policy_recorded {
        diagnostics.push(
            "build capability does not record full workspace build as operator-gated evidence"
                .to_string(),
        );
    }

    BuildReadinessReport {
        capability_manifest_ready,
        workflow_ready,
        doctor_build_validation_declared,
        targeted_command_is_allowlisted,
        full_workspace_policy_recorded,
        diagnostics,
    }
}

fn read_to_string(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path).ok()
}

fn has_yaml_field(text: &str, key: &str, value: &str) -> bool {
    let prefix = format!("{key}:");
    text.lines().any(|line| {
        line.trim()
            .strip_prefix(&prefix)
            .map(|rest| rest.trim().trim_matches('"').trim_matches('\'') == value)
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_root() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vac-build-readiness-{nonce}"));
        fs::create_dir_all(root.join(".vac/capabilities")).expect("capabilities dir");
        fs::create_dir_all(root.join(".vac/workflows")).expect("workflows dir");
        root
    }

    #[test]
    fn build_readiness_passes_for_ready_manifest_workflow_and_operator_gated_full_build() {
        let root = fixture_root();
        fs::write(
            root.join(".vac/capabilities/build.yaml"),
            "id: vac.build\nstatus: ready\nreason: targeted vac doctor build is ready; full workspace build remains operator-gated release evidence.\nvalidation:\n  commands:\n    - vac doctor build .\n",
        )
        .expect("capability");
        fs::write(
            root.join(".vac/workflows/maintenance.build-check.yaml"),
            "status: ready\npolicy:\n  approval_required_for:\n    - execute_process\nsteps:\n  - id: validate\n    uses: capability.build.cargo_check\n",
        )
        .expect("workflow");

        let report = load_build_readiness_report(&root);
        assert!(report.is_ready(), "{}", report.render_text());
    }

    #[test]
    fn build_readiness_requires_execute_process_approval() {
        let root = fixture_root();
        fs::write(
            root.join(".vac/capabilities/build.yaml"),
            "id: vac.build\nstatus: ready\nreason: targeted vac doctor build is ready; full workspace build remains operator-gated release evidence.\nvalidation:\n  commands:\n    - vac doctor build .\n",
        )
        .expect("capability");
        fs::write(
            root.join(".vac/workflows/maintenance.build-check.yaml"),
            "status: ready\nsteps:\n  - id: validate\n    uses: capability.build.cargo_check\n",
        )
        .expect("workflow");

        let report = load_build_readiness_report(&root);
        assert!(!report.is_ready());
        assert!(
            report
                .diagnostics
                .iter()
                .any(|line| line.contains("execute_process approval")),
            "{}",
            report.render_text()
        );
    }
}
