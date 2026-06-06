use super::*;

impl PluginsManager {

    pub fn maybe_start_plugin_list_background_tasks_for_config(
        self: &Arc<Self>,
        config: &PluginsConfigInput,
        auth: Option<VACAuth>,
        roots: &[AbsolutePathBuf],
        on_effective_plugins_changed: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    ) {
        self.maybe_start_non_curated_plugin_cache_refresh(roots);
        self.maybe_start_remote_installed_plugins_cache_refresh(
            config,
            auth.clone(),
            on_effective_plugins_changed.clone(),
        );
        self.maybe_start_remote_installed_plugin_bundle_sync(
            config,
            auth,
            on_effective_plugins_changed,
        );
    }

    pub(super) fn cached_featured_plugin_ids(
        &self,
        cache_key: &FeaturedPluginIdsCacheKey,
    ) -> Option<Vec<String>> {
        {
            let cache = match self.featured_plugin_ids_cache.read() {
                Ok(cache) => cache,
                Err(err) => err.into_inner(),
            };
            let now = Instant::now();
            if let Some(cached) = cache.as_ref()
                && now < cached.expires_at
                && cached.key == *cache_key
            {
                return Some(cached.featured_plugin_ids.clone());
            }
        }

        let mut cache = match self.featured_plugin_ids_cache.write() {
            Ok(cache) => cache,
            Err(err) => err.into_inner(),
        };
        let now = Instant::now();
        if cache
            .as_ref()
            .is_some_and(|cached| now >= cached.expires_at || cached.key != *cache_key)
        {
            *cache = None;
        }
        None
    }

    pub(super) fn write_featured_plugin_ids_cache(
        &self,
        cache_key: FeaturedPluginIdsCacheKey,
        featured_plugin_ids: &[String],
    ) {
        let mut cache = match self.featured_plugin_ids_cache.write() {
            Ok(cache) => cache,
            Err(err) => err.into_inner(),
        };
        *cache = Some(CachedFeaturedPluginIds {
            key: cache_key,
            expires_at: Instant::now() + FEATURED_PLUGIN_IDS_CACHE_TTL,
            featured_plugin_ids: featured_plugin_ids.to_vec(),
        });
    }

    pub async fn featured_plugin_ids_for_config(
        &self,
        config: &PluginsConfigInput,
        auth: Option<&VACAuth>,
    ) -> Result<Vec<String>, RemotePluginFetchError> {
        if !config.plugins_enabled {
            return Ok(Vec::new());
        }

        let cache_key = featured_plugin_ids_cache_key(config, auth);
        if let Some(featured_plugin_ids) = self.cached_featured_plugin_ids(&cache_key) {
            return Ok(featured_plugin_ids);
        }
        let featured_plugin_ids = crate::remote_legacy::fetch_remote_featured_plugin_ids(
            &remote_plugin_service_config(config),
            auth,
            self.restriction_product,
        )
        .await?;
        self.write_featured_plugin_ids_cache(cache_key, &featured_plugin_ids);
        Ok(featured_plugin_ids)
    }
}
