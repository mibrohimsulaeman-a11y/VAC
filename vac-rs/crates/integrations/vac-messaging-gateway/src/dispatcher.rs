use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use chrono::Utc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use vac_agent_loop::ProposedToolCall;
use vac_provider_core::{Message, Role};

use vac_foundation::utils::truncate_chars_with_ellipsis;

use crate::{
    channels::{ApprovalButton, ButtonStyle, Channel},
    client::{
        AutoApproveOverride, CallerContextInput, MessageType, RunErrorPayload, RunOverrides,
        SendMessageOptions, ToolCallsProposedPayload, ToolDecisionAction, ToolDecisionInput,
        VACClient,
    },
    config::{ApprovalMode, ChannelOverrides},
    router::{RouterConfig, resolve_routing_key},
    store::{GatewayStore, SessionMapping},
    targeting::{render_title_template, target_key_from_inbound},
    types::{ChatType, DeliveryContext, InboundMessage, OutboundReply, PeerId},
};

pub trait RunOverrideResolver: Send + Sync {
    fn resolve_run_overrides(&self, profile_name: &str) -> Option<RunOverrides>;
}

#[derive(Default)]
struct NoopRunOverrideResolver;

impl RunOverrideResolver for NoopRunOverrideResolver {
    fn resolve_run_overrides(&self, _profile_name: &str) -> Option<RunOverrides> {
        None
    }
}

pub fn noop_run_override_resolver() -> Arc<dyn RunOverrideResolver> {
    Arc::new(NoopRunOverrideResolver)
}

pub struct Dispatcher {
    client: VACClient,
    channels: HashMap<String, Arc<dyn Channel>>,
    store: Arc<GatewayStore>,
    router_config: RouterConfig,
    // TODO: persist dispatcher state (active_runs, pending_queues, event_cursors) to store
    // for crash recovery. Current behavior relies on watch-side reconciler for eventual
    // consistency after gateway restart.
    active_runs: Mutex<HashMap<String, ActiveRun>>,
    pending_queues: Mutex<HashMap<String, Vec<QueuedMessage>>>,
    // Invariant: at most one pending approval batch per session.
    // The run stream pauses on `tool_calls_proposed` until decisions are submitted.
    pending_approvals: Mutex<HashMap<String, PendingApproval>>,
    event_cursors: Mutex<HashMap<String, u64>>,
    default_model: Option<String>,
    approval_mode: ApprovalMode,
    approval_allowlist: HashSet<String>,
    channel_overrides: HashMap<String, ChannelOverrides>,
    channel_profiles: HashMap<String, String>,
    override_resolver: Arc<dyn RunOverrideResolver>,
    title_template: String,
}

#[derive(Debug, Clone)]
struct ActiveRun {
    run_id: String,
    cancel: CancellationToken,
    approval_mode: ApprovalMode,
    approval_allowlist: HashSet<String>,
}

#[derive(Debug, Clone)]
struct PendingApproval {
    session_id: String,
    run_id: String,
    tool_calls: Vec<ProposedToolCall>,
    approval_id: String,
    prompt_message_id: String,
    channel_name: String,
    delivery: DeliveryContext,
    cursor: Option<u64>,
    timeout_seconds: Option<u64>,
    requested_at: Instant,
}

#[derive(Debug, Clone, Default)]
struct RunStartOptions {
    model: Option<String>,
    sandbox: Option<bool>,
    timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone)]
struct ApprovalNeededContext {
    session_id: String,
    run_id: String,
    tool_calls: Vec<ProposedToolCall>,
    auto_resolved_count: usize,
    delivery: DeliveryContext,
    cursor: Option<u64>,
    timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone)]
struct RejectAndResumeContext {
    session_id: String,
    run_id: String,
    tool_calls: Vec<ProposedToolCall>,
    delivery: DeliveryContext,
    cursor: Option<u64>,
    timeout_seconds: Option<u64>,
    reason: String,
}

#[derive(Debug, Clone)]
struct QueuedMessage {
    inbound: InboundMessage,
    text: String,
    run_options: RunStartOptions,
    context: Vec<CallerContextInput>,
}

#[derive(Debug)]
struct RunTaskResult {
    session_id: String,
    run_id: String,
    outcome: RunOutcome,
}

#[derive(Clone)]
struct RunContext {
    channels: HashMap<String, Arc<dyn Channel>>,
    delivery: DeliveryContext,
    session_id: String,
    run_id: String,
    timeout_seconds: Option<u64>,
}

#[derive(Debug)]
enum RunOutcome {
    Completed {
        cursor: Option<u64>,
    },
    Error {
        error: Option<RunErrorPayload>,
        cursor: Option<u64>,
    },
    Cancelled {
        cursor: Option<u64>,
    },
    StreamEnded {
        cursor: Option<u64>,
    },
    ApprovalNeeded {
        cursor: Option<u64>,
        session_id: String,
        run_id: String,
        tool_calls: Vec<ProposedToolCall>,
        auto_resolved_count: usize,
        delivery: DeliveryContext,
        timeout_seconds: Option<u64>,
    },
}

#[derive(Debug)]
struct ResolveApprovalError {
    message: String,
    decision_sent: bool,
}

