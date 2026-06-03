// O5/O6 balanced session group: source session_part_000.rs
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Debug;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use crate::agent::AgentControl;
use crate::agent::AgentStatus;
use crate::agent::Mailbox;
use crate::agent::MailboxReceiver;
use crate::agent::agent_status_from_event;
use crate::agent::status::is_final;
use crate::build_available_skills;
use crate::commit_attribution::commit_message_trailer_instruction;
use crate::compact;
use crate::config::ManagedFeatures;
use crate::config::resolve_tool_suggest_config_from_layer_stack;
use crate::connectors;
use crate::context::ApprovedCommandPrefixSaved;
use crate::context::AppsInstructions;
use crate::context::AvailablePluginsInstructions;
use crate::context::AvailableSkillsInstructions;
use crate::context::CollaborationModeInstructions;
use crate::context::ContextualUserFragment;
use crate::context::NetworkRuleSaved;
use crate::context::PermissionsInstructions;
use crate::context::PersonalitySpecInstructions;
use crate::default_skill_metadata_budget;
use crate::environment_selection::ResolvedTurnEnvironments;
use crate::exec_policy::ExecPolicyManager;
use crate::installation_id::resolve_installation_id;
use crate::parse_turn_item;
use crate::path_utils::normalize_for_native_workdir;
use crate::realtime_conversation::RealtimeConversationManager;
use crate::rollout::find_thread_name_by_id;
use crate::session_prefix::format_subagent_notification_message;
use crate::skills::SkillRenderSideEffects;
use crate::skills_load_input_from_config;
use crate::turn_metadata::TurnMetadataState;
use async_channel::Receiver;
use async_channel::Sender;
use chrono::Local;
use chrono::Utc;
use futures::future::BoxFuture;
use futures::future::Shared;
use futures::prelude::*;
use rmcp::model::ListResourceTemplatesResult;
use rmcp::model::ListResourcesResult;
use rmcp::model::PaginatedRequestParams;
use rmcp::model::ReadResourceRequestParams;
use rmcp::model::ReadResourceResult;
use rmcp::model::RequestId;
use serde_json::Value;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use toml::Value as TomlValue;
use tracing::Instrument;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::info_span;
use tracing::instrument;
use tracing::warn;
use uuid::Uuid;
use vac_analytics::AnalyticsEventsClient;
use vac_analytics::SubAgentThreadStartedInput;
use vac_runtime_protocol::McpServerElicitationRequest;
use vac_runtime_protocol::McpServerElicitationRequestParams;
use vac_config::types::OAuthCredentialsStoreMode;
use vac_exec_server::Environment;
use vac_exec_server::EnvironmentManager;
use vac_exec_server::FileSystemSandboxContext;
use vac_features::FEATURES;
use vac_features::Feature;
use vac_features::unstable_features_warning_event;
use vac_hooks::Hooks;
use vac_hooks::HooksConfig;
use vac_login::AuthManager;
use vac_login::VACAuth;
use vac_login::auth_env_telemetry::collect_auth_env_telemetry;
use vac_login::default_client::originator;
use vac_mcp::McpConnectionManager;
use vac_mcp::McpRuntimeEnvironment;
use vac_mcp::ToolInfo;
use vac_mcp::vac_apps_tools_cache_key;
use vac_models_manager::manager::RefreshStrategy;
use vac_models_manager::manager::SharedModelsManager;
use vac_network_proxy::NetworkProxy;
use vac_network_proxy::NetworkProxyAuditMetadata;
use vac_network_proxy::normalize_host;
use vac_otel::current_span_trace_id;
use vac_otel::current_span_w3c_trace_context;
use vac_otel::set_parent_from_w3c_trace_context;
use vac_protocol::ThreadId;
use vac_protocol::ToolName;
use vac_protocol::account::PlanType as AccountPlanType;
use vac_protocol::approvals::ElicitationRequestEvent;
use vac_protocol::approvals::ExecPolicyAmendment;
use vac_protocol::approvals::NetworkPolicyAmendment;
use vac_protocol::approvals::NetworkPolicyRuleAction;
use vac_protocol::config_types::ApprovalsReviewer;
use vac_protocol::config_types::ModeKind;
use vac_protocol::config_types::Settings;
use vac_protocol::config_types::WebSearchMode;
use vac_protocol::dynamic_tools::DynamicToolResponse;
use vac_protocol::dynamic_tools::DynamicToolSpec;
use vac_protocol::items::TurnItem;
use vac_protocol::items::UserMessageItem;
use vac_protocol::mcp::CallToolResult;
use vac_protocol::models::ActivePermissionProfile;
use vac_protocol::models::AdditionalPermissionProfile;
use vac_protocol::models::BaseInstructions;
use vac_protocol::models::PermissionProfile;
use vac_protocol::models::SandboxEnforcement;
use vac_protocol::models::format_allow_prefixes;
use vac_protocol::permissions::FileSystemSandboxPolicy;
use vac_protocol::permissions::NetworkSandboxPolicy;
use vac_protocol::protocol::FileChange;
use vac_protocol::protocol::HasLegacyEvent;
use vac_protocol::protocol::InterAgentCommunication;
use vac_protocol::protocol::ItemCompletedEvent;
use vac_protocol::protocol::ItemStartedEvent;
use vac_protocol::protocol::RawResponseItemEvent;
use vac_protocol::protocol::ReviewRequest;
use vac_protocol::protocol::RolloutItem;
use vac_protocol::protocol::SessionSource;
use vac_protocol::protocol::SubAgentSource;
use vac_protocol::protocol::TurnAbortReason;
use vac_protocol::protocol::TurnContextItem;
use vac_protocol::protocol::TurnContextNetworkItem;
use vac_protocol::protocol::W3cTraceContext;
use vac_protocol::request_permissions::PermissionGrantScope;
use vac_protocol::request_permissions::RequestPermissionProfile;
use vac_protocol::request_permissions::RequestPermissionsArgs;
use vac_protocol::request_permissions::RequestPermissionsEvent;
use vac_protocol::request_permissions::RequestPermissionsResponse;
use vac_protocol::request_user_input::RequestUserInputArgs;
use vac_protocol::request_user_input::RequestUserInputResponse;
use vac_protocol::vastar_models::ModelInfo;
use vac_rmcp_client::ElicitationResponse;
use vac_rollout::state_db;
use vac_rollout_trace::AgentResultTracePayload;
use vac_rollout_trace::ThreadStartedTraceMetadata;
use vac_rollout_trace::ThreadTraceContext;
use vac_sandboxing::policy_transforms::intersect_permission_profiles;
use vac_shell_command::parse_command::parse_command;
use vac_terminal_detection::user_agent;
use vac_thread_store::CreateThreadParams;
use vac_thread_store::LiveThread;
use vac_thread_store::LiveThreadInitGuard;
use vac_thread_store::LocalThreadStore;
use vac_thread_store::ResumeThreadParams;
use vac_thread_store::ThreadEventPersistenceMode;
use vac_thread_store::ThreadPersistenceMetadata;
use vac_thread_store::ThreadStore;
use vac_utils_output_truncation::TruncationPolicy;

