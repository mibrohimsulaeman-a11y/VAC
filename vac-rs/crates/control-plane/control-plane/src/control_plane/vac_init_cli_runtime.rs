#![allow(dead_code)]
//! VAC-Init CLI runtime foundation contracts.
//!
//! This module keeps the B1-B4 CLI surface rules dependency-free so sandbox
//! gates can use targeted `rustc --test` instead of a full workspace build.

use std::fmt;

pub const CLI_RUNTIME_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CliRuntimeCommandKind {
    Init,
    PlanValidate,
    Why,
    DoctorRegistry,
    DoctorSurfaces,
    DoctorPolicy,
    DoctorOwnership,
    DoctorWorkflow,
    DoctorSessions,
    DoctorEvidence,
    DoctorEnforcement,
    DoctorBuild,
    DoctorMemory,
    DoctorInit,
    DoctorRelease,
}

impl CliRuntimeCommandKind {
    pub const fn command(self) -> &'static str {
        match self {
            Self::Init => "vac init",
            Self::PlanValidate => "vac plan validate",
            Self::Why => "vac why",
            Self::DoctorRegistry => "vac doctor registry",
            Self::DoctorSurfaces => "vac doctor surfaces",
            Self::DoctorPolicy => "vac doctor policy",
            Self::DoctorOwnership => "vac doctor ownership",
            Self::DoctorWorkflow => "vac doctor workflow",
            Self::DoctorSessions => "vac doctor sessions",
            Self::DoctorEvidence => "vac doctor evidence",
            Self::DoctorEnforcement => "vac doctor enforcement",
            Self::DoctorBuild => "vac doctor build",
            Self::DoctorMemory => "vac doctor memory",
            Self::DoctorInit => "vac doctor init",
            Self::DoctorRelease => "vac doctor release",
        }
    }

    pub const fn is_required_for_b(self) -> bool {
        true
    }
}

