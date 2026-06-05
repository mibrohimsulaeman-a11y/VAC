use std::fs;
use std::path::Path;
use std::path::PathBuf;

use serde::Serialize;
use serde_yaml::Value;

use super::hash::hash_evidence_v2;
use super::hash::hash_merkle_root;
use super::hash::hash_xref_marker;
use super::merkle::calculate_merkle_root;
use super::signing::EvidenceSigner;
use super::types::CapabilityId;
use super::types::CasConflict;
use super::types::EpochTrigger;
use super::types::EvidenceV2;
use super::types::Head;
use super::types::MerkleLeaf;
use super::types::MerkleRoot;
use super::types::PreviousEpoch;
use super::types::XrefMarker;

pub trait EvidenceStore {
    fn append(&self, capability: &CapabilityId, record: EvidenceV2) -> Result<Head, CasConflict>;
    fn append_xref(
        &self,
        capability: &CapabilityId,
        marker: XrefMarker,
    ) -> Result<Head, CasConflict>;
    fn read_head(&self, capability: &CapabilityId) -> Result<Option<Head>, CasConflict>;
    fn seal_epoch(&self, trigger: EpochTrigger) -> Result<MerkleRoot, CasConflict>;
}

#[derive(Debug, Clone)]
pub struct GitRefEvidenceStore {
    root: PathBuf,
    signer: EvidenceSigner,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvidenceV2ResignReport {
    pub evidence_records: usize,
    pub xref_markers: usize,
    pub anchors: usize,
    pub rewritten_files: Vec<PathBuf>,
}

impl GitRefEvidenceStore {
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            root: workspace_root.as_ref().join(".vac/registry/evidence-v2"),
            signer: EvidenceSigner::from_env(),
        }
    }

    pub fn new_with_signer(workspace_root: impl AsRef<Path>, signer: EvidenceSigner) -> Self {
        Self {
            root: workspace_root.as_ref().join(".vac/registry/evidence-v2"),
            signer,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn resign_existing_records(&self) -> Result<EvidenceV2ResignReport, CasConflict> {
        if !self.signer.has_broker() || !self.signer.has_operator() {
            return Err(CasConflict::InvalidRecord(
                "evidence v2 resign requires both broker and operator Ed25519 keys".to_string(),
            ));
        }
        if !self.root.exists() {
            return Ok(EvidenceV2ResignReport::default());
        }

        let mut report = EvidenceV2ResignReport::default();
        let mut paths = Vec::new();
        collect_yaml_files_sorted(&self.root, &mut paths).map_err(CasConflict::Io)?;
        for path in paths {
            let source =
                fs::read_to_string(&path).map_err(|err| CasConflict::Io(err.to_string()))?;
            let value: Value =
                serde_yaml::from_str(&source).map_err(|err| CasConflict::Codec(err.to_string()))?;
            match yaml_kind(&value).as_deref() {
                Some("evidence") => {
                    let mut record: EvidenceV2 = serde_yaml::from_str(&source)
                        .map_err(|err| CasConflict::Codec(err.to_string()))?;
                    self.signer.sign_evidence(&mut record);
                    write_yaml_atomic(&path, &record)?;
                    report.evidence_records += 1;
                    report.rewritten_files.push(path);
                }
                Some("xref_marker") => {
                    let mut marker: XrefMarker = serde_yaml::from_str(&source)
                        .map_err(|err| CasConflict::Codec(err.to_string()))?;
                    self.signer.sign_xref(&mut marker);
                    write_yaml_atomic(&path, &marker)?;
                    report.xref_markers += 1;
                    report.rewritten_files.push(path);
                }
                Some("merkle_root") => {
                    let mut root: MerkleRoot = serde_yaml::from_str(&source)
                        .map_err(|err| CasConflict::Codec(err.to_string()))?;
                    self.signer.sign_merkle_root(&mut root);
                    write_yaml_atomic(&path, &root)?;
                    report.anchors += 1;
                    report.rewritten_files.push(path);
                }
                _ => {}
            }
        }
        Ok(report)
    }

    fn capability_dir(&self, capability: &str) -> PathBuf {
        self.root
            .join("capabilities")
            .join(sanitize_path_segment(capability))
    }

    fn head_path(&self, capability: &str) -> PathBuf {
        self.capability_dir(capability).join("head.yaml")
    }

    fn record_path(&self, capability: &str, seq: u64, kind: &str) -> PathBuf {
        self.capability_dir(capability)
            .join(format!("{seq:020}.{kind}.yaml"))
    }

    fn anchor_dir(&self) -> PathBuf {
        self.root.join("anchors")
    }

    fn anchor_head_path(&self) -> PathBuf {
        self.anchor_dir().join("head.yaml")
    }

    fn anchor_path(&self, epoch: u64) -> PathBuf {
        self.anchor_dir().join(format!("{epoch:020}.yaml"))
    }

    fn read_anchor_head(&self) -> Result<Option<MerkleRoot>, CasConflict> {
        let path = self.anchor_head_path();
        if !path.exists() {
            return Ok(None);
        }
        let source = fs::read_to_string(&path).map_err(|err| CasConflict::Io(err.to_string()))?;
        serde_yaml::from_str(&source).map_err(|err| CasConflict::Codec(err.to_string()))
    }

    fn append_head(
        &self,
        capability: &CapabilityId,
        id: String,
        seq: u64,
        hash: String,
        kind: &str,
        write: impl FnOnce(&Path) -> Result<(), CasConflict>,
    ) -> Result<Head, CasConflict> {
        let previous = self.read_head(capability)?;
        let expected_seq = previous.as_ref().map(|head| head.seq + 1).unwrap_or(1);
        if seq != expected_seq {
            return Err(CasConflict::HeadMismatch {
                capability: capability.clone(),
                expected_seq,
                actual_seq: seq,
            });
        }

        let path = self.record_path(capability, seq, kind);
        write(&path)?;
        let head = Head { id, hash, seq };
        write_yaml_atomic(&self.head_path(capability), &head)?;
        Ok(head)
    }
}

impl EvidenceStore for GitRefEvidenceStore {
    fn append(
        &self,
        capability: &CapabilityId,
        mut record: EvidenceV2,
    ) -> Result<Head, CasConflict> {
        if &record.capability != capability {
            return Err(CasConflict::InvalidRecord(format!(
                "record capability `{}` does not match append capability `{capability}`",
                record.capability
            )));
        }
        let previous = self.read_head(capability)?;
        record.sub_chain.prev_id = previous.as_ref().map(|head| head.id.clone());
        record.sub_chain.prev_hash = previous.as_ref().map(|head| head.hash.clone());
        record.sub_chain.self_hash =
            hash_evidence_v2(&record).map_err(CasConflict::InvalidRecord)?;
        self.signer.sign_evidence(&mut record);
        let id = record.id.clone();
        let hash = record.sub_chain.self_hash.clone();
        let seq = record.seq;
        self.append_head(capability, id, seq, hash, "evidence", |path| {
            write_yaml_atomic(path, &record)
        })
    }

    fn append_xref(
        &self,
        capability: &CapabilityId,
        mut marker: XrefMarker,
    ) -> Result<Head, CasConflict> {
        if &marker.capability != capability {
            return Err(CasConflict::InvalidRecord(format!(
                "xref capability `{}` does not match append capability `{capability}`",
                marker.capability
            )));
        }
        let previous = self.read_head(capability)?;
        marker.sub_chain.prev_id = previous.as_ref().map(|head| head.id.clone());
        marker.sub_chain.prev_hash = previous.as_ref().map(|head| head.hash.clone());
        marker.sub_chain.self_hash =
            hash_xref_marker(&marker).map_err(CasConflict::InvalidRecord)?;
        self.signer.sign_xref(&mut marker);
        let id = marker.id.clone();
        let hash = marker.sub_chain.self_hash.clone();
        let seq = marker.seq;
        self.append_head(capability, id, seq, hash, "xref", |path| {
            write_yaml_atomic(path, &marker)
        })
    }

    fn read_head(&self, capability: &CapabilityId) -> Result<Option<Head>, CasConflict> {
        let path = self.head_path(capability);
        if !path.exists() {
            return Ok(None);
        }
        let source = fs::read_to_string(&path).map_err(|err| CasConflict::Io(err.to_string()))?;
        serde_yaml::from_str(&source).map_err(|err| CasConflict::Codec(err.to_string()))
    }

    fn seal_epoch(&self, trigger: EpochTrigger) -> Result<MerkleRoot, CasConflict> {
        let previous = self.read_anchor_head()?;
        let epoch = previous.as_ref().map(|root| root.epoch + 1).unwrap_or(1);
        let prev_epoch = PreviousEpoch {
            id: previous.as_ref().map(|root| root.id.clone()),
            root_hash: previous.as_ref().map(|root| root.root_hash.clone()),
        };

        let mut leaves = Vec::new();
        let capabilities_dir = self.root.join("capabilities");
        if capabilities_dir.exists() {
            for entry in fs::read_dir(&capabilities_dir)
                .map_err(|err| CasConflict::Io(err.to_string()))?
                .flatten()
            {
                let head_path = entry.path().join("head.yaml");
                if !head_path.exists() {
                    continue;
                }
                let source = fs::read_to_string(&head_path)
                    .map_err(|err| CasConflict::Io(err.to_string()))?;
                let head: Head = serde_yaml::from_str(&source)
                    .map_err(|err| CasConflict::Codec(err.to_string()))?;
                let capability = entry
                    .file_name()
                    .to_string_lossy()
                    .replace("__", "/")
                    .replace('_', ".");
                leaves.push(MerkleLeaf {
                    capability,
                    head_id: head.id,
                    head_hash: head.hash,
                    head_seq: head.seq,
                });
            }
        }
        leaves.sort_by(|left, right| left.capability.cmp(&right.capability));

        let mut root = MerkleRoot::new(epoch, prev_epoch, leaves, trigger);
        let inclusion_hash = calculate_merkle_root(&root.leaves);
        root.root_hash = hash_merkle_root(&root)
            .map(|root_hash| {
                super::super::vac_init_evidence_chain::sha256_hex(
                    format!("{inclusion_hash}:{root_hash}").as_bytes(),
                )
            })
            .map_err(CasConflict::InvalidRecord)?;
        self.signer.sign_merkle_root(&mut root);
        write_yaml_atomic(&self.anchor_path(epoch), &root)?;
        write_yaml_atomic(&self.anchor_head_path(), &root)?;
        Ok(root)
    }
}

fn write_yaml_atomic(path: &Path, value: &impl Serialize) -> Result<(), CasConflict> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| CasConflict::Io(err.to_string()))?;
    }
    let tmp = path.with_extension("yaml.tmp");
    let source = serde_yaml::to_string(value).map_err(|err| CasConflict::Codec(err.to_string()))?;
    fs::write(&tmp, source).map_err(|err| CasConflict::Io(err.to_string()))?;
    fs::rename(&tmp, path).map_err(|err| CasConflict::Io(err.to_string()))?;
    Ok(())
}

