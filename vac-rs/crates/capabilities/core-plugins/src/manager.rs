use super::PluginLoadOutcome;
use super::startup_remote_sync::start_startup_remote_plugin_sync_once;
use crate::VASTAR_CURATED_MARKETPLACE_NAME;
use crate::installed_marketplaces::installed_marketplace_roots_from_layer_stack;
use crate::loader::configured_curated_plugin_ids_from_vac_home;
use crate::loader::curated_plugin_cache_version;
use crate::loader::installed_plugin_telemetry_metadata;
use crate::loader::load_plugin_apps;
use crate::loader::load_plugin_mcp_servers;
use crate::loader::load_plugin_skills;
use crate::loader::load_plugins_from_layer_stack;
use crate::loader::log_plugin_load_errors;
use crate::loader::materialize_marketplace_plugin_source;
use crate::loader::plugin_telemetry_metadata_from_root;
use crate::loader::refresh_curated_plugin_cache;
use crate::loader::refresh_non_curated_plugin_cache;
use crate::loader::refresh_non_curated_plugin_cache_force_reinstall;
use crate::loader::remote_installed_plugins_to_config;
use crate::manifest::PluginManifestInterface;
use crate::manifest::load_plugin_manifest;
use crate::marketplace::MarketplaceError;
use crate::marketplace::MarketplaceInterface;
use crate::marketplace::MarketplaceListError;
use crate::marketplace::MarketplacePluginAuthPolicy;
use crate::marketplace::MarketplacePluginPolicy;
use crate::marketplace::MarketplacePluginSource;
use crate::marketplace::ResolvedMarketplacePlugin;
use crate::marketplace::find_installable_marketplace_plugin;
use crate::marketplace::find_marketplace_plugin;
use crate::marketplace::list_marketplaces;
use crate::marketplace::load_marketplace;
use crate::marketplace::plugin_interface_with_marketplace_category;
use crate::marketplace_upgrade::ConfiguredMarketplaceUpgradeError;
use crate::marketplace_upgrade::ConfiguredMarketplaceUpgradeOutcome;
use crate::marketplace_upgrade::configured_git_marketplace_names;
use crate::marketplace_upgrade::upgrade_configured_git_marketplaces;
use crate::remote::RemoteInstalledPlugin;
use crate::remote::RemotePluginCatalogError;
use crate::remote::RemotePluginServiceConfig;
use crate::remote_legacy::RemotePluginFetchError;
use crate::remote_legacy::RemotePluginMutationError;
use crate::startup_sync::curated_plugins_repo_path;
use crate::startup_sync::read_curated_plugins_sha;
use crate::startup_sync::sync_vastar_plugins_repo;
use crate::store::PluginInstallResult as StorePluginInstallResult;
use crate::store::PluginStore;
use crate::store::PluginStoreError;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Instant;
use tokio::sync::Semaphore;
use tracing::info;
use tracing::warn;
use vac_analytics::AnalyticsEventsClient;
use vac_config::ConfigLayerStack;
use vac_config::PluginConfigEdit;
use vac_config::apply_user_plugin_config_edits;
use vac_config::clear_user_plugin;
use vac_config::set_user_plugin_enabled;
use vac_config::types::PluginConfig;
use vac_config::version_for_toml;
use vac_core_skills::SkillMetadata;
use vac_login::AuthManager;
use vac_login::VACAuth;
use vac_plugin::AppConnectorId;
use vac_plugin::PluginCapabilitySummary;
use vac_plugin::PluginId;
use vac_plugin::PluginIdError;
use vac_plugin::prompt_safe_plugin_description;
use vac_protocol::protocol::Product;
use vac_utils_absolute_path::AbsolutePathBuf;

static CURATED_REPO_SYNC_STARTED: AtomicBool = AtomicBool::new(false);
const FEATURED_PLUGIN_IDS_CACHE_TTL: std::time::Duration =
    std::time::Duration::from_secs(60 * 60 * 3);

#[derive(Debug, Clone)]
pub struct PluginsConfigInput {
    pub config_layer_stack: ConfigLayerStack,
    pub plugins_enabled: bool,
    pub remote_plugin_enabled: bool,
    pub plugin_hooks_enabled: bool,
    pub chatgpt_base_url: String,
}

