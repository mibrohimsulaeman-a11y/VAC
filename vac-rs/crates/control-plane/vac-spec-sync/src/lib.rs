//! VAC SpecSync reconciliation engine contracts.
//!
//! SpecSync maps changed files to capabilities, detects drift against intent
//! specs/manifests/readiness/evidence, produces schema-valid proposals, and
//! keeps runtime authority immutable until approval + compile + evidence close.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpecSyncDrift {
    pub drift_type: String,
    pub severity: String,
    pub capability: String,
    pub suggested_update: String,
    pub evidence_span: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangedFile {
    pub path: String,
    pub sha256_before: Option<String>,
    pub sha256_after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpecUpdateProposal {
    pub proposal_id: String,
    pub capability: String,
    pub manifest_path: String,
    pub proposed_patch_hash: String,
    pub requires_approval: bool,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpecSyncReport {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub changed_files: Vec<ChangedFile>,
    pub drift: Vec<SpecSyncDrift>,
    pub proposals: Vec<SpecUpdateProposal>,
    pub unresolved_critical_drift: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityPathMap {
    pub capability: String,
    pub include: Vec<String>,
}

#[must_use]
pub fn map_changed_files_to_capabilities(
    files: &[ChangedFile],
    maps: &[CapabilityPathMap],
) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for file in files {
        for map in maps {
            if map
                .include
                .iter()
                .any(|pattern| path_matches(pattern, &file.path))
            {
                pairs.push((file.path.clone(), map.capability.clone()));
            }
        }
    }
    pairs.sort();
    pairs.dedup();
    pairs
}

#[must_use]
pub fn critical_drift_count(drift: &[SpecSyncDrift]) -> u32 {
    drift
        .iter()
        .filter(|item| matches!(item.severity.as_str(), "P0" | "critical"))
        .count() as u32
}

#[must_use]
pub fn path_matches(pattern: &str, path: &str) -> bool {
    vac_paths::path_matches(pattern, path)
}

#[must_use]
pub fn detect_spec_drift(
    changed_files: &[ChangedFile],
    maps: &[CapabilityPathMap],
    capabilities_with_intent_specs: &[String],
) -> SpecSyncReport {
    let pairs = map_changed_files_to_capabilities(changed_files, maps);
    let mut drift = Vec::new();
    let mut proposals = Vec::new();
    for (path, capability) in pairs {
        if !capabilities_with_intent_specs
            .iter()
            .any(|item| item == &capability)
        {
            drift.push(SpecSyncDrift {
                drift_type: "capability_without_current_intent_spec".to_string(),
                severity: "P0".to_string(),
                capability: capability.clone(),
                suggested_update: format!(
                    "Create or refresh .vac/specs/{capability}.yaml for changed file {path}"
                ),
                evidence_span: Some(format!("span:{path}")),
            });
            proposals.push(SpecUpdateProposal {
                proposal_id: format!("proposal.specsync.{}", capability.replace('.', "_")),
                capability: capability.clone(),
                manifest_path: format!(".vac/specs/{}.yaml", capability.replace('.', "-")),
                proposed_patch_hash: "sha256:pending-operator-approval".to_string(),
                requires_approval: true,
                evidence_refs: vec![format!("changed:{path}")],
            });
        }
    }
    let unresolved_critical_drift = critical_drift_count(&drift);
    SpecSyncReport {
        schema_version: 1,
        kind: "spec_sync_report".to_string(),
        id: "specsync.current".to_string(),
        changed_files: changed_files.to_vec(),
        drift,
        proposals,
        unresolved_critical_drift,
    }
}

#[must_use]
pub fn closeout_blocks_on_critical_drift(report: &SpecSyncReport) -> bool {
    report.unresolved_critical_drift > 0 || critical_drift_count(&report.drift) > 0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SymbolIntentInvariant {
    pub capability: String,
    pub symbol: String,
    pub invariant_id: String,
    pub expected_text: String,
}

/// Broader semantic-drift scaffold: changed symbols are reconciled against
/// capability intent invariants, not only missing intent specs.  It is still a
/// proposal engine: generated updates require approval before runtime authority
/// changes.
#[must_use]
pub fn detect_symbol_invariant_drift(
    changed_symbols: &[(String, String)],
    invariants: &[SymbolIntentInvariant],
) -> Vec<SpecSyncDrift> {
    let mut drift = Vec::new();
    for (capability, symbol) in changed_symbols {
        let has_invariant = invariants
            .iter()
            .any(|item| item.capability == *capability && item.symbol == *symbol);
        if !has_invariant {
            drift.push(SpecSyncDrift {
                drift_type: "changed_symbol_without_intent_invariant".to_string(),
                severity: "P1".to_string(),
                capability: capability.clone(),
                suggested_update: format!(
                    "Add or refresh intent invariant for changed symbol {symbol}"
                ),
                evidence_span: Some(format!("symbol:{capability}:{symbol}")),
            });
        }
    }
    drift
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_intent_spec_creates_critical_approval_bound_proposal() {
        let changed = vec![ChangedFile {
            path: "vac-rs/crates/control-plane/vac-policy/src/lib.rs".to_string(),
            sha256_before: None,
            sha256_after: "sha256:after".to_string(),
        }];
        let maps = vec![CapabilityPathMap {
            capability: "vac.control_plane.policy".to_string(),
            include: vec!["vac-rs/crates/control-plane/vac-policy/**".to_string()],
        }];

        let report = detect_spec_drift(&changed, &maps, &[]);

        assert_eq!(report.unresolved_critical_drift, 1);
        assert!(closeout_blocks_on_critical_drift(&report));
        assert_eq!(report.proposals.len(), 1);
        assert!(report.proposals[0].requires_approval);
    }

    #[test]
    fn changed_symbol_without_invariant_is_reported() {
        let drift = detect_symbol_invariant_drift(
            &[(
                "vac.control_plane.policy".to_string(),
                "evaluate".to_string(),
            )],
            &[],
        );

        assert_eq!(drift.len(), 1);
        assert_eq!(
            drift[0].drift_type,
            "changed_symbol_without_intent_invariant"
        );
    }
}
