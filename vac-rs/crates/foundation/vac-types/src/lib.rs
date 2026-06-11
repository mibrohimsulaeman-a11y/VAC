#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CapabilityId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlanId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvidenceId(pub String);

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Sha256Hex(pub String);

impl fmt::Debug for Sha256Hex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Sha256Hex").field(&self.0).finish()
    }
}

pub fn sha256_hex(bytes: impl AsRef<[u8]>) -> Sha256Hex {
    let digest = Sha256::digest(bytes.as_ref());
    Sha256Hex(format!("sha256:{digest:x}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hashes_are_prefixed() {
        assert!(sha256_hex(b"vac").0.starts_with("sha256:"));
    }
}