use crate::client::ModelClient;
use crate::compact::collect_user_messages;
use crate::config::Config;
use crate::config::Constrained;
use crate::config::ConstraintResult;
use crate::config::StartedNetworkProxy;
use crate::config::resolve_web_search_mode_for_turn;
use crate::context_manager::ContextManager;
use crate::context_manager::TotalTokenUsageBreakdown;
use crate::thread_rollout_truncation::initial_history_has_prior_user_turns;
use crate::vac_thread::ThreadConfigSnapshot;
use vac_config::CONFIG_TOML_FILE;
use vac_config::types::McpServerConfig;
use vac_model_provider_info::ModelProviderInfo;
use vac_protocol::config_types::ShellEnvironmentPolicy;
use vac_protocol::error::Result as VACResult;
use vac_protocol::error::VACErr;
#[cfg(test)]
use vac_protocol::exec_output::StreamOutput;

mod config_lock;
mod handlers;
mod mcp;
mod multi_agents;
mod review;
mod rollout_reconstruction;
#[allow(clippy::module_inception)]
pub(crate) mod session;
pub(crate) mod turn;
pub(crate) mod turn_context;
use self::config_lock::export_config_lock_if_configured;
use self::config_lock::validate_config_lock_if_configured;
#[cfg(test)]
use self::handlers::submission_dispatch_span;
use self::handlers::submission_loop;
use self::review::spawn_review_thread;
use self::session::AppServerClientMetadata;
use self::session::Session;
use self::session::SessionConfiguration;
pub(crate) use self::session::SessionSettingsUpdate;
#[cfg(test)]
use self::turn::AssistantMessageStreamParsers;
#[cfg(test)]
use self::turn::collect_explicit_app_ids_from_skill_items;
#[cfg(test)]
use self::turn::filter_connectors_for_input;
use self::turn::realtime_text_for_event;
use self::turn_context::TurnContext;
use self::turn_context::TurnSkillsContext;
#[cfg(test)]
mod rollout_reconstruction_tests;

#[derive(Debug, PartialEq)]
pub enum SteerInputError {
    NoActiveTurn(Vec<UserInput>),
    ExpectedTurnMismatch { expected: String, actual: String },
    ActiveTurnNotSteerable { turn_kind: NonSteerableTurnKind },
    EmptyInput,
}

