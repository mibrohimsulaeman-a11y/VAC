//! Donor domain contract registry.
//!
//! This module is the code-backed closeout for `docs/donor-migration/*`.
//! Donor migration is intentionally contract-first: every donor domain is
//! represented by a root-owned contract, validation gate, surface expectation,
//! and migration stance before any donor code may become product runtime code.

use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DonorDomainExecutionStance {
    ReadyForGatedSlices,
    DeferredGuardrail,
    DocOnlyOverlay,
}

impl DonorDomainExecutionStance {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadyForGatedSlices => "ready_for_gated_slices",
            Self::DeferredGuardrail => "deferred_guardrail",
            Self::DocOnlyOverlay => "doc_only_overlay",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DonorDomainContract {
    pub plan_id: &'static str,
    pub title: &'static str,
    pub doc_path: &'static str,
    pub stance: DonorDomainExecutionStance,
    pub owner: &'static str,
    pub donor_sources: &'static [&'static str],
    pub target_capabilities: &'static [&'static str],
    pub surfaces: &'static [&'static str],
    pub gates: &'static [&'static str],
    pub blocked_runtime_claim: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DonorDomainContractSeverity {
    Error,
    Warning,
}

impl DonorDomainContractSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DonorDomainContractFinding {
    pub severity: DonorDomainContractSeverity,
    pub plan_id: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DonorDomainContractReport {
    pub repo_root: PathBuf,
    pub inventory_path: PathBuf,
    pub contracts: Vec<DonorDomainContract>,
    pub findings: Vec<DonorDomainContractFinding>,
}

impl DonorDomainContractReport {
    pub fn is_failure(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.severity == DonorDomainContractSeverity::Error)
    }

    pub fn render_text(&self) -> String {
        self.render_lines().join("\n")
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!(
            "donor domain contracts: registry={} contracts={} findings={}",
            self.inventory_path.display(),
            self.contracts.len(),
            self.findings.len()
        ));
        for contract in &self.contracts {
            lines.push(format!(
                "  {} {} -> stance={} owner={} sources=[{}] capabilities=[{}] surfaces=[{}]",
                contract.plan_id,
                contract.title,
                contract.stance.as_str(),
                contract.owner,
                contract.donor_sources.join(", "),
                contract.target_capabilities.join(", "),
                contract.surfaces.join(", "),
            ));
            lines.push(format!("     gates=[{}]", contract.gates.join(", ")));
            lines.push(format!(
                "     runtime_claim_guardrail={}",
                contract.blocked_runtime_claim
            ));
        }
        for finding in &self.findings {
            lines.push(format!(
                "  {} {}: {}",
                finding.severity.as_str().to_uppercase(),
                finding.plan_id,
                finding.message
            ));
        }
        if self.is_failure() {
            lines.push("donor domain contracts: FAIL".to_string());
        } else {
            lines.push("donor domain contracts: ok".to_string());
        }
        lines
    }
}

