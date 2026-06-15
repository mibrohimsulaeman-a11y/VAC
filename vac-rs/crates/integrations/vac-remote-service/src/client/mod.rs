//! Unified AgentClient
//!
//! The AgentClient provides a unified interface that:
//! - Uses vac-provider-core for all LLM inference (with VacProvider when available)
//! - Uses VACApiClient for non-inference APIs (sessions, billing, etc.)
//! - Falls back to local SQLite DB when VAC is unavailable
//! - Integrates with hooks for lifecycle events

mod provider;

use crate::local::hooks::task_board_context::{TaskBoardContextHook, TaskBoardContextHookOptions};
use crate::local::storage::LocalStorage;
use crate::models::AgentState;
use crate::storage::SessionStorage;
use crate::vac::storage::VACStorage;
use crate::vac::{VACApiClient, VACApiConfig};

use std::sync::Arc;
use vac_foundation::hooks::{HookRegistry, LifecycleEvent};
use vac_foundation::models::llm::{LLMProviderConfig, ProviderConfig};
use vac_foundation::models::provider_core_adapter::VacProviderClient;

// =============================================================================
// AgentClient Configuration
// =============================================================================

/// Default VAC API endpoint
pub const DEFAULT_VAC_ENDPOINT: &str = "https://api.vastar.ai/vac";

/// VAC connection configuration
#[derive(Clone)]
pub struct VacCloudConfig {
    /// VAC API key
    pub api_key: String,
    /// VAC API endpoint (default: https://api.vastar.ai/vac)
    pub api_endpoint: String,
}

impl std::fmt::Debug for VacCloudConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VacCloudConfig")
            .field("api_key", &"[REDACTED]")
            .field("api_endpoint", &self.api_endpoint)
            .finish()
    }
}

impl VacCloudConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            api_endpoint: DEFAULT_VAC_ENDPOINT.to_string(),
        }
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.api_endpoint = endpoint.into();
        self
    }
}

/// Configuration for creating an AgentClient
#[derive(Default)]
pub struct AgentClientConfig {
    /// VAC configuration (optional - enables remote features when present)
    pub vac: Option<VacCloudConfig>,
    /// LLM provider configurations
    pub providers: LLMProviderConfig,
    /// Local database path (default: ~/.vac/data/local.db)
    pub store_path: Option<String>,
    /// Hook registry for lifecycle events
    pub hook_registry: Option<HookRegistry<AgentState>>,
}

impl std::fmt::Debug for AgentClientConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentClientConfig")
            .field("vac", &self.vac)
            .field("provider_count", &self.providers.providers.len())
            .field("store_path", &self.store_path)
            .field(
                "hook_registry",
                &self.hook_registry.as_ref().map(|_| "configured"),
            )
            .finish()
    }
}

impl AgentClientConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set VAC configuration
    ///
    /// Use `VacCloudConfig::new(api_key).with_endpoint(endpoint)` to configure.
    pub fn with_vac(mut self, config: VacCloudConfig) -> Self {
        self.vac = Some(config);
        self
    }

    /// Set providers
    pub fn with_providers(mut self, providers: LLMProviderConfig) -> Self {
        self.providers = providers;
        self
    }

    /// Set local database path
    pub fn with_store_path(mut self, path: impl Into<String>) -> Self {
        self.store_path = Some(path.into());
        self
    }

    /// Set hook registry
    pub fn with_hook_registry(mut self, registry: HookRegistry<AgentState>) -> Self {
        self.hook_registry = Some(registry);
        self
    }
}

// =============================================================================
// AgentClient
// =============================================================================

const DEFAULT_STORE_PATH: &str = ".vac/data/local.db";

/// Unified agent client
///
/// Provides a single interface for:
/// - LLM inference via vac-provider-core (with VAC or direct providers)
/// - Session/checkpoint management via SessionStorage trait (VAC API or local SQLite)
/// - MCP tools, billing, rulebooks (VAC API only)
#[derive(Clone)]
pub struct AgentClient {
    /// VAC Provider Core client for all LLM inference
    pub(crate) provider_core: VacProviderClient,
    /// VAC API client for non-inference operations (optional)
    pub(crate) vac_remote_service: Option<VACApiClient>,
    /// Session storage implementation (abstracts VAC API vs local SQLite)
    pub(crate) session_storage: Arc<dyn SessionStorage>,
    /// Hook registry for lifecycle events
    pub(crate) hook_registry: Arc<HookRegistry<AgentState>>,
    /// VAC configuration (for reference)
    pub(crate) vac: Option<VacCloudConfig>,
}

impl AgentClient {
    /// Build just the `SessionStorage` backend for a given config, without
    /// initializing LLM providers, hook registries, or the VACApiClient.
    ///
    /// Use this for read-only session inspection commands where initializing
    /// a full `AgentClient` would needlessly refresh OAuth tokens and instantiate
    /// providers the command never uses.
    pub async fn build_session_storage(
        vac: Option<VacCloudConfig>,
        store_path: Option<String>,
        profile_name: Option<String>,
    ) -> Result<Arc<dyn SessionStorage>, String> {
        if let Some(vac) = vac
            && !vac.api_key.is_empty()
        {
            let storage =
                VACStorage::new_with_profile(&vac.api_key, &vac.api_endpoint, profile_name)
                    .map_err(|e| format!("Failed to create VAC storage: {}", e))?;
            Ok(Arc::new(storage))
        } else {
            let store_path = store_path.unwrap_or_else(|| {
                std::env::var("HOME")
                    .map(|h| format!("{}/{}", h, DEFAULT_STORE_PATH))
                    .unwrap_or_else(|_| DEFAULT_STORE_PATH.to_string())
            });
            let storage = LocalStorage::new(&store_path)
                .await
                .map_err(|e| format!("Failed to create local storage: {}", e))?;
            Ok(Arc::new(storage))
        }
    }

