use super::*;

impl PluginsManager {

    pub async fn sync_plugins_from_remote(
        &self,
        config: &PluginsConfigInput,
        auth: Option<&VACAuth>,
        additive_only: bool,
    ) -> Result<RemotePluginSyncResult, PluginRemoteSyncError> {
        let _remote_sync_guard = self.remote_sync_lock.acquire().await.map_err(|_| {
            PluginRemoteSyncError::Config(anyhow::anyhow!("remote plugin sync semaphore closed"))
        })?;

        if !config.plugins_enabled {
            return Ok(RemotePluginSyncResult::default());
        }

        info!("starting remote plugin sync");
        let remote_plugins = crate::remote_legacy::fetch_remote_plugin_status(
            &remote_plugin_service_config(config),
            auth,
        )
        .await
        .map_err(PluginRemoteSyncError::from)?;
        let configured_plugins = configured_plugins_from_stack(&config.config_layer_stack);
        let curated_marketplace_root = curated_plugins_repo_path(self.vac_home.as_path());
        let curated_marketplace_path = AbsolutePathBuf::try_from(
            curated_marketplace_root.join(".agents/plugins/marketplace.json"),
        )
        .map_err(|_| PluginRemoteSyncError::LocalMarketplaceNotFound)?;
        let curated_marketplace = match load_marketplace(&curated_marketplace_path) {
            Ok(marketplace) => marketplace,
            Err(MarketplaceError::MarketplaceNotFound { .. }) => {
                return Err(PluginRemoteSyncError::LocalMarketplaceNotFound);
            }
            Err(err) => return Err(err.into()),
        };

        let marketplace_name = curated_marketplace.name.clone();
        let curated_plugin_version =
            read_curated_plugins_sha(self.vac_home.as_path()).ok_or_else(|| {
                PluginStoreError::Invalid(
                    "local curated marketplace sha is not available".to_string(),
                )
            })?;
        let cache_plugin_version = curated_plugin_cache_version(&curated_plugin_version);
        let mut local_plugins = Vec::<(
            String,
            PluginId,
            AbsolutePathBuf,
            Option<bool>,
            Option<String>,
            bool,
        )>::new();
        let mut local_plugin_names = HashSet::new();
        for plugin in curated_marketplace.plugins {
            let plugin_name = plugin.name;
            if !local_plugin_names.insert(plugin_name.clone()) {
                warn!(
                    plugin = plugin_name,
                    marketplace = %marketplace_name,
                    "ignoring duplicate local plugin entry during remote sync"
                );
                continue;
            }

            let plugin_id = PluginId::new(plugin_name.clone(), marketplace_name.clone())?;
            let plugin_key = plugin_id.as_key();
            let source_path = match plugin.source {
                MarketplacePluginSource::Local { path } => path,
                MarketplacePluginSource::Git { .. } => {
                    warn!(
                        plugin = plugin_name,
                        marketplace = %marketplace_name,
                        "skipping remote plugin source during remote sync"
                    );
                    continue;
                }
            };
            let current_enabled = configured_plugins
                .get(&plugin_key)
                .map(|plugin| plugin.enabled);
            let installed_version = self.store.active_plugin_version(&plugin_id);
            let product_allowed =
                self.restriction_product_matches(plugin.policy.products.as_deref());
            local_plugins.push((
                plugin_name,
                plugin_id,
                source_path,
                current_enabled,
                installed_version,
                product_allowed,
            ));
        }

        let mut missing_remote_plugins = Vec::<String>::new();
        let mut remote_installed_plugin_names = HashSet::<String>::new();
        for plugin in remote_plugins {
            if plugin.marketplace_name != marketplace_name {
                return Err(PluginRemoteSyncError::UnknownRemoteMarketplace {
                    marketplace_name: plugin.marketplace_name,
                });
            }
            if !local_plugin_names.contains(&plugin.name) {
                missing_remote_plugins.push(plugin.name);
                continue;
            }
            // For now, sync treats remote `enabled = false` as uninstall rather than a distinct
            // disabled state.
            // VAC-DEBT(owner=vac-o5o6-zero-residual,target=post-cargo-hardening): Switch sync to `plugins/installed` so install and enable states stay distinct.
            if !plugin.enabled {
                continue;
            }
            if !remote_installed_plugin_names.insert(plugin.name.clone()) {
                return Err(PluginRemoteSyncError::DuplicateRemotePlugin {
                    plugin_name: plugin.name,
                });
            }
        }

        let mut config_edits = Vec::new();
        let mut installs = Vec::new();
        let mut uninstalls = Vec::new();
        let mut result = RemotePluginSyncResult::default();
        let remote_plugin_count = remote_installed_plugin_names.len();
        let local_plugin_count = local_plugins.len();
        if !missing_remote_plugins.is_empty() {
            let sample_missing_plugins = missing_remote_plugins
                .iter()
                .take(10)
                .cloned()
                .collect::<Vec<_>>();
            warn!(
                marketplace = %marketplace_name,
                missing_remote_plugin_count = missing_remote_plugins.len(),
                missing_remote_plugin_examples = ?sample_missing_plugins,
                "ignoring remote plugins missing from local marketplace during sync"
            );
        }

        for (
            plugin_name,
            plugin_id,
            source_path,
            current_enabled,
            installed_version,
            product_allowed,
        ) in local_plugins
        {
            let plugin_key = plugin_id.as_key();
            let is_installed = installed_version.is_some();
            if !product_allowed {
                continue;
            }
            if remote_installed_plugin_names.contains(&plugin_name) {
                if !is_installed {
                    installs.push((source_path, plugin_id.clone(), cache_plugin_version.clone()));
                }
                if !is_installed {
                    result.installed_plugin_ids.push(plugin_key.clone());
                }

                if current_enabled != Some(true) {
                    result.enabled_plugin_ids.push(plugin_key.clone());
                    config_edits.push(PluginConfigEdit::SetEnabled {
                        plugin_key,
                        enabled: true,
                    });
                }
            } else if !additive_only {
                if is_installed {
                    uninstalls.push(plugin_id);
                }
                if is_installed || current_enabled.is_some() {
                    result.uninstalled_plugin_ids.push(plugin_key.clone());
                }
                if current_enabled.is_some() {
                    config_edits.push(PluginConfigEdit::Clear { plugin_key });
                }
            }
        }

        let store = self.store.clone();
        let store_result = tokio::task::spawn_blocking(move || {
            for (source_path, plugin_id, plugin_version) in installs {
                store.install_with_version(source_path, plugin_id, plugin_version)?;
            }
            for plugin_id in uninstalls {
                store.uninstall(&plugin_id)?;
            }
            Ok::<(), PluginStoreError>(())
        })
        .await
        .map_err(PluginRemoteSyncError::join)?;
        if let Err(err) = store_result {
            self.clear_cache();
            return Err(err.into());
        }

        let config_result = if config_edits.is_empty() {
            Ok(())
        } else {
            apply_user_plugin_config_edits(&self.vac_home, config_edits).await
        };
        self.clear_cache();
        config_result.map_err(anyhow::Error::from)?;

        info!(
            marketplace = %marketplace_name,
            remote_plugin_count,
            local_plugin_count,
            installed_plugin_ids = ?result.installed_plugin_ids,
            enabled_plugin_ids = ?result.enabled_plugin_ids,
            disabled_plugin_ids = ?result.disabled_plugin_ids,
            uninstalled_plugin_ids = ?result.uninstalled_plugin_ids,
            "completed remote plugin sync"
        );

        Ok(result)
    }
}
