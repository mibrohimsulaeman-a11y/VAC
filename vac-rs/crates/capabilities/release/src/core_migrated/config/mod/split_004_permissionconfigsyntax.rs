#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PermissionConfigSyntax {
    Legacy,
    Profiles,
}

#[derive(Debug, Deserialize, Default)]
struct PermissionSelectionToml {
    default_permissions: Option<String>,
    sandbox_mode: Option<SandboxMode>,
}

fn resolve_permission_config_syntax(
    config_layer_stack: &ConfigLayerStack,
    cfg: &ConfigToml,
    sandbox_mode_override: Option<SandboxMode>,
    profile_sandbox_mode: Option<SandboxMode>,
) -> Option<PermissionConfigSyntax> {
    if sandbox_mode_override.is_some() {
        return Some(PermissionConfigSyntax::Legacy);
    }

    let session_flags_select_profiles = config_layer_stack
        .get_layers(
            ConfigLayerStackOrdering::HighestPrecedenceFirst,
            /*include_disabled*/ false,
        )
        .into_iter()
        .find(|layer| matches!(layer.name, ConfigLayerSource::SessionFlags))
        .and_then(|layer| {
            layer
                .config
                .clone()
                .try_into::<PermissionSelectionToml>()
                .ok()
        })
        .is_some_and(|selection| selection.default_permissions.is_some());
    if session_flags_select_profiles {
        return Some(PermissionConfigSyntax::Profiles);
    }

    if profile_sandbox_mode.is_some() {
        return Some(PermissionConfigSyntax::Legacy);
    }

    let mut selection = None;
    for layer in config_layer_stack.get_layers(
        ConfigLayerStackOrdering::LowestPrecedenceFirst,
        /*include_disabled*/ false,
    ) {
        let Ok(layer_selection) = layer.config.clone().try_into::<PermissionSelectionToml>() else {
            continue;
        };

        if layer_selection.sandbox_mode.is_some() {
            selection = Some(PermissionConfigSyntax::Legacy);
        }
        if layer_selection.default_permissions.is_some() {
            selection = Some(PermissionConfigSyntax::Profiles);
        }
    }

    selection.or_else(|| {
        if cfg.default_permissions.is_some() {
            Some(PermissionConfigSyntax::Profiles)
        } else if cfg.sandbox_mode.is_some() {
            Some(PermissionConfigSyntax::Legacy)
        } else {
            None
        }
    })
}

fn apply_managed_filesystem_constraints(
    file_system_sandbox_policy: &mut FileSystemSandboxPolicy,
    filesystem_constraints: &vac_config::FilesystemConstraints,
) {
    for deny_read in &filesystem_constraints.deny_read {
        let deny_entry = if deny_read.contains_glob() {
            vac_protocol::permissions::FileSystemSandboxEntry {
                path: vac_protocol::permissions::FileSystemPath::GlobPattern {
                    pattern: deny_read.as_str().to_string(),
                },
                access: vac_protocol::permissions::FileSystemAccessMode::None,
            }
        } else {
            let Ok(path) = AbsolutePathBuf::try_from(deny_read.as_str()) else {
                continue;
            };
            vac_protocol::permissions::FileSystemSandboxEntry {
                path: vac_protocol::permissions::FileSystemPath::Path { path },
                access: vac_protocol::permissions::FileSystemAccessMode::None,
            }
        };
        if !file_system_sandbox_policy
            .entries
            .iter()
            .any(|existing| existing == &deny_entry)
        {
            file_system_sandbox_policy.entries.push(deny_entry);
        }
    }
}

