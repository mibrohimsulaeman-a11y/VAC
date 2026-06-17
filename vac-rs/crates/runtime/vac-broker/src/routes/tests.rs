use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{
        Request,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
};
use http_body_util::BodyExt as _;
use std::sync::Arc;
use tower::ServiceExt;
use vac_agent_loop::{ToolApprovalAction, ToolApprovalPolicy};
use vac_foundation::models::context::{
    MAX_CALLER_CONTEXT_CONTENT_CHARS, MAX_CALLER_CONTEXT_ITEMS, MAX_CALLER_CONTEXT_NAME_CHARS,
};
use vac_remote_service::SessionStorage;

async fn test_state_with_event_capacity(capacity: usize) -> Result<AppState, String> {
    let storage_backend = vac_remote_service::LocalStorage::new(":memory:")
        .await
        .map_err(|error| error.to_string())?;
    let storage: Arc<dyn SessionStorage> = Arc::new(storage_backend);

    let inference = Arc::new(vac_provider_core::Inference::new());
    let events = Arc::new(crate::EventLog::new(capacity));
    let idempotency = Arc::new(crate::IdempotencyStore::new(Duration::from_secs(3600)));
    let model = vac_provider_core::Model::custom("test-model", "openai");

    let checkpoint_root =
        std::env::temp_dir().join(format!("vac-broker-routes-checkpoints-{}", Uuid::new_v4()));

    Ok(AppState::new(
        storage,
        events,
        idempotency,
        inference,
        vec![model.clone()],
        Some(model),
        ToolApprovalPolicy::Custom {
            rules: HashMap::from([("vac__view".to_string(), ToolApprovalAction::Approve)]),
            default: ToolApprovalAction::Ask,
        },
    )
    .with_checkpoint_store(Arc::new(crate::CheckpointStore::new(checkpoint_root))))
}

async fn test_state() -> Result<AppState, String> {
    test_state_with_event_capacity(256).await
}

async fn next_sse_chunk(body: &mut Body) -> Option<String> {
    let next = match tokio::time::timeout(Duration::from_millis(750), body.frame()).await {
        Ok(next) => next,
        Err(_) => return None,
    };

    let frame = match next {
        Some(Ok(frame)) => frame,
        Some(Err(_)) | None => return None,
    };

    let data = match frame.into_data() {
        Ok(data) => data,
        Err(_) => return None,
    };

    Some(String::from_utf8_lossy(&data).to_string())
}

#[tokio::test]
async fn openapi_endpoint_is_generated() {
    let app = match test_state().await {
        Ok(state) => router(state, AuthConfig::token("secret")),
        Err(error) => panic!("failed to create app state: {error}"),
    };

    let request = match Request::builder()
        .method("GET")
        .uri("/v1/openapi.json")
        .body(Body::empty())
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build request: {error}"),
    };

    let response = match app.oneshot(request).await {
        Ok(response) => response,
        Err(error) => panic!("request should succeed: {error}"),
    };

    assert_eq!(response.status(), StatusCode::OK);

    let body = match to_bytes(response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read body: {error}"),
    };

    let openapi: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(error) => panic!("invalid openapi json: {error}"),
    };

    assert_eq!(openapi.get("openapi"), Some(&json!("3.1.0")));
    assert!(
        openapi
            .get("paths")
            .and_then(|value| value.get("/v1/sessions/{id}/messages"))
            .is_some(),
        "expected generated paths to include /v1/sessions/{{id}}/messages"
    );

    assert!(
        openapi
            .get("components")
            .and_then(|value| value.get("securitySchemes"))
            .and_then(|value| value.get("bearer_auth"))
            .is_some(),
        "expected generated components.securitySchemes.bearer_auth"
    );

    let message_content_schema = openapi
        .get("components")
        .and_then(|value| value.get("schemas"))
        .and_then(|value| value.get("MessageContentDoc"));

    assert!(
        message_content_schema
            .and_then(|value| value.get("oneOf").or_else(|| value.get("anyOf")))
            .is_some(),
        "expected MessageContentDoc to model text-or-parts variants"
    );
}

#[tokio::test]
async fn health_endpoint_is_public() {
    let app = match test_state().await {
        Ok(state) => router(state, AuthConfig::token("secret")),
        Err(error) => panic!("failed to create app state: {error}"),
    };

    let request = match Request::builder().uri("/v1/health").body(Body::empty()) {
        Ok(request) => request,
        Err(error) => panic!("failed to build request: {error}"),
    };

    let response = match app.oneshot(request).await {
        Ok(response) => response,
        Err(error) => panic!("request should succeed: {error}"),
    };

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn events_endpoint_replays_from_last_event_id() {
    let state = match test_state().await {
        Ok(state) => state,
        Err(error) => panic!("failed to create app state: {error}"),
    };

    let app = router(state.clone(), AuthConfig::token("secret"));

    let session_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();

    state
        .events
        .publish(session_id, Some(run_id), AgentEvent::RunStarted { run_id })
        .await;
    state
        .events
        .publish(
            session_id,
            Some(run_id),
            AgentEvent::TurnStarted { run_id, turn: 1 },
        )
        .await;
    state
        .events
        .publish(
            session_id,
            Some(run_id),
            AgentEvent::TextDelta {
                run_id,
                delta: "hello".to_string(),
            },
        )
        .await;

    let request = match Request::builder()
        .method("GET")
        .uri(format!("/v1/sessions/{session_id}/events"))
        .header(AUTHORIZATION, "Bearer secret")
        .header("Last-Event-ID", "1")
        .body(Body::empty())
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build events request: {error}"),
    };

    let response = match app.oneshot(request).await {
        Ok(response) => response,
        Err(error) => panic!("events request should succeed: {error}"),
    };

    assert_eq!(response.status(), StatusCode::OK);

    let mut body = response.into_body();

    let first = match next_sse_chunk(&mut body).await {
        Some(chunk) => chunk,
        None => panic!("expected first replay SSE chunk"),
    };

    let second = match next_sse_chunk(&mut body).await {
        Some(chunk) => chunk,
        None => panic!("expected second replay SSE chunk"),
    };

    let replay = format!("{first}{second}");

    assert!(
        replay.contains("id:2") || replay.contains("id: 2"),
        "expected replay to include event id 2, got: {replay}"
    );
    assert!(
        replay.contains("id:3") || replay.contains("id: 3"),
        "expected replay to include event id 3, got: {replay}"
    );
    assert!(
        replay.contains("event:turn_started") || replay.contains("event: turn_started"),
        "expected replay to include turn_started event"
    );
    assert!(
        replay.contains("event:text_delta") || replay.contains("event: text_delta"),
        "expected replay to include text_delta event"
    );
}

