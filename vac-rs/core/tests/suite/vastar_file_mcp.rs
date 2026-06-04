#![cfg(not(target_os = "windows"))]

use anyhow::Result;
use core_test_support::apps_test_server::AppsTestServer;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call_with_namespace;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::test_vac::test_vac;
use serde_json::Value;
use serde_json::json;
use vac_core::config::Config;
use vac_features::Feature;
use vac_login::VACAuth;
use vac_protocol::models::PermissionProfile;
use vac_protocol::protocol::{AskForApproval, EventMsg};
use core_test_support::wait_for_event_with_timeout;

const DOCUMENT_EXTRACT_NAMESPACE: &str = "mcp__vac_apps__calendar";
const DOCUMENT_EXTRACT_TOOL: &str = "_extract_text";

fn configure_apps(config: &mut Config, chatgpt_base_url: &str) {
    if let Err(err) = config.features.enable(Feature::Apps) {
        panic!("test config should allow feature update: {err}");
    }
    config.chatgpt_base_url = chatgpt_base_url.to_string();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn vac_apps_file_params_upload_local_paths_before_mcp_tool_call() -> Result<()> {
    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount(&server).await?;

    // Cloud upload is intentionally disabled in the local coding-agent build.
    // No upload mocks are registered — none should be called.

    let call_id = "extract-call-1";
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_function_call_with_namespace(
                    call_id,
                    DOCUMENT_EXTRACT_NAMESPACE,
                    DOCUMENT_EXTRACT_TOOL,
                    &json!({"file": "report.txt"}).to_string(),
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "upload is not available"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let mut builder = test_vac()
        .with_auth(VACAuth::create_dummy_chatgpt_auth_for_testing())
        .with_config(move |config| {
            configure_apps(config, apps_server.chatgpt_base_url.as_str());
        });
    let test = builder.build(&server).await?;

    wait_for_event_with_timeout(
        &test.vac,
        |ev| matches!(ev, EventMsg::McpStartupComplete(_)),
        std::time::Duration::from_secs(10),
    )
    .await;

    tokio::fs::write(test.cwd.path().join("report.txt"), b"hello world").await?;

    test.submit_turn_with_approval_and_permission_profile(
        "Extract the report text with the app tool.",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    // The model should have received a tool error result containing the disabled message.
    let requests = mock.requests();
    let second_request_body = requests[1].body_json();
    let tool_result_input = second_request_body
        .pointer("/input")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .find(|item| item.get("type").and_then(Value::as_str) == Some("function_call_output"))
        .cloned()
        .expect("model second request should contain a function_call_output for the failed upload");

    let output_str = tool_result_input
        .pointer("/output")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        output_str.contains("legacy ChatGPT-account backend integration is disabled"),
        "tool error output should mention disabled backend; got: {output_str}"
    );

    server.verify().await;
    Ok(())
}
