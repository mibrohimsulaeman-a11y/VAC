use super::*;
use crate::{
    channels::{Channel, ChannelTestResult},
    client::{CallerContextInput, VACClient},
    config::{ApprovalMode, ChannelOverrides},
    router::RouterConfig,
    store::GatewayStore,
    types::{ChannelId, ChatType, DeliveryContext, InboundMessage, OutboundReply, PeerId},
};
use anyhow::Result;
use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, Sse},
    routing::{get, post},
};
use chrono::Utc;
use futures_util::stream;
use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex as AsyncMutex, mpsc};
use tokio_util::sync::CancellationToken;
use vac_agent_loop::ProposedToolCall;

fn queued(text: &str, display_name: Option<&str>, peer: &str) -> QueuedMessage {
    let metadata = match display_name {
        Some(name) => serde_json::json!({"display_name": name}),
        None => serde_json::json!({}),
    };

    QueuedMessage {
        inbound: InboundMessage {
            channel: ChannelId("slack".to_string()),
            peer_id: PeerId(peer.to_string()),
            chat_type: ChatType::Direct,
            text: text.to_string(),
            media: Vec::new(),
            metadata,
            timestamp: Utc::now(),
        },
        text: text.to_string(),
        run_options: RunStartOptions::default(),
        context: Vec::new(),
    }
}

#[derive(Clone)]
struct TestServerState {
    run_id: String,
    resolve_payloads: Arc<AsyncMutex<Vec<serde_json::Value>>>,
    last_event_ids: Arc<AsyncMutex<Vec<Option<String>>>>,
}

#[derive(Clone)]
struct TestChannel {
    id: ChannelId,
    edits: Arc<AsyncMutex<Vec<(String, String)>>>,
}

