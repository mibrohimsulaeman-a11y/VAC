//! VAC assessment engine contracts.
//!
//! Findings are valid only when grounded in deterministic index span evidence.
//! The engine supports both intent->code and code->intent checks and degrades
//! ungrounded high-severity findings to blocking invalid findings.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GapFinding {
    pub finding_id: String,
    pub category: String,
    pub severity: String,
    pub recommendation: String,
    pub evidence: AssessmentEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssessmentEvidence {
    pub span_id: Option<String>,
    #[serde(default)]
    pub matched_spans: Vec<String>,
    #[serde(default)]
    pub read_plan_ticket: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntentCodeMapping {
    pub capability: String,
    pub intent_spec: String,
    pub code_paths: Vec<String>,
    pub evidence_spans: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssessmentReport {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub mappings: Vec<IntentCodeMapping>,
    pub findings: Vec<GapFinding>,
}

impl GapFinding {
    #[must_use]
    pub fn is_span_grounded(&self) -> bool {
        self.evidence
            .span_id
            .as_ref()
            .is_some_and(|item| !item.is_empty())
            || !self.evidence.matched_spans.is_empty()
    }

    #[must_use]
    pub fn is_blocking_if_ungrounded(&self) -> bool {
        matches!(self.severity.as_str(), "P0" | "P1" | "critical" | "high")
            && !self.is_span_grounded()
    }
}

impl AssessmentReport {
    #[must_use]
    pub fn validate_span_grounding(&self) -> Vec<String> {
        self.findings
            .iter()
            .filter(|finding| finding.is_blocking_if_ungrounded())
            .map(|finding| finding.finding_id.clone())
            .collect()
    }

    #[must_use]
    pub fn changed_file_capabilities(&self, changed_path: &str) -> Vec<String> {
        self.mappings
            .iter()
            .filter(|mapping| mapping.code_paths.iter().any(|path| path == changed_path))
            .map(|mapping| mapping.capability.clone())
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntentSpecRecord {
    pub id: String,
    pub capability: String,
    pub expected_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodeInventoryRecord {
    pub path: String,
    pub capability: Option<String>,
    pub span_id: String,
}

#[must_use]
pub fn bidirectional_gap_analysis(
    specs: &[IntentSpecRecord],
    code: &[CodeInventoryRecord],
) -> Vec<GapFinding> {
    let mut findings = Vec::new();
    for spec in specs {
        let has_code = spec
            .expected_paths
            .iter()
            .any(|expected| code.iter().any(|item| item.path == *expected));
        if !has_code {
            findings.push(GapFinding {
                finding_id: format!("intent_without_code:{}", spec.id),
                category: "intent_without_code".to_string(),
                severity: "P1".to_string(),
                recommendation: format!(
                    "Implement or explicitly defer capability {}",
                    spec.capability
                ),
                evidence: AssessmentEvidence {
                    span_id: None,
                    matched_spans: Vec::new(),
                    read_plan_ticket: None,
                },
            });
        }
    }
    for item in code {
        if item.capability.is_none() {
            findings.push(GapFinding {
                finding_id: format!("code_without_intent:{}", item.path),
                category: "code_without_intent".to_string(),
                severity: "P1".to_string(),
                recommendation: "Register capability/intent spec or quarantine the path"
                    .to_string(),
                evidence: AssessmentEvidence {
                    span_id: Some(item.span_id.clone()),
                    matched_spans: vec![item.span_id.clone()],
                    read_plan_ticket: None,
                },
            });
        }
    }
    findings
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MissingCodeProofResult {
    Absent,
    NotFoundInIndex,
    CoverageInsufficient,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MissingCodeCoverage {
    pub required_coverage_bps: u16,
    pub actual_coverage_bps: u16,
    #[serde(default)]
    pub low_confidence_residues: Vec<String>,
    #[serde(default)]
    pub residues_ruled_irrelevant: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MissingCodeProof {
    pub coverage_set_id: String,
    pub query_spec_hash: String,
    pub searched_roots: Vec<String>,
    pub excluded_roots: Vec<String>,
    pub required_record_types: Vec<String>,
    pub coverage: MissingCodeCoverage,
}

impl MissingCodeCoverage {
    #[must_use]
    pub fn all_low_confidence_residues_ruled_irrelevant(&self) -> bool {
        self.low_confidence_residues.iter().all(|residue| {
            self.residues_ruled_irrelevant
                .iter()
                .any(|ruled| ruled == residue)
        })
    }
}

#[must_use]
pub fn evaluate_missing_code_proof(proof: &MissingCodeProof) -> MissingCodeProofResult {
    if proof.coverage.actual_coverage_bps < proof.coverage.required_coverage_bps
        || proof.coverage.required_coverage_bps == 0
        || proof.required_record_types.is_empty()
        || proof.searched_roots.is_empty()
    {
        return MissingCodeProofResult::CoverageInsufficient;
    }
    if !proof
        .coverage
        .all_low_confidence_residues_ruled_irrelevant()
    {
        return MissingCodeProofResult::NotFoundInIndex;
    }
    MissingCodeProofResult::Absent
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssessmentBaselineSummary {
    pub files_indexed: u64,
    pub symbols_indexed: u64,
    pub relations_indexed: u64,
    pub risk_findings: u64,
    pub spans_indexed: u64,
    pub read_plans_indexed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexedSpanEvidence {
    pub span_id: String,
    pub span_sha256: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssessmentFreshnessInput {
    pub baseline: AssessmentBaselineSummary,
    pub current: AssessmentBaselineSummary,
    pub report_spans: Vec<IndexedSpanEvidence>,
    pub current_spans: Vec<IndexedSpanEvidence>,
}

#[must_use]
pub fn validate_assessment_freshness(input: &AssessmentFreshnessInput) -> Vec<String> {
    let mut failures = Vec::new();
    if input.baseline != input.current {
        failures.push(
            "assessment baseline summary does not match current deterministic index".to_string(),
        );
    }
    for reported in &input.report_spans {
        match input
            .current_spans
            .iter()
            .find(|span| span.span_id == reported.span_id)
        {
            Some(current) if current.span_sha256 == reported.span_sha256 => {}
            Some(_) => failures.push(format!("stale span_sha256 for {}", reported.span_id)),
            None => failures.push(format!("missing current span_id {}", reported.span_id)),
        }
    }
    failures
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SeverityRubric {
    pub severity: String,
    pub deterministic_fact_required: bool,
    pub span_grounding_required: bool,
    pub blocks_release: bool,
}

#[must_use]
pub fn severity_rubric(category: &str, severity: &str) -> SeverityRubric {
    let blocks_release = matches!(severity, "P0" | "critical")
        || category.contains("stale")
        || category.contains("unowned");
    SeverityRubric {
        severity: severity.to_string(),
        deterministic_fact_required: true,
        span_grounding_required: matches!(severity, "P0" | "P1" | "critical" | "high"),
        blocks_release,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssessmentPipelineStage {
    pub id: String,
    pub input_authority: String,
    pub output_artifact: String,
    pub fail_closed_on_missing_evidence: bool,
}

#[must_use]
pub fn assessment_engine_pipeline() -> Vec<AssessmentPipelineStage> {
    vec![
        AssessmentPipelineStage {
            id: "intent_encoder".to_string(),
            input_authority: ".vac/specs + .vac/capabilities".to_string(),
            output_artifact: "intent vocabulary".to_string(),
            fail_closed_on_missing_evidence: true,
        },
        AssessmentPipelineStage {
            id: "deterministic_index_join".to_string(),
            input_authority: ".vac/index/*.jsonl".to_string(),
            output_artifact: "current span-grounded facts".to_string(),
            fail_closed_on_missing_evidence: true,
        },
        AssessmentPipelineStage {
            id: "bidirectional_gap_join".to_string(),
            input_authority: "intent vocabulary + code spans".to_string(),
            output_artifact: "intent_without_code + code_without_intent + baseline_violation"
                .to_string(),
            fail_closed_on_missing_evidence: true,
        },
        AssessmentPipelineStage {
            id: "severity_by_rule".to_string(),
            input_authority: "deterministic rubric".to_string(),
            output_artifact: ".vac/assessment/gap_report.json".to_string(),
            fail_closed_on_missing_evidence: true,
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssessmentAuthoritySummary {
    pub parser_mode: String,
    pub heuristic_severity: bool,
    pub p1_count: u64,
    pub blocking_count: u64,
    pub warnings: Vec<String>,
}

/// Validate a JSON gap report through the Rust assessment crate so `vac assess`
/// and `vac doctor assessment` no longer treat the Python SV script as product
/// authority.  The Python generator may still produce fixture data; this crate
/// decides whether the artifact is honest enough for runtime closeout.
#[must_use]
pub fn validate_gap_report_value(value: &serde_json::Value) -> AssessmentAuthoritySummary {
    let parser_mode = value
        .pointer("/index/parser_mode")
        .or_else(|| value.pointer("/index/rust_ast_mode"))
        .or_else(|| value.pointer("/metadata/parser_mode"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("static_heuristic_fail_closed")
        .to_string();
    let findings = value
        .get("findings")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let p1_count = findings
        .iter()
        .filter(|finding| {
            finding
                .get("severity")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|sev| sev == "P1" || sev == "high")
        })
        .count() as u64;
    let blocking_count = findings
        .iter()
        .filter(|finding| {
            finding
                .get("blocking")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
        })
        .count() as u64;
    let heuristic_severity = parser_mode.contains("heuristic")
        || parser_mode.contains("sv")
        || parser_mode.contains("regex");
    let mut warnings = Vec::new();
    if heuristic_severity && p1_count > 0 {
        warnings
            .push("P1 findings are heuristic triage, not AST-grounded release truth".to_string());
    }
    if p1_count > 0 && blocking_count == 0 {
        warnings.push("P1 findings require explicit waiver/debt before release; blocking_count=0 is not enough for product-ready claim".to_string());
    }
    AssessmentAuthoritySummary {
        parser_mode,
        heuristic_severity,
        p1_count,
        blocking_count,
        warnings,
    }
}

#[must_use]
pub fn p1_blocks_without_waiver(summary: &AssessmentAuthoritySummary, waiver_count: u64) -> bool {
    summary.p1_count > waiver_count && summary.heuristic_severity
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proof(coverage: MissingCodeCoverage) -> MissingCodeProof {
        MissingCodeProof {
            coverage_set_id: "coverage.fixture".to_string(),
            query_spec_hash: "sha256:query".to_string(),
            searched_roots: vec!["vac-rs/crates/control-plane".to_string()],
            excluded_roots: vec!["target".to_string()],
            required_record_types: vec![
                "symbol".to_string(),
                "route".to_string(),
                "call".to_string(),
            ],
            coverage,
        }
    }

    #[test]
    fn missing_code_full_coverage_without_residue_claims_absent() {
        let result = evaluate_missing_code_proof(&proof(MissingCodeCoverage {
            required_coverage_bps: 10_000,
            actual_coverage_bps: 10_000,
            low_confidence_residues: Vec::new(),
            residues_ruled_irrelevant: Vec::new(),
        }));
        assert_eq!(result, MissingCodeProofResult::Absent);
    }

    #[test]
    fn missing_code_low_confidence_residue_downgrades_absence_claim() {
        let result = evaluate_missing_code_proof(&proof(MissingCodeCoverage {
            required_coverage_bps: 10_000,
            actual_coverage_bps: 10_000,
            low_confidence_residues: vec!["src/generated.rs:unknown macro expansion".to_string()],
            residues_ruled_irrelevant: Vec::new(),
        }));
        assert_eq!(result, MissingCodeProofResult::NotFoundInIndex);
    }

    #[test]
    fn missing_code_insufficient_coverage_makes_no_absence_claim() {
        let result = evaluate_missing_code_proof(&proof(MissingCodeCoverage {
            required_coverage_bps: 10_000,
            actual_coverage_bps: 8_500,
            low_confidence_residues: Vec::new(),
            residues_ruled_irrelevant: Vec::new(),
        }));
        assert_eq!(result, MissingCodeProofResult::CoverageInsufficient);
    }
}
