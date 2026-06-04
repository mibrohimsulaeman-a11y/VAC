// Forbid accidental stdout/stderr writes in the *library* portion of the TUI.
// The standalone `vac-tui` binary prints a short help message before the
// alternate‑screen mode starts; that file opts‑out locally via `allow`.
use crate::legacy_core::check_execpolicy_for_warnings;
use crate::legacy_core::config::Config;
use crate::legacy_core::config::ConfigBuilder;
use crate::legacy_core::config::ConfigOverrides;
use crate::legacy_core::config::find_vac_home;
use crate::legacy_core::config::load_config_as_toml_with_cli_overrides;
use crate::legacy_core::config::resolve_oss_provider;
use crate::legacy_core::format_exec_policy_error_with_source;
use crate::legacy_core::windows_sandbox::WindowsSandboxLevelExt;
use crate::local_runtime_session::AppServerClient;
use crate::local_runtime_session::DEFAULT_IN_PROCESS_CHANNEL_CAPACITY;
use crate::local_runtime_session::EnvironmentManager;
use crate::local_runtime_session::EnvironmentManagerArgs;
use crate::local_runtime_session::ExecServerRuntimePaths;
use crate::local_runtime_session::InProcessAppServerClient;
use crate::local_runtime_session::InProcessClientStartArgs;
use crate::local_runtime_session::LocalRuntimeSession;
use crate::local_runtime_session::TuiSessionRequestHandle;
use crate::local_runtime_session::into_local_runtime_session;
use crate::session_protocol::Account as AppServerAccount;
use crate::session_protocol::AppServerThreadSortKey;
use crate::session_protocol::AskForApproval;
use crate::session_protocol::AuthMode as AppServerAuthMode;
use crate::session_protocol::ConfigWarningNotification;
use crate::session_protocol::Thread as AppServerThread;
use crate::session_protocol::ThreadListCwdFilter;
use crate::session_protocol::ThreadListParams;
use crate::session_protocol::ThreadSourceKind;
use crate::session_resume::ResolveCwdOutcome;
use crate::session_resume::resolve_cwd_for_resume_or_branch;
use additional_dirs::add_dir_warning_message;
use app::App;
pub use app::AppExitInfo;
pub use app::ExitReason;
use color_eyre::eyre::WrapErr;
use cwd_prompt::CwdPromptAction;
use std::fs::OpenOptions;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
pub use token_usage::TokenUsage;
use tracing::Level;
use tracing::error;
use tracing::warn;
const EXPLICIT_AUTH_STATE_NO_SILENT_SKIP: &str = "explicit_auth_state_no_silent_skip";
mod auth_state_banner;
use tracing_appender::non_blocking;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::prelude::*;
use uuid::Uuid;
use vac_config::ConfigLoadError;
use vac_config::LoaderOverrides;
use vac_config::format_config_error_with_source;
use vac_core::project_workspace::build_project_workspace_confirmation_dialog;
use vac_core::project_workspace::project_workspace_startup_notice;
use vac_login::AuthConfig;
use vac_login::default_client::set_default_client_residency_requirement;
use vac_login::enforce_login_restrictions;
use vac_protocol::ThreadId;
use vac_protocol::config_types::AltScreenMode;
use vac_protocol::config_types::SandboxMode;
use vac_protocol::config_types::WindowsSandboxLevel;
use vac_rollout::state_db::get_state_db;
use vac_state::log_db;
use vac_utils_absolute_path::AbsolutePathBuf;
use vac_utils_absolute_path::canonicalize_existing_preserving_symlinks;
use vac_utils_oss::ensure_oss_provider_ready;
use vac_utils_oss::get_default_model_for_oss_provider;

mod legacy_core;

mod additional_dirs;
mod app;
mod app_backtrack;
mod app_command;
mod app_event;
mod app_event_sender;
mod app_server_approval_conversions;
mod approval_events;
mod ascii_animation;
mod runtime_owner_session;
#[allow(dead_code)]
mod audio_device {
    use crate::app_event::RealtimeAudioDeviceKind;

    pub(crate) fn list_realtime_audio_device_names(
        kind: RealtimeAudioDeviceKind,
    ) -> Result<Vec<String>, String> {
        Err(format!(
            "realtime {} selection is unavailable in the local coding tool build",
            kind.noun()
        ))
    }
}
mod bottom_pane;
mod chatwidget;
mod cli;
mod clipboard_copy;
mod clipboard_paste;
mod collaboration_modes;
mod color;
pub(crate) mod custom_terminal;
pub use custom_terminal::Terminal;
mod auto_review_denials;
mod capability_dashboard;
mod cwd_prompt;
mod debug_config;
mod diff_model;
mod diff_render;
mod exec_cell;
mod exec_command;
mod external_agent_config_migration;
mod external_agent_config_migration_startup;
mod external_editor;
mod file_search;
mod frames;
mod get_git_diff;
mod goal_display;
mod history_cell;
mod ide_context;
pub(crate) mod insert_history;
mod workflow_browser;
mod workflow_progress;
pub use insert_history::insert_history_lines;
mod key_hint;
mod keymap;
mod keymap_setup;
mod line_truncation;
pub(crate) mod live_wrap;
mod local_runtime_session;
mod owner_native_operation_parity;
mod owner_native_runtime_support;
pub use live_wrap::RowBuilder;
mod activity_sidebar;
#[cfg(feature = "provider-chatgpt")]
#[cfg(feature = "provider-chatgpt")]
mod provider_credential_import;
mod markdown;
mod markdown_render;
#[cfg(test)]
mod perf_bench_harness;
mod markdown_stream;
mod mention_codec;
mod model_catalog;
mod model_migration;
mod motion;
mod multi_agents;
mod notifications;
mod startup_task_graph;
#[cfg(any(not(debug_assertions), test))]
mod npm_registry;
pub(crate) mod onboarding;
mod operator_style;
mod operator_console;
mod operator_ui;
mod operator_ui_styles;
mod operator_widget_render;
mod oss_selection;
mod pager_overlay;
#[cfg(test)]
mod palette_surface;
mod permission_compat;
pub(crate) mod public_widgets;
mod render;
mod resize_reflow_cap;
mod resume_picker;
mod runtime_adapter;
mod selection_list;
mod session_log;
mod session_protocol;
mod session_resume;
mod session_state;
mod shimmer;
mod skills_helpers;
mod slash_command;
mod status;
mod status_indicator_widget;
mod streaming;
mod style;
mod surface_route_catalog;
mod terminal_palette;
mod terminal_probe;
mod terminal_title;
mod text_formatting;
mod theme_picker;
mod token_usage;
mod tooltips;
mod transcript_reflow;
mod tui;
mod ui_consts;
pub(crate) mod update_action;
pub use update_action::UpdateAction;
#[cfg(not(debug_assertions))]
pub use update_action::get_update_action;
mod update_prompt;
#[cfg(any(not(debug_assertions), test))]
mod update_versions;
mod updates;
mod version;
mod width;
#[allow(dead_code)]
mod voice {
    use crate::app_event_sender::AppEventSender;
    use crate::legacy_core::config::Config;
    use crate::session_protocol::ThreadRealtimeAudioChunk;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU16};

    pub struct VoiceCapture;
    pub(crate) struct RecordingMeterState;
    pub(crate) struct RealtimeAudioPlayer;

    impl VoiceCapture {
        pub fn start_realtime(_config: &Config, _tx: AppEventSender) -> Result<Self, String> {
            Err("voice input was removed from the local coding tool build".to_string())
        }
        pub fn stop(self) {}
        pub fn stopped_flag(&self) -> Arc<AtomicBool> { Arc::new(AtomicBool::new(true)) }
        pub fn last_peak_arc(&self) -> Arc<AtomicU16> { Arc::new(AtomicU16::new(0)) }
    }
    impl RecordingMeterState {
        pub(crate) fn new() -> Self { Self }
        pub(crate) fn next_text(&mut self, _peak: u16) -> String { "voice removed".to_string() }
    }
    impl RealtimeAudioPlayer {
        pub(crate) fn start(_config: &Config) -> Result<Self, String> {
            Err("voice output was removed from the local coding tool build".to_string())
        }
        pub(crate) fn enqueue_frame(&self, _frame: &ThreadRealtimeAudioChunk) -> Result<(), String> {
            Err("voice output was removed from the local coding tool build".to_string())
        }
        pub(crate) fn clear(&self) {}
    }
}