/// Optional overrides for user configuration (e.g., from CLI flags).
#[derive(Default, Debug, Clone)]
pub struct ConfigOverrides {
    pub model: Option<String>,
    pub review_model: Option<String>,
    pub cwd: Option<PathBuf>,
    pub approval_policy: Option<AskForApproval>,
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    pub sandbox_mode: Option<SandboxMode>,
    pub permission_profile: Option<PermissionProfile>,
    pub default_permissions: Option<String>,
    pub model_provider: Option<String>,
    pub service_tier: Option<Option<ServiceTier>>,
    pub config_profile: Option<String>,
    pub vac_self_exe: Option<PathBuf>,
    pub vac_linux_sandbox_exe: Option<PathBuf>,
    pub main_execve_wrapper_exe: Option<PathBuf>,
    pub zsh_path: Option<PathBuf>,
    pub base_instructions: Option<String>,
    pub developer_instructions: Option<String>,
    pub personality: Option<Personality>,
    pub compact_prompt: Option<String>,
    pub include_apply_patch_tool: Option<bool>,
    pub show_raw_agent_reasoning: Option<bool>,
    pub tools_web_search_request: Option<bool>,
    pub ephemeral: Option<bool>,
    /// Additional directories that should be treated as writable roots for this session.
    pub additional_writable_roots: Vec<PathBuf>,
}

/// Resolves the OSS provider from CLI override, profile config, or global config.
/// Returns `None` if no provider is configured at any level.
pub fn resolve_oss_provider(
    explicit_provider: Option<&str>,
    config_toml: &ConfigToml,
    config_profile: Option<String>,
) -> Option<String> {
    if let Some(provider) = explicit_provider {
        // Explicit provider specified (e.g., via --local-provider)
        Some(provider.to_string())
    } else {
        // Check profile config first, then global config
        let profile = config_toml.get_config_profile(config_profile).ok();
        if let Some(profile) = &profile {
            // Check if profile has an oss provider
            if let Some(profile_oss_provider) = &profile.oss_provider {
                Some(profile_oss_provider.clone())
            }
            // If not then check if the toml has an oss provider
            else {
                config_toml.oss_provider.clone()
            }
        } else {
            config_toml.oss_provider.clone()
        }
    }
}

/// Resolve the web search mode from explicit config and feature flags.
fn resolve_web_search_mode(
    config_toml: &ConfigToml,
    config_profile: &ConfigProfile,
    features: &Features,
) -> Option<WebSearchMode> {
    if let Some(mode) = config_profile.web_search.or(config_toml.web_search) {
        return Some(mode);
    }
    if features.enabled(Feature::WebSearchCached) {
        return Some(WebSearchMode::Cached);
    }
    if features.enabled(Feature::WebSearchRequest) {
        return Some(WebSearchMode::Live);
    }
    None
}

fn resolve_web_search_config(
    config_toml: &ConfigToml,
    config_profile: &ConfigProfile,
) -> Option<WebSearchConfig> {
    let base = config_toml
        .tools
        .as_ref()
        .and_then(|tools| tools.web_search.as_ref());
    let profile = config_profile
        .tools
        .as_ref()
        .and_then(|tools| tools.web_search.as_ref());

    match (base, profile) {
        (None, None) => None,
        (Some(base), None) => Some(base.clone().into()),
        (None, Some(profile)) => Some(profile.clone().into()),
        (Some(base), Some(profile)) => Some(base.merge(profile).into()),
    }
}

fn resolve_multi_agent_v2_config(
    config_toml: &ConfigToml,
    config_profile: &ConfigProfile,
) -> MultiAgentV2Config {
    let base = multi_agent_v2_toml_config(config_toml.features.as_ref());
    let profile = multi_agent_v2_toml_config(config_profile.features.as_ref());
    let default = MultiAgentV2Config::default();

    let max_concurrent_threads_per_session = profile
        .and_then(|config| config.max_concurrent_threads_per_session)
        .or_else(|| base.and_then(|config| config.max_concurrent_threads_per_session))
        .unwrap_or(default.max_concurrent_threads_per_session);
    let min_wait_timeout_ms = profile
        .and_then(|config| config.min_wait_timeout_ms)
        .or_else(|| base.and_then(|config| config.min_wait_timeout_ms))
        .unwrap_or(default.min_wait_timeout_ms);
    let usage_hint_enabled = profile
        .and_then(|config| config.usage_hint_enabled)
        .or_else(|| base.and_then(|config| config.usage_hint_enabled))
        .unwrap_or(default.usage_hint_enabled);
    let usage_hint_text = profile
        .and_then(|config| config.usage_hint_text.as_ref())
        .or_else(|| base.and_then(|config| config.usage_hint_text.as_ref()))
        .cloned()
        .or(default.usage_hint_text);
    let root_agent_usage_hint_text = profile
        .and_then(|config| config.root_agent_usage_hint_text.as_ref())
        .or_else(|| base.and_then(|config| config.root_agent_usage_hint_text.as_ref()))
        .cloned()
        .or(default.root_agent_usage_hint_text);
    let subagent_usage_hint_text = profile
        .and_then(|config| config.subagent_usage_hint_text.as_ref())
        .or_else(|| base.and_then(|config| config.subagent_usage_hint_text.as_ref()))
        .cloned()
        .or(default.subagent_usage_hint_text);
    let hide_spawn_agent_metadata = profile
        .and_then(|config| config.hide_spawn_agent_metadata)
        .or_else(|| base.and_then(|config| config.hide_spawn_agent_metadata))
        .unwrap_or(default.hide_spawn_agent_metadata);

    MultiAgentV2Config {
        max_concurrent_threads_per_session,
        min_wait_timeout_ms,
        usage_hint_enabled,
        usage_hint_text,
        root_agent_usage_hint_text,
        subagent_usage_hint_text,
        hide_spawn_agent_metadata,
    }
}