pub const DONOR_DOMAIN_CONTRACTS: &[DonorDomainContract] = &[
    DonorDomainContract {
        plan_id: "01",
        title: "Session Engine",
        doc_path: "docs/donor-migration/domain-plans/01-session-engine.md",
        stance: DonorDomainExecutionStance::ReadyForGatedSlices,
        owner: "vac-core/control_plane::donor_domain_contract",
        donor_sources: &["vac_session_engine", "vac_session_control"],
        target_capabilities: &["vac.session_engine", "vac.transcript", "vac.checkpoint"],
        surfaces: &["/sessions", "/activity", "/evidence", "/recovery"],
        gates: &[
            "session state visible",
            "checkpoint metadata recorded",
            "no app-server runtime owner",
        ],
        blocked_runtime_claim: "No hidden donor/app-server session runtime may be used as product owner.",
    },
    DonorDomainContract {
        plan_id: "02",
        title: "Tool Contract",
        doc_path: "docs/donor-migration/domain-plans/02-tool-contract.md",
        stance: DonorDomainExecutionStance::ReadyForGatedSlices,
        owner: "vac-core/control_plane::donor_domain_contract",
        donor_sources: &["vac_tool_core", "vac_tools"],
        target_capabilities: &["vac.tool_contract", "vac.tools", "vac.policy"],
        surfaces: &["/activity", "/approvals", "/doctor"],
        gates: &[
            "policy envelope required",
            "mutating tools approval-gated",
            "unknown tools fail closed",
        ],
        blocked_runtime_claim: "No ad-hoc donor tool dispatch may bypass root policy/approval envelopes.",
    },
    DonorDomainContract {
        plan_id: "03",
        title: "Managed Connectors",
        doc_path: "docs/donor-migration/domain-plans/03-managed-connectors.md",
        stance: DonorDomainExecutionStance::ReadyForGatedSlices,
        owner: "vac-core/control_plane::donor_domain_contract",
        donor_sources: &["vac_mcp_core", "vac_connectors", "vac_ingest"],
        target_capabilities: &["vac.connectors", "vac.connector_acl", "vac.source_ingest"],
        surfaces: &["/connectors", "/status", "/doctor"],
        gates: &[
            "credential absence is visible",
            "connector health has recovery hint",
            "redaction gate before external send",
        ],
        blocked_runtime_claim: "No connector may persist or send data before tool/trust/redaction gates classify it.",
    },
    DonorDomainContract {
        plan_id: "04",
        title: "Changeset and Evidence",
        doc_path: "docs/donor-migration/domain-plans/04-changeset-evidence.md",
        stance: DonorDomainExecutionStance::ReadyForGatedSlices,
        owner: "vac-core/control_plane::donor_domain_contract",
        donor_sources: &["vac_apply_patch", "vac_changeset"],
        target_capabilities: &["vac.patch", "vac.changeset", "vac.semantic_diff"],
        surfaces: &["/evidence", "/approvals", "/activity"],
        gates: &[
            "planned edits previewed",
            "mutation waits for approval",
            "failure state remains visible",
        ],
        blocked_runtime_claim: "No donor patch/change path may mutate files without root preview and evidence.",
    },
    DonorDomainContract {
        plan_id: "05",
        title: "Trust and Redaction",
        doc_path: "docs/donor-migration/domain-plans/05-trust-redaction.md",
        stance: DonorDomainExecutionStance::ReadyForGatedSlices,
        owner: "vac-core/control_plane::donor_domain_contract",
        donor_sources: &["vac_trust", "vac_redaction", "vac_sandbox"],
        target_capabilities: &["vac.trust", "vac.redaction", "vac.sandbox"],
        surfaces: &["/approvals", "/activity", "/evidence", "/doctor"],
        gates: &[
            "secret-like fixtures are redacted",
            "redaction status is explicit",
            "blocked external send has recovery hint",
        ],
        blocked_runtime_claim: "No raw secret-like payload may enter evidence/activity without redaction status.",
    },
    DonorDomainContract {
        plan_id: "06",
        title: "Context, Memory, Search, and RAG",
        doc_path: "docs/donor-migration/domain-plans/06-context-rag-memory.md",
        stance: DonorDomainExecutionStance::ReadyForGatedSlices,
        owner: "vac-core/control_plane::donor_domain_contract",
        donor_sources: &["vac_memory", "vac_ingest", "vac_search"],
        target_capabilities: &["vac.context", "vac.curated_memory", "vac.source_catalog"],
        surfaces: &["/context", "/memory", "/status", "/why"],
        gates: &[
            "memory writes require mutation contract",
            "source attribution required",
            "freshness is visible",
        ],
        blocked_runtime_claim: "No semantic memory/RAG provider may claim migration without mutation/redaction/source evidence.",
    },
    DonorDomainContract {
        plan_id: "07",
        title: "VIL Native",
        doc_path: "docs/donor-migration/domain-plans/07-vil-native.md",
        stance: DonorDomainExecutionStance::ReadyForGatedSlices,
        owner: "vac-core/control_plane::donor_domain_contract",
        donor_sources: &["vac_vil", "vac_vwfd", "vac_manifest"],
        target_capabilities: &["vac.vil", "vac.vwfd", "vac.workflow"],
        surfaces: &["/workflow", "/capabilities", "/doctor"],
        gates: &[
            "repo markers or user intent required",
            "mutation goes through changeset/evidence",
            "knowledge lookup attributed",
        ],
        blocked_runtime_claim: "No generic runtime rewrite or hidden VIL execution may land as donor migration.",
    },
    DonorDomainContract {
        plan_id: "08",
        title: "Trace, Signal, and Trajectory",
        doc_path: "docs/donor-migration/domain-plans/08-trace-signal-trajectory.md",
        stance: DonorDomainExecutionStance::ReadyForGatedSlices,
        owner: "vac-core/control_plane::donor_domain_contract",
        donor_sources: &["vac_signal", "vac_trace", "vac_trajectory"],
        target_capabilities: &["vac.signal", "vac.trace", "vac.why"],
        surfaces: &["/why", "/evidence", "/activity", "statusline"],
        gates: &[
            "local signal capture first",
            "export waits for redaction",
            "trajectory scoring is advisory only",
        ],
        blocked_runtime_claim: "No trajectory score may replace explicit evidence, policy, or approval outcome.",
    },
    DonorDomainContract {
        plan_id: "09",
        title: "Agent Orchestration",
        doc_path: "docs/donor-migration/domain-plans/09-agent-orchestration.md",
        stance: DonorDomainExecutionStance::DeferredGuardrail,
        owner: "vac-core/control_plane::donor_domain_contract",
        donor_sources: &["vac_agent_orchestration", "vac_scheduler"],
        target_capabilities: &["vac.agent_orchestration", "vac.scheduler"],
        surfaces: &["/workflow", "/activity", "/approvals"],
        gates: &[
            "single-agent safety first",
            "nested agents remain deferred",
            "stop/retry must be visible",
        ],
        blocked_runtime_claim: "No donor swarm/multi-agent runtime may execute before prerequisite gates clear.",
    },
    DonorDomainContract {
        plan_id: "10",
        title: "TUI Concept Extraction",
        doc_path: "docs/donor-migration/domain-plans/10-tui-concept-extraction.md",
        stance: DonorDomainExecutionStance::ReadyForGatedSlices,
        owner: "vac-core/control_plane::donor_domain_contract",
        donor_sources: &["vac_shell_*", "vac_tui_runtime"],
        target_capabilities: &["vac.tui", "vac.zero_config_workspace"],
        surfaces: &["/status", "/workflow", "/capabilities", "/sessions"],
        gates: &[
            "root TUI remains single",
            "donor shell runtime quarantined",
            "first-run .vac UX is non-blocking",
        ],
        blocked_runtime_claim: "No donor shell/TUI runtime may become product code; concepts must be reimplemented in root TUI.",
    },
    DonorDomainContract {
        plan_id: "11",
        title: "External Agent Concept Adoption",
        doc_path: "docs/donor-migration/domain-plans/11-external-agent-concept-adoption.md",
        stance: DonorDomainExecutionStance::DocOnlyOverlay,
        owner: "vac-core/control_plane::donor_domain_contract",
        donor_sources: &["Hermes Agent concept", "external agent concept"],
        target_capabilities: &[
            "vac.project_workspace",
            "vac.init.memory-governance",
            "vac.init.safe-rationale",
            "vac.init.evidence-why-live",
            "vac.init.semantic-plan",
            "vac.init.runtime-gate-enforcement",
            "vac.policy",
        ],
        surfaces: &["/memory", "/workflow", "/status", "/approvals"],
        gates: &[
            "concept only",
            "no external runtime code",
            "implementation routes through owning domain plan",
        ],
        blocked_runtime_claim: "No external conceptual donor may promote a MIGRATED status or clone runtime code.",
    },
];