impl SteerInputError {
    fn to_error_event(&self) -> ErrorEvent {
        match self {
            Self::NoActiveTurn(_) => ErrorEvent {
                message: "no active turn to steer".to_string(),
                vac_error_info: Some(VACErrorInfo::BadRequest),
            },
            Self::ExpectedTurnMismatch { expected, actual } => ErrorEvent {
                message: format!("expected active turn id `{expected}` but found `{actual}`"),
                vac_error_info: Some(VACErrorInfo::BadRequest),
            },
            Self::ActiveTurnNotSteerable { turn_kind } => {
                let turn_kind_label = match turn_kind {
                    NonSteerableTurnKind::Review => "review",
                    NonSteerableTurnKind::Compact => "compact",
                };
                ErrorEvent {
                    message: format!("cannot steer a {turn_kind_label} turn"),
                    vac_error_info: Some(VACErrorInfo::ActiveTurnNotSteerable {
                        turn_kind: *turn_kind,
                    }),
                }
            }
            Self::EmptyInput => ErrorEvent {
                message: "input must not be empty".to_string(),
                vac_error_info: Some(VACErrorInfo::BadRequest),
            },
        }
    }
}

/// Notes from the previous real user turn.
///
/// Conceptually this is the same role that `previous_model` used to fill, but
/// it can carry other prior-turn settings that matter when constructing
/// sensible state-change diffs or full-context reinjection, such as model
/// switches or detecting a prior `realtime_active -> false` transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreviousTurnSettings {
    pub(crate) model: String,
    pub(crate) realtime_active: Option<bool>,
}

use crate::SkillError;
use crate::SkillLoadOutcome;
use crate::SkillMetadata;
use crate::SkillsManager;
use crate::agents_md::AgentsMdManager;
use crate::context::UserInstructions;
use crate::exec_policy::ExecPolicyUpdateError;
use crate::guardian::GuardianReviewSessionManager;
use crate::mcp::McpManager;
use crate::network_policy_decision::execpolicy_network_rule_amendment;
use crate::rollout::map_session_init_error;
use crate::session_startup_prewarm::SessionStartupPrewarmHandle;
use crate::shell;
use crate::shell_snapshot::ShellSnapshot;
use crate::skills_watcher::SkillsWatcher;
use crate::skills_watcher::SkillsWatcherEvent;
use crate::state::ActiveTurn;
use crate::state::MailboxDeliveryPhase;
use crate::state::PendingRequestPermissions;
use crate::state::SessionServices;
use crate::state::SessionState;
#[cfg(test)]
use crate::stream_events_utils::HandleOutputCtx;
#[cfg(test)]
use crate::stream_events_utils::handle_output_item_done;
use crate::tasks::ReviewTask;
use crate::tools::network_approval::NetworkApprovalService;
use crate::tools::network_approval::build_blocked_request_observer;
use crate::tools::network_approval::build_network_policy_decider;
#[cfg(test)]
use crate::tools::parallel::ToolCallRuntime;
use crate::tools::sandboxing::ApprovalStore;
use crate::turn_timing::TurnTimingState;
use crate::turn_timing::record_turn_ttfm_metric;
use crate::unified_exec::UnifiedExecProcessManager;
use crate::windows_sandbox::WindowsSandboxLevelExt;
use vac_core_plugins::PluginsManager;
use vac_git_utils::get_git_repo_root;
use vac_mcp::compute_auth_statuses;
use vac_otel::SessionTelemetry;
use vac_otel::THREAD_STARTED_METRIC;
use vac_otel::TelemetryAuthMode;
use vac_protocol::config_types::CollaborationMode;
use vac_protocol::config_types::Personality;
use vac_protocol::config_types::ReasoningSummary as ReasoningSummaryConfig;
use vac_protocol::config_types::ServiceTier;
use vac_protocol::config_types::WindowsSandboxLevel;
use vac_protocol::models::ContentItem;
use vac_protocol::models::ResponseInputItem;
use vac_protocol::models::ResponseItem;
use vac_protocol::protocol::ApplyPatchApprovalRequestEvent;
use vac_protocol::protocol::AskForApproval;
use vac_protocol::protocol::CompactedItem;
use vac_protocol::protocol::DeprecationNoticeEvent;
use vac_protocol::protocol::ErrorEvent;
use vac_protocol::protocol::Event;
use vac_protocol::protocol::EventMsg;
use vac_protocol::protocol::ExecApprovalRequestEvent;
use vac_protocol::protocol::InitialHistory;
use vac_protocol::protocol::McpServerRefreshConfig;
use vac_protocol::protocol::ModelRerouteEvent;
use vac_protocol::protocol::ModelRerouteReason;
use vac_protocol::protocol::ModelVerification;
use vac_protocol::protocol::ModelVerificationEvent;
use vac_protocol::protocol::NetworkApprovalContext;
use vac_protocol::protocol::NonSteerableTurnKind;
use vac_protocol::protocol::Op;
use vac_protocol::protocol::RateLimitSnapshot;
use vac_protocol::protocol::RequestUserInputEvent;
use vac_protocol::protocol::ReviewDecision;
use vac_protocol::protocol::SandboxPolicy;
use vac_protocol::protocol::SessionConfiguredEvent;
use vac_protocol::protocol::SessionNetworkProxyRuntime;
use vac_protocol::protocol::SkillDependencies as ProtocolSkillDependencies;
use vac_protocol::protocol::SkillErrorInfo;
use vac_protocol::protocol::SkillInterface as ProtocolSkillInterface;
use vac_protocol::protocol::SkillMetadata as ProtocolSkillMetadata;
use vac_protocol::protocol::SkillToolDependency as ProtocolSkillToolDependency;
use vac_protocol::protocol::StreamErrorEvent;
use vac_protocol::protocol::Submission;
use vac_protocol::protocol::ThreadMemoryMode;
use vac_protocol::protocol::TokenCountEvent;
use vac_protocol::protocol::TokenUsage;
use vac_protocol::protocol::TokenUsageInfo;
use vac_protocol::protocol::VACErrorInfo;
use vac_protocol::protocol::WarningEvent;
use vac_protocol::user_input::UserInput;
use vac_protocol::vastar_models::ReasoningEffort as ReasoningEffortConfig;
use vac_tools::ToolsConfig;
use vac_tools::ToolsConfigParams;
use vac_utils_absolute_path::AbsolutePathBuf;
use vac_utils_readiness::ReadinessFlag;
#[cfg(test)]
use vac_utils_stream_parser::ProposedPlanSegment;

