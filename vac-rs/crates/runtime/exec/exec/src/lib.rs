// VAC exec subcommand library entry point.
//
// Output mode contract:
// - In the default output mode, it is paramount that the only thing written to
//   stdout is the final message (if any).
// - In --json mode, stdout must be valid JSONL, one event per line.
// For both modes, any other output must be written to stderr.
#![deny(clippy::print_stdout)]

mod cli;
pub(crate) mod exec_events;
mod runtime_adapter;

pub use cli::Cli;
pub use cli::Command;
pub use cli::ReviewArgs;
pub use exec_events::AgentMessageItem;
pub use exec_events::CollabAgentState;
pub use exec_events::CollabAgentStatus;
pub use exec_events::CollabTool;
pub use exec_events::CollabToolCallItem;
pub use exec_events::CollabToolCallStatus;
pub use exec_events::CommandExecutionItem;
pub use exec_events::CommandExecutionStatus;
pub use exec_events::ErrorItem;
pub use exec_events::FileChangeItem;
pub use exec_events::FileUpdateChange;
pub use exec_events::ItemCompletedEvent;
pub use exec_events::ItemStartedEvent;
pub use exec_events::ItemUpdatedEvent;
pub use exec_events::McpToolCallItem;
pub use exec_events::McpToolCallItemError;
pub use exec_events::McpToolCallItemResult;
pub use exec_events::McpToolCallStatus;
pub use exec_events::PatchApplyStatus;
pub use exec_events::PatchChangeKind;
pub use exec_events::ReasoningItem;
pub use exec_events::ThreadErrorEvent;
pub use exec_events::ThreadEvent;
pub use exec_events::ThreadItem as ExecThreadItem;
pub use exec_events::ThreadItemDetails;
pub use exec_events::ThreadStartedEvent;
pub use exec_events::TodoItem;
pub use exec_events::TodoListItem;
pub use exec_events::TurnCompletedEvent;
pub use exec_events::TurnFailedEvent;
pub use exec_events::TurnStartedEvent;
pub use exec_events::Usage;
pub use exec_events::WebSearchItem;

use serde_json::Value;
use std::io::IsTerminal;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use supports_color::Stream;
use tracing::Instrument;
use tracing::error;
use tracing::field;
use tracing::info_span;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;
use vac_arg0::Arg0DispatchPaths;
use vac_config::ConfigLoadError;
use vac_config::LoaderOverrides;
use vac_config::format_config_error_with_source;
use vac_core::check_execpolicy_for_warnings;
use vac_core::config::Config;
use vac_core::config::ConfigBuilder;
use vac_core::config::ConfigOverrides;
use vac_core::config::find_vac_home;
use vac_core::config::load_config_as_toml_with_cli_and_loader_overrides;
use vac_core::config::resolve_oss_provider;
use vac_core::format_exec_policy_error_with_source;
use vac_core::local_runtime::AutonomyMode;
use vac_core::local_runtime::RuntimeCommand;
use vac_core::local_runtime::RuntimeEntrypoint;
use vac_exec_server::EnvironmentManager;
use vac_exec_server::EnvironmentManagerArgs;
use vac_exec_server::ExecServerRuntimePaths;
use vac_git_utils::get_git_repo_root;
use vac_login::AuthConfig;
use vac_login::default_client::set_default_client_residency_requirement;
use vac_login::default_client::set_default_originator;
use vac_login::enforce_login_restrictions;
use vac_model_provider_info::LMSTUDIO_OSS_PROVIDER_ID;
use vac_model_provider_info::OLLAMA_OSS_PROVIDER_ID;
use vac_otel::set_parent_from_context;
use vac_otel::traceparent_context_from_env;
use vac_protocol::config_types::SandboxMode;
use vac_protocol::protocol::AskForApproval;
use vac_protocol::protocol::ReviewRequest;
use vac_protocol::protocol::ReviewTarget;
use vac_protocol::user_input::UserInput;
use vac_utils_absolute_path::AbsolutePathBuf;
use vac_utils_absolute_path::canonicalize_existing_preserving_symlinks;
use vac_utils_cli::SharedCliOptions;
use vac_utils_oss::ensure_oss_provider_ready;
use vac_utils_oss::get_default_model_for_oss_provider;

