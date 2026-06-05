use serde::Deserialize;
use serde::Serialize;

pub type CapabilityId = String;
pub type Sha256 = String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubChainLink {
    pub prev_id: Option<String>,
    pub prev_hash: Option<Sha256>,
    pub self_hash: Sha256,
}

impl SubChainLink {
    pub fn empty() -> Self {
        Self {
            prev_id: None,
            prev_hash: None,
            self_hash: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitEvidence {
    pub code_commit: String,
    pub parent_commit: String,
    pub worktree_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequestV2 {
    pub approval_id: String,
    pub content_hash: Sha256,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealRef {
    pub merkle_epoch: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SigAlgorithm {
    Ed25519,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SigMode {
    Signed,
    IntegrityHint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureEnvelope {
    pub algorithm: SigAlgorithm,
    pub key_id: String,
    pub value: Option<String>,
    pub mode: SigMode,
}

impl SignatureEnvelope {
    pub fn integrity_hint(key_id: impl Into<String>) -> Self {
        Self {
            algorithm: SigAlgorithm::None,
            key_id: key_id.into(),
            value: None,
            mode: SigMode::IntegrityHint,
        }
    }

    pub fn signed_ed25519(key_id: impl Into<String>, signature_base64: impl Into<String>) -> Self {
        Self {
            algorithm: SigAlgorithm::Ed25519,
            key_id: key_id.into(),
            value: Some(signature_base64.into()),
            mode: SigMode::Signed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceV2 {
    pub schema_version: u8,
    pub kind: String,
    pub id: String,
    pub capability: CapabilityId,
    pub seq: u64,
    pub session: String,
    pub sub_chain: SubChainLink,
    pub git: GitEvidence,
    pub cross_capability_refs: Vec<String>,
    pub approval: ApprovalRequestV2,
    pub seal: SealRef,
    pub broker_sig: SignatureEnvelope,
    pub operator_sig: Option<SignatureEnvelope>,
}

impl EvidenceV2 {
    pub fn new(
        capability: impl Into<String>,
        seq: u64,
        session: impl Into<String>,
        git: GitEvidence,
        approval: ApprovalRequestV2,
    ) -> Self {
        let capability = capability.into();
        Self {
            schema_version: 2,
            kind: "evidence".to_string(),
            id: format!("evidence.{capability}.{seq}"),
            capability,
            seq,
            session: session.into(),
            sub_chain: SubChainLink::empty(),
            git,
            cross_capability_refs: Vec::new(),
            approval,
            seal: SealRef { merkle_epoch: None },
            broker_sig: SignatureEnvelope::integrity_hint("broker.local"),
            operator_sig: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct XrefMarker {
    pub schema_version: u8,
    pub kind: String,
    pub id: String,
    pub capability: CapabilityId,
    pub seq: u64,
    pub points_to: String,
    pub sub_chain: SubChainLink,
    pub broker_sig: SignatureEnvelope,
}

impl XrefMarker {
    pub fn new(capability: impl Into<String>, seq: u64, points_to: impl Into<String>) -> Self {
        let capability = capability.into();
        Self {
            schema_version: 2,
            kind: "xref_marker".to_string(),
            id: format!("xref.{capability}.{seq}"),
            capability,
            seq,
            points_to: points_to.into(),
            sub_chain: SubChainLink::empty(),
            broker_sig: SignatureEnvelope::integrity_hint("broker.local"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Head {
    pub id: String,
    pub hash: Sha256,
    pub seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviousEpoch {
    pub id: Option<String>,
    pub root_hash: Option<Sha256>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleLeaf {
    pub capability: CapabilityId,
    pub head_id: String,
    pub head_hash: Sha256,
    pub head_seq: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpochTrigger {
    MergeToMain,
    SizeThreshold,
    TimeThreshold,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalAnchor {
    pub kind: String,
    pub ref_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchoredTo {
    pub git_ref: String,
    pub external: ExternalAnchor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleRoot {
    pub schema_version: u8,
    pub kind: String,
    pub id: String,
    pub epoch: u64,
    pub prev_epoch: PreviousEpoch,
    pub leaves: Vec<MerkleLeaf>,
    pub root_hash: Sha256,
    pub trigger: EpochTrigger,
    pub anchored_to: AnchoredTo,
    pub broker_sig: SignatureEnvelope,
}

impl MerkleRoot {
    pub fn new(
        epoch: u64,
        prev_epoch: PreviousEpoch,
        leaves: Vec<MerkleLeaf>,
        trigger: EpochTrigger,
    ) -> Self {
        Self {
            schema_version: 2,
            kind: "merkle_root".to_string(),
            id: format!("anchor.{epoch}"),
            epoch,
            prev_epoch,
            leaves,
            root_hash: String::new(),
            trigger,
            anchored_to: AnchoredTo {
                git_ref: "refs/vac/anchors".to_string(),
                external: ExternalAnchor {
                    kind: "none".to_string(),
                    ref_id: None,
                },
            },
            broker_sig: SignatureEnvelope::integrity_hint("broker.local"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CasConflict {
    HeadMismatch {
        capability: CapabilityId,
        expected_seq: u64,
        actual_seq: u64,
    },
    InvalidRecord(String),
    Io(String),
    Codec(String),
}
