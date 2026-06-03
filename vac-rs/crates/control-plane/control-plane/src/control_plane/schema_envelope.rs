#![allow(dead_code)]
//! VAC schema envelope contract for manifest-first control-plane files.
//!
//! This module intentionally stays dependency-free so the schema-envelope
//! contract can be validated with direct `rustc --test` in constrained
//! sandboxes before full crate-level serde/YAML wiring is available.

use std::collections::BTreeMap;
use std::fmt;

#[cfg(vac_standalone_schema_envelope)]
#[path = "kind_registry.rs"]
mod kind_registry;

#[cfg(vac_standalone_schema_envelope)]
use kind_registry::KindRegistryError;
#[cfg(vac_standalone_schema_envelope)]
use kind_registry::VacManifestKind;
#[cfg(vac_standalone_schema_envelope)]
use kind_registry::validate_manifest_kind;

#[cfg(not(vac_standalone_schema_envelope))]
use super::kind_registry::KindRegistryError;
#[cfg(not(vac_standalone_schema_envelope))]
use super::kind_registry::VacManifestKind;
#[cfg(not(vac_standalone_schema_envelope))]
use super::kind_registry::validate_manifest_kind;

/// Current VAC control-plane schema version supported by the v1-alpha
/// manifest envelope. Future schema bumps should be handled through registry
/// migrations, not silently accepted by the v1 loader.
pub const SUPPORTED_SCHEMA_VERSION: u32 = 1;

/// Raw top-level envelope fields extracted from YAML before kind/id validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaEnvelopeFields {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
}

/// The minimal typed envelope that every `.vac/` manifest must expose at the
/// top level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaEnvelope {
    pub schema_version: u32,
    pub kind: VacManifestKind,
    pub id: String,
}

impl SchemaEnvelope {
    pub fn new(
        schema_version: u32,
        kind: impl AsRef<str>,
        id: impl Into<String>,
    ) -> Result<Self, SchemaEnvelopeError> {
        let envelope = Self {
            schema_version,
            kind: validate_manifest_kind(kind.as_ref()).map_err(SchemaEnvelopeError::Kind)?,
            id: id.into(),
        };
        validate_schema_envelope(&envelope)?;
        Ok(envelope)
    }

    pub fn is_schema_v1(&self) -> bool {
        self.schema_version == SUPPORTED_SCHEMA_VERSION
    }

