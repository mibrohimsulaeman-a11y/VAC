#![allow(dead_code)]
//! VAC-Init risk scanner and initial policy inference contracts.
//!
//! This module is intentionally dependency-free so sandbox validation can run
//! direct `rustc --test` without compiling the full workspace. The runtime YAML
//! loader can map these contracts to serde-backed manifests in later batches.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RiskAction {
    FilesystemRead,
    FilesystemWrite,
    FilesystemDelete,
    ExecuteProcess,
    NetworkAccess,
    CredentialRead,
    MemoryWrite,
    PlanModify,
    CapabilityRegister,
}

impl RiskAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FilesystemRead => "filesystem_read",
            Self::FilesystemWrite => "filesystem_write",
            Self::FilesystemDelete => "filesystem_delete",
            Self::ExecuteProcess => "execute_process",
            Self::NetworkAccess => "network_access",
            Self::CredentialRead => "credential_read",
            Self::MemoryWrite => "memory_write",
            Self::PlanModify => "plan_modify",
            Self::CapabilityRegister => "capability_register",
        }
    }
}

impl fmt::Display for RiskAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RiskDetectionMethod {
    AstExact,
    AstHeuristic,
    ImportChain,
    FilenamePattern,
}

impl RiskDetectionMethod {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AstExact => "ast_exact",
            Self::AstHeuristic => "ast_heuristic",
            Self::ImportChain => "import_chain",
            Self::FilenamePattern => "filename_pattern",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "ast_exact" => Some(Self::AstExact),
            "ast_heuristic" => Some(Self::AstHeuristic),
            "import_chain" => Some(Self::ImportChain),
            "filename_pattern" => Some(Self::FilenamePattern),
            _ => None,
        }
    }

    pub fn validate(value: &str) -> Result<Self, String> {
        Self::from_str(value).ok_or_else(|| {
            format!(
                "invalid risk detection method `{value}`; expected ast_exact | ast_heuristic | import_chain | filename_pattern"
            )
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConfidenceLabel {
    Uncertain,
    Low,
    Moderate,
    High,
    Certain,
}

impl ConfidenceLabel {
    pub fn from_score(score: f32) -> Self {
        if score >= 0.90 {
            Self::Certain
        } else if score >= 0.70 {
            Self::High
        } else if score >= 0.50 {
            Self::Moderate
        } else if score >= 0.30 {
            Self::Low
        } else {
            Self::Uncertain
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Certain => "certain",
            Self::High => "high",
            Self::Moderate => "moderate",
            Self::Low => "low",
            Self::Uncertain => "uncertain",
        }
    }

    pub const fn is_auto_policy_candidate(self) -> bool {
        matches!(self, Self::Certain | Self::High)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RiskAlternative {
    pub risk: RiskAction,
    pub confidence: f32,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RiskFinding {
    pub id: String,
    pub file: String,
    pub line: usize,
    pub pattern: String,
    pub inferred_risk: RiskAction,
    pub confidence: f32,
    pub method: RiskDetectionMethod,
    pub ambiguous: bool,
    pub alternatives: Vec<RiskAlternative>,
}

impl RiskFinding {
    pub fn confidence_label(&self) -> ConfidenceLabel {
        ConfidenceLabel::from_score(self.confidence)
    }

    pub fn is_auto_assignable(&self) -> bool {
        self.confidence_label().is_auto_policy_candidate() && !self.ambiguous
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskPattern {
    pub needle: &'static str,
    pub action: RiskAction,
    pub method: RiskDetectionMethod,
    pub confidence_millis: u16,
}

impl RiskPattern {
    pub const fn confidence(&self) -> f32 {
        self.confidence_millis as f32 / 1000.0
    }
}

pub const RISK_PATTERNS: &[RiskPattern] = &[
    RiskPattern {
        needle: "Command::new",
        action: RiskAction::ExecuteProcess,
        method: RiskDetectionMethod::AstExact,
        confidence_millis: 960,
    },
    RiskPattern {
        needle: "tokio::process",
        action: RiskAction::ExecuteProcess,
        method: RiskDetectionMethod::ImportChain,
        confidence_millis: 910,
    },
    RiskPattern {
        needle: "std::process",
        action: RiskAction::ExecuteProcess,
        method: RiskDetectionMethod::ImportChain,
        confidence_millis: 900,
    },
    RiskPattern {
        needle: "tokio::net",
        action: RiskAction::NetworkAccess,
        method: RiskDetectionMethod::ImportChain,
        confidence_millis: 900,
    },
    RiskPattern {
        needle: "TcpStream",
        action: RiskAction::NetworkAccess,
        method: RiskDetectionMethod::AstHeuristic,
        confidence_millis: 780,
    },
    RiskPattern {
        needle: "reqwest",
        action: RiskAction::NetworkAccess,
        method: RiskDetectionMethod::ImportChain,
        confidence_millis: 880,
    },
    RiskPattern {
        needle: "hyper::",
        action: RiskAction::NetworkAccess,
        method: RiskDetectionMethod::ImportChain,
        confidence_millis: 820,
    },
    RiskPattern {
        needle: "std::fs::write",
        action: RiskAction::FilesystemWrite,
        method: RiskDetectionMethod::AstExact,
        confidence_millis: 940,
    },
    RiskPattern {
        needle: "remove_file",
        action: RiskAction::FilesystemDelete,
        method: RiskDetectionMethod::AstExact,
        confidence_millis: 950,
    },
    RiskPattern {
        needle: "remove_dir_all",
        action: RiskAction::FilesystemDelete,
        method: RiskDetectionMethod::AstExact,
        confidence_millis: 970,
    },
    RiskPattern {
        needle: "env::var",
        action: RiskAction::CredentialRead,
        method: RiskDetectionMethod::AstHeuristic,
        confidence_millis: 740,
    },
    RiskPattern {
        needle: "dotenv",
        action: RiskAction::CredentialRead,
        method: RiskDetectionMethod::ImportChain,
        confidence_millis: 780,
    },
    RiskPattern {
        needle: "secret",
        action: RiskAction::CredentialRead,
        method: RiskDetectionMethod::FilenamePattern,
        confidence_millis: 520,
    },
    RiskPattern {
        needle: "credential",
        action: RiskAction::CredentialRead,
        method: RiskDetectionMethod::FilenamePattern,
        confidence_millis: 620,
    },
    RiskPattern {
        needle: "migration",
        action: RiskAction::FilesystemWrite,
        method: RiskDetectionMethod::FilenamePattern,
        confidence_millis: 560,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InferredPolicyDecision {
    Allow,
    ApprovalRequired,
    Deny,
}

impl InferredPolicyDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::ApprovalRequired => "approval_required",
            Self::Deny => "deny",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferredPolicyRule {
    pub id: String,
    pub action: RiskAction,
    pub decision: InferredPolicyDecision,
    pub reason: String,
    pub source_finding: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PolicyInferenceReport {
    pub rules: Vec<InferredPolicyRule>,
    pub review_required: Vec<String>,
    pub log_only: Vec<String>,
}

impl PolicyInferenceReport {
    pub fn is_operator_review_required(&self) -> bool {
        !self.review_required.is_empty()
    }
}

pub fn detect_risk_findings(file: &str, source: &str) -> Vec<RiskFinding> {
    let mut findings = Vec::new();
    for (idx, line) in source.lines().enumerate() {
        let line_number = idx + 1;
        for pattern in RISK_PATTERNS {
            if line.contains(pattern.needle) || file.contains(pattern.needle) {
                let ambiguous = pattern.confidence_millis < 700;
                let mut alternatives = Vec::new();
                if ambiguous {
                    alternatives.push(RiskAlternative {
                        risk: RiskAction::FilesystemRead,
                        confidence: 0.30,
                        rationale: "pattern may be a filename or documentation mention only"
                            .to_string(),
                    });
                }
                findings.push(RiskFinding {
                    id: format!(
                        "finding.{}.{}.{}",
                        sanitize_id(file),
                        line_number,
                        sanitize_id(pattern.needle)
                    ),
                    file: file.to_string(),
                    line: line_number,
                    pattern: pattern.needle.to_string(),
                    inferred_risk: pattern.action,
                    confidence: pattern.confidence(),
                    method: pattern.method,
                    ambiguous,
                    alternatives,
                });
            }
        }
    }
    if file.contains("migration")
        && findings
            .iter()
            .all(|finding| finding.pattern != "migration")
    {
        findings.push(RiskFinding {
            id: format!("finding.{}.file-pattern", sanitize_id(file)),
            file: file.to_string(),
            line: 1,
            pattern: "migration".to_string(),
            inferred_risk: RiskAction::FilesystemWrite,
            confidence: 0.56,
            method: RiskDetectionMethod::FilenamePattern,
            ambiguous: true,
            alternatives: vec![RiskAlternative {
                risk: RiskAction::FilesystemRead,
                confidence: 0.35,
                rationale: "migration path may be documentation or fixture".to_string(),
            }],
        });
    }
    findings
}

pub fn infer_policy_from_findings(findings: &[RiskFinding]) -> PolicyInferenceReport {
    let mut report = PolicyInferenceReport::default();
    for finding in findings {
        match finding.confidence_label() {
            ConfidenceLabel::Certain | ConfidenceLabel::High if !finding.ambiguous => {
                let decision = match finding.inferred_risk {
                    RiskAction::CredentialRead | RiskAction::FilesystemDelete => {
                        InferredPolicyDecision::Deny
                    }
                    RiskAction::ExecuteProcess
                    | RiskAction::NetworkAccess
                    | RiskAction::FilesystemWrite => InferredPolicyDecision::ApprovalRequired,
                    _ => InferredPolicyDecision::ApprovalRequired,
                };
                report.rules.push(InferredPolicyRule {
                    id: format!("policy.inferred.{}", sanitize_id(&finding.id)),
                    action: finding.inferred_risk,
                    decision,
                    reason: format!(
                        "{} risk inferred from {} with {} confidence",
                        finding.inferred_risk,
                        finding.pattern,
                        finding.confidence_label().as_str()
                    ),
                    source_finding: finding.id.clone(),
                });
            }
            ConfidenceLabel::Moderate | ConfidenceLabel::Low => {
                report.review_required.push(finding.id.clone());
            }
            ConfidenceLabel::Uncertain | ConfidenceLabel::High | ConfidenceLabel::Certain => {
                if finding.ambiguous {
                    report.review_required.push(finding.id.clone());
                } else {
                    report.log_only.push(finding.id.clone());
                }
            }
        }
    }
    report
}

fn sanitize_id(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('.') {
            out.push('.');
        }
    }
    out.trim_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_process_spawn_as_certain_execute_process() {
        let findings =
            detect_risk_findings("vac-rs/core/src/run.rs", "let cmd = Command::new(\"git\");");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].inferred_risk, RiskAction::ExecuteProcess);
        assert_eq!(findings[0].confidence_label(), ConfidenceLabel::Certain);
        assert!(findings[0].is_auto_assignable());
    }

    #[test]
    fn detects_network_import() {
        let findings = detect_risk_findings("net.rs", "use tokio::net::TcpStream;");
        assert!(
            findings
                .iter()
                .any(|finding| finding.inferred_risk == RiskAction::NetworkAccess)
        );
    }

    #[test]
    fn detects_destructive_file_delete() {
        let findings = detect_risk_findings("cleanup.rs", "std::fs::remove_dir_all(path)?;");
        let report = infer_policy_from_findings(&findings);
        assert!(
            report
                .rules
                .iter()
                .any(|rule| rule.decision == InferredPolicyDecision::Deny)
        );
    }

    #[test]
    fn ambiguous_filename_pattern_requires_operator_review() {
        let findings = detect_risk_findings("docs/migrations/README.md", "migration notes");
        assert!(findings.iter().any(|finding| finding.ambiguous));
        let report = infer_policy_from_findings(&findings);
        assert!(report.is_operator_review_required());
        assert!(report.rules.is_empty());
    }

    #[test]
    fn credential_reads_are_not_inferred_as_allow() {
        let findings = detect_risk_findings(
            "settings.rs",
            "let token = std::env::var(\"SECRET_TOKEN\");",
        );
        let report = infer_policy_from_findings(&findings);
        assert!(
            !report
                .rules
                .iter()
                .any(|rule| rule.decision == InferredPolicyDecision::Allow)
        );
    }

    #[test]
    fn no_findings_produces_empty_report() {
        let findings =
            detect_risk_findings("lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }");
        assert!(findings.is_empty());
        assert_eq!(
            infer_policy_from_findings(&findings),
            PolicyInferenceReport::default()
        );
    }

    #[test]
    fn confidence_fixture_matrix_covers_all_labels() {
        let cases = [
            (0.00, ConfidenceLabel::Uncertain),
            (0.30, ConfidenceLabel::Low),
            (0.50, ConfidenceLabel::Moderate),
            (0.70, ConfidenceLabel::High),
            (0.90, ConfidenceLabel::Certain),
        ];
        for (score, expected) in cases {
            assert_eq!(ConfidenceLabel::from_score(score), expected);
        }
    }

    #[test]
    fn risk_pattern_inventory_covers_detection_methods() {
        for method in [
            RiskDetectionMethod::AstExact,
            RiskDetectionMethod::AstHeuristic,
            RiskDetectionMethod::ImportChain,
            RiskDetectionMethod::FilenamePattern,
        ] {
            assert!(
                RISK_PATTERNS.iter().any(|pattern| pattern.method == method),
                "missing detection method fixture: {}",
                method.as_str()
            );
        }
    }
}

#[cfg(test)]
mod risk_detection_method_contract_tests {
    use super::RiskDetectionMethod;

    #[test]
    fn scanner_output_method_validates_spec_enum() {
        assert_eq!(
            RiskDetectionMethod::validate("ast_exact").unwrap(),
            RiskDetectionMethod::AstExact
        );
        assert!(RiskDetectionMethod::validate(concat!("ast_exact", "_static")).is_err());
    }
}