pub const REQUIRED_B_COMMANDS: &[CliRuntimeCommandKind] = &[
    CliRuntimeCommandKind::Init,
    CliRuntimeCommandKind::PlanValidate,
    CliRuntimeCommandKind::Why,
    CliRuntimeCommandKind::DoctorRegistry,
    CliRuntimeCommandKind::DoctorSurfaces,
    CliRuntimeCommandKind::DoctorPolicy,
    CliRuntimeCommandKind::DoctorOwnership,
    CliRuntimeCommandKind::DoctorWorkflow,
    CliRuntimeCommandKind::DoctorSessions,
    CliRuntimeCommandKind::DoctorEvidence,
    CliRuntimeCommandKind::DoctorEnforcement,
    CliRuntimeCommandKind::DoctorBuild,
    CliRuntimeCommandKind::DoctorMemory,
    CliRuntimeCommandKind::DoctorInit,
    CliRuntimeCommandKind::DoctorRelease,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliExitClass {
    Pass,
    ChecksFailed,
    Fatal,
}

impl CliExitClass {
    pub const fn code(self) -> i32 {
        match self {
            Self::Pass => 0,
            Self::ChecksFailed => 1,
            Self::Fatal => 2,
        }
    }
}

impl fmt::Display for CliExitClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Pass => "pass",
            Self::ChecksFailed => "checks_failed",
            Self::Fatal => "fatal",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitCliMode {
    Apply,
    DryRun,
    Resume,
    Status,
    Scan,
    RescanAst,
}

impl InitCliMode {
    pub const fn mutates_workspace(self) -> bool {
        matches!(
            self,
            Self::Apply | Self::Resume | Self::Scan | Self::RescanAst
        )
    }

    pub const fn writes_init_state(self) -> bool {
        matches!(
            self,
            Self::Apply | Self::Resume | Self::Scan | Self::RescanAst
        )
    }
}

pub fn resolve_init_mode(
    dry_run: bool,
    resume: bool,
    status: bool,
    scan: bool,
    rescan_ast: bool,
) -> Result<InitCliMode, &'static str> {
    let selected = [dry_run, resume, status, scan, rescan_ast]
        .iter()
        .filter(|flag| **flag)
        .count();
    if selected > 1 {
        return Err(
            "vac init accepts only one of --dry-run, --resume, --status, --scan, or --rescan-ast",
        );
    }
    if dry_run {
        Ok(InitCliMode::DryRun)
    } else if resume {
        Ok(InitCliMode::Resume)
    } else if status {
        Ok(InitCliMode::Status)
    } else if scan {
        Ok(InitCliMode::Scan)
    } else if rescan_ast {
        Ok(InitCliMode::RescanAst)
    } else {
        Ok(InitCliMode::Apply)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanValidateCliContract {
    pub plan_path: String,
    pub workspace_root: String,
    pub requires_policy: bool,
    pub rejects_free_form_commands: bool,
}

impl PlanValidateCliContract {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.plan_path.trim().is_empty() {
            return Err("vac plan validate requires a plan file");
        }
        if self.plan_path.starts_with('/') || self.plan_path.contains("..") {
            return Err("plan path must be workspace-relative for contract validation");
        }
        if self.workspace_root.trim().is_empty() {
            return Err("workspace root must be known");
        }
        if !self.requires_policy {
            return Err("plan validation must fail closed when policy is missing");
        }
        if !self.rejects_free_form_commands {
            return Err("plan validation must reject free-form validation commands");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WhyCliTarget {
    File {
        file: String,
    },
    Line {
        file: String,
        line: usize,
    },
    Range {
        file: String,
        start: usize,
        end: usize,
    },
    Symbol {
        file: String,
        symbol: String,
    },
}

impl WhyCliTarget {
    pub fn file(&self) -> &str {
        match self {
            Self::File { file }
            | Self::Line { file, .. }
            | Self::Range { file, .. }
            | Self::Symbol { file, .. } => file,
        }
    }
}

pub fn parse_why_target(input: &str) -> Result<WhyCliTarget, String> {
    let value = input.trim();
    if value.is_empty() {
        return Err("vac why requires a target such as <file>:<line>".to_string());
    }
    if let Some((file, symbol)) = value.split_once("::") {
        validate_relative_path(file)?;
        if symbol.trim().is_empty() {
            return Err("symbol query must not be empty".to_string());
        }
        return Ok(WhyCliTarget::Symbol {
            file: file.to_string(),
            symbol: symbol.to_string(),
        });
    }

    let Some((file, suffix)) = value.rsplit_once(':') else {
        validate_relative_path(value)?;
        return Ok(WhyCliTarget::File {
            file: value.to_string(),
        });
    };
    validate_relative_path(file)?;
    if let Some((start, end)) = suffix.split_once('-') {
        let start = parse_one_based(start)?;
        let end = parse_one_based(end)?;
        if end < start {
            return Err("range end must be >= start".to_string());
        }
        return Ok(WhyCliTarget::Range {
            file: file.to_string(),
            start,
            end,
        });
    }
    let line = parse_one_based(suffix)?;
    Ok(WhyCliTarget::Line {
        file: file.to_string(),
        line,
    })
}

fn parse_one_based(value: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("line value `{value}` is not a number"))?;
    if parsed == 0 {
        return Err("line values are one-based".to_string());
    }
    Ok(parsed)
}

fn validate_relative_path(path: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("file path is empty".to_string());
    }
    if path.starts_with('/') || path.contains("..") || path.contains('\\') {
        return Err(format!("file path must be workspace-relative: {path}"));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorTaxonomyContract {
    pub commands: Vec<CliRuntimeCommandKind>,
}

impl DoctorTaxonomyContract {
    pub fn production_b() -> Self {
        Self {
            commands: REQUIRED_B_COMMANDS.to_vec(),
        }
    }

    pub fn missing_required(&self) -> Vec<CliRuntimeCommandKind> {
        REQUIRED_B_COMMANDS
            .iter()
            .copied()
            .filter(|required| !self.commands.contains(required))
            .collect()
    }

    pub fn validate(&self) -> Result<(), Vec<CliRuntimeCommandKind>> {
        let missing = self.missing_required();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_mode_is_exclusive_and_status_is_read_only() {
        assert_eq!(
            resolve_init_mode(false, false, false, false, false).unwrap(),
            InitCliMode::Apply
        );
        assert_eq!(
            resolve_init_mode(true, false, false, false, false).unwrap(),
            InitCliMode::DryRun
        );
        assert!(!InitCliMode::DryRun.mutates_workspace());
        assert!(!InitCliMode::Status.writes_init_state());
        assert!(InitCliMode::RescanAst.writes_init_state());
        assert!(resolve_init_mode(true, true, false, false, false).is_err());
    }

    #[test]
    fn plan_validate_contract_is_fail_closed() {
        let valid = PlanValidateCliContract {
            plan_path: "tests/fixtures/plans/valid_plan.yaml".to_string(),
            workspace_root: ".".to_string(),
            requires_policy: true,
            rejects_free_form_commands: true,
        };
        assert!(valid.validate().is_ok());

        let missing_policy = PlanValidateCliContract {
            requires_policy: false,
            ..valid.clone()
        };
        assert_eq!(
            missing_policy.validate(),
            Err("plan validation must fail closed when policy is missing")
        );
    }

    #[test]
    fn why_target_parser_handles_line_range_symbol_and_file() {
        assert_eq!(
            parse_why_target("src/lib.rs:10").unwrap(),
            WhyCliTarget::Line {
                file: "src/lib.rs".to_string(),
                line: 10
            }
        );
        assert_eq!(
            parse_why_target("src/lib.rs:10-12").unwrap(),
            WhyCliTarget::Range {
                file: "src/lib.rs".to_string(),
                start: 10,
                end: 12
            }
        );
        assert_eq!(
            parse_why_target("src/lib.rs::render").unwrap(),
            WhyCliTarget::Symbol {
                file: "src/lib.rs".to_string(),
                symbol: "render".to_string()
            }
        );
        assert_eq!(
            parse_why_target("src/lib.rs").unwrap(),
            WhyCliTarget::File {
                file: "src/lib.rs".to_string()
            }
        );
    }

    #[test]
    fn why_target_parser_rejects_unsafe_paths() {
        assert!(parse_why_target("../secret:1").is_err());
        assert!(parse_why_target("/tmp/file:1").is_err());
        assert!(parse_why_target("src/lib.rs:0").is_err());
    }

    #[test]
    fn doctor_taxonomy_contains_b_required_commands() {
        let taxonomy = DoctorTaxonomyContract::production_b();
        assert!(taxonomy.validate().is_ok());
        assert!(
            taxonomy
                .commands
                .iter()
                .any(|command| command.command() == "vac doctor evidence")
        );
        assert!(
            taxonomy
                .commands
                .iter()
                .any(|command| command.command() == "vac doctor enforcement")
        );
        assert!(
            taxonomy
                .commands
                .iter()
                .any(|command| command.command() == "vac doctor sessions")
        );
        assert!(
            taxonomy
                .commands
                .iter()
                .any(|command| command.command() == "vac doctor memory")
        );
        assert!(
            taxonomy
                .commands
                .iter()
                .any(|command| command.command() == "vac doctor init")
        );
    }

    #[test]
    fn exit_classes_match_doctor_contract() {
        assert_eq!(CliExitClass::Pass.code(), 0);
        assert_eq!(CliExitClass::ChecksFailed.code(), 1);
        assert_eq!(CliExitClass::Fatal.code(), 2);
    }
}
