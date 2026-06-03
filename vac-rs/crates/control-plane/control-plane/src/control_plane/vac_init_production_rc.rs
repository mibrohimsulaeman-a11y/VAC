#![allow(dead_code)]
//! Production RC readiness contracts for VAC-Init.
//!
//! This module deliberately avoids self-declared "5/5" or "production ready"
//! constants. Readiness is computed from measured doctor/gate inputs, and any
//! unmeasured area is represented as `NotEvaluated` rather than `Pass`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseDoctorPass {
    pub registry: bool,
    pub surfaces: bool,
    pub policy: bool,
    pub ownership: bool,
    pub workflow: bool,
    pub evidence: bool,
    pub memory: bool,
    pub init: bool,
    pub release: bool,
    pub hard_quarantine_count: usize,
    pub broken_evidence_chain: bool,
    pub no_policy_loaded: bool,
    pub free_form_validation_commands: usize,
    pub ready_capability_missing_fields: usize,
}

impl ReleaseDoctorPass {
    pub fn validate(&self) -> Result<(), String> {
        let all_doctors = self.registry
            && self.surfaces
            && self.policy
            && self.ownership
            && self.workflow
            && self.evidence
            && self.memory
            && self.init
            && self.release;
        if !all_doctors {
            return Err("one or more required release doctors did not pass".to_string());
        }
        if self.hard_quarantine_count > 0 {
            return Err("hard quarantine blocks release".to_string());
        }
        if self.broken_evidence_chain {
            return Err("broken evidence chain blocks release".to_string());
        }
        if self.no_policy_loaded {
            return Err("missing policy blocks release".to_string());
        }
        if self.free_form_validation_commands > 0 {
            return Err("free-form validation commands block release".to_string());
        }
        if self.ready_capability_missing_fields > 0 {
            return Err("ready capability completeness blocks release".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E2eDryRunStep {
    pub command: String,
    pub passed: bool,
    pub mutates_outside_temp_workspace: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E2eDryRunScenario {
    pub steps: Vec<E2eDryRunStep>,
}

impl E2eDryRunScenario {
    pub fn validate(&self) -> Result<(), String> {
        if self.steps.is_empty() {
            return Err("E2E dry-run scenario requires steps".to_string());
        }
        for step in &self.steps {
            if !step.passed {
                return Err(format!("E2E dry-run step failed: {}", step.command));
            }
            if step.mutates_outside_temp_workspace {
                return Err(format!(
                    "E2E dry-run step mutated outside temp workspace: {}",
                    step.command
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ReadinessAreaStatus {
    Pass,
    Warning,
    Failed,
    NotEvaluated,
}

impl ReadinessAreaStatus {
    pub const fn is_pass(self) -> bool {
        matches!(self, Self::Pass)
    }

    pub const fn blocks_release(self) -> bool {
        matches!(self, Self::Failed)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Warning => "warning",
            Self::Failed => "failed",
            Self::NotEvaluated => "not_evaluated",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasuredReadinessInput {
    pub registry_strictness_passed: bool,
    pub capability_readiness_passed: bool,
    pub workflow_spec_passed: bool,
    pub cli_release_real_reports_passed: bool,
    pub runtime_gate_callsites_present: bool,
    pub durable_stores_live_passed: bool,
    pub scanner_policy_live_passed: bool,
    pub tui_real_data_passed: bool,
    pub evidence_why_live_passed: bool,
    pub minimal_build_metadata_passed: Option<bool>,
    pub minimal_e2e_dry_run_passed: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeterministicReadinessReport {
    pub registry_strictness: ReadinessAreaStatus,
    pub capability_readiness: ReadinessAreaStatus,
    pub workflow_spec: ReadinessAreaStatus,
    pub cli_surface: ReadinessAreaStatus,
    pub runtime_enforcement: ReadinessAreaStatus,
    pub durable_stores: ReadinessAreaStatus,
    pub scanner_policy: ReadinessAreaStatus,
    pub tui_real_data: ReadinessAreaStatus,
    pub evidence_why: ReadinessAreaStatus,
    pub build_test_minimal: ReadinessAreaStatus,
    pub e2e_minimal: ReadinessAreaStatus,
}

impl DeterministicReadinessReport {
    pub fn from_input(input: &MeasuredReadinessInput) -> Self {
        Self {
            registry_strictness: pass_or_failed(input.registry_strictness_passed),
            capability_readiness: pass_or_failed(input.capability_readiness_passed),
            workflow_spec: pass_or_failed(input.workflow_spec_passed),
            cli_surface: pass_or_failed(input.cli_release_real_reports_passed),
            runtime_enforcement: pass_or_failed(input.runtime_gate_callsites_present),
            durable_stores: pass_or_failed(input.durable_stores_live_passed),
            scanner_policy: pass_or_failed(input.scanner_policy_live_passed),
            tui_real_data: pass_or_failed(input.tui_real_data_passed),
            evidence_why: pass_or_failed(input.evidence_why_live_passed),
            build_test_minimal: optional_status(input.minimal_build_metadata_passed),
            e2e_minimal: optional_status(input.minimal_e2e_dry_run_passed),
        }
    }

    pub fn is_release_blocked(&self) -> bool {
        [
            self.registry_strictness,
            self.capability_readiness,
            self.workflow_spec,
            self.cli_surface,
            self.runtime_enforcement,
            self.durable_stores,
            self.scanner_policy,
            self.tui_real_data,
            self.evidence_why,
        ]
        .iter()
        .any(|status| status.blocks_release())
    }

    pub fn all_primary_areas_pass(&self) -> bool {
        [
            self.registry_strictness,
            self.capability_readiness,
            self.workflow_spec,
            self.cli_surface,
            self.runtime_enforcement,
            self.durable_stores,
            self.scanner_policy,
            self.tui_real_data,
            self.evidence_why,
        ]
        .iter()
        .all(|status| status.is_pass())
    }

    pub fn render_lines(&self) -> Vec<String> {
        [
            ("registry_strictness", self.registry_strictness),
            ("capability_readiness", self.capability_readiness),
            ("workflow_spec", self.workflow_spec),
            ("cli_surface", self.cli_surface),
            ("runtime_enforcement", self.runtime_enforcement),
            ("durable_stores", self.durable_stores),
            ("scanner_policy", self.scanner_policy),
            ("tui_real_data", self.tui_real_data),
            ("evidence_why", self.evidence_why),
            ("build_test_minimal", self.build_test_minimal),
            ("e2e_minimal", self.e2e_minimal),
        ]
        .iter()
        .map(|(name, status)| format!("{name}: {}", status.as_str()))
        .collect()
    }
}

fn pass_or_failed(value: bool) -> ReadinessAreaStatus {
    if value {
        ReadinessAreaStatus::Pass
    } else {
        ReadinessAreaStatus::Failed
    }
}

fn optional_status(value: Option<bool>) -> ReadinessAreaStatus {
    match value {
        Some(true) => ReadinessAreaStatus::Pass,
        Some(false) => ReadinessAreaStatus::Failed,
        None => ReadinessAreaStatus::NotEvaluated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_doctor_blocks_core_release_failures() {
        let pass = ReleaseDoctorPass {
            registry: true,
            surfaces: true,
            policy: true,
            ownership: true,
            workflow: true,
            evidence: true,
            memory: true,
            init: true,
            release: true,
            hard_quarantine_count: 0,
            broken_evidence_chain: false,
            no_policy_loaded: false,
            free_form_validation_commands: 0,
            ready_capability_missing_fields: 0,
        };
        assert!(pass.validate().is_ok());

        let blocked = ReleaseDoctorPass {
            hard_quarantine_count: 1,
            ..pass
        };
        assert!(blocked.validate().is_err());
    }

    #[test]
    fn e2e_dry_run_rejects_failed_or_mutating_steps() {
        let scenario = E2eDryRunScenario {
            steps: vec![
                E2eDryRunStep {
                    command: "vac init --dry-run".to_string(),
                    passed: true,
                    mutates_outside_temp_workspace: false,
                },
                E2eDryRunStep {
                    command: "vac doctor release .".to_string(),
                    passed: true,
                    mutates_outside_temp_workspace: false,
                },
            ],
        };
        assert!(scenario.validate().is_ok());

        let bad = E2eDryRunScenario {
            steps: vec![E2eDryRunStep {
                command: "vac init".to_string(),
                passed: true,
                mutates_outside_temp_workspace: true,
            }],
        };
        assert!(bad.validate().is_err());
    }

    #[test]
    fn deterministic_readiness_is_computed_from_measured_inputs() {
        let input = MeasuredReadinessInput {
            registry_strictness_passed: true,
            capability_readiness_passed: true,
            workflow_spec_passed: false,
            cli_release_real_reports_passed: true,
            runtime_gate_callsites_present: false,
            durable_stores_live_passed: true,
            scanner_policy_live_passed: true,
            tui_real_data_passed: true,
            evidence_why_live_passed: true,
            minimal_build_metadata_passed: None,
            minimal_e2e_dry_run_passed: Some(true),
        };
        let report = DeterministicReadinessReport::from_input(&input);
        assert!(report.is_release_blocked());
        assert_eq!(report.workflow_spec, ReadinessAreaStatus::Failed);
        assert_eq!(report.runtime_enforcement, ReadinessAreaStatus::Failed);
        assert_eq!(report.build_test_minimal, ReadinessAreaStatus::NotEvaluated);
        assert_eq!(report.e2e_minimal, ReadinessAreaStatus::Pass);
    }
}