pub const PLAN11_HERMES_MEMORY_CANONICAL_CAPABILITIES: &[&str] = &[
    "vac.init.memory-governance",
    "vac.init.safe-rationale",
    "vac.init.evidence-why-live",
    "vac.init.semantic-plan",
    "vac.init.runtime-gate-enforcement",
];

pub const PLAN11_HERMES_MEMORY_FORBIDDEN_CAPABILITIES: &[&str] = &[
    "vac.memory",
    "vac.curated_memory",
    "vac.memory_skill_synthesis",
    "vac.episodic_recall",
    "vac.semantic_retrieval",
    "vac.memory_provider",
];

pub const PLAN11_HERMES_CONCEPT_ROUTES: &[(&str, &str, &str)] = &[
    (
        "closed_learning_loop",
        "vac.init.memory-governance",
        "evidence-backed memory/skill proposal loop; no autonomous persistence",
    ),
    (
        "persistent_memory",
        "vac.init.memory-governance",
        "tiered memory governance with source refs, TTL, redaction, and review",
    ),
    (
        "skill_creation",
        "vac.init.memory-governance",
        "proposal queue only until a future skills plan is reviewed and evidence-gated",
    ),
    (
        "trajectory_export",
        "vac.init.evidence-why-live",
        "safe rationale and trajectory records; raw chain-of-thought excluded",
    ),
    (
        "scheduled_memory_nudges",
        "vac.init.runtime-gate-enforcement",
        "scheduler/autopilot may suggest, but not persist team memory without approval",
    ),
];

