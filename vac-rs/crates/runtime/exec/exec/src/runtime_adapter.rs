use std::io::IsTerminal;
use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;
use tracing::warn;
use vac_core::ThreadManager;
use vac_core::config::Config;
use vac_core::local_runtime::AutonomyMode;
use vac_core::local_runtime::LocalRuntimeBridge;
use vac_core::local_runtime::RuntimeCommand;
use vac_core::local_runtime::RuntimeEntrypoint;
use vac_core::local_runtime::RuntimeEvent;
use vac_core::local_runtime::RuntimeSession;
use vac_core::local_runtime::RuntimeTask;
use vac_core::local_runtime::RuntimeTaskKind;
use vac_core::thread_store_from_config;
use vac_exec_server::EnvironmentManager;
use vac_login::AuthManager;
use vac_protocol::models::PermissionProfile;
use vac_protocol::protocol::Op;
use vac_protocol::protocol::SessionConfiguredEvent;
use vac_protocol::protocol::SessionSource;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_local_runtime_prompt(
    config: &Config,
    environment_manager: Arc<EnvironmentManager>,
    prompt_text: String,
    items: Vec<vac_protocol::user_input::UserInput>,
    output_schema: Option<Value>,
    json_mode: bool,
    last_message_path: Option<PathBuf>,
    stderr_with_ansi: bool,
) -> anyhow::Result<()> {
    let start_command = RuntimeCommand::start_task(
        prompt_text.clone(),
        AutonomyMode::Autopilot,
        RuntimeEntrypoint::Exec,
        config.cwd.to_path_buf(),
    );
    tracing::debug!(?start_command, "prepared local runtime start task");
    let RuntimeCommand::StartTask(start_task) = start_command else {
        unreachable!("runtime command start_task helper must return StartTask");
    };

    let auth_manager =
        AuthManager::shared_from_config(config, /*enable_vac_api_key_env*/ true).await;
    let thread_store = thread_store_from_config(config);
    let thread_manager = ThreadManager::new(
        config,
        auth_manager,
        SessionSource::Exec,
        environment_manager,
        None,
        thread_store,
    );
    let new_thread = thread_manager.start_thread(config.clone()).await?;
    let session_configured = new_thread.session_configured.clone();
    let session = RuntimeSession::new(
        start_task.cwd.clone(),
        start_task.entrypoint,
        start_task.autonomy_mode,
    );
    let task = RuntimeTask::new(
        session.id,
        RuntimeTaskKind::SemanticCoding,
        start_task.prompt.clone(),
    );

    let mut processor: Box<dyn LocalRuntimeOutput> = if json_mode {
        Box::new(LocalRuntimeJsonOutput::new(last_message_path))
    } else {
        Box::new(LocalRuntimeHumanOutput::create(
            stderr_with_ansi,
            last_message_path,
            config,
            &session_configured,
            &session,
            &task,
            &prompt_text,
        ))
    };

    processor.print_config_summary(&session, &task, &session_configured, &prompt_text);

    if !json_mode
        && let Some(message) =
            vac_core::config::system_bwrap_warning(config.permissions.permission_profile.get())
    {
        let _ = processor.process_warning(message);
    }

    for warning in &config.startup_warnings {
        let _ = processor.process_warning(warning.clone());
    }

    let turn_submission = build_user_turn_submission(&session_configured, items, output_schema)?;

    let thread = Arc::clone(&new_thread.thread);
    let interrupt_thread = Arc::clone(&thread);
    let interrupt_handle = tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            tracing::debug!("Keyboard interrupt");
            if let Err(err) = interrupt_thread.submit(Op::Interrupt).await {
                warn!("failed to interrupt local runtime session: {err}");
            }
        }
    });

    let mut bridge = LocalRuntimeBridge::new(session.id, task.id, task.prompt.clone());
    if let Err(err) = thread.submit(turn_submission).await {
        warn!("failed to submit local runtime turn: {err}");
        let _ = thread.shutdown_and_wait().await;
        interrupt_handle.abort();
        anyhow::bail!("failed to submit local runtime turn: {err}");
    }

    let mut error_seen = false;
    loop {
        let event = match thread.next_event().await {
            Ok(event) => event,
            Err(err) => {
                warn!("local runtime event stream ended unexpectedly: {err}");
                error_seen = true;
                break;
            }
        };

        for warning in bridge.warnings_for_event(&event) {
            let _ = processor.process_warning(warning);
        }

        let bridge_output = bridge.map_core_event(event);
        error_seen |= bridge_output.error_seen;

        for runtime_event in bridge_output.events {
            if let RuntimeEventDisposition::Terminate { fatal } =
                processor.process_runtime_event(runtime_event)
            {
                error_seen |= fatal;
                break;
            }
        }

        if bridge_output.terminate {
            break;
        }
    }

    let shutdown_result = thread.shutdown_and_wait().await;
    if let Err(err) = shutdown_result {
        warn!("local runtime shutdown failed: {err}");
    }
    interrupt_handle.abort();
    processor.print_final_output();

    if error_seen {
        anyhow::bail!("local runtime prompt ended with an error");
    }

    Ok(())
}

