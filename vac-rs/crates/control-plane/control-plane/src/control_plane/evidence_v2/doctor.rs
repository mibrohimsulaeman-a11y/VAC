use std::fs;
use std::path::Path;

use serde_yaml::Value;

use super::hash::hash_evidence_v2;
use super::hash::hash_merkle_root;
use super::hash::hash_xref_marker;
use super::migration::evidence_v1_to_v2_migration_path;
use super::signing::verify_signature_payload;
use super::types::EvidenceV2;
use super::types::MerkleRoot;
use super::types::SigMode;
use super::types::SignatureEnvelope;
use super::types::XrefMarker;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvidenceV2DoctorReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub infos: Vec<String>,
}

impl EvidenceV2DoctorReport {
    pub fn exit_code(&self) -> i32 {
        if self.errors.is_empty() { 0 } else { 1 }
    }

    pub fn render_text(&self) -> String {
        let status = if !self.errors.is_empty() {
            "FAIL"
        } else if !self.warnings.is_empty() {
            "WARN"
        } else {
            "PASS"
        };
        let mut lines = vec![format!("vac doctor evidence --v2: {status}")];
        for info in &self.infos {
            lines.push(format!("  INFO: {info}"));
        }
        for warning in &self.warnings {
            lines.push(format!("  WARN: {warning}"));
        }
        for error in &self.errors {
            lines.push(format!("  ERROR: {error}"));
        }
        lines.join("\n")
    }
}

pub fn load_evidence_v2_doctor_report(root: impl AsRef<Path>) -> EvidenceV2DoctorReport {
    let root = root.as_ref();
    let mut report = EvidenceV2DoctorReport::default();
    let migration_path = evidence_v1_to_v2_migration_path(root);
    if migration_path.exists() {
        report
            .infos
            .push(format!("migration: {}", migration_path.display()));
    } else {
        report.warnings.push(format!(
            "missing migration record {}",
            migration_path.display()
        ));
    }

    let store_root = root.join(".vac/registry/evidence-v2");
    if !store_root.exists() {
        report.warnings.push(format!(
            "no evidence v2 store yet at {}; runtime will create it on first append",
            store_root.display()
        ));
        return report;
    }

    let mut records = 0usize;
    collect_yaml_files(&store_root, &mut |path| {
        if path.file_name().and_then(|value| value.to_str()) == Some("head.yaml") {
            return;
        }
        let Ok(source) = fs::read_to_string(path) else {
            report
                .errors
                .push(format!("{}: unreadable evidence v2 record", path.display()));
            return;
        };
        let Ok(value) = serde_yaml::from_str::<Value>(&source) else {
            report
                .errors
                .push(format!("{}: invalid YAML", path.display()));
            return;
        };
        match scalar(&value, "kind").as_deref() {
            Some("evidence") => {
                records += 1;
                match serde_yaml::from_str::<EvidenceV2>(&source) {
                    Ok(record) => verify_evidence(path, &record, &mut report),
                    Err(err) => report.errors.push(format!(
                        "{}: invalid evidence v2 shape: {err}",
                        path.display()
                    )),
                }
            }
            Some("xref_marker") => {
                records += 1;
                match serde_yaml::from_str::<XrefMarker>(&source) {
                    Ok(record) => verify_xref(path, &record, &mut report),
                    Err(err) => report
                        .errors
                        .push(format!("{}: invalid xref v2 shape: {err}", path.display())),
                }
            }
            Some("merkle_root") => match serde_yaml::from_str::<MerkleRoot>(&source) {
                Ok(record) => verify_anchor(path, &record, &mut report),
                Err(err) => report.errors.push(format!(
                    "{}: invalid merkle root shape: {err}",
                    path.display()
                )),
            },
            Some(kind) => report
                .warnings
                .push(format!("{}: ignored v2 kind `{kind}`", path.display())),
            None => report
                .errors
                .push(format!("{}: missing kind", path.display())),
        }
    });

    if records == 0 {
        report
            .warnings
            .push("evidence v2 store exists but has no evidence/xref records".to_string());
    } else {
        report
            .infos
            .push(format!("validated {records} evidence v2 chain record(s)"));
    }
    report
}

fn verify_evidence(path: &Path, record: &EvidenceV2, report: &mut EvidenceV2DoctorReport) {
    if record.schema_version != 2 {
        report.errors.push(format!(
            "{}: evidence schema_version must be 2",
            path.display()
        ));
    }
    if record.kind != "evidence" {
        report
            .errors
            .push(format!("{}: evidence kind mismatch", path.display()));
    }
    match hash_evidence_v2(record) {
        Ok(expected) if expected == record.sub_chain.self_hash => {}
        Ok(expected) => report.errors.push(format!(
            "{}: invalid evidence self_hash expected {expected}",
            path.display()
        )),
        Err(err) => report
            .errors
            .push(format!("{}: hash failure: {err}", path.display())),
    }
    verify_signature(
        path,
        "broker_sig",
        &record.sub_chain.self_hash,
        &record.broker_sig,
        report,
    );
    if let Some(operator_sig) = &record.operator_sig {
        verify_signature(
            path,
            "operator_sig",
            &record.sub_chain.self_hash,
            operator_sig,
            report,
        );
    } else {
        report
            .warnings
            .push(format!("{}: missing operator_sig", path.display()));
    }
}

