#![allow(dead_code)]
//! Structured command contract and pre-command gate for VAC-Init.

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommandRisk {
    SafeRead,
    Low,
    Medium,
    High,
    Critical,
    ExecuteProcess,
}

impl CommandRisk {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SafeRead => "safe_read",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
            Self::ExecuteProcess => "execute_process",
        }
    }

    pub const fn requires_approval_by_default(self) -> bool {
        matches!(self, Self::High | Self::Critical | Self::ExecuteProcess)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommandApprovalMode {
    Policy,
    Always,
    Never,
}

impl CommandApprovalMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Policy => "policy",
            Self::Always => "always",
            Self::Never => "never",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredCommand {
    pub id: String,
    pub runner: String,
    pub args: Vec<String>,
    pub risk: CommandRisk,
    pub approval: CommandApprovalMode,
}

impl StructuredCommand {
    pub fn new(
        id: impl Into<String>,
        runner: impl Into<String>,
        args: Vec<String>,
        risk: CommandRisk,
        approval: CommandApprovalMode,
    ) -> Self {
        Self {
            id: id.into(),
            runner: runner.into(),
            args,
            risk,
            approval,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandGateDecision {
    Allow,
    ApprovalRequired,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandGateIssue {
    pub code: String,
    pub message: String,
}

impl CommandGateIssue {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandGateReport {
    pub command_id: String,
    pub decision: CommandGateDecision,
    pub issues: Vec<CommandGateIssue>,
}

impl CommandGateReport {
    pub fn is_allowed(&self) -> bool {
        self.decision == CommandGateDecision::Allow
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRunnerRegistry {
    pub allowed_runners: BTreeSet<String>,
}

impl CommandRunnerRegistry {
    pub fn with_defaults() -> Self {
        let mut allowed_runners = BTreeSet::new();
        for runner in [
            "cargo", "rustc", "rustfmt", "python3", "python", "bash", "sh", "git", "vac", "echo",
            "ls", "cat", "sed", "rg", "grep", "find", "head", "tail", "wc",
        ] {
            allowed_runners.insert(runner.to_string());
        }
        Self { allowed_runners }
    }

    pub fn contains_runner(&self, runner: &str) -> bool {
        self.allowed_runners.contains(runner)
    }
}

pub fn evaluate_structured_command(
    command: &StructuredCommand,
    registry: &CommandRunnerRegistry,
) -> CommandGateReport {
    let mut issues = Vec::new();

    if command.id.trim().is_empty() || !is_dotted_id(&command.id) {
        issues.push(CommandGateIssue::new(
            "command.id.invalid",
            "command id must be a dotted identifier",
        ));
    }
    if command.runner.trim().is_empty() {
        issues.push(CommandGateIssue::new(
            "command.runner.empty",
            "runner must not be empty",
        ));
    }
    if command.runner.contains('/') || command.runner.contains('\\') {
        issues.push(CommandGateIssue::new(
            "command.runner.path",
            "runner must be a registered executable name, not an arbitrary path",
        ));
    }
    if !registry.contains_runner(&command.runner) {
        issues.push(CommandGateIssue::new(
            "command.runner.unknown",
            format!("runner '{}' is not in the runner registry", command.runner),
        ));
    }
    if is_shell_runner(&command.runner) {
        if command
            .args
            .iter()
            .any(|arg| arg == "-c" || arg == "--command")
        {
            issues.push(CommandGateIssue::new(
                "command.shell.inline",
                "shell inline execution with -c/--command is forbidden",
            ));
        }
        if command.args.is_empty() {
            issues.push(CommandGateIssue::new(
                "command.shell.no_script",
                "shell runner must point to a script path, not an interactive shell",
            ));
        }
    }
    for arg in &command.args {
        if contains_shell_meta(arg) {
            issues.push(CommandGateIssue::new(
                "command.arg.shell_meta",
                format!("argument '{}' contains shell metacharacters", arg),
            ));
        }
    }

    let decision = if issues.iter().any(|issue| {
        issue.code.starts_with("command.runner")
            || issue.code.starts_with("command.shell")
            || issue.code == "command.arg.shell_meta"
            || issue.code == "command.id.invalid"
    }) {
        CommandGateDecision::Deny
    } else if command.approval == CommandApprovalMode::Always
        || (command.approval == CommandApprovalMode::Policy
            && command.risk.requires_approval_by_default())
    {
        CommandGateDecision::ApprovalRequired
    } else {
        CommandGateDecision::Allow
    };

    CommandGateReport {
        command_id: command.id.clone(),
        decision,
        issues,
    }
}

pub fn validate_command_set(
    commands: &[StructuredCommand],
    registry: &CommandRunnerRegistry,
) -> Vec<CommandGateReport> {
    commands
        .iter()
        .map(|command| evaluate_structured_command(command, registry))
        .collect()
}

fn is_shell_runner(runner: &str) -> bool {
    matches!(
        runner,
        "bash" | "sh" | "zsh" | "fish" | "cmd" | "powershell" | "pwsh"
    )
}

fn contains_shell_meta(arg: &str) -> bool {
    ["|", ">", "<", "&&", "||", ";", "`", "$(", "${"]
        .iter()
        .any(|needle| arg.contains(needle))
        || (arg.contains('*')
            && !arg.ends_with(".yaml")
            && !arg.ends_with(".rs")
            && !arg.starts_with("--"))
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

    #[test]
    fn allows_structured_safe_read_command() {
        let registry = CommandRunnerRegistry::with_defaults();
        let command = StructuredCommand::new(
            "fixture.echo.pass",
            "echo",
            vec!["hello".into()],
            CommandRisk::SafeRead,
            CommandApprovalMode::Never,
        );
        let report = evaluate_structured_command(&command, &registry);
        assert_eq!(report.decision, CommandGateDecision::Allow);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn rejects_shell_inline_execution() {
        let registry = CommandRunnerRegistry::with_defaults();
        let command = StructuredCommand::new(
            "bad.shell",
            "bash",
            vec!["-c".into(), "cargo test | tee out.log".into()],
            CommandRisk::ExecuteProcess,
            CommandApprovalMode::Policy,
        );
        let report = evaluate_structured_command(&command, &registry);
        assert_eq!(report.decision, CommandGateDecision::Deny);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "command.shell.inline")
        );
    }

    #[test]
    fn rejects_runner_path() {
        let registry = CommandRunnerRegistry::with_defaults();
        let command = StructuredCommand::new(
            "bad.path",
            "/usr/bin/cargo",
            vec!["test".into()],
            CommandRisk::ExecuteProcess,
            CommandApprovalMode::Policy,
        );
        let report = evaluate_structured_command(&command, &registry);
        assert_eq!(report.decision, CommandGateDecision::Deny);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "command.runner.path")
        );
    }

    #[test]
    fn rejects_shell_metacharacters_in_args() {
        let registry = CommandRunnerRegistry::with_defaults();
        let command = StructuredCommand::new(
            "bad.pipe",
            "cargo",
            vec!["test".into(), "|".into(), "tee".into()],
            CommandRisk::ExecuteProcess,
            CommandApprovalMode::Policy,
        );
        let report = evaluate_structured_command(&command, &registry);
        assert_eq!(report.decision, CommandGateDecision::Deny);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "command.arg.shell_meta")
        );
    }

    #[test]
    fn execute_process_policy_requires_approval() {
        let registry = CommandRunnerRegistry::with_defaults();
        let command = StructuredCommand::new(
            "cargo.test.registry",
            "cargo",
            vec!["test".into(), "registry".into()],
            CommandRisk::ExecuteProcess,
            CommandApprovalMode::Policy,
        );
        let report = evaluate_structured_command(&command, &registry);
        assert_eq!(report.decision, CommandGateDecision::ApprovalRequired);
    }

    #[test]
    fn shell_script_path_is_allowed_when_no_inline_shell() {
        let registry = CommandRunnerRegistry::with_defaults();
        let command = StructuredCommand::new(
            "script.check",
            "bash",
            vec!["scripts/check.sh".into()],
            CommandRisk::SafeRead,
            CommandApprovalMode::Never,
        );
        let report = evaluate_structured_command(&command, &registry);
        assert_eq!(report.decision, CommandGateDecision::Allow);
    }

    #[test]
    fn invalid_id_is_denied() {
        let registry = CommandRunnerRegistry::with_defaults();
        let command = StructuredCommand::new(
            "Bad ID",
            "echo",
            vec!["hello".into()],
            CommandRisk::SafeRead,
            CommandApprovalMode::Never,
        );
        let report = evaluate_structured_command(&command, &registry);
        assert_eq!(report.decision, CommandGateDecision::Deny);
    }
}
