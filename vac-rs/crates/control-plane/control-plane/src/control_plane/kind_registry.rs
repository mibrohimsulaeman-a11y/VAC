#![allow(dead_code)]
use std::fmt;
use std::str::FromStr;

/// Typed registry of every `.vac/` manifest kind accepted by the VAC-Init
/// v1-alpha control plane.
///
/// The refined spec kinds are marked as `Specification`. A few current-root
/// descriptors are marked as `CurrentWorkspaceCompatibility` so existing
/// `.vac/registry/*.yaml` files remain readable until an explicit registry
/// migration converts them to the refined names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VacManifestKind {
    Capability,
    Policy,
    Workflow,
    WorkflowStep,
    Surface,
    RegistryStatus,
    Domains,
    InitState,
    Evidence,
    Plan,
    ApprovalRequest,
    OwnershipReport,
    MemoryRecord,
    RiskFinding,
    Migration,
    Trajectory,
    TestAssertion,
    RuntimeOwnerSupport,
    SemanticPlan,
    Product,
    Status,
    DonorInventory,
}

impl VacManifestKind {
    pub const SPECIFICATION: &'static [Self] = &[
        Self::Capability,
        Self::Policy,
        Self::Workflow,
        Self::WorkflowStep,
        Self::Surface,
        Self::RegistryStatus,
        Self::Domains,
        Self::InitState,
        Self::Evidence,
        Self::Plan,
        Self::ApprovalRequest,
        Self::OwnershipReport,
        Self::MemoryRecord,
        Self::RiskFinding,
        Self::Migration,
        Self::Trajectory,
        Self::TestAssertion,
        Self::RuntimeOwnerSupport,
        Self::SemanticPlan,
    ];

    pub const COMPATIBILITY: &'static [Self] = &[Self::Product, Self::Status, Self::DonorInventory];

    pub const ALL: &'static [Self] = &[
        Self::Capability,
        Self::Policy,
        Self::Workflow,
        Self::WorkflowStep,
        Self::Surface,
        Self::RegistryStatus,
        Self::Domains,
        Self::InitState,
        Self::Evidence,
        Self::Plan,
        Self::ApprovalRequest,
        Self::OwnershipReport,
        Self::MemoryRecord,
        Self::RiskFinding,
        Self::Migration,
        Self::Trajectory,
        Self::TestAssertion,
        Self::RuntimeOwnerSupport,
        Self::SemanticPlan,
        Self::Product,
        Self::Status,
        Self::DonorInventory,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Capability => "capability",
            Self::Policy => "policy",
            Self::Workflow => "workflow",
            Self::WorkflowStep => "workflow_step",
            Self::Surface => "surface",
            Self::RegistryStatus => "registry_status",
            Self::Domains => "domains",
            Self::InitState => "init_state",
            Self::Evidence => "evidence",
            Self::Plan => "plan",
            Self::ApprovalRequest => "approval_request",
            Self::OwnershipReport => "ownership_report",
            Self::MemoryRecord => "memory_record",
            Self::RiskFinding => "risk_finding",
            Self::Migration => "migration",
            Self::Trajectory => "trajectory",
            Self::TestAssertion => "test_assertion",
            Self::RuntimeOwnerSupport => "runtime_owner_support",
            Self::SemanticPlan => "semantic_plan",
            Self::Product => "product",
            Self::Status => "status",
            Self::DonorInventory => "donor_inventory",
        }
    }

    pub const fn domain(self) -> ManifestKindDomain {
        match self {
            Self::Capability
            | Self::Policy
            | Self::Workflow
            | Self::WorkflowStep
            | Self::Surface => ManifestKindDomain::Core,
            Self::RegistryStatus
            | Self::Domains
            | Self::Migration
            | Self::Product
            | Self::Status
            | Self::DonorInventory => ManifestKindDomain::Registry,
            Self::InitState => ManifestKindDomain::Init,
            Self::Evidence | Self::Trajectory => ManifestKindDomain::Audit,
            Self::Plan | Self::ApprovalRequest | Self::SemanticPlan => ManifestKindDomain::Agent,
            Self::OwnershipReport | Self::RiskFinding => ManifestKindDomain::Scan,
            Self::MemoryRecord => ManifestKindDomain::Memory,
            Self::RuntimeOwnerSupport => ManifestKindDomain::Runtime,
            Self::TestAssertion => ManifestKindDomain::Test,
        }
    }

    pub const fn stance(self) -> ManifestKindStance {
        match self {
            Self::Product | Self::Status | Self::DonorInventory => {
                ManifestKindStance::CurrentWorkspaceCompatibility
            }
            _ => ManifestKindStance::Specification,
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::Capability => "Feature registration manifest",
            Self::Policy => "Security and access rule set",
            Self::Workflow => "Ordered workflow definition",
            Self::WorkflowStep => "Reusable custom workflow step",
            Self::Surface => "CLI/TUI/slash/palette binding",
            Self::RegistryStatus => "Workspace health and migration status",
            Self::Domains => "Domain readiness summary",
            Self::InitState => "vac init state persistence",
            Self::Evidence => "Hash-chained evidence record",
            Self::Plan => "Bounded Semantic Plan",
            Self::ApprovalRequest => "Operator approval request/response",
            Self::OwnershipReport => "File ownership scan result",
            Self::MemoryRecord => "Persisted memory record",
            Self::RiskFinding => "AST/import/path scanner risk finding",
            Self::Migration => "Schema migration definition",
            Self::Trajectory => "Per-file safe rationale index for vac why",
            Self::TestAssertion => "Fixture expectation",
            Self::RuntimeOwnerSupport => "Runtime owner capability support report",
            Self::SemanticPlan => "Semantic implementation plan with typed anchors",
            Self::Product => "Current root product descriptor compatibility kind",
            Self::Status => "Current root registry status compatibility kind",
            Self::DonorInventory => "Current donor migration inventory compatibility kind",
        }
    }

    pub const fn is_control_plane_pillar(self) -> bool {
        matches!(
            self,
            Self::Capability | Self::Policy | Self::Workflow | Self::Surface | Self::RegistryStatus
        )
    }
}

