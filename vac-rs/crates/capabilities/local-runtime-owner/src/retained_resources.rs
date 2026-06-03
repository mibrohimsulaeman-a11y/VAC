//! Retained-resource ownership for the local runtime owner.
//!
//! This module deliberately owns the manager set directly through lower-level
//! runtime crates. It must not depend on app-server crates or copy
//! `MessageProcessor` internals.

use std::sync::Arc;

use vac_analytics::AnalyticsEventsClient;
use vac_core::ThreadManager;
use vac_core::config::Config;
use vac_exec_server::EnvironmentManager;
use vac_login::AuthManager;
use vac_thread_store::ThreadStore;

#[derive(Clone)]
pub struct RuntimeRetainedResources {
    config: Arc<Config>,
    auth_manager: Arc<AuthManager>,
    thread_manager: Arc<ThreadManager>,
    thread_store: Arc<dyn ThreadStore>,
    environment_manager: Arc<EnvironmentManager>,
    analytics_events_client: AnalyticsEventsClient,
}

impl RuntimeRetainedResources {
    #[must_use]
    pub fn new(
        config: Arc<Config>,
        auth_manager: Arc<AuthManager>,
        thread_manager: Arc<ThreadManager>,
        thread_store: Arc<dyn ThreadStore>,
        environment_manager: Arc<EnvironmentManager>,
        analytics_events_client: AnalyticsEventsClient,
    ) -> Self {
        Self {
            config,
            auth_manager,
            thread_manager,
            thread_store,
            environment_manager,
            analytics_events_client,
        }
    }

    #[must_use]
    pub fn config(&self) -> Arc<Config> {
        Arc::clone(&self.config)
    }

    #[must_use]
    pub fn auth_manager(&self) -> Arc<AuthManager> {
        Arc::clone(&self.auth_manager)
    }

    #[must_use]
    pub fn thread_manager(&self) -> Arc<ThreadManager> {
        Arc::clone(&self.thread_manager)
    }

    #[must_use]
    pub fn thread_store(&self) -> Arc<dyn ThreadStore> {
        Arc::clone(&self.thread_store)
    }

    #[must_use]
    pub fn environment_manager(&self) -> Arc<EnvironmentManager> {
        Arc::clone(&self.environment_manager)
    }

    #[must_use]
    pub fn analytics_events_client(&self) -> AnalyticsEventsClient {
        self.analytics_events_client.clone()
    }
}
