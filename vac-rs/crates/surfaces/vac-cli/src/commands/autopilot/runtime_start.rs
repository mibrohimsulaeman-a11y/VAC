use super::*;

pub(super) async fn run_startup_preflight(
    config: &AppConfig,
    bind_addr: &str,
) -> Result<(), String> {
    let base_url = loopback_base_url_from_bind(bind_addr);
    let server_health_url = format!("{base_url}/v1/health");
    let probe_client = build_probe_http_client();
    let server_reachable = if let Some(client) = probe_client.as_ref() {
        endpoint_ok(client, &server_health_url).await
    } else {
        false
    };

    let env = RealProbeEnvironment;
    let ctx = AutopilotProbeContext {
        app_config: config,
        bind_addr: Some(bind_addr),
        server_reachable,
    };
    let results = run_autopilot_probes(ProbeMode::Startup, &ctx, &env);
    presenter::print_probe_report("Preflight checks", &results);

    let summary = summarize_probe_results(&results);
    if summary.blocking_failures > 0 {
        return Err(format!(
            "Preflight checks failed with {} blocking issue(s)",
            summary.blocking_failures
        ));
    }

    Ok(())
}

pub(super) async fn start_autopilot(
    config: &mut AppConfig,
    options: StartOptions,
) -> Result<(), String> {
    let autopilot_config_path = AutopilotConfigFile::path();
    let needs_setup = !autopilot_config_path.exists() || options.force;

    // ── First-run setup (merged from old `init`) ──────────────────────
    if needs_setup {
        println!("VAC Autopilot setup");
        println!("Profile: {}", config.profile_name);
        println!();

        // Credential check — interactive gets onboarding wizard, non-interactive gets error
        let has_vac_key = config.get_vac_remote_service_key().is_some();
        let has_provider_keys = !config.get_llm_provider_config().providers.is_empty();

        if !has_vac_key && !has_provider_keys {
            if options.non_interactive {
                return Err(
                    "No provider credentials configured. Run with credentials in env or run interactive setup without --non-interactive.".to_string(),
                );
            }

            println!("No credentials found. Launching onboarding...");
            run_onboarding(config, OnboardingMode::Default).await;
            println!();
        }

        // Write default config template (or overwrite with --force)
        write_default_autopilot_config(&autopilot_config_path, options.force).await?;
        println!(
            "✓ Autopilot config created: {}",
            autopilot_config_path.display()
        );

        // Pick up channel tokens from environment
        let telegram_token = std::env::var("TELEGRAM_BOT_TOKEN").ok();
        let discord_token = std::env::var("DISCORD_BOT_TOKEN").ok();
        let slack_bot_token = std::env::var("SLACK_BOT_TOKEN").ok();
        let slack_app_token = std::env::var("SLACK_APP_TOKEN").ok();

        let has_env_channels = telegram_token.is_some()
            || discord_token.is_some()
            || (slack_bot_token.is_some() && slack_app_token.is_some());

        if has_env_channels {
            let mut gateway_config = vac_messaging_gateway::GatewayConfig::load(
                autopilot_config_path.as_path(),
                &vac_messaging_gateway::GatewayCliFlags::default(),
            )
            .unwrap_or_default();

            if let Some(token) = telegram_token {
                gateway_config.channels.telegram =
                    Some(vac_messaging_gateway::config::TelegramConfig {
                        token,
                        require_mention: false,
                        model: None,
                        auto_approve: None,
                        profile: Some(config.profile_name.clone()),
                    });
            }
            if let Some(token) = discord_token {
                gateway_config.channels.discord =
                    Some(vac_messaging_gateway::config::DiscordConfig {
                        token,
                        guilds: Vec::new(),
                        model: None,
                        auto_approve: None,
                        profile: Some(config.profile_name.clone()),
                    });
            }
            if let (Some(bot_token), Some(app_token)) = (slack_bot_token, slack_app_token) {
                gateway_config.channels.slack = Some(vac_messaging_gateway::config::SlackConfig {
                    bot_token,
                    app_token,
                    model: None,
                    auto_approve: None,
                    profile: Some(config.profile_name.clone()),
                });
            }

            gateway_config
                .save(autopilot_config_path.as_path())
                .map_err(|e| format!("Failed to save channel config: {e}"))?;

            let channels = gateway_config.enabled_channels();
            println!(
                "✓ Channels configured from environment: {}",
                channels.join(", ")
            );
        } else if !options.non_interactive {
            println!();
            println!("Channels let autopilot talk to you on Slack, Telegram, or Discord.");
            println!("You can add them now or later with: vac autopilot channel add");
            println!();
            println!("  Slack quick setup: use the app manifest at");
            println!(
                "  local source path: vac-rs/crates/integrations/vac-messaging-gateway/src/channels/slack-manifest.yaml"
            );
            println!();
        }
    }

    // ── Load config and apply runtime overrides ──────────────────────
    let config_path = AutopilotConfigFile::path();
    let saved_config = AutopilotConfigFile::load_or_default()?;

    let has_runtime_overrides = options.has_runtime_overrides();
    let effective_server = if has_runtime_overrides || needs_setup {
        let server_config = AutopilotServerConfig::from_start_options(&options);
        validate_no_auth_bind(&server_config.listen, server_config.no_auth)?;
        let mut config_file = saved_config.clone();
        config_file.server = server_config.clone();
        config_file.save()?;
        if has_runtime_overrides && !needs_setup {
            println!("✓ Saved server overrides to {}", config_path.display());
        }
        server_config
    } else {
        saved_config.server.clone()
    };

    validate_no_auth_bind(&effective_server.listen, effective_server.no_auth)?;

    let effective_options = options.clone().with_server_config(&effective_server);

    if effective_options.foreground || effective_options.from_service {
        if !effective_options.from_service {
            run_startup_preflight(config, &effective_server.listen).await?;
        }
        return start_foreground_runtime(config, &effective_options).await;
    }

    // Idempotency: if autopilot is already running, skip start
    if let Some(pid) = is_autopilot_running() {
        println!("Autopilot is already running (PID {}).", pid);
        println!();
        println!("  Status      vac autopilot status");
        println!("  Restart     vac autopilot restart");
        println!("  Stop        vac autopilot down");
        return Ok(());
    }

    run_startup_preflight(config, &effective_server.listen).await?;

    if !autopilot_service_installed() {
        install_autopilot_service(config)?;
        println!("✓ Installed autopilot service");
    }

    // Ensure the sandbox container image is available locally before starting the
    // service. Pulling inside the service process produces no visible output — the
    // user just sees a frozen "waiting" message. By pulling here we inherit
    // stdout/stderr so Docker's progress bars are visible. This applies to both
    // persistent and ephemeral modes since both need the image.
    ensure_sandbox_image_available()?;

    let expects_sandbox = effective_server.sandbox_mode == vac_broker::SandboxMode::Persistent;

    start_autopilot_service()?;

    // Wait for the server (and persistent sandbox if configured) to be ready
    // before printing the status summary. This ensures `vac up` only returns
    // once the autopilot is fully operational.
    let base_url = format!("http://{}", effective_server.listen);
    let health_url = format!("{base_url}/v1/health");
    let max_wait = std::time::Duration::from_secs(120);
    let poll_interval = std::time::Duration::from_millis(500);
    let start = std::time::Instant::now();

    // Spinner frames for the waiting animation
    const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let mut spinner_idx: usize = 0;
    let mut last_phase = String::new();

    let ready = loop {
        if start.elapsed() > max_wait {
            break false;
        }
        tokio::time::sleep(poll_interval).await;

        // Determine current phase for display
        let phase = match reqwest::get(&health_url).await {
            Ok(resp) => match resp.json::<serde_json::Value>().await {
                Ok(body) => {
                    if expects_sandbox {
                        if let Some(sandbox) = body.get("sandbox")
                            && sandbox.get("healthy").and_then(|v| v.as_bool()) == Some(true)
                        {
                            // Clear the spinner line and break
                            print!("\r\x1b[2K");
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                            break true;
                        }
                        "Starting sandbox container...".to_string()
                    } else {
                        // Server is up and no sandbox needed — done
                        print!("\r\x1b[2K");
                        let _ = std::io::Write::flush(&mut std::io::stdout());
                        break true;
                    }
                }
                Err(_) => "Starting server...".to_string(),
            },
            Err(_) => "Starting server...".to_string(),
        };

        // Update spinner on every tick
        let elapsed = start.elapsed().as_secs();
        let frame = SPINNER[spinner_idx % SPINNER.len()];
        spinner_idx += 1;

        // Only log phase transitions (avoid spamming the same message)
        if phase != last_phase {
            last_phase.clone_from(&phase);
        }

        print!("\r\x1b[2K  {frame} {phase} ({elapsed}s)");
        let _ = std::io::Write::flush(&mut std::io::stdout());
    };

    if ready {
        println!("  Autopilot ready ✓");
    } else {
        // Clear spinner line before printing error
        print!("\r\x1b[2K");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        eprintln!(
            "  ✗ Timed out waiting for autopilot to become healthy ({}s)",
            max_wait.as_secs()
        );
        eprintln!();
        eprintln!("  Troubleshoot:");
        eprintln!("    vac autopilot logs -c server    View server logs");
        eprintln!("    vac autopilot status            Check component health");
        if expects_sandbox {
            eprintln!("    docker ps                           Verify sandbox container");
        }
        return Err(format!(
            "Autopilot did not become healthy within {}s",
            max_wait.as_secs()
        ));
    }

    // Clean post-start status summary
    let schedule_count = saved_config.schedules.len();
    let channel_list = vac_messaging_gateway::GatewayConfig::load(
        config_path.as_path(),
        &vac_messaging_gateway::GatewayCliFlags::default(),
    )
    .map(|c| c.enabled_channels().join(", "))
    .unwrap_or_default();
    let resolved_tool_policy = resolve_server_tool_policy(
        config.allowed_tools.as_ref(),
        config.auto_approve.as_ref(),
        effective_server.auto_approve_all,
    );

    println!();
    println!("  Autopilot is running.");
    println!();
    println!("  Server      http://{}", effective_server.listen);
    println!(
        "  Tools       {}",
        describe_tool_policy(&resolved_tool_policy)
    );
    println!(
        "  Schedules   {}",
        if schedule_count > 0 {
            format!("{} active", schedule_count)
        } else {
            "none (edit ~/.vac/autopilot.toml)".to_string()
        }
    );
    println!(
        "  Channels    {}",
        if channel_list.is_empty() {
            "none (vac autopilot channel add)".to_string()
        } else {
            channel_list
        }
    );
    println!();
    println!("  View logs   vac autopilot logs");
    println!("  Status      vac autopilot status");
    println!("  Stop        vac down");

    Ok(())
}

