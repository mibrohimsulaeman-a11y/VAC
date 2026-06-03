use super::approval_store::FileApprovalStore;
use super::workflow_runner::WORKFLOW_STEP_VOCABULARY;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalReadinessReport {
    pub capability_manifest_ready: bool,
    pub approval_policy_present: bool,
    pub release_workflow_has_approval_step: bool,
    pub workflow_vocabulary_has_approval_step: bool,
    pub durable_store_available: bool,
    pub diagnostics: Vec<String>,
}

impl ApprovalReadinessReport {
    pub fn is_ready(&self) -> bool {
        self.capability_manifest_ready
            && self.approval_policy_present
            && self.release_workflow_has_approval_step
            && self.workflow_vocabulary_has_approval_step
            && self.durable_store_available
            && self.diagnostics.is_empty()
    }

    pub fn render_text(&self) -> String {
        let mut lines = vec![format!(
            "approval readiness: ready={} capability_manifest_ready={} approval_policy_present={} release_workflow_has_approval_step={} workflow_vocabulary_has_approval_step={} durable_store_available={}",
            self.is_ready(),
            self.capability_manifest_ready,
            self.approval_policy_present,
            self.release_workflow_has_approval_step,
            self.workflow_vocabulary_has_approval_step,
            self.durable_store_available,
        )];
        if !self.diagnostics.is_empty() {
            lines.push("diagnostics:".to_string());
            lines.extend(self.diagnostics.iter().map(|line| format!("  {line}")));
        }
        lines.join("\n")
    }
}

pub fn load_approval_readiness_report(repo_root: impl AsRef<Path>) -> ApprovalReadinessReport {
    let repo_root = repo_root.as_ref();
    let capability = read_to_string(repo_root.join(".vac/capabilities/approvals.yaml"));
    let policy = read_to_string(repo_root.join(".vac/policies/approval.yaml"));
    let release = read_to_string(repo_root.join(".vac/workflows/maintenance.release-gate.yaml"));

    let capability_manifest_ready = capability
        .as_deref()
        .map(|text| {
            has_yaml_field(text, "id", "vac.approvals") && has_yaml_field(text, "status", "ready")
        })
        .unwrap_or(false);
    let approval_policy_present = policy
        .as_deref()
        .map(|text| {
            has_yaml_field(text, "id", "vac.policy.approval") && text.contains("approval_required")
        })
        .unwrap_or(false);
    let release_workflow_has_approval_step = release
        .as_deref()
        .map(|text| text.contains("uses: capability.approval.request"))
        .unwrap_or(false);
    let workflow_vocabulary_has_approval_step = WORKFLOW_STEP_VOCABULARY.iter().any(|entry| {
        entry.uses == "capability.approval.request"
            && entry.canonical_capability_id == "vac.approvals"
    });

    let durable_store_type = std::any::type_name::<FileApprovalStore>();
    let durable_store_available = durable_store_type.ends_with("FileApprovalStore");

    let mut diagnostics = Vec::new();
    if !capability_manifest_ready {
        diagnostics.push("approvals capability manifest is missing or not ready".to_string());
    }
    if !approval_policy_present {
        diagnostics.push(
            "approval policy manifest is missing or lacks approval_required fallback".to_string(),
        );
    }
    if !release_workflow_has_approval_step {
        diagnostics.push("maintenance.release-gate lacks capability.approval.request".to_string());
    }
    if !workflow_vocabulary_has_approval_step {
        diagnostics
            .push("workflow vocabulary lacks canonical approval request mapping".to_string());
    }
    if !durable_store_available {
        diagnostics.push("durable FileApprovalStore is not reachable".to_string());
    }

    ApprovalReadinessReport {
        capability_manifest_ready,
        approval_policy_present,
        release_workflow_has_approval_step,
        workflow_vocabulary_has_approval_step,
        durable_store_available,
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
        let root = std::env::temp_dir().join(format!("vac-approval-readiness-{nonce}"));
        fs::create_dir_all(root.join(".vac/capabilities")).expect("capabilities dir");
        fs::create_dir_all(root.join(".vac/policies")).expect("policies dir");
        fs::create_dir_all(root.join(".vac/workflows")).expect("workflows dir");
        root
    }

    #[test]
    fn approval_readiness_passes_when_required_manifests_and_vocabulary_are_present() {
        let root = fixture_root();
        fs::write(
            root.join(".vac/capabilities/approvals.yaml"),
            "id: vac.approvals\nstatus: ready\n",
        )
        .expect("capability");
        fs::write(
            root.join(".vac/policies/approval.yaml"),
            "id: vac.policy.approval\ndefault_decision: approval_required\n",
        )
        .expect("policy");
        fs::write(
            root.join(".vac/workflows/maintenance.release-gate.yaml"),
            "steps:\n  - id: policy_check\n    uses: capability.approval.request\n",
        )
        .expect("workflow");

        let report = load_approval_readiness_report(&root);
        assert!(report.is_ready(), "{}", report.render_text());
    }

    #[test]
    fn approval_readiness_reports_missing_policy_as_diagnostic() {
        let root = fixture_root();
        fs::write(
            root.join(".vac/capabilities/approvals.yaml"),
            "id: vac.approvals\nstatus: ready\n",
        )
        .expect("capability");
        fs::write(
            root.join(".vac/workflows/maintenance.release-gate.yaml"),
            "steps:\n  - id: policy_check\n    uses: capability.approval.request\n",
        )
        .expect("workflow");

        let report = load_approval_readiness_report(&root);
        assert!(!report.is_ready());
        assert!(
            report
                .diagnostics
                .iter()
                .any(|line| line.contains("approval policy manifest")),
            "{}",
            report.render_text()
        );
    }
}
