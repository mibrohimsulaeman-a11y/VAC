use super::*;

impl PluginsManager {

    pub(super) fn remote_installed_plugin_configs(
        &self,
        config: &PluginsConfigInput,
    ) -> HashMap<String, PluginConfig> {
        if !config.remote_plugin_enabled {
            return HashMap::new();
        }

        let cache = match self.remote_installed_plugins_cache.read() {
            Ok(cache) => cache,
            Err(err) => err.into_inner(),
        };
        let Some(plugins) = cache.as_ref() else {
            return HashMap::new();
        };

        remote_installed_plugins_to_config(plugins, &self.store)
    }

    pub(super) fn write_remote_installed_plugins_cache(&self, plugins: Vec<RemoteInstalledPlugin>) -> bool {
        let mut cache = match self.remote_installed_plugins_cache.write() {
            Ok(cache) => cache,
            Err(err) => err.into_inner(),
        };
        if cache.as_ref().is_some_and(|cache| cache.eq(&plugins)) {
            return false;
        }
        *cache = Some(plugins);
        drop(cache);
        self.clear_enabled_outcome_cache();
        true
    }

    pub fn clear_remote_installed_plugins_cache(&self) -> bool {
        let mut cache = match self.remote_installed_plugins_cache.write() {
            Ok(cache) => cache,
            Err(err) => err.into_inner(),
        };
        if cache.is_none() {
            return false;
        }
        *cache = None;
        drop(cache);
        self.clear_enabled_outcome_cache();
        true
    }

    pub fn maybe_start_remote_installed_plugins_cache_refresh(
        self: &Arc<Self>,
        config: &PluginsConfigInput,
        auth: Option<VACAuth>,
        on_effective_plugins_changed: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    ) {
        self.maybe_start_remote_installed_plugins_cache_refresh_with_notify(
            config,
            auth,
            RemoteInstalledPluginsCacheRefreshNotify::IfCacheChanged,
            on_effective_plugins_changed,
        );
    }

    pub fn maybe_start_remote_installed_plugins_cache_refresh_after_mutation(
        self: &Arc<Self>,
        config: &PluginsConfigInput,
        auth: Option<VACAuth>,
        on_effective_plugins_changed: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    ) {
        self.maybe_start_remote_installed_plugins_cache_refresh_with_notify(
            config,
            auth,
            RemoteInstalledPluginsCacheRefreshNotify::AfterSuccessfulRefresh,
            on_effective_plugins_changed,
        );
    }

    pub(super) fn maybe_start_remote_installed_plugins_cache_refresh_with_notify(
        self: &Arc<Self>,
        config: &PluginsConfigInput,
        auth: Option<VACAuth>,
        notify: RemoteInstalledPluginsCacheRefreshNotify,
        on_effective_plugins_changed: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    ) {
        if !config.plugins_enabled || !config.remote_plugin_enabled {
            return;
        }

        self.schedule_remote_installed_plugins_cache_refresh(
            RemoteInstalledPluginsCacheRefreshRequest {
                service_config: remote_plugin_service_config(config),
                auth,
                notify,
                on_effective_plugins_changed,
            },
        );
    }

    pub(super) fn maybe_start_remote_installed_plugin_bundle_sync(
        self: &Arc<Self>,
        config: &PluginsConfigInput,
        auth: Option<VACAuth>,
        on_effective_plugins_changed: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    ) {
        if !config.plugins_enabled || !config.remote_plugin_enabled {
            return;
        }

        let manager = Arc::clone(self);
        let config_for_refresh = config.clone();
        let auth_for_refresh = auth.clone();
        let on_local_cache_changed = Arc::new(move || {
            manager.maybe_start_remote_installed_plugins_cache_refresh_after_mutation(
                &config_for_refresh,
                auth_for_refresh.clone(),
                on_effective_plugins_changed.clone(),
            );
        });

        crate::remote::maybe_start_remote_installed_plugin_bundle_sync(
            self.vac_home.clone(),
            remote_plugin_service_config(config),
            auth,
            Some(on_local_cache_changed),
        );
    }
}
