pub mod installed_marketplaces;
pub mod loader;
mod manager;
pub mod manifest;
pub mod marketplace;
pub mod marketplace_add;
pub mod marketplace_remove;
pub mod marketplace_upgrade;
pub mod remote;
pub mod remote_bundle;
pub mod remote_legacy;
pub(crate) mod startup_remote_sync;
pub mod startup_sync;
pub mod store;
#[cfg(test)]
mod test_support;
pub mod toggles;

pub const VASTAR_CURATED_MARKETPLACE_NAME: &str = "vastar-curated";
pub const VASTAR_BUNDLED_MARKETPLACE_NAME: &str = "vastar-bundled";

pub const TOOL_SUGGEST_DISCOVERABLE_PLUGIN_ALLOWLIST: &[&str] = &[
    "github@vastar-curated",
    "notion@vastar-curated",
    "slack@vastar-curated",
    "gmail@vastar-curated",
    "google-calendar@vastar-curated",
    "google-drive@vastar-curated",
    "canva@vastar-curated",
    "teams@vastar-curated",
    "sharepoint@vastar-curated",
    "outlook-email@vastar-curated",
    "outlook-calendar@vastar-curated",
    "linear@vastar-curated",
    "figma@vastar-curated",
    "chrome@vastar-bundled",
    "computer-use@vastar-bundled",
];

pub type LoadedPlugin = vac_plugin::LoadedPlugin<vac_config::McpServerConfig>;
pub type PluginLoadOutcome = vac_plugin::PluginLoadOutcome<vac_config::McpServerConfig>;

pub use manager::ConfiguredMarketplace;
pub use manager::ConfiguredMarketplaceListOutcome;
pub use manager::ConfiguredMarketplacePlugin;
pub use manager::PluginDetail;
pub use manager::PluginDetailsUnavailableReason;
pub use manager::PluginInstallError;
pub use manager::PluginInstallOutcome;
pub use manager::PluginInstallRequest;
pub use manager::PluginReadOutcome;
pub use manager::PluginReadRequest;
pub use manager::PluginRemoteSyncError;
pub use manager::PluginUninstallError;
pub use manager::PluginsConfigInput;
pub use manager::PluginsManager;
pub use manager::RemotePluginSyncResult;
pub use marketplace_upgrade::ConfiguredMarketplaceUpgradeError as PluginMarketplaceUpgradeError;
pub use marketplace_upgrade::ConfiguredMarketplaceUpgradeOutcome as PluginMarketplaceUpgradeOutcome;
