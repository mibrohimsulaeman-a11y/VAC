use crate::approval_boundary::require_vac_bound_approval;
pub use crate::approval_boundary::{VacBoundApproval, VacSignatureHint};
pub use crate::command_authority::VacStructuredCommandRequest;
use crate::command_authority::{
    VacStructuredCommand, normalized_vac_structured_command_parts, parse_vac_structured_command,
    resolve_vac_structured_command_authority,
};
use crate::file_operations::{
    ViewOptions, create_local, create_remote, remove_local_path, remove_remote_path,
    str_replace_local, str_replace_remote, view_local_path, view_remote_path,
};
pub use crate::read_authorization::VacReadPlanTicket;
use crate::read_authorization::require_vac_view_governance;
use crate::remote_authority::{
    is_remote_path, remote_connection_error, resolve_remote_path_authority,
    validate_remote_connection,
};
use crate::tool_container::ToolContainer;
use rmcp::service::RequestContext;
use rmcp::{ErrorData as McpError, handler::server::wrapper::Parameters, model::*, schemars, tool};
use rmcp::{RoleServer, tool_router};
use serde::{Deserialize, Deserializer};
use vac_foundation::remote_connection::{RemoteConnection, RemoteConnectionInfo};

use html2md;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde_json::json;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::{Duration, sleep, timeout as tokio_timeout};
use tracing::error;
use url;
use uuid::Uuid;
use vac_foundation::models::async_manifest::{AsyncManifest, PendingToolCall};
use vac_foundation::models::integrations::mcp::CallToolResultExt;
use vac_foundation::models::integrations::openai::{
    ProgressType, TaskPauseInfo, TaskUpdate, ToolCallResultProgress,
};
use vac_foundation::task_manager::{StartTaskOptions, TaskInfo};
use vac_foundation::tls_client::{TlsClientConfig, create_tls_client};
use vac_foundation::utils::{handle_large_output, sanitize_text_output};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunCommandRequest {
    #[schemars(
        description = "Deprecated mirror string. Runtime authority is structured_command; free-form shell text is rejected."
    )]
    pub command: String,
    #[serde(default)]
    #[schemars(
        description = "VAC v1.9 structured command authority: {id, runner, args, risk, approval}"
    )]
    pub structured_command: Option<VacStructuredCommandRequest>,
    #[schemars(description = "Optional description of the command to execute")]
    pub description: Option<String>,
    #[schemars(description = "Optional timeout for the command execution in seconds")]
    pub timeout: Option<u64>,
    #[serde(default)]
    #[schemars(
        description = "VAC v1.9 bound-runtime proof stamped by BoundRuntimeToolBoundary; mutating/process tools fail closed without it"
    )]
    pub vac_bound_approval: Option<VacBoundApproval>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunRemoteCommandRequest {
    #[schemars(
        description = "Deprecated mirror string. Runtime authority is structured_command; free-form shell text is rejected."
    )]
    pub command: String,
    #[serde(default)]
    #[schemars(
        description = "VAC v1.9 structured command authority: {id, runner, args, risk, approval}"
    )]
    pub structured_command: Option<VacStructuredCommandRequest>,
    #[schemars(description = "Optional description of the command to execute")]
    pub description: Option<String>,
    #[schemars(description = "Optional timeout for the command execution in seconds")]
    pub timeout: Option<u64>,
    #[schemars(description = "Remote connection string (format: user@host or user@host:port)")]
    pub remote: String,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_nonempty_preserved_string"
    )]
    #[schemars(description = "Optional password for remote connection")]
    pub password: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_nonempty_trimmed_string"
    )]
    #[schemars(description = "Optional path to private key for remote connection")]
    pub private_key_path: Option<String>,
    #[serde(default)]
    #[schemars(
        description = "VAC v1.9 bound-runtime proof stamped by BoundRuntimeToolBoundary; mutating/process tools fail closed without it"
    )]
    pub vac_bound_approval: Option<VacBoundApproval>,
}

#[derive(Debug)]
pub struct CommandResult {
    pub output: String,
    pub exit_code: i32,
}

fn is_loopback_http_url(parsed_url: &url::Url) -> bool {
    if parsed_url.scheme() != "http" {
        return false;
    }
    let Some(host) = parsed_url.host_str() else {
        return false;
    };
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    let normalized_host = host.trim_matches(|ch| ch == '[' || ch == ']');
    normalized_host
        .parse::<std::net::IpAddr>()
        .is_ok_and(|addr| addr.is_loopback())
}

fn deserialize_optional_nonempty_trimmed_string<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    Ok(value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }))
}

fn deserialize_optional_nonempty_preserved_string<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    Ok(value.and_then(|raw| {
        if raw.trim().is_empty() {
            None
        } else {
            Some(raw)
        }
    }))
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TaskStatusRequest {
    #[schemars(description = "The task ID to get status for")]
    pub task_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTaskDetailsRequest {
    #[schemars(description = "The task ID to get details for")]
    pub task_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetAllTasksRequest {
    #[schemars(description = "View parameter (required for compatibility, any value works)")]
    pub view: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AwaitTasksRequest {
    #[schemars(description = "Space-separated list of task IDs to wait for completion")]
    pub task_ids: String,
    #[schemars(description = "Optional timeout in seconds. If not specified, waits indefinitely")]
    pub timeout: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ViewRequest {
    #[schemars(
        description = "The path to the file or directory to view. For remote files, use format: user@host:/path or user@host#port:/path (use ABSOLUTE paths for remote files)"
    )]
    pub path: String,
    #[schemars(
        description = "Optional line range to view [start_line, end_line]. Line numbers are 1-indexed. Use -1 for end_line to read to end of file."
    )]
    pub view_range: Option<[i32; 2]>,
    #[schemars(
        description = "Regex pattern to search for in file contents. Returns matching lines with line numbers. For directories, searches all files recursively (respects .gitignore)."
    )]
    pub grep: Option<String>,
    #[schemars(
        description = "Glob pattern to filter files when viewing directories (e.g., '*.rs', '**/*.ts', 'src/**/*.go'). Only applies to directory views."
    )]
    pub glob: Option<String>,
    #[schemars(description = "Optional password for remote connection (if path is remote)")]
    pub password: Option<String>,
    #[schemars(
        description = "Optional path to private key for remote connection (if path is remote)"
    )]
    pub private_key_path: Option<String>,
    #[serde(default)]
    #[schemars(
        description = "VAC deterministic read-plan ticket generated from .vac/index/read_plans.jsonl; local reads fail closed without this or vac_bound_approval"
    )]
    pub read_plan_ticket: Option<VacReadPlanTicket>,
    #[serde(default)]
    #[schemars(
        description = "VAC v1.9 bound-runtime proof stamped by BoundRuntimeToolBoundary; remote/credential reads require it"
    )]
    pub vac_bound_approval: Option<VacBoundApproval>,
    #[schemars(description = "Display directory as a nested tree structure (default: false)")]
    pub tree: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StrReplaceRequest {
    #[schemars(
        description = "The path to the file to modify. For remote files, use format: user@host:/path or user@host#port:/path (use ABSOLUTE paths for remote files)"
    )]
    pub path: String,
    #[schemars(
        description = "The exact text to replace (must match exactly, including whitespace and indentation)"
    )]
    pub old_str: String,
    #[schemars(
        description = "The new text to insert in place of the old text. When replacing code, ensure the new text maintains proper syntax, indentation, and follows the codebase style."
    )]
    pub new_str: String,
    #[schemars(
        description = "Whether to replace all occurrences of the old text in the file (default: false)"
    )]
    pub replace_all: Option<bool>,
    #[schemars(description = "Optional password for remote connection (if path is remote)")]
    pub password: Option<String>,
    #[schemars(
        description = "Optional path to private key for remote connection (if path is remote)"
    )]
    pub private_key_path: Option<String>,
    #[serde(default)]
    #[schemars(
        description = "VAC v1.9 bound-runtime proof stamped by BoundRuntimeToolBoundary; file mutations fail closed without it"
    )]
    pub vac_bound_approval: Option<VacBoundApproval>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateRequest {
    #[schemars(
        description = "The path where the new file should be created. For remote files, use format: user@host:/path or user@host#port:/path (use ABSOLUTE paths for remote files)"
    )]
    pub path: String,
    #[schemars(
        description = "The content to write to the new file, when creating code, ensure the new text has proper syntax, indentation, and follows the codebase style."
    )]
    pub file_text: String,
    #[schemars(description = "Optional password for remote connection (if path is remote)")]
    pub password: Option<String>,
    #[schemars(
        description = "Optional path to private key for remote connection (if path is remote)"
    )]
    pub private_key_path: Option<String>,
    #[serde(default)]
    #[schemars(
        description = "VAC v1.9 bound-runtime proof stamped by BoundRuntimeToolBoundary; file creation fails closed without it"
    )]
    pub vac_bound_approval: Option<VacBoundApproval>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GeneratePasswordRequest {
    #[schemars(description = "The length of the password to generate")]
    pub length: Option<usize>,
    #[schemars(description = "Whether to disallow symbols in the password (default: false)")]
    pub no_symbols: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RemoveRequest {
    #[schemars(
        description = "The path to the file or directory to remove. For remote files, use format: user@host:/path or user@host#port:/path (use ABSOLUTE paths for remote files)"
    )]
    pub path: String,
    #[schemars(
        description = "Whether to remove directories recursively (required for non-empty directories, default: false)"
    )]
    pub recursive: Option<bool>,
    #[schemars(description = "Optional password for remote connection (if path is remote)")]
    pub password: Option<String>,
    #[schemars(
        description = "Optional path to private key for remote connection (if path is remote)"
    )]
    pub private_key_path: Option<String>,
    #[serde(default)]
    #[schemars(
        description = "VAC v1.9 bound-runtime proof stamped by BoundRuntimeToolBoundary; remove operations fail closed without it"
    )]
    pub vac_bound_approval: Option<VacBoundApproval>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ViewWebPageRequest {
    #[schemars(description = "The HTTPS URL of the web page to fetch and convert to markdown")]
    pub url: String,
    #[serde(default)]
    #[schemars(
        description = "VAC v1.9 bound-runtime proof stamped after network_access policy evaluation; network reads fail closed without it"
    )]
    pub vac_bound_approval: Option<VacBoundApproval>,
}

