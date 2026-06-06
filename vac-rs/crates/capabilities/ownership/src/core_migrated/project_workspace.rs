//! Zero-config project workspace classification.
//!
//! This module is intentionally small and side-effect free for classification.
//! It does not create `.vac`, does not persist memory, and does not weaken
//! strict product-repo gates. Higher-level CLI/TUI flows can use the
//! classification/report to decide whether an arbitrary user project should
//! continue with conservative in-memory defaults while strict VAC product
//! repositories keep manifest gates.

use std::fs;
use std::path::{Path, PathBuf};

const LOCAL_ONLY_DIRS: &[&str] = &[
    "db/",
    "sessions/",
    "index/",
    "artifacts/",
    "logs/",
    "cache/",
    "tmp/",
];

const COMMIT_FRIENDLY_PATHS: &[&str] = &["profile.yaml", "memory/", "manifests/"];

/// First-run workspace posture for a project root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectWorkspaceMode {
    /// No durable `.vac` state is required; use conservative session-only defaults.
    InMemory,
    /// A lightweight `.vac` profile could be proposed after explicit approval.
    Soft,
    /// Reviewed project profile/memory exists but strict manifests are not required.
    Curated,
    /// Strict product/control-plane manifests are required for gate success.
    Strict,
}

impl ProjectWorkspaceMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ProjectWorkspaceMode::InMemory => "in_memory",
            ProjectWorkspaceMode::Soft => "soft",
            ProjectWorkspaceMode::Curated => "curated",
            ProjectWorkspaceMode::Strict => "strict",
        }
    }
}

/// Strict-gate posture for the detected workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrictGateStatus {
    /// Strict manifests are available or not required for the current mode.
    Satisfied,
    /// Missing `.vac` is a setup warning for an arbitrary user project.
    SetupWarning,
    /// Missing `.vac` is fatal for strict product-repo validation gates.
    FatalMissingVac,
}

impl StrictGateStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            StrictGateStatus::Satisfied => "satisfied",
            StrictGateStatus::SetupWarning => "setup_warning",
            StrictGateStatus::FatalMissingVac => "fatal_missing_vac",
        }
    }
}

/// Side-effect-free classification for a project root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWorkspaceClassification {
    pub mode: ProjectWorkspaceMode,
    pub strict_gate_status: StrictGateStatus,
    pub vac_dir_exists: bool,
    pub ordinary_prompt_allowed: bool,
    pub durable_state_write_requires_approval: bool,
    pub reason: &'static str,
}

impl ProjectWorkspaceClassification {
    /// Returns true when strict product/control-plane checks may proceed.
    pub fn strict_gates_allowed(&self) -> bool {
        self.strict_gate_status == StrictGateStatus::Satisfied
    }

    /// Returns true when the user may still ask ordinary chat/coding questions.
    pub fn can_submit_ordinary_prompt(&self) -> bool {
        self.ordinary_prompt_allowed
    }
}

/// Classify a root without touching the filesystem beyond checking whether
/// `.vac` exists.
///
/// `strict_product_repo` should be true only for the VAC product repository or
/// an explicitly promoted strict workspace. Arbitrary user repositories should
/// pass false so missing `.vac` becomes an in-memory setup warning, not a fatal
/// chat blocker.
pub fn classify_project_workspace(
    root: impl AsRef<Path>,
    strict_product_repo: bool,
) -> ProjectWorkspaceClassification {
    let vac_dir_exists = root.as_ref().join(".vac").is_dir();

    if vac_dir_exists {
        return ProjectWorkspaceClassification {
            mode: ProjectWorkspaceMode::Strict,
            strict_gate_status: StrictGateStatus::Satisfied,
            vac_dir_exists,
            ordinary_prompt_allowed: true,
            durable_state_write_requires_approval: true,
            reason: "`.vac` exists; strict manifests may be evaluated by product gates",
        };
    }

    if strict_product_repo {
        return ProjectWorkspaceClassification {
            mode: ProjectWorkspaceMode::Strict,
            strict_gate_status: StrictGateStatus::FatalMissingVac,
            vac_dir_exists,
            ordinary_prompt_allowed: false,
            durable_state_write_requires_approval: true,
            reason: "strict product repository is missing `.vac`; product gates must fail closed",
        };
    }

    ProjectWorkspaceClassification {
        mode: ProjectWorkspaceMode::InMemory,
        strict_gate_status: StrictGateStatus::SetupWarning,
        vac_dir_exists,
        ordinary_prompt_allowed: true,
        durable_state_write_requires_approval: true,
        reason: "arbitrary user project is missing `.vac`; continue with conservative in-memory defaults",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectWorkspaceDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

impl ProjectWorkspaceDiagnosticSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            ProjectWorkspaceDiagnosticSeverity::Info => "info",
            ProjectWorkspaceDiagnosticSeverity::Warning => "warning",
            ProjectWorkspaceDiagnosticSeverity::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWorkspaceDiagnostic {
    pub severity: ProjectWorkspaceDiagnosticSeverity,
    pub code: &'static str,
    pub detail: String,
}

/// User-facing choice shown by the zero-config workspace confirmation dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectWorkspaceUserChoice {
    /// Continue without writing `.vac` state.
    ContinueInMemory,
    /// Create the reviewed soft `.vac` profile and local-only boundaries.
    ApproveSoftBootstrap,
    /// Open the strict promotion preview instead of silently promoting.
    ReviewStrictPromotion,
    /// Cancel the prompt without writing files or promoting.
    Cancel,
}

impl ProjectWorkspaceUserChoice {
    pub fn as_str(self) -> &'static str {
        match self {
            ProjectWorkspaceUserChoice::ContinueInMemory => "continue_in_memory",
            ProjectWorkspaceUserChoice::ApproveSoftBootstrap => "approve_soft_bootstrap",
            ProjectWorkspaceUserChoice::ReviewStrictPromotion => "review_strict_promotion",
            ProjectWorkspaceUserChoice::Cancel => "cancel",
        }
    }
}