#[tokio::test]
async fn events_endpoint_emits_gap_detected_control_event() {
    let state = match test_state_with_event_capacity(2).await {
        Ok(state) => state,
        Err(error) => panic!("failed to create app state: {error}"),
    };

    let app = router(state.clone(), AuthConfig::token("secret"));

    let session_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();

    for turn in 1..=4 {
        state
            .events
            .publish(
                session_id,
                Some(run_id),
                AgentEvent::TurnStarted { run_id, turn },
            )
            .await;
    }

    let request = match Request::builder()
        .method("GET")
        .uri(format!("/v1/sessions/{session_id}/events"))
        .header(AUTHORIZATION, "Bearer secret")
        .header("Last-Event-ID", "1")
        .body(Body::empty())
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build events request: {error}"),
    };

    let response = match app.oneshot(request).await {
        Ok(response) => response,
        Err(error) => panic!("events request should succeed: {error}"),
    };

    assert_eq!(response.status(), StatusCode::OK);

    let mut body = response.into_body();
    let first_chunk = match next_sse_chunk(&mut body).await {
        Some(chunk) => chunk,
        None => panic!("expected gap_detected SSE chunk"),
    };

    assert!(
        first_chunk.contains("event:gap_detected") || first_chunk.contains("event: gap_detected"),
        "expected gap_detected control event, got: {first_chunk}"
    );
    assert!(
        first_chunk.contains("\"requested_after_id\":1"),
        "expected requested_after_id in gap payload"
    );
    assert!(
        first_chunk.contains("\"resume_hint\":\"refresh_snapshot_then_resume\""),
        "expected resume hint in gap payload"
    );
}

#[tokio::test]
async fn sessions_messages_accepts_provider_core_message_input() {
    let app = match test_state().await {
        Ok(state) => router(state.clone(), AuthConfig::token("secret")),
        Err(error) => panic!("failed to create app state: {error}"),
    };

    let create_payload = json!({"title":"test"});
    let create_request = match Request::builder()
        .method("POST")
        .uri("/v1/sessions")
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(create_payload.to_string()))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build create request: {error}"),
    };

    let create_response = match app.clone().oneshot(create_request).await {
        Ok(response) => response,
        Err(error) => panic!("create request should succeed: {error}"),
    };

    let create_body = match to_bytes(create_response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read create response body: {error}"),
    };

    let created_session: serde_json::Value = match serde_json::from_slice(&create_body) {
        Ok(value) => value,
        Err(error) => panic!("invalid create response json: {error}"),
    };

    let session_id = match created_session.get("id").and_then(|value| value.as_str()) {
        Some(session_id) => session_id,
        None => panic!("create response missing session id"),
    };

    let payload = json!({
        "message": {
            "role": "user",
            "content": "hello from provider core"
        },
        "model": "openai/test-model",
        "context": [
            {
                "name": "watch_result",
                "content": "Health check completed successfully.",
                "priority": "high"
            }
        ]
    });

    let request = match Request::builder()
        .method("POST")
        .uri(format!("/v1/sessions/{session_id}/messages"))
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(payload.to_string()))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build request: {error}"),
    };

    let response = match app.oneshot(request).await {
        Ok(response) => response,
        Err(error) => panic!("request should succeed: {error}"),
    };

    assert_eq!(response.status(), StatusCode::OK);

    let body = match to_bytes(response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read response body: {error}"),
    };

    let parsed: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(parsed) => parsed,
        Err(error) => panic!("response should be valid json: {error}"),
    };

    assert!(parsed.get("run_id").is_some());
}

#[tokio::test]
async fn sessions_messages_rejects_non_user_role_input() {
    let app = match test_state().await {
        Ok(state) => router(state.clone(), AuthConfig::token("secret")),
        Err(error) => panic!("failed to create app state: {error}"),
    };

    let create_payload = json!({"title":"test-invalid-role"});
    let create_request = match Request::builder()
        .method("POST")
        .uri("/v1/sessions")
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(create_payload.to_string()))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build create request: {error}"),
    };

    let create_response = match app.clone().oneshot(create_request).await {
        Ok(response) => response,
        Err(error) => panic!("create request should succeed: {error}"),
    };

    let create_body = match to_bytes(create_response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read create response body: {error}"),
    };

    let created_session: serde_json::Value = match serde_json::from_slice(&create_body) {
        Ok(value) => value,
        Err(error) => panic!("invalid create response json: {error}"),
    };

    let session_id = match created_session.get("id").and_then(|value| value.as_str()) {
        Some(session_id) => session_id,
        None => panic!("create response missing session id"),
    };

    let payload = json!({
        "message": {
            "role": "assistant",
            "content": "this should be rejected"
        },
        "type": "message"
    });

    let request = match Request::builder()
        .method("POST")
        .uri(format!("/v1/sessions/{session_id}/messages"))
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(payload.to_string()))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build request: {error}"),
    };

    let response = match app.oneshot(request).await {
        Ok(response) => response,
        Err(error) => panic!("request should succeed: {error}"),
    };

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = match to_bytes(response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read response body: {error}"),
    };

    let parsed: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(parsed) => parsed,
        Err(error) => panic!("response should be valid json: {error}"),
    };

    assert_eq!(parsed.get("code"), Some(&json!("invalid_message_role")));
}