impl TestChannel {
    fn new(id: &str) -> Self {
        Self {
            id: ChannelId(id.to_string()),
            edits: Arc::new(AsyncMutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl Channel for TestChannel {
    fn id(&self) -> &ChannelId {
        &self.id
    }

    fn display_name(&self) -> &str {
        "Test"
    }

    async fn start(
        &self,
        _inbound_tx: mpsc::Sender<InboundMessage>,
        _cancel: CancellationToken,
    ) -> Result<()> {
        Ok(())
    }

    async fn send(&self, _reply: OutboundReply) -> Result<()> {
        Ok(())
    }

    async fn edit_message(&self, message_id: &str, new_text: &str) -> Result<()> {
        self.edits
            .lock()
            .await
            .push((message_id.to_string(), new_text.to_string()));
        Ok(())
    }

    async fn test(&self) -> Result<ChannelTestResult> {
        Ok(ChannelTestResult {
            channel: self.id.0.clone(),
            identity: "test-bot".to_string(),
            details: "ok".to_string(),
        })
    }
}

async fn test_resolve_tools_handler(
    State(state): State<TestServerState>,
    Path(_session_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> StatusCode {
    state.resolve_payloads.lock().await.push(payload);
    StatusCode::OK
}

async fn test_events_handler(
    State(state): State<TestServerState>,
    Path(_session_id): Path<String>,
    headers: HeaderMap,
) -> Sse<impl futures_util::Stream<Item = std::result::Result<Event, Infallible>>> {
    let last_event_id = headers
        .get("last-event-id")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    state.last_event_ids.lock().await.push(last_event_id);

    let event = Event::default()
        .id("17")
        .event("run_completed")
        .data(serde_json::json!({"run_id": state.run_id}).to_string());

    Sse::new(stream::iter(vec![Ok::<Event, Infallible>(event)]))
}

#[tokio::test]
async fn approval_not_reinserted_when_resume_fails_after_decision_sent() {
    let server_state = TestServerState {
        run_id: "run-1".to_string(),
        resolve_payloads: Arc::new(AsyncMutex::new(Vec::new())),
        last_event_ids: Arc::new(AsyncMutex::new(Vec::new())),
    };

    let app = Router::new()
        .route(
            "/v1/sessions/{session_id}/tools/decisions",
            post(test_resolve_tools_handler),
        )
        .with_state(server_state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test listener");
    let addr = listener.local_addr().expect("read listener addr");
    let server_handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app.into_make_service()).await;
    });

    let store = Arc::new(
        GatewayStore::open_in_memory()
            .await
            .expect("open in-memory gateway store"),
    );

    let test_channel = Arc::new(TestChannel::new("slack"));
    let mut channels: HashMap<String, Arc<dyn Channel>> = HashMap::new();
    channels.insert("slack".to_string(), test_channel);

    let dispatcher = Arc::new(Dispatcher::new(
        VACClient::new(format!("http://{addr}"), String::new()),
        channels,
        store,
        RouterConfig::default(),
        None,
        ApprovalMode::Allowlist,
        Vec::new(),
        HashMap::new(),
        "{channel}-{peer}".to_string(),
    ));

    let session_id = "session-1".to_string();
    let run_id = "run-1".to_string();

    dispatcher
        .pending_approvals
        .lock()
        .expect("lock pending_approvals")
        .insert(
            session_id.clone(),
            PendingApproval {
                session_id: session_id.clone(),
                run_id: run_id.clone(),
                tool_calls: vec![ProposedToolCall {
                    id: "tc-1".to_string(),
                    name: "mcp__run_command".to_string(),
                    arguments: serde_json::json!({"command": "kubectl get pods"}),
                    metadata: None,
                }],
                approval_id: "a3f0c92d".to_string(),
                prompt_message_id: "C123:123.456".to_string(),
                channel_name: "slack".to_string(),
                delivery: DeliveryContext {
                    channel: ChannelId("slack".to_string()),
                    peer_id: PeerId("u1".to_string()),
                    chat_type: ChatType::Direct,
                    channel_meta: serde_json::json!({"channel": "C123"}),
                    updated_at: Utc::now().timestamp_millis(),
                },
                cursor: Some(5),
                timeout_seconds: None,
                requested_at: Instant::now(),
            },
        );

    let inbound = InboundMessage {
        channel: ChannelId("slack".to_string()),
        peer_id: PeerId("u1".to_string()),
        chat_type: ChatType::Direct,
        text: String::new(),
        media: Vec::new(),
        metadata: serde_json::json!({
            "type": "approval_response",
            "approval_id": "a3f0c92d",
            "decision": "allow"
        }),
        timestamp: Utc::now(),
    };

    let (run_tx, _run_rx) = mpsc::channel(4);
    let result = dispatcher.handle_approval_response(inbound, run_tx).await;
    assert!(result.is_err());

    let payloads = server_state.resolve_payloads.lock().await.clone();
    assert_eq!(payloads.len(), 1, "decision should have been sent");
    assert!(
        dispatcher
            .pending_approvals
            .lock()
            .expect("lock pending_approvals")
            .is_empty(),
        "pending approval should not be reinserted after decision was sent"
    );

    server_handle.abort();
}

#[tokio::test]
async fn auto_reject_pending_approval_resolves_tools_edits_message_and_resumes() {
    let server_state = TestServerState {
        run_id: "run-1".to_string(),
        resolve_payloads: Arc::new(AsyncMutex::new(Vec::new())),
        last_event_ids: Arc::new(AsyncMutex::new(Vec::new())),
    };

    let app = Router::new()
        .route(
            "/v1/sessions/{session_id}/tools/decisions",
            post(test_resolve_tools_handler),
        )
        .route("/v1/sessions/{session_id}/events", get(test_events_handler))
        .with_state(server_state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test listener");
    let addr = listener.local_addr().expect("read listener addr");
    let server_handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app.into_make_service()).await;
    });

    let store = Arc::new(
        GatewayStore::open_in_memory()
            .await
            .expect("open in-memory gateway store"),
    );

    let test_channel = Arc::new(TestChannel::new("slack"));
    let mut channels: HashMap<String, Arc<dyn Channel>> = HashMap::new();
    channels.insert("slack".to_string(), test_channel.clone());

    let dispatcher = Arc::new(Dispatcher::new(
        VACClient::new(format!("http://{addr}"), String::new()),
        channels,
        store,
        RouterConfig::default(),
        None,
        ApprovalMode::Allowlist,
        Vec::new(),
        HashMap::new(),
        "{channel}-{peer}".to_string(),
    ));

    let session_id = "session-1".to_string();
    let run_id = "run-1".to_string();

    dispatcher
        .active_runs
        .lock()
        .expect("lock active_runs")
        .insert(
            session_id.clone(),
            ActiveRun {
                run_id: run_id.clone(),
                cancel: CancellationToken::new(),
                approval_mode: ApprovalMode::Allowlist,
                approval_allowlist: HashSet::new(),
            },
        );

    dispatcher
        .pending_approvals
        .lock()
        .expect("lock pending_approvals")
        .insert(
            session_id.clone(),
            PendingApproval {
                session_id: session_id.clone(),
                run_id: run_id.clone(),
                tool_calls: vec![ProposedToolCall {
                    id: "tc-1".to_string(),
                    name: "mcp__run_command".to_string(),
                    arguments: serde_json::json!({"command": "kubectl get pods"}),
                    metadata: None,
                }],
                approval_id: "a3f0c92d".to_string(),
                prompt_message_id: "C123:123.456".to_string(),
                channel_name: "slack".to_string(),
                delivery: DeliveryContext {
                    channel: ChannelId("slack".to_string()),
                    peer_id: PeerId("u1".to_string()),
                    chat_type: ChatType::Direct,
                    channel_meta: serde_json::json!({"channel": "C123"}),
                    updated_at: Utc::now().timestamp_millis(),
                },
                cursor: Some(5),
                timeout_seconds: None,
                requested_at: Instant::now(),
            },
        );

    let (run_tx, mut run_rx) = mpsc::channel(8);

    dispatcher
        .reject_pending_approval_for_session(&session_id, run_tx)
        .await
        .expect("auto reject pending approval");

    assert!(
        dispatcher
            .pending_approvals
            .lock()
            .expect("lock pending_approvals")
            .is_empty(),
        "pending approval should be removed"
    );

    let run_result = tokio::time::timeout(Duration::from_secs(3), run_rx.recv())
        .await
        .expect("timed out waiting for resumed run result")
        .expect("expected resumed run result");

    assert_eq!(run_result.session_id, session_id);
    assert_eq!(run_result.run_id, run_id);
    assert!(matches!(
        run_result.outcome,
        RunOutcome::Completed { cursor: Some(17) }
    ));

    let payloads = server_state.resolve_payloads.lock().await.clone();
    assert_eq!(payloads.len(), 1);
    assert_eq!(
        payloads[0].get("run_id").and_then(|value| value.as_str()),
        Some("run-1")
    );

    let decision = payloads[0]
        .get("decisions")
        .and_then(|value| value.get("tc-1"))
        .expect("expected tc-1 decision payload");
    assert_eq!(
        decision.get("action").and_then(|value| value.as_str()),
        Some("reject")
    );
    assert_eq!(
        decision.get("content").and_then(|value| value.as_str()),
        Some("Cancelled — new message received")
    );

    let last_event_ids = server_state.last_event_ids.lock().await.clone();
    assert_eq!(last_event_ids, vec![Some("5".to_string())]);

    let edits = test_channel.edits.lock().await.clone();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].0, "C123:123.456");
    assert_eq!(edits[0].1, "⏭️ Tools skipped — new message received");