use crate::cli::Command as ExecCommand;
use crate::runtime_adapter::resolve_resume_rollout_path;
use crate::runtime_adapter::run_local_runtime_prompt;
use crate::runtime_adapter::run_local_runtime_resume;
use crate::runtime_adapter::run_local_runtime_review;

const DEFAULT_ANALYTICS_ENABLED: bool = true;

enum StdinPromptBehavior {
    /// Read stdin only when there is no positional prompt, which is the legacy
    /// `vac exec` behavior for `vac exec` with piped input.
    RequiredIfPiped,
    /// Always treat stdin as the prompt, used for the explicit `vac exec -`
    /// sentinel and similar forced-stdin call sites.
    Forced,
    /// If stdin is piped alongside a positional prompt, treat stdin as
    /// additional context to append rather than as the primary prompt.
    OptionalAppend,
}

struct ExecRunArgs {
    environment_manager: std::sync::Arc<EnvironmentManager>,
    command: Option<ExecCommand>,
    config: Config,
    dangerously_bypass_approvals_and_sandbox: bool,
    images: Vec<PathBuf>,
    json_mode: bool,
    last_message_file: Option<PathBuf>,
    model_provider: Option<String>,
    oss: bool,
    output_schema_path: Option<PathBuf>,
    prompt: Option<String>,
    skip_git_repo_check: bool,
    stderr_with_ansi: bool,
}

fn exec_root_span() -> tracing::Span {
    info_span!(
        "vac.exec",
        otel.kind = "internal",
        thread.id = field::Empty,
        turn.id = field::Empty,
    )
}

