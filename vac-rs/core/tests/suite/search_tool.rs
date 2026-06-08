#![cfg(not(target_os = "windows"))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use anyhow::Result;
use core_test_support::apps_test_server::AppsTestServer;
use core_test_support::responses::ResponsesRequest;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::ev_tool_search_call;
use core_test_support::responses::mount_sse_once;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::namespace_child_tool;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::stdio_server_bin;
use core_test_support::test_vac::TestVACBuilder;
use core_test_support::test_vac::test_vac;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use vac_config::types::McpServerConfig;
use vac_config::types::McpServerTransportConfig;
use vac_core::config::Config;
use vac_features::Feature;
use vac_login::VACAuth;
use vac_models_manager::bundled_models_response;
use vac_protocol::dynamic_tools::DynamicToolCallOutputContentItem;
use vac_protocol::dynamic_tools::DynamicToolResponse;
use vac_protocol::dynamic_tools::DynamicToolSpec;
use vac_protocol::models::FunctionCallOutputPayload;
use vac_protocol::models::PermissionProfile;
use vac_protocol::protocol::AskForApproval;
use vac_protocol::protocol::EventMsg;
use vac_protocol::protocol::Op;
use vac_protocol::user_input::UserInput;

const SEARCH_TOOL_DESCRIPTION_SNIPPETS: [&str; 2] = [
    "You have access to tools from the following sources",
    "- Calendar: Plan events and manage your calendar.",
];
const TOOL_SEARCH_TOOL_NAME: &str = "tool_search";
const CALENDAR_CREATE_TOOL: &str = "mcp__vac_apps__calendar_create_event";
const CALENDAR_LIST_TOOL: &str = "mcp__vac_apps__calendar_list_events";
const SEARCH_CALENDAR_NAMESPACE: &str = "mcp__vac_apps__calendar";
const SEARCH_CALENDAR_CREATE_TOOL: &str = "_create_event";
const SEARCH_CALENDAR_LIST_TOOL: &str = "_list_events";

