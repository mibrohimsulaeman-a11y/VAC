use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct CapabilityManifest {
    pub schema_version: u32,
    pub kind: CapabilityManifestKind,
    pub id: String,
    pub title: String,
    pub status: CapabilityStatus,
    pub owner: CapabilityOwner,
    pub surfaces: CapabilitySurfaces,
    pub policy: CapabilityPolicy,
    pub validation: CapabilityValidation,
    pub description: Option<String>,
    pub reason: Option<String>,
    pub depends_on: Vec<String>,
    pub optional_depends_on: Vec<String>,
    pub runtime_events: Vec<String>,
    pub states: Option<CapabilityStates>,
    pub ownership: Option<CapabilityOwnership>,
    pub donor_source: Option<CapabilityDonorSource>,
    pub compatibility_transport: Vec<CapabilityCompatibilityTransport>,
    pub docs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
pub enum CapabilityManifestKind {
    #[serde(rename = "capability")]
    Capability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    Planned,
    Partial,
    Ready,
    Deprecated,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum CapabilityOwner {
    Path(String),
    Structured(CapabilityOwnerObject),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct CapabilityOwnerObject {
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub module: String,
    #[serde(default)]
    pub team: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct CapabilitySurfaces {
    #[serde(default)]
    pub tui: Option<CapabilityRouteCollection>,
    #[serde(default)]
    pub slash: Option<CapabilityCommandCollection>,
    pub palette: bool,
    #[serde(default)]
    pub cli: Option<CapabilityCliSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct CapabilityRouteCollection {
    pub routes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct CapabilityCommandCollection {
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct CapabilityCliSurface {
    pub enabled: bool,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct CapabilityPolicy {
    #[serde(default)]
    pub risk: Option<String>,
    #[serde(default)]
    pub mutates_files: Option<CapabilityPolicyValue>,
    #[serde(default)]
    pub network: Option<CapabilityPolicyValue>,
    #[serde(default)]
    pub redaction: Option<CapabilityPolicyValue>,
    #[serde(default)]
    pub approval_required_for: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum CapabilityPolicyValue {
    Bool(bool),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct CapabilityValidation {
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub gates: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityOwnershipTargetKind {
    Module,
    Crate,
    Path,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CapabilityOwnershipTarget {
    #[serde(default = "default_ownership_target_kind")]
    pub kind: CapabilityOwnershipTargetKind,
    #[serde(default)]
    pub crate_name: Option<String>,
    #[serde(default)]
    pub module: Option<String>,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub test_only: bool,
    #[serde(default)]
    pub retired: bool,
}

const fn default_ownership_target_kind() -> CapabilityOwnershipTargetKind {
    CapabilityOwnershipTargetKind::Module
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct CapabilityOwnership {
    #[serde(default)]
    pub crates: Vec<String>,
    #[serde(default)]
    pub modules: Vec<String>,
    #[serde(default)]
    pub test_only: bool,
    #[serde(default)]
    pub retired: bool,
    pub deletion_plan: Option<String>,
    #[serde(default)]
    pub targets: Vec<CapabilityOwnershipTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityCompatibilityTransportStatus {
    CompatibilityTransport,
    Quarantined,
    Deferred,
}

impl std::fmt::Display for CapabilityCompatibilityTransportStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::CompatibilityTransport => "compatibility_transport",
            Self::Quarantined => "quarantined",
            Self::Deferred => "deferred",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct CapabilityCompatibilityTransport {
    pub dependency: String,
    pub status: CapabilityCompatibilityTransportStatus,
    pub owner: String,
    pub deletion_condition: String,
    pub replacement: String,
    pub target_removal_plan: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum CapabilityStates {
    Labels {
        empty: String,
        loading: String,
        success: String,
        failure: String,
    },
    List(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityDonorCleanupStatus {
    DonorOnly,
    Migrated,
    DeleteCandidate,
}

impl std::fmt::Display for CapabilityDonorCleanupStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::DonorOnly => "donor_only",
            Self::Migrated => "migrated",
            Self::DeleteCandidate => "delete_candidate",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct CapabilityDonorSource {
    pub source: String,
    pub root_target: String,
    pub cleanup_status: CapabilityDonorCleanupStatus,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{path}:{field_path}: {message}")]
pub struct CapabilityManifestError {
    path: PathBuf,
    field_path: String,
    message: String,
}

impl CapabilityManifestError {
    pub fn new(
        path: impl Into<PathBuf>,
        field_path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            field_path: field_path.into(),
            message: message.into(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn field_path(&self) -> &str {
        &self.field_path
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCapabilityManifest {
    schema_version: Option<u32>,
    kind: Option<String>,
    id: Option<String>,
    title: Option<String>,
    status: Option<String>,
    owner: Option<RawCapabilityOwner>,
    surfaces: Option<RawCapabilitySurfaces>,
    policy: Option<RawCapabilityPolicy>,
    validation: Option<RawCapabilityValidation>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    depends_on: Option<Vec<String>>,
    #[serde(default)]
    optional_depends_on: Option<Vec<String>>,
    #[serde(default)]
    runtime_events: Option<Vec<String>>,
    #[serde(default)]
    states: Option<RawCapabilityStates>,
    #[serde(default)]
    ownership: Option<RawCapabilityOwnership>,
    #[serde(default)]
    donor_source: Option<RawCapabilityDonorSource>,
    #[serde(default)]
    compatibility_transport: Option<Vec<RawCapabilityCompatibilityTransport>>,
    #[serde(default)]
    docs: Option<Vec<String>>,
    #[serde(default)]
    runtime_support: Option<serde_yaml::Value>,
    #[serde(default)]
    operator_evidence: Option<serde_yaml::Value>,
    #[serde(default)]
    operator_ui_batch3_validation: Option<serde_yaml::Value>,
    #[serde(default)]
    operator_ui_hardening_validation: Option<serde_yaml::Value>,
    #[serde(default)]
    hardening_batch_4_10: Option<serde_yaml::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawCapabilityOwner {
    Path(String),
    Object(RawCapabilityOwnerObject),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCapabilityOwnerObject {
    #[serde(rename = "crate")]
    crate_name: Option<String>,
    module: Option<String>,
    #[serde(default)]
    team: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCapabilitySurfaces {
    #[serde(default)]
    tui: Option<RawCapabilityRouteCollection>,
    #[serde(default)]
    slash: Option<RawCapabilityCommandCollection>,
    #[serde(default)]
    palette: Option<bool>,
    #[serde(default)]
    cli: Option<RawCapabilityCliSurface>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawCapabilityRouteCollection {
    Route(String),
    Routes(RawCapabilityRouteCollectionObject),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCapabilityRouteCollectionObject {
    #[serde(default)]
    routes: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawCapabilityCommandCollection {
    Command(String),
    Commands(RawCapabilityCommandCollectionObject),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCapabilityCommandCollectionObject {
    #[serde(default)]
    commands: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawCapabilityCliSurface {
    Bool(bool),
    Object(RawCapabilityCliSurfaceObject),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCapabilityCliSurfaceObject {
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    commands: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCapabilityPolicy {
    #[serde(default)]
    risk: Option<String>,
    #[serde(default)]
    mutates_files: Option<RawCapabilityPolicyValue>,
    #[serde(default)]
    network: Option<RawCapabilityPolicyValue>,
    #[serde(default)]
    redaction: Option<RawCapabilityPolicyValue>,
    #[serde(default)]
    approval_required_for: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawCapabilityPolicyValue {
    Bool(bool),
    Text(String),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCapabilityValidation {
    #[serde(default)]
    commands: Option<Vec<RawValidationEntry>>,
    #[serde(default)]
    gates: Option<Vec<RawValidationEntry>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawValidationEntry {
    String(String),
    Object(RawValidationCommand),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawValidationCommand {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    runner: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    risk: Option<String>,
    #[serde(default)]
    approval: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawCapabilityStates {
    List(Vec<String>),
    Labels(RawCapabilityStatesLabels),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCapabilityStatesLabels {
    empty: String,
    loading: String,
    success: String,
    failure: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOwnershipTarget {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    crate_name: Option<String>,
    #[serde(default, rename = "crate")]
    crate_alias: Option<String>,
    #[serde(default)]
    module: Option<String>,
    #[serde(default)]
    include: Option<Vec<String>>,
    #[serde(default)]
    exclude: Option<Vec<String>>,
    #[serde(default)]
    test_only: bool,
    #[serde(default)]
    retired: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCapabilityOwnership {
    #[serde(default)]
    crates: Option<Vec<String>>,
    #[serde(default)]
    modules: Option<Vec<String>>,
    #[serde(default)]
    test_only: bool,
    #[serde(default)]
    retired: bool,
    deletion_plan: Option<String>,
    #[serde(default)]
    targets: Option<Vec<RawOwnershipTarget>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCapabilityDonorSource {
    source: Option<String>,
    #[serde(default)]
    root_target: Option<String>,
    #[serde(default)]
    cleanup_status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCapabilityCompatibilityTransport {
    dependency: Option<String>,
    status: Option<String>,
    owner: Option<String>,
    #[serde(default)]
    quarantine_marker: Option<String>,
    deletion_condition: Option<String>,
    replacement: Option<String>,
    target_removal_plan: Option<String>,
}

pub fn load_capability_manifest(
    path: impl AsRef<Path>,
) -> Result<CapabilityManifest, CapabilityManifestError> {
    let path = path.as_ref();
    let contents = std::fs::read_to_string(path)
        .map_err(|err| CapabilityManifestError::new(path, "root", err.to_string()))?;
    parse_capability_manifest(path, &contents)
}

pub fn parse_capability_manifest(
    path: impl AsRef<Path>,
    contents: &str,
) -> Result<CapabilityManifest, CapabilityManifestError> {
    let path = path.as_ref();
    let deserializer = serde_yaml::Deserializer::from_str(contents);
    let raw: RawCapabilityManifest =
        serde_path_to_error::deserialize(deserializer).map_err(|error| {
            CapabilityManifestError::new(
                path,
                serde_path_to_string(error.path()),
                error.into_inner().to_string(),
            )
        })?;
    CapabilityManifest::from_raw(path, raw)
}

fn validate_manifest_internal(
    manifest: &CapabilityManifest,
    path: &Path,
) -> Result<(), CapabilityManifestError> {
    if manifest.schema_version != 1 {
        return Err(CapabilityManifestError::new(
            path,
            "schema_version",
            format!(
                "unsupported schema version {}; expected 1",
                manifest.schema_version
            ),
        ));
    }
    validate_manifest_id(path, &manifest.id)?;
    if !manifest.policy.has_any_declaration() {
        return Err(CapabilityManifestError::new(
            path,
            "policy",
            "policy must declare at least one rule or decision",
        ));
    }
    if !manifest.surfaces.has_visible_surface() {
        return Err(CapabilityManifestError::new(
            path,
            "surfaces",
            "at least one surface must be declared",
        ));
    }
    if manifest.donor_source.is_some() && !manifest.surfaces.has_tui_or_cli_surface() {
        return Err(CapabilityManifestError::new(
            path,
            "surfaces",
            "donor-backed capabilities must declare a TUI route or CLI command",
        ));
    }
    if matches!(manifest.status, CapabilityStatus::Ready) && !manifest.validation.has_any_evidence()
    {
        return Err(CapabilityManifestError::new(
            path,
            "validation",
            "ready capabilities must declare at least one validation command or gate",
        ));
    }
    if !matches!(manifest.status, CapabilityStatus::Deprecated) && manifest.states.is_none() {
        return Err(CapabilityManifestError::new(
            path,
            "states",
            "non-deprecated capabilities must declare states",
        ));
    }
    if matches!(
        manifest.status,
        CapabilityStatus::Planned | CapabilityStatus::Partial
    ) && manifest
        .reason
        .as_ref()
        .is_none_or(|reason| reason.trim().is_empty())
    {
        return Err(CapabilityManifestError::new(
            path,
            "reason",
            "planned or partial capabilities must declare a reason",
        ));
    }
    if let Some(donor) = &manifest.donor_source {
        validate_donor_root_target(&donor.root_target, path)?;
    }
    Ok(())
}

pub fn validate_capability_manifest(
    manifest: &CapabilityManifest,
) -> Result<(), CapabilityManifestError> {
    validate_manifest_internal(manifest, Path::new("<memory>"))
}

impl CapabilityManifest {
    fn from_raw(path: &Path, raw: RawCapabilityManifest) -> Result<Self, CapabilityManifestError> {
        let schema_version = raw.schema_version.ok_or_else(|| {
            CapabilityManifestError::new(path, "schema_version", "missing required field")
        })?;
        if schema_version != 1 {
            return Err(CapabilityManifestError::new(
                path,
                "schema_version",
                format!("unsupported schema version {schema_version}; expected 1"),
            ));
        }

        let kind = match raw.kind.as_deref() {
            Some("capability") => CapabilityManifestKind::Capability,
            Some(other) => {
                return Err(CapabilityManifestError::new(
                    path,
                    "kind",
                    format!("unknown manifest kind `{other}`; expected `capability`"),
                ));
            }
            None => {
                return Err(CapabilityManifestError::new(
                    path,
                    "kind",
                    "missing required field",
                ));
            }
        };

        let id = normalize_manifest_id(raw.id, path)?;
        let title = normalize_non_empty_string(raw.title, path, "title")?;
        let status = parse_status(raw.status, path)?;
        let owner = parse_owner(raw.owner, path)?;
        let surfaces = parse_surfaces(raw.surfaces, path)?;
        let policy = parse_policy(raw.policy, path)?;
        let validation = parse_validation(raw.validation, path)?;
        let _metadata = (
            raw.runtime_support,
            raw.operator_evidence,
            raw.operator_ui_batch3_validation,
            raw.operator_ui_hardening_validation,
            raw.hardening_batch_4_10,
        );

        let description = raw.description.filter(|value| !value.trim().is_empty());
        let reason = raw.reason.filter(|value| !value.trim().is_empty());
        let depends_on = normalize_string_list(raw.depends_on, path, "depends_on")?;
        let optional_depends_on =
            normalize_string_list(raw.optional_depends_on, path, "optional_depends_on")?;
        let runtime_events = normalize_string_list(raw.runtime_events, path, "runtime_events")?;
        let states = raw
            .states
            .map(|states| parse_states(states, path))
            .transpose()?;
        let ownership = raw
            .ownership
            .map(|ownership| parse_ownership(ownership, path))
            .transpose()?;
        let donor_source = raw
            .donor_source
            .map(|source| parse_donor_source(source, path))
            .transpose()?;
        let compatibility_transport = raw
            .compatibility_transport
            .unwrap_or_default()
            .into_iter()
            .map(|transport| parse_compatibility_transport(transport, path))
            .collect::<Result<Vec<_>, _>>()?;
        let docs = normalize_string_list(raw.docs, path, "docs")?;

        let manifest = Self {
            schema_version,
            kind,
            id,
            title,
            status,
            owner,
            surfaces,
            policy,
            validation,
            description,
            reason,
            depends_on,
            optional_depends_on,
            runtime_events,
            states,
            ownership,
            donor_source,
            compatibility_transport,
            docs,
        };

        validate_manifest_internal(&manifest, path)?;

        Ok(manifest)
    }
}

fn parse_owner(
    raw: Option<RawCapabilityOwner>,
    path: &Path,
) -> Result<CapabilityOwner, CapabilityManifestError> {
    let raw =
        raw.ok_or_else(|| CapabilityManifestError::new(path, "owner", "missing required field"))?;
    match raw {
        RawCapabilityOwner::Path(value) => {
            let value = normalize_owner_path(value, path, "owner")?;
            Ok(CapabilityOwner::Path(value))
        }
        RawCapabilityOwner::Object(object) => {
            let crate_name = normalize_non_empty_string(object.crate_name, path, "owner.crate")?;
            let module = normalize_non_empty_string(object.module, path, "owner.module")?;
            let team = object
                .team
                .map(|team| normalize_non_empty_string(Some(team), path, "owner.team"))
                .transpose()?;
            Ok(CapabilityOwner::Structured(CapabilityOwnerObject {
                crate_name,
                module,
                team,
            }))
        }
    }
}

fn parse_surfaces(
    raw: Option<RawCapabilitySurfaces>,
    path: &Path,
) -> Result<CapabilitySurfaces, CapabilityManifestError> {
    let raw = raw
        .ok_or_else(|| CapabilityManifestError::new(path, "surfaces", "missing required field"))?;

    let tui = match raw.tui {
        None => None,
        Some(value) => Some(parse_routes(value, path, "surfaces.tui")?),
    };
    let slash = match raw.slash {
        None => None,
        Some(value) => Some(parse_commands(value, path, "surfaces.slash")?),
    };
    let cli = match raw.cli {
        None => None,
        Some(value) => Some(parse_cli_surface(value, path, "surfaces.cli")?),
    };
    let palette = raw.palette.unwrap_or(false);

    let surfaces = CapabilitySurfaces {
        tui,
        slash,
        palette,
        cli,
    };
    if !surfaces.has_visible_surface() {
        return Err(CapabilityManifestError::new(
            path,
            "surfaces",
            "at least one surface must be declared",
        ));
    }

    Ok(surfaces)
}

fn parse_policy(
    raw: Option<RawCapabilityPolicy>,
    path: &Path,
) -> Result<CapabilityPolicy, CapabilityManifestError> {
    let raw =
        raw.ok_or_else(|| CapabilityManifestError::new(path, "policy", "missing required field"))?;
    let risk = raw.risk.filter(|value| !value.trim().is_empty());
    let mutates_files = raw
        .mutates_files
        .map(CapabilityPolicyValue::from_raw)
        .transpose()?;
    let network = raw
        .network
        .map(CapabilityPolicyValue::from_raw)
        .transpose()?;
    let redaction = raw
        .redaction
        .map(CapabilityPolicyValue::from_raw)
        .transpose()?;
    let approval_required_for = normalize_string_list(
        raw.approval_required_for,
        path,
        "policy.approval_required_for",
    )?;

    let policy = CapabilityPolicy {
        risk,
        mutates_files,
        network,
        redaction,
        approval_required_for,
    };

    if !policy.has_any_declaration() {
        return Err(CapabilityManifestError::new(
            path,
            "policy",
            "policy must declare at least one rule or decision",
        ));
    }

    Ok(policy)
}

fn parse_validation(
    raw: Option<RawCapabilityValidation>,
    path: &Path,
) -> Result<CapabilityValidation, CapabilityManifestError> {
    let raw = raw.ok_or_else(|| {
        CapabilityManifestError::new(path, "validation", "missing required field")
    })?;
    let commands = normalize_validation_entries(raw.commands, path, "validation.commands")?;
    let gates = normalize_validation_entries(raw.gates, path, "validation.gates")?;
    Ok(CapabilityValidation { commands, gates })
}

fn parse_states(
    raw: RawCapabilityStates,
    path: &Path,
) -> Result<CapabilityStates, CapabilityManifestError> {
    match raw {
        RawCapabilityStates::List(values) => Ok(CapabilityStates::List(normalize_list(
            values, path, "states",
        )?)),
        RawCapabilityStates::Labels(labels) => Ok(CapabilityStates::Labels {
            empty: normalize_non_empty_string(Some(labels.empty), path, "states.empty")?,
            loading: normalize_non_empty_string(Some(labels.loading), path, "states.loading")?,
            success: normalize_non_empty_string(Some(labels.success), path, "states.success")?,
            failure: normalize_non_empty_string(Some(labels.failure), path, "states.failure")?,
        }),
    }
}

fn parse_ownership(
    raw: RawCapabilityOwnership,
    path: &Path,
) -> Result<CapabilityOwnership, CapabilityManifestError> {
    let crates = normalize_string_list(raw.crates, path, "ownership.crates")?;
    let modules = normalize_string_list(raw.modules, path, "ownership.modules")?;

    let has_classic = !crates.is_empty() || !modules.is_empty();
    let has_targets = raw.targets.as_ref().map(|t| !t.is_empty()).unwrap_or(false);

    if !has_classic && !has_targets {
        return Err(CapabilityManifestError::new(
            path,
            "ownership",
            "at least one classic crate/module or granular target must be declared",
        ));
    }

    if raw.test_only && raw.retired {
        return Err(CapabilityManifestError::new(
            path,
            "ownership",
            "test_only and retired cannot both be true",
        ));
    }

    let mut targets = Vec::new();
    if let Some(raw_targets) = raw.targets {
        for (index, raw_target) in raw_targets.into_iter().enumerate() {
            let field_path = format!("ownership.targets[{index}]");
            let kind = parse_ownership_target_kind(raw_target.kind, path, &field_path)?;
            let crate_name = raw_target.crate_name.or(raw_target.crate_alias);
            let crate_name = crate_name
                .map(|value| {
                    normalize_non_empty_string(Some(value), path, &format!("{field_path}.crate"))
                })
                .transpose()?;
            let module = raw_target
                .module
                .map(|value| {
                    normalize_non_empty_string(Some(value), path, &format!("{field_path}.module"))
                })
                .transpose()?;
            let include =
                normalize_string_list(raw_target.include, path, &format!("{field_path}.include"))?;
            let exclude =
                normalize_string_list(raw_target.exclude, path, &format!("{field_path}.exclude"))?;
            validate_ownership_target(path, &field_path, kind, &crate_name, &module, &include)?;
            if raw_target.test_only && raw_target.retired {
                return Err(CapabilityManifestError::new(
                    path,
                    field_path,
                    "test_only and retired cannot both be true",
                ));
            }
            targets.push(CapabilityOwnershipTarget {
                kind,
                crate_name,
                module,
                include,
                exclude,
                test_only: raw_target.test_only,
                retired: raw_target.retired,
            });
        }
    }

    let deletion_plan = if let Some(plan) = raw.deletion_plan {
        let normalized = normalize_non_empty_string(Some(plan), path, "ownership.deletion_plan")?;
        Some(normalized)
    } else {
        None
    };

    let has_retired = raw.retired || targets.iter().any(|t| t.retired);
    if has_retired && deletion_plan.is_none() {
        return Err(CapabilityManifestError::new(
            path,
            "ownership",
            "retired capabilities or targets must specify a non-empty deletion_plan",
        ));
    }

    Ok(CapabilityOwnership {
        crates,
        modules,
        test_only: raw.test_only,
        retired: raw.retired,
        deletion_plan,
        targets,
    })
}

fn parse_ownership_target_kind(
    raw: Option<String>,
    path: &Path,
    field_path: &str,
) -> Result<CapabilityOwnershipTargetKind, CapabilityManifestError> {
    match raw.as_deref().unwrap_or("module") {
        "module" => Ok(CapabilityOwnershipTargetKind::Module),
        "crate" => Ok(CapabilityOwnershipTargetKind::Crate),
        "path" => Ok(CapabilityOwnershipTargetKind::Path),
        other => Err(CapabilityManifestError::new(
            path,
            format!("{field_path}.kind"),
            format!("unknown ownership target kind `{other}`"),
        )),
    }
}

fn validate_ownership_target(
    path: &Path,
    field_path: &str,
    kind: CapabilityOwnershipTargetKind,
    crate_name: &Option<String>,
    module: &Option<String>,
    include: &[String],
) -> Result<(), CapabilityManifestError> {
    match kind {
        CapabilityOwnershipTargetKind::Crate => {
            if crate_name.is_none() {
                return Err(CapabilityManifestError::new(
                    path,
                    format!("{field_path}.crate"),
                    "crate ownership targets require crate",
                ));
            }
        }
        CapabilityOwnershipTargetKind::Module => {
            if crate_name.is_none() || module.is_none() {
                return Err(CapabilityManifestError::new(
                    path,
                    format!("{field_path}.module"),
                    "module ownership targets require crate and module",
                ));
            }
        }
        CapabilityOwnershipTargetKind::Path => {
            if include.is_empty() {
                return Err(CapabilityManifestError::new(
                    path,
                    format!("{field_path}.include"),
                    "path ownership targets require include patterns",
                ));
            }
        }
    }
    Ok(())
}

fn parse_donor_source(
    raw: RawCapabilityDonorSource,
    path: &Path,
) -> Result<CapabilityDonorSource, CapabilityManifestError> {
    let source = normalize_non_empty_string(raw.source, path, "donor_source.source")?;
    let root_target =
        normalize_non_empty_string(raw.root_target, path, "donor_source.root_target")?;
    validate_donor_root_target(&root_target, path)?;

    let cleanup_status_str =
        normalize_non_empty_string(raw.cleanup_status, path, "donor_source.cleanup_status")?;

    let cleanup_status = match cleanup_status_str.as_str() {
        "donor_only" => CapabilityDonorCleanupStatus::DonorOnly,
        "migrated" => CapabilityDonorCleanupStatus::Migrated,
        "delete_candidate" => CapabilityDonorCleanupStatus::DeleteCandidate,
        _ => {
            return Err(CapabilityManifestError::new(
                path,
                "donor_source.cleanup_status",
                "must be one of donor_only, migrated, delete_candidate",
            ));
        }
    };

    Ok(CapabilityDonorSource {
        source,
        root_target,
        cleanup_status,
    })
}

fn validate_donor_root_target(
    root_target: &str,
    path: &Path,
) -> Result<(), CapabilityManifestError> {
    const ALLOWED_PREFIXES: &[&str] = &[
        "vac-rs/core/",
        "vac-rs/cli/",
        "vac-rs/tui/",
        "vac-rs/app-server/",
    ];
    let forbidden = root_target.starts_with("donor/")
        || root_target.contains("/frontend/")
        || root_target.contains("/web/")
        || root_target.contains("node_modules")
        || root_target.ends_with(".tsx")
        || root_target.ends_with(".jsx");
    let allowed = ALLOWED_PREFIXES
        .iter()
        .any(|prefix| root_target.starts_with(prefix));
    if allowed && !forbidden {
        return Ok(());
    }
    Err(CapabilityManifestError::new(
        path,
        "donor_source.root_target",
        "must target an allowlisted VAC Rust/product path, not donor/frontend source",
    ))
}

fn parse_compatibility_transport(
    raw: RawCapabilityCompatibilityTransport,
    path: &Path,
) -> Result<CapabilityCompatibilityTransport, CapabilityManifestError> {
    let dependency =
        normalize_non_empty_string(raw.dependency, path, "compatibility_transport.dependency")?;
    let owner = normalize_non_empty_string(raw.owner, path, "compatibility_transport.owner")?;
    let deletion_condition = normalize_non_empty_string(
        raw.deletion_condition,
        path,
        "compatibility_transport.deletion_condition",
    )?;
    let replacement =
        normalize_non_empty_string(raw.replacement, path, "compatibility_transport.replacement")?;
    let target_removal_plan = normalize_non_empty_string(
        raw.target_removal_plan,
        path,
        "compatibility_transport.target_removal_plan",
    )?;
    let _quarantine_marker = raw.quarantine_marker;
    let status = match raw.status.as_deref() {
        Some("compatibility_transport") => {
            CapabilityCompatibilityTransportStatus::CompatibilityTransport
        }
        Some("quarantined") => CapabilityCompatibilityTransportStatus::Quarantined,
        Some("deferred") => CapabilityCompatibilityTransportStatus::Deferred,
        Some(other) => {
            return Err(CapabilityManifestError::new(
                path,
                "compatibility_transport.status",
                format!("unsupported compatibility transport status `{other}`"),
            ));
        }
        None => {
            return Err(CapabilityManifestError::new(
                path,
                "compatibility_transport.status",
                "missing required field",
            ));
        }
    };

    Ok(CapabilityCompatibilityTransport {
        dependency,
        status,
        owner,
        deletion_condition,
        replacement,
        target_removal_plan,
    })
}

fn parse_routes(
    raw: RawCapabilityRouteCollection,
    path: &Path,
    field_path: &str,
) -> Result<CapabilityRouteCollection, CapabilityManifestError> {
    let routes = match raw {
        RawCapabilityRouteCollection::Route(route) => {
            vec![normalize_non_empty_string(Some(route), path, field_path)?]
        }
        RawCapabilityRouteCollection::Routes(collection) => {
            normalize_list(collection.routes.unwrap_or_default(), path, field_path)?
        }
    };
    if routes.is_empty() {
        return Err(CapabilityManifestError::new(
            path,
            field_path,
            "at least one route must be declared",
        ));
    }
    Ok(CapabilityRouteCollection { routes })
}

fn parse_commands(
    raw: RawCapabilityCommandCollection,
    path: &Path,
    field_path: &str,
) -> Result<CapabilityCommandCollection, CapabilityManifestError> {
    let commands = match raw {
        RawCapabilityCommandCollection::Command(command) => {
            vec![normalize_non_empty_string(Some(command), path, field_path)?]
        }
        RawCapabilityCommandCollection::Commands(collection) => {
            normalize_list(collection.commands.unwrap_or_default(), path, field_path)?
        }
    };
    if commands.is_empty() {
        return Err(CapabilityManifestError::new(
            path,
            field_path,
            "at least one command must be declared",
        ));
    }
    Ok(CapabilityCommandCollection { commands })
}

fn parse_cli_surface(
    raw: RawCapabilityCliSurface,
    path: &Path,
    field_path: &str,
) -> Result<CapabilityCliSurface, CapabilityManifestError> {
    match raw {
        RawCapabilityCliSurface::Bool(enabled) => Ok(CapabilityCliSurface {
            enabled,
            commands: Vec::new(),
        }),
        RawCapabilityCliSurface::Object(surface) => {
            let commands = normalize_list(surface.commands.unwrap_or_default(), path, field_path)?;
            if matches!(surface.enabled, Some(false)) && !commands.is_empty() {
                return Err(CapabilityManifestError::new(
                    path,
                    format!("{field_path}.enabled"),
                    "enabled=false cannot carry CLI commands",
                ));
            }
            Ok(CapabilityCliSurface {
                enabled: surface.enabled.unwrap_or(!commands.is_empty()),
                commands,
            })
        }
    }
}

fn validate_manifest_id(path: &Path, value: &str) -> Result<(), CapabilityManifestError> {
    if !value.starts_with("vac.") {
        return Err(CapabilityManifestError::new(
            path,
            "id",
            "capability ids must start with `vac.`",
        ));
    }
    if value.chars().any(|character| {
        !(character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '.' | '_' | '-'))
    }) {
        return Err(CapabilityManifestError::new(
            path,
            "id",
            "capability ids may only contain lowercase ASCII letters, digits, `.`, `_`, and `-`",
        ));
    }
    Ok(())
}

fn normalize_manifest_id(
    value: Option<String>,
    path: &Path,
) -> Result<String, CapabilityManifestError> {
    let value = normalize_non_empty_string(value, path, "id")?;
    validate_manifest_id(path, &value)?;
    Ok(value)
}

fn parse_status(
    value: Option<String>,
    path: &Path,
) -> Result<CapabilityStatus, CapabilityManifestError> {
    let value = normalize_non_empty_string(value, path, "status")?;
    match value.as_str() {
        "planned" => Ok(CapabilityStatus::Planned),
        "partial" => Ok(CapabilityStatus::Partial),
        "ready" => Ok(CapabilityStatus::Ready),
        "deprecated" => Ok(CapabilityStatus::Deprecated),
        "disabled" => Ok(CapabilityStatus::Disabled),
        _ => Err(CapabilityManifestError::new(
            path,
            "status",
            "unknown status; expected planned, partial, ready, deprecated, or disabled",
        )),
    }
}

fn normalize_owner_path(
    value: String,
    path: &Path,
    field_path: &str,
) -> Result<String, CapabilityManifestError> {
    let value = normalize_non_empty_string(Some(value), path, field_path)?;
    if value.contains(char::is_whitespace) {
        return Err(CapabilityManifestError::new(
            path,
            field_path,
            "owner paths must not contain whitespace",
        ));
    }
    Ok(value)
}

fn normalize_non_empty_string(
    value: Option<String>,
    path: &Path,
    field_path: &str,
) -> Result<String, CapabilityManifestError> {
    let value = value
        .ok_or_else(|| CapabilityManifestError::new(path, field_path, "missing required field"))?;
    if value.trim().is_empty() {
        return Err(CapabilityManifestError::new(
            path,
            field_path,
            "value must not be empty",
        ));
    }
    Ok(value)
}

fn normalize_list(
    values: Vec<String>,
    path: &Path,
    field_path: &str,
) -> Result<Vec<String>, CapabilityManifestError> {
    let mut normalized = Vec::with_capacity(values.len());
    for (index, value) in values.into_iter().enumerate() {
        normalized.push(normalize_non_empty_string(
            Some(value),
            path,
            &format!("{field_path}[{index}]"),
        )?);
    }
    Ok(normalized)
}

fn normalize_string_list(
    values: Option<Vec<String>>,
    path: &Path,
    field_path: &str,
) -> Result<Vec<String>, CapabilityManifestError> {
    let values = values.unwrap_or_default();
    normalize_list(values, path, field_path)
}

fn normalize_validation_entries(
    values: Option<Vec<RawValidationEntry>>,
    path: &Path,
    field_path: &str,
) -> Result<Vec<String>, CapabilityManifestError> {
    let values = values.unwrap_or_default();
    let mut normalized = Vec::with_capacity(values.len());
    for (index, value) in values.into_iter().enumerate() {
        let item_path = format!("{field_path}[{index}]");
        let value = match value {
            RawValidationEntry::String(value) => {
                normalize_non_empty_string(Some(value), path, &item_path)?
            }
            RawValidationEntry::Object(command) => {
                normalize_validation_command(command, path, &item_path)?
            }
        };
        normalized.push(value);
    }
    Ok(normalized)
}

fn normalize_validation_command(
    command: RawValidationCommand,
    path: &Path,
    field_path: &str,
) -> Result<String, CapabilityManifestError> {
    if let Some(id) = command.id.as_deref() {
        validate_command_id(path, &format!("{field_path}.id"), id)?;
    }
    if let Some(risk) = command.risk.as_deref() {
        validate_command_id(path, &format!("{field_path}.risk"), risk)?;
    }
    if let Some(approval) = command.approval.as_deref() {
        validate_command_id(path, &format!("{field_path}.approval"), approval)?;
    }

    let runner = command
        .runner
        .map(|value| normalize_non_empty_string(Some(value), path, &format!("{field_path}.runner")))
        .transpose()?;
    let args = normalize_list(command.args, path, &format!("{field_path}.args"))?;

    if runner.is_none() && args.is_empty() {
        return Err(CapabilityManifestError::new(
            path,
            field_path,
            "structured validation command requires runner or args",
        ));
    }

    let mut parts = Vec::new();
    if let Some(runner) = runner {
        parts.push(runner);
    }
    parts.extend(args);
    Ok(parts.join(" "))
}

fn validate_command_id(
    path: &Path,
    field_path: &str,
    value: &str,
) -> Result<(), CapabilityManifestError> {
    normalize_non_empty_string(Some(value.to_string()), path, field_path)?;
    if value.chars().any(char::is_whitespace) {
        return Err(CapabilityManifestError::new(
            path,
            field_path,
            "value must not contain whitespace",
        ));
    }
    Ok(())
}

fn serde_path_to_string(path: &serde_path_to_error::Path) -> String {
    let mut segments = Vec::new();
    for segment in path.iter() {
        match segment {
            serde_path_to_error::Segment::Map { key } => segments.push(key.to_string()),
            serde_path_to_error::Segment::Seq { index } => segments.push(format!("[{index}]")),
            serde_path_to_error::Segment::Enum { variant } => segments.push(variant.to_string()),
            &serde_path_to_error::Segment::Unknown => segments.push("?".to_string()),
        }
    }
    if segments.is_empty() {
        "root".to_string()
    } else {
        segments.join(".")
    }
}

impl CapabilityPolicyValue {
    fn from_raw(raw: RawCapabilityPolicyValue) -> Result<Self, CapabilityManifestError> {
        Ok(match raw {
            RawCapabilityPolicyValue::Bool(value) => Self::Bool(value),
            RawCapabilityPolicyValue::Text(value) => Self::Text(value),
        })
    }
}

impl CapabilitySurfaces {
    fn has_tui_or_cli_surface(&self) -> bool {
        self.tui
            .as_ref()
            .map(|routes| !routes.routes.is_empty())
            .unwrap_or(false)
            || self
                .cli
                .as_ref()
                .map(|cli| cli.enabled && !cli.commands.is_empty())
                .unwrap_or(false)
    }

    fn has_visible_surface(&self) -> bool {
        self.palette
            || self
                .tui
                .as_ref()
                .map(|routes| !routes.routes.is_empty())
                .unwrap_or(false)
            || self
                .slash
                .as_ref()
                .map(|commands| !commands.commands.is_empty())
                .unwrap_or(false)
            || self.cli.as_ref().map(|cli| cli.enabled).unwrap_or(false)
    }
}

impl CapabilityPolicy {
    fn has_any_declaration(&self) -> bool {
        self.risk.is_some()
            || self.mutates_files.is_some()
            || self.network.is_some()
            || self.redaction.is_some()
            || !self.approval_required_for.is_empty()
    }
}

impl CapabilityValidation {
    fn has_any_evidence(&self) -> bool {
        !self.commands.is_empty() || !self.gates.is_empty()
    }
}

#[cfg(test)]
impl CapabilityManifest {
    /// Minimal in-memory stub for tests that only need a syntactically valid
    /// `CapabilityManifest` with a custom `id`. Status is `Deprecated` so the
    /// states/validation/reason validation rules do not apply. Callers can
    /// override any field via struct-update syntax (`..test_stub("...")`).
    pub(crate) fn test_stub(id: &str) -> Self {
        Self {
            schema_version: 1,
            kind: CapabilityManifestKind::Capability,
            id: id.to_string(),
            title: "test stub".to_string(),
            status: CapabilityStatus::Deprecated,
            owner: CapabilityOwner::Path("test_stub".to_string()),
            surfaces: CapabilitySurfaces {
                tui: None,
                slash: None,
                palette: true,
                cli: None,
            },
            policy: CapabilityPolicy {
                risk: Some("safe_read".to_string()),
                mutates_files: None,
                network: None,
                redaction: None,
                approval_required_for: Vec::new(),
            },
            validation: CapabilityValidation {
                commands: Vec::new(),
                gates: Vec::new(),
            },
            description: None,
            reason: None,
            depends_on: Vec::new(),
            optional_depends_on: Vec::new(),
            runtime_events: Vec::new(),
            states: None,
            ownership: None,
            donor_source: None,
            compatibility_transport: Vec::new(),
            docs: Vec::new(),
        }
    }
}

#[cfg(test)]
#[path = "capability_manifest_tests.rs"]
mod tests;
