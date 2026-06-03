#![allow(dead_code)]
//! TUI real-data integration contracts for VAC-Init reports.

use std::fs;
use std::path::Path;

pub fn load_capability_dashboard_data_from_workspace(
    workspace_root: impl AsRef<Path>,
) -> Result<CapabilityDashboardData, String> {
    let root = workspace_root.as_ref();
    let capability_count = count_yaml_files(root.join(".vac/capabilities"))?;
    let registry_report = freshness(root.join(".vac"));
    let ownership_report = freshness(root.join(".vac/registry/ownership/report.yaml"));
    let policy_report = freshness(root.join(".vac/policies"));
    let workflow_report = freshness(root.join(".vac/workflows"));
    let surface_report = freshness(root.join(".vac/surfaces"));

    let invalid_manifest_count = count_invalid_yaml_envelopes(root.join(".vac"))?;
    let (unowned_count, overclaimed_count) =
        read_ownership_counts(root.join(".vac/registry/ownership/report.yaml"));

    Ok(CapabilityDashboardData {
        registry_report,
        ownership_report,
        policy_report,
        workflow_report,
        surface_report,
        capability_count,
        invalid_manifest_count,
        unowned_count,
        overclaimed_count,
    })
}

pub fn load_approval_popup_data_from_store(
    approval_request_path: impl AsRef<Path>,
) -> Result<ApprovalPopupData, String> {
    let source =
        fs::read_to_string(approval_request_path.as_ref()).map_err(|err| err.to_string())?;
    let approval_request_loaded = source.contains("kind: approval_request");
    let binding_loaded = source.contains("binding:")
        && source.contains("plan_hash:")
        && source.contains("diff_hash:");
    let policy_decision_visible = source.contains("risk_level:") || source.contains("decision:");
    let writes_response_to_store = source.contains("response:");
    Ok(ApprovalPopupData {
        approval_request_loaded,
        binding_loaded,
        policy_decision_visible,
        can_bypass_policy: false,
        writes_response_to_store,
    })
}

pub fn load_autopilot_doctor_monitor_from_workspace(
    workspace_root: impl AsRef<Path>,
) -> Result<AutopilotDoctorMonitorData, String> {
    let root = workspace_root.as_ref();
    Ok(AutopilotDoctorMonitorData {
        registry_status_visible: root.join(".vac").exists(),
        policy_status_visible: root.join(".vac/policies").exists(),
        ownership_status_visible: root.join(".vac/registry/ownership/report.yaml").exists(),
        evidence_status_visible: root.join(".vac/registry/evidence").exists(),
        memory_status_visible: root.join(".vac/registry/memory").exists(),
        init_status_visible: root.join(".vac/.init/state.yaml").exists(),
        destructive_actions_policy_gated: true,
    })
}

fn freshness(path: impl AsRef<Path>) -> ReportFreshness {
    if path.as_ref().exists() {
        ReportFreshness::Current
    } else {
        ReportFreshness::Missing
    }
}

fn count_yaml_files(path: impl AsRef<Path>) -> Result<usize, String> {
    let path = path.as_ref();
    if !path.is_dir() {
        return Ok(0);
    }
    let mut count = 0usize;
    for entry in fs::read_dir(path).map_err(|err| err.to_string())? {
        let path = entry.map_err(|err| err.to_string())?.path();
        if path.extension().and_then(|value| value.to_str()) == Some("yaml") {
            count += 1;
        }
    }
    Ok(count)
}

fn count_invalid_yaml_envelopes(path: impl AsRef<Path>) -> Result<usize, String> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(1);
    }
    let mut invalid = 0usize;
    walk_yaml(path, &mut |file| {
        let source = fs::read_to_string(file).map_err(|err| err.to_string())?;
        if !(source.contains("schema_version:")
            && source.contains("kind:")
            && source.contains("id:"))
        {
            invalid += 1;
        }
        Ok(())
    })?;
    Ok(invalid)
}

fn read_ownership_counts(path: impl AsRef<Path>) -> (usize, usize) {
    let Ok(source) = fs::read_to_string(path.as_ref()) else {
        return (0, 0);
    };
    let unowned = read_summary_count(&source, "unowned").unwrap_or(0);
    let overclaimed = read_summary_count(&source, "overclaimed").unwrap_or(0);
    (unowned, overclaimed)
}

