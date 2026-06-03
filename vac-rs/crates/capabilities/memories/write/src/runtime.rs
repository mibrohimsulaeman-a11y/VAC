use futures::StreamExt;
use std::sync::Arc;
use std::time::Duration;
use vac_core::ModelClient;
use vac_core::NewThread;
use vac_core::Prompt;
use vac_core::ResponseEvent;
use vac_core::StartThreadOptions;
use vac_core::ThreadManager;
use vac_core::VACThread;
use vac_core::config::Config;
use vac_core::content_items_to_text;
use vac_core::resolve_installation_id;
use vac_features::Feature;
use vac_login::AuthManager;
use vac_login::VACAuth;
use vac_login::auth_env_telemetry::collect_auth_env_telemetry;
use vac_login::default_client::originator;
use vac_otel::SessionTelemetry;
use vac_otel::TelemetryAuthMode;
use vac_protocol::ThreadId;
use vac_protocol::config_types::ReasoningSummary;
use vac_protocol::config_types::ServiceTier;
use vac_protocol::protocol::InitialHistory;
use vac_protocol::protocol::InternalSessionSource;
use vac_protocol::protocol::Op;
use vac_protocol::protocol::SessionSource;
use vac_protocol::protocol::TokenUsage;
use vac_protocol::user_input::UserInput;
use vac_protocol::vastar_models::ModelInfo;
use vac_protocol::vastar_models::ReasoningEffort;
use vac_rollout_trace::InferenceTraceContext;
use vac_state::StateRuntime;
use vac_terminal_detection::user_agent;

pub(crate) struct SpawnedConsolidationAgent {
    pub(crate) thread_id: ThreadId,
    pub(crate) thread: Arc<VACThread>,
}

#[derive(Clone, Debug)]
pub(crate) struct StageOneRequestContext {
    pub(crate) model_info: ModelInfo,
    pub(crate) session_telemetry: SessionTelemetry,
    pub(crate) reasoning_effort: Option<ReasoningEffort>,
    pub(crate) reasoning_summary: ReasoningSummary,
    pub(crate) service_tier: Option<ServiceTier>,
    pub(crate) turn_metadata_header: Option<String>,
}

impl StageOneRequestContext {
    pub(crate) fn start_timer(&self, name: &str) -> Option<vac_otel::Timer> {
        self.session_telemetry.start_timer(name, &[]).ok()
    }

    pub(crate) fn counter(&self, name: &str, inc: i64, tags: &[(&str, &str)]) {
        self.session_telemetry.counter(name, inc, tags);
    }

    pub(crate) fn histogram(&self, name: &str, value: i64, tags: &[(&str, &str)]) {
        self.session_telemetry.histogram(name, value, tags);
    }
}

pub(crate) struct MemoryStartupContext {
    thread_id: ThreadId,
    thread: Arc<VACThread>,
    thread_manager: Arc<ThreadManager>,
    auth_manager: Arc<AuthManager>,
    session_telemetry: SessionTelemetry,
}

impl MemoryStartupContext {
    pub(crate) fn new(
        thread_manager: Arc<ThreadManager>,
        auth_manager: Arc<AuthManager>,
        thread_id: ThreadId,
        thread: Arc<VACThread>,
        config: &Config,
        source: SessionSource,
    ) -> Self {
        let auth = auth_manager.auth_cached();
        let auth = auth.as_ref();
        let auth_mode = auth.map(VACAuth::auth_mode).map(TelemetryAuthMode::from);
        let account_id = auth.and_then(VACAuth::get_account_id);
        let account_email = auth.and_then(VACAuth::get_account_email);
        let model = config.model.as_deref().unwrap_or("unknown");
        let auth_env_telemetry = collect_auth_env_telemetry(
            &config.model_provider,
            auth_manager.vac_api_key_env_enabled(),
        );
        let session_telemetry = SessionTelemetry::new(
            thread_id,
            model,
            model,
            account_id,
            account_email,
            auth_mode,
            originator().value,
            config.otel.log_user_prompt,
            user_agent(),
            source,
        )
        .with_auth_env(auth_env_telemetry.to_otel_metadata());

        Self {
            thread_id,
            thread,
            thread_manager,
            auth_manager,
            session_telemetry,
        }
    }

    pub(crate) fn thread_id(&self) -> ThreadId {
        self.thread_id
    }

    pub(crate) fn state_db(&self) -> Option<Arc<StateRuntime>> {
        self.thread.state_db()
    }

    pub(crate) fn counter(&self, name: &str, inc: i64, tags: &[(&str, &str)]) {
        self.session_telemetry.counter(name, inc, tags);
    }

    pub(crate) fn histogram(&self, name: &str, value: i64, tags: &[(&str, &str)]) {
        self.session_telemetry.histogram(name, value, tags);
    }

