use clap::Parser;
use clap::Subcommand;
use serde_yaml::Value;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use vac_core::control_plane::load_architecture_invariant_report;
use vac_core::control_plane::load_control_plane_registry_report;
use vac_core::control_plane::load_docs_maintenance_report;
use vac_core::control_plane::load_donor_domain_contract_report;
use vac_core::control_plane::load_donor_status_report;
use vac_core::control_plane::load_ownership_scan_report;
use vac_core::control_plane::load_policy_doctor_report_for_path;
use vac_core::control_plane::load_surface_doctor_report;
use vac_core::control_plane::load_workflow_run_report;
use vac_core::control_plane::workflow_runner::WorkflowExecutionEvent;
use vac_core::control_plane::{
    BuildCheckRequest, DoctorCheckReport, DoctorFinding, DoctorKind, DoctorSeverity, DoctorStatus,
    ReleaseGateContext, aggregate_doctor_release, run_build_check,
};
use vac_core::project_workspace::ProjectWorkspaceBootstrapError;
use vac_core::project_workspace::build_project_workspace_confirmation_dialog;
use vac_core::project_workspace::build_soft_workspace_bootstrap_plan;
use vac_core::project_workspace::build_strict_workspace_promotion_preview;
use vac_core::project_workspace::load_project_workspace_report_with_options;
use vac_core::project_workspace::materialize_soft_workspace_bootstrap;
use vac_core::project_workspace::materialize_strict_workspace_promotion;

#[path = "doctor/runtime_owner_gates.rs"]
mod runtime_owner_gates;

#[derive(Debug, Parser)]
pub struct DoctorCommand {
    #[clap(subcommand)]
    command: DoctorSubcommand,
}

impl DoctorCommand {
    pub fn run(self) {
        match self.command {
            DoctorSubcommand::Registry(command) => command.run(),
            DoctorSubcommand::Architecture(command) => command.run(),
            DoctorSubcommand::Docs(command) => command.run(),
            DoctorSubcommand::Workflow(command) => command.run(),
            DoctorSubcommand::Ownership(command) => command.run(),
            DoctorSubcommand::Policy(command) => command.run(),
            DoctorSubcommand::ProjectWorkspace(command) => command.run(),
            DoctorSubcommand::Surfaces(command) => command.run(),
            DoctorSubcommand::Build(command) => command.run(),
            DoctorSubcommand::Evidence(command) => command.run(),
            DoctorSubcommand::EvidenceSigning(command) => command.run(),
            DoctorSubcommand::Layering(command) => command.run(),
            DoctorSubcommand::Memory(command) => command.run(),
            DoctorSubcommand::Init(command) => command.run(),
            DoctorSubcommand::Migrations(command) => command.run(),
            DoctorSubcommand::Donor(command) => command.run(),
            DoctorSubcommand::Release(command) => command.run(),
            DoctorSubcommand::RuntimeOwnerGates(args) => {
                runtime_owner_gates::run_external_subcommand(args)
            }
        }
    }
}

#[derive(Debug, Subcommand)]
enum DoctorSubcommand {
    /// Load the control plane registry and render diagnostics.
    Registry(RegistryDoctorCommand),
    /// Check architecture invariants and compatibility transport classification.
    Architecture(ArchitectureDoctorCommand),
    /// Check docs and manifests for stale references and missing files.
    Docs(DocsDoctorCommand),
    /// Inspect workflow runner support for loaded workflow manifests.
    Workflow(WorkflowDoctorCommand),
    /// Scan capability ownership coverage across loaded manifests.
    Ownership(OwnershipDoctorCommand),
    /// Show zero-config project workspace classification and setup diagnostics.
    ProjectWorkspace(ProjectWorkspaceDoctorCommand),
    /// Show loaded surface manifests, route counts, and diagnostics.
    Surfaces(SurfacesDoctorCommand),
    /// Show loaded policy manifests, decisions, and coverage.
    Policy(PolicyDoctorCommand),
    /// Execute cargo check to verify root product compilation and warning checks.
    Build(BuildDoctorCommand),
    /// Validate hash-chain evidence records.
    Evidence(EvidenceDoctorCommand),
    /// Validate evidence signing policy and key availability.
    EvidenceSigning(EvidenceSigningDoctorCommand),
    /// Validate workspace crate layering migration status.
    Layering(LayeringDoctorCommand),
    /// Validate memory record persistence and governance shape.
    Memory(MemoryDoctorCommand),
    /// Validate VAC-Init lifecycle state persistence.
    Init(InitDoctorCommand),
    /// Validate registry migration records and compatibility cleanup.
    Migrations(MigrationDoctorCommand),
    /// Render donor migration status from declared capability manifests.
    Donor(DonorDoctorCommand),
    /// Execute the release gate workflow and render a production readiness report.
    Release(ReleaseDoctorCommand),
    #[command(external_subcommand)]
    RuntimeOwnerGates(Vec<OsString>),
}

#[derive(Debug, Parser)]
struct RegistryDoctorCommand {
    /// Path to start discovery from.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,
}

impl RegistryDoctorCommand {
    fn run(self) {
        let report = load_control_plane_registry_report(&self.path);
        println!("{}", report.render_text());
        std::process::exit(report.cli_exit_code());
    }
}

#[derive(Debug, Parser)]
struct ArchitectureDoctorCommand {
    /// Path to start discovery from.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,

    /// Validate planned/partial/ready visibility contract.
    #[arg(long = "visibility-contract", default_value_t = false, hide = true)]
    visibility_contract: bool,

}

impl ArchitectureDoctorCommand {
    fn run(self) {
        let report = load_architecture_invariant_report(&self.path);
        println!("{}", report.render_text());
        if self.visibility_contract {
            println!("surface visibility contract: enforced");
            println!("  - planned + visible=true is rejected");
            println!("  - ready + visible=false is rejected");
            println!("  - partial routes require reason/degraded guidance");
        }
        if report.is_failure() {
            std::process::exit(1);
        }
    }
}

#[derive(Debug, Parser)]
struct DocsDoctorCommand {
    /// Path to start discovery from.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,

    /// Validate planned/partial/ready visibility contract.
    #[arg(long = "visibility-contract", default_value_t = false, hide = true)]
    visibility_contract: bool,

}

impl DocsDoctorCommand {
    fn run(self) {
        let report = load_docs_maintenance_report(&self.path);
        println!("{}", report.render_text());
        if self.visibility_contract {
            println!("surface visibility contract: enforced");
            println!("  - planned + visible=true is rejected");
            println!("  - ready + visible=false is rejected");
            println!("  - partial routes require reason/degraded guidance");
        }
        if report.is_failure() {
            std::process::exit(1);
        }
    }
}

#[derive(Debug, Parser)]
struct WorkflowDoctorCommand {
    /// Path to start discovery from.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,

    /// Validate planned/partial/ready visibility contract.
    #[arg(long = "visibility-contract", default_value_t = false, hide = true)]
    visibility_contract: bool,

}

impl WorkflowDoctorCommand {
    fn run(self) {
        let report = load_workflow_run_report(&self.path);
        println!("{}", report.render_text());
        if self.visibility_contract {
            println!("surface visibility contract: enforced");
            println!("  - planned + visible=true is rejected");
            println!("  - ready + visible=false is rejected");
            println!("  - partial routes require reason/degraded guidance");
        }

        let aggregate = build_real_doctor_aggregate(&self.path);

        println!("VAC Doctor Taxonomy Aggregate");
        println!("========================================");
        println!("release_status: {:?}", aggregate.release_status);
        for reason in &aggregate.blocking_reasons {
            println!("  BLOCK: {reason}");
        }
        if aggregate.exit_code != 0 {
            std::process::exit(aggregate.exit_code);
        }
        if report.is_failure() {
            std::process::exit(1);
        }
    }
}

#[derive(Debug, Parser)]
struct OwnershipDoctorCommand {
    /// Path to start discovery from.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,

    /// Validate planned/partial/ready visibility contract.
    #[arg(long = "visibility-contract", default_value_t = false, hide = true)]
    visibility_contract: bool,

}

impl OwnershipDoctorCommand {
    fn run(self) {
        let report = load_ownership_scan_report(&self.path);
        println!("{}", report.render_text());
        if self.visibility_contract {
            println!("surface visibility contract: enforced");
            println!("  - planned + visible=true is rejected");
            println!("  - ready + visible=false is rejected");
            println!("  - partial routes require reason/degraded guidance");
        }
        if report.is_failure() {
            std::process::exit(1);
        }
    }
}

#[derive(Debug, Parser)]
struct SurfacesDoctorCommand {
    /// Path to start discovery from.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,

    /// Validate planned/partial/ready visibility contract.
    #[arg(long = "visibility-contract", default_value_t = false)]
    visibility_contract: bool,
}

