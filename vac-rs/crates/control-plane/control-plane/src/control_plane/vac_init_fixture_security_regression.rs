#![allow(dead_code)]
//! Fixture matrix and security regression contracts for VAC-Init.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FixtureCategory {
    Schema,
    StateMachine,
    Policy,
    Workspace,
    Evidence,
    Plans,
    Patches,
    Doctor,
    Memory,
    Trajectory,
}

impl FixtureCategory {
    pub const ALL: &'static [Self] = &[
        Self::Schema,
        Self::StateMachine,
        Self::Policy,
        Self::Workspace,
        Self::Evidence,
        Self::Plans,
        Self::Patches,
        Self::Doctor,
        Self::Memory,
        Self::Trajectory,
    ];

    pub const fn dir(self) -> &'static str {
        match self {
            Self::Schema => "tests/fixtures/schema",
            Self::StateMachine => "tests/fixtures/state_machine",
            Self::Policy => "tests/fixtures/policy",
            Self::Workspace => "tests/fixtures/workspace",
            Self::Evidence => "tests/fixtures/evidence",
            Self::Plans => "tests/fixtures/plans",
            Self::Patches => "tests/fixtures/patches",
            Self::Doctor => "tests/fixtures/doctor",
            Self::Memory => "tests/fixtures/memory",
            Self::Trajectory => "tests/fixtures/trajectory",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureMatrixReport {
    pub categories_present: Vec<FixtureCategory>,
    pub has_positive_fixture: bool,
    pub has_negative_fixture: bool,
}

impl FixtureMatrixReport {
    pub fn missing_categories(&self) -> Vec<FixtureCategory> {
        FixtureCategory::ALL
            .iter()
            .copied()
            .filter(|category| !self.categories_present.contains(category))
            .collect()
    }

    pub fn is_complete(&self) -> bool {
        self.missing_categories().is_empty()
            && self.has_positive_fixture
            && self.has_negative_fixture
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SecurityAttack {
    ShellInjection,
    PathTraversal,
    AbsolutePathWrite,
    FreeFormCommand,
    ApprovalReplay,
    ExpiredApproval,
    ChangedPolicyHash,
    BrokenEvidenceChain,
    SecretMemoryContent,
    UnownedFilePatch,
    UndeclaredNewFile,
    PlannedCapabilityExecution,
}

impl SecurityAttack {
    pub const ALL: &'static [Self] = &[
        Self::ShellInjection,
        Self::PathTraversal,
        Self::AbsolutePathWrite,
        Self::FreeFormCommand,
        Self::ApprovalReplay,
        Self::ExpiredApproval,
        Self::ChangedPolicyHash,
        Self::BrokenEvidenceChain,
        Self::SecretMemoryContent,
        Self::UnownedFilePatch,
        Self::UndeclaredNewFile,
        Self::PlannedCapabilityExecution,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityRegressionCase {
    pub attack: SecurityAttack,
    pub rejected: bool,
    pub diagnostic: String,
    pub panicked: bool,
}

pub fn validate_security_regressions(cases: &[SecurityRegressionCase]) -> Result<(), String> {
    for attack in SecurityAttack::ALL {
        let Some(case) = cases.iter().find(|case| &case.attack == attack) else {
            return Err(format!("missing security regression case: {attack:?}"));
        };
        if case.panicked {
            return Err(format!("security regression panicked: {attack:?}"));
        }
        if !case.rejected {
            return Err(format!("security regression was not rejected: {attack:?}"));
        }
        if case.diagnostic.trim().is_empty() {
            return Err(format!("security regression lacks diagnostic: {attack:?}"));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoGatePlan {
    pub full_workspace_required: bool,
    pub selected_package_gates: Vec<String>,
    pub offline_vendor_available: bool,
}

impl CargoGatePlan {
    pub fn production_sandbox_default() -> Self {
        Self {
            full_workspace_required: false,
            selected_package_gates: vec![
                "cargo test --manifest-path vac-rs/Cargo.toml -p vac-core schema_validation --offline".to_string(),
                "cargo test --manifest-path vac-rs/Cargo.toml -p vac-core policy_evaluation --offline".to_string(),
                "cargo test --manifest-path vac-rs/Cargo.toml -p vac-core evidence_chain --offline".to_string(),
                "cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-cli doctor_integration --offline".to_string(),
            ],
            offline_vendor_available: false,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.full_workspace_required {
            return Err("full workspace build must not be a required sandbox gate".to_string());
        }
        if self.selected_package_gates.is_empty() {
            return Err("selected package cargo gates must be registered".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_matrix_requires_all_spec_categories() {
        let complete = FixtureMatrixReport {
            categories_present: FixtureCategory::ALL.to_vec(),
            has_positive_fixture: true,
            has_negative_fixture: true,
        };
        assert!(complete.is_complete());

        let incomplete = FixtureMatrixReport {
            categories_present: vec![FixtureCategory::Schema],
            has_positive_fixture: true,
            has_negative_fixture: false,
        };
        assert!(!incomplete.is_complete());
        assert!(
            incomplete
                .missing_categories()
                .contains(&FixtureCategory::Policy)
        );
    }

    #[test]
    fn all_security_attacks_must_be_rejected_with_diagnostics() {
        let cases = SecurityAttack::ALL
            .iter()
            .map(|attack| SecurityRegressionCase {
                attack: *attack,
                rejected: true,
                diagnostic: format!("{attack:?} rejected"),
                panicked: false,
            })
            .collect::<Vec<_>>();
        assert!(validate_security_regressions(&cases).is_ok());

        let mut unsafe_cases = cases.clone();
        unsafe_cases[0].rejected = false;
        assert!(validate_security_regressions(&unsafe_cases).is_err());
    }

    #[test]
    fn cargo_gate_plan_avoids_full_workspace_required_gate() {
        let plan = CargoGatePlan::production_sandbox_default();
        assert!(plan.validate().is_ok());
        assert!(!plan.full_workspace_required);
        assert!(!plan.selected_package_gates.is_empty());
    }
}