impl Dispatcher {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        client: VACClient,
        channels: HashMap<String, Arc<dyn Channel>>,
        store: Arc<GatewayStore>,
        router_config: RouterConfig,
        default_model: Option<String>,
        approval_mode: ApprovalMode,
        approval_allowlist: Vec<String>,
        channel_overrides: HashMap<String, ChannelOverrides>,
        title_template: String,
    ) -> Self {
        Self {
            client,
            channels,
            store,
            router_config,
            active_runs: Mutex::new(HashMap::new()),
            pending_queues: Mutex::new(HashMap::new()),
            pending_approvals: Mutex::new(HashMap::new()),
            event_cursors: Mutex::new(HashMap::new()),
            default_model,
            approval_mode,
            approval_allowlist: approval_allowlist.into_iter().collect(),
            channel_overrides,
            channel_profiles: HashMap::new(),
            override_resolver: Arc::new(NoopRunOverrideResolver),
            title_template,
        }
    }

    pub fn with_profile_resolution(
        mut self,
        channel_profiles: HashMap<String, String>,
        override_resolver: Arc<dyn RunOverrideResolver>,
    ) -> Self {
        self.channel_profiles = channel_profiles;
        self.override_resolver = override_resolver;
        self
    }

    pub async fn run(
        self: Arc<Self>,
        mut inbound_rx: mpsc::Receiver<InboundMessage>,
        cancel: CancellationToken,
    ) {
        let (run_tx, mut run_rx) = mpsc::channel::<RunTaskResult>(128);

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    self.cancel_all_runs().await;
                    break;
                }
                maybe_inbound = inbound_rx.recv() => {
                    let Some(inbound) = maybe_inbound else {
                        break;
                    };
                    if let Err(error) = self.handle_inbound(inbound, run_tx.clone()).await {
                        error!(error = %error, "failed to handle inbound message");
                    }
                }
                maybe_result = run_rx.recv() => {
                    let Some(result) = maybe_result else {
                        continue;
                    };
                    if let Err(error) = self.handle_run_result(result, run_tx.clone()).await {
                        error!(error = %error, "failed to handle run result");
                    }
                }
            }
        }
    }

    async fn handle_inbound(
        self: &Arc<Self>,
        inbound: InboundMessage,
        run_tx: mpsc::Sender<RunTaskResult>,
    ) -> Result<(), String> {
        if inbound
            .metadata
            .get("type")
            .and_then(|value| value.as_str())
            == Some("approval_response")
        {
            self.handle_approval_response(inbound, run_tx).await?;
            return Ok(());
        }

        let routing_key = resolve_routing_key(
            &self.router_config,
            &inbound.channel,
            &inbound.peer_id,
            &inbound.chat_type,
        );

        let target_key = target_key_from_inbound(&inbound);
        let caller_context = match self
            .store
            .pop_delivery_context(&inbound.channel.0, &target_key)
            .await
        {
            Ok(Some(context)) => delivery_context_to_caller_context(&context),
            Ok(None) => Vec::new(),
            Err(error) => {
                warn!(error = %error, "failed to pop delivery context");
                Vec::new()
            }
        };

        let maybe_mapping = self
            .store
            .get(&routing_key)
            .await
            .map_err(|error| format!("failed to get mapping: {error}"))?;

        let mapping = if let Some(mapping) = maybe_mapping {
            let delivery = self.delivery_context_from_inbound(&inbound);
            if let Err(error) = self.store.update_delivery(&routing_key, &delivery).await {
                warn!(error = %error, "failed to update delivery context");
            }
            mapping
        } else {
            let title = self.render_title(&inbound);
            let created = self
                .client
                .create_session(&title)
                .await
                .map_err(|error| format!("create session failed: {error}"))?;

            let now = Utc::now().timestamp_millis();
            let mapping = SessionMapping {
                session_id: created.id.to_string(),
                title,
                delivery: self.delivery_context_from_inbound(&inbound),
                created_at: now,
            };

            self.store
                .set(&routing_key, &mapping)
                .await
                .map_err(|error| format!("failed to persist mapping: {error}"))?;

            mapping
        };

        let run_options = extract_run_options(&inbound.metadata);
        let queued = QueuedMessage {
            text: inbound.text.clone(),
            run_options,
            context: caller_context,
            inbound,
        };

        if self.is_run_active(&mapping.session_id) {
            self.enqueue_message(mapping.session_id.clone(), queued)?;
            self.reject_pending_approval_for_session(&mapping.session_id, run_tx)
                .await?;
            return Ok(());
        }

        self.start_run(mapping.session_id, queued, run_tx).await
    }

    async fn handle_approval_response(
        self: &Arc<Self>,
        inbound: InboundMessage,
        run_tx: mpsc::Sender<RunTaskResult>,
    ) -> Result<(), String> {
        let approval_id = inbound
            .metadata
            .get("approval_id")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        let decision = inbound
            .metadata
            .get("decision")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();

        if approval_id.is_empty() || !matches!(decision.as_str(), "allow" | "deny") {
            warn!(
                approval_id = %approval_id,
                decision = %decision,
                "ignoring invalid approval callback"
            );
            return Ok(());
        }

        let pending = {
            let mut guard = self
                .pending_approvals
                .lock()
                .map_err(|_| "failed to lock pending_approvals".to_string())?;

            let session_id = guard.iter().find_map(|(session_id, pending)| {
                if pending.approval_id == approval_id {
                    Some(session_id.clone())
                } else {
                    None
                }
            });

            session_id.and_then(|session_id| guard.remove(&session_id))
        };

        let Some(pending) = pending else {
            warn!(
                approval_id = %approval_id,
                "approval already resolved or expired"
            );
            return Ok(());
        };

        debug!(
            approval_id = %approval_id,
            decision = %decision,
            resolved_by = %inbound.peer_id.0,
            "received approval response"
        );

        let approve = decision == "allow";
        if let Err(error) = self
            .resolve_approval(&pending, approve, &inbound.peer_id, run_tx)
            .await
        {
            if !error.decision_sent
                && let Ok(mut guard) = self.pending_approvals.lock()
            {
                guard.insert(pending.session_id.clone(), pending);
            }
            return Err(error.message);
        }

        Ok(())
    }

    async fn handle_run_result(
        self: &Arc<Self>,
        result: RunTaskResult,
        run_tx: mpsc::Sender<RunTaskResult>,
    ) -> Result<(), String> {
        match result.outcome {
            RunOutcome::ApprovalNeeded {
                cursor,
                session_id,
                run_id,
                tool_calls,
                auto_resolved_count,
                delivery,
                timeout_seconds,
            } => {
                if let Some(cursor) = cursor {
                    self.set_cursor(&session_id, cursor)?;
                }

                self.handle_approval_needed(
                    ApprovalNeededContext {
                        session_id,
                        run_id,
                        tool_calls,
                        auto_resolved_count,
                        delivery,
                        cursor,
                        timeout_seconds,
                    },
                    run_tx,
                )
                .await
            }
            RunOutcome::Error { error, cursor } => {
                if let Some(error) = error
                    && let Some(message) = error.error
                {
                    warn!(session_id = %result.session_id, run_id = %result.run_id, error = %message, "run failed");
                }

                self.remove_active_run(&result.session_id, &result.run_id);

                if let Some(cursor) = cursor {
                    self.set_cursor(&result.session_id, cursor)?;
                }

                self.drain_queue(&result.session_id, run_tx).await
            }
            RunOutcome::Completed { cursor }
            | RunOutcome::Cancelled { cursor }
            | RunOutcome::StreamEnded { cursor } => {
                self.remove_active_run(&result.session_id, &result.run_id);

                if let Some(cursor) = cursor {
                    self.set_cursor(&result.session_id, cursor)?;
                }

                self.drain_queue(&result.session_id, run_tx).await
            }
        }
    }

    async fn handle_approval_needed(
        self: &Arc<Self>,
        approval: ApprovalNeededContext,
        run_tx: mpsc::Sender<RunTaskResult>,
    ) -> Result<(), String> {
        let ApprovalNeededContext {
            session_id,
            run_id,
            tool_calls,
            auto_resolved_count,
            delivery,
            cursor,
            timeout_seconds,
        } = approval;

        // NOTE: TOCTOU between this early check and final insert is intentional.
        // We can't hold `std::sync::Mutex` across `.await` while posting the prompt.
        // The second check before insert preserves the "one pending approval per session"
        // invariant and marks any raced duplicate prompt as ignored.
        {
            let guard = self
                .pending_approvals
                .lock()
                .map_err(|_| "failed to lock pending_approvals".to_string())?;
            if let Some(existing) = guard.get(&session_id) {
                warn!(
                    session_id = %session_id,
                    existing_approval_id = %existing.approval_id,
                    "ignoring duplicate approval request; only one pending approval is allowed per session"
                );
                return Ok(());
            }
        }

        let channel_name = delivery.channel.0.clone();
        let Some(channel) = self.channels.get(&channel_name) else {
            warn!(channel = %channel_name, "approval channel not connected; auto-rejecting tools");
            self.reject_tools_and_resume(
                RejectAndResumeContext {
                    session_id,
                    run_id,
                    tool_calls,
                    delivery,
                    cursor,
                    timeout_seconds,
                    reason: "Cancelled — approval channel unavailable".to_string(),
                },
                run_tx,
            )
            .await?;
            return Ok(());
        };

        let approval_id = generate_approval_id();
        let text = render_approval_prompt(&tool_calls, auto_resolved_count);
        let button_label_suffix = if tool_calls.len() == 1 { "" } else { " All" };
        let buttons = vec![
            ApprovalButton {
                label: format!("Allow{button_label_suffix}"),
                callback_data: format!("a:{approval_id}:allow"),
                style: ButtonStyle::Success,
            },
            ApprovalButton {
                label: format!("Deny{button_label_suffix}"),
                callback_data: format!("a:{approval_id}:deny"),
                style: ButtonStyle::Danger,
            },
        ];

        let reply = OutboundReply {
            channel: delivery.channel.clone(),
            peer_id: delivery.peer_id.clone(),
            chat_type: delivery.chat_type.clone(),
            text,
            metadata: delivery.channel_meta.clone(),
        };

        let prompt_message_id = match channel.send_with_buttons(reply, buttons).await {
            Ok(message_id) => message_id,
            Err(error) => {
                warn!(error = %error, "failed to send approval prompt; auto-rejecting tools");
                self.reject_tools_and_resume(
                    RejectAndResumeContext {
                        session_id,
                        run_id,
                        tool_calls,
                        delivery,
                        cursor,
                        timeout_seconds,
                        reason: "Cancelled — failed to post approval prompt".to_string(),
                    },
                    run_tx,
                )
                .await?;
                return Ok(());
            }
        };

        let pending = PendingApproval {
            session_id: session_id.clone(),
            run_id,
            tool_calls,
            approval_id,
            // Keep a local copy for duplicate-race fallback edit_message path below.
            prompt_message_id: prompt_message_id.clone(),
            channel_name,
            delivery,
            cursor,
            timeout_seconds,
            requested_at: Instant::now(),
        };

        let has_pending = {
            let mut guard = self
                .pending_approvals
                .lock()
                .map_err(|_| "failed to lock pending_approvals".to_string())?;

            if let Some(existing) = guard.get(&session_id) {
                warn!(
                    session_id = %session_id,
                    existing_approval_id = %existing.approval_id,
                    "approval prompt sent but session already has a pending approval; dropping duplicate"
                );
                true
            } else {
                guard.insert(session_id, pending);
                false
            }
        };

        if has_pending {
            if let Err(error) = channel
                .edit_message(
                    &prompt_message_id,
                    "⏭️ Ignored — another approval is already pending",
                )
                .await
            {
                warn!(error = %error, "failed to edit duplicate approval prompt");
            }
            return Ok(());
        }

        Ok(())
    }

    async fn reject_pending_approval_for_session(
        self: &Arc<Self>,
        session_id: &str,
        run_tx: mpsc::Sender<RunTaskResult>,
    ) -> Result<(), String> {
        let pending = {
            let mut guard = self
                .pending_approvals
                .lock()
                .map_err(|_| "failed to lock pending_approvals".to_string())?;
            guard.remove(session_id)
        };

        let Some(pending) = pending else {
            return Ok(());
        };

        info!(
            session_id = %pending.session_id,
            run_id = %pending.run_id,
            tool_count = pending.tool_calls.len(),
            "auto-rejecting pending approval: new message received"
        );

        let decisions = build_decisions_for_tool_calls(
            &pending.tool_calls,
            ToolDecisionAction::Reject,
            Some("Cancelled — new message received"),
        );

        if let Err(error) = self
            .client
            .resolve_tools(&pending.session_id, &pending.run_id, decisions)
            .await
        {
            if let Ok(mut guard) = self.pending_approvals.lock() {
                guard.insert(pending.session_id.clone(), pending);
            }
            return Err(format!("resolve_tools failed: {error}"));
        }

        if let Some(channel) = self.channels.get(&pending.channel_name)
            && let Err(error) = channel
                .edit_message(
                    &pending.prompt_message_id,
                    "⏭️ Tools skipped — new message received",
                )
                .await
        {
            warn!(error = %error, "failed to edit approval prompt after auto-reject");
        }

        self.resume_run_after_approval(
            &pending.session_id,
            &pending.run_id,
            &pending.delivery,
            pending.cursor,
            remaining_timeout_after_approval(
                pending.timeout_seconds,
                pending.requested_at.elapsed(),
            ),
            run_tx,
        )
    }

    async fn resolve_approval(
        self: &Arc<Self>,
        pending: &PendingApproval,
        approve: bool,
        resolved_by: &PeerId,
        run_tx: mpsc::Sender<RunTaskResult>,
    ) -> Result<(), ResolveApprovalError> {
        if matches!(pending.delivery.chat_type, ChatType::Direct)
            && pending.delivery.peer_id != *resolved_by
        {
            return Err(ResolveApprovalError {
                message: "approval responder does not match the direct-chat requester".to_string(),
                decision_sent: false,
            });
        }

        // For group/thread chats we currently allow non-requesters to resolve approvals,
        // but emit an audit warning with both identities.
        if pending.delivery.peer_id != *resolved_by {
            warn!(
                session_id = %pending.session_id,
                run_id = %pending.run_id,
                requested_by = %pending.delivery.peer_id.0,
                resolved_by = %resolved_by.0,
                "approval resolved by a different user than the original requester"
            );
        }

        let action = if approve {
            ToolDecisionAction::Accept
        } else {
            ToolDecisionAction::Reject
        };

        let decisions = build_decisions_for_tool_calls(&pending.tool_calls, action, None);

        if let Err(error) = self
            .client
            .resolve_tools(&pending.session_id, &pending.run_id, decisions)
            .await
        {
            let is_conflict = matches!(error, crate::client::ClientError::Conflict);

            if is_conflict
                && let Some(channel) = self.channels.get(&pending.channel_name)
                && let Err(edit_error) = channel
                    .edit_message(
                        &pending.prompt_message_id,
                        "⏱️ Approval expired — run no longer waiting for decision",
                    )
                    .await
            {
                warn!(error = %edit_error, "failed to edit expired approval prompt");
            }

            return Err(ResolveApprovalError {
                message: format!("resolve_tools failed: {error}"),
                // Avoid re-inserting stale approvals when server rejects with 409.
                decision_sent: is_conflict,
            });
        }

        if let Some(channel) = self.channels.get(&pending.channel_name) {
            let resolved_by_display = render_approver_display(&pending.channel_name, resolved_by);
            let status = if approve {
                format!(
                    "✅ {} tool(s) approved by {}",
                    pending.tool_calls.len(),
                    resolved_by_display
                )
            } else {
                format!(
                    "❌ {} tool(s) denied by {}",
                    pending.tool_calls.len(),
                    resolved_by_display
                )
            };

            if let Err(error) = channel
                .edit_message(&pending.prompt_message_id, &status)
                .await
            {
                warn!(error = %error, "failed to edit approval prompt after resolution");
            }
        }

        self.resume_run_after_approval(
            &pending.session_id,
            &pending.run_id,
            &pending.delivery,
            pending.cursor,
            remaining_timeout_after_approval(
                pending.timeout_seconds,
                pending.requested_at.elapsed(),
            ),
            run_tx,
        )
        .map_err(|error| ResolveApprovalError {
            message: error,
            decision_sent: true,
        })
    }

    async fn reject_tools_and_resume(
        self: &Arc<Self>,
        request: RejectAndResumeContext,
        run_tx: mpsc::Sender<RunTaskResult>,
    ) -> Result<(), String> {
        let RejectAndResumeContext {
            session_id,
            run_id,
            tool_calls,
            delivery,
            cursor,
            timeout_seconds,
            reason,
        } = request;

        info!(
            session_id = %session_id,
            run_id = %run_id,
            tool_count = tool_calls.len(),
            reason = %reason,
            "auto-rejecting tools and resuming run"
        );

        let decisions = build_decisions_for_tool_calls(
            &tool_calls,
            ToolDecisionAction::Reject,
            Some(reason.as_str()),
        );

        self.client
            .resolve_tools(&session_id, &run_id, decisions)
            .await
            .map_err(|error| format!("resolve_tools failed: {error}"))?;

        self.resume_run_after_approval(
            &session_id,
            &run_id,
            &delivery,
            cursor,
            timeout_seconds,
            run_tx,
        )
    }

    fn spawn_run_consumer(
        self: &Arc<Self>,
        run_context: RunContext,
        last_event_id: Option<u64>,
        approval_mode: ApprovalMode,
        approval_allowlist: HashSet<String>,
        cancel: CancellationToken,
        run_tx: mpsc::Sender<RunTaskResult>,
    ) {
        let client = self.client.clone();
        let session_id_for_task = run_context.session_id.clone();
        let run_id_for_task = run_context.run_id.clone();

        tokio::spawn(async move {
            let outcome = consume_run_events(
                client,
                run_context,
                last_event_id,
                approval_mode,
                approval_allowlist,
                cancel,
            )
            .await;

            if let Err(error) = run_tx
                .send(RunTaskResult {
                    session_id: session_id_for_task,
                    run_id: run_id_for_task,
                    outcome,
                })
                .await
            {
                error!(error = %error, "failed to send run outcome");
            }
        });
    }

    async fn start_run(
        self: &Arc<Self>,
        session_id: String,
        queued: QueuedMessage,
        run_tx: mpsc::Sender<RunTaskResult>,
    ) -> Result<(), String> {
        let channel_name = queued.inbound.channel.0.clone();
        let run_overrides = self.build_run_overrides(&channel_name);
        let (run_approval_mode, run_approval_allowlist) =
            self.resolve_run_approval(&channel_name, run_overrides.as_ref());

        let top_level_model = if run_overrides
            .as_ref()
            .and_then(|overrides| overrides.model.as_ref())
            .is_some()
        {
            None
        } else {
            self.resolve_effective_model(&channel_name, queued.run_options.model.clone())
        };

        let message = Message::new(Role::User, queued.text.clone());
        let response = self
            .client
            .send_messages(
                &session_id,
                vec![message],
                SendMessageOptions {
                    model: top_level_model,
                    message_type: MessageType::Message,
                    run_id: None,
                    sandbox: queued.run_options.sandbox,
                    context: queued.context.clone(),
                    overrides: run_overrides,
                },
            )
            .await;

        let response = match response {
            Ok(response) => response,
            Err(crate::client::ClientError::Conflict) => {
                self.enqueue_message(session_id, queued)?;
                return Ok(());
            }
            Err(error) => return Err(format!("send message failed: {error}")),
        };

        let run_id = response.run_id.to_string();
        let cancel = CancellationToken::new();

        {
            let mut guard = self
                .active_runs
                .lock()
                .map_err(|_| "failed to lock active_runs".to_string())?;
            guard.insert(
                session_id.clone(),
                ActiveRun {
                    run_id: run_id.clone(),
                    cancel: cancel.clone(),
                    approval_mode: run_approval_mode.clone(),
                    approval_allowlist: run_approval_allowlist.clone(),
                },
            );
        }

        let run_context = RunContext {
            channels: self.channels.clone(),
            delivery: self.delivery_context_from_inbound(&queued.inbound),
            session_id: session_id.clone(),
            run_id,
            timeout_seconds: queued.run_options.timeout_seconds,
        };

        let last_event_id = self.get_cursor(&session_id)?;
        self.spawn_run_consumer(
            run_context,
            last_event_id,
            run_approval_mode,
            run_approval_allowlist,
            cancel,
            run_tx,
        );

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn resume_run_after_approval(
        self: &Arc<Self>,
        session_id: &str,
        run_id: &str,
        delivery: &DeliveryContext,
        cursor: Option<u64>,
        timeout_seconds: Option<u64>,
        run_tx: mpsc::Sender<RunTaskResult>,
    ) -> Result<(), String> {
        let (cancel, run_approval_mode, run_approval_allowlist) = {
            let guard = self
                .active_runs
                .lock()
                .map_err(|_| "failed to lock active_runs".to_string())?;
            guard
                .get(session_id)
                .and_then(|active| {
                    if active.run_id == run_id {
                        Some((
                            active.cancel.clone(),
                            active.approval_mode.clone(),
                            active.approval_allowlist.clone(),
                        ))
                    } else {
                        None
                    }
                })
                .ok_or_else(|| "run is no longer active".to_string())?
        };

        let run_context = RunContext {
            channels: self.channels.clone(),
            delivery: delivery.clone(),
            session_id: session_id.to_string(),
            run_id: run_id.to_string(),
            timeout_seconds,
        };

        self.spawn_run_consumer(
            run_context,
            cursor,
            run_approval_mode,
            run_approval_allowlist,
            cancel,
            run_tx,
        );

        Ok(())
    }

    async fn drain_queue(
        self: &Arc<Self>,
        session_id: &str,
        run_tx: mpsc::Sender<RunTaskResult>,
    ) -> Result<(), String> {
        let queue = {
            let mut guard = self
                .pending_queues
                .lock()
                .map_err(|_| "failed to lock pending_queues".to_string())?;
            guard.remove(session_id).unwrap_or_default()
        };

        if queue.is_empty() {
            return Ok(());
        }

        let combined_text = format_batched_queue_messages(&queue);

        // Keep only the latest caller context snapshot to avoid breaching
        // context item limits during long queue drains.
        let combined_context = latest_non_empty_context(&queue);

        let latest = &queue[queue.len() - 1];
        let routing_key = resolve_routing_key(
            &self.router_config,
            &latest.inbound.channel,
            &latest.inbound.peer_id,
            &latest.inbound.chat_type,
        );
        let delivery = self.delivery_context_from_inbound(&latest.inbound);
        if let Err(error) = self.store.update_delivery(&routing_key, &delivery).await {
            warn!(error = %error, "failed to refresh delivery context from queue");
        }

        let queued = QueuedMessage {
            inbound: latest.inbound.clone(),
            text: combined_text,
            run_options: latest.run_options.clone(),
            context: combined_context,
        };

        if let Err(error) = self.start_run(session_id.to_string(), queued, run_tx).await {
            self.restore_queue(session_id.to_string(), queue)?;
            return Err(error);
        }

        Ok(())
    }

    fn render_title(&self, inbound: &InboundMessage) -> String {
        render_title_template(
            &self.title_template,
            &inbound.channel.0,
            &inbound.peer_id.0,
            &inbound.chat_type,
        )
    }

    fn delivery_context_from_inbound(&self, inbound: &InboundMessage) -> DeliveryContext {
        DeliveryContext {
            channel: inbound.channel.clone(),
            peer_id: inbound.peer_id.clone(),
            chat_type: inbound.chat_type.clone(),
            channel_meta: inbound.metadata.clone(),
            updated_at: Utc::now().timestamp_millis(),
        }
    }

    pub fn is_run_active(&self, session_id: &str) -> bool {
        self.active_runs
            .lock()
            .ok()
            .and_then(|guard| guard.get(session_id).cloned())
            .is_some()
    }

    fn enqueue_message(&self, session_id: String, message: QueuedMessage) -> Result<(), String> {
        let mut guard = self
            .pending_queues
            .lock()
            .map_err(|_| "failed to lock pending_queues".to_string())?;

        guard.entry(session_id).or_default().push(message);
        Ok(())
    }

    fn restore_queue(&self, session_id: String, drained: Vec<QueuedMessage>) -> Result<(), String> {
        let mut guard = self
            .pending_queues
            .lock()
            .map_err(|_| "failed to lock pending_queues".to_string())?;

        let entry = guard.entry(session_id).or_default();
        let existing = std::mem::take(entry);
        *entry = merge_drained_queue(drained, existing);

        Ok(())
    }

    fn remove_active_run(&self, session_id: &str, run_id: &str) {
        if let Ok(mut guard) = self.active_runs.lock()
            && let Some(active) = guard.get(session_id)
            && active.run_id == run_id
        {
            guard.remove(session_id);
        }
    }

    async fn cancel_all_runs(&self) {
        if let Ok(guard) = self.active_runs.lock() {
            for active in guard.values() {
                active.cancel.cancel();
            }
        }

        let pending_approvals = if let Ok(mut guard) = self.pending_approvals.lock() {
            std::mem::take(&mut *guard)
        } else {
            HashMap::new()
        };

        if pending_approvals.is_empty() {
            return;
        }

        const SHUTDOWN_REJECT_TIMEOUT: Duration = Duration::from_secs(5);
        const SHUTDOWN_REJECT_CONCURRENCY: usize = 8;

        let client = self.client.clone();
        let mut set = tokio::task::JoinSet::new();

        for pending in pending_approvals.into_values() {
            while set.len() >= SHUTDOWN_REJECT_CONCURRENCY {
                if let Some(result) = set.join_next().await
                    && let Err(error) = result
                {
                    warn!(error = %error, "shutdown approval rejection task join failed");
                }
            }

            let client = client.clone();
            set.spawn(async move {
                let decisions = build_decisions_for_tool_calls(
                    &pending.tool_calls,
                    ToolDecisionAction::Reject,
                    Some("Cancelled — gateway shutting down"),
                );

                match tokio::time::timeout(
                    SHUTDOWN_REJECT_TIMEOUT,
                    client.resolve_tools(&pending.session_id, &pending.run_id, decisions),
                )
                .await
                {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        warn!(
                            session_id = %pending.session_id,
                            run_id = %pending.run_id,
                            error = %error,
                            "failed to reject pending approval during shutdown"
                        );
                    }
                    Err(error) => {
                        warn!(
                            session_id = %pending.session_id,
                            run_id = %pending.run_id,
                            error = %error,
                            "timed out rejecting pending approval during shutdown"
                        );
                    }
                }
            });
        }

        while let Some(result) = set.join_next().await {
            if let Err(error) = result {
                warn!(error = %error, "shutdown approval rejection task join failed");
            }
        }
    }

    fn get_cursor(&self, session_id: &str) -> Result<Option<u64>, String> {
        let guard = self
            .event_cursors
            .lock()
            .map_err(|_| "failed to lock event_cursors".to_string())?;
        Ok(guard.get(session_id).copied())
    }

    fn set_cursor(&self, session_id: &str, cursor: u64) -> Result<(), String> {
        let mut guard = self
            .event_cursors
            .lock()
            .map_err(|_| "failed to lock event_cursors".to_string())?;
        let current = guard.get(session_id).copied();
        let next = current.map_or(cursor, |value| value.max(cursor));
        guard.insert(session_id.to_string(), next);
        Ok(())
    }

    fn resolve_effective_model(
        &self,
        channel_name: &str,
        request_model: Option<String>,
    ) -> Option<String> {
        self.channel_overrides
            .get(channel_name)
            .and_then(|overrides| overrides.model.clone())
            .or(request_model)
            .or_else(|| self.default_model.clone())
    }

    fn resolve_run_approval(
        &self,
        channel_name: &str,
        run_overrides: Option<&RunOverrides>,
    ) -> (ApprovalMode, HashSet<String>) {
        if let Some(run_overrides) = run_overrides
            && let Some(override_auto_approve) = run_overrides.auto_approve.as_ref()
            && let Some(resolved) = resolve_approval_from_override(override_auto_approve)
        {
            return resolved;
        }

        self.resolve_channel_approval(channel_name)
    }

    fn resolve_channel_approval(&self, channel_name: &str) -> (ApprovalMode, HashSet<String>) {
        let Some(overrides) = self.channel_overrides.get(channel_name) else {
            return (self.approval_mode.clone(), self.approval_allowlist.clone());
        };

        let approval_mode = overrides
            .approval_mode
            .clone()
            .unwrap_or_else(|| self.approval_mode.clone());

        let approval_allowlist = overrides
            .approval_allowlist
            .as_ref()
            .map(|list| list.iter().cloned().collect())
            .unwrap_or_else(|| self.approval_allowlist.clone());

        (approval_mode, approval_allowlist)
    }

    fn build_run_overrides(&self, channel_name: &str) -> Option<RunOverrides> {
        if let Some(profile_name) = self.channel_profiles.get(channel_name)
            && let Some(overrides) = self.override_resolver.resolve_run_overrides(profile_name)
            && !overrides.is_empty()
        {
            return Some(overrides);
        }

        let channel_overrides = self.channel_overrides.get(channel_name)?;

        let auto_approve = channel_overrides
            .approval_allowlist
            .as_ref()
            .map(|allowlist| AutoApproveOverride::AllowList(allowlist.clone()));

        let overrides = RunOverrides {
            model: channel_overrides.model.clone(),
            auto_approve,
            ..RunOverrides::default()
        };

        if overrides.is_empty() {
            None
        } else {
            Some(overrides)
        }
    }
}