impl SurfacesDoctorCommand {
    fn run(self) {
        let report = load_surface_doctor_report(&self.path);
        println!("{}", report.render_text());
        if self.visibility_contract {
            println!("surface visibility contract: enforced");
            println!("  - planned + visible=true is rejected");
            println!("  - ready + visible=false is rejected");
            println!("  - partial routes require reason/degraded guidance");
        }
        if report.is_failure() {
            std::process::exit(1);
        }
    }
}

#[derive(Debug, Parser)]
struct PolicyDoctorCommand {
    /// Path to start discovery from.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,

    /// Validate planned/partial/ready visibility contract.
    #[arg(long = "visibility-contract", default_value_t = false, hide = true)]
    visibility_contract: bool,

}

impl PolicyDoctorCommand {
    fn run(self) {
        let report = load_policy_doctor_report_for_path(&self.path);
        println!("{}", report.render_text());
        if self.visibility_contract {
            println!("surface visibility contract: enforced");
            println!("  - planned + visible=true is rejected");
            println!("  - ready + visible=false is rejected");
            println!("  - partial routes require reason/degraded guidance");
        }
        if report.is_failure() {
            std::process::exit(1);
        }
    }
}

#[derive(Debug, Parser)]
struct ProjectWorkspaceDoctorCommand {
    /// Path to start discovery from.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,

    /// Treat missing `.vac` as a strict product-repo gate error instead of a user-project setup warning.
    #[arg(long, default_value_t = false)]
    strict_product_repo: bool,

    /// Create a lightweight soft `.vac` workspace after explicit approval.
    #[arg(long, default_value_t = false)]
    bootstrap_soft: bool,

    /// Explicitly approve disk writes for `--bootstrap-soft`.
    #[arg(long, default_value_t = false)]
    yes: bool,

    /// Render the rich first-run setup dialog model without writing files.
    #[arg(long, default_value_t = false)]
    setup_dialog_preview: bool,

    /// Render the strict promotion preview without generating manifests.
    #[arg(long, default_value_t = false)]
    promote_strict_preview: bool,

    /// Promote an existing soft/curated `.vac` workspace to strict directories after explicit approval.
    #[arg(long, default_value_t = false)]
    promote_strict: bool,

    /// Validate planned/partial/ready visibility contract.
    #[arg(long = "visibility-contract", default_value_t = false, hide = true)]
    visibility_contract: bool,

}

impl ProjectWorkspaceDoctorCommand {
    fn run(self) {
        if self.setup_dialog_preview {
            match build_project_workspace_confirmation_dialog(&self.path) {
                Some(dialog) => println!("{}", dialog.render_text()),
                None => println!(
                    "setup_dialog: unavailable; project workspace is already explicit or strict-only"
                ),
            }
            return;
        }

        if self.promote_strict_preview {
            println!(
                "{}",
                build_strict_workspace_promotion_preview(&self.path).render_text()
            );
            return;
        }

        if self.promote_strict {
            if !self.yes {
                println!(
                    "{}",
                    build_strict_workspace_promotion_preview(&self.path).render_text()
                );
                println!(
                    "approval_required: rerun with `--promote-strict --yes` after reviewing strict manifest migration"
                );
                std::process::exit(1);
            }

            match materialize_strict_workspace_promotion(&self.path, true) {
                Ok(result) => {
                    println!("VAC Project Workspace Strict Promotion Applied");
                    println!("==============================================");
                    println!("mode: {}", result.mode.as_str());
                    println!("created_paths:");
                    for path in result.created_paths {
                        println!("  - {}", path.display());
                    }
                }
                Err(err) => {
                    println!("strict_promotion_failed: {}", err.as_str());
                    std::process::exit(1);
                }
            }
            return;
        }

        if self.bootstrap_soft {
            if !self.yes {
                let plan = build_soft_workspace_bootstrap_plan(&self.path);
                println!("{}", plan.render_text());
                println!("approval_required: rerun with `--bootstrap-soft --yes` to create files");
                std::process::exit(1);
            }

            match materialize_soft_workspace_bootstrap(&self.path, true) {
                Ok(result) => {
                    println!("VAC Project Workspace Bootstrap Applied");
                    println!("=======================================");
                    println!("mode: {}", result.mode.as_str());
                    println!("created_paths:");
                    for path in result.created_paths {
                        println!("  - {}", path.display());
                    }
                    println!("local_only_dirs:");
                    for path in result.local_only_dirs {
                        println!("  - {}", path.display());
                    }
                }
                Err(ProjectWorkspaceBootstrapError::ApprovalRequired) => {
                    println!(
                        "approval_required: rerun with `--bootstrap-soft --yes` to create files"
                    );
                    std::process::exit(1);
                }
                Err(err) => {
                    println!("bootstrap_failed: {}", err.as_str());
                    std::process::exit(1);
                }
            }
        }

        let report =
            load_project_workspace_report_with_options(&self.path, self.strict_product_repo);
        println!("{}", report.render_text());
        if self.visibility_contract {
            println!("surface visibility contract: enforced");
            println!("  - planned + visible=true is rejected");
            println!("  - ready + visible=false is rejected");
            println!("  - partial routes require reason/degraded guidance");
        }
        if report.is_failure() {
            std::process::exit(1);
        }
    }
}

#[derive(Debug, Parser)]
struct EvidenceDoctorCommand {
    /// Path to start discovery from.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,

    /// Check canonical evidence field coverage in addition to hash-chain records.
    #[arg(long = "canonical-coverage", default_value_t = false)]
    canonical_coverage: bool,
}

impl EvidenceDoctorCommand {
    fn run(self) {
        if self.canonical_coverage {
            let coverage = vac_core::control_plane::vac_init_evidence_chain::canonical_evidence_field_coverage_report();
            println!("vac doctor evidence --canonical-coverage");
            println!("included: {}", coverage.included.len());
            println!("excluded: {}", coverage.excluded.join(","));
            if !coverage.missing.is_empty() { println!("missing: {}", coverage.missing.join(",")); }
            if !coverage.unexpected.is_empty() { println!("unexpected: {}", coverage.unexpected.join(",")); }
            std::process::exit(if coverage.is_pass() { 0 } else { 1 });
        }
        let report = inspect_evidence_records(&self.path);
        println!("{}", report.render_text("evidence"));
        std::process::exit(report.exit_code());
    }
}

#[derive(Debug, Parser)]
struct EvidenceSigningDoctorCommand {
    /// Path to start discovery from.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,
}

impl EvidenceSigningDoctorCommand {
    fn run(self) {
        let mut report = SimpleDoctorInspection::default();
        let signing_required = evidence_signing_required_for_workspace(&self.path);
        let key_present = std::env::var("VAC_EVIDENCE_ED25519_SIGNING_KEY_BASE64")
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
        if signing_required && !key_present {
            report.errors.push(
                "evidence signing required but VAC_EVIDENCE_ED25519_SIGNING_KEY_BASE64 is missing"
                    .to_string(),
            );
        } else if signing_required {
            report.infos.push("evidence signing required and key material is present".to_string());
        } else if key_present {
            report.infos.push("evidence signing optional; key material is present".to_string());
        } else {
            report.warnings.push(
                "evidence signing optional and key missing; writer will emit unsigned governance warning"
                    .to_string(),
            );
        }
        println!("{}", report.render_text("evidence-signing"));
        std::process::exit(report.exit_code());
    }
}