pub async fn run_main(cli: Cli, arg0_paths: Arg0DispatchPaths) -> anyhow::Result<()> {
    #[allow(clippy::print_stderr)]
    if let Some(message) = cli.removed_full_auto_warning() {
        eprintln!("{message}");
    }

    if let Err(err) = set_default_originator("vac_exec".to_string()) {
        tracing::warn!(?err, "Failed to set vac exec originator override {err:?}");
    }

    let Cli {
        command,
        shared,
        skip_git_repo_check,
        ephemeral,
        ignore_user_config,
        ignore_rules,
        removed_full_auto,
        color,
        last_message_file,
        json: json_mode,
        prompt,
        output_schema: output_schema_path,
        config_overrides,
    } = cli;
    let shared = shared.into_inner();
    let SharedCliOptions {
        images,
        model: model_cli_arg,
        oss,
        oss_provider,
        config_profile,
        sandbox_mode: sandbox_mode_cli_arg,
        dangerously_bypass_approvals_and_sandbox,
        cwd,
        add_dir,
    } = shared;

    let (_stdout_with_ansi, stderr_with_ansi) = match color {
        cli::Color::Always => (true, true),
        cli::Color::Never => (false, false),
        cli::Color::Auto => (
            supports_color::on_cached(Stream::Stdout).is_some(),
            supports_color::on_cached(Stream::Stderr).is_some(),
        ),
    };
    let default_level = "error";
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(default_level))
        .unwrap_or_else(|_| EnvFilter::new(default_level));
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(stderr_with_ansi)
        .with_writer(std::io::stderr)
        .with_filter(env_filter);

    let sandbox_mode = if removed_full_auto {
        Some(SandboxMode::WorkspaceWrite)
    } else if dangerously_bypass_approvals_and_sandbox {
        Some(SandboxMode::DangerFullAccess)
    } else {
        sandbox_mode_cli_arg.map(Into::<SandboxMode>::into)
    };

    let cli_kv_overrides = match config_overrides.parse_overrides() {
        Ok(v) => v,
        #[allow(clippy::print_stderr)]
        Err(e) => {
            eprintln!("Error parsing -c overrides: {e}");
            std::process::exit(1);
        }
    };

    let resolved_cwd = cwd.clone();
    let config_cwd = match resolved_cwd.as_deref() {
        Some(path) => {
            AbsolutePathBuf::from_absolute_path(canonicalize_existing_preserving_symlinks(path)?)?
        }
        None => AbsolutePathBuf::current_dir()?,
    };

    #[allow(clippy::print_stderr)]
    let vac_home = match find_vac_home() {
        Ok(vac_home) => vac_home,
        Err(err) => {
            eprintln!("Error finding vac home: {err}");
            std::process::exit(1);
        }
    };

    #[allow(clippy::print_stderr)]
    let loader_overrides = LoaderOverrides {
        ignore_user_config,
        ignore_user_and_project_exec_policy_rules: ignore_rules,
        ..Default::default()
    };

    let config_toml = match load_config_as_toml_with_cli_and_loader_overrides(
        &vac_home,
        Some(&config_cwd),
        cli_kv_overrides.clone(),
        loader_overrides.clone(),
    )
    .await
    {
        Ok(config_toml) => config_toml,
        Err(err) => {
            let config_error = err
                .get_ref()
                .and_then(|err| err.downcast_ref::<ConfigLoadError>())
                .map(ConfigLoadError::config_error);
            if let Some(config_error) = config_error {
                eprintln!(
                    "Error loading config.toml:\n{}",
                    format_config_error_with_source(config_error)
                );
            } else {
                eprintln!("Error loading config.toml: {err}");
            }
            std::process::exit(1);
        }
    };

    let model_provider = if oss {
        let resolved = resolve_oss_provider(
            oss_provider.as_deref(),
            &config_toml,
            config_profile.clone(),
        );

        if let Some(provider) = resolved {
            Some(provider)
        } else {
            return Err(anyhow::anyhow!(
                "No default OSS provider configured. Use --local-provider=provider or set oss_provider to one of: {LMSTUDIO_OSS_PROVIDER_ID}, {OLLAMA_OSS_PROVIDER_ID} in config.toml"
            ));
        }
    } else {
        None
    };

    let model = if let Some(model) = model_cli_arg {
        Some(model)
    } else if oss {
        model_provider
            .as_ref()
            .and_then(|provider_id| get_default_model_for_oss_provider(provider_id))
            .map(std::borrow::ToOwned::to_owned)
    } else {
        None
    };

    let overrides = ConfigOverrides {
        model,
        review_model: None,
        config_profile,
        approval_policy: Some(AskForApproval::Never),
        approvals_reviewer: None,
        sandbox_mode,
        permission_profile: None,
        default_permissions: None,
        cwd: resolved_cwd,
        model_provider: model_provider.clone(),
        service_tier: None,
        vac_self_exe: arg0_paths.vac_self_exe.clone(),
        vac_linux_sandbox_exe: arg0_paths.vac_linux_sandbox_exe.clone(),
        main_execve_wrapper_exe: arg0_paths.main_execve_wrapper_exe.clone(),
        zsh_path: None,
        base_instructions: None,
        developer_instructions: None,
        personality: None,
        compact_prompt: None,
        include_apply_patch_tool: None,
        show_raw_agent_reasoning: oss.then_some(true),
        tools_web_search_request: None,
        ephemeral: ephemeral.then_some(true),
        additional_writable_roots: add_dir,
    };

    let config = ConfigBuilder::default()
        .cli_overrides(cli_kv_overrides)
        .harness_overrides(overrides)
        .loader_overrides(loader_overrides)
        .build()
        .await?;

    #[allow(clippy::print_stderr)]
    match check_execpolicy_for_warnings(&config.config_layer_stack).await {
        Ok(None) => {}
        Ok(Some(err)) | Err(err) => {
            eprintln!(
                "Error loading rules:\n{}",
                format_exec_policy_error_with_source(&err)
            );
            std::process::exit(1);
        }
    }

    set_default_client_residency_requirement(config.enforce_residency.value());

    if let Err(err) = enforce_login_restrictions(&AuthConfig {
        vac_home: config.vac_home.to_path_buf(),
        auth_credentials_store_mode: config.cli_auth_credentials_store_mode,
        forced_login_method: config.forced_login_method,
        forced_chatgpt_workspace_id: config.forced_chatgpt_workspace_id.clone(),
        chatgpt_base_url: Some(config.chatgpt_base_url.clone()),
    })
    .await
    {
        eprintln!("{err}");
        std::process::exit(1);
    }

    let otel = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        vac_core::otel_init::build_provider(
            &config,
            env!("CARGO_PKG_VERSION"),
            /*service_name_override*/ None,
            DEFAULT_ANALYTICS_ENABLED,
        )
    })) {
        Ok(Ok(otel)) => otel,
        Ok(Err(e)) => {
            eprintln!("Could not create otel exporter: {e}");
            None
        }
        Err(_) => {
            eprintln!("Could not create otel exporter: panicked during initialization");
            None
        }
    };

    let otel_logger_layer = otel.as_ref().and_then(|o| o.logger_layer());
    let otel_tracing_layer = otel.as_ref().and_then(|o| o.tracing_layer());

    let _ = tracing_subscriber::registry()
        .with(fmt_layer)
        .with(otel_tracing_layer)
        .with(otel_logger_layer)
        .try_init();

    let exec_span = exec_root_span();
    if let Some(context) = traceparent_context_from_env() {
        set_parent_from_context(&exec_span, context);
    }
    let local_runtime_paths = ExecServerRuntimePaths::from_optional_paths(
        arg0_paths.vac_self_exe.clone(),
        arg0_paths.vac_linux_sandbox_exe.clone(),
    )?;
    let environment_manager = std::sync::Arc::new(
        EnvironmentManager::new(EnvironmentManagerArgs::new(local_runtime_paths)).await,
    );
    run_exec_session(ExecRunArgs {
        environment_manager,
        command,
        config,
        dangerously_bypass_approvals_and_sandbox,
        images,
        json_mode,
        last_message_file,
        model_provider,
        oss,
        output_schema_path,
        prompt,
        skip_git_repo_check,
        stderr_with_ansi,
    })
    .instrument(exec_span)
    .await
}

