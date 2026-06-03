use super::build_check;

use super::approval_lifecycle::ApprovalAction;
use super::approval_lifecycle::ApprovalDecision;
use super::approval_lifecycle::ApprovalRequest;
use super::approval_lifecycle::ApprovalRequestId;
use super::approval_lifecycle::ApprovalRisk;
use super::approval_lifecycle::ApprovalStatus;
use super::approval_lifecycle::ApprovalStore;
use super::approval_lifecycle::InMemoryApprovalStore;
use super::approval_store::FileApprovalStore;
use super::architecture_invariants::ArchitectureInvariantReport;
use super::capability_manifest::CapabilityPolicy;
use super::capability_manifest::CapabilityPolicyValue;
use super::capability_manifest::CapabilityStatus;
use super::donor_status::DonorStatusReport;
use super::identity_check::IdentityCheckReport;
use super::identity_check::load_identity_check_report;
use super::no_duplicate_tui::NoDuplicateTuiReport;
use super::no_duplicate_tui::load_no_duplicate_tui_report;
use super::ownership_scan::OwnershipScanReport;
use super::ownership_scan::load_ownership_scan_report;
use super::policy_manifest::PolicyAction;
use super::policy_manifest::PolicyDataClass;
use super::policy_manifest::PolicyPathScope;
use super::registry::ControlPlaneRegistry;
use super::registry_diagnostics::RegistryLoadReport;
use super::workflow_manifest::WorkflowManifest;
use super::workflow_manifest::WorkflowPolicyValue;
use chrono::DateTime;
use chrono::Duration;
use chrono::Utc;
use std::fmt;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const BUILD_CHECK_REPO_ROOT_ENV: &str = "VAC_BUILD_CHECK_REPO_ROOT";
pub const SKIP_BUILD_CHECK_ENV: &str = "VAC_SKIP_BUILD_CHECK";

pub const INITIAL_ALLOWED_STEP_USES: &[&str] = &[
    "capability.root_seed.coverage",
    "capability.build.cargo_check",
    "capability.identity.check",
    "capability.tui.no_duplicate_runtime",
    "capability.ownership.scan",
    "capability.architecture.invariants",
    "capability.activity.emit",
    "capability.approval.request",
    "capability.tui.pty_gate",
    "capability.donor_migration.inventory_check",
    "capability.donor_migration.drift_check",
    "capability.donor_migration.manifest_check",
    "capability.donor_migration.reachability_check",
    "capability.donor_migration.evidence_check",
    "capability.donor_migration.commit_phrase_check",
    "capability.registry.schema_check",
    "capability.dashboard.check",
    "capability.browser.check",
];

/// Single source of truth for the built-in safe workflow step vocabulary.
///
/// `capability.*` step ids are an internal namespace used by seeded workflows
/// and are aliased to canonical `vac.*` capability ids until the workflow step
/// namespace and capability id namespace converge.
#[derive(Debug, Clone, Copy)]
pub struct WorkflowStepVocabularyEntry {
    pub uses: &'static str,
    pub handler: WorkflowStepHandler,
    pub canonical_capability_id: &'static str,
}