fn resolve_approval_from_override(
    override_value: &AutoApproveOverride,
) -> Option<(ApprovalMode, HashSet<String>)> {
    match override_value {
        AutoApproveOverride::Mode(mode) => match mode.trim().to_ascii_lowercase().as_str() {
            "all" => Some((ApprovalMode::AllowAll, HashSet::new())),
            "none" => Some((ApprovalMode::DenyAll, HashSet::new())),
            _ => {
                debug!(mode = %mode, "unknown auto_approve override mode; falling back to channel/default approval policy");
                None
            }
        },
        AutoApproveOverride::AllowList(tools) => {
            let allowlist = tools
                .iter()
                .filter_map(|tool| {
                    let trimmed = tool.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .collect::<HashSet<_>>();
            Some((ApprovalMode::Allowlist, allowlist))
        }
    }
}

fn merge_drained_queue(
    mut drained: Vec<QueuedMessage>,
    mut existing: Vec<QueuedMessage>,
) -> Vec<QueuedMessage> {
    drained.append(&mut existing);
    drained
}

async fn consume_run_events(
    client: VACClient,
    run_context: RunContext,
    last_event_id: Option<u64>,
    approval_mode: ApprovalMode,
    approval_allowlist: HashSet<String>,
    cancel: CancellationToken,
) -> RunOutcome {
    let mut stream = match client
        .subscribe_events(&run_context.session_id, last_event_id)
        .await
    {
        Ok(stream) => stream,
        Err(error) => {
            warn!(error = %error, "failed to subscribe to run event stream");
            return RunOutcome::Error {
                error: None,
                cursor: last_event_id,
            };
        }
    };

    let mut streamed_buffer = String::new();
    let mut last_stream_at = Instant::now();
    let mut cursor = last_event_id;
    let timeout_deadline = run_context
        .timeout_seconds
        .map(|seconds| tokio::time::Instant::now() + Duration::from_secs(seconds));
    let timeout_future = async {
        if let Some(deadline) = timeout_deadline {
            tokio::time::sleep_until(deadline).await;
        } else {
            std::future::pending::<()>().await;
        }
    };
    tokio::pin!(timeout_future);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                return RunOutcome::Cancelled { cursor };
            }
            _ = &mut timeout_future => {
                flush_stream_buffer(&run_context.channels, &run_context.delivery, &mut streamed_buffer, true).await;
                deliver_channel_text(&run_context.channels, &run_context.delivery, "⏱️ Interactive run timed out.").await;
                return RunOutcome::Error {
                    error: Some(RunErrorPayload {
                        run_id: None,
                        error: Some("Interactive run timed out".to_string()),
                    }),
                    cursor,
                };
            }
            next = stream.next_event() => {
                let event = match next {
                    Ok(Some(event)) => event,
                    Ok(None) => {
                        flush_stream_buffer(&run_context.channels, &run_context.delivery, &mut streamed_buffer, true).await;
                        return RunOutcome::StreamEnded { cursor };
                    }
                    Err(error) => {
                        flush_stream_buffer(&run_context.channels, &run_context.delivery, &mut streamed_buffer, true).await;
                        warn!(error = %error, "run event stream read failed");
                        return RunOutcome::Error {
                            error: None,
                            cursor,
                        };
                    }
                };

                if let Some(id) = event.event_id_u64 {
                    cursor = Some(cursor.map_or(id, |value| value.max(id)));
                }

                if event.run_id().as_deref() != Some(run_context.run_id.as_str()) {
                    continue;
                }

                match event.event_type.as_str() {
                    "text_delta" => {
                        if let Some(delta) = event.as_text_delta() {
                            streamed_buffer.push_str(&delta);

                            if should_flush_stream_buffer(&streamed_buffer, last_stream_at.elapsed()) {
                                flush_stream_buffer(&run_context.channels, &run_context.delivery, &mut streamed_buffer, false).await;
                                last_stream_at = Instant::now();
                            }
                        }
                    }
                    "tool_calls_proposed" => {
                        if let Some(proposed) = event.as_tool_calls_proposed() {
                            flush_stream_buffer(&run_context.channels, &run_context.delivery, &mut streamed_buffer, true).await;

                            match approval_mode {
                                ApprovalMode::Allowlist => {
                                    let mut auto = HashMap::new();
                                    let mut need_ask = Vec::new();

                                    for tool_call in proposed.tool_calls {
                                        if is_allowlisted(&tool_call.name, &approval_allowlist) {
                                            auto.insert(
                                                tool_call.id.clone(),
                                                ToolDecisionInput {
                                                    action: ToolDecisionAction::Accept,
                                                    content: None,
                                                },
                                            );
                                        } else {
                                            need_ask.push(tool_call);
                                        }
                                    }

                                    let auto_resolved_count = auto.len();
                                    if !auto.is_empty()
                                        && let Err(error) = client
                                            .resolve_tools(&run_context.session_id, &run_context.run_id, auto)
                                            .await
                                    {
                                        warn!(error = %error, "resolve_tools failed");
                                        return RunOutcome::Error {
                                            error: Some(RunErrorPayload {
                                                run_id: None,
                                                error: Some(format!("resolve_tools failed: {error}")),
                                            }),
                                            cursor,
                                        };
                                    }

                                    if !need_ask.is_empty() {
                                        return RunOutcome::ApprovalNeeded {
                                            cursor,
                                            session_id: run_context.session_id.clone(),
                                            run_id: run_context.run_id.clone(),
                                            tool_calls: need_ask,
                                            auto_resolved_count,
                                            delivery: run_context.delivery.clone(),
                                            timeout_seconds: run_context.timeout_seconds,
                                        };
                                    }
                                }
                                ApprovalMode::AllowAll | ApprovalMode::DenyAll => {
                                    if !proposed.tool_calls.is_empty() {
                                        let text = render_running_tools_summary(&proposed.tool_calls);
                                        deliver_channel_text(&run_context.channels, &run_context.delivery, text).await;
                                    }

                                    let decisions = build_tool_decisions(
                                        proposed,
                                        &approval_mode,
                                        &approval_allowlist,
                                    );
                                    if let Err(error) = client
                                        .resolve_tools(&run_context.session_id, &run_context.run_id, decisions)
                                        .await
                                    {
                                        warn!(error = %error, "resolve_tools failed");
                                        return RunOutcome::Error {
                                            error: Some(RunErrorPayload {
                                                run_id: None,
                                                error: Some(format!("resolve_tools failed: {error}")),
                                            }),
                                            cursor,
                                        };
                                    }
                                }
                            }

                            last_stream_at = Instant::now();
                        }
                    }
                    "run_completed" => {
                        flush_stream_buffer(&run_context.channels, &run_context.delivery, &mut streamed_buffer, true).await;
                        return RunOutcome::Completed { cursor };
                    }
                    "run_error" => {
                        flush_stream_buffer(
                            &run_context.channels,
                            &run_context.delivery,
                            &mut streamed_buffer,
                            true,
                        )
                        .await;
                        let payload = event.as_run_error();
                        let error_text = payload
                            .as_ref()
                            .and_then(|payload| payload.error.clone())
                            .unwrap_or_else(|| "unknown error".to_string());
                        warn!(
                            session_id = %run_context.session_id,
                            run_id = %run_context.run_id,
                            error = %error_text,
                            "interactive run failed"
                        );
                        deliver_channel_text(
                            &run_context.channels,
                            &run_context.delivery,
                            format!("⚠️ Agent run failed (session: {})", run_context.session_id),
                        )
                        .await;

                        return RunOutcome::Error {
                            error: payload,
                            cursor,
                        };
                    }
                    _ => {}
                }
            }
        }
    }
}

