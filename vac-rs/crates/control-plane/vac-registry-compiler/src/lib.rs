//! Deterministic VAC registry compiler.
//!
//! YAML under `.vac/` is authoring-plane input. This crate models the compiled
//! JSON runtime truth used by VAC v1.5: sorted inputs, stable source hashes,
//! readiness triplets, policy/surface/workflow/spec projection, and a JCS-style
//! canonical hash. Nondeterministic timestamps are intentionally kept out of the
//! hashed runtime authority projection.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceManifestHash {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompiledCapabilityRef {
    pub id: String,
    pub declared: String,
    pub computed: String,
    pub effective: String,
    pub source_path: String,
    pub source_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompiledRegistrySnapshot {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub runtime_truth: String,
    pub audit_truth: String,
    pub snapshot_hash: String,
    #[serde(default)]
    pub source_hashes: Vec<SourceManifestHash>,
    #[serde(default)]
    pub capabilities: Vec<CompiledCapabilityRef>,
    #[serde(default)]
    pub policies: Vec<SourceManifestHash>,
    #[serde(default)]
    pub workflows: Vec<SourceManifestHash>,
    #[serde(default)]
    pub surfaces: Vec<SourceManifestHash>,
    #[serde(default)]
    pub specs: Vec<SourceManifestHash>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompiledSnapshot {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub generated_at: Option<String>,
    pub source_hashes: Vec<SourceManifestHash>,
}

#[must_use]
pub fn source_manifest_hash(path: impl Into<String>, bytes: &[u8]) -> SourceManifestHash {
    SourceManifestHash {
        path: path.into(),
        sha256: sha256_bytes(bytes),
    }
}

#[must_use]
pub fn compile_deterministic_registry(
    mut source_hashes: Vec<SourceManifestHash>,
    mut capabilities: Vec<CompiledCapabilityRef>,
    mut policies: Vec<SourceManifestHash>,
    mut workflows: Vec<SourceManifestHash>,
    mut surfaces: Vec<SourceManifestHash>,
    mut specs: Vec<SourceManifestHash>,
) -> CompiledRegistrySnapshot {
    source_hashes.sort_by(|a, b| a.path.cmp(&b.path));
    capabilities.sort_by(|a, b| a.id.cmp(&b.id));
    policies.sort_by(|a, b| a.path.cmp(&b.path));
    workflows.sort_by(|a, b| a.path.cmp(&b.path));
    surfaces.sort_by(|a, b| a.path.cmp(&b.path));
    specs.sort_by(|a, b| a.path.cmp(&b.path));

    let mut snapshot = CompiledRegistrySnapshot {
        schema_version: 1,
        kind: "compiled_registry_snapshot".to_string(),
        id: "vac.registry.compiled".to_string(),
        runtime_truth: "compiled-json".to_string(),
        audit_truth: "jcs-json".to_string(),
        snapshot_hash: String::new(),
        source_hashes,
        capabilities,
        policies,
        workflows,
        surfaces,
        specs,
    };
    snapshot.snapshot_hash = compiled_snapshot_hash(&snapshot);
    snapshot
}

#[must_use]
pub fn compiled_snapshot_hash(snapshot: &CompiledRegistrySnapshot) -> String {
    let mut projection = serde_json::to_value(snapshot).unwrap_or(Value::Null);
    if let Value::Object(map) = &mut projection {
        map.insert("snapshot_hash".to_string(), Value::String(String::new()));
    }
    canonical_json_sha256(&projection)
}

pub fn deterministic_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let value = serde_json::to_value(value)?;
    vac_jcs::to_canonical_pretty_string(&value)
}

#[must_use]
pub fn canonical_json_sha256(value: &Value) -> String {
    vac_jcs::canonical_json_sha256(value)
}

#[must_use]
pub fn sha256_bytes(bytes: &[u8]) -> String {
    vac_jcs::sha256_bytes(bytes)
}

