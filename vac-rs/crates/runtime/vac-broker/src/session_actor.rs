use crate::{
    context::{ContextFile, EnvironmentContext, ProjectContext, SessionContextBuilder},
    message_bridge,
    sandbox::{SandboxConfig, SandboxMode, SandboxedMcpServer},
    state::AppState,
    types::{RunConfig, SessionHandle},
};
use async_trait::async_trait;
use rmcp::model::{
    CallToolRequestParam, CancelledNotification, CancelledNotificationMethod,
    CancelledNotificationParam, ServerResult,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{path::Path, sync::Arc};
use tokio::sync::{Mutex, mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use vac_agent_loop::{
    AgentCommand, AgentConfig, AgentEvent, AgentHook, AgentRunContext, BudgetAwareContextReducer,
    CheckpointEnvelopeV1, CloseoutState, CompactionConfig, PassthroughCompactionEngine, PlanStatus,
    ProposedToolCall, RetryConfig, RuntimeJournalCloseoutState, SemanticPlan, StopReason,
    ToolExecutionResult, ToolExecutor, VacRuntimeMetadataBootstrap, canonical_json_sha256,
    evaluate_completion_lock_v1_5, run_agent,
};
use vac_foundation::{
    runtime_journal_writer::{
        RuntimeJournalDecisionDraft, RuntimeJournalEvidenceHintDraft, RuntimeJournalSessionDraft,
        RuntimeJournalValidationSummaryDraft, RuntimeJournalWriter, RuntimeJournalWriterError,
    },
    utils::sanitize_text_output,
};
use vac_mcp_client::McpClient;
use vac_provider_core::{ContentPart, Message, MessageContent, Role};
use vac_remote_service::CreateCheckpointRequest;
use vac_state::{
    RuntimeJournalEventDraft, RuntimeJournalOpenRequest, RuntimeManifestBinding, RuntimeTrustClaim,
    TRUST_WORDING_LOCAL_SELF_REPORTED,
};

const CHECKPOINT_FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);
pub(crate) const ACTIVE_MODEL_METADATA_KEY: &str = "active_model";

pub fn build_run_context(session_id: Uuid, run_id: Uuid) -> AgentRunContext {
    AgentRunContext { run_id, session_id }
}

pub fn build_checkpoint_envelope(
    run_id: Uuid,
    messages: Vec<vac_provider_core::Message>,
    metadata: serde_json::Value,
) -> CheckpointEnvelopeV1 {
    CheckpointEnvelopeV1::new(Some(run_id), messages, metadata)
}

const VAC_RUNTIME_METADATA_KEY: &str = "vac_runtime";

async fn bootstrap_runtime_journal_product_path(
    workspace_root: &Path,
    session_id: &str,
    run_id: Uuid,
    metadata: &mut Value,
) -> Result<RuntimeJournalCloseoutState, String> {
    let manifest_binding = runtime_manifest_binding_from_workspace(workspace_root)?;
    let trust_claim = local_runtime_trust_claim();
    let writer = RuntimeJournalWriter::open(RuntimeJournalOpenRequest {
        workspace_id: workspace_id(workspace_root),
        db_path: workspace_root
            .join(".vac/db/runtime.db")
            .to_string_lossy()
            .to_string(),
        manifest_binding: manifest_binding.clone(),
        writer_id: runtime_journal_writer_id(run_id),
        session_id: session_id.to_string(),
        lease_reason: "VAC-managed broker session lifecycle".to_string(),
    })
    .await
    .map_err(runtime_journal_error)?;
    writer
        .acquire_writer_lease(0)
        .await
        .map_err(runtime_journal_error)?;

    let plan: SemanticPlan = metadata_runtime_value(metadata, "plan")?;
    let closeout: CloseoutState = metadata_runtime_value(metadata, "closeout")?;
    writer
        .ensure_session(&RuntimeJournalSessionDraft {
            status: "open".to_string(),
            user_prompt_summary: "VAC broker managed session".to_string(),
            current_phase: "intake".to_string(),
            default_trust_claim: trust_claim.clone(),
        })
        .await
        .map_err(runtime_journal_error)?;
    writer
        .append_event(RuntimeJournalEventDraft {
            session_id: session_id.to_string(),
            phase: "intake".to_string(),
            event_type: "runtime_journal_product_path_bootstrap".to_string(),
            severity: "info".to_string(),
            summary: "VAC broker attached runtime journal to managed session lifecycle".to_string(),
            manifest_binding: manifest_binding.clone(),
            payload_cbor_sha256: Some(canonical_json_sha256(metadata)),
            trust_claim_override: Some(trust_claim.clone()),
            proof_ref: None,
        })
        .await
        .map_err(runtime_journal_error)?;

    let plan_terminal = matches!(plan.status, PlanStatus::Completed);
    let validation_terminal = closeout.evidence.valid && closeout.artifacts.all_complete();
    writer
        .record_validation_summary(&RuntimeJournalValidationSummaryDraft {
            validation_id: format!("validation.{session_id}.closeout"),
            command_id: plan
                .validation_commands
                .first()
                .cloned()
                .unwrap_or_else(|| "vac.completion_lock.closeout".to_string()),
            structured_command_hash: canonical_json_sha256(&json!(plan.validation_commands)),
            exit_code: validation_terminal.then_some(0),
            stdout_hash: None,
            stderr_hash: None,
            duration_ms: None,
            status: if validation_terminal {
                "done"
            } else {
                "pending"
            }
            .to_string(),
        })
        .await
        .map_err(runtime_journal_error)?;
    writer
        .record_decision(&RuntimeJournalDecisionDraft {
            decision_id: format!("decision.{session_id}.manifest_refresh"),
            decision_class: "slice_local".to_string(),
            decision_type: "manifest_sync_refresh".to_string(),
            subject_type: "session".to_string(),
            subject_id: session_id.to_string(),
            decided_by: "vac-broker".to_string(),
            decision: "refreshed_current_manifest".to_string(),
            reason_summary:
                "Runtime journal product path refreshed session decision under current manifest set"
                    .to_string(),
            scope_hash: sha256_string(format!(
                "{}|{}|{}",
                session_id, plan.id, manifest_binding.manifest_set_hash
            )),
            policy_snapshot_hash: None,
            locked: true,
            proof_ref: None,
        })
        .await
        .map_err(runtime_journal_error)?;

    let evidence_summary_present = closeout.evidence.valid
        && closeout.evidence.self_hash.starts_with("sha256:")
        && !closeout.evidence.self_hash.trim().is_empty();
    if evidence_summary_present {
        writer
            .record_evidence_hint(&RuntimeJournalEvidenceHintDraft {
                evidence_id: format!("evidence.{session_id}.closeout"),
                capability_id: plan.capability.clone(),
                evidence_class: "completion_summary".to_string(),
                content_hash: closeout.evidence.self_hash.clone(),
                previous_hash: None,
                trust_claim: trust_claim.clone(),
                proof_ref: None,
            })
            .await
            .map_err(runtime_journal_error)?;
    }

    let counts = writer
        .product_projection_counts()
        .await
        .map_err(runtime_journal_error)?;
    let projection = RuntimeJournalCloseoutState {
        session_recorded: counts.sessions > 0,
        event_count: counts.events.try_into().unwrap_or(u32::MAX),
        plan_terminal,
        plan_manifest_current: true,
        validation_terminal: validation_terminal && counts.validation_summaries > 0,
        validation_manifest_current: true,
        decisions_locked_and_current: counts.decisions > 0,
        manifest_sync: vac_agent_loop::ManifestSyncCloseoutState::current(),
        evidence_summary_present: evidence_summary_present && counts.evidence_hints > 0,
        evidence_summary_trust_wording: evidence_summary_present
            .then(|| TRUST_WORDING_LOCAL_SELF_REPORTED.to_string()),
    };
    set_runtime_journal_projection(metadata, &projection)?;
    Ok(projection)
}

async fn finalize_runtime_journal_product_path(
    workspace_root: &Path,
    session_id: &str,
    run_id: Uuid,
    metadata: &Value,
    stop_reason: StopReason,
) -> Result<(), String> {
    let closeout: CloseoutState = metadata_runtime_value(metadata, "closeout")?;
    let completion = evaluate_completion_lock_v1_5(&closeout);
    let Some((status, phase, expected_heartbeat_counter)) = (match completion.disposition {
        vac_agent_loop::CompletionDisposition::Done
            if matches!(stop_reason, StopReason::Completed) =>
        {
            Some(("done", "done", 1))
        }
        vac_agent_loop::CompletionDisposition::PausedForDiscussion => {
            Some(("paused_for_operator", "paused_for_operator", 1))
        }
        _ => None,
    }) else {
        return Ok(());
    };

    let writer = RuntimeJournalWriter::open(RuntimeJournalOpenRequest {
        workspace_id: workspace_id(workspace_root),
        db_path: workspace_root
            .join(".vac/db/runtime.db")
            .to_string_lossy()
            .to_string(),
        manifest_binding: runtime_manifest_binding_from_workspace(workspace_root)?,
        writer_id: runtime_journal_writer_id(run_id),
        session_id: session_id.to_string(),
        lease_reason: "VAC-managed broker session closeout".to_string(),
    })
    .await
    .map_err(runtime_journal_error)?;
    writer
        .acquire_writer_lease(expected_heartbeat_counter)
        .await
        .map_err(runtime_journal_error)?;
    writer
        .close_session(status, phase)
        .await
        .map_err(runtime_journal_error)
}

fn metadata_runtime_value<T>(metadata: &Value, key: &str) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    metadata
        .get(VAC_RUNTIME_METADATA_KEY)
        .and_then(|runtime| runtime.get(key))
        .cloned()
        .ok_or_else(|| format!("vac runtime metadata missing {key}"))
        .and_then(|value| {
            serde_json::from_value(value)
                .map_err(|error| format!("invalid vac runtime metadata {key}: {error}"))
        })
}