use vac_foundation::models::tools::ask_user::AskUserRequest;

#[tool_router(router = tool_router_local, vis = "pub")]
impl ToolContainer {
    #[tool(
        description = "Execute a VAC-bound structured command locally. Free-form shell execution is blocked; the call must carry vac_bound_approval from the bound runtime.

If the command's output exceeds 300 lines the result will be truncated and the full output will be saved to a file in the current directory.

For remote command execution via SSH, use the run_remote_command tool instead."
    )]
    pub async fn run_command(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(RunCommandRequest {
            command,
            structured_command,
            description,
            timeout,
            vac_bound_approval,
        }): Parameters<RunCommandRequest>,
    ) -> Result<CallToolResult, McpError> {
        let actual_arguments = json!({
            "command": &command,
            "structured_command": &structured_command,
            "description": &description,
            "timeout": &timeout,
        });
        if let Err(error_result) = require_vac_bound_approval(
            &vac_bound_approval,
            "execute_process",
            &command,
            &actual_arguments,
        ) {
            return Ok(error_result);
        }
        let command = match resolve_vac_structured_command_authority(&command, &structured_command)
        {
            Ok(command) => command,
            Err(error_result) => return Ok(error_result),
        };
        match self.execute_local_command(&command, timeout, &ctx).await {
            Ok(mut command_result) => Self::format_command_result(&mut command_result),
            Err(error_result) => Ok(error_result),
        }
    }

    #[tool(
        description = "Execute a VAC-bound structured command on a remote system via SSH.

REMOTE EXECUTION:
- Set 'remote' parameter to 'user@host' or 'user@host:port'
- Use 'password' for password authentication or 'private_key_path' for key-based auth
- Automatic SSH key discovery from ~/.ssh/ (id_ed25519, id_rsa, etc.) if no credentials provided
- Examples:
  * 'user@server.com' (uses default port 22 and auto-discovered keys)
  * 'user@server.com:2222' with password authentication

If the command's output exceeds 300 lines the result will be truncated and the full output will be saved to a file in the current directory.

For local command execution, use the run_command tool instead."
    )]
    pub async fn run_remote_command(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(RunRemoteCommandRequest {
            command,
            structured_command,
            description,
            timeout,
            remote,
            password,
            private_key_path,
            vac_bound_approval,
        }): Parameters<RunRemoteCommandRequest>,
    ) -> Result<CallToolResult, McpError> {
        let actual_arguments = json!({
            "command": &command,
            "structured_command": &structured_command,
            "description": &description,
            "timeout": &timeout,
            "remote": &remote,
            "password": &password,
            "private_key_path": &private_key_path,
        });
        if let Err(error_result) = require_vac_bound_approval(
            &vac_bound_approval,
            "remote_execute_process",
            &command,
            &actual_arguments,
        ) {
            return Ok(error_result);
        }
        let command = match resolve_vac_structured_command_authority(&command, &structured_command)
        {
            Ok(command) => normalized_vac_structured_command_parts(&command),
            Err(error_result) => return Ok(error_result),
        };
        let remote = match validate_remote_connection(&remote) {
            Ok(r) => r,
            Err(err) => return Ok(err),
        };

        match self
            .execute_command_unified(
                &command,
                timeout,
                Some(remote),
                password,
                private_key_path,
                &ctx,
            )
            .await
        {
            Ok(mut command_result) => Self::format_command_result(&mut command_result),
            Err(error_result) => Ok(error_result),
        }
    }

    #[tool(
        description = "Execute a VAC-bound local structured command asynchronously in the background and return immediately with task information without waiting for completion.

Use this for starting servers, tailing logs, or other long-running commands that you want to monitor separately, or whenever the user wants to run a command in the background.

For remote background tasks via SSH, use the run_remote_command_task tool instead.

RETURNS:
- task_id: Unique identifier for the background task
- status: Current task status (will be 'Running' initially)
- start_time: When the task was started

Use the get_all_tasks tool to monitor task progress, or the cancel_task tool to cancel a task."
    )]
    pub async fn run_command_task(
        &self,
        _ctx: RequestContext<RoleServer>,
        Parameters(RunCommandRequest {
            command,
            structured_command,
            description,
            timeout,
            vac_bound_approval,
        }): Parameters<RunCommandRequest>,
    ) -> Result<CallToolResult, McpError> {
        let actual_arguments = json!({
            "command": &command,
            "structured_command": &structured_command,
            "description": &description,
            "timeout": &timeout,
        });
        if let Err(error_result) = require_vac_bound_approval(
            &vac_bound_approval,
            "execute_process_task",
            &command,
            &actual_arguments,
        ) {
            return Ok(error_result);
        }
        let command = match resolve_vac_structured_command_authority(&command, &structured_command)
        {
            Ok(command) => normalized_vac_structured_command_parts(&command),
            Err(error_result) => return Ok(error_result),
        };
        let timeout_duration = timeout.map(std::time::Duration::from_secs);

        let result = self
            .get_task_manager()
            .start_task(
                command,
                StartTaskOptions {
                    description,
                    timeout: timeout_duration,
                    remote_connection: None,
                    child_env: self.task_child_env_defaults(None),
                },
            )
            .await;

        Self::format_task_result(result)
    }

    #[tool(
        description = "Execute a VAC-bound structured command asynchronously in the background on a remote system via SSH and return immediately with task information without waiting for completion.

REMOTE EXECUTION:
- Set 'remote' parameter to 'user@host' or 'user@host:port'
- Use 'password' for password authentication or 'private_key_path' for key-based auth
- Automatic SSH key discovery from ~/.ssh/ if no credentials provided
- Examples:
  * 'user@server.com' - Remote background task with auto-discovered keys
  * 'user@server.com:2222' - Remote background task with custom port

Use this for port-forwarding, starting servers, tailing logs, or other long-running remote commands.

For local background tasks, use the run_command_task tool instead.

RETURNS:
- task_id: Unique identifier for the background task
- status: Current task status (will be 'Running' initially)
- start_time: When the task was started

