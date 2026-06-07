#![allow(dead_code)]
//! Semantic Plan contract and pre-plan validator for VAC-Init.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlanStatus {
    Draft,
    Approved,
    Executing,
    Completed,
    Rejected,
    Abandoned,
}

impl PlanStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Approved => "approved",
            Self::Executing => "executing",
            Self::Completed => "completed",
            Self::Rejected => "rejected",
            Self::Abandoned => "abandoned",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CapabilityExecutionStatus {
    Planned,
    Partial,
    Ready,
    Deprecated,
}

impl CapabilityExecutionStatus {
    pub const fn can_execute(self) -> bool {
        matches!(self, Self::Partial | Self::Ready)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FileOperation {
    Create,
    Modify,
    Delete,
}

impl FileOperation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Modify => "modify",
            Self::Delete => "delete",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SemanticAnchorKind {
    Function,
    Struct,
    Impl,
    Module,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineRange {
    pub start: usize,
    pub end: usize,
}

impl LineRange {
    pub const fn is_valid(&self) -> bool {
        self.start > 0 && self.end >= self.start
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticAnchor {
    pub symbol: String,
    pub kind: SemanticAnchorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanAllowedFile {
    pub path: String,
    pub operation: FileOperation,
    pub line_range: Option<LineRange>,
    pub semantic_anchor: Option<SemanticAnchor>,
    pub ownership: String,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanValidationCommand {
    pub id: String,
    pub runner: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanBounds {
    pub max_patches: usize,
    pub max_new_files: usize,
    pub max_line_delta: isize,
    pub timeout_seconds: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticPlan {
    pub id: String,
    pub title: String,
    pub status: PlanStatus,
    pub capability: String,
    pub allowed_files: Vec<PlanAllowedFile>,
    pub forbidden_files: Vec<String>,
    pub forbidden_actions: Vec<String>,
    pub validation_commands: Vec<PlanValidationCommand>,
    pub approval_required: bool,
    pub bounds: PlanBounds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanValidationIssue {
    pub code: String,
    pub message: String,
}

impl PlanValidationIssue {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlanValidationReport {
    pub issues: Vec<PlanValidationIssue>,
}

impl PlanValidationReport {
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn push(&mut self, code: impl Into<String>, message: impl Into<String>) {
        self.issues.push(PlanValidationIssue::new(code, message));
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlanValidationContext {
    pub capabilities: BTreeMap<String, CapabilityExecutionStatus>,
    pub file_owners: BTreeMap<String, String>,
    pub known_files: BTreeSet<String>,
}

pub fn validate_semantic_plan(
    plan: &SemanticPlan,
    ctx: &PlanValidationContext,
) -> PlanValidationReport {
    let mut report = PlanValidationReport::default();

    if !is_dotted_plan_id(&plan.id) {
        report.push(
            "plan.id.invalid",
            "plan id must be a dotted identifier starting with plan.",
        );
    }
    match ctx.capabilities.get(&plan.capability) {
        None => report.push(
            "plan.capability.missing",
            format!("capability '{}' is not registered", plan.capability),
        ),
        Some(status) if !status.can_execute() => report.push(
            "plan.capability.not_executable",
            "planned/deprecated capability cannot execute a semantic plan",
        ),
        Some(_) => {}
    }
    if plan.allowed_files.is_empty() {
        report.push(
            "plan.allowed_files.empty",
            "plan must declare at least one allowed file",
        );
    }
    if plan.bounds.max_patches == 0 || plan.bounds.timeout_seconds == 0 {
        report.push(
            "plan.bounds.invalid",
            "max_patches and timeout_seconds must be greater than zero",
        );
    }

    let mut new_file_count = 0usize;
    for file in &plan.allowed_files {
        if file.path.trim().is_empty() || file.path.starts_with('/') || file.path.contains("..") {
            report.push(
                "plan.file.path.invalid",
                format!("invalid relative path '{}'", file.path),
            );
        }
        if file.operation == FileOperation::Create {
            new_file_count += 1;
        } else if !ctx.known_files.contains(&file.path) {
            report.push(
                "plan.file.unknown",
                format!("file '{}' is not known to source inventory", file.path),
            );
        }
        if let Some(range) = &file.line_range {
            if !range.is_valid() {
                report.push(
                    "plan.file.line_range.invalid",
                    format!("invalid line range for '{}'", file.path),
                );
            }
        } else if file.operation != FileOperation::Create && file.semantic_anchor.is_none() {
            report.push(
                "plan.file.bounds.missing",
                format!(
                    "file '{}' must have a line range or semantic anchor",
                    file.path
                ),
            );
        }
        match ctx.file_owners.get(&file.path) {
            None if file.operation != FileOperation::Create => report.push(
                "plan.file.unowned",
                format!("file '{}' has no ownership record", file.path),
            ),
            Some(owner) if owner != &file.ownership => report.push(
                "plan.file.owner_mismatch",
                format!(
                    "file '{}' owner '{}' does not match plan ownership '{}'",
                    file.path, owner, file.ownership
                ),
            ),
            _ => {}
        }
        if is_forbidden(&file.path, &plan.forbidden_files) {
            report.push(
                "plan.file.forbidden",
                format!("file '{}' matches forbidden globs", file.path),
            );
        }
    }
    if new_file_count > plan.bounds.max_new_files {
        report.push(
            "plan.bounds.max_new_files",
            "plan declares more new files than bounds allow",
        );
    }
    for command in &plan.validation_commands {
        if !is_dotted_id(&command.id)
            || command.runner.contains('/')
            || command.runner.trim().is_empty()
        {
            report.push(
                "plan.command.invalid",
                format!("validation command '{}' is not structured", command.id),
            );
        }
        if command.args.iter().any(|arg| contains_shell_meta(arg)) {
            report.push(
                "plan.command.shell_meta",
                format!(
                    "validation command '{}' contains shell metacharacters",
                    command.id
                ),
            );
        }
    }

    report
}

pub fn can_transition_plan_status(from: PlanStatus, to: PlanStatus) -> bool {
    matches!(
        (from, to),
        (PlanStatus::Draft, PlanStatus::Approved)
            | (PlanStatus::Draft, PlanStatus::Rejected)
            | (PlanStatus::Approved, PlanStatus::Executing)
            | (PlanStatus::Executing, PlanStatus::Completed)
            | (PlanStatus::Executing, PlanStatus::Abandoned)
            | (PlanStatus::Executing, PlanStatus::Draft)
    )
}

fn is_forbidden(path: &str, patterns: &[String]) -> bool {
    let path = path.trim_start_matches("./");
    patterns.iter().any(|pattern| {
        let pattern = pattern.trim().trim_start_matches("./");
        if pattern.is_empty() {
            return false;
        }
        if path == pattern {
            return true;
        }
        if let Some(directory) = pattern.strip_suffix("/**") {
            return path == directory
                || path
                    .strip_prefix(directory)
                    .is_some_and(|suffix| suffix.starts_with('/'));
        }
        if let Some(prefix) = pattern.strip_suffix('*') {
            return path.starts_with(prefix);
        }
        // `**/<rest>`: match `rest` at any directory depth (the trailing path
        // component(s)), e.g. `**/secrets.yaml` matches `a/b/secrets.yaml`.
        if let Some(rest) = pattern.strip_prefix("**/") {
            return rest.is_empty() || path == rest || path.ends_with(&format!("/{rest}"));
        }
        // `*<suffix>`: suffix match within the final path segment only, never
        // crossing a `/` boundary, e.g. `*.env` matches `config/prod.env` and
        // `.env` but not `prod.env.bak`.
        if let Some(suffix) = pattern.strip_prefix('*') {
            let segment = path.rsplit('/').next().unwrap_or(path);
            return !suffix.is_empty() && segment.ends_with(suffix);
        }
        false
    })
}

fn contains_shell_meta(arg: &str) -> bool {
    ["|", ">", "<", "&&", "||", ";", "`", "$(", "${"]
        .iter()
        .any(|needle| arg.contains(needle))
}

fn is_dotted_plan_id(value: &str) -> bool {
    value.starts_with("plan.") && is_dotted_id(value)
}

fn is_dotted_id(value: &str) -> bool {
    if value.starts_with('.') || value.ends_with('.') || value.contains("..") {
        return false;
    }
    value.split('.').all(|part| {
        !part.is_empty()
            && part
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
            && part
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_lowercase())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_context() -> PlanValidationContext {
        PlanValidationContext {
            capabilities: BTreeMap::from([(
                "vac.test.fixture".to_string(),
                CapabilityExecutionStatus::Ready,
            )]),
            file_owners: BTreeMap::from([(
                "src/lib.rs".to_string(),
                "vac.test.fixture".to_string(),
            )]),
            known_files: BTreeSet::from(["src/lib.rs".to_string()]),
        }
    }

    fn valid_plan() -> SemanticPlan {
        SemanticPlan {
            id: "plan.test.fixture".to_string(),
            title: "Test plan".to_string(),
            status: PlanStatus::Draft,
            capability: "vac.test.fixture".to_string(),
            allowed_files: vec![PlanAllowedFile {
                path: "src/lib.rs".to_string(),
                operation: FileOperation::Modify,
                line_range: Some(LineRange { start: 1, end: 10 }),
                semantic_anchor: None,
                ownership: "vac.test.fixture".to_string(),
                rationale: "fixture".to_string(),
            }],
            forbidden_files: vec!["target/**".to_string()],
            forbidden_actions: vec![],
            validation_commands: vec![PlanValidationCommand {
                id: "cargo.test.fixture".to_string(),
                runner: "cargo".to_string(),
                args: vec!["test".to_string()],
            }],
            approval_required: true,
            bounds: PlanBounds {
                max_patches: 3,
                max_new_files: 1,
                max_line_delta: 50,
                timeout_seconds: 600,
            },
        }
    }

    #[test]
    fn accepts_valid_plan() {
        let report = validate_semantic_plan(&valid_plan(), &base_context());
        assert!(report.is_valid(), "{:?}", report.issues);
    }

    #[test]
    fn forbidden_files_precede_explicit_allowed_files() {
        let mut plan = valid_plan();
        plan.forbidden_files = vec!["src/**".to_string()];

        let report = validate_semantic_plan(&plan, &base_context());

        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "plan.file.forbidden")
        );
    }

    #[test]
    fn forbidden_suffix_and_recursive_globs_match() {
        // `*.ext` suffix globs match within the final path segment only.
        assert!(is_forbidden(".env", &["*.env".to_string()]));
        assert!(is_forbidden("config/prod.env", &["*.env".to_string()]));
        assert!(!is_forbidden("config/prod.toml", &["*.env".to_string()]));
        assert!(!is_forbidden("prod.env.bak", &["*.env".to_string()]));
        // `**/<rest>` recursive globs match at any directory depth.
        assert!(is_forbidden(
            "secrets.yaml",
            &["**/secrets.yaml".to_string()]
        ));
        assert!(is_forbidden(
            "a/b/secrets.yaml",
            &["**/secrets.yaml".to_string()]
        ));
        assert!(!is_forbidden(
            "a/b/other.yaml",
            &["**/secrets.yaml".to_string()]
        ));
    }

    #[test]
    fn forbidden_directory_globs_match_path_segments_only() {
        let mut plan = valid_plan();
        plan.allowed_files[0].path = "targeted/lib.rs".to_string();
        let mut ctx = base_context();
        ctx.file_owners.insert(
            "targeted/lib.rs".to_string(),
            "vac.test.fixture".to_string(),
        );
        ctx.known_files.insert("targeted/lib.rs".to_string());

        let report = validate_semantic_plan(&plan, &ctx);

        assert!(
            report
                .issues
                .iter()
                .all(|issue| issue.code != "plan.file.forbidden"),
            "{:?}",
            report.issues
        );
    }

    #[test]
    fn rejects_missing_capability() {
        let mut plan = valid_plan();
        plan.capability = "vac.unknown".to_string();
        let report = validate_semantic_plan(&plan, &base_context());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "plan.capability.missing")
        );
    }

    #[test]
    fn rejects_planned_capability_execution() {
        let mut ctx = base_context();
        ctx.capabilities.insert(
            "vac.test.fixture".to_string(),
            CapabilityExecutionStatus::Planned,
        );
        let report = validate_semantic_plan(&valid_plan(), &ctx);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "plan.capability.not_executable")
        );
    }

    #[test]
    fn rejects_unowned_target_file() {
        let mut ctx = base_context();
        ctx.file_owners.clear();
        let report = validate_semantic_plan(&valid_plan(), &ctx);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "plan.file.unowned")
        );
    }

    #[test]
    fn rejects_owner_mismatch() {
        let mut ctx = base_context();
        ctx.file_owners
            .insert("src/lib.rs".to_string(), "vac.other".to_string());
        let report = validate_semantic_plan(&valid_plan(), &ctx);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "plan.file.owner_mismatch")
        );
    }

    #[test]
    fn rejects_shell_meta_in_validation_command() {
        let mut plan = valid_plan();
        plan.validation_commands[0].args.push("|".to_string());
        let report = validate_semantic_plan(&plan, &base_context());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "plan.command.shell_meta")
        );
    }

    #[test]
    fn rejects_new_files_over_budget() {
        let mut plan = valid_plan();
        plan.bounds.max_new_files = 0;
        plan.allowed_files[0].operation = FileOperation::Create;
        plan.allowed_files[0].path = "src/new.rs".to_string();
        let report = validate_semantic_plan(&plan, &base_context());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "plan.bounds.max_new_files")
        );
    }

    #[test]
    fn transition_graph_requires_reapproval_after_revision() {
        assert!(can_transition_plan_status(
            PlanStatus::Draft,
            PlanStatus::Approved
        ));
        assert!(can_transition_plan_status(
            PlanStatus::Executing,
            PlanStatus::Draft
        ));
        assert!(!can_transition_plan_status(
            PlanStatus::Draft,
            PlanStatus::Executing
        ));
    }
}