fn set_runtime_journal_projection(
    metadata: &mut Value,
    projection: &RuntimeJournalCloseoutState,
) -> Result<(), String> {
    let closeout = metadata
        .get_mut(VAC_RUNTIME_METADATA_KEY)
        .and_then(|runtime| runtime.get_mut("closeout"))
        .ok_or_else(|| "vac runtime metadata missing closeout".to_string())?;
    let object = closeout
        .as_object_mut()
        .ok_or_else(|| "vac runtime closeout must be a JSON object".to_string())?;
    object.insert(
        "runtime_journal".to_string(),
        serde_json::to_value(projection)
            .map_err(|error| format!("cannot serialize runtime journal projection: {error}"))?,
    );
    Ok(())
}

fn runtime_manifest_binding_from_workspace(root: &Path) -> Result<RuntimeManifestBinding, String> {
    let (snapshot_hash, compiled_snapshot_id) = read_compiled_snapshot_identity(root)?;
    Ok(RuntimeManifestBinding {
        manifest_set_hash: snapshot_hash,
        compiled_snapshot_id,
        git_head: git_output(root, &["rev-parse", "HEAD"]),
        git_dirty_tree_hash: git_output(root, &["status", "--short", "--untracked-files=all"])
            .map(sha256_string),
    })
}

