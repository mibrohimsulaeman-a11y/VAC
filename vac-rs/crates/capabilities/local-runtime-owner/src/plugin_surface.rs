use std::path::PathBuf;

use vac_core_plugins::manifest::PluginManifestInterface;
use vac_core_plugins::marketplace::MarketplacePluginAuthPolicy;
use vac_core_plugins::marketplace::MarketplacePluginInstallPolicy;
use vac_core_plugins::marketplace::MarketplacePluginSource;
use vac_utils_absolute_path::AbsolutePathBuf;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimePluginListResponse {
    pub marketplaces: Vec<RuntimePluginMarketplaceEntry>,
    pub marketplace_load_errors: Vec<RuntimeMarketplaceLoadErrorInfo>,
    pub featured_plugin_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeMarketplaceLoadErrorInfo {
    pub marketplace_path: AbsolutePathBuf,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePluginMarketplaceEntry {
    pub name: String,
    pub path: Option<AbsolutePathBuf>,
    pub interface: Option<RuntimeMarketplaceInterface>,
    pub plugins: Vec<RuntimePluginSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeMarketplaceInterface {
    pub display_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePluginSummary {
    pub id: String,
    pub name: String,
    pub source: RuntimePluginSource,
    pub installed: bool,
    pub enabled: bool,
    pub install_policy: RuntimePluginInstallPolicy,
    pub auth_policy: RuntimePluginAuthPolicy,
    pub availability: RuntimePluginAvailability,
    pub interface: Option<RuntimePluginInterface>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePluginReadResponse {
    pub plugin: RuntimePluginDetail,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePluginDetail {
    pub marketplace_name: String,
    pub marketplace_path: Option<AbsolutePathBuf>,
    pub summary: RuntimePluginSummary,
    pub description: Option<String>,
    pub skills: Vec<RuntimeSkillSummary>,
    pub app_ids: Vec<String>,
    pub mcp_servers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePluginInstallResponse {
    pub auth_policy: RuntimePluginAuthPolicy,
    pub installed_path: AbsolutePathBuf,
    pub plugin_id: String,
    pub app_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePluginUninstallResponse;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePluginSetEnabledResponse;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimePluginInstallPolicy {
    NotAvailable,
    Available,
    InstalledByDefault,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimePluginAuthPolicy {
    OnInstall,
    OnUse,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RuntimePluginAvailability {
    #[default]
    Available,
    DisabledByAdmin,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimePluginSource {
    Local {
        path: AbsolutePathBuf,
    },
    Git {
        url: String,
        path: Option<String>,
        ref_name: Option<String>,
        sha: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePluginInterface {
    pub display_name: Option<String>,
    pub short_description: Option<String>,
    pub long_description: Option<String>,
    pub developer_name: Option<String>,
    pub category: Option<String>,
    pub capabilities: Vec<String>,
    pub website_url: Option<String>,
    pub privacy_policy_url: Option<String>,
    pub terms_of_service_url: Option<String>,
    pub default_prompt: Option<Vec<String>>,
    pub brand_color: Option<String>,
    pub composer_icon: Option<AbsolutePathBuf>,
    pub logo: Option<AbsolutePathBuf>,
    pub screenshots: Vec<AbsolutePathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeSkillSummary {
    pub name: String,
    pub description: String,
    pub short_description: Option<String>,
    pub interface: Option<RuntimeSkillInterface>,
    pub path: Option<AbsolutePathBuf>,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeSkillInterface {
    pub display_name: Option<String>,
    pub short_description: Option<String>,
    pub icon_small: Option<AbsolutePathBuf>,
    pub icon_large: Option<AbsolutePathBuf>,
    pub brand_color: Option<String>,
    pub default_prompt: Option<String>,
}

impl From<MarketplacePluginInstallPolicy> for RuntimePluginInstallPolicy {
    fn from(value: MarketplacePluginInstallPolicy) -> Self {
        match value {
            MarketplacePluginInstallPolicy::NotAvailable => Self::NotAvailable,
            MarketplacePluginInstallPolicy::Available => Self::Available,
            MarketplacePluginInstallPolicy::InstalledByDefault => Self::InstalledByDefault,
        }
    }
}

impl From<MarketplacePluginAuthPolicy> for RuntimePluginAuthPolicy {
    fn from(value: MarketplacePluginAuthPolicy) -> Self {
        match value {
            MarketplacePluginAuthPolicy::OnInstall => Self::OnInstall,
            MarketplacePluginAuthPolicy::OnUse => Self::OnUse,
        }
    }
}

impl From<MarketplacePluginSource> for RuntimePluginSource {
    fn from(value: MarketplacePluginSource) -> Self {
        match value {
            MarketplacePluginSource::Local { path } => Self::Local { path },
            MarketplacePluginSource::Git {
                url,
                path,
                ref_name,
                sha,
            } => Self::Git {
                url,
                path,
                ref_name,
                sha,
            },
        }
    }
}

impl From<PluginManifestInterface> for RuntimePluginInterface {
    fn from(value: PluginManifestInterface) -> Self {
        Self {
            display_name: value.display_name,
            short_description: value.short_description,
            long_description: value.long_description,
            developer_name: value.developer_name,
            category: value.category,
            capabilities: value.capabilities,
            website_url: value.website_url,
            privacy_policy_url: value.privacy_policy_url,
            terms_of_service_url: value.terms_of_service_url,
            default_prompt: value.default_prompt,
            brand_color: value.brand_color,
            composer_icon: value.composer_icon,
            logo: value.logo,
            screenshots: value.screenshots,
        }
    }
}

impl From<vac_core_skills::model::SkillInterface> for RuntimeSkillInterface {
    fn from(value: vac_core_skills::model::SkillInterface) -> Self {
        Self {
            display_name: value.display_name,
            short_description: value.short_description,
            icon_small: value.icon_small,
            icon_large: value.icon_large,
            brand_color: value.brand_color,
            default_prompt: value.default_prompt,
        }
    }
}

pub(crate) fn runtime_skill_summaries_from_core(
    skills: &[vac_core_skills::SkillMetadata],
    disabled_skill_paths: &std::collections::HashSet<AbsolutePathBuf>,
) -> Vec<RuntimeSkillSummary> {
    skills
        .iter()
        .map(|skill| RuntimeSkillSummary {
            name: skill.name.clone(),
            description: skill.description.clone(),
            short_description: skill.short_description.clone(),
            interface: skill.interface.clone().map(RuntimeSkillInterface::from),
            path: Some(skill.path_to_skills_md.clone()),
            enabled: !disabled_skill_paths.contains(&skill.path_to_skills_md),
        })
        .collect()
}

pub(crate) fn plugin_app_ids(apps: Vec<vac_plugin::AppConnectorId>) -> Vec<String> {
    apps.into_iter().map(|app| app.0).collect()
}

pub(crate) fn path_parent(path: &AbsolutePathBuf) -> Option<PathBuf> {
    path.as_path().parent().map(std::path::Path::to_path_buf)
}