impl PluginsConfigInput {
    pub fn new(
        config_layer_stack: ConfigLayerStack,
        plugins_enabled: bool,
        remote_plugin_enabled: bool,
        plugin_hooks_enabled: bool,
        chatgpt_base_url: String,
    ) -> Self {
        Self {
            config_layer_stack,
            plugins_enabled,
            remote_plugin_enabled,
            plugin_hooks_enabled,
            chatgpt_base_url,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
struct FeaturedPluginIdsCacheKey {
    chatgpt_base_url: String,
    account_id: Option<String>,
    chatgpt_user_id: Option<String>,
    is_workspace_account: bool,
}

#[derive(Clone)]
struct CachedFeaturedPluginIds {
    key: FeaturedPluginIdsCacheKey,
    expires_at: Instant,
    featured_plugin_ids: Vec<String>,
}

struct RemoteInstalledPluginsCacheRefreshRequest {
    service_config: RemotePluginServiceConfig,
    auth: Option<VACAuth>,
    notify: RemoteInstalledPluginsCacheRefreshNotify,
    // App-server attaches side effects such as skills metadata invalidation and MCP refreshes when
    // remote installed state changes.
    on_effective_plugins_changed: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
}

#[derive(Clone, Copy)]
enum RemoteInstalledPluginsCacheRefreshNotify {
    IfCacheChanged,
    // Remote mutations may change local bundles or active MCP state even when the installed set is
    // unchanged. Notify after `/installed` succeeds so MCP refreshes are ordered after the remote
    // installed cache.
    AfterSuccessfulRefresh,
}

#[derive(Default)]
struct RemoteInstalledPluginsCacheRefreshState {
    requested: Option<RemoteInstalledPluginsCacheRefreshRequest>,
    in_flight: bool,
}

#[derive(Clone, PartialEq, Eq)]
struct NonCuratedCacheRefreshRequest {
    roots: Vec<AbsolutePathBuf>,
    mode: NonCuratedCacheRefreshMode,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NonCuratedCacheRefreshMode {
    IfVersionChanged,
    ForceReinstall,
}

#[derive(Default)]
struct NonCuratedCacheRefreshState {
    requested: Option<NonCuratedCacheRefreshRequest>,
    last_refreshed: Option<NonCuratedCacheRefreshRequest>,
    in_flight: bool,
}

#[derive(Default)]
struct ConfiguredMarketplaceUpgradeState {
    in_flight: bool,
}

fn remote_plugin_service_config(config: &PluginsConfigInput) -> RemotePluginServiceConfig {
    RemotePluginServiceConfig {
        chatgpt_base_url: config.chatgpt_base_url.clone(),
    }
}

fn featured_plugin_ids_cache_key(
    config: &PluginsConfigInput,
    auth: Option<&VACAuth>,
) -> FeaturedPluginIdsCacheKey {
    FeaturedPluginIdsCacheKey {
        chatgpt_base_url: config.chatgpt_base_url.clone(),
        account_id: auth.and_then(VACAuth::get_account_id),
        chatgpt_user_id: auth.and_then(VACAuth::get_chatgpt_user_id),
        is_workspace_account: auth.is_some_and(VACAuth::is_workspace_account),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginInstallRequest {
    pub plugin_name: String,
    pub marketplace_path: AbsolutePathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginReadRequest {
    pub plugin_name: String,
    pub marketplace_path: AbsolutePathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginInstallOutcome {
    pub plugin_id: PluginId,
    pub plugin_version: String,
    pub installed_path: AbsolutePathBuf,
    pub auth_policy: MarketplacePluginAuthPolicy,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PluginReadOutcome {
    pub marketplace_name: String,
    pub marketplace_path: Option<AbsolutePathBuf>,
    pub plugin: PluginDetail,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PluginDetail {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub source: MarketplacePluginSource,
    pub policy: MarketplacePluginPolicy,
    pub interface: Option<PluginManifestInterface>,
    pub installed: bool,
    pub enabled: bool,
    pub skills: Vec<SkillMetadata>,
    pub disabled_skill_paths: HashSet<AbsolutePathBuf>,
    pub apps: Vec<AppConnectorId>,
    pub mcp_server_names: Vec<String>,
    pub details_unavailable_reason: Option<PluginDetailsUnavailableReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginDetailsUnavailableReason {
    InstallRequiredForRemoteSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfiguredMarketplace {
    pub name: String,
    pub path: AbsolutePathBuf,
    pub interface: Option<MarketplaceInterface>,
    pub plugins: Vec<ConfiguredMarketplacePlugin>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfiguredMarketplacePlugin {
    pub id: String,
    pub name: String,
    pub source: MarketplacePluginSource,
    pub policy: MarketplacePluginPolicy,
    pub interface: Option<PluginManifestInterface>,
    pub installed: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfiguredMarketplaceListOutcome {
    pub marketplaces: Vec<ConfiguredMarketplace>,
    pub errors: Vec<MarketplaceListError>,
}

impl From<PluginDetail> for PluginCapabilitySummary {
    fn from(value: PluginDetail) -> Self {
        let has_skills = value.skills.iter().any(|skill| {
            !value
                .disabled_skill_paths
                .contains(&skill.path_to_skills_md)
        });
        Self {
            config_name: value.id,
            display_name: value.name,
            description: prompt_safe_plugin_description(value.description.as_deref()),
            has_skills,
            mcp_server_names: value.mcp_server_names,
            app_connector_ids: value.apps,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemotePluginSyncResult {
    /// Plugin ids newly installed into the local plugin cache.
    pub installed_plugin_ids: Vec<String>,
    /// Plugin ids whose local config was changed to enabled.
    pub enabled_plugin_ids: Vec<String>,
    /// Plugin ids whose local config was changed to disabled.
    /// This is not populated by `sync_plugins_from_remote`.
    pub disabled_plugin_ids: Vec<String>,
    /// Plugin ids removed from local cache or plugin config.
    pub uninstalled_plugin_ids: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum PluginRemoteSyncError {
    #[error("chatgpt authentication required to sync remote plugins")]
    AuthRequired,

    #[error(
        "chatgpt authentication required to sync remote plugins; api key auth is not supported"
    )]
    UnsupportedAuthMode,

    #[error("failed to read auth token for remote plugin sync: {0}")]
    AuthToken(#[source] std::io::Error),

    #[error("failed to send remote plugin sync request to {url}: {source}")]
    Request {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("remote plugin sync request to {url} failed with status {status}: {body}")]
    UnexpectedStatus {
        url: String,
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("failed to parse remote plugin sync response from {url}: {source}")]
    Decode {
        url: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("local curated marketplace is not available")]
    LocalMarketplaceNotFound,

    #[error("remote marketplace `{marketplace_name}` is not available locally")]
    UnknownRemoteMarketplace { marketplace_name: String },

    #[error("duplicate remote plugin `{plugin_name}` in sync response")]
    DuplicateRemotePlugin { plugin_name: String },

    #[error(
        "remote plugin `{plugin_name}` was not found in local marketplace `{marketplace_name}`"
    )]
    UnknownRemotePlugin {
        plugin_name: String,
        marketplace_name: String,
    },

    #[error("{0}")]
    InvalidPluginId(#[from] PluginIdError),

    #[error("{0}")]
    Marketplace(#[from] MarketplaceError),

    #[error("{0}")]
    Store(#[from] PluginStoreError),

    #[error("{0}")]
    Config(#[from] anyhow::Error),

    #[error("failed to join remote plugin sync task: {0}")]
    Join(#[from] tokio::task::JoinError),
}

impl PluginRemoteSyncError {
    fn join(source: tokio::task::JoinError) -> Self {
        Self::Join(source)
    }
}

impl From<RemotePluginFetchError> for PluginRemoteSyncError {
    fn from(value: RemotePluginFetchError) -> Self {
        match value {
            RemotePluginFetchError::AuthRequired => Self::AuthRequired,
            RemotePluginFetchError::UnsupportedAuthMode => Self::UnsupportedAuthMode,
            RemotePluginFetchError::AuthToken(source) => Self::AuthToken(source),
            RemotePluginFetchError::Request { url, source } => Self::Request { url, source },
            RemotePluginFetchError::UnexpectedStatus { url, status, body } => {
                Self::UnexpectedStatus { url, status, body }
            }
            RemotePluginFetchError::Decode { url, source } => Self::Decode { url, source },
        }
    }
}

pub struct PluginsManager {
    vac_home: PathBuf,
    store: PluginStore,
    featured_plugin_ids_cache: RwLock<Option<CachedFeaturedPluginIds>>,
    configured_marketplace_upgrade_state: RwLock<ConfiguredMarketplaceUpgradeState>,
    non_curated_cache_refresh_state: RwLock<NonCuratedCacheRefreshState>,
    cached_enabled_outcome: RwLock<Option<CachedPluginLoadOutcome>>,
    remote_installed_plugins_cache: RwLock<Option<Vec<RemoteInstalledPlugin>>>,
    remote_installed_plugins_cache_refresh_state: RwLock<RemoteInstalledPluginsCacheRefreshState>,
    remote_sync_lock: Semaphore,
    restriction_product: Option<Product>,
    analytics_events_client: RwLock<Option<AnalyticsEventsClient>>,
}

#[derive(Clone)]
struct CachedPluginLoadOutcome {
    config_version: String,
    plugin_hooks_enabled: bool,
    outcome: PluginLoadOutcome,
}

mod lifecycle;
mod remote_cache;
mod featured;
mod install;
mod sync;
mod marketplace;
mod startup;
mod schedule;


fn remote_plugin_install_required_description(source: &MarketplacePluginSource) -> String {
    let source_description = match source {
        MarketplacePluginSource::Git {
            url,
            path,
            ref_name,
            sha,
        } => {
            let mut parts = vec![url.clone()];
            if let Some(path) = path {
                parts.push(format!("path `{path}`"));
            }
            if let Some(ref_name) = ref_name {
                parts.push(format!("ref `{ref_name}`"));
            }
            if let Some(sha) = sha {
                parts.push(format!("sha `{sha}`"));
            }
            parts.join(", ")
        }
        MarketplacePluginSource::Local { path } => path.as_path().display().to_string(),
    };

    format!(
        "This is a cross-repo plugin. Install it to view more detailed information. The source of the plugin is {source_description}."
    )
}

#[derive(Debug, thiserror::Error)]
pub enum PluginInstallError {
    #[error("{0}")]
    Marketplace(#[from] MarketplaceError),

    #[error("{0}")]
    Remote(#[from] RemotePluginMutationError),

    #[error("{0}")]
    Store(#[from] PluginStoreError),

    #[error("{0}")]
    Config(#[from] anyhow::Error),

    #[error("failed to join plugin install task: {0}")]
    Join(#[from] tokio::task::JoinError),
}

impl PluginInstallError {
    fn join(source: tokio::task::JoinError) -> Self {
        Self::Join(source)
    }

    pub fn is_invalid_request(&self) -> bool {
        matches!(
            self,
            Self::Marketplace(
                MarketplaceError::MarketplaceNotFound { .. }
                    | MarketplaceError::InvalidMarketplaceFile { .. }
                    | MarketplaceError::PluginNotFound { .. }
                    | MarketplaceError::PluginNotAvailable { .. }
                    | MarketplaceError::InvalidPlugin(_)
            ) | Self::Store(PluginStoreError::Invalid(_))
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PluginUninstallError {
    #[error("{0}")]
    InvalidPluginId(#[from] PluginIdError),

    #[error("{0}")]
    Remote(#[from] RemotePluginMutationError),

    #[error("{0}")]
    Store(#[from] PluginStoreError),

    #[error("{0}")]
    Config(#[from] anyhow::Error),

    #[error("failed to join plugin uninstall task: {0}")]
    Join(#[from] tokio::task::JoinError),
}

impl PluginUninstallError {
    fn join(source: tokio::task::JoinError) -> Self {
        Self::Join(source)
    }

    pub fn is_invalid_request(&self) -> bool {
        matches!(self, Self::InvalidPluginId(_))
    }
}

pub(crate) fn configured_plugins_from_stack(
    config_layer_stack: &ConfigLayerStack,
) -> HashMap<String, PluginConfig> {
    // Plugin entries remain persisted user config only.
    let Some(user_layer) = config_layer_stack.get_user_layer() else {
        return HashMap::new();
    };
    configured_plugins_from_user_config_value(&user_layer.config)
}

fn configured_plugins_from_user_config_value(
    user_config: &toml::Value,
) -> HashMap<String, PluginConfig> {
    let Some(plugins_value) = user_config.get("plugins") else {
        return HashMap::new();
    };
    match plugins_value.clone().try_into() {
        Ok(plugins) => plugins,
        Err(err) => {
            warn!("invalid plugins config: {err}");
            HashMap::new()
        }
    }
}

#[cfg(test)]
#[path = "manager_tests.rs"]
mod tests;