mod wrapping;

#[cfg(test)]
pub(crate) mod test_backend;
#[cfg(test)]
pub(crate) mod test_support;

use crate::onboarding::onboarding_screen::OnboardingScreenArgs;
use crate::onboarding::onboarding_screen::run_onboarding_app;
use crate::tui::Tui;
pub use cli::Cli;
pub use markdown_render::render_markdown_text;
pub use public_widgets::composer_input::ComposerAction;
pub use public_widgets::composer_input::ComposerInput;
use vac_arg0::Arg0DispatchPaths;
// (tests access modules directly within the crate)

#[allow(clippy::too_many_arguments)]
async fn start_embedded_app_server(
    arg0_paths: Arg0DispatchPaths,
    config: Config,
    cli_kv_overrides: Vec<(String, toml::Value)>,
    loader_overrides: LoaderOverrides,
    feedback: vac_feedback::VACFeedback,
    log_db: Option<log_db::LogDbLayer>,
    environment_manager: Arc<EnvironmentManager>,
) -> color_eyre::Result<InProcessAppServerClient> {
    start_embedded_app_server_with(
        arg0_paths,
        config,
        cli_kv_overrides,
        loader_overrides,
        feedback,
        log_db,
        environment_manager,
        InProcessAppServerClient::start,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn start_app_server(
    arg0_paths: Arg0DispatchPaths,
    config: Config,
    cli_kv_overrides: Vec<(String, toml::Value)>,
    loader_overrides: LoaderOverrides,
    feedback: vac_feedback::VACFeedback,
    log_db: Option<log_db::LogDbLayer>,
    environment_manager: Arc<EnvironmentManager>,
) -> color_eyre::Result<AppServerClient> {
    start_embedded_app_server(
        arg0_paths,
        config,
        cli_kv_overrides,
        loader_overrides,
        feedback,
        log_db,
        environment_manager,
    )
    .await
    .map(AppServerClient::InProcess)
}

pub(crate) async fn start_app_server_for_picker(
    config: &Config,
    environment_manager: Arc<EnvironmentManager>,
) -> color_eyre::Result<impl LocalRuntimeSession<RequestHandle = TuiSessionRequestHandle> + use<>> {
    let app_server = start_app_server(
        Arg0DispatchPaths::default(),
        config.clone(),
        Vec::new(),
        LoaderOverrides::default(),
        vac_feedback::VACFeedback::new(),
        /*log_db*/ None,
        environment_manager,
    )
    .await?;
    Ok(into_local_runtime_session(app_server))
}

#[cfg(test)]
pub(crate) async fn start_embedded_app_server_for_picker(
    config: &Config,
) -> color_eyre::Result<impl LocalRuntimeSession<RequestHandle = TuiSessionRequestHandle> + use<>> {
    start_app_server_for_picker(config, Arc::new(EnvironmentManager::default_for_tests())).await
}

#[allow(clippy::too_many_arguments)]
async fn start_embedded_app_server_with<F, Fut>(
    arg0_paths: Arg0DispatchPaths,
    config: Config,
    cli_kv_overrides: Vec<(String, toml::Value)>,
    loader_overrides: LoaderOverrides,
    feedback: vac_feedback::VACFeedback,
    log_db: Option<log_db::LogDbLayer>,
    environment_manager: Arc<EnvironmentManager>,
    start_client: F,
) -> color_eyre::Result<InProcessAppServerClient>
where
    F: FnOnce(InProcessClientStartArgs) -> Fut,
    Fut: Future<Output = std::io::Result<InProcessAppServerClient>>,
{
    let config_warnings = config
        .startup_warnings
        .iter()
        .map(|warning| ConfigWarningNotification {
            summary: warning.clone(),
            details: None,
            path: None,
            range: None,
        })
        .collect();
    let client = start_client(InProcessClientStartArgs {
        arg0_paths,
        config: Arc::new(config),
        cli_overrides: cli_kv_overrides,
        loader_overrides,
        feedback,
        log_db,
        environment_manager,
        config_warnings,
        session_source: serde_json::from_value(serde_json::json!("cli"))
            .unwrap_or_else(|err| panic!("cli session source should deserialize: {err}")),
        enable_vac_api_key_env: false,
        client_name: "vac-tui".to_string(),
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        experimental_api: true,
        opt_out_notification_methods: Vec::new(),
        channel_capacity: DEFAULT_IN_PROCESS_CHANNEL_CAPACITY,
    })
    .await
    .wrap_err("failed to start embedded app server")?;
    Ok(client)
}

async fn shutdown_app_server_if_present<S: LocalRuntimeSession>(app_server: Option<S>) {
    if let Some(app_server) = app_server
        && let Err(err) = app_server.shutdown().await
    {
        warn!(%err, "Failed to shut down temporary embedded app server");
    }
}

fn session_target_from_app_server_thread(
    thread: AppServerThread,
) -> Option<resume_picker::SessionTarget> {
    match ThreadId::from_string(&thread.id) {
        Ok(thread_id) => Some(resume_picker::SessionTarget {
            path: thread.path,
            thread_id,
        }),
        Err(err) => {
            warn!(
                thread_id = thread.id,
                %err,
                "Ignoring app-server thread with invalid thread id during TUI session lookup"
            );
            None
        }
    }
}

async fn lookup_session_target_by_name_with_app_server<S>(
    app_server: &mut S,
    name: &str,
) -> color_eyre::Result<Option<resume_picker::SessionTarget>>
where
    S: LocalRuntimeSession,
{
    let mut cursor = None;
    loop {
        let response = app_server
            .thread_list(ThreadListParams {
                cursor: cursor.clone(),
                limit: Some(100),
                sort_key: Some(AppServerThreadSortKey::UpdatedAt),
                sort_direction: None,
                model_providers: None,
                source_kinds: Some(vec![ThreadSourceKind::Cli, ThreadSourceKind::VsCode]),
                archived: Some(false),
                cwd: None,
                use_state_db_only: false,
                search_term: Some(name.to_string()),
            })
            .await?;
        if let Some(thread) = response
            .data
            .into_iter()
            .find(|thread| thread.name.as_deref() == Some(name))
        {
            return Ok(session_target_from_app_server_thread(thread));
        }
        if response.next_cursor.is_none() {
            return Ok(None);
        }
        cursor = response.next_cursor;
    }
}

async fn lookup_session_target_with_app_server<S>(
    app_server: &mut S,
    id_or_name: &str,
) -> color_eyre::Result<Option<resume_picker::SessionTarget>>
where
    S: LocalRuntimeSession,
{
    if Uuid::parse_str(id_or_name).is_ok() {
        let thread_id = match ThreadId::from_string(id_or_name) {
            Ok(thread_id) => thread_id,
            Err(err) => {
                warn!(
                    session = id_or_name,
                    %err,
                    "Failed to parse session id during TUI lookup"
                );
                return Ok(None);
            }
        };
        return match app_server
            .thread_read(thread_id, /*include_turns*/ false)
            .await
        {
            Ok(thread) => Ok(session_target_from_app_server_thread(thread)),
            Err(err) => {
                warn!(
                    session = id_or_name,
                    %err,
                    "thread/read failed during TUI session lookup"
                );
                Ok(None)
            }
        };
    }

    lookup_session_target_by_name_with_app_server(app_server, id_or_name).await
}

async fn lookup_latest_session_target_with_app_server<S>(
    app_server: &mut S,
    config: &Config,
    cwd_filter: Option<&Path>,
    include_non_interactive: bool,
) -> color_eyre::Result<Option<resume_picker::SessionTarget>>
where
    S: LocalRuntimeSession,
{
    let response = app_server
        .thread_list(latest_session_lookup_params(
            config,
            cwd_filter,
            include_non_interactive,
        ))
        .await?;
    Ok(response
        .data
        .into_iter()
        .find_map(session_target_from_app_server_thread))
}

fn latest_session_lookup_params(
    config: &Config,
    cwd_filter: Option<&Path>,
    include_non_interactive: bool,
) -> ThreadListParams {
    ThreadListParams {
        cursor: None,
        limit: Some(1),
        sort_key: Some(AppServerThreadSortKey::UpdatedAt),
        sort_direction: None,
        model_providers: Some(vec![config.model_provider_id.clone()]),
        source_kinds: (!include_non_interactive)
            .then_some(vec![ThreadSourceKind::Cli, ThreadSourceKind::VsCode]),
        archived: Some(false),
        cwd: cwd_filter.map(|cwd| ThreadListCwdFilter::One(cwd.to_string_lossy().to_string())),
        use_state_db_only: false,
        search_term: None,
    }
}

fn config_cwd_for_app_server_target(
    cwd: Option<&Path>,
    environment_manager: &EnvironmentManager,
) -> std::io::Result<Option<AbsolutePathBuf>> {
    if environment_manager
        .default_environment()
        .is_some_and(|environment| environment.is_remote())
    {
        return Ok(None);
    }

    let cwd = match cwd {
        Some(path) => {
            AbsolutePathBuf::from_absolute_path(canonicalize_existing_preserving_symlinks(path)?)
        }
        None => AbsolutePathBuf::current_dir(),
    }?;
    Ok(Some(cwd))
}


fn startup_profile_enabled() -> bool {
    std::env::var_os("VAC_TUI_PROFILE_STARTUP").is_some()
}

fn log_startup_phase(started_at: Instant, phase: &str) {
    if startup_profile_enabled() {
        tracing::info!(
            target: "vac_tui::startup_profile",
            phase,
            elapsed_ms = started_at.elapsed().as_millis(),
            "TUI startup phase"
        );
    }
}

fn latest_session_cwd_filter<'a>(config: &'a Config, show_all: bool) -> Option<&'a Path> {
    if show_all {
        return None;
    }

    Some(config.cwd.as_path())
}