    server_handle.abort();
}

fn inbound() -> InboundMessage {
    InboundMessage {
        channel: ChannelId("slack".to_string()),
        peer_id: PeerId("u1".to_string()),
        chat_type: ChatType::Direct,
        text: "hello".to_string(),
        media: Vec::new(),
        metadata: serde_json::Value::Null,
        timestamp: Utc::now(),
    }
}

#[test]
fn delivery_context_maps_to_caller_context_entry() {
    let context = serde_json::json!({
        "trigger": "nightly",
        "status": "failed",
        "summary": "disk at 95%",
        "check_output": "df -h"
    });

    let mapped = delivery_context_to_caller_context(&context);
    assert_eq!(mapped.len(), 1);
    assert_eq!(mapped[0].name, "watch_delivery_context");
    assert_eq!(mapped[0].priority.as_deref(), Some("high"));
    assert!(mapped[0].content.contains("Trigger: nightly"));
    assert!(mapped[0].content.contains("Status: failed"));
}

#[test]
fn truncate_chars_respects_unicode_boundaries() {
    let input = "ééééé";
    let output = truncate_chars_with_ellipsis(input, 3);
    assert_eq!(output, "ééé...");
}

#[test]
fn delivery_context_maps_partial_payload() {
    let context = serde_json::json!({ "trigger": "manual" });

    let mapped = delivery_context_to_caller_context(&context);
    assert_eq!(mapped.len(), 1);
    assert!(mapped[0].content.contains("Trigger: manual"));
    assert!(!mapped[0].content.contains("Status:"));
    assert!(!mapped[0].content.contains("Summary:"));
    assert!(!mapped[0].content.contains("Check output:"));
}