Use the get_all_tasks tool to monitor task progress, or the cancel_task tool to cancel a task."
    )]
    pub async fn run_remote_command_task(
        &self,
        _ctx: RequestContext<RoleServer>,
        Parameters(RunRemoteCommandRequest {
            command,
            structured_command,
            description,
            timeout,
            remote,
            password,
            private_key_path,
            vac_bound_approval,
        }): Parameters<RunRemoteCommandRequest>,
    ) -> Result<CallToolResult, McpError> {
        let actual_arguments = json!({
            "command": &command,
            "structured_command": &structured_command,
            "description": &description,
            "timeout": &timeout,
            "remote": &remote,
            "password": &password,
            "private_key_path": &private_key_path,
        });
        if let Err(error_result) = require_vac_bound_approval(
            &vac_bound_approval,
            "remote_execute_process_task",
            &command,
            &actual_arguments,
        ) {
            return Ok(error_result);
        }
        let command = match resolve_vac_structured_command_authority(&command, &structured_command)
        {
            Ok(command) => normalized_vac_structured_command_parts(&command),
            Err(error_result) => return Ok(error_result),
        };
        let remote = match validate_remote_connection(&remote) {
            Ok(r) => r,
            Err(err) => return Ok(err),
        };

        let timeout_duration = timeout.map(std::time::Duration::from_secs);

        let remote_connection = RemoteConnectionInfo {
            connection_string: remote,
            password,
            private_key_path,
        };

        let child_env = self.task_child_env_defaults(Some(&remote_connection));
        let result = self
            .get_task_manager()
            .start_task(
                command,
                StartTaskOptions {
                    description,
                    timeout: timeout_duration,
                    remote_connection: Some(remote_connection),
                    child_env,
                },
            )
            .await;

        Self::format_task_result(result)
    }

    #[tool(
        description = "Get the status of all background tasks started with run_command_task or run_remote_command_task.

RETURNS:
- A markdown-formatted table showing all background tasks with:
  - Task ID: Full unique identifier (required for cancel_task)
  - Status: Current status (Running, Completed, Failed, Cancelled, TimedOut)
  - Start Time: When the task was started
  - Duration: How long the task has been running or took to complete
  - Output: Command output preview (truncated to 80 chars)

This tool provides a clean tabular overview of all background tasks and their current state.
Use the full Task ID from this output with cancel_task to cancel specific tasks."
    )]
    pub async fn get_all_tasks(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(GetAllTasksRequest { view: _ }): Parameters<GetAllTasksRequest>,
    ) -> Result<CallToolResult, McpError> {
        match self.get_task_manager().get_all_tasks().await {
            Ok(tasks) => {
                if tasks.is_empty() {
                    return Ok(CallToolResult::success(vec![Content::text(
                        "No background tasks found.",
                    )]));
                }

                // Send progress notifications for any paused tasks so the TUI
                // caches their pause_info for the approval display.
                let paused_updates: Vec<TaskUpdate> = tasks
                    .iter()
                    .filter(|t| {
                        matches!(t.status, vac_foundation::task_manager::TaskStatus::Paused)
                    })
                    .map(Self::build_task_update)
                    .collect();

                if !paused_updates.is_empty() {
                    let progress_id = uuid::Uuid::new_v4();
                    let _ = ctx
                        .peer
                        .notify_progress(ProgressNotificationParam {
                            progress_token: ProgressToken(NumberOrString::Number(0)),
                            progress: 100.0,
                            total: Some(100.0),
                            message: Some(
                                serde_json::to_string(&ToolCallResultProgress {
                                    id: progress_id,
                                    message: String::new(),
                                    progress_type: Some(ProgressType::TaskWait),
                                    task_updates: Some(paused_updates),
                                    progress: Some(100.0),
                                })
                                .unwrap_or_default(),
                            ),
                        })
                        .await;
                }

                // Create markdown table format
                let mut table = String::new();
                table.push_str("# Background Tasks\n\n");

                // Markdown table header
                table.push_str("| Task ID | Status | Command | Start Time | Duration | Output |\n");
                table.push_str("|---------|--------|------------|----------|--------|--------|\n");

                // Markdown table rows
                for task in &tasks {
                    let task_id = task.id.clone();
                    let status = format!("{:?}", task.status);
                    let start_time = task.start_time.to_rfc3339();
                    let duration = if let Some(duration) = task.duration {
                        format!("{:.2}s", duration.as_secs_f64())
                    } else {
                        "N/A".to_string()
                    };

                    let output_str = if let Some(ref out) = task.output {
                        out.clone()
                    } else {
                        "No output yet".to_string()
                    };

                    let escaped_command = task
                        .command
                        .chars()
                        .take(100)
                        .collect::<String>()
                        .replace('|', "\\|")
                        .replace('\n', " ");
                    let escaped_output = output_str
                        .chars()
                        .take(100)
                        .collect::<String>()
                        .replace('|', "\\|")
                        .replace('\n', " ");

                    table.push_str(&format!(
                        "| {} | {} | {} | {} | {} | {} |\n",
                        task_id, status, escaped_command, start_time, duration, escaped_output
                    ));
                }

                table.push_str(&format!("\n**Total: {} task(s)**", tasks.len()));

                Ok(CallToolResult::success(vec![Content::text(table)]))
            }
            Err(e) => {
                error!("Failed to get all tasks: {}", e);

                Ok(CallToolResult::error(vec![
                    Content::text("GET_ALL_TASKS_ERROR"),
                    Content::text(format!("Failed to get all tasks: {}", e)),
                ]))
            }
        }
    }

    #[tool(
        description = "Cancel a running asynchronous background task started with run_command_task or run_remote_command_task.

PARAMETERS:
- task_id: The unique identifier of the task to cancel. Use the get_all_tasks tool to get the task ID.

This will immediately terminate the background task and update the task status to 'Cancelled'.
The task will be removed from the active tasks list."
    )]
    pub async fn cancel_task(
        &self,
        _ctx: RequestContext<RoleServer>,
        Parameters(TaskStatusRequest { task_id }): Parameters<TaskStatusRequest>,
    ) -> Result<CallToolResult, McpError> {
        match self.get_task_manager().cancel_task(task_id.clone()).await {
            Ok(task_info) => {
                let output = serde_json::to_string_pretty(&task_info)
                    .unwrap_or_else(|_| format!("Task cancelled: {}", task_info.id));

                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Task cancelled:\n{}",
                    output
                ))]))
            }
            Err(e) => {
                error!("Failed to cancel task: {}", e);

                Ok(CallToolResult::error(vec![
                    Content::text("CANCEL_TASK_ERROR"),
                    Content::text(format!("Failed to cancel task: {}", e)),
                ]))
            }
        }
    }

    #[tool(
        description = "Wait for one or more background tasks to complete or fail, then return the status of all tasks.

PARAMETERS:
- task_ids: Space-separated list of task IDs to wait for completion (e.g., \"abc123 def456 ghi789\")
- timeout: Optional timeout in seconds. If not specified, waits indefinitely

BEHAVIOR:
- Waits until ALL specified tasks reach a final state (Completed, Failed, Cancelled, or TimedOut)
- If timeout is specified, returns an error if tasks don't complete within that time
- Returns the same format as get_all_tasks showing all background tasks after waiting
- If any task ID doesn't exist, returns an error immediately
- This is useful for coordinating async tasks and getting results once they're done

EXAMPLE USAGE:
1. Start multiple async tasks with run_command_task
2. Use wait_for_tasks with those IDs to wait for completion
3. Process the results from all tasks

This tool enables proper task synchronization and coordination in complex workflows."
    )]
    pub async fn wait_for_tasks(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(AwaitTasksRequest { task_ids, timeout }): Parameters<AwaitTasksRequest>,
    ) -> Result<CallToolResult, McpError> {
        let task_ids: Vec<String> = task_ids.split_whitespace().map(|s| s.to_string()).collect();

        if task_ids.is_empty() {
            return Ok(CallToolResult::error(vec![
                Content::text("AWAIT_TASKS_ERROR"),
                Content::text(
                    "No task IDs provided. Please provide a space-separated list of task IDs.",
                ),
            ]));
        }

        let timeout = timeout.map(std::time::Duration::from_secs);

        match self
            .wait_for_tasks_with_streaming(&task_ids, timeout, &ctx)
            .await
        {
            Ok(tasks) => {
                let table = self.format_tasks_table(&tasks, &task_ids);

                Ok(CallToolResult::success(vec![Content::text(table)]))
            }
            Err(e) => {
                error!("Failed to await tasks: {}", e);

                Ok(CallToolResult::error(vec![
                    Content::text("AWAIT_TASKS_ERROR"),
                    Content::text(format!("Failed to await tasks: {}", e)),
                ]))
            }
        }
    }

    #[tool(
        description = "Get detailed information about a specific background task by its ID.

This tool provides comprehensive details about a background task started with run_command_task, including:
- Current status (Running, Completed, Failed, Cancelled, TimedOut, Pending)
- Task ID and start time
- Duration (elapsed time for running tasks, total time for completed tasks)
- Complete command output
- Error information if the task failed

If the task output exceeds 300 lines the result will be truncated and the full output will be saved to a file in the current directory.