/// Runtime action selected from a first-run confirmation prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectWorkspacePromptAction {
    ContinueInMemory,
    BootstrapSoft,
    ReviewStrictPromotion,
    Cancel,
    RefuseStrictPromotion,
}

impl ProjectWorkspacePromptAction {
    pub fn as_str(self) -> &'static str {
        match self {
            ProjectWorkspacePromptAction::ContinueInMemory => "continue_in_memory",
            ProjectWorkspacePromptAction::BootstrapSoft => "bootstrap_soft",
            ProjectWorkspacePromptAction::ReviewStrictPromotion => "review_strict_promotion",
            ProjectWorkspacePromptAction::Cancel => "cancel",
            ProjectWorkspacePromptAction::RefuseStrictPromotion => "refuse_strict_promotion",
        }
    }
}

/// Rich, side-effect-free first-run confirmation data for CLI/TUI surfaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWorkspaceConfirmationDialog {
    pub title: String,
    pub root: PathBuf,
    pub mode: ProjectWorkspaceMode,
    pub strict_gate_status: StrictGateStatus,
    pub ordinary_prompt_allowed: bool,
    pub bootstrap_offer_available: bool,
    pub strict_promotion_available: bool,
    pub inferred_stack: Vec<String>,
    pub summary_lines: Vec<String>,
    pub choices: Vec<ProjectWorkspaceUserChoice>,
    pub default_choice: ProjectWorkspaceUserChoice,
}

impl ProjectWorkspaceConfirmationDialog {
    pub fn resolve_choice(
        &self,
        choice: ProjectWorkspaceUserChoice,
    ) -> ProjectWorkspacePromptAction {
        match choice {
            ProjectWorkspaceUserChoice::ContinueInMemory => {
                ProjectWorkspacePromptAction::ContinueInMemory
            }
            ProjectWorkspaceUserChoice::ApproveSoftBootstrap if self.bootstrap_offer_available => {
                ProjectWorkspacePromptAction::BootstrapSoft
            }
            ProjectWorkspaceUserChoice::ReviewStrictPromotion
                if self.strict_promotion_available =>
            {
                ProjectWorkspacePromptAction::ReviewStrictPromotion
            }
            ProjectWorkspaceUserChoice::ApproveSoftBootstrap
            | ProjectWorkspaceUserChoice::ReviewStrictPromotion => {
                ProjectWorkspacePromptAction::RefuseStrictPromotion
            }
            ProjectWorkspaceUserChoice::Cancel => ProjectWorkspacePromptAction::Cancel,
        }
    }

    pub fn render_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&self.title);
        out.push('\n');
        out.push_str(&"=".repeat(self.title.len()));
        out.push('\n');
        out.push_str(&format!("root: {}\n", self.root.display()));
        out.push_str(&format!("current_mode: {}\n", self.mode.as_str()));
        out.push_str(&format!(
            "strict_gate_status: {}\n",
            self.strict_gate_status.as_str()
        ));
        out.push_str(&format!(
            "ordinary_prompt_allowed: {}\n",
            self.ordinary_prompt_allowed
        ));
        out.push_str("summary:\n");
        for line in &self.summary_lines {
            out.push_str(&format!("  - {line}\n"));
        }
        out.push_str("choices:\n");
        for choice in &self.choices {
            let default = if *choice == self.default_choice {
                " (default)"
            } else {
                ""
            };
            out.push_str(&format!("  - {}{}\n", choice.as_str(), default));
        }
        out
    }
}

/// Side-effect-free strict promotion preview. It tells the operator what would
/// be required; it never creates strict manifests implicitly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWorkspaceStrictPromotionPreview {
    pub root: PathBuf,
    pub required_manifest_dirs: Vec<&'static str>,
    pub requires_review: bool,
    pub automatic_yaml_migration: bool,
}

impl ProjectWorkspaceStrictPromotionPreview {
    pub fn render_text(&self) -> String {
        let mut out = String::new();
        out.push_str("VAC Strict Workspace Promotion Preview\n");
        out.push_str("======================================\n");
        out.push_str(&format!("root: {}\n", self.root.display()));
        out.push_str("target_mode: strict\n");
        out.push_str(&format!("requires_review: {}\n", self.requires_review));
        out.push_str(&format!(
            "automatic_yaml_migration: {}\n",
            self.automatic_yaml_migration
        ));
        out.push_str("required_manifest_dirs:\n");
        for dir in &self.required_manifest_dirs {
            out.push_str(&format!("  - .vac/{dir}\n"));
        }
        out.push_str("operator_notes:\n");
        out.push_str(
            "  - strict promotion is an explicit reviewed migration, not a first-run side effect\n",
        );
        out.push_str("  - create/review manifests before claiming strict gates are green\n");
        out.push_str("  - rollback is safe by keeping soft local-only paths ignored\n");
        out
    }
}

#[derive(Debug, Clone)]
pub struct ProjectWorkspaceReport {
    pub root: PathBuf,
    pub vac_dir: PathBuf,
    pub mode: ProjectWorkspaceMode,
    pub strict_gate_status: StrictGateStatus,
    pub ordinary_prompt_allowed: bool,
    pub strict_manifests_available: bool,
    pub bootstrap_offer_available: bool,
    pub inferred_stack: Vec<String>,
    pub local_only_dirs: Vec<&'static str>,
    pub commit_friendly_paths: Vec<&'static str>,
    pub diagnostics: Vec<ProjectWorkspaceDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWorkspaceStartupNotice {
    pub root: PathBuf,
    pub mode: ProjectWorkspaceMode,
    pub strict_gate_status: StrictGateStatus,
    pub bootstrap_offer_available: bool,
    pub ordinary_prompt_allowed: bool,
    pub inferred_stack: Vec<String>,
}

impl ProjectWorkspaceStartupNotice {
    pub fn render_tui_warning(&self) -> String {
        let inferred_stack = if self.inferred_stack.is_empty() {
            "unknown".to_string()
        } else {
            self.inferred_stack.join(", ")
        };
        format!(
            "VAC did not find a .vac workspace at `{}`. Continuing in `{}` mode for ordinary prompts; write/exec actions remain approval-gated. Inferred stack: {inferred_stack}. To persist a lightweight workspace, review `vac doctor project-workspace {}` and approve soft bootstrap explicitly.",
            self.root.display(),
            self.mode.as_str(),
            self.root.display()
        )
    }

