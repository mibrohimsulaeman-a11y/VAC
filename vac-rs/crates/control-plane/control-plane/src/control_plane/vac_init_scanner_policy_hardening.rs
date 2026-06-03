#![allow(dead_code)]
//! Scanner and policy inference production-hardening contracts.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SourceClass {
    Source,
    Manifest,
    Test,
    Generated,
    Vendor,
    BuildOutput,
    Documentation,
    Unknown,
}

pub fn classify_workspace_path(path: &str) -> SourceClass {
    let normalized = path.replace('\\', "/");
    if normalized.contains("/target/") || normalized.starts_with("target/") {
        SourceClass::BuildOutput
    } else if normalized.contains("/vendor/")
        || normalized.starts_with("vendor/")
        || normalized.starts_with("vac-rs/vendor/")
    {
        SourceClass::Vendor
    } else if normalized.ends_with(".generated.rs")
        || normalized.contains("/generated/")
        || normalized.contains("/schema/json/")
        || normalized.contains("/schema/typescript/")
    {
        SourceClass::Generated
    } else if normalized.starts_with(".vac/") && normalized.ends_with(".yaml") {
        SourceClass::Manifest
    } else if normalized.contains("/tests/") || normalized.starts_with("tests/") {
        SourceClass::Test
    } else if normalized.ends_with(".md") {
        SourceClass::Documentation
    } else if normalized.ends_with(".rs") || normalized.ends_with(".toml") {
        SourceClass::Source
    } else {
        SourceClass::Unknown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OwnershipFindingStatus {
    Complete,
    Partial,
    Hidden,
    Overclaimed,
    Unowned,
}

impl OwnershipFindingStatus {
    pub const fn blocks_write(self) -> bool {
        matches!(self, Self::Hidden | Self::Overclaimed | Self::Unowned)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RiskAction {
    ExecuteProcess,
    NetworkAccess,
    FilesystemWrite,
    FilesystemDelete,
    CredentialRead,
    MemoryWrite,
    SafeRead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConfidenceLabel {
    Certain,
    High,
    Moderate,
    Low,
    Uncertain,
}

pub fn confidence_label(confidence: f32) -> ConfidenceLabel {
    if confidence >= 0.90 {
        ConfidenceLabel::Certain
    } else if confidence >= 0.70 {
        ConfidenceLabel::High
    } else if confidence >= 0.50 {
        ConfidenceLabel::Moderate
    } else if confidence >= 0.30 {
        ConfidenceLabel::Low
    } else {
        ConfidenceLabel::Uncertain
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum InferredPolicyDecision {
    Allow,
    ApprovalRequired,
    Deny,
    ReviewOnly,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScannerRiskFinding {
    pub file: String,
    pub line: usize,
    pub pattern: String,
    pub action: RiskAction,
    pub confidence: f32,
    pub ambiguous: bool,
}

pub fn infer_policy_decision(finding: &ScannerRiskFinding) -> InferredPolicyDecision {
    if finding.ambiguous || confidence_label(finding.confidence) == ConfidenceLabel::Low {
        return InferredPolicyDecision::ReviewOnly;
    }
    if confidence_label(finding.confidence) == ConfidenceLabel::Uncertain {
        return InferredPolicyDecision::ReviewOnly;
    }
    match finding.action {
        RiskAction::CredentialRead => InferredPolicyDecision::Deny,
        RiskAction::NetworkAccess
        | RiskAction::ExecuteProcess
        | RiskAction::FilesystemDelete
        | RiskAction::FilesystemWrite
        | RiskAction::MemoryWrite => InferredPolicyDecision::ApprovalRequired,
        RiskAction::SafeRead => InferredPolicyDecision::Allow,
    }
}

pub fn detect_rust_risk_pattern(source: &str) -> Option<RiskAction> {
    let compact = source.replace(' ', "");
    if compact.contains("std::process::Command")
        || compact.contains("tokio::process::Command")
        || compact.contains("Command::new(")
    {
        Some(RiskAction::ExecuteProcess)
    } else if compact.contains("std::fs::write")
        || compact.contains("tokio::fs::write")
        || compact.contains("File::create(")
    {
        Some(RiskAction::FilesystemWrite)
    } else if compact.contains("remove_file(") || compact.contains("remove_dir_all(") {
        Some(RiskAction::FilesystemDelete)
    } else if compact.contains("tokio::net")
        || compact.contains("reqwest::")
        || compact.contains("hyper::")
    {
        Some(RiskAction::NetworkAccess)
    } else if compact.contains("env::var(") || compact.contains("dotenv") {
        Some(RiskAction::CredentialRead)
    } else {
        None
    }
}

pub fn should_scan_path(path: &str) -> bool {
    matches!(
        classify_workspace_path(path),
        SourceClass::Source | SourceClass::Test | SourceClass::Manifest
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_classifier_excludes_vendor_generated_and_build_outputs() {
        assert_eq!(
            classify_workspace_path("vac-rs/core/src/lib.rs"),
            SourceClass::Source
        );
        assert_eq!(
            classify_workspace_path("vac-rs/target/debug/lib.rmeta"),
            SourceClass::BuildOutput
        );
        assert_eq!(
            classify_workspace_path("vac-rs/vendor/serde/src/lib.rs"),
            SourceClass::Vendor
        );
        assert_eq!(
            classify_workspace_path("vac-rs/app-server-protocol/schema/json/X.json"),
            SourceClass::Generated
        );
    }

    #[test]
    fn confidence_thresholds_match_spec() {
        assert_eq!(confidence_label(0.95), ConfidenceLabel::Certain);
        assert_eq!(confidence_label(0.75), ConfidenceLabel::High);
        assert_eq!(confidence_label(0.60), ConfidenceLabel::Moderate);
        assert_eq!(confidence_label(0.40), ConfidenceLabel::Low);
        assert_eq!(confidence_label(0.10), ConfidenceLabel::Uncertain);
    }

    #[test]
    fn inference_never_auto_allows_ambiguous_or_low_confidence_risk() {
        let finding = ScannerRiskFinding {
            file: "src/lib.rs".to_string(),
            line: 1,
            pattern: "Command::new".to_string(),
            action: RiskAction::ExecuteProcess,
            confidence: 0.40,
            ambiguous: false,
        };
        assert_eq!(
            infer_policy_decision(&finding),
            InferredPolicyDecision::ReviewOnly
        );

        let ambiguous = ScannerRiskFinding {
            ambiguous: true,
            confidence: 0.95,
            ..finding
        };
        assert_eq!(
            infer_policy_decision(&ambiguous),
            InferredPolicyDecision::ReviewOnly
        );
    }

    #[test]
    fn high_risk_actions_infer_deny_or_approval_required() {
        let credential = ScannerRiskFinding {
            file: "src/lib.rs".to_string(),
            line: 1,
            pattern: "env::var".to_string(),
            action: RiskAction::CredentialRead,
            confidence: 0.95,
            ambiguous: false,
        };
        assert_eq!(
            infer_policy_decision(&credential),
            InferredPolicyDecision::Deny
        );

        let process = ScannerRiskFinding {
            action: RiskAction::ExecuteProcess,
            pattern: "Command::new".to_string(),
            ..credential
        };
        assert_eq!(
            infer_policy_decision(&process),
            InferredPolicyDecision::ApprovalRequired
        );
    }

    #[test]
    fn rust_pattern_detector_covers_process_network_fs_and_credentials() {
        assert_eq!(
            detect_rust_risk_pattern("std::process::Command::new(\"git\")"),
            Some(RiskAction::ExecuteProcess)
        );
        assert_eq!(
            detect_rust_risk_pattern("reqwest::get(url).await"),
            Some(RiskAction::NetworkAccess)
        );
        assert_eq!(
            detect_rust_risk_pattern("std::fs::write(path, data)"),
            Some(RiskAction::FilesystemWrite)
        );
        assert_eq!(
            detect_rust_risk_pattern("std::env::var(\"TOKEN\")"),
            Some(RiskAction::CredentialRead)
        );
    }
}
