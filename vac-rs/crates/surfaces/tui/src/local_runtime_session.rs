// Product-level local runtime session boundary for the TUI.
//
// This module keeps the rest of TUI importing neutral names while the
// default implementation is backed by the owner-native local runtime.
//
// Provider network access remains intentionally available through the
// configured model/login stack; local means no separate app-server process.

#![allow(unused_imports)]

use crate::bottom_pane::FeedbackAudience;
use crate::legacy_core::config::Config;
use crate::session_protocol::AskForApproval;
use crate::session_protocol::ClientRequest;
use crate::session_protocol::ExternalAgentConfigDetectParams;
use crate::session_protocol::ExternalAgentConfigDetectResponse;
use crate::session_protocol::ExternalAgentConfigImportResponse;
use crate::session_protocol::ExternalAgentConfigMigrationItem;
use crate::session_protocol::GetAccountResponse;
use crate::session_protocol::JSONRPCErrorError;
use crate::session_protocol::RequestId;
use crate::session_protocol::ReviewStartResponse;
use crate::session_protocol::SkillsListParams;
use crate::session_protocol::SkillsListResponse;
use crate::session_protocol::Thread;
use crate::session_protocol::ThreadGoalClearResponse;
use crate::session_protocol::ThreadGoalGetResponse;
use crate::session_protocol::ThreadGoalSetResponse;
use crate::session_protocol::ThreadGoalStatus;
use crate::session_protocol::ThreadInjectItemsResponse;
use crate::session_protocol::ThreadListParams;
use crate::session_protocol::ThreadListResponse;
use crate::session_protocol::ThreadLoadedListParams;
use crate::session_protocol::ThreadLoadedListResponse;
use crate::session_protocol::ThreadMemoryMode;
use crate::session_protocol::ThreadRealtimeAudioChunk;
use crate::session_protocol::ThreadRealtimeStartTransport;
use crate::session_protocol::ThreadRollbackResponse;
use crate::session_protocol::ThreadStartSource;
use crate::session_protocol::Turn;
use crate::session_protocol::TurnStartResponse;
use crate::session_protocol::TurnSteerResponse;
use crate::session_protocol::UserInput;
use crate::session_state::ThreadSessionState;
use crate::status::StatusAccountDisplay;
use color_eyre::eyre::Result;
use serde::de::DeserializeOwned;
use std::path::PathBuf;
use vac_otel::TelemetryAuthMode;
use vac_protocol::ThreadId;
use vac_protocol::account::PlanType;
use vac_protocol::approvals::GuardianAssessmentEvent;
use vac_protocol::models::ActivePermissionProfile;
use vac_protocol::models::PermissionProfile;
use vac_protocol::models::ResponseItem;
use vac_protocol::protocol::ReviewTarget;
use vac_protocol::vastar_models::ModelPreset;

pub(crate) use crate::runtime_owner_session::AppServerClient;
pub(crate) use crate::runtime_owner_session::AppServerRequestHandle;
pub(crate) use crate::runtime_owner_session::DEFAULT_IN_PROCESS_CHANNEL_CAPACITY;
pub(crate) use crate::runtime_owner_session::EnvironmentManager;
pub(crate) use crate::runtime_owner_session::EnvironmentManagerArgs;
pub(crate) use crate::runtime_owner_session::ExecServerRuntimePaths;
pub(crate) use crate::runtime_owner_session::InProcessAppServerClient;
pub(crate) use crate::runtime_owner_session::InProcessClientStartArgs;
pub(crate) use crate::runtime_owner_session::LOCAL_FS;
pub(crate) use crate::runtime_owner_session::TuiSessionEvent;
pub(crate) use crate::runtime_owner_session::TuiSessionRequestHandle;
pub(crate) use crate::runtime_owner_session::TypedRequestError;
pub(crate) use crate::runtime_owner_session::app_server_rate_limit_snapshots;
pub(crate) use crate::runtime_owner_session::into_local_runtime_session;
pub(crate) use crate::runtime_owner_session::status_account_display_from_auth_mode;

/// TUI-facing bootstrap data for the local runtime session boundary.
///
/// Owner-native startup data exposed without app-server transport DTOs.
pub(crate) struct LocalRuntimeBootstrap {
    pub(crate) account_email: Option<String>,
    pub(crate) auth_mode: Option<TelemetryAuthMode>,
    pub(crate) status_account_display: Option<StatusAccountDisplay>,
    pub(crate) plan_type: Option<PlanType>,
    pub(crate) requires_vastar_auth: bool,
    pub(crate) default_model: String,
    pub(crate) feedback_audience: FeedbackAudience,
    pub(crate) has_chatgpt_account: bool,
    pub(crate) available_models: Vec<ModelPreset>,
}