fn evidence_signing_required_for_workspace(root: &PathBuf) -> bool {
    if matches!(
        std::env::var("VAC_EVIDENCE_SIGNING_REQUIRED")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "required"
    ) {
        return true;
    }
    let policies = root.join(".vac/policies");
    let Ok(entries) = fs::read_dir(policies) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("yaml") {
            continue;
        }
        let Ok(value) = load_yaml(&path) else {
            continue;
        };
        if nested_scalar(&value, &["evidence", "signing", "required"])
            .map(|value| matches!(value.as_str(), "true" | "1" | "yes" | "required"))
            .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

#[derive(Debug, Parser)]
struct LayeringDoctorCommand {
    /// Path to start discovery from.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,
}

impl LayeringDoctorCommand {
    fn run(self) {
        let status_path = self.path.join(".vac/registry/layering-status.yaml");
        println!("VAC Doctor Layering");
        println!("===================");
        println!("status_file: {}", status_path.display());
        match fs::read_to_string(&status_path) {
            Ok(contents) => {
                let failed = contents.contains("status: failed");
                print!("{contents}");
                if failed {
                    std::process::exit(1);
                }
            }
            Err(error) => {
                println!("layering_status: missing");
                println!("error: {error}");
                std::process::exit(1);
            }
        }
    }
}

#[derive(Debug, Parser)]
struct MemoryDoctorCommand {
    /// Path to start discovery from.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,
}

impl MemoryDoctorCommand {
    fn run(self) {
        let report = inspect_memory_records(&self.path);
        println!("{}", report.render_text("memory"));
        std::process::exit(report.exit_code());
    }
}

#[derive(Debug, Parser)]
struct InitDoctorCommand {
    /// Path to start discovery from.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,
}

impl InitDoctorCommand {
    fn run(self) {
        let report = inspect_init_state(&self.path);
        println!("{}", report.render_text("init"));
        std::process::exit(report.exit_code());
    }
}

#[derive(Debug, Clone, Default)]
struct SimpleDoctorInspection {
    errors: Vec<String>,
    warnings: Vec<String>,
    infos: Vec<String>,
}

impl SimpleDoctorInspection {
    #[allow(dead_code)]
    fn pass(message: impl Into<String>) -> Self {
        Self {
            infos: vec![message.into()],
            ..Self::default()
        }
    }

    fn exit_code(&self) -> i32 {
        if self.errors.is_empty() { 0 } else { 1 }
    }

    fn status(&self) -> DoctorStatus {
        if !self.errors.is_empty() {
            DoctorStatus::Failed
        } else if !self.warnings.is_empty() {
            DoctorStatus::Warning
        } else {
            DoctorStatus::Pass
        }
    }

    fn render_text(&self, label: &str) -> String {
        let status = if !self.errors.is_empty() {
            "FAIL"
        } else if !self.warnings.is_empty() {
            "WARN"
        } else {
            "PASS"
        };
        let mut lines = vec![format!("vac doctor {label}: {status}")];
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

    fn as_check(&self, kind: DoctorKind) -> DoctorCheckReport {
        let mut findings = Vec::new();
        for warning in &self.warnings {
            findings.push(DoctorFinding {
                code: format!("doctor.{}.warning", kind.as_str()),
                severity: DoctorSeverity::Warning,
                message: warning.clone(),
                path: None,
                hint: None,
            });
        }
        for error in &self.errors {
            findings.push(DoctorFinding {
                code: format!("doctor.{}.error", kind.as_str()),
                severity: DoctorSeverity::Error,
                message: error.clone(),
                path: None,
                hint: None,
            });
        }
        DoctorCheckReport::new(kind, self.status(), findings)
    }
}

fn doctor_check_from_failure(
    kind: DoctorKind,
    failed: bool,
    message: impl Into<String>,
) -> DoctorCheckReport {
    let message = message.into();
    let findings = if failed {
        vec![DoctorFinding {
            code: format!("doctor.{}.failed", kind.as_str()),
            severity: DoctorSeverity::Error,
            message,
            path: None,
            hint: None,
        }]
    } else {
        vec![DoctorFinding {
            code: format!("doctor.{}.source", kind.as_str()),
            severity: DoctorSeverity::Info,
            message,
            path: None,
            hint: None,
        }]
    };
    let status = if failed {
        DoctorStatus::Failed
    } else {
        DoctorStatus::Pass
    };
    DoctorCheckReport::new(kind, status, findings)
}

fn build_not_evaluated_build_check(message: impl Into<String>) -> DoctorCheckReport {
    DoctorCheckReport::new(
        DoctorKind::Build,
        DoctorStatus::Warning,
        vec![DoctorFinding {
            code: "doctor.build.not_evaluated".to_string(),
            severity: DoctorSeverity::Warning,
            message: message.into(),
            path: None,
            hint: Some("run `cargo build --manifest-path vac-rs/Cargo.toml -p vac-surface-cli` or `vac doctor build .` before release aggregation".to_string()),
        }],
    )
}

fn build_artifact_doctor_check(path: &Path) -> DoctorCheckReport {
    let binary_name = if cfg!(windows) { "vac.exe" } else { "vac" };
    let binary = path.join("vac-rs").join("target").join("debug").join(binary_name);
    let Ok(binary_meta) = fs::metadata(&binary) else {
        return build_not_evaluated_build_check(format!(
            "build artifact not found at {}; run cargo build for vac-cli",
            binary.display()
        ));
    };
    let Ok(binary_modified) = binary_meta.modified() else {
        return build_not_evaluated_build_check(format!(
            "build artifact mtime unreadable at {}; rebuild before release aggregation",
            binary.display()
        ));
    };

    let mut latest_source = None;
    for source_root in [
        path.join("vac-rs").join("Cargo.toml"),
        path.join("vac-rs").join("Cargo.lock"),
        path.join("vac-rs").join("core"),
        path.join("vac-rs").join("crates"),
    ] {
        collect_latest_source_mtime(&source_root, &mut latest_source);
    }

    if latest_source.is_some_and(|modified| modified > binary_modified) {
        return build_not_evaluated_build_check(format!(
            "build artifact {} is older than source changes; rebuild vac-cli",
            binary.display()
        ));
    }

    DoctorCheckReport::new(
        DoctorKind::Build,
        DoctorStatus::Pass,
        vec![DoctorFinding {
            code: "doctor.build.fresh_artifact".to_string(),
            severity: DoctorSeverity::Info,
            message: format!("fresh build artifact found at {}", binary.display()),
            path: Some(binary.display().to_string()),
            hint: None,
        }],
    )
}

fn collect_latest_source_mtime(path: &Path, latest: &mut Option<SystemTime>) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };
    if metadata.is_file() {
        if let Ok(modified) = metadata.modified() {
            if latest.is_none_or(|current| modified > current) {
                *latest = Some(modified);
            }
        }
        return;
    }
    if !metadata.is_dir() {
        return;
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return;
    };
    if matches!(name, "target" | ".git" | "donor") {
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        collect_latest_source_mtime(&entry.path(), latest);
    }
}

fn count_release_ownership_context(path: &PathBuf) -> (usize, usize) {
    let mut hard_quarantine_count = 0usize;
    let mut unowned_file_count = 0usize;
    let report_path = path.join(".vac/registry/ownership/report.yaml");
    if let Ok(source) = std::fs::read_to_string(&report_path) {
        for line in source.lines() {
            let value = line.trim();
            if value == "status: hard_quarantine" || value == "level: hard_quarantine" {
                hard_quarantine_count += 1;
            }
            if value == "status: unowned" {
                unowned_file_count += 1;
            }
        }
    }
    (hard_quarantine_count, unowned_file_count)
}

fn build_real_doctor_aggregate(path: &PathBuf) -> vac_core::control_plane::DoctorAggregateReport {
    let registry_report = load_control_plane_registry_report(path);
    let surface_report = load_surface_doctor_report(path);
    let policy_report = load_policy_doctor_report_for_path(path);
    let ownership_report = load_ownership_scan_report(path);
    let workflow_report = load_workflow_run_report(path);
    let evidence = inspect_evidence_records(path);
    let memory = inspect_memory_records(path);
    let init = inspect_init_state(path);
    let (hard_quarantine_count, unowned_file_count) = count_release_ownership_context(path);

    aggregate_doctor_release(
        vec![
            doctor_check_from_failure(
                DoctorKind::Registry,
                registry_report.cli_exit_code() != 0,
                "registry doctor evaluated real registry report",
            ),
            doctor_check_from_failure(
                DoctorKind::Surfaces,
                surface_report.is_failure(),
                "surface doctor evaluated real surface report",
            ),
            doctor_check_from_failure(
                DoctorKind::Policy,
                policy_report.is_failure(),
                "policy doctor evaluated real policy report",
            ),
            doctor_check_from_failure(
                DoctorKind::Ownership,
                ownership_report.is_failure(),
                "ownership doctor evaluated real ownership report",
            ),
            doctor_check_from_failure(
                DoctorKind::Workflow,
                workflow_report.is_failure(),
                "workflow doctor evaluated real workflow report",
            ),
            evidence.as_check(DoctorKind::Evidence),
            build_artifact_doctor_check(path),
            memory.as_check(DoctorKind::Memory),
            init.as_check(DoctorKind::Init),
        ],
        ReleaseGateContext {
            hard_quarantine_count,
            broken_evidence_chain: !evidence.errors.is_empty(),
            unowned_file_count,
            policy_loaded: !policy_report.is_failure(),
            registry_loaded: registry_report.cli_exit_code() == 0,
        },
    )
}

fn inspect_evidence_records(root: &PathBuf) -> SimpleDoctorInspection {
    let evidence_dir = root.join(".vac/registry/evidence");
    if !evidence_dir.exists() {
        return SimpleDoctorInspection {
            warnings: vec![
                "no evidence directory yet; evidence writer lands in Production Hardening C/D"
                    .to_string(),
            ],
            ..SimpleDoctorInspection::default()
        };
    }

    let mut report = SimpleDoctorInspection::default();
    let mut count = 0usize;
    collect_yaml_files(&evidence_dir, &mut |path| {
        count += 1;
        match load_yaml(path) {
            Ok(value) => {
                if scalar(&value, "kind").as_deref() != Some("evidence") {
                    report
                        .errors
                        .push(format!("{}: expected kind evidence", path.display()));
                }
                if nested_scalar(&value, &["chain", "self_hash"])
                    .unwrap_or_default()
                    .is_empty()
                {
                    report
                        .errors
                        .push(format!("{}: missing chain.self_hash", path.display()));
                }
            }
            Err(err) => report.errors.push(format!("{}: {err}", path.display())),
        }
    });
    if count == 0 {
        report
            .warnings
            .push("evidence directory exists but has no evidence records".to_string());
    } else {
        report
            .infos
            .push(format!("validated {count} evidence record(s)"));
    }
    report
}

