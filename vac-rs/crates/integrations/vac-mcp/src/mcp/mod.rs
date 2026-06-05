pub use auth::McpAuthStatusEntry;
pub use auth::McpOAuthLoginConfig;
pub use auth::McpOAuthLoginSupport;
pub use auth::McpOAuthScopesSource;
pub use auth::ResolvedMcpOAuthScopes;
pub use auth::compute_auth_statuses;
pub use auth::discover_supported_scopes;
pub use auth::oauth_login_support;
pub use auth::resolve_oauth_scopes;
pub use auth::should_retry_without_scopes;

pub(crate) mod auth;

use std::collections::HashMap;
use std::path::PathBuf;

use async_channel::unbounded;
use rmcp::model::ReadResourceRequestParams;
use rmcp::model::ReadResourceResult;
use serde_json::Value;
use vac_config::Constrained;
use vac_config::McpServerConfig;
#[cfg(test)]
use vac_config::McpServerTransportConfig;
use vac_config::types::AppToolApproval;
use vac_config::types::ApprovalsReviewer;
use vac_config::types::OAuthCredentialsStoreMode;
use vac_login::VACAuth;
use vac_plugin::PluginCapabilitySummary;
use vac_protocol::mcp::Resource;
use vac_protocol::mcp::ResourceTemplate;
use vac_protocol::mcp::Tool;
use vac_protocol::models::PermissionProfile;
use vac_protocol::protocol::AskForApproval;
use vac_protocol::protocol::McpAuthStatus;
use vac_protocol::protocol::McpListToolsResponseEvent;

use crate::connection_manager::McpConnectionManager;
use crate::runtime::McpRuntimeEnvironment;
use crate::vac_apps::vac_apps_tools_cache_key;

pub const VAC_APPS_MCP_SERVER_NAME: &str = "vac_apps";
const MCP_TOOL_NAME_PREFIX: &str = "mcp";
const MCP_TOOL_NAME_DELIMITER: &str = "__";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum McpSnapshotDetail {
    #[default]
    Full,
    ToolsAndAuthOnly,
}

impl McpSnapshotDetail {
    fn include_resources(self) -> bool {
        matches!(self, Self::Full)
    }
}

pub fn qualified_mcp_tool_name_prefix(server_name: &str) -> String {
    sanitize_responses_api_tool_name(&format!(
        "{MCP_TOOL_NAME_PREFIX}{MCP_TOOL_NAME_DELIMITER}{server_name}{MCP_TOOL_NAME_DELIMITER}"
    ))
}