fn resolve_terminal_resize_reflow_config(config_toml: &ConfigToml) -> TerminalResizeReflowConfig {
    let Some(tui) = config_toml.tui.as_ref() else {
        return TerminalResizeReflowConfig::default();
    };

    TerminalResizeReflowConfig {
        max_rows: match tui.terminal_resize_reflow_max_rows {
            Some(0) => TerminalResizeReflowMaxRows::Disabled,
            Some(rows) => TerminalResizeReflowMaxRows::Limit(rows),
            None => TerminalResizeReflowMaxRows::Auto,
        },
    }
}

fn multi_agent_v2_toml_config(features: Option<&FeaturesToml>) -> Option<&MultiAgentV2ConfigToml> {
    match features?.multi_agent_v2.as_ref()? {
        FeatureToml::Enabled(_) => None,
        FeatureToml::Config(config) => Some(config),
    }
}

fn apps_mcp_path_override_toml_config(
    features: Option<&FeaturesToml>,
) -> Option<&AppsMcpPathOverrideConfigToml> {
    match features?.apps_mcp_path_override.as_ref()? {
        FeatureToml::Enabled(_) => None,
        FeatureToml::Config(config) => Some(config),
    }
}

pub(crate) fn resolve_web_search_mode_for_turn(
    web_search_mode: &Constrained<WebSearchMode>,
    permission_profile: &PermissionProfile,
) -> WebSearchMode {
    let preferred = web_search_mode.value();

    if matches!(permission_profile, PermissionProfile::Disabled)
        && preferred != WebSearchMode::Disabled
    {
        for mode in [
            WebSearchMode::Live,
            WebSearchMode::Cached,
            WebSearchMode::Disabled,
        ] {
            if web_search_mode.can_set(&mode).is_ok() {
                return mode;
            }
        }
    } else {
        if web_search_mode.can_set(&preferred).is_ok() {
            return preferred;
        }
        for mode in [
            WebSearchMode::Cached,
            WebSearchMode::Live,
            WebSearchMode::Disabled,
        ] {
            if web_search_mode.can_set(&mode).is_ok() {
                return mode;
            }
        }
    }

    WebSearchMode::Disabled
}

impl Config {
    #[cfg(test)]
    async fn load_from_base_config_with_overrides(
        cfg: ConfigToml,
        overrides: ConfigOverrides,
        vac_home: AbsolutePathBuf,
    ) -> std::io::Result<Self> {
        // Note this ignores requirements.toml enforcement for tests.
        let config_layer_stack = ConfigLayerStack::default();
        Self::load_config_with_layer_stack(
            LOCAL_FS.as_ref(),
            cfg,
            overrides,
            vac_home,
            config_layer_stack,
        )
        .await
    }

