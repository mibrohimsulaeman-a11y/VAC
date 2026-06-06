use super::*;

impl PluginsManager {

    pub(super) fn schedule_remote_installed_plugins_cache_refresh(
        self: &Arc<Self>,
        mut request: RemoteInstalledPluginsCacheRefreshRequest,
    ) {
        let should_spawn = {
            let mut state = match self.remote_installed_plugins_cache_refresh_state.write() {
                Ok(state) => state,
                Err(err) => err.into_inner(),
            };
            if let Some(existing_request) = state.requested.as_ref() {
                if matches!(
                    existing_request.notify,
                    RemoteInstalledPluginsCacheRefreshNotify::AfterSuccessfulRefresh
                ) {
                    request.notify =
                        RemoteInstalledPluginsCacheRefreshNotify::AfterSuccessfulRefresh;
                }
                if request.on_effective_plugins_changed.is_none() {
                    request.on_effective_plugins_changed =
                        existing_request.on_effective_plugins_changed.clone();
                }
            }
            state.requested = Some(request);
            if state.in_flight {
                false
            } else {
                state.in_flight = true;
                true
            }
        };
        if !should_spawn {
            return;
        }

        let manager = Arc::clone(self);
        tokio::spawn(async move {
            manager
                .run_remote_installed_plugins_cache_refresh_loop()
                .await;
        });
    }

    pub(super) fn schedule_non_curated_plugin_cache_refresh(
        self: &Arc<Self>,
        roots: &[AbsolutePathBuf],
        mode: NonCuratedCacheRefreshMode,
    ) {
        let mut roots = roots.to_vec();
        roots.sort_unstable();
        roots.dedup();
        if roots.is_empty() {
            return;
        }
        let request = NonCuratedCacheRefreshRequest { roots, mode };

        let should_spawn = {
            let mut state = match self.non_curated_cache_refresh_state.write() {
                Ok(state) => state,
                Err(err) => err.into_inner(),
            };
            // Collapse repeated plugin/list requests onto one worker and only queue another pass
            // when the requested roots set actually changes. Forced reinstall requests are not
            // deduped against the last completed pass because the same marketplace root path can
            // point at newly activated files after an auto-upgrade.
            if state.requested.as_ref() == Some(&request)
                || (mode == NonCuratedCacheRefreshMode::IfVersionChanged
                    && !state.in_flight
                    && state.last_refreshed.as_ref() == Some(&request))
            {
                return;
            }
            if mode == NonCuratedCacheRefreshMode::IfVersionChanged
                && state.requested.as_ref().is_some_and(|requested| {
                    requested.mode == NonCuratedCacheRefreshMode::ForceReinstall
                        && requested.roots == request.roots
                })
            {
                return;
            }
            state.requested = Some(request);
            if state.in_flight {
                false
            } else {
                state.in_flight = true;
                true
            }
        };
        if !should_spawn {
            return;
        }

        let manager = Arc::clone(self);
        if let Err(err) = std::thread::Builder::new()
            .name("plugins-non-curated-cache-refresh".to_string())
            .spawn(move || manager.run_non_curated_plugin_cache_refresh_loop())
        {
            let mut state = match self.non_curated_cache_refresh_state.write() {
                Ok(state) => state,
                Err(err) => err.into_inner(),
            };
            state.in_flight = false;
            state.requested = None;
            warn!("failed to start non-curated plugin cache refresh task: {err}");
        }
    }

    pub(super) fn start_curated_repo_sync(self: &Arc<Self>) {
        if CURATED_REPO_SYNC_STARTED.swap(true, Ordering::SeqCst) {
            return;
        }
        let manager = Arc::clone(self);
        let vac_home = self.vac_home.clone();
        if let Err(err) = std::thread::Builder::new()
            .name("plugins-curated-repo-sync".to_string())
            .spawn(move || match sync_vastar_plugins_repo(vac_home.as_path()) {
                Ok(curated_plugin_version) => {
                    let configured_curated_plugin_ids =
                        configured_curated_plugin_ids_from_vac_home(vac_home.as_path());
                    match refresh_curated_plugin_cache(
                        vac_home.as_path(),
                        &curated_plugin_version,
                        &configured_curated_plugin_ids,
                    ) {
                        Ok(cache_refreshed) => {
                            if cache_refreshed {
                                manager.clear_cache();
                            }
                        }
                        Err(err) => {
                            manager.clear_cache();
                            CURATED_REPO_SYNC_STARTED.store(false, Ordering::SeqCst);
                            warn!("failed to refresh curated plugin cache after sync: {err}");
                        }
                    }
                }
                Err(err) => {
                    CURATED_REPO_SYNC_STARTED.store(false, Ordering::SeqCst);
                    warn!("failed to sync curated plugins repo: {err}");
                }
            })
        {
            CURATED_REPO_SYNC_STARTED.store(false, Ordering::SeqCst);
            warn!("failed to start curated plugins repo sync task: {err}");
        }
    }