pub async fn run_main(
    mut cli: Cli,
    arg0_paths: Arg0DispatchPaths,
    loader_overrides: LoaderOverrides,
) -> std::io::Result<AppExitInfo> {
    let startup_started_at = Instant::now();
    log_startup_phase(startup_started_at, "run_main_enter");
    startup_task_graph::startup_task_graph().log_contract();
    let (sandbox_mode, approval_policy) = if cli.dangerously_bypass_approvals_and_sandbox {
        (
            Some(SandboxMode::DangerFullAccess),
            Some(AskForApproval::Never),
        )
    } else {
        (
            cli.sandbox_mode.map(Into::<SandboxMode>::into),
            cli.approval_policy.map(Into::into),
        )
    };

    // Map the legacy --search flag to the canonical web_search mode.
    if cli.web_search {
        cli.config_overrides
            .raw_overrides
            .push("web_search=\"live\"".to_string());
    }

    // When using `--oss`, let the bootstrapper pick the model (defaulting to
    // gpt-oss:20b) and ensure it is present locally. Also, force the built‑in
    let raw_overrides = cli.config_overrides.raw_overrides.clone();
    // `oss` model provider.
    let overrides_cli = vac_utils_cli::CliConfigOverrides { raw_overrides };
    let cli_kv_overrides = match overrides_cli.parse_overrides() {
        // Parse `-c` overrides from the CLI.
        Ok(v) => v,
        #[allow(clippy::print_stderr)]
        Err(e) => {
            eprintln!("Error parsing -c overrides: {e}");
            std::process::exit(1);
        }
    };

    // we load config.toml here to determine project state.
    #[allow(clippy::print_stderr)]
    let vac_home = match find_vac_home() {
        Ok(vac_home) => vac_home.to_path_buf(),
        Err(err) => {
            eprintln!("Error finding vac home: {err}");
            std::process::exit(1);
        }
    };

    log_startup_phase(startup_started_at, "vac_home_resolved");

    let environment_manager = Arc::new(
        EnvironmentManager::new(EnvironmentManagerArgs::new(
            ExecServerRuntimePaths::from_optional_paths(
                arg0_paths.vac_self_exe.clone(),
                arg0_paths.vac_linux_sandbox_exe.clone(),
            )?,
        ))
        .await,
    );
    let cwd = cli.cwd.clone();
    let config_cwd = config_cwd_for_app_server_target(cwd.as_deref(), &environment_manager)?;

    #[allow(clippy::print_stderr)]
    log_startup_phase(startup_started_at, "environment_manager_ready");

    let config_toml = match load_config_as_toml_with_cli_overrides(
        &vac_home,
        config_cwd.as_ref(),
        cli_kv_overrides.clone(),
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

    if let Err(err) = crate::legacy_core::personality_migration::maybe_migrate_personality(
        &vac_home,
        &config_toml,
    )
    .await
    {
        tracing::warn!(error = %err, "failed to run personality migration");
    }

    let model_provider_override = if cli.oss {
        let resolved = resolve_oss_provider(
            cli.oss_provider.as_deref(),
            &config_toml,
            cli.config_profile.clone(),
        );

        if let Some(provider) = resolved {
            Some(provider)
        } else {
            // No provider configured, prompt the user
            let provider = oss_selection::select_oss_provider(&vac_home).await?;
            if provider == "__CANCELLED__" {
                return Err(std::io::Error::other(
                    "OSS provider selection was cancelled by user",
                ));
            }
            Some(provider)
        }
    } else {
        None
    };

    // When using `--oss`, let the bootstrapper pick the model based on selected provider
    let model = if let Some(model) = &cli.model {
        Some(model.clone())
    } else if cli.oss {
        // Use the provider from model_provider_override
        model_provider_override
            .as_ref()
            .and_then(|provider_id| get_default_model_for_oss_provider(provider_id))
            .map(std::borrow::ToOwned::to_owned)
    } else {
        None // No model specified, will use the default.
    };

    let additional_dirs = cli.add_dir.clone();

    let overrides = ConfigOverrides {
        model,
        approval_policy,
        sandbox_mode,
        cwd,
        model_provider: model_provider_override.clone(),
        config_profile: cli.config_profile.clone(),
        vac_self_exe: arg0_paths.vac_self_exe.clone(),
        vac_linux_sandbox_exe: arg0_paths.vac_linux_sandbox_exe.clone(),
        main_execve_wrapper_exe: arg0_paths.main_execve_wrapper_exe.clone(),
        show_raw_agent_reasoning: cli.oss.then_some(true),
        additional_writable_roots: additional_dirs,
        ..Default::default()
    };

    log_startup_phase(startup_started_at, "config_toml_loaded");

    let config = load_config_or_exit(cli_kv_overrides.clone(), overrides.clone()).await;
    log_startup_phase(startup_started_at, "config_resolved");

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

    if let Some(warning) = add_dir_warning_message(
        &cli.add_dir,
        &config.permissions.permission_profile(),
        config.cwd.as_path(),
    ) {
        #[allow(clippy::print_stderr)]
        {
            eprintln!("Error adding directories: {warning}");
            std::process::exit(1);
        }
    }

    {
        #[allow(clippy::print_stderr)]
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
    }

    let log_dir = crate::legacy_core::config::log_dir(&config)?;
    std::fs::create_dir_all(&log_dir)?;
    // Open (or create) your log file, appending to it.
    let mut log_file_opts = OpenOptions::new();
    log_file_opts.create(true).append(true);

    // Ensure the file is only readable and writable by the current user.
    // Doing the equivalent to `chmod 600` on Windows is quite a bit more code
    // and requires the Windows API crates, so we can reconsider that when
    // VAC CLI is officially supported on Windows.
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        log_file_opts.mode(0o600);
    }

    let log_file = log_file_opts.open(log_dir.join("vac-tui.log"))?;

    // Wrap file in non‑blocking writer.
    let (non_blocking, _guard) = non_blocking(log_file);

    // use RUST_LOG env var, default to info for vac crates.
    let env_filter = || {
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("vac_core=info,vac_tui=info,vac_rmcp_client=info"))
    };

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        // `with_target(true)` is the default, but we previously disabled it for file output.
        // Keep it enabled so we can selectively enable targets via `RUST_LOG=...` and then
        // grep for a specific module/target while troubleshooting.
        .with_target(true)
        .with_ansi(false)
        .with_span_events(
            tracing_subscriber::fmt::format::FmtSpan::NEW
                | tracing_subscriber::fmt::format::FmtSpan::CLOSE,
        )
        .with_filter(env_filter());

    let feedback = vac_feedback::VACFeedback::new();
    let feedback_layer = feedback.logger_layer();
    let feedback_metadata_layer = feedback.metadata_layer();

    if cli.oss && model_provider_override.is_some() {
        // We're in the oss section, so provider_id should be Some
        // Let's handle None case gracefully though just in case
        let provider_id = match model_provider_override.as_ref() {
            Some(id) => id,
            None => {
                error!("OSS provider unexpectedly not set when oss flag is used");
                return Err(std::io::Error::other(
                    "OSS provider not set but oss flag was used",
                ));
            }
        };
        ensure_oss_provider_ready(provider_id, &config).await?;
    }

    let otel = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        crate::legacy_core::otel_init::build_provider(
            &config,
            env!("CARGO_PKG_VERSION"),
            /*service_name_override*/ None,
            /*default_analytics_enabled*/ true,
        )
    })) {
        Ok(Ok(otel)) => otel,
        Ok(Err(e)) => {
            #[allow(clippy::print_stderr)]
            {
                eprintln!("Could not create otel exporter: {e}");
            }
            None
        }
        Err(_) => {
            #[allow(clippy::print_stderr)]
            {
                eprintln!("Could not create otel exporter: panicked during initialization");
            }
            None
        }
    };

    let otel_logger_layer = otel.as_ref().and_then(|o| o.logger_layer());

    let otel_tracing_layer = otel.as_ref().and_then(|o| o.tracing_layer());

    let log_db = get_state_db(&config).await.map(log_db::start);
    let log_db_layer = log_db
        .clone()
        .map(|layer| layer.with_filter(Targets::new().with_default(Level::TRACE)));

    log_startup_phase(startup_started_at, "telemetry_layers_ready");

    let _ = tracing_subscriber::registry()
        .with(file_layer)
        .with(feedback_layer)
        .with(feedback_metadata_layer)
        .with(log_db_layer)
        .with(otel_logger_layer)
        .with(otel_tracing_layer)
        .try_init();

    log_startup_phase(startup_started_at, "before_first_ratatui_frame");

    run_ratatui_app(
        startup_started_at,
        cli,
        arg0_paths,
        loader_overrides,
        config,
        overrides,
        cli_kv_overrides,
        feedback,
        log_db,
        environment_manager,
    )
    .await
    .map_err(|err| std::io::Error::other(err.to_string()))
}