fn read_compiled_snapshot_identity(root: &Path) -> Result<(String, String), String> {
    for candidate in [
        ".vac/cache/compiled/workspace.json",
        ".vac/cache/compiled/runtime/current.json",
        ".vac/registry/compiled/workspace.json",
        ".vac/registry/compiled/runtime/current.json",
    ] {
        let path = root.join(candidate);
        if !path.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let value: Value =
            serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
        let snapshot_hash = value
            .get("snapshot_hash")
            .or_else(|| value.get("manifest_set_hash"))
            .or_else(|| value.pointer("/runtime_registry_snapshot/snapshot_hash"))
            .and_then(Value::as_str)
            .filter(|hash| hash.starts_with("sha256:") && !hash.trim().is_empty())
            .map(ToString::to_string);
        if let Some(snapshot_hash) = snapshot_hash {
            let compiled_snapshot_id = value
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| !id.trim().is_empty())
                .map(ToString::to_string)
                .unwrap_or_else(|| {
                    format!("snapshot.{}", snapshot_hash.trim_start_matches("sha256:"))
                });
            return Ok((snapshot_hash, compiled_snapshot_id));
        }
    }
    Err(
        "compiled snapshot hash missing; run `vac compile registry .` before VAC-managed session"
            .to_string(),
    )
}

fn workspace_id(root: &Path) -> String {
    root.canonicalize()
        .unwrap_or_else(|_| root.to_path_buf())
        .to_string_lossy()
        .to_string()
}

fn runtime_journal_writer_id(run_id: Uuid) -> String {
    format!("vac-broker.{run_id}")
}

fn local_runtime_trust_claim() -> RuntimeTrustClaim {
    RuntimeTrustClaim {
        execution: "observed_l1".to_string(),
        custody: "local_only".to_string(),
        proof_ref: None,
    }
}

fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    Some(text.trim().to_string())
}

fn sha256_string(value: impl AsRef<str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_ref().as_bytes());
    format!("sha256:{}", hex_lower(&hasher.finalize()))
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn runtime_journal_error(error: RuntimeJournalWriterError) -> String {
    format!("runtime journal product path failed: {error}")
}

pub fn spawn_session_actor(
    state: AppState,
    session_id: Uuid,
    run_id: Uuid,
    run_config: RunConfig,
    user_message: Message,
    caller_context: Vec<ContextFile>,
    sandbox_config: Option<SandboxConfig>,
) -> Result<SessionHandle, String> {
    let (command_tx, command_rx) = mpsc::channel(128);
    let cancel = CancellationToken::new();

    let handle = SessionHandle::new(command_tx, cancel.clone());

    let state_for_task = state.clone();
    tokio::spawn(async move {
        let actor_result = run_session_actor(
            state_for_task.clone(),
            session_id,
            run_id,
            run_config,
            user_message,
            caller_context,
            command_rx,
            cancel,
            sandbox_config,
        )
        .await;

        let finish_result = actor_result.map(|_| ());
        let _ = state_for_task
            .run_manager
            .mark_run_finished(session_id, run_id, finish_result)
            .await;
    });

    Ok(handle)
}