fn should_flush_stream_buffer(buffer: &str, elapsed_since_last_stream: Duration) -> bool {
    const STREAM_MIN_INTERVAL: Duration = Duration::from_secs(3);
    const STREAM_MAX_BUFFER_LEN: usize = 500;

    if buffer.trim().is_empty() {
        return false;
    }

    if buffer.contains("\n\n") {
        return true;
    }

    let has_complete_line = buffer.contains('\n');
    has_complete_line
        && (buffer.chars().count() >= STREAM_MAX_BUFFER_LEN
            || elapsed_since_last_stream >= STREAM_MIN_INTERVAL)
}

fn take_completed_line_chunk(buffer: &mut String) -> Option<String> {
    let split_after = last_safe_markdown_split(buffer)?;

    let remainder = buffer.split_off(split_after);
    let chunk = std::mem::replace(buffer, remainder);

    Some(chunk)
}

fn last_safe_markdown_split(buffer: &str) -> Option<usize> {
    let mut in_fenced_code_block = false;
    let mut scanned_bytes = 0;
    let mut last_safe_split: Option<usize> = None;

    for line in buffer.split_inclusive('\n') {
        scanned_bytes += line.len();

        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fenced_code_block = !in_fenced_code_block;
        }

        if line.ends_with('\n') && !in_fenced_code_block {
            last_safe_split = Some(scanned_bytes);
        }
    }

    last_safe_split
}

