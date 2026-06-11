//! Typed VAC registry transactions initiated from the TUI.
//!
//! The operator console must not edit compiled/effective readiness directly.
//! It may only write `readiness.declared` transactions. The control-plane
//! compiler/doctor layer recomputes `readiness.computed` and lowers
//! `readiness.effective` fail-closed.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeclaredReadiness {
    Planned,
    Partial,
    Ready,
    Deprecated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadinessDeclaredTransaction {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub capability: String,
    pub declared: DeclaredReadiness,
    pub reason: String,
    pub requested_by: String,
    pub requires_compile: bool,
    pub requires_doctor: bool,
    pub requires_evidence: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryTransactionReceipt {
    pub accepted: bool,
    pub transaction_path: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct RegistryTransactionApi {
    root: PathBuf,
}

impl RegistryTransactionApi {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn write_declared_readiness_transaction(
        &self,
        tx: &ReadinessDeclaredTransaction,
    ) -> std::io::Result<RegistryTransactionReceipt> {
        validate_declared_readiness_transaction(tx)?;
        let dir = self
            .root
            .join(".vac/registry/transactions/readiness-declared");
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", sanitize_tx_id(&tx.id)));
        let payload = json!({
            "schema_version": tx.schema_version,
            "kind": tx.kind,
            "id": tx.id,
            "capability": tx.capability,
            "declared": tx.declared,
            "reason": tx.reason,
            "requested_by": tx.requested_by,
            "requires_compile": tx.requires_compile,
            "requires_doctor": tx.requires_doctor,
            "requires_evidence": tx.requires_evidence,
            "runtime_note": "TUI changed declared readiness only; computed/effective readiness remains compiler/doctor-owned",
        });
        let payload_bytes = serde_json::to_vec_pretty(&payload)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        fs::write(&path, payload_bytes)?;
        Ok(RegistryTransactionReceipt {
            accepted: true,
            transaction_path: path.display().to_string(),
            message: "declared readiness transaction written; run compile/doctor/evidence before effective readiness can change".to_string(),
        })
    }
}

pub fn validate_declared_readiness_transaction(
    tx: &ReadinessDeclaredTransaction,
) -> std::io::Result<()> {
    if tx.schema_version != 1 || tx.kind != "readiness_declared_transaction" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid readiness transaction envelope",
        ));
    }
    if tx.capability.trim().is_empty() || !tx.capability.starts_with("vac.") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "readiness transaction must reference a VAC capability id",
        ));
    }
    if tx.reason.trim().len() < 8 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "readiness transaction requires a substantive reason",
        ));
    }
    if !(tx.requires_compile && tx.requires_doctor && tx.requires_evidence) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "declared readiness cannot alter effective readiness without compile, doctor, and evidence gates",
        ));
    }
    Ok(())
}

fn sanitize_tx_id(id: &str) -> String {
    id.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

#[allow(dead_code)]
pub fn default_transaction_dir(root: &Path) -> PathBuf {
    root.join(".vac/registry/transactions/readiness-declared")
}
