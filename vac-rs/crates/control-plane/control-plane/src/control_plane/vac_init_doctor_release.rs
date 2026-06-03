#![allow(dead_code)]
//! Doctor aggregate taxonomy and release gate contract for VAC-Init.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DoctorKind {
    Registry,
    Surfaces,
    Policy,
    Ownership,
    Workflow,
    Evidence,
    Build,
    Memory,
    Init,
    Release,
}

impl DoctorKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Registry => "registry",
            Self::Surfaces => "surfaces",
            Self::Policy => "policy",
            Self::Ownership => "ownership",
            Self::Workflow => "workflow",
            Self::Evidence => "evidence",
            Self::Build => "build",
            Self::Memory => "memory",
            Self::Init => "init",
            Self::Release => "release",
        }
    }

    pub const fn is_required_for_release(self) -> bool {
        !matches!(self, Self::Release)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DoctorSeverity {
    Info,
    Warning,
    Error,
    Fatal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DoctorStatus {
    Pass,
    Warning,
    Failed,
    Fatal,
    Skipped,
}

impl DoctorStatus {
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::Pass | Self::Warning | Self::Skipped => 0,
            Self::Failed => 1,
            Self::Fatal => 2,
        }
    }

    pub const fn blocks_release(self) -> bool {
        matches!(self, Self::Failed | Self::Fatal | Self::Skipped)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorFinding {
    pub code: String,
    pub severity: DoctorSeverity,
    pub message: String,
    pub path: Option<String>,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorCheckReport {
    pub kind: DoctorKind,
    pub status: DoctorStatus,
    pub findings: Vec<DoctorFinding>,
    pub exit_code: i32,
}

impl DoctorCheckReport {
    pub fn new(kind: DoctorKind, status: DoctorStatus, findings: Vec<DoctorFinding>) -> Self {
        let mut effective = status;
        if findings.iter().any(|f| f.severity == DoctorSeverity::Fatal) {
            effective = DoctorStatus::Fatal;
        } else if findings.iter().any(|f| f.severity == DoctorSeverity::Error)
            && matches!(effective, DoctorStatus::Pass | DoctorStatus::Warning)
        {
            effective = DoctorStatus::Failed;
        } else if findings
            .iter()
            .any(|f| f.severity == DoctorSeverity::Warning)
            && effective == DoctorStatus::Pass
        {
            effective = DoctorStatus::Warning;
        }
        Self {
            kind,
            status: effective,
            findings,
            exit_code: effective.exit_code(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseGateContext {
    pub hard_quarantine_count: usize,
    pub broken_evidence_chain: bool,
    pub unowned_file_count: usize,
    pub policy_loaded: bool,
    pub registry_loaded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorAggregateReport {
    pub checks: Vec<DoctorCheckReport>,
    pub release_context: ReleaseGateContext,
    pub release_status: DoctorStatus,
    pub exit_code: i32,
    pub blocking_reasons: Vec<String>,
}

pub const REQUIRED_DOCTORS: [DoctorKind; 9] = [
    DoctorKind::Registry,
    DoctorKind::Surfaces,
    DoctorKind::Policy,
    DoctorKind::Ownership,
    DoctorKind::Workflow,
    DoctorKind::Evidence,
    DoctorKind::Build,
    DoctorKind::Memory,
    DoctorKind::Init,
];

pub fn missing_required_doctors(checks: &[DoctorCheckReport]) -> Vec<DoctorKind> {
    REQUIRED_DOCTORS
        .iter()
        .copied()
        .filter(|required| !checks.iter().any(|check| check.kind == *required))
        .collect()
}

pub fn aggregate_doctor_release(
    checks: Vec<DoctorCheckReport>,
    context: ReleaseGateContext,
) -> DoctorAggregateReport {
    let mut blocking_reasons = Vec::new();
    let mut status = DoctorStatus::Pass;

    for missing in missing_required_doctors(&checks) {
        blocking_reasons.push(format!("missing required doctor: {}", missing.as_str()));
        status = DoctorStatus::Failed;
    }

    for check in &checks {
        match check.status {
            DoctorStatus::Fatal => {
                status = DoctorStatus::Fatal;
                blocking_reasons.push(format!("{} doctor fatal", check.kind.as_str()));
            }
            DoctorStatus::Failed | DoctorStatus::Skipped => {
                if status != DoctorStatus::Fatal {
                    status = DoctorStatus::Failed;
                }
                blocking_reasons.push(format!(
                    "{} doctor did not pass: {:?}",
                    check.kind.as_str(),
                    check.status
                ));
            }
            DoctorStatus::Warning => {
                if status == DoctorStatus::Pass {
                    status = DoctorStatus::Warning;
                }
            }
            DoctorStatus::Pass => {}
        }
    }

    if !context.policy_loaded {
        blocking_reasons.push("fail-closed: no policy loaded".to_string());
        status = DoctorStatus::Failed;
    }
    if !context.registry_loaded {
        blocking_reasons.push("fail-closed: registry not loaded".to_string());
        status = DoctorStatus::Failed;
    }
    if context.hard_quarantine_count > 0 {
        blocking_reasons.push(format!(
            "hard quarantine blocks release: {}",
            context.hard_quarantine_count
        ));
        status = DoctorStatus::Failed;
    }
    if context.unowned_file_count > 0 {
        blocking_reasons.push(format!(
            "unowned files block release: {}",
            context.unowned_file_count
        ));
        status = DoctorStatus::Failed;
    }
    if context.broken_evidence_chain {
        blocking_reasons.push("broken evidence chain blocks release".to_string());
        status = DoctorStatus::Failed;
    }

    let exit_code = status.exit_code();
    DoctorAggregateReport {
        checks,
        release_context: context,
        release_status: status,
        exit_code,
        blocking_reasons,
    }
}

pub fn validate_doctor_report(report: &DoctorAggregateReport) -> Result<(), String> {
    if report.release_status.blocks_release() && report.blocking_reasons.is_empty() {
        return Err("blocking release must include at least one reason".to_string());
    }
    if report.exit_code != report.release_status.exit_code() {
        return Err("exit code does not match release status".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pass(kind: DoctorKind) -> DoctorCheckReport {
        DoctorCheckReport::new(kind, DoctorStatus::Pass, Vec::new())
    }

    fn all_passing_checks() -> Vec<DoctorCheckReport> {
        REQUIRED_DOCTORS.iter().copied().map(pass).collect()
    }

    fn clean_context() -> ReleaseGateContext {
        ReleaseGateContext {
            hard_quarantine_count: 0,
            broken_evidence_chain: false,
            unowned_file_count: 0,
            policy_loaded: true,
            registry_loaded: true,
        }
    }

    #[test]
    fn clean_release_passes() {
        let report = aggregate_doctor_release(all_passing_checks(), clean_context());
        assert_eq!(report.release_status, DoctorStatus::Pass);
        assert_eq!(report.exit_code, 0);
        assert!(report.blocking_reasons.is_empty());
        assert!(validate_doctor_report(&report).is_ok());
    }

    #[test]
    fn missing_doctor_blocks_release() {
        let mut checks = all_passing_checks();
        checks.retain(|c| c.kind != DoctorKind::Evidence);
        let report = aggregate_doctor_release(checks, clean_context());
        assert_eq!(report.release_status, DoctorStatus::Failed);
        assert!(
            report
                .blocking_reasons
                .iter()
                .any(|r| r.contains("missing required doctor: evidence"))
        );
    }

    #[test]
    fn fatal_finding_sets_exit_code_two() {
        let mut checks = all_passing_checks();
        checks[0] = DoctorCheckReport::new(
            DoctorKind::Registry,
            DoctorStatus::Pass,
            vec![DoctorFinding {
                code: "registry.fatal".to_string(),
                severity: DoctorSeverity::Fatal,
                message: "registry crashed".to_string(),
                path: None,
                hint: None,
            }],
        );
        let report = aggregate_doctor_release(checks, clean_context());
        assert_eq!(report.release_status, DoctorStatus::Fatal);
        assert_eq!(report.exit_code, 2);
    }

    #[test]
    fn hard_quarantine_blocks_release() {
        let mut ctx = clean_context();
        ctx.hard_quarantine_count = 1;
        let report = aggregate_doctor_release(all_passing_checks(), ctx);
        assert_eq!(report.release_status, DoctorStatus::Failed);
        assert!(
            report
                .blocking_reasons
                .iter()
                .any(|r| r.contains("hard quarantine"))
        );
    }

    #[test]
    fn no_policy_is_fail_closed() {
        let mut ctx = clean_context();
        ctx.policy_loaded = false;
        let report = aggregate_doctor_release(all_passing_checks(), ctx);
        assert_eq!(report.release_status, DoctorStatus::Failed);
        assert!(
            report
                .blocking_reasons
                .iter()
                .any(|r| r.contains("no policy loaded"))
        );
    }

    #[test]
    fn warning_keeps_exit_code_zero() {
        let mut checks = all_passing_checks();
        checks[0] = DoctorCheckReport::new(
            DoctorKind::Registry,
            DoctorStatus::Pass,
            vec![DoctorFinding {
                code: "registry.warn".to_string(),
                severity: DoctorSeverity::Warning,
                message: "minor warning".to_string(),
                path: None,
                hint: None,
            }],
        );
        let report = aggregate_doctor_release(checks, clean_context());
        assert_eq!(report.release_status, DoctorStatus::Warning);
        assert_eq!(report.exit_code, 0);
    }
}