fn render_startup_skeleton(tui: &mut Tui, message: &str) -> color_eyre::Result<()> {
    use ratatui::widgets::Paragraph;
    use ratatui::widgets::Widget;
    let text = format!("VAC starting… {message}\nLoading config, local runtime, MCP, and skills in background-safe phases.");
    tui.draw(2, |frame| {
        Paragraph::new(text.as_str()).render(frame.area(), frame.buffer);
    })?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_ratatui_app(
    startup_started_at: Instant,
    cli: Cli,
    arg0_paths: Arg0DispatchPaths,
    loader_overrides: LoaderOverrides,
    initial_config: Config,
    overrides: ConfigOverrides,
    cli_kv_overrides: Vec<(String, toml::Value)>,
    feedback: vac_feedback::VACFeedback,
    log_db: Option<log_db::LogDbLayer>,
    environment_manager: Arc<EnvironmentManager>,
) -> color_eyre::Result<AppExitInfo> {
    color_eyre::install()?;

    tooltips::announcement::prewarm();

    // Forward panic reports through tracing so they appear in the UI status
    // line, but do not swallow the default/color-eyre panic handler.
    // Chain to the previous hook so users still get a rich panic report
    // (including backtraces) after we restore the terminal.
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("panic: {info}");
        prev_hook(info);
    }));
    let mut terminal = tui::init()?;
    terminal.clear()?;

    let mut tui = Tui::new(terminal);
    render_startup_skeleton(&mut tui, "initializing local VAC runtime")?;
    let skeleton_rendered_at = Instant::now();
    log_startup_phase(skeleton_rendered_at, "skeleton_first_frame_rendered");
    let startup_metrics = startup_task_graph::StartupTaskGraphMetrics::from_skeleton_frame(
        startup_started_at,
        skeleton_rendered_at,
    );
    if let Err(err) = startup_task_graph::write_startup_metrics_registry(
        initial_config.cwd.as_path(),
        &startup_metrics,
    ) {
        tracing::debug!(error = %err, "failed to persist TUI startup metrics registry");
    }
    let mut terminal_restore_guard = TerminalRestoreGuard::new();

    #[cfg(not(debug_assertions))]
    {
        use crate::update_prompt::UpdatePromptOutcome;

        let skip_update_prompt = cli.prompt.as_ref().is_some_and(|prompt| !prompt.is_empty());
        if !skip_update_prompt {
            match update_prompt::run_update_prompt_if_needed(&mut tui, &initial_config).await? {
                UpdatePromptOutcome::Continue => {}
                UpdatePromptOutcome::RunUpdate(action) => {
                    terminal_restore_guard.restore()?;
                    return Ok(AppExitInfo {
                        token_usage: crate::token_usage::TokenUsage::default(),
                        thread_id: None,
                        thread_name: None,
                        update_action: Some(action),
                        exit_reason: ExitReason::UserRequested,
                    });
                }
            }
        }
    }

    // Initialize high-fidelity session event logging if enabled.
    session_log::maybe_init(&initial_config);

    let mut app_server = Some(
        match start_app_server(
            arg0_paths.clone(),
            initial_config.clone(),
            cli_kv_overrides.clone(),
            loader_overrides.clone(),
            feedback.clone(),
            log_db.clone(),
            environment_manager.clone(),
        )
        .await
        {
            Ok(app_server) => into_local_runtime_session(app_server),
            Err(err) => {
                terminal_restore_guard.restore_silently();
                session_log::log_session_end();
                return Err(err);
            }
        },
    );

    let should_show_trust_screen_flag = should_show_trust_screen(&initial_config);
    let mut trust_decision_was_made = false;
    let login_status = if initial_config.model_provider.requires_vastar_auth {
        let Some(app_server) = app_server.as_mut() else {
            unreachable!("app server should exist when auth is required");
        };
        get_login_status(app_server, &initial_config).await?
    } else {
        LoginStatus::NotAuthenticated
    };
    let should_show_onboarding =
        should_show_onboarding(login_status, &initial_config, should_show_trust_screen_flag);

    let config = if should_show_onboarding {
        let show_login_screen = should_show_login_screen(login_status, &initial_config);
        let onboarding_result = run_onboarding_app(
            OnboardingScreenArgs {
                show_login_screen,
                show_trust_screen: should_show_trust_screen_flag,
                login_status,
                app_server_request_handle: app_server
                    .as_ref()
                    .map(LocalRuntimeSession::request_handle),
                config: initial_config.clone(),
            },
            if show_login_screen {
                app_server.as_mut()
            } else {
                None
            },
            &mut tui,
        )
        .await?;
        if onboarding_result.should_exit {
            shutdown_app_server_if_present(app_server.take()).await;
            terminal_restore_guard.restore_silently();
            session_log::log_session_end();
            let _ = tui.terminal.clear();
            return Ok(AppExitInfo {
                token_usage: crate::token_usage::TokenUsage::default(),
                thread_id: None,
                thread_name: None,
                update_action: None,
                exit_reason: ExitReason::UserRequested,
            });
        }
        trust_decision_was_made = onboarding_result.directory_trust_decision.is_some();
        // If this onboarding run included the login step, reload config so
        // persisted auth/trust changes are reflected in the current process.
        if show_login_screen {}

        // If the user made an explicit trust decision, or we showed the login flow, reload config
        // so current process state reflects persisted trust/auth changes.
        if onboarding_result.directory_trust_decision.is_some() || show_login_screen {
            load_config_or_exit(cli_kv_overrides.clone(), overrides.clone()).await
        } else {
            initial_config
        }
    } else {
        initial_config
    };

    let mut missing_session_exit = |id_str: &str, action: &str| {
        error!("Error finding conversation path: {id_str}");
        terminal_restore_guard.restore_silently();
        session_log::log_session_end();
        let _ = tui.terminal.clear();
        Ok(AppExitInfo {
            token_usage: crate::token_usage::TokenUsage::default(),
            thread_id: None,
            thread_name: None,
            update_action: None,
            exit_reason: ExitReason::Fatal(format!(
                "No saved session found with ID {id_str}. Run `vac {action}` without an ID to choose from existing sessions."
            )),
        })
    };

    let use_branch = cli.branch_picker || cli.branch_last || cli.branch_session_id.is_some();
    let session_selection = if use_branch {
        if let Some(id_str) = cli.branch_session_id.as_deref() {
            let Some(startup_app_server) = app_server.as_mut() else {
                unreachable!("app server should be initialized for --branch <id>");
            };
            match lookup_session_target_with_app_server(startup_app_server, id_str).await? {
                Some(target_session) => resume_picker::SessionSelection::Branch(target_session),
                None => {
                    shutdown_app_server_if_present(app_server.take()).await;
                    return missing_session_exit(id_str, "branch");
                }
            }
        } else if cli.branch_last {
            let filter_cwd = None;
            let Some(app_server) = app_server.as_mut() else {
                unreachable!("app server should be initialized for --branch --last");
            };
            match lookup_latest_session_target_with_app_server(
                app_server, &config, filter_cwd, /*include_non_interactive*/ false,
            )
            .await?
            {
                Some(target_session) => resume_picker::SessionSelection::Branch(target_session),
                None => resume_picker::SessionSelection::StartFresh,
            }
        } else if cli.branch_picker {
            let Some(app_server) = app_server.take() else {
                unreachable!("app server should be initialized for --branch picker");
            };
            match resume_picker::run_branch_picker_with_app_server(
                &mut tui,
                &config,
                cli.branch_show_all,
                app_server,
            )
            .await?
            {
                resume_picker::SessionSelection::Exit => {
                    terminal_restore_guard.restore_silently();
                    session_log::log_session_end();
                    return Ok(AppExitInfo {
                        token_usage: crate::token_usage::TokenUsage::default(),
                        thread_id: None,
                        thread_name: None,
                        update_action: None,
                        exit_reason: ExitReason::UserRequested,
                    });
                }
                other => other,
            }
        } else {
            resume_picker::SessionSelection::StartFresh
        }
    } else if let Some(id_str) = cli.resume_session_id.as_deref() {
        let Some(startup_app_server) = app_server.as_mut() else {
            unreachable!("app server should be initialized for --resume <id>");
        };
        match lookup_session_target_with_app_server(startup_app_server, id_str).await? {
            Some(target_session) => resume_picker::SessionSelection::Resume(target_session),
            None => {
                shutdown_app_server_if_present(app_server.take()).await;
                return missing_session_exit(id_str, "resume");
            }
        }
    } else if cli.resume_last {
        let filter_cwd = latest_session_cwd_filter(&config, cli.resume_show_all);
        let Some(app_server) = app_server.as_mut() else {
            unreachable!("app server should be initialized for --resume --last");
        };
        match lookup_latest_session_target_with_app_server(
            app_server,
            &config,
            filter_cwd,
            cli.resume_include_non_interactive,
        )
        .await?
        {
            Some(target_session) => resume_picker::SessionSelection::Resume(target_session),
            None => resume_picker::SessionSelection::StartFresh,
        }
    } else if cli.resume_picker {
        let Some(app_server) = app_server.take() else {
            unreachable!("app server should be initialized for --resume picker");
        };
        match resume_picker::run_resume_picker_with_app_server(
            &mut tui,
            &config,
            cli.resume_show_all,
            cli.resume_include_non_interactive,
            app_server,
        )
        .await?
        {
            resume_picker::SessionSelection::Exit => {
                terminal_restore_guard.restore_silently();
                session_log::log_session_end();
                return Ok(AppExitInfo {
                    token_usage: crate::token_usage::TokenUsage::default(),
                    thread_id: None,
                    thread_name: None,
                    update_action: None,
                    exit_reason: ExitReason::UserRequested,
                });
            }
            other => other,
        }
    } else {
        resume_picker::SessionSelection::StartFresh
    };

    let current_cwd = config.cwd.clone();
    let allow_prompt = cli.cwd.is_none();
    let action_and_target_session_if_resume_or_branch = match &session_selection {
        resume_picker::SessionSelection::Resume(target_session) => {
            Some((CwdPromptAction::Resume, target_session))
        }
        resume_picker::SessionSelection::Branch(target_session) => {
            Some((CwdPromptAction::Branch, target_session))
        }
        _ => None,
    };
    let fallback_cwd = match action_and_target_session_if_resume_or_branch {
        Some((action, target_session)) => match resolve_cwd_for_resume_or_branch(
            &mut tui,
            &config,
            &current_cwd,
            target_session.thread_id,
            target_session.path.as_deref(),
            action,
            allow_prompt,
        )
        .await?
        {
            ResolveCwdOutcome::Continue(cwd) => cwd,
            ResolveCwdOutcome::Exit => {
                terminal_restore_guard.restore_silently();
                session_log::log_session_end();
                return Ok(AppExitInfo {
                    token_usage: crate::token_usage::TokenUsage::default(),
                    thread_id: None,
                    thread_name: None,
                    update_action: None,
                    exit_reason: ExitReason::UserRequested,
                });
            }
        },
        None => None,
    };

    let mut config = match &session_selection {
        resume_picker::SessionSelection::Resume(_) | resume_picker::SessionSelection::Branch(_) => {
            load_config_or_exit_with_fallback_cwd(
                cli_kv_overrides.clone(),
                overrides.clone(),
                fallback_cwd,
            )
            .await
        }
        _ => config,
    };

    // Configure syntax highlighting theme from the final config — onboarding
    // and resume/branch can both reload config with a different tui_theme, so
    // this must happen after the last possible reload.
    if let Some(w) = crate::render::highlight::set_theme_override(
        config.tui_theme.clone(),
        find_vac_home().ok(),
    ) {
        config.startup_warnings.push(w);
    }

    if let Some(dialog) = build_project_workspace_confirmation_dialog(config.cwd.as_path()) {
        config.startup_warnings.push(format!(
            "{}
{}",
            project_workspace_startup_notice(config.cwd.as_path())
                .map(|notice| notice.render_tui_warning())
                .unwrap_or_else(|| "VAC project workspace setup is available.".to_string()),
            dialog.render_text()
        ));
    }

    set_default_client_residency_requirement(config.enforce_residency.value());
    let active_profile = config.active_profile.clone();
    let should_show_trust_screen = should_show_trust_screen(&config);
    let should_prompt_windows_sandbox_nux_at_startup = cfg!(target_os = "windows")
        && trust_decision_was_made
        && WindowsSandboxLevel::from_config(&config) == WindowsSandboxLevel::Disabled;

    let Cli {
        prompt,
        shared,
        no_alt_screen,
        ..
    } = cli;
    let images = shared.into_inner().images;

    let use_alt_screen = determine_alt_screen_mode(no_alt_screen, config.tui_alternate_screen);
    tui.set_alt_screen_enabled(use_alt_screen);
    if use_alt_screen {
        tui.enter_alt_screen()?;
    }
    let app_server = match app_server {
        Some(app_server) => app_server,
        None => match start_app_server(
            arg0_paths,
            config.clone(),
            cli_kv_overrides.clone(),
            loader_overrides,
            feedback.clone(),
            log_db.clone(),
            environment_manager.clone(),
        )
        .await
        {
            Ok(app_server) => into_local_runtime_session(app_server),
            Err(err) => {
                terminal_restore_guard.restore_silently();
                session_log::log_session_end();
                return Err(err);
            }
        },
    };

    let app_result = App::run(
        &mut tui,
        app_server,
        config,
        cli_kv_overrides.clone(),
        overrides.clone(),
        active_profile,
        prompt,
        images,
        session_selection,
        feedback,
        should_show_trust_screen, // Proxy to: is it a first run in this directory?
        should_show_trust_screen_flag, // Preserve the startup-time trust NUX signal before onboarding
        should_prompt_windows_sandbox_nux_at_startup,
        environment_manager,
    )
    .await;

    terminal_restore_guard.restore_silently();
    // Mark the end of the recorded session.
    session_log::log_session_end();
    // ignore error when collecting usage – report underlying error instead
    app_result
}