#[tokio::test]
async fn get_messages_returns_provider_core_parts_from_checkpoint_envelope() {
    let state = match test_state().await {
        Ok(state) => state,
        Err(error) => panic!("failed to create app state: {error}"),
    };

    let app = router(state.clone(), AuthConfig::token("secret"));

    let create_payload = json!({"title":"checkpoint-message-test"});
    let create_request = match Request::builder()
        .method("POST")
        .uri("/v1/sessions")
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(create_payload.to_string()))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build create request: {error}"),
    };

    let create_response = match app.clone().oneshot(create_request).await {
        Ok(response) => response,
        Err(error) => panic!("create request should succeed: {error}"),
    };

    let create_body = match to_bytes(create_response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read create body: {error}"),
    };

    let created_session: serde_json::Value = match serde_json::from_slice(&create_body) {
        Ok(value) => value,
        Err(error) => panic!("invalid create response json: {error}"),
    };

    let session_id = match created_session.get("id").and_then(|value| value.as_str()) {
        Some(session_id) => session_id,
        None => panic!("create response missing session id"),
    };

    let session_uuid = match Uuid::parse_str(session_id) {
        Ok(uuid) => uuid,
        Err(error) => panic!("invalid session id: {error}"),
    };

    let envelope = vac_agent_loop::CheckpointEnvelopeV1::new(
        Some(Uuid::new_v4()),
        vec![
            vac_provider_core::Message::new(vac_provider_core::Role::User, "hello"),
            vac_provider_core::Message::new(
                vac_provider_core::Role::Assistant,
                vec![
                    vac_provider_core::ContentPart::text("calling tool"),
                    vac_provider_core::ContentPart::tool_call(
                        "tc_1",
                        "vac__view",
                        json!({"path":"README.md"}),
                    ),
                ],
            ),
        ],
        json!({"source": "test"}),
    );

    let save = state
        .checkpoint_store
        .save_latest(session_uuid, &envelope)
        .await;
    assert!(save.is_ok(), "checkpoint save should succeed");

    let request = match Request::builder()
        .method("GET")
        .uri(format!("/v1/sessions/{session_id}/messages"))
        .header(AUTHORIZATION, "Bearer secret")
        .body(Body::empty())
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build messages request: {error}"),
    };

    let response = match app.oneshot(request).await {
        Ok(response) => response,
        Err(error) => panic!("messages request should succeed: {error}"),
    };

    assert_eq!(response.status(), StatusCode::OK);

    let body = match to_bytes(response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read messages body: {error}"),
    };

    let parsed: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(error) => panic!("invalid messages json: {error}"),
    };

    assert_eq!(parsed.get("total"), Some(&json!(2)));

    let messages = match parsed.get("messages").and_then(|value| value.as_array()) {
        Some(messages) => messages,
        None => panic!("messages payload should be an array"),
    };

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[1].get("role"), Some(&json!("assistant")));
    assert_eq!(
        messages[1]
            .get("content")
            .and_then(|value| value.get(1))
            .and_then(|value| value.get("type")),
        Some(&json!("tool_call"))
    );
    assert_eq!(
        messages[1]
            .get("content")
            .and_then(|value| value.get(1))
            .and_then(|value| value.get("name")),
        Some(&json!("vac__view"))
    );
}

#[tokio::test]
async fn update_session_accepts_visibility() {
    let app = match test_state().await {
        Ok(state) => router(state.clone(), AuthConfig::token("secret")),
        Err(error) => panic!("failed to create app state: {error}"),
    };

    let create_payload = json!({"title":"visibility-test"});
    let create_request = match Request::builder()
        .method("POST")
        .uri("/v1/sessions")
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(create_payload.to_string()))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build create request: {error}"),
    };

    let create_response = match app.clone().oneshot(create_request).await {
        Ok(response) => response,
        Err(error) => panic!("create request should succeed: {error}"),
    };

    let create_body = match to_bytes(create_response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read create body: {error}"),
    };

    let create_json: serde_json::Value = match serde_json::from_slice(&create_body) {
        Ok(value) => value,
        Err(error) => panic!("invalid create json: {error}"),
    };

    let session_id = match create_json.get("id").and_then(|value| value.as_str()) {
        Some(value) => value,
        None => panic!("missing session id"),
    };

    let update_payload = json!({"visibility":"PUBLIC"});
    let update_request = match Request::builder()
        .method("PATCH")
        .uri(format!("/v1/sessions/{session_id}"))
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(update_payload.to_string()))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build update request: {error}"),
    };

    let update_response = match app.oneshot(update_request).await {
        Ok(response) => response,
        Err(error) => panic!("update request should succeed: {error}"),
    };

    assert_eq!(update_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn get_session_includes_runtime_config() {
    let app = match test_state().await {
        Ok(state) => router(state.clone(), AuthConfig::token("secret")),
        Err(error) => panic!("failed to create app state: {error}"),
    };

    let create_payload = json!({"title":"runtime-config-test"});
    let create_request = match Request::builder()
        .method("POST")
        .uri("/v1/sessions")
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(create_payload.to_string()))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build create request: {error}"),
    };

    let create_response = match app.clone().oneshot(create_request).await {
        Ok(response) => response,
        Err(error) => panic!("create request should succeed: {error}"),
    };

    let create_body = match to_bytes(create_response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read create response body: {error}"),
    };

    let created_session: serde_json::Value = match serde_json::from_slice(&create_body) {
        Ok(value) => value,
        Err(error) => panic!("invalid create response json: {error}"),
    };

    let session_id = match created_session.get("id").and_then(|value| value.as_str()) {
        Some(session_id) => session_id,
        None => panic!("create response missing session id"),
    };

    let get_request = match Request::builder()
        .method("GET")
        .uri(format!("/v1/sessions/{session_id}"))
        .header(AUTHORIZATION, "Bearer secret")
        .body(Body::empty())
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build get request: {error}"),
    };

    let get_response = match app.oneshot(get_request).await {
        Ok(response) => response,
        Err(error) => panic!("get request should succeed: {error}"),
    };

    assert_eq!(get_response.status(), StatusCode::OK);

    let body = match to_bytes(get_response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read get response body: {error}"),
    };

    let parsed: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(error) => panic!("invalid get response json: {error}"),
    };

    assert_eq!(
        parsed
            .get("config")
            .and_then(|value| value.get("default_model")),
        Some(&json!("openai/test-model"))
    );
    assert_eq!(
        parsed
            .get("config")
            .and_then(|value| value.get("auto_approve_mode")),
        Some(&json!("custom"))
    );
}