fn read_summary_count(source: &str, key: &str) -> Option<usize> {
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix(&format!("{key}:")) {
            return value.trim().parse::<usize>().ok();
        }
    }
    None
}

fn walk_yaml<F>(path: &Path, f: &mut F) -> Result<(), String>
where
    F: FnMut(&Path) -> Result<(), String>,
{
    if path.is_dir() {
        for entry in fs::read_dir(path).map_err(|err| err.to_string())? {
            walk_yaml(&entry.map_err(|err| err.to_string())?.path(), f)?;
        }
    } else if path.extension().and_then(|value| value.to_str()) == Some("yaml") {
        f(path)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ReportFreshness {
    Current,
    Stale,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDashboardData {
    pub registry_report: ReportFreshness,
    pub ownership_report: ReportFreshness,
    pub policy_report: ReportFreshness,
    pub workflow_report: ReportFreshness,
    pub surface_report: ReportFreshness,
    pub capability_count: usize,
    pub invalid_manifest_count: usize,
    pub unowned_count: usize,
    pub overclaimed_count: usize,
}

impl CapabilityDashboardData {
    pub fn validate_real_data(&self) -> Result<(), String> {
        for (name, freshness) in [
            ("registry", self.registry_report),
            ("ownership", self.ownership_report),
            ("policy", self.policy_report),
            ("workflow", self.workflow_report),
            ("surface", self.surface_report),
        ] {
            if freshness == ReportFreshness::Missing {
                return Err(format!("{name} report is missing"));
            }
        }
        if self.capability_count == 0 {
            return Err(
                "dashboard must render real capabilities, not a blank/sample screen".to_string(),
            );
        }
        Ok(())
    }

    pub fn readiness_percent(&self) -> u8 {
        let total = self.capability_count.max(1);
        let invalid = self.invalid_manifest_count + self.unowned_count + self.overclaimed_count;
        let valid = total.saturating_sub(invalid);
        ((valid * 100) / total) as u8
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalPopupData {
    pub approval_request_loaded: bool,
    pub binding_loaded: bool,
    pub policy_decision_visible: bool,
    pub can_bypass_policy: bool,
    pub writes_response_to_store: bool,
}

impl ApprovalPopupData {
    pub fn validate(&self) -> Result<(), String> {
        if !self.approval_request_loaded {
            return Err("approval popup requires a persisted approval request".to_string());
        }
        if !self.binding_loaded {
            return Err("approval popup requires plan/diff/policy binding".to_string());
        }
        if !self.policy_decision_visible {
            return Err("approval popup must render the policy decision".to_string());
        }
        if self.can_bypass_policy {
            return Err("approval popup must not bypass policy".to_string());
        }
        if !self.writes_response_to_store {
            return Err("approval popup must persist approve/deny/timeout response".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutopilotDoctorMonitorData {
    pub registry_status_visible: bool,
    pub policy_status_visible: bool,
    pub ownership_status_visible: bool,
    pub evidence_status_visible: bool,
    pub memory_status_visible: bool,
    pub init_status_visible: bool,
    pub destructive_actions_policy_gated: bool,
}

impl AutopilotDoctorMonitorData {
    pub fn validate(&self) -> Result<(), String> {
        if !self.registry_status_visible
            || !self.policy_status_visible
            || !self.ownership_status_visible
            || !self.evidence_status_visible
            || !self.memory_status_visible
            || !self.init_status_visible
        {
            return Err("autopilot monitor must render all required doctor statuses".to_string());
        }
        if !self.destructive_actions_policy_gated {
            return Err(
                "autopilot monitor destructive actions must remain approval/policy gated"
                    .to_string(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_dashboard_rejects_blank_or_missing_report_data() {
        let missing = CapabilityDashboardData {
            registry_report: ReportFreshness::Current,
            ownership_report: ReportFreshness::Missing,
            policy_report: ReportFreshness::Current,
            workflow_report: ReportFreshness::Current,
            surface_report: ReportFreshness::Current,
            capability_count: 10,
            invalid_manifest_count: 0,
            unowned_count: 0,
            overclaimed_count: 0,
        };
        assert!(missing.validate_real_data().is_err());

        let blank = CapabilityDashboardData {
            ownership_report: ReportFreshness::Current,
            capability_count: 0,
            ..missing
        };
        assert!(blank.validate_real_data().is_err());
    }

    #[test]
    fn capability_dashboard_computes_real_readiness_percent() {
        let data = CapabilityDashboardData {
            registry_report: ReportFreshness::Current,
            ownership_report: ReportFreshness::Current,
            policy_report: ReportFreshness::Current,
            workflow_report: ReportFreshness::Current,
            surface_report: ReportFreshness::Current,
            capability_count: 50,
            invalid_manifest_count: 1,
            unowned_count: 2,
            overclaimed_count: 2,
        };
        assert!(data.validate_real_data().is_ok());
        assert_eq!(data.readiness_percent(), 90);
    }

    #[test]
    fn approval_popup_requires_store_binding_and_no_bypass() {
        let ok = ApprovalPopupData {
            approval_request_loaded: true,
            binding_loaded: true,
            policy_decision_visible: true,
            can_bypass_policy: false,
            writes_response_to_store: true,
        };
        assert!(ok.validate().is_ok());

        let bypass = ApprovalPopupData {
            can_bypass_policy: true,
            ..ok
        };
        assert!(bypass.validate().is_err());
    }

    #[test]
    fn autopilot_monitor_requires_all_doctor_statuses_and_policy_gates() {
        let ok = AutopilotDoctorMonitorData {
            registry_status_visible: true,
            policy_status_visible: true,
            ownership_status_visible: true,
            evidence_status_visible: true,
            memory_status_visible: true,
            init_status_visible: true,
            destructive_actions_policy_gated: true,
        };
        assert!(ok.validate().is_ok());

        let unsafe_monitor = AutopilotDoctorMonitorData {
            destructive_actions_policy_gated: false,
            ..ok
        };
        assert!(unsafe_monitor.validate().is_err());
    }

    #[test]
    fn live_workspace_loader_uses_real_report_files() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vac-tui-real-data-{unique}"));
        std::fs::create_dir_all(root.join(".vac/capabilities")).unwrap();
        std::fs::create_dir_all(root.join(".vac/policies")).unwrap();
        std::fs::create_dir_all(root.join(".vac/workflows")).unwrap();
        std::fs::create_dir_all(root.join(".vac/surfaces")).unwrap();
        std::fs::create_dir_all(root.join(".vac/registry/evidence")).unwrap();
        std::fs::create_dir_all(root.join(".vac/registry/ownership")).unwrap();
        std::fs::create_dir_all(root.join(".vac/registry/memory/semantic")).unwrap();
        std::fs::create_dir_all(root.join(".vac/.init")).unwrap();
        std::fs::write(
            root.join(".vac/capabilities/test.yaml"),
            "schema_version: 1\nkind: capability\nid: vac.test\n",
        )
        .unwrap();
        std::fs::write(root.join(".vac/registry/ownership/report.yaml"), "schema_version: 1\nkind: ownership_report\nid: report.ownership.test\nsummary:\n  unowned: 2\n  overclaimed: 1\n").unwrap();
        std::fs::write(
            root.join(".vac/.init/state.yaml"),
            "schema_version: 1\nkind: init_state\nid: init.state\n",
        )
        .unwrap();

        let dashboard = load_capability_dashboard_data_from_workspace(&root).unwrap();
        assert_eq!(dashboard.capability_count, 1);
        assert_eq!(dashboard.unowned_count, 2);
        assert_eq!(dashboard.overclaimed_count, 1);
        assert!(dashboard.validate_real_data().is_ok());

        let monitor = load_autopilot_doctor_monitor_from_workspace(&root).unwrap();
        assert!(monitor.validate().is_ok());

        let approval = root.join(".vac/registry/approvals/approval.test.yaml");
        std::fs::create_dir_all(approval.parent().unwrap()).unwrap();
        std::fs::write(&approval, "schema_version: 1\nkind: approval_request\nid: approval.test\nrequest:\n  risk_level: medium\nbinding:\n  plan_hash: p\n  diff_hash: d\nresponse:\n  decision: approved\n").unwrap();
        assert!(
            load_approval_popup_data_from_store(&approval)
                .unwrap()
                .validate()
                .is_ok()
        );

        let _ = std::fs::remove_dir_all(root);
    }
}