    pub fn dotted_id_segments(&self) -> Vec<&str> {
        self.id.split('.').collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaEnvelopeError {
    MissingField(&'static str),
    DuplicateField(&'static str),
    InvalidSchemaVersion { found: u32, expected: u32 },
    InvalidSchemaVersionValue(String),
    EmptyField(&'static str),
    InvalidId(String),
    Kind(KindRegistryError),
    InvalidLine { line_number: usize, message: String },
}

impl fmt::Display for SchemaEnvelopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "missing required envelope field `{field}`"),
            Self::DuplicateField(field) => write!(f, "duplicate envelope field `{field}`"),
            Self::InvalidSchemaVersion { found, expected } => {
                write!(f, "unsupported schema_version {found}; expected {expected}")
            }
            Self::InvalidSchemaVersionValue(value) => {
                write!(f, "invalid schema_version value `{value}`")
            }
            Self::EmptyField(field) => write!(f, "envelope field `{field}` must not be empty"),
            Self::InvalidId(id) => write!(
                f,
                "manifest id `{id}` must be a dotted identifier without whitespace"
            ),
            Self::Kind(error) => error.fmt(f),
            Self::InvalidLine {
                line_number,
                message,
            } => write!(f, "invalid envelope line {line_number}: {message}"),
        }
    }
}

impl std::error::Error for SchemaEnvelopeError {}

/// Extract only the top-level YAML scalar envelope fields. This is not a full
/// YAML parser by design; full manifest deserialization belongs to the later
/// typed manifest loaders. The goal here is to fail fast when a file does not
/// expose the mandatory control-plane envelope.
pub fn extract_schema_envelope_fields(
    source: &str,
) -> Result<SchemaEnvelopeFields, SchemaEnvelopeError> {
    let mut fields: BTreeMap<&'static str, String> = BTreeMap::new();

    for (line_index, raw_line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        let line = raw_line.trim_end();
        if line.trim().is_empty() || line.trim_start().starts_with('#') || line.trim() == "---" {
            continue;
        }

        // Nested YAML keys are ignored. The envelope MUST be top-level.
        if raw_line.starts_with(' ') || raw_line.starts_with('\t') {
            continue;
        }

        let Some((raw_key, raw_value)) = line.split_once(':') else {
            continue;
        };
        let key = raw_key.trim();
        let Some(canonical_key) = envelope_key(key) else {
            continue;
        };
        if fields.contains_key(canonical_key) {
            return Err(SchemaEnvelopeError::DuplicateField(canonical_key));
        }
        let value = parse_top_level_scalar(raw_value).map_err(|message| {
            SchemaEnvelopeError::InvalidLine {
                line_number,
                message,
            }
        })?;
        fields.insert(canonical_key, value);
    }

    let schema_version = fields
        .remove("schema_version")
        .ok_or(SchemaEnvelopeError::MissingField("schema_version"))?;
    let kind = fields
        .remove("kind")
        .ok_or(SchemaEnvelopeError::MissingField("kind"))?;
    let id = fields
        .remove("id")
        .ok_or(SchemaEnvelopeError::MissingField("id"))?;

    Ok(SchemaEnvelopeFields {
        schema_version,
        kind,
        id,
    })
}

pub fn parse_schema_envelope_from_yaml_str(
    source: &str,
) -> Result<SchemaEnvelope, SchemaEnvelopeError> {
    let fields = extract_schema_envelope_fields(source)?;
    let schema_version = fields.schema_version.parse::<u32>().map_err(|_| {
        SchemaEnvelopeError::InvalidSchemaVersionValue(fields.schema_version.clone())
    })?;

    SchemaEnvelope::new(schema_version, fields.kind, fields.id)
}

pub fn parse_schema_envelope_str(source: &str) -> Result<SchemaEnvelope, SchemaEnvelopeError> {
    parse_schema_envelope_from_yaml_str(source)
}

pub fn validate_schema_envelope(envelope: &SchemaEnvelope) -> Result<(), SchemaEnvelopeError> {
    if envelope.schema_version != SUPPORTED_SCHEMA_VERSION {
        return Err(SchemaEnvelopeError::InvalidSchemaVersion {
            found: envelope.schema_version,
            expected: SUPPORTED_SCHEMA_VERSION,
        });
    }
    validate_manifest_id(&envelope.id)?;
    Ok(())
}

pub fn validate_manifest_id(id: &str) -> Result<(), SchemaEnvelopeError> {
    // Current workspace compatibility: `.vac/registry/product.yaml` still uses
    // `id: vac`. Keep it readable until the registry migration protocol can
    // move the root product descriptor to a refined dotted registry id.
    if id == "vac" {
        return Ok(());
    }
    if id.trim() != id || id.chars().any(char::is_whitespace) {
        return Err(SchemaEnvelopeError::InvalidId(id.to_string()));
    }
    if id.starts_with('.') || id.ends_with('.') || !id.contains('.') {
        return Err(SchemaEnvelopeError::InvalidId(id.to_string()));
    }

    for segment in id.split('.') {
        if segment.is_empty()
            || !segment
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        {
            return Err(SchemaEnvelopeError::InvalidId(id.to_string()));
        }
    }
    Ok(())
}

pub fn validate_dotted_manifest_id(id: &str) -> Result<(), SchemaEnvelopeError> {
    validate_manifest_id(id)
}

fn envelope_key(key: &str) -> Option<&'static str> {
    match key {
        "schema_version" => Some("schema_version"),
        "kind" => Some("kind"),
        "id" => Some("id"),
        _ => None,
    }
}

fn parse_top_level_scalar(raw_value: &str) -> Result<String, String> {
    let value = raw_value.trim();
    if value.is_empty() {
        return Ok(String::new());
    }

    let without_comment = strip_inline_comment(value).trim();
    if without_comment.is_empty() {
        return Ok(String::new());
    }
    if let Some(stripped) = strip_quoted_scalar(without_comment, '"') {
        return Ok(stripped.to_string());
    }
    if let Some(stripped) = strip_quoted_scalar(without_comment, '\'') {
        return Ok(stripped.to_string());
    }
    Ok(without_comment.to_string())
}

fn strip_quoted_scalar(value: &str, quote: char) -> Option<&str> {
    if value.starts_with(quote) && value.ends_with(quote) && value.len() >= 2 {
        Some(&value[quote.len_utf8()..value.len() - quote.len_utf8()])
    } else {
        None
    }
}

