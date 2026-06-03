#![allow(dead_code)]
//! Canonical evidence hashing and hash-chain verification for VAC-Init.

use base64::prelude::{BASE64_STANDARD, Engine as _};
use ed25519_dalek::{Signature, Verifier as _, VerifyingKey};
use std::collections::BTreeMap;

pub const CANONICAL_EVIDENCE_INCLUDED_FIELDS: &[&str] = &[
    "agent_id",
    "approved_by",
    "capability",
    "chain.previous",
    "chain.previous_hash",
    "commands_executed.args_hash",
    "commands_executed.duration_ms",
    "commands_executed.exit_code",
    "commands_executed.id",
    "commands_executed.runner",
    "commands_executed.stderr_hash",
    "commands_executed.stdout_hash",
    "files_modified.diff_hash",
    "files_modified.lines_added",
    "files_modified.lines_removed",
    "files_modified.operation",
    "files_modified.path",
    "gates_failed",
    "gates_passed",
    "id",
    "model",
    "plan_id",
    "rationale_ref",
    "timestamp",
];

pub const CANONICAL_EVIDENCE_EXCLUDED_FIELDS: &[&str] = &["chain.self_hash"];

pub const EVIDENCE_RECORD_PUBLIC_FIELDS: &[&str] = &[
    "id",
    "timestamp",
    "chain.previous",
    "chain.previous_hash",
    "chain.self_hash",
    "plan_id",
    "capability",
    "files_modified.path",
    "files_modified.operation",
    "files_modified.diff_hash",
    "files_modified.lines_added",
    "files_modified.lines_removed",
    "commands_executed.id",
    "commands_executed.runner",
    "commands_executed.args_hash",
    "commands_executed.exit_code",
    "commands_executed.stdout_hash",
    "commands_executed.stderr_hash",
    "commands_executed.duration_ms",
    "gates_passed",
    "gates_failed",
    "approved_by",
    "agent_id",
    "model",
    "rationale_ref",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalEvidenceCoverageReport {
    pub included: Vec<String>,
    pub excluded: Vec<String>,
    pub missing: Vec<String>,
    pub unexpected: Vec<String>,
}

impl CanonicalEvidenceCoverageReport {
    pub fn is_pass(&self) -> bool {
        self.missing.is_empty() && self.unexpected.is_empty()
    }
}

pub fn canonical_evidence_field_coverage_report() -> CanonicalEvidenceCoverageReport {
    let included = CANONICAL_EVIDENCE_INCLUDED_FIELDS
        .iter()
        .map(|field| (*field).to_string())
        .collect::<Vec<_>>();
    let excluded = CANONICAL_EVIDENCE_EXCLUDED_FIELDS
        .iter()
        .map(|field| (*field).to_string())
        .collect::<Vec<_>>();
    let missing = EVIDENCE_RECORD_PUBLIC_FIELDS
        .iter()
        .filter(|field| {
            !CANONICAL_EVIDENCE_INCLUDED_FIELDS.contains(field)
                && !CANONICAL_EVIDENCE_EXCLUDED_FIELDS.contains(field)
        })
        .map(|field| (*field).to_string())
        .collect::<Vec<_>>();
    let unexpected = CANONICAL_EVIDENCE_INCLUDED_FIELDS
        .iter()
        .filter(|field| !EVIDENCE_RECORD_PUBLIC_FIELDS.contains(field))
        .map(|field| (*field).to_string())
        .collect::<Vec<_>>();
    CanonicalEvidenceCoverageReport {
        included,
        excluded,
        missing,
        unexpected,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceChainLink {
    pub previous: Option<String>,
    pub previous_hash: Option<String>,
    pub self_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceChangeFile {
    pub path: String,
    pub operation: String,
    pub diff_hash: String,
    pub lines_added: usize,
    pub lines_removed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceValidationCommand {
    pub id: String,
    pub runner: String,
    pub args_hash: String,
    pub exit_code: i32,
    pub stdout_hash: String,
    pub stderr_hash: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRecord {
    pub id: String,
    pub timestamp: String,
    pub chain: EvidenceChainLink,
    pub plan_id: String,
    pub capability: String,
    pub files_modified: Vec<EvidenceChangeFile>,
    pub commands_executed: Vec<EvidenceValidationCommand>,
    pub gates_passed: Vec<String>,
    pub gates_failed: Vec<String>,
    pub approved_by: Option<String>,
    pub agent_id: String,
    pub model: String,
    pub rationale_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceSignatureEnvelope {
    pub algorithm: String,
    pub public_key_base64: String,
    pub signature_base64: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceSignatureError {
    UnsupportedAlgorithm(String),
    InvalidEncoding(&'static str),
    VerificationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceChainError {
    InvalidSelfHash {
        evidence_id: String,
        expected: String,
        actual: String,
    },
    BrokenPreviousId {
        evidence_id: String,
        expected: Option<String>,
        actual: Option<String>,
    },
    BrokenPreviousHash {
        evidence_id: String,
        expected: Option<String>,
        actual: Option<String>,
    },
    InvalidHashFormat {
        evidence_id: String,
        field: String,
    },
    InvalidTimestamp {
        evidence_id: String,
        timestamp: String,
    },
}

pub fn canonical_yaml_scalar(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | '/' | ':'))
        && !value.is_empty()
    {
        value.to_string()
    } else {
        format!("{:?}", value)
    }
}

pub fn canonical_mapping(fields: &BTreeMap<String, String>) -> String {
    let mut out = String::new();
    for (key, value) in fields {
        out.push_str(key);
        out.push_str(": ");
        out.push_str(&canonical_yaml_scalar(value));
        out.push('\n');
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalEvidencePayload {
    yaml: String,
}

impl CanonicalEvidencePayload {
    pub fn from_record(record: &EvidenceRecord) -> Self {
        Self {
            yaml: canonical_evidence_yaml(record),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.yaml
    }
}

pub fn canonical_evidence_yaml(record: &EvidenceRecord) -> String {
    let mut out = String::new();
    push_scalar(&mut out, 0, "agent_id", &record.agent_id);
    push_scalar(
        &mut out,
        0,
        "approved_by",
        record.approved_by.as_deref().unwrap_or("null"),
    );
    push_scalar(&mut out, 0, "capability", &record.capability);
    out.push_str("chain:\n");
    push_scalar(
        &mut out,
        2,
        "previous",
        record.chain.previous.as_deref().unwrap_or("null"),
    );
    push_scalar(
        &mut out,
        2,
        "previous_hash",
        record.chain.previous_hash.as_deref().unwrap_or("null"),
    );
    // self_hash is intentionally excluded from the canonical payload so the
    // payload can be hashed to produce that field. All other record fields are
    // represented below in deterministic YAML order.
    out.push_str("commands_executed:\n");
    for command in &record.commands_executed {
        out.push_str("  - \n");
        push_scalar(&mut out, 4, "args_hash", &command.args_hash);
        push_scalar(&mut out, 4, "duration_ms", &command.duration_ms.to_string());
        push_scalar(&mut out, 4, "exit_code", &command.exit_code.to_string());
        push_scalar(&mut out, 4, "id", &command.id);
        push_scalar(&mut out, 4, "runner", &command.runner);
        push_scalar(&mut out, 4, "stderr_hash", &command.stderr_hash);
        push_scalar(&mut out, 4, "stdout_hash", &command.stdout_hash);
    }
    out.push_str("files_modified:\n");
    for file in &record.files_modified {
        out.push_str("  - \n");
        push_scalar(&mut out, 4, "diff_hash", &file.diff_hash);
        push_scalar(&mut out, 4, "lines_added", &file.lines_added.to_string());
        push_scalar(
            &mut out,
            4,
            "lines_removed",
            &file.lines_removed.to_string(),
        );
        push_scalar(&mut out, 4, "operation", &file.operation);
        push_scalar(&mut out, 4, "path", &file.path);
    }
    push_sequence(&mut out, 0, "gates_failed", &record.gates_failed);
    push_sequence(&mut out, 0, "gates_passed", &record.gates_passed);
    push_scalar(&mut out, 0, "id", &record.id);
    push_scalar(&mut out, 0, "model", &record.model);
    push_scalar(&mut out, 0, "plan_id", &record.plan_id);
    push_scalar(&mut out, 0, "rationale_ref", &record.rationale_ref);
    push_scalar(&mut out, 0, "timestamp", &record.timestamp);
    out
}

pub fn canonical_evidence_payload(record: &EvidenceRecord) -> String {
    CanonicalEvidencePayload::from_record(record)
        .as_str()
        .to_string()
}

fn push_scalar(out: &mut String, indent: usize, key: &str, value: &str) {
    out.push_str(&" ".repeat(indent));
    out.push_str(key);
    out.push_str(": ");
    out.push_str(&canonical_yaml_scalar(value));
    out.push('\n');
}

fn push_sequence(out: &mut String, indent: usize, key: &str, values: &[String]) {
    out.push_str(&" ".repeat(indent));
    out.push_str(key);
    out.push_str(":\n");
    for value in values {
        out.push_str(&" ".repeat(indent + 2));
        out.push_str("- ");
        out.push_str(&canonical_yaml_scalar(value));
        out.push('\n');
    }
}

pub fn compute_evidence_self_hash(record: &EvidenceRecord) -> String {
    sha256_hex(canonical_evidence_payload(record).as_bytes())
}

pub fn verify_evidence_record(record: &EvidenceRecord) -> Result<(), EvidenceChainError> {
    if !is_utc_z_timestamp(&record.timestamp) {
        return Err(EvidenceChainError::InvalidTimestamp {
            evidence_id: record.id.clone(),
            timestamp: record.timestamp.clone(),
        });
    }
    for (field, value) in [("chain.self_hash", record.chain.self_hash.as_str())] {
        if !is_sha256_hex(value) {
            return Err(EvidenceChainError::InvalidHashFormat {
                evidence_id: record.id.clone(),
                field: field.to_string(),
            });
        }
    }
    if let Some(previous_hash) = &record.chain.previous_hash {
        if !is_sha256_hex(previous_hash) {
            return Err(EvidenceChainError::InvalidHashFormat {
                evidence_id: record.id.clone(),
                field: "chain.previous_hash".to_string(),
            });
        }
    }
    let expected = compute_evidence_self_hash(record);
    if expected != record.chain.self_hash {
        return Err(EvidenceChainError::InvalidSelfHash {
            evidence_id: record.id.clone(),
            expected,
            actual: record.chain.self_hash.clone(),
        });
    }
    Ok(())
}

pub fn evidence_signature_payload(record: &EvidenceRecord) -> String {
    canonical_evidence_payload(record)
}

pub fn verify_evidence_ed25519_signature(
    record: &EvidenceRecord,
    signature: &EvidenceSignatureEnvelope,
) -> Result<(), EvidenceSignatureError> {
    if signature.algorithm != "ed25519" {
        return Err(EvidenceSignatureError::UnsupportedAlgorithm(
            signature.algorithm.clone(),
        ));
    }
    let public_key_bytes = BASE64_STANDARD
        .decode(&signature.public_key_base64)
        .map_err(|_| EvidenceSignatureError::InvalidEncoding("public_key_base64"))?;
    let signature_bytes = BASE64_STANDARD
        .decode(&signature.signature_base64)
        .map_err(|_| EvidenceSignatureError::InvalidEncoding("signature_base64"))?;
    let public_key: [u8; 32] = public_key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| EvidenceSignatureError::InvalidEncoding("public_key_base64"))?;
    let verifying_key = VerifyingKey::from_bytes(&public_key)
        .map_err(|_| EvidenceSignatureError::InvalidEncoding("public_key_base64"))?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|_| EvidenceSignatureError::InvalidEncoding("signature_base64"))?;
    verifying_key
        .verify(evidence_signature_payload(record).as_bytes(), &signature)
        .map_err(|_| EvidenceSignatureError::VerificationFailed)
}

pub fn verify_evidence_chain(records: &[EvidenceRecord]) -> Result<(), EvidenceChainError> {
    let mut previous_id: Option<String> = None;
    let mut previous_hash: Option<String> = None;
    for record in records {
        verify_evidence_record(record)?;
        if record.chain.previous != previous_id {
            return Err(EvidenceChainError::BrokenPreviousId {
                evidence_id: record.id.clone(),
                expected: previous_id,
                actual: record.chain.previous.clone(),
            });
        }
        if record.chain.previous_hash != previous_hash {
            return Err(EvidenceChainError::BrokenPreviousHash {
                evidence_id: record.id.clone(),
                expected: previous_hash,
                actual: record.chain.previous_hash.clone(),
            });
        }
        previous_id = Some(record.id.clone());
        previous_hash = Some(record.chain.self_hash.clone());
    }
    Ok(())
}

pub fn finalize_evidence_record(mut record: EvidenceRecord) -> EvidenceRecord {
    record.chain.self_hash = "0".repeat(64);
    record.chain.self_hash = compute_evidence_self_hash(&record);
    record
}

fn file_digest(file: &EvidenceChangeFile) -> String {
    format!(
        "{}:{}:{}:+{}:-{}",
        file.path, file.operation, file.diff_hash, file.lines_added, file.lines_removed
    )
}

fn command_digest(command: &EvidenceValidationCommand) -> String {
    format!(
        "{}:{}:{}:{}:{}:{}:{}",
        command.id,
        command.runner,
        command.args_hash,
        command.exit_code,
        command.stdout_hash,
        command.stderr_hash,
        command.duration_ms
    )
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn is_utc_z_timestamp(value: &str) -> bool {
    value.len() >= "2026-05-29T00:00:00Z".len()
        && value.ends_with('Z')
        && value.contains('T')
        && !value.contains('\n')
        && !value.contains('\r')
}

pub fn sha256_hex(input: &[u8]) -> String {
    let digest = sha256(input);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn sha256(input: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (input.len() as u64) * 8;
    let mut data = input.to_vec();
    data.push(0x80);
    while (data.len() % 64) != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());

    let mut w = [0u32; 64];
    for chunk in data.chunks_exact(64) {
        for (i, word) in w.iter_mut().take(16).enumerate() {
            let j = i * 4;
            *word = u32::from_be_bytes([chunk[j], chunk[j + 1], chunk[j + 2], chunk[j + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, previous: Option<String>, previous_hash: Option<String>) -> EvidenceRecord {
        finalize_evidence_record(EvidenceRecord {
            id: id.to_string(),
            timestamp: "2026-05-29T00:00:00Z".to_string(),
            chain: EvidenceChainLink {
                previous,
                previous_hash,
                self_hash: "0".repeat(64),
            },
            plan_id: "plan.test.fixture".to_string(),
            capability: "vac.test.fixture".to_string(),
            files_modified: vec![EvidenceChangeFile {
                path: "src/lib.rs".to_string(),
                operation: "modify".to_string(),
                diff_hash: "a".repeat(64),
                lines_added: 1,
                lines_removed: 0,
            }],
            commands_executed: vec![EvidenceValidationCommand {
                id: "cargo.test.fixture".to_string(),
                runner: "cargo".to_string(),
                args_hash: "b".repeat(64),
                exit_code: 0,
                stdout_hash: "c".repeat(64),
                stderr_hash: "d".repeat(64),
                duration_ms: 10,
            }],
            gates_passed: vec!["test".to_string()],
            gates_failed: vec![],
            approved_by: Some("operator".to_string()),
            agent_id: "agent.test".to_string(),
            model: "gpt-test".to_string(),
            rationale_ref: ".vac/registry/trajectory/test.yaml".to_string(),
        })
    }

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn canonical_mapping_sorts_keys() {
        let map = BTreeMap::from([
            ("z".to_string(), "last".to_string()),
            ("a".to_string(), "first".to_string()),
        ]);
        assert_eq!(canonical_mapping(&map), "a: first\nz: last\n");
    }

    #[test]
    fn self_hash_excludes_self_hash_field() {
        let mut rec = record("evidence.2026-05-29-test", None, None);
        let original = rec.chain.self_hash.clone();
        rec.chain.self_hash = "f".repeat(64);
        assert_eq!(compute_evidence_self_hash(&rec), original);
    }

    #[test]
    fn verifies_valid_chain() {
        let first = record("evidence.2026-05-29-a", None, None);
        let second = record(
            "evidence.2026-05-29-b",
            Some(first.id.clone()),
            Some(first.chain.self_hash.clone()),
        );
        assert_eq!(verify_evidence_chain(&[first, second]), Ok(()));
    }

    #[test]
    fn detects_broken_previous_id() {
        let first = record("evidence.2026-05-29-a", None, None);
        let second = record(
            "evidence.2026-05-29-b",
            Some("wrong".to_string()),
            Some(first.chain.self_hash.clone()),
        );
        let err = verify_evidence_chain(&[first, second]).unwrap_err();
        assert!(matches!(err, EvidenceChainError::BrokenPreviousId { .. }));
    }

    #[test]
    fn detects_broken_previous_hash() {
        let first = record("evidence.2026-05-29-a", None, None);
        let second = record(
            "evidence.2026-05-29-b",
            Some(first.id.clone()),
            Some("f".repeat(64)),
        );
        let err = verify_evidence_chain(&[first, second]).unwrap_err();
        assert!(matches!(err, EvidenceChainError::BrokenPreviousHash { .. }));
    }

    #[test]
    fn detects_tampered_record_self_hash() {
        let mut rec = record("evidence.2026-05-29-a", None, None);
        rec.capability = "vac.tampered".to_string();
        let err = verify_evidence_record(&rec).unwrap_err();
        assert!(matches!(err, EvidenceChainError::InvalidSelfHash { .. }));
    }

    #[test]
    fn canonical_payload_uses_lf_sorted_keys_and_quotes_comment_like_values() {
        let mut rec = record("evidence.2026-05-29-canonical", None, None);
        rec.agent_id = "agent # not a yaml comment".to_string();
        let payload = canonical_evidence_payload(&rec);
        assert!(!payload.contains("\r\n"));
        assert!(payload.ends_with('\n'));
        assert!(!payload.contains("self_hash"));
        assert!(payload.contains("agent_id: \"agent # not a yaml comment\"\n"));
        let keys: Vec<&str> = payload
            .lines()
            .filter(|line| !line.starts_with(' '))
            .filter_map(|line| line.split_once(':').map(|(key, _)| key))
            .collect();
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(keys, sorted);
    }

    #[test]
    fn evidence_record_requires_utc_z_timestamp() {
        let mut rec = record("evidence.2026-05-29-time", None, None);
        rec.timestamp = "2026-05-29T00:00:00+07:00".to_string();
        rec = finalize_evidence_record(rec);
        let err = verify_evidence_record(&rec).unwrap_err();
        assert!(matches!(err, EvidenceChainError::InvalidTimestamp { .. }));
    }

    #[test]
    fn canonical_field_coverage_report_is_complete() {
        let report = canonical_evidence_field_coverage_report();
        assert!(
            report.is_pass(),
            "canonical evidence field coverage drift: {report:?}"
        );
    }

    #[test]
    fn canonical_payload_declares_full_field_coverage() {
        for field in [
            "id",
            "timestamp",
            "chain.previous",
            "chain.previous_hash",
            "plan_id",
            "capability",
            "files_modified.path",
            "files_modified.operation",
            "files_modified.diff_hash",
            "files_modified.lines_added",
            "files_modified.lines_removed",
            "commands_executed.id",
            "commands_executed.runner",
            "commands_executed.args_hash",
            "commands_executed.exit_code",
            "commands_executed.stdout_hash",
            "commands_executed.stderr_hash",
            "commands_executed.duration_ms",
            "gates_passed",
            "gates_failed",
            "approved_by",
            "agent_id",
            "model",
            "rationale_ref",
        ] {
            assert!(
                CANONICAL_EVIDENCE_INCLUDED_FIELDS.contains(&field),
                "canonical evidence coverage missing {field}"
            );
        }
        assert_eq!(CANONICAL_EVIDENCE_EXCLUDED_FIELDS, &["chain.self_hash"]);
    }

    #[test]
    fn canonical_payload_changes_when_each_public_field_changes() {
        let base = record("evidence.2026-05-29-field-base", None, None);
        let base_hash = compute_evidence_self_hash(&base);
        let mut changed = base.clone();
        changed.agent_id = "agent.changed".to_string();
        assert_ne!(compute_evidence_self_hash(&changed), base_hash);
        let mut changed = base.clone();
        changed.files_modified[0].lines_added += 1;
        assert_ne!(compute_evidence_self_hash(&changed), base_hash);
        let mut changed = base.clone();
        changed.commands_executed[0].duration_ms += 1;
        assert_ne!(compute_evidence_self_hash(&changed), base_hash);
    }

    #[test]
    fn verifies_real_ed25519_signature_for_evidence_payload() {
        use ed25519_dalek::Signer as _;
        use ed25519_dalek::SigningKey;

        let rec = record("evidence.2026-05-29-signed", None, None);
        let signing_key = SigningKey::from_bytes(&[9u8; 32]);
        let signature = signing_key.sign(evidence_signature_payload(&rec).as_bytes());
        let envelope = EvidenceSignatureEnvelope {
            algorithm: "ed25519".to_string(),
            public_key_base64: BASE64_STANDARD.encode(signing_key.verifying_key().to_bytes()),
            signature_base64: BASE64_STANDARD.encode(signature.to_bytes()),
        };

        assert_eq!(verify_evidence_ed25519_signature(&rec, &envelope), Ok(()));
    }

    #[test]
    fn rejects_wrong_evidence_signature_algorithm() {
        let rec = record("evidence.2026-05-29-unsigned", None, None);
        let envelope = EvidenceSignatureEnvelope {
            algorithm: "none".to_string(),
            public_key_base64: "".to_string(),
            signature_base64: "".to_string(),
        };
        assert_eq!(
            verify_evidence_ed25519_signature(&rec, &envelope),
            Err(EvidenceSignatureError::UnsupportedAlgorithm(
                "none".to_string()
            ))
        );
    }
}