#[test]
fn delivery_context_handles_empty_payload() {
    let mapped = delivery_context_to_caller_context(&serde_json::json!({}));
    assert_eq!(mapped.len(), 1);
    assert!(
        mapped[0]
            .content
            .contains("The user is replying to a previous notification")
    );
    assert!(!mapped[0].content.contains("Trigger:"));
}

#[test]
fn stream_buffer_flush_rules() {
    assert!(should_flush_stream_buffer(
        "hello\n\nworld",
        Duration::from_millis(100)
    ));
    assert!(!should_flush_stream_buffer(
        &"x".repeat(501),
        Duration::from_millis(100)
    ));
    assert!(should_flush_stream_buffer(
        "hello\nworld",
        Duration::from_secs(3)
    ));
    assert!(!should_flush_stream_buffer("hello", Duration::from_secs(3)));
}

#[test]
fn take_completed_line_chunk_keeps_remainder() {
    let mut buffer = String::from("line1\nline2\npartial");
    let chunk = take_completed_line_chunk(&mut buffer).expect("chunk should exist");

    assert_eq!(chunk, "line1\nline2\n");
    assert_eq!(buffer, "partial");
}

#[test]
fn take_completed_line_chunk_avoids_splitting_open_fenced_code_block() {
    let mut buffer = String::from("before\n```sh\necho one\n");
    let chunk = take_completed_line_chunk(&mut buffer).expect("chunk should exist");

    assert_eq!(chunk, "before\n");
    assert_eq!(buffer, "```sh\necho one\n");
}

#[test]
fn take_completed_line_chunk_flushes_only_completed_fenced_code_blocks() {
    let mut buffer = String::from("```sh\necho one\n```\n```sh\necho two\n");
    let chunk = take_completed_line_chunk(&mut buffer).expect("chunk should exist");

    assert_eq!(chunk, "```sh\necho one\n```\n");
    assert_eq!(buffer, "```sh\necho two\n");
}

#[test]
fn queue_batching_keeps_sender_attribution() {
    let batch = vec![
        queued("Can you check logs?", Some("alice"), "u1"),
        queued("Also include disk usage", Some("bob"), "u2"),
    ];

    let combined = format_batched_queue_messages(&batch);
    assert!(combined.contains("alice: Can you check logs?"));
    assert!(combined.contains("bob: Also include disk usage"));
}

#[test]
fn sender_name_falls_back_to_username() {
    let metadata = serde_json::json!({"username": "carol"});
    assert_eq!(sender_name(&metadata).as_deref(), Some("carol"));
}

#[test]
fn merge_drained_queue_keeps_drained_messages_first() {
    let drained = vec![queued("drained-1", Some("alice"), "u1")];
    let existing = vec![queued("existing-1", Some("bob"), "u2")];

    let merged = merge_drained_queue(drained, existing);
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].text, "drained-1");
    assert_eq!(merged[1].text, "existing-1");
}

#[test]
fn extract_run_options_reads_timeout_model_and_sandbox() {
    let metadata = serde_json::json!({
        "gateway_run_options": {
            "model": "claude-sonnet",
            "sandbox": true,
            "timeout": 60
        }
    });

    let options = extract_run_options(&metadata);
    assert_eq!(options.model.as_deref(), Some("claude-sonnet"));
    assert_eq!(options.sandbox, Some(true));
    assert_eq!(options.timeout_seconds, Some(60));
}

#[test]
fn strip_mcp_prefix_removes_namespace() {
    assert_eq!(strip_mcp_prefix("mcp__run_command"), "run_command");
    assert_eq!(strip_mcp_prefix("mcp__tools__run_command"), "run_command");
    assert_eq!(strip_mcp_prefix("run_command"), "run_command");
}

#[test]
fn truncate_respects_char_boundaries() {
    let value = "héllo world";
    assert_eq!(truncate(value, 3), "hél…");
}