pub const WORKFLOW_STEP_VOCABULARY: &[WorkflowStepVocabularyEntry] = &[
    WorkflowStepVocabularyEntry {
        uses: "capability.root_seed.coverage",
        handler: WorkflowStepHandler::RootSeedCoverage,
        canonical_capability_id: "vac.workflow",
    },
    WorkflowStepVocabularyEntry {
        uses: "capability.build.cargo_check",
        handler: WorkflowStepHandler::BuildCargoCheck,
        canonical_capability_id: "vac.build",
    },
    WorkflowStepVocabularyEntry {
        uses: "capability.identity.check",
        handler: WorkflowStepHandler::IdentityCheck,
        canonical_capability_id: "vac.identity.check",
    },
    WorkflowStepVocabularyEntry {
        uses: "capability.tui.no_duplicate_runtime",
        handler: WorkflowStepHandler::NoDuplicateTui,
        canonical_capability_id: "vac.tui",
    },
    WorkflowStepVocabularyEntry {
        uses: "capability.ownership.scan",
        handler: WorkflowStepHandler::OwnershipScan,
        canonical_capability_id: "vac.ownership",
    },
    WorkflowStepVocabularyEntry {
        uses: "capability.architecture.invariants",
        handler: WorkflowStepHandler::ArchitectureInvariants,
        canonical_capability_id: "vac.architecture",
    },
    WorkflowStepVocabularyEntry {
        uses: "capability.activity.emit",
        handler: WorkflowStepHandler::ActivityEmit,
        canonical_capability_id: "vac.workflow",
    },
    WorkflowStepVocabularyEntry {
        uses: "capability.approval.request",
        handler: WorkflowStepHandler::ApprovalRequest,
        canonical_capability_id: "vac.approvals",
    },
    WorkflowStepVocabularyEntry {
        uses: "capability.tui.pty_gate",
        handler: WorkflowStepHandler::TuiPtyGate,
        canonical_capability_id: "vac.sandbox",
    },
    WorkflowStepVocabularyEntry {
        uses: "capability.donor_migration.inventory_check",
        handler: WorkflowStepHandler::DonorMigrationInventoryCheck,
        canonical_capability_id: "vac.donor_migration",
    },
    WorkflowStepVocabularyEntry {
        uses: "capability.donor_migration.drift_check",
        handler: WorkflowStepHandler::DonorMigrationDriftCheck,
        canonical_capability_id: "vac.donor_migration",
    },
    WorkflowStepVocabularyEntry {
        uses: "capability.donor_migration.manifest_check",
        handler: WorkflowStepHandler::DonorMigrationManifestCheck,
        canonical_capability_id: "vac.donor_migration",
    },
    WorkflowStepVocabularyEntry {
        uses: "capability.donor_migration.reachability_check",
        handler: WorkflowStepHandler::DonorMigrationReachabilityCheck,
        canonical_capability_id: "vac.donor_migration",
    },
    WorkflowStepVocabularyEntry {
        uses: "capability.donor_migration.evidence_check",
        handler: WorkflowStepHandler::DonorMigrationEvidenceCheck,
        canonical_capability_id: "vac.donor_migration",
    },
    WorkflowStepVocabularyEntry {
        uses: "capability.donor_migration.commit_phrase_check",
        handler: WorkflowStepHandler::DonorMigrationCommitPhraseCheck,
        canonical_capability_id: "vac.donor_migration",
    },
    WorkflowStepVocabularyEntry {
        uses: "capability.registry.schema_check",
        handler: WorkflowStepHandler::RegistrySchemaCheck,
        canonical_capability_id: "vac.workflow",
    },
    WorkflowStepVocabularyEntry {
        uses: "capability.dashboard.check",
        handler: WorkflowStepHandler::CapabilityDashboardCheck,
        canonical_capability_id: "vac.ownership",
    },
    WorkflowStepVocabularyEntry {
        uses: "capability.browser.check",
        handler: WorkflowStepHandler::WorkflowBrowserCheck,
        canonical_capability_id: "vac.workflow",
    },
];

const _: () = {
    assert!(WORKFLOW_STEP_VOCABULARY.len() == INITIAL_ALLOWED_STEP_USES.len());
};

/// Policy intent for a built-in workflow step.
/// Kept adjacent to WORKFLOW_STEP_VOCABULARY so the allowlist and intent
/// mapping cannot drift independently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowStepPolicyIntent {
    pub uses: &'static str,
    pub primary_action: PolicyAction,
    pub path_scope: Option<PolicyPathScope>,
    pub data_class: Option<PolicyDataClass>,
    /// Whether this step inherently requires approval regardless of manifest rules.
    pub requires_approval_inherently: bool,
}

