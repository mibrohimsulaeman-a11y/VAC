use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

const IDENTITY_CHECK_SCAN_ROOTS: &[&str] = &[
    "vac-rs",
    "vac-cli",
    ".vac",
    "docs",
    "README.md",
    "AGENTS.md",
];
const TERM_VAC_TUI_RUNTIME: &str = concat!("vac", "_tui", "_runtime");
const TERM_DUPLICATE_TUI: &str = concat!("duplicate ", "TUI");
const TERM_OLD_RUNTIME: &str = concat!("old ", "runtime");
const TERM_OLD_PRODUCT_ASSUMPTION: &str = concat!("old ", "product assumption");
const TERM_RENAMED_TERMINAL_APP: &str = concat!("renamed terminal ", "app");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityTermSeverity {
    Fail,
}

impl IdentityTermSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fail => "fail",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForbiddenIdentityTerm {
    pub term: &'static str,
    pub reason: &'static str,
    pub severity: IdentityTermSeverity,
    pub replacement_hint: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdentityExemption {
    pub path: &'static str,
    pub reason: &'static str,
    pub expires_when: &'static str,
}

const FORBIDDEN_TERMS: &[ForbiddenIdentityTerm] = &[
    ForbiddenIdentityTerm {
        term: TERM_VAC_TUI_RUNTIME,
        reason: "Use the root TUI/operator surface name, not a retired runtime identifier.",
        severity: IdentityTermSeverity::Fail,
        replacement_hint: "root TUI operator surface",
    },
    ForbiddenIdentityTerm {
        term: TERM_DUPLICATE_TUI,
        reason: "Describe the canonical root TUI surface instead of suggesting a parallel TUI.",
        severity: IdentityTermSeverity::Fail,
        replacement_hint: "root TUI surface",
    },
    ForbiddenIdentityTerm {
        term: TERM_OLD_RUNTIME,
        reason: "Use local runtime transition or compatibility transport retirement terminology.",
        severity: IdentityTermSeverity::Fail,
        replacement_hint: "local runtime transition",
    },
    ForbiddenIdentityTerm {
        term: TERM_OLD_PRODUCT_ASSUMPTION,
        reason: "Use current VAC product language instead of retired product framing.",
        severity: IdentityTermSeverity::Fail,
        replacement_hint: "current VAC product assumption",
    },
    ForbiddenIdentityTerm {
        term: TERM_RENAMED_TERMINAL_APP,
        reason: "Use VAC root product terminology instead of renamed-terminal framing.",
        severity: IdentityTermSeverity::Fail,
        replacement_hint: "VAC root product",
    },
];

const IDENTITY_CHECK_EXEMPTIONS: &[IdentityExemption] = &[
    IdentityExemption {
        path: "AGENTS.md",
        reason: "Repository guardrail doc quotes retired identifiers as negative examples.",
        expires_when: "Local runtime transition docs are archived",
    },
    IdentityExemption {
        path: "README.md",
        reason: "Root README still carries migration-context guardrail examples.",
        expires_when: "Local runtime transition docs are archived",
    },
    IdentityExemption {
        path: ".vac/registry/status.yaml",
        reason: "Registry status lists canonical maintenance workflow ids and invariant names.",
        expires_when: "Workflow identity vocabulary policy supports canonical ids",
    },
    IdentityExemption {
        path: ".vac/.init/**",
        reason: "VAC-Init generated source inventory and risk reports can quote historical scanner findings.",
        expires_when: "Generated init projections are refreshed without retired vocabulary",
    },
    IdentityExemption {
        path: ".vac/registry/donor-inventory.yaml",
        reason: "Donor inventory records retired donor source identifiers while donor frontend stacks remain quarantined.",
        expires_when: "Donor inventory is rewritten with opaque source ids",
    },
    IdentityExemption {
        path: ".vac/registry/plan-state.yaml",
        reason: "Plan state is historical registry metadata and can quote retired plan titles.",
        expires_when: "Historical plan-state snapshots are archived",
    },
    IdentityExemption {
        path: ".vac/registry/plans/**",
        reason: "Registry plan snapshots quote historical/generated findings and ownership labels.",
        expires_when: "Registry plan snapshots are moved outside identity-check scan roots",
    },
    IdentityExemption {
        path: ".vac/workflows/README.md",
        reason: "Workflow README lists canonical maintenance workflow ids.",
        expires_when: "Workflow identity vocabulary policy supports canonical ids",
    },
    IdentityExemption {
        path: ".vac/workflows/maintenance.no-duplicate-tui.yaml",
        reason: "Canonical workflow id intentionally names the TUI uniqueness invariant.",
        expires_when: "Workflow id migration is explicitly planned",
    },
    IdentityExemption {
        path: ".vac/workflows/maintenance.release-gate.yaml",
        reason: "Release gate references canonical maintenance workflow steps.",
        expires_when: "Workflow id migration is explicitly planned",
    },
    IdentityExemption {
        path: "docs/architecture/local-runtime-contract.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/donor-migration/DONOR_BACKEND_DOC_COVERAGE_AUDIT.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/donor-migration/DONOR_DELETE_QUARANTINE_GATE.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/donor-migration/DONOR_E2E_UX_PRODUCTION_RULE.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/donor-migration/DONOR_INVENTORY_MATRIX.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/donor-migration/DONOR_SHELL_STACK_QUARANTINE.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/donor-migration/DONOR_STATUS_BOARD.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/donor-migration/INDEX.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/donor-migration/domain-plans/10-tui-concept-extraction.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/executor-prompts/00B-local-runtime-contract-implementation-plan.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/executor-prompts/00C-rewire-vac-exec.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/executor-prompts/00E-runtime-delete-gate.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/migration/00E_REACHABILITY_AUDIT.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/migration/APP_SERVER_TO_LOCAL_RUNTIME_MIGRATION.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/migration/CODE_REVIEW_LOCAL_RUNTIME_LANDED.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/migration/INDEX.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/migration/OLD_RUNTIME_DELETE_GATE.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/migration/PHASE00_CLOSEOUT_STATUS.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/migration/VAC_EXEC_REWIRE_SPEC.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/product/requirements-matrix.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/validation/LOCAL_RUNTIME_GATE.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/workflow-control-plane/IMPLEMENTATION_PLAN.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/workflow-control-plane/INTERFERENCE_AUDIT.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/workflow-control-plane/plans/00A-build-unblock.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/workflow-control-plane/plans/00D-rewire-vac-tui.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/workflow-control-plane/plans/00E-runtime-reachability-delete-gate.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/workflow-control-plane/plans/00F-tui-legacy-transport-retirement.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/workflow-control-plane/plans/12-safe-workflow-runner.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/workflow-control-plane/plans/15-maintenance-identity-check.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/workflow-control-plane/plans/17-maintenance-no-duplicate-tui.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/workflow-control-plane/plans/19-root-feature-conversion.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/workflow-control-plane/plans/INDEX.md",
        reason: "Migration/control-plane document intentionally quotes retired vocabulary as historical context.",
        expires_when: "Historical migration docs are archived or rewritten",
    },
    IdentityExemption {
        path: "docs/product/CAPABILITY_MAP.md",
        reason: "Historical capability-map boundary document retained until product docs converge.",
        expires_when: "Plan 19 root feature conversion closes",
    },
    IdentityExemption {
        path: "docs/product/domain-prds/tui-action-recorder-replay.md",
        reason: "Domain PRD records retired TUI-action naming while the root TUI action model migrates.",
        expires_when: "Plan 11 surface convergence closes",
    },
    IdentityExemption {
        path: "docs/scheduled-audits/**",
        reason: "Scheduled audit snapshots quote historical/generated findings and can recursively mention forbidden terms.",
        expires_when: "Scheduled audit snapshots are moved outside identity-check scan roots",
    },
    IdentityExemption {
        path: "docs/scheduled-plans/**",
        reason: "Scheduled/generated planning snapshots quote historical filenames and findings that can normalize into forbidden terms.",
        expires_when: "Scheduled plan snapshots are moved outside identity-check scan roots",
    },
    IdentityExemption {
        path: "docs/workflow-control-plane/plans/33-evidence/**",
        reason: "Evidence files can quote historical or uncommitted file names that include forbidden identity terms.",
        expires_when: "Evidence directory is archived or removed from identity-check scan roots",
    },
    IdentityExemption {
        path: "vac-rs/core/src/control_plane/identity_check.rs",
        reason: "Scanner implementation defines forbidden terms and regression fixtures.",
        expires_when: "Identity scanner stores terms outside scanned source",
    },
    IdentityExemption {
        path: "vac-rs/crates/control-plane/control-plane/src/control_plane/identity_check.rs",
        reason: "Scanner implementation defines forbidden terms and regression fixtures.",
        expires_when: "Identity scanner stores terms outside scanned source",
    },
    IdentityExemption {
        path: "vac-rs/core/src/control_plane/no_duplicate_tui.rs",
        reason: "TUI uniqueness scanner defines forbidden terms and regression fixtures.",
        expires_when: "TUI uniqueness scanner stores terms outside scanned source",
    },
    IdentityExemption {
        path: "vac-rs/crates/control-plane/control-plane/src/control_plane/no_duplicate_tui.rs",
        reason: "TUI uniqueness scanner defines forbidden terms and regression fixtures.",
        expires_when: "TUI uniqueness scanner stores terms outside scanned source",
    },
    IdentityExemption {
        path: "vac-rs/core/src/control_plane/workflow_runner.rs",
        reason: "Workflow runner renders canonical maintenance workflow labels and diagnostics.",
        expires_when: "Workflow identity vocabulary policy supports canonical ids",
    },
    IdentityExemption {
        path: "vac-rs/crates/control-plane/control-plane/src/control_plane/workflow_runner/**",
        reason: "Workflow runner renders canonical maintenance workflow labels and diagnostics.",
        expires_when: "Workflow identity vocabulary policy supports canonical ids",
    },
    IdentityExemption {
        path: "vac-rs/core/src/control_plane/mod.rs",
        reason: "Control-plane module names expose the canonical TUI uniqueness scanner.",
        expires_when: "Module naming migration is explicitly planned",
    },
    IdentityExemption {
        path: "vac-rs/crates/control-plane/control-plane/src/control_plane/mod.rs",
        reason: "Control-plane module names expose the canonical TUI uniqueness scanner.",
        expires_when: "Module naming migration is explicitly planned",
    },
    IdentityExemption {
        path: "vac-rs/core/src/control_plane/root_feature_catalog.rs",
        reason: "Root feature catalog includes canonical maintenance workflow names.",
        expires_when: "Workflow id migration is explicitly planned",
    },
    IdentityExemption {
        path: "vac-rs/crates/control-plane/control-plane/src/control_plane/root_feature_catalog.rs",
        reason: "Root feature catalog includes canonical maintenance workflow names.",
        expires_when: "Workflow id migration is explicitly planned",
    },
    IdentityExemption {
        path: "vac-rs/core/src/control_plane/surface_doctor_tests.rs",
        reason: "Surface doctor regression fixtures quote historical drift terms.",
        expires_when: "Fixture vocabulary is migrated",
    },
    IdentityExemption {
        path: "vac-rs/crates/control-plane/control-plane/src/control_plane/surface_doctor_tests.rs",
        reason: "Surface doctor regression fixtures quote historical drift terms.",
        expires_when: "Fixture vocabulary is migrated",
    },
    IdentityExemption {
        path: "vac-rs/cli/src/doctor_cli.rs",
        reason: "CLI doctor tests quote forbidden terms as regression fixtures.",
        expires_when: "Fixtures are migrated to synthetic tokens",
    },
    IdentityExemption {
        path: "vac-rs/crates/surfaces/cli/src/doctor_cli.rs.inc",
        reason: "CLI doctor tests quote forbidden terms as regression fixtures.",
        expires_when: "Fixtures are migrated to synthetic tokens",
    },
    IdentityExemption {
        path: "vac-rs/crates/control-plane/control-plane/src/control_plane/donor_domain_contract.rs",
        reason: "Donor domain contract quotes retired donor source identifiers as a quarantine check.",
        expires_when: "Donor source identifiers are externalized from scanned source",
    },
    IdentityExemption {
        path: "vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_live_scanner_policy.rs",
        reason: "VAC-Init scanner implementation matches retired donor paths as quarantine input.",
        expires_when: "Scanner fixtures move outside identity-check scan roots",
    },
    IdentityExemption {
        path: "vac-rs/tui/src/workflow_browser.rs",
        reason: "TUI workflow browser tests quote forbidden terms as regression fixtures.",
        expires_when: "Fixtures are migrated to synthetic tokens",
    },
    IdentityExemption {
        path: "vac-rs/crates/surfaces/tui/src/workflow_browser.rs",
        reason: "TUI workflow browser renders canonical scanner labels and quotes regression fixtures.",
        expires_when: "Fixtures are migrated to synthetic tokens",
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityCheckFinding {
    pub path: String,
    pub line: usize,
    pub term: String,
    pub reason: String,
    pub severity: String,
    pub replacement_hint: String,
    pub snippet: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityCheckReport {
    scanned_file_count: usize,
    findings: Vec<IdentityCheckFinding>,
}

impl IdentityCheckReport {
    pub fn scanned_file_count(&self) -> usize {
        self.scanned_file_count
    }

    pub fn finding_count(&self) -> usize {
        self.findings.len()
    }

    pub fn passed(&self) -> bool {
        self.findings.is_empty()
    }

    pub fn findings(&self) -> &[IdentityCheckFinding] {
        &self.findings
    }

    pub fn exemptions(&self) -> &'static [IdentityExemption] {
        IDENTITY_CHECK_EXEMPTIONS
    }

    pub fn summary_line(&self) -> String {
        format!(
            "scanned={} findings={}",
            self.scanned_file_count,
            self.finding_count()
        )
    }

    pub fn failure_reason(&self) -> Option<String> {
        let finding_count = self.finding_count();
        if finding_count == 0 {
            return None;
        }
        let first = &self.findings[0];
        let mut reason = format!(
            "identity check found {} forbidden term{}: {}:{} {}",
            finding_count,
            if finding_count == 1 { "" } else { "s" },
            first.path,
            first.line,
            first.term
        );
        if finding_count > 1 {
            reason.push_str(&format!(" (+{} more)", finding_count - 1));
        }
        Some(reason)
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = vec![format!("identity check: {}", self.summary_line())];
        if self.findings.is_empty() {
            lines.push("identity check findings: <none>".to_string());
        } else {
            lines.push("identity check findings:".to_string());
            for finding in &self.findings {
                lines.push(format!(
                    "  {}:{} term=\"{}\" severity={} reason=\"{}\" hint=\"{}\" -> {}",
                    finding.path,
                    finding.line,
                    finding.term,
                    finding.severity,
                    finding.reason,
                    finding.replacement_hint,
                    finding.snippet
                ));
            }
        }

        if self.exemptions().is_empty() {
            lines.push("identity check exemptions: <none>".to_string());
        } else {
            lines.push("identity check exemptions:".to_string());
            for exemption in self.exemptions() {
                lines.push(format!(
                    "  {} reason=\"{}\" expires_when=\"{}\"",
                    exemption.path, exemption.reason, exemption.expires_when
                ));
            }
        }
        lines
    }

    pub fn render_text(&self) -> String {
        self.render_lines().join("\n")
    }
}

fn normalize_for_comparison(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

pub fn load_identity_check_report(root: impl AsRef<Path>) -> IdentityCheckReport {
    let root = root.as_ref();
    let mut files = Vec::new();
    for relative_root in IDENTITY_CHECK_SCAN_ROOTS {
        collect_identity_check_files(root, &root.join(relative_root), &mut files);
    }
    files.sort();
    files.dedup();

    let mut findings = Vec::new();
    for path in &files {
        let Ok(contents) = fs::read_to_string(path) else {
            continue;
        };
        let relative = path.strip_prefix(root).unwrap_or(path.as_path());
        let relative = relative.display().to_string();
        for (line_number, line) in contents.lines().enumerate() {
            let normalized_line = normalize_for_comparison(line);
            for term in FORBIDDEN_TERMS {
                let normalized_term = normalize_for_comparison(term.term);
                if normalized_line.contains(&normalized_term) {
                    findings.push(IdentityCheckFinding {
                        path: relative.clone(),
                        line: line_number + 1,
                        term: term.term.to_string(),
                        reason: term.reason.to_string(),
                        severity: term.severity.as_str().to_string(),
                        replacement_hint: term.replacement_hint.to_string(),
                        snippet: line.trim().chars().take(160).collect(),
                    });
                }
            }
        }
    }
    findings.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.line.cmp(&right.line))
            .then(left.term.cmp(&right.term))
    });

    IdentityCheckReport {
        scanned_file_count: files.len(),
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::load_identity_check_report;
    use std::fs;

    #[test]
    fn identity_check_reports_product_terms_and_ignores_allowlisted_legacy_docs() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let term = concat!("vac", "_tui", "_runtime");

        fs::create_dir_all(tempdir.path().join("vac-rs/src")).expect("product source dir");
        fs::create_dir_all(tempdir.path().join("donor/vac/src")).expect("donor source dir");
        fs::create_dir_all(tempdir.path().join(".git")).expect("git dir");
        fs::create_dir_all(tempdir.path().join("docs/product")).expect("product docs dir");

        fs::write(
            tempdir.path().join("vac-rs/src/lib.rs"),
            format!("pub const LEGACY: &str = \"{term}\";"),
        )
        .expect("product source fixture");
        fs::write(
            tempdir.path().join("donor/vac/src/lib.rs"),
            format!("pub const LEGACY: &str = \"{term}\";"),
        )
        .expect("donor source fixture");
        fs::write(
            tempdir.path().join(".git/HEAD"),
            format!("ref: refs/heads/{term}"),
        )
        .expect("git fixture");
        fs::write(
            tempdir.path().join("docs/product/CAPABILITY_MAP.md"),
            format!("quarantine {term}"),
        )
        .expect("allowlisted legacy docs fixture");

        let report = load_identity_check_report(tempdir.path());
        assert_eq!(report.scanned_file_count(), 1);
        assert_eq!(report.finding_count(), 1);
        assert_eq!(report.findings()[0].path, "vac-rs/src/lib.rs");
        assert_eq!(report.findings()[0].line, 1);
        assert_eq!(report.findings()[0].term, term);
        assert_eq!(report.findings()[0].severity, "fail");
        assert!(report.findings()[0].reason.contains("root TUI"));
        assert!(report.findings()[0].replacement_hint.contains("root TUI"));

        let rendered = report.render_text();
        assert!(rendered.contains("term=\""));
        assert!(rendered.contains("severity=fail"));
        assert!(rendered.contains("reason=\""));
        assert!(rendered.contains("hint=\""));
        assert!(rendered.contains("identity check exemptions:"));
        assert!(rendered.contains("docs/product/CAPABILITY_MAP.md"));
        assert!(rendered.contains("Plan 19 root feature conversion closes"));
    }

    #[test]
    fn identity_check_scans_workflow_docs_case_insensitively() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("docs/workflow-control-plane")).expect("docs dir");
        fs::write(
            tempdir.path().join("docs/workflow-control-plane/plan.md"),
            "This still says DUPLICATE TUI in a plan.",
        )
        .expect("doc fixture");