#[test]
fn render_run_command_preview_shows_code_block() {
    let preview = render_tool_preview(
        "run_command",
        &serde_json::json!({"command": "kubectl get pods -n staging"}),
    );
    assert!(preview.contains("```\nkubectl get pods -n staging\n```"));
}

#[test]
fn render_run_command_preview_shows_remote_host() {
    let preview = render_tool_preview(
        "run_command",
        &serde_json::json!({"command": "uptime", "remote": "root@10.0.1.5"}),
    );
    assert!(preview.contains("on `root@10.0.1.5`"));
    assert!(preview.contains("```\nuptime\n```"));
}

#[test]
fn render_create_preview_shows_path_and_content() {
    let preview = render_tool_preview(
        "create",
        &serde_json::json!({"path": "/tmp/hello.sh", "file_text": "#!/bin/bash\necho hello"}),
    );
    assert!(preview.contains("`/tmp/hello.sh`"));
    assert!(preview.contains("```\n#!/bin/bash\necho hello\n```"));
}

#[test]
fn render_str_replace_preview_shows_diff() {
    let preview = render_tool_preview(
        "str_replace",
        &serde_json::json!({
            "path": "deploy.yaml",
            "old_str": "replicas: 1",
            "new_str": "replicas: 3"
        }),
    );
    assert!(preview.contains("`deploy.yaml`"));
    assert!(preview.contains("- replicas: 1"));
    assert!(preview.contains("+ replicas: 3"));
}

#[test]
fn render_remove_preview_shows_recursive() {
    let preview = render_tool_preview(
        "remove",
        &serde_json::json!({"path": "/tmp/old", "recursive": true}),
    );
    assert_eq!(preview, "`/tmp/old` (recursive)");
}

#[test]
fn render_view_preview_shows_grep_and_range() {
    let preview = render_tool_preview(
        "view",
        &serde_json::json!({"path": "src/main.rs", "grep": "TODO", "view_range": [1, 50]}),
    );
    assert!(preview.contains("`src/main.rs`"));
    assert!(preview.contains("grep=`TODO`"));
    assert!(preview.contains("lines [1,50]"));
}

#[test]
fn render_generic_preview_falls_back_to_first_key() {
    let preview = render_tool_preview(
        "some_custom_tool",
        &serde_json::json!({"url": "https://example.com"}),
    );
    assert!(preview.contains("`https://example.com`"));
}

#[test]
fn render_subagent_preview_shows_description_and_instructions() {
    let preview = render_tool_preview(
        "dynamic_subagent_task",
        &serde_json::json!({
            "description": "Enumerate S3 buckets",
            "instructions": "List all S3 buckets in the account.\nInclude regions and sizes.",
            "tools": ["view", "run_command", "search_docs"],
            "enable_sandbox": true
        }),
    );
    assert!(preview.starts_with("**Enumerate S3 buckets** (🛡 sandboxed)\n"));
    assert!(preview.contains("tools: `view`, `run_command`, `search_docs`"));
    assert!(
        preview
            .contains("```\nList all S3 buckets in the account.\nInclude regions and sizes.\n```")
    );
    // tools line appears before the code block
    let tools_pos = preview.find("tools:").expect("tools line");
    let code_pos = preview.find("```").expect("code block");
    assert!(tools_pos < code_pos);
}

#[test]
fn render_subagent_preview_trims_instruction_whitespace() {
    let preview = render_tool_preview(
        "dynamic_subagent_task",
        &serde_json::json!({
            "description": "Trim test",
            "instructions": "\n  Do the thing\n\n",
            "tools": []
        }),
    );
    assert!(preview.contains("```\nDo the thing\n```"));
}

#[test]
fn render_subagent_preview_without_sandbox() {
    let preview = render_tool_preview(
        "dynamic_subagent_task",
        &serde_json::json!({
            "description": "Check logs",
            "instructions": "Tail the last 100 lines of app.log",
            "tools": ["view"]
        }),
    );
    assert!(!preview.contains("sandboxed"));
    assert!(preview.starts_with("**Check logs**\n"));
    assert!(preview.contains("tools: `view`"));
}

#[test]
fn render_subagent_preview_strips_tool_prefixes() {
    let preview = render_tool_preview(
        "dynamic_subagent_task",
        &serde_json::json!({
            "description": "Investigate",
            "instructions": "Check things",
            "tools": ["vac__view", "vac__run_command"]
        }),
    );
    assert!(preview.contains("tools: `view`, `run_command`"));
}

