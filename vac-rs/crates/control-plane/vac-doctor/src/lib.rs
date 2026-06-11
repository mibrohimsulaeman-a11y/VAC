//! VAC doctor structural validation contracts.
//!
//! Doctor checks are no longer file-existence markers only: every gate carries
//! structural pass criteria, failure code, severity, and release aggregation.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DoctorResult {
    pub gate: String,
    pub status: DoctorStatus,
    pub warnings: Vec<String>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DoctorStatus {
    Pass,
    Warn,
    Fail,
    Fatal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DoctorCheck {
    pub id: String,
    pub gate: DoctorGate,
    pub pass_criteria: Vec<String>,
    pub block_criteria: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DoctorGate {
    Registry,
    Compiled,
    Index,
    Intent,
    Assessment,
    SpecSync,
    Ownership,
    Policy,
    Evidence,
    Workflow,
    Memory,
    Enforcement,
    Release,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseDoctorReport {
    pub schema_version: u32,
    pub kind: String,
    pub checks: Vec<DoctorResult>,
}

impl DoctorResult {
    #[must_use]
    pub fn pass(gate: impl Into<String>) -> Self {
        Self {
            gate: gate.into(),
            status: DoctorStatus::Pass,
            warnings: Vec::new(),
            failures: Vec::new(),
        }
    }

    #[must_use]
    pub fn fail(gate: impl Into<String>, failure: impl Into<String>) -> Self {
        Self {
            gate: gate.into(),
            status: DoctorStatus::Fail,
            warnings: Vec::new(),
            failures: vec![failure.into()],
        }
    }

    #[must_use]
    pub fn blocks_release(&self) -> bool {
        matches!(self.status, DoctorStatus::Fail | DoctorStatus::Fatal)
    }
}

impl ReleaseDoctorReport {
    #[must_use]
    pub fn aggregate(checks: Vec<DoctorResult>) -> Self {
        Self {
            schema_version: 1,
            kind: "release_doctor_report".to_string(),
            checks,
        }
    }

    #[must_use]
    pub fn status(&self) -> DoctorStatus {
        if self
            .checks
            .iter()
            .any(|check| matches!(check.status, DoctorStatus::Fatal))
        {
            DoctorStatus::Fatal
        } else if self
            .checks
            .iter()
            .any(|check| matches!(check.status, DoctorStatus::Fail))
        {
            DoctorStatus::Fail
        } else if self
            .checks
            .iter()
            .any(|check| matches!(check.status, DoctorStatus::Warn))
        {
            DoctorStatus::Warn
        } else {
            DoctorStatus::Pass
        }
    }
}

#[must_use]
pub fn aggregate_release_status(checks: Vec<DoctorResult>) -> DoctorStatus {
    ReleaseDoctorReport::aggregate(checks).status()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssessmentFreshnessDoctorInput {
    pub findings_checked: u32,
    pub stale_span_ids: u32,
    pub stale_span_hashes: u32,
    pub baseline_count_mismatches: u32,
}

#[must_use]
pub fn doctor_assessment_freshness(input: &AssessmentFreshnessDoctorInput) -> DoctorResult {
    let total_failures = input
        .stale_span_ids
        .saturating_add(input.stale_span_hashes)
        .saturating_add(input.baseline_count_mismatches);
    if total_failures == 0 {
        let mut result = DoctorResult::pass("assessment_freshness");
        if input.findings_checked == 0 {
            result.status = DoctorStatus::Warn;
            result
                .warnings
                .push("no assessment findings were checked".to_string());
        }
        result
    } else {
        DoctorResult::fail(
            "assessment_freshness",
            format!(
                "assessment artifacts are stale against deterministic index: {total_failures} mismatch(es)"
            ),
        )
    }
}

#[must_use]
pub fn doctor_release_with_v1_5_gates(mut checks: Vec<DoctorResult>) -> ReleaseDoctorReport {
    let required = [
        "compiled_source_hashes",
        "index_counts",
        "assessment_freshness",
        "spec_sync",
        "readiness_authority",
        "runtime_realpath_e2e",
    ];
    for gate in required {
        if !checks.iter().any(|check| check.gate == gate) {
            checks.push(DoctorResult::fail(
                gate,
                "required v1.5 release doctor gate did not run",
            ));
        }
    }
    ReleaseDoctorReport::aggregate(checks)
}

#[must_use]
pub fn product_path_gate(gate: &str, ok: bool, warnings: Vec<String>) -> DoctorResult {
    DoctorResult {
        gate: gate.to_string(),
        status: if ok {
            if warnings.is_empty() {
                DoctorStatus::Pass
            } else {
                DoctorStatus::Warn
            }
        } else {
            DoctorStatus::Fail
        },
        warnings,
        failures: if ok {
            Vec::new()
        } else {
            vec![format!("{gate} failed product-path structural validation")]
        },
    }
}

#[must_use]
pub fn release_blocks_on(result: &DoctorResult) -> bool {
    matches!(result.status, DoctorStatus::Fail | DoctorStatus::Fatal)
}