fn verify_xref(path: &Path, record: &XrefMarker, report: &mut EvidenceV2DoctorReport) {
    if record.schema_version != 2 {
        report
            .errors
            .push(format!("{}: xref schema_version must be 2", path.display()));
    }
    if record.kind != "xref_marker" {
        report
            .errors
            .push(format!("{}: xref kind mismatch", path.display()));
    }
    match hash_xref_marker(record) {
        Ok(expected) if expected == record.sub_chain.self_hash => {}
        Ok(expected) => report.errors.push(format!(
            "{}: invalid xref self_hash expected {expected}",
            path.display()
        )),
        Err(err) => report
            .errors
            .push(format!("{}: hash failure: {err}", path.display())),
    }
    verify_signature(
        path,
        "broker_sig",
        &record.sub_chain.self_hash,
        &record.broker_sig,
        report,
    );
}

fn verify_anchor(path: &Path, record: &MerkleRoot, report: &mut EvidenceV2DoctorReport) {
    if record.schema_version != 2 {
        report.errors.push(format!(
            "{}: merkle root schema_version must be 2",
            path.display()
        ));
    }
    if record.kind != "merkle_root" {
        report
            .errors
            .push(format!("{}: merkle root kind mismatch", path.display()));
    }
    if record.root_hash.is_empty() {
        report
            .errors
            .push(format!("{}: missing root_hash", path.display()));
    }
    if let Err(err) = hash_merkle_root(record) {
        report
            .errors
            .push(format!("{}: merkle hash failure: {err}", path.display()));
    }
    verify_signature(
        path,
        "broker_sig",
        &record.root_hash,
        &record.broker_sig,
        report,
    );
}

fn verify_signature(
    path: &Path,
    field: &str,
    payload: &str,
    envelope: &SignatureEnvelope,
    report: &mut EvidenceV2DoctorReport,
) {
    match verify_signature_payload(payload, envelope) {
        Ok(()) if envelope.mode == SigMode::Signed => report
            .infos
            .push(format!("{}: {field} verified", path.display())),
        Ok(()) => report.warnings.push(format!(
            "{}: {field} is integrity_hint, not a cryptographic signature",
            path.display()
        )),
        Err(err) => report
            .errors
            .push(format!("{}: invalid {field}: {err}", path.display())),
    }
}

fn collect_yaml_files(dir: &Path, visit: &mut impl FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_yaml_files(&path, visit);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("yaml") {
            visit(&path);
        }
    }
}

fn scalar(value: &Value, key: &str) -> Option<String> {
    let value = value.as_mapping()?.get(Value::String(key.to_string()))?;
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control_plane::evidence_v2::signing::EvidenceSigner;
    use crate::control_plane::evidence_v2::store::EvidenceStore;
    use crate::control_plane::evidence_v2::store::GitRefEvidenceStore;
    use crate::control_plane::evidence_v2::types::ApprovalRequestV2;
    use crate::control_plane::evidence_v2::types::EpochTrigger;
    use crate::control_plane::evidence_v2::types::EvidenceV2;
    use crate::control_plane::evidence_v2::types::GitEvidence;

    #[test]
    fn doctor_validates_written_v2_store() {
        let temp = tempfile::tempdir().unwrap();
        let migration_path =
            crate::control_plane::evidence_v2::migration::evidence_v1_to_v2_migration_path(
                temp.path(),
            );
        std::fs::create_dir_all(migration_path.parent().unwrap()).unwrap();
        std::fs::write(
            migration_path,
            crate::control_plane::evidence_v2::migration::render_evidence_v1_to_v2_migration_yaml(),
        )
        .unwrap();

        let store = GitRefEvidenceStore::new(temp.path());
        let capability = "vac.test".to_string();
        store
            .append(
                &capability,
                EvidenceV2::new(
                    &capability,
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
                ),
            )
            .unwrap();
        store.seal_epoch(EpochTrigger::Manual).unwrap();

        let report = load_evidence_v2_doctor_report(temp.path());
        assert_eq!(report.exit_code(), 0, "{}", report.render_text());
        assert!(report.render_text().contains("validated 1 evidence v2"));
    }

    #[test]
    fn doctor_verifies_signed_broker_and_operator_envelopes() {
        let temp = tempfile::tempdir().unwrap();
        let migration_path =
            crate::control_plane::evidence_v2::migration::evidence_v1_to_v2_migration_path(
                temp.path(),
            );
        std::fs::create_dir_all(migration_path.parent().unwrap()).unwrap();
        std::fs::write(
            migration_path,
            crate::control_plane::evidence_v2::migration::render_evidence_v1_to_v2_migration_yaml(),
        )
        .unwrap();

        let store = GitRefEvidenceStore::new_with_signer(
            temp.path(),
            EvidenceSigner::with_broker_and_operator_for_tests([7u8; 32], [8u8; 32]),
        );
        let capability = "vac.test".to_string();
        store
            .append(
                &capability,
                EvidenceV2::new(
                    &capability,
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
                ),
            )
            .unwrap();
        store.seal_epoch(EpochTrigger::Manual).unwrap();

        let report = load_evidence_v2_doctor_report(temp.path());
        assert_eq!(report.exit_code(), 0, "{}", report.render_text());
        assert!(
            report
                .infos
                .iter()
                .any(|info| info.contains("broker_sig verified"))
        );
        assert!(
            report
                .infos
                .iter()
                .any(|info| info.contains("operator_sig verified"))
        );
    }
}