#[must_use]
pub fn normalize_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted = BTreeMap::new();
            for (key, val) in map {
                sorted.insert(key.clone(), normalize_json(val));
            }
            let mut out = Map::new();
            for (key, val) in sorted {
                out.insert(key, val);
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(normalize_json).collect()),
        other => other.clone(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompiledAuthoringManifest {
    pub path: String,
    pub schema_version: Option<u32>,
    pub kind: String,
    pub id: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryCompileReport {
    pub snapshot: CompiledRegistrySnapshot,
    pub manifests: Vec<CompiledAuthoringManifest>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryCompileError {
    Io(String),
    InvalidEnvelope {
        path: String,
        reason: String,
    },
    DuplicateManifestId {
        id: String,
        first_path: String,
        duplicate_path: String,
    },
}

/// Compile the `.vac` YAML authoring plane into the runtime-truth snapshot.
///
/// v1.9 hardening: this path now parses YAML structurally, fails closed on
/// duplicate manifest IDs, and computes capability readiness from declared
/// status plus presence of policy/surface/validation material.  It still keeps
/// TV/Cargo gates pending, so `ready` cannot become effective-ready in the
/// sandbox compiler.
pub fn compile_registry_from_disk(
    root: &std::path::Path,
) -> Result<RegistryCompileReport, RegistryCompileError> {
    let mut parsed: Vec<(CompiledAuthoringManifest, YamlValue)> = Vec::new();
    let mut warnings = Vec::new();
    let mut seen_ids: BTreeMap<String, String> = BTreeMap::new();
    let mut seen_paths: BTreeSet<String> = BTreeSet::new();
    let authoring_dirs = [
        ".vac/capabilities",
        ".vac/policies",
        ".vac/workflows",
        ".vac/surfaces",
        ".vac/specs/confirmed",
    ];
    for dir in authoring_dirs {
        let base = root.join(dir);
        if !base.exists() {
            warnings.push(format!("missing authoring directory: {dir}"));
            continue;
        }
        let entries =
            std::fs::read_dir(&base).map_err(|err| RegistryCompileError::Io(err.to_string()))?;
        for entry in entries {
            let entry = entry.map_err(|err| RegistryCompileError::Io(err.to_string()))?;
            let path = entry.path();
            if path.extension().and_then(|item| item.to_str()) != Some("yaml") {
                continue;
            }
            let bytes =
                std::fs::read(&path).map_err(|err| RegistryCompileError::Io(err.to_string()))?;
            let rel = path
                .strip_prefix(root)
                .unwrap_or(path.as_path())
                .to_string_lossy()
                .replace('\\', "/");
            let value: YamlValue = serde_yaml::from_slice(&bytes).map_err(|err| {
                RegistryCompileError::InvalidEnvelope {
                    path: rel.clone(),
                    reason: format!("invalid YAML: {err}"),
                }
            })?;
            let envelope = parse_authoring_envelope_value(&rel, &value)?;
            if !seen_paths.insert(rel.clone()) {
                return Err(RegistryCompileError::DuplicateManifestId {
                    id: envelope.id,
                    first_path: rel.clone(),
                    duplicate_path: rel,
                });
            }
            if let Some(first_path) = seen_ids.insert(envelope.id.clone(), rel.clone()) {
                return Err(RegistryCompileError::DuplicateManifestId {
                    id: envelope.id,
                    first_path,
                    duplicate_path: rel,
                });
            }
            parsed.push((
                CompiledAuthoringManifest {
                    path: rel,
                    schema_version: envelope.schema_version,
                    kind: envelope.kind,
                    id: envelope.id,
                    sha256: sha256_bytes(&bytes),
                },
                value,
            ));
        }
    }
    parsed.sort_by(|a, b| a.0.path.cmp(&b.0.path));
    let manifests = parsed
        .iter()
        .map(|(manifest, _)| manifest.clone())
        .collect::<Vec<_>>();
    let source_hashes = manifests
        .iter()
        .map(|item| SourceManifestHash {
            path: item.path.clone(),
            sha256: item.sha256.clone(),
        })
        .collect::<Vec<_>>();
    let capabilities = parsed
        .iter()
        .filter(|(item, _)| item.kind == "capability")
        .map(|(item, yaml)| {
            let declared = yaml_string(yaml, "readiness.declared")
                .or_else(|| yaml_string(yaml, "status"))
                .unwrap_or_else(|| "partial".to_string());
            let has_policy = yaml_has_mapping(yaml, "policy");
            let has_surface = yaml_has_mapping(yaml, "surfaces");
            let has_validation = yaml_sequence_nonempty(yaml, "validation.commands")
                || yaml_sequence_nonempty(yaml, "validation.gates");
            let readiness = compute_effective_readiness(
                &declared,
                has_policy,
                has_surface,
                has_validation,
                true,
            );
            CompiledCapabilityRef {
                id: item.id.clone(),
                declared: readiness.declared,
                computed: readiness.computed,
                effective: readiness.effective,
                source_path: item.path.clone(),
                source_hash: item.sha256.clone(),
            }
        })
        .collect::<Vec<_>>();
    let by_kind = |kind: &str| -> Vec<SourceManifestHash> {
        manifests
            .iter()
            .filter(|item| item.kind == kind)
            .map(|item| SourceManifestHash {
                path: item.path.clone(),
                sha256: item.sha256.clone(),
            })
            .collect()
    };
    let snapshot = compile_deterministic_registry(
        source_hashes,
        capabilities,
        by_kind("policy"),
        by_kind("workflow"),
        by_kind("surface"),
        by_kind("intent_spec"),
    );
    Ok(RegistryCompileReport {
        snapshot,
        manifests,
        warnings,
    })
}

struct ParsedEnvelope {
    schema_version: Option<u32>,
    kind: String,
    id: String,
}

fn parse_authoring_envelope_value(
    path: &str,
    value: &YamlValue,
) -> Result<ParsedEnvelope, RegistryCompileError> {
    let schema_version = yaml_u32(value, "schema_version");
    let kind = yaml_string(value, "kind").ok_or_else(|| RegistryCompileError::InvalidEnvelope {
        path: path.to_string(),
        reason: "missing kind".to_string(),
    })?;
    let id = yaml_string(value, "id").ok_or_else(|| RegistryCompileError::InvalidEnvelope {
        path: path.to_string(),
        reason: "missing id".to_string(),
    })?;
    if kind.trim().is_empty() || id.trim().is_empty() {
        return Err(RegistryCompileError::InvalidEnvelope {
            path: path.to_string(),
            reason: "empty kind/id".to_string(),
        });
    }
    Ok(ParsedEnvelope {
        schema_version,
        kind,
        id,
    })
}

fn yaml_get<'a>(value: &'a YamlValue, dotted: &str) -> Option<&'a YamlValue> {
    let mut current = value;
    for segment in dotted.split('.') {
        let YamlValue::Mapping(map) = current else {
            return None;
        };
        current = map.get(YamlValue::String(segment.to_string()))?;
    }
    Some(current)
}

fn yaml_string(value: &YamlValue, dotted: &str) -> Option<String> {
    match yaml_get(value, dotted)? {
        YamlValue::String(s) => Some(s.clone()),
        YamlValue::Number(n) => Some(n.to_string()),
        YamlValue::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn yaml_u32(value: &YamlValue, dotted: &str) -> Option<u32> {
    match yaml_get(value, dotted)? {
        YamlValue::Number(n) => n.as_u64().and_then(|n| u32::try_from(n).ok()),
        YamlValue::String(s) => s.parse::<u32>().ok(),
        _ => None,
    }
}

fn yaml_has_mapping(value: &YamlValue, dotted: &str) -> bool {
    matches!(yaml_get(value, dotted), Some(YamlValue::Mapping(map)) if !map.is_empty())
}

fn yaml_sequence_nonempty(value: &YamlValue, dotted: &str) -> bool {
    matches!(yaml_get(value, dotted), Some(YamlValue::Sequence(seq)) if !seq.is_empty())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadinessTriplet {
    pub declared: String,
    pub computed: String,
    pub effective: String,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeTruthProjection {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub runtime_truth: String,
    pub audit_truth: String,
    pub source_manifest: Vec<SourceManifestHash>,
    pub readiness_summary: BTreeMap<String, u32>,
    pub snapshot_hash: String,
}

#[must_use]
pub fn compute_effective_readiness(
    declared: &str,
    has_policy: bool,
    has_surface: bool,
    has_validation: bool,
    tv_pending: bool,
) -> ReadinessTriplet {
    let mut reasons = Vec::new();
    let computed = if has_policy && has_surface && has_validation && !tv_pending {
        "ready"
    } else if declared == "planned" {
        "planned"
    } else {
        if !has_policy {
            reasons.push("missing policy".to_string());
        }
        if !has_surface {
            reasons.push("missing surface".to_string());
        }
        if !has_validation {
            reasons.push("missing validation".to_string());
        }
        if tv_pending {
            reasons.push("TV cargo gates pending".to_string());
        }
        "partial"
    };
    let effective = match (declared, computed) {
        ("deprecated", _) => "deprecated",
        ("planned", _) | (_, "planned") => "planned",
        ("ready", "ready") => "ready",
        _ => "partial",
    };
    ReadinessTriplet {
        declared: declared.to_string(),
        computed: computed.to_string(),
        effective: effective.to_string(),
        reasons,
    }
}

#[must_use]
pub fn runtime_truth_projection(snapshot: &CompiledRegistrySnapshot) -> RuntimeTruthProjection {
    let mut readiness_summary = BTreeMap::new();
    for cap in &snapshot.capabilities {
        *readiness_summary.entry(cap.effective.clone()).or_insert(0) += 1;
    }
    let mut projection = RuntimeTruthProjection {
        schema_version: snapshot.schema_version,
        kind: "runtime_truth_projection".to_string(),
        id: snapshot.id.clone(),
        runtime_truth: snapshot.runtime_truth.clone(),
        audit_truth: snapshot.audit_truth.clone(),
        source_manifest: snapshot.source_hashes.clone(),
        readiness_summary,
        snapshot_hash: String::new(),
    };
    projection.snapshot_hash =
        canonical_json_sha256(&serde_json::to_value(&projection).unwrap_or(Value::Null));
    projection
}

#[must_use]
pub fn verify_source_manifest_hashes(
    snapshot: &CompiledRegistrySnapshot,
    actual: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut mismatches = Vec::new();
    for item in &snapshot.source_hashes {
        match actual.get(&item.path) {
            Some(hash) if hash == &item.sha256 => {}
            Some(hash) => mismatches.push(format!(
                "{} expected {} actual {}",
                item.path, item.sha256, hash
            )),
            None => mismatches.push(format!("{} missing from packaged source", item.path)),
        }
    }
    mismatches
}