#[expect(
    clippy::print_stderr,
    reason = "TUI should no longer be displayed, so we can write to stderr."
)]
fn restore() {
    if let Err(err) = tui::restore_after_exit() {
        eprintln!(
            "failed to restore terminal. Run `reset` or restart your terminal to recover: {err}"
        );
    }
}

struct TerminalRestoreGuard {
    active: bool,
}

impl TerminalRestoreGuard {
    fn new() -> Self {
        Self { active: true }
    }

    #[cfg_attr(debug_assertions, allow(dead_code))]
    fn restore(&mut self) -> color_eyre::Result<()> {
        if self.active {
            crate::tui::restore_after_exit()?;
            self.active = false;
        }
        Ok(())
    }

    fn restore_silently(&mut self) {
        if self.active {
            restore();
            self.active = false;
        }
    }
}

impl Drop for TerminalRestoreGuard {
    fn drop(&mut self) {
        self.restore_silently();
    }
}

/// Determine whether to use the terminal's alternate screen buffer.
///
/// The alternate screen buffer provides the fullscreen operator experience expected from VAC.
/// Users who need inline scrollback can opt out with `--no-alt-screen` or
/// `tui.alternate_screen = "never"`.
///
/// This function implements a pragmatic workaround:
/// - If `--no-alt-screen` is explicitly passed, always disable alternate screen
/// - Otherwise, respect the `tui.alternate_screen` config setting:
///   - `always`: Use alternate screen everywhere (original behavior)
///   - `never`: Inline mode only, preserves scrollback
///   - `auto` (default): Use alternate screen as the product default
fn determine_alt_screen_mode(no_alt_screen: bool, tui_alternate_screen: AltScreenMode) -> bool {
    if no_alt_screen {
        false
    } else {
        match tui_alternate_screen {
            AltScreenMode::Always => true,
            AltScreenMode::Never => false,
            AltScreenMode::Auto => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginStatus {
    AuthMode(AppServerAuthMode),
    NotAuthenticated,
}

/// Determines the user's authentication mode using a lightweight account read
/// rather than a full `bootstrap`, avoiding the model-list fetch and
/// rate-limit round-trip that `bootstrap` would trigger.
async fn get_login_status<S>(app_server: &mut S, config: &Config) -> color_eyre::Result<LoginStatus>
where
    S: LocalRuntimeSession,
{
    if !config.model_provider.requires_vastar_auth {
        return Ok(LoginStatus::NotAuthenticated);
    }

    let account = app_server.read_account().await?;
    Ok(match account.account {
        Some(AppServerAccount::ApiKey {}) => LoginStatus::AuthMode(AppServerAuthMode::ApiKey),
        Some(AppServerAccount::Chatgpt { .. })
            if vac_protocol::auth::AuthMode::legacy_chatgpt_account_enabled() =>
        {
            LoginStatus::AuthMode(AppServerAuthMode::ProviderCredential)
        }
        Some(AppServerAccount::Chatgpt { .. }) => LoginStatus::NotAuthenticated,
        Some(AppServerAccount::AmazonBedrock {}) => LoginStatus::NotAuthenticated,
        None => LoginStatus::NotAuthenticated,
    })
}

async fn load_config_or_exit(
    cli_kv_overrides: Vec<(String, toml::Value)>,
    overrides: ConfigOverrides,
) -> Config {
    load_config_or_exit_with_fallback_cwd(cli_kv_overrides, overrides, /*fallback_cwd*/ None).await
}

async fn load_config_or_exit_with_fallback_cwd(
    cli_kv_overrides: Vec<(String, toml::Value)>,
    overrides: ConfigOverrides,
    fallback_cwd: Option<PathBuf>,
) -> Config {
    #[allow(clippy::print_stderr)]
    match ConfigBuilder::default()
        .cli_overrides(cli_kv_overrides)
        .harness_overrides(overrides)
        .fallback_cwd(fallback_cwd)
        .build()
        .await
    {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Error loading configuration: {err}");
            std::process::exit(1);
        }
    }
}

/// Determine if the user has decided whether to trust the current directory.
fn should_show_trust_screen(config: &Config) -> bool {
    config.active_project.trust_level.is_none()
}

fn should_show_onboarding(
    login_status: LoginStatus,
    config: &Config,
    show_trust_screen: bool,
) -> bool {
    if show_trust_screen {
        return true;
    }

    should_show_login_screen(login_status, config)
}

fn should_show_login_screen(login_status: LoginStatus, config: &Config) -> bool {
    // Only show the login screen for providers that actually require Vastar auth
    // (Vastar or equivalents). For OSS/other providers, skip login entirely.
    if !config.model_provider.requires_vastar_auth {
        return false;
    }

    login_status == LoginStatus::NotAuthenticated
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_runtime_session::LocalRuntimeStartedThread;
    use crate::legacy_core::config::ConfigBuilder;
    use crate::legacy_core::config::ConfigOverrides;
    use crate::session_protocol::AskForApproval;
    use pretty_assertions::assert_eq;
    use serial_test::serial;
    use tempfile::TempDir;
    use vac_config::config_toml::ProjectConfig;

    async fn build_config(temp_dir: &TempDir) -> std::io::Result<Config> {
        ConfigBuilder::default()
            .vac_home(temp_dir.path().to_path_buf())
            .build()
            .await
    }

    async fn start_test_embedded_app_server(
        config: Config,
    ) -> color_eyre::Result<InProcessAppServerClient> {
        start_embedded_app_server(
            Arg0DispatchPaths::default(),
            config,
            Vec::new(),
            LoaderOverrides::default(),
            vac_feedback::VACFeedback::new(),
            /*log_db*/ None,
            Arc::new(EnvironmentManager::default_for_tests()),
        )
        .await
    }

    #[test]
    fn session_target_display_label_falls_back_to_thread_id() {
        let thread_id = ThreadId::new();
        let target = crate::resume_picker::SessionTarget {
            path: None,
            thread_id,
        };

        assert_eq!(target.display_label(), format!("thread {thread_id}"));
    }

    #[tokio::test]
    async fn latest_session_lookup_params_keep_local_filters_for_embedded_sessions()
    -> std::io::Result<()> {
        let temp_dir = TempDir::new()?;
        let config = build_config(&temp_dir).await?;
        let cwd = temp_dir.path().join("project");

        let params = latest_session_lookup_params(
            &config,
            Some(cwd.as_path()),
            /*include_non_interactive*/ false,
        );

        assert_eq!(params.model_providers, Some(vec![config.model_provider_id]));
        assert_eq!(
            params.cwd,
            Some(ThreadListCwdFilter::One(cwd.to_string_lossy().to_string()))
        );
        Ok(())
    }

    #[tokio::test]
    async fn config_cwd_for_app_server_target_canonicalizes_embedded_cli_cwd() -> std::io::Result<()>
    {
        let temp_dir = TempDir::new()?;
        let environment_manager = EnvironmentManager::default_for_tests();

        let config_cwd =
            config_cwd_for_app_server_target(Some(temp_dir.path()), &environment_manager)?;

        assert_eq!(
            config_cwd,
            Some(AbsolutePathBuf::from_absolute_path(dunce::canonicalize(
                temp_dir.path()
            )?)?)
        );
        Ok(())
    }

    #[tokio::test]
    async fn config_cwd_for_app_server_target_errors_for_missing_embedded_cli_cwd()
    -> std::io::Result<()> {
        let temp_dir = TempDir::new()?;
        let missing = temp_dir.path().join("missing");
        let environment_manager = EnvironmentManager::default_for_tests();

        let err = config_cwd_for_app_server_target(Some(&missing), &environment_manager)
            .expect_err("missing embedded cwd should fail");

        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
        Ok(())
    }

    #[tokio::test]
    async fn config_cwd_for_app_server_target_omits_cwd_for_remote_exec_server()
    -> std::io::Result<()> {
        let remote_only_cwd = if cfg!(windows) {
            Path::new(r"C:\definitely\not\local\to\this\test")
        } else {
            Path::new("/definitely/not/local/to/this/test")
        };
        let environment_manager = EnvironmentManager::create_for_tests(
            Some("ws://127.0.0.1:8765".to_string()),
            ExecServerRuntimePaths::new(
                std::env::current_exe().expect("current exe"),
                /*vac_linux_sandbox_exe*/ None,
            )?,
        )
        .await;

        let config_cwd =
            config_cwd_for_app_server_target(Some(remote_only_cwd), &environment_manager)?;

        assert_eq!(config_cwd, None);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn windows_shows_trust_prompt_without_sandbox() -> std::io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut config = build_config(&temp_dir).await?;
        config.active_project = ProjectConfig { trust_level: None };
        config.set_windows_sandbox_enabled(/*value*/ false);

        let should_show = should_show_trust_screen(&config);
        assert!(
            should_show,
            "Trust prompt should be shown when project trust is undecided"
        );
        Ok(())
    }

    #[tokio::test]
    async fn embedded_app_server_supports_thread_start_rpc() -> color_eyre::Result<()> {
        let temp_dir = TempDir::new()?;
        let config = build_config(&temp_dir).await?;
        let mut app_server = into_local_runtime_session(AppServerClient::InProcess(
            start_test_embedded_app_server(config.clone()).await?,
        ));
        let response = app_server
            .start_thread(&config)
            .await
            .expect("thread/start should succeed");
        let (session, _turns) = response.into_parts();
        assert!(!session.thread_id.to_string().is_empty());

        app_server.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn lookup_session_target_by_name_uses_backend_title_search() -> color_eyre::Result<()> {
        Box::pin(async {
            let temp_dir = TempDir::new()?;
            let config = build_config(&temp_dir).await?;
            let thread_id = ThreadId::new();
            let rollout_path = temp_dir
                .path()
                .join("sessions/2025/02/01")
                .join(format!("rollout-2025-02-01T10-00-00-{thread_id}.jsonl"));
            let rollout_dir = rollout_path.parent().expect("rollout parent");
            std::fs::create_dir_all(rollout_dir)?;
            std::fs::write(&rollout_path, "")?;

            let state_runtime = vac_state::StateRuntime::init(
                config.vac_home.to_path_buf(),
                config.model_provider_id.clone(),
            )
            .await
            .map_err(std::io::Error::other)?;
            state_runtime
                .mark_backfill_complete(/*last_watermark*/ None)
                .await
                .map_err(std::io::Error::other)?;

            let session_cwd = temp_dir.path().join("project");
            std::fs::create_dir_all(&session_cwd)?;
            let created_at = chrono::DateTime::parse_from_rfc3339("2025-02-01T10:00:00Z")
                .expect("timestamp should parse")
                .with_timezone(&chrono::Utc);
            let mut builder = vac_state::ThreadMetadataBuilder::new(
                thread_id,
                rollout_path.clone(),
                created_at,
                serde_json::from_value(serde_json::json!("cli"))
                    .expect("cli session source should deserialize"),
            );
            builder.cwd = session_cwd;
            let mut metadata = builder.build(config.model_provider_id.as_str());
            metadata.title = "saved-session".to_string();
            metadata.first_user_message = Some("preview text".to_string());
            state_runtime
                .upsert_thread(&metadata)
                .await
                .map_err(std::io::Error::other)?;

            let mut app_server = into_local_runtime_session(AppServerClient::InProcess(
                start_test_embedded_app_server(config).await?,
            ));
            let target =
                lookup_session_target_by_name_with_app_server(&mut app_server, "saved-session")
                    .await?;
            let target = target.expect("name lookup should find the saved thread");
            assert_eq!(target.path, Some(rollout_path));
            assert_eq!(target.thread_id, thread_id);

            app_server.shutdown().await?;
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn embedded_app_server_start_failure_is_returned() -> color_eyre::Result<()> {
        let temp_dir = TempDir::new()?;
        let config = build_config(&temp_dir).await?;
        let result = start_embedded_app_server_with(
            Arg0DispatchPaths::default(),
            config,
            Vec::new(),
            LoaderOverrides::default(),
            vac_feedback::VACFeedback::new(),
            /*log_db*/ None,
            Arc::new(EnvironmentManager::default_for_tests()),
            |_args| async { Err(std::io::Error::other("boom")) },
        )
        .await;
        let err = match result {
            Ok(_) => panic!("startup failure should be returned"),
            Err(err) => err,
        };

        assert!(
            err.to_string()
                .contains("failed to start embedded app server"),
            "error should preserve the embedded app server startup context"
        );
        Ok(())
    }
    #[tokio::test]
    #[serial]
    async fn windows_shows_trust_prompt_with_sandbox() -> std::io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut config = build_config(&temp_dir).await?;
        config.active_project = ProjectConfig { trust_level: None };
        config.set_windows_sandbox_enabled(/*value*/ true);

        let should_show = should_show_trust_screen(&config);
        if cfg!(target_os = "windows") {
            assert!(
                should_show,
                "Windows trust prompt should be shown on native Windows with sandbox enabled"
            );
        } else {
            assert!(
                should_show,
                "Non-Windows should still show trust prompt when project is untrusted"
            );
        }
        Ok(())
    }
    #[tokio::test]
    async fn untrusted_project_skips_trust_prompt() -> std::io::Result<()> {
        use vac_protocol::config_types::TrustLevel;
        let temp_dir = TempDir::new()?;
        let mut config = build_config(&temp_dir).await?;
        config.active_project = ProjectConfig {
            trust_level: Some(TrustLevel::Untrusted),
        };

        let should_show = should_show_trust_screen(&config);
        assert!(
            !should_show,
            "Trust prompt should not be shown for projects explicitly marked as untrusted"
        );
        Ok(())
    }

    #[tokio::test]
    async fn config_rebuild_changes_trust_defaults_with_cwd() -> std::io::Result<()> {
        let temp_dir = TempDir::new()?;
        let vac_home = temp_dir.path().to_path_buf();
        let trusted = temp_dir.path().join("trusted");
        let untrusted = temp_dir.path().join("untrusted");
        std::fs::create_dir_all(&trusted)?;
        std::fs::create_dir_all(&untrusted)?;

        // TOML keys need escaped backslashes on Windows paths.
        let trusted_display = trusted.display().to_string().replace('\\', "\\\\");
        let untrusted_display = untrusted.display().to_string().replace('\\', "\\\\");
        let config_toml = format!(
            r#"[projects."{trusted_display}"]
trust_level = "trusted"

[projects."{untrusted_display}"]
trust_level = "untrusted"
"#
        );
        std::fs::write(temp_dir.path().join("config.toml"), config_toml)?;

        let trusted_overrides = ConfigOverrides {
            cwd: Some(trusted.clone()),
            ..Default::default()
        };
        let trusted_config = ConfigBuilder::default()
            .loader_overrides(LoaderOverrides::without_managed_config_for_tests())
            .vac_home(vac_home.clone())
            .harness_overrides(trusted_overrides.clone())
            .build()
            .await?;
        assert_eq!(
            AskForApproval::from(trusted_config.permissions.approval_policy.value()),
            AskForApproval::OnRequest
        );

        let untrusted_overrides = ConfigOverrides {
            cwd: Some(untrusted),
            ..trusted_overrides
        };
        let untrusted_config = ConfigBuilder::default()
            .loader_overrides(LoaderOverrides::without_managed_config_for_tests())
            .vac_home(vac_home)
            .harness_overrides(untrusted_overrides)
            .build()
            .await?;
        assert_eq!(
            AskForApproval::from(untrusted_config.permissions.approval_policy.value()),
            AskForApproval::UnlessTrusted
        );
        Ok(())
    }

    /// Regression: theme must be configured from the *final* config.
    ///
    /// `run_ratatui_app` can reload config during onboarding and again
    /// during session resume/branch.  The syntax theme override (stored in
    /// a `OnceLock`) must use the final config's `tui_theme`, not the
    /// initial one — otherwise users resuming a thread in a project with
    /// a different theme get the wrong highlighting.
    ///
    /// We verify the invariant indirectly: `validate_theme_name` (the
    /// pure validation core of `set_theme_override`) must be called with
    /// the *final* config's theme, and its warning must land in the
    /// final config's `startup_warnings`.
    #[tokio::test]
    async fn theme_warning_uses_final_config() -> std::io::Result<()> {
        use crate::render::highlight::validate_theme_name;

        let temp_dir = TempDir::new()?;

        // initial_config has a valid theme — no warning.
        let initial_config = build_config(&temp_dir).await?;
        assert!(initial_config.tui_theme.is_none());

        // Simulate resume/branch reload: the final config has an invalid theme.
        let mut config = build_config(&temp_dir).await?;
        config.tui_theme = Some("bogus-theme".into());

        // Theme override must use the final config (not initial_config).
        // This mirrors the real call site in run_ratatui_app.
        if let Some(w) = validate_theme_name(config.tui_theme.as_deref(), Some(temp_dir.path())) {
            config.startup_warnings.push(w);
        }

        assert_eq!(
            config.startup_warnings.len(),
            1,
            "warning from final config's invalid theme should be present"
        );
        assert!(
            config.startup_warnings[0].contains("bogus-theme"),
            "warning should reference the final config's theme name"
        );
        Ok(())
    }
}