#[allow(dead_code)]
pub(crate) async fn run_local_runtime_review(
    config: &Config,
    environment_manager: Arc<EnvironmentManager>,
    review_request: vac_protocol::protocol::ReviewRequest,
    prompt_summary: String,
    json_mode: bool,
    last_message_path: Option<PathBuf>,
    stderr_with_ansi: bool,
) -> anyhow::Result<()> {
    tracing::debug!(?review_request, "prepared local runtime review");

    let auth_manager =
        AuthManager::shared_from_config(config, /*enable_vac_api_key_env*/ true).await;
    let thread_store = thread_store_from_config(config);
    let thread_manager = ThreadManager::new(
        config,
        auth_manager,
        SessionSource::Exec,
        environment_manager,
        None,
        thread_store,
    );
    let new_thread = thread_manager.start_thread(config.clone()).await?;
    let session_configured = new_thread.session_configured.clone();
    let session = RuntimeSession::new(
        config.cwd.to_path_buf(),
        RuntimeEntrypoint::Exec,
        AutonomyMode::Autopilot,
    );
    let task = RuntimeTask::new(
        session.id,
        RuntimeTaskKind::SemanticCoding,
        prompt_summary.clone(),
    );

    let mut processor: Box<dyn LocalRuntimeOutput> = if json_mode {
        Box::new(LocalRuntimeJsonOutput::new(last_message_path))
    } else {
        Box::new(LocalRuntimeHumanOutput::create(
            stderr_with_ansi,
            last_message_path,
            config,
            &session_configured,
            &session,
            &task,
            &prompt_summary,
        ))
    };

    processor.print_config_summary(&session, &task, &session_configured, &prompt_summary);

    if !json_mode
        && let Some(message) =
            vac_core::config::system_bwrap_warning(config.permissions.permission_profile.get())
    {
        let _ = processor.process_warning(message);
    }

    for warning in &config.startup_warnings {
        let _ = processor.process_warning(warning.clone());
    }

    let thread = Arc::clone(&new_thread.thread);
    let interrupt_thread = Arc::clone(&thread);
    let interrupt_handle = tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            tracing::debug!("Keyboard interrupt");
            if let Err(err) = interrupt_thread.submit(Op::Interrupt).await {
                warn!("failed to interrupt local runtime review: {err}");
            }
        }
    });

    let mut bridge = LocalRuntimeBridge::new(session.id, task.id, task.prompt.clone());
    if let Err(err) = thread.submit(Op::Review { review_request }).await {
        warn!("failed to submit local runtime review: {err}");
        let _ = thread.shutdown_and_wait().await;
        interrupt_handle.abort();
        anyhow::bail!("failed to submit local runtime review: {err}");
    }

    let mut error_seen = false;
    loop {
        let event = match thread.next_event().await {
            Ok(event) => event,
            Err(err) => {
                warn!("local runtime event stream ended unexpectedly: {err}");
                error_seen = true;
                break;
            }
        };

        for warning in bridge.warnings_for_event(&event) {
            let _ = processor.process_warning(warning);
        }

        let bridge_output = bridge.map_core_event(event);
        error_seen |= bridge_output.error_seen;

        for runtime_event in bridge_output.events {
            if let RuntimeEventDisposition::Terminate { fatal } =
                processor.process_runtime_event(runtime_event)
            {
                error_seen |= fatal;
                break;
            }
        }

        if bridge_output.terminate {
            break;
        }
    }

    let shutdown_result = thread.shutdown_and_wait().await;
    if let Err(err) = shutdown_result {
        warn!("local runtime shutdown failed: {err}");
    }
    interrupt_handle.abort();
    processor.print_final_output();

    if error_seen {
        anyhow::bail!("local runtime review ended with an error");
    }

    Ok(())
}