pub const WORKFLOW_STEP_POLICY_INTENTS: &[WorkflowStepPolicyIntent] = &[
    WorkflowStepPolicyIntent {
        uses: "capability.root_seed.coverage",
        primary_action: PolicyAction::FilesystemRead,
        path_scope: Some(PolicyPathScope::Project),
        data_class: None,
        requires_approval_inherently: false,
    },
    WorkflowStepPolicyIntent {
        uses: "capability.build.cargo_check",
        primary_action: PolicyAction::ProcessExecute,
        path_scope: Some(PolicyPathScope::Project),
        data_class: None,
        requires_approval_inherently: true,
    },
    WorkflowStepPolicyIntent {
        uses: "capability.identity.check",
        primary_action: PolicyAction::FilesystemRead,
        path_scope: Some(PolicyPathScope::Project),
        data_class: None,
        requires_approval_inherently: false,
    },
    WorkflowStepPolicyIntent {
        uses: "capability.tui.no_duplicate_runtime",
        primary_action: PolicyAction::FilesystemRead,
        path_scope: Some(PolicyPathScope::Project),
        data_class: None,
        requires_approval_inherently: false,
    },
    WorkflowStepPolicyIntent {
        uses: "capability.ownership.scan",
        primary_action: PolicyAction::FilesystemRead,
        path_scope: Some(PolicyPathScope::Project),
        data_class: None,
        requires_approval_inherently: false,
    },
    WorkflowStepPolicyIntent {
        uses: "capability.architecture.invariants",
        primary_action: PolicyAction::FilesystemRead,
        path_scope: Some(PolicyPathScope::Project),
        data_class: None,
        requires_approval_inherently: false,
    },
    WorkflowStepPolicyIntent {
        uses: "capability.activity.emit",
        primary_action: PolicyAction::SessionWrite,
        path_scope: None,
        data_class: Some(PolicyDataClass::Logs),
        requires_approval_inherently: false,
    },
    WorkflowStepPolicyIntent {
        uses: "capability.approval.request",
        primary_action: PolicyAction::SessionWrite,
        path_scope: None,
        data_class: None,
        requires_approval_inherently: true,
    },
    WorkflowStepPolicyIntent {
        uses: "capability.tui.pty_gate",
        primary_action: PolicyAction::ProcessExecute,
        path_scope: Some(PolicyPathScope::Project),
        data_class: None,
        requires_approval_inherently: true,
    },
    WorkflowStepPolicyIntent {
        uses: "capability.donor_migration.inventory_check",
        primary_action: PolicyAction::FilesystemRead,
        path_scope: Some(PolicyPathScope::Project),
        data_class: None,
        requires_approval_inherently: false,
    },
    WorkflowStepPolicyIntent {
        uses: "capability.donor_migration.drift_check",
        primary_action: PolicyAction::FilesystemRead,
        path_scope: Some(PolicyPathScope::Project),
        data_class: None,
        requires_approval_inherently: false,
    },
    WorkflowStepPolicyIntent {
        uses: "capability.donor_migration.manifest_check",
        primary_action: PolicyAction::FilesystemRead,
        path_scope: Some(PolicyPathScope::Project),
        data_class: None,
        requires_approval_inherently: false,
    },
    WorkflowStepPolicyIntent {
        uses: "capability.donor_migration.reachability_check",
        primary_action: PolicyAction::FilesystemRead,
        path_scope: Some(PolicyPathScope::Project),
        data_class: None,
        requires_approval_inherently: false,
    },
    WorkflowStepPolicyIntent {
        uses: "capability.donor_migration.evidence_check",
        primary_action: PolicyAction::FilesystemRead,
        path_scope: Some(PolicyPathScope::Project),
        data_class: None,
        requires_approval_inherently: false,
    },
    WorkflowStepPolicyIntent {
        uses: "capability.donor_migration.commit_phrase_check",
        primary_action: PolicyAction::FilesystemRead,
        path_scope: Some(PolicyPathScope::Project),
        data_class: None,
        requires_approval_inherently: false,
    },
    WorkflowStepPolicyIntent {
        uses: "capability.registry.schema_check",
        primary_action: PolicyAction::FilesystemRead,
        path_scope: Some(PolicyPathScope::Project),
        data_class: None,
        requires_approval_inherently: false,
    },
    WorkflowStepPolicyIntent {
        uses: "capability.dashboard.check",
        primary_action: PolicyAction::FilesystemRead,
        path_scope: Some(PolicyPathScope::Project),
        data_class: None,
        requires_approval_inherently: false,
    },
    WorkflowStepPolicyIntent {
        uses: "capability.browser.check",
        primary_action: PolicyAction::FilesystemRead,
        path_scope: Some(PolicyPathScope::Project),
        data_class: None,
        requires_approval_inherently: false,
    },
];

const _POLICY_INTENTS_VOCABULARY_SYNC: () = {
    assert!(WORKFLOW_STEP_POLICY_INTENTS.len() == WORKFLOW_STEP_VOCABULARY.len());
};