/// Returns true when MCP permission prompts should resolve as approved instead
/// of being shown to the user.
pub fn mcp_permission_prompt_is_auto_approved(
    approval_policy: AskForApproval,
    permission_profile: &PermissionProfile,
    context: McpPermissionPromptAutoApproveContext,
) -> bool {
    if matches!(
        approval_policy,
        AskForApproval::OnRequest | AskForApproval::Granular(_)
    ) && context.approvals_reviewer == Some(ApprovalsReviewer::AutoReview)
        && context.tool_approval_mode == Some(AppToolApproval::Approve)
    {
        return true;
    }

    if approval_policy != AskForApproval::Never {
        return false;
    }

    match permission_profile {
        PermissionProfile::Disabled | PermissionProfile::External { .. } => true,
        PermissionProfile::Managed { file_system, .. } => {
            file_system.to_sandbox_policy().has_full_disk_write_access()
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct McpPermissionPromptAutoApproveContext {
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    pub tool_approval_mode: Option<AppToolApproval>,
}

/// MCP runtime settings derived from `vac_core::config::Config`.
///
/// This struct should contain only long-lived configuration values that the
/// `vac-mcp` crate needs to construct server transports, enforce MCP
/// approval/sandbox policy, locate OAuth state, and merge plugin-provided MCP
/// servers. Request-scoped or auth-scoped state should not be stored here;
/// thread those values explicitly into runtime entry points such as
/// [`with_vac_apps_mcp`] and snapshot collection helpers so config objects do
/// not go stale when auth changes.
#[derive(Debug, Clone)]
pub struct McpConfig {
    /// Provider-neutral base URL for built-in app MCP servers, copied from the root config.
    ///
    /// The local VAC default does not inject any cloud app directory. When a
    /// provider-specific app directory is configured explicitly, callers pass
    /// a provider URL here without VAC appending legacy backend paths.
    pub chatgpt_base_url: String,
    /// Optional path override for the built-in apps MCP server.
    pub apps_mcp_path_override: Option<String>,
    /// VAC home directory used for MCP OAuth state and app-tool cache files.
    pub vac_home: PathBuf,
    /// Preferred credential store for MCP OAuth tokens.
    pub mcp_oauth_credentials_store_mode: OAuthCredentialsStoreMode,
    /// Optional fixed localhost callback port for MCP OAuth login.
    pub mcp_oauth_callback_port: Option<u16>,
    /// Optional OAuth redirect URI override for MCP login.
    pub mcp_oauth_callback_url: Option<String>,
    /// Whether skill MCP dependency installation prompts are enabled.
    pub skill_mcp_dependency_install_enabled: bool,
    /// Approval policy used for MCP tool calls and MCP elicitation requests.
    pub approval_policy: Constrained<AskForApproval>,
    /// Optional path to `vac-linux-sandbox` for sandboxed MCP tool execution.
    pub vac_linux_sandbox_exe: Option<PathBuf>,
    /// Whether to use legacy Landlock behavior in the MCP sandbox state.
    pub use_legacy_landlock: bool,
    /// Whether the app MCP integration is enabled by config.
    ///
    /// Auth/provider checks are explicit at runtime before any built-in apps
    /// MCP server is added.
    pub apps_enabled: bool,
    /// User-configured and plugin-provided MCP servers keyed by server name.
    pub configured_mcp_servers: HashMap<String, McpServerConfig>,
    /// Plugin metadata used to attribute MCP tools/connectors to plugin display names.
    pub plugin_capability_summaries: Vec<PluginCapabilitySummary>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolPluginProvenance {
    plugin_display_names_by_connector_id: HashMap<String, Vec<String>>,
    plugin_display_names_by_mcp_server_name: HashMap<String, Vec<String>>,
}

impl ToolPluginProvenance {
    pub fn plugin_display_names_for_connector_id(&self, connector_id: &str) -> &[String] {
        self.plugin_display_names_by_connector_id
            .get(connector_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn plugin_display_names_for_mcp_server_name(&self, server_name: &str) -> &[String] {
        self.plugin_display_names_by_mcp_server_name
            .get(server_name)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn from_capability_summaries(capability_summaries: &[PluginCapabilitySummary]) -> Self {
        let mut tool_plugin_provenance = Self::default();
        for plugin in capability_summaries {
            for connector_id in &plugin.app_connector_ids {
                tool_plugin_provenance
                    .plugin_display_names_by_connector_id
                    .entry(connector_id.0.clone())
                    .or_default()
                    .push(plugin.display_name.clone());
            }

            for server_name in &plugin.mcp_server_names {
                tool_plugin_provenance
                    .plugin_display_names_by_mcp_server_name
                    .entry(server_name.clone())
                    .or_default()
                    .push(plugin.display_name.clone());
            }
        }

        for plugin_names in tool_plugin_provenance
            .plugin_display_names_by_connector_id
            .values_mut()
            .chain(
                tool_plugin_provenance
                    .plugin_display_names_by_mcp_server_name
                    .values_mut(),
            )
        {
            plugin_names.sort_unstable();
            plugin_names.dedup();
        }

        tool_plugin_provenance
    }
}

pub fn with_vac_apps_mcp(
    servers: HashMap<String, McpServerConfig>,
    _auth: Option<&VACAuth>,
    _config: &McpConfig,
) -> HashMap<String, McpServerConfig> {
    // Local coding build: do not inject the cloud ChatGPT Apps MCP directory server.
    servers
}

pub fn configured_mcp_servers(config: &McpConfig) -> HashMap<String, McpServerConfig> {
    config.configured_mcp_servers.clone()
}

pub fn effective_mcp_servers(
    config: &McpConfig,
    auth: Option<&VACAuth>,
) -> HashMap<String, McpServerConfig> {
    let servers = configured_mcp_servers(config);
    with_vac_apps_mcp(servers, auth, config)
}

pub fn tool_plugin_provenance(config: &McpConfig) -> ToolPluginProvenance {
    ToolPluginProvenance::from_capability_summaries(&config.plugin_capability_summaries)
}

pub async fn read_mcp_resource(
    config: &McpConfig,
    auth: Option<&VACAuth>,
    runtime_environment: McpRuntimeEnvironment,
    server: &str,
    uri: &str,
) -> anyhow::Result<ReadResourceResult> {
    let mut mcp_servers = effective_mcp_servers(config, auth);
    mcp_servers.retain(|name, _| name == server);
    let auth_statuses = compute_auth_statuses(
        mcp_servers.iter(),
        config.mcp_oauth_credentials_store_mode,
        auth,
    )
    .await;
    let (tx_event, rx_event) = unbounded();
    drop(rx_event);
    let (manager, cancel_token) = McpConnectionManager::new(
        &mcp_servers,
        config.mcp_oauth_credentials_store_mode,
        auth_statuses,
        &config.approval_policy,
        String::new(),
        tx_event,
        PermissionProfile::default(),
        runtime_environment,
        config.vac_home.clone(),
        vac_apps_tools_cache_key(auth),
        tool_plugin_provenance(config),
        auth,
    )
    .await;

    let result = manager
        .read_resource(
            server,
            ReadResourceRequestParams {
                meta: None,
                uri: uri.to_string(),
            },
        )
        .await;
    cancel_token.cancel();
    result
}

#[derive(Debug, Clone)]
pub struct McpServerStatusSnapshot {
    pub tools_by_server: HashMap<String, HashMap<String, Tool>>,
    pub resources: HashMap<String, Vec<Resource>>,
    pub resource_templates: HashMap<String, Vec<ResourceTemplate>>,
    pub auth_statuses: HashMap<String, McpAuthStatus>,
}

pub async fn collect_mcp_server_status_snapshot_with_detail(
    config: &McpConfig,
    auth: Option<&VACAuth>,
    submit_id: String,
    runtime_environment: McpRuntimeEnvironment,
    detail: McpSnapshotDetail,
) -> McpServerStatusSnapshot {
    let mcp_servers = effective_mcp_servers(config, auth);
    let tool_plugin_provenance = tool_plugin_provenance(config);
    if mcp_servers.is_empty() {
        return McpServerStatusSnapshot {
            tools_by_server: HashMap::new(),
            resources: HashMap::new(),
            resource_templates: HashMap::new(),
            auth_statuses: HashMap::new(),
        };
    }

    let auth_status_entries = compute_auth_statuses(
        mcp_servers.iter(),
        config.mcp_oauth_credentials_store_mode,
        auth,
    )
    .await;

    let (tx_event, rx_event) = unbounded();
    drop(rx_event);

    let (mcp_connection_manager, cancel_token) = McpConnectionManager::new(
        &mcp_servers,
        config.mcp_oauth_credentials_store_mode,
        auth_status_entries.clone(),
        &config.approval_policy,
        submit_id,
        tx_event,
        PermissionProfile::default(),
        runtime_environment,
        config.vac_home.clone(),
        vac_apps_tools_cache_key(auth),
        tool_plugin_provenance,
        auth,
    )
    .await;

    let snapshot = collect_mcp_server_status_snapshot_from_manager(
        &mcp_connection_manager,
        auth_status_entries,
        detail,
    )
    .await;

    cancel_token.cancel();

    snapshot
}

pub async fn collect_mcp_snapshot_from_manager(
    mcp_connection_manager: &McpConnectionManager,
    auth_status_entries: HashMap<String, McpAuthStatusEntry>,
) -> McpListToolsResponseEvent {
    collect_mcp_snapshot_from_manager_with_detail(
        mcp_connection_manager,
        auth_status_entries,
        McpSnapshotDetail::Full,
    )
    .await
}

#[cfg(test)]
pub(crate) fn vac_apps_mcp_url(config: &McpConfig) -> String {
    vac_apps_mcp_url_for_base_url(
        &config.chatgpt_base_url,
        config.apps_mcp_path_override.as_deref(),
    )
}

/// The Responses API requires tool names to match `^[a-zA-Z0-9_-]+$`.
/// MCP server/tool names are user-controlled, so sanitize the fully-qualified
/// name we expose to the model by replacing any disallowed character with `_`.
pub(crate) fn sanitize_responses_api_tool_name(name: &str) -> String {
    let mut sanitized = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            sanitized.push(c);
        } else {
            sanitized.push('_');
        }
    }

    if sanitized.is_empty() {
        "_".to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
fn normalize_vac_apps_base_url(base_url: &str) -> String {
    base_url.trim_end_matches('/').to_string()
}

#[cfg(test)]
fn vac_apps_mcp_url_for_base_url(base_url: &str, apps_mcp_path_override: Option<&str>) -> String {
    let base_url = normalize_vac_apps_base_url(base_url);
    let base_url = if base_url.ends_with("/api/vac") {
        base_url
    } else {
        format!("{base_url}/api/vac")
    };
    let path = apps_mcp_path_override
        .unwrap_or("apps")
        .trim_start_matches('/');
    format!("{base_url}/{path}")
}

fn protocol_tool_from_rmcp_tool(name: &str, tool: &rmcp::model::Tool) -> Option<Tool> {
    match serde_json::to_value(tool) {
        Ok(value) => match Tool::from_mcp_value(value) {
            Ok(tool) => Some(tool),
            Err(err) => {
                tracing::warn!("Failed to convert MCP tool '{name}': {err}");
                None
            }
        },
        Err(err) => {
            tracing::warn!("Failed to serialize MCP tool '{name}': {err}");
            None
        }
    }
}

fn auth_statuses_from_entries(
    auth_status_entries: &HashMap<String, crate::mcp::auth::McpAuthStatusEntry>,
) -> HashMap<String, McpAuthStatus> {
    auth_status_entries
        .iter()
        .map(|(name, entry)| (name.clone(), entry.auth_status))
        .collect::<HashMap<_, _>>()
}

fn convert_mcp_resources(
    resources: HashMap<String, Vec<rmcp::model::Resource>>,
) -> HashMap<String, Vec<Resource>> {
    resources
        .into_iter()
        .map(|(name, resources)| {
            let resources = resources
                .into_iter()
                .filter_map(|resource| match serde_json::to_value(resource) {
                    Ok(value) => match Resource::from_mcp_value(value.clone()) {
                        Ok(resource) => Some(resource),
                        Err(err) => {
                            let (uri, resource_name) = match value {
                                Value::Object(obj) => (
                                    obj.get("uri")
                                        .and_then(|v| v.as_str().map(ToString::to_string)),
                                    obj.get("name")
                                        .and_then(|v| v.as_str().map(ToString::to_string)),
                                ),
                                _ => (None, None),
                            };

                            tracing::warn!(
                                "Failed to convert MCP resource (uri={uri:?}, name={resource_name:?}): {err}"
                            );
                            None
                        }
                    },
                    Err(err) => {
                        tracing::warn!("Failed to serialize MCP resource: {err}");
                        None
                    }
                })
                .collect::<Vec<_>>();
            (name, resources)
        })
        .collect::<HashMap<_, _>>()
}

fn convert_mcp_resource_templates(
    resource_templates: HashMap<String, Vec<rmcp::model::ResourceTemplate>>,
) -> HashMap<String, Vec<ResourceTemplate>> {
    resource_templates
        .into_iter()
        .map(|(name, templates)| {
            let templates = templates
                .into_iter()
                .filter_map(|template| match serde_json::to_value(template) {
                    Ok(value) => match ResourceTemplate::from_mcp_value(value.clone()) {
                        Ok(template) => Some(template),
                        Err(err) => {
                            let (uri_template, template_name) = match value {
                                Value::Object(obj) => (
                                    obj.get("uriTemplate")
                                        .or_else(|| obj.get("uri_template"))
                                        .and_then(|v| v.as_str().map(ToString::to_string)),
                                    obj.get("name")
                                        .and_then(|v| v.as_str().map(ToString::to_string)),
                                ),
                                _ => (None, None),
                            };

                            tracing::warn!(
                                "Failed to convert MCP resource template (uri_template={uri_template:?}, name={template_name:?}): {err}"
                            );
                            None
                        }
                    },
                    Err(err) => {
                        tracing::warn!("Failed to serialize MCP resource template: {err}");
                        None
                    }
                })
                .collect::<Vec<_>>();
            (name, templates)
        })
        .collect::<HashMap<_, _>>()
}

async fn collect_mcp_server_status_snapshot_from_manager(
    mcp_connection_manager: &McpConnectionManager,
    auth_status_entries: HashMap<String, crate::mcp::auth::McpAuthStatusEntry>,
    detail: McpSnapshotDetail,
) -> McpServerStatusSnapshot {
    let (tools, resources, resource_templates) = tokio::join!(
        mcp_connection_manager.list_all_tools(),
        async {
            if detail.include_resources() {
                mcp_connection_manager.list_all_resources().await
            } else {
                HashMap::new()
            }
        },
        async {
            if detail.include_resources() {
                mcp_connection_manager.list_all_resource_templates().await
            } else {
                HashMap::new()
            }
        },
    );

    let mut tools_by_server = HashMap::<String, HashMap<String, Tool>>::new();
    for (_qualified_name, tool_info) in tools {
        let raw_tool_name = tool_info.tool.name.to_string();
        let Some(tool) = protocol_tool_from_rmcp_tool(&raw_tool_name, &tool_info.tool) else {
            continue;
        };
        let tool_name = tool.name.clone();
        tools_by_server
            .entry(tool_info.server_name)
            .or_default()
            .insert(tool_name, tool);
    }

    McpServerStatusSnapshot {
        tools_by_server,
        resources: convert_mcp_resources(resources),
        resource_templates: convert_mcp_resource_templates(resource_templates),
        auth_statuses: auth_statuses_from_entries(&auth_status_entries),
    }
}

async fn collect_mcp_snapshot_from_manager_with_detail(
    mcp_connection_manager: &McpConnectionManager,
    auth_status_entries: HashMap<String, McpAuthStatusEntry>,
    detail: McpSnapshotDetail,
) -> McpListToolsResponseEvent {
    let (tools, resources, resource_templates) = tokio::join!(
        mcp_connection_manager.list_all_tools(),
        async {
            if detail.include_resources() {
                mcp_connection_manager.list_all_resources().await
            } else {
                HashMap::new()
            }
        },
        async {
            if detail.include_resources() {
                mcp_connection_manager.list_all_resource_templates().await
            } else {
                HashMap::new()
            }
        },
    );

    let tools = tools
        .into_iter()
        .filter_map(|(name, tool)| {
            protocol_tool_from_rmcp_tool(&name, &tool.tool).map(|tool| (name, tool))
        })
        .collect::<HashMap<_, _>>();

    McpListToolsResponseEvent {
        tools,
        resources: convert_mcp_resources(resources),
        resource_templates: convert_mcp_resource_templates(resource_templates),
        auth_statuses: auth_statuses_from_entries(&auth_status_entries),
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