impl fmt::Display for VacManifestKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for VacManifestKind {
    type Err = KindRegistryError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        validate_manifest_kind(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ManifestKindDomain {
    Core,
    Registry,
    Init,
    Audit,
    Agent,
    Scan,
    Memory,
    Runtime,
    Test,
}

impl ManifestKindDomain {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Registry => "registry",
            Self::Init => "init",
            Self::Audit => "audit",
            Self::Agent => "agent",
            Self::Scan => "scan",
            Self::Memory => "memory",
            Self::Runtime => "runtime",
            Self::Test => "test",
        }
    }
}

impl fmt::Display for ManifestKindDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ManifestKindStance {
    Specification,
    CurrentWorkspaceCompatibility,
}

impl ManifestKindStance {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Specification => "specification",
            Self::CurrentWorkspaceCompatibility => "current_workspace_compatibility",
        }
    }
}

impl fmt::Display for ManifestKindStance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KindRegistryEntry {
    pub kind: VacManifestKind,
    pub canonical_name: &'static str,
    pub domain: ManifestKindDomain,
    pub stance: ManifestKindStance,
    pub description: &'static str,
}

pub const KIND_REGISTRY: &[KindRegistryEntry] = &[
    entry(VacManifestKind::Capability),
    entry(VacManifestKind::Policy),
    entry(VacManifestKind::Workflow),
    entry(VacManifestKind::WorkflowStep),
    entry(VacManifestKind::Surface),
    entry(VacManifestKind::RegistryStatus),
    entry(VacManifestKind::Domains),
    entry(VacManifestKind::InitState),
    entry(VacManifestKind::Evidence),
    entry(VacManifestKind::Plan),
    entry(VacManifestKind::ApprovalRequest),
    entry(VacManifestKind::OwnershipReport),
    entry(VacManifestKind::MemoryRecord),
    entry(VacManifestKind::RiskFinding),
    entry(VacManifestKind::Migration),
    entry(VacManifestKind::Trajectory),
    entry(VacManifestKind::TestAssertion),
    entry(VacManifestKind::RuntimeOwnerSupport),
    entry(VacManifestKind::SemanticPlan),
    entry(VacManifestKind::Product),
    entry(VacManifestKind::Status),
    entry(VacManifestKind::DonorInventory),
];

const fn entry(kind: VacManifestKind) -> KindRegistryEntry {
    KindRegistryEntry {
        kind,
        canonical_name: kind.as_str(),
        domain: kind.domain(),
        stance: kind.stance(),
        description: kind.description(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KindRegistryError {
    EmptyKind,
    UnknownKind {
        value: String,
    },
    NonCanonicalKind {
        value: String,
        canonical: &'static str,
    },
}

impl KindRegistryError {
    pub fn value(&self) -> &str {
        match self {
            Self::EmptyKind => "",
            Self::UnknownKind { value } | Self::NonCanonicalKind { value, .. } => value,
        }
    }
}

impl fmt::Display for KindRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyKind => f.write_str("manifest kind must not be empty"),
            Self::UnknownKind { value } => write!(
                f,
                "unknown .vac manifest kind `{value}`; expected one of: {}",
                canonical_kind_names().join(", ")
            ),
            Self::NonCanonicalKind { value, canonical } => write!(
                f,
                "manifest kind `{value}` is accepted only as canonical `{canonical}`"
            ),
        }
    }
}

impl std::error::Error for KindRegistryError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KindRegistry;