async fn run_exec_session(args: ExecRunArgs) -> anyhow::Result<()> {
    let ExecRunArgs {
        environment_manager,
        command,
        config,
        dangerously_bypass_approvals_and_sandbox,
        images,
        json_mode,
        last_message_file,
        model_provider,
        oss,
        output_schema_path,
        prompt,
        skip_git_repo_check,
        stderr_with_ansi,
    } = args;

    if oss {
        let provider_id = match model_provider.as_ref() {
            Some(id) => id,
            None => {
                error!("OSS provider unexpectedly not set when oss flag is used");
                return Err(anyhow::anyhow!(
                    "OSS provider not set but oss flag was used"
                ));
            }
        };
        ensure_oss_provider_ready(provider_id, &config)
            .await
            .map_err(|e| anyhow::anyhow!("OSS setup failed: {e}"))?;
    }

    let default_cwd = config.cwd.to_path_buf();

    if command.is_some()
        && !skip_git_repo_check
        && !dangerously_bypass_approvals_and_sandbox
        && get_git_repo_root(&default_cwd).is_none()
    {
        eprintln!("Not inside a trusted directory and --skip-git-repo-check was not specified.");
        std::process::exit(1);
    }

    match (command, prompt, images) {
        (Some(ExecCommand::Review(review_cli)), _, _) => {
            let review_request = build_review_request(&review_cli)?;
            let summary = vac_core::review_prompts::user_facing_hint(&review_request.target);
            run_local_runtime_review(
                &config,
                environment_manager,
                review_request,
                summary,
                json_mode,
                last_message_file,
                stderr_with_ansi,
            )
            .await
        }
        (Some(ExecCommand::Resume(resume_args)), root_prompt, imgs) => {
            let prompt_arg = resume_args
                .prompt
                .clone()
                .or_else(|| {
                    if resume_args.last {
                        resume_args.session_id.clone()
                    } else {
                        None
                    }
                })
                .or(root_prompt);
            let prompt_text = resolve_prompt(prompt_arg);
            let mut items: Vec<UserInput> = imgs
                .into_iter()
                .chain(resume_args.images.iter().cloned())
                .map(|path| UserInput::LocalImage { path })
                .collect();
            items.push(UserInput::Text {
                text: prompt_text.clone(),
                text_elements: Vec::new(),
            });
            log_local_runtime_start_task(&prompt_text, default_cwd.as_path());
            let output_schema = load_output_schema(output_schema_path);
            let rollout_path = match resolve_resume_rollout_path(&config, &resume_args).await? {
                Some(path) => path,
                None => return Err(anyhow::anyhow!("no matching session found to resume")),
            };
            run_local_runtime_resume(
                &config,
                environment_manager,
                rollout_path,
                prompt_text,
                items,
                output_schema,
                json_mode,
                last_message_file,
                stderr_with_ansi,
            )
            .await
        }
        (None, root_prompt, imgs) => {
            let prompt_text = resolve_root_prompt(root_prompt);
            let mut items: Vec<UserInput> = imgs
                .into_iter()
                .map(|path| UserInput::LocalImage { path })
                .collect();
            items.push(UserInput::Text {
                text: prompt_text.clone(),
                text_elements: Vec::new(),
            });
            log_local_runtime_start_task(&prompt_text, default_cwd.as_path());
            let output_schema = load_output_schema(output_schema_path);
            run_local_runtime_prompt(
                &config,
                environment_manager,
                prompt_text,
                items,
                output_schema,
                json_mode,
                last_message_file,
                stderr_with_ansi,
            )
            .await
        }
    }
}

