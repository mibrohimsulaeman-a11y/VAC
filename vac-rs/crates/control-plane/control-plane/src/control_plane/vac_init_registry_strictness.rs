#![allow(dead_code)]
//! Strict-mode VAC-Init registry hardening helpers.
//!
//! This module is intentionally dependency-free. Runtime YAML traversal lives in
//! `scripts/check-vac-init-registry-strictness-contract.sh`; the Rust side keeps
//! the diagnostic vocabulary and command/ready-capability invariants stable for
//! targeted `rustc --test` gates.

use std::fmt;

pub const STRICTNESS_SCHEMA_VERSION: u32 = 1;

pub const SPEC_KINDS: &[&str] = &[
    "capability",
    "policy",
    "workflow",
    "workflow_step",
    "surface",
    "registry_status",
    "domains",
    "init_state",
    "evidence",
    "plan",
    "approval_request",
    "ownership_report",
    "memory_record",
    "risk_finding",
    "migration",
    "trajectory",
    "test_assertion",
];

pub const COMPATIBILITY_KINDS: &[&str] = &["product", "status", "donor_inventory"];

pub const READY_CAPABILITY_REQUIRED_FIELDS: &[&str] = &[
    "owner",
    "ownership",
    "policy",
    "surfaces",
    "validation",
    "docs",
];

pub const VALIDATION_COMMAND_REQUIRED_FIELDS: &[&str] =
    &["id", "runner", "args", "risk", "approval"];

pub const ALLOWED_COMMAND_RISKS: &[&str] = &[
    "safe_read",
    "low",
    "medium",
    "high",
    "critical",
    "execute_process",
];

pub const ALLOWED_APPROVAL_MODES: &[&str] = &["policy", "always", "never"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StrictnessSeverity {
    Info,
    Warning,
    Error,
    Blocked,
}

impl StrictnessSeverity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Blocked => "blocked",
        }
    }

    pub const fn is_failure(self) -> bool {
        matches!(self, Self::Error | Self::Blocked)
    }
}

impl fmt::Display for StrictnessSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrictnessDiagnostic {
    pub severity: StrictnessSeverity,
    pub code: &'static str,
    pub field_path: String,
    pub message: String,
    pub remediation: String,
}

impl StrictnessDiagnostic {
    pub fn new(
        severity: StrictnessSeverity,
        code: &'static str,
        field_path: impl Into<String>,
        message: impl Into<String>,
        remediation: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code,
            field_path: field_path.into(),
            message: message.into(),
            remediation: remediation.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredCommandShape<'a> {
    pub id: &'a str,
    pub runner: &'a str,
    pub args: &'a [&'a str],
    pub risk: &'a str,
    pub approval: &'a str,
}

pub fn is_spec_kind(kind: &str) -> bool {
    SPEC_KINDS.contains(&kind)
}

pub fn is_compatibility_kind(kind: &str) -> bool {
    COMPATIBILITY_KINDS.contains(&kind)
}

pub fn validate_kind_strict(kind: &str) -> Option<StrictnessDiagnostic> {
    if is_spec_kind(kind) {
        None
    } else if is_compatibility_kind(kind) {
        Some(StrictnessDiagnostic::new(
            StrictnessSeverity::Blocked,
            "VAC-STRICT-KIND-COMPAT",
            "kind",
            format!("compatibility manifest kind `{kind}` is forbidden in strict mode"),
            "migrate this manifest to a refined spec kind such as registry_status",
        ))
    } else {
        Some(StrictnessDiagnostic::new(
            StrictnessSeverity::Blocked,
            "VAC-STRICT-KIND-UNKNOWN",
            "kind",
            format!("unknown manifest kind `{kind}`"),
            "use a kind from the v1-alpha refined kind registry",
        ))
    }
}

pub fn missing_ready_capability_fields<I>(present_fields: I) -> Vec<&'static str>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let present = present_fields
        .into_iter()
        .map(|field| field.as_ref().to_string())
        .collect::<Vec<_>>();
    READY_CAPABILITY_REQUIRED_FIELDS
        .iter()
        .copied()
        .filter(|field| !present.iter().any(|present| present == field))
        .collect()
}