Use this tool to check the progress and results of long-running background tasks."
    )]
    pub async fn get_task_details(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(GetTaskDetailsRequest { task_id }): Parameters<GetTaskDetailsRequest>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .get_task_manager()
            .get_task_details(task_id.clone())
            .await
        {
            Ok(Some(task_info)) => {
                let duration_str = if let Some(duration) = task_info.duration {
                    format!("{:.2}s", duration.as_secs_f64())
                } else {
                    "N/A".to_string()
                };

                // If the task is paused, send a progress notification so the TUI
                // caches the pause_info (pending tool calls) for the approval display.
                // Without this, only wait_for_tasks populates the cache and
                // resume_subagent_task shows a generic "Resume subagent task" message.
                if matches!(
                    task_info.status,
                    vac_foundation::task_manager::TaskStatus::Paused
                ) {
                    let task_update = Self::build_task_update(&task_info);
                    let progress_id = uuid::Uuid::new_v4();
                    let _ = ctx
                        .peer
                        .notify_progress(ProgressNotificationParam {
                            progress_token: ProgressToken(NumberOrString::Number(0)),
                            progress: 100.0,
                            total: Some(100.0),
                            message: Some(
                                serde_json::to_string(&ToolCallResultProgress {
                                    id: progress_id,
                                    message: String::new(),
                                    progress_type: Some(ProgressType::TaskWait),
                                    task_updates: Some(vec![task_update]),
                                    progress: Some(100.0),
                                })
                                .unwrap_or_default(),
                            ),
                        })
                        .await;
                }

                // Try to parse output as AsyncManifest (subagent JSON output)
                // If successful, format it in a human/LLM-friendly way
                let output_str = if let Some(ref output) = task_info.output {
                    if let Some(manifest) = AsyncManifest::try_parse(output) {
                        // Subagent output - use Display impl for LLM-friendly formatting
                        manifest.to_string()
                    } else {
                        // Regular task output - use standard handling
                        match handle_large_output(output, "task.output", 300, false) {
                            Ok(result) => result,
                            Err(e) => {
                                return Ok(CallToolResult::error(vec![
                                    Content::text("OUTPUT_HANDLING_ERROR"),
                                    Content::text(format!("Failed to handle task output: {}", e)),
                                ]));
                            }
                        }
                    }
                } else {
                    "No output available".to_string()
                };

                let output = format!(
                    "# Task Details: {}\n\nStatus: {:?}\nTask ID: {}\nStarted: {}\nDuration: {}\n\n## Output:\n{}",
                    task_info.id,
                    task_info.status,
                    task_info.id,
                    task_info.start_time.format("%Y-%m-%d %H:%M:%S UTC"),
                    duration_str,
                    output_str
                );

                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Ok(None) => Ok(CallToolResult::error(vec![
                Content::text("TASK_NOT_FOUND"),
                Content::text(format!("Task not found: {}", task_id)),
            ])),
            Err(e) => {
                error!("Failed to get task details: {}", e);

                Ok(CallToolResult::error(vec![
                    Content::text("GET_TASK_DETAILS_ERROR"),
                    Content::text(format!("Failed to get task details: {}", e)),
                ]))
            }
        }
    }

    #[tool(
        description = "View the contents of a local or remote file/directory. Can read entire files or specific line ranges.

REMOTE FILE ACCESS:
- Use path formats: 'user@host:/path' or 'user@host#port:/path' for remote files
- IMPORTANT: Use ABSOLUTE paths for remote files/directories (e.g., '/etc/config' not 'config')
- Use 'password' for password authentication or 'private_key_path' for key-based auth
- Automatic SSH key discovery from ~/.ssh/ if no credentials provided
- Examples:
  * 'user@server.com:/etc/nginx/nginx.conf' - Remote file with auto-discovered keys
  * 'ssh://user@server.com/var/log/app.log' - Remote file with SSH URL format
  * 'user@server.com:/home/user/documents' - Remote directory listing
  * '/local/path/file.txt' - Local file (default behavior)

For directories:
- Default behavior: Lists immediate directory contents
- With tree=true: Displays nested directory structure as a tree (limited to 3 levels deep)

GREP (Content Search):
- Use 'grep' parameter with a regex pattern to search file contents
- For files: Returns matching lines with line numbers (format: line_num:content)
- For directories: Recursively searches all files, respects .gitignore (format: file:line_num:content)
- Examples:
  * grep='TODO|FIXME' - Find all TODO/FIXME comments
  * grep='fn\\s+\\w+' - Find Rust function definitions
  * grep='error' - Simple text search

GLOB (File Filtering):
- Use 'glob' parameter to filter files in directories by pattern
- Supports standard glob syntax: *, ?, [abc], **
- Examples:
  * glob='*.rs' - All Rust files
  * glob='**/*.ts' - All TypeScript files (recursive)
  * glob='test_*.py' - Python test files

A maximum of 300 lines will be shown at a time, the rest will be truncated."
    )]
    pub async fn view(
        &self,
        Parameters(ViewRequest {
            path,
            view_range,
            grep,
            glob,
            password,
            private_key_path,
            read_plan_ticket,
            vac_bound_approval,
            tree,
        }): Parameters<ViewRequest>,
    ) -> Result<CallToolResult, McpError> {
        const MAX_LINES: usize = 300;

        let actual_arguments = json!({
            "path": &path,
            "view_range": &view_range,
            "grep": &grep,
            "glob": &glob,
            "password": &password,
            "private_key_path": &private_key_path,
            "read_plan_ticket": &read_plan_ticket,
            "tree": &tree,
        });
        if let Err(error_result) = require_vac_view_governance(
            &vac_bound_approval,
            &read_plan_ticket,
            &path,
            &password,
            &private_key_path,
            &actual_arguments,
        ) {
            return Ok(error_result);
        }

        // Check if this is a remote path
        if is_remote_path(&path) {
            // Handle remote file/directory viewing
            match self
                .get_remote_connection(&path, password, private_key_path)
                .await
            {
                Ok((conn, remote_path)) => {
                    let opts = ViewOptions {
                        view_range,
                        max_lines: MAX_LINES,
                        tree,
                        grep: grep.as_deref(),
                        glob: glob.as_deref(),
                    };
                    view_remote_path(&conn, &remote_path, &path, &opts).await
                }
                Err(error_result) => Ok(error_result),
            }
        } else {
            // Handle local file/directory viewing
            let opts = ViewOptions {
                view_range,
                max_lines: MAX_LINES,
                tree,
                grep: grep.as_deref(),
                glob: glob.as_deref(),
            };
            view_local_path(&path, &opts).await
        }
    }

    #[tool(
        description = "Replace a specific string in a local or remote file with new text. The old_str must match exactly including whitespace and indentation.

REMOTE FILE EDITING:
- Use path formats: 'user@host:/path' or 'user@host#port:/path' for remote files
- IMPORTANT: Use ABSOLUTE paths for remote files (e.g., '/etc/config' not 'config')
- Use 'password' for password authentication or 'private_key_path' for key-based auth
- Automatic SSH key discovery from ~/.ssh/ if no credentials provided
- Examples:
  * 'user@server.com:/etc/nginx/sites-available/default' - Edit remote config
  * 'ssh://user@server.com/var/www/app/config.php' - Edit remote application config
  * '/local/path/file.txt' - Edit local file (default behavior)

When replacing code, ensure the new text maintains proper syntax, indentation, and follows the codebase style."
    )]
    pub async fn str_replace(
        &self,
        Parameters(StrReplaceRequest {
            path,
            old_str,
            new_str,
            replace_all,
            password,
            private_key_path,
            vac_bound_approval,
        }): Parameters<StrReplaceRequest>,
    ) -> Result<CallToolResult, McpError> {
        let actual_arguments = json!({
            "path": &path,
            "old_str": &old_str,
            "new_str": &new_str,
            "replace_all": &replace_all,
            "password": &password,
            "private_key_path": &private_key_path,
        });
        if let Err(error_result) = require_vac_bound_approval(
            &vac_bound_approval,
            "filesystem_write",
            &path,
            &actual_arguments,
        ) {
            return Ok(error_result);
        }
        // Check if this is a remote path
        if is_remote_path(&path) {
            // Handle remote file replacement
            match self
                .get_remote_connection(&path, password, private_key_path)
                .await
            {
                Ok((conn, remote_path)) => {
                    str_replace_remote(&conn, &remote_path, &path, &old_str, &new_str, replace_all)
                        .await
                }
                Err(error_result) => Ok(error_result),
            }
        } else {
            // Handle local file replacement
            str_replace_local(&path, &old_str, &new_str, replace_all).await
        }
    }

    #[tool(
        description = "Create a new local or remote file with the specified content. Will fail if file already exists. When creating code, ensure the new text has proper syntax, indentation, and follows the codebase style. Parent directories will be created automatically if they don't exist.