impl KindRegistry {
    pub fn all() -> &'static [KindRegistryEntry] {
        KIND_REGISTRY
    }

    pub fn allowed_values() -> Vec<&'static str> {
        canonical_kind_names()
    }

    pub fn contains(value: &str) -> bool {
        is_known_kind(value)
    }

    pub fn parse(value: &str) -> Result<VacManifestKind, KindRegistryError> {
        validate_manifest_kind(value)
    }
}

pub fn canonical_kind_names() -> Vec<&'static str> {
    KIND_REGISTRY
        .iter()
        .map(|entry| entry.canonical_name)
        .collect()
}

pub fn specification_kind_names() -> Vec<&'static str> {
    KIND_REGISTRY
        .iter()
        .filter(|entry| entry.stance == ManifestKindStance::Specification)
        .map(|entry| entry.canonical_name)
        .collect()
}

pub fn compatibility_kind_names() -> Vec<&'static str> {
    KIND_REGISTRY
        .iter()
        .filter(|entry| entry.stance == ManifestKindStance::CurrentWorkspaceCompatibility)
        .map(|entry| entry.canonical_name)
        .collect()
}

/// Historical spellings deliberately kept as a report-only list so migration
/// tooling can explain a failure without accepting stale manifest input.
pub fn legacy_kind_names() -> &'static [(&'static str, &'static str)] {
    &[
        ("approval", "approval_request"),
        ("ownership", "ownership_report"),
        ("memory", "memory_record"),
        ("risk", "risk_finding"),
    ]
}

pub fn is_known_kind(value: &str) -> bool {
    validate_manifest_kind(value).is_ok()
}

pub fn kind_registry_entry(kind: VacManifestKind) -> &'static KindRegistryEntry {
    KIND_REGISTRY
        .iter()
        .find(|entry| entry.kind == kind)
        .expect("all VacManifestKind variants must be present in KIND_REGISTRY")
}

pub fn validate_manifest_kind(value: &str) -> Result<VacManifestKind, KindRegistryError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(KindRegistryError::EmptyKind);
    }

    for entry in KIND_REGISTRY {
        if entry.canonical_name == value {
            return Ok(entry.kind);
        }
    }

    for (legacy, canonical) in legacy_kind_names() {
        if *legacy == value {
            return Err(KindRegistryError::NonCanonicalKind {
                value: value.to_string(),
                canonical,
            });
        }
    }

    Err(KindRegistryError::UnknownKind {
        value: value.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_every_vac_init_v1_alpha_spec_kind() {
        let values = specification_kind_names();
        for expected in [
            "capability",
            "policy",
            "workflow",
            "workflow_step",
            "surface",
            "registry_status",
            "domains",
            "init_state",
            "evidence",
            "plan",
            "approval_request",
            "ownership_report",
            "memory_record",
            "risk_finding",
            "migration",
            "trajectory",
            "test_assertion",
            "runtime_owner_support",
            "semantic_plan",
        ] {
            assert!(values.contains(&expected), "missing kind {expected}");
        }
        assert_eq!(values.len(), 19);
    }

    #[test]
    fn registry_accepts_current_workspace_compatibility_kinds() {
        assert_eq!(
            compatibility_kind_names(),
            vec!["product", "status", "donor_inventory"]
        );
        assert_eq!(
            KindRegistry::parse("product").unwrap(),
            VacManifestKind::Product
        );
        assert_eq!(
            KindRegistry::parse("status").unwrap(),
            VacManifestKind::Status
        );
        assert_eq!(
            KindRegistry::parse("donor_inventory").unwrap(),
            VacManifestKind::DonorInventory
        );
        assert_eq!(KIND_REGISTRY.len(), 22);
    }

    #[test]
    fn parser_accepts_known_kind_and_rejects_unknown_kind() {
        assert_eq!(
            KindRegistry::parse("capability").unwrap(),
            VacManifestKind::Capability
        );
        assert_eq!(
            KindRegistry::parse("approval_request").unwrap(),
            VacManifestKind::ApprovalRequest
        );
        let error = KindRegistry::parse("freeform_script").unwrap_err();
        assert_eq!(error.value(), "freeform_script");
    }

    #[test]
    fn parser_rejects_non_canonical_legacy_kind() {
        let error = validate_manifest_kind("approval").unwrap_err();
        assert_eq!(
            error,
            KindRegistryError::NonCanonicalKind {
                value: "approval".to_string(),
                canonical: "approval_request",
            }
        );
    }

    #[test]
    fn kind_entries_report_domain_description_and_stance() {
        let capability = kind_registry_entry(VacManifestKind::Capability);
        assert_eq!(capability.domain, ManifestKindDomain::Core);
        assert_eq!(capability.stance, ManifestKindStance::Specification);
        assert!(capability.description.contains("Feature"));

        let product = kind_registry_entry(VacManifestKind::Product);
        assert_eq!(product.domain, ManifestKindDomain::Registry);
        assert_eq!(
            product.stance,
            ManifestKindStance::CurrentWorkspaceCompatibility
        );
    }
}