fn strip_inline_comment(value: &str) -> &str {
    let mut in_single = false;
    let mut in_double = false;
    let mut previous_was_whitespace = true;

    for (index, ch) in value.char_indices() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '#' if !in_single && !in_double && previous_was_whitespace => return &value[..index],
            _ => {}
        }
        previous_was_whitespace = ch.is_whitespace();
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_schema_envelope() {
        let yaml = r#"
schema_version: 1
kind: capability
id: vac.init.schema-envelope
title: Example
"#;
        let envelope = parse_schema_envelope_from_yaml_str(yaml).expect("valid envelope");
        assert_eq!(envelope.schema_version, 1);
        assert_eq!(envelope.kind, VacManifestKind::Capability);
        assert_eq!(envelope.id, "vac.init.schema-envelope");
        assert_eq!(
            envelope.dotted_id_segments(),
            vec!["vac", "init", "schema-envelope"]
        );
    }

    #[test]
    fn extracts_raw_fields_before_kind_validation() {
        let fields = extract_schema_envelope_fields(
            r#"
schema_version: 1
kind: custom_kind
id: vac.custom.kind
"#,
        )
        .expect("raw fields");
        assert_eq!(fields.schema_version, "1");
        assert_eq!(fields.kind, "custom_kind");
        assert_eq!(fields.id, "vac.custom.kind");
    }

    #[test]
    fn accepts_quoted_scalar_values() {
        let yaml = r#"
schema_version: 1
kind: "workflow"
id: 'maintenance.vac-init-schema-envelope'
"#;
        let envelope = parse_schema_envelope_from_yaml_str(yaml).expect("valid quoted envelope");
        assert_eq!(envelope.kind, VacManifestKind::Workflow);
        assert_eq!(envelope.id, "maintenance.vac-init-schema-envelope");
    }

    #[test]
    fn strips_inline_comments_outside_quotes() {
        let yaml = r#"
schema_version: 1 # supported schema
kind: capability # kind comment
id: "vac.init.not-comment" # actual comment
"#;
        let envelope = parse_schema_envelope_str(yaml).expect("valid envelope");
        assert_eq!(envelope.id, "vac.init.not-comment");
    }

    #[test]
    fn rejects_missing_kind() {
        let yaml = r#"
schema_version: 1
id: vac.init.schema-envelope
"#;
        let err = parse_schema_envelope_str(yaml).expect_err("missing kind");
        assert_eq!(err, SchemaEnvelopeError::MissingField("kind"));
    }

    #[test]
    fn rejects_unknown_kind() {
        let yaml = r#"
schema_version: 1
kind: freeform_script
id: vac.init.schema-envelope
"#;
        let err = parse_schema_envelope_str(yaml).expect_err("unknown kind");
        assert!(matches!(err, SchemaEnvelopeError::Kind(_)));
        assert!(err.to_string().contains("freeform_script"));
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let yaml = r#"
schema_version: 2
kind: capability
id: vac.init.schema-envelope
"#;
        let err = parse_schema_envelope_str(yaml).expect_err("unsupported schema");
        assert_eq!(
            err,
            SchemaEnvelopeError::InvalidSchemaVersion {
                found: 2,
                expected: SUPPORTED_SCHEMA_VERSION,
            }
        );
    }

    #[test]
    fn rejects_invalid_schema_version_value() {
        let yaml = r#"
schema_version: one
kind: capability
id: vac.init.schema-envelope
"#;
        let err = parse_schema_envelope_str(yaml).expect_err("invalid version");
        assert_eq!(
            err,
            SchemaEnvelopeError::InvalidSchemaVersionValue("one".to_string())
        );
    }

    #[test]
    fn accepts_current_root_product_id_compatibility() {
        let yaml = r#"
schema_version: 1
kind: product
id: vac
"#;
        let envelope = parse_schema_envelope_str(yaml).expect("root product descriptor");
        assert_eq!(envelope.kind, VacManifestKind::Product);
        assert_eq!(envelope.id, "vac");
    }

    #[test]
    fn rejects_bad_manifest_id() {
        let yaml = r#"
schema_version: 1
kind: capability
id: "vac init"
"#;
        let err = parse_schema_envelope_str(yaml).expect_err("bad id");
        assert_eq!(err, SchemaEnvelopeError::InvalidId("vac init".to_string()));
    }

    #[test]
    fn rejects_duplicate_envelope_field() {
        let yaml = r#"
schema_version: 1
schema_version: 1
kind: capability
id: vac.init.schema-envelope
"#;
        let err = parse_schema_envelope_str(yaml).expect_err("duplicate field");
        assert_eq!(err, SchemaEnvelopeError::DuplicateField("schema_version"));
    }

    #[test]
    fn ignores_nested_yaml_keys_when_finding_envelope() {
        let yaml = r#"
schema_version: 1
kind: capability
id: vac.init.schema-envelope
owner:
  kind: nested-value
  id: nested.value
"#;
        let envelope = parse_schema_envelope_str(yaml).expect("valid envelope");
        assert_eq!(envelope.kind, VacManifestKind::Capability);
        assert_eq!(envelope.id, "vac.init.schema-envelope");
    }
}
