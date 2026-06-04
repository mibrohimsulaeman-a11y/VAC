use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;
use vac_protocol::approvals::NetworkApprovalProtocol;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PolicyManifest {
    pub schema_version: u32,
    pub kind: PolicyManifestKind,
    pub id: String,
    pub title: String,
    pub default_decision: PolicyDecision,
    pub rules: Vec<PolicyRule>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum PolicyManifestKind {
    #[serde(rename = "policy")]
    Policy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecision {
    Allow,
    Deny,
    ApprovalRequired,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PolicyAction {
    FilesystemRead,
    FilesystemWrite,
    FilesystemDelete,
    ProcessExecute,
    NetworkAccess,
    ToolCall,
    CredentialRead,
    SessionWrite,
    CheckpointWrite,
}

impl PolicyAction {
    fn requires_path_scope(self) -> bool {
        matches!(
            self,
            PolicyAction::FilesystemRead
                | PolicyAction::FilesystemWrite
                | PolicyAction::FilesystemDelete
        )
    }

    fn requires_tool_scope(self) -> bool {
        matches!(self, PolicyAction::ToolCall)
    }

    fn requires_network_scope(self) -> bool {
        matches!(self, PolicyAction::NetworkAccess)
    }

    fn is_mutating_or_sensitive(self) -> bool {
        matches!(
            self,
            PolicyAction::FilesystemWrite
                | PolicyAction::FilesystemDelete
                | PolicyAction::ProcessExecute
                | PolicyAction::NetworkAccess
                | PolicyAction::ToolCall
                | PolicyAction::CredentialRead
                | PolicyAction::SessionWrite
                | PolicyAction::CheckpointWrite
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PolicyPathScope {
    Project,
    Workspace,
    Any,
}

impl PolicyPathScope {
    fn is_narrow(self) -> bool {
        matches!(self, PolicyPathScope::Project)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDataClass {
    SourceCode,
    ProjectDocs,
    Config,
    Logs,
    Diff,
    SecretLike,
    Credential,
    PersonalData,
    ConnectorKnowledge,
    ModelOutput,
    CommandOutput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PolicyNetworkScope {
    pub host: String,
    #[serde(default)]
    pub protocol: Option<NetworkApprovalProtocol>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PolicyMatch {
    #[serde(default)]
    pub action: Option<PolicyAction>,
    #[serde(default)]
    pub path: Option<PolicyPathScope>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub network: Option<PolicyNetworkScope>,
    #[serde(default)]
    pub data_class: Option<PolicyDataClass>,
}

impl PolicyMatch {
    fn is_empty(&self) -> bool {
        self.action.is_none()
            && self.path.is_none()
            && self.tool.is_none()
            && self.network.is_none()
            && self.data_class.is_none()
    }

    fn has_narrow_scope(&self) -> bool {
        self.path.is_some_and(PolicyPathScope::is_narrow)
            || self.tool.is_some()
            || self.network.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PolicyRule {
    pub id: String,
    #[serde(rename = "match")]
    pub match_: PolicyMatch,
    pub decision: PolicyDecision,
    pub reason: String,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{path}:{field_path}: {message}")]
pub struct PolicyManifestError {
    path: PathBuf,
    field_path: String,
    message: String,
}

impl PolicyManifestError {
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
struct RawPolicyManifest {
    schema_version: Option<u32>,
    kind: Option<String>,
    id: Option<String>,
    title: Option<String>,
    default_decision: Option<String>,
    rules: Option<Vec<RawPolicyRule>>,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    evidence: Option<serde_yaml::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPolicyRule {
    id: Option<String>,
    #[serde(rename = "match")]
    match_: Option<RawPolicyMatch>,
    decision: Option<String>,
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPolicyMatch {
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    tool: Option<String>,
    #[serde(default)]
    network: Option<RawPolicyNetworkScope>,
    #[serde(default)]
    data_class: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPolicyNetworkScope {
    host: Option<String>,
    #[serde(default)]
    protocol: Option<NetworkApprovalProtocol>,
}

pub fn load_policy_manifest(path: impl AsRef<Path>) -> Result<PolicyManifest, PolicyManifestError> {
    let path = path.as_ref();
    let contents = std::fs::read_to_string(path)
        .map_err(|err| PolicyManifestError::new(path, "root", err.to_string()))?;
    parse_policy_manifest(path, &contents)
}

pub fn parse_policy_manifest(
    path: impl AsRef<Path>,
    contents: &str,
) -> Result<PolicyManifest, PolicyManifestError> {
    let path = path.as_ref();
    let deserializer = serde_yaml::Deserializer::from_str(contents);
    let raw: RawPolicyManifest =
        serde_path_to_error::deserialize(deserializer).map_err(|error| {
            PolicyManifestError::new(
                path,
                serde_path_to_string(error.path()),
                error.into_inner().to_string(),
            )
        })?;
    PolicyManifest::from_raw(path, raw)
}

pub fn validate_policy_manifest(
    path: impl AsRef<Path>,
    manifest: &PolicyManifest,
) -> Result<(), PolicyManifestError> {
    validate_policy_manifest_internal(path.as_ref(), manifest)
}

impl PolicyManifest {
    fn from_raw(path: &Path, raw: RawPolicyManifest) -> Result<Self, PolicyManifestError> {
        let schema_version = raw.schema_version.ok_or_else(|| {
            PolicyManifestError::new(path, "schema_version", "missing required field")
        })?;
        if schema_version != 1 {
            return Err(PolicyManifestError::new(
                path,
                "schema_version",
                format!("unsupported schema version {schema_version}; expected 1"),
            ));
        }

        let kind = match raw.kind.as_deref() {
            Some("policy") => PolicyManifestKind::Policy,
            Some(other) => {
                return Err(PolicyManifestError::new(
                    path,
                    "kind",
                    format!("unknown manifest kind `{other}`; expected `policy`"),
                ));
            }
            None => {
                return Err(PolicyManifestError::new(
                    path,
                    "kind",
                    "missing required field",
                ));
            }
        };

        let id = normalize_policy_id(raw.id, path)?;
        let title = normalize_non_empty_string(raw.title, path, "title")?;
        let default_decision = parse_decision(raw.default_decision, path, "default_decision")?;
        let rules = parse_rules(raw.rules, path)?;
        let _metadata = (raw.owner, raw.status, raw.evidence);

        let manifest = Self {
            schema_version,
            kind,
            id,
            title,
            default_decision,
            rules,
        };
        validate_policy_manifest_internal(path, &manifest)?;
        Ok(manifest)
    }
}

fn validate_policy_manifest_internal(
    path: &Path,
    manifest: &PolicyManifest,
) -> Result<(), PolicyManifestError> {
    validate_policy_schema_version(path, manifest.schema_version)?;
    validate_policy_id(path, &manifest.id)?;
    validate_policy_title(path, &manifest.title)?;
    validate_policy_default_decision(path, manifest.default_decision)?;
    validate_policy_rules(path, &manifest.rules)?;
    Ok(())
}

fn validate_policy_schema_version(
    path: &Path,
    schema_version: u32,
) -> Result<(), PolicyManifestError> {
    if schema_version != 1 {
        return Err(PolicyManifestError::new(
            path,
            "schema_version",
            format!("unsupported schema version {schema_version}; expected 1"),
        ));
    }

    Ok(())
}

fn validate_policy_id(path: &Path, id: &str) -> Result<(), PolicyManifestError> {
    validate_non_empty(path, "id", id)?;
    ensure_no_whitespace(path, "id", id)?;
    if !id.starts_with("vac.") && !id.starts_with("product.") && !id.starts_with("maintenance.") {
        return Err(PolicyManifestError::new(
            path,
            "id",
            "policy ids must start with `vac.`, `product.`, or `maintenance.`",
        ));
    }
    Ok(())
}

fn validate_policy_title(path: &Path, title: &str) -> Result<(), PolicyManifestError> {
    validate_non_empty(path, "title", title)
}

fn validate_policy_default_decision(
    path: &Path,
    default_decision: PolicyDecision,
) -> Result<(), PolicyManifestError> {
    if default_decision == PolicyDecision::Allow {
        return Err(PolicyManifestError::new(
            path,
            "default_decision",
            "default_decision cannot be `allow`; use `approval_required` or `deny` as the fallback posture",
        ));
    }

    Ok(())
}

fn validate_policy_rules(path: &Path, rules: &[PolicyRule]) -> Result<(), PolicyManifestError> {
    let mut seen_ids = HashSet::new();
    for (index, rule) in rules.iter().enumerate() {
        let field_path = format!("rules[{index}]");
        validate_non_empty(path, &format!("{field_path}.id"), &rule.id)?;
        ensure_no_whitespace(path, &format!("{field_path}.id"), &rule.id)?;
        if !seen_ids.insert(rule.id.clone()) {
            return Err(PolicyManifestError::new(
                path,
                format!("{field_path}.id"),
                "rule ids must be unique",
            ));
        }

        validate_non_empty(path, &format!("{field_path}.reason"), &rule.reason)?;
        if rule.match_.is_empty() {
            return Err(PolicyManifestError::new(
                path,
                format!("{field_path}.match"),
                "policy rule match must declare at least one matcher",
            ));
        }
        validate_policy_match(path, &field_path, &rule.match_)?;

        if rule.decision == PolicyDecision::Allow {
            validate_policy_allow_rule(path, &field_path, &rule.match_)?;
        }
    }

    Ok(())
}

fn validate_policy_allow_rule(
    path: &Path,
    field_path: &str,
    match_: &PolicyMatch,
) -> Result<(), PolicyManifestError> {
    let Some(action) = match_.action else {
        return Err(PolicyManifestError::new(
            path,
            format!("{field_path}.decision"),
            "broad allow rules require an explicit action and scope",
        ));
    };

    if action.is_mutating_or_sensitive() {
        return Err(PolicyManifestError::new(
            path,
            format!("{field_path}.decision"),
            "unsafe broad allow rule requires explicit review",
        ));
    }

    if !match_.has_narrow_scope() {
        return Err(PolicyManifestError::new(
            path,
            format!("{field_path}.match"),
            "unsafe broad allow rule requires explicit review",
        ));
    }

    Ok(())
}

fn validate_policy_match(
    path: &Path,
    field_path: &str,
    match_: &PolicyMatch,
) -> Result<(), PolicyManifestError> {
    if let Some(action) = match_.action {
        if action.requires_path_scope() && match_.path.is_none() {
            return Err(PolicyManifestError::new(
                path,
                format!("{field_path}.match.path"),
                "filesystem policy matches require a path scope",
            ));
        }
        if action.requires_network_scope() && match_.network.is_none() {
            return Err(PolicyManifestError::new(
                path,
                format!("{field_path}.match.network"),
                "network policy matches require a network scope",
            ));
        }
        if action.requires_tool_scope() && match_.tool.is_none() {
            return Err(PolicyManifestError::new(
                path,
                format!("{field_path}.match.tool"),
                "tool policy matches require a tool scope",
            ));
        }
        if matches!(action, PolicyAction::CredentialRead) && match_.data_class.is_none() {
            return Err(PolicyManifestError::new(
                path,
                format!("{field_path}.match.data_class"),
                "credential policy matches require a data class scope",
            ));
        }
    }

    if let Some(network) = &match_.network {
        validate_non_empty(
            path,
            &format!("{field_path}.match.network.host"),
            &network.host,
        )?;
        ensure_no_whitespace(
            path,
            &format!("{field_path}.match.network.host"),
            &network.host,
        )?;
    }

    if let Some(tool) = &match_.tool {
        validate_non_empty(path, &format!("{field_path}.match.tool"), tool)?;
        ensure_no_whitespace(path, &format!("{field_path}.match.tool"), tool)?;
    }

    Ok(())
}

fn normalize_policy_id(raw: Option<String>, path: &Path) -> Result<String, PolicyManifestError> {
    let value = normalize_non_empty_string(raw, path, "id")?;
    if !value.starts_with("vac.")
        && !value.starts_with("product.")
        && !value.starts_with("maintenance.")
    {
        return Err(PolicyManifestError::new(
            path,
            "id",
            "policy ids must start with `vac.`, `product.`, or `maintenance.`",
        ));
    }
    Ok(value)
}

fn parse_rules(
    raw: Option<Vec<RawPolicyRule>>,
    path: &Path,
) -> Result<Vec<PolicyRule>, PolicyManifestError> {
    let raw =
        raw.ok_or_else(|| PolicyManifestError::new(path, "rules", "missing required field"))?;
    let mut rules = Vec::with_capacity(raw.len());
    for (index, raw_rule) in raw.into_iter().enumerate() {
        let field_path = format!("rules[{index}]");
        let id = normalize_non_empty_string(raw_rule.id, path, &format!("{field_path}.id"))?;
        ensure_no_whitespace(path, &format!("{field_path}.id"), &id)?;
        let match_ = raw_rule.match_.ok_or_else(|| {
            PolicyManifestError::new(
                path,
                format!("{field_path}.match"),
                "missing required field",
            )
        })?;
        let match_ = parse_rule_match(path, &field_path, match_)?;
        let decision = parse_decision(raw_rule.decision, path, &format!("{field_path}.decision"))?;
        let reason =
            normalize_non_empty_string(raw_rule.reason, path, &format!("{field_path}.reason"))?;

        rules.push(PolicyRule {
            id,
            match_,
            decision,
            reason,
        });
    }
    Ok(rules)
}

fn parse_rule_match(
    path: &Path,
    field_path: &str,
    raw: RawPolicyMatch,
) -> Result<PolicyMatch, PolicyManifestError> {
    let action = raw
        .action
        .map(|value| parse_policy_action(value, path, &format!("{field_path}.match.action")))
        .transpose()?;
    let path_scope = raw
        .path
        .map(|value| parse_policy_path_scope(value, path, &format!("{field_path}.match.path")))
        .transpose()?;
    let tool = raw
        .tool
        .map(|value| {
            normalize_non_empty_string(Some(value), path, &format!("{field_path}.match.tool"))
        })
        .transpose()?;
    let network = raw
        .network
        .map(|value| {
            parse_policy_network_scope(path, &format!("{field_path}.match.network"), value)
        })
        .transpose()?;
    let data_class = raw
        .data_class
        .map(|value| {
            parse_policy_data_class(value, path, &format!("{field_path}.match.data_class"))
        })
        .transpose()?;

    Ok(PolicyMatch {
        action,
        path: path_scope,
        tool,
        network,
        data_class,
    })
}

fn parse_policy_network_scope(
    path: &Path,
    field_path: &str,
    raw: RawPolicyNetworkScope,
) -> Result<PolicyNetworkScope, PolicyManifestError> {
    let host = normalize_non_empty_string(raw.host, path, &format!("{field_path}.host"))?;
    ensure_no_whitespace(path, &format!("{field_path}.host"), &host)?;
    Ok(PolicyNetworkScope {
        host,
        protocol: raw.protocol,
    })
}

fn parse_decision(
    raw: Option<String>,
    path: &Path,
    field_path: &str,
) -> Result<PolicyDecision, PolicyManifestError> {
    let raw =
        raw.ok_or_else(|| PolicyManifestError::new(path, field_path, "missing required field"))?;
    match raw.as_str() {
        "allow" => Ok(PolicyDecision::Allow),
        "deny" => Ok(PolicyDecision::Deny),
        "approval_required" => Ok(PolicyDecision::ApprovalRequired),
        "unavailable" => Ok(PolicyDecision::Unavailable),
        other => Err(PolicyManifestError::new(
            path,
            field_path,
            format!(
                "unknown policy decision `{other}`; expected `allow`, `deny`, `approval_required`, or `unavailable`"
            ),
        )),
    }
}

fn parse_policy_action(
    raw: String,
    path: &Path,
    field_path: &str,
) -> Result<PolicyAction, PolicyManifestError> {
    match raw.as_str() {
        "filesystem_read" => Ok(PolicyAction::FilesystemRead),
        "filesystem_write" => Ok(PolicyAction::FilesystemWrite),
        "filesystem_delete" => Ok(PolicyAction::FilesystemDelete),
        "process_execute" => Ok(PolicyAction::ProcessExecute),
        "network_access" => Ok(PolicyAction::NetworkAccess),
        "tool_call" => Ok(PolicyAction::ToolCall),
        "credential_read" => Ok(PolicyAction::CredentialRead),
        "session_write" => Ok(PolicyAction::SessionWrite),
        "checkpoint_write" => Ok(PolicyAction::CheckpointWrite),
        other => Err(PolicyManifestError::new(
            path,
            field_path,
            format!("unknown policy action `{other}`"),
        )),
    }
}

fn parse_policy_path_scope(
    raw: String,
    path: &Path,
    field_path: &str,
) -> Result<PolicyPathScope, PolicyManifestError> {
    match raw.as_str() {
        "project" => Ok(PolicyPathScope::Project),
        "workspace" => Ok(PolicyPathScope::Workspace),
        "any" => Ok(PolicyPathScope::Any),
        other => Err(PolicyManifestError::new(
            path,
            field_path,
            format!("unknown policy path scope `{other}`"),
        )),
    }
}

fn parse_policy_data_class(
    raw: String,
    path: &Path,
    field_path: &str,
) -> Result<PolicyDataClass, PolicyManifestError> {
    match raw.as_str() {
        "source_code" => Ok(PolicyDataClass::SourceCode),
        "project_docs" => Ok(PolicyDataClass::ProjectDocs),
        "config" => Ok(PolicyDataClass::Config),
        "logs" => Ok(PolicyDataClass::Logs),
        "diff" => Ok(PolicyDataClass::Diff),
        "secret_like" => Ok(PolicyDataClass::SecretLike),
        "credential" => Ok(PolicyDataClass::Credential),
        "personal_data" => Ok(PolicyDataClass::PersonalData),
        "connector_knowledge" => Ok(PolicyDataClass::ConnectorKnowledge),
        "model_output" => Ok(PolicyDataClass::ModelOutput),
        "command_output" => Ok(PolicyDataClass::CommandOutput),
        other => Err(PolicyManifestError::new(
            path,
            field_path,
            format!("unknown policy data class `{other}`"),
        )),
    }
}

fn normalize_non_empty_string(
    raw: Option<String>,
    path: &Path,
    field_path: &str,
) -> Result<String, PolicyManifestError> {
    let value =
        raw.ok_or_else(|| PolicyManifestError::new(path, field_path, "missing required field"))?;
    if value.trim().is_empty() {
        return Err(PolicyManifestError::new(
            path,
            field_path,
            "value must not be empty",
        ));
    }
    Ok(value)
}

fn validate_non_empty(
    path: &Path,
    field_path: &str,
    value: &str,
) -> Result<(), PolicyManifestError> {
    if value.trim().is_empty() {
        return Err(PolicyManifestError::new(
            path,
            field_path,
            "value must not be empty",
        ));
    }
    Ok(())
}

fn ensure_no_whitespace(
    path: &Path,
    field_path: &str,
    value: &str,
) -> Result<(), PolicyManifestError> {
    if value.chars().any(char::is_whitespace) {
        return Err(PolicyManifestError::new(
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

// ---- Policy evaluator (Phase 3) ----

/// Static description of what a workflow step does in policy terms.
/// Used to match against loaded policy manifests before step execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowStepExecutionIntent {
    pub action: PolicyAction,
    pub path_scope: Option<PolicyPathScope>,
    pub data_class: Option<PolicyDataClass>,
    pub tool: Option<String>,
    pub network_scope: Option<PolicyNetworkScope>,
    pub requires_approval_inherently: bool,
    pub step_uses: String,
    pub capability_id: Option<String>,
}

/// Result of evaluating a step intent against the loaded policy manifests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecisionReport {
    Allow { matching_rule_ids: Vec<String> },
    RequireApproval { reasons: Vec<String> },
    Block { reasons: Vec<String> },
    UnknownPolicy { reasons: Vec<String> },
}

impl PolicyDecisionReport {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow { .. })
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Block { .. })
    }

    pub fn requires_approval(&self) -> bool {
        matches!(
            self,
            Self::RequireApproval { .. } | Self::UnknownPolicy { .. }
        )
    }

    pub fn summary(&self) -> &'static str {
        match self {
            Self::Allow { .. } => "allow",
            Self::RequireApproval { .. } => "approval_required",
            Self::Block { .. } => "block",
            Self::UnknownPolicy { .. } => "unknown_policy",
        }
    }

    pub fn reasons(&self) -> &[String] {
        match self {
            Self::Allow { matching_rule_ids } => matching_rule_ids.as_slice(),
            Self::RequireApproval { reasons }
            | Self::Block { reasons }
            | Self::UnknownPolicy { reasons } => reasons.as_slice(),
        }
    }
}

fn decision_label(decision: PolicyDecision) -> &'static str {
    match decision {
        PolicyDecision::Allow => "allow",
        PolicyDecision::Deny => "deny",
        PolicyDecision::ApprovalRequired => "approval_required",
        PolicyDecision::Unavailable => "unavailable",
    }
}

fn action_label(action: PolicyAction) -> &'static str {
    match action {
        PolicyAction::FilesystemRead => "filesystem_read",
        PolicyAction::FilesystemWrite => "filesystem_write",
        PolicyAction::FilesystemDelete => "filesystem_delete",
        PolicyAction::ProcessExecute => "process_execute",
        PolicyAction::NetworkAccess => "network_access",
        PolicyAction::ToolCall => "tool_call",
        PolicyAction::CredentialRead => "credential_read",
        PolicyAction::SessionWrite => "session_write",
        PolicyAction::CheckpointWrite => "checkpoint_write",
    }
}

fn rule_matches_intent(rule: &PolicyRule, intent: &WorkflowStepExecutionIntent) -> bool {
    let m = &rule.match_;
    if let Some(action) = m.action {
        if action != intent.action {
            return false;
        }
    }
    if let Some(path_scope) = m.path {
        match intent.path_scope {
            None => return false,
            Some(intent_path) => {
                if !path_scope_covers(path_scope, intent_path) {
                    return false;
                }
            }
        }
    }
    if let Some(rule_data_class) = m.data_class {
        match intent.data_class {
            None => return false,
            Some(intent_data_class) => {
                if rule_data_class != intent_data_class {
                    return false;
                }
            }
        }
    }
    if let Some(ref rule_tool) = m.tool {
        let matches_tool = intent.tool.as_deref() == Some(rule_tool.as_str())
            || intent.step_uses == *rule_tool
            || intent.capability_id.as_deref() == Some(rule_tool.as_str());
        if !matches_tool {
            return false;
        }
    }
    if let Some(ref rule_network) = m.network {
        match &intent.network_scope {
            None => return false,
            Some(intent_network) => {
                if !network_scope_covers(rule_network, intent_network) {
                    return false;
                }
            }
        }
    }
    true
}

fn network_scope_covers(
    rule_scope: &PolicyNetworkScope,
    intent_scope: &PolicyNetworkScope,
) -> bool {
    (rule_scope.host == "*" || rule_scope.host == intent_scope.host)
        && rule_scope
            .protocol
            .is_none_or(|protocol| intent_scope.protocol == Some(protocol))
}

fn path_scope_covers(rule_scope: PolicyPathScope, intent_scope: PolicyPathScope) -> bool {
    match rule_scope {
        PolicyPathScope::Any => true,
        PolicyPathScope::Workspace => matches!(
            intent_scope,
            PolicyPathScope::Workspace | PolicyPathScope::Project
        ),
        PolicyPathScope::Project => matches!(intent_scope, PolicyPathScope::Project),
    }
}

/// Evaluate a step execution intent against a set of policy manifests.
///
/// # Decision precedence
///
/// `Deny > ApprovalRequired > Allow`. Each manifest contributes exactly one
/// decision: its first matching rule, or its own `default_decision` when no
/// rule matches. Across manifests, any `Deny` blocks; otherwise any
/// `ApprovalRequired` (or `Unavailable`) gates; otherwise `Allow` wins.
///
/// # Empty manifests
///
/// When `manifests.is_empty()`, every step short-circuits to
/// `UnknownPolicy`. This is treated as approval-required by
/// `PolicyDecisionReport::requires_approval()`, giving an effective
/// default-deny posture before any policy YAML has been authored.
///
/// # Per-manifest default decision
///
/// Each manifest must declare a top-level `default_decision`. When a manifest
/// does not match the intent via any rule, that manifest's own
/// `default_decision` is contributed even if a different manifest did match.
/// Operators choose policy strictness per manifest by setting
/// `default_decision: deny` or `approval_required`; `allow` is rejected as a
/// top-level fallback posture.
///
/// # Fall-through (defensive)
///
/// The trailing branch returning `UnknownPolicy` for
/// `requires_approval_inherently`/mutating actions is defensive: since the
/// loop always contributes to at least one decision bucket when manifests
/// are non-empty, and the empty case is short-circuited above, this branch
/// is unreachable today. It exists as a guard against future refactors
/// that might skip a manifest contribution.
pub fn evaluate_step_policy(
    intent: &WorkflowStepExecutionIntent,
    manifests: &[PolicyManifest],
) -> PolicyDecisionReport {
    if manifests.is_empty() {
        return PolicyDecisionReport::UnknownPolicy {
            reasons: vec![format!(
                "no policy manifests loaded; step `{}` has unknown policy coverage",
                intent.step_uses
            )],
        };
    }

    let mut deny_reasons: Vec<String> = Vec::new();
    let mut approval_reasons: Vec<String> = Vec::new();
    let mut allow_ids: Vec<String> = Vec::new();

    let mut decisions: Vec<(PolicyDecision, String)> = Vec::new();
    for manifest in manifests {
        if let Some(rule) = manifest
            .rules
            .iter()
            .find(|r| rule_matches_intent(r, intent))
        {
            decisions.push((
                rule.decision,
                format!("[{}] {}: {}", manifest.id, rule.id, rule.reason),
            ));
        } else {
            decisions.push((
                manifest.default_decision,
                format!(
                    "[{}] default: {}",
                    manifest.id,
                    decision_label(manifest.default_decision)
                ),
            ));
        }
    }

    for (decision, reason) in decisions {
        match decision {
            PolicyDecision::Deny => deny_reasons.push(reason),
            PolicyDecision::ApprovalRequired => approval_reasons.push(reason),
            PolicyDecision::Allow => allow_ids.push(reason),
            PolicyDecision::Unavailable => {
                approval_reasons.push(format!("{reason} (unavailable)"));
            }
        }
    }

    if !deny_reasons.is_empty() {
        return PolicyDecisionReport::Block {
            reasons: deny_reasons,
        };
    }
    if !approval_reasons.is_empty() {
        return PolicyDecisionReport::RequireApproval {
            reasons: approval_reasons,
        };
    }
    if !allow_ids.is_empty() {
        return PolicyDecisionReport::Allow {
            matching_rule_ids: allow_ids,
        };
    }

    if intent.requires_approval_inherently || intent.action.is_mutating_or_sensitive() {
        PolicyDecisionReport::UnknownPolicy {
            reasons: vec![format!(
                "step `{}` performs sensitive action `{}` but no manifest has a matching rule",
                intent.step_uses,
                action_label(intent.action),
            )],
        }
    } else {
        PolicyDecisionReport::Allow {
            matching_rule_ids: Vec::new(),
        }
    }
}

// ---- Policy doctor report (Phase 4) ----

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDoctorManifestEntry {
    pub id: String,
    pub title: String,
    pub default_decision: PolicyDecision,
    pub rule_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDoctorReport {
    pub manifest_count: usize,
    pub rule_count: usize,
    pub allow_rule_count: usize,
    pub deny_rule_count: usize,
    pub approval_rule_count: usize,
    pub manifests: Vec<PolicyDoctorManifestEntry>,
    /// Errors encountered while loading policy manifests. Populated by
    /// `load_policy_doctor_report_for_path` when the policy registry fails
    /// to parse. `is_failure()` returns true when this is non-empty so
    /// CLI/TUI surfaces can exit non-zero on policy failures.
    pub load_errors: Vec<String>,
}

impl PolicyDoctorReport {
    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!(
            "policy: manifests={} rules={} allow={} approval_required={} deny={}",
            self.manifest_count,
            self.rule_count,
            self.allow_rule_count,
            self.approval_rule_count,
            self.deny_rule_count,
        ));
        for error in &self.load_errors {
            lines.push(format!("  error: {error}"));
        }
        if self.manifests.is_empty() {
            lines.push("  (no policy manifests loaded)".to_string());
        } else {
            for entry in &self.manifests {
                lines.push(format!(
                    "  {} -- {} (default: {}, rules: {})",
                    entry.id,
                    entry.title,
                    decision_label(entry.default_decision),
                    entry.rule_count,
                ));
            }
        }
        lines
    }

    pub fn render_text(&self) -> String {
        self.render_lines().join("\n")
    }

    /// TUI-surface rendering. Phase 4 Plan 04 step 5: expose policy metadata
    /// to status/capability rows. Mirrors `RegistryLoadReport::render_tui_lines`
    /// so capability dashboard wiring can iterate this report uniformly.
    pub fn render_tui_lines(&self) -> Vec<String> {
        self.render_lines()
    }

    pub fn render_tui_text(&self) -> String {
        self.render_text()
    }

    pub fn is_empty(&self) -> bool {
        self.manifest_count == 0
    }

    /// Returns true when policy manifest loading produced one or more errors.
    /// Use this in CLI/automation gates to exit non-zero on malformed policy
    /// YAML, matching the behavior of `RegistryLoadReport::is_failure()`.
    pub fn is_failure(&self) -> bool {
        !self.load_errors.is_empty()
    }
}

pub fn load_policy_doctor_report(manifests: &[PolicyManifest]) -> PolicyDoctorReport {
    let mut rule_count = 0;
    let mut allow_rule_count = 0;
    let mut deny_rule_count = 0;
    let mut approval_rule_count = 0;
    let mut entries = Vec::new();

    for manifest in manifests {
        rule_count += manifest.rules.len();
        for rule in &manifest.rules {
            match rule.decision {
                PolicyDecision::Allow => allow_rule_count += 1,
                PolicyDecision::Deny => deny_rule_count += 1,
                PolicyDecision::ApprovalRequired => approval_rule_count += 1,
                PolicyDecision::Unavailable => {}
            }
        }
        entries.push(PolicyDoctorManifestEntry {
            id: manifest.id.clone(),
            title: manifest.title.clone(),
            default_decision: manifest.default_decision,
            rule_count: manifest.rules.len(),
        });
    }

    PolicyDoctorReport {
        manifest_count: manifests.len(),
        rule_count,
        allow_rule_count,
        deny_rule_count,
        approval_rule_count,
        manifests: entries,
        load_errors: Vec::new(),
    }
}
#[cfg(test)]
#[path = "policy_manifest_tests.rs"]
mod tests;
