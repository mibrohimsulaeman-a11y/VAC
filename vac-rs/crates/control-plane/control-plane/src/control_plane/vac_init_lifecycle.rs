#![allow(dead_code)]
//! VAC-Init lifecycle state machine contract.
//!
//! The state machine is dependency-free and append-only friendly. It models the
//! bootstrap progression required by the refined VAC-Init control-plane spec
//! and supports dry-run/resume semantics before the interactive CLI is wired.

use std::fmt;

pub const INIT_STATE_KIND: &str = "init_state";
pub const INIT_STATE_ID: &str = "init.state";
pub const INIT_STATE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VacInitLifecycleState {
    Uninitialized,
    Discovered,
    PartitionSelected,
    PolicyInferred,
    ManifestsSynthesized,
    DoctorVerified,
    Ready,
    ScanFailed,
    OperatorCancelled,
    PolicyConflict,
    OwnershipMissing,
    DoctorFailed,
}

impl VacInitLifecycleState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Uninitialized => "uninitialized",
            Self::Discovered => "discovered",
            Self::PartitionSelected => "partition_selected",
            Self::PolicyInferred => "policy_inferred",
            Self::ManifestsSynthesized => "manifests_synthesized",
            Self::DoctorVerified => "doctor_verified",
            Self::Ready => "ready",
            Self::ScanFailed => "scan_failed",
            Self::OperatorCancelled => "operator_cancelled",
            Self::PolicyConflict => "policy_conflict",
            Self::OwnershipMissing => "ownership_missing",
            Self::DoctorFailed => "doctor_failed",
        }
    }

    pub const fn is_failure(self) -> bool {
        matches!(
            self,
            Self::ScanFailed
                | Self::OperatorCancelled
                | Self::PolicyConflict
                | Self::OwnershipMissing
                | Self::DoctorFailed
        )
    }

    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Ready
                | Self::ScanFailed
                | Self::OperatorCancelled
                | Self::PolicyConflict
                | Self::OwnershipMissing
                | Self::DoctorFailed
        )
    }

    pub const fn output_artifact(self) -> Option<&'static str> {
        match self {
            Self::Uninitialized => None,
            Self::Discovered => Some(".vac/.init/scan_report.yaml"),
            Self::PartitionSelected => Some(".vac/.init/strategy.yaml"),
            Self::PolicyInferred => Some(".vac/.init/risk_findings.yaml"),
            Self::ManifestsSynthesized => Some(".vac/capabilities/*.yaml + .vac/policies/*.yaml"),
            Self::DoctorVerified => Some(".vac/.init/doctor_report.yaml"),
            Self::Ready => Some(".vac/registry/status.yaml"),
            Self::ScanFailed
            | Self::OperatorCancelled
            | Self::PolicyConflict
            | Self::OwnershipMissing
            | Self::DoctorFailed => Some(".vac/.init/state.yaml"),
        }
    }

    pub const fn evidence_event(self) -> Option<&'static str> {
        match self {
            Self::Uninitialized => Some("init.started"),
            Self::Discovered => Some("scan.complete"),
            Self::PartitionSelected => Some("strategy.selected"),
            Self::PolicyInferred => Some("ast.complete"),
            Self::ManifestsSynthesized => Some("synthesis.complete"),
            Self::DoctorVerified => Some("doctor.passed"),
            Self::Ready => Some("init.complete"),
            Self::ScanFailed => Some("scan.failed"),
            Self::OperatorCancelled => Some("operator.cancelled"),
            Self::PolicyConflict => Some("policy.conflict"),
            Self::OwnershipMissing => Some("ownership.missing"),
            Self::DoctorFailed => Some("doctor.failed"),
        }
    }
}