#[tokio::test]
async fn delete_session_returns_no_content() {
    let app = match test_state().await {
        Ok(state) => router(state.clone(), AuthConfig::token("secret")),
        Err(error) => panic!("failed to create app state: {error}"),
    };

    let create_payload = json!({"title":"to-delete"});
    let create_request = match Request::builder()
        .method("POST")
        .uri("/v1/sessions")
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(create_payload.to_string()))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build create request: {error}"),
    };

    let create_response = match app.clone().oneshot(create_request).await {
        Ok(response) => response,
        Err(error) => panic!("create request should succeed: {error}"),
    };

    let create_body = match to_bytes(create_response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read create response body: {error}"),
    };

    let created_session: serde_json::Value = match serde_json::from_slice(&create_body) {
        Ok(value) => value,
        Err(error) => panic!("invalid create response json: {error}"),
    };

    let session_id = match created_session.get("id").and_then(|value| value.as_str()) {
        Some(session_id) => session_id,
        None => panic!("create response missing session id"),
    };

    let delete_request = match Request::builder()
        .method("DELETE")
        .uri(format!("/v1/sessions/{session_id}"))
        .header(AUTHORIZATION, "Bearer secret")
        .body(Body::empty())
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build delete request: {error}"),
    };

    let delete_response = match app.oneshot(delete_request).await {
        Ok(response) => response,
        Err(error) => panic!("delete request should succeed: {error}"),
    };

    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn create_session_is_idempotent_for_same_key_and_payload() {
    let app = match test_state().await {
        Ok(state) => router(state.clone(), AuthConfig::token("secret")),
        Err(error) => panic!("failed to create app state: {error}"),
    };

    let payload = json!({"title":"idem"}).to_string();

    let request_a = match Request::builder()
        .method("POST")
        .uri("/v1/sessions")
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .header("Idempotency-Key", "idem-key")
        .body(Body::from(payload.clone()))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build first request: {error}"),
    };

    let response_a = match app.clone().oneshot(request_a).await {
        Ok(response) => response,
        Err(error) => panic!("first request should succeed: {error}"),
    };
    assert_eq!(response_a.status(), StatusCode::CREATED);

    let body_a = match to_bytes(response_a.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read first body: {error}"),
    };

    let request_b = match Request::builder()
        .method("POST")
        .uri("/v1/sessions")
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .header("Idempotency-Key", "idem-key")
        .body(Body::from(payload))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build second request: {error}"),
    };

    let response_b = match app.oneshot(request_b).await {
        Ok(response) => response,
        Err(error) => panic!("second request should succeed: {error}"),
    };
    assert_eq!(response_b.status(), StatusCode::CREATED);

    let body_b = match to_bytes(response_b.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read second body: {error}"),
    };

    let json_a: serde_json::Value = match serde_json::from_slice(&body_a) {
        Ok(value) => value,
        Err(error) => panic!("first body should be valid json: {error}"),
    };
    let json_b: serde_json::Value = match serde_json::from_slice(&body_b) {
        Ok(value) => value,
        Err(error) => panic!("second body should be valid json: {error}"),
    };

    assert_eq!(json_a, json_b);
}

#[tokio::test]
async fn cancel_rejects_run_mismatch() {
    let state = match test_state().await {
        Ok(state) => state,
        Err(error) => panic!("failed to create app state: {error}"),
    };

    let app = router(state.clone(), AuthConfig::token("secret"));

    let create_payload = json!({"title":"run-mismatch"});
    let create_request = match Request::builder()
        .method("POST")
        .uri("/v1/sessions")
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(create_payload.to_string()))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build create request: {error}"),
    };

    let create_response = match app.clone().oneshot(create_request).await {
        Ok(response) => response,
        Err(error) => panic!("create request should succeed: {error}"),
    };

    let create_body = match to_bytes(create_response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read create body: {error}"),
    };

    let session_value: serde_json::Value = match serde_json::from_slice(&create_body) {
        Ok(value) => value,
        Err(error) => panic!("invalid create json: {error}"),
    };

    let session_id_str = match session_value.get("id").and_then(|value| value.as_str()) {
        Some(value) => value,
        None => panic!("missing session id"),
    };

    let session_id = match Uuid::parse_str(session_id_str) {
        Ok(id) => id,
        Err(error) => panic!("invalid session id: {error}"),
    };

    let active_run_id = match state
        .run_manager
        .start_run(session_id, |_run_id| async move {
            let (command_tx, _command_rx) = tokio::sync::mpsc::channel(8);
            Ok(crate::SessionHandle::new(
                command_tx,
                tokio_util::sync::CancellationToken::new(),
            ))
        })
        .await
    {
        Ok(run_id) => run_id,
        Err(error) => panic!("failed to start run: {error}"),
    };

    let mismatched_run_id = Uuid::new_v4();
    let cancel_payload = json!({"run_id": mismatched_run_id});
    let cancel_request = match Request::builder()
        .method("POST")
        .uri(format!("/v1/sessions/{session_id}/cancel"))
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(cancel_payload.to_string()))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build cancel request: {error}"),
    };

    let cancel_response = match app.oneshot(cancel_request).await {
        Ok(response) => response,
        Err(error) => panic!("cancel request should succeed: {error}"),
    };

    assert_eq!(cancel_response.status(), StatusCode::CONFLICT);

    let cancel_body = match to_bytes(cancel_response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read cancel body: {error}"),
    };
    let cancel_json: serde_json::Value = match serde_json::from_slice(&cancel_body) {
        Ok(value) => value,
        Err(error) => panic!("invalid cancel json: {error}"),
    };

    assert_eq!(cancel_json.get("code"), Some(&json!("run_mismatch")));

    let _ = state
        .run_manager
        .cancel_run(session_id, active_run_id)
        .await;
    let _ = state
        .run_manager
        .mark_run_finished(session_id, active_run_id, Ok(()))
        .await;
}