/// Resolve the policy intent for a step `uses` identifier.
pub fn resolve_workflow_step_policy_intent(
    uses: &str,
) -> Option<&'static WorkflowStepPolicyIntent> {
    WORKFLOW_STEP_POLICY_INTENTS.iter().find(|i| i.uses == uses)
}

/// Build a runtime policy execution intent for a workflow step `uses` identifier.
///
/// Combines the compile-time `WorkflowStepPolicyIntent` (action, scope, data
/// class, inherent-approval flag) with the canonical capability id from the
/// vocabulary so the policy evaluator can match capability-scoped rules.
pub fn build_workflow_step_execution_intent(
    uses: &str,
) -> Option<super::policy_manifest::WorkflowStepExecutionIntent> {
    let intent = resolve_workflow_step_policy_intent(uses)?;
    let vocab = WORKFLOW_STEP_VOCABULARY
        .iter()
        .find(|entry| entry.uses == uses)?;
    Some(super::policy_manifest::WorkflowStepExecutionIntent {
        action: intent.primary_action,
        path_scope: intent.path_scope,
        data_class: intent.data_class,
        tool: Some(intent.uses.to_string()),
        network_scope: None,
        requires_approval_inherently: intent.requires_approval_inherently,
        step_uses: intent.uses.to_string(),
        capability_id: Some(vocab.canonical_capability_id.to_string()),
    })
}

/// Evaluate per-step policy decisions for a workflow manifest against the loaded
/// policy manifests. Returns one entry per step. Built-in workflow vocabulary
/// entries are evaluated through their typed execution intent; custom/unknown
/// `uses` entries deliberately become `UnknownPolicy` instead of `None` so a
/// custom step cannot bypass the policy gate by falling outside the compile-time
/// vocabulary.
pub fn evaluate_workflow_step_policy_decisions(
    manifest: &WorkflowManifest,
    policy_manifests: &[super::policy_manifest::PolicyManifest],
) -> Vec<Option<super::policy_manifest::PolicyDecisionReport>> {
    use super::policy_manifest::PolicyDecisionReport;

    manifest
        .steps
        .iter()
        .map(|step| match build_workflow_step_execution_intent(&step.uses) {
            Some(intent) => Some(super::policy_manifest::evaluate_step_policy(
                &intent,
                policy_manifests,
            )),
            None => Some(PolicyDecisionReport::UnknownPolicy {
                reasons: vec![format!(
                    "workflow step `{}` is outside the built-in policy vocabulary; explicit approval is required",
                    step.uses
                )],
            }),
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowStepHandler {
    RootSeedCoverage,
    BuildCargoCheck,
    IdentityCheck,
    NoDuplicateTui,
    OwnershipScan,
    ArchitectureInvariants,
    ActivityEmit,
    ApprovalRequest,
    TuiPtyGate,
    DonorMigrationInventoryCheck,
    DonorMigrationDriftCheck,
    DonorMigrationManifestCheck,
    DonorMigrationReachabilityCheck,
    DonorMigrationEvidenceCheck,
    DonorMigrationCommitPhraseCheck,
    RegistrySchemaCheck,
    CapabilityDashboardCheck,
    WorkflowBrowserCheck,
}

impl WorkflowStepHandler {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RootSeedCoverage => "root_seed.coverage",
            Self::BuildCargoCheck => "build.cargo_check",
            Self::IdentityCheck => "identity.check",
            Self::NoDuplicateTui => "tui.no_duplicate_runtime",
            Self::OwnershipScan => "ownership.scan",
            Self::ArchitectureInvariants => "architecture.invariants",
            Self::ActivityEmit => "activity.emit",
            Self::ApprovalRequest => "approval.request",
            Self::TuiPtyGate => "tui.pty_gate",
            Self::DonorMigrationInventoryCheck => "donor_migration.inventory_check",
            Self::DonorMigrationDriftCheck => "donor_migration.drift_check",
            Self::DonorMigrationManifestCheck => "donor_migration.manifest_check",
            Self::DonorMigrationReachabilityCheck => "donor_migration.reachability_check",
            Self::DonorMigrationEvidenceCheck => "donor_migration.evidence_check",
            Self::DonorMigrationCommitPhraseCheck => "donor_migration.commit_phrase_check",
            Self::RegistrySchemaCheck => "registry.schema_check",
            Self::CapabilityDashboardCheck => "dashboard.check",
            Self::WorkflowBrowserCheck => "browser.check",
        }
    }
}

impl fmt::Display for WorkflowStepHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiPtyGateResultState {
    Passed,
    Failed,
    BlockedOperator,
    SkippedNotApplicable,
}

impl TuiPtyGateResultState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::BlockedOperator => "blocked_operator",
            Self::SkippedNotApplicable => "skipped_not_applicable",
        }
    }

    pub fn satisfies_release_gate(self) -> bool {
        matches!(self, Self::Passed)
    }
}

