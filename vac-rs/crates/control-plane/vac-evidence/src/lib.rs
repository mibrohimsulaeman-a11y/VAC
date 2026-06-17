//! VAC evidence v2 store contracts.
//!
//! Implements the source-level model for per-capability sub-chains, CAS append,
//! xref markers, Merkle roots, and signature-mode honesty. `algorithm: none`
//! is explicit integrity-hint mode and never presented as tamper-evident.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceRef {
    pub id: String,
    pub capability: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Sha256Hex(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SigAlgorithm {
    Ed25519,
    Webauthn,
    Minisign,
    Sigstore,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SigMode {
    Signed,
    IntegrityHint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignatureRecord {
    pub algorithm: SigAlgorithm,
    pub key_id: String,
    pub value: Option<String>,
    pub mode: SigMode,
}

impl SignatureRecord {
    #[must_use]
    pub fn integrity_hint(key_id: impl Into<String>) -> Self {
        Self {
            algorithm: SigAlgorithm::None,
            key_id: key_id.into(),
            value: None,
            mode: SigMode::IntegrityHint,
        }
    }

    #[must_use]
    pub fn warning_label(&self) -> Option<&'static str> {
        matches!(self.mode, SigMode::IntegrityHint).then_some("integrity-hint, NOT tamper-evident")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubChain {
    pub prev_id: Option<String>,
    pub prev_hash: Option<String>,
    pub self_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceRecordV2 {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub capability: String,
    pub seq: u64,
    pub session: String,
    pub sub_chain: SubChain,
    pub git: GitBinding,
    #[serde(default)]
    pub cross_capability_refs: Vec<String>,
    pub approval: ApprovalBinding,
    pub validation: ValidationSummary,
    pub attribution: Attribution,
    pub seal: SealState,
    pub broker_sig: SignatureRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitBinding {
    pub code_commit: String,
    pub parent_commit: String,
    pub worktree_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovalBinding {
    pub approval_id: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ValidationSummary {
    #[serde(default)]
    pub gates_passed: Vec<String>,
    #[serde(default)]
    pub gates_failed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Attribution {
    pub agent_id: String,
    pub model: String,
    pub rationale_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SealState {
    pub merkle_epoch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct XrefMarker {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub capability: String,
    pub seq: u64,
    pub points_to: String,
    pub sub_chain: SubChain,
    pub broker_sig: SignatureRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceHead {
    pub capability: String,
    pub head_id: Option<String>,
    pub head_hash: Option<String>,
    pub head_seq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MerkleRoot {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub epoch: u64,
    pub prev_epoch_id: Option<String>,
    pub prev_epoch_root_hash: Option<String>,
    pub leaves: Vec<EvidenceHead>,
    pub root_hash: String,
    pub trigger: String,
    pub broker_sig: SignatureRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvidenceStoreError {
    CasConflict,
    InvalidHash,
    MissingPreviousHead,
    GitObjectWriteFailed,
}

pub trait EvidenceStore {
    fn head(&self, capability: &str) -> Option<EvidenceHead>;
    fn append(&mut self, record: EvidenceRecordV2) -> Result<EvidenceHead, EvidenceStoreError>;
    fn append_xref(&mut self, marker: XrefMarker) -> Result<EvidenceHead, EvidenceStoreError>;
    fn seal_epoch(&self, epoch: u64, trigger: &str, broker_sig: SignatureRecord) -> MerkleRoot;
}

#[derive(Debug, Default)]
pub struct InMemoryEvidenceStore {
    heads: HashMap<String, EvidenceHead>,
    records: HashMap<String, EvidenceRecordV2>,
    xrefs: HashMap<String, XrefMarker>,
}

impl EvidenceStore for InMemoryEvidenceStore {
    fn head(&self, capability: &str) -> Option<EvidenceHead> {
        self.heads.get(capability).cloned()
    }

    fn append(&mut self, mut record: EvidenceRecordV2) -> Result<EvidenceHead, EvidenceStoreError> {
        let current = self.heads.get(&record.capability).cloned();
        if current.as_ref().and_then(|head| head.head_hash.clone()) != record.sub_chain.prev_hash {
            return Err(EvidenceStoreError::CasConflict);
        }
        record.sub_chain.self_hash = evidence_record_hash(&record);
        let head = EvidenceHead {
            capability: record.capability.clone(),
            head_id: Some(record.id.clone()),
            head_hash: Some(record.sub_chain.self_hash.clone()),
            head_seq: record.seq,
        };
        self.records.insert(record.id.clone(), record);
        self.heads.insert(head.capability.clone(), head.clone());
        Ok(head)
    }

    fn append_xref(&mut self, mut marker: XrefMarker) -> Result<EvidenceHead, EvidenceStoreError> {
        let current = self.heads.get(&marker.capability).cloned();
        if current.as_ref().and_then(|head| head.head_hash.clone()) != marker.sub_chain.prev_hash {
            return Err(EvidenceStoreError::CasConflict);
        }
        marker.sub_chain.self_hash = xref_marker_hash(&marker);
        let head = EvidenceHead {
            capability: marker.capability.clone(),
            head_id: Some(marker.id.clone()),
            head_hash: Some(marker.sub_chain.self_hash.clone()),
            head_seq: marker.seq,
        };
        self.xrefs.insert(marker.id.clone(), marker);
        self.heads.insert(head.capability.clone(), head.clone());
        Ok(head)
    }

    fn seal_epoch(&self, epoch: u64, trigger: &str, broker_sig: SignatureRecord) -> MerkleRoot {
        let mut leaves: Vec<EvidenceHead> = self.heads.values().cloned().collect();
        leaves.sort_by(|a, b| a.capability.cmp(&b.capability));
        let root_hash = merkle_root_hash(&leaves);
        MerkleRoot {
            schema_version: 2,
            kind: "merkle_root".to_string(),
            id: format!("anchor.{epoch}"),
            epoch,
            prev_epoch_id: None,
            prev_epoch_root_hash: None,
            leaves,
            root_hash,
            trigger: trigger.to_string(),
            broker_sig,
        }
    }
}

#[must_use]
pub fn evidence_record_hash(record: &EvidenceRecordV2) -> String {
    let mut projection = serde_json::to_value(record).unwrap_or(Value::Null);
    strip_hash_and_sig(&mut projection);
    canonical_json_sha256(&projection)
}

#[must_use]
pub fn xref_marker_hash(marker: &XrefMarker) -> String {
    let mut projection = serde_json::to_value(marker).unwrap_or(Value::Null);
    strip_hash_and_sig(&mut projection);
    canonical_json_sha256(&projection)
}

#[must_use]
pub fn merkle_root_hash(leaves: &[EvidenceHead]) -> String {
    let mut leaf_hashes: Vec<String> = leaves
        .iter()
        .filter_map(|leaf| leaf.head_hash.clone())
        .collect();
    leaf_hashes.sort();
    canonical_json_sha256(&serde_json::to_value(leaf_hashes).unwrap_or(Value::Null))
}

fn strip_hash_and_sig(value: &mut Value) {
    if let Value::Object(map) = value {
        map.remove("broker_sig");
        map.remove("operator_sig");
        if let Some(Value::Object(chain)) = map.get_mut("sub_chain") {
            chain.insert("self_hash".to_string(), Value::String(String::new()));
        }
    }
}

#[must_use]
pub fn canonical_json_sha256(value: &Value) -> String {
    vac_jcs::canonical_json_sha256(value)
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

#[derive(Debug, Clone)]
pub struct GitRefsEvidenceStore {
    root: std::path::PathBuf,
}

impl GitRefsEvidenceStore {
    #[must_use]
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn ref_path(&self, capability: &str) -> std::path::PathBuf {
        self.root
            .join(".git/refs/vac/evidence")
            .join(capability.replace('.', "/"))
    }

    fn object_path(&self, hash: &str) -> std::path::PathBuf {
        let clean = hash.trim_start_matches("sha256:");
        self.root.join(".vac/registry/evidence/objects").join(clean)
    }

    fn read_ref(&self, capability: &str) -> Option<String> {
        std::fs::read_to_string(self.ref_path(capability))
            .ok()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
    }

    fn write_ref_cas(
        &self,
        capability: &str,
        expected_git_oid: Option<&str>,
        new_ref_value: &str,
    ) -> Result<(), EvidenceStoreError> {
        let current = self.read_ref(capability);
        if current.as_deref() != expected_git_oid {
            return Err(EvidenceStoreError::CasConflict);
        }
        let ref_name = format!("refs/vac/evidence/{}", capability.replace('.', "/"));
        if self.git_dir_exists() {
            if !is_git_object_id(new_ref_value) {
                return Err(EvidenceStoreError::InvalidHash);
            }
            return self.git_update_ref_cas(&ref_name, expected_git_oid, new_ref_value);
        }
        let path = self.ref_path(capability);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| EvidenceStoreError::MissingPreviousHead)?;
        }
        std::fs::write(path, format!("{new_ref_value}\n"))
            .map_err(|_| EvidenceStoreError::MissingPreviousHead)
    }

    fn git_dir_exists(&self) -> bool {
        self.root.join(".git").is_dir()
    }

    fn git_hash_object_write(&self, bytes: &[u8]) -> Result<String, EvidenceStoreError> {
        // Real git refs/vac/evidence/* values must be Git object IDs, not the
        // content SHA-256 carried inside EvidenceRecordV2.sub_chain.self_hash.
        // The JCS/content hash remains the evidence authority; the ref points to
        // a Git blob containing that canonical record.
        let mut child = std::process::Command::new("git")
            .current_dir(&self.root)
            .arg("hash-object")
            .arg("-w")
            .arg("--stdin")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|_| EvidenceStoreError::GitObjectWriteFailed)?;
        {
            let Some(stdin) = child.stdin.as_mut() else {
                return Err(EvidenceStoreError::GitObjectWriteFailed);
            };
            use std::io::Write;
            stdin
                .write_all(bytes)
                .map_err(|_| EvidenceStoreError::GitObjectWriteFailed)?;
        }
        let output = child
            .wait_with_output()
            .map_err(|_| EvidenceStoreError::GitObjectWriteFailed)?;
        if !output.status.success() {
            return Err(EvidenceStoreError::GitObjectWriteFailed);
        }
        let oid = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if is_git_object_id(&oid) {
            Ok(oid)
        } else {
            Err(EvidenceStoreError::InvalidHash)
        }
    }

    fn git_update_ref_cas(
        &self,
        ref_name: &str,
        expected_git_oid: Option<&str>,
        new_git_oid: &str,
    ) -> Result<(), EvidenceStoreError> {
        // Protected refs contract: update refs/vac/* through git update-ref with
        // an expected old Git object id. Never pass VAC content hashes as ref
        // values. Broker custody and branch protection enforce this at L2.
        let mut cmd = std::process::Command::new("git");
        cmd.current_dir(&self.root)
            .arg("update-ref")
            .arg(ref_name)
            .arg(new_git_oid);
        if let Some(old) = expected_git_oid {
            cmd.arg(old);
        } else {
            cmd.arg("");
        }
        let status = cmd
            .status()
            .map_err(|_| EvidenceStoreError::MissingPreviousHead)?;
        if status.success() {
            Ok(())
        } else {
            Err(EvidenceStoreError::CasConflict)
        }
    }

    pub fn append_persistent(
        &self,
        mut record: EvidenceRecordV2,
    ) -> Result<EvidenceHead, EvidenceStoreError> {
        let expected_content_hash = record.sub_chain.prev_hash.clone();
        let expected_git_oid = self.read_ref(&record.capability);
        // L1 source contract: the record still carries the previous content hash
        // for evidence verification. The git ref CAS compares object IDs because
        // refs/vac/evidence/* stores Git blob IDs.
        if expected_content_hash.is_some() && expected_git_oid.is_none() && self.git_dir_exists() {
            return Err(EvidenceStoreError::CasConflict);
        }
        record.sub_chain.self_hash = evidence_record_hash(&record);
        let object_path = self.object_path(&record.sub_chain.self_hash);
        if let Some(parent) = object_path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| EvidenceStoreError::MissingPreviousHead)?;
        }
        let bytes = serde_json::to_vec_pretty(&normalize_json(
            &serde_json::to_value(&record).unwrap_or(Value::Null),
        ))
        .map_err(|_| EvidenceStoreError::InvalidHash)?;
        std::fs::write(&object_path, &bytes)
            .map_err(|_| EvidenceStoreError::MissingPreviousHead)?;
        let ref_value = if self.git_dir_exists() {
            self.git_hash_object_write(&bytes)?
        } else {
            record.sub_chain.self_hash.clone()
        };
        self.write_ref_cas(&record.capability, expected_git_oid.as_deref(), &ref_value)?;
        Ok(EvidenceHead {
            capability: record.capability,
            head_id: Some(record.id),
            head_hash: Some(record.sub_chain.self_hash),
            head_seq: record.seq,
        })
    }

    #[must_use]
    pub fn protected_ref_namespace(&self) -> &'static str {
        "refs/vac/evidence/*"
    }
}

#[must_use]
pub fn is_git_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignatureVerificationInput {
    pub canonical_payload_hash: String,
    pub key_id: String,
    pub algorithm: SigAlgorithm,
    pub signature_value: String,
}

pub trait SignatureVerifier {
    fn verify_detached(
        &self,
        input: &SignatureVerificationInput,
    ) -> Result<(), SignatureVerificationError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SignatureVerificationError {
    IntegrityHintOnly,
    MissingSignatureValue,
    MissingKeyId,
    UnsupportedAlgorithm,
    InvalidSignatureEncoding,
    CryptoBackendUnavailable,
}

pub fn verify_signature_record(
    signature: &SignatureRecord,
) -> Result<(), SignatureVerificationError> {
    match signature.mode {
        SigMode::IntegrityHint => Err(SignatureVerificationError::IntegrityHintOnly),
        SigMode::Signed => {
            if signature.key_id.trim().is_empty() {
                return Err(SignatureVerificationError::MissingKeyId);
            }
            let value = signature.value.as_deref().unwrap_or_default();
            if value.trim().is_empty() {
                return Err(SignatureVerificationError::MissingSignatureValue);
            }
            if !signature_value_has_strict_encoding(signature.algorithm.clone(), value) {
                return Err(SignatureVerificationError::InvalidSignatureEncoding);
            }
            Err(SignatureVerificationError::CryptoBackendUnavailable)
        }
    }
}

#[must_use]
pub fn signature_value_has_strict_encoding(algorithm: SigAlgorithm, value: &str) -> bool {
    match algorithm {
        SigAlgorithm::Ed25519 => value.strip_prefix("ed25519:").is_some_and(|body| {
            body.len() >= 86
                && body.chars().all(|ch| {
                    ch.is_ascii_alphanumeric()
                        || ch == '+'
                        || ch == '/'
                        || ch == '='
                        || ch == '-'
                        || ch == '_'
                })
        }),
        SigAlgorithm::Minisign => {
            value.starts_with("minisign:") && value.len() > "minisign:".len() + 32
        }
        SigAlgorithm::Sigstore => value.starts_with("sigstore:") && value.contains("bundle="),
        SigAlgorithm::Webauthn => {
            value.starts_with("webauthn:") && value.contains("client_data_hash=")
        }
        SigAlgorithm::None => false,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceAuthoritySummary {
    pub mode: String,
    pub signed: bool,
    pub integrity_hint: bool,
    pub warning: Option<String>,
}

#[must_use]
pub fn classify_evidence_authority(value: &serde_json::Value) -> EvidenceAuthoritySummary {
    let mode = value
        .pointer("/broker_sig/mode")
        .or_else(|| value.pointer("/signature/mode"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("integrity_hint")
        .to_string();
    let algorithm = value
        .pointer("/broker_sig/algorithm")
        .or_else(|| value.pointer("/signature/algorithm"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("none");
    let signed = algorithm != "none" && !mode.contains("integrity_hint");
    EvidenceAuthoritySummary {
        mode: mode.clone(),
        signed,
        integrity_hint: !signed,
        warning: if signed {
            None
        } else {
            Some("evidence is integrity-hint, not tamper-evident signing authority".to_string())
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig() -> SignatureRecord {
        SignatureRecord::integrity_hint("test-key")
    }

    fn record(id: &str, capability: &str, seq: u64, prev_hash: Option<String>) -> EvidenceRecordV2 {
        EvidenceRecordV2 {
            schema_version: 2,
            kind: "evidence".to_string(),
            id: id.to_string(),
            capability: capability.to_string(),
            seq,
            session: "session.fixture".to_string(),
            sub_chain: SubChain {
                prev_id: None,
                prev_hash,
                self_hash: String::new(),
            },
            git: GitBinding {
                code_commit: "commit".to_string(),
                parent_commit: "parent".to_string(),
                worktree_ref: "refs/vac/worktree".to_string(),
            },
            cross_capability_refs: vec![],
            approval: ApprovalBinding {
                approval_id: "approval".to_string(),
                content_hash: "sha256:test".to_string(),
            },
            validation: ValidationSummary::default(),
            attribution: Attribution {
                agent_id: "agent".to_string(),
                model: "model".to_string(),
                rationale_ref: "rationale".to_string(),
            },
            seal: SealState { merkle_epoch: None },
            broker_sig: sig(),
        }
    }

    #[test]
    fn append_enforces_cas_previous_head() {
        let mut store = InMemoryEvidenceStore::default();
        let first = store
            .append(record("ev1", "cap", 1, None))
            .expect("first append");

        let conflict = store.append(record("ev2", "cap", 2, Some("wrong".to_string())));
        assert_eq!(conflict, Err(EvidenceStoreError::CasConflict));

        let second = store
            .append(record("ev2", "cap", 2, first.head_hash.clone()))
            .expect("second append");
        assert_eq!(
            store.head("cap").and_then(|h| h.head_id),
            Some("ev2".to_string())
        );
        assert_eq!(second.head_seq, 2);
    }

    #[test]
    fn integrity_hint_is_labeled_as_not_tamper_evident() {
        assert_eq!(
            sig().warning_label(),
            Some("integrity-hint, NOT tamper-evident")
        );
    }
}
