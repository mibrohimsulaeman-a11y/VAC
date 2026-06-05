use serde::Serialize;
use serde_json::Value;

use super::jcs::canonicalize_value;
use super::types::EvidenceV2;
use super::types::MerkleRoot;
use super::types::XrefMarker;

pub fn hash_evidence_v2(record: &EvidenceV2) -> Result<String, String> {
    hash_serializable_without_volatile_fields(record)
}

pub fn hash_xref_marker(record: &XrefMarker) -> Result<String, String> {
    hash_serializable_without_volatile_fields(record)
}

pub fn hash_merkle_root(record: &MerkleRoot) -> Result<String, String> {
    hash_serializable_without_volatile_fields(record)
}

pub fn hash_serializable_without_volatile_fields(value: &impl Serialize) -> Result<String, String> {
    let mut value = serde_json::to_value(value).map_err(|err| err.to_string())?;
    prune_volatile_hash_fields(&mut value);
    let canonical = canonicalize_value(&value);
    Ok(super::super::vac_init_evidence_chain::sha256_hex(
        canonical.as_bytes(),
    ))
}

fn prune_volatile_hash_fields(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.remove("self_hash");
            map.remove("root_hash");
            map.remove("broker_sig");
            map.remove("operator_sig");
            for value in map.values_mut() {
                prune_volatile_hash_fields(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                prune_volatile_hash_fields(value);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control_plane::evidence_v2::types::ApprovalRequestV2;
    use crate::control_plane::evidence_v2::types::EvidenceV2;
    use crate::control_plane::evidence_v2::types::GitEvidence;
    use crate::control_plane::evidence_v2::types::SignatureEnvelope;

    fn fixture() -> EvidenceV2 {
        EvidenceV2::new(
            "vac.test",
            1,
            "session.test",
            GitEvidence {
                code_commit: "abc".to_string(),
                parent_commit: "def".to_string(),
                worktree_ref: "refs/heads/main".to_string(),
            },
            ApprovalRequestV2 {
                approval_id: "approval.test".to_string(),
                content_hash: "0".repeat(64),
            },
        )
    }

    #[test]
    fn evidence_hash_ignores_self_hash_and_signatures() {
        let mut left = fixture();
        left.sub_chain.self_hash = "a".repeat(64);
        let mut right = fixture();
        right.sub_chain.self_hash = "b".repeat(64);
        right.broker_sig = SignatureEnvelope::integrity_hint("changed");

        assert_eq!(
            hash_evidence_v2(&left).unwrap(),
            hash_evidence_v2(&right).unwrap()
        );
    }
}