#[allow(clippy::too_many_arguments)]
async fn run_session_actor(
    state: AppState,
    session_id: Uuid,
    run_id: Uuid,
    run_config: RunConfig,
    mut user_message: Message,
    caller_context: Vec<ContextFile>,
    command_rx: mpsc::Receiver<AgentCommand>,
    cancel: CancellationToken,
    sandbox_config: Option<SandboxConfig>,
) -> Result<(), String> {
    let active_checkpoint = state
        .session_store
        .get_active_checkpoint(session_id)
        .await
        .ok();
    let parent_checkpoint_id = active_checkpoint.as_ref().map(|checkpoint| checkpoint.id);

    let (initial_messages, mut initial_metadata) =
        match state.checkpoint_store.load_latest(session_id).await {
            Ok(Some(envelope)) => (envelope.messages, envelope.metadata),
            Ok(None) => {
                let messages = active_checkpoint
                    .as_ref()
                    .map(|checkpoint| {
                        message_bridge::chat_to_provider_core(checkpoint.state.messages.clone())
                    })
                    .unwrap_or_default();
                let metadata = active_checkpoint
                    .as_ref()
                    .and_then(|checkpoint| checkpoint.state.metadata.clone())
                    .unwrap_or_else(|| json!({}));
                (messages, metadata)
            }
            Err(error) => {
                return Err(format!("Failed to load checkpoint envelope: {error}"));
            }
        };

    // If sandbox is requested, determine how to provide it based on the configured mode:
    // - Persistent: reuse the pre-spawned sandbox from AppState (no per-session overhead)
    // - Ephemeral: spawn a new sandbox container for this session
    //
    // `ephemeral_sandbox` holds the owned sandbox for ephemeral mode so we can
    // shut it down at the end. Persistent sandboxes are not owned by the session.
    let mut ephemeral_sandbox: Option<SandboxedMcpServer> = None;

    let (run_tools, tool_executor): (
        Vec<vac_provider_core::Tool>,
        Box<dyn ToolExecutor + Send + Sync>,
    ) = if let Some(ref sandbox_cfg) = sandbox_config {
        if let Some(ref persistent) = state.persistent_sandbox {
            // Persistent mode: reuse the pre-spawned sandbox
            tracing::info!(session_id = %session_id, "Using persistent sandbox for session");
            (
                persistent.tools().await,
                Box::new(SandboxedToolExecutor {
                    mcp_client: persistent.client().await,
                }),
            )
        } else if sandbox_cfg.mode == SandboxMode::Persistent {
            // Persistent mode was configured but the sandbox is not available.
            // This should not happen because the server hard-fails on startup
            // if the persistent sandbox cannot be spawned. Fail explicitly rather
            // than silently falling back to ephemeral mode.
            return Err(format!(
                "Sandbox mode is 'persistent' but no persistent sandbox is available for session {session_id}. \
                     This indicates the server started without a healthy sandbox. Restart the autopilot to fix."
            ));
        } else {
            // Ephemeral mode: spawn a new sandbox for this session
            tracing::info!(session_id = %session_id, image = %sandbox_cfg.image, "Spawning ephemeral sandbox container for session");
            let sandbox = SandboxedMcpServer::spawn(sandbox_cfg)
                .await
                .map_err(|e| format!("Failed to start sandbox for session {session_id}: {e}"))?;
            let tools = sandbox.tools.clone();
            let client = sandbox.client.clone();
            ephemeral_sandbox = Some(sandbox);
            (
                tools,
                Box::new(SandboxedToolExecutor { mcp_client: client }),
            )
        }
    } else {
        (
            state.current_mcp_tools().await,
            Box::new(ServerToolExecutor {
                state: state.clone(),
            }),
        )
    };

    let is_new_session = is_new_session_history(&initial_messages);
    let session_cwd = resolve_session_cwd(&state, session_id).await;
    let session_id_text = session_id.to_string();

    // VAC v1.9 operational closure: broker/session startup must attach
    // compiled-registry runtime authority, approved Semantic Plan JSON, mandatory
    // task/spec/todo artifacts, and closeout metadata before `run_agent`. Without
    // this bridge ordinary sessions would fail closed or drift into legacy MCP
    // execution without a VAC runtime contract.
    let vac_bootstrap = VacRuntimeMetadataBootstrap::new(&session_cwd);
    let bootstrap_report = vac_bootstrap
        .set_vac_runtime_metadata(&mut initial_metadata, &session_id_text)
        .map_err(|error| {
            format!("VAC runtime metadata bootstrap failed for session {session_id}: {error}")
        })?;
    if let Some(obj) = initial_metadata.as_object_mut() {
        obj.insert(
            "vac_runtime_bootstrap_report".to_string(),
            serde_json::to_value(bootstrap_report)
                .unwrap_or_else(|_| json!({"error":"bootstrap_report_serialize_failed"})),
        );
    }
    let journal_projection = bootstrap_runtime_journal_product_path(
        Path::new(&session_cwd),
        &session_id_text,
        run_id,
        &mut initial_metadata,
    )
    .await?;
    if let Some(obj) = initial_metadata.as_object_mut() {
        obj.insert(
            "vac_runtime_journal_projection".to_string(),
            serde_json::to_value(journal_projection)
                .unwrap_or_else(|_| json!({"error":"journal_projection_serialize_failed"})),
        );
    }

    let environment = EnvironmentContext::snapshot(&session_cwd).await;

    // Combine caller context with pre-loaded remote skills context from AppState.
    // Explicit caller context should force per-turn injection, even on resumed
    // sessions, while startup remote skills remain baseline context.
    let has_runtime_caller_context = !caller_context.is_empty();
    let mut all_caller_context = caller_context;
    all_caller_context.extend(state.current_skills().await);

    let project =
        ProjectContext::discover(Path::new(&session_cwd)).with_caller_context(all_caller_context);

    let session_context = SessionContextBuilder::new()
        .base_system_prompt(
            run_config
                .system_prompt
                .clone()
                .or_else(|| state.base_system_prompt.clone())
                .unwrap_or_default(),
        )
        .environment(environment)
        .project(project)
        .tools(&run_tools)
        .budget(state.context_budget.clone())
        .build();

    if (is_new_session || has_runtime_caller_context)
        && let Some(context_block) = session_context.user_context_block.as_deref()
    {
        user_message = prepend_context_to_user_message(user_message, context_block);
    }

    let mut baseline_messages = initial_messages.clone();
    baseline_messages.push(user_message.clone());

    let checkpoint_runtime = Arc::new(CheckpointRuntime::new(
        state.clone(),
        session_id,
        run_id,
        run_config.model.clone(),
        parent_checkpoint_id,
        baseline_messages,
        initial_metadata.clone(),
    ));

    checkpoint_runtime
        .persist_snapshot()
        .await
        .map_err(|error| format!("Failed to persist baseline checkpoint: {error}"))?;

    let periodic_checkpoint_cancel = CancellationToken::new();
    let periodic_checkpoint_runtime = checkpoint_runtime.clone();
    let periodic_checkpoint_cancel_task = periodic_checkpoint_cancel.clone();
    let periodic_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(CHECKPOINT_FLUSH_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = periodic_checkpoint_cancel_task.cancelled() => break,
                _ = interval.tick() => {
                    let _ = periodic_checkpoint_runtime.persist_snapshot().await;
                }
            }
        }
    });

    let (core_event_tx, mut core_event_rx) = mpsc::channel::<AgentEvent>(256);

    let event_state = state.clone();
    let event_forwarder = tokio::spawn(async move {
        while let Some(event) = core_event_rx.recv().await {
            handle_core_event(&event_state, session_id, run_id, event).await;
        }
    });

    // Use the model's maximum output capacity as the output budget for context
    // window calculations. This is conservative — the actual response may be shorter,
    // but reserving the full limit avoids mid-response context truncation.
    let max_output_tokens = run_config.model.limit.output as u32;
    let agent_config = AgentConfig {
        model: run_config.model.clone(),
        system_prompt: session_context.system_prompt,
        max_turns: run_config.max_turns,
        max_output_tokens,
        provider_options: None,
        tool_approval: run_config.tool_approval_policy.clone(),
        retry: RetryConfig::default(),
        compaction: CompactionConfig::default(),
        tools: run_tools,
    };

    let hooks: Vec<Box<dyn AgentHook>> = vec![Box::new(ServerCheckpointHook {
        checkpoint_runtime: checkpoint_runtime.clone(),
    })];

    let compactor = PassthroughCompactionEngine;
    let context_reducer = BudgetAwareContextReducer::new(5, 0.8);
    let run_context = build_run_context(session_id, run_id);

    let run_result = run_agent(
        run_context,
        run_config.inference.as_ref(),
        &agent_config,
        initial_messages,
        &mut initial_metadata,
        user_message,
        tool_executor.as_ref(),
        &hooks,
        core_event_tx,
        command_rx,
        cancel,
        &compactor,
        &context_reducer,
    )
    .await;

    periodic_checkpoint_cancel.cancel();
    let _ = periodic_task.await;

    // Shut down ephemeral sandbox container if one was started.
    // Persistent sandboxes are NOT shut down here — they live for the process lifetime.
    if let Some(sandbox) = ephemeral_sandbox {
        sandbox.shutdown().await;
    }

    state.clear_pending_tools(session_id, run_id).await;

    match &run_result {
        Ok(result) => {
            finalize_runtime_journal_product_path(
                Path::new(&session_cwd),
                &session_id_text,
                run_id,
                &result.metadata,
                result.stop_reason,
            )
            .await?;
            checkpoint_runtime.update_messages(&result.messages).await;
            checkpoint_runtime.update_metadata(&result.metadata).await;
            checkpoint_runtime
                .persist_snapshot()
                .await
                .map_err(|error| format!("Failed to persist terminal checkpoint: {error}"))?;
        }
        Err(_) => {
            checkpoint_runtime.update_metadata(&initial_metadata).await;
            let _ = checkpoint_runtime.persist_snapshot().await;
        }
    }

    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), event_forwarder).await;

    run_result
        .map(|_| ())
        .map_err(|error| format!("Agent run failed: {error}"))
}