#[tokio::test]
async fn create_session_rejects_reused_idempotency_key_with_different_payload() {
    let app = match test_state().await {
        Ok(state) => router(state.clone(), AuthConfig::token("secret")),
        Err(error) => panic!("failed to create app state: {error}"),
    };

    let first_payload = json!({"title":"idem-a"}).to_string();
    let second_payload = json!({"title":"idem-b"}).to_string();

    let first_request = match Request::builder()
        .method("POST")
        .uri("/v1/sessions")
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .header("Idempotency-Key", "idem-key-conflict")
        .body(Body::from(first_payload))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build first request: {error}"),
    };

    let first_response = match app.clone().oneshot(first_request).await {
        Ok(response) => response,
        Err(error) => panic!("first request should succeed: {error}"),
    };
    assert_eq!(first_response.status(), StatusCode::CREATED);

    let second_request = match Request::builder()
        .method("POST")
        .uri("/v1/sessions")
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .header("Idempotency-Key", "idem-key-conflict")
        .body(Body::from(second_payload))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build second request: {error}"),
    };

    let second_response = match app.oneshot(second_request).await {
        Ok(response) => response,
        Err(error) => panic!("second request should succeed: {error}"),
    };
    assert_eq!(second_response.status(), StatusCode::CONFLICT);

    let second_body = match to_bytes(second_response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read second body: {error}"),
    };

    let second_json: serde_json::Value = match serde_json::from_slice(&second_body) {
        Ok(value) => value,
        Err(error) => panic!("invalid second json: {error}"),
    };

    assert_eq!(
        second_json.get("code"),
        Some(&json!("idempotency_key_reused"))
    );
}

#[tokio::test]
async fn steering_requires_active_run() {
    let app = match test_state().await {
        Ok(state) => router(state.clone(), AuthConfig::token("secret")),
        Err(error) => panic!("failed to create app state: {error}"),
    };

    let create_payload = json!({"title":"steering"});
    let create_request = match Request::builder()
        .method("POST")
        .uri("/v1/sessions")
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(create_payload.to_string()))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build create request: {error}"),
    };

    let create_response = match app.clone().oneshot(create_request).await {
        Ok(response) => response,
        Err(error) => panic!("create request should succeed: {error}"),
    };

    let create_body = match to_bytes(create_response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read create body: {error}"),
    };

    let create_json: serde_json::Value = match serde_json::from_slice(&create_body) {
        Ok(value) => value,
        Err(error) => panic!("invalid create json: {error}"),
    };

    let session_id = match create_json.get("id").and_then(|value| value.as_str()) {
        Some(value) => value,
        None => panic!("missing session id"),
    };

    let steering_payload = json!({
        "message": {
            "role": "user",
            "content": "steer"
        },
        "type": "steering",
        "run_id": Uuid::new_v4()
    });

    let steering_request = match Request::builder()
        .method("POST")
        .uri(format!("/v1/sessions/{session_id}/messages"))
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(steering_payload.to_string()))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build steering request: {error}"),
    };

    let steering_response = match app.oneshot(steering_request).await {
        Ok(response) => response,
        Err(error) => panic!("steering request should succeed: {error}"),
    };

    assert_eq!(steering_response.status(), StatusCode::CONFLICT);

    let steering_body = match to_bytes(steering_response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read steering body: {error}"),
    };

    let steering_json: serde_json::Value = match serde_json::from_slice(&steering_body) {
        Ok(value) => value,
        Err(error) => panic!("invalid steering json: {error}"),
    };

    assert_eq!(
        steering_json.get("code"),
        Some(&json!("session_not_running"))
    );
}

#[tokio::test]
async fn message_rejects_mismatched_run_id_when_active_run_exists() {
    let state = match test_state().await {
        Ok(state) => state,
        Err(error) => panic!("failed to create app state: {error}"),
    };

    let app = router(state.clone(), AuthConfig::token("secret"));

    let create_payload = json!({"title":"mismatch"});
    let create_request = match Request::builder()
        .method("POST")
        .uri("/v1/sessions")
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(create_payload.to_string()))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build create request: {error}"),
    };

    let create_response = match app.clone().oneshot(create_request).await {
        Ok(response) => response,
        Err(error) => panic!("create request should succeed: {error}"),
    };

    let create_body = match to_bytes(create_response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read create body: {error}"),
    };

    let create_json: serde_json::Value = match serde_json::from_slice(&create_body) {
        Ok(value) => value,
        Err(error) => panic!("invalid create json: {error}"),
    };

    let session_id = match create_json.get("id").and_then(|value| value.as_str()) {
        Some(value) => value,
        None => panic!("missing session id"),
    };

    let session_uuid = match Uuid::parse_str(session_id) {
        Ok(value) => value,
        Err(error) => panic!("invalid session id: {error}"),
    };

    let active_run_id = match state
        .run_manager
        .start_run(session_uuid, |_run_id| async move {
            let (command_tx, _command_rx) = tokio::sync::mpsc::channel(8);
            Ok(crate::SessionHandle::new(
                command_tx,
                tokio_util::sync::CancellationToken::new(),
            ))
        })
        .await
    {
        Ok(run_id) => run_id,
        Err(error) => panic!("failed to start run: {error}"),
    };

    let mismatch_payload = json!({
        "message": {
            "role": "user",
            "content": "hello"
        },
        "type": "message",
        "run_id": Uuid::new_v4()
    });

    let mismatch_request = match Request::builder()
        .method("POST")
        .uri(format!("/v1/sessions/{session_id}/messages"))
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(mismatch_payload.to_string()))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build mismatch request: {error}"),
    };

    let mismatch_response = match app.oneshot(mismatch_request).await {
        Ok(response) => response,
        Err(error) => panic!("mismatch request should succeed: {error}"),
    };

    assert_eq!(mismatch_response.status(), StatusCode::CONFLICT);

    let mismatch_body = match to_bytes(mismatch_response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read mismatch body: {error}"),
    };

    let mismatch_json: serde_json::Value = match serde_json::from_slice(&mismatch_body) {
        Ok(value) => value,
        Err(error) => panic!("invalid mismatch json: {error}"),
    };

    assert_eq!(mismatch_json.get("code"), Some(&json!("run_mismatch")));

    let _ = state
        .run_manager
        .cancel_run(session_uuid, active_run_id)
        .await;
    let _ = state
        .run_manager
        .mark_run_finished(session_uuid, active_run_id, Ok(()))
        .await;
}