    /// Create a new AgentClient
    pub async fn new(config: AgentClientConfig) -> Result<Self, String> {
        // 1. Build LLMProviderConfig with VAC if configured (only if api_key is not empty)
        let mut providers = config.providers.clone();
        if let Some(vac) = &config.vac
            && !vac.api_key.is_empty()
        {
            providers.providers.insert(
                "vac".to_string(),
                ProviderConfig::VAC {
                    api_key: Some(vac.api_key.clone()),
                    api_endpoint: Some(vac.api_endpoint.clone()),
                    auth: None,
                },
            );
        }

        // 2. Create VacProviderClient with all providers
        let provider_core = VacProviderClient::new(&providers)
            .map_err(|e| format!("Failed to create VAC Provider Core client: {}", e))?;

        // 3. Create VACApiClient if configured (only if api_key is not empty)
        let vac_remote_service = if let Some(vac) = &config.vac {
            if !vac.api_key.is_empty() {
                Some(
                    VACApiClient::new(&VACApiConfig {
                        api_key: vac.api_key.clone(),
                        api_endpoint: vac.api_endpoint.clone(),
                    })
                    .map_err(|e| format!("Failed to create VAC API client: {}", e))?,
                )
            } else {
                None
            }
        } else {
            None
        };

        // 4. Create session storage (VAC API or local SQLite)
        let session_storage: Arc<dyn SessionStorage> = if let Some(vac) = &config.vac
            && !vac.api_key.is_empty()
        {
            Arc::new(
                VACStorage::new(&vac.api_key, &vac.api_endpoint)
                    .map_err(|e| format!("Failed to create VAC storage: {}", e))?,
            )
        } else {
            let store_path = config.store_path.clone().unwrap_or_else(|| {
                std::env::var("HOME")
                    .map(|h| format!("{}/{}", h, DEFAULT_STORE_PATH))
                    .unwrap_or_else(|_| DEFAULT_STORE_PATH.to_string())
            });
            Arc::new(
                LocalStorage::new(&store_path)
                    .await
                    .map_err(|e| format!("Failed to create local storage: {}", e))?,
            )
        };

        // 6. Setup hook registry with context management hooks
        let mut hook_registry = config.hook_registry.unwrap_or_default();
        hook_registry.register(
            LifecycleEvent::BeforeInference,
            Box::new(TaskBoardContextHook::new(TaskBoardContextHookOptions {
                keep_last_n_assistant_messages: Some(5), // Keep the last 5 assistant messages in context
                context_budget_threshold: Some(0.8),     // defaults to 0.8 (80%)
            })),
        );
        let hook_registry = Arc::new(hook_registry);

        Ok(Self {
            provider_core,
            vac_remote_service,
            session_storage,
            hook_registry,
            vac: config.vac,
        })
    }

    /// Check if VAC API is available
    pub fn has_vac(&self) -> bool {
        self.vac_remote_service.is_some()
    }

    /// Get the VAC API endpoint (with default fallback)
    pub fn get_vac_remote_service_endpoint(&self) -> &str {
        self.vac
            .as_ref()
            .map(|s| s.api_endpoint.as_str())
            .unwrap_or(DEFAULT_VAC_ENDPOINT)
    }

    /// Get reference to the VAC Provider Core client
    pub fn provider_core(&self) -> &VacProviderClient {
        &self.provider_core
    }

    /// Get reference to the VAC API client (if available)
    pub fn vac_remote_service(&self) -> Option<&VACApiClient> {
        self.vac_remote_service.as_ref()
    }

    /// Get reference to the hook registry
    pub fn hook_registry(&self) -> &Arc<HookRegistry<AgentState>> {
        &self.hook_registry
    }

    /// Get reference to the session storage
    ///
    /// Use this for all session and checkpoint operations.
    pub fn session_storage(&self) -> &Arc<dyn SessionStorage> {
        &self.session_storage
    }
}

// Debug implementation for AgentClient
impl std::fmt::Debug for AgentClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentClient")
            .field("has_vac", &self.has_vac())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vac_foundation::models::llm::ProviderConfig;

    #[test]
    fn vac_cloud_config_debug_redacts_api_key() {
        let raw = "vac-cloud-debug-raw";
        let config = VacCloudConfig::new(raw).with_endpoint("https://example.invalid/vac");
        let rendered = format!("{config:?}");

        assert!(rendered.contains("[REDACTED]"));
        assert!(rendered.contains("https://example.invalid/vac"));
        assert!(!rendered.contains(raw));
    }

    #[test]
    fn agent_client_config_debug_does_not_render_provider_credentials() {
        let raw_vac = "vac-client-debug-raw";
        let raw_provider = "provider-debug-raw";
        let mut providers = LLMProviderConfig::new();
        providers.add_provider(
            "openai",
            ProviderConfig::OpenAI {
                api_key: Some(raw_provider.to_string()),
                api_endpoint: Some("https://example.invalid/v1".to_string()),
                auth: None,
            },
        );
        let config = AgentClientConfig::new()
            .with_vac(VacCloudConfig::new(raw_vac))
            .with_providers(providers)
            .with_store_path(".vac/data/local.db");
        let rendered = format!("{config:?}");

        assert!(rendered.contains("provider_count"));
        assert!(rendered.contains("[REDACTED]"));
        assert!(!rendered.contains(raw_vac));
        assert!(!rendered.contains(raw_provider));
    }
}