impl From<crate::runtime_owner_session::AppServerBootstrap> for LocalRuntimeBootstrap {
    fn from(bootstrap: crate::runtime_owner_session::AppServerBootstrap) -> Self {
        Self {
            account_email: bootstrap.account_email,
            auth_mode: bootstrap.auth_mode,
            status_account_display: bootstrap.status_account_display,
            plan_type: bootstrap.plan_type,
            requires_vastar_auth: bootstrap.requires_vastar_auth,
            default_model: bootstrap.default_model,
            feedback_audience: bootstrap.feedback_audience,
            has_chatgpt_account: bootstrap.has_chatgpt_account,
            available_models: bootstrap.available_models,
        }
    }
}

impl From<vac_local_runtime_owner::LocalRuntimeBootstrap> for LocalRuntimeBootstrap {
    fn from(bootstrap: vac_local_runtime_owner::LocalRuntimeBootstrap) -> Self {
        Self {
            account_email: bootstrap.account_email,
            auth_mode: bootstrap.auth_mode.map(owner_auth_mode_into_telemetry),
            status_account_display: bootstrap
                .status_account_display
                .map(owner_account_display_into_status),
            plan_type: bootstrap.plan_type,
            requires_vastar_auth: bootstrap.requires_vastar_auth,
            default_model: bootstrap.default_model,
            feedback_audience: match bootstrap.feedback_audience {
                vac_local_runtime_owner::LocalRuntimeFeedbackAudience::InternalTester => {
                    FeedbackAudience::InternalTester
                }
                vac_local_runtime_owner::LocalRuntimeFeedbackAudience::External => {
                    FeedbackAudience::External
                }
            },
            has_chatgpt_account: bootstrap.has_chatgpt_account,
            available_models: bootstrap.available_models,
        }
    }
}

fn owner_auth_mode_into_telemetry(
    auth_mode: vac_local_runtime_owner::LocalRuntimeAuthMode,
) -> TelemetryAuthMode {
    match auth_mode {
        vac_local_runtime_owner::LocalRuntimeAuthMode::ApiKey => TelemetryAuthMode::ApiKey,
        vac_local_runtime_owner::LocalRuntimeAuthMode::ProviderCredential
        | vac_local_runtime_owner::LocalRuntimeAuthMode::AgentIdentity => {
            TelemetryAuthMode::ProviderCredential
        }
    }
}

fn owner_account_display_into_status(
    display: vac_local_runtime_owner::LocalRuntimeAccountDisplay,
) -> StatusAccountDisplay {
    match display {
        vac_local_runtime_owner::LocalRuntimeAccountDisplay::ApiKey => StatusAccountDisplay::ApiKey,
        vac_local_runtime_owner::LocalRuntimeAccountDisplay::ChatGpt { email, plan } => {
            StatusAccountDisplay::ProviderCredential { email, plan }
        }
    }
}

#[async_trait::async_trait]
pub(crate) trait LocalRuntimeRequestHandle: Clone + Send + Sync + 'static {
    async fn request_typed<T>(
        &self,
        request: ClientRequest,
    ) -> std::result::Result<T, TypedRequestError>
    where
        T: DeserializeOwned + Send;

    fn is_in_process(&self) -> bool;
}

pub(crate) trait LocalRuntimeStartedThread {
    fn into_parts(self) -> (ThreadSessionState, Vec<Turn>);
}

#[async_trait::async_trait]
pub(crate) trait LocalRuntimeSession: Send {
    type RequestHandle: LocalRuntimeRequestHandle;
    type StartedThread: LocalRuntimeStartedThread;

    fn request_handle(&self) -> Self::RequestHandle;

    async fn external_agent_config_detect(
        &mut self,
        params: ExternalAgentConfigDetectParams,
    ) -> Result<ExternalAgentConfigDetectResponse>;

    async fn external_agent_config_import(
        &mut self,
        migration_items: Vec<ExternalAgentConfigMigrationItem>,
    ) -> Result<ExternalAgentConfigImportResponse>;

    async fn thread_list(&mut self, params: ThreadListParams) -> Result<ThreadListResponse>;

    async fn thread_goal_get(&mut self, thread_id: ThreadId) -> Result<ThreadGoalGetResponse>;

    async fn thread_goal_set(
        &mut self,
        thread_id: ThreadId,
        objective: Option<String>,
        status: Option<ThreadGoalStatus>,
        token_budget: Option<Option<i64>>,
    ) -> Result<ThreadGoalSetResponse>;

    async fn thread_goal_clear(&mut self, thread_id: ThreadId) -> Result<ThreadGoalClearResponse>;

    async fn thread_read(&mut self, thread_id: ThreadId, include_turns: bool) -> Result<Thread>;

    async fn thread_loaded_list(
        &mut self,
        params: ThreadLoadedListParams,
    ) -> Result<ThreadLoadedListResponse>;