#[tokio::test]
async fn pending_tools_endpoint_returns_pending_calls() {
    let state = match test_state().await {
        Ok(state) => state,
        Err(error) => panic!("failed to create app state: {error}"),
    };

    let app = router(state.clone(), AuthConfig::token("secret"));

    let create_payload = json!({"title":"pending"});
    let create_request = match Request::builder()
        .method("POST")
        .uri("/v1/sessions")
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(create_payload.to_string()))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build create request: {error}"),
    };

    let create_response = match app.clone().oneshot(create_request).await {
        Ok(response) => response,
        Err(error) => panic!("create request should succeed: {error}"),
    };

    let create_body = match to_bytes(create_response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read create body: {error}"),
    };

    let create_json: serde_json::Value = match serde_json::from_slice(&create_body) {
        Ok(value) => value,
        Err(error) => panic!("invalid create json: {error}"),
    };

    let session_id = match create_json.get("id").and_then(|value| value.as_str()) {
        Some(value) => value,
        None => panic!("missing session id"),
    };

    let session_uuid = match Uuid::parse_str(session_id) {
        Ok(value) => value,
        Err(error) => panic!("invalid session id: {error}"),
    };

    let run_id = Uuid::new_v4();
    state
        .set_pending_tools(
            session_uuid,
            run_id,
            vec![vac_agent_loop::ProposedToolCall {
                id: "tc_1".to_string(),
                name: "vac__view".to_string(),
                arguments: json!({"path":"README.md"}),
                metadata: None,
            }],
        )
        .await;

    let pending_request = match Request::builder()
        .method("GET")
        .uri(format!("/v1/sessions/{session_id}/tools/pending"))
        .header(AUTHORIZATION, "Bearer secret")
        .body(Body::empty())
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build pending request: {error}"),
    };

    let pending_response = match app.oneshot(pending_request).await {
        Ok(response) => response,
        Err(error) => panic!("pending request should succeed: {error}"),
    };

    assert_eq!(pending_response.status(), StatusCode::OK);

    let pending_body = match to_bytes(pending_response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read pending body: {error}"),
    };

    let pending_json: serde_json::Value = match serde_json::from_slice(&pending_body) {
        Ok(value) => value,
        Err(error) => panic!("invalid pending json: {error}"),
    };

    assert_eq!(pending_json.get("run_id"), Some(&json!(run_id)));
    assert_eq!(
        pending_json
            .get("tool_calls")
            .and_then(|value| value.as_array())
            .map(|array| array.len()),
        Some(1)
    );
}

#[tokio::test]
async fn model_switch_rejects_run_mismatch() {
    let state = match test_state().await {
        Ok(state) => state,
        Err(error) => panic!("failed to create app state: {error}"),
    };

    let app = router(state.clone(), AuthConfig::token("secret"));

    let create_payload = json!({"title":"model-switch"});
    let create_request = match Request::builder()
        .method("POST")
        .uri("/v1/sessions")
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(create_payload.to_string()))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build create request: {error}"),
    };

    let create_response = match app.clone().oneshot(create_request).await {
        Ok(response) => response,
        Err(error) => panic!("create request should succeed: {error}"),
    };

    let create_body = match to_bytes(create_response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read create body: {error}"),
    };

    let create_json: serde_json::Value = match serde_json::from_slice(&create_body) {
        Ok(value) => value,
        Err(error) => panic!("invalid create json: {error}"),
    };

    let session_id = match create_json.get("id").and_then(|value| value.as_str()) {
        Some(value) => value,
        None => panic!("missing session id"),
    };

    let session_uuid = match Uuid::parse_str(session_id) {
        Ok(value) => value,
        Err(error) => panic!("invalid session id: {error}"),
    };

    let active_run_id = match state
        .run_manager
        .start_run(session_uuid, |_run_id| async move {
            let (command_tx, _command_rx) = tokio::sync::mpsc::channel(8);
            Ok(crate::SessionHandle::new(
                command_tx,
                tokio_util::sync::CancellationToken::new(),
            ))
        })
        .await
    {
        Ok(run_id) => run_id,
        Err(error) => panic!("failed to start run: {error}"),
    };

    let switch_payload = json!({
        "run_id": Uuid::new_v4(),
        "model": "openai/test-model"
    });

    let switch_request = match Request::builder()
        .method("POST")
        .uri(format!("/v1/sessions/{session_id}/model"))
        .header(AUTHORIZATION, "Bearer secret")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(switch_payload.to_string()))
    {
        Ok(request) => request,
        Err(error) => panic!("failed to build switch request: {error}"),
    };

    let switch_response = match app.oneshot(switch_request).await {
        Ok(response) => response,
        Err(error) => panic!("switch request should succeed: {error}"),
    };

    assert_eq!(switch_response.status(), StatusCode::CONFLICT);

    let switch_body = match to_bytes(switch_response.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => panic!("failed to read switch body: {error}"),
    };

    let switch_json: serde_json::Value = match serde_json::from_slice(&switch_body) {
        Ok(value) => value,
        Err(error) => panic!("invalid switch json: {error}"),
    };

    assert_eq!(switch_json.get("code"), Some(&json!("run_mismatch")));

    let _ = state
        .run_manager
        .cancel_run(session_uuid, active_run_id)
        .await;
    let _ = state
        .run_manager
        .mark_run_finished(session_uuid, active_run_id, Ok(()))
        .await;
}

