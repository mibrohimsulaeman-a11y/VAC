//! Legacy shell risk classifier retained only for fixture/back-compat callers. Use structured command + vac-policy + BoundRuntimeController.
//!
//! VAC Runtime v1.5 command authority lives in the structured command parser,
//! compiled policy snapshot, and bound runtime pre-command gate.  This module is
//! intentionally labelled as heuristic so it cannot be mistaken for the product
//! command gate or L2 enforcement boundary.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellRisk {
    SafeRead,
    FileMutation,
    Destructive,
    Network,
    Credential,
}

pub const CLASSIFIER_AUTHORITY: &str = "legacy_heuristic_fixture_only_not_runtime_authority";

pub fn is_legacy_heuristic_only() -> bool {
    true
}

pub fn classify_shell_command(command: &str) -> ShellRisk {
    let normalized = command.trim().to_lowercase();
    if normalized.contains("rm -rf")
        || normalized.contains("rm -fr")
        || normalized.starts_with("rm -r ")
        || normalized.starts_with("sudo rm")
    {
        return ShellRisk::Destructive;
    }
    if normalized.contains("curl ") || normalized.contains("wget ") || normalized.contains("nc ") {
        return ShellRisk::Network;
    }
    if normalized.contains("cat ~/.ssh")
        || normalized.contains("printenv") && normalized.contains("key")
    {
        return ShellRisk::Credential;
    }
    if normalized.starts_with("mv ")
        || normalized.starts_with("cp ")
        || normalized.starts_with("chmod ")
        || normalized.starts_with("chown ")
        || normalized.contains('>')
    {
        return ShellRisk::FileMutation;
    }
    ShellRisk::SafeRead
}

pub fn requires_explicit_approval(command: &str) -> bool {
    matches!(
        classify_shell_command(command),
        ShellRisk::Destructive | ShellRisk::Credential | ShellRisk::Network
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destructive_delete_requires_explicit_approval_in_legacy_fixture_classifier() {
        assert!(is_legacy_heuristic_only());
        assert_eq!(
            classify_shell_command("rm -rf target/debug/incremental"),
            ShellRisk::Destructive
        );
        assert!(requires_explicit_approval(
            "rm -rf target/debug/incremental"
        ));
    }
}