fn is_new_session_history(messages: &[Message]) -> bool {
    !messages
        .iter()
        .any(|message| matches!(message.role, Role::User | Role::Assistant | Role::Tool))
}

async fn resolve_session_cwd(state: &AppState, session_id: Uuid) -> String {
    // 1. Session-specific cwd (set by API caller)
    if let Ok(session) = state.session_store.get_session(session_id).await
        && let Some(cwd) = session.cwd
        && !cwd.trim().is_empty()
    {
        return cwd;
    }

    // 2. Configured project directory (set at server startup, e.g. from `vac up`)
    if let Some(project_dir) = &state.project_dir {
        return project_dir.clone();
    }

    // 3. Process working directory
    std::env::current_dir()
        .ok()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string())
}

fn prepend_context_to_user_message(mut message: Message, context_block: &str) -> Message {
    if context_block.trim().is_empty() {
        return message;
    }

    match &mut message.content {
        MessageContent::Text(text) => {
            let existing = std::mem::take(text);
            *text = if existing.trim().is_empty() {
                context_block.to_string()
            } else {
                format!("{context_block}\n\n{existing}")
            };
        }
        MessageContent::Parts(parts) => {
            let mut prefixed = Vec::with_capacity(parts.len() + 1);
            prefixed.push(ContentPart::text(context_block));
            prefixed.append(parts);
            *parts = prefixed;
        }
    }

    message
}

async fn handle_core_event(state: &AppState, session_id: Uuid, run_id: Uuid, event: AgentEvent) {
    match &event {
        AgentEvent::ToolCallsProposed { tool_calls, .. } => {
            state
                .set_pending_tools(session_id, run_id, tool_calls.clone())
                .await;
        }
        AgentEvent::TurnCompleted { .. }
        | AgentEvent::RunCompleted { .. }
        | AgentEvent::RunError { .. } => {
            state.clear_pending_tools(session_id, run_id).await;
        }
        _ => {}
    }

    state.events.publish(session_id, Some(run_id), event).await;
}