    async fn resume_thread(
        &mut self,
        config: Config,
        thread_id: ThreadId,
    ) -> Result<Self::StartedThread>;

    async fn branch_thread(
        &mut self,
        config: Config,
        thread_id: ThreadId,
    ) -> Result<Self::StartedThread>;

    async fn start_thread_with_session_start_source(
        &mut self,
        config: Config,
        session_start_source: Option<ThreadStartSource>,
    ) -> Result<Self::StartedThread>;

    #[cfg(test)]
    async fn start_thread(&mut self, config: &Config) -> Result<Self::StartedThread> {
        self.start_thread_with_session_start_source(config.clone(), None)
            .await
    }

    async fn thread_unsubscribe(&mut self, thread_id: ThreadId) -> Result<()>;

    async fn thread_inject_items(
        &mut self,
        thread_id: ThreadId,
        items: Vec<ResponseItem>,
    ) -> Result<ThreadInjectItemsResponse>;

    async fn turn_interrupt(&mut self, thread_id: ThreadId, turn_id: String) -> Result<()>;

    async fn startup_interrupt(&mut self, thread_id: ThreadId) -> Result<()>;

    async fn thread_memory_mode_set(
        &mut self,
        thread_id: ThreadId,
        mode: ThreadMemoryMode,
    ) -> Result<()>;

    async fn memory_reset(&mut self) -> Result<()>;

    async fn logout_account(&mut self) -> Result<()>;

    async fn thread_compact_start(&mut self, thread_id: ThreadId) -> Result<()>;

    async fn thread_set_name(&mut self, thread_id: ThreadId, name: String) -> Result<()>;

    async fn thread_rollback(
        &mut self,
        thread_id: ThreadId,
        num_turns: u32,
    ) -> Result<ThreadRollbackResponse>;

    async fn review_start(
        &mut self,
        thread_id: ThreadId,
        target: ReviewTarget,
    ) -> Result<ReviewStartResponse>;

    async fn skills_list(&mut self, params: SkillsListParams) -> Result<SkillsListResponse>;

    async fn reload_user_config(&mut self) -> Result<()>;

    async fn thread_realtime_start(
        &mut self,
        thread_id: ThreadId,
        transport: Option<ThreadRealtimeStartTransport>,
        voice: Option<serde_json::Value>,
    ) -> Result<()>;

    async fn thread_realtime_audio(
        &mut self,
        thread_id: ThreadId,
        frame: ThreadRealtimeAudioChunk,
    ) -> Result<()>;

    async fn thread_realtime_stop(&mut self, thread_id: ThreadId) -> Result<()>;

    async fn thread_shell_command(&mut self, thread_id: ThreadId, command: String) -> Result<()>;

    async fn thread_approve_guardian_denied_action(
        &mut self,
        thread_id: ThreadId,
        event: &GuardianAssessmentEvent,
    ) -> Result<()>;

    async fn thread_background_terminals_clean(&mut self, thread_id: ThreadId) -> Result<()>;

    #[allow(clippy::too_many_arguments)]
    async fn turn_start(
        &mut self,
        thread_id: ThreadId,
        items: Vec<UserInput>,
        cwd: PathBuf,
        approval_policy: AskForApproval,
        approvals_reviewer: vac_protocol::config_types::ApprovalsReviewer,
        permission_profile: PermissionProfile,
        active_permission_profile: Option<ActivePermissionProfile>,
        model: String,
        effort: Option<vac_protocol::vastar_models::ReasoningEffort>,
        summary: Option<vac_protocol::config_types::ReasoningSummary>,
        service_tier: Option<Option<vac_protocol::config_types::ServiceTier>>,
        collaboration_mode: Option<vac_protocol::config_types::CollaborationMode>,
        personality: Option<vac_protocol::config_types::Personality>,
        output_schema: Option<serde_json::Value>,
    ) -> Result<TurnStartResponse>;

    async fn turn_steer(
        &mut self,
        thread_id: ThreadId,
        turn_id: String,
        items: Vec<UserInput>,
    ) -> std::result::Result<TurnSteerResponse, TypedRequestError>;

    async fn bootstrap(&mut self, config: &Config) -> Result<LocalRuntimeBootstrap>;

    async fn read_account(&mut self) -> Result<GetAccountResponse>;

    async fn reject_server_request(
        &mut self,
        request_id: RequestId,
        error: JSONRPCErrorError,
    ) -> std::io::Result<()>;

    async fn resolve_server_request(
        &mut self,
        request_id: RequestId,
        result: serde_json::Value,
    ) -> std::io::Result<()>;

    #[allow(dead_code)]
    async fn next_event(&mut self) -> Option<TuiSessionEvent>;

    async fn shutdown(self) -> std::io::Result<()>;
}
