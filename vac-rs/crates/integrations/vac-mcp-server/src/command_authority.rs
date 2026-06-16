use rmcp::{
    model::{CallToolResult, Content},
    schemars,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VacStructuredCommandRequest {
    pub id: String,
    pub runner: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub risk: Option<String>,
    #[serde(default)]
    pub approval: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct VacStructuredCommand {
    pub(crate) id: String,
    pub(crate) runner: String,
    pub(crate) args: Vec<String>,
}

pub(crate) fn parse_vac_structured_command(
    command: &str,
) -> Result<VacStructuredCommand, CallToolResult> {
    if command.trim().is_empty() {
        return Err(vac_command_error("empty command"));
    }
    if command.contains('\n') || command.contains('\r') {
        return Err(vac_command_error(
            "multi-line shell commands are not a structured command",
        ));
    }
    let mut parts = Vec::new();
    for raw in command.split_whitespace() {
        if raw.contains('"') || raw.contains('\'') || contains_shell_metachar(raw) {
            return Err(vac_command_error(&format!(
                "shell syntax/metacharacter is not allowed in structured command token: {raw}"
            )));
        }
        parts.push(raw.to_string());
    }
    let Some(runner) = parts.first().cloned() else {
        return Err(vac_command_error("missing runner"));
    };
    if runner.contains('/')
        || runner.contains('\\')
        || matches!(
            runner.as_str(),
            "sh" | "bash" | "zsh" | "fish" | "cmd" | "powershell"
        )
    {
        return Err(vac_command_error(
            "runner must be a registry executable name and cannot be a shell/path",
        ));
    }
    Ok(VacStructuredCommand {
        id: "legacy-string-migration-shim".to_string(),
        runner,
        args: parts.into_iter().skip(1).collect(),
    })
}

fn parse_vac_structured_command_object(
    request: &VacStructuredCommandRequest,
) -> Result<VacStructuredCommand, CallToolResult> {
    if request.id.trim().is_empty() {
        return Err(vac_command_error("structured_command.id is required"));
    }
    if request.runner.trim().is_empty() {
        return Err(vac_command_error("structured_command.runner is required"));
    }
    if request.runner.contains('/')
        || request.runner.contains('\\')
        || matches!(
            request.runner.as_str(),
            "sh" | "bash" | "zsh" | "fish" | "cmd" | "powershell"
        )
    {
        return Err(vac_command_error(
            "structured_command.runner must be registry executable name, not shell/path",
        ));
    }
    for arg in &request.args {
        if arg.contains('"')
            || arg.contains('\'')
            || arg.contains('\n')
            || arg.contains('\r')
            || contains_shell_metachar(arg)
        {
            return Err(vac_command_error(&format!(
                "structured_command arg contains shell syntax/metacharacter: {arg}"
            )));
        }
    }
    Ok(VacStructuredCommand {
        id: request.id.clone(),
        runner: request.runner.clone(),
        args: request.args.clone(),
    })
}

pub(crate) fn resolve_vac_structured_command_authority(
    command: &str,
    structured_command: &Option<VacStructuredCommandRequest>,
) -> Result<VacStructuredCommand, CallToolResult> {
    let Some(request) = structured_command else {
        return Err(vac_command_error(
            "free-form command strings are migration mirrors only; structured_command object is required",
        ));
    };
    let structured = parse_vac_structured_command_object(request)?;
    let normalized = normalized_vac_structured_command_parts(&structured);
    if !command.trim().is_empty() && command.trim() != normalized {
        return Err(vac_command_error(&format!(
            "command mirror does not match structured_command {}: expected `{}`",
            structured.id, normalized
        )));
    }
    Ok(structured)
}

pub(crate) fn normalized_vac_structured_command_parts(command: &VacStructuredCommand) -> String {
    let mut tokens = Vec::with_capacity(command.args.len() + 1);
    tokens.push(command.runner.clone());
    tokens.extend(command.args.clone());
    tokens.join(" ")
}

fn vac_command_error(detail: &str) -> CallToolResult {
    CallToolResult::error(vec![
        Content::text("VAC_STRUCTURED_COMMAND_REQUIRED"),
        Content::text(format!(
            "VAC v1.5 requires structured runner+args; free-form shell is blocked: {detail}"
        )),
    ])
}

fn contains_shell_metachar(token: &str) -> bool {
    token.chars().any(|ch| {
        matches!(
            ch,
            '|' | '>' | '<' | ';' | '&' | '`' | '$' | '*' | '?' | '~'
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(id: &str, runner: &str, args: &[&str]) -> VacStructuredCommandRequest {
        VacStructuredCommandRequest {
            id: id.to_string(),
            runner: runner.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            risk: None,
            approval: None,
        }
    }

    #[test]
    fn free_form_command_without_structured_object_is_rejected() {
        let result = resolve_vac_structured_command_authority("cargo check", &None);

        assert!(result.is_err());
    }

    #[test]
    fn structured_command_mirror_mismatch_is_rejected() {
        let structured = Some(request("cargo.check", "cargo", &["check"]));

        let result = resolve_vac_structured_command_authority("cargo test", &structured);

        assert!(result.is_err());
    }

    #[test]
    fn structured_command_rejects_path_and_shell_runners() {
        for runner in ["/bin/echo", "bash", "powershell"] {
            let structured = Some(request("bad.runner", runner, &["-c", "echo"]));

            let result = resolve_vac_structured_command_authority("", &structured);

            assert!(result.is_err(), "runner {runner:?} should be rejected");
        }
    }

    #[test]
    fn structured_command_rejects_shell_metachar_arguments() {
        for arg in ["target/*", "foo;bar", "$HOME"] {
            let structured = Some(request("bad.arg", "cargo", &[arg]));

            let result = resolve_vac_structured_command_authority("", &structured);

            assert!(result.is_err(), "arg {arg:?} should be rejected");
        }
    }

    #[test]
    fn structured_command_accepts_normalized_mirror() {
        let structured = Some(request(
            "cargo.check.mcp",
            "cargo",
            &["check", "-p", "vac-mcp-server"],
        ));

        let command =
            resolve_vac_structured_command_authority("cargo check -p vac-mcp-server", &structured)
                .expect("structured command should be accepted");

        assert_eq!(
            normalized_vac_structured_command_parts(&command),
            "cargo check -p vac-mcp-server"
        );
    }
}