async fn flush_stream_buffer(
    channels: &HashMap<String, Arc<dyn Channel>>,
    delivery: &DeliveryContext,
    buffer: &mut String,
    force: bool,
) {
    if buffer.trim().is_empty() {
        buffer.clear();
        return;
    }

    let text = if force {
        std::mem::take(buffer)
    } else {
        let Some(chunk) = take_completed_line_chunk(buffer) else {
            return;
        };
        chunk
    };

    if text.trim().is_empty() {
        return;
    }

    deliver_channel_text(channels, delivery, text.trim()).await;
}

async fn deliver_channel_text(
    channels: &HashMap<String, Arc<dyn Channel>>,
    delivery: &DeliveryContext,
    text: impl Into<String>,
) {
    let Some(channel) = channels.get(&delivery.channel.0) else {
        warn!(channel = %delivery.channel.0, "channel not connected");
        return;
    };

    let reply = OutboundReply {
        channel: delivery.channel.clone(),
        peer_id: delivery.peer_id.clone(),
        chat_type: delivery.chat_type.clone(),
        text: text.into(),
        metadata: delivery.channel_meta.clone(),
    };

    if let Err(error) = channel.send(reply).await {
        warn!(error = %error, "failed to send channel reply");
    }
}

fn format_batched_queue_messages(queue: &[QueuedMessage]) -> String {
    if queue.len() <= 1 {
        return queue
            .first()
            .map(|item| item.text.clone())
            .unwrap_or_default();
    }

    queue
        .iter()
        .map(|item| {
            let sender = sender_name(&item.inbound.metadata)
                .unwrap_or_else(|| item.inbound.peer_id.0.clone());
            format!("{sender}: {}", item.text.trim())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn sender_name(metadata: &serde_json::Value) -> Option<String> {
    metadata
        .get("display_name")
        .and_then(|value| value.as_str())
        .or_else(|| metadata.get("username").and_then(|value| value.as_str()))
        .map(ToOwned::to_owned)
}

fn extract_run_options(metadata: &serde_json::Value) -> RunStartOptions {
    let options = metadata
        .get("gateway_run_options")
        .and_then(|value| value.as_object());

    let model = options
        .and_then(|value| value.get("model"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let sandbox = options
        .and_then(|value| value.get("sandbox"))
        .and_then(|value| value.as_bool());

    let timeout_seconds = options
        .and_then(|value| value.get("timeout"))
        .and_then(|value| value.as_u64())
        .filter(|value| *value > 0);

    RunStartOptions {
        model,
        sandbox,
        timeout_seconds,
    }
}

fn build_tool_decisions(
    proposed: ToolCallsProposedPayload,
    approval_mode: &ApprovalMode,
    approval_allowlist: &HashSet<String>,
) -> HashMap<String, ToolDecisionInput> {
    proposed
        .tool_calls
        .into_iter()
        .map(|tool_call| {
            let action = match approval_mode {
                ApprovalMode::AllowAll => ToolDecisionAction::Accept,
                ApprovalMode::DenyAll => ToolDecisionAction::Reject,
                ApprovalMode::Allowlist => {
                    if is_allowlisted(&tool_call.name, approval_allowlist) {
                        ToolDecisionAction::Accept
                    } else {
                        ToolDecisionAction::Reject
                    }
                }
            };

            (
                tool_call.id,
                ToolDecisionInput {
                    action,
                    content: None,
                },
            )
        })
        .collect()
}

fn is_allowlisted(tool_name: &str, approval_allowlist: &HashSet<String>) -> bool {
    let normalized = strip_mcp_prefix(tool_name);
    approval_allowlist.contains(tool_name) || approval_allowlist.contains(normalized)
}

fn build_decisions_for_tool_calls(
    tool_calls: &[ProposedToolCall],
    action: ToolDecisionAction,
    content: Option<&str>,
) -> HashMap<String, ToolDecisionInput> {
    tool_calls
        .iter()
        .map(|tool_call| {
            (
                tool_call.id.clone(),
                ToolDecisionInput {
                    action: action.clone(),
                    content: content.map(ToOwned::to_owned),
                },
            )
        })
        .collect()
}

fn render_approver_display(channel_name: &str, resolved_by: &PeerId) -> String {
    match channel_name {
        // Channel-native mention format renders the user's @name in clients.
        "slack" | "discord" => format!("<@{}>", resolved_by.0),
        _ => resolved_by.0.clone(),
    }
}

/// Maximum characters for the entire approval prompt text.
/// Slack mrkdwn section blocks allow 3000 chars; Discord messages 2000.
/// We target the lower bound with some headroom for the header/footer.
const MAX_APPROVAL_PROMPT_CHARS: usize = 1800;

/// Maximum characters for a single tool preview body (code block content, etc.).
const MAX_TOOL_PREVIEW_CHARS: usize = 500;

fn render_approval_prompt(tool_calls: &[ProposedToolCall], auto_count: usize) -> String {
    let mut text = if tool_calls.len() == 1 {
        "🔧 Tool approval required\n\n".to_string()
    } else {
        format!("🔧 {} tools need approval\n\n", tool_calls.len())
    };

    for (index, tool_call) in tool_calls.iter().enumerate() {
        let name = strip_mcp_prefix(&tool_call.name);
        let preview = render_tool_preview(name, &tool_call.arguments);
        text.push_str(&format!("**{} · {}**\n{}\n\n", index + 1, name, preview));

        if text.len() > MAX_APPROVAL_PROMPT_CHARS {
            let remaining = tool_calls.len() - index - 1;
            if remaining > 0 {
                text.push_str(&format!("_…and {remaining} more tool(s)_\n"));
            }
            break;
        }
    }

    if auto_count > 0 {
        text.push_str(&format!("ℹ️ {auto_count} tool(s) auto-approved\n"));
    }

    text
}

fn render_running_tools_summary(tool_calls: &[ProposedToolCall]) -> String {
    let mut text = if tool_calls.len() == 1 {
        "🔧 Running tool\n\n".to_string()
    } else {
        format!("🔧 Running {} tools\n\n", tool_calls.len())
    };

    for (index, tool_call) in tool_calls.iter().enumerate() {
        let name = strip_mcp_prefix(&tool_call.name);
        let preview = render_tool_preview(name, &tool_call.arguments);
        text.push_str(&format!("**{} · {}**\n{}\n\n", index + 1, name, preview));

        if text.len() > MAX_APPROVAL_PROMPT_CHARS {
            let remaining = tool_calls.len() - index - 1;
            if remaining > 0 {
                text.push_str(&format!("_…and {remaining} more tool(s)_\n"));
            }
            break;
        }
    }

    text
}

fn render_tool_preview(tool_name: &str, args: &serde_json::Value) -> String {
    let object = match args.as_object() {
        Some(obj) => obj,
        None => return format!("`{}`", truncate(&args.to_string(), 80)),
    };

    match tool_name {
        "run_command" | "run_command_task" | "run_remote_command" | "run_remote_command_task" => {
            render_run_command_preview(object)
        }
        "create" => render_create_preview(object),
        "str_replace" => render_str_replace_preview(object),
        "remove" => render_remove_preview(object),
        "view" => render_view_preview(object),
        "dynamic_subagent_task" => render_subagent_preview(object),
        "search_docs" => render_search_docs_preview(object),
        "ask_user" => render_ask_user_preview(object),
        _ => render_generic_preview(object),
    }
}

fn render_run_command_preview(args: &serde_json::Map<String, serde_json::Value>) -> String {
    let command = args
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap_or("(no command)");
    let remote = args.get("remote").and_then(|v| v.as_str());

    let mut out = String::new();
    if let Some(host) = remote {
        out.push_str(&format!("on `{host}`\n"));
    }
    out.push_str(&format!(
        "```\n{}\n```",
        truncate(command, MAX_TOOL_PREVIEW_CHARS)
    ));
    out
}

fn render_create_preview(args: &serde_json::Map<String, serde_json::Value>) -> String {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("(unknown path)");
    let file_text = args.get("file_text").and_then(|v| v.as_str());

    let mut out = format!("`{path}`\n");
    if let Some(content) = file_text {
        out.push_str(&format!(
            "```\n{}\n```",
            truncate(content, MAX_TOOL_PREVIEW_CHARS)
        ));
    }
    out
}

fn render_str_replace_preview(args: &serde_json::Map<String, serde_json::Value>) -> String {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("(unknown path)");
    let old_str = args.get("old_str").and_then(|v| v.as_str());
    let new_str = args.get("new_str").and_then(|v| v.as_str());

    let half_budget = MAX_TOOL_PREVIEW_CHARS / 2;
    let mut out = format!("`{path}`\n");
    if let Some(old) = old_str {
        out.push_str(&format!("```\n- {}\n```\n", truncate(old, half_budget)));
    }
    if let Some(new) = new_str {
        out.push_str(&format!("```\n+ {}\n```", truncate(new, half_budget)));
    }
    out
}

fn render_remove_preview(args: &serde_json::Map<String, serde_json::Value>) -> String {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("(unknown path)");
    let recursive = args
        .get("recursive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if recursive {
        format!("`{path}` (recursive)")
    } else {
        format!("`{path}`")
    }
}

fn render_view_preview(args: &serde_json::Map<String, serde_json::Value>) -> String {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("(unknown path)");
    let grep = args.get("grep").and_then(|v| v.as_str());
    let range = args.get("view_range");

    let mut out = format!("`{path}`");
    if let Some(pattern) = grep {
        out.push_str(&format!(" grep=`{}`", truncate(pattern, 60)));
    }
    if let Some(r) = range {
        out.push_str(&format!(" lines {r}"));
    }
    out
}

fn render_subagent_preview(args: &serde_json::Map<String, serde_json::Value>) -> String {
    let description = args
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("(unnamed task)");
    let instructions = args.get("instructions").and_then(|v| v.as_str());
    let tools = args.get("tools").and_then(|v| v.as_array());
    let sandbox = args
        .get("enable_sandbox")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut out = String::new();

    if sandbox {
        out.push_str(&format!(
            "**{}** (🛡 sandboxed)\n",
            truncate(description, 80)
        ));
    } else {
        out.push_str(&format!("**{}**\n", truncate(description, 80)));
    }

    if let Some(tool_list) = tools {
        let names: Vec<&str> = tool_list
            .iter()
            .filter_map(|v| v.as_str())
            .map(strip_mcp_prefix)
            .collect();
        if !names.is_empty() {
            out.push_str(&format!("tools: `{}`\n", names.join("`, `")));
        }
    }

    if let Some(inst) = instructions {
        let trimmed = inst.trim();
        if !trimmed.is_empty() {
            out.push_str(&format!(
                "```\n{}\n```",
                truncate(trimmed, MAX_TOOL_PREVIEW_CHARS)
            ));
        }
    }

    out
}

fn render_search_docs_preview(args: &serde_json::Map<String, serde_json::Value>) -> String {
    let keywords = match args.get("keywords") {
        Some(serde_json::Value::Array(arr)) => {
            let kws: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
            kws.join(", ")
        }
        Some(serde_json::Value::String(s)) => s.clone(),
        _ => return "(no keywords)".to_string(),
    };
    format!("`{}`", truncate(&keywords, 120))
}

fn render_ask_user_preview(args: &serde_json::Map<String, serde_json::Value>) -> String {
    let questions = match args.get("questions").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return "(no questions)".to_string(),
    };

    let mut out = String::new();
    for (i, q) in questions.iter().enumerate() {
        let label = q
            .get("label")
            .and_then(|v| v.as_str())
            .or_else(|| q.get("question").and_then(|v| v.as_str()))
            .unwrap_or("?");
        out.push_str(&format!("{}. {}\n", i + 1, truncate(label, 80)));

        if out.len() > MAX_TOOL_PREVIEW_CHARS {
            let remaining = questions.len() - i - 1;
            if remaining > 0 {
                out.push_str(&format!("_…and {remaining} more_\n"));
            }
            break;
        }
    }
    out
}

fn render_generic_preview(args: &serde_json::Map<String, serde_json::Value>) -> String {
    // Try the old priority-key heuristic for unknown tools.
    for key in [
        "command",
        "path",
        "query",
        "file_text",
        "search",
        "keywords",
        "url",
    ] {
        if let Some(value) = args.get(key).and_then(|v| v.as_str()) {
            return format!("`{}`", truncate(value, 120));
        }
    }

    // For multi-param tools, build a compact summary instead of dumping one raw value.
    if args.len() > 1 {
        let mut parts = Vec::new();
        for (key, value) in args.iter() {
            let display = match value {
                serde_json::Value::String(s) => {
                    if s.len() > 60 {
                        // Long strings get a label + truncated preview.
                        format!("{key}: {}", truncate(s, 60))
                    } else {
                        format!("{key}=`{s}`")
                    }
                }
                serde_json::Value::Array(arr) => format!("{key}: [{} items]", arr.len()),
                serde_json::Value::Bool(b) => format!("{key}={b}"),
                serde_json::Value::Number(n) => format!("{key}={n}"),
                serde_json::Value::Null => continue,
                serde_json::Value::Object(obj) => format!("{key}: {{{} keys}}", obj.len()),
            };
            parts.push(display);
            if parts.len() >= 4 {
                let remaining = args.len() - parts.len();
                if remaining > 0 {
                    parts.push(format!("…+{remaining} more"));
                }
                break;
            }
        }
        return parts.join(" · ");
    }

    // Single-param fallback: show key=value.
    if let Some((key, value)) = args.iter().next() {
        let display = value
            .as_str()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| value.to_string());
        return format!("{key}=`{}`", truncate(&display, 100));
    }

    "(no arguments)".to_string()
}

fn generate_approval_id() -> String {
    Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(8)
        .collect()
}

fn strip_mcp_prefix(name: &str) -> &str {
    // Tool names can be namespaced (e.g. `mcp__run_command`,
    // `mcp__server__run_command`, `vac__view`).
    // Normalize display/allowlist matching to the bare tool segment.
    if let Some((_, suffix)) = name.rsplit_once("__")
        && !suffix.is_empty()
    {
        return suffix;
    }

    name
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_string()
    } else {
        format!("{}…", value.chars().take(max_chars).collect::<String>())
    }
}

// UX trade-off: if approval wait consumes the entire timeout budget, give the resumed
// stream a small floor so it can flush/complete instead of timing out immediately.
const MIN_RESUME_TIMEOUT_SECONDS: u64 = 5;

fn remaining_timeout_after_approval(
    timeout_seconds: Option<u64>,
    approval_wait: Duration,
) -> Option<u64> {
    timeout_seconds.map(|seconds| {
        let remaining = seconds.saturating_sub(approval_wait.as_secs());
        if remaining == 0 {
            MIN_RESUME_TIMEOUT_SECONDS
        } else {
            remaining
        }
    })
}

const MAX_CONTEXT_FIELD_CHARS: usize = 8_000;

fn latest_non_empty_context(queue: &[QueuedMessage]) -> Vec<CallerContextInput> {
    queue
        .iter()
        .rev()
        .find_map(|item| {
            if item.context.is_empty() {
                None
            } else {
                Some(item.context.clone())
            }
        })
        .unwrap_or_default()
}

fn delivery_context_to_caller_context(context: &serde_json::Value) -> Vec<CallerContextInput> {
    let mut lines = vec![
        "The user is replying to a previous notification.".to_string(),
        "--- Watch Context ---".to_string(),
    ];

    if let Some(trigger) = context.get("trigger").and_then(|value| value.as_str()) {
        lines.push(format!(
            "Trigger: {}",
            truncate_chars_with_ellipsis(trigger, MAX_CONTEXT_FIELD_CHARS)
        ));
    }

    if let Some(status) = context.get("status").and_then(|value| value.as_str()) {
        lines.push(format!(
            "Status: {}",
            truncate_chars_with_ellipsis(status, MAX_CONTEXT_FIELD_CHARS)
        ));
    }

    if let Some(summary) = context.get("summary").and_then(|value| value.as_str()) {
        lines.push(format!(
            "Summary: {}",
            truncate_chars_with_ellipsis(summary, MAX_CONTEXT_FIELD_CHARS)
        ));
    }

    if let Some(check_output) = context.get("check_output").and_then(|value| value.as_str()) {
        lines.push(format!(
            "Check output: {}",
            truncate_chars_with_ellipsis(check_output, MAX_CONTEXT_FIELD_CHARS)
        ));
    }

    lines.push("---".to_string());

    vec![CallerContextInput {
        name: "watch_delivery_context".to_string(),
        content: lines.join("\n\n"),
        priority: Some("high".to_string()),
    }]
}

#[cfg(test)]
mod tests;