REMOTE FILE CREATION:
- Use path formats: 'user@host:/path' or 'user@host#port:/path' for remote files
- IMPORTANT: Use ABSOLUTE paths for remote files (e.g., '/tmp/script.sh' not 'script.sh')
- Use 'password' for password authentication or 'private_key_path' for key-based auth
- Automatic SSH key discovery from ~/.ssh/ if no credentials provided
- Parent directories will be created automatically on remote systems
- Examples:
  * 'user@server.com:/tmp/script.sh' - Create remote script
  * 'ssh://user@server.com/var/www/new-config.json' - Create remote config
  * '/local/path/file.txt' - Create local file (default behavior)"
    )]
    pub async fn create(
        &self,
        Parameters(CreateRequest {
            path,
            file_text,
            password,
            private_key_path,
            vac_bound_approval,
        }): Parameters<CreateRequest>,
    ) -> Result<CallToolResult, McpError> {
        let actual_arguments = json!({
            "path": &path,
            "file_text": &file_text,
            "password": &password,
            "private_key_path": &private_key_path,
        });
        // SV marker for second file-mutation path: require_vac_bound_approval(&vac_bound_approval, "filesystem_write"
        if let Err(error_result) = require_vac_bound_approval(
            &vac_bound_approval,
            "filesystem_write",
            &path,
            &actual_arguments,
        ) {
            return Ok(error_result);
        }
        // Check if this is a remote path
        if is_remote_path(&path) {
            // Handle remote file creation
            match self
                .get_remote_connection(&path, password, private_key_path)
                .await
            {
                Ok((conn, remote_path)) => {
                    create_remote(&conn, &remote_path, &path, &file_text).await
                }
                Err(error_result) => Ok(error_result),
            }
        } else {
            // Handle local file creation
            create_local(&path, &file_text)
        }
    }

    #[tool(
        description = "Generate a cryptographically secure password with the specified constraints.

PARAMETERS:
- length: The length of the password to generate (default: 15 characters)
- no_symbols: Whether to exclude symbols from the password (default: false, includes symbols)

CHARACTER SETS:
- Letters: A-Z, a-z (always included)
- Numbers: 0-9 (always included)
- Symbols: !@#$%^&*()_+-=[]{}|;:,.<>? (included unless no_symbols=true)

SECURITY FEATURES:
- Uses cryptographically secure random number generation
"
    )]
    pub async fn generate_password(
        &self,
        Parameters(GeneratePasswordRequest { length, no_symbols }): Parameters<
            GeneratePasswordRequest,
        >,
    ) -> Result<CallToolResult, McpError> {
        let length = length.unwrap_or(15);
        let no_symbols = no_symbols.unwrap_or(false);

        let password = vac_foundation::utils::generate_password(length, no_symbols);

        Ok(CallToolResult::success(vec![Content::text(&password)]))
    }

    #[tool(
        description = "Fetch and view the text content of a web page by converting its HTML to markdown format.

SECURITY FEATURES:
- Only allows HTTPS URLs for secure connections
- Requires VAC bound approval from the compiled network_access policy gate
- Follows redirects safely with limits

The tool fetches the HTML content from the specified URL and converts it to clean, readable markdown. This is useful for reading web articles, documentation, or any web content in a text-friendly format.

The response will be truncated if it exceeds 300 lines, with the full content saved to a local file."
    )]
    pub async fn view_web_page(
        &self,
        _ctx: RequestContext<RoleServer>,
        Parameters(ViewWebPageRequest {
            url,
            vac_bound_approval,
        }): Parameters<ViewWebPageRequest>,
    ) -> Result<CallToolResult, McpError> {
        let actual_arguments = json!({
            "url": &url,
        });
        if let Err(error_result) = require_vac_bound_approval(
            &vac_bound_approval,
            "network_access",
            &url,
            &actual_arguments,
        ) {
            return Ok(error_result);
        }
        let parsed_url = match url::Url::parse(&url) {
            Ok(u) => u,
            Err(e) => {
                return Ok(CallToolResult::error(vec![
                    Content::text("INVALID_URL"),
                    Content::text(format!("Invalid URL format: {}", e)),
                ]));
            }
        };

        if parsed_url.scheme() != "https" && !is_loopback_http_url(&parsed_url) {
            return Ok(CallToolResult::error(vec![
                Content::text("INSECURE_URL"),
                Content::text(
                    "Only HTTPS URLs are allowed, except loopback HTTP URLs used for local VAC-governed tooling.",
                ),
            ]));
        }

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Mozilla/5.0 (compatible; VAC-MCP-Bot/1.0)"),
        );

        let client = match create_tls_client(TlsClientConfig::default().with_headers(headers)) {
            Ok(client) => client,
            Err(e) => {
                error!("Failed to create HTTP client: {}", e);
                return Ok(CallToolResult::error(vec![
                    Content::text("HTTP_CLIENT_ERROR"),
                    Content::text(format!("Failed to create HTTP client: {}", e)),
                ]));
            }
        };

        let response = match client.get(&url).send().await {
            Ok(response) => response,
            Err(e) => {
                error!("Failed to fetch web page: {}", e);
                return Ok(CallToolResult::error(vec![
                    Content::text("FAILED_TO_FETCH_WEB_PAGE"),
                    Content::text(format!("Failed to fetch web page: {}", e)),
                ]));
            }
        };

        if !response.status().is_success() {
            return Ok(CallToolResult::error(vec![
                Content::text("HTTP_ERROR"),
                Content::text(format!(
                    "HTTP request failed with status: {}",
                    response.status()
                )),
            ]));
        }

        let html_bytes = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to read response body: {}", e);
                return Ok(CallToolResult::error(vec![
                    Content::text("RESPONSE_READ_ERROR"),
                    Content::text(format!("Failed to read response body: {}", e)),
                ]));
            }
        };

        let html_content = String::from_utf8_lossy(&html_bytes).to_string();
        let markdown_content = html2md::rewrite_html(&html_content, false);
        let sanitized_content = sanitize_text_output(&markdown_content);

        let result = match handle_large_output(&sanitized_content, "webpage", 300, false) {
            Ok(result) => result,
            Err(e) => {
                return Ok(CallToolResult::error(vec![
                    Content::text("OUTPUT_HANDLING_ERROR"),
                    Content::text(format!("Failed to handle output: {}", e)),
                ]));
            }
        };

        let formatted_output = format!("# Web Page Content: {}\n\n{}", url, result);

        Ok(CallToolResult::success(vec![Content::text(
            &formatted_output,
        )]))
    }

    #[tool(
        description = "Remove/delete a local or remote file or directory. Files are automatically backed up before removal and can be recovered.

REMOTE FILE REMOVAL:
- Supports SSH connections for remote file operations
- Use format: 'user@host:/path' or 'user@host#port:/path'
- IMPORTANT: Use ABSOLUTE paths for remote files (e.g., '/tmp/file.txt' not 'file.txt')
- Use 'password' for password authentication or 'private_key_path' for key-based auth
- Automatic SSH key discovery from ~/.ssh/ if no credentials provided
- Examples:
  * 'user@server.com:/tmp/old-file.txt' - Remove remote file
  * 'user@server.com#2222:/var/log/old-logs/' - Remove remote directory (with recursive=true)
  * '/local/path/file.txt' - Remove local file (default behavior)

DIRECTORY REMOVAL:
- Use 'recursive=true' to remove directories and their contents
- Files can be removed without the recursive flag

BACKUP & RECOVERY:
- ALL removed files and directories are automatically backed up before deletion
- Local files: Moved to '.vac/session/backups/{uuid}/' on the local machine
- Remote files: Moved to '.vac/session/backups/{uuid}/' on the remote machine
- Backup paths are returned in XML format showing original and backup locations
- Files are moved (not copied) to backup location, making removal efficient
- Both files and entire directories can be recovered from backup locations