fn log_local_runtime_start_task(prompt: &str, cwd: &Path) {
    let local_runtime_command = RuntimeCommand::start_task(
        prompt,
        AutonomyMode::Autopilot,
        RuntimeEntrypoint::Exec,
        cwd.to_path_buf(),
    );
    tracing::debug!(?local_runtime_command, "prepared local runtime start task");
}

fn build_review_request(args: &ReviewArgs) -> anyhow::Result<ReviewRequest> {
    let target = if args.uncommitted {
        ReviewTarget::UncommittedChanges
    } else if let Some(branch) = args.base.clone() {
        ReviewTarget::BaseBranch { branch }
    } else if let Some(sha) = args.commit.clone() {
        ReviewTarget::Commit {
            sha,
            title: args.commit_title.clone(),
        }
    } else if let Some(prompt_arg) = args.prompt.clone() {
        let prompt = resolve_prompt(Some(prompt_arg)).trim().to_string();
        if prompt.is_empty() {
            anyhow::bail!("Review prompt cannot be empty");
        }
        ReviewTarget::Custom {
            instructions: prompt,
        }
    } else {
        anyhow::bail!(
            "Specify --uncommitted, --base, --commit, or provide custom review instructions"
        );
    };

    Ok(ReviewRequest {
        target,
        user_facing_hint: None,
    })
}