#[derive(Clone)]
struct ServerToolExecutor {
    state: AppState,
}

#[async_trait]
impl ToolExecutor for ServerToolExecutor {
    async fn execute_tool_call(
        &self,
        run: &AgentRunContext,
        tool_call: &ProposedToolCall,
        cancel: &CancellationToken,
    ) -> Result<ToolExecutionResult, vac_agent_loop::AgentError> {
        Ok(execute_mcp_tool_call(&self.state, run.session_id, run.run_id, tool_call, cancel).await)
    }
}

/// Tool executor that routes calls through a per-session sandboxed MCP client.
#[derive(Clone)]
struct SandboxedToolExecutor {
    mcp_client: Arc<McpClient>,
}

#[async_trait]
impl ToolExecutor for SandboxedToolExecutor {
    async fn execute_tool_call(
        &self,
        run: &AgentRunContext,
        tool_call: &ProposedToolCall,
        cancel: &CancellationToken,
    ) -> Result<ToolExecutionResult, vac_agent_loop::AgentError> {
        Ok(execute_mcp_tool_call_with_client(
            &self.mcp_client,
            run.session_id,
            run.run_id,
            tool_call,
            cancel,
        )
        .await)
    }
}

struct CheckpointRuntime {
    state: AppState,
    session_id: Uuid,
    run_id: Uuid,
    active_model: vac_provider_core::Model,
    inner: Mutex<CheckpointRuntimeInner>,
}

struct CheckpointRuntimeInner {
    parent_checkpoint_id: Option<Uuid>,
    latest_messages: Vec<Message>,
    latest_metadata: serde_json::Value,
    last_persisted_signature: Option<String>,
    dirty: bool,
}

impl CheckpointRuntime {
    fn new(
        state: AppState,
        session_id: Uuid,
        run_id: Uuid,
        active_model: vac_provider_core::Model,
        parent_checkpoint_id: Option<Uuid>,
        latest_messages: Vec<Message>,
        latest_metadata: serde_json::Value,
    ) -> Self {
        Self {
            state,
            session_id,
            run_id,
            active_model,
            inner: Mutex::new(CheckpointRuntimeInner {
                parent_checkpoint_id,
                latest_messages,
                latest_metadata,
                last_persisted_signature: None,
                dirty: true,
            }),
        }
    }

    async fn update_messages(&self, messages: &[Message]) {
        let mut guard = self.inner.lock().await;
        guard.latest_messages = messages.to_vec();
        guard.dirty = true;
    }

    async fn update_metadata(&self, metadata: &serde_json::Value) {
        let mut guard = self.inner.lock().await;
        guard.latest_metadata = metadata.clone();
        guard.dirty = true;
    }

    async fn persist_snapshot(&self) -> Result<Uuid, String> {
        let mut guard = self.inner.lock().await;
        self.persist_if_needed(&mut guard).await
    }

    async fn persist_if_needed(&self, guard: &mut CheckpointRuntimeInner) -> Result<Uuid, String> {
        if !guard.dirty
            && let Some(checkpoint_id) = guard.parent_checkpoint_id
        {
            return Ok(checkpoint_id);
        }

        let signature = checkpoint_signature(&guard.latest_messages, &guard.latest_metadata)?;
        let changed = guard.last_persisted_signature.as_deref() != Some(signature.as_str());
        let should_persist = guard.parent_checkpoint_id.is_none() || (guard.dirty && changed);

        if !should_persist {
            guard.dirty = false;
            if let Some(checkpoint_id) = guard.parent_checkpoint_id {
                return Ok(checkpoint_id);
            }
        }

        let checkpoint_id = persist_checkpoint(
            &self.state,
            self.session_id,
            self.run_id,
            &self.active_model,
            guard.parent_checkpoint_id,
            &guard.latest_messages,
            &guard.latest_metadata,
        )
        .await?;

        guard.parent_checkpoint_id = Some(checkpoint_id);
        guard.last_persisted_signature = Some(signature);
        guard.dirty = false;

        Ok(checkpoint_id)
    }
}

struct ServerCheckpointHook {
    checkpoint_runtime: Arc<CheckpointRuntime>,
}

#[async_trait]
impl AgentHook for ServerCheckpointHook {
    async fn before_inference(
        &self,
        _run: &AgentRunContext,
        messages: &[Message],
        _model: &vac_provider_core::Model,
    ) -> Result<(), vac_agent_loop::AgentError> {
        self.checkpoint_runtime.update_messages(messages).await;
        Ok(())
    }

    async fn after_inference(
        &self,
        _run: &AgentRunContext,
        messages: &[Message],
        _model: &vac_provider_core::Model,
    ) -> Result<(), vac_agent_loop::AgentError> {
        self.checkpoint_runtime.update_messages(messages).await;
        Ok(())
    }

    async fn after_tool_execution(
        &self,
        _run: &AgentRunContext,
        _tool_call: &ProposedToolCall,
        messages: &[Message],
    ) -> Result<(), vac_agent_loop::AgentError> {
        self.checkpoint_runtime.update_messages(messages).await;
        Ok(())
    }

    async fn on_error(
        &self,
        _run: &AgentRunContext,
        _error: &vac_agent_loop::AgentError,
        messages: &[Message],
    ) -> Result<(), vac_agent_loop::AgentError> {
        self.checkpoint_runtime.update_messages(messages).await;
        let _ = self.checkpoint_runtime.persist_snapshot().await;
        Ok(())
    }
}

