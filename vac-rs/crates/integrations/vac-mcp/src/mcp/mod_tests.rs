use super::*;
use pretty_assertions::assert_eq;
use std::collections::HashMap;
use std::path::PathBuf;
use vac_config::Constrained;
use vac_config::types::AppToolApproval;
use vac_config::types::ApprovalsReviewer;
use vac_login::VACAuth;
use vac_plugin::AppConnectorId;
use vac_plugin::PluginCapabilitySummary;
use vac_protocol::models::ManagedFileSystemPermissions;
use vac_protocol::models::PermissionProfile;
use vac_protocol::permissions::NetworkSandboxPolicy;
use vac_protocol::protocol::AskForApproval;
use vac_protocol::protocol::GranularApprovalConfig;

fn test_mcp_config(vac_home: PathBuf) -> McpConfig {
    McpConfig {
        chatgpt_base_url: "https://provider.vac.invalid".to_string(),
        apps_mcp_path_override: None,
        vac_home,
        mcp_oauth_credentials_store_mode: OAuthCredentialsStoreMode::default(),
        mcp_oauth_callback_port: None,
        mcp_oauth_callback_url: None,
        skill_mcp_dependency_install_enabled: true,
        approval_policy: Constrained::allow_any(AskForApproval::OnFailure),
        vac_linux_sandbox_exe: None,
        use_legacy_landlock: false,
        apps_enabled: false,
        configured_mcp_servers: HashMap::new(),
        plugin_capability_summaries: Vec::new(),
    }
}

#[test]
fn qualified_mcp_tool_name_prefix_sanitizes_server_names_without_lowercasing() {
    assert_eq!(
        qualified_mcp_tool_name_prefix("Some-Server"),
        "mcp__Some_Server__".to_string()
    );
}

#[test]
fn mcp_prompt_auto_approval_honors_unrestricted_managed_profiles() {
    assert!(mcp_permission_prompt_is_auto_approved(
        AskForApproval::Never,
        &PermissionProfile::Managed {
            file_system: ManagedFileSystemPermissions::Unrestricted,
            network: NetworkSandboxPolicy::Enabled,
        },
        McpPermissionPromptAutoApproveContext::default(),
    ));
    assert!(mcp_permission_prompt_is_auto_approved(
        AskForApproval::Never,
        &PermissionProfile::Managed {
            file_system: ManagedFileSystemPermissions::Unrestricted,
            network: NetworkSandboxPolicy::Restricted,
        },
        McpPermissionPromptAutoApproveContext::default(),
    ));
    assert!(!mcp_permission_prompt_is_auto_approved(
        AskForApproval::Never,
        &PermissionProfile::read_only(),
        McpPermissionPromptAutoApproveContext::default(),
    ));
    assert!(!mcp_permission_prompt_is_auto_approved(
        AskForApproval::OnRequest,
        &PermissionProfile::Managed {
            file_system: ManagedFileSystemPermissions::Unrestricted,
            network: NetworkSandboxPolicy::Enabled,
        },
        McpPermissionPromptAutoApproveContext::default(),
    ));
}

#[test]
fn mcp_prompt_auto_approval_honors_auto_review_approved_tools() {
    assert!(mcp_permission_prompt_is_auto_approved(
        AskForApproval::OnRequest,
        &PermissionProfile::read_only(),
        McpPermissionPromptAutoApproveContext {
            approvals_reviewer: Some(ApprovalsReviewer::AutoReview),
            tool_approval_mode: Some(AppToolApproval::Approve),
        },
    ));
    assert!(mcp_permission_prompt_is_auto_approved(
        AskForApproval::Granular(GranularApprovalConfig {
            sandbox_approval: true,
            rules: true,
            skill_approval: true,
            request_permissions: true,
            mcp_elicitations: true,
        }),
        &PermissionProfile::read_only(),
        McpPermissionPromptAutoApproveContext {
            approvals_reviewer: Some(ApprovalsReviewer::AutoReview),
            tool_approval_mode: Some(AppToolApproval::Approve),
        },
    ));
    assert!(!mcp_permission_prompt_is_auto_approved(
        AskForApproval::OnRequest,
        &PermissionProfile::read_only(),
        McpPermissionPromptAutoApproveContext {
            approvals_reviewer: Some(ApprovalsReviewer::User),
            tool_approval_mode: Some(AppToolApproval::Approve),
        },
    ));
    assert!(!mcp_permission_prompt_is_auto_approved(
        AskForApproval::OnFailure,
        &PermissionProfile::read_only(),
        McpPermissionPromptAutoApproveContext {
            approvals_reviewer: Some(ApprovalsReviewer::AutoReview),
            tool_approval_mode: Some(AppToolApproval::Approve),
        },
    ));
    assert!(!mcp_permission_prompt_is_auto_approved(
        AskForApproval::UnlessTrusted,
        &PermissionProfile::read_only(),
        McpPermissionPromptAutoApproveContext {
            approvals_reviewer: Some(ApprovalsReviewer::AutoReview),
            tool_approval_mode: Some(AppToolApproval::Approve),
        },
    ));
}