#[test]
fn render_subagent_preview_minimal() {
    let preview = render_tool_preview(
        "dynamic_subagent_task",
        &serde_json::json!({"instructions": "do stuff", "tools": []}),
    );
    assert!(preview.contains("(unnamed task)"));
}

#[test]
fn render_search_docs_preview_array_keywords() {
    let preview = render_tool_preview(
        "search_docs",
        &serde_json::json!({"keywords": ["kubernetes", "ingress", "nginx"]}),
    );
    assert!(preview.contains("`kubernetes, ingress, nginx`"));
}

#[test]
fn render_search_docs_preview_string_keywords() {
    let preview = render_tool_preview(
        "search_docs",
        &serde_json::json!({"keywords": "terraform aws vpc"}),
    );
    assert!(preview.contains("`terraform aws vpc`"));
}

#[test]
fn render_ask_user_preview_shows_labels() {
    let preview = render_tool_preview(
        "ask_user",
        &serde_json::json!({
            "questions": [
                {"label": "Region", "question": "Which AWS region?", "options": []},
                {"label": "Env", "question": "Which environment?", "options": []}
            ]
        }),
    );
    assert!(preview.contains("1. Region"));
    assert!(preview.contains("2. Env"));
}

#[test]
fn render_generic_preview_multi_param_compact_summary() {
    let preview = render_tool_preview(
        "unknown_tool",
        &serde_json::json!({
            "name": "test",
            "count": 42,
            "verbose": true,
            "items": [1, 2, 3]
        }),
    );
    assert!(preview.contains("name=`test`"));
    assert!(preview.contains("count=42"));
    assert!(preview.contains("verbose=true"));
    assert!(preview.contains("items: [3 items]"));
}

#[test]
fn render_generic_preview_multi_param_truncates_long_strings() {
    let long_value = "a".repeat(200);
    let preview = render_tool_preview(
        "unknown_tool",
        &serde_json::json!({
            "short": "ok",
            "long_field": long_value
        }),
    );
    assert!(preview.contains("short=`ok`"));
    assert!(preview.contains("long_field:"));
    assert!(preview.contains("…"));
}

#[test]
fn render_generic_preview_multi_param_caps_at_four() {
    let preview = render_tool_preview(
        "unknown_tool",
        &serde_json::json!({
            "a": "1", "b": "2", "c": "3", "d": "4", "e": "5", "f": "6"
        }),
    );
    // Should show at most 4 params + "…+N more"
    assert!(preview.contains("more"));
}

#[test]
fn allowlist_matches_prefixed_and_unprefixed_names() {
    let mut allowlist = HashSet::new();
    allowlist.insert("run_command".to_string());

    assert!(is_allowlisted("run_command", &allowlist));
    assert!(is_allowlisted("mcp__run_command", &allowlist));
    assert!(!is_allowlisted("mcp__str_replace", &allowlist));
}

#[test]
fn render_approver_display_uses_channel_mentions() {
    let peer = PeerId("U123".to_string());
    assert_eq!(render_approver_display("slack", &peer), "<@U123>");
    assert_eq!(render_approver_display("discord", &peer), "<@U123>");
    assert_eq!(render_approver_display("telegram", &peer), "U123");
}

#[test]
fn render_approval_prompt_includes_tools_and_auto_approved_count() {
    let tool_calls = vec![
        ProposedToolCall {
            id: "tc1".to_string(),
            name: "mcp__run_command".to_string(),
            arguments: serde_json::json!({"command": "kubectl get pods -n staging"}),
            metadata: None,
        },
        ProposedToolCall {
            id: "tc2".to_string(),
            name: "mcp__str_replace".to_string(),
            arguments: serde_json::json!({"path": "deploy.yaml", "old_str": "replicas: 1", "new_str": "replicas: 3"}),
            metadata: None,
        },
    ];

    let prompt = render_approval_prompt(&tool_calls, 1);
    assert!(prompt.contains("2 tools need approval"));
    assert!(prompt.contains("**1 · run_command**"));
    assert!(prompt.contains("```\nkubectl get pods -n staging\n```"));
    assert!(prompt.contains("**2 · str_replace**"));
    assert!(prompt.contains("`deploy.yaml`"));
    assert!(prompt.contains("- replicas: 1"));
    assert!(prompt.contains("+ replicas: 3"));
    assert!(prompt.contains("1 tool(s) auto-approved"));
}