pub(super) async fn start_foreground_runtime(
    config: &AppConfig,
    options: &StartOptions,
) -> Result<(), String> {
    // --- Per-component file logging ---
    // Each runtime gets its own log file under ~/.vac/autopilot/logs/.
    // Guards must be held for the lifetime of the runtime to ensure logs are flushed.
    let log_dir = autopilot_log_dir();
    std::fs::create_dir_all(&log_dir)
        .map_err(|e| format!("Failed to create autopilot log directory: {}", e))?;

    // TODO: add log rotation (daily or size-based) via tracing_appender::rolling::daily()
    let scheduler_appender = tracing_appender::rolling::never(&log_dir, "scheduler.log");
    let server_appender = tracing_appender::rolling::never(&log_dir, "server.log");
    let gateway_appender = tracing_appender::rolling::never(&log_dir, "gateway.log");

    let (scheduler_nb, _scheduler_guard) = tracing_appender::non_blocking(scheduler_appender);
    let (server_nb, _server_guard) = tracing_appender::non_blocking(server_appender);
    let (gateway_nb, _gateway_guard) = tracing_appender::non_blocking(gateway_appender);

    {
        use tracing_subscriber::fmt::writer::MakeWriterExt;
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        // Each component gets its own fmt layer with a target-based filter.
        let scheduler_layer = tracing_subscriber::fmt::layer()
            .with_writer(scheduler_nb.with_filter(|meta: &tracing::Metadata<'_>| {
                meta.target().starts_with("vac::commands::watch")
            }))
            .with_target(true)
            .with_ansi(false);

        let server_layer = tracing_subscriber::fmt::layer()
            .with_writer(server_nb.with_filter(|meta: &tracing::Metadata<'_>| {
                meta.target().starts_with("vac_broker")
            }))
            .with_target(true)
            .with_ansi(false);

        let gateway_layer = tracing_subscriber::fmt::layer()
            .with_writer(gateway_nb.with_filter(|meta: &tracing::Metadata<'_>| {
                meta.target().starts_with("vac_messaging_gateway")
            }))
            .with_target(true)
            .with_ansi(false);

        let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info,vac=info,vac_broker=info,vac_messaging_gateway=info".into());

        // Best-effort: if a global subscriber is already set (e.g. --debug), skip.
        let _ = tracing_subscriber::registry()
            .with(env_filter)
            .with(scheduler_layer)
            .with(server_layer)
            .with(gateway_layer)
            .try_init();
    }

    // --- 1. Server runtime (initialize before scheduler to avoid sqlite3Close/sqlite3_open
    //     race on libsql's global state when run_scheduler exits early) ---
    let bind = options.bind.clone();
    validate_no_auth_bind(&bind, options.no_auth)?;
    let (auth_config, generated_auth_token) = if options.no_auth {
        (vac_broker::AuthConfig::disabled(), None)
    } else {
        let token = vac_foundation::utils::generate_password(64, true);
        (vac_broker::AuthConfig::token(token.clone()), Some(token))
    };

    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .map_err(|e| format!("Failed to bind {}: {}", bind, e))?;

    let listener_addr = listener
        .local_addr()
        .map_err(|e| format!("Failed to inspect listener address: {}", e))?;

    let runtime_client = crate::commands::build_agent_client(config).await?;
    let storage = runtime_client.session_storage().clone();

    let events = Arc::new(vac_broker::EventLog::new(4096));
    let idempotency = Arc::new(vac_broker::IdempotencyStore::new(
        std::time::Duration::from_secs(24 * 60 * 60),
    ));
    let inference = Arc::new(
        vac_provider_core::Inference::builder()
            .with_registry(runtime_client.provider_core().registry().clone())
            .build()
            .map_err(|e| format!("Failed to initialize inference runtime: {}", e))?,
    );

    let mut models = runtime_client.list_models().await;
    let requested_model = options.model.clone().or(config.model.clone());
    let resolved_tool_policy = resolve_server_tool_policy(
        config.allowed_tools.as_ref(),
        config.auto_approve.as_ref(),
        options.auto_approve_all,
    );

    let requested_model_from_catalog = requested_model.as_deref().and_then(|name| {
        if let Some((provider, id)) = name.split_once('/') {
            return models
                .iter()
                .find(|model| model.provider == provider && model.id == id)
                .cloned();
        }
        models.iter().find(|model| model.id == name).cloned()
    });

    let requested_custom_model = requested_model.as_deref().and_then(|name| {
        name.split_once('/')
            .map(|(provider, id)| vac_provider_core::Model::custom(id, provider))
    });

    let default_model = requested_model_from_catalog
        .clone()
        .or(requested_custom_model)
        .or_else(|| models.first().cloned())
        .or_else(|| Some(vac_provider_core::Model::custom("gpt-4o-mini", "openai")));

    if let Some(requested) = requested_model.as_deref()
        && requested_model_from_catalog.is_none()
    {
        if requested.contains('/') {
            eprintln!(
                "⚠ Requested model '{}' is not in the catalog; using it as a custom model id.",
                requested
            );
        } else if let Some(resolved) = default_model.as_ref() {
            eprintln!(
                "⚠ Requested model '{}' not found in catalog; using fallback '{}/{}'.",
                requested, resolved.provider, resolved.id
            );
        }
    }

    if models.is_empty()
        && let Some(default_model) = default_model.clone()
    {
        models.push(default_model);
    }

    let mcp_allowed_tools =
        mcp_allowed_tools_from_policy(&resolved_tool_policy, config.allowed_tools.as_ref());

    let mcp_init_config = crate::commands::agent::run::mcp_init::McpInitConfig {
        redact_secrets: true, // applied in proxy layer
        privacy_mode: false,  // applied in proxy layer
        enabled_tools: vac_mcp_server::EnabledToolsConfig { slack: false },
        enable_mtls: true,
        enable_subagents: true,
        allowed_tools: mcp_allowed_tools,
        subagent_config: vac_mcp_server::SubagentConfig {
            profile_name: Some(config.profile_name.clone()),
            config_path: Some(config.config_path.clone()),
            model: config.subagent_model(),
        },
        ..crate::commands::agent::run::mcp_init::McpInitConfig::default()
    };

    let mcp_init_result = crate::commands::agent::run::mcp_init::initialize_mcp_server_and_tools(
        config,
        mcp_init_config,
        None,
    )
    .await
    .map_err(|e| format!("Failed to initialize MCP stack: {}", e))?;

    let mcp_tools = mcp_init_result
        .mcp_tools
        .iter()
        .map(|tool| vac_provider_core::Tool {
            tool_type: "function".to_string(),
            function: vac_provider_core::ToolFunction {
                name: tool.name.as_ref().to_string(),
                description: tool
                    .description
                    .as_ref()
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                parameters: serde_json::Value::Object((*tool.input_schema).clone()),
            },
            provider_options: None,
        })
        .collect();

    // Pre-load remote skills context (fetched from rulebooks API) and capture
    // startup project directory for gateway/channel sessions.
    let startup_project_dir = startup_project_dir();
    let startup_remote_skills = match load_remote_skills_context(&runtime_client).await {
        Ok(context_files) => {
            tracing::info!(
                count = context_files.len(),
                "Loaded remote skills context for session bootstrap"
            );
            context_files
        }
        Err(error) => {
            tracing::warn!(error = %error, "Failed to load remote skills context; sessions will start without it");
            Vec::new()
        }
    };

    let app_state = vac_broker::AppState::new(
        storage,
        events,
        idempotency,
        inference,
        models,
        default_model,
        resolved_tool_policy.clone(),
    )
    .with_base_system_prompt(Some(DEFAULT_SYSTEM_PROMPT.trim().to_string()))
    .with_project_dir(startup_project_dir)
    .with_skills(startup_remote_skills)
    .with_mcp(
        mcp_init_result.client,
        mcp_tools,
        Some(mcp_init_result.server_shutdown_tx),
        Some(mcp_init_result.proxy_shutdown_tx),
    );

    // --- 1b. Sandbox configuration (warden + container image) ---
    let warden_path = crate::commands::warden::get_warden_plugin_path().await;
    let vac_image = crate::commands::warden::vac_agent_image();
    let volumes = crate::commands::warden::prepare_volumes(config, false);
    // Pre-create named volumes to prevent race conditions when parallel sandboxes start
    vac_foundation::container::ensure_named_volumes_exist();

    let sandbox_mode = &options.sandbox_mode;
    let sandbox_user_mapping = sandbox_user_mapping_for_mode(sandbox_mode);

    let sandbox_config = vac_broker::SandboxConfig {
        warden_path,
        image: vac_image.clone(),
        volumes,
        mode: sandbox_mode.clone(),
        user_mapping: sandbox_user_mapping,
    };
    tracing::info!(image = %vac_image, mode = %sandbox_mode, warden = %sandbox_config.warden_path, "Sandbox config initialized");
    let app_state = app_state.with_sandbox(sandbox_config.clone());

    // If persistent mode, spawn the sandbox now so sessions get near-zero startup overhead.
    // This is a hard requirement — if the sandbox fails to start, the server cannot operate.
    let app_state = if *sandbox_mode == vac_broker::SandboxMode::Persistent {
        tracing::info!("Persistent sandbox mode: spawning sandbox at startup");
        let persistent = vac_broker::PersistentSandbox::spawn(&sandbox_config)
            .await
            .map_err(|e| format!("Failed to spawn persistent sandbox: {e}. The server requires a healthy sandbox to operate. Check Docker is running and the image is available."))?;
        app_state.with_persistent_sandbox(persistent)
    } else {
        app_state
    };

    // --- 2. Loopback connection for schedule + gateway runtimes ---
    let loopback_url = loopback_server_url(listener_addr);
    let loopback_token = if options.no_auth {
        String::new()
    } else {
        generated_auth_token.clone().unwrap_or_default()
    };

    // --- 3. Gateway runtime ---
    let config_path = AutopilotConfigFile::path();

    let gateway_cli = vac_messaging_gateway::GatewayCliFlags {
        url: Some(loopback_url.clone()),
        token: Some(loopback_token.clone()),
        ..Default::default()
    };

    let mut gateway_cfg =
        vac_messaging_gateway::GatewayConfig::load(config_path.as_path(), &gateway_cli)
            .unwrap_or_default();

    for warning in gateway_cfg.check_deprecations() {
        tracing::warn!("{}", warning);
    }

    apply_gateway_policy_from_resolved_tools(&mut gateway_cfg, &resolved_tool_policy);

    let gateway_profile_overrides = vac_messaging_gateway::runtime::DispatcherProfileOverrides::new(
        gateway_channel_profiles_with_default(&gateway_cfg.channels, &config.profile_name),
        Arc::new(ProfileRunOverrideResolver::new(config.config_path.clone())),
    );

    let gateway_runtime = if gateway_cfg.has_channels() {
        match vac_messaging_gateway::Gateway::new_with_profile_overrides(
            gateway_cfg,
            gateway_profile_overrides,
        )
        .await
        {
            Ok(gw) => Some(Arc::new(gw)),
            Err(e) => {
                eprintln!(
                    "⚠ Failed to initialize gateway: {}. Continuing without channels.",
                    e
                );
                None
            }
        }
    } else {
        None
    };

    // --- Build HTTP app ---
    let refresh_state = app_state.clone();
    let refresh_client = runtime_client.clone();
    let (refresh_shutdown_tx, mut refresh_shutdown_rx) = tokio::sync::watch::channel(false);
    let refresh_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
                    if let Err(error) = refresh_state.refresh_mcp_tools().await {
                        eprintln!("[mcp-refresh] {}", error);
                    }

                    match load_remote_skills_context(&refresh_client).await {
                        Ok(context_files) => {
                            refresh_state.replace_skills(context_files).await;
                        }
                        Err(error) => {
                            eprintln!("[context-refresh] {}", error);
                        }
                    }
                }
                changed = refresh_shutdown_rx.changed() => {
                    if changed.is_err() || *refresh_shutdown_rx.borrow() {
                        break;
                    }
                }
            }
        }
    });

    let shutdown_state = app_state.clone();
    let shutdown_refresh_tx = refresh_shutdown_tx.clone();
    let server_model_id = app_state
        .default_model
        .as_ref()
        .map(|m| format!("{}/{}", m.provider, m.id));

    let base_app = vac_broker::router(app_state, auth_config);
    let app = if let Some(gateway_runtime) = gateway_runtime.as_ref() {
        let gateway_routes = gateway_runtime.api_router();
        base_app.nest_service("/v1/gateway", gateway_routes.into_service())
    } else {
        base_app
    };

    let gateway_cancel = tokio_util::sync::CancellationToken::new();
    let gateway_task = if let Some(gateway_runtime) = gateway_runtime.as_ref() {
        let gateway_runtime = gateway_runtime.clone();
        let cancel = gateway_cancel.clone();
        Some(tokio::spawn(
            async move { gateway_runtime.run(cancel).await },
        ))
    } else {
        None
    };
    let gateway_cancel_for_shutdown = gateway_cancel.clone();

    // --- 4. Schedule runtime (spawned AFTER all SQLite initialization to avoid
    //     sqlite3Close/sqlite3_open race in libsql on musl) ---
    let watch_allowed_tools: HashSet<String> = approved_tools_from_policy(&resolved_tool_policy)
        .into_iter()
        .collect();

    let schedule_server = crate::commands::watch::AgentServerConnection {
        url: loopback_url,
        token: loopback_token,
        model: server_model_id,
        default_allowed_tools: watch_allowed_tools,
        boot_profile: config.profile_name.clone(),
        config_path: config.config_path.clone(),
    };
    let schedule_task = tokio::spawn(async move {
        if let Err(error) = crate::commands::watch::commands::run_scheduler(schedule_server).await {
            eprintln!("Schedule runtime exited: {}", error);
        }
    });

    // --- Print status ---
    println!("Autopilot running in foreground. Press Ctrl+C to stop.");
    println!();
    println!("  Server      http://{}", bind);
    if let Some(ref token) = generated_auth_token {
        if options.show_token {
            println!("  Auth token  Bearer {}", token);
        } else {
            println!("  Auth        enabled");
        }
    } else if options.no_auth {
        println!("  Auth        disabled");
    }
    if gateway_runtime.is_some() {
        println!("  Gateway     enabled");
    } else {
        println!("  Gateway     no channels configured");
    }
    println!(
        "  Tools       {}",
        describe_tool_policy(&resolved_tool_policy)
    );

    // --- Shutdown handler ---
    let shutdown = async move {
        wait_for_shutdown_signal().await;

        gateway_cancel_for_shutdown.cancel();

        for (session_id, run_id) in shutdown_state.run_manager.running_runs().await {
            let _ = shutdown_state
                .run_manager
                .cancel_run(session_id, run_id)
                .await;
        }

        let drain_deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if shutdown_state.run_manager.running_runs().await.is_empty()
                || tokio::time::Instant::now() >= drain_deadline
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let _ = shutdown_refresh_tx.send(true);

        if let Some(tx) = shutdown_state.mcp_server_shutdown_tx.as_ref() {
            let _ = tx.send(());
        }
        if let Some(tx) = shutdown_state.mcp_proxy_shutdown_tx.as_ref() {
            let _ = tx.send(());
        }

        // Kill the persistent sandbox container (if one is running)
        if let Some(ref sandbox) = shutdown_state.persistent_sandbox {
            sandbox.kill().await;
        }
    };

    // --- Run server ---
    let serve_result = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await;

    // --- Cleanup ---
    gateway_cancel.cancel();
    if let Some(task) = gateway_task {
        match task.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => eprintln!("Gateway runtime error: {}", e),
            Err(e) => eprintln!("Gateway runtime task failed: {}", e),
        }
    }

    let _ = refresh_shutdown_tx.send(true);
    if !refresh_task.is_finished() {
        refresh_task.abort();
    }
    let _ = refresh_task.await;

    // Abort the schedule task
    schedule_task.abort();
    let _ = schedule_task.await;

    println!();
    println!("Shutting down...");

    serve_result.map_err(|e| format!("Server error: {}", e))?;

    Ok(())
}

pub(super) fn loopback_server_url(listener_addr: std::net::SocketAddr) -> String {
    let port = listener_addr.port();
    if listener_addr.ip().is_ipv6() {
        format!("http://[::1]:{port}")
    } else {
        format!("http://127.0.0.1:{port}")
    }
}
