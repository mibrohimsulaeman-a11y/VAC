use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;

use super::workflow_runner::WORKFLOW_STEP_VOCABULARY;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowManifest {
    pub schema_version: u32,
    pub kind: WorkflowManifestKind,
    pub id: String,
    pub title: String,
    pub status: WorkflowStatus,
    pub inputs: BTreeMap<String, WorkflowInputSpec>,
    pub steps: Vec<WorkflowStep>,
    pub ui: WorkflowUi,
    pub policy: WorkflowPolicy,
    pub validation: WorkflowValidation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum WorkflowManifestKind {
    #[serde(rename = "workflow")]
    Workflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    Planned,
    Partial,
    Ready,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowInputKind {
    String,
    Enum,
    Boolean,
    Integer,
    Number,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum WorkflowInputDefault {
    String(String),
    Boolean(bool),
    Integer(i64),
    Number(f64),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowInputSpec {
    #[serde(rename = "type")]
    pub kind: WorkflowInputKind,
    pub required: bool,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default)]
    pub default: Option<WorkflowInputDefault>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowUi {
    pub surface: String,
    #[serde(default)]
    pub inspect_surface: Option<String>,
    pub progress_panel: bool,
    pub activity_log: bool,
    #[serde(default)]
    pub approval_surface: bool,
    #[serde(default)]
    pub evidence_surface: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum WorkflowPolicyValue {
    Bool(bool),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPolicy {
    #[serde(default)]
    pub default_risk: Option<String>,
    #[serde(default)]
    pub mutates_files: Option<WorkflowPolicyValue>,
    #[serde(default)]
    pub network: Option<WorkflowPolicyValue>,
    #[serde(default)]
    pub redaction: Option<WorkflowPolicyValue>,
    #[serde(default)]
    pub approval_required_for: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowValidation {
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub gates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowStep {
    pub id: String,
    pub uses: String,
    #[serde(default)]
    pub when: Option<String>,
    #[serde(default)]
    pub policy: Option<WorkflowStepPolicy>,
    #[serde(default)]
    pub ui: Option<WorkflowStepUi>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowStepPolicy {
    #[serde(default)]
    pub default_risk: Option<String>,
    #[serde(default)]
    pub mutates_files: Option<WorkflowPolicyValue>,
    #[serde(default)]
    pub network: Option<WorkflowPolicyValue>,
    #[serde(default)]
    pub redaction: Option<WorkflowPolicyValue>,
    #[serde(default)]
    pub approval_required_for: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowStepUi {
    #[serde(default)]
    pub surface: Option<String>,
    #[serde(default)]
    pub inspect_surface: Option<String>,
    #[serde(default)]
    pub progress_panel: Option<bool>,
    #[serde(default)]
    pub activity_log: Option<bool>,
    #[serde(default)]
    pub approval_surface: Option<bool>,
    #[serde(default)]
    pub evidence_surface: Option<bool>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{path}:{field_path}: {message}")]
pub struct WorkflowManifestError {
    path: PathBuf,
    field_path: String,
    message: String,
}

impl WorkflowManifestError {
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

pub fn load_workflow_manifest(
    path: impl AsRef<Path>,
) -> Result<WorkflowManifest, WorkflowManifestError> {
    let path = path.as_ref();
    let contents = std::fs::read_to_string(path)
        .map_err(|err| WorkflowManifestError::new(path, "root", err.to_string()))?;
    parse_workflow_manifest(path, &contents)
}

pub fn parse_workflow_manifest(
    path: impl AsRef<Path>,
    contents: &str,
) -> Result<WorkflowManifest, WorkflowManifestError> {
    let path = path.as_ref();
    let deserializer = serde_yaml::Deserializer::from_str(contents);
    let manifest: WorkflowManifest =
        serde_path_to_error::deserialize(deserializer).map_err(|error| {
            WorkflowManifestError::new(
                path,
                serde_path_to_string(error.path()),
                error.into_inner().to_string(),
            )
        })?;
    validate_workflow_manifest(path, &manifest)?;
    Ok(manifest)
}

pub fn validate_workflow_manifest(
    path: impl AsRef<Path>,
    manifest: &WorkflowManifest,
) -> Result<(), WorkflowManifestError> {
    validate_workflow_manifest_internal(path.as_ref(), manifest, None)
}

/// Validate a workflow manifest against a registry-provided set of known
/// capability ids. The validator implicitly unions the caller-provided set
/// with the built-in `WORKFLOW_STEP_VOCABULARY` (imported from
/// `workflow_runner`) so vocabulary entries are always treated as known.
pub fn validate_workflow_manifest_against_known_capabilities(
    path: impl AsRef<Path>,
    manifest: &WorkflowManifest,
    known_capabilities: &HashSet<String>,
) -> Result<(), WorkflowManifestError> {
    validate_workflow_manifest_internal(path.as_ref(), manifest, Some(known_capabilities))
}

pub(crate) fn workflow_step_use_resolves(uses: &str, known_capabilities: &HashSet<String>) -> bool {
    WORKFLOW_STEP_VOCABULARY
        .iter()
        .any(|entry| entry.uses == uses)
        || known_capabilities.contains(uses)
        || uses
            .strip_prefix("capability.")
            .map(|suffix| known_capabilities.contains(&format!("vac.{suffix}")))
            .unwrap_or(false)
}

fn validate_workflow_manifest_internal(
    path: &Path,
    manifest: &WorkflowManifest,
    known_capabilities: Option<&HashSet<String>>,
) -> Result<(), WorkflowManifestError> {
    validate_workflow_schema_version(path, manifest.schema_version)?;
    validate_workflow_id(path, &manifest.id)?;
    validate_workflow_title(path, &manifest.title)?;
    validate_workflow_inputs(path, &manifest.inputs)?;
    validate_workflow_steps(path, &manifest.steps, known_capabilities)?;
    validate_workflow_ui(path, &manifest.ui)?;
    validate_workflow_policy(path, &manifest.policy)?;
    validate_workflow_validation(path, &manifest.validation, manifest.status, &manifest.steps)?;
    Ok(())
}

fn validate_workflow_schema_version(
    path: &Path,
    schema_version: u32,
) -> Result<(), WorkflowManifestError> {
    if schema_version != 1 {
        return Err(WorkflowManifestError::new(
            path,
            "schema_version",
            format!("unsupported schema version {schema_version}; expected 1"),
        ));
    }

    Ok(())
}

fn validate_workflow_id(path: &Path, id: &str) -> Result<(), WorkflowManifestError> {
    validate_non_empty(path, "id", id)?;
    if !id.starts_with("product.") && !id.starts_with("maintenance.") && !id.starts_with("vac.") {
        return Err(WorkflowManifestError::new(
            path,
            "id",
            "workflow ids must start with `product.`, `maintenance.`, or `vac.`",
        ));
    }
    ensure_no_whitespace(path, "id", id)?;
    Ok(())
}

fn validate_workflow_title(path: &Path, title: &str) -> Result<(), WorkflowManifestError> {
    validate_non_empty(path, "title", title)?;
    Ok(())
}

fn validate_workflow_inputs(
    path: &Path,
    inputs: &BTreeMap<String, WorkflowInputSpec>,
) -> Result<(), WorkflowManifestError> {
    for (name, spec) in inputs {
        let field_path = format!("inputs.{name}");
        validate_non_empty(path, &field_path, name)?;
        ensure_no_whitespace(path, &field_path, name)?;
        validate_non_empty(path, &format!("{field_path}.type"), spec.kind.as_str())?;
        match spec.kind {
            WorkflowInputKind::Enum => {
                if spec.values.is_empty() {
                    return Err(WorkflowManifestError::new(
                        path,
                        format!("{field_path}.values"),
                        "enum inputs must declare at least one allowed value",
                    ));
                }
            }
            _ if !spec.values.is_empty() => {
                return Err(WorkflowManifestError::new(
                    path,
                    format!("{field_path}.values"),
                    "only enum inputs may declare allowed values",
                ));
            }
            _ => {}
        }

        if let Some(default) = &spec.default {
            validate_input_default(path, &field_path, spec.kind, default, &spec.values)?;
        }
    }
    Ok(())
}

fn validate_input_default(
    path: &Path,
    field_path: &str,
    kind: WorkflowInputKind,
    default: &WorkflowInputDefault,
    enum_values: &[String],
) -> Result<(), WorkflowManifestError> {
    match (kind, default) {
        (WorkflowInputKind::String, WorkflowInputDefault::String(_))
        | (WorkflowInputKind::Boolean, WorkflowInputDefault::Boolean(_))
        | (WorkflowInputKind::Integer, WorkflowInputDefault::Integer(_))
        | (WorkflowInputKind::Number, WorkflowInputDefault::Number(_)) => Ok(()),
        (WorkflowInputKind::Enum, WorkflowInputDefault::String(value)) => {
            if enum_values.iter().any(|candidate| candidate == value) {
                Ok(())
            } else {
                Err(WorkflowManifestError::new(
                    path,
                    format!("{field_path}.default"),
                    "enum default must be one of the declared values",
                ))
            }
        }
        _ => Err(WorkflowManifestError::new(
            path,
            format!("{field_path}.default"),
            "default value type does not match input type",
        )),
    }
}

fn validate_workflow_steps(
    path: &Path,
    steps: &[WorkflowStep],
    known_capabilities: Option<&HashSet<String>>,
) -> Result<(), WorkflowManifestError> {
    if steps.is_empty() {
        return Err(WorkflowManifestError::new(
            path,
            "steps",
            "workflow has no steps",
        ));
    }

    let mut seen_ids = HashSet::new();
    for (index, step) in steps.iter().enumerate() {
        let field_path = format!("steps[{index}]");
        validate_non_empty(path, &format!("{field_path}.id"), &step.id)?;
        ensure_no_whitespace(path, &format!("{field_path}.id"), &step.id)?;
        validate_step_identifier(path, &format!("{field_path}.id"), &step.id)?;
        validate_non_empty(path, &format!("{field_path}.uses"), &step.uses)?;
        ensure_no_whitespace(path, &format!("{field_path}.uses"), &step.uses)?;
        if !step.uses.starts_with("capability.") && !step.uses.starts_with("vac.") {
            return Err(WorkflowManifestError::new(
                path,
                format!("{field_path}.uses"),
                "step uses must start with `capability.` or `vac.`",
            ));
        }
        if !seen_ids.insert(step.id.clone()) {
            return Err(WorkflowManifestError::new(
                path,
                format!("{field_path}.id"),
                "step ids must be unique",
            ));
        }

        if let Some(when) = &step.when {
            validate_condition_expression(path, &format!("{field_path}.when"), when)?;
        }

        if let Some(policy) = &step.policy {
            validate_step_policy(path, &format!("{field_path}.policy"), policy)?;
        }

        if let Some(ui) = &step.ui {
            validate_step_ui(path, &format!("{field_path}.ui"), ui)?;
        }

        if let Some(known_capabilities) = known_capabilities {
            if !workflow_step_use_resolves(&step.uses, known_capabilities) {
                return Err(WorkflowManifestError::new(
                    path,
                    format!("{field_path}.uses"),
                    "step references an unknown capability",
                ));
            }
        }
    }
    Ok(())
}

fn validate_step_policy(
    path: &Path,
    field_path: &str,
    policy: &WorkflowStepPolicy,
) -> Result<(), WorkflowManifestError> {
    if policy.default_risk.is_none()
        && policy.mutates_files.is_none()
        && policy.network.is_none()
        && policy.redaction.is_none()
        && policy.approval_required_for.is_empty()
    {
        return Err(WorkflowManifestError::new(
            path,
            field_path,
            "step policy must declare at least one decision",
        ));
    }
    Ok(())
}

fn validate_step_ui(
    path: &Path,
    field_path: &str,
    ui: &WorkflowStepUi,
) -> Result<(), WorkflowManifestError> {
    if let Some(surface) = &ui.surface {
        validate_surface_path(path, &format!("{field_path}.surface"), surface)?;
    }
    if let Some(surface) = &ui.inspect_surface {
        validate_surface_path(path, &format!("{field_path}.inspect_surface"), surface)?;
    }
    if ui.surface.is_none()
        && ui.inspect_surface.is_none()
        && ui.progress_panel.is_none()
        && ui.activity_log.is_none()
        && ui.approval_surface.is_none()
        && ui.evidence_surface.is_none()
    {
        return Err(WorkflowManifestError::new(
            path,
            field_path,
            "step ui must declare at least one projection hint",
        ));
    }
    Ok(())
}

fn validate_workflow_ui(path: &Path, ui: &WorkflowUi) -> Result<(), WorkflowManifestError> {
    validate_surface_path(path, "ui.surface", &ui.surface)?;
    if let Some(surface) = &ui.inspect_surface {
        validate_surface_path(path, "ui.inspect_surface", surface)?;
    }
    Ok(())
}

fn validate_workflow_policy(
    path: &Path,
    policy: &WorkflowPolicy,
) -> Result<(), WorkflowManifestError> {
    if policy.default_risk.is_none()
        && policy.mutates_files.is_none()
        && policy.network.is_none()
        && policy.redaction.is_none()
        && policy.approval_required_for.is_empty()
    {
        return Err(WorkflowManifestError::new(
            path,
            "policy",
            "workflow policy must declare at least one decision",
        ));
    }
    Ok(())
}

fn validate_workflow_validation(
    path: &Path,
    validation: &WorkflowValidation,
    status: WorkflowStatus,
    steps: &[WorkflowStep],
) -> Result<(), WorkflowManifestError> {
    if validation.commands.is_empty() && validation.gates.is_empty() {
        if matches!(status, WorkflowStatus::Ready) {
            return Err(WorkflowManifestError::new(
                path,
                "validation",
                "ready workflow lacks validation gate",
            ));
        }
    }

    let step_ids = steps
        .iter()
        .map(|step| step.id.as_str())
        .collect::<HashSet<_>>();
    for (index, gate) in validation.gates.iter().enumerate() {
        validate_non_empty(path, &format!("validation.gates[{index}]"), gate)?;
        ensure_no_whitespace(path, &format!("validation.gates[{index}]"), gate)?;
        validate_step_identifier(path, &format!("validation.gates[{index}]"), gate)?;
        if !step_ids.contains(gate.as_str()) {
            return Err(WorkflowManifestError::new(
                path,
                format!("validation.gates[{index}]"),
                "validation gate must reference an existing step id",
            ));
        }
    }
    Ok(())
}

fn validate_step_identifier(
    path: &Path,
    field_path: &str,
    value: &str,
) -> Result<(), WorkflowManifestError> {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() => {}
        _ => {
            return Err(WorkflowManifestError::new(
                path,
                field_path,
                "step ids and validation gates must start with a lowercase ascii letter",
            ));
        }
    }
    if !chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-') {
        return Err(WorkflowManifestError::new(
            path,
            field_path,
            "step ids and validation gates may contain only lowercase ascii letters, digits, `_`, or `-`",
        ));
    }
    Ok(())
}

fn validate_condition_expression(
    path: &Path,
    field_path: &str,
    expression: &str,
) -> Result<(), WorkflowManifestError> {
    let mut parser = ConditionParser::new(expression).map_err(|message| {
        WorkflowManifestError::new(path, field_path, format!("invalid condition: {message}"))
    })?;
    parser.parse_expression().map_err(|message| {
        WorkflowManifestError::new(path, field_path, format!("invalid condition: {message}"))
    })?;
    parser.ensure_eof().map_err(|message| {
        WorkflowManifestError::new(path, field_path, format!("invalid condition: {message}"))
    })?;
    Ok(())
}

fn validate_surface_path(
    path: &Path,
    field_path: &str,
    value: &str,
) -> Result<(), WorkflowManifestError> {
    validate_non_empty(path, field_path, value)?;
    if !value.starts_with('/') {
        return Err(WorkflowManifestError::new(
            path,
            field_path,
            "surface routes must start with `/`",
        ));
    }
    Ok(())
}

fn validate_non_empty(
    path: &Path,
    field_path: &str,
    value: &str,
) -> Result<(), WorkflowManifestError> {
    if value.trim().is_empty() {
        return Err(WorkflowManifestError::new(
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
) -> Result<(), WorkflowManifestError> {
    if value.chars().any(char::is_whitespace) {
        return Err(WorkflowManifestError::new(
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

impl WorkflowInputKind {
    fn as_str(self) -> &'static str {
        match self {
            WorkflowInputKind::String => "string",
            WorkflowInputKind::Enum => "enum",
            WorkflowInputKind::Boolean => "boolean",
            WorkflowInputKind::Integer => "integer",
            WorkflowInputKind::Number => "number",
        }
    }
}

// Strict condition expression parser (Plan 03).
//
//   expression := or_expr
//   or_expr    := and_expr ('or' and_expr)*
//   and_expr   := unary ('and' unary)*
//   unary      := 'not' unary | primary
//   primary    := identifier | '(' expression ')'
//
// Token set: identifier, '(', ')', and 'and'/'or'/'not' keywords. No symbolic
// boolean operators, no comparisons, no literals.

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConditionToken {
    Identifier(String),
    LParen,
    RParen,
    And,
    Or,
    Not,
    Eof,
}

struct ConditionParser<'a> {
    input: &'a str,
    index: usize,
    current: ConditionToken,
}

impl<'a> ConditionParser<'a> {
    fn new(input: &'a str) -> Result<Self, &'static str> {
        let mut parser = Self {
            input,
            index: 0,
            current: ConditionToken::Eof,
        };
        parser.current = parser.next_token()?;
        Ok(parser)
    }

    fn parse_expression(&mut self) -> Result<(), &'static str> {
        self.parse_or()
    }

    fn ensure_eof(&self) -> Result<(), &'static str> {
        if matches!(self.current, ConditionToken::Eof) {
            Ok(())
        } else {
            Err("unexpected trailing input")
        }
    }

    fn parse_or(&mut self) -> Result<(), &'static str> {
        self.parse_and()?;
        while matches!(self.current, ConditionToken::Or) {
            self.bump()?;
            self.parse_and()?;
        }
        Ok(())
    }

    fn parse_and(&mut self) -> Result<(), &'static str> {
        self.parse_unary()?;
        while matches!(self.current, ConditionToken::And) {
            self.bump()?;
            self.parse_unary()?;
        }
        Ok(())
    }

    fn parse_unary(&mut self) -> Result<(), &'static str> {
        if matches!(self.current, ConditionToken::Not) {
            self.bump()?;
            self.parse_unary()
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> Result<(), &'static str> {
        match &self.current {
            ConditionToken::Identifier(_) => {
                self.bump()?;
                Ok(())
            }
            ConditionToken::LParen => {
                self.bump()?;
                self.parse_expression()?;
                if !matches!(self.current, ConditionToken::RParen) {
                    return Err("missing closing `)`");
                }
                self.bump()?;
                Ok(())
            }
            ConditionToken::Eof => Err("unexpected end of condition"),
            ConditionToken::RParen => Err("unexpected `)`"),
            ConditionToken::And | ConditionToken::Or | ConditionToken::Not => {
                Err("expected identifier or `(`")
            }
        }
    }

    fn bump(&mut self) -> Result<(), &'static str> {
        self.current = self.next_token()?;
        Ok(())
    }

    fn next_token(&mut self) -> Result<ConditionToken, &'static str> {
        self.skip_whitespace();
        let rest = &self.input[self.index..];
        let Some(first) = rest.chars().next() else {
            return Ok(ConditionToken::Eof);
        };
        match first {
            '(' => {
                self.index += 1;
                Ok(ConditionToken::LParen)
            }
            ')' => {
                self.index += 1;
                Ok(ConditionToken::RParen)
            }
            c if is_identifier_start(c) => Ok(self.read_identifier()),
            _ => Err("unexpected character in condition"),
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.input[self.index..].chars().next() {
            if ch.is_whitespace() {
                self.index += ch.len_utf8();
            } else {
                break;
            }
        }
    }

    fn read_identifier(&mut self) -> ConditionToken {
        let start = self.index;
        while let Some(ch) = self.input[self.index..].chars().next() {
            if is_identifier_continue(ch) {
                self.index += ch.len_utf8();
            } else {
                break;
            }
        }
        let value = &self.input[start..self.index];
        match value {
            "and" => ConditionToken::And,
            "or" => ConditionToken::Or,
            "not" => ConditionToken::Not,
            _ => ConditionToken::Identifier(value.to_string()),
        }
    }
}

fn is_identifier_start(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_'
}

fn is_identifier_continue(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
}

#[cfg(test)]
#[path = "workflow_manifest_tests.rs"]
mod tests;