#[test]
fn tool_plugin_provenance_collects_app_and_mcp_sources() {
    let provenance = ToolPluginProvenance::from_capability_summaries(&[
        PluginCapabilitySummary {
            display_name: "alpha-plugin".to_string(),
            app_connector_ids: vec![AppConnectorId("connector_example".to_string())],
            mcp_server_names: vec!["alpha".to_string()],
            ..PluginCapabilitySummary::default()
        },
        PluginCapabilitySummary {
            display_name: "beta-plugin".to_string(),
            app_connector_ids: vec![
                AppConnectorId("connector_example".to_string()),
                AppConnectorId("connector_gmail".to_string()),
            ],
            mcp_server_names: vec!["beta".to_string()],
            ..PluginCapabilitySummary::default()
        },
    ]);

    assert_eq!(
        provenance,
        ToolPluginProvenance {
            plugin_display_names_by_connector_id: HashMap::from([
                (
                    "connector_example".to_string(),
                    vec!["alpha-plugin".to_string(), "beta-plugin".to_string()],
                ),
                (
                    "connector_gmail".to_string(),
                    vec!["beta-plugin".to_string()],
                ),
            ]),
            plugin_display_names_by_mcp_server_name: HashMap::from([
                ("alpha".to_string(), vec!["alpha-plugin".to_string()]),
                ("beta".to_string(), vec!["beta-plugin".to_string()]),
            ]),
        }
    );
}

#[test]
fn vac_apps_mcp_url_for_base_url_keeps_existing_paths() {
    assert_eq!(
        vac_apps_mcp_url_for_base_url(
            "https://provider.vac.invalid/provider-api",
            /*apps_mcp_path_override*/ None,
        ),
        "https://provider.vac.invalid/provider-api/wham/apps"
    );
    assert_eq!(
        vac_apps_mcp_url_for_base_url(
            "https://chat.vastar.com",
            /*apps_mcp_path_override*/ None,
        ),
        "https://chat.vastar.com/provider-api/wham/apps"
    );
    assert_eq!(
        vac_apps_mcp_url_for_base_url(
            "http://localhost:8080/api/vac",
            /*apps_mcp_path_override*/ None,
        ),
        "http://localhost:8080/api/vac/apps"
    );
    assert_eq!(
        vac_apps_mcp_url_for_base_url(
            "http://localhost:8080",
            /*apps_mcp_path_override*/ None,
        ),
        "http://localhost:8080/api/vac/apps"
    );
}

#[test]
fn vac_apps_mcp_url_uses_legacy_vac_apps_path() {
    let config = test_mcp_config(PathBuf::from("/tmp"));

    assert_eq!(
        vac_apps_mcp_url(&config),
        "https://provider.vac.invalid/provider-api/wham/apps"
    );
}