    pub fn render_cli_preflight(&self) -> String {
        let mut out = String::new();
        out.push_str("VAC did not find a .vac workspace.\n");
        out.push_str(&format!("root: {}\n", self.root.display()));
        out.push_str(&format!("mode: {}\n", self.mode.as_str()));
        out.push_str(&format!(
            "strict_gate_status: {}\n",
            self.strict_gate_status.as_str()
        ));
        out.push_str(&format!(
            "ordinary_prompt_allowed: {}\n",
            self.ordinary_prompt_allowed
        ));
        out.push_str(&format!(
            "bootstrap_offer_available: {}\n",
            self.bootstrap_offer_available
        ));
        out.push_str("next_steps:\n");
        out.push_str("  - continue now in conservative in-memory mode\n");
        out.push_str(
            "  - review the TUI setup dialog or run `vac doctor project-workspace <path>` for the bootstrap preview\n",
        );
        out.push_str("  - run `vac doctor project-workspace <path> --bootstrap-soft --yes` only after approving disk writes\n");
        out.push_str("  - run `vac doctor project-workspace <path> --promote-strict-preview` before any strict manifest migration\n");
        out
    }
}

impl ProjectWorkspaceReport {
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == ProjectWorkspaceDiagnosticSeverity::Warning)
            .count()
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == ProjectWorkspaceDiagnosticSeverity::Error)
            .count()
    }

    pub fn is_failure(&self) -> bool {
        self.error_count() > 0
    }

    pub fn render_text(&self) -> String {
        let mut out = String::new();
        out.push_str("VAC Project Workspace Diagnostics\n");
        out.push_str("=================================\n");
        out.push_str(&format!("root: {}\n", self.root.display()));
        out.push_str(&format!("vac_dir: {}\n", self.vac_dir.display()));
        out.push_str(&format!("mode: {}\n", self.mode.as_str()));
        out.push_str(&format!(
            "strict_gate_status: {}\n",
            self.strict_gate_status.as_str()
        ));
        out.push_str(&format!(
            "ordinary_prompt_allowed: {}\n",
            self.ordinary_prompt_allowed
        ));
        out.push_str(&format!(
            "strict_manifests_available: {}\n",
            self.strict_manifests_available
        ));
        out.push_str(&format!(
            "bootstrap_offer_available: {}\n",
            self.bootstrap_offer_available
        ));
        out.push_str(&format!(
            "summary: warnings={} errors={}\n",
            self.warning_count(),
            self.error_count()
        ));
        out.push_str("inferred_stack:\n");
        if self.inferred_stack.is_empty() {
            out.push_str("  - unknown\n");
        } else {
            for stack in &self.inferred_stack {
                out.push_str(&format!("  - {stack}\n"));
            }
        }
        out.push_str("local_only_dirs:\n");
        for dir in &self.local_only_dirs {
            out.push_str(&format!("  - .vac/{dir}\n"));
        }
        out.push_str("commit_friendly_paths:\n");
        for path in &self.commit_friendly_paths {
            out.push_str(&format!("  - .vac/{path}\n"));
        }
        out.push_str("diagnostics:\n");
        if self.diagnostics.is_empty() {
            out.push_str("  - level: info\n");
            out.push_str("    code: project_workspace_ready\n");
            out.push_str("    detail: Project workspace contract is explicit and non-fatal.\n");
        } else {
            for diagnostic in &self.diagnostics {
                out.push_str(&format!("  - level: {}\n", diagnostic.severity.as_str()));
                out.push_str(&format!("    code: {}\n", diagnostic.code));
                out.push_str(&format!("    detail: {}\n", diagnostic.detail));
            }
        }
        if self.bootstrap_offer_available {
            out.push_str("bootstrap_preview:\n");
            let plan = build_soft_workspace_bootstrap_plan(&self.root);
            for line in plan.render_text().lines() {
                out.push_str(&format!("  {line}\n"));
            }
        }
        out
    }
}

pub fn project_workspace_startup_notice(root: &Path) -> Option<ProjectWorkspaceStartupNotice> {
    let report = load_project_workspace_report(root);
    if !report.bootstrap_offer_available {
        return None;
    }
    Some(ProjectWorkspaceStartupNotice {
        root: report.root,
        mode: report.mode,
        strict_gate_status: report.strict_gate_status,
        bootstrap_offer_available: report.bootstrap_offer_available,
        ordinary_prompt_allowed: report.ordinary_prompt_allowed,
        inferred_stack: report.inferred_stack,
    })
}

/// Build the rich first-run confirmation dialog without touching disk.
pub fn build_project_workspace_confirmation_dialog(
    root: &Path,
) -> Option<ProjectWorkspaceConfirmationDialog> {
    let report = load_project_workspace_report(root);
    if !report.bootstrap_offer_available && report.mode != ProjectWorkspaceMode::Soft {
        return None;
    }
    let mut choices = vec![ProjectWorkspaceUserChoice::ContinueInMemory];
    if report.bootstrap_offer_available {
        choices.push(ProjectWorkspaceUserChoice::ApproveSoftBootstrap);
    }
    choices.push(ProjectWorkspaceUserChoice::ReviewStrictPromotion);
    choices.push(ProjectWorkspaceUserChoice::Cancel);

    let inferred_stack = if report.inferred_stack.is_empty() {
        vec!["unknown".to_string()]
    } else {
        report.inferred_stack.clone()
    };
    let summary_lines = vec![
        "Continue ordinary prompts immediately in conservative in-memory mode.".to_string(),
        "Persisting `.vac` requires explicit approval and writes only a soft profile plus local-only ignore boundaries.".to_string(),
        "Strict manifests are not generated automatically; promotion requires a reviewed manifest migration.".to_string(),
        format!("Inferred stack: {}", inferred_stack.join(", ")),
    ];

    Some(ProjectWorkspaceConfirmationDialog {
        title: "VAC Project Workspace Setup".to_string(),
        root: report.root,
        mode: report.mode,
        strict_gate_status: report.strict_gate_status,
        ordinary_prompt_allowed: report.ordinary_prompt_allowed,
        bootstrap_offer_available: report.bootstrap_offer_available,
        strict_promotion_available: true,
        inferred_stack,
        summary_lines,
        choices,
        default_choice: ProjectWorkspaceUserChoice::ContinueInMemory,
    })
}

