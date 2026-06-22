//! VAC structured command model.
//!
//! This crate is intentionally shell-free. It describes process execution as
//! `runner + args`, validates the runner against an explicit registry, and
//! reports why a request cannot be treated as a VAC structured command. It is
//! the shared contract used by the v1.9 agent boundary, MCP command tools, and
//! doctor fixtures.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredCommand {
    pub id: String,
    pub runner: String,
    pub args: Vec<String>,
    pub risk: CommandRisk,
    pub approval: CommandApproval,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandRisk {
    SafeRead,
    Low,
    Medium,
    High,
    Critical,
    ExecuteProcess,
    Destructive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandApproval {
    Never,
    Policy,
    Always,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunnerRegistry {
    pub allowed_runners: Vec<String>,
}

impl Default for RunnerRegistry {
    fn default() -> Self {
        Self {
            allowed_runners: vec![
                "cargo".to_string(),
                "rustc".to_string(),
                "python3".to_string(),
                "python".to_string(),
                "node".to_string(),
                "npm".to_string(),
                "pnpm".to_string(),
                "git".to_string(),
                "vac".to_string(),
                "vac-script-runner".to_string(),
                "echo".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellFreeViolation {
    Empty,
    ShellRunner,
    RunnerPath,
    UnknownRunner,
    ShellMetacharacter,
    Multiline,
    QuotingSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandExecutionPlan {
    pub command: StructuredCommand,
    pub shell_free: bool,
    pub violations: Vec<ShellFreeViolation>,
    pub policy_snapshot_hash: Option<String>,
    pub plan_id: Option<String>,
}

impl StructuredCommand {
    #[must_use]
    pub fn is_shell_free(&self) -> bool {
        evaluate_shell_free(self, &RunnerRegistry::default()).is_empty()
    }
}

#[must_use]
pub fn evaluate_shell_free(
    command: &StructuredCommand,
    registry: &RunnerRegistry,
) -> Vec<ShellFreeViolation> {
    let mut violations = Vec::new();
    if command.runner.trim().is_empty() {
        violations.push(ShellFreeViolation::Empty);
    }
    if matches!(
        command.runner.as_str(),
        "sh" | "bash" | "zsh" | "fish" | "cmd" | "powershell" | "pwsh"
    ) {
        violations.push(ShellFreeViolation::ShellRunner);
    }
    if command.runner.contains('/') || command.runner.contains('\\') {
        violations.push(ShellFreeViolation::RunnerPath);
    }
    if !registry
        .allowed_runners
        .iter()
        .any(|runner| runner == &command.runner)
    {
        violations.push(ShellFreeViolation::UnknownRunner);
    }
    for arg in &command.args {
        if arg.contains('\n') || arg.contains('\r') {
            violations.push(ShellFreeViolation::Multiline);
        }
        if arg.contains('"') || arg.contains('\'') {
            violations.push(ShellFreeViolation::QuotingSyntax);
        }
        if contains_shell_metachar(arg) {
            violations.push(ShellFreeViolation::ShellMetacharacter);
        }
    }
    violations.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    violations.dedup();
    violations
}

pub fn parse_command(
    id: impl Into<String>,
    command: &str,
    risk: CommandRisk,
    approval: CommandApproval,
    registry: &RunnerRegistry,
) -> Result<CommandExecutionPlan, Vec<ShellFreeViolation>> {
    if command.trim().is_empty() {
        return Err(vec![ShellFreeViolation::Empty]);
    }
    if command.contains('\n') || command.contains('\r') {
        return Err(vec![ShellFreeViolation::Multiline]);
    }
    let tokens = command
        .split_whitespace()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let Some(runner) = tokens.first().cloned() else {
        return Err(vec![ShellFreeViolation::Empty]);
    };
    let structured = StructuredCommand {
        id: id.into(),
        runner,
        args: tokens.into_iter().skip(1).collect(),
        risk,
        approval,
    };
    let violations = evaluate_shell_free(&structured, registry);
    if violations.is_empty() {
        Ok(CommandExecutionPlan {
            command: structured,
            shell_free: true,
            violations,
            policy_snapshot_hash: None,
            plan_id: None,
        })
    } else {
        Err(violations)
    }
}

#[must_use]
pub fn contains_shell_metachar(token: &str) -> bool {
    token.chars().any(|ch| {
        matches!(
            ch,
            '|' | '>' | '<' | ';' | '&' | '`' | '$' | '*' | '?' | '~'
        )
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrokerOwnedScriptBinding {
    pub script_id: String,
    pub immutable_script_path: String,
    pub script_sha256: String,
    pub runner: String,
}

#[must_use]
pub fn broker_script_runner_command(
    id: impl Into<String>,
    binding: &BrokerOwnedScriptBinding,
    risk: CommandRisk,
    approval: CommandApproval,
) -> StructuredCommand {
    StructuredCommand {
        id: id.into(),
        runner: binding.runner.clone(),
        args: vec!["--script-id".to_string(), binding.script_id.clone()],
        risk,
        approval,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRegistryEntry {
    pub id: String,
    pub runner: String,
    pub args: Vec<String>,
    pub risk: CommandRisk,
    pub approval: CommandApproval,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandContractViolation {
    IdMismatch,
    RunnerMismatch,
    ArgsMismatch,
    ShellFree(Vec<ShellFreeViolation>),
}

#[must_use]
pub fn validate_command_object_against_registry(
    command: &StructuredCommand,
    expected: &CommandRegistryEntry,
    registry: &RunnerRegistry,
) -> Vec<CommandContractViolation> {
    let mut out = Vec::new();
    if command.id != expected.id {
        out.push(CommandContractViolation::IdMismatch);
    }
    if command.runner != expected.runner {
        out.push(CommandContractViolation::RunnerMismatch);
    }
    if command.args != expected.args {
        out.push(CommandContractViolation::ArgsMismatch);
    }
    let shell = evaluate_shell_free(command, registry);
    if !shell.is_empty() {
        out.push(CommandContractViolation::ShellFree(shell));
    }
    out
}

#[must_use]
pub fn command_object_authorized_by_registry(
    command: &StructuredCommand,
    expected: &CommandRegistryEntry,
    registry: &RunnerRegistry,
) -> bool {
    validate_command_object_against_registry(command, expected, registry).is_empty()
}