fn load_output_schema(path: Option<PathBuf>) -> Option<Value> {
    let path = path?;

    let schema_str = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(err) => {
            eprintln!(
                "Failed to read output schema file {}: {err}",
                path.display()
            );
            std::process::exit(1);
        }
    };

    match serde_json::from_str::<Value>(&schema_str) {
        Ok(value) => Some(value),
        Err(err) => {
            eprintln!(
                "Output schema file {} is not valid JSON: {err}",
                path.display()
            );
            std::process::exit(1);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PromptDecodeError {
    InvalidUtf8 { valid_up_to: usize },
    InvalidUtf16 { encoding: &'static str },
    UnsupportedBom { encoding: &'static str },
}

impl std::fmt::Display for PromptDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PromptDecodeError::InvalidUtf8 { valid_up_to } => write!(
                f,
                "input is not valid UTF-8 (invalid byte at offset {valid_up_to}). Convert it to UTF-8 and retry (e.g., `iconv -f <ENC> -t UTF-8 prompt.txt`)."
            ),
            PromptDecodeError::InvalidUtf16 { encoding } => write!(
                f,
                "input looked like {encoding} but could not be decoded. Convert it to UTF-8 and retry."
            ),
            PromptDecodeError::UnsupportedBom { encoding } => write!(
                f,
                "input appears to be {encoding}. Convert it to UTF-8 and retry."
            ),
        }
    }
}

fn decode_prompt_bytes(input: &[u8]) -> Result<String, PromptDecodeError> {
    let input = input.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(input);

    if input.starts_with(&[0xFF, 0xFE, 0x00, 0x00]) {
        return Err(PromptDecodeError::UnsupportedBom {
            encoding: "UTF-32LE",
        });
    }

    if input.starts_with(&[0x00, 0x00, 0xFE, 0xFF]) {
        return Err(PromptDecodeError::UnsupportedBom {
            encoding: "UTF-32BE",
        });
    }

    if let Some(rest) = input.strip_prefix(&[0xFF, 0xFE]) {
        return decode_utf16(rest, "UTF-16LE", u16::from_le_bytes);
    }

    if let Some(rest) = input.strip_prefix(&[0xFE, 0xFF]) {
        return decode_utf16(rest, "UTF-16BE", u16::from_be_bytes);
    }

    std::str::from_utf8(input)
        .map(str::to_string)
        .map_err(|e| PromptDecodeError::InvalidUtf8 {
            valid_up_to: e.valid_up_to(),
        })
}

fn decode_utf16(
    input: &[u8],
    encoding: &'static str,
    decode_unit: fn([u8; 2]) -> u16,
) -> Result<String, PromptDecodeError> {
    if !input.len().is_multiple_of(2) {
        return Err(PromptDecodeError::InvalidUtf16 { encoding });
    }
    let units: Vec<u16> = input
        .chunks_exact(2)
        .map(|chunk| decode_unit([chunk[0], chunk[1]]))
        .collect();

    String::from_utf16(&units).map_err(|_| PromptDecodeError::InvalidUtf16 { encoding })
}

fn read_prompt_from_stdin(behavior: StdinPromptBehavior) -> Option<String> {
    let stdin_is_terminal = std::io::stdin().is_terminal();

    match behavior {
        StdinPromptBehavior::RequiredIfPiped if stdin_is_terminal => {
            eprintln!(
                "No prompt provided. Either specify one as an argument or pipe the prompt into stdin."
            );
            std::process::exit(1);
        }
        StdinPromptBehavior::RequiredIfPiped => {
            eprintln!("Reading prompt from stdin...");
        }
        StdinPromptBehavior::Forced => {}
        StdinPromptBehavior::OptionalAppend if stdin_is_terminal => return None,
        StdinPromptBehavior::OptionalAppend => {
            eprintln!("Reading additional input from stdin...");
        }
    }

    let mut bytes = Vec::new();
    if let Err(e) = std::io::stdin().read_to_end(&mut bytes) {
        eprintln!("Failed to read prompt from stdin: {e}");
        std::process::exit(1);
    }

    let buffer = match decode_prompt_bytes(&bytes) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read prompt from stdin: {e}");
            std::process::exit(1);
        }
    };

    if buffer.trim().is_empty() {
        match behavior {
            StdinPromptBehavior::OptionalAppend => None,
            StdinPromptBehavior::RequiredIfPiped | StdinPromptBehavior::Forced => {
                eprintln!("No prompt provided via stdin.");
                std::process::exit(1);
            }
        }
    } else {
        Some(buffer)
    }
}

fn prompt_with_stdin_context(prompt: &str, stdin_text: &str) -> String {
    let mut combined = format!("{prompt}\n\n<stdin>\n{stdin_text}");
    if !stdin_text.ends_with('\n') {
        combined.push('\n');
    }
    combined.push_str("</stdin>");
    combined
}

fn resolve_prompt(prompt_arg: Option<String>) -> String {
    match prompt_arg {
        Some(p) if p != "-" => p,
        maybe_dash => {
            let behavior = if matches!(maybe_dash.as_deref(), Some("-")) {
                StdinPromptBehavior::Forced
            } else {
                StdinPromptBehavior::RequiredIfPiped
            };
            let Some(prompt) = read_prompt_from_stdin(behavior) else {
                unreachable!("required stdin prompt should produce content");
            };
            prompt
        }
    }
}

fn resolve_root_prompt(prompt_arg: Option<String>) -> String {
    match prompt_arg {
        Some(prompt) if prompt != "-" => {
            if let Some(stdin_text) = read_prompt_from_stdin(StdinPromptBehavior::OptionalAppend) {
                prompt_with_stdin_context(&prompt, &stdin_text)
            } else {
                prompt
            }
        }
        maybe_dash => resolve_prompt(maybe_dash),
    }
}