    pub(crate) async fn load_config_with_layer_stack(
        fs: &dyn ExecutorFileSystem,
        cfg: ConfigToml,
        overrides: ConfigOverrides,
        vac_home: AbsolutePathBuf,
        config_layer_stack: ConfigLayerStack,
    ) -> std::io::Result<Self> {
        // Keep the large config-construction future off small test thread stacks.
        Box::pin(async move {
        validate_model_providers(&cfg.model_providers)
            .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidInput, message))?;
        // Ensure that every field of ConfigRequirements is applied to the final
        // Config.
        let ConfigRequirements {
            approval_policy: mut constrained_approval_policy,
            approvals_reviewer: mut constrained_approvals_reviewer,
            permission_profile: mut constrained_permission_profile,
            web_search_mode: mut constrained_web_search_mode,
            feature_requirements,
            managed_hooks: _,
            mcp_servers,
            plugins: _,
            exec_policy: _,
            enforce_residency,
            network: network_requirements,
            filesystem: filesystem_requirements,
            guardian_policy_config_source: _,
        } = config_layer_stack.requirements().clone();

        let user_instructions = AgentsMdManager::load_global_instructions(Some(&vac_home))
            .map(|loaded| loaded.contents);
        let mut startup_warnings = config_layer_stack
            .startup_warnings()
            .unwrap_or_default()
            .to_vec();

        // Destructure ConfigOverrides fully to ensure all overrides are applied.
        let ConfigOverrides {
            model,
            review_model: override_review_model,
            cwd,
            approval_policy: approval_policy_override,
            approvals_reviewer: approvals_reviewer_override,
            sandbox_mode,
            permission_profile,
            default_permissions: default_permissions_override,
            model_provider,
            service_tier: service_tier_override,
            config_profile: config_profile_key,
            vac_self_exe,
            vac_linux_sandbox_exe,
            main_execve_wrapper_exe,
            zsh_path: zsh_path_override,
            base_instructions,
            developer_instructions,
            personality,
            compact_prompt,
            include_apply_patch_tool: include_apply_patch_tool_override,
            show_raw_agent_reasoning,
            tools_web_search_request: override_tools_web_search_request,
            ephemeral,
            additional_writable_roots,
        } = overrides;

        if sandbox_mode.is_some() && permission_profile.is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "`sandbox_mode` and `permission_profile` overrides cannot both be set",
            ));
        }
        if sandbox_mode.is_some() && default_permissions_override.is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "`sandbox_mode` and `default_permissions` overrides cannot both be set",
            ));
        }
        if permission_profile.is_some() && default_permissions_override.is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "`permission_profile` and `default_permissions` overrides cannot both be set",
            ));
        }

        let active_profile_name = config_profile_key
            .as_ref()
            .or(cfg.profile.as_ref())
            .cloned();
        let config_profile = match active_profile_name.as_ref() {
            Some(key) => cfg
                .profiles
                .get(key)
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("config profile `{key}` not found"),
                    )
                })?
                .clone(),
            None => ConfigProfile::default(),
        };
        let tool_suggest = resolve_tool_suggest_config(&cfg, &config_layer_stack);
        let feature_overrides = FeatureOverrides {
            include_apply_patch_tool: include_apply_patch_tool_override,
            web_search_request: override_tools_web_search_request,
        };

        let configured_features = Features::from_sources(
            FeatureConfigSource {
                features: cfg.features.as_ref(),
                include_apply_patch_tool: None,
                experimental_use_freeform_apply_patch: cfg.experimental_use_freeform_apply_patch,
                experimental_use_unified_exec_tool: cfg.experimental_use_unified_exec_tool,
            },
            FeatureConfigSource {
                features: config_profile.features.as_ref(),
                include_apply_patch_tool: config_profile.include_apply_patch_tool,
                experimental_use_freeform_apply_patch: config_profile
                    .experimental_use_freeform_apply_patch,
                experimental_use_unified_exec_tool: config_profile
                    .experimental_use_unified_exec_tool,
            },
            feature_overrides,
        );
        let features = ManagedFeatures::from_configured_with_warnings(
            configured_features,
            feature_requirements,
            &mut startup_warnings,
        )?;
        let windows_sandbox_mode = resolve_windows_sandbox_mode(&cfg, &config_profile);
        let windows_sandbox_private_desktop =
            resolve_windows_sandbox_private_desktop(&cfg, &config_profile);
        let resolved_cwd = AbsolutePathBuf::try_from(normalize_for_native_workdir({