#[allow(dead_code)]
pub(crate) async fn resolve_resume_rollout_path(
    config: &Config,
    args: &crate::cli::ResumeArgs,
) -> anyhow::Result<Option<PathBuf>> {
    use vac_protocol::protocol::SessionSource as ProtoSessionSource;

    if args.last {
        let cwd_filters_owned: Vec<PathBuf> = if args.all {
            Vec::new()
        } else {
            vec![config.cwd.to_path_buf()]
        };
        let cwd_filters: Option<&[PathBuf]> = if cwd_filters_owned.is_empty() {
            None
        } else {
            Some(cwd_filters_owned.as_slice())
        };
        let allowed_sources: &[ProtoSessionSource] = &[];
        let provider = config.model_provider_id.clone();
        let list_cfg = vac_core::ThreadListConfig {
            allowed_sources,
            model_providers: None,
            cwd_filters,
            default_provider: provider.as_str(),
            layout: vac_core::ThreadListLayout::NestedByDate,
        };
        let root = config.vac_home.join(vac_core::SESSIONS_SUBDIR);
        let page = vac_core::get_threads_in_root(
            root.to_path_buf(),
            1,
            None,
            vac_core::ThreadSortKey::UpdatedAt,
            list_cfg,
        )
        .await?;
        return Ok(page.items.into_iter().next().map(|item| item.path));
    }

    let Some(session_id) = args.session_id.as_deref() else {
        return Ok(None);
    };

    if uuid::Uuid::parse_str(session_id).is_ok() {
        return vac_core::find_thread_path_by_id_str(&config.vac_home, session_id)
            .await
            .map_err(Into::into);
    }

    if let Some(state_db) = vac_core::get_state_db(config).await {
        let cwd = (!args.all).then_some(config.cwd.as_path());
        if let Some(thread) = state_db
            .find_thread_by_exact_title(
                session_id,
                &[],
                /*model_providers*/ None,
                /*archived_only*/ false,
                cwd,
            )
            .await?
            && let Some(path) =
                vac_core::find_thread_path_by_id_str(&config.vac_home, &thread.id.to_string())
                    .await?
        {
            return Ok(Some(path));
        }
    }

    if let Some((path, session_meta)) =
        vac_core::find_thread_meta_by_name_str(&config.vac_home, session_id).await?
        && (args.all
            || vac_core::paths_match_after_normalization(
                config.cwd.as_path(),
                &session_meta.meta.cwd,
            ))
    {
        return Ok(Some(path));
    }

    Ok(None)
}

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_local_runtime_resume(
    config: &Config,
    environment_manager: Arc<EnvironmentManager>,
    rollout_path: PathBuf,
    prompt_text: String,
    items: Vec<vac_protocol::user_input::UserInput>,
    output_schema: Option<Value>,
    json_mode: bool,
    last_message_path: Option<PathBuf>,
    stderr_with_ansi: bool,
) -> anyhow::Result<()> {
    tracing::debug!(?rollout_path, "prepared local runtime resume");

    let auth_manager =
        AuthManager::shared_from_config(config, /*enable_vac_api_key_env*/ true).await;
    let thread_store = thread_store_from_config(config);
    let thread_manager = ThreadManager::new(
        config,
        Arc::clone(&auth_manager),
        SessionSource::Exec,
        environment_manager,
        None,
        thread_store,
    );
    let new_thread = thread_manager
        .resume_thread_from_rollout(
            config.clone(),
            rollout_path,
            Arc::clone(&auth_manager),
            /*parent_trace*/ None,
        )
        .await?;
    let session_configured = new_thread.session_configured.clone();
    let session = RuntimeSession::new(
        config.cwd.to_path_buf(),
        RuntimeEntrypoint::Exec,
        AutonomyMode::Autopilot,
    );
    let task = RuntimeTask::new(
        session.id,
        RuntimeTaskKind::SemanticCoding,
        prompt_text.clone(),
    );

    let mut processor: Box<dyn LocalRuntimeOutput> = if json_mode {
        Box::new(LocalRuntimeJsonOutput::new(last_message_path))
    } else {
        Box::new(LocalRuntimeHumanOutput::create(
            stderr_with_ansi,
            last_message_path,
            config,
            &session_configured,
            &session,
            &task,
            &prompt_text,
        ))
    };

    processor.print_config_summary(&session, &task, &session_configured, &prompt_text);

    if !json_mode
        && let Some(message) =
            vac_core::config::system_bwrap_warning(config.permissions.permission_profile.get())
    {
        let _ = processor.process_warning(message);
    }

    for warning in &config.startup_warnings {
        let _ = processor.process_warning(warning.clone());
    }

    let turn_submission = build_user_turn_submission(&session_configured, items, output_schema)?;

    let thread = Arc::clone(&new_thread.thread);
    let interrupt_thread = Arc::clone(&thread);
    let interrupt_handle = tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            tracing::debug!("Keyboard interrupt");
            if let Err(err) = interrupt_thread.submit(Op::Interrupt).await {
                warn!("failed to interrupt local runtime resume: {err}");
            }
        }
    });

    let mut bridge = LocalRuntimeBridge::new(session.id, task.id, task.prompt.clone());
    if let Err(err) = thread.submit(turn_submission).await {
        warn!("failed to submit local runtime resume turn: {err}");
        let _ = thread.shutdown_and_wait().await;
        interrupt_handle.abort();
        anyhow::bail!("failed to submit local runtime resume turn: {err}");
    }

    let mut error_seen = false;
    loop {
        let event = match thread.next_event().await {
            Ok(event) => event,
            Err(err) => {
                warn!("local runtime event stream ended unexpectedly: {err}");
                error_seen = true;
                break;
            }
        };

        for warning in bridge.warnings_for_event(&event) {
            let _ = processor.process_warning(warning);
        }

        let bridge_output = bridge.map_core_event(event);
        error_seen |= bridge_output.error_seen;

        for runtime_event in bridge_output.events {
            if let RuntimeEventDisposition::Terminate { fatal } =
                processor.process_runtime_event(runtime_event)
            {
                error_seen |= fatal;
                break;
            }
        }

        if bridge_output.terminate {
            break;
        }
    }

    let shutdown_result = thread.shutdown_and_wait().await;
    if let Err(err) = shutdown_result {
        warn!("local runtime shutdown failed: {err}");
    }
    interrupt_handle.abort();
    processor.print_final_output();

    if error_seen {
        anyhow::bail!("local runtime resume ended with an error");
    }

    Ok(())
}