SAFETY NOTES:
- Files are moved to backup location (not permanently deleted)
- Backup locations are preserved until manually cleaned up
- Use backup paths from XML output to restore files if needed"
    )]
    pub async fn remove(
        &self,
        _ctx: RequestContext<RoleServer>,
        Parameters(RemoveRequest {
            path,
            recursive,
            password,
            private_key_path,
            vac_bound_approval,
        }): Parameters<RemoveRequest>,
    ) -> Result<CallToolResult, McpError> {
        let actual_arguments = json!({
            "path": &path,
            "recursive": &recursive,
            "password": &password,
            "private_key_path": &private_key_path,
        });
        if let Err(error_result) = require_vac_bound_approval(
            &vac_bound_approval,
            "filesystem_delete",
            &path,
            &actual_arguments,
        ) {
            return Ok(error_result);
        }
        let recursive = recursive.unwrap_or(false);

        if is_remote_path(&path) {
            match self
                .get_remote_connection(&path, password, private_key_path)
                .await
            {
                Ok((conn, remote_path)) => {
                    remove_remote_path(&conn, &remote_path, &path, recursive).await
                }
                Err(error_result) => Ok(error_result),
            }
        } else {
            remove_local_path(&path, recursive).await
        }
    }

    /// Get remote connection for a path, handling authentication
    async fn get_remote_connection(
        &self,
        path: &str,
        password: Option<String>,
        private_key_path: Option<String>,
    ) -> Result<(Arc<RemoteConnection>, String), CallToolResult> {
        let remote_authority = resolve_remote_path_authority(path, password, private_key_path)?;

        let connection_manager = self.get_remote_connection_manager();
        let conn = connection_manager
            .get_connection(&remote_authority.connection)
            .await
            .map_err(|e| {
                error!("Failed to establish remote connection: {}", e);
                remote_connection_error(&e)
            })?;

        Ok((conn, remote_authority.remote_path))
    }

    /// Format a command result into a CallToolResult
    fn format_command_result(
        command_result: &mut CommandResult,
    ) -> Result<CallToolResult, McpError> {
        command_result.output =
            match handle_large_output(&command_result.output, "command.output", 300, false) {
                Ok(result) => result,
                Err(e) => {
                    return Ok(CallToolResult::error(vec![
                        Content::text("OUTPUT_HANDLING_ERROR"),
                        Content::text(format!("Failed to handle command output: {}", e)),
                    ]));
                }
            };

        if command_result.output.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text("No output")]));
        }

        if command_result.exit_code != 0 {
            return Ok(CallToolResult::error(vec![
                Content::text("COMMAND_FAILED"),
                Content::text(&command_result.output),
            ]));
        }
        Ok(CallToolResult::success(vec![Content::text(
            &command_result.output,
        )]))
    }

    /// Format a task start result into a CallToolResult
    fn format_task_result(
        result: Result<TaskInfo, vac_foundation::task_manager::TaskError>,
    ) -> Result<CallToolResult, McpError> {
        match result {
            Ok(task_info) => {
                let output = serde_json::to_string_pretty(&task_info)
                    .unwrap_or_else(|_| format!("Task started: {}", task_info.id));

                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Background task started:\n{}",
                    output
                ))]))
            }
            Err(e) => {
                error!("Failed to start background task: {}", e);

                Ok(CallToolResult::error(vec![
                    Content::text("RUN_COMMAND_TASK_ERROR"),
                    Content::text(format!("Failed to start background task: {}", e)),
                ]))
            }
        }
    }

    fn apply_local_command_env(&self, cmd: &mut Command) {
        if let Some(profile_name) = self.local_runtime_defaults.active_profile_name() {
            cmd.env("VAC_PROFILE", profile_name);
        }
    }

    fn local_child_env_defaults(&self) -> std::collections::HashMap<String, String> {
        let mut child_env = std::collections::HashMap::new();
        if let Some(profile_name) = self.local_runtime_defaults.active_profile_name() {
            child_env.insert("VAC_PROFILE".to_string(), profile_name.to_string());
        }
        child_env
    }

    fn task_child_env_defaults(
        &self,
        remote_connection: Option<&RemoteConnectionInfo>,
    ) -> std::collections::HashMap<String, String> {
        if remote_connection.is_some() {
            std::collections::HashMap::new()
        } else {
            self.local_child_env_defaults()
        }
    }

    /// Execute command either locally or remotely based on parameters.
    async fn execute_command_unified(
        &self,
        command: &str,
        timeout: Option<u64>,
        remote: Option<String>,
        password: Option<String>,
        private_key_path: Option<String>,
        ctx: &RequestContext<RoleServer>,
    ) -> Result<CommandResult, CallToolResult> {
        if let Some(remote_str) = &remote {
            let connection_info = RemoteConnectionInfo {
                connection_string: remote_str.clone(),
                password,
                private_key_path,
            };

            let connection_manager = self.get_remote_connection_manager();
            let connection = connection_manager
                .get_connection(&connection_info)
                .await
                .map_err(|e| {
                    error!("Failed to establish remote connection: {}", e);
                    CallToolResult::error(vec![
                        Content::text("REMOTE_CONNECTION_ERROR"),
                        Content::text(format!("Failed to connect to remote host: {}", e)),
                    ])
                })?;

            let timeout_duration = timeout.map(std::time::Duration::from_secs);
            let (output, exit_code) = connection
                .execute_command(command, timeout_duration, Some(ctx))
                .await
                .map_err(|e| {
                    error!("Failed to execute remote command: {}", e);
                    CallToolResult::error(vec![
                        Content::text("REMOTE_COMMAND_ERROR"),
                        Content::text(format!("Failed to execute remote command: {}", e)),
                    ])
                })?;

            let mut result = output;
            if exit_code != 0 {
                result.push_str(&format!("\nCommand exited with code {}", exit_code));
            }

            Ok(CommandResult {
                output: result,
                exit_code,
            })
        } else {
            let structured = parse_vac_structured_command(command)?;
            self.execute_local_command(&structured, timeout, ctx).await
        }
    }

    /// Execute local command with existing logic extracted to avoid duplication
    async fn execute_local_command(
        &self,
        structured: &VacStructuredCommand,
        timeout: Option<u64>,
        ctx: &RequestContext<RoleServer>,
    ) -> Result<CommandResult, CallToolResult> {
        let mut cmd = Command::new(&structured.runner);
        cmd.args(&structured.args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        self.apply_local_command_env(&mut cmd);
        #[cfg(unix)]
        {
            cmd.env("DEBIAN_FRONTEND", "noninteractive")
                .env("SUDO_ASKPASS", "/bin/false")
                .process_group(0);
        }
        #[cfg(windows)]
        {
            // On Windows, create a new process group
            cmd.creation_flags(0x00000200); // CREATE_NEW_PROCESS_GROUP
        }

        let mut child = cmd.spawn().map_err(|e| {
            error!("Failed to run command: {}", e);
            CallToolResult::error(vec![
                Content::text("COMMAND_ERROR"),
                Content::text(format!("Failed to run command: {}", e)),
            ])
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            CallToolResult::error(vec![
                Content::text("COMMAND_ERROR"),
                Content::text("Failed to capture command stdout pipe after spawn"),
            ])
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            CallToolResult::error(vec![
                Content::text("COMMAND_ERROR"),
                Content::text("Failed to capture command stderr pipe after spawn"),
            ])
        })?;
        let mut stdout_reader = BufReader::new(stdout);
        let mut stderr_reader = BufReader::new(stderr);
        let mut stdout_buf = String::new();
        let mut stderr_buf = String::new();
        let mut result = String::new();
        let progress_id = Uuid::new_v4();

        // Stall detection: track last output time and stall start time for incrementing counter
        let mut last_output_time = std::time::Instant::now();
        let mut stall_start_time: Option<std::time::Instant> = None;
        const STALL_TIMEOUT_SECS: u64 = 5;

        // Helper function to stream output and wait for process completion
        let stream_and_wait = async {
            // Stall check interval
            let mut stall_check_interval = tokio::time::interval(Duration::from_secs(1));
            stall_check_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            macro_rules! handle_output {
                ($read_result:expr, $buf:expr) => {
                    match $read_result {
                        Ok(Ok(0)) => break, // EOF
                        Ok(Ok(_)) => {
                            last_output_time = std::time::Instant::now();
                            stall_start_time = None; // Reset stall tracking on output
                            let line = $buf.trim_end_matches('\n').to_string();
                            $buf.clear();
                            result.push_str(&format!("{}\n", line));
                            let _ = ctx
                                .peer
                                .notify_progress(ProgressNotificationParam {
                                    progress_token: ProgressToken(NumberOrString::Number(0)),
                                    progress: 50.0,
                                    total: Some(100.0),
                                    message: Some(
                                        serde_json::to_string(&ToolCallResultProgress {
                                            id: progress_id,
                                            message: format!("{}\n", line),
                                            progress_type: Some(ProgressType::CommandOutput),
                                            task_updates: None,
                                            progress: None,
                                        })
                                        .unwrap_or_default(),
                                    ),
                                })
                                .await;
                        }
                        Ok(Err(_)) => break, // Read error
                        Err(_) => {}         // Timeout - continue loop
                    }
                };
            }

            // Read from both streams concurrently
            loop {
                // Use biased selection so interval gets priority
                tokio::select! {
                    biased;

                    _ = stall_check_interval.tick() => {
                        // Check for stall condition: no output for 5 seconds
                        let elapsed_since_output = last_output_time.elapsed().as_secs();
                        if elapsed_since_output >= STALL_TIMEOUT_SECS {
                            // Initialize stall start time on first detection
                            if stall_start_time.is_none() {
                                stall_start_time = Some(std::time::Instant::now());
                            }

                            // Calculate running time (stall duration + initial 5s threshold)
                            let stall_duration = stall_start_time.map(|t| t.elapsed().as_secs()).unwrap_or(0);
                            let running_secs = STALL_TIMEOUT_SECS + stall_duration;

                            // Send stall notification with incrementing counter
                            let stall_msg = format!("__INTERACTIVE_STALL__: Running for {}s . ctrl+r to re-run in shell mode", running_secs);
                            let _ = ctx.peer.notify_progress(ProgressNotificationParam {
                                progress_token: ProgressToken(NumberOrString::Number(0)),
                                progress: 50.0,
                                total: Some(100.0),
                                message: Some(serde_json::to_string(&ToolCallResultProgress {
                                    id: progress_id,
                                    message: stall_msg,
                                    progress_type: Some(ProgressType::CommandOutput),
                                    task_updates: None,
                                    progress: None,
                                }).unwrap_or_default()),
                            }).await;
                        }
                    }

                    read_result = tokio::time::timeout(Duration::from_millis(100), stderr_reader.read_line(&mut stderr_buf)) => {
                        handle_output!(read_result, stderr_buf);
                    }

                    read_result = tokio::time::timeout(Duration::from_millis(100), stdout_reader.read_line(&mut stdout_buf)) => {
                        handle_output!(read_result, stdout_buf);
                    }
                }

                // Check if process has exited
                if let Ok(Some(_)) = child.try_wait() {
                    break;
                }
            }

            // Wait for the process to complete
            child.wait().await
        };

        // Execute with timeout and cancellation support
        let execution_result = if let Some(timeout_secs) = timeout {
            let timeout_duration = std::time::Duration::from_secs(timeout_secs);

            tokio::select! {
                result = tokio::time::timeout(timeout_duration, stream_and_wait) => result,
                _ = ctx.ct.cancelled() => {
                    // Cancellation occurred, kill the process
                    let _ = child.kill().await;
                    return Err(CallToolResult::cancel(Some(&vec![
                        Content::text("COMMAND_CANCELLED"),
                        Content::text("Command execution was cancelled"),
                    ])));
                }
            }
        } else {
            tokio::select! {
                result = stream_and_wait => Ok(result),
                _ = ctx.ct.cancelled() => {
                    let _ = child.kill().await;
                    return Err(CallToolResult::cancel(Some(&vec![
                        Content::text("COMMAND_CANCELLED"),
                        Content::text("Command execution was cancelled"),
                    ])));
                }
            }
        };

        let exit_code = match execution_result {
            Ok(Ok(exit_status)) => exit_status.code().unwrap_or(-1),
            Ok(Err(e)) => {
                return Err(CallToolResult::error(vec![
                    Content::text("COMMAND_ERROR"),
                    Content::text(format!("Failed to wait for command: {}", e)),
                ]));
            }
            Err(_) => {
                // Timeout occurred, kill the process
                let _ = child.kill().await;
                result.push_str(&format!(
                    "Command timed out after {} seconds\n",
                    timeout.unwrap_or_default()
                ));
                -1
            }
        };

        if exit_code != 0 {
            result.push_str(&format!("Command exited with code {}\n", exit_code));
        }

        Ok(CommandResult {
            output: result,
            exit_code,
        })
    }

    /// Build a TaskUpdate from a TaskInfo, extracting pause_info from raw_output if present.
    /// Used by both get_task_details and wait_for_tasks_with_streaming to populate
    /// the TUI's subagent_pause_info cache.
    fn build_task_update(task_info: &TaskInfo) -> TaskUpdate {
        let duration_secs = task_info.duration.map(|d| d.as_secs_f64());
        let output_preview = task_info.output.as_ref().and_then(|o| {
            let lines: Vec<&str> = o.lines().collect();
            if lines.is_empty() {
                None
            } else {
                lines.iter().rev().find(|l| !l.is_empty()).map(|l| {
                    if l.chars().count() > 50 {
                        let truncated: String = l.chars().take(50).collect();
                        format!("{}...", truncated)
                    } else {
                        l.to_string()
                    }
                })
            }
        });

        let pause_info = task_info.pause_info.as_ref().and_then(|pi| {
            pi.raw_output.as_ref().and_then(|raw| {
                serde_json::from_str::<serde_json::Value>(raw)
                    .ok()
                    .and_then(|json| {
                        let agent_message = json
                            .get("agent_message")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        let pending_tool_calls = json
                            .get("pause_reason")
                            .and_then(|pr| pr.get("pending_tool_calls"))
                            .and_then(|ptc| ptc.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|tc| {
                                        Some(PendingToolCall {
                                            id: tc.get("id")?.as_str()?.to_string(),
                                            name: tc.get("name")?.as_str()?.to_string(),
                                            arguments: tc
                                                .get("arguments")
                                                .cloned()
                                                .unwrap_or(serde_json::Value::Null),
                                        })
                                    })
                                    .collect()
                            });

                        if agent_message.is_some() || pending_tool_calls.is_some() {
                            Some(TaskPauseInfo {
                                agent_message,
                                pending_tool_calls,
                            })
                        } else {
                            None
                        }
                    })
            })
        });

        TaskUpdate {
            task_id: task_info.id.clone(),
            status: format!("{:?}", task_info.status),
            description: task_info.description.clone(),
            duration_secs,
            output_preview,
            is_target: true,
            pause_info,
        }
    }

    async fn wait_for_tasks_with_streaming(
        &self,
        task_ids: &[String],
        timeout: Option<std::time::Duration>,
        ctx: &RequestContext<RoleServer>,
    ) -> Result<Vec<TaskInfo>, vac_foundation::task_manager::TaskError> {
        let mut missing_tasks: Vec<String> = Vec::new();
        for task_id in task_ids {
            let task_status = self
                .get_task_manager()
                .get_task_status(task_id.clone())
                .await?;
            if task_status.is_none() {
                missing_tasks.push(task_id.clone());
            }
        }

        if !missing_tasks.is_empty() {
            return Err(vac_foundation::task_manager::TaskError::TaskNotFound(
                format!("Tasks not found: {}", missing_tasks.join(", ")),
            ));
        }

        let progress_id = Uuid::new_v4();

        let wait_operation = async {
            loop {
                let all_tasks = self.get_task_manager().get_all_tasks().await?;

                // Calculate real progress based on completed target tasks
                let mut completed_count = 0;
                let mut target_tasks_completed = true;

                for task_id in task_ids {
                    if let Some(task) = all_tasks.iter().find(|t| &t.id == task_id) {
                        match task.status {
                            vac_foundation::task_manager::TaskStatus::Pending
                            | vac_foundation::task_manager::TaskStatus::Running => {
                                target_tasks_completed = false;
                            }
                            _ => {
                                completed_count += 1;
                            }
                        }
                    }
                }

                // Calculate progress percentage
                let progress_pct = if task_ids.is_empty() {
                    100.0
                } else {
                    (completed_count as f64 / task_ids.len() as f64) * 100.0
                };

                // Build structured task updates
                let task_updates: Vec<TaskUpdate> = all_tasks
                    .iter()
                    .filter(|t| task_ids.contains(&t.id))
                    .map(Self::build_task_update)
                    .collect();

                // Also include fallback message for backwards compatibility
                let progress_table = self.format_tasks_table(&all_tasks, task_ids);

                let _ = ctx
                    .peer
                    .notify_progress(ProgressNotificationParam {
                        progress_token: ProgressToken(NumberOrString::Number(0)),
                        progress: progress_pct,
                        total: Some(100.0),
                        message: Some(
                            serde_json::to_string(&ToolCallResultProgress {
                                id: progress_id,
                                message: progress_table,
                                progress_type: Some(ProgressType::TaskWait),
                                task_updates: Some(task_updates),
                                progress: Some(progress_pct),
                            })
                            .unwrap_or_default(),
                        ),
                    })
                    .await;

                if target_tasks_completed {
                    return Ok(all_tasks);
                }

                sleep(Duration::from_millis(1000)).await;
            }
        };

        // Apply timeout if specified
        if let Some(timeout_duration) = timeout {
            match tokio_timeout(timeout_duration, wait_operation).await {
                Ok(result) => result,
                Err(_) => Err(vac_foundation::task_manager::TaskError::TaskTimeout),
            }
        } else {
            wait_operation.await
        }
    }

    fn format_tasks_table(&self, tasks: &[TaskInfo], target_task_ids: &[String]) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};

        let mut table = String::new();

        // Add timestamp and clear separator for streaming
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs();
        let time_str = chrono::DateTime::from_timestamp(timestamp as i64, 0)
            .unwrap_or_else(chrono::Utc::now)
            .format("%H:%M:%S");

        table.push_str(&format!("═══ Background Tasks Update [{}] ═══\n", time_str));
        table.push_str(&format!("Waiting for: {}\n", target_task_ids.join(", ")));

        if tasks.is_empty() {
            table.push_str("No background tasks found.\n");
            table.push_str("═══════════════════════════════════════\n\n");
            return table;
        }

        // Sort tasks by start time (newest first)
        let mut sorted_tasks = tasks.to_vec();
        sorted_tasks.sort_by_key(|task| std::cmp::Reverse(task.start_time));

        // Compact format for streaming - one line per task
        for task in &sorted_tasks {
            let task_id = &task.id;
            let status = format!("{:?}", task.status);
            let duration = if let Some(duration) = task.duration {
                format!("{:.1}s", duration.as_secs_f64())
            } else {
                "running".to_string()
            };

            let truncated_command = task
                .command
                .chars()
                .take(30)
                .collect::<String>()
                .replace('\n', " ");

            // Highlight target tasks and show status
            let marker = if target_task_ids.contains(task_id) {
                ">"
            } else {
                " "
            };
            let status_icon = match status.as_str() {
                "Running" => "[RUN]",
                "Completed" => "[OK]",
                "Failed" => "[ERR]",
                _ => "[---]",
            };

            table.push_str(&format!(
                "{} {} {} [{}] {} - {}\n",
                marker, status_icon, task_id, duration, status, truncated_command
            ));
        }

        table.push_str(&format!(
            "Total: {} tasks | Waiting for: {}\n",
            sorted_tasks.len(),
            target_task_ids.len()
        ));
        table.push_str("═══════════════════════════════════════\n\n");

        table
    }

    #[tool(
        description = "Ask the user one or more questions with predefined options. Use this when you need user input to make decisions or gather preferences.

