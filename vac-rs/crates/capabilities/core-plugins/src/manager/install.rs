use super::*;

impl PluginsManager {

    pub async fn install_plugin(
        &self,
        request: PluginInstallRequest,
    ) -> Result<PluginInstallOutcome, PluginInstallError> {
        let resolved = find_installable_marketplace_plugin(
            &request.marketplace_path,
            &request.plugin_name,
            self.restriction_product,
        )?;
        self.install_resolved_plugin(resolved).await
    }

    pub async fn install_plugin_with_remote_sync(
        &self,
        config: &PluginsConfigInput,
        auth: Option<&VACAuth>,
        request: PluginInstallRequest,
    ) -> Result<PluginInstallOutcome, PluginInstallError> {
        let resolved = find_installable_marketplace_plugin(
            &request.marketplace_path,
            &request.plugin_name,
            self.restriction_product,
        )?;
        let plugin_id = resolved.plugin_id.as_key();
        // This only forwards the backend mutation before the local install flow.
        crate::remote_legacy::enable_remote_plugin(
            &remote_plugin_service_config(config),
            auth,
            &plugin_id,
        )
        .await
        .map_err(PluginInstallError::from)?;
        self.install_resolved_plugin(resolved).await
    }

    pub(super) async fn install_resolved_plugin(
        &self,
        resolved: ResolvedMarketplacePlugin,
    ) -> Result<PluginInstallOutcome, PluginInstallError> {
        let auth_policy = resolved.policy.authentication;
        let plugin_version =
            if resolved.plugin_id.marketplace_name == VASTAR_CURATED_MARKETPLACE_NAME {
                let curated_plugin_version = read_curated_plugins_sha(self.vac_home.as_path())
                    .ok_or_else(|| {
                        PluginStoreError::Invalid(
                            "local curated marketplace sha is not available".to_string(),
                        )
                    })?;
                Some(curated_plugin_cache_version(&curated_plugin_version))
            } else {
                None
            };
        let store = self.store.clone();
        let vac_home = self.vac_home.clone();
        let result: StorePluginInstallResult = tokio::task::spawn_blocking(move || {
            let materialized =
                materialize_marketplace_plugin_source(vac_home.as_path(), &resolved.source)
                    .map_err(PluginStoreError::Invalid)?;
            let source_path = materialized.path;
            if let Some(plugin_version) = plugin_version {
                store.install_with_version(source_path, resolved.plugin_id, plugin_version)
            } else {
                store.install(source_path, resolved.plugin_id)
            }
        })
        .await
        .map_err(PluginInstallError::join)??;

        set_user_plugin_enabled(
            &self.vac_home,
            result.plugin_id.as_key(),
            /*enabled*/ true,
        )
        .await
        .map_err(anyhow::Error::from)?;

        let analytics_events_client = match self.analytics_events_client.read() {
            Ok(client) => client.clone(),
            Err(err) => err.into_inner().clone(),
        };
        if let Some(analytics_events_client) = analytics_events_client {
            analytics_events_client.track_plugin_installed(
                plugin_telemetry_metadata_from_root(&result.plugin_id, &result.installed_path)
                    .await,
            );
        }

        Ok(PluginInstallOutcome {
            plugin_id: result.plugin_id,
            plugin_version: result.plugin_version,
            installed_path: result.installed_path,
            auth_policy,
        })
    }

    pub async fn uninstall_plugin(&self, plugin_id: String) -> Result<(), PluginUninstallError> {
        let plugin_id = PluginId::parse(&plugin_id)?;
        self.uninstall_plugin_id(plugin_id).await
    }

    pub async fn uninstall_plugin_with_remote_sync(
        &self,
        config: &PluginsConfigInput,
        auth: Option<&VACAuth>,
        plugin_id: String,
    ) -> Result<(), PluginUninstallError> {
        // VAC-DEBT(owner=vac-o5o6-zero-residual,target=post-cargo-hardening): Remove this legacy remote-sync path once remote plugins have
        // their own manager and installed-state API.
        let plugin_id = PluginId::parse(&plugin_id)?;
        let plugin_key = plugin_id.as_key();
        // This only forwards the backend mutation before the local uninstall flow.
        crate::remote_legacy::uninstall_remote_plugin(
            &remote_plugin_service_config(config),
            auth,
            &plugin_key,
        )
        .await
        .map_err(PluginUninstallError::from)?;
        self.uninstall_plugin_id(plugin_id).await
    }

    pub(super) async fn uninstall_plugin_id(&self, plugin_id: PluginId) -> Result<(), PluginUninstallError> {
        let plugin_telemetry = if self.store.active_plugin_root(&plugin_id).is_some() {
            Some(installed_plugin_telemetry_metadata(self.vac_home.as_path(), &plugin_id).await)
        } else {
            None
        };
        let store = self.store.clone();
        let plugin_id_for_store = plugin_id.clone();
        tokio::task::spawn_blocking(move || store.uninstall(&plugin_id_for_store))
            .await
            .map_err(PluginUninstallError::join)??;

        clear_user_plugin(&self.vac_home, plugin_id.as_key())
            .await
            .map_err(anyhow::Error::from)?;

        let analytics_events_client = match self.analytics_events_client.read() {
            Ok(client) => client.clone(),
            Err(err) => err.into_inner().clone(),
        };
        if let Some(plugin_telemetry) = plugin_telemetry
            && let Some(analytics_events_client) = analytics_events_client
        {
            analytics_events_client.track_plugin_uninstalled(plugin_telemetry);
        }

        Ok(())
    }
}
