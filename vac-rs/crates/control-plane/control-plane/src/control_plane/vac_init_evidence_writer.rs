#![allow(dead_code)]
//! Live evidence writer and trajectory index helpers for VAC-Init.
//!
//! P7 uses this layer to make evidence and `vac why` data producible by runtime
//! completion paths instead of relying on seed-only trajectory records.

use std::fs;
use std::path::{Path, PathBuf};

use base64::prelude::{BASE64_STANDARD, Engine as _};
use ed25519_dalek::Signer as _;
use ed25519_dalek::SigningKey;

const EVIDENCE_SIGNING_KEY_ENV: &str = "VAC_EVIDENCE_ED25519_SIGNING_KEY_BASE64";
const EVIDENCE_SIGNING_REQUIRED_ENV: &str = "VAC_EVIDENCE_SIGNING_REQUIRED";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveEvidenceWriteRequest {
    pub evidence_id: String,
    pub timestamp: String,
    pub plan_id: String,
    pub capability: String,
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
    pub symbol: Option<String>,
    pub rationale_summary: String,
    pub approval_content_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveEvidenceWriteResult {
    pub evidence_path: PathBuf,
    pub trajectory_index_path: PathBuf,
    pub trajectory_file_path: PathBuf,
}

pub fn validate_safe_rationale(summary: &str) -> Result<(), String> {
    let lower = summary.to_ascii_lowercase();
    if lower.contains("chain of thought")
        || lower.contains("raw_chain_of_thought")
        || lower.contains("private reasoning")
        || lower.contains("scratchpad")
    {
        return Err(
            "safe rationale must not contain raw/private chain-of-thought markers".to_string(),
        );
    }
    if summary.trim().is_empty() {
        return Err("safe rationale summary must not be empty".to_string());
    }
    Ok(())
}

pub fn write_live_evidence_and_trajectory(
    workspace_root: impl AsRef<Path>,
    request: &LiveEvidenceWriteRequest,
) -> Result<LiveEvidenceWriteResult, String> {
    validate_request(request)?;
    let root = workspace_root.as_ref();
    let evidence_rel = format!(
        ".vac/registry/evidence/{}.yaml",
        sanitize_file_name(&request.evidence_id)
    );
    let trajectory_index_rel = ".vac/registry/trajectory/index.yaml".to_string();
    let trajectory_file_rel = format!(
        ".vac/registry/trajectory/{}/{}.trajectory.yaml",
        sanitize_file_name(&request.capability),
        sanitize_file_name(&request.file)
    );

    let signing_required = evidence_signing_required_for_root(root);
    let evidence_yaml = render_evidence_yaml_with_policy(request, signing_required);
    if signing_required && evidence_yaml.contains("missing-required-ed25519-key") {
        return Err("evidence signing is required by policy/env but VAC_EVIDENCE_ED25519_SIGNING_KEY_BASE64 is missing".to_string());
    }
    let index_yaml = render_trajectory_index_yaml(request);
    let trajectory_yaml = render_trajectory_file_yaml(request);

    atomic_write(root, &evidence_rel, &evidence_yaml)?;
    atomic_write(root, &trajectory_index_rel, &index_yaml)?;
    atomic_write(root, &trajectory_file_rel, &trajectory_yaml)?;

    Ok(LiveEvidenceWriteResult {
        evidence_path: root.join(evidence_rel),
        trajectory_index_path: root.join(trajectory_index_rel),
        trajectory_file_path: root.join(trajectory_file_rel),
    })
}

pub fn render_evidence_yaml(request: &LiveEvidenceWriteRequest) -> String {
    render_evidence_yaml_with_policy(request, evidence_signing_required())
}

pub fn render_evidence_yaml_with_policy(
    request: &LiveEvidenceWriteRequest,
    signing_required: bool,
) -> String {
    let canonical_payload = format!(
        "evidence_id={}\ntimestamp={}\nplan_id={}\ncapability={}\nfile={}\nstart_line={}\nend_line={}\nrationale_summary={}\n",
        request.evidence_id,
        request.timestamp,
        request.plan_id,
        request.capability,
        request.file,
        request.start_line,
        request.end_line,
        request.rationale_summary
    );
    let payload_hash = super::vac_init_evidence_chain::sha256_hex(canonical_payload.as_bytes());
    let signature =
        optional_ed25519_signature_with_policy(canonical_payload.as_bytes(), signing_required);
    let approval_hash = request
        .approval_content_hash
        .clone()
        .unwrap_or_else(|| "none".to_string());
    format!(
        "schema_version: 1\nkind: evidence\nid: {}\ntimestamp: {}\nchain:\n  previous: null\n  previous_hash: null\n  self_hash: {}\nchangeset:\n  plan_id: {}\n  capability: {}\n  files_modified:\n    - path: {}\n      operation: modify\n      diff_hash: {}\n      lines_added: 0\n      lines_removed: 0\nvalidation:\n  commands_executed: []\n  gates_passed:\n    - evidence_completion_gate\n  gates_failed: []\napproval:\n  approved_by: operator\n  approved_at: {}\n  content_hash: {}\n  signature:\n    algorithm: {signature_algorithm}\n    public_key_base64: {signature_public_key}\n    value: {signature_value}\n    signing_required: {signing_required}\nattribution:\n  agent_id: vac-init-live-evidence-writer\n  model: deterministic\n  memory_snapshot:\n    working_facts: 0\n    team_rules_consulted: []\n    semantic_refs: []\n  rationale_ref: .vac/registry/trajectory/index.yaml\n",
        yaml_scalar(&request.evidence_id),
        yaml_scalar(&request.timestamp),
        payload_hash,
        yaml_scalar(&request.plan_id),
        yaml_scalar(&request.capability),
        yaml_scalar(&request.file),
        payload_hash,
        yaml_scalar(&request.timestamp),
        yaml_scalar(&approval_hash),
        signature_algorithm = yaml_scalar(signature.algorithm),
        signature_public_key = yaml_scalar(&signature.public_key_base64),
        signature_value = yaml_scalar(&signature.signature_base64),
        signing_required = signing_required,
    )
}

struct OptionalEvidenceSignature {
    algorithm: &'static str,
    public_key_base64: String,
    signature_base64: String,
}

fn evidence_signing_required_for_root(root: &Path) -> bool {
    if evidence_signing_required() {
        return true;
    }
    let policies = root.join(".vac/policies");
    let Ok(entries) = fs::read_dir(policies) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("yaml") {
            continue;
        }
        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        let compact = contents
            .lines()
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("\n");
        if compact.contains("evidence:")
            && compact.contains("signing:")
            && compact.contains("required: true")
        {
            return true;
        }
    }
    false
}

