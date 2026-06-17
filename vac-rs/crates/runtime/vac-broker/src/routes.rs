use crate::{
    auth::{AuthConfig, require_bearer},
    context::{ContextFile, ContextPriority},
    idempotency::{IdempotencyRequest, LookupResult, StoredResponse},
    message_bridge,
    session_actor::{ACTIVE_MODEL_METADATA_KEY, spawn_session_actor},
    state::AppState,
    types::{AutoApproveOverride, RunConfig, RunOverrides, SessionRuntimeState},
};
use async_stream::stream;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    middleware,
    response::{IntoResponse, Response, sse::Event, sse::KeepAlive, sse::Sse},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, convert::Infallible, time::Duration};
use tokio::sync::broadcast;
use uuid::Uuid;
use vac_agent_loop::{AgentCommand, AgentEvent, ToolDecision};
use vac_foundation::models::context::{CallerContextInput, validate_caller_context};
use vac_remote_service::{
    ListSessionsQuery, SessionStatus, StorageCreateSessionRequest, StorageUpdateSessionRequest,
};

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    uptime_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    sandbox: Option<SandboxStatusResponse>,
}

#[derive(Debug, Serialize)]
struct SandboxStatusResponse {
    mode: String,
    healthy: bool,
    consecutive_ok: u64,
    consecutive_failures: u64,
    last_ok: Option<String>,
    last_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct ApiErrorBody {
    error: String,
    code: String,
    request_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum RunState {
    Idle,
    Starting,
    Running,
    Failed,
}

#[derive(Debug, Serialize)]
struct RunStatusDto {
    state: RunState,
    #[serde(skip_serializing_if = "Option::is_none")]
    run_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
struct SessionDto {
    id: Uuid,
    title: String,
    cwd: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    run_status: RunStatusDto,
}

#[derive(Debug, Serialize)]
struct SessionsResponse {
    sessions: Vec<SessionDto>,
    total: usize,
}

#[derive(Debug, Deserialize, Serialize)]
struct CreateSessionBody {
    title: String,
    #[serde(default)]
    cwd: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct UpdateSessionBody {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    visibility: Option<vac_remote_service::SessionVisibility>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum SessionMessageType {
    #[default]
    Message,
    Steering,
    FollowUp,
}

#[derive(Debug, Deserialize, Serialize)]
struct SessionMessageRequest {
    message: vac_provider_core::Message,
    #[serde(default)]
    r#type: SessionMessageType,
    #[serde(default)]
    run_id: Option<Uuid>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    sandbox: Option<bool>,
    #[serde(default)]
    context: Option<Vec<CallerContextInput>>,
    #[serde(default)]
    overrides: Option<RunOverrides>,
}

#[derive(Debug, Serialize)]
struct SessionMessageResponse {
    run_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct ListSessionsParams {
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
    #[serde(default)]
    search: Option<String>,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListMessagesParams {
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    offset: Option<usize>,
}

#[derive(Debug, Serialize)]
struct SessionMessagesResponse {
    messages: Vec<vac_provider_core::Message>,
    total: usize,
}

#[derive(Debug, Serialize)]
struct SessionDetailResponse {
    session: SessionDto,
    config: ConfigResponse,
}

#[derive(Debug, Serialize)]
struct PendingToolsResponse {
    run_id: Option<Uuid>,
    tool_calls: Vec<vac_agent_loop::ProposedToolCall>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
enum DecisionAction {
    Accept,
    Reject,
    CustomResult,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct DecisionInput {
    action: DecisionAction,
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ToolDecisionRequest {
    run_id: Uuid,
    #[serde(flatten)]
    decision: DecisionInput,
}

#[derive(Debug, Deserialize, Serialize)]
struct ToolDecisionsRequest {
    run_id: Uuid,
    decisions: HashMap<String, DecisionInput>,
}

#[derive(Debug, Serialize)]
struct ToolDecisionResponse {
    accepted: bool,
    run_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize)]
struct CancelRequest {
    run_id: Uuid,
}

#[derive(Debug, Serialize)]
struct CancelResponse {
    cancelled: bool,
    run_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize)]
struct ModelSwitchRequest {
    run_id: Uuid,
    model: String,
}

#[derive(Debug, Serialize)]
struct ModelSwitchResponse {
    accepted: bool,
    run_id: Uuid,
    model: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum AutoApproveMode {
    None,
    All,
    Custom,
}

#[derive(Debug, Serialize)]
struct ConfigResponse {
    default_model: Option<String>,
    auto_approve_mode: AutoApproveMode,
}

#[derive(Debug, Serialize)]
struct ModelsResponse {
    models: Vec<vac_provider_core::Model>,
}

const DEFAULT_MAX_TURNS: usize = 64;
const MIN_MAX_TURNS: usize = 1;
const MAX_MAX_TURNS: usize = 256;
const MAX_SYSTEM_PROMPT_CHARS: usize = 32 * 1024;

pub fn router(state: AppState, auth: AuthConfig) -> Router {
    public_router()
        .merge(protected_router(auth))
        .with_state(state)
}

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/v1/health", get(health_handler))
        .route("/v1/openapi.json", get(openapi_handler))
}

pub fn protected_router(auth: AuthConfig) -> Router<AppState> {
    Router::new()
        .route(
            "/v1/sessions",
            get(list_sessions_handler).post(create_session_handler),
        )
        .route(
            "/v1/sessions/{id}",
            get(get_session_handler)
                .patch(update_session_handler)
                .delete(delete_session_handler),
        )
        .route(
            "/v1/sessions/{id}/messages",
            post(sessions_message_handler).get(get_session_messages_handler),
        )
        .route("/v1/sessions/{id}/events", get(session_events_handler))
        .route(
            "/v1/sessions/{id}/tools/pending",
            get(pending_tools_handler),
        )
        .route(
            "/v1/sessions/{id}/tools/{tool_call_id}/decision",
            post(tool_decision_handler),
        )
        .route(
            "/v1/sessions/{id}/tools/decisions",
            post(tool_decisions_handler),
        )
        .route(
            "/v1/sessions/{id}/tools/resolve",
            post(tool_decisions_handler),
        )
        .route("/v1/sessions/{id}/cancel", post(cancel_handler))
        .route("/v1/sessions/{id}/model", post(model_switch_handler))
        .route("/v1/models", get(models_handler))
        .route("/v1/config", get(config_handler))
        .route_layer(middleware::from_fn_with_state(auth, require_bearer))
}

async fn health_handler(State(state): State<AppState>) -> Json<HealthResponse> {
    let sandbox = state.persistent_sandbox.as_ref().map(|ps| {
        let h = ps.health();
        SandboxStatusResponse {
            mode: ps.mode().to_string(),
            healthy: h.healthy,
            consecutive_ok: h.consecutive_ok,
            consecutive_failures: h.consecutive_failures,
            last_ok: h.last_ok,
            last_error: h.last_error,
        }
    });

    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: state.uptime_seconds(),
        sandbox,
    })
}

async fn openapi_handler() -> Json<utoipa::openapi::OpenApi> {
    Json(crate::openapi::generate_openapi())
}

async fn list_sessions_handler(
    State(state): State<AppState>,
    Query(params): Query<ListSessionsParams>,
) -> Result<Json<SessionsResponse>, Response> {
    let mut query = ListSessionsQuery::new();

    if let Some(limit) = params.limit {
        query = query.with_limit(limit);
    }
    if let Some(offset) = params.offset {
        query = query.with_offset(offset);
    }
    if let Some(search) = params.search {
        query = query.with_search(search);
    }
    if let Some(status) = params.status {
        query.status = parse_status_param(&status);
    }

    let result = state
        .session_store
        .list_sessions(&query)
        .await
        .map_err(storage_error)?;

    let mut sessions = Vec::with_capacity(result.sessions.len());
    for summary in result.sessions {
        let run_status = state.run_manager.state(summary.id).await;
        sessions.push(SessionDto {
            id: summary.id,
            title: summary.title,
            cwd: summary.cwd,
            created_at: summary.created_at,
            updated_at: summary.updated_at,
            run_status: map_run_status(run_status),
        });
    }

    Ok(Json(SessionsResponse {
        total: sessions.len(),
        sessions,
    }))
}

async fn create_session_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateSessionBody>,
) -> Result<Response, Response> {
    let idempotency = prepare_idempotency(
        &state,
        &headers,
        "POST",
        "/v1/sessions".to_string(),
        serde_json::to_value(&body).unwrap_or_else(|_| json!({})),
    )
    .await?;

    if let Some(replayed) = idempotency_replay_response(idempotency.lookup_result.clone()) {
        return Ok(replayed);
    }

    let mut request = StorageCreateSessionRequest::new(body.title, Vec::new());
    if let Some(cwd) = body.cwd {
        request = request.with_cwd(cwd);
    }

    let created = state
        .session_store
        .create_session(&request)
        .await
        .map_err(storage_error)?;

    let session = state
        .session_store
        .get_session(created.session_id)
        .await
        .map_err(storage_error)?;

    let payload = SessionDto {
        id: session.id,
        title: session.title,
        cwd: session.cwd,
        created_at: session.created_at,
        updated_at: session.updated_at,
        run_status: map_run_status(SessionRuntimeState::Idle),
    };

    save_idempotency_response(&state, idempotency.request, StatusCode::CREATED, &payload).await;

    Ok((StatusCode::CREATED, Json(payload)).into_response())
}

async fn get_session_handler(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<SessionDetailResponse>, Response> {
    let session = state
        .session_store
        .get_session(session_id)
        .await
        .map_err(storage_error)?;

    let run_status = state.run_manager.state(session_id).await;

    Ok(Json(SessionDetailResponse {
        session: SessionDto {
            id: session.id,
            title: session.title,
            cwd: session.cwd,
            created_at: session.created_at,
            updated_at: session.updated_at,
            run_status: map_run_status(run_status),
        },
        config: runtime_config(&state),
    }))
}

async fn update_session_handler(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(body): Json<UpdateSessionBody>,
) -> Result<Json<SessionDetailResponse>, Response> {
    let mut request = StorageUpdateSessionRequest::new();
    if let Some(title) = body.title {
        request = request.with_title(title);
    }
    if let Some(visibility) = body.visibility {
        request = request.with_visibility(visibility);
    }

    let session = state
        .session_store
        .update_session(session_id, &request)
        .await
        .map_err(storage_error)?;

    let run_status = state.run_manager.state(session_id).await;

    Ok(Json(SessionDetailResponse {
        session: SessionDto {
            id: session.id,
            title: session.title,
            cwd: session.cwd,
            created_at: session.created_at,
            updated_at: session.updated_at,
            run_status: map_run_status(run_status),
        },
        config: runtime_config(&state),
    }))
}

async fn delete_session_handler(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<StatusCode, Response> {
    if let Some(run_id) = state.run_manager.active_run_id(session_id).await {
        let _ = state.run_manager.cancel_run(session_id, run_id).await;
    }

    state
        .session_store
        .delete_session(session_id)
        .await
        .map_err(storage_error)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn sessions_message_handler(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<SessionMessageRequest>,
) -> Result<Response, Response> {
    let _ = state
        .session_store
        .get_session(session_id)
        .await
        .map_err(storage_error)?;

    if let Some(error_response) = validate_session_message_request(&request) {
        return Err(error_response);
    }

    let idempotency = prepare_idempotency(
        &state,
        &headers,
        "POST",
        format!("/v1/sessions/{session_id}/messages"),
        serde_json::to_value(&request).unwrap_or_else(|_| json!({})),
    )
    .await?;

    if let Some(replayed) = idempotency_replay_response(idempotency.lookup_result.clone()) {
        return Ok(replayed);
    }

    let active_run_id = state.run_manager.active_run_id(session_id).await;

    match request.r#type {
        SessionMessageType::Message => {
            if let (Some(requested_run_id), Some(active_run_id)) = (request.run_id, active_run_id)
                && requested_run_id != active_run_id
            {
                return Err(api_error(
                    StatusCode::CONFLICT,
                    "run_mismatch",
                    "Provided run_id does not match active run",
                ));
            }

            let _ = state.refresh_mcp_tools().await;

            let overrides = request.overrides.as_ref();

            let requested_or_persisted_model = match overrides
                .and_then(|value| value.model.clone())
                .or_else(|| request.model.clone())
            {
                Some(model) => Some(model),
                None => load_persisted_model_for_session(&state, session_id).await,
            };

            let model = state
                .resolve_model(requested_or_persisted_model.as_deref())
                .ok_or_else(|| {
                    api_error(StatusCode::BAD_REQUEST, "invalid_model", "Unknown model")
                })?;

            let tool_approval_policy = resolve_tool_approval_override(
                overrides.and_then(|value| value.auto_approve.as_ref()),
                &state.tool_approval_policy,
            );

            let system_prompt_override = overrides
                .and_then(|value| value.system_prompt.clone())
                .filter(|value| !value.trim().is_empty());

            let max_turns = overrides
                .and_then(|value| value.max_turns)
                .unwrap_or(DEFAULT_MAX_TURNS)
                // Defense-in-depth: clamp after validation in case a code path
                // bypasses validate_session_message_request.
                .clamp(MIN_MAX_TURNS, MAX_MAX_TURNS);

            let run_config = RunConfig {
                model,
                inference: state.inference.clone(),
                tool_approval_policy,
                system_prompt: system_prompt_override,
                max_turns,
            };

            let caller_context = map_caller_context_inputs(request.context.as_deref());

            let state_for_spawn = state.clone();
            let message_for_spawn = request.message;
            let run_config_for_spawn = run_config.clone();
            let sandbox_config = if request.sandbox.unwrap_or(false) {
                state.sandbox_config.clone()
            } else {
                None
            };

            let run_id = state
                .run_manager
                .start_run(session_id, move |allocated_run_id| {
                    let state = state_for_spawn.clone();
                    let message = message_for_spawn.clone();
                    let run_config = run_config_for_spawn.clone();
                    let caller_context = caller_context.clone();
                    let sandbox_config = sandbox_config.clone();
                    async move {
                        spawn_session_actor(
                            state,
                            session_id,
                            allocated_run_id,
                            run_config,
                            message,
                            caller_context,
                            sandbox_config,
                        )
                    }
                })
                .await
                .map_err(run_manager_error)?;

            let payload = SessionMessageResponse { run_id };
            save_idempotency_response(&state, idempotency.request, StatusCode::OK, &payload).await;
            Ok((StatusCode::OK, Json(payload)).into_response())
        }
        SessionMessageType::Steering | SessionMessageType::FollowUp => {
            let Some(run_id) = request.run_id else {
                return Err(api_error(
                    StatusCode::CONFLICT,
                    "run_mismatch",
                    "run_id is required for steering/follow_up",
                ));
            };

            let text = extract_message_text(&request.message);

            let command = match request.r#type {
                SessionMessageType::Steering => AgentCommand::Steering(text),
                SessionMessageType::FollowUp => AgentCommand::FollowUp(text),
                SessionMessageType::Message => AgentCommand::FollowUp(String::new()),
            };

            state
                .run_manager
                .send_command(session_id, run_id, command)
                .await
                .map_err(run_manager_error)?;

            let payload = SessionMessageResponse { run_id };
            save_idempotency_response(&state, idempotency.request, StatusCode::OK, &payload).await;
            Ok((StatusCode::OK, Json(payload)).into_response())
        }
    }
}

async fn get_session_messages_handler(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Query(query): Query<ListMessagesParams>,
) -> Result<Json<SessionMessagesResponse>, Response> {
    let all_messages = match state.checkpoint_store.load_latest(session_id).await {
        Ok(Some(envelope)) => envelope.messages,
        Ok(None) => {
            let checkpoint = state
                .session_store
                .get_active_checkpoint(session_id)
                .await
                .map_err(storage_error)?;
            message_bridge::chat_to_provider_core(checkpoint.state.messages)
        }
        Err(error) => {
            return Err(api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "checkpoint_read_failed",
                &format!("Failed to read checkpoint envelope: {error}"),
            ));
        }
    };

    let total = all_messages.len();

    let offset = query.offset.unwrap_or(0).min(total);
    let limit = query.limit.unwrap_or(100);
    let end = offset.saturating_add(limit).min(total);

    let messages = all_messages[offset..end].to_vec();

    Ok(Json(SessionMessagesResponse { messages, total }))
}

async fn session_events_handler(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Response, Response> {
    let last_event_id = headers
        .get("Last-Event-ID")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());

    let subscription = state.events.subscribe(session_id, last_event_id).await;
    let replay = subscription.replay;
    let gap_detected = subscription.gap_detected;
    let mut live = subscription.live;

    let stream = stream! {
        if let Some(gap) = gap_detected {
            let data = serde_json::to_string(&gap)
                .unwrap_or_else(|_| "{\"error\":\"serialization_failed\"}".to_string());
            yield Ok::<Event, Infallible>(Event::default().event("gap_detected").data(data));
        }

        for envelope in replay {
            yield Ok::<Event, Infallible>(envelope_to_sse_event(envelope));
        }

        loop {
            match live.recv().await {
                Ok(envelope) => yield Ok::<Event, Infallible>(envelope_to_sse_event(envelope)),
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
            }
        }
    };

    Ok(Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keepalive"),
        )
        .into_response())
}

async fn pending_tools_handler(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<PendingToolsResponse>, Response> {
    let pending = state.pending_tools(session_id).await;

    Ok(Json(PendingToolsResponse {
        run_id: pending.as_ref().map(|value| value.run_id),
        tool_calls: pending.map(|value| value.tool_calls).unwrap_or_default(),
    }))
}

async fn tool_decision_handler(
    State(state): State<AppState>,
    Path((session_id, tool_call_id)): Path<(Uuid, String)>,
    headers: HeaderMap,
    Json(request): Json<ToolDecisionRequest>,
) -> Result<Response, Response> {
    let idempotency = prepare_idempotency(
        &state,
        &headers,
        "POST",
        format!("/v1/sessions/{session_id}/tools/{tool_call_id}/decision"),
        serde_json::to_value(&request).unwrap_or_else(|_| json!({})),
    )
    .await?;

    if let Some(replayed) = idempotency_replay_response(idempotency.lookup_result.clone()) {
        return Ok(replayed);
    }

    state
        .run_manager
        .send_command(
            session_id,
            request.run_id,
            AgentCommand::ResolveTool {
                tool_call_id,
                decision: map_decision(request.decision)
                    .map_err(DecisionMappingError::into_response)?,
            },
        )
        .await
        .map_err(run_manager_error)?;

    let payload = ToolDecisionResponse {
        accepted: true,
        run_id: request.run_id,
    };
    save_idempotency_response(&state, idempotency.request, StatusCode::OK, &payload).await;

    Ok((StatusCode::OK, Json(payload)).into_response())
}

async fn tool_decisions_handler(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ToolDecisionsRequest>,
) -> Result<Response, Response> {
    let idempotency = prepare_idempotency(
        &state,
        &headers,
        "POST",
        format!("/v1/sessions/{session_id}/tools/decisions"),
        serde_json::to_value(&request).unwrap_or_else(|_| json!({})),
    )
    .await?;

    if let Some(replayed) = idempotency_replay_response(idempotency.lookup_result.clone()) {
        return Ok(replayed);
    }

    let mut decisions = HashMap::new();
    for (tool_call_id, decision) in request.decisions {
        decisions.insert(
            tool_call_id,
            map_decision(decision).map_err(DecisionMappingError::into_response)?,
        );
    }

    state
        .run_manager
        .send_command(
            session_id,
            request.run_id,
            AgentCommand::ResolveTools { decisions },
        )
        .await
        .map_err(run_manager_error)?;

    let payload = ToolDecisionResponse {
        accepted: true,
        run_id: request.run_id,
    };
    save_idempotency_response(&state, idempotency.request, StatusCode::OK, &payload).await;

    Ok((StatusCode::OK, Json(payload)).into_response())
}

async fn cancel_handler(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CancelRequest>,
) -> Result<Response, Response> {
    let idempotency = prepare_idempotency(
        &state,
        &headers,
        "POST",
        format!("/v1/sessions/{session_id}/cancel"),
        serde_json::to_value(&request).unwrap_or_else(|_| json!({})),
    )
    .await?;

    if let Some(replayed) = idempotency_replay_response(idempotency.lookup_result.clone()) {
        return Ok(replayed);
    }

    state
        .run_manager
        .cancel_run(session_id, request.run_id)
        .await
        .map_err(run_manager_error)?;

    let payload = CancelResponse {
        cancelled: true,
        run_id: request.run_id,
    };
    save_idempotency_response(&state, idempotency.request, StatusCode::OK, &payload).await;

    Ok((StatusCode::OK, Json(payload)).into_response())
}

async fn model_switch_handler(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ModelSwitchRequest>,
) -> Result<Response, Response> {
    let idempotency = prepare_idempotency(
        &state,
        &headers,
        "POST",
        format!("/v1/sessions/{session_id}/model"),
        serde_json::to_value(&request).unwrap_or_else(|_| json!({})),
    )
    .await?;

    if let Some(replayed) = idempotency_replay_response(idempotency.lookup_result.clone()) {
        return Ok(replayed);
    }

    let model = state
        .resolve_model(Some(&request.model))
        .ok_or_else(|| api_error(StatusCode::BAD_REQUEST, "invalid_model", "Unknown model"))?;

    state
        .run_manager
        .send_command(session_id, request.run_id, AgentCommand::SwitchModel(model))
        .await
        .map_err(run_manager_error)?;

    let payload = ModelSwitchResponse {
        accepted: true,
        run_id: request.run_id,
        model: request.model,
    };
    save_idempotency_response(&state, idempotency.request, StatusCode::OK, &payload).await;

    Ok((StatusCode::OK, Json(payload)).into_response())
}

async fn models_handler(State(state): State<AppState>) -> Json<ModelsResponse> {
    Json(ModelsResponse {
        models: state.models.as_ref().clone(),
    })
}

async fn config_handler(State(state): State<AppState>) -> Json<ConfigResponse> {
    Json(runtime_config(&state))
}

async fn load_persisted_model_for_session(state: &AppState, session_id: Uuid) -> Option<String> {
    match state.checkpoint_store.load_latest(session_id).await {
        Ok(Some(envelope)) => envelope
            .metadata
            .get(ACTIVE_MODEL_METADATA_KEY)
            .and_then(serde_json::Value::as_str)
            .map(std::string::ToString::to_string),
        Ok(None) | Err(_) => None,
    }
}

fn runtime_config(state: &AppState) -> ConfigResponse {
    ConfigResponse {
        default_model: state
            .default_model
            .as_ref()
            .map(|model| format!("{}/{}", model.provider, model.id)),
        auto_approve_mode: match &state.tool_approval_policy {
            vac_agent_loop::ToolApprovalPolicy::None => AutoApproveMode::None,
            vac_agent_loop::ToolApprovalPolicy::All => AutoApproveMode::All,
            vac_agent_loop::ToolApprovalPolicy::Custom { .. } => AutoApproveMode::Custom,
        },
    }
}

fn resolve_tool_approval_override(
    override_value: Option<&AutoApproveOverride>,
    default: &vac_agent_loop::ToolApprovalPolicy,
) -> vac_agent_loop::ToolApprovalPolicy {
    let Some(override_value) = override_value else {
        return default.clone();
    };

    match override_value {
        AutoApproveOverride::Mode(mode) => match mode.trim().to_ascii_lowercase().as_str() {
            "all" => vac_agent_loop::ToolApprovalPolicy::All,
            "none" => vac_agent_loop::ToolApprovalPolicy::None,
            _ => default.clone(),
        },
        AutoApproveOverride::AllowList(tools) => vac_agent_loop::ToolApprovalPolicy::Custom {
            rules: std::collections::HashMap::new(),
            default: vac_agent_loop::ToolApprovalAction::Ask,
        }
        .with_overrides(tools.iter().filter_map(|tool| {
            let normalized = vac_agent_loop::strip_tool_prefix(tool).trim().to_string();
            if normalized.is_empty() {
                None
            } else {
                Some((normalized, vac_agent_loop::ToolApprovalAction::Approve))
            }
        })),
    }
}

fn parse_status_param(status: &str) -> Option<SessionStatus> {
    match status.to_ascii_uppercase().as_str() {
        "ACTIVE" => Some(SessionStatus::Active),
        "DELETED" => Some(SessionStatus::Deleted),
        _ => None,
    }
}

fn map_run_status(state: SessionRuntimeState) -> RunStatusDto {
    match state {
        SessionRuntimeState::Idle => RunStatusDto {
            state: RunState::Idle,
            run_id: None,
        },
        SessionRuntimeState::Starting { run_id } => RunStatusDto {
            state: RunState::Starting,
            run_id: Some(run_id),
        },
        SessionRuntimeState::Running { run_id, .. } => RunStatusDto {
            state: RunState::Running,
            run_id: Some(run_id),
        },
        SessionRuntimeState::Failed { .. } => RunStatusDto {
            state: RunState::Failed,
            run_id: None,
        },
    }
}

fn storage_error(error: vac_remote_service::StorageError) -> Response {
    match error {
        vac_remote_service::StorageError::NotFound(message) => {
            api_error(StatusCode::NOT_FOUND, "not_found", &message)
        }
        vac_remote_service::StorageError::InvalidRequest(message) => {
            api_error(StatusCode::BAD_REQUEST, "invalid_request", &message)
        }
        vac_remote_service::StorageError::Unauthorized(message) => {
            api_error(StatusCode::UNAUTHORIZED, "unauthorized", &message)
        }
        vac_remote_service::StorageError::RateLimited(message) => {
            api_error(StatusCode::TOO_MANY_REQUESTS, "rate_limited", &message)
        }
        vac_remote_service::StorageError::Connection(message) => {
            api_error(StatusCode::BAD_GATEWAY, "connection_error", &message)
        }
        vac_remote_service::StorageError::Internal(message) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &message,
        ),
    }
}

fn run_manager_error(error: crate::error::SessionManagerError) -> Response {
    match error {
        crate::error::SessionManagerError::SessionAlreadyRunning
        | crate::error::SessionManagerError::SessionStarting => api_error(
            StatusCode::CONFLICT,
            "session_already_running",
            "Session already has an active run",
        ),
        crate::error::SessionManagerError::SessionNotRunning
        | crate::error::SessionManagerError::CommandChannelClosed => api_error(
            StatusCode::CONFLICT,
            "session_not_running",
            "Session has no active run",
        ),
        crate::error::SessionManagerError::RunMismatch { .. } => api_error(
            StatusCode::CONFLICT,
            "run_mismatch",
            "Provided run_id does not match active run",
        ),
        crate::error::SessionManagerError::ActorStartupFailed(message) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "actor_startup_failed",
            &message,
        ),
    }
}

fn api_error(status: StatusCode, code: &str, message: &str) -> Response {
    let body = ApiErrorBody {
        error: message.to_string(),
        code: code.to_string(),
        request_id: format!("req_{}", Uuid::new_v4().simple()),
    };

    (status, Json(body)).into_response()
}

fn validate_session_message_request(request: &SessionMessageRequest) -> Option<Response> {
    if request.message.role != vac_provider_core::Role::User {
        return Some(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_message_role",
            "message.role must be 'user' for this endpoint",
        ));
    }

    if let Some(context_inputs) = request.context.as_ref()
        && let Err(message) = validate_caller_context(context_inputs)
    {
        return Some(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_context",
            &message,
        ));
    }

    if let Some(overrides) = request.overrides.as_ref() {
        if let Some(max_turns) = overrides.max_turns
            && !(MIN_MAX_TURNS..=MAX_MAX_TURNS).contains(&max_turns)
        {
            return Some(api_error(
                StatusCode::BAD_REQUEST,
                "invalid_overrides",
                "max_turns must be between 1 and 256",
            ));
        }

        if let Some(system_prompt) = overrides.system_prompt.as_ref()
            && system_prompt.chars().count() > MAX_SYSTEM_PROMPT_CHARS
        {
            return Some(api_error(
                StatusCode::BAD_REQUEST,
                "invalid_overrides",
                "system_prompt exceeds maximum length",
            ));
        }
    }

    None
}

fn map_caller_context_inputs(inputs: Option<&[CallerContextInput]>) -> Vec<ContextFile> {
    let mut files = Vec::new();

    for input in inputs.unwrap_or_default() {
        let name = input.name.trim();
        let content = input.content.trim();

        if name.is_empty() || content.is_empty() {
            continue;
        }

        let priority = parse_context_priority(input.priority.as_deref());
        files.push(ContextFile::new(
            name,
            format!("caller://{name}"),
            content,
            priority,
        ));
    }

    files
}

/// Map caller-supplied priority strings to `ContextPriority`.
/// `Critical` is reserved for internally-discovered files (e.g. AGENTS.md) and
/// cannot be set by external API callers — it maps to `High` instead.
fn parse_context_priority(input: Option<&str>) -> ContextPriority {
    match input.map(|value| value.trim().to_ascii_lowercase()) {
        Some(value) if value == "critical" || value == "high" => ContextPriority::High,
        Some(value) if value == "normal" => ContextPriority::Normal,
        _ => ContextPriority::CallerSupplied,
    }
}

fn extract_message_text(message: &vac_provider_core::Message) -> String {
    if let Some(text) = message.text() {
        return text;
    }

    match &message.content {
        vac_provider_core::MessageContent::Text(text) => text.clone(),
        vac_provider_core::MessageContent::Parts(parts) => parts
            .iter()
            .filter_map(|part| match part {
                vac_provider_core::ContentPart::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

#[derive(Debug, Clone, Copy)]
enum DecisionMappingError {
    MissingCustomResultContent,
}

impl DecisionMappingError {
    fn into_response(self) -> Response {
        match self {
            DecisionMappingError::MissingCustomResultContent => api_error(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "custom_result requires content",
            ),
        }
    }
}

fn map_decision(input: DecisionInput) -> Result<ToolDecision, DecisionMappingError> {
    match input.action {
        DecisionAction::Accept => Ok(ToolDecision::Accept),
        DecisionAction::Reject => Ok(ToolDecision::Reject),
        DecisionAction::CustomResult => {
            let Some(content) = input.content else {
                return Err(DecisionMappingError::MissingCustomResultContent);
            };
            Ok(ToolDecision::CustomResult { content })
        }
    }
}

fn event_name(event: &AgentEvent) -> &'static str {
    match event {
        AgentEvent::RunStarted { .. } => "run_started",
        AgentEvent::TurnStarted { .. } => "turn_started",
        AgentEvent::TurnCompleted { .. } => "turn_completed",
        AgentEvent::RunCompleted { .. } => "run_completed",
        AgentEvent::RunError { .. } => "run_error",
        AgentEvent::PausedForDiscussion { .. } => "paused_for_discussion",
        AgentEvent::TextDelta { .. } => "text_delta",
        AgentEvent::ThinkingDelta { .. } => "thinking_delta",
        AgentEvent::TextComplete { .. } => "text_complete",
        AgentEvent::ToolCallsProposed { .. } => "tool_calls_proposed",
        AgentEvent::WaitingForToolApproval { .. } => "waiting_for_tool_approval",
        AgentEvent::ToolExecutionStarted { .. } => "tool_execution_started",
        AgentEvent::ToolExecutionProgress { .. } => "tool_execution_progress",
        AgentEvent::ToolExecutionCompleted { .. } => "tool_execution_completed",
        AgentEvent::ToolRejected { .. } => "tool_rejected",
        AgentEvent::RetryAttempt { .. } => "retry_attempt",
        AgentEvent::CompactionStarted { .. } => "compaction_started",
        AgentEvent::CompactionCompleted { .. } => "compaction_completed",
        AgentEvent::UsageReport { .. } => "usage_report",
    }
}

fn envelope_to_sse_event(envelope: crate::event_log::EventEnvelope) -> Event {
    let event = event_name(&envelope.event);
    let id = envelope.id.to_string();
    let data = serde_json::to_string(&envelope)
        .unwrap_or_else(|_| "{\"error\":\"serialization_failed\"}".to_string());

    Event::default().id(id).event(event).data(data)
}

#[derive(Clone)]
struct IdempotencyCheck {
    request: Option<IdempotencyRequest>,
    lookup_result: Option<LookupResult>,
}

async fn prepare_idempotency(
    state: &AppState,
    headers: &HeaderMap,
    method: &str,
    path: String,
    body: serde_json::Value,
) -> Result<IdempotencyCheck, Response> {
    let key = headers
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());

    let Some(key) = key else {
        return Ok(IdempotencyCheck {
            request: None,
            lookup_result: None,
        });
    };

    let request = IdempotencyRequest::new(method, path, key, body);
    let lookup = state.idempotency.lookup(&request).await;

    if matches!(lookup, LookupResult::Conflict) {
        return Err(api_error(
            StatusCode::CONFLICT,
            "idempotency_key_reused",
            "Idempotency key was reused with different payload",
        ));
    }

    Ok(IdempotencyCheck {
        request: Some(request),
        lookup_result: Some(lookup),
    })
}

fn idempotency_replay_response(lookup: Option<LookupResult>) -> Option<Response> {
    let LookupResult::Replay(stored) = lookup? else {
        return None;
    };

    let status = match StatusCode::from_u16(stored.status_code) {
        Ok(status) => status,
        Err(_) => StatusCode::OK,
    };

    Some((status, Json(stored.body)).into_response())
}

async fn save_idempotency_response<T: Serialize>(
    state: &AppState,
    request: Option<IdempotencyRequest>,
    status: StatusCode,
    payload: &T,
) {
    let Some(request) = request else {
        return;
    };

    if let Ok(body) = serde_json::to_value(payload) {
        state
            .idempotency
            .save(&request, StoredResponse::new(status.as_u16(), body))
            .await;
    }
}

#[cfg(test)]
mod tests;
