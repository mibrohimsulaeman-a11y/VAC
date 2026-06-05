use std::path::Path;
use std::path::PathBuf;

pub fn evidence_v1_to_v2_migration_path(root: impl AsRef<Path>) -> PathBuf {
    root.as_ref()
        .join(".vac/registry/migrations/migration.evidence-v1-to-v2.yaml")
}

pub fn render_evidence_v1_to_v2_migration_yaml() -> String {
    r#"schema_version: 1
kind: migration
id: migration.evidence-v1-to-v2
from_version: 1
to_version: 2
changes:
  - action: add_schema
    target: evidence
    schema_version: 2
  - action: migrate_linear_chain_to_capability_subchains
  - action: create_merkle_anchor
verification:
  command:
    id: vac.doctor.evidence.v2
    runner: vac
    args: ["doctor", "evidence", "--v2"]
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_declares_v2_doctor_verification() {
        let rendered = render_evidence_v1_to_v2_migration_yaml();
        assert!(rendered.contains("from_version: 1"));
        assert!(rendered.contains("to_version: 2"));
        assert!(rendered.contains(r#"args: ["doctor", "evidence", "--v2"]"#));
    }
}