fn sanitize_path_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else if ch == '/' {
                '_'
            } else {
                '_'
            }
        })
        .collect()
}

fn collect_yaml_files_sorted(dir: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|err| err.to_string())? {
        let path = entry.map_err(|err| err.to_string())?.path();
        if path.is_dir() {
            collect_yaml_files_sorted(&path, paths)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("yaml") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(())
}

fn yaml_kind(value: &Value) -> Option<String> {
    let value = value.as_mapping()?.get(Value::String("kind".to_string()))?;
    value.as_str().map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control_plane::evidence_v2::signing::EvidenceSigner;
    use crate::control_plane::evidence_v2::signing::verify_signature_payload;
    use crate::control_plane::evidence_v2::types::ApprovalRequestV2;
    use crate::control_plane::evidence_v2::types::GitEvidence;
    use crate::control_plane::evidence_v2::types::SigMode;

    fn record(capability: &str, seq: u64) -> EvidenceV2 {
        EvidenceV2::new(
            capability,
            seq,
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
    fn store_appends_per_capability_subchain_and_seals_epoch() {
        let temp = tempfile::tempdir().unwrap();
        let store = GitRefEvidenceStore::new(temp.path());
        let capability = "vac.test".to_string();

        let head = store.append(&capability, record(&capability, 1)).unwrap();
        assert_eq!(head.seq, 1);
        assert!(store.read_head(&capability).unwrap().is_some());

        let anchor = store.seal_epoch(EpochTrigger::Manual).unwrap();
        assert_eq!(anchor.epoch, 1);
        assert_eq!(anchor.leaves.len(), 1);
        assert!(!anchor.root_hash.is_empty());
    }

    #[test]
    fn store_signs_records_when_broker_key_is_configured() {
        let temp = tempfile::tempdir().unwrap();
        let signer = EvidenceSigner::with_broker_and_operator_for_tests([7u8; 32], [8u8; 32]);
        let store = GitRefEvidenceStore::new_with_signer(temp.path(), signer);
        let capability = "vac.test".to_string();

        store.append(&capability, record(&capability, 1)).unwrap();
        let source = fs::read_to_string(
            store
                .root()
                .join("capabilities/vac_test/00000000000000000001.evidence.yaml"),
        )
        .unwrap();
        let record: EvidenceV2 = serde_yaml::from_str(&source).unwrap();
        assert_eq!(record.broker_sig.mode, SigMode::Signed);
        assert_eq!(record.operator_sig.as_ref().unwrap().mode, SigMode::Signed);
        verify_signature_payload(&record.sub_chain.self_hash, &record.broker_sig).unwrap();
        verify_signature_payload(
            &record.sub_chain.self_hash,
            record.operator_sig.as_ref().unwrap(),
        )
        .unwrap();

        let anchor = store.seal_epoch(EpochTrigger::Manual).unwrap();
        assert_eq!(anchor.broker_sig.mode, SigMode::Signed);
        verify_signature_payload(&anchor.root_hash, &anchor.broker_sig).unwrap();
    }

    #[test]
    fn resign_existing_records_rewrites_integrity_hints_to_signatures() {
        let temp = tempfile::tempdir().unwrap();
        let unsigned_store = GitRefEvidenceStore::new(temp.path());
        let capability = "vac.test".to_string();

        unsigned_store
            .append(&capability, record(&capability, 1))
            .unwrap();
        unsigned_store.seal_epoch(EpochTrigger::Manual).unwrap();

        let signer = EvidenceSigner::with_broker_and_operator_for_tests([7u8; 32], [8u8; 32]);
        let signed_store = GitRefEvidenceStore::new_with_signer(temp.path(), signer);
        let report = signed_store.resign_existing_records().unwrap();
        assert_eq!(report.evidence_records, 1);
        assert!(report.anchors >= 1);

        let source = fs::read_to_string(
            signed_store
                .root()
                .join("capabilities/vac_test/00000000000000000001.evidence.yaml"),
        )
        .unwrap();
        let record: EvidenceV2 = serde_yaml::from_str(&source).unwrap();
        assert_eq!(record.broker_sig.mode, SigMode::Signed);
        assert_eq!(record.operator_sig.as_ref().unwrap().mode, SigMode::Signed);
        verify_signature_payload(&record.sub_chain.self_hash, &record.broker_sig).unwrap();
        verify_signature_payload(
            &record.sub_chain.self_hash,
            record.operator_sig.as_ref().unwrap(),
        )
        .unwrap();
    }
}