#[test]
fn vac_apps_server_config_uses_legacy_vac_apps_path() {
    let mut config = test_mcp_config(PathBuf::from("/tmp"));
    let auth = VACAuth::create_dummy_chatgpt_auth_for_testing();

    let mut servers = with_vac_apps_mcp(HashMap::new(), /*auth*/ None, &config);
    assert!(!servers.contains_key(VAC_APPS_MCP_SERVER_NAME));

    config.apps_enabled = true;

    servers = with_vac_apps_mcp(servers, Some(&auth), &config);
    let server = servers
        .get(VAC_APPS_MCP_SERVER_NAME)
        .expect("vac apps should be present when apps is enabled");
    let url = match &server.transport {
        McpServerTransportConfig::StreamableHttp { url, .. } => url,
        _ => panic!("expected streamable http transport for vac apps"),
    };

    assert_eq!(url, "https://provider.vac.invalid/provider-api/wham/apps");
}

#[test]
fn vac_apps_server_config_uses_configured_apps_mcp_path_override() {
    let mut config = test_mcp_config(PathBuf::from("/tmp"));
    config.apps_mcp_path_override = Some("/custom/mcp".to_string());
    config.apps_enabled = true;
    let auth = VACAuth::create_dummy_chatgpt_auth_for_testing();

    let servers = with_vac_apps_mcp(HashMap::new(), Some(&auth), &config);
    let server = servers
        .get(VAC_APPS_MCP_SERVER_NAME)
        .expect("vac apps should be present when apps is enabled");
    let url = match &server.transport {
        McpServerTransportConfig::StreamableHttp { url, .. } => url,
        _ => panic!("expected streamable http transport for vac apps"),
    };

    assert_eq!(url, "https://provider.vac.invalid/provider-api/custom/mcp");
}

#[tokio::test]
async fn effective_mcp_servers_preserve_user_servers_and_add_vac_apps() {
    let vac_home = tempfile::tempdir().expect("tempdir");
    let mut config = test_mcp_config(vac_home.path().to_path_buf());
    config.apps_enabled = true;
    let auth = VACAuth::create_dummy_chatgpt_auth_for_testing();

    config.configured_mcp_servers.insert(
        "sample".to_string(),
        McpServerConfig {
            transport: McpServerTransportConfig::StreamableHttp {
                url: "https://user.example/mcp".to_string(),
                bearer_token_env_var: None,
                http_headers: None,
                env_http_headers: None,
            },
            experimental_environment: None,
            enabled: true,
            required: false,
            supports_parallel_tool_calls: false,
            disabled_reason: None,
            startup_timeout_sec: None,
            tool_timeout_sec: None,
            default_tools_approval_mode: None,
            enabled_tools: None,
            disabled_tools: None,
            scopes: None,
            oauth_resource: None,
            tools: HashMap::new(),
        },
    );
    config.configured_mcp_servers.insert(
        "docs".to_string(),
        McpServerConfig {
            transport: McpServerTransportConfig::StreamableHttp {
                url: "https://docs.example/mcp".to_string(),
                bearer_token_env_var: None,
                http_headers: None,
                env_http_headers: None,
            },
            experimental_environment: None,
            enabled: true,
            required: false,
            supports_parallel_tool_calls: false,
            disabled_reason: None,
            startup_timeout_sec: None,
            tool_timeout_sec: None,
            default_tools_approval_mode: None,
            enabled_tools: None,
            disabled_tools: None,
            scopes: None,
            oauth_resource: None,
            tools: HashMap::new(),
        },
    );

    let effective = effective_mcp_servers(&config, Some(&auth));

    let sample = effective.get("sample").expect("user server should exist");
    let docs = effective
        .get("docs")
        .expect("configured server should exist");
    let vac_apps = effective
        .get(VAC_APPS_MCP_SERVER_NAME)
        .expect("vac apps server should exist");

    match &sample.transport {
        McpServerTransportConfig::StreamableHttp { url, .. } => {
            assert_eq!(url, "https://user.example/mcp");
        }
        other => panic!("expected streamable http transport, got {other:?}"),
    }
    match &docs.transport {
        McpServerTransportConfig::StreamableHttp { url, .. } => {
            assert_eq!(url, "https://docs.example/mcp");
        }
        other => panic!("expected streamable http transport, got {other:?}"),
    }
    match &vac_apps.transport {
        McpServerTransportConfig::StreamableHttp { url, .. } => {
            assert_eq!(url, "https://provider.vac.invalid/provider-api/wham/apps");
        }
        other => panic!("expected streamable http transport, got {other:?}"),
    }
}