fn tool_names(body: &Value) -> Vec<String> {
    body.get("tools")
        .and_then(Value::as_array)
        .map(|tools| {
            tools
                .iter()
                .filter_map(|tool| {
                    tool.get("name")
                        .or_else(|| tool.get("type"))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn tool_search_description(body: &Value) -> Option<String> {
    body.get("tools")
        .and_then(Value::as_array)
        .and_then(|tools| {
            tools.iter().find_map(|tool| {
                if tool.get("type").and_then(Value::as_str) == Some(TOOL_SEARCH_TOOL_NAME) {
                    tool.get("description")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                } else {
                    None
                }
            })
        })
}

fn tool_search_output_item(request: &ResponsesRequest, call_id: &str) -> Value {
    request.tool_search_output(call_id)
}

fn tool_search_output_tools(request: &ResponsesRequest, call_id: &str) -> Vec<Value> {
    tool_search_output_item(request, call_id)
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn configure_search_capable_model(config: &mut Config) {
    let mut model_catalog = bundled_models_response()
        .unwrap_or_else(|err| panic!("bundled models.json should parse: {err}"));
    let model = model_catalog
        .models
        .iter_mut()
        .find(|model| model.slug == "gpt-5.4")
        .expect("gpt-5.4 exists in bundled models.json");
    config.model = Some("gpt-5.4".to_string());
    model.supports_search_tool = true;
    config.model_catalog = Some(model_catalog);
}

fn configure_search_capable_apps(config: &mut Config, apps_base_url: &str) {
    config
        .features
        .enable(Feature::Apps)
        .expect("test config should allow feature update");
    config.chatgpt_base_url = apps_base_url.to_string();
    configure_search_capable_model(config);
}

fn configure_apps_without_tool_search(config: &mut Config, apps_base_url: &str) {
    configure_search_capable_apps(config, apps_base_url);
    config
        .features
        .disable(Feature::ToolSearch)
        .expect("test config should allow feature update");
}

fn configure_apps(config: &mut Config, apps_base_url: &str) {
    configure_search_capable_apps(config, apps_base_url);
}

fn configured_builder(apps_base_url: String) -> TestVACBuilder {
    test_vac()
        .with_auth(VACAuth::create_dummy_chatgpt_auth_for_testing())
        .with_config(move |config| configure_apps(config, apps_base_url.as_str()))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn search_tool_enabled_by_default_adds_tool_search() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount_searchable(&server).await?;
    let mock = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let mut builder = configured_builder(apps_server.chatgpt_base_url.clone());
    let test = builder.build(&server).await?;

    test.submit_turn_with_approval_and_permission_profile(
        "list tools",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let body = mock.single_request().body_json();
    let tools = body
        .get("tools")
        .and_then(Value::as_array)
        .expect("tools array should exist");
    let tool_search = tools
        .iter()
        .find(|tool| tool.get("type").and_then(Value::as_str) == Some(TOOL_SEARCH_TOOL_NAME))
        .cloned()
        .expect("tool_search should be present");

    assert_eq!(
        tool_search,
        json!({
            "type": "tool_search",
            "execution": "client",
            "description": tool_search["description"].as_str().expect("description should exist"),
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Search query for deferred tools."},
                    "limit": {"type": "number", "description": "Maximum number of tools to return (defaults to 8)."},
                },
                "required": ["query"],
                "additionalProperties": false,
            }
        })
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn always_defer_feature_hides_small_app_tool_sets() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount(&server).await?;
    let mock = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let mut builder =
        configured_builder(apps_server.chatgpt_base_url.clone()).with_config(|config| {
            config
                .features
                .enable(Feature::ToolSearchAlwaysDeferMcpTools)
                .expect("test config should allow feature update");
        });
    let test = builder.build(&server).await?;

    test.submit_turn_with_approval_and_permission_profile(
        "list tools",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let body = mock.single_request().body_json();
    let tools = tool_names(&body);
    assert!(
        tools.iter().any(|name| name == TOOL_SEARCH_TOOL_NAME),
        "small app tool sets should be deferred behind tool_search: {tools:?}"
    );
    assert!(
        tools.iter().all(|name| !name.starts_with("mcp__")),
        "MCP tools should not be directly exposed: {tools:?}"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tool_search_disabled_exposes_apps_tools_directly() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount_searchable(&server).await?;
    let mock = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let mut builder = test_vac()
        .with_auth(VACAuth::create_dummy_chatgpt_auth_for_testing())
        .with_config(move |config| {
            configure_apps_without_tool_search(config, apps_server.chatgpt_base_url.as_str())
        });
    let test = builder.build(&server).await?;

    test.submit_turn_with_approval_and_permission_profile(
        "list tools",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let body = mock.single_request().body_json();
    let tools = tool_names(&body);
    assert!(!tools.iter().any(|name| name == TOOL_SEARCH_TOOL_NAME));
    assert!(
        namespace_child_tool(
            &body,
            SEARCH_CALENDAR_NAMESPACE,
            SEARCH_CALENDAR_CREATE_TOOL
        )
        .is_some()
    );
    assert!(
        namespace_child_tool(&body, SEARCH_CALENDAR_NAMESPACE, SEARCH_CALENDAR_LIST_TOOL).is_some()
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn search_tool_is_hidden_for_api_key_auth() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount(&server).await?;
    let mock = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let mut builder = test_vac()
        .with_auth(VACAuth::from_api_key("Test API Key"))
        .with_config(move |config| configure_apps(config, apps_server.chatgpt_base_url.as_str()));
    let test = builder.build(&server).await?;

    test.submit_turn_with_approval_and_permission_profile(
        "list tools",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let body = mock.single_request().body_json();
    let tools = tool_names(&body);
    assert!(
        !tools.iter().any(|name| name == TOOL_SEARCH_TOOL_NAME),
        "tools list should not include {TOOL_SEARCH_TOOL_NAME} for API key auth: {tools:?}"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn search_tool_adds_discovery_instructions_to_tool_description() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount_searchable(&server).await?;
    let mock = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let mut builder = configured_builder(apps_server.chatgpt_base_url.clone());
    let test = builder.build(&server).await?;

    test.submit_turn_with_approval_and_permission_profile(
        "list tools",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let body = mock.single_request().body_json();
    let description = tool_search_description(&body).expect("tool_search description should exist");
    assert!(
        SEARCH_TOOL_DESCRIPTION_SNIPPETS
            .iter()
            .all(|snippet| description.contains(snippet)),
        "tool_search description should include the updated workflow: {description:?}"
    );
    assert!(
        !description.contains("remainder of the current session/thread"),
        "tool_search description should not mention legacy client-side persistence: {description:?}"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn search_tool_hides_apps_tools_without_search() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount_searchable(&server).await?;
    let mock = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let mut builder = configured_builder(apps_server.chatgpt_base_url.clone());
    let test = builder.build(&server).await?;

    test.submit_turn_with_approval_and_permission_profile(
        "hello tools",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let body = mock.single_request().body_json();
    let tools = tool_names(&body);
    assert!(tools.iter().any(|name| name == TOOL_SEARCH_TOOL_NAME));
    assert!(!tools.iter().any(|name| name == CALENDAR_CREATE_TOOL));
    assert!(!tools.iter().any(|name| name == CALENDAR_LIST_TOOL));
    assert!(!tools.iter().any(|name| name == SEARCH_CALENDAR_NAMESPACE));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_app_mentions_do_not_expose_legacy_chatgpt_app_tools() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount(&server).await?;
    let mock = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let mut builder = configured_builder(apps_server.chatgpt_base_url.clone());
    let test = builder.build(&server).await?;

    test.submit_turn_with_approval_and_permission_profile(
        "Use [$calendar](app://calendar) and then call tools.",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let body = mock.single_request().body_json();
    let tools = tool_names(&body);
    assert!(
        namespace_child_tool(
            &body,
            SEARCH_CALENDAR_NAMESPACE,
            SEARCH_CALENDAR_CREATE_TOOL
        )
        .is_none(),
        "legacy ChatGPT app mentions should not expose remote create tool, got tools: {tools:?}"
    );
    assert!(
        namespace_child_tool(&body, SEARCH_CALENDAR_NAMESPACE, SEARCH_CALENDAR_LIST_TOOL).is_none(),
        "legacy ChatGPT app mentions should not expose remote list tool, got tools: {tools:?}"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tool_search_omits_legacy_chatgpt_apps_without_local_deferred_tools() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount_searchable(&server).await?;
    let mock = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let mut builder = configured_builder(apps_server.chatgpt_base_url.clone());
    let test = builder.build(&server).await?;
    test.vac
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "Find the calendar create tool".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    wait_for_event(&test.vac, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = mock.requests();
    assert_eq!(requests.len(), 1);
    let first_request_tools = tool_names(&requests[0].body_json());
    assert!(
        !first_request_tools
            .iter()
            .any(|name| name == TOOL_SEARCH_TOOL_NAME),
        "legacy ChatGPT Apps alone should not advertise tool_search: {first_request_tools:?}"
    );
    assert!(
        !first_request_tools
            .iter()
            .any(|name| name == CALENDAR_CREATE_TOOL),
        "legacy ChatGPT Apps should not expose direct app tools: {first_request_tools:?}"
    );
    assert!(
        !first_request_tools
            .iter()
            .any(|name| name == SEARCH_CALENDAR_NAMESPACE),
        "legacy ChatGPT Apps should not expose app namespace: {first_request_tools:?}"
    );

    let apps_tool_call = server
        .received_requests()
        .await
        .unwrap_or_default()
        .into_iter()
        .find(|request| {
            let Ok(body) = serde_json::from_slice::<Value>(&request.body) else {
                return false;
            };
            request.url.path() == "/api/vac/apps"
                && body.get("method").and_then(Value::as_str) == Some("tools/call")
        });
    assert!(
        apps_tool_call.is_none(),
        "legacy remote apps tools/call should not be used after local-only apps cleanup"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tool_search_returns_deferred_dynamic_tool_and_routes_follow_up_call() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let search_call_id = "tool-search-1";
    let dynamic_call_id = "dyn-search-call-1";
    let tool_name = "automation_update";
    let tool_description = "Create, update, view, or delete recurring automations.";
    let tool_args = json!({ "mode": "create" });
    let tool_call_arguments = serde_json::to_string(&tool_args)?;
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_tool_search_call(
                    search_call_id,
                    &json!({
                        "query": "recurring automations",
                        "limit": 8,
                    }),
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                json!({
                    "type": "response.output_item.done",
                    "item": {
                        "type": "function_call",
                        "call_id": dynamic_call_id,
                        "namespace": "vac_app",
                        "name": tool_name,
                        "arguments": tool_call_arguments,
                    }
                }),
                ev_completed("resp-2"),
            ]),
            sse(vec![
                ev_response_created("resp-3"),
                ev_assistant_message("msg-1", "done"),
                ev_completed("resp-3"),
            ]),
        ],
    )
    .await;

    let input_schema = json!({
        "type": "object",
        "properties": {
            "mode": { "type": "string" },
        },
        "required": ["mode"],
        "additionalProperties": false,
    });
    let dynamic_tool = DynamicToolSpec {
        namespace: Some("vac_app".to_string()),
        name: tool_name.to_string(),
        description: tool_description.to_string(),
        input_schema: input_schema.clone(),
        defer_loading: true,
    };

    let mut builder = test_vac().with_config(configure_search_capable_model);
    let base_test = builder.build(&server).await?;
    let new_thread = base_test
        .thread_manager
        .start_thread_with_tools(
            base_test.config.clone(),
            vec![dynamic_tool],
            /*persist_extended_history*/ false,
        )
        .await?;
    let mut test = base_test;
    test.vac = new_thread.thread;
    test.session_configured = new_thread.session_configured;

    test.vac
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "Use the automation tool".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let EventMsg::DynamicToolCallRequest(request) = wait_for_event(&test.vac, |event| {
        matches!(event, EventMsg::DynamicToolCallRequest(_))
    })
    .await
    else {
        unreachable!("event guard guarantees DynamicToolCallRequest");
    };
    assert_eq!(request.call_id, dynamic_call_id);
    assert_eq!(request.namespace.as_deref(), Some("vac_app"));
    assert_eq!(request.tool, tool_name);
    assert_eq!(request.arguments, tool_args);

    test.vac
        .submit(Op::DynamicToolResponse {
            id: request.call_id,
            response: DynamicToolResponse {
                content_items: vec![DynamicToolCallOutputContentItem::InputText {
                    text: "dynamic-search-ok".to_string(),
                }],
                success: true,
            },
        })
        .await?;

    wait_for_event(&test.vac, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = mock.requests();
    assert_eq!(requests.len(), 3);

    let first_request_body = requests[0].body_json();
    let first_request_tools = tool_names(&first_request_body);
    assert!(
        first_request_tools
            .iter()
            .any(|name| name == TOOL_SEARCH_TOOL_NAME),
        "first request should advertise tool_search: {first_request_tools:?}"
    );
    assert!(
        !first_request_tools.iter().any(|name| name == tool_name),
        "deferred dynamic tool should be hidden before search: {first_request_tools:?}"
    );

    let tools = tool_search_output_tools(&requests[1], search_call_id);
    assert_eq!(
        tools,
        vec![json!({
            "type": "namespace",
            "name": "vac_app",
            "description": "Tools in the vac_app namespace.",
            "tools": [{
                "type": "function",
                "name": tool_name,
                "description": tool_description,
                "strict": false,
                "defer_loading": true,
                "parameters": input_schema,
            }],
        })]
    );

    let second_request_body = requests[1].body_json();
    let second_request_tools = tool_names(&second_request_body);
    assert!(
        !second_request_tools.iter().any(|name| name == tool_name),
        "follow-up request should rely on tool_search_output history, not tool injection: {second_request_tools:?}"
    );

    let output = requests[2]
        .function_call_output(dynamic_call_id)
        .get("output")
        .cloned()
        .expect("dynamic tool output should be present");
    let payload: FunctionCallOutputPayload = serde_json::from_value(output)?;
    assert_eq!(
        payload,
        FunctionCallOutputPayload::from_text("dynamic-search-ok".to_string())
    );

    let third_request_body = requests[2].body_json();
    let third_request_tools = tool_names(&third_request_body);
    assert!(
        !third_request_tools.iter().any(|name| name == tool_name),
        "post-tool follow-up should rely on tool_search_output history, not tool injection: {third_request_tools:?}"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tool_search_indexes_only_enabled_non_app_mcp_tools() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount_searchable(&server).await?;
    let echo_call_id = "tool-search-echo";
    let image_call_id = "tool-search-image";
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_tool_search_call(
                    echo_call_id,
                    &json!({
                        "query": "Echo back the provided message and include environment data.",
                        "limit": 8,
                    }),
                ),
                ev_tool_search_call(
                    image_call_id,
                    &json!({
                        "query": "Return a single image content block.",
                        "limit": 8,
                    }),
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "done"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let rmcp_test_server_bin = stdio_server_bin()?;
    let mut builder =
        configured_builder(apps_server.chatgpt_base_url.clone()).with_config(move |config| {
            let mut servers = config.mcp_servers.get().clone();
            servers.insert(
                "rmcp".to_string(),
                McpServerConfig {
                    transport: McpServerTransportConfig::Stdio {
                        command: rmcp_test_server_bin,
                        args: Vec::new(),
                        env: None,
                        env_vars: Vec::new(),
                        cwd: None,
                    },
                    experimental_environment: None,
                    enabled: true,
                    required: false,
                    disabled_reason: None,
                    startup_timeout_sec: Some(Duration::from_secs(10)),
                    tool_timeout_sec: None,
                    default_tools_approval_mode: None,
                    enabled_tools: Some(vec!["echo".to_string(), "image".to_string()]),
                    disabled_tools: Some(vec!["image".to_string()]),
                    scopes: None,
                    oauth_resource: None,
                    supports_parallel_tool_calls: false,
                    tools: HashMap::new(),
                },
            );
            config
                .mcp_servers
                .set(servers)
                .expect("test mcp servers should accept any configuration");
        });
    let test = builder.build(&server).await?;

    test.submit_turn_with_approval_and_permission_profile(
        "Find the rmcp echo and image tools.",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let requests = mock.requests();
    assert_eq!(requests.len(), 2);

    let first_request_tools = tool_names(&requests[0].body_json());
    assert!(
        first_request_tools
            .iter()
            .any(|name| name == TOOL_SEARCH_TOOL_NAME),
        "first request should advertise tool_search: {first_request_tools:?}"
    );
    assert!(
        !first_request_tools
            .iter()
            .any(|name| name == "mcp__rmcp__echo"),
        "non-app MCP tools should be hidden before search in large-search mode: {first_request_tools:?}"
    );
    assert!(
        !first_request_tools.iter().any(|name| name == "mcp__rmcp__"),
        "non-app MCP namespace should be hidden before search in large-search mode: {first_request_tools:?}"
    );

    let echo_tools = tool_search_output_tools(&requests[1], echo_call_id);
    let echo_output = json!({ "tools": echo_tools });
    let rmcp_echo_tool = namespace_child_tool(&echo_output, "mcp__rmcp__", "echo")
        .expect("tool_search should return rmcp echo as a namespace child tool");
    assert_eq!(
        rmcp_echo_tool.get("type").and_then(Value::as_str),
        Some("function")
    );

    let image_tools = tool_search_output_tools(&requests[1], image_call_id);
    let found_rmcp_image_tool = image_tools
        .iter()
        .filter(|tool| tool.get("name").and_then(Value::as_str) == Some("mcp__rmcp__"))
        .flat_map(|namespace| namespace.get("tools").and_then(Value::as_array))
        .flatten()
        .any(|tool| tool.get("name").and_then(Value::as_str).is_some());
    assert!(
        !found_rmcp_image_tool,
        "disabled non-app MCP tools should not be searchable: {image_tools:?}"
    );

    Ok(())
}
