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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceWeightedEvent {
    pub event_type: String,
    pub count: u64,
    pub weight: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceSliceRiskInput {
    pub risk_class: String,
    pub count: u64,
    pub points_each: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceSliceRiskBreakdown {
    pub risk_class: String,
    pub count: u64,
    pub points_each: u64,
    pub points: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceThresholds {
    pub warn_at_bps: u64,
    pub advisory_block_at_bps: u64,
    pub release_block_at_bps: u64,
}

impl Default for GovernanceThresholds {
    fn default() -> Self {
        Self {
            warn_at_bps: 2_500,
            advisory_block_at_bps: 4_500,
            release_block_at_bps: 7_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceWindowInput {
    pub weighted_events: Vec<GovernanceWeightedEvent>,
    pub slice_risk_points: Vec<GovernanceSliceRiskInput>,
    pub thresholds: GovernanceThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceWindowReport {
    pub weighted_event_points: u64,
    pub risk_weighted_slice_points: u64,
    pub denominator: u64,
    pub score_basis_points: u64,
    pub threshold_state: String,
    pub denominator_breakdown: Vec<GovernanceSliceRiskBreakdown>,
}

#[must_use]
pub fn reduce_governance_window(input: &GovernanceWindowInput) -> GovernanceWindowReport {
    let weighted_event_points = input
        .weighted_events
        .iter()
        .map(|event| event.count.saturating_mul(event.weight))
        .sum::<u64>();
    let denominator_breakdown = input
        .slice_risk_points
        .iter()
        .map(|slice| GovernanceSliceRiskBreakdown {
            risk_class: slice.risk_class.clone(),
            count: slice.count,
            points_each: slice.points_each,
            points: slice.count.saturating_mul(slice.points_each),
        })
        .collect::<Vec<_>>();
    let risk_weighted_slice_points = denominator_breakdown
        .iter()
        .map(|slice| slice.points)
        .sum::<u64>();
    let denominator = risk_weighted_slice_points.max(1);
    let score_basis_points = weighted_event_points
        .saturating_mul(10_000)
        .saturating_div(denominator);
    let threshold_state = if score_basis_points >= input.thresholds.release_block_at_bps {
        "release_block"
    } else if score_basis_points >= input.thresholds.advisory_block_at_bps {
        "advisory_block"
    } else if score_basis_points >= input.thresholds.warn_at_bps {
        "warn"
    } else {
        "pass"
    }
    .to_string();

    GovernanceWindowReport {
        weighted_event_points,
        risk_weighted_slice_points,
        denominator,
        score_basis_points,
        threshold_state,
        denominator_breakdown,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseTrustClaimInput {
    pub execution: String,
    pub custody: String,
    pub proof_verified: bool,
    pub aggregate_claim: String,
}

fn claim_contains_stronger_than_l1_local_language(claim: &str) -> bool {
    let lower = claim.to_ascii_lowercase();
    [
        "audit-grade",
        "tamper-evident",
        "tamper evident",
        "broker-mediated",
        "broker mediated",
        "ci-attested",
        "ci attested",
        "external-attested",
        "external attested",
        "enforced l2",
        "untrusted-agent safe",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

#[must_use]
pub fn doctor_release_trust_language(input: &ReleaseTrustClaimInput) -> DoctorResult {
    let l1_local = input.execution == "observed_l1"
        && matches!(input.custody.as_str(), "local_only" | "self_promoted");
    let unverified_attestation = matches!(
        input.custody.as_str(),
        "ci_attested" | "broker_attested" | "external_attested"
    ) && !input.proof_verified;

    if l1_local && claim_contains_stronger_than_l1_local_language(&input.aggregate_claim) {
        return DoctorResult::fail(
            "release",
            "release doctor must not overclaim L1 local/self-promoted records as audit-grade, tamper-evident, CI-attested, broker-mediated, or externally attested",
        );
    }

    if unverified_attestation {
        return DoctorResult::fail(
            "release",
            "release doctor must downgrade unverified attestation claims at read time",
        );
    }

    let mut result = DoctorResult::pass("release");
    if l1_local {
        result.status = DoctorStatus::Warn;
        result.warnings.push(
            "release claim is L1 local/self-promoted; local evidence is an integrity hint only"
                .to_string(),
        );
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn governance_zero_denominator_uses_max_one_and_passes() {
        let report = reduce_governance_window(&GovernanceWindowInput {
            weighted_events: Vec::new(),
            slice_risk_points: Vec::new(),
            thresholds: GovernanceThresholds::default(),
        });
        assert_eq!(report.risk_weighted_slice_points, 0);
        assert_eq!(report.denominator, 1);
        assert_eq!(report.score_basis_points, 0);
        assert_eq!(report.threshold_state, "pass");
    }

    #[test]
    fn governance_reducer_matches_spec_denominator_breakdown() {
        let report = reduce_governance_window(&GovernanceWindowInput {
            weighted_events: vec![
                GovernanceWeightedEvent {
                    event_type: "needs_discussion".to_string(),
                    count: 3,
                    weight: 12,
                },
                GovernanceWeightedEvent {
                    event_type: "needs_discussion_release_relevant".to_string(),
                    count: 2,
                    weight: 80,
                },
                GovernanceWeightedEvent {
                    event_type: "scoped_grant".to_string(),
                    count: 1,
                    weight: 80,
                },
                GovernanceWeightedEvent {
                    event_type: "policy_downgrade".to_string(),
                    count: 1,
                    weight: 300,
                },
            ],
            slice_risk_points: vec![
                GovernanceSliceRiskInput {
                    risk_class: "micro".to_string(),
                    count: 6,
                    points_each: 5,
                },
                GovernanceSliceRiskInput {
                    risk_class: "low".to_string(),
                    count: 5,
                    points_each: 10,
                },
                GovernanceSliceRiskInput {
                    risk_class: "medium".to_string(),
                    count: 4,
                    points_each: 30,
                },
                GovernanceSliceRiskInput {
                    risk_class: "high".to_string(),
                    count: 3,
                    points_each: 80,
                },
                GovernanceSliceRiskInput {
                    risk_class: "critical".to_string(),
                    count: 1,
                    points_each: 200,
                },
                GovernanceSliceRiskInput {
                    risk_class: "release".to_string(),
                    count: 1,
                    points_each: 300,
                },
            ],
            thresholds: GovernanceThresholds::default(),
        });
        assert_eq!(report.weighted_event_points, 576);
        assert_eq!(report.risk_weighted_slice_points, 940);
        assert_eq!(report.denominator, 940);
        assert_eq!(report.score_basis_points, 6127);
        assert_eq!(report.threshold_state, "advisory_block");
        assert_eq!(report.denominator_breakdown[0].points, 30);
    }

    #[test]
    fn release_doctor_blocks_l1_local_overclaim_language() {
        let result = doctor_release_trust_language(&ReleaseTrustClaimInput {
            execution: "observed_l1".to_string(),
            custody: "local_only".to_string(),
            proof_verified: false,
            aggregate_claim: "release is audit-grade and tamper-evident".to_string(),
        });
        assert_eq!(result.status, DoctorStatus::Fail);
        assert!(result.failures[0].contains("must not overclaim L1"));
    }

    #[test]
    fn release_doctor_warns_for_honest_l1_local_claim() {
        let result = doctor_release_trust_language(&ReleaseTrustClaimInput {
            execution: "observed_l1".to_string(),
            custody: "local_only".to_string(),
            proof_verified: false,
            aggregate_claim: "local self-reported trace; integrity hint only".to_string(),
        });
        assert_eq!(result.status, DoctorStatus::Warn);
        assert!(result.warnings[0].contains("integrity hint only"));
    }

    #[test]
    fn release_doctor_blocks_unverified_attestation_claim() {
        let result = doctor_release_trust_language(&ReleaseTrustClaimInput {
            execution: "observed_l1".to_string(),
            custody: "ci_attested".to_string(),
            proof_verified: false,
            aggregate_claim: "CI-attested self-report".to_string(),
        });
        assert_eq!(result.status, DoctorStatus::Fail);
        assert!(result.failures[0].contains("downgrade unverified attestation"));
    }

    #[test]
    fn doctor_trust_wording_golden_snapshots_match() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../tests/fixtures/v19/doctor-output/trust-wording-golden.json"
        ))
        .expect("trust wording fixture must parse");
        let cases = fixture
            .get("cases")
            .and_then(serde_json::Value::as_array)
            .expect("fixture cases array");
        for case in cases {
            let input = case.get("input").expect("case input");
            let result = doctor_release_trust_language(&ReleaseTrustClaimInput {
                execution: input
                    .get("execution")
                    .and_then(serde_json::Value::as_str)
                    .expect("execution")
                    .to_string(),
                custody: input
                    .get("custody")
                    .and_then(serde_json::Value::as_str)
                    .expect("custody")
                    .to_string(),
                proof_verified: input
                    .get("proof_verified")
                    .and_then(serde_json::Value::as_bool)
                    .expect("proof_verified"),
                aggregate_claim: input
                    .get("aggregate_claim")
                    .and_then(serde_json::Value::as_str)
                    .expect("aggregate_claim")
                    .to_string(),
            });
            let expected = case.get("expected").expect("case expected");
            let expected_status = expected
                .get("status")
                .and_then(serde_json::Value::as_str)
                .expect("expected status");
            let actual_status = match result.status {
                DoctorStatus::Pass => "pass",
                DoctorStatus::Warn => "warn",
                DoctorStatus::Fail => "fail",
                DoctorStatus::Fatal => "fatal",
            };
            assert_eq!(actual_status, expected_status, "case={case}");
            let message_contains = expected
                .get("message_contains")
                .and_then(serde_json::Value::as_str)
                .expect("message_contains");
            let combined = result
                .warnings
                .iter()
                .chain(result.failures.iter())
                .cloned()
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                combined.contains(message_contains),
                "case={case} expected message containing {message_contains:?}, got {combined:?}"
            );
        }
    }

    #[test]
    fn release_aggregation_fails_when_required_gate_missing() {
        let report = doctor_release_with_v1_5_gates(vec![DoctorResult::pass("spec_sync")]);
        assert_eq!(report.status(), DoctorStatus::Fail);
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.gate == "assessment_freshness" && check.blocks_release())
        );
    }
}