fn evidence_signing_required() -> bool {
    matches!(
        std::env::var(EVIDENCE_SIGNING_REQUIRED_ENV)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "required"
    )
}

fn optional_ed25519_signature(payload: &[u8]) -> OptionalEvidenceSignature {
    optional_ed25519_signature_with_policy(payload, evidence_signing_required())
}

fn optional_ed25519_signature_with_policy(
    payload: &[u8],
    signing_required: bool,
) -> OptionalEvidenceSignature {
    let Ok(encoded_key) = std::env::var(EVIDENCE_SIGNING_KEY_ENV) else {
        return OptionalEvidenceSignature {
            algorithm: if signing_required {
                "missing-required-ed25519-key"
            } else {
                "unsigned"
            },
            public_key_base64: "null".to_string(),
            signature_base64: "null".to_string(),
        };
    };
    let Ok(bytes) = BASE64_STANDARD.decode(encoded_key.trim()) else {
        return OptionalEvidenceSignature {
            algorithm: "invalid-ed25519-key",
            public_key_base64: "null".to_string(),
            signature_base64: "null".to_string(),
        };
    };
    let secret: [u8; 32] = match bytes.as_slice().try_into() {
        Ok(secret) => secret,
        Err(_) => {
            return OptionalEvidenceSignature {
                algorithm: "invalid-ed25519-key",
                public_key_base64: "null".to_string(),
                signature_base64: "null".to_string(),
            };
        }
    };
    let signing_key = SigningKey::from_bytes(&secret);
    let signature = signing_key.sign(payload);
    OptionalEvidenceSignature {
        algorithm: "ed25519",
        public_key_base64: BASE64_STANDARD.encode(signing_key.verifying_key().to_bytes()),
        signature_base64: BASE64_STANDARD.encode(signature.to_bytes()),
    }
}

pub fn render_trajectory_index_yaml(request: &LiveEvidenceWriteRequest) -> String {
    let symbol_line = request
        .symbol
        .as_ref()
        .map(|symbol| format!("    symbol: {}\n", yaml_scalar(symbol)))
        .unwrap_or_default();
    format!(
        "schema_version: 1\nkind: trajectory\nid: trajectory.index\nspans:\n  - file: {}\n    start_line: {}\n    end_line: {}\n{}    evidence_id: {}\nresults_by_evidence:\n  {}:\n    evidence_id: {}\n    timestamp: {}\n    task: {}\n    plan_id: {}\n    capability: {}\n    rationale:\n      summary: {}\n      policy_refs: []\n      evidence_refs:\n        - {}\n      memory_refs:\n        team_rules: []\n        semantic_refs: []\n      excluded:\n        raw_chain_of_thought: true\n    changeset:\n      operation: modify\n      diff_excerpt: \"\"\n      lines_added: 0\n      lines_removed: 0\n    approval:\n      approved_by: operator\n      content_hash: {}\n    chain_depth: 1\n",
        yaml_scalar(&request.file),
        request.start_line,
        request.end_line,
        symbol_line,
        yaml_scalar(&request.evidence_id),
        yaml_scalar(&request.evidence_id),
        yaml_scalar(&request.evidence_id),
        yaml_scalar(&request.timestamp),
        yaml_scalar(&request.plan_id),
        yaml_scalar(&request.plan_id),
        yaml_scalar(&request.capability),
        yaml_scalar(&request.rationale_summary),
        yaml_scalar(&request.evidence_id),
        yaml_scalar(request.approval_content_hash.as_deref().unwrap_or("none"))
    )
}