pub fn build_strict_workspace_promotion_preview(
    root: &Path,
) -> ProjectWorkspaceStrictPromotionPreview {
    ProjectWorkspaceStrictPromotionPreview {
        root: root.to_path_buf(),
        required_manifest_dirs: vec!["capabilities/", "policies/", "workflows/", "surfaces/"],
        requires_review: true,
        automatic_yaml_migration: false,
    }
}

/// Disk changes proposed for an approved soft `.vac` bootstrap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWorkspaceBootstrapPlan {
    pub root: PathBuf,
    pub vac_dir: PathBuf,
    pub profile_yaml: String,
    pub gitignore: String,
    pub local_only_dirs: Vec<&'static str>,
    pub commit_friendly_paths: Vec<&'static str>,
    pub inferred_stack: Vec<String>,
}

impl ProjectWorkspaceBootstrapPlan {
    pub fn render_text(&self) -> String {
        let mut out = String::new();
        out.push_str("VAC Project Workspace Bootstrap Preview\n");
        out.push_str("=======================================\n");
        out.push_str("VAC did not find a .vac workspace.\n");
        out.push_str("You can continue in-memory now, or approve creation of a lightweight .vac workspace.\n");
        out.push_str(&format!("root: {}\n", self.root.display()));
        out.push_str(&format!("vac_dir: {}\n", self.vac_dir.display()));
        out.push_str("proposed_mode: soft\n");
        out.push_str("ordinary_prompt_allowed_without_write: true\n");
        out.push_str("durable_write_requires_approval: true\n");
        out.push_str("inferred_stack:\n");
        if self.inferred_stack.is_empty() {
            out.push_str("  - unknown\n");
        } else {
            for stack in &self.inferred_stack {
                out.push_str(&format!("  - {stack}\n"));
            }
        }
        out.push_str("would_create_files:\n");
        out.push_str("  - .vac/profile.yaml\n");
        out.push_str("  - .vac/.gitignore\n");
        out.push_str("would_create_local_only_dirs:\n");
        for dir in &self.local_only_dirs {
            out.push_str(&format!("  - .vac/{dir}\n"));
        }
        out.push_str("commit_friendly_paths:\n");
        for path in &self.commit_friendly_paths {
            out.push_str(&format!("  - .vac/{path}\n"));
        }
        out.push_str("disabled_until_promotion:\n");
        out.push_str("  - strict capability/policy/workflow/surface gates\n");
        out.push_str("  - reviewed memory persistence\n");
        out.push_str("  - automatic validation execution\n");
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectWorkspaceBootstrapError {
    ApprovalRequired,
    VacPathAlreadyExists(PathBuf),
    VacPathNotDirectory(PathBuf),
    StrictPromotionRequiresExistingVac(PathBuf),
    StrictPromotionRequiresProfile(PathBuf),
    Io { path: PathBuf, message: String },
}

impl ProjectWorkspaceBootstrapError {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectWorkspaceBootstrapError::ApprovalRequired => "approval_required",
            ProjectWorkspaceBootstrapError::VacPathAlreadyExists(_) => "vac_path_already_exists",
            ProjectWorkspaceBootstrapError::VacPathNotDirectory(_) => "vac_path_not_directory",
            ProjectWorkspaceBootstrapError::StrictPromotionRequiresExistingVac(_) => {
                "strict_promotion_requires_existing_vac"
            }
            ProjectWorkspaceBootstrapError::StrictPromotionRequiresProfile(_) => {
                "strict_promotion_requires_profile"
            }
            ProjectWorkspaceBootstrapError::Io { .. } => "io_error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWorkspaceBootstrapResult {
    pub mode: ProjectWorkspaceMode,
    pub created_paths: Vec<PathBuf>,
    pub local_only_dirs: Vec<PathBuf>,
}

/// Build a side-effect-free preview for an approved soft `.vac` bootstrap.
pub fn build_soft_workspace_bootstrap_plan(root: &Path) -> ProjectWorkspaceBootstrapPlan {
    let root = root.to_path_buf();
    let inferred_stack = infer_stack(&root);
    let stack_yaml = if inferred_stack.is_empty() {
        "  - unknown\n".to_string()
    } else {
        inferred_stack
            .iter()
            .map(|stack| format!("  - {stack}\n"))
            .collect::<String>()
    };

    ProjectWorkspaceBootstrapPlan {
        vac_dir: root.join(".vac"),
        root,
        profile_yaml: format!(
            "# Generated by VAC after explicit user approval.\n\
             adoption_mode: soft\n\
             source: inferred\n\
             inferred_stack:\n{stack_yaml}\
             strict_manifests_required: false\n\
             durable_writes_require_approval: true\n"
        ),
        gitignore: LOCAL_ONLY_DIRS.join("\n") + "\n",
        local_only_dirs: LOCAL_ONLY_DIRS.to_vec(),
        commit_friendly_paths: COMMIT_FRIENDLY_PATHS.to_vec(),
        inferred_stack,
    }
}

/// Create a soft `.vac` workspace only after the caller records explicit approval.
///
/// This helper intentionally refuses to write when `approved` is false so CLI/TUI
/// callers cannot accidentally persist project state from an inferred setup flow.
pub fn materialize_soft_workspace_bootstrap(
    root: &Path,
    approved: bool,
) -> Result<ProjectWorkspaceBootstrapResult, ProjectWorkspaceBootstrapError> {
    if !approved {
        return Err(ProjectWorkspaceBootstrapError::ApprovalRequired);
    }

    let plan = build_soft_workspace_bootstrap_plan(root);
    if plan.vac_dir.exists() && !plan.vac_dir.is_dir() {
        return Err(ProjectWorkspaceBootstrapError::VacPathNotDirectory(
            plan.vac_dir,
        ));
    }
    if plan.vac_dir.is_dir()
        && (plan.vac_dir.join("profile.yaml").exists() || strict_manifest_dirs_exist(&plan.vac_dir))
    {
        return Err(ProjectWorkspaceBootstrapError::VacPathAlreadyExists(
            plan.vac_dir,
        ));
    }

    create_dir_all(&plan.vac_dir)?;
    let mut created_paths = vec![plan.vac_dir.clone()];

    let profile_path = plan.vac_dir.join("profile.yaml");
    write_file(&profile_path, &plan.profile_yaml)?;
    created_paths.push(profile_path);

    let gitignore_path = plan.vac_dir.join(".gitignore");
    write_file(&gitignore_path, &plan.gitignore)?;
    created_paths.push(gitignore_path);

    let mut local_only_dirs = Vec::new();
    for dir in &plan.local_only_dirs {
        let dir_path = plan.vac_dir.join(dir);
        create_dir_all(&dir_path)?;
        local_only_dirs.push(dir_path.clone());
        created_paths.push(dir_path);
    }

    Ok(ProjectWorkspaceBootstrapResult {
        mode: ProjectWorkspaceMode::Soft,
        created_paths,
        local_only_dirs,
    })
}

pub fn materialize_strict_workspace_promotion(
    root: &Path,
    approved: bool,
) -> Result<ProjectWorkspaceBootstrapResult, ProjectWorkspaceBootstrapError> {
    if !approved {
        return Err(ProjectWorkspaceBootstrapError::ApprovalRequired);
    }

    let root = root.to_path_buf();
    let vac_dir = root.join(".vac");
    if !vac_dir.exists() {
        return Err(ProjectWorkspaceBootstrapError::StrictPromotionRequiresExistingVac(vac_dir));
    }
    if !vac_dir.is_dir() {
        return Err(ProjectWorkspaceBootstrapError::VacPathNotDirectory(vac_dir));
    }
    if !vac_dir.join("profile.yaml").is_file() {
        return Err(
            ProjectWorkspaceBootstrapError::StrictPromotionRequiresProfile(
                vac_dir.join("profile.yaml"),
            ),
        );
    }

    let inferred_stack = infer_stack(&root);
    let stack_yaml = if inferred_stack.is_empty() {
        "  - unknown
"
        .to_string()
    } else {
        inferred_stack
            .iter()
            .map(|stack| {
                format!(
                    "  - {stack}
"
                )
            })
            .collect::<String>()
    };
    let profile_yaml = format!(
        "# Promoted by VAC after explicit project-owner approval.
         adoption_mode: strict
         source: reviewed
         inferred_stack:
{stack_yaml}         strict_manifests_required: true
         durable_writes_require_approval: true
         promotion_review_required: true
"
    );

    let mut created_paths = Vec::new();
    let profile_path = vac_dir.join("profile.yaml");
    write_file(&profile_path, &profile_yaml)?;
    created_paths.push(profile_path);

    for dir in ["capabilities", "policies", "workflows", "surfaces"] {
        let dir_path = vac_dir.join(dir);
        create_dir_all(&dir_path)?;
        created_paths.push(dir_path.clone());
        let readme_path = dir_path.join("README.md");
        if !readme_path.exists() {
            write_file(
                &readme_path,
                "# VAC strict manifest directory

This directory was created by explicit strict-promotion approval. Add reviewed YAML manifests before claiming strict gate readiness.
",
            )?;
            created_paths.push(readme_path);
        }
    }

    Ok(ProjectWorkspaceBootstrapResult {
        mode: ProjectWorkspaceMode::Strict,
        created_paths,
        local_only_dirs: LOCAL_ONLY_DIRS
            .iter()
            .map(|dir| vac_dir.join(dir))
            .collect(),
    })
}

fn create_dir_all(path: &Path) -> Result<(), ProjectWorkspaceBootstrapError> {
    fs::create_dir_all(path).map_err(|err| ProjectWorkspaceBootstrapError::Io {
        path: path.to_path_buf(),
        message: err.to_string(),
    })
}

fn write_file(path: &Path, content: &str) -> Result<(), ProjectWorkspaceBootstrapError> {
    fs::write(path, content).map_err(|err| ProjectWorkspaceBootstrapError::Io {
        path: path.to_path_buf(),
        message: err.to_string(),
    })
}

pub fn load_project_workspace_report(root: &Path) -> ProjectWorkspaceReport {
    load_project_workspace_report_with_options(root, false)
}

/// Load project workspace diagnostics with explicit strict product-repo semantics.
///
/// Arbitrary user projects should pass `false`, making missing `.vac` a warning
/// while preserving ordinary prompt submission. Strict VAC product repositories
/// should pass `true`, making missing `.vac` an error for product gates.
pub fn load_project_workspace_report_with_options(
    root: &Path,
    strict_product_repo: bool,
) -> ProjectWorkspaceReport {
    let root = root.to_path_buf();
    let vac_dir = root.join(".vac");
    let inferred_stack = infer_stack(&root);
    let mut diagnostics = Vec::new();

    if !vac_dir.exists() {
        let classification = classify_project_workspace(&root, strict_product_repo);
        let (severity, code, detail, bootstrap_offer_available) = if classification
            .strict_gate_status
            == StrictGateStatus::FatalMissingVac
        {
            (
                ProjectWorkspaceDiagnosticSeverity::Error,
                "missing_vac_workspace_strict_product_repo",
                "strict product repository is missing `.vac`; product gates must fail closed without reporting zero-config green",
                false,
            )
        } else {
            (
                ProjectWorkspaceDiagnosticSeverity::Warning,
                "missing_vac_workspace",
                "`.vac` is missing; ordinary assistance should continue in in-memory mode with write/exec actions still approval-gated",
                true,
            )
        };
        diagnostics.push(ProjectWorkspaceDiagnostic {
            severity,
            code,
            detail: detail.to_string(),
        });
        return ProjectWorkspaceReport {
            root,
            vac_dir,
            mode: classification.mode,
            strict_gate_status: classification.strict_gate_status,
            ordinary_prompt_allowed: classification.ordinary_prompt_allowed,
            strict_manifests_available: false,
            bootstrap_offer_available,
            inferred_stack,
            local_only_dirs: LOCAL_ONLY_DIRS.to_vec(),
            commit_friendly_paths: COMMIT_FRIENDLY_PATHS.to_vec(),
            diagnostics,
        };
    }

    if !vac_dir.is_dir() {
        diagnostics.push(ProjectWorkspaceDiagnostic {
            severity: ProjectWorkspaceDiagnosticSeverity::Error,
            code: "vac_workspace_not_directory",
            detail: "`.vac` exists but is not a directory, so workspace setup cannot be classified safely".to_string(),
        });
        return ProjectWorkspaceReport {
            root,
            vac_dir,
            mode: ProjectWorkspaceMode::InMemory,
            strict_gate_status: StrictGateStatus::FatalMissingVac,
            ordinary_prompt_allowed: true,
            strict_manifests_available: false,
            bootstrap_offer_available: false,
            inferred_stack,
            local_only_dirs: LOCAL_ONLY_DIRS.to_vec(),
            commit_friendly_paths: COMMIT_FRIENDLY_PATHS.to_vec(),
            diagnostics,
        };
    }

    let strict_manifests_available = strict_manifest_dirs_exist(&vac_dir);
    let mode = profile_mode(&vac_dir).unwrap_or(if strict_manifests_available {
        ProjectWorkspaceMode::Strict
    } else {
        ProjectWorkspaceMode::Soft
    });

    diagnostics.push(ProjectWorkspaceDiagnostic {
        severity: ProjectWorkspaceDiagnosticSeverity::Info,
        code: "vac_workspace_present",
        detail: format!(
            "`.vac` workspace detected with `{}` adoption mode",
            mode.as_str()
        ),
    });

    if mode != ProjectWorkspaceMode::InMemory && !vac_dir.join("profile.yaml").is_file() {
        diagnostics.push(ProjectWorkspaceDiagnostic {
            severity: ProjectWorkspaceDiagnosticSeverity::Warning,
            code: "project_profile_missing",
            detail: "`.vac/profile.yaml` is absent; inferred workspace state remains unreviewed until a profile is approved".to_string(),
        });
    }

    for missing in missing_gitignore_entries(&vac_dir) {
        diagnostics.push(ProjectWorkspaceDiagnostic {
            severity: ProjectWorkspaceDiagnosticSeverity::Warning,
            code: "local_only_boundary_missing",
            detail: format!("`.vac/.gitignore` does not protect local-only path `.vac/{missing}`"),
        });
    }

    ProjectWorkspaceReport {
        root,
        vac_dir,
        mode,
        strict_gate_status: StrictGateStatus::Satisfied,
        ordinary_prompt_allowed: true,
        strict_manifests_available,
        bootstrap_offer_available: false,
        inferred_stack,
        local_only_dirs: LOCAL_ONLY_DIRS.to_vec(),
        commit_friendly_paths: COMMIT_FRIENDLY_PATHS.to_vec(),
        diagnostics,
    }
}

fn profile_mode(vac_dir: &Path) -> Option<ProjectWorkspaceMode> {
    let raw = fs::read_to_string(vac_dir.join("profile.yaml")).ok()?;
    for line in raw.lines() {
        let line = line.trim();
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key.trim() != "adoption_mode" && key.trim() != "mode" {
            continue;
        }
        let normalized = value.trim().trim_matches('"').trim_matches('\'');
        return match normalized {
            "in_memory" => Some(ProjectWorkspaceMode::InMemory),
            "soft" => Some(ProjectWorkspaceMode::Soft),
            "curated" => Some(ProjectWorkspaceMode::Curated),
            "strict" => Some(ProjectWorkspaceMode::Strict),
            _ => None,
        };
    }
    None
}

fn strict_manifest_dirs_exist(vac_dir: &Path) -> bool {
    ["capabilities", "policies", "workflows", "surfaces"]
        .iter()
        .all(|dir| vac_dir.join(dir).is_dir())
}

fn missing_gitignore_entries(vac_dir: &Path) -> Vec<&'static str> {
    let Ok(raw) = fs::read_to_string(vac_dir.join(".gitignore")) else {
        return LOCAL_ONLY_DIRS.to_vec();
    };
    LOCAL_ONLY_DIRS
        .iter()
        .copied()
        .filter(|entry| {
            let entry_without_slash = entry.trim_end_matches('/');
            !raw.lines().any(|line| {
                let line = line.trim();
                line == *entry || line == entry_without_slash
            })
        })
        .collect()
}

fn infer_stack(root: &Path) -> Vec<String> {
    let mut stack = Vec::new();
    if root.join("Cargo.toml").is_file() || root.join("vac-rs/Cargo.toml").is_file() {
        stack.push("rust/cargo".to_string());
    }
    if root.join("package.json").is_file() {
        stack.push("node/package-json".to_string());
    }
    if root.join("pyproject.toml").is_file() {
        stack.push("python/pyproject".to_string());
    }
    if root.join("go.mod").is_file() {
        stack.push("go/modules".to_string());
    }
    stack
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vac-project-workspace-{name}-{unique}"));
        fs::create_dir_all(&root).expect("create temp root");
        root
    }