fn build_user_turn_submission(
    session_configured: &SessionConfiguredEvent,
    items: Vec<vac_protocol::user_input::UserInput>,
    output_schema: Option<Value>,
) -> anyhow::Result<Op> {
    let sandbox_policy = session_configured
        .permission_profile
        .to_legacy_sandbox_policy(session_configured.cwd.as_path())?;
    Ok(Op::UserTurn {
        items,
        cwd: session_configured.cwd.as_path().to_path_buf(),
        approval_policy: session_configured.approval_policy,
        approvals_reviewer: Some(session_configured.approvals_reviewer),
        sandbox_policy,
        permission_profile: Some(session_configured.permission_profile.clone()),
        model: session_configured.model.clone(),
        effort: session_configured.reasoning_effort,
        summary: None,
        service_tier: Some(session_configured.service_tier),
        final_output_json_schema: output_schema,
        collaboration_mode: None,
        personality: None,
        environments: None,
    })
}

trait LocalRuntimeOutput {
    fn print_config_summary(
        &mut self,
        session: &RuntimeSession,
        task: &RuntimeTask,
        session_configured: &SessionConfiguredEvent,
        prompt_text: &str,
    );

    fn process_runtime_event(&mut self, event: RuntimeEvent) -> RuntimeEventDisposition;