fn inspect_memory_records(root: &PathBuf) -> SimpleDoctorInspection {
    let memory_dir = root.join(".vac/registry/memory");
    if !memory_dir.exists() {
        return SimpleDoctorInspection {
            warnings: vec![
                "no memory registry directory yet; memory store lands in Production Hardening D"
                    .to_string(),
            ],
            ..SimpleDoctorInspection::default()
        };
    }

    let mut report = SimpleDoctorInspection::default();
    let mut count = 0usize;
    collect_yaml_files(&memory_dir, &mut |path| {
        count += 1;
        match load_yaml(path) {
            Ok(value) => {
                if scalar(&value, "kind").as_deref() != Some("memory_record") {
                    report
                        .errors
                        .push(format!("{}: expected kind memory_record", path.display()));
                }
                if scalar(&value, "tier").unwrap_or_default().is_empty() {
                    report
                        .errors
                        .push(format!("{}: missing tier", path.display()));
                }
                let text = nested_scalar(&value, &["content", "text"]).unwrap_or_default();
                if looks_secret_like(&text) {
                    report.errors.push(format!(
                        "{}: secret-like memory content rejected",
                        path.display()
                    ));
                }
            }
            Err(err) => report.errors.push(format!("{}: {err}", path.display())),
        }
    });
    if count == 0 {
        report
            .warnings
            .push("memory directory exists but has no memory records".to_string());
    } else {
        report
            .infos
            .push(format!("validated {count} memory record(s)"));
    }
    report
}

fn inspect_init_state(root: &PathBuf) -> SimpleDoctorInspection {
    let state_path = root.join(".vac/.init/state.yaml");
    if !state_path.exists() {
        return SimpleDoctorInspection {
            errors: vec![
                "missing .vac/.init/state.yaml; run `vac init` or `vac init --dry-run` first"
                    .to_string(),
            ],
            ..SimpleDoctorInspection::default()
        };
    }
    match load_yaml(&state_path) {
        Ok(value) => {
            let mut report = SimpleDoctorInspection::default();
            if scalar(&value, "kind").as_deref() != Some("init_state") {
                report
                    .errors
                    .push("state kind must be init_state".to_string());
            }
            if scalar(&value, "id").as_deref() != Some("init.state") {
                report
                    .errors
                    .push("state id must be init.state".to_string());
            }
            match scalar(&value, "current_state").as_deref() {
                Some(
                    "ready"
                    | "doctor_verified"
                    | "manifests_synthesized"
                    | "discovered"
                    | "partition_selected"
                    | "policy_inferred",
                ) => {}
                Some(state) => report
                    .errors
                    .push(format!("invalid or failure current_state `{state}`")),
                None => report.errors.push("missing current_state".to_string()),
            }
            if report.errors.is_empty() {
                report
                    .infos
                    .push(format!("state_path: {}", state_path.display()));
            }
            report
        }
        Err(err) => SimpleDoctorInspection {
            errors: vec![format!("{}: {err}", state_path.display())],
            ..SimpleDoctorInspection::default()
        },
    }
}

fn collect_yaml_files(dir: &std::path::Path, visit: &mut impl FnMut(&std::path::Path)) {
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

fn load_yaml(path: &std::path::Path) -> Result<Value, String> {
    let source = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_yaml::from_str(&source).map_err(|err| err.to_string())
}

fn scalar(value: &Value, key: &str) -> Option<String> {
    nested_scalar(value, &[key])
}

fn nested_scalar(value: &Value, keys: &[&str]) -> Option<String> {
    let mut current = value;
    for key in keys {
        current = current
            .as_mapping()
            .and_then(|mapping| mapping.get(&Value::String((*key).to_string())))?;
    }
    match current {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn looks_secret_like(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    ["api_key", "secret", "password", "token=", "bearer "]
        .iter()
        .any(|needle| lower.contains(needle))
}

#[derive(Debug, Parser)]
struct MigrationDoctorCommand {
    /// Path to start discovery from.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,
}

impl MigrationDoctorCommand {
    fn run(self) {
        let migration_dir = self.path.join(".vac/registry/migrations");
        let mut checked = 0usize;
        let mut failures = Vec::new();

        if !migration_dir.is_dir() {
            failures.push(format!(
                "missing migration directory {}",
                migration_dir.display()
            ));
        } else {
            match fs::read_dir(&migration_dir) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|value| value.to_str()) != Some("yaml") {
                            continue;
                        }
                        checked += 1;
                        match fs::read_to_string(&path) {
                            Ok(source) => {
                                for required in [
                                    "schema_version:",
                                    "kind: migration",
                                    "id:",
                                    "changes:",
                                    "rollback:",
                                    "verification:",
                                ] {
                                    if !source.contains(required) {
                                        failures.push(format!(
                                            "{} missing `{}`",
                                            path.display(),
                                            required
                                        ));
                                    }
                                }
                                if source.contains("command: ") && !source.contains("runner:") {
                                    failures.push(format!(
                                        "{} has non-structured verification command",
                                        path.display()
                                    ));
                                }
                            }
                            Err(err) => {
                                failures.push(format!("{} read failed: {err}", path.display()))
                            }
                        }
                    }
                }
                Err(err) => failures.push(format!(
                    "{} read_dir failed: {err}",
                    migration_dir.display()
                )),
            }
        }

        println!(
            "migration doctor: checked={checked} failures={}",
            failures.len()
        );
        for failure in &failures {
            println!("  FAIL: {failure}");
        }
        if failures.is_empty() {
            println!("migration doctor: PASS");
        } else {
            std::process::exit(1);
        }
    }
}

#[derive(Debug, Parser)]
struct DonorDoctorCommand {
    /// Path to start discovery from.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,

    /// Include donor domain contract readiness from docs/donor-migration.
    #[arg(long, default_value_t = false)]
    contracts: bool,
}

impl DonorDoctorCommand {
    fn run(self) {
        let report = load_donor_status_report(&self.path);
        println!("{}", report.render_text());
        let mut failed = report.is_failure();

        if self.contracts {
            let contract_report = load_donor_domain_contract_report(&self.path);
            println!("{}", contract_report.render_text());
            failed |= contract_report.is_failure();
        }

        if failed {
            std::process::exit(1);
        }
    }
}

#[derive(Debug, Parser)]
struct BuildDoctorCommand {
    /// Path to start discovery from.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,
}

impl BuildDoctorCommand {
    fn run(self) {
        let request = BuildCheckRequest::for_repo_root(&self.path);
        println!(
            "Executing real build gate command: {}",
            request.command_display()
        );
        match run_build_check(&request) {
            Ok(report) => {
                println!("{}", report.render_text());
                if !report.success {
                    std::process::exit(1);
                }
            }
            Err(err) => {
                eprintln!("Failed to execute build check: {err}");
                std::process::exit(1);
            }
        }
    }
}

#[derive(Debug, Parser)]
struct ReleaseDoctorCommand {
    /// Path to start discovery from.
    #[arg(value_name = "PATH", default_value = ".")]
    path: PathBuf,
}