    #[test]
    fn arbitrary_project_without_vac_uses_in_memory_warning() {
        let root = temp_root("missing-vac-user");
        let classification = classify_project_workspace(&root, false);

        assert_eq!(classification.mode, ProjectWorkspaceMode::InMemory);
        assert_eq!(
            classification.strict_gate_status,
            StrictGateStatus::SetupWarning
        );
        assert!(classification.can_submit_ordinary_prompt());
        assert!(!classification.strict_gates_allowed());
        assert!(classification.durable_state_write_requires_approval);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn strict_product_repo_without_vac_fails_closed() {
        let root = temp_root("missing-vac-product");
        let classification = classify_project_workspace(&root, true);

        assert_eq!(classification.mode, ProjectWorkspaceMode::Strict);
        assert_eq!(
            classification.strict_gate_status,
            StrictGateStatus::FatalMissingVac
        );
        assert!(!classification.can_submit_ordinary_prompt());
        assert!(!classification.strict_gates_allowed());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn existing_vac_allows_strict_gate_evaluation() {
        let root = temp_root("existing-vac");
        fs::create_dir(root.join(".vac")).expect("create .vac");

        let classification = classify_project_workspace(&root, true);

        assert_eq!(classification.mode, ProjectWorkspaceMode::Strict);
        assert_eq!(
            classification.strict_gate_status,
            StrictGateStatus::Satisfied
        );
        assert!(classification.can_submit_ordinary_prompt());
        assert!(classification.strict_gates_allowed());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn report_missing_vac_allows_in_memory_prompting() {
        let root = temp_root("missing-vac-report");
        fs::write(root.join("Cargo.toml"), "[workspace]\n").expect("write cargo");

        let report = load_project_workspace_report(&root);

        assert_eq!(report.mode, ProjectWorkspaceMode::InMemory);
        assert_eq!(report.strict_gate_status, StrictGateStatus::SetupWarning);
        assert!(report.ordinary_prompt_allowed);
        assert!(report.bootstrap_offer_available);
        assert!(!report.strict_manifests_available);
        assert_eq!(report.error_count(), 0);
        assert_eq!(report.warning_count(), 1);
        assert!(report.render_text().contains("code: missing_vac_workspace"));
        assert!(
            report
                .inferred_stack
                .iter()
                .any(|stack| stack == "rust/cargo")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn report_soft_workspace_surfaces_local_only_boundary_warnings() {
        let root = temp_root("soft-report");
        let vac_dir = root.join(".vac");
        fs::create_dir_all(&vac_dir).expect("create vac dir");
        fs::write(vac_dir.join("profile.yaml"), "adoption_mode: soft\n").expect("profile");
        fs::write(vac_dir.join(".gitignore"), "db/\ncache/\n").expect("gitignore");

        let report = load_project_workspace_report(&root);

        assert_eq!(report.mode, ProjectWorkspaceMode::Soft);
        assert!(report.ordinary_prompt_allowed);
        assert!(!report.strict_manifests_available);
        assert!(report.warning_count() >= 1);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "local_only_boundary_missing")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn report_strict_workspace_detects_manifest_dirs() {
        let root = temp_root("strict-report");
        let vac_dir = root.join(".vac");
        for dir in ["capabilities", "policies", "workflows", "surfaces"] {
            fs::create_dir_all(vac_dir.join(dir)).expect("create strict dir");
        }
        fs::write(vac_dir.join("profile.yaml"), "adoption_mode: strict\n").expect("profile");
        fs::write(
            vac_dir.join(".gitignore"),
            "db/\nsessions/\nindex/\nartifacts/\nlogs/\ncache/\ntmp/\n",
        )
        .expect("gitignore");

        let report = load_project_workspace_report(&root);

        assert_eq!(report.mode, ProjectWorkspaceMode::Strict);
        assert!(report.strict_manifests_available);
        assert_eq!(report.error_count(), 0);
        assert_eq!(report.warning_count(), 0);
        assert!(report.render_text().contains("mode: strict"));

        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn bootstrap_preview_is_side_effect_free_and_reviewable() {
        let root = temp_root("bootstrap-preview");
        fs::write(root.join("Cargo.toml"), "[workspace]\n").expect("write cargo");

        let plan = build_soft_workspace_bootstrap_plan(&root);
        let rendered = plan.render_text();

        assert!(!root.join(".vac").exists());
        assert!(rendered.contains("VAC did not find a .vac workspace."));
        assert!(rendered.contains("ordinary_prompt_allowed_without_write: true"));
        assert!(rendered.contains("durable_write_requires_approval: true"));
        assert!(rendered.contains(".vac/profile.yaml"));
        assert!(rendered.contains("rust/cargo"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn denied_bootstrap_keeps_in_memory_mode_without_files() {
        let root = temp_root("bootstrap-denied");

        let result = materialize_soft_workspace_bootstrap(&root, false);

        assert_eq!(
            result,
            Err(ProjectWorkspaceBootstrapError::ApprovalRequired)
        );
        assert!(!root.join(".vac").exists());
        let report = load_project_workspace_report(&root);
        assert_eq!(report.mode, ProjectWorkspaceMode::InMemory);
        assert!(report.ordinary_prompt_allowed);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn approved_bootstrap_creates_soft_workspace_boundaries() {
        let root = temp_root("bootstrap-approved");
        fs::write(root.join("package.json"), "{}\n").expect("package");

        let result = materialize_soft_workspace_bootstrap(&root, true).expect("bootstrap");

        assert_eq!(result.mode, ProjectWorkspaceMode::Soft);
        assert!(root.join(".vac/profile.yaml").is_file());
        assert!(root.join(".vac/.gitignore").is_file());
        assert!(root.join(".vac/db").is_dir());
        assert!(root.join(".vac/tmp").is_dir());
        let report = load_project_workspace_report(&root);
        assert_eq!(report.mode, ProjectWorkspaceMode::Soft);
        assert_eq!(report.warning_count(), 0);
        assert!(
            report
                .inferred_stack
                .iter()
                .any(|stack| stack == "node/package-json")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn approved_bootstrap_refuses_existing_strict_workspace() {
        let root = temp_root("bootstrap-existing-strict");
        let vac_dir = root.join(".vac");
        for dir in ["capabilities", "policies", "workflows", "surfaces"] {
            fs::create_dir_all(vac_dir.join(dir)).expect("strict dir");
        }

        let result = materialize_soft_workspace_bootstrap(&root, true);

        assert!(matches!(
            result,
            Err(ProjectWorkspaceBootstrapError::VacPathAlreadyExists(_))
        ));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn strict_product_report_without_vac_is_error_without_bootstrap_green() {
        let root = temp_root("strict-product-report-missing-vac");

        let report = load_project_workspace_report_with_options(&root, true);

        assert_eq!(report.mode, ProjectWorkspaceMode::Strict);
        assert_eq!(report.strict_gate_status, StrictGateStatus::FatalMissingVac);
        assert!(!report.ordinary_prompt_allowed);
        assert!(!report.bootstrap_offer_available);
        assert_eq!(report.warning_count(), 0);
        assert_eq!(report.error_count(), 1);
        assert!(report.is_failure());
        assert!(
            report
                .render_text()
                .contains("code: missing_vac_workspace_strict_product_repo")
        );
        assert!(!report.render_text().contains("bootstrap_preview:"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_vac_report_renders_bootstrap_preview() {
        let root = temp_root("missing-vac-preview-render");

        let report = load_project_workspace_report(&root);
        let rendered = report.render_text();

        assert!(rendered.contains("bootstrap_preview:"));
        assert!(rendered.contains("VAC Project Workspace Bootstrap Preview"));
        assert!(rendered.contains("would_create_files:"));
        assert!(rendered.contains("disabled_until_promotion:"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn startup_notice_exists_only_for_user_project_missing_vac() {
        let root = temp_root("startup-notice-missing-vac");
        fs::write(root.join("Cargo.toml"), "[workspace]\n").expect("cargo");

        let notice = project_workspace_startup_notice(&root).expect("notice");

        assert_eq!(notice.mode, ProjectWorkspaceMode::InMemory);
        assert_eq!(notice.strict_gate_status, StrictGateStatus::SetupWarning);
        assert!(notice.ordinary_prompt_allowed);
        assert!(notice.bootstrap_offer_available);
        assert!(
            notice
                .render_tui_warning()
                .contains("Continuing in `in_memory` mode")
        );
        assert!(
            notice
                .render_cli_preflight()
                .contains("--bootstrap-soft --yes")
        );

        fs::create_dir(root.join(".vac")).expect("create vac");
        assert_eq!(project_workspace_startup_notice(&root), None);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn confirmation_dialog_renders_soft_bootstrap_and_strict_promotion_choices() {
        let root = temp_root("workspace-confirmation-dialog");
        fs::write(root.join("Cargo.toml"), "[workspace]\n").expect("cargo");

        let dialog = build_project_workspace_confirmation_dialog(&root).expect("dialog");
        let rendered = dialog.render_text();

        assert_eq!(
            dialog.default_choice,
            ProjectWorkspaceUserChoice::ContinueInMemory
        );
        assert!(dialog.ordinary_prompt_allowed);
        assert!(dialog.bootstrap_offer_available);
        assert!(dialog.strict_promotion_available);
        assert!(
            dialog
                .choices
                .contains(&ProjectWorkspaceUserChoice::ApproveSoftBootstrap)
        );
        assert!(
            dialog
                .choices
                .contains(&ProjectWorkspaceUserChoice::ReviewStrictPromotion)
        );
        assert!(rendered.contains("VAC Project Workspace Setup"));
        assert!(rendered.contains("approve_soft_bootstrap"));
        assert!(rendered.contains("review_strict_promotion"));
        assert_eq!(
            dialog.resolve_choice(ProjectWorkspaceUserChoice::ApproveSoftBootstrap),
            ProjectWorkspacePromptAction::BootstrapSoft
        );
        assert_eq!(
            dialog.resolve_choice(ProjectWorkspaceUserChoice::ReviewStrictPromotion),
            ProjectWorkspacePromptAction::ReviewStrictPromotion
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn strict_promotion_preview_is_review_only_and_has_manifest_dirs() {
        let root = temp_root("strict-promotion-preview");

        let preview = build_strict_workspace_promotion_preview(&root);
        let rendered = preview.render_text();

        assert!(preview.requires_review);
        assert!(!preview.automatic_yaml_migration);
        assert!(rendered.contains("target_mode: strict"));
        assert!(rendered.contains(".vac/capabilities/"));
        assert!(rendered.contains(".vac/policies/"));
        assert!(rendered.contains("automatic_yaml_migration: false"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn denied_strict_promotion_writes_nothing() {
        let root = temp_root("strict-promotion-denied");
        materialize_soft_workspace_bootstrap(&root, true).expect("soft bootstrap");

        let result = materialize_strict_workspace_promotion(&root, false);

        assert_eq!(
            result,
            Err(ProjectWorkspaceBootstrapError::ApprovalRequired)
        );
        assert!(!root.join(".vac/capabilities").exists());
        assert_eq!(
            load_project_workspace_report(&root).mode,
            ProjectWorkspaceMode::Soft
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn approved_strict_promotion_creates_reviewed_manifest_boundaries() {
        let root = temp_root("strict-promotion-approved");
        fs::write(root.join("Cargo.toml"), "[workspace]\n").expect("cargo");
        materialize_soft_workspace_bootstrap(&root, true).expect("soft bootstrap");

        let result = materialize_strict_workspace_promotion(&root, true).expect("strict promotion");

        assert_eq!(result.mode, ProjectWorkspaceMode::Strict);
        assert!(root.join(".vac/capabilities/README.md").is_file());
        assert!(root.join(".vac/policies/README.md").is_file());
        assert!(root.join(".vac/workflows/README.md").is_file());
        assert!(root.join(".vac/surfaces/README.md").is_file());
        assert!(
            fs::read_to_string(root.join(".vac/profile.yaml"))
                .expect("profile")
                .contains("promotion_review_required: true")
        );
        let report = load_project_workspace_report(&root);
        assert_eq!(report.mode, ProjectWorkspaceMode::Strict);
        assert!(report.strict_manifests_available);

        let _ = fs::remove_dir_all(root);
    }
}