/// The high-level interface to the VAC system.
/// It operates as a queue pair where you send submissions and receive events.
pub struct VAC {
    pub(crate) tx_sub: Sender<Submission>,
    pub(crate) rx_event: Receiver<Event>,
    // Last known status of the agent.
    pub(crate) agent_status: watch::Receiver<AgentStatus>,
    pub(crate) session: Arc<Session>,
    // Shared future for the background submission loop completion so multiple
    // callers can wait for shutdown.
    pub(crate) session_loop_termination: SessionLoopTermination,
}

pub(crate) type SessionLoopTermination = Shared<BoxFuture<'static, ()>>;

/// Wrapper returned by [`VAC::spawn`] containing the spawned [`VAC`] and
/// the unique session id.
pub struct VACSpawnOk {
    pub vac: VAC,
    pub thread_id: ThreadId,
}

pub(crate) struct VACSpawnArgs {
    pub(crate) config: Config,
    pub(crate) auth_manager: Arc<AuthManager>,
    pub(crate) models_manager: SharedModelsManager,
    pub(crate) environment_manager: Arc<EnvironmentManager>,
    pub(crate) skills_manager: Arc<SkillsManager>,
    pub(crate) plugins_manager: Arc<PluginsManager>,
    pub(crate) mcp_manager: Arc<McpManager>,
    pub(crate) skills_watcher: Arc<SkillsWatcher>,
    pub(crate) conversation_history: InitialHistory,
    pub(crate) session_source: SessionSource,
    pub(crate) agent_control: AgentControl,
    pub(crate) dynamic_tools: Vec<DynamicToolSpec>,
    pub(crate) persist_extended_history: bool,
    pub(crate) metrics_service_name: Option<String>,
    pub(crate) inherited_shell_snapshot: Option<Arc<ShellSnapshot>>,
    pub(crate) inherited_exec_policy: Option<Arc<ExecPolicyManager>>,
    /// Parent rollout trace used only to derive fresh spawned child traces.
    ///
    /// Root sessions and non-thread-spawn subagents pass a disabled context;
    /// `Session::new` creates the root trace itself when rollout tracing is enabled.
    pub(crate) parent_rollout_thread_trace: ThreadTraceContext,
    pub(crate) user_shell_override: Option<shell::Shell>,
    pub(crate) parent_trace: Option<W3cTraceContext>,
    pub(crate) environment_selections: ResolvedTurnEnvironments,
    pub(crate) analytics_events_client: Option<AnalyticsEventsClient>,
    pub(crate) thread_store: Arc<dyn ThreadStore>,
}

pub(crate) const INITIAL_SUBMIT_ID: &str = "";
pub(crate) const SUBMISSION_CHANNEL_CAPACITY: usize = 512;
pub(crate) const EVENT_CHANNEL_CAPACITY: usize = 2048;
const VAC_LEGACY_PROVIDER_VERIFY_URL: &str = "legacy-chatgpt-cyber-verify-disabled";
const CYBER_SAFETY_URL: &str = "https://developers.vastar.com/vac/concepts/cyber-safety";