#[test]
fn render_running_tools_summary_strips_prefixes_and_shows_previews() {
    let tool_calls = vec![
        ProposedToolCall {
            id: "tc1".to_string(),
            name: "vac__run_command".to_string(),
            arguments: serde_json::json!({"command": "kubectl get pods"}),
            metadata: None,
        },
        ProposedToolCall {
            id: "tc2".to_string(),
            name: "mcp__server__view".to_string(),
            arguments: serde_json::json!({"path": "/etc/config.yaml"}),
            metadata: None,
        },
    ];

    let summary = render_running_tools_summary(&tool_calls);
    assert!(summary.contains("Running 2 tools"));
    // Tool names should be stripped and numbered
    assert!(summary.contains("**1 · run_command**"));
    assert!(summary.contains("**2 · view**"));
    // Should NOT contain raw prefixed names
    assert!(!summary.contains("vac__"));
    assert!(!summary.contains("mcp__"));
    // Should include tool previews
    assert!(summary.contains("kubectl get pods"));
    assert!(summary.contains("/etc/config.yaml"));
}

#[test]
fn render_running_tools_summary_single_tool() {
    let tool_calls = vec![ProposedToolCall {
        id: "tc1".to_string(),
        name: "vac__create".to_string(),
        arguments: serde_json::json!({"path": "/tmp/test.txt", "file_text": "hello"}),
        metadata: None,
    }];

    let summary = render_running_tools_summary(&tool_calls);
    assert!(summary.contains("Running tool"));
    assert!(!summary.contains("Running 1 tools"));
    assert!(summary.contains("**1 · create**"));
    assert!(summary.contains("/tmp/test.txt"));
}

#[test]
fn remaining_timeout_after_approval_deducts_wait_time() {
    assert_eq!(
        remaining_timeout_after_approval(Some(60), Duration::from_secs(55)),
        Some(5)
    );
    assert_eq!(
        remaining_timeout_after_approval(Some(60), Duration::from_secs(120)),
        Some(MIN_RESUME_TIMEOUT_SECONDS)
    );
    assert_eq!(
        remaining_timeout_after_approval(None, Duration::from_secs(5)),
        None
    );
}

#[test]
fn generate_approval_id_is_short_hex() {
    let id = generate_approval_id();
    assert_eq!(id.len(), 8);
    assert!(id.chars().all(|ch| ch.is_ascii_hexdigit()));
}

#[test]
fn latest_non_empty_context_prefers_last_non_empty() {
    let queue = vec![
        QueuedMessage {
            inbound: inbound(),
            text: "one".to_string(),
            run_options: RunStartOptions::default(),
            context: Vec::new(),
        },
        QueuedMessage {
            inbound: inbound(),
            text: "two".to_string(),
            run_options: RunStartOptions::default(),
            context: vec![CallerContextInput {
                name: "ctx".to_string(),
                content: "value".to_string(),
                priority: Some("high".to_string()),
            }],
        },
    ];

    let context = latest_non_empty_context(&queue);
    assert_eq!(context.len(), 1);
    assert_eq!(context[0].name, "ctx");
}

#[test]
fn latest_non_empty_context_all_empty_returns_empty() {
    let queue = vec![QueuedMessage {
        inbound: inbound(),
        text: "one".to_string(),
        run_options: RunStartOptions::default(),
        context: Vec::new(),
    }];

    let context = latest_non_empty_context(&queue);
    assert!(context.is_empty());
}

#[derive(Default)]
struct StaticOverrideResolver {
    overrides: HashMap<String, RunOverrides>,
}

impl RunOverrideResolver for StaticOverrideResolver {
    fn resolve_run_overrides(&self, profile_name: &str) -> Option<RunOverrides> {
        self.overrides.get(profile_name).cloned()
    }
}