impl fmt::Display for TuiPtyGateResultState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiPtyGateResult {
    pub state: TuiPtyGateResultState,
    pub reason: String,
    pub exit_code: Option<i32>,
    pub evidence: Vec<String>,
}

impl TuiPtyGateResult {
    pub fn passed(exit_code: Option<i32>, evidence: Vec<String>) -> Self {
        Self {
            state: TuiPtyGateResultState::Passed,
            reason: "PTY launched root vac, accepted slash input, and exited after Ctrl-C"
                .to_string(),
            exit_code,
            evidence,
        }
    }

    pub fn failed(
        reason: impl Into<String>,
        exit_code: Option<i32>,
        evidence: Vec<String>,
    ) -> Self {
        Self {
            state: TuiPtyGateResultState::Failed,
            reason: reason.into(),
            exit_code,
            evidence,
        }
    }

    pub fn blocked_operator(reason: impl Into<String>, evidence: Vec<String>) -> Self {
        Self {
            state: TuiPtyGateResultState::BlockedOperator,
            reason: reason.into(),
            exit_code: None,
            evidence,
        }
    }

    pub fn skipped_not_applicable(reason: impl Into<String>, evidence: Vec<String>) -> Self {
        Self {
            state: TuiPtyGateResultState::SkippedNotApplicable,
            reason: reason.into(),
            exit_code: None,
            evidence,
        }
    }

    pub fn satisfies_release_gate(&self) -> bool {
        self.state.satisfies_release_gate()
    }

    pub fn report_reason(&self) -> String {
        match self.state {
            TuiPtyGateResultState::Passed => self.reason.clone(),
            TuiPtyGateResultState::BlockedOperator => {
                format!("BLOCKED-OPERATOR: {}", self.reason)
            }
            TuiPtyGateResultState::SkippedNotApplicable => {
                format!("SKIPPED-NOT-APPLICABLE: {}", self.reason)
            }
            TuiPtyGateResultState::Failed => self.reason.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowStepLifecycle {
    Running,
    WaitingApproval,
    ResumedAfterApproval,
    Succeeded,
    Failed,
}

impl WorkflowStepLifecycle {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::WaitingApproval => "waiting_approval",
            Self::ResumedAfterApproval => "resumed_after_approval",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
}

impl fmt::Display for WorkflowStepLifecycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowBlockedStep {
    pub id: String,
    pub uses: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowStepResolution {
    pub id: String,
    pub uses: String,
    pub handler: Option<WorkflowStepHandler>,
    pub blocked_reason: Option<String>,
}

impl WorkflowStepResolution {
    pub fn supported(&self) -> bool {
        self.handler.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowDryRunEvent {
    Started {
        workflow_id: String,
        title: String,
        step_count: usize,
    },
    StepResolved {
        index: usize,
        resolution: WorkflowStepResolution,
    },
    StepBlocked {
        index: usize,
        resolution: WorkflowStepResolution,
    },
    Finished {
        supported_step_count: usize,
        blocked_step_count: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowDryRunState {
    Ready {
        workflow_id: String,
        title: String,
        step_count: usize,
    },
    Started {
        workflow_id: String,
        title: String,
        step_count: usize,
    },
    Step {
        index: usize,
        step_count: usize,
        resolution: WorkflowStepResolution,
        supported_step_count: usize,
        blocked_step_count: usize,
    },
    Finished {
        workflow_id: String,
        title: String,
        supported_step_count: usize,
        blocked_step_count: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowDryRunMachine {
    workflow_id: String,
    title: String,
    step_count: usize,
    step_resolutions: Vec<WorkflowStepResolution>,
    cursor: usize,
    supported_step_count: usize,
    blocked_step_count: usize,
    started: bool,
    finished: bool,
    state_trace: Vec<WorkflowDryRunState>,
    events: Vec<WorkflowDryRunEvent>,
}

