use super::*;

impl PluginsManager {

    pub fn list_marketplaces_for_config(
        &self,
        config: &PluginsConfigInput,
        additional_roots: &[AbsolutePathBuf],
    ) -> Result<ConfiguredMarketplaceListOutcome, MarketplaceError> {
        if !config.plugins_enabled {
            return Ok(ConfiguredMarketplaceListOutcome::default());
        }

        let (installed_plugins, enabled_plugins) = self.configured_plugin_states(config);
        let marketplace_outcome =
            list_marketplaces(&self.marketplace_roots(config, additional_roots))?;
        let mut seen_plugin_keys = HashSet::new();
        let marketplaces = marketplace_outcome
            .marketplaces
            .into_iter()
            .filter_map(|marketplace| {
                let marketplace_name = marketplace.name.clone();
                let plugins = marketplace
                    .plugins
                    .into_iter()
                    .filter_map(|plugin| {
                        let plugin_key = format!("{}@{marketplace_name}", plugin.name);
                        if !seen_plugin_keys.insert(plugin_key.clone()) {
                            return None;
                        }
                        if !self.restriction_product_matches(plugin.policy.products.as_deref()) {
                            return None;
                        }

                        Some(ConfiguredMarketplacePlugin {
                            // Enabled state is keyed by `<plugin>@<marketplace>`, so duplicate
                            // plugin entries from duplicate marketplace files intentionally
                            // resolve to the first discovered source.
                            id: plugin_key.clone(),
                            installed: installed_plugins.contains(&plugin_key),
                            enabled: enabled_plugins.contains(&plugin_key),
                            name: plugin.name,
                            source: plugin.source,
                            policy: plugin.policy,
                            interface: plugin.interface,
                        })
                    })
                    .collect::<Vec<_>>();

                (!plugins.is_empty()).then_some(ConfiguredMarketplace {
                    name: marketplace.name,
                    path: marketplace.path,
                    interface: marketplace.interface,
                    plugins,
                })
            })
            .collect();

        Ok(ConfiguredMarketplaceListOutcome {
            marketplaces,
            errors: marketplace_outcome.errors,
        })
    }

    pub async fn read_plugin_for_config(
        &self,
        config: &PluginsConfigInput,
        request: &PluginReadRequest,
    ) -> Result<PluginReadOutcome, MarketplaceError> {
        if !config.plugins_enabled {
            return Err(MarketplaceError::PluginsDisabled);
        }

        let plugin = find_marketplace_plugin(&request.marketplace_path, &request.plugin_name)?;
        if !self.restriction_product_matches(plugin.policy.products.as_deref()) {
            return Err(MarketplaceError::PluginNotFound {
                plugin_name: plugin.plugin_id.plugin_name,
                marketplace_name: plugin.plugin_id.marketplace_name,
            });
        }

        let marketplace_name = plugin.plugin_id.marketplace_name.clone();
        let plugin_key = plugin.plugin_id.as_key();
        let (installed_plugins, enabled_plugins) = self.configured_plugin_states(config);
        let plugin = self
            .read_plugin_detail_for_marketplace_plugin(
                config,
                &marketplace_name,
                ConfiguredMarketplacePlugin {
                    id: plugin_key.clone(),
                    name: plugin.plugin_id.plugin_name,
                    source: plugin.source,
                    policy: plugin.policy,
                    interface: plugin.interface,
                    installed: installed_plugins.contains(&plugin_key),
                    enabled: enabled_plugins.contains(&plugin_key),
                },
            )
            .await?;

        Ok(PluginReadOutcome {
            marketplace_name,
            marketplace_path: Some(request.marketplace_path.clone()),
            plugin,
        })
    }

    pub async fn read_plugin_detail_for_marketplace_plugin(
        &self,
        config: &PluginsConfigInput,
        marketplace_name: &str,
        plugin: ConfiguredMarketplacePlugin,
    ) -> Result<PluginDetail, MarketplaceError> {
        if !self.restriction_product_matches(plugin.policy.products.as_deref()) {
            return Err(MarketplaceError::PluginNotFound {
                plugin_name: plugin.name,
                marketplace_name: marketplace_name.to_string(),
            });
        }

        let plugin_id =
            PluginId::new(plugin.name.clone(), marketplace_name.to_string()).map_err(|err| {
                match err {
                    PluginIdError::Invalid(message) => MarketplaceError::InvalidPlugin(message),
                }
            })?;
        let plugin_key = plugin_id.as_key();
        if matches!(plugin.source, MarketplacePluginSource::Git { .. }) && !plugin.installed {
            let description = remote_plugin_install_required_description(&plugin.source);
            return Ok(PluginDetail {
                id: plugin_key,
                name: plugin.name,
                description: Some(description),
                source: plugin.source,
                policy: plugin.policy,
                interface: plugin.interface,
                installed: plugin.installed,
                enabled: plugin.enabled,
                skills: Vec::new(),
                disabled_skill_paths: HashSet::new(),
                apps: Vec::new(),
                mcp_server_names: Vec::new(),
                details_unavailable_reason: Some(
                    PluginDetailsUnavailableReason::InstallRequiredForRemoteSource,
                ),
            });
        }

        let source_path =
            if matches!(plugin.source, MarketplacePluginSource::Git { .. }) && plugin.installed {
                self.store.active_plugin_root(&plugin_id).ok_or_else(|| {
                    MarketplaceError::InvalidPlugin(format!(
                        "installed plugin cache entry is missing for {plugin_key}"
                    ))
                })?
            } else {
                let vac_home = self.vac_home.clone();
                let source = plugin.source.clone();
                let materialized = tokio::task::spawn_blocking(move || {
                    materialize_marketplace_plugin_source(vac_home.as_path(), &source)
                })
                .await
                .map_err(|err| {
                    MarketplaceError::InvalidPlugin(format!(
                        "failed to materialize plugin source: {err}"
                    ))
                })?
                .map_err(MarketplaceError::InvalidPlugin)?;
                materialized.path.clone()
            };
        if !source_path.as_path().is_dir() {
            return Err(MarketplaceError::InvalidPlugin(
                "path does not exist or is not a directory".to_string(),
            ));
        }
        let manifest = load_plugin_manifest(source_path.as_path()).ok_or_else(|| {
            MarketplaceError::InvalidPlugin("missing or invalid plugin.json".to_string())
        })?;
        let description = manifest.description.clone();
        let marketplace_category = plugin
            .interface
            .as_ref()
            .and_then(|interface| interface.category.clone());
        let interface = plugin_interface_with_marketplace_category(
            manifest.interface.clone(),
            marketplace_category,
        );
        let resolved_skills = load_plugin_skills(
            &source_path,
            &manifest.paths,
            self.restriction_product,
            &vac_core_skills::config_rules::skill_config_rules_from_stack(
                &config.config_layer_stack,
            ),
        )
        .await;
        let apps = load_plugin_apps(source_path.as_path()).await;
        let mut mcp_server_names = load_plugin_mcp_servers(source_path.as_path())
            .await
            .into_keys()
            .collect::<Vec<_>>();
        mcp_server_names.sort_unstable();
        mcp_server_names.dedup();

        Ok(PluginDetail {
            id: plugin.id,
            name: plugin.name,
            description,
            source: plugin.source,
            policy: plugin.policy,
            interface,
            installed: plugin.installed,
            enabled: plugin.enabled,
            skills: resolved_skills.skills,
            disabled_skill_paths: resolved_skills.disabled_skill_paths,
            apps,
            mcp_server_names,
            details_unavailable_reason: None,
        })
    }
}