#[tokio::test]
async fn channel_overrides_affect_model_approval_and_server_overrides() {
    let store = Arc::new(
        GatewayStore::open_in_memory()
            .await
            .expect("open in-memory gateway store"),
    );

    let dispatcher = Dispatcher::new(
        VACClient::new("http://127.0.0.1:4096".to_string(), String::new()),
        HashMap::new(),
        store,
        RouterConfig::default(),
        Some("openai/default-model".to_string()),
        ApprovalMode::AllowAll,
        Vec::new(),
        HashMap::from([(
            "slack".to_string(),
            ChannelOverrides {
                model: Some("anthropic/claude-sonnet-4-5".to_string()),
                approval_mode: Some(ApprovalMode::Allowlist),
                approval_allowlist: Some(vec!["view".to_string()]),
            },
        )]),
        "{channel}-{peer}".to_string(),
    );

    assert_eq!(
        dispatcher.resolve_effective_model("slack", Some("meta/model".to_string())),
        Some("anthropic/claude-sonnet-4-5".to_string())
    );

    let (mode, allowlist) = dispatcher.resolve_channel_approval("slack");
    assert!(matches!(mode, ApprovalMode::Allowlist));
    assert!(allowlist.contains("view"));

    let overrides = dispatcher
        .build_run_overrides("slack")
        .expect("expected run overrides");
    assert_eq!(
        overrides.model.as_deref(),
        Some("anthropic/claude-sonnet-4-5")
    );
    assert!(matches!(
        overrides.auto_approve,
        Some(AutoApproveOverride::AllowList(_))
    ));
}

#[tokio::test]
async fn profile_resolver_overrides_inline_channel_overrides() {
    let store = Arc::new(
        GatewayStore::open_in_memory()
            .await
            .expect("open in-memory gateway store"),
    );

    let resolver = StaticOverrideResolver {
        overrides: HashMap::from([(
            "ops".to_string(),
            RunOverrides {
                model: Some("anthropic/claude-opus-4-5".to_string()),
                auto_approve: Some(AutoApproveOverride::AllowList(vec!["view".to_string()])),
                system_prompt: Some("ops prompt".to_string()),
                max_turns: Some(16),
            },
        )]),
    };

    let dispatcher = Dispatcher::new(
        VACClient::new("http://127.0.0.1:4096".to_string(), String::new()),
        HashMap::new(),
        store,
        RouterConfig::default(),
        None,
        ApprovalMode::AllowAll,
        Vec::new(),
        HashMap::from([(
            "slack".to_string(),
            ChannelOverrides {
                model: Some("openai/gpt-4o-mini".to_string()),
                approval_mode: Some(ApprovalMode::Allowlist),
                approval_allowlist: Some(vec!["search_docs".to_string()]),
            },
        )]),
        "{channel}-{peer}".to_string(),
    )
    .with_profile_resolution(
        HashMap::from([("slack".to_string(), "ops".to_string())]),
        Arc::new(resolver),
    );

    let overrides = dispatcher
        .build_run_overrides("slack")
        .expect("expected run overrides");
    assert_eq!(
        overrides.model.as_deref(),
        Some("anthropic/claude-opus-4-5")
    );
    assert_eq!(overrides.system_prompt.as_deref(), Some("ops prompt"));
    assert_eq!(overrides.max_turns, Some(16));
}

#[tokio::test]
async fn profile_resolver_falls_back_to_inline_when_profile_missing() {
    let store = Arc::new(
        GatewayStore::open_in_memory()
            .await
            .expect("open in-memory gateway store"),
    );

    let dispatcher = Dispatcher::new(
        VACClient::new("http://127.0.0.1:4096".to_string(), String::new()),
        HashMap::new(),
        store,
        RouterConfig::default(),
        None,
        ApprovalMode::AllowAll,
        Vec::new(),
        HashMap::from([(
            "slack".to_string(),
            ChannelOverrides {
                model: Some("openai/gpt-4o-mini".to_string()),
                approval_mode: Some(ApprovalMode::Allowlist),
                approval_allowlist: Some(vec!["view".to_string()]),
            },
        )]),
        "{channel}-{peer}".to_string(),
    )
    .with_profile_resolution(
        HashMap::from([("slack".to_string(), "missing".to_string())]),
        Arc::new(StaticOverrideResolver::default()),
    );

    let overrides = dispatcher
        .build_run_overrides("slack")
        .expect("expected inline fallback overrides");
    assert_eq!(overrides.model.as_deref(), Some("openai/gpt-4o-mini"));
}
