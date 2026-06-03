#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MultiAgentV2Config {
    pub max_concurrent_threads_per_session: usize,
    pub min_wait_timeout_ms: i64,
    pub usage_hint_enabled: bool,
    pub usage_hint_text: Option<String>,
    pub root_agent_usage_hint_text: Option<String>,
    pub subagent_usage_hint_text: Option<String>,
    pub hide_spawn_agent_metadata: bool,
}

impl Default for MultiAgentV2Config {
    fn default() -> Self {
        Self {
            max_concurrent_threads_per_session:
                DEFAULT_MULTI_AGENT_V2_MAX_CONCURRENT_THREADS_PER_SESSION,
            min_wait_timeout_ms: DEFAULT_MULTI_AGENT_V2_MIN_WAIT_TIMEOUT_MS,
            usage_hint_enabled: true,
            usage_hint_text: None,
            root_agent_usage_hint_text: None,
            subagent_usage_hint_text: None,
            hide_spawn_agent_metadata: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerminalResizeReflowMaxRows {
    /// Use the runtime terminal detector to choose a scrollback-sized cap.
    #[default]
    Auto,
    /// Keep all rendered transcript rows during resize reflow.
    Disabled,
    /// Keep at most this many rendered transcript rows during resize reflow.
    Limit(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TerminalResizeReflowConfig {
    pub max_rows: TerminalResizeReflowMaxRows,
}

impl AuthManagerConfig for Config {
    fn vac_home(&self) -> PathBuf {
        self.vac_home.to_path_buf()
    }

    fn cli_auth_credentials_store_mode(&self) -> AuthCredentialsStoreMode {
        self.cli_auth_credentials_store_mode
    }

    fn forced_chatgpt_workspace_id(&self) -> Option<String> {
        self.forced_chatgpt_workspace_id.clone()
    }

    fn chatgpt_base_url(&self) -> String {
        self.chatgpt_base_url.clone()
    }
}

#[derive(Clone, Default)]
pub struct ConfigBuilder {
    vac_home: Option<PathBuf>,
    cli_overrides: Option<Vec<(String, TomlValue)>>,
    harness_overrides: Option<ConfigOverrides>,
    loader_overrides: Option<LoaderOverrides>,
    thread_config_loader: Option<Arc<dyn ThreadConfigLoader>>,
    fallback_cwd: Option<PathBuf>,
}

impl ConfigBuilder {
    pub fn vac_home(mut self, vac_home: PathBuf) -> Self {
        self.vac_home = Some(vac_home);
        self
    }

    pub fn cli_overrides(mut self, cli_overrides: Vec<(String, TomlValue)>) -> Self {
        self.cli_overrides = Some(cli_overrides);
        self
    }

    pub fn harness_overrides(mut self, harness_overrides: ConfigOverrides) -> Self {
        self.harness_overrides = Some(harness_overrides);
        self
    }

    pub fn loader_overrides(mut self, loader_overrides: LoaderOverrides) -> Self {
        self.loader_overrides = Some(loader_overrides);
        self
    }

    pub fn thread_config_loader(
        mut self,
        thread_config_loader: Arc<dyn ThreadConfigLoader>,
    ) -> Self {
        self.thread_config_loader = Some(thread_config_loader);
        self
    }

    pub fn fallback_cwd(mut self, fallback_cwd: Option<PathBuf>) -> Self {
        self.fallback_cwd = fallback_cwd;
        self
    }

    pub async fn build(self) -> std::io::Result<Config> {
        // Keep the large config-loading future off small runtime thread stacks.
        Box::pin(self.build_inner()).await
    }

    async fn build_inner(self) -> std::io::Result<Config> {
        let Self {
            vac_home,
            cli_overrides,
            harness_overrides,
            loader_overrides,
            thread_config_loader,
            fallback_cwd,
        } = self;
        let vac_home = match vac_home {
            Some(vac_home) => AbsolutePathBuf::from_absolute_path(vac_home)?,
            None => find_vac_home()?,
        };
        let cli_overrides = cli_overrides.unwrap_or_default();
        let mut harness_overrides = harness_overrides.unwrap_or_default();
        let loader_overrides = loader_overrides.unwrap_or_default();
        let cwd_override = harness_overrides.cwd.as_deref().or(fallback_cwd.as_deref());
        let cwd = match cwd_override {
            Some(path) => AbsolutePathBuf::relative_to_current_dir(path)?,
            None => AbsolutePathBuf::current_dir()?,
        };
        harness_overrides.cwd = Some(cwd.to_path_buf());
        let config_layer_stack = load_config_layers_state(
            LOCAL_FS.as_ref(),
            &vac_home,
            Some(cwd),
            &cli_overrides,
            loader_overrides,
            thread_config_loader
                .as_deref()
                .unwrap_or(&vac_config::NoopThreadConfigLoader),
        )
        .await?;
        let merged_toml = config_layer_stack.effective_config();

        // Note that each layer in ConfigLayerStack should have resolved
        // relative paths to absolute paths based on the parent folder of the
        // respective config file, so we should be safe to deserialize without
        // AbsolutePathBufGuard here.
        let config_toml: ConfigToml = match merged_toml.try_into() {
            Ok(config_toml) => config_toml,
            Err(err) => {
                if let Some(config_error) = vac_config::first_layer_config_error::<ConfigToml>(
                    &config_layer_stack,
                    vac_config::CONFIG_TOML_FILE,
                )
                .await
                {
                    return Err(vac_config::io_error_from_config_error(
                        std::io::ErrorKind::InvalidData,
                        config_error,
                        Some(err),
                    ));
                }
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, err));
            }
        };
        let config_lock_settings = config_toml
            .debug
            .as_ref()
            .and_then(|debug| debug.config_lockfile.as_ref());
        if let Some(config_lock_load_path) =
            config_lock_settings.and_then(|config_lock| config_lock.load_path.as_ref())
        {
            let allow_vac_version_mismatch = config_lock_settings
                .and_then(|config_lock| config_lock.allow_vac_version_mismatch)
                .unwrap_or(false);
            let save_fields_resolved_from_model_catalog = config_lock_settings
                .and_then(|config_lock| config_lock.save_fields_resolved_from_model_catalog)
                .unwrap_or(true);
            let lockfile_toml = read_config_lock_from_path(config_lock_load_path).await?;
            let expected_lock_config = lockfile_toml.clone();
            let lock_layer = lock_layer_from_config(config_lock_load_path, &lockfile_toml)?;
            let lock_config_toml = config_without_lock_controls(&lockfile_toml.config);
            let lock_config_layer_stack = ConfigLayerStack::new(
                vec![lock_layer],
                config_layer_stack.requirements().clone(),
                config_layer_stack.requirements_toml().clone(),
            )?;
            let mut config = Config::load_config_with_layer_stack(
                LOCAL_FS.as_ref(),
                lock_config_toml,
                harness_overrides,
                vac_home,
                lock_config_layer_stack,
            )
            .await?;
            config.config_lock_toml = Some(Arc::new(expected_lock_config));
            config.config_lock_allow_vac_version_mismatch = allow_vac_version_mismatch;
            config.config_lock_save_fields_resolved_from_model_catalog =
                save_fields_resolved_from_model_catalog;
            return Ok(config);
        }
        Config::load_config_with_layer_stack(
            LOCAL_FS.as_ref(),
            config_toml,
            harness_overrides,
            vac_home,
            config_layer_stack,
        )
        .await
    }

    #[cfg(test)]
    pub(crate) fn without_managed_config_for_tests() -> Self {
        Self::default().loader_overrides(LoaderOverrides::without_managed_config_for_tests())
    }
}

impl Config {
    pub fn legacy_sandbox_policy(&self) -> SandboxPolicy {
        self.permissions.legacy_sandbox_policy(self.cwd.as_path())
    }

    pub fn set_legacy_sandbox_policy(
        &mut self,
        sandbox_policy: SandboxPolicy,
    ) -> ConstraintResult<()> {
        self.permissions
            .set_legacy_sandbox_policy(sandbox_policy, self.cwd.as_path())
    }

    pub fn to_models_manager_config(&self) -> ModelsManagerConfig {
        ModelsManagerConfig {
            model_context_window: self.model_context_window,
            model_auto_compact_token_limit: self.model_auto_compact_token_limit,
            tool_output_token_limit: self.tool_output_token_limit,
            base_instructions: self.base_instructions.clone(),
            personality_enabled: self.features.enabled(Feature::Personality),
            model_supports_reasoning_summaries: self.model_supports_reasoning_summaries,
            model_catalog: self.model_catalog.clone(),
        }
    }

    /// Build the plugin-manager input from the effective config.
    pub fn plugins_config_input(&self) -> PluginsConfigInput {
        PluginsConfigInput::new(
            self.config_layer_stack.clone(),
            self.features.enabled(Feature::Plugins),
            self.features.enabled(Feature::RemotePlugin),
            self.features.enabled(Feature::PluginHooks),
            self.chatgpt_base_url.clone(),
        )
    }

    pub async fn to_mcp_config(
        &self,
        plugins_manager: &vac_core_plugins::PluginsManager,
    ) -> McpConfig {
        let plugins_input = self.plugins_config_input();
        let loaded_plugins = plugins_manager.plugins_for_config(&plugins_input).await;
        let mut configured_mcp_servers = self.mcp_servers.get().clone();
        for plugin in loaded_plugins
            .plugins()
            .iter()
            .filter(|plugin| plugin.is_active())
        {
            let mut plugin_mcp_servers = plugin.mcp_servers.clone();
            filter_plugin_mcp_servers_by_requirements(
                &plugin.config_name,
                &mut plugin_mcp_servers,
                self.config_layer_stack.requirements().plugins.as_ref(),
            );
            for (name, plugin_server) in plugin_mcp_servers {
                configured_mcp_servers.entry(name).or_insert(plugin_server);
            }
        }
        if let Some(mcp_requirements) = self.config_layer_stack.requirements().mcp_servers.as_ref()
            && mcp_requirements.value.is_empty()
        {
            // A present empty allowlist bans all MCPs, including plugin MCPs merged above.
            filter_mcp_servers_by_requirements(&mut configured_mcp_servers, Some(mcp_requirements));
        }

        McpConfig {
            chatgpt_base_url: self.chatgpt_base_url.clone(),
            apps_mcp_path_override: self.apps_mcp_path_override.clone(),
            vac_home: self.vac_home.to_path_buf(),
            mcp_oauth_credentials_store_mode: self.mcp_oauth_credentials_store_mode,
            mcp_oauth_callback_port: self.mcp_oauth_callback_port,
            mcp_oauth_callback_url: self.mcp_oauth_callback_url.clone(),
            skill_mcp_dependency_install_enabled: self
                .features
                .enabled(Feature::SkillMcpDependencyInstall),
            approval_policy: self.permissions.approval_policy.clone(),
            vac_linux_sandbox_exe: self.vac_linux_sandbox_exe.clone(),
            use_legacy_landlock: self.features.use_legacy_landlock(),
            apps_enabled: self.features.enabled(Feature::Apps),
            configured_mcp_servers,
            plugin_capability_summaries: loaded_plugins.capability_summaries().to_vec(),
        }
    }

    /// This is the preferred way to create an instance of [Config].
    pub async fn load_with_cli_overrides(
        cli_overrides: Vec<(String, TomlValue)>,
    ) -> std::io::Result<Self> {
        ConfigBuilder::default()
            .cli_overrides(cli_overrides)
            .build()
            .await
    }

    /// Load a default configuration when user config files are invalid.
    pub async fn load_default_with_cli_overrides(
        cli_overrides: Vec<(String, TomlValue)>,
    ) -> std::io::Result<Self> {
        let vac_home = find_vac_home()?;
        Self::load_default_with_cli_overrides_for_vac_home(vac_home.to_path_buf(), cli_overrides)
            .await
    }

    /// Load a default configuration for a specific VAC home without reading
    /// user, project, or system config layers.
    pub async fn load_default_with_cli_overrides_for_vac_home(
        vac_home: PathBuf,
        cli_overrides: Vec<(String, TomlValue)>,
    ) -> std::io::Result<Self> {
        let mut merged = toml::Value::try_from(ConfigToml::default()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("failed to serialize default config: {e}"),
            )
        })?;
        let cli_layer = vac_config::build_cli_overrides_layer(&cli_overrides);
        vac_config::merge_toml_values(&mut merged, &cli_layer);
        let vac_home = AbsolutePathBuf::from_absolute_path_checked(vac_home)?;
        let config_toml = deserialize_config_toml_with_base(merged, &vac_home)?;
        Self::load_config_with_layer_stack(
            LOCAL_FS.as_ref(),
            config_toml,
            ConfigOverrides::default(),
            vac_home,
            ConfigLayerStack::default(),
        )
        .await
    }

    /// This is a secondary way of creating [Config], which is appropriate when
    /// the harness is meant to be used with a specific configuration that
    /// ignores user settings. For example, the `vac exec` subcommand is
    /// designed to use [AskForApproval::Never] exclusively.
    ///
    /// Further, [ConfigOverrides] contains some options that are not supported
    /// in [ConfigToml], such as `cwd`, `vac_self_exe`, `vac_linux_sandbox_exe`, and
    /// `main_execve_wrapper_exe`.
    pub async fn load_with_cli_overrides_and_harness_overrides(
        cli_overrides: Vec<(String, TomlValue)>,
        harness_overrides: ConfigOverrides,
    ) -> std::io::Result<Self> {
        ConfigBuilder::default()
            .cli_overrides(cli_overrides)
            .harness_overrides(harness_overrides)
            .build()
            .await
    }
}

/// DEPRECATED: Use [Config::load_with_cli_overrides()] instead because working
/// with [ConfigToml] directly means that [ConfigRequirements] have not been
/// applied yet, which risks failing to enforce required constraints.
pub async fn load_config_as_toml_with_cli_overrides(
    vac_home: &Path,
    cwd: Option<&AbsolutePathBuf>,
    cli_overrides: Vec<(String, TomlValue)>,
) -> std::io::Result<ConfigToml> {
    load_config_as_toml_with_cli_and_loader_overrides(
        vac_home,
        cwd,
        cli_overrides,
        LoaderOverrides::default(),
    )
    .await
}

pub async fn load_config_as_toml_with_cli_and_loader_overrides(
    vac_home: &Path,
    cwd: Option<&AbsolutePathBuf>,
    cli_overrides: Vec<(String, TomlValue)>,
    loader_overrides: LoaderOverrides,
) -> std::io::Result<ConfigToml> {
    let config_layer_stack = load_config_layers_state(
        LOCAL_FS.as_ref(),
        vac_home,
        cwd.cloned(),
        &cli_overrides,
        loader_overrides,
        &vac_config::NoopThreadConfigLoader,
    )
    .await?;

    let merged_toml = config_layer_stack.effective_config();
    let cfg = deserialize_config_toml_with_base(merged_toml, vac_home).map_err(|e| {
        tracing::error!("Failed to deserialize overridden config: {e}");
        e
    })?;

    Ok(cfg)
}

pub fn deserialize_config_toml_with_base(
    root_value: TomlValue,
    config_base_dir: &Path,
) -> std::io::Result<ConfigToml> {
    // This guard ensures that any relative paths that is deserialized into an
    // [AbsolutePathBuf] is resolved against `config_base_dir`.
    let _guard = AbsolutePathBufGuard::new(config_base_dir);
    root_value
        .try_into()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Validate user-visible feature settings against managed feature requirements.