    fn process_warning(&mut self, message: String) -> RuntimeEventDisposition;

    fn print_final_output(&mut self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeEventDisposition {
    Continue,
    Terminate { fatal: bool },
}

struct LocalRuntimeHumanOutput {
    stderr_with_ansi: bool,
    last_message_path: Option<PathBuf>,
    final_message: Option<String>,
    final_message_rendered: bool,
    emit_final_message_on_shutdown: bool,
    assistant_buffer: String,
    stdout: Box<dyn std::io::Write + Send>,
    stderr: Box<dyn std::io::Write + Send>,
}

impl LocalRuntimeHumanOutput {
    fn create(
        stderr_with_ansi: bool,
        last_message_path: Option<PathBuf>,
        _config: &Config,
        _session_configured: &SessionConfiguredEvent,
        _session: &RuntimeSession,
        _task: &RuntimeTask,
        _prompt_text: &str,
    ) -> Self {
        Self {
            stderr_with_ansi,
            last_message_path,
            final_message: None,
            final_message_rendered: false,
            emit_final_message_on_shutdown: false,
            assistant_buffer: String::new(),
            stdout: Box::new(std::io::stdout()),
            stderr: Box::new(std::io::stderr()),
        }
    }

    #[cfg(test)]
    fn with_writers(
        stdout: Box<dyn std::io::Write + Send>,
        stderr: Box<dyn std::io::Write + Send>,
    ) -> Self {
        Self {
            stderr_with_ansi: false,
            last_message_path: None,
            final_message: None,
            final_message_rendered: false,
            emit_final_message_on_shutdown: false,
            assistant_buffer: String::new(),
            stdout,
            stderr,
        }
    }
}

impl LocalRuntimeOutput for LocalRuntimeHumanOutput {
    fn print_config_summary(
        &mut self,
        session: &RuntimeSession,
        task: &RuntimeTask,
        session_configured: &SessionConfiguredEvent,
        prompt_text: &str,
    ) {
        let _ = self.stderr_with_ansi;
        let _ = writeln!(
            self.stderr,
            "Vastar VAC v{} (local runtime)\n--------",
            env!("CARGO_PKG_VERSION")
        );
        let _ = writeln!(self.stderr, "session: {}", session.id);
        let _ = writeln!(self.stderr, "task: {}", task.id);
        let _ = writeln!(self.stderr, "entrypoint: {}", session.entrypoint);
        let _ = writeln!(self.stderr, "autonomy: {}", session.autonomy_mode);
        let _ = writeln!(self.stderr, "workdir: {}", session_configured.cwd.display());
        let _ = writeln!(self.stderr, "model: {}", session_configured.model);
        let _ = writeln!(
            self.stderr,
            "provider: {}",
            session_configured.model_provider_id
        );
        let _ = writeln!(
            self.stderr,
            "approval: {}",
            session_configured.approval_policy
        );
        let _ = writeln!(
            self.stderr,
            "sandbox: {}",
            sandbox_label(
                &session_configured.permission_profile,
                session_configured.cwd.as_path()
            )
        );
        let _ = writeln!(self.stderr, "--------");
        let _ = writeln!(self.stderr, "user\n{prompt_text}");
    }

    fn process_runtime_event(&mut self, event: RuntimeEvent) -> RuntimeEventDisposition {
        match event {
            RuntimeEvent::SessionStarted(session) => {
                let _ = writeln!(self.stderr, "session started: {}", session.session.id);
                RuntimeEventDisposition::Continue
            }
            RuntimeEvent::TaskStarted(task) => {
                let _ = writeln!(
                    self.stderr,
                    "task started: {} ({})",
                    task.task.kind, task.task.status
                );
                RuntimeEventDisposition::Continue
            }
            RuntimeEvent::AssistantDelta(delta) => {
                self.assistant_buffer.push_str(&delta.text);
                let _ = write!(self.stderr, "{}", delta.text);
                RuntimeEventDisposition::Continue
            }
            RuntimeEvent::ToolCallStarted(tool) => {
                let _ = writeln!(
                    self.stderr,
                    "tool started: {}{}",
                    tool.tool_name,
                    tool.input_preview
                        .as_deref()
                        .map(|preview| format!(" - {preview}"))
                        .unwrap_or_default()
                );
                RuntimeEventDisposition::Continue
            }
            RuntimeEvent::ToolCallFinished(tool) => {
                let _ = writeln!(
                    self.stderr,
                    "tool {}: {}{}",
                    if tool.success { "done" } else { "failed" },
                    tool.tool_name,
                    tool.output_preview
                        .as_deref()
                        .map(|preview| format!(" - {preview}"))
                        .unwrap_or_default()
                );
                RuntimeEventDisposition::Continue
            }
            RuntimeEvent::ApprovalRequested(request) => {
                let _ = writeln!(self.stderr, "approval requested: {}", request.reason);
                let _ = writeln!(
                    self.stderr,
                    "action: {}, risk: {}",
                    request.action, request.risk
                );
                for resource in request.resources {
                    let _ = writeln!(self.stderr, "resource: {resource:?}");
                }
                if !request.validation_after.is_empty() {
                    let _ = writeln!(
                        self.stderr,
                        "validation after: {}",
                        request.validation_after.join(", ")
                    );
                }
                RuntimeEventDisposition::Continue
            }
            RuntimeEvent::ApprovalResolved(_) => RuntimeEventDisposition::Continue,
            RuntimeEvent::ValidationStarted(validation) => {
                let _ = writeln!(
                    self.stderr,
                    "validation started: {}",
                    validation.command_display
                );
                RuntimeEventDisposition::Continue
            }
            RuntimeEvent::ValidationFinished(validation) => {
                let _ = writeln!(
                    self.stderr,
                    "validation {}: {}",
                    validation.status, validation.command_display
                );
                if let Some(summary) = validation.summary.as_deref()
                    && !summary.trim().is_empty()
                {
                    let _ = writeln!(self.stderr, "{summary}");
                }
                RuntimeEventDisposition::Continue
            }
            RuntimeEvent::TaskCompleted(completed) => {
                if let Some(summary) = completed.summary.as_deref() {
                    self.final_message = Some(summary.to_string());
                } else if !self.assistant_buffer.trim().is_empty() {
                    self.final_message = Some(self.assistant_buffer.trim().to_string());
                }
                if !completed.evidence.is_empty() {
                    let _ = writeln!(self.stderr, "evidence:");
                    for evidence in completed.evidence {
                        let _ = writeln!(self.stderr, "- {evidence}");
                    }
                }
                self.emit_final_message_on_shutdown = true;
                RuntimeEventDisposition::Terminate { fatal: false }
            }
            RuntimeEvent::TaskFailed(failed) => {
                let _ = writeln!(self.stderr, "task failed: {}", failed.error.message);
                if let Some(hint) = failed.error.recovery_hint.as_deref() {
                    let _ = writeln!(self.stderr, "{hint}");
                }
                self.final_message = None;
                self.emit_final_message_on_shutdown = false;
                RuntimeEventDisposition::Terminate { fatal: true }
            }
            RuntimeEvent::TaskCancelled(cancelled) => {
                let _ = writeln!(
                    self.stderr,
                    "task cancelled{}",
                    cancelled
                        .reason
                        .as_deref()
                        .map(|reason| format!(": {reason}"))
                        .unwrap_or_default()
                );
                self.final_message = None;
                self.emit_final_message_on_shutdown = false;
                RuntimeEventDisposition::Terminate { fatal: true }
            }
            RuntimeEvent::SessionEnded(ended) => {
                let _ = writeln!(self.stderr, "session ended: {}", ended.session_id);
                RuntimeEventDisposition::Continue
            }
            RuntimeEvent::EnteredReviewMode(_) | RuntimeEvent::ExitedReviewMode(_) => {
                RuntimeEventDisposition::Continue
            }
        }
    }

    fn process_warning(&mut self, message: String) -> RuntimeEventDisposition {
        let _ = writeln!(self.stderr, "warning: {message}");
        RuntimeEventDisposition::Continue
    }

    fn print_final_output(&mut self) {
        if self.emit_final_message_on_shutdown
            && let Some(path) = self.last_message_path.as_deref()
        {
            handle_last_message(self.final_message.as_deref(), path);
        }

        let final_message = self.final_message.as_deref();
        if should_print_final_message_to_stdout(
            final_message,
            std::io::stdout().is_terminal(),
            std::io::stderr().is_terminal(),
        ) && let Some(message) = final_message
        {
            self.final_message_rendered = true;
            let _ = writeln!(self.stdout, "{message}");
        } else if should_print_final_message_to_tty(
            final_message,
            self.final_message_rendered,
            std::io::stdout().is_terminal(),
            std::io::stderr().is_terminal(),
        ) && let Some(message) = final_message
        {
            self.final_message_rendered = true;
            let _ = writeln!(self.stderr, "vac\n{message}");
        }
    }
}

struct LocalRuntimeJsonOutput {
    last_message_path: Option<PathBuf>,
    final_message: Option<String>,
    emit_final_message_on_shutdown: bool,
}

impl LocalRuntimeJsonOutput {
    fn new(last_message_path: Option<PathBuf>) -> Self {
        Self {
            last_message_path,
            final_message: None,
            emit_final_message_on_shutdown: false,
        }
    }

    // Deliberately writes serialized JSON events to stdout: this is the json_mode output channel.
    #[allow(clippy::print_stdout)]
    fn emit<T: serde::Serialize>(&self, event: &T) {
        println!(
            "{}",
            serde_json::to_string(event).unwrap_or_else(|err| {
                serde_json::json!({
                    "type": "error",
                    "message": format!("failed to serialize local runtime event: {err}"),
                })
                .to_string()
            })
        );
    }
}

impl LocalRuntimeOutput for LocalRuntimeJsonOutput {
    fn print_config_summary(
        &mut self,
        session: &RuntimeSession,
        _task: &RuntimeTask,
        _session_configured: &SessionConfiguredEvent,
        _prompt_text: &str,
    ) {
        self.emit(&RuntimeEvent::SessionStarted(
            vac_core::local_runtime::SessionStarted::new(session.clone()),
        ));
    }

    fn process_runtime_event(&mut self, event: RuntimeEvent) -> RuntimeEventDisposition {
        match &event {
            RuntimeEvent::TaskCompleted(completed) => {
                self.final_message = completed.summary.clone();
                self.emit_final_message_on_shutdown = true;
                self.emit(&event);
                RuntimeEventDisposition::Terminate { fatal: false }
            }
            RuntimeEvent::TaskFailed(_) | RuntimeEvent::TaskCancelled(_) => {
                self.final_message = None;
                self.emit_final_message_on_shutdown = false;
                self.emit(&event);
                RuntimeEventDisposition::Terminate { fatal: true }
            }
            RuntimeEvent::ApprovalRequested(_) => {
                self.emit(&event);
                RuntimeEventDisposition::Continue
            }
            _ => {
                self.emit(&event);
                RuntimeEventDisposition::Continue
            }
        }
    }

    fn process_warning(&mut self, message: String) -> RuntimeEventDisposition {
        eprintln!("warning: {message}");
        RuntimeEventDisposition::Continue
    }

    fn print_final_output(&mut self) {
        if self.emit_final_message_on_shutdown
            && let Some(path) = self.last_message_path.as_deref()
        {
            handle_last_message(self.final_message.as_deref(), path);
        }
    }
}

fn sandbox_label(permission_profile: &PermissionProfile, cwd: &Path) -> String {
    permission_profile
        .to_legacy_sandbox_policy(cwd)
        .map(|sandbox| sandbox.to_string())
        .unwrap_or_else(|_| {
            format!(
                "{} / {}",
                permission_profile.file_system_sandbox_policy().kind,
                permission_profile.network_sandbox_policy()
            )
        })
}

fn should_print_final_message_to_stdout(
    final_message: Option<&str>,
    stdout_is_terminal: bool,
    stderr_is_terminal: bool,
) -> bool {
    final_message.is_some() && !(stdout_is_terminal && stderr_is_terminal)
}

fn should_print_final_message_to_tty(
    final_message: Option<&str>,
    final_message_rendered: bool,
    stdout_is_terminal: bool,
    stderr_is_terminal: bool,
) -> bool {
    final_message.is_some() && !final_message_rendered && stdout_is_terminal && stderr_is_terminal
}

pub(crate) fn handle_last_message(last_agent_message: Option<&str>, output_file: &Path) {
    let message = last_agent_message.unwrap_or_default();
    write_last_message_file(message, Some(output_file));
    if last_agent_message.is_none() {
        eprintln!(
            "Warning: no last agent message; wrote empty content to {}",
            output_file.display()
        );
    }
}

fn write_last_message_file(contents: &str, last_message_path: Option<&Path>) {
    if let Some(path) = last_message_path
        && let Err(e) = std::fs::write(path, contents)
    {
        eprintln!("Failed to write last message file {path:?}: {e}");
    }
}

#[cfg(test)]
mod human_output_tests {
    use super::LocalRuntimeHumanOutput;
    use super::LocalRuntimeOutput;
    use std::io::Write;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::MutexGuard;

    fn lock_buffer(buffer: &Mutex<Vec<u8>>) -> MutexGuard<'_, Vec<u8>> {
        match buffer.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    struct SharedWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            lock_buffer(&self.0).write(buf)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            lock_buffer(&self.0).flush()
        }
    }