        let report = load_identity_check_report(tempdir.path());
        assert_eq!(report.finding_count(), 1);
        assert_eq!(report.findings()[0].term, "duplicate TUI");
    }

    #[test]
    fn identity_check_exempts_scheduled_snapshots_but_keeps_normal_docs_strict() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("docs/workflow-control-plane")).expect("docs dir");
        fs::create_dir_all(tempdir.path().join("docs/scheduled-audits/2026-05-25"))
            .expect("scheduled audits dir");
        fs::create_dir_all(tempdir.path().join("docs/scheduled-plans/commit-batches"))
            .expect("scheduled plans dir");

        fs::write(
            tempdir.path().join("docs/workflow-control-plane/plan.md"),
            "Normal docs must still fail on duplicate TUI.",
        )
        .expect("normal docs fixture");
        fs::write(
            tempdir
                .path()
                .join("docs/scheduled-audits/2026-05-25/0804-repo-sentinel.md"),
            "Historical audit snapshot quoted duplicate TUI.",
        )
        .expect("scheduled audit fixture");
        fs::write(
            tempdir
                .path()
                .join("docs/scheduled-plans/commit-batches/2026-05-24-uncommitted-batches.md"),
            "Generated plan snapshot quoted duplicate TUI.",
        )
        .expect("scheduled plan fixture");

        let report = load_identity_check_report(tempdir.path());
        assert_eq!(report.finding_count(), 1);
        assert_eq!(
            report.findings()[0].path,
            "docs/workflow-control-plane/plan.md"
        );
        assert_eq!(report.findings()[0].term, "duplicate TUI");
    }
}