impl fmt::Display for VacInitLifecycleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for VacInitLifecycleState {
    type Err = InitLifecycleError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "uninitialized" => Ok(Self::Uninitialized),
            "discovered" => Ok(Self::Discovered),
            "partition_selected" => Ok(Self::PartitionSelected),
            "policy_inferred" => Ok(Self::PolicyInferred),
            "manifests_synthesized" => Ok(Self::ManifestsSynthesized),
            "doctor_verified" => Ok(Self::DoctorVerified),
            "ready" => Ok(Self::Ready),
            "scan_failed" => Ok(Self::ScanFailed),
            "operator_cancelled" => Ok(Self::OperatorCancelled),
            "policy_conflict" => Ok(Self::PolicyConflict),
            "ownership_missing" => Ok(Self::OwnershipMissing),
            "doctor_failed" => Ok(Self::DoctorFailed),
            _ => Err(InitLifecycleError::UnknownState(value.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitLifecycleError {
    UnknownState(String),
    InvalidTransition {
        from: VacInitLifecycleState,
        to: VacInitLifecycleState,
    },
    MissingRequiredArtifact(&'static str),
    InvalidEnvelope(String),
}

impl fmt::Display for InitLifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownState(state) => write!(f, "unknown init lifecycle state `{state}`"),
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid vac init transition `{from}` -> `{to}`")
            }
            Self::MissingRequiredArtifact(field) => {
                write!(f, "missing required artifact field `{field}`")
            }
            Self::InvalidEnvelope(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for InitLifecycleError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VacInitStateRecord {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub current_state: VacInitLifecycleState,
    pub previous_state: Option<VacInitLifecycleState>,
    pub timestamp: String,
    pub retry_count: u32,
    pub scan_report: Option<String>,
    pub strategy: Option<String>,
    pub risk_findings: Option<String>,
    pub doctor_report: Option<String>,
    pub error: Option<String>,
}

impl VacInitStateRecord {
    pub fn new(timestamp: impl Into<String>) -> Self {
        Self {
            schema_version: INIT_STATE_SCHEMA_VERSION,
            kind: INIT_STATE_KIND.to_string(),
            id: INIT_STATE_ID.to_string(),
            current_state: VacInitLifecycleState::Uninitialized,
            previous_state: None,
            timestamp: timestamp.into(),
            retry_count: 0,
            scan_report: None,
            strategy: None,
            risk_findings: None,
            doctor_report: None,
            error: None,
        }
    }

    pub fn should_resume(&self) -> bool {
        self.current_state != VacInitLifecycleState::Ready
    }

    pub fn validate(&self) -> Result<(), InitLifecycleError> {
        if self.schema_version != INIT_STATE_SCHEMA_VERSION {
            return Err(InitLifecycleError::InvalidEnvelope(format!(
                "expected schema_version {}, found {}",
                INIT_STATE_SCHEMA_VERSION, self.schema_version
            )));
        }
        if self.kind != INIT_STATE_KIND {
            return Err(InitLifecycleError::InvalidEnvelope(format!(
                "expected kind `{}`, found `{}`",
                INIT_STATE_KIND, self.kind
            )));
        }
        if self.id != INIT_STATE_ID {
            return Err(InitLifecycleError::InvalidEnvelope(format!(
                "expected id `{}`, found `{}`",
                INIT_STATE_ID, self.id
            )));
        }
        match self.current_state {
            VacInitLifecycleState::Discovered
            | VacInitLifecycleState::PartitionSelected
            | VacInitLifecycleState::PolicyInferred
            | VacInitLifecycleState::ManifestsSynthesized
            | VacInitLifecycleState::DoctorVerified
            | VacInitLifecycleState::Ready => {
                if self
                    .scan_report
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
                {
                    return Err(InitLifecycleError::MissingRequiredArtifact("scan_report"));
                }
            }
            _ => {}
        }
        if matches!(
            self.current_state,
            VacInitLifecycleState::PartitionSelected
                | VacInitLifecycleState::PolicyInferred
                | VacInitLifecycleState::ManifestsSynthesized
                | VacInitLifecycleState::DoctorVerified
                | VacInitLifecycleState::Ready
        ) && self
            .strategy
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            return Err(InitLifecycleError::MissingRequiredArtifact("strategy"));
        }
        if matches!(
            self.current_state,
            VacInitLifecycleState::PolicyInferred
                | VacInitLifecycleState::ManifestsSynthesized
                | VacInitLifecycleState::DoctorVerified
                | VacInitLifecycleState::Ready
        ) && self
            .risk_findings
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            return Err(InitLifecycleError::MissingRequiredArtifact("risk_findings"));
        }
        if matches!(self.current_state, VacInitLifecycleState::Ready)
            && self
                .doctor_report
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(InitLifecycleError::MissingRequiredArtifact("doctor_report"));
        }
        Ok(())
    }

    pub fn render_yaml(&self) -> String {
        let mut lines = vec![
            format!("schema_version: {}", self.schema_version),
            format!("kind: {}", self.kind),
            format!("id: {}", self.id),
            format!("current_state: {}", self.current_state),
            format!(
                "previous_state: {}",
                self.previous_state
                    .map(|state| state.as_str())
                    .unwrap_or("null")
            ),
            format!("timestamp: {}", self.timestamp),
            format!("retry_count: {}", self.retry_count),
            format!(
                "scan_report: {}",
                self.scan_report.as_deref().unwrap_or("null")
            ),
            format!("strategy: {}", self.strategy.as_deref().unwrap_or("null")),
            format!(
                "risk_findings: {}",
                self.risk_findings.as_deref().unwrap_or("null")
            ),
            format!(
                "doctor_report: {}",
                self.doctor_report.as_deref().unwrap_or("null")
            ),
            format!("error: {}", self.error.as_deref().unwrap_or("null")),
        ];
        lines.push(String::new());
        lines.join("\n")
    }
}

pub fn can_transition(from: VacInitLifecycleState, to: VacInitLifecycleState) -> bool {
    use VacInitLifecycleState::*;
    match (from, to) {
        (Uninitialized, Discovered | ScanFailed | OperatorCancelled) => true,
        (Discovered, PartitionSelected | ScanFailed | OperatorCancelled) => true,
        (PartitionSelected, PolicyInferred | PolicyConflict | OperatorCancelled) => true,
        (
            PolicyInferred,
            ManifestsSynthesized | PolicyConflict | OwnershipMissing | OperatorCancelled,
        ) => true,
        (
            ManifestsSynthesized,
            DoctorVerified | DoctorFailed | OwnershipMissing | OperatorCancelled,
        ) => true,
        (DoctorVerified, Ready | DoctorFailed | OperatorCancelled) => true,
        (ScanFailed, Discovered | OperatorCancelled) => true,
        (PolicyConflict, PartitionSelected | PolicyInferred | OperatorCancelled) => true,
        (OwnershipMissing, Discovered | PartitionSelected | OperatorCancelled) => true,
        (DoctorFailed, ManifestsSynthesized | DoctorVerified | OperatorCancelled) => true,
        (Ready, _) => false,
        (OperatorCancelled, _) => false,
        _ => false,
    }
}

pub fn advance_init_state(
    record: &VacInitStateRecord,
    next: VacInitLifecycleState,
    timestamp: impl Into<String>,
) -> Result<VacInitStateRecord, InitLifecycleError> {
    if !can_transition(record.current_state, next) {
        return Err(InitLifecycleError::InvalidTransition {
            from: record.current_state,
            to: next,
        });
    }
    let mut updated = record.clone();
    updated.previous_state = Some(record.current_state);
    updated.current_state = next;
    updated.timestamp = timestamp.into();
    if next.is_failure() {
        updated.retry_count = updated.retry_count.saturating_add(1);
    }
    Ok(updated)
}

pub fn allowed_commands_for_state(state: VacInitLifecycleState) -> &'static [&'static str] {
    match state {
        VacInitLifecycleState::Uninitialized => &["vac init"],
        VacInitLifecycleState::Discovered => &["vac init --rescan"],
        VacInitLifecycleState::PartitionSelected => &["vac init --restrategy"],
        VacInitLifecycleState::PolicyInferred => &["vac init --rescan-ast"],
        VacInitLifecycleState::ManifestsSynthesized => &["vac doctor registry ."],
        VacInitLifecycleState::DoctorVerified => &["vac init --finalize"],
        VacInitLifecycleState::Ready => &["all vac commands"],
        VacInitLifecycleState::ScanFailed => &["vac init --rescan"],
        VacInitLifecycleState::OperatorCancelled => &["vac init --resume"],
        VacInitLifecycleState::PolicyConflict => {
            &["vac init --restrategy", "vac init --rescan-ast"]
        }
        VacInitLifecycleState::OwnershipMissing => &["vac doctor ownership .", "vac init --rescan"],
        VacInitLifecycleState::DoctorFailed => &["vac doctor registry .", "vac init --resume"],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_transitions_are_allowed() {
        let states = [
            VacInitLifecycleState::Uninitialized,
            VacInitLifecycleState::Discovered,
            VacInitLifecycleState::PartitionSelected,
            VacInitLifecycleState::PolicyInferred,
            VacInitLifecycleState::ManifestsSynthesized,
            VacInitLifecycleState::DoctorVerified,
            VacInitLifecycleState::Ready,
        ];
        for pair in states.windows(2) {
            assert!(
                can_transition(pair[0], pair[1]),
                "{} -> {}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn ready_is_terminal() {
        assert!(!can_transition(
            VacInitLifecycleState::Ready,
            VacInitLifecycleState::Discovered
        ));
        assert!(VacInitLifecycleState::Ready.is_terminal());
    }

    #[test]
    fn invalid_transition_is_rejected() {
        let record = VacInitStateRecord::new("2026-05-29T00:00:00Z");
        let error = advance_init_state(
            &record,
            VacInitLifecycleState::Ready,
            "2026-05-29T00:01:00Z",
        )
        .unwrap_err();
        assert!(error.to_string().contains("invalid"));
    }

    #[test]
    fn advance_records_previous_state() {
        let record = VacInitStateRecord::new("2026-05-29T00:00:00Z");
        let next = advance_init_state(
            &record,
            VacInitLifecycleState::Discovered,
            "2026-05-29T00:01:00Z",
        )
        .unwrap();
        assert_eq!(
            next.previous_state,
            Some(VacInitLifecycleState::Uninitialized)
        );
        assert_eq!(next.current_state, VacInitLifecycleState::Discovered);
    }

    #[test]
    fn failure_transition_increments_retry_count() {
        let record = VacInitStateRecord::new("2026-05-29T00:00:00Z");
        let next = advance_init_state(
            &record,
            VacInitLifecycleState::ScanFailed,
            "2026-05-29T00:01:00Z",
        )
        .unwrap();
        assert_eq!(next.retry_count, 1);
        assert!(next.current_state.is_failure());
    }

    #[test]
    fn discovered_state_requires_scan_report() {
        let mut record = VacInitStateRecord::new("2026-05-29T00:00:00Z");
        record.current_state = VacInitLifecycleState::Discovered;
        let error = record.validate().unwrap_err();
        assert!(error.to_string().contains("scan_report"));
        record.scan_report = Some(".vac/.init/scan_report.yaml".to_string());
        assert!(record.validate().is_ok());
    }

    #[test]
    fn ready_state_requires_all_artifacts() {
        let mut record = VacInitStateRecord::new("2026-05-29T00:00:00Z");
        record.current_state = VacInitLifecycleState::Ready;
        record.scan_report = Some(".vac/.init/scan_report.yaml".to_string());
        record.strategy = Some(".vac/.init/strategy.yaml".to_string());
        record.risk_findings = Some(".vac/.init/risk_findings.yaml".to_string());
        assert!(record.validate().is_err());
        record.doctor_report = Some(".vac/.init/doctor_report.yaml".to_string());
        assert!(record.validate().is_ok());
    }

    #[test]
    fn render_yaml_uses_required_envelope() {
        let record = VacInitStateRecord::new("2026-05-29T00:00:00Z");
        let yaml = record.render_yaml();
        assert!(yaml.contains("schema_version: 1"));
        assert!(yaml.contains("kind: init_state"));
        assert!(yaml.contains("id: init.state"));
    }

    #[test]
    fn state_from_str_rejects_unknown_value() {
        let error = "mystery".parse::<VacInitLifecycleState>().unwrap_err();
        assert!(error.to_string().contains("unknown"));
    }

    #[test]
    fn allowed_commands_match_spec_states() {
        assert_eq!(
            allowed_commands_for_state(VacInitLifecycleState::ManifestsSynthesized),
            &["vac doctor registry ."]
        );
        assert!(
            allowed_commands_for_state(VacInitLifecycleState::Ready).contains(&"all vac commands")
        );
    }
}