pub fn validate_plan11_hermes_memory_alignment(declared_capabilities: &[&str]) -> Vec<String> {
    let mut findings = Vec::new();

    for forbidden in PLAN11_HERMES_MEMORY_FORBIDDEN_CAPABILITIES {
        if declared_capabilities.contains(forbidden) {
            findings.push(format!(
                "Plan 11 must not declare memory capability `{forbidden}`; route through existing memory governance"
            ));
        }
    }

    for required in PLAN11_HERMES_MEMORY_CANONICAL_CAPABILITIES {
        if !declared_capabilities.contains(required) {
            findings.push(format!(
                "Plan 11 missing canonical memory alignment capability `{required}`"
            ));
        }
    }

    findings
}

pub fn load_donor_domain_contract_report(root: impl AsRef<Path>) -> DonorDomainContractReport {
    let repo_root = root.as_ref().to_path_buf();
    let inventory_path = repo_root.join(".vac/registry/donor-inventory.yaml");
    let mut findings = Vec::new();

    if !inventory_path.exists() {
        findings.push(DonorDomainContractFinding {
            severity: DonorDomainContractSeverity::Error,
            plan_id: "registry".to_string(),
            message: "missing .vac/registry/donor-inventory.yaml".to_string(),
        });
    } else {
        let inventory_text = std::fs::read_to_string(&inventory_path).unwrap_or_default();
        for contract in DONOR_DOMAIN_CONTRACTS {
            if !inventory_text.contains(contract.plan_id)
                || !inventory_text.contains(contract.title)
            {
                findings.push(DonorDomainContractFinding {
                    severity: DonorDomainContractSeverity::Error,
                    plan_id: contract.plan_id.to_string(),
                    message: "donor inventory does not reference plan contract".to_string(),
                });
            }
        }
    }

    let status_board = read_root_file(&repo_root, "docs/donor-migration/DONOR_STATUS_BOARD.md");
    let domain_index = read_root_file(&repo_root, "docs/donor-migration/domain-plans/INDEX.md");

    if let Some(plan11) = DONOR_DOMAIN_CONTRACTS
        .iter()
        .find(|contract| contract.plan_id == "11")
    {
        for finding in validate_plan11_hermes_memory_alignment(plan11.target_capabilities) {
            findings.push(DonorDomainContractFinding {
                severity: DonorDomainContractSeverity::Error,
                plan_id: "11".to_string(),
                message: finding,
            });
        }
    }

    for contract in DONOR_DOMAIN_CONTRACTS {
        let doc_path = repo_root.join(contract.doc_path);
        match std::fs::read_to_string(&doc_path) {
            Ok(text) => {
                for required in [
                    "## Active execution contract",
                    "### Current status",
                    "### Target outcome",
                    "## Done criteria",
                ] {
                    if !text.contains(required) {
                        findings.push(DonorDomainContractFinding {
                            severity: DonorDomainContractSeverity::Error,
                            plan_id: contract.plan_id.to_string(),
                            message: format!(
                                "{} missing required section `{required}`",
                                contract.doc_path
                            ),
                        });
                    }
                }
                if !text.contains("CONTRACT IMPLEMENTED") {
                    findings.push(DonorDomainContractFinding {
                        severity: DonorDomainContractSeverity::Warning,
                        plan_id: contract.plan_id.to_string(),
                        message: format!(
                            "{} does not carry CONTRACT IMPLEMENTED closeout wording",
                            contract.doc_path
                        ),
                    });
                }
            }
            Err(err) => findings.push(DonorDomainContractFinding {
                severity: DonorDomainContractSeverity::Error,
                plan_id: contract.plan_id.to_string(),
                message: format!("{} unreadable: {err}", contract.doc_path),
            }),
        }

        if !status_board.contains(contract.plan_id) || !status_board.contains(contract.title) {
            findings.push(DonorDomainContractFinding {
                severity: DonorDomainContractSeverity::Error,
                plan_id: contract.plan_id.to_string(),
                message: "DONOR_STATUS_BOARD.md missing domain contract row".to_string(),
            });
        }

        if !domain_index.contains(
            contract
                .doc_path
                .rsplit('/')
                .next()
                .unwrap_or(contract.doc_path),
        ) {
            findings.push(DonorDomainContractFinding {
                severity: DonorDomainContractSeverity::Error,
                plan_id: contract.plan_id.to_string(),
                message: "domain-plans/INDEX.md missing domain doc link".to_string(),
            });
        }
    }

    DonorDomainContractReport {
        repo_root,
        inventory_path,
        contracts: DONOR_DOMAIN_CONTRACTS.to_vec(),
        findings,
    }
}