WHEN TO USE:
- When you need to clarify requirements before proceeding
- When there are multiple valid approaches and user preference matters
- When confirming / gathering information
- It's easier / faster for the user than prompting them for a full text response
"
    )]
    pub async fn ask_user(
        &self,
        _ctx: RequestContext<RoleServer>,
        Parameters(_request): Parameters<AskUserRequest>,
    ) -> Result<CallToolResult, McpError> {
        // This tool is handled specially by the TUI - it should never reach here
        // If it does, return an error indicating the tool requires interactive mode
        Ok(CallToolResult::error(vec![
            Content::text("INTERACTIVE_REQUIRED"),
            Content::text(
                "The ask_user tool requires interactive mode. It cannot be used in headless/async execution.",
            ),
        ]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_remote_command_request_whitespace_remote_deserializes_but_validated_at_runtime() {
        let result = serde_json::from_value::<RunRemoteCommandRequest>(serde_json::json!({
            "command": "echo hello",
            "remote": "   "
        }));
        // Whitespace remote deserializes (it's a plain String) but will be
        // rejected at handler level by the trim + contains('@') check
        assert!(result.is_ok());
        let req = result.unwrap();
        let trimmed = req.remote.trim();
        assert!(trimmed.is_empty() || !trimmed.contains('@'));
    }

    #[test]
    fn run_remote_command_request_password_preserves_whitespace() {
        let request: RunRemoteCommandRequest = serde_json::from_value(serde_json::json!({
            "command": "echo hello",
            "remote": "user@host",
            "password": "  pass with spaces  "
        }))
        .expect("run remote command request should deserialize");

        assert_eq!(request.password.as_deref(), Some("  pass with spaces  "));
    }

    #[test]
    fn run_command_request_has_no_remote_fields() {
        let request: RunCommandRequest = serde_json::from_value(serde_json::json!({
            "command": "echo hello"
        }))
        .expect("run command request should deserialize");

        assert_eq!(request.command, "echo hello");
    }

    #[test]
    fn run_command_rejects_legacy_remote_field() {
        // A legacy payload with `remote` must NOT silently deserialize as a
        // local RunCommandRequest — it should fail due to deny_unknown_fields.
        let result = serde_json::from_value::<RunCommandRequest>(serde_json::json!({
            "command": "rm -rf /tmp/x",
            "remote": "user@prod"
        }));
        assert!(
            result.is_err(),
            "RunCommandRequest must reject unknown 'remote' field to prevent wrong-host execution"
        );
    }

    #[test]
    fn run_command_rejects_legacy_password_field() {
        let result = serde_json::from_value::<RunCommandRequest>(serde_json::json!({
            "command": "echo hello",
            "password": "secret"
        }));
        assert!(
            result.is_err(),
            "RunCommandRequest must reject unknown 'password' field"
        );
    }

    #[test]
    fn view_web_page_allows_only_loopback_http_exception() {
        let loopback_v4 = url::Url::parse("http://127.0.0.1:41731/fixture.html").unwrap();
        let loopback_v6 = url::Url::parse("http://[::1]:41731/fixture.html").unwrap();
        let localhost = url::Url::parse("http://localhost:41731/fixture.html").unwrap();
        let external_http = url::Url::parse("http://example.com/fixture.html").unwrap();
        let external_https = url::Url::parse("https://example.com/fixture.html").unwrap();

        assert!(is_loopback_http_url(&loopback_v4));
        assert!(is_loopback_http_url(&loopback_v6));
        assert!(is_loopback_http_url(&localhost));
        assert!(!is_loopback_http_url(&external_http));
        assert!(!is_loopback_http_url(&external_https));
    }

    fn local_container_with_profile(profile_name: Option<&str>) -> ToolContainer {
        let task_manager = vac_foundation::task_manager::TaskManager::new();
        ToolContainer::new(
            None,
            crate::EnabledToolsConfig::default(),
            task_manager.handle(),
            ToolContainer::tool_router_local(),
            Vec::new(),
            crate::SubagentConfig {
                profile_name: profile_name.map(str::to_string),
                config_path: None,
                model: None,
            },
        )
        .expect("tool container should be constructed")
    }

    async fn run_profile_probe(container: &ToolContainer, shell: &str) -> String {
        let mut cmd = tokio::process::Command::new("sh");
        cmd.env_remove("VAC_PROFILE")
            .arg("-c")
            .arg(shell)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        container.apply_local_command_env(&mut cmd);

        let output = cmd.output().await.expect("profile probe should spawn");
        assert!(
            output.status.success(),
            "profile probe should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("profile probe output should be utf8")
    }

    #[tokio::test]
    async fn run_command_spawn_inherits_active_profile_env() {
        let container = local_container_with_profile(Some("ops"));

        let output = run_profile_probe(&container, "printf '%s' \"$VAC_PROFILE\"").await;

        assert_eq!(output, "ops");
    }

    #[tokio::test]
    async fn run_command_spawn_does_not_inject_empty_profile_env() {
        let container = local_container_with_profile(Some("   "));

        let output = run_profile_probe(
            &container,
            "if [ \"${VAC_PROFILE+x}\" = x ]; then printf 'present:%s' \"$VAC_PROFILE\"; else printf missing; fi",
        )
        .await;

        assert_eq!(output, "missing");
    }

    #[tokio::test]
    async fn run_command_inline_profile_env_override_wins() {
        let container = local_container_with_profile(Some("ops"));

        let output = run_profile_probe(
            &container,
            "VAC_PROFILE=readonly sh -c 'printf %s \"$VAC_PROFILE\"'",
        )
        .await;

        assert_eq!(output, "readonly");
    }

    #[test]
    fn remote_command_task_gets_no_local_profile_child_env_defaults() {
        let container = local_container_with_profile(Some("ops"));
        let remote_connection = RemoteConnectionInfo {
            connection_string: "user@example.com".to_string(),
            password: None,
            private_key_path: None,
        };

        let child_env = container.task_child_env_defaults(Some(&remote_connection));

        assert!(
            !child_env.contains_key("VAC_PROFILE"),
            "remote task child env must not inherit the local profile"
        );
    }

    #[test]
    fn remote_request_rejects_unknown_fields() {
        let result = serde_json::from_value::<RunRemoteCommandRequest>(serde_json::json!({
            "command": "echo hello",
            "remote": "user@host",
            "unknown_field": "value"
        }));
        assert!(
            result.is_err(),
            "RunRemoteCommandRequest must reject unknown fields"
        );
    }
}