impl ReleaseDoctorCommand {
    fn run(self) {
        let aggregate = build_real_doctor_aggregate(&self.path);

        println!("VAC Doctor Taxonomy Aggregate");
        println!("========================================");
        println!("release_status: {:?}", aggregate.release_status);
        for reason in &aggregate.blocking_reasons {
            println!("  BLOCK: {reason}");
        }
        if aggregate.exit_code != 0 {
            std::process::exit(aggregate.exit_code);
        }

        let report = load_workflow_run_report(&self.path);

        if report.registry().is_failure() {
            println!("{}", report.registry().render_text());
            std::process::exit(1);
        }

        let release_gate = report
            .workflows()
            .iter()
            .find(|entry| entry.id == "maintenance.release-gate");

        let entry = match release_gate {
            Some(entry) => entry,
            None => {
                eprintln!("Error: 'maintenance.release-gate' workflow not found in registry.");
                std::process::exit(1);
            }
        };

        println!("VAC Release Gate Diagnostics");
        println!("========================================");
        println!("Workflow ID: {}", entry.id);
        println!("Title:       {}", entry.title);
        println!("Path:        {}", entry.path);
        println!("----------------------------------------");

        let mut failed = false;

        for (index, step) in entry.step_resolutions.iter().enumerate() {
            let fail_event = entry.execution.events().iter().find_map(|event| {
                if let WorkflowExecutionEvent::StepFailed {
                    index: ev_idx,
                    reason,
                    ..
                } = event
                {
                    if *ev_idx == index {
                        return Some(reason);
                    }
                }
                None
            });

            let success_event = entry.execution.events().iter().any(|event| {
                if let WorkflowExecutionEvent::StepSucceeded { index: ev_idx, .. } = event {
                    *ev_idx == index
                } else {
                    false
                }
            });

            if let Some(reason) = fail_event {
                println!("  [FAIL] {} ({})", step.id, step.uses);
                println!("         Reason: {}", reason);
                failed = true;
            } else if success_event {
                println!("  [OK]   {} ({})", step.id, step.uses);
            } else if let Some(blocked_reason) = &step.blocked_reason {
                println!("  [BLOCK] {} ({})", step.id, step.uses);
                println!("          Reason: {}", blocked_reason);
                failed = true;
            } else {
                println!("  [PEND] {} ({})", step.id, step.uses);
            }
        }

        println!("----------------------------------------");
        if entry.is_failure() || failed {
            println!("STATUS: RELEASE BLOCKED ❌");
            std::process::exit(1);
        } else {
            println!("STATUS: RELEASE READY ✅");
            std::process::exit(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ArchitectureDoctorCommand;
    use super::DocsDoctorCommand;
    use super::SurfacesDoctorCommand;
    use super::WorkflowDoctorCommand;
    use clap::CommandFactory;
    use std::fs;
    use std::path::PathBuf;
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    fn load_cli_surface_manifest() -> String {
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(".vac/surfaces/cli.yaml");
        fs::read_to_string(&manifest_path)
            .unwrap_or_else(|err| panic!("read {}: {err}", manifest_path.display()))
    }

    fn assert_cli_surface_declares(manifest: &str, command: &str) {
        let expected = format!("command: {command}");
        assert!(
            manifest.lines().any(|line| line.trim() == expected),
            "missing CLI surface route `{command}` in .vac/surfaces/cli.yaml"
        );
    }

    fn make_temp_dir() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("vac-doctor-workflow-{unique}"));
        fs::create_dir_all(path.join(".vac/workflows")).expect("create temp workflow dir");
        path
    }

    fn seed_core_capabilities(root: &std::path::Path) {
        fs::create_dir_all(root.join(".vac/capabilities")).expect("create temp capability dir");
        fs::write(
            root.join(".vac/capabilities/build.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.build
title: Build
status: ready
owner:
  crate: vac-surface-cli
  module: main
states:
  - empty
  - loading
  - success
  - failure
description: Build and maintenance validation gates for the root product.
depends_on:
  - vac.workflow
surfaces:
  palette: true
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
  approval_required_for:
    - execute_process
validation:
  commands:
    - CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
"#,
        )
        .expect("build capability manifest");
        fs::write(
            root.join(".vac/capabilities/identity-check.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.identity.check
title: Identity Check
status: partial
reason: Test fixture models the in-progress identity-check capability.
owner:
  crate: vac-core
  module: control_plane
states:
  - empty
  - loading
  - success
  - failure
description: Product identity and legacy term scanning for the root product.
depends_on:
  - vac.workflow
surfaces:
  palette: true
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands:
    - cargo +1.93.0 check -p vac-surface-cli
"#,
        )
        .expect("identity check capability manifest");
        fs::write(
            root.join(".vac/capabilities/workflow.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: partial
reason: Test fixture models the in-progress workflow capability.
owner:
  crate: vac-core
  module: control_plane
states:
  - empty
  - loading
  - success
  - failure
description: Typed workflow manifests, registry loading, and diagnostics.
depends_on:
  - vac.sessions
surfaces:
  palette: true
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
  approval_required_for:
    - write_files
    - execute_process
validation:
  commands:
    - CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
"#,
        )
        .expect("workflow capability manifest");
    }

    fn seed_architecture_environment(root: &std::path::Path, include_compatibility: bool) {
        fs::create_dir_all(root.join(".vac/registry")).expect("registry dir");
        fs::create_dir_all(root.join(".vac/capabilities")).expect("capabilities dir");
        fs::create_dir_all(root.join(".vac/workflows")).expect("workflows dir");
        fs::create_dir_all(root.join(".vac/policies")).expect("policies dir");
        fs::create_dir_all(root.join(".vac/surfaces")).expect("surfaces dir");
        fs::create_dir_all(root.join("vac-rs/tui")).expect("tui dir");
        fs::create_dir_all(root.join("docs/workflow-control-plane/plans")).expect("plans dir");
        fs::create_dir_all(root.join("docs/workflow-control-plane/schema")).expect("schema dir");
        fs::create_dir_all(root.join("docs/legal")).expect("legal docs dir");

        fs::write(
            root.join(".vac/registry/product.yaml"),
            r#"
schema_version: 1
kind: product
id: vac
title: VAC
command: vac
status: migration
control_plane: .vac
runtime: vac-rs
launcher: vac-cli/bin/vac.js
"#,
        )
        .expect("product manifest");
        fs::write(
            root.join(".vac/registry/status.yaml"),
            r#"
schema_version: 1
kind: status
id: vac.migration
status: in_progress
notes:
  - Root product is being migrated to workflow-control-plane pattern.
"#,
        )
        .expect("status manifest");

        let mut compatibility_block = String::new();
        if include_compatibility {
            compatibility_block.push_str(
                r#"
compatibility_transport:
  - dependency: vac-app-server
    status: compatibility_transport
    owner: vac-tui::app_server_session
    deletion_condition: remove when the local runtime session no longer needs the adapter
    replacement: vac-rs/tui/src/local_runtime_session.rs
    target_removal_plan: Plan 00F
  - dependency: vac-app-server-client
    status: compatibility_transport
    owner: vac-tui::app_server_session
    deletion_condition: remove when the local runtime session no longer needs the client bridge
    replacement: vac-rs/tui/src/local_runtime_session.rs
    target_removal_plan: Plan 00F
  - dependency: vac-app-server-protocol
    status: compatibility_transport
    owner: vac-tui::session_protocol
    deletion_condition: remove when session DTOs are fully owned by vac-protocol
    replacement: vac-rs/tui/src/session_protocol.rs
    target_removal_plan: Plan 00F
"#,
            );
        }

        let capability_template = |id: &str, title: &str, owner_crate: &str, owner_module: &str| {
            format!(
                r#"
schema_version: 1
kind: capability
id: {id}
title: {title}
status: ready
owner:
  crate: {owner_crate}
  module: {owner_module}
states:
  - empty
  - loading
  - success
  - failure
ownership:
  crates:
    - {owner_crate}
  modules:
    - {owner_module}
surfaces:
  palette: true
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
  approval_required_for:
    - execute_process
validation:
  commands:
    - cargo +1.93.0 check -p vac-surface-cli
{compatibility_block}
"#,
                id = id,
                title = title,
                owner_crate = owner_crate,
                owner_module = owner_module,
                compatibility_block = compatibility_block
            )
        };

        for (id, title, owner_crate, owner_module) in [
            (
                "vac.approvals",
                "Approvals",
                "vac-tui",
                "bottom_pane.approval_overlay",
            ),
            (
                "vac.architecture",
                "Architecture",
                "vac-core",
                "control_plane",
            ),
            ("vac.build", "Build", "vac-cli", "main"),
            ("vac.chat", "Chat", "vac-tui", "chatwidget"),
            ("vac.identity", "Identity", "vac-core", "control_plane"),
            (
                "vac.identity.check",
                "Identity Check",
                "vac-core",
                "control_plane",
            ),
            ("vac.ownership", "Ownership", "vac-core", "control_plane"),
            ("vac.release", "Release", "vac-cli", "doctor_cli"),
            ("vac.sandbox", "Sandbox", "vac-core", "sandboxing"),
            ("vac.sessions", "Sessions", "vac-core", "session"),
            ("vac.tools", "Tools", "vac-core", "tools"),
            ("vac.tui", "TUI", "vac-tui", "lib"),
            ("vac.workflow", "Workflow", "vac-core", "control_plane"),
        ] {
            let file_name = id.trim_start_matches("vac.").replace('.', "-");
            fs::write(
                root.join(format!(".vac/capabilities/{file_name}.yaml")),
                capability_template(id, title, owner_crate, owner_module),
            )
            .expect("capability manifest");
        }

        fs::write(
            root.join(".vac/workflows/maintenance.release-gate.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.release-gate
title: Release gate
status: planned
inputs: {}
ui:
  surface: /workflow
  inspect_surface: /debug-config
  progress_panel: true
  activity_log: true
  approval_surface: true
  evidence_surface: true
policy:
  default_risk: safe_read
  approval_required_for:
    - execute_process
steps:
  - id: orient
    uses: capability.architecture.invariants
  - id: approve
    uses: capability.approval.request
  - id: validate
    uses: capability.tui.pty_gate
  - id: report
    uses: capability.activity.emit
validation:
  commands:
    - cargo +1.93.0 run -p vac-surface-cli -- doctor registry .
"#,
        )
        .expect("release gate workflow");

        fs::write(
            root.join(".vac/policies/default-local.yaml"),
            r#"
schema_version: 1
kind: policy
id: vac.default-local
title: Default local policy
default_decision: approval_required
rules: []
"#,
        )
        .expect("default local policy");

        fs::write(
            root.join(".vac/surfaces/slash.yaml"),
            r#"
schema_version: 1
kind: surface
id: surface.slash
title: Slash surface
capabilities:
  - vac.workflow
routes:
  - kind: slash
    command: /workflow
    capability: vac.workflow
    owner: "vac-tui::workflow_browser"
    visible: true
    status: ready
"#,
        )
        .expect("slash surface manifest");
        fs::write(
            root.join(".vac/surfaces/palette.yaml"),
            r#"
schema_version: 1
kind: surface
id: surface.palette
title: Palette surface
capabilities:
  - vac.workflow
routes:
  - kind: palette
    action: open_workflow
    capability: vac.workflow
    owner: "vac-tui::workflow_browser"
    visible: true
    status: ready
"#,
        )
        .expect("palette surface manifest");

        fs::write(
            root.join("vac-rs/tui/Cargo.toml"),
            r#"
[package]
name = "vac-tui"
version = "0.0.0"

[dependencies]
vac-app-server = { workspace = true }
vac-app-server-client = { workspace = true }
vac-app-server-protocol = { workspace = true }
"#,
        )
        .expect("tui cargo");

        fs::write(
            root.join("README.md"),
            "[Workflow control plane](docs/workflow-control-plane/INDEX.md)\n[Legal notices](docs/legal/NOTICES.md)\n",
        )
        .expect("readme");
        fs::write(
            root.join("AGENTS.md"),
            "[Workflow control plane](docs/workflow-control-plane/INDEX.md)\n[Legal notices](docs/legal/NOTICES.md)\n",
        )
        .expect("agents");
        fs::write(
            root.join("docs/workflow-control-plane/INDEX.md"),
            "[Plans](plans/INDEX.md)\n[Schema](schema/INDEX.md)\n[Implementation](IMPLEMENTATION_PLAN.md)\n[Legal notices](../legal/NOTICES.md)\n",
        )
        .expect("docs index");
        fs::write(
            root.join("docs/workflow-control-plane/IMPLEMENTATION_PLAN.md"),
            "# Plan\n",
        )
        .expect("implementation plan");
        fs::write(
            root.join("docs/workflow-control-plane/plans/INDEX.md"),
            "[Plan 22](22-enterprise-hygiene.md)\n",
        )
        .expect("plan index");
        fs::write(
            root.join("docs/workflow-control-plane/plans/22-enterprise-hygiene.md"),
            "# Plan 22\n",
        )
        .expect("plan 22");
        fs::write(
            root.join("docs/workflow-control-plane/schema/INDEX.md"),
            "[Capability](capability-manifest.schema.md)\n",
        )
        .expect("schema index");
        fs::write(
            root.join("docs/workflow-control-plane/schema/capability-manifest.schema.md"),
            "# schema\n",
        )
        .expect("schema doc");
        fs::write(root.join("docs/legal/NOTICES.md"), "# Legal notices\n")
            .expect("legal notices doc");
    }

    fn seed_docs_maintenance_tree(
        root: &std::path::Path,
        broken_link: bool,
        empty_workflows: bool,
    ) {
        fs::write(
            root.join("README.md"),
            "[Workflow control plane](docs/workflow-control-plane/INDEX.md)\n[Legal notices](docs/legal/NOTICES.md)\n",
        )
        .expect("readme");
        fs::write(
            root.join("AGENTS.md"),
            "[Workflow control plane](docs/workflow-control-plane/INDEX.md)\n[Legal notices](docs/legal/NOTICES.md)\n",
        )
        .expect("agents");
        fs::create_dir_all(root.join("docs/workflow-control-plane/plans")).expect("plans dir");
        fs::create_dir_all(root.join("docs/workflow-control-plane/schema")).expect("schema dir");
        fs::create_dir_all(root.join("docs/legal")).expect("legal docs dir");
        fs::write(
            root.join("docs/workflow-control-plane/INDEX.md"),
            if broken_link {
                "[Broken](plans/MISSING.md)\n"
            } else {
                "[Plans](plans/INDEX.md)\n[Schema](schema/INDEX.md)\n[Implementation](IMPLEMENTATION_PLAN.md)\n[Legal notices](../legal/NOTICES.md)\n"
            },
        )
        .expect("control plane index");
        fs::write(
            root.join("docs/workflow-control-plane/IMPLEMENTATION_PLAN.md"),
            "# Plan\n",
        )
        .expect("implementation plan");
        fs::write(
            root.join("docs/workflow-control-plane/plans/INDEX.md"),
            "[Plan 22](22-enterprise-hygiene.md)\n",
        )
        .expect("plan index");
        fs::write(
            root.join("docs/workflow-control-plane/plans/22-enterprise-hygiene.md"),
            "# Plan 22\n",
        )
        .expect("plan 22");
        fs::write(
            root.join("docs/workflow-control-plane/schema/INDEX.md"),
            "[Capability](capability-manifest.schema.md)\n",
        )
        .expect("schema index");
        fs::write(
            root.join("docs/workflow-control-plane/schema/capability-manifest.schema.md"),
            "# schema\n",
        )
        .expect("schema doc");
        fs::write(root.join("docs/legal/NOTICES.md"), "# Legal notices\n")
            .expect("legal notices doc");

        fs::create_dir_all(root.join(".vac/capabilities")).expect("capabilities dir");
        fs::create_dir_all(root.join(".vac/policies")).expect("policies dir");
        fs::create_dir_all(root.join(".vac/surfaces")).expect("surfaces dir");
        fs::create_dir_all(root.join(".vac/registry")).expect("registry dir");
        if !empty_workflows {
            fs::create_dir_all(root.join(".vac/workflows")).expect("workflows dir");
            fs::write(
                root.join(".vac/workflows/maintenance.build-check.yaml"),
                "schema_version: 1\nkind: workflow\nid: maintenance.build-check\n",
            )
            .expect("workflow manifest");
        } else {
            fs::create_dir_all(root.join(".vac/workflows")).expect("workflows dir");
        }
        fs::write(
            root.join(".vac/capabilities/chat.yaml"),
            "schema_version: 1\nkind: capability\nid: vac.chat\n",
        )
        .expect("capability manifest");
        fs::write(
            root.join(".vac/policies/default-local.yaml"),
            "schema_version: 1\nkind: policy\nid: vac.default-local\n",
        )
        .expect("policy manifest");
        fs::write(
            root.join(".vac/surfaces/capabilities.yaml"),
            "schema_version: 1\nkind: surface\nid: surface.capabilities\n",
        )
        .expect("surface manifest");
        fs::write(
            root.join(".vac/registry/product.yaml"),
            "schema_version: 1\nkind: registry\nid: vac.product\n",
        )
        .expect("registry manifest");
    }

    fn seed_tui_uniqueness_capabilities(root: &std::path::Path) {
        fs::create_dir_all(root.join(".vac/capabilities")).expect("create temp capability dir");
        fs::write(
            root.join(".vac/capabilities/tui.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.tui
title: TUI
status: partial
reason: Test fixture models the in-progress TUI capability.
owner:
  crate: vac-surface-tui
  module: lib
states:
  - empty
  - loading
  - success
  - failure
description: Root interactive frontend and operator cockpit for the product.
depends_on:
  - vac.workflow
  - vac.approvals
surfaces:
  tui:
    routes:
      - /
      - /capabilities
      - /workflow
      - /debug-config
      - /activity
      - /approvals
      - /evidence
  slash:
    commands:
      - /capabilities
      - /workflow
      - /debug-config
      - /activity
      - /approvals
      - /evidence
  palette: true
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands:
    - CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
"#,
        )
        .expect("tui capability manifest");
        fs::write(
            root.join(".vac/capabilities/workflow.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: partial
reason: Test fixture models the in-progress workflow capability.
owner:
  crate: vac-core
  module: control_plane
states:
  - empty
  - loading
  - success
  - failure
description: Typed workflow manifests, registry loading, and diagnostics.
depends_on:
  - vac.sessions
surfaces:
  palette: true
  cli:
    enabled: true
    commands:
      - vac doctor workflow
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
  approval_required_for:
    - write_files
    - execute_process
validation:
  commands:
    - CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
"#,
        )
        .expect("workflow capability manifest");
    }

    #[test]
    fn workflow_doctor_renders_runner_preview() {
        let tempdir = make_temp_dir();
        seed_core_capabilities(&tempdir);
        fs::write(
            tempdir.join(".vac/workflows/maintenance.build-check.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.build-check
title: Build check
status: ready
inputs: {}
steps:
  - id: orient
    uses: capability.workflow
ui:
  surface: /workflow
  inspect_surface: /debug-config
  progress_panel: true
  activity_log: true
  approval_surface: true
  evidence_surface: true
policy:
  default_risk: safe_read
  approval_required_for:
    - execute_process
validation:
  commands:
    - cargo +1.93.0 check -p vac-surface-cli
"#,
        )
        .expect("workflow manifest");

        let report = super::load_workflow_run_report(&tempdir);
        let rendered = report.render_text();
        assert!(rendered.contains("workflow runner: supported=0 blocked=1"));
        assert!(rendered.contains("maintenance.build-check"));
        assert!(rendered.contains("step resolution:"));
        assert!(rendered.contains("runner: blocked:"));
        assert!(rendered.contains("dry-run: supported=0 blocked=1"));
        assert!(rendered.contains("dry-run: step 1. orient -> blocked:"));
        let _ = fs::remove_dir_all(&tempdir);
    }

    #[test]
    fn workflow_doctor_renders_approval_aware_runner_preview() {
        let tempdir = make_temp_dir();
        seed_core_capabilities(&tempdir);
        fs::write(
            tempdir.join(".vac/workflows/maintenance.build-check.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.build-check
title: Build check
status: ready
inputs: {}
steps:
  - id: orient
    uses: capability.identity.check
  - id: validate
    uses: capability.build.cargo_check
  - id: report
    uses: capability.activity.emit
ui:
  surface: /workflow
  inspect_surface: /debug-config
  progress_panel: true
  activity_log: true
  approval_surface: true
  evidence_surface: true
policy:
  default_risk: safe_read
  approval_required_for:
    - execute_process
validation:
  commands:
    - cargo +1.93.0 check -p vac-surface-cli
"#,
        )
        .expect("workflow manifest");

        let report = super::load_workflow_run_report(&tempdir);
        let rendered = report.render_text();
        assert!(rendered.contains("workflow runner: supported=1 blocked=0"));
        assert!(rendered.contains("approval policy: capability registry=loaded 3"));
        assert!(rendered.contains("waiting_approval"));
        assert!(rendered.contains("execution events:"));
        assert!(rendered.contains("step 1. orient -> waiting approval"));
        let _ = fs::remove_dir_all(&tempdir);
    }

    #[test]
    fn ownership_doctor_renders_ownership_matrix() {
        let tempdir = make_temp_dir();
        fs::create_dir_all(tempdir.join(".vac/capabilities")).expect("capabilities dir");
        fs::write(
            tempdir.join(".vac/capabilities/ownership.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.ownership
title: Ownership
status: partial
reason: Test fixture models an in-progress ownership capability.
owner:
  crate: vac-core
  module: control_plane
states:
  - empty
  - loading
  - success
  - failure
ownership:
  crates:
    - vac-core
  modules:
    - control_plane
surfaces:
  palette: true
policy:
  mutates_files: false
validation:
  commands:
    - cargo +1.93.0 check -p vac-surface-cli
"#,
        )
        .expect("ownership capability manifest");
        fs::write(
            tempdir.join(".vac/capabilities/build.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.build
title: Build
status: ready
owner:
  crate: vac-surface-cli
  module: main
states:
  - empty
  - loading
  - success
  - failure
surfaces:
  palette: true
policy:
  mutates_files: false
validation:
  commands:
    - cargo +1.93.0 check -p vac-surface-cli
"#,
        )
        .expect("build capability manifest");

        let report = super::load_ownership_scan_report(&tempdir);
        let rendered = report.render_text();
        assert!(
            rendered.contains("ownership scan: total=2 owned=1 unowned=1 ready_unowned=1"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("ownership targets: total=1 test_only=0 retired=0"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("ownership targets matrix:"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("classification: owned"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("ownership matrix:"),
            "rendered:\n{rendered}"
        );
        assert!(rendered.contains("vac.ownership"), "rendered:\n{rendered}");
        assert!(
            rendered.contains("ownership: crates=[vac-core] modules=[control_plane]"),
            "rendered:\n{rendered}"
        );
        assert!(rendered.contains("vac.build"), "rendered:\n{rendered}");
        assert!(
            rendered.contains("warning: capability missing ownership metadata"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("warning: ready capability missing ownership metadata"),
            "rendered:\n{rendered}"
        );
        let _ = fs::remove_dir_all(&tempdir);
    }

    #[test]
    fn workflow_doctor_reports_no_duplicate_tui_failure() {
        let tempdir = make_temp_dir();
        seed_tui_uniqueness_capabilities(&tempdir);
        fs::create_dir_all(tempdir.join("vac-rs/src")).expect("product source dir");
        fs::write(
            tempdir.join("vac-rs/src/lib.rs"),
            format!(
                "pub const LEGACY: &str = \"{}\";",
                concat!("vac", "_tui", "_runtime")
            ),
        )
        .expect("product source fixture");
        fs::write(
            tempdir.join(".vac/workflows/maintenance.no-duplicate-tui.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.no-duplicate-tui
title: TUI uniqueness check
status: ready
inputs: {}
steps:
  - id: orient
    uses: capability.tui.no_duplicate_runtime
  - id: report
    uses: capability.activity.emit
ui:
  surface: /workflow
  inspect_surface: /debug-config
  progress_panel: true
  activity_log: true
  approval_surface: false
  evidence_surface: true
policy:
  default_risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands:
    - cargo +1.93.0 check -p vac-surface-cli
"#,
        )
        .expect("workflow manifest");

        let report = super::load_workflow_run_report(&tempdir);
        let rendered = report.render_text();
        assert!(
            rendered.contains("workflow runner: supported=1 blocked=0"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("approval policy: capability registry=loaded 2"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("tui uniqueness: scanned="),
            "rendered:\n{rendered}"
        );
        assert!(rendered.contains("findings=1"), "rendered:\n{rendered}");
        assert!(
            rendered.contains("vac-rs/src/lib.rs:1"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("execution: failed"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("TUI Uniqueness Invariant Violation"),
            "rendered:\n{rendered}"
        );
        let _ = fs::remove_dir_all(&tempdir);
    }

    #[test]
    fn workflow_doctor_reports_identity_check_failure() {
        let tempdir = make_temp_dir();
        seed_core_capabilities(&tempdir);
        fs::create_dir_all(tempdir.join("vac-rs/src")).expect("product source dir");
        fs::write(
            tempdir.join("vac-rs/src/lib.rs"),
            format!(
                "pub const LEGACY: &str = \"{}\";",
                concat!("vac", "_tui", "_runtime")
            ),
        )
        .expect("product source fixture");
        fs::write(
            tempdir.join(".vac/workflows/maintenance.identity-check.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.identity-check
title: Identity check
status: ready
inputs: {}
steps:
  - id: orient
    uses: capability.identity.check
  - id: report
    uses: capability.activity.emit
ui:
  surface: /workflow
  inspect_surface: /debug-config
  progress_panel: true
  activity_log: true
  approval_surface: false
  evidence_surface: true
policy:
  default_risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands:
    - cargo +1.93.0 check -p vac-surface-cli
"#,
        )
        .expect("workflow manifest");

        let report = super::load_workflow_run_report(&tempdir);
        let rendered = report.render_text();
        assert!(
            rendered.contains("registry: loaded"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("identity check: scanned="),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("identity check exemptions:"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("docs/product/CAPABILITY_MAP.md"),
            "rendered:\n{rendered}"
        );
        assert!(rendered.contains("findings=1"), "rendered:\n{rendered}");
        assert!(
            rendered.contains("vac-rs/src/lib.rs:1"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("execution: failed"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("reason=identity check found 1 forbidden term"),
            "rendered:\n{rendered}"
        );
        let _ = fs::remove_dir_all(&tempdir);
    }

    #[test]
    fn doctor_subcommands_are_declared_in_cli_surface_manifest() {
        let manifest = load_cli_surface_manifest();
        for command in super::DoctorCommand::command().get_subcommands() {
            assert_cli_surface_declares(&manifest, &format!("vac doctor {}", command.get_name()));
        }
    }

    #[test]
    fn surfaces_doctor_helper_compiles_for_root_paths() {
        let _ = SurfacesDoctorCommand {
            path: PathBuf::from("."),
            visibility_contract: false,
        };
    }

    #[test]
    fn workflow_doctor_helper_compiles_for_root_paths() {
        let _ = WorkflowDoctorCommand {
            path: PathBuf::from("."),
            visibility_contract: false,
        };
    }

    #[test]
    fn architecture_doctor_renders_hardening_summary() {
        let tempdir = make_temp_dir();
        seed_architecture_environment(&tempdir, true);
        let report = super::load_architecture_invariant_report(&tempdir);
        let rendered = report.render_text();
        assert!(
            rendered.contains("architecture invariants:"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("source-of-truth: pass"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("legacy-transport: warn"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("rust-executes: pass"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("approval-protects: pass"),
            "rendered:\n{rendered}"
        );
        let _ = fs::remove_dir_all(&tempdir);
    }

    #[test]
    fn architecture_doctor_helper_compiles_for_root_paths() {
        let _ = ArchitectureDoctorCommand {
            path: PathBuf::from("."),
            visibility_contract: false,
        };
    }

    #[test]
    fn docs_doctor_renders_maintenance_ok() {
        let tempdir = make_temp_dir();
        seed_docs_maintenance_tree(&tempdir, false, false);
        let report = super::load_docs_maintenance_report(&tempdir);
        let rendered = report.render_text();
        assert!(
            rendered.contains("docs maintenance: ok"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("docs maintenance summary: docs_checked=7"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("manifest_dirs_present=5/5"),
            "rendered:\n{rendered}"
        );
        assert!(rendered.contains("repo root:"), "rendered:\n{rendered}");
        let _ = fs::remove_dir_all(&tempdir);
    }

    #[test]
    fn docs_doctor_reports_stale_link_and_empty_manifest_dir() {
        let tempdir = make_temp_dir();
        seed_docs_maintenance_tree(&tempdir, true, true);
        let report = super::load_docs_maintenance_report(&tempdir);
        let rendered = report.render_text();
        assert!(report.is_failure(), "rendered:\n{rendered}");
        assert!(
            rendered.contains("stale docs link `plans/MISSING.md`"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("manifest directory exists but contains no YAML manifests"),
            "rendered:\n{rendered}"
        );
        let _ = fs::remove_dir_all(&tempdir);
    }

    #[test]
    fn docs_doctor_helper_compiles_for_root_paths() {
        let _ = DocsDoctorCommand {
            path: PathBuf::from("."),
            visibility_contract: false,
        };
    }

    #[test]
    fn doctor_release_reports_failure_when_release_gate_fails() {
        let tempdir = make_temp_dir();

        // Create necessary directories
        fs::create_dir_all(tempdir.join(".vac/capabilities")).unwrap();
        fs::create_dir_all(tempdir.join(".vac/workflows")).unwrap();
        fs::create_dir_all(tempdir.join(".vac/policies")).unwrap();
        fs::create_dir_all(tempdir.join(".vac/surfaces")).unwrap();

        // Seed baseline policies to satisfy ROOT_SEED_POLICY_IDS
        let policies = &[
            ("vac.policy.approval", "approval.yaml"),
            ("vac.default-local", "default-local.yaml"),
            ("vac.policy.filesystem", "filesystem.yaml"),
            ("vac.policy.network", "network.yaml"),
            ("vac.policy.sandbox", "sandbox.yaml"),
            ("vac.policy.tools", "tools.yaml"),
        ];
        for (policy_id, filename) in policies {
            fs::write(
                tempdir.join(format!(".vac/policies/{}", filename)),
                format!(
                    r#"
schema_version: 1
kind: policy
id: {}
title: {} Policy
default_decision: approval_required
rules: []
"#,
                    policy_id,
                    policy_id
                        .replace("vac.policy.", "")
                        .replace("vac.", "")
                        .to_uppercase()
                ),
            )
            .unwrap();
        }

        // Seed the canonical capability used by capability.dashboard.check policy resolution.
        fs::write(
            tempdir.join(".vac/capabilities/ownership.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.ownership
title: Ownership
status: partial
reason: Test fixture models the in-progress ownership capability.
owner:
  crate: vac-core
  module: control_plane
states:
  - empty
  - loading
  - success
  - failure
ownership:
  crates:
    - vac-core
  modules:
    - control_plane
surfaces:
  palette: true
policy:
  mutates_files: false
validation:
  commands:
    - cargo +1.93.0 check -p vac-surface-cli
"#,
        )
        .unwrap();

        // Seed an unowned ready capability (has owner but no ownership metadata)
        fs::write(
            tempdir.join(".vac/capabilities/unowned.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.dashboard.check
title: Unowned Capability
status: ready
owner:
  crate: vac-surface-cli
  module: main
states:
  - empty
  - loading
  - success
  - failure
surfaces:
  tui: /workflow
  slash: /workflow
  palette: true
  cli:
    enabled: true
    commands:
      - vac check
policy:
  mutates_files: false
validation:
  commands:
    - cargo +1.93.0 check -p vac-surface-cli
"#,
        )
        .unwrap();

        // Seed a minimal release gate workflow utilizing capability.dashboard.check
        fs::write(
            tempdir.join(".vac/workflows/maintenance.release-gate.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.release-gate
title: Release gate
status: planned
inputs: {}
ui:
  surface: /workflow
  progress_panel: true
  activity_log: true
policy:
  default_risk: safe_read
steps:
  - id: capability_dashboard_check
    uses: capability.dashboard.check
validation:
  gates:
    - capability_dashboard_check
  commands: []
"#,
        )
        .unwrap();

        // Seed other required workflows for validation
        fs::write(
            tempdir.join(".vac/workflows/build-check.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.build-check
title: Build Check
status: planned
inputs: {}
ui:
  surface: /workflow
  progress_panel: true
  activity_log: true
policy:
  default_risk: safe_read
steps:
  - id: dummy
    uses: capability.dashboard.check
validation:
  gates: []
"#,
        )
        .unwrap();
        fs::write(
            tempdir.join(".vac/workflows/identity-check.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.identity-check
title: Identity Check
status: planned
inputs: {}
ui:
  surface: /workflow
  progress_panel: true
  activity_log: true
policy:
  default_risk: safe_read
steps:
  - id: dummy
    uses: capability.dashboard.check
validation:
  gates: []
"#,
        )
        .unwrap();
        for (workflow_id, filename) in [
            ("maintenance.ownership-scan", "ownership-scan.yaml"),
            ("maintenance.no-duplicate-tui", "no-duplicate-tui.yaml"),
            (
                "maintenance.donor-migration-gate",
                "donor-migration-gate.yaml",
            ),
        ] {
            fs::write(
                tempdir.join(format!(".vac/workflows/{filename}")),
                format!(
                    r#"
schema_version: 1
kind: workflow
id: {workflow_id}
title: Required root seed workflow
status: planned
inputs: {{}}
ui:
  surface: /workflow
  progress_panel: true
  activity_log: true
policy:
  default_risk: safe_read
steps:
  - id: dummy
    uses: capability.dashboard.check
validation:
  gates: []
"#
                ),
            )
            .unwrap();
        }

        // Seed required surfaces
        for surface_id in &[
            "surface.tui",
            "surface.slash",
            "surface.palette",
            "surface.cli",
        ] {
            let filename = format!("{}.yaml", surface_id.strip_prefix("surface.").unwrap());
            let routes_yaml = match *surface_id {
                "surface.tui" => {
                    r#"
  - kind: tui
    path: /workflow
    capability: vac.dashboard.check
    owner: vac-cli/main
    visible: true
    status: ready
"#
                }
                "surface.slash" => {
                    r#"
  - kind: slash
    command: /workflow
    capability: vac.dashboard.check
    owner: vac-cli/main
    visible: true
    status: ready
"#
                }
                "surface.palette" => {
                    r#"
  - kind: palette
    action: check
    capability: vac.dashboard.check
    owner: vac-cli/main
    visible: true
    status: ready
"#
                }
                "surface.cli" => {
                    r#"
  - kind: cli
    command: vac check
    capability: vac.dashboard.check
    owner: vac-cli/main
    visible: true
    status: ready
"#
                }
                _ => unreachable!(),
            };

            fs::write(
                tempdir.join(format!(".vac/surfaces/{}", filename)),
                format!(
                    r#"
schema_version: 1
kind: surface
id: {}
title: {} Surface
capabilities:
  - vac.dashboard.check
routes:{}
"#,
                    surface_id,
                    surface_id.strip_prefix("surface.").unwrap().to_uppercase(),
                    routes_yaml
                ),
            )
            .unwrap();
        }

        // Load the workflow run report. The release-gate entry itself should carry
        // the failure under test; registry root-seed completeness is covered by
        // separate doctor tests and is intentionally not part of this fixture.
        let ownership_report = super::load_ownership_scan_report(&tempdir);
        assert!(
            ownership_report.is_failure(),
            "ownership scan should fail due to unowned capability: {}",
            ownership_report.render_text()
        );
        assert_eq!(ownership_report.ready_unowned_count(), 1);
        assert!(
            ownership_report
                .render_text()
                .contains("capability missing ownership metadata"),
            "rendered:
{}",
            ownership_report.render_text()
        );

        let report = super::load_workflow_run_report(&tempdir);

        let release_gate = report
            .workflows()
            .iter()
            .find(|entry| entry.id == "maintenance.release-gate");

        assert!(release_gate.is_some(), "release gate workflow not found");
        let entry = release_gate.unwrap();
        assert!(
            entry.ownership_scan.is_some(),
            "release gate should attach ownership scan evidence"
        );

        let _ = fs::remove_dir_all(&tempdir);
    }

    #[test]
    fn release_doctor_helper_compiles_for_root_paths() {
        let _ = super::ReleaseDoctorCommand {
            path: PathBuf::from("."),
        };
    }
}
