#![allow(dead_code)]
//! Approval request binding and replay-protection contract for VAC-Init.

use base64::prelude::{BASE64_STANDARD, Engine as _};
use ed25519_dalek::{Signature, Verifier as _, VerifyingKey};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
    Timeout,
}

impl ApprovalStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::Timeout => "timeout",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ApprovalDecision {
    Approved,
    Denied,
    Timeout,
}

impl ApprovalDecision {
    pub const fn as_status(self) -> ApprovalStatus {
        match self {
            Self::Approved => ApprovalStatus::Approved,
            Self::Denied => ApprovalStatus::Denied,
            Self::Timeout => ApprovalStatus::Timeout,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalScope {
    pub file: Option<String>,
    pub command: Option<String>,
    pub network_host: Option<String>,
    pub network_protocol: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRequestPayload {
    pub action: String,
    pub risk_level: String,
    pub capability: String,
    pub plan_id: String,
    pub rationale: String,
    pub scope: ApprovalScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalBinding {
    pub plan_hash: String,
    pub diff_hash: String,
    pub policy_snapshot_hash: String,
    pub nonce: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalResponseSignature {
    pub algorithm: String,
    pub public_key_base64: String,
    pub signature_base64: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalSignaturePolicy {
    AllowUnsigned,
    RequireEd25519,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalResponse {
    pub decided_by: String,
    pub decided_at: Option<String>,
    pub decision: ApprovalDecision,
    pub comment: Option<String>,
    pub content_hash: Option<String>,
    pub signature_algorithm: String,
    pub signature: Option<ApprovalResponseSignature>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRequestRecord {
    pub id: String,
    pub timestamp: String,
    pub status: ApprovalStatus,
    pub request: ApprovalRequestPayload,
    pub binding: ApprovalBinding,
    pub response: Option<ApprovalResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalBindingError {
    InvalidId,
    NotApproved,
    PlanHashChanged,
    DiffHashChanged,
    PolicySnapshotHashChanged,
    Expired,
    NonceReplay,
    InvalidHash(&'static str),
    InvalidNonce,
    MissingResponse,
    MissingResponseSignature,
    UnsupportedSignatureAlgorithm(String),
    InvalidSignatureEncoding(&'static str),
    SignatureVerificationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalBindingSnapshot {
    pub plan_hash: String,
    pub diff_hash: String,
    pub policy_snapshot_hash: String,
    pub now: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApprovalReplayStore {
    used_nonces: BTreeSet<String>,
}

impl ApprovalReplayStore {
    pub fn has_nonce(&self, nonce: &str) -> bool {
        self.used_nonces.contains(nonce)
    }

    pub fn mark_nonce(&mut self, nonce: impl Into<String>) {
        self.used_nonces.insert(nonce.into());
    }
}

pub fn validate_approval_binding(
    record: &ApprovalRequestRecord,
    snapshot: &ApprovalBindingSnapshot,
    replay_store: &ApprovalReplayStore,
) -> Result<(), ApprovalBindingError> {
    if !record.id.starts_with("approval.") || record.id.len() <= "approval.".len() {
        return Err(ApprovalBindingError::InvalidId);
    }
    if record.status != ApprovalStatus::Approved {
        return Err(ApprovalBindingError::NotApproved);
    }
    validate_hash(&record.binding.plan_hash, "plan_hash")?;
    validate_hash(&record.binding.diff_hash, "diff_hash")?;
    validate_hash(&record.binding.policy_snapshot_hash, "policy_snapshot_hash")?;
    if !is_nonce_like(&record.binding.nonce) {
        return Err(ApprovalBindingError::InvalidNonce);
    }
    if record.binding.plan_hash != snapshot.plan_hash {
        return Err(ApprovalBindingError::PlanHashChanged);
    }
    if record.binding.diff_hash != snapshot.diff_hash {
        return Err(ApprovalBindingError::DiffHashChanged);
    }
    if record.binding.policy_snapshot_hash != snapshot.policy_snapshot_hash {
        return Err(ApprovalBindingError::PolicySnapshotHashChanged);
    }
    if !snapshot.now.is_empty()
        && !record.binding.expires_at.is_empty()
        && snapshot.now > record.binding.expires_at
    {
        return Err(ApprovalBindingError::Expired);
    }
    if replay_store.has_nonce(&record.binding.nonce) {
        return Err(ApprovalBindingError::NonceReplay);
    }
    Ok(())
}

pub fn consume_approval(
    record: &ApprovalRequestRecord,
    snapshot: &ApprovalBindingSnapshot,
    replay_store: &mut ApprovalReplayStore,
) -> Result<(), ApprovalBindingError> {
    validate_approval_binding(record, snapshot, replay_store)?;
    replay_store.mark_nonce(record.binding.nonce.clone());
    Ok(())
}

pub fn validate_approval_binding_with_signature_policy(
    record: &ApprovalRequestRecord,
    snapshot: &ApprovalBindingSnapshot,
    replay_store: &ApprovalReplayStore,
    signature_policy: ApprovalSignaturePolicy,
) -> Result<(), ApprovalBindingError> {
    validate_approval_binding(record, snapshot, replay_store)?;
    match signature_policy {
        ApprovalSignaturePolicy::AllowUnsigned => Ok(()),
        ApprovalSignaturePolicy::RequireEd25519 => {
            verify_approval_ed25519_signature(record, snapshot)
        }
    }
}

pub fn approval_signature_payload(
    record: &ApprovalRequestRecord,
    snapshot: &ApprovalBindingSnapshot,
) -> String {
    let response = record.response.as_ref();
    let decision = response
        .map(|value| value.decision.as_status().as_str())
        .unwrap_or("missing");
    let content_hash = response
        .and_then(|value| value.content_hash.as_deref())
        .unwrap_or("missing");
    [
        ("approval.id", record.id.as_str()),
        ("approval.status", record.status.as_str()),
        ("binding.plan_hash", record.binding.plan_hash.as_str()),
        ("binding.diff_hash", record.binding.diff_hash.as_str()),
        (
            "binding.policy_snapshot_hash",
            record.binding.policy_snapshot_hash.as_str(),
        ),
        ("binding.nonce", record.binding.nonce.as_str()),
        ("binding.expires_at", record.binding.expires_at.as_str()),
        ("snapshot.plan_hash", snapshot.plan_hash.as_str()),
        ("snapshot.diff_hash", snapshot.diff_hash.as_str()),
        (
            "snapshot.policy_snapshot_hash",
            snapshot.policy_snapshot_hash.as_str(),
        ),
        ("request.action", record.request.action.as_str()),
        ("request.capability", record.request.capability.as_str()),
        ("request.plan_id", record.request.plan_id.as_str()),
        ("request.risk_level", record.request.risk_level.as_str()),
        ("response.decision", decision),
        ("response.content_hash", content_hash),
    ]
    .into_iter()
    .map(|(key, value)| format!("{key}={value}"))
    .collect::<Vec<_>>()
    .join("\n")
}

pub fn verify_approval_ed25519_signature(
    record: &ApprovalRequestRecord,
    snapshot: &ApprovalBindingSnapshot,
) -> Result<(), ApprovalBindingError> {
    let response = record
        .response
        .as_ref()
        .ok_or(ApprovalBindingError::MissingResponse)?;
    let signature = response
        .signature
        .as_ref()
        .ok_or(ApprovalBindingError::MissingResponseSignature)?;
    if signature.algorithm != "ed25519" || response.signature_algorithm != "ed25519" {
        return Err(ApprovalBindingError::UnsupportedSignatureAlgorithm(
            signature.algorithm.clone(),
        ));
    }
    let public_key_bytes = BASE64_STANDARD
        .decode(&signature.public_key_base64)
        .map_err(|_| ApprovalBindingError::InvalidSignatureEncoding("public_key_base64"))?;
    let signature_bytes = BASE64_STANDARD
        .decode(&signature.signature_base64)
        .map_err(|_| ApprovalBindingError::InvalidSignatureEncoding("signature_base64"))?;
    let public_key: [u8; 32] = public_key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| ApprovalBindingError::InvalidSignatureEncoding("public_key_base64"))?;
    let verifying_key = VerifyingKey::from_bytes(&public_key)
        .map_err(|_| ApprovalBindingError::InvalidSignatureEncoding("public_key_base64"))?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|_| ApprovalBindingError::InvalidSignatureEncoding("signature_base64"))?;
    verifying_key
        .verify(
            approval_signature_payload(record, snapshot).as_bytes(),
            &signature,
        )
        .map_err(|_| ApprovalBindingError::SignatureVerificationFailed)
}

fn validate_hash(value: &str, field: &'static str) -> Result<(), ApprovalBindingError> {
    if value.len() != 64 || !value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(ApprovalBindingError::InvalidHash(field));
    }
    Ok(())
}

fn is_nonce_like(value: &str) -> bool {
    let len_ok = value.len() >= 16;
    let chars_ok = value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_');
    len_ok && chars_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn approved_record() -> ApprovalRequestRecord {
        ApprovalRequestRecord {
            id: "approval.123e4567-e89b-12d3-a456-426614174000".to_string(),
            timestamp: "2026-05-29T00:00:00Z".to_string(),
            status: ApprovalStatus::Approved,
            request: ApprovalRequestPayload {
                action: "execute_process".to_string(),
                risk_level: "high".to_string(),
                capability: "vac.test.fixture".to_string(),
                plan_id: "plan.test.fixture".to_string(),
                rationale: "run validation".to_string(),
                scope: ApprovalScope {
                    file: None,
                    command: Some("cargo.test.fixture".to_string()),
                    network_host: None,
                    network_protocol: None,
                },
            },
            binding: ApprovalBinding {
                plan_hash: hash('a'),
                diff_hash: hash('b'),
                policy_snapshot_hash: hash('c'),
                nonce: "nonce-123e4567-e89b-12d3".to_string(),
                expires_at: "2026-05-29T01:00:00Z".to_string(),
            },
            response: Some(ApprovalResponse {
                decided_by: "operator".to_string(),
                decided_at: Some("2026-05-29T00:10:00Z".to_string()),
                decision: ApprovalDecision::Approved,
                comment: None,
                content_hash: Some(hash('d')),
                signature_algorithm: "none".to_string(),
                signature: None,
            }),
        }
    }

    fn snapshot() -> ApprovalBindingSnapshot {
        ApprovalBindingSnapshot {
            plan_hash: hash('a'),
            diff_hash: hash('b'),
            policy_snapshot_hash: hash('c'),
            now: "2026-05-29T00:30:00Z".to_string(),
        }
    }

    #[test]
    fn validates_matching_approved_binding() {
        let store = ApprovalReplayStore::default();
        assert_eq!(
            validate_approval_binding(&approved_record(), &snapshot(), &store),
            Ok(())
        );
    }

    #[test]
    fn rejects_plan_hash_change() {
        let mut snap = snapshot();
        snap.plan_hash = hash('f');
        let err =
            validate_approval_binding(&approved_record(), &snap, &ApprovalReplayStore::default())
                .unwrap_err();
        assert_eq!(err, ApprovalBindingError::PlanHashChanged);
    }

    #[test]
    fn rejects_diff_hash_change() {
        let mut snap = snapshot();
        snap.diff_hash = hash('f');
        let err =
            validate_approval_binding(&approved_record(), &snap, &ApprovalReplayStore::default())
                .unwrap_err();
        assert_eq!(err, ApprovalBindingError::DiffHashChanged);
    }

    #[test]
    fn rejects_policy_snapshot_change() {
        let mut snap = snapshot();
        snap.policy_snapshot_hash = hash('f');
        let err =
            validate_approval_binding(&approved_record(), &snap, &ApprovalReplayStore::default())
                .unwrap_err();
        assert_eq!(err, ApprovalBindingError::PolicySnapshotHashChanged);
    }

    #[test]
    fn rejects_expired_approval() {
        let mut snap = snapshot();
        snap.now = "2026-05-29T02:00:00Z".to_string();
        let err =
            validate_approval_binding(&approved_record(), &snap, &ApprovalReplayStore::default())
                .unwrap_err();
        assert_eq!(err, ApprovalBindingError::Expired);
    }

    #[test]
    fn rejects_replay_nonce() {
        let record = approved_record();
        let mut store = ApprovalReplayStore::default();
        consume_approval(&record, &snapshot(), &mut store).unwrap();
        let err = consume_approval(&record, &snapshot(), &mut store).unwrap_err();
        assert_eq!(err, ApprovalBindingError::NonceReplay);
    }

    #[test]
    fn pending_record_is_not_usable() {
        let mut record = approved_record();
        record.status = ApprovalStatus::Pending;
        let err = validate_approval_binding(&record, &snapshot(), &ApprovalReplayStore::default())
            .unwrap_err();
        assert_eq!(err, ApprovalBindingError::NotApproved);
    }
    #[test]
    fn validates_real_ed25519_signature_when_required() {
        use ed25519_dalek::Signer as _;
        use ed25519_dalek::SigningKey;

        let mut record = approved_record();
        let snapshot = snapshot();
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let verifying_key = signing_key.verifying_key();
        let signature = signing_key.sign(approval_signature_payload(&record, &snapshot).as_bytes());
        let response = record.response.as_mut().unwrap();
        response.signature_algorithm = "ed25519".to_string();
        response.signature = Some(ApprovalResponseSignature {
            algorithm: "ed25519".to_string(),
            public_key_base64: BASE64_STANDARD.encode(verifying_key.to_bytes()),
            signature_base64: BASE64_STANDARD.encode(signature.to_bytes()),
        });

        assert_eq!(
            validate_approval_binding_with_signature_policy(
                &record,
                &snapshot,
                &ApprovalReplayStore::default(),
                ApprovalSignaturePolicy::RequireEd25519,
            ),
            Ok(())
        );
    }

    #[test]
    fn rejects_missing_signature_when_ed25519_required() {
        let err = validate_approval_binding_with_signature_policy(
            &approved_record(),
            &snapshot(),
            &ApprovalReplayStore::default(),
            ApprovalSignaturePolicy::RequireEd25519,
        )
        .unwrap_err();
        assert_eq!(err, ApprovalBindingError::MissingResponseSignature);
    }
}