pub fn render_trajectory_file_yaml(request: &LiveEvidenceWriteRequest) -> String {
    format!(
        "schema_version: 1\nkind: trajectory\nid: trajectory.{}.{}\nfile: {}\ncapability: {}\nevidence:\n  - {}\nrationale:\n  summary: {}\n  excluded:\n    raw_chain_of_thought: true\n",
        sanitize_id(&request.capability),
        sanitize_id(&request.file),
        yaml_scalar(&request.file),
        yaml_scalar(&request.capability),
        yaml_scalar(&request.evidence_id),
        yaml_scalar(&request.rationale_summary)
    )
}

fn validate_request(request: &LiveEvidenceWriteRequest) -> Result<(), String> {
    if request.evidence_id.trim().is_empty() || !request.evidence_id.contains('.') {
        return Err("evidence id must be dotted".to_string());
    }
    if request.start_line == 0 || request.end_line < request.start_line {
        return Err("trajectory range must be one-based and ordered".to_string());
    }
    if request.file.starts_with('/') || request.file.contains("..") || request.file.contains('\\') {
        return Err("evidence file path must be workspace-relative".to_string());
    }
    validate_safe_rationale(&request.rationale_summary)
}

fn atomic_write(root: &Path, relative: &str, content: &str) -> Result<(), String> {
    if relative.starts_with('/') || relative.contains("..") || relative.contains('\\') {
        return Err("relative store path is invalid".to_string());
    }
    let final_path = root.join(relative);
    let parent = final_path
        .parent()
        .ok_or_else(|| "store path must have a parent".to_string())?;
    fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    let tmp = final_path.with_extension("yaml.tmp");
    fs::write(&tmp, content).map_err(|err| err.to_string())?;
    fs::rename(&tmp, &final_path).map_err(|err| err.to_string())?;
    Ok(())
}

fn stable_hash_hex(input: &str) -> String {
    // Deterministic non-cryptographic fallback for live writer metadata. The
    // canonical evidence-chain module still owns cryptographic sha256 checks.
    let mut state: u64 = 0xcbf29ce484222325;
    for byte in input.as_bytes() {
        state ^= u64::from(*byte);
        state = state.wrapping_mul(0x100000001b3);
    }
    format!("{state:016x}{state:016x}{state:016x}{state:016x}")
}

fn sanitize_file_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn sanitize_id(value: &str) -> String {
    sanitize_file_name(value).replace(['-', '/'], ".")
}

fn yaml_scalar(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | '/' | ':'))
        && !value.is_empty()
    {
        value.to_string()
    } else {
        format!("{value:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("vac-evidence-writer-{unique}"))
    }

    fn request() -> LiveEvidenceWriteRequest {
        LiveEvidenceWriteRequest {
            evidence_id: "evidence.test.live".to_string(),
            timestamp: "2026-05-29T00:00:00Z".to_string(),
            plan_id: "plan.test".to_string(),
            capability: "vac.test".to_string(),
            file: "src/lib.rs".to_string(),
            start_line: 1,
            end_line: 3,
            symbol: Some("run".to_string()),
            rationale_summary: "Safe summary generated from task evidence.".to_string(),
            approval_content_hash: Some("approval-hash".to_string()),
        }
    }

    #[test]
    fn rejects_raw_chain_of_thought_markers() {
        assert!(validate_safe_rationale("raw_chain_of_thought: secret").is_err());
        assert!(validate_safe_rationale("safe summary").is_ok());
    }

    #[test]
    fn writes_evidence_and_trajectory_index() {
        let root = temp_root();
        let result = write_live_evidence_and_trajectory(&root, &request()).unwrap();
        assert!(result.evidence_path.exists());
        assert!(result.trajectory_index_path.exists());
        assert!(result.trajectory_file_path.exists());
        let index = fs::read_to_string(&result.trajectory_index_path).unwrap();
        assert!(index.contains("raw_chain_of_thought: true"));
        assert!(index.contains("evidence.test.live"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rendered_yaml_has_required_envelopes() {
        let req = request();
        assert!(render_evidence_yaml(&req).starts_with("schema_version: 1\nkind: evidence\n"));
        assert!(render_trajectory_index_yaml(&req).contains("kind: trajectory"));
        assert!(render_trajectory_file_yaml(&req).contains("kind: trajectory"));
    }
}
