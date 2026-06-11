//! Central VAC canonical JSON hashing utilities.
//!
//! This module is the single source for VAC runtime/evidence binding hashes.
//! It implements a deterministic RFC8785/JCS-compatible subset for the current
//! VAC value domain: object keys are sorted lexicographically, arrays preserve
//! order, strings/booleans/null are serialized by `serde_json`, and numbers are
//! accepted only through serde_json's stable representation.  Until full RFC8785
//! test-vector coverage is added, callers must label signatures as
//! `integrity_hint`, not cross-language crypto authority.

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[must_use]
pub fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted = BTreeMap::new();
            for (key, val) in map {
                sorted.insert(key.clone(), canonicalize(val));
            }
            let mut out = Map::new();
            for (key, val) in sorted {
                out.insert(key, val);
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize).collect()),
        other => other.clone(),
    }
}

pub fn to_canonical_vec(value: &Value) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(&canonicalize(value))
}

pub fn to_canonical_pretty_string(value: &Value) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&canonicalize(value))
}

#[must_use]
pub fn canonical_json_sha256(value: &Value) -> String {
    let bytes = to_canonical_vec(value).unwrap_or_default();
    let digest = Sha256::digest(bytes);
    format!("sha256:{digest:x}")
}

#[must_use]
pub fn sha256_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("sha256:{digest:x}")
}

#[must_use]
pub fn implementation_label() -> &'static str {
    "vac_jcs_sorted_json_subset_integrity_hint_until_full_rfc8785_vectors"
}