#[test]
fn parse_context_priority_honors_known_values() {
    // Critical is reserved for internal use; callers get High instead
    assert!(matches!(
        parse_context_priority(Some("critical")),
        ContextPriority::High
    ));
    assert!(matches!(
        parse_context_priority(Some("HIGH")),
        ContextPriority::High
    ));
    assert!(matches!(
        parse_context_priority(Some("normal")),
        ContextPriority::Normal
    ));
    assert!(matches!(
        parse_context_priority(Some("unknown")),
        ContextPriority::CallerSupplied
    ));
}

#[test]
fn map_caller_context_inputs_handles_none() {
    let mapped = map_caller_context_inputs(None);
    assert!(mapped.is_empty());
}

#[test]
fn map_caller_context_inputs_skips_empty_values() {
    let mapped = map_caller_context_inputs(Some(&[
        CallerContextInput {
            name: "watch_result".to_string(),
            content: "system check complete".to_string(),
            priority: Some("high".to_string()),
        },
        CallerContextInput {
            name: " ".to_string(),
            content: "ignored".to_string(),
            priority: None,
        },
        CallerContextInput {
            name: "ignored".to_string(),
            content: "   ".to_string(),
            priority: None,
        },
    ]));

    assert_eq!(mapped.len(), 1);
    assert_eq!(mapped[0].name, "watch_result");
    assert!(matches!(mapped[0].priority, ContextPriority::High));
}

#[test]
fn resolve_tool_approval_override_uses_default_when_missing() {
    let default = ToolApprovalPolicy::None;
    let resolved = resolve_tool_approval_override(None, &default);
    assert!(matches!(resolved, ToolApprovalPolicy::None));
}

#[test]
fn resolve_tool_approval_override_accepts_mode_all() {
    let default = ToolApprovalPolicy::None;
    let resolved = resolve_tool_approval_override(
        Some(&AutoApproveOverride::Mode("all".to_string())),
        &default,
    );
    assert!(matches!(resolved, ToolApprovalPolicy::All));
}

#[test]
fn resolve_tool_approval_override_accepts_mode_none() {
    let default = ToolApprovalPolicy::All;
    let resolved = resolve_tool_approval_override(
        Some(&AutoApproveOverride::Mode("none".to_string())),
        &default,
    );
    assert!(matches!(resolved, ToolApprovalPolicy::None));
}

#[test]
fn resolve_tool_approval_override_allowlist_normalizes_prefixed_names() {
    let default = ToolApprovalPolicy::None;
    let resolved = resolve_tool_approval_override(
        Some(&AutoApproveOverride::AllowList(vec![
            "vac__view".to_string(),
            " run_command ".to_string(),
            "".to_string(),
        ])),
        &default,
    );

    assert_eq!(
        resolved.action_for("view", None),
        ToolApprovalAction::Approve
    );
    assert_eq!(
        resolved.action_for("run_command", None),
        ToolApprovalAction::Approve
    );
    assert_eq!(resolved.action_for("create", None), ToolApprovalAction::Ask);
}

#[test]
fn resolve_tool_approval_override_unknown_mode_falls_back_to_default() {
    let default = ToolApprovalPolicy::Custom {
        rules: HashMap::from([("view".to_string(), ToolApprovalAction::Approve)]),
        default: ToolApprovalAction::Ask,
    };

    let resolved = resolve_tool_approval_override(
        Some(&AutoApproveOverride::Mode("unexpected".to_string())),
        &default,
    );

    assert_eq!(
        resolved.action_for("view", None),
        ToolApprovalAction::Approve
    );
    assert_eq!(
        resolved.action_for("run_command", None),
        ToolApprovalAction::Ask
    );
}

#[test]
fn validate_session_message_request_rejects_too_many_context_items() {
    let request = SessionMessageRequest {
        message: vac_provider_core::Message::new(vac_provider_core::Role::User, "hello"),
        r#type: SessionMessageType::Message,
        run_id: None,
        model: None,
        sandbox: None,
        context: Some(
            (0..(MAX_CALLER_CONTEXT_ITEMS + 1))
                .map(|idx| CallerContextInput {
                    name: format!("ctx-{idx}"),
                    content: "value".to_string(),
                    priority: None,
                })
                .collect(),
        ),
        overrides: None,
    };

    assert!(validate_session_message_request(&request).is_some());
}

#[test]
fn validate_session_message_request_rejects_oversized_context_name() {
    let request = SessionMessageRequest {
        message: vac_provider_core::Message::new(vac_provider_core::Role::User, "hello"),
        r#type: SessionMessageType::Message,
        run_id: None,
        model: None,
        sandbox: None,
        context: Some(vec![CallerContextInput {
            name: "n".repeat(MAX_CALLER_CONTEXT_NAME_CHARS + 1),
            content: "value".to_string(),
            priority: None,
        }]),
        overrides: None,
    };

    assert!(validate_session_message_request(&request).is_some());
}

