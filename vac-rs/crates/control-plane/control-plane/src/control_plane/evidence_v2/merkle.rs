use super::types::MerkleLeaf;

pub fn calculate_merkle_root(leaves: &[MerkleLeaf]) -> String {
    if leaves.is_empty() {
        return super::super::vac_init_evidence_chain::sha256_hex(b"vac-empty-merkle-root-v2");
    }

    let mut level = leaves
        .iter()
        .map(|leaf| {
            super::super::vac_init_evidence_chain::sha256_hex(
                format!(
                    "{}:{}:{}:{}",
                    leaf.capability, leaf.head_id, leaf.head_hash, leaf.head_seq
                )
                .as_bytes(),
            )
        })
        .collect::<Vec<_>>();
    level.sort();

    while level.len() > 1 {
        let mut next = Vec::new();
        for pair in level.chunks(2) {
            let right = pair.get(1).unwrap_or(&pair[0]);
            next.push(super::super::vac_init_evidence_chain::sha256_hex(
                format!("{}{}", pair[0], right).as_bytes(),
            ));
        }
        level = next;
    }

    level.remove(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control_plane::evidence_v2::types::MerkleLeaf;

    #[test]
    fn merkle_root_is_order_independent_for_capability_heads() {
        let mut leaves = vec![
            MerkleLeaf {
                capability: "vac.b".to_string(),
                head_id: "evidence.vac.b.1".to_string(),
                head_hash: "b".repeat(64),
                head_seq: 1,
            },
            MerkleLeaf {
                capability: "vac.a".to_string(),
                head_id: "evidence.vac.a.1".to_string(),
                head_hash: "a".repeat(64),
                head_seq: 1,
            },
        ];
        let first = calculate_merkle_root(&leaves);
        leaves.reverse();
        assert_eq!(first, calculate_merkle_root(&leaves));
    }
}