async fn execute_mcp_tool_call(
    state: &AppState,
    session_id: Uuid,
    run_id: Uuid,
    tool_call: &ProposedToolCall,
    cancel: &CancellationToken,
) -> ToolExecutionResult {
    let Some(mcp_client) = state.mcp_client.as_ref() else {
        return ToolExecutionResult::Completed {
            result: "MCP client is not initialized".to_string(),
            is_error: true,
        };
    };

    execute_mcp_tool_call_with_client(mcp_client, session_id, run_id, tool_call, cancel).await
}

async fn execute_mcp_tool_call_with_client(
    mcp_client: &McpClient,
    session_id: Uuid,
    run_id: Uuid,
    tool_call: &ProposedToolCall,
    cancel: &CancellationToken,
) -> ToolExecutionResult {
    let metadata = Some(serde_json::Map::from_iter([
        (
            "session_id".to_string(),
            serde_json::Value::String(session_id.to_string()),
        ),
        (
            "run_id".to_string(),
            serde_json::Value::String(run_id.to_string()),
        ),
        (
            "tool_call_id".to_string(),
            serde_json::Value::String(tool_call.id.clone()),
        ),
    ]));

    let arguments = match &tool_call.arguments {
        serde_json::Value::Object(map) => Some(map.clone()),
        serde_json::Value::Null => None,
        other => Some(serde_json::Map::from_iter([(
            "input".to_string(),
            other.clone(),
        )])),
    };

    let request_handle = match vac_mcp_client::call_tool(
        mcp_client,
        CallToolRequestParam {
            name: tool_call.name.clone().into(),
            arguments,
        },
        metadata,
    )
    .await
    {
        Ok(handle) => handle,
        Err(error) => {
            return ToolExecutionResult::Completed {
                result: format!("MCP tool call failed: {error}"),
                is_error: true,
            };
        }
    };

    let peer_for_cancel = request_handle.peer.clone();
    let request_id = request_handle.id.clone();

    tokio::select! {
        _ = cancel.cancelled() => {
            let notification = CancelledNotification {
                method: CancelledNotificationMethod,
                params: CancelledNotificationParam {
                    request_id,
                    reason: Some("user cancel".to_string()),
                },
                extensions: Default::default(),
            };

            let _ = peer_for_cancel.send_notification(notification.into()).await;
            ToolExecutionResult::Cancelled
        }
        server_result = request_handle.await_response() => {
            match server_result {
                Ok(ServerResult::CallToolResult(result)) => {
                    ToolExecutionResult::Completed {
                        result: render_call_tool_result(&result),
                        is_error: result.is_error.unwrap_or(false),
                    }
                }
                Ok(_) => ToolExecutionResult::Completed {
                    result: "Unexpected MCP response type".to_string(),
                    is_error: true,
                },
                Err(error) => ToolExecutionResult::Completed {
                    result: format!("MCP tool execution error: {error}"),
                    is_error: true,
                },
            }
        }
    }
}

fn render_call_tool_result(result: &rmcp::model::CallToolResult) -> String {
    let rendered = result
        .content
        .iter()
        .filter_map(|content| content.raw.as_text().map(|text| text.text.clone()))
        .collect::<Vec<_>>()
        .join("\n");

    if !rendered.is_empty() {
        return sanitize_text_output(&rendered);
    }

    if result.content.is_empty() {
        return "<empty tool result>".to_string();
    }

    "<non-text tool result omitted for safety>".to_string()
}

fn checkpoint_signature(
    messages: &[Message],
    metadata: &serde_json::Value,
) -> Result<String, String> {
    serde_json::to_string(&(messages, metadata))
        .map_err(|error| format!("Failed to serialize checkpoint messages: {error}"))
}