    pub(super) async fn run_remote_installed_plugins_cache_refresh_loop(self: Arc<Self>) {
        loop {
            let request = {
                let mut state = match self.remote_installed_plugins_cache_refresh_state.write() {
                    Ok(state) => state,
                    Err(err) => err.into_inner(),
                };
                match state.requested.take() {
                    Some(request) => request,
                    None => {
                        state.in_flight = false;
                        return;
                    }
                }
            };

            let installed_plugins = crate::remote::fetch_remote_installed_plugins(
                &request.service_config,
                request.auth.as_ref(),
            )
            .await;
            match installed_plugins {
                Ok(installed_plugins) => {
                    // VAC-DEBT(owner=vac-o5o6-zero-residual,target=post-cargo-hardening)(remote plugins): reconcile missing or stale local bundles before
                    // publishing remote installed state as effective local plugin config.
                    let changed = self.write_remote_installed_plugins_cache(installed_plugins);
                    let should_notify = changed
                        || matches!(
                            request.notify,
                            RemoteInstalledPluginsCacheRefreshNotify::AfterSuccessfulRefresh
                        );
                    if should_notify
                        && let Some(on_effective_plugins_changed) =
                            request.on_effective_plugins_changed
                    {
                        on_effective_plugins_changed();
                    }
                }
                Err(
                    RemotePluginCatalogError::AuthRequired
                    | RemotePluginCatalogError::UnsupportedAuthMode,
                ) => {
                    let changed = self.clear_remote_installed_plugins_cache();
                    if changed
                        && let Some(on_effective_plugins_changed) =
                            request.on_effective_plugins_changed
                    {
                        on_effective_plugins_changed();
                    }
                }
                Err(err) => {
                    warn!(
                        error = %err,
                        "failed to refresh remote installed plugins cache"
                    );
                }
            }
        }
    }

    pub(super) fn run_non_curated_plugin_cache_refresh_loop(self: Arc<Self>) {
        loop {
            let request = {
                let state = match self.non_curated_cache_refresh_state.read() {
                    Ok(state) => state,
                    Err(err) => err.into_inner(),
                };
                state.requested.clone()
            };

            let Some(request) = request else {
                let mut state = match self.non_curated_cache_refresh_state.write() {
                    Ok(state) => state,
                    Err(err) => err.into_inner(),
                };
                state.in_flight = false;
                return;
            };

            let refresh_result = match request.mode {
                NonCuratedCacheRefreshMode::IfVersionChanged => {
                    refresh_non_curated_plugin_cache(self.vac_home.as_path(), &request.roots)
                }
                NonCuratedCacheRefreshMode::ForceReinstall => {
                    refresh_non_curated_plugin_cache_force_reinstall(
                        self.vac_home.as_path(),
                        &request.roots,
                    )
                }
            };
            let refreshed = match refresh_result {
                Ok(cache_refreshed) => {
                    if cache_refreshed {
                        self.clear_cache();
                    }
                    true
                }
                Err(err) => {
                    self.clear_cache();
                    warn!("failed to refresh non-curated plugin cache: {err}");
                    false
                }
            };

            let mut state = match self.non_curated_cache_refresh_state.write() {
                Ok(state) => state,
                Err(err) => err.into_inner(),
            };
            if refreshed {
                state.last_refreshed = Some(request.clone());
            }
            if state.requested.as_ref() == Some(&request) {
                state.requested = None;
                state.in_flight = false;
                return;
            }
        }
    }

    pub(super) fn configured_plugin_states(
        &self,
        config: &PluginsConfigInput,
    ) -> (HashSet<String>, HashSet<String>) {
        let configured_plugins = configured_plugins_from_stack(&config.config_layer_stack);
        let installed_plugins = configured_plugins
            .keys()
            .filter(|plugin_key| {
                PluginId::parse(plugin_key)
                    .ok()
                    .is_some_and(|plugin_id| self.store.is_installed(&plugin_id))
            })
            .cloned()
            .collect::<HashSet<_>>();
        let enabled_plugins = configured_plugins
            .into_iter()
            .filter_map(|(plugin_key, plugin)| plugin.enabled.then_some(plugin_key))
            .collect::<HashSet<_>>();
        (installed_plugins, enabled_plugins)
    }

    pub(super) fn marketplace_roots(
        &self,
        config: &PluginsConfigInput,
        additional_roots: &[AbsolutePathBuf],
    ) -> Vec<AbsolutePathBuf> {
        // Treat the curated catalog as an extra marketplace root so plugin listing can surface it
        // without requiring every caller to know where it is stored.
        let mut roots = additional_roots.to_vec();
        roots.extend(installed_marketplace_roots_from_layer_stack(
            &config.config_layer_stack,
            self.vac_home.as_path(),
        ));
        let curated_repo_root = curated_plugins_repo_path(self.vac_home.as_path());
        if curated_repo_root.is_dir()
            && let Ok(curated_repo_root) = AbsolutePathBuf::try_from(curated_repo_root)
        {
            roots.push(curated_repo_root);
        }
        roots.sort_unstable();
        roots.dedup();
        roots
    }
}