    #[allow(clippy::type_complexity)]
    fn make_buffers() -> (
        Arc<Mutex<Vec<u8>>>,
        Arc<Mutex<Vec<u8>>>,
        LocalRuntimeHumanOutput,
    ) {
        let stdout_buf = Arc::new(Mutex::new(Vec::<u8>::new()));
        let stderr_buf = Arc::new(Mutex::new(Vec::<u8>::new()));
        let output = LocalRuntimeHumanOutput::with_writers(
            Box::new(SharedWriter(Arc::clone(&stdout_buf))),
            Box::new(SharedWriter(Arc::clone(&stderr_buf))),
        );
        (stdout_buf, stderr_buf, output)
    }

    fn read_buf(buf: &Arc<Mutex<Vec<u8>>>) -> String {
        String::from_utf8(lock_buffer(buf).clone()).expect("utf8")
    }

    #[test]
    fn process_warning_writes_to_injected_stderr() {
        let (_stdout, stderr, mut output) = make_buffers();
        let _ = output.process_warning("test warning".to_string());
        let captured = read_buf(&stderr);
        assert!(
            captured.contains("warning: test warning"),
            "captured: {captured:?}"
        );
    }

    #[test]
    fn with_writers_starts_with_empty_buffers() {
        let (stdout, stderr, _output) = make_buffers();
        assert!(read_buf(&stdout).is_empty());
        assert!(read_buf(&stderr).is_empty());
    }
}