#[test]
fn validate_session_message_request_rejects_oversized_whitespace_only_context_name() {
    let request = SessionMessageRequest {
        message: vac_provider_core::Message::new(vac_provider_core::Role::User, "hello"),
        r#type: SessionMessageType::Message,
        run_id: None,
        model: None,
        sandbox: None,
        context: Some(vec![CallerContextInput {
            name: " ".repeat(MAX_CALLER_CONTEXT_NAME_CHARS + 1),
            content: "value".to_string(),
            priority: None,
        }]),
        overrides: None,
    };

    assert!(
        validate_session_message_request(&request).is_some(),
        "raw name length must be enforced even when trimmed name is empty"
    );
}

#[test]
fn validate_session_message_request_rejects_oversized_trimmed_context_name() {
    let request = SessionMessageRequest {
        message: vac_provider_core::Message::new(vac_provider_core::Role::User, "hello"),
        r#type: SessionMessageType::Message,
        run_id: None,
        model: None,
        sandbox: None,
        context: Some(vec![CallerContextInput {
            name: format!(" {} ", "n".repeat(MAX_CALLER_CONTEXT_NAME_CHARS + 1)),
            content: "value".to_string(),
            priority: None,
        }]),
        overrides: None,
    };

    assert!(validate_session_message_request(&request).is_some());
}

#[test]
fn validate_session_message_request_rejects_oversized_context_content() {
    let request = SessionMessageRequest {
        message: vac_provider_core::Message::new(vac_provider_core::Role::User, "hello"),
        r#type: SessionMessageType::Message,
        run_id: None,
        model: None,
        sandbox: None,
        context: Some(vec![CallerContextInput {
            name: "ctx".to_string(),
            content: "x".repeat(MAX_CALLER_CONTEXT_CONTENT_CHARS + 1),
            priority: None,
        }]),
        overrides: None,
    };

    assert!(validate_session_message_request(&request).is_some());
}

#[test]
fn validate_session_message_request_rejects_oversized_whitespace_only_context_content() {
    let request = SessionMessageRequest {
        message: vac_provider_core::Message::new(vac_provider_core::Role::User, "hello"),
        r#type: SessionMessageType::Message,
        run_id: None,
        model: None,
        sandbox: None,
        context: Some(vec![CallerContextInput {
            name: "ctx".to_string(),
            content: " ".repeat(MAX_CALLER_CONTEXT_CONTENT_CHARS + 1),
            priority: None,
        }]),
        overrides: None,
    };

    assert!(
        validate_session_message_request(&request).is_some(),
        "raw content length must be enforced even when trimmed content is empty"
    );
}

#[test]
fn validate_session_message_request_rejects_out_of_bounds_max_turns() {
    let request = SessionMessageRequest {
        message: vac_provider_core::Message::new(vac_provider_core::Role::User, "hello"),
        r#type: SessionMessageType::Message,
        run_id: None,
        model: None,
        sandbox: None,
        context: None,
        overrides: Some(RunOverrides {
            max_turns: Some(0),
            ..RunOverrides::default()
        }),
    };

    assert!(validate_session_message_request(&request).is_some());

    let request_too_large = SessionMessageRequest {
        overrides: Some(RunOverrides {
            max_turns: Some(MAX_MAX_TURNS + 1),
            ..RunOverrides::default()
        }),
        ..request
    };
    assert!(validate_session_message_request(&request_too_large).is_some());
}

#[test]
fn validate_session_message_request_accepts_boundary_max_turns() {
    let request_min = SessionMessageRequest {
        message: vac_provider_core::Message::new(vac_provider_core::Role::User, "hello"),
        r#type: SessionMessageType::Message,
        run_id: None,
        model: None,
        sandbox: None,
        context: None,
        overrides: Some(RunOverrides {
            max_turns: Some(MIN_MAX_TURNS),
            ..RunOverrides::default()
        }),
    };
    assert!(validate_session_message_request(&request_min).is_none());

    let request_max = SessionMessageRequest {
        message: vac_provider_core::Message::new(vac_provider_core::Role::User, "hello"),
        r#type: SessionMessageType::Message,
        run_id: None,
        model: None,
        sandbox: None,
        context: None,
        overrides: Some(RunOverrides {
            max_turns: Some(MAX_MAX_TURNS),
            ..RunOverrides::default()
        }),
    };
    assert!(validate_session_message_request(&request_max).is_none());
}

#[test]
fn validate_session_message_request_rejects_oversized_system_prompt_override() {
    let request = SessionMessageRequest {
        message: vac_provider_core::Message::new(vac_provider_core::Role::User, "hello"),
        r#type: SessionMessageType::Message,
        run_id: None,
        model: None,
        sandbox: None,
        context: None,
        overrides: Some(RunOverrides {
            system_prompt: Some("x".repeat(MAX_SYSTEM_PROMPT_CHARS + 1)),
            ..RunOverrides::default()
        }),
    };

    assert!(validate_session_message_request(&request).is_some());
}

#[test]
fn validate_session_message_request_accepts_boundary_system_prompt_override() {
    let request = SessionMessageRequest {
        message: vac_provider_core::Message::new(vac_provider_core::Role::User, "hello"),
        r#type: SessionMessageType::Message,
        run_id: None,
        model: None,
        sandbox: None,
        context: None,
        overrides: Some(RunOverrides {
            system_prompt: Some("x".repeat(MAX_SYSTEM_PROMPT_CHARS)),
            ..RunOverrides::default()
        }),
    };

    assert!(validate_session_message_request(&request).is_none());
}

#[tokio::test]
async fn protected_endpoint_rejects_missing_token() {
    let app = match test_state().await {
        Ok(state) => router(state, AuthConfig::token("secret")),
        Err(error) => panic!("failed to create app state: {error}"),
    };

    let request = match Request::builder().uri("/v1/sessions").body(Body::empty()) {
        Ok(request) => request,
        Err(error) => panic!("failed to build request: {error}"),
    };

    let response = match app.oneshot(request).await {
        Ok(response) => response,
        Err(error) => panic!("request should succeed: {error}"),
    };

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