fn collect_identity_check_files(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) {
    if !dir.exists() {
        return;
    }
    if should_skip_path(root, dir) {
        return;
    }
    if dir.is_file() {
        if should_scan_file(dir) {
            files.push(dir.to_path_buf());
        }
        return;
    }
    if !dir.is_dir() {
        return;
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if should_skip_path(root, &path) {
            continue;
        }

        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            collect_identity_check_files(root, &path, files);
        } else if file_type.is_file() && should_scan_file(&path) {
            files.push(path);
        }
    }
}

fn should_skip_path(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };

    if identity_exemption_for_path(relative).is_some() {
        return true;
    }

    relative.components().any(|component| {
        matches!(
            component.as_os_str(),
            os if os == OsStr::new(".git")
                || os == OsStr::new("donor")
                || os == OsStr::new("target")
        )
    })
}

fn identity_exemption_for_path(relative: &Path) -> Option<&'static IdentityExemption> {
    IDENTITY_CHECK_EXEMPTIONS
        .iter()
        .find(|exemption| identity_exemption_matches(relative, exemption.path))
}

fn identity_exemption_matches(relative: &Path, exemption_path: &str) -> bool {
    if let Some(prefix) = exemption_path.strip_suffix("/**") {
        return relative.starts_with(prefix);
    }

    relative == Path::new(exemption_path)
}

fn should_scan_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name() else {
        return false;
    };
    if matches!(file_name, name if name == OsStr::new("Cargo.toml") || name == OsStr::new("Cargo.lock") || name == OsStr::new("package.json") || name == OsStr::new("README"))
    {
        return true;
    }

    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("rs" | "md" | "toml" | "yaml" | "yml" | "json" | "txt" | "lock")
    )
}
