use super::*;

impl PluginsManager {
    pub fn new(vac_home: PathBuf) -> Self {
        Self::new_with_restriction_product(vac_home, Some(Product::VAC))
    }

    pub fn new_with_restriction_product(
        vac_home: PathBuf,
        restriction_product: Option<Product>,
    ) -> Self {
        // Product restrictions are enforced at marketplace admission time for a given VAC_HOME:
        // listing, install, and curated refresh all consult this restriction context before new
        // plugins enter local config or cache. After admission, runtime plugin loading trusts the
        // contents of that VAC_HOME and does not re-filter configured plugins by product, so
        // already-admitted plugins may continue exposing MCP servers/tools from shared local state.
        //
        // This assumes a single VAC_HOME is only used by one product.
        Self {
            vac_home: vac_home.clone(),
            store: PluginStore::new(vac_home),
            featured_plugin_ids_cache: RwLock::new(None),
            configured_marketplace_upgrade_state: RwLock::new(
                ConfiguredMarketplaceUpgradeState::default(),
            ),
            non_curated_cache_refresh_state: RwLock::new(NonCuratedCacheRefreshState::default()),
            cached_enabled_outcome: RwLock::new(None),
            remote_installed_plugins_cache: RwLock::new(None),
            remote_installed_plugins_cache_refresh_state: RwLock::new(
                RemoteInstalledPluginsCacheRefreshState::default(),
            ),
            remote_sync_lock: Semaphore::new(/*permits*/ 1),
            restriction_product,
            analytics_events_client: RwLock::new(None),
        }
    }

    pub fn set_analytics_events_client(&self, analytics_events_client: AnalyticsEventsClient) {
        let mut stored_client = match self.analytics_events_client.write() {
            Ok(client_guard) => client_guard,
            Err(err) => err.into_inner(),
        };
        *stored_client = Some(analytics_events_client);
    }

    pub(super) fn restriction_product_matches(&self, products: Option<&[Product]>) -> bool {
        match products {
            None => true,
            Some([]) => false,
            Some(products) => self
                .restriction_product
                .is_some_and(|product| product.matches_product_restriction(products)),
        }
    }

    pub async fn plugins_for_config(&self, config: &PluginsConfigInput) -> PluginLoadOutcome {
        self.plugins_for_config_with_force_reload(config, /*force_reload*/ false)
            .await
    }

    pub(crate) async fn plugins_for_config_with_force_reload(
        &self,
        config: &PluginsConfigInput,
        force_reload: bool,
    ) -> PluginLoadOutcome {
        if !config.plugins_enabled {
            return PluginLoadOutcome::default();
        }

        let plugin_hooks_enabled = config.plugin_hooks_enabled;
        let config_version = version_for_toml(&config.config_layer_stack.effective_config());
        if !force_reload
            && let Some(outcome) =
                self.cached_enabled_outcome(&config_version, plugin_hooks_enabled)
        {
            return outcome;
        }

        let outcome = load_plugins_from_layer_stack(
            &config.config_layer_stack,
            self.remote_installed_plugin_configs(config),
            &self.store,
            self.restriction_product,
            plugin_hooks_enabled,
        )
        .await;
        log_plugin_load_errors(&outcome);
        let mut cache = match self.cached_enabled_outcome.write() {
            Ok(cache) => cache,
            Err(err) => err.into_inner(),
        };
        *cache = Some(CachedPluginLoadOutcome {
            config_version,
            plugin_hooks_enabled,
            outcome: outcome.clone(),
        });
        outcome
    }

    pub fn clear_cache(&self) {
        self.clear_enabled_outcome_cache();
        let mut featured_plugin_ids_cache = match self.featured_plugin_ids_cache.write() {
            Ok(cache) => cache,
            Err(err) => err.into_inner(),
        };
        *featured_plugin_ids_cache = None;
    }

    pub(super) fn clear_enabled_outcome_cache(&self) {
        let mut cached_enabled_outcome = match self.cached_enabled_outcome.write() {
            Ok(cache) => cache,
            Err(err) => err.into_inner(),
        };
        *cached_enabled_outcome = None;
    }

    /// Load plugins for a config layer stack without touching the plugins cache.
    pub async fn plugins_for_layer_stack(
        &self,
        config_layer_stack: &ConfigLayerStack,
        config: &PluginsConfigInput,
        plugin_hooks_feature_enabled: bool,
    ) -> PluginLoadOutcome {
        if !config.plugins_enabled {
            return PluginLoadOutcome::default();
        }
        load_plugins_from_layer_stack(
            config_layer_stack,
            self.remote_installed_plugin_configs(config),
            &self.store,
            self.restriction_product,
            plugin_hooks_feature_enabled,
        )
        .await
    }

    /// Resolve plugin skill roots for a config layer stack without touching the plugins cache.
    pub async fn effective_skill_roots_for_layer_stack(
        &self,
        config_layer_stack: &ConfigLayerStack,
        config: &PluginsConfigInput,
    ) -> Vec<AbsolutePathBuf> {
        self.plugins_for_layer_stack(config_layer_stack, config, config.plugin_hooks_enabled)
            .await
            .effective_skill_roots()
    }

    pub(super) fn cached_enabled_outcome(
        &self,
        config_version: &str,
        plugin_hooks_enabled: bool,
    ) -> Option<PluginLoadOutcome> {
        match self.cached_enabled_outcome.read() {
            Ok(cache) => cache
                .as_ref()
                .filter(|cached| {
                    cached.config_version == config_version
                        && cached.plugin_hooks_enabled == plugin_hooks_enabled
                })
                .map(|cached| cached.outcome.clone()),
            Err(err) => err
                .into_inner()
                .as_ref()
                .filter(|cached| {
                    cached.config_version == config_version
                        && cached.plugin_hooks_enabled == plugin_hooks_enabled
                })
                .map(|cached| cached.outcome.clone()),
        }
    }
}