async fn persist_checkpoint(
    state: &AppState,
    session_id: Uuid,
    run_id: Uuid,
    active_model: &vac_provider_core::Model,
    parent_id: Option<Uuid>,
    messages: &[Message],
    metadata: &serde_json::Value,
) -> Result<Uuid, String> {
    // TODO(ahmed): Migrate server/session checkpoint storage to `Vec<vac_provider_core::Message>` directly
    // and remove the ChatMessage adapter conversion (`message_bridge::provider_core_to_chat`).
    let mut request = CreateCheckpointRequest::new(message_bridge::provider_core_to_chat(messages))
        .with_metadata(metadata.clone());

    if let Some(parent_id) = parent_id {
        request = request.with_parent(parent_id);
    }

    let checkpoint = state
        .session_store
        .create_checkpoint(session_id, &request)
        .await
        .map_err(|error| error.to_string())?;

    let mut envelope_metadata = if metadata.is_object() {
        metadata.clone()
    } else {
        json!({})
    };

    if let Some(obj) = envelope_metadata.as_object_mut() {
        obj.insert(
            "session_id".to_string(),
            serde_json::Value::String(session_id.to_string()),
        );
        obj.insert(
            "checkpoint_id".to_string(),
            serde_json::Value::String(checkpoint.id.to_string()),
        );
        obj.insert(
            ACTIVE_MODEL_METADATA_KEY.to_string(),
            serde_json::Value::String(format!("{}/{}", active_model.provider, active_model.id)),
        );
    }

    let envelope = build_checkpoint_envelope(run_id, messages.to_vec(), envelope_metadata);

    state
        .checkpoint_store
        .save_latest(session_id, &envelope)
        .await
        .map_err(|error| {
            format!(
                "Failed to persist checkpoint envelope for session {}: {}",
                session_id, error
            )
        })?;

    Ok(checkpoint.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::{CallToolResult, Content};
    use serde_json::json;
    use vac_provider_core::{ContentPart, Message, MessageContent, Role};

    #[test]
    fn run_id_is_not_regenerated_when_building_run_context() {
        let session_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();

        let run_context = build_run_context(session_id, run_id);

        assert_eq!(run_context.session_id, session_id);
        assert_eq!(run_context.run_id, run_id);
    }

    #[test]
    fn checkpoint_envelope_carries_same_run_id() {
        let run_id = Uuid::new_v4();
        let envelope = build_checkpoint_envelope(
            run_id,
            vec![Message::new(Role::User, "hello")],
            json!({"turn": 1}),
        );

        assert_eq!(envelope.run_id, Some(run_id));
    }

    #[test]
    fn render_call_tool_result_sanitizes_text_blocks() {
        let result = CallToolResult::success(vec![Content::text("ok\u{0007}done")]);

        assert_eq!(render_call_tool_result(&result), "okdone");
    }

    #[test]
    fn render_call_tool_result_omits_non_text_blocks() {
        let result = CallToolResult::success(vec![Content::image("dGVzdA==", "image/png")]);

        assert_eq!(
            render_call_tool_result(&result),
            "<non-text tool result omitted for safety>"
        );
    }

    #[test]
    fn checkpoint_signature_changes_when_messages_change() {
        let messages_a = vec![Message::new(Role::User, "hello")];
        let messages_b = vec![
            Message::new(Role::User, "hello"),
            Message::new(Role::Assistant, "hi"),
        ];

        let sig_a = checkpoint_signature(&messages_a, &json!({}))
            .unwrap_or_else(|error| panic!("signature failed: {error}"));
        let sig_b = checkpoint_signature(&messages_b, &json!({}))
            .unwrap_or_else(|error| panic!("signature failed: {error}"));

        assert_ne!(sig_a, sig_b);
    }

    #[test]
    fn checkpoint_signature_changes_when_metadata_changes() {
        let messages = vec![Message::new(Role::User, "hello")];

        let sig_a = checkpoint_signature(&messages, &json!({}))
            .unwrap_or_else(|error| panic!("signature failed: {error}"));
        let sig_b = checkpoint_signature(&messages, &json!({"trimmed_up_to_message_index": 5}))
            .unwrap_or_else(|error| panic!("signature failed: {error}"));

        assert_ne!(sig_a, sig_b);
    }

    #[test]
    fn is_new_session_empty_history() {
        assert!(is_new_session_history(&[]));
    }

    #[test]
    fn is_new_session_system_only() {
        let messages = vec![Message::new(Role::System, "you are an agent")];
        assert!(is_new_session_history(&messages));
    }

    #[test]
    fn is_not_new_session_with_user_message() {
        let messages = vec![Message::new(Role::User, "hello")];
        assert!(!is_new_session_history(&messages));
    }

    #[test]
    fn is_not_new_session_with_system_and_user() {
        let messages = vec![
            Message::new(Role::System, "system"),
            Message::new(Role::User, "hello"),
        ];
        assert!(!is_new_session_history(&messages));
    }

    #[test]
    fn is_not_new_session_with_assistant() {
        let messages = vec![Message::new(Role::Assistant, "hi there")];
        assert!(!is_new_session_history(&messages));
    }

    #[test]
    fn prepend_context_to_text_message() {
        let msg = Message::new(Role::User, "how do I deploy?");
        let result = prepend_context_to_user_message(msg, "<context>env info</context>");

        let text = result.text().unwrap_or_default();
        assert!(
            text.starts_with("<context>env info</context>"),
            "context should be prepended"
        );
        assert!(
            text.contains("how do I deploy?"),
            "original text should be preserved"
        );
    }

    #[test]
    fn prepend_context_to_empty_text_message() {
        let msg = Message::new(Role::User, "  ");
        let result = prepend_context_to_user_message(msg, "<context>env info</context>");

        let text = result.text().unwrap_or_default();
        assert_eq!(text, "<context>env info</context>");
    }

    #[test]
    fn prepend_context_to_parts_message() {
        let msg = Message {
            role: Role::User,
            content: MessageContent::Parts(vec![ContentPart::text("original text")]),
            name: None,
            provider_options: None,
        };
        let result = prepend_context_to_user_message(msg, "<context>env info</context>");

        if let MessageContent::Parts(parts) = &result.content {
            assert_eq!(parts.len(), 2, "should have context part + original part");
            if let ContentPart::Text { text, .. } = &parts[0] {
                assert_eq!(text, "<context>env info</context>");
            } else {
                panic!("first part should be text");
            }
        } else {
            panic!("expected Parts content");
        }
    }

    #[test]
    fn prepend_empty_context_is_noop() {
        let msg = Message::new(Role::User, "hello");
        let result = prepend_context_to_user_message(msg, "   ");

        assert_eq!(result.text().unwrap_or_default(), "hello");
    }
}