    pub(crate) fn start_timer(&self, name: &str) -> Option<vac_otel::Timer> {
        self.session_telemetry.start_timer(name, &[]).ok()
    }

    pub(crate) async fn stage_one_request_context(
        &self,
        config: &Config,
        model_name: &str,
        reasoning_effort: ReasoningEffort,
    ) -> StageOneRequestContext {
        let config_snapshot = self.thread.config_snapshot().await;
        let model_info = self
            .thread_manager
            .get_models_manager()
            .get_model_info(model_name, &config.to_models_manager_config())
            .await;
        let turn_metadata_header =
            vac_core::build_turn_metadata_header(&config.cwd, /*sandbox*/ None).await;
        let reasoning_summary = config
            .model_reasoning_summary
            .unwrap_or(model_info.default_reasoning_summary);

        StageOneRequestContext {
            model_info,
            turn_metadata_header,
            session_telemetry: self
                .session_telemetry
                .clone()
                .with_model(model_name, model_name),
            reasoning_effort: Some(reasoning_effort),
            reasoning_summary,
            service_tier: config_snapshot.service_tier,
        }
    }

    pub(crate) async fn stream_stage_one_prompt(
        &self,
        config: &Config,
        prompt: &Prompt,
        context: &StageOneRequestContext,
    ) -> anyhow::Result<(String, Option<TokenUsage>)> {
        let installation_id = resolve_installation_id(&config.vac_home).await?;
        let session_source = self.thread.config_snapshot().await.session_source;
        let model_client = ModelClient::new(
            Some(Arc::clone(&self.auth_manager)),
            self.thread_id,
            installation_id,
            config.model_provider.clone(),
            session_source,
            config.model_verbosity,
            config.features.enabled(Feature::EnableRequestCompression),
            config.features.enabled(Feature::RuntimeMetrics),
            /*beta_features_header*/ None,
        );

        let mut client_session = model_client.new_session();
        let mut stream = client_session
            .stream(
                prompt,
                &context.model_info,
                &context.session_telemetry,
                context.reasoning_effort,
                context.reasoning_summary,
                context.service_tier,
                context.turn_metadata_header.as_deref(),
                &InferenceTraceContext::disabled(),
            )
            .await?;

        let mut result = String::new();
        let mut token_usage = None;
        while let Some(message) = stream.next().await.transpose()? {
            match message {
                ResponseEvent::OutputTextDelta(delta) => result.push_str(&delta),
                ResponseEvent::OutputItemDone(item) => {
                    if result.is_empty()
                        && let vac_protocol::models::ResponseItem::Message { content, .. } = item
                        && let Some(text) = content_items_to_text(&content)
                    {
                        result.push_str(&text);
                    }
                }
                ResponseEvent::Completed {
                    token_usage: usage, ..
                } => {
                    token_usage = usage;
                    break;
                }
                _ => {}
            }
        }

        Ok((result, token_usage))
    }

    pub(crate) async fn spawn_consolidation_agent(
        &self,
        config: Config,
        prompt: Vec<UserInput>,
    ) -> anyhow::Result<SpawnedConsolidationAgent> {
        let environments = self
            .thread_manager
            .default_environment_selections(&config.cwd);
        let NewThread {
            thread_id, thread, ..
        } = self
            .thread_manager
            .start_thread_with_options(StartThreadOptions {
                config,
                initial_history: InitialHistory::New,
                session_source: Some(SessionSource::Internal(
                    InternalSessionSource::MemoryConsolidation,
                )),
                dynamic_tools: Vec::new(),
                persist_extended_history: false,
                metrics_service_name: None,
                parent_trace: None,
                environments,
            })
            .await?;

        let agent = SpawnedConsolidationAgent { thread_id, thread };
        if let Err(err) = agent
            .thread
            .submit(Op::UserInput {
                items: prompt,
                environments: None,
                final_output_json_schema: None,
                responsesapi_client_metadata: None,
            })
            .await
        {
            if let Err(shutdown_err) = self.shutdown_consolidation_agent(agent).await {
                tracing::warn!(
                    "failed to shut down consolidation agent after submit error: {shutdown_err}"
                );
            }
            return Err(err.into());
        }

        Ok(agent)
    }

    pub(crate) async fn shutdown_consolidation_agent(
        &self,
        agent: SpawnedConsolidationAgent,
    ) -> anyhow::Result<()> {
        let SpawnedConsolidationAgent { thread_id, thread } = agent;
        let thread = self
            .thread_manager
            .remove_thread(&thread_id)
            .await
            .unwrap_or(thread);

        tokio::time::timeout(Duration::from_secs(10), thread.shutdown_and_wait())
            .await
            .map_err(|_| {
                anyhow::anyhow!("memory consolidation agent {thread_id} shutdown timed out")
            })??;

        Ok(())
    }
}