fn read_root_file(root: &Path, relative: &str) -> String {
    std::fs::read_to_string(root.join(relative)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_contract_registry_covers_all_domain_plans() {
        assert_eq!(DONOR_DOMAIN_CONTRACTS.len(), 11);
        assert!(
            DONOR_DOMAIN_CONTRACTS
                .iter()
                .any(|contract| contract.plan_id == "01")
        );
        assert!(
            DONOR_DOMAIN_CONTRACTS
                .iter()
                .any(|contract| contract.plan_id == "11")
        );
        assert!(
            DONOR_DOMAIN_CONTRACTS
                .iter()
                .any(|contract| contract.stance == DonorDomainExecutionStance::DeferredGuardrail)
        );
        assert!(
            DONOR_DOMAIN_CONTRACTS
                .iter()
                .any(|contract| contract.stance == DonorDomainExecutionStance::DocOnlyOverlay)
        );
    }

    #[test]
    fn plan11_hermes_memory_alignment_uses_existing_memory_capabilities_only() {
        let plan11 = DONOR_DOMAIN_CONTRACTS
            .iter()
            .find(|contract| contract.plan_id == "11")
            .expect("plan 11");
        assert!(
            validate_plan11_hermes_memory_alignment(plan11.target_capabilities).is_empty(),
            "{:?}",
            plan11.target_capabilities
        );
        for forbidden in PLAN11_HERMES_MEMORY_FORBIDDEN_CAPABILITIES {
            assert!(
                !plan11.target_capabilities.contains(forbidden),
                "Plan 11 must not drift into new memory capability `{forbidden}`"
            );
        }
    }

    #[test]
    fn plan11_hermes_memory_alignment_rejects_parallel_memory_capability() {
        let findings = validate_plan11_hermes_memory_alignment(&[
            "vac.project_workspace",
            "vac.memory",
            "vac.init.safe-rationale",
            "vac.init.evidence-why-live",
            "vac.init.semantic-plan",
            "vac.init.runtime-gate-enforcement",
        ]);
        assert!(
            findings
                .iter()
                .any(|finding| finding.contains("vac.memory")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.contains("vac.init.memory-governance")),
            "{findings:?}"
        );
    }

    #[test]
    fn every_contract_has_owner_surface_gate_and_guardrail() {
        for contract in DONOR_DOMAIN_CONTRACTS {
            assert!(!contract.owner.is_empty(), "{contract:?}");
            assert!(!contract.donor_sources.is_empty(), "{contract:?}");
            assert!(!contract.target_capabilities.is_empty(), "{contract:?}");
            assert!(!contract.surfaces.is_empty(), "{contract:?}");
            assert!(!contract.gates.is_empty(), "{contract:?}");
            assert!(
                contract.blocked_runtime_claim.starts_with("No "),
                "{}",
                contract.blocked_runtime_claim
            );
        }
    }
}