pub fn validate_structured_command_shape(
    command: &StructuredCommandShape<'_>,
) -> Vec<StrictnessDiagnostic> {
    let mut diagnostics = Vec::new();

    if !is_dotted_id(command.id) {
        diagnostics.push(StrictnessDiagnostic::new(
            StrictnessSeverity::Blocked,
            "VAC-STRICT-CMD-ID",
            "validation.commands[].id",
            "structured command id must be a dotted identifier",
            "use a stable id such as vac.init.registry-strictness.validation.cmd001",
        ));
    }

    if command.runner.trim().is_empty() {
        diagnostics.push(StrictnessDiagnostic::new(
            StrictnessSeverity::Blocked,
            "VAC-STRICT-CMD-RUNNER",
            "validation.commands[].runner",
            "runner must not be empty",
            "set runner to a registered executable name",
        ));
    }

    if command.runner.contains('/')
        || command.runner.contains('\\')
        || contains_shell_meta(command.runner)
    {
        diagnostics.push(StrictnessDiagnostic::new(
            StrictnessSeverity::Blocked,
            "VAC-STRICT-CMD-RUNNER-PATH",
            "validation.commands[].runner",
            "runner must be an executable name, not an arbitrary path or shell fragment",
            "use runner: vac, cargo, rustc, bash, python3, rg, or echo",
        ));
    }

    if is_shell_runner(command.runner)
        && (command.args.is_empty()
            || command
                .args
                .iter()
                .any(|arg| *arg == "-c" || *arg == "--command"))
    {
        diagnostics.push(StrictnessDiagnostic::new(
            StrictnessSeverity::Blocked,
            "VAC-STRICT-CMD-SHELL-INLINE",
            "validation.commands[].args",
            "shell runners must point to a checked-in script and must not use -c",
            "move inline shell into scripts/check-*.sh and call it as an argument",
        ));
    }

    for arg in command.args {
        if contains_shell_meta(arg) {
            diagnostics.push(StrictnessDiagnostic::new(
                StrictnessSeverity::Blocked,
                "VAC-STRICT-CMD-ARG-SHELL-META",
                "validation.commands[].args[]",
                format!("argument `{arg}` contains shell metacharacters"),
                "split arguments into literal argv entries or move compound logic into a checked script",
            ));
        }
    }

    if !ALLOWED_COMMAND_RISKS.contains(&command.risk) {
        diagnostics.push(StrictnessDiagnostic::new(
            StrictnessSeverity::Blocked,
            "VAC-STRICT-CMD-RISK",
            "validation.commands[].risk",
            format!("risk `{}` is not allowed", command.risk),
            "use safe_read, low, medium, high, critical, or execute_process",
        ));
    }

    if !ALLOWED_APPROVAL_MODES.contains(&command.approval) {
        diagnostics.push(StrictnessDiagnostic::new(
            StrictnessSeverity::Blocked,
            "VAC-STRICT-CMD-APPROVAL",
            "validation.commands[].approval",
            format!("approval mode `{}` is not allowed", command.approval),
            "use policy, always, or never",
        ));
    }

    diagnostics
}

pub fn contains_shell_meta(value: &str) -> bool {
    ["|", ">", "<", "&&", "||", ";", "`", "$(", "${"]
        .iter()
        .any(|needle| value.contains(needle))
}

fn is_shell_runner(runner: &str) -> bool {
    matches!(
        runner,
        "bash" | "sh" | "zsh" | "fish" | "cmd" | "powershell" | "pwsh"
    )
}

fn is_dotted_id(value: &str) -> bool {
    if value.starts_with('.')
        || value.ends_with('.')
        || value.contains("..")
        || !value.contains('.')
    {
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

    #[test]
    fn strict_kind_rejects_current_compatibility_names() {
        for kind in COMPATIBILITY_KINDS {
            let diagnostic = validate_kind_strict(kind).expect("compatibility kind must fail");
            assert_eq!(diagnostic.severity, StrictnessSeverity::Blocked);
            assert_eq!(diagnostic.code, "VAC-STRICT-KIND-COMPAT");
        }
        assert!(validate_kind_strict("registry_status").is_none());
    }

    #[test]
    fn ready_capability_requires_docs_and_runtime_blocks() {
        let missing =
            missing_ready_capability_fields(["owner", "policy", "surfaces", "validation"]);
        assert_eq!(missing, vec!["ownership", "docs"]);
    }

    #[test]
    fn structured_command_shape_accepts_script_runner() {
        let command = StructuredCommandShape {
            id: "vac.init.registry-strictness.validation.cmd001",
            runner: "bash",
            args: &["scripts/check-vac-init-registry-strictness-contract.sh"],
            risk: "execute_process",
            approval: "policy",
        };
        assert!(validate_structured_command_shape(&command).is_empty());
    }

    #[test]
    fn structured_command_shape_rejects_shell_fragments() {
        let command = StructuredCommandShape {
            id: "bad",
            runner: "bash",
            args: &["-c", "cargo test && rm -rf target"],
            risk: "execute_process",
            approval: "policy",
        };
        let diagnostics = validate_structured_command_shape(&command);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "VAC-STRICT-CMD-ID")
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "VAC-STRICT-CMD-SHELL-INLINE")
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "VAC-STRICT-CMD-ARG-SHELL-META")
        );
    }
}
