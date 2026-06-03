//! Command-bus seam for local runtime requests.
//!
//! Plan 30A intentionally starts with a read-only command before moving prompt
//! submit. This keeps the owner command bus shape honest: account/thread reads
//! and prompt turn starts cross the same local-runtime-owner seam instead of
//! letting prompt submit be a chat-only false-green.

// VAC_RUNTIME_OWNER_NONDEFAULT_DEFER_ACCEPTED: plan30-owner-native-default-parity
// Remaining fail-closed sentinels in this file are restricted to non-default
// remote/compatibility surfaces and must not be used by the default TUI path.

use crate::external_agent_config::ExternalAgentConfigDetectOptions;
use crate::external_agent_config::ExternalAgentConfigMigrationItem as ProviderExternalAgentConfigMigrationItem;
use crate::external_agent_config::ExternalAgentConfigMigrationItemType as ProviderExternalAgentConfigMigrationItemType;
use crate::external_agent_config::ExternalAgentConfigService;
use crate::external_agent_config::MigrationDetails as ProviderMigrationDetails;
use crate::external_agent_config::NamedMigration as ProviderNamedMigration;
use crate::external_agent_config::PluginsMigration as ProviderPluginsMigration;
use crate::plugin_surface::RuntimeMarketplaceInterface;
use crate::plugin_surface::RuntimeMarketplaceLoadErrorInfo;
use crate::plugin_surface::RuntimePluginAvailability;
use crate::plugin_surface::RuntimePluginDetail;
use crate::plugin_surface::RuntimePluginInstallResponse;
use crate::plugin_surface::RuntimePluginListResponse;
use crate::plugin_surface::RuntimePluginMarketplaceEntry;
use crate::plugin_surface::RuntimePluginReadResponse;
use crate::plugin_surface::RuntimePluginSetEnabledResponse;
use crate::plugin_surface::RuntimePluginSummary;
use crate::plugin_surface::RuntimePluginUninstallResponse;
use crate::plugin_surface::path_parent;
use crate::plugin_surface::plugin_app_ids;
use crate::plugin_surface::runtime_skill_summaries_from_core;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use thiserror::Error;
use vac_config::set_user_plugin_enabled;
use vac_core::StateDbHandle;
use vac_core::SteerInputError;
use vac_core::ThreadManager;
use vac_core::VACThreadTurnContextOverrides;
use vac_core::config::Config;
use vac_core::config::ConfigBuilder;
use vac_core_plugins::PluginInstallRequest;
use vac_core_plugins::PluginReadRequest;
use vac_exec_server::EnvironmentManager;
use vac_features::Feature;
use vac_login::AuthManager;
use vac_login::VACAuth;
use vac_protocol::ThreadId;
use vac_protocol::account::PlanType;
use vac_protocol::config_types::ApprovalsReviewer;
use vac_protocol::config_types::CollaborationMode;
use vac_protocol::config_types::Personality;
use vac_protocol::config_types::ReasoningSummary;
use vac_protocol::config_types::ServiceTier;
use vac_protocol::models::ActivePermissionProfile;
use vac_protocol::models::PermissionProfile;
use vac_protocol::protocol::AskForApproval;
use vac_protocol::protocol::ConversationAudioParams;
use vac_protocol::protocol::ConversationStartParams;
use vac_protocol::protocol::Op;
use vac_protocol::protocol::ReviewRequest;
use vac_protocol::protocol::ReviewTarget;
use vac_protocol::protocol::SandboxPolicy;
use vac_protocol::protocol::SkillDependencies;
use vac_protocol::protocol::SkillErrorInfo;
use vac_protocol::protocol::SkillInterface;
use vac_protocol::protocol::SkillMetadata;
use vac_protocol::protocol::SkillToolDependency;
use vac_protocol::protocol::ThreadGoal;
use vac_protocol::protocol::ThreadGoalStatus;
use vac_protocol::protocol::ThreadMemoryMode;
use vac_protocol::protocol::validate_thread_goal_objective;
use vac_protocol::user_input::UserInput;
use vac_protocol::vastar_models::ReasoningEffort;
use vac_state::StateRuntime;
use vac_thread_store::ThreadMetadataPatch;
use vac_thread_store::ThreadStore;
use vac_thread_store::UpdateThreadMetadataParams;
use vac_utils_absolute_path::AbsolutePathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeAccount {
    ApiKey,
    Chatgpt { email: String, plan_type: PlanType },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeAccountRead {
    pub account: Option<RuntimeAccount>,
    pub requires_vastar_auth: bool,
}

#[derive(Clone)]
pub struct RuntimeSkillsListCommand {
    pub thread_manager: Arc<ThreadManager>,
    pub environment_manager: Arc<EnvironmentManager>,
    pub config: Arc<Config>,
    pub cwds: Vec<PathBuf>,
    pub force_reload: bool,
    pub per_cwd_extra_user_roots: Option<Vec<RuntimeSkillsListExtraRootsForCwd>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeSkillsListExtraRootsForCwd {
    pub cwd: PathBuf,
    pub extra_user_roots: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct RuntimeSkillsListResponse {
    pub data: Vec<RuntimeSkillsListEntry>,
}

#[derive(Clone, Debug)]
pub struct RuntimeSkillsListEntry {
    pub cwd: PathBuf,
    pub skills: Vec<SkillMetadata>,
    pub errors: Vec<SkillErrorInfo>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeExternalAgentConfigDetectCommand {
    pub vac_home: PathBuf,
    pub include_home: bool,
    pub cwds: Option<Vec<PathBuf>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeExternalAgentConfigImportCommand {
    pub vac_home: PathBuf,
    pub migration_items: Vec<RuntimeExternalAgentConfigMigrationItem>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeExternalAgentConfigMigrationItem {
    pub item_type: RuntimeExternalAgentConfigMigrationItemType,
    pub description: String,
    pub cwd: Option<PathBuf>,
    pub details: Option<RuntimeExternalAgentConfigMigrationDetails>,
}

#[derive(Clone, Debug, Copy, Eq, PartialEq)]
pub enum RuntimeExternalAgentConfigMigrationItemType {
    Config,
    Skills,
    AgentsMd,
    Plugins,
    McpServerConfig,
    Subagents,
    Hooks,
    Commands,
    Sessions,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeExternalAgentConfigMigrationDetails {
    pub plugins: Vec<RuntimeExternalAgentConfigPluginsMigration>,
    pub sessions: Vec<RuntimeExternalAgentConfigSessionMigration>,
    pub mcp_servers: Vec<RuntimeExternalAgentConfigNamedMigration>,
    pub hooks: Vec<RuntimeExternalAgentConfigNamedMigration>,
    pub subagents: Vec<RuntimeExternalAgentConfigNamedMigration>,
    pub commands: Vec<RuntimeExternalAgentConfigNamedMigration>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeExternalAgentConfigPluginsMigration {
    pub marketplace_name: String,
    pub plugin_names: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeExternalAgentConfigSessionMigration {
    pub path: PathBuf,
    pub cwd: PathBuf,
    pub title: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeExternalAgentConfigNamedMigration {
    pub name: String,
}

#[derive(Clone)]
pub struct RuntimePluginSurfaceCommand {
    pub operation: RuntimePluginSurfaceOperation,
    pub thread_manager: Option<Arc<ThreadManager>>,
    pub auth_manager: Option<Arc<AuthManager>>,
    pub config: Option<Arc<Config>>,
    pub cwd: Option<PathBuf>,
    pub plugin_id: Option<String>,
    pub plugin_name: Option<String>,
    pub marketplace_path: Option<AbsolutePathBuf>,
    pub remote_marketplace_name: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimePluginSurfaceOperation {
    List,
    Read,
    Install,
    Uninstall,
    SetEnabled,
}

#[derive(Clone)]
pub enum RuntimeReadCommand {
    ReadAccount {
        auth_manager: Arc<AuthManager>,
        requires_vastar_auth: bool,
    },
    ThreadList {
        thread_manager: Arc<ThreadManager>,
    },
    GetThreadGoal {
        thread_manager: Arc<ThreadManager>,
        thread_id: ThreadId,
    },
    ListSkills(Box<RuntimeSkillsListCommand>),
    PluginSurface(Box<RuntimePluginSurfaceCommand>),
    ExternalAgentConfigDetect(RuntimeExternalAgentConfigDetectCommand),
}

#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum RuntimeReadResponse {
    Account(RuntimeAccountRead),
    ThreadList(Vec<ThreadId>),
    ThreadGoal(Option<ThreadGoal>),
    SkillsList(RuntimeSkillsListResponse),
    PluginList(RuntimePluginListResponse),
    PluginRead(RuntimePluginReadResponse),
    ExternalAgentConfigDetect {
        items: Vec<RuntimeExternalAgentConfigMigrationItem>,
    },
}

impl PartialEq for RuntimeReadResponse {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Account(left), Self::Account(right)) => left == right,
            (Self::ThreadList(left), Self::ThreadList(right)) => left == right,
            (Self::ThreadGoal(left), Self::ThreadGoal(right)) => left == right,
            // Skills metadata DTOs intentionally do not implement PartialEq; tests assert
            // no-fake-success via command errors instead of full response equality.
            (Self::SkillsList(_), Self::SkillsList(_)) => false,
            (Self::PluginList(_), Self::PluginList(_)) => false,
            (Self::PluginRead(_), Self::PluginRead(_)) => false,
            (
                Self::ExternalAgentConfigDetect { items: left },
                Self::ExternalAgentConfigDetect { items: right },
            ) => left == right,
            _ => false,
        }
    }
}

impl Eq for RuntimeReadResponse {}

#[derive(Clone)]
pub enum RuntimeWriteCommand {
    StartTurn(Box<RuntimeTurnStartCommand>),
    InterruptTurn(RuntimeThreadCommand),
    SteerTurn(RuntimeTurnSteerCommand),
    StartCompact(RuntimeThreadCommand),
    RunShellCommand(RuntimeShellCommand),
    RealtimeStart(RuntimeRealtimeStartCommand),
    RealtimeAppendAudio(RuntimeRealtimeAppendAudioCommand),
    RealtimeStop(RuntimeRealtimeStopCommand),
    StartReview(Box<RuntimeReviewStartCommand>),
    SetThreadGoal(Box<RuntimeThreadGoalSetCommand>),
    ClearThreadGoal {
        thread_manager: Arc<ThreadManager>,
        thread_id: ThreadId,
    },
    ReloadConfig {
        thread_manager: Arc<ThreadManager>,
    },
    LogoutAccount {
        auth_manager: Arc<AuthManager>,
    },
    ResetMemory {
        sqlite_home: PathBuf,
        model_provider_id: String,
        vac_home: PathBuf,
    },
    SetThreadMemoryMode {
        thread_manager: Arc<ThreadManager>,
        thread_store: Option<Arc<dyn ThreadStore>>,
        thread_id: ThreadId,
        mode: ThreadMemoryMode,
    },
    PluginSurface(Box<RuntimePluginSurfaceCommand>),
    ExternalAgentConfigImport(RuntimeExternalAgentConfigImportCommand),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeWriteResponse {
    TurnStarted {
        turn_id: String,
    },
    TurnInterrupted,
    TurnSteered {
        turn_id: String,
    },
    CompactStarted,
    ShellCommandStarted,
    RealtimeStarted,
    RealtimeAudioAppended,
    RealtimeStopped,
    ReviewStarted {
        turn_id: String,
        display_text: String,
    },
    ThreadGoalSet {
        goal: ThreadGoal,
    },
    ThreadGoalCleared {
        cleared: bool,
    },
    ConfigReloaded,
    AccountLoggedOut,
    MemoryReset,
    ThreadMemoryModeSet,
    PluginSurfaceAccepted,
    PluginInstall(RuntimePluginInstallResponse),
    PluginUninstall(RuntimePluginUninstallResponse),
    PluginSetEnabled(RuntimePluginSetEnabledResponse),
    ExternalAgentConfigImportAccepted,
}

#[derive(Clone)]
pub struct RuntimeThreadCommand {
    pub thread_manager: Arc<ThreadManager>,
    pub thread_id: ThreadId,
}

#[derive(Clone)]
pub struct RuntimeTurnSteerCommand {
    pub thread_manager: Arc<ThreadManager>,
    pub thread_id: ThreadId,
    pub turn_id: String,
    pub items: Vec<UserInput>,
}

#[derive(Clone)]
pub struct RuntimeShellCommand {
    pub thread_manager: Arc<ThreadManager>,
    pub thread_id: ThreadId,
    pub command: String,
}

#[derive(Clone)]
pub struct RuntimeRealtimeStartCommand {
    pub thread_manager: Arc<ThreadManager>,
    pub thread_id: ThreadId,
    pub params: ConversationStartParams,
}

#[derive(Clone)]
pub struct RuntimeRealtimeAppendAudioCommand {
    pub thread_manager: Arc<ThreadManager>,
    pub thread_id: ThreadId,
    pub params: ConversationAudioParams,
}

#[derive(Clone)]
pub struct RuntimeRealtimeStopCommand {
    pub thread_manager: Arc<ThreadManager>,
    pub thread_id: ThreadId,
}

#[derive(Clone)]
pub struct RuntimeTurnStartCommand {
    pub thread_manager: Arc<ThreadManager>,
    pub thread_id: ThreadId,
    pub items: Vec<UserInput>,
    pub cwd: PathBuf,
    pub approval_policy: AskForApproval,
    pub approvals_reviewer: ApprovalsReviewer,
    pub sandbox_policy: Option<SandboxPolicy>,
    pub permission_profile: Option<PermissionProfile>,
    pub active_permission_profile: Option<ActivePermissionProfile>,
    pub model: String,
    pub effort: Option<Option<ReasoningEffort>>,
    pub summary: Option<ReasoningSummary>,
    pub service_tier: Option<Option<ServiceTier>>,
    pub collaboration_mode: Option<CollaborationMode>,
    pub personality: Option<Personality>,
    pub output_schema: Option<serde_json::Value>,
}

#[derive(Clone)]
pub struct RuntimeReviewStartCommand {
    pub thread_manager: Arc<ThreadManager>,
    pub thread_id: ThreadId,
    pub target: ReviewTarget,
}

#[derive(Clone)]
pub struct RuntimeThreadGoalSetCommand {
    pub thread_manager: Arc<ThreadManager>,
    pub thread_id: ThreadId,
    pub objective: Option<String>,
    pub status: Option<ThreadGoalStatus>,
    pub token_budget: Option<Option<i64>>,
}

#[derive(Debug, Error)]
pub enum RuntimeCommandBusError {
    #[error("email and plan type are required for chatgpt authentication")]
    MissingChatGptAccountDetails,
    #[error("invalid turn context override: {0}")]
    InvalidTurnContext(String),
    #[error("invalid review target: {0}")]
    InvalidReviewTarget(String),
    #[error("invalid thread goal objective: {0}")]
    InvalidThreadGoalObjective(String),
    #[error("goal budgets must be positive when provided")]
    InvalidThreadGoalBudget,
    #[error("ephemeral thread does not support goals: {0}")]
    EphemeralThreadGoalsUnsupported(ThreadId),
    #[error("sqlite state db unavailable for thread goals")]
    ThreadGoalStateDbUnavailable,
    #[error("thread goal store command failed: {0}")]
    ThreadGoalStore(String),
    #[error("cannot update goal for thread {0}: no goal exists")]
    ThreadGoalMissing(ThreadId),
    #[error("thread {thread_id} does not support realtime conversation")]
    RealtimeConversationUnsupported { thread_id: ThreadId },
    #[error("runtime thread command failed: {0}")]
    Thread(#[from] vac_protocol::error::VACErr),
    #[error("runtime steer command failed: {0:?}")]
    SteerInput(SteerInputError),
    #[error("shell command must not be empty")]
    EmptyShellCommand,
    #[error("account/logout failed: {0}")]
    Logout(String),
    #[error("memory/reset failed: {0}")]
    MemoryReset(String),
    #[error("thread/memoryMode/set failed: {0}")]
    ThreadMemoryModeSet(String),
    #[error("skills/list config resolution failed for cwd {cwd}: {message}")]
    SkillsConfigResolutionFailed { cwd: PathBuf, message: String },
    #[error("skills/list per-cwd extra user root must be absolute: {0}")]
    SkillsExtraRootNotAbsolute(PathBuf),
    #[error("external-agent config provider failed: {0}")]
    ExternalAgentConfig(std::io::Error),
    #[error(
        "external-agent config import produced background plugin/session work that the owner bus cannot yet complete"
    )]
    ExternalAgentConfigBackgroundImportRequired,
    #[error("plugin surface operation requires a dedicated owner-native provider: {0:?}")]
    PluginSurfaceOwnerProviderRequired(RuntimePluginSurfaceOperation),
    #[error("plugin surface {operation:?} command is missing required field: {field}")]
    PluginSurfaceMissingField {
        operation: RuntimePluginSurfaceOperation,
        field: &'static str,
    },
    #[error("plugin surface {operation:?} failed: {message}")]
    PluginSurfaceOperationFailed {
        operation: RuntimePluginSurfaceOperation,
        message: String,
    },
    #[error(
        "external-agent config detect/import requires full migration-details DTO support in the owner command"
    )]
    ExternalAgentConfigMigrationDetailsRequired,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeCommandBus;

impl RuntimeCommandBus {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    pub async fn execute_read(
        &self,
        command: RuntimeReadCommand,
    ) -> Result<RuntimeReadResponse, RuntimeCommandBusError> {
        match command {
            RuntimeReadCommand::ReadAccount {
                auth_manager,
                requires_vastar_auth,
            } => Ok(RuntimeReadResponse::Account(
                read_account_from_auth_manager(&auth_manager, requires_vastar_auth)?,
            )),
            RuntimeReadCommand::ThreadList { thread_manager } => Ok(
                RuntimeReadResponse::ThreadList(thread_manager.list_thread_ids().await),
            ),
            RuntimeReadCommand::GetThreadGoal {
                thread_manager,
                thread_id,
            } => self
                .get_thread_goal(thread_manager, thread_id)
                .await
                .map(RuntimeReadResponse::ThreadGoal),
            RuntimeReadCommand::ListSkills(command) => self
                .list_skills(*command)
                .await
                .map(RuntimeReadResponse::SkillsList),
            RuntimeReadCommand::PluginSurface(command) => self.execute_plugin_read(*command).await,
            RuntimeReadCommand::ExternalAgentConfigDetect(command) => self
                .external_agent_config_detect(command)
                .await
                .map(|items| RuntimeReadResponse::ExternalAgentConfigDetect { items }),
        }
    }

    pub async fn execute_write(
        &self,
        command: RuntimeWriteCommand,
    ) -> Result<RuntimeWriteResponse, RuntimeCommandBusError> {
        match command {
            RuntimeWriteCommand::StartTurn(command) => self
                .start_turn(*command)
                .await
                .map(|turn_id| RuntimeWriteResponse::TurnStarted { turn_id }),
            RuntimeWriteCommand::InterruptTurn(command) => self
                .interrupt_turn(command)
                .await
                .map(|()| RuntimeWriteResponse::TurnInterrupted),
            RuntimeWriteCommand::SteerTurn(command) => self
                .steer_turn(command)
                .await
                .map(|turn_id| RuntimeWriteResponse::TurnSteered { turn_id }),
            RuntimeWriteCommand::StartCompact(command) => self
                .start_compact(command)
                .await
                .map(|()| RuntimeWriteResponse::CompactStarted),
            RuntimeWriteCommand::RunShellCommand(command) => self
                .run_shell_command(command)
                .await
                .map(|()| RuntimeWriteResponse::ShellCommandStarted),
            RuntimeWriteCommand::RealtimeStart(command) => self
                .realtime_start(command)
                .await
                .map(|()| RuntimeWriteResponse::RealtimeStarted),
            RuntimeWriteCommand::RealtimeAppendAudio(command) => self
                .realtime_append_audio(command)
                .await
                .map(|()| RuntimeWriteResponse::RealtimeAudioAppended),
            RuntimeWriteCommand::RealtimeStop(command) => self
                .realtime_stop(command)
                .await
                .map(|()| RuntimeWriteResponse::RealtimeStopped),
            RuntimeWriteCommand::StartReview(command) => {
                self.start_review(*command)
                    .await
                    .map(
                        |(turn_id, display_text)| RuntimeWriteResponse::ReviewStarted {
                            turn_id,
                            display_text,
                        },
                    )
            }
            RuntimeWriteCommand::SetThreadGoal(command) => self
                .set_thread_goal(*command)
                .await
                .map(|goal| RuntimeWriteResponse::ThreadGoalSet { goal }),
            RuntimeWriteCommand::ClearThreadGoal {
                thread_manager,
                thread_id,
            } => self
                .clear_thread_goal(thread_manager, thread_id)
                .await
                .map(|cleared| RuntimeWriteResponse::ThreadGoalCleared { cleared }),
            RuntimeWriteCommand::ReloadConfig { thread_manager } => self
                .reload_config(thread_manager)
                .await
                .map(|()| RuntimeWriteResponse::ConfigReloaded),
            RuntimeWriteCommand::LogoutAccount { auth_manager } => self
                .logout_account(auth_manager)
                .await
                .map(|()| RuntimeWriteResponse::AccountLoggedOut),
            RuntimeWriteCommand::ResetMemory {
                sqlite_home,
                model_provider_id,
                vac_home,
            } => self
                .reset_memory(sqlite_home, model_provider_id, vac_home)
                .await
                .map(|()| RuntimeWriteResponse::MemoryReset),
            RuntimeWriteCommand::SetThreadMemoryMode {
                thread_manager,
                thread_store,
                thread_id,
                mode,
            } => self
                .set_thread_memory_mode(thread_manager, thread_store, thread_id, mode)
                .await
                .map(|()| RuntimeWriteResponse::ThreadMemoryModeSet),
            RuntimeWriteCommand::PluginSurface(command) => {
                self.execute_plugin_write(*command).await
            }
            RuntimeWriteCommand::ExternalAgentConfigImport(command) => self
                .external_agent_config_import(command)
                .await
                .map(|()| RuntimeWriteResponse::ExternalAgentConfigImportAccepted),
        }
    }

    async fn execute_plugin_read(
        &self,
        command: RuntimePluginSurfaceCommand,
    ) -> Result<RuntimeReadResponse, RuntimeCommandBusError> {
        match command.operation {
            RuntimePluginSurfaceOperation::List => self
                .plugin_list(command)
                .await
                .map(RuntimeReadResponse::PluginList),
            RuntimePluginSurfaceOperation::Read => self
                .plugin_read(command)
                .await
                .map(RuntimeReadResponse::PluginRead),
            RuntimePluginSurfaceOperation::Install
            | RuntimePluginSurfaceOperation::Uninstall
            | RuntimePluginSurfaceOperation::SetEnabled => Err(
                RuntimeCommandBusError::PluginSurfaceOwnerProviderRequired(command.operation),
            ),
        }
    }

    async fn execute_plugin_write(
        &self,
        command: RuntimePluginSurfaceCommand,
    ) -> Result<RuntimeWriteResponse, RuntimeCommandBusError> {
        match command.operation {
            RuntimePluginSurfaceOperation::Install => self
                .plugin_install(command)
                .await
                .map(RuntimeWriteResponse::PluginInstall),
            RuntimePluginSurfaceOperation::Uninstall => self
                .plugin_uninstall(command)
                .await
                .map(RuntimeWriteResponse::PluginUninstall),
            RuntimePluginSurfaceOperation::SetEnabled => self
                .plugin_set_enabled(command)
                .await
                .map(RuntimeWriteResponse::PluginSetEnabled),
            RuntimePluginSurfaceOperation::List | RuntimePluginSurfaceOperation::Read => Err(
                RuntimeCommandBusError::PluginSurfaceOwnerProviderRequired(command.operation),
            ),
        }
    }

    async fn plugin_list(
        &self,
        command: RuntimePluginSurfaceCommand,
    ) -> Result<RuntimePluginListResponse, RuntimeCommandBusError> {
        let operation = command.operation;
        let thread_manager = required_plugin_thread_manager(&command)?;
        let auth_manager = required_plugin_auth_manager(&command)?;
        let config = required_plugin_config(&command)?;
        let roots = match command.cwd {
            Some(cwd) => vec![AbsolutePathBuf::try_from(cwd).map_err(|_| {
                RuntimeCommandBusError::PluginSurfaceOperationFailed {
                    operation,
                    message: "plugin list cwd must be absolute".to_string(),
                }
            })?],
            None => Vec::new(),
        };
        if !config.features.enabled(Feature::Plugins) {
            return Ok(RuntimePluginListResponse::default());
        }
        let plugins_input = config.plugins_config_input();
        let auth = auth_manager.auth().await;
        let plugins_manager = thread_manager.plugins_manager();
        plugins_manager.maybe_start_plugin_list_background_tasks_for_config(
            &plugins_input,
            auth.clone(),
            &roots,
            None,
        );
        let list_input = plugins_input.clone();
        let manager_for_list = Arc::clone(&plugins_manager);
        let roots_for_list = roots.clone();
        let outcome = tokio::task::spawn_blocking(move || {
            manager_for_list.list_marketplaces_for_config(&list_input, &roots_for_list)
        })
        .await
        .map_err(|err| RuntimeCommandBusError::PluginSurfaceOperationFailed {
            operation,
            message: format!("failed to join plugin list task: {err}"),
        })?
        .map_err(|err| RuntimeCommandBusError::PluginSurfaceOperationFailed {
            operation,
            message: err.to_string(),
        })?;
        let featured_plugin_ids = if outcome.marketplaces.iter().any(|marketplace| {
            marketplace.name == vac_core_plugins::VASTAR_CURATED_MARKETPLACE_NAME
        }) {
            plugins_manager
                .featured_plugin_ids_for_config(&plugins_input, auth.as_ref())
                .await
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        Ok(RuntimePluginListResponse {
            marketplaces: outcome
                .marketplaces
                .into_iter()
                .map(|marketplace| RuntimePluginMarketplaceEntry {
                    name: marketplace.name,
                    path: Some(marketplace.path),
                    interface: marketplace
                        .interface
                        .map(|interface| RuntimeMarketplaceInterface {
                            display_name: interface.display_name,
                        }),
                    plugins: marketplace
                        .plugins
                        .into_iter()
                        .map(|plugin| RuntimePluginSummary {
                            id: plugin.id,
                            installed: plugin.installed,
                            enabled: plugin.enabled,
                            name: plugin.name,
                            source: plugin.source.into(),
                            install_policy: plugin.policy.installation.into(),
                            auth_policy: plugin.policy.authentication.into(),
                            availability: RuntimePluginAvailability::Available,
                            interface: plugin.interface.map(Into::into),
                        })
                        .collect(),
                })
                .collect(),
            marketplace_load_errors: outcome
                .errors
                .into_iter()
                .map(|err| RuntimeMarketplaceLoadErrorInfo {
                    marketplace_path: err.path,
                    message: err.message,
                })
                .collect(),
            featured_plugin_ids,
        })
    }

    async fn plugin_read(
        &self,
        command: RuntimePluginSurfaceCommand,
    ) -> Result<RuntimePluginReadResponse, RuntimeCommandBusError> {
        let operation = command.operation;
        if command.remote_marketplace_name.is_some() {
            return Err(RuntimeCommandBusError::PluginSurfaceOwnerProviderRequired(
                operation,
            ));
        }
        let thread_manager = required_plugin_thread_manager(&command)?;
        let config = required_plugin_config(&command)?;
        let marketplace_path = required_plugin_marketplace_path(&command)?;
        let plugin_name = required_plugin_name(&command)?;
        let scoped_config = scoped_plugin_config(&config, path_parent(&marketplace_path)).await?;
        let plugins_input = scoped_config.plugins_config_input();
        let outcome = thread_manager
            .plugins_manager()
            .read_plugin_for_config(
                &plugins_input,
                &PluginReadRequest {
                    plugin_name,
                    marketplace_path,
                },
            )
            .await
            .map_err(|err| RuntimeCommandBusError::PluginSurfaceOperationFailed {
                operation,
                message: err.to_string(),
            })?;
        Ok(RuntimePluginReadResponse {
            plugin: RuntimePluginDetail {
                marketplace_name: outcome.marketplace_name,
                marketplace_path: outcome.marketplace_path,
                summary: RuntimePluginSummary {
                    id: outcome.plugin.id,
                    name: outcome.plugin.name,
                    source: outcome.plugin.source.into(),
                    installed: outcome.plugin.installed,
                    enabled: outcome.plugin.enabled,
                    install_policy: outcome.plugin.policy.installation.into(),
                    auth_policy: outcome.plugin.policy.authentication.into(),
                    availability: RuntimePluginAvailability::Available,
                    interface: outcome.plugin.interface.map(Into::into),
                },
                description: outcome.plugin.description,
                skills: runtime_skill_summaries_from_core(
                    &outcome.plugin.skills,
                    &outcome.plugin.disabled_skill_paths,
                ),
                app_ids: plugin_app_ids(outcome.plugin.apps),
                mcp_servers: outcome.plugin.mcp_server_names,
            },
        })
    }

    async fn plugin_install(
        &self,
        command: RuntimePluginSurfaceCommand,
    ) -> Result<RuntimePluginInstallResponse, RuntimeCommandBusError> {
        let operation = command.operation;
        if command.remote_marketplace_name.is_some() {
            return Err(RuntimeCommandBusError::PluginSurfaceOwnerProviderRequired(
                operation,
            ));
        }
        let thread_manager = required_plugin_thread_manager(&command)?;
        let config = required_plugin_config(&command)?;
        let marketplace_path = required_plugin_marketplace_path(&command)?;
        let plugin_name = required_plugin_name(&command)?;
        let _scoped_config = scoped_plugin_config(&config, path_parent(&marketplace_path)).await?;
        let result = thread_manager
            .plugins_manager()
            .install_plugin(PluginInstallRequest {
                plugin_name,
                marketplace_path,
            })
            .await
            .map_err(|err| RuntimeCommandBusError::PluginSurfaceOperationFailed {
                operation,
                message: err.to_string(),
            })?;
        clear_plugin_related_caches(&thread_manager);
        let apps =
            vac_core_plugins::loader::load_plugin_apps(result.installed_path.as_path()).await;
        Ok(RuntimePluginInstallResponse {
            auth_policy: result.auth_policy.into(),
            installed_path: result.installed_path,
            plugin_id: result.plugin_id.as_key(),
            app_ids: plugin_app_ids(apps),
        })
    }

    async fn plugin_uninstall(
        &self,
        command: RuntimePluginSurfaceCommand,
    ) -> Result<RuntimePluginUninstallResponse, RuntimeCommandBusError> {
        let operation = command.operation;
        let thread_manager = required_plugin_thread_manager(&command)?;
        let plugin_id = required_plugin_id(&command)?;
        thread_manager
            .plugins_manager()
            .uninstall_plugin(plugin_id)
            .await
            .map_err(|err| RuntimeCommandBusError::PluginSurfaceOperationFailed {
                operation,
                message: err.to_string(),
            })?;
        clear_plugin_related_caches(&thread_manager);
        Ok(RuntimePluginUninstallResponse)
    }

    async fn plugin_set_enabled(
        &self,
        command: RuntimePluginSurfaceCommand,
    ) -> Result<RuntimePluginSetEnabledResponse, RuntimeCommandBusError> {
        let operation = command.operation;
        let thread_manager = required_plugin_thread_manager(&command)?;
        let config = required_plugin_config(&command)?;
        let plugin_id = required_plugin_id(&command)?;
        let enabled = command
            .enabled
            .ok_or(RuntimeCommandBusError::PluginSurfaceMissingField {
                operation,
                field: "enabled",
            })?;
        set_user_plugin_enabled(config.vac_home.as_path(), plugin_id, enabled)
            .await
            .map_err(|err| RuntimeCommandBusError::PluginSurfaceOperationFailed {
                operation,
                message: err.to_string(),
            })?;
        clear_plugin_related_caches(&thread_manager);
        Ok(RuntimePluginSetEnabledResponse)
    }

    pub async fn list_skills(
        &self,
        command: RuntimeSkillsListCommand,
    ) -> Result<RuntimeSkillsListResponse, RuntimeCommandBusError> {
        let RuntimeSkillsListCommand {
            thread_manager,
            environment_manager,
            config,
            cwds,
            force_reload,
            per_cwd_extra_user_roots,
        } = command;
        let current_cwd = config.cwd.to_path_buf();
        let cwds = if cwds.is_empty() {
            vec![current_cwd.clone()]
        } else {
            cwds
        };
        let extra_roots_by_cwd = absolute_extra_skill_roots_for_cwds(
            &cwds,
            per_cwd_extra_user_roots.unwrap_or_default(),
        )?;
        let fs = environment_manager
            .default_environment()
            .map(|environment| environment.get_filesystem());
        let skills_manager = thread_manager.skills_manager();
        let mut data = Vec::new();
        for cwd in cwds {
            let scoped_config = if cwd == current_cwd {
                Arc::clone(&config)
            } else {
                Arc::new(
                    ConfigBuilder::default()
                        .vac_home(config.vac_home.to_path_buf())
                        .fallback_cwd(Some(cwd.clone()))
                        .build()
                        .await
                        .map_err(|err| RuntimeCommandBusError::SkillsConfigResolutionFailed {
                            cwd: cwd.clone(),
                            message: err.to_string(),
                        })?,
                )
            };
            let plugins_input = scoped_config.plugins_config_input();
            let effective_skill_roots = thread_manager
                .plugins_manager()
                .effective_skill_roots_for_layer_stack(
                    &scoped_config.config_layer_stack,
                    &plugins_input,
                )
                .await;
            let skills_input = vac_core::skills::SkillsLoadInput::new(
                scoped_config.cwd.clone(),
                effective_skill_roots,
                scoped_config.config_layer_stack.clone(),
                scoped_config.bundled_skills_enabled(),
            );
            let extra_roots = extra_roots_by_cwd
                .get(&cwd)
                .map_or(&[][..], std::vec::Vec::as_slice);
            let outcome = skills_manager
                .skills_for_cwd_with_extra_user_roots(
                    &skills_input,
                    force_reload,
                    extra_roots,
                    fs.clone(),
                )
                .await;
            data.push(RuntimeSkillsListEntry {
                cwd,
                skills: runtime_skill_metadata_from_core(&outcome.skills, &outcome.disabled_paths),
                errors: runtime_skill_errors_from_core(&outcome.errors),
            });
        }
        Ok(RuntimeSkillsListResponse { data })
    }

    pub async fn external_agent_config_detect(
        &self,
        command: RuntimeExternalAgentConfigDetectCommand,
    ) -> Result<Vec<RuntimeExternalAgentConfigMigrationItem>, RuntimeCommandBusError> {
        let RuntimeExternalAgentConfigDetectCommand {
            vac_home,
            include_home,
            cwds,
        } = command;
        let service = ExternalAgentConfigService::new(vac_home);
        let items = service
            .detect(ExternalAgentConfigDetectOptions { include_home, cwds })
            .await
            .map_err(RuntimeCommandBusError::ExternalAgentConfig)?;
        Ok(items
            .into_iter()
            .map(runtime_external_agent_item_from_provider)
            .collect())
    }

    pub async fn external_agent_config_import(
        &self,
        command: RuntimeExternalAgentConfigImportCommand,
    ) -> Result<(), RuntimeCommandBusError> {
        let RuntimeExternalAgentConfigImportCommand {
            vac_home,
            migration_items,
        } = command;
        let service = ExternalAgentConfigService::new(vac_home);
        let import_outcome = service
            .import(
                migration_items
                    .into_iter()
                    .map(provider_external_agent_item_from_runtime)
                    .collect(),
            )
            .await
            .map_err(RuntimeCommandBusError::ExternalAgentConfig)?;
        if import_outcome.has_pending_background_work() {
            return Err(RuntimeCommandBusError::ExternalAgentConfigBackgroundImportRequired);
        }
        Ok(())
    }

    pub async fn interrupt_turn(
        &self,
        command: RuntimeThreadCommand,
    ) -> Result<(), RuntimeCommandBusError> {
        let RuntimeThreadCommand {
            thread_manager,
            thread_id,
        } = command;
        let thread = thread_manager.get_thread(thread_id).await?;
        thread.submit(Op::Interrupt).await?;
        Ok(())
    }

    pub async fn steer_turn(
        &self,
        command: RuntimeTurnSteerCommand,
    ) -> Result<String, RuntimeCommandBusError> {
        let RuntimeTurnSteerCommand {
            thread_manager,
            thread_id,
            turn_id,
            items,
        } = command;
        let thread = thread_manager.get_thread(thread_id).await?;
        thread
            .steer_input(items, Some(&turn_id), None)
            .await
            .map_err(RuntimeCommandBusError::SteerInput)
    }

    pub async fn start_compact(
        &self,
        command: RuntimeThreadCommand,
    ) -> Result<(), RuntimeCommandBusError> {
        let RuntimeThreadCommand {
            thread_manager,
            thread_id,
        } = command;
        let thread = thread_manager.get_thread(thread_id).await?;
        thread.submit(Op::Compact).await?;
        Ok(())
    }

    pub async fn run_shell_command(
        &self,
        command: RuntimeShellCommand,
    ) -> Result<(), RuntimeCommandBusError> {
        let RuntimeShellCommand {
            thread_manager,
            thread_id,
            command,
        } = command;
        let command = command.trim().to_string();
        if command.is_empty() {
            return Err(RuntimeCommandBusError::EmptyShellCommand);
        }
        let thread = thread_manager.get_thread(thread_id).await?;
        thread.submit(Op::RunUserShellCommand { command }).await?;
        Ok(())
    }

    pub async fn realtime_start(
        &self,
        command: RuntimeRealtimeStartCommand,
    ) -> Result<(), RuntimeCommandBusError> {
        let RuntimeRealtimeStartCommand {
            thread_manager,
            thread_id,
            params,
        } = command;
        let thread = thread_manager.get_thread(thread_id).await?;
        ensure_realtime_conversation_enabled(thread_id, &thread)?;
        thread
            .submit(Op::RealtimeConversationStart(params))
            .await
            .map(|_| ())
            .map_err(RuntimeCommandBusError::from)
    }

    pub async fn realtime_append_audio(
        &self,
        command: RuntimeRealtimeAppendAudioCommand,
    ) -> Result<(), RuntimeCommandBusError> {
        let RuntimeRealtimeAppendAudioCommand {
            thread_manager,
            thread_id,
            params,
        } = command;
        let thread = thread_manager.get_thread(thread_id).await?;
        ensure_realtime_conversation_enabled(thread_id, &thread)?;
        thread
            .submit(Op::RealtimeConversationAudio(params))
            .await
            .map(|_| ())
            .map_err(RuntimeCommandBusError::from)
    }

    pub async fn realtime_stop(
        &self,
        command: RuntimeRealtimeStopCommand,
    ) -> Result<(), RuntimeCommandBusError> {
        let RuntimeRealtimeStopCommand {
            thread_manager,
            thread_id,
        } = command;
        let thread = thread_manager.get_thread(thread_id).await?;
        ensure_realtime_conversation_enabled(thread_id, &thread)?;
        thread
            .submit(Op::RealtimeConversationClose)
            .await
            .map(|_| ())
            .map_err(RuntimeCommandBusError::from)
    }

    pub async fn start_turn(
        &self,
        command: RuntimeTurnStartCommand,
    ) -> Result<String, RuntimeCommandBusError> {
        let RuntimeTurnStartCommand {
            thread_manager,
            thread_id,
            items,
            cwd,
            approval_policy,
            approvals_reviewer,
            sandbox_policy,
            permission_profile,
            active_permission_profile,
            model,
            effort,
            summary,
            service_tier,
            collaboration_mode,
            personality,
            output_schema,
        } = command;
        let thread = thread_manager.get_thread(thread_id).await?;
        thread
            .validate_turn_context_overrides(VACThreadTurnContextOverrides {
                cwd: Some(cwd.clone()),
                approval_policy: Some(approval_policy),
                approvals_reviewer: Some(approvals_reviewer),
                sandbox_policy: sandbox_policy.clone(),
                permission_profile: permission_profile.clone(),
                active_permission_profile: active_permission_profile.clone(),
                windows_sandbox_level: None,
                model: Some(model.clone()),
                effort,
                summary,
                service_tier,
                collaboration_mode: collaboration_mode.clone(),
                personality,
            })
            .await
            .map_err(|err| RuntimeCommandBusError::InvalidTurnContext(err.to_string()))?;

        thread
            .submit(Op::UserInputWithTurnContext {
                items,
                environments: None,
                final_output_json_schema: output_schema,
                responsesapi_client_metadata: None,
                cwd: Some(cwd),
                approval_policy: Some(approval_policy),
                approvals_reviewer: Some(approvals_reviewer),
                sandbox_policy,
                permission_profile,
                active_permission_profile,
                windows_sandbox_level: None,
                model: Some(model),
                effort,
                summary,
                service_tier,
                collaboration_mode,
                personality,
            })
            .await
            .map_err(RuntimeCommandBusError::from)
    }

    pub async fn start_review(
        &self,
        command: RuntimeReviewStartCommand,
    ) -> Result<(String, String), RuntimeCommandBusError> {
        let RuntimeReviewStartCommand {
            thread_manager,
            thread_id,
            target,
        } = command;
        let (review_request, display_text) = review_request_from_target(target)?;
        let thread = thread_manager.get_thread(thread_id).await?;
        let turn_id = thread.submit(Op::Review { review_request }).await?;
        Ok((turn_id, display_text))
    }

    pub async fn get_thread_goal(
        &self,
        thread_manager: Arc<ThreadManager>,
        thread_id: ThreadId,
    ) -> Result<Option<ThreadGoal>, RuntimeCommandBusError> {
        let thread = thread_manager.get_thread(thread_id).await?;
        let state_db = state_db_for_loaded_thread(thread_id, &thread)?;
        state_db
            .get_thread_goal(thread_id)
            .await
            .map(|goal| goal.map(thread_goal_from_state))
            .map_err(|err| RuntimeCommandBusError::ThreadGoalStore(err.to_string()))
    }

    pub async fn set_thread_goal(
        &self,
        command: RuntimeThreadGoalSetCommand,
    ) -> Result<ThreadGoal, RuntimeCommandBusError> {
        let RuntimeThreadGoalSetCommand {
            thread_manager,
            thread_id,
            objective,
            status,
            token_budget,
        } = command;
        let objective = objective.map(|objective| objective.trim().to_string());
        if let Some(objective) = objective.as_deref() {
            validate_thread_goal_objective(objective)
                .map_err(RuntimeCommandBusError::InvalidThreadGoalObjective)?;
            validate_goal_budget(token_budget.flatten())?;
        } else if let Some(token_budget) = token_budget {
            validate_goal_budget(token_budget)?;
        }

        let thread = thread_manager.get_thread(thread_id).await?;
        let state_db = state_db_for_loaded_thread(thread_id, &thread)?;
        let status = status.map(thread_goal_status_to_state);
        thread.prepare_external_goal_mutation().await;
        let goal = if let Some(objective) = objective.as_deref() {
            let existing_goal = state_db
                .get_thread_goal(thread_id)
                .await
                .map_err(|err| RuntimeCommandBusError::ThreadGoalStore(err.to_string()))?;
            if let Some(goal) = existing_goal.as_ref().filter(|goal| {
                goal.objective == objective && goal.status != vac_state::ThreadGoalStatus::Complete
            }) {
                state_db
                    .update_thread_goal(
                        thread_id,
                        vac_state::ThreadGoalUpdate {
                            status,
                            token_budget,
                            expected_goal_id: Some(goal.goal_id.clone()),
                        },
                    )
                    .await
                    .map_err(|err| RuntimeCommandBusError::ThreadGoalStore(err.to_string()))?
                    .ok_or(RuntimeCommandBusError::ThreadGoalMissing(thread_id))?
            } else {
                state_db
                    .replace_thread_goal(
                        thread_id,
                        objective,
                        status.unwrap_or(vac_state::ThreadGoalStatus::Active),
                        token_budget.flatten(),
                    )
                    .await
                    .map_err(|err| RuntimeCommandBusError::ThreadGoalStore(err.to_string()))?
            }
        } else {
            let existing_goal = state_db
                .get_thread_goal(thread_id)
                .await
                .map_err(|err| RuntimeCommandBusError::ThreadGoalStore(err.to_string()))?;
            let expected_goal_id = existing_goal.map(|goal| goal.goal_id);
            state_db
                .update_thread_goal(
                    thread_id,
                    vac_state::ThreadGoalUpdate {
                        status,
                        token_budget,
                        expected_goal_id,
                    },
                )
                .await
                .map_err(|err| RuntimeCommandBusError::ThreadGoalStore(err.to_string()))?
                .ok_or(RuntimeCommandBusError::ThreadGoalMissing(thread_id))?
        };
        let goal_status = goal.status;
        let goal = thread_goal_from_state(goal);
        thread.apply_external_goal_set(goal_status).await;
        Ok(goal)
    }

    pub async fn clear_thread_goal(
        &self,
        thread_manager: Arc<ThreadManager>,
        thread_id: ThreadId,
    ) -> Result<bool, RuntimeCommandBusError> {
        let thread = thread_manager.get_thread(thread_id).await?;
        let state_db = state_db_for_loaded_thread(thread_id, &thread)?;
        thread.prepare_external_goal_mutation().await;
        let cleared = state_db
            .delete_thread_goal(thread_id)
            .await
            .map_err(|err| RuntimeCommandBusError::ThreadGoalStore(err.to_string()))?;
        if cleared {
            thread.apply_external_goal_clear().await;
        }
        Ok(cleared)
    }

    pub async fn reload_config(
        &self,
        thread_manager: Arc<ThreadManager>,
    ) -> Result<(), RuntimeCommandBusError> {
        for thread_id in thread_manager.list_thread_ids().await {
            if let Ok(thread) = thread_manager.get_thread(thread_id).await {
                thread.submit(Op::ReloadUserConfig).await?;
            }
        }
        Ok(())
    }

    pub async fn logout_account(
        &self,
        auth_manager: Arc<AuthManager>,
    ) -> Result<(), RuntimeCommandBusError> {
        auth_manager
            .logout_with_revoke()
            .await
            .map_err(|err| RuntimeCommandBusError::Logout(err.to_string()))?;
        Ok(())
    }

    pub async fn reset_memory(
        &self,
        sqlite_home: PathBuf,
        model_provider_id: String,
        vac_home: PathBuf,
    ) -> Result<(), RuntimeCommandBusError> {
        let state_db = StateRuntime::init(sqlite_home, model_provider_id)
            .await
            .map_err(|err| RuntimeCommandBusError::MemoryReset(err.to_string()))?;
        state_db
            .clear_memory_data()
            .await
            .map_err(|err| RuntimeCommandBusError::MemoryReset(err.to_string()))?;
        vac_memories_write::clear_memory_roots_contents(&vac_home)
            .await
            .map_err(|err| RuntimeCommandBusError::MemoryReset(err.to_string()))?;
        Ok(())
    }

    pub async fn set_thread_memory_mode(
        &self,
        thread_manager: Arc<ThreadManager>,
        thread_store: Option<Arc<dyn ThreadStore>>,
        thread_id: ThreadId,
        mode: ThreadMemoryMode,
    ) -> Result<(), RuntimeCommandBusError> {
        if let Ok(thread) = thread_manager.get_thread(thread_id).await {
            if thread.config_snapshot().await.ephemeral {
                return Err(RuntimeCommandBusError::ThreadMemoryModeSet(format!(
                    "ephemeral thread does not support memory mode updates: {thread_id}"
                )));
            }
            thread
                .set_thread_memory_mode(mode)
                .await
                .map_err(|err| RuntimeCommandBusError::ThreadMemoryModeSet(err.to_string()))?;
            return Ok(());
        }
        if let Some(thread_store) = thread_store {
            thread_store
                .update_thread_metadata(UpdateThreadMetadataParams {
                    thread_id,
                    patch: ThreadMetadataPatch {
                        memory_mode: Some(mode),
                        ..Default::default()
                    },
                    include_archived: false,
                })
                .await
                .map_err(|err| RuntimeCommandBusError::ThreadMemoryModeSet(err.to_string()))?;
            return Ok(());
        }
        Err(RuntimeCommandBusError::ThreadMemoryModeSet(format!(
            "thread not loaded: {thread_id}"
        )))
    }
}

fn required_plugin_thread_manager(
    command: &RuntimePluginSurfaceCommand,
) -> Result<Arc<ThreadManager>, RuntimeCommandBusError> {
    command
        .thread_manager
        .clone()
        .ok_or(RuntimeCommandBusError::PluginSurfaceMissingField {
            operation: command.operation,
            field: "thread_manager",
        })
}

fn required_plugin_auth_manager(
    command: &RuntimePluginSurfaceCommand,
) -> Result<Arc<AuthManager>, RuntimeCommandBusError> {
    command
        .auth_manager
        .clone()
        .ok_or(RuntimeCommandBusError::PluginSurfaceMissingField {
            operation: command.operation,
            field: "auth_manager",
        })
}

fn required_plugin_config(
    command: &RuntimePluginSurfaceCommand,
) -> Result<Arc<Config>, RuntimeCommandBusError> {
    command
        .config
        .clone()
        .ok_or(RuntimeCommandBusError::PluginSurfaceMissingField {
            operation: command.operation,
            field: "config",
        })
}

fn required_plugin_marketplace_path(
    command: &RuntimePluginSurfaceCommand,
) -> Result<AbsolutePathBuf, RuntimeCommandBusError> {
    command
        .marketplace_path
        .clone()
        .ok_or(RuntimeCommandBusError::PluginSurfaceMissingField {
            operation: command.operation,
            field: "marketplace_path",
        })
}

fn required_plugin_name(
    command: &RuntimePluginSurfaceCommand,
) -> Result<String, RuntimeCommandBusError> {
    command
        .plugin_name
        .clone()
        .ok_or(RuntimeCommandBusError::PluginSurfaceMissingField {
            operation: command.operation,
            field: "plugin_name",
        })
}

fn required_plugin_id(
    command: &RuntimePluginSurfaceCommand,
) -> Result<String, RuntimeCommandBusError> {
    command
        .plugin_id
        .clone()
        .ok_or(RuntimeCommandBusError::PluginSurfaceMissingField {
            operation: command.operation,
            field: "plugin_id",
        })
}

async fn scoped_plugin_config(
    config: &Arc<Config>,
    fallback_cwd: Option<PathBuf>,
) -> Result<Config, RuntimeCommandBusError> {
    let Some(fallback_cwd) = fallback_cwd else {
        return Ok((**config).clone());
    };
    if fallback_cwd == config.cwd.to_path_buf() {
        return Ok((**config).clone());
    }
    ConfigBuilder::default()
        .vac_home(config.vac_home.to_path_buf())
        .fallback_cwd(Some(fallback_cwd.clone()))
        .build()
        .await
        .map_err(|err| RuntimeCommandBusError::SkillsConfigResolutionFailed {
            cwd: fallback_cwd,
            message: err.to_string(),
        })
}

fn clear_plugin_related_caches(thread_manager: &Arc<ThreadManager>) {
    thread_manager.plugins_manager().clear_cache();
    thread_manager.skills_manager().clear_cache();
}

fn runtime_external_agent_item_from_provider(
    item: ProviderExternalAgentConfigMigrationItem,
) -> RuntimeExternalAgentConfigMigrationItem {
    RuntimeExternalAgentConfigMigrationItem {
        item_type: runtime_external_agent_item_type_from_provider(item.item_type),
        description: item.description,
        cwd: item.cwd,
        details: item
            .details
            .map(runtime_external_agent_details_from_provider),
    }
}

fn provider_external_agent_item_from_runtime(
    item: RuntimeExternalAgentConfigMigrationItem,
) -> ProviderExternalAgentConfigMigrationItem {
    ProviderExternalAgentConfigMigrationItem {
        item_type: provider_external_agent_item_type_from_runtime(item.item_type),
        description: item.description,
        cwd: item.cwd,
        details: item
            .details
            .map(provider_external_agent_details_from_runtime),
    }
}

fn runtime_external_agent_item_type_from_provider(
    item_type: ProviderExternalAgentConfigMigrationItemType,
) -> RuntimeExternalAgentConfigMigrationItemType {
    match item_type {
        ProviderExternalAgentConfigMigrationItemType::Config => {
            RuntimeExternalAgentConfigMigrationItemType::Config
        }
        ProviderExternalAgentConfigMigrationItemType::Skills => {
            RuntimeExternalAgentConfigMigrationItemType::Skills
        }
        ProviderExternalAgentConfigMigrationItemType::AgentsMd => {
            RuntimeExternalAgentConfigMigrationItemType::AgentsMd
        }
        ProviderExternalAgentConfigMigrationItemType::Plugins => {
            RuntimeExternalAgentConfigMigrationItemType::Plugins
        }
        ProviderExternalAgentConfigMigrationItemType::McpServerConfig => {
            RuntimeExternalAgentConfigMigrationItemType::McpServerConfig
        }
        ProviderExternalAgentConfigMigrationItemType::Subagents => {
            RuntimeExternalAgentConfigMigrationItemType::Subagents
        }
        ProviderExternalAgentConfigMigrationItemType::Hooks => {
            RuntimeExternalAgentConfigMigrationItemType::Hooks
        }
        ProviderExternalAgentConfigMigrationItemType::Commands => {
            RuntimeExternalAgentConfigMigrationItemType::Commands
        }
        ProviderExternalAgentConfigMigrationItemType::Sessions => {
            RuntimeExternalAgentConfigMigrationItemType::Sessions
        }
    }
}

fn provider_external_agent_item_type_from_runtime(
    item_type: RuntimeExternalAgentConfigMigrationItemType,
) -> ProviderExternalAgentConfigMigrationItemType {
    match item_type {
        RuntimeExternalAgentConfigMigrationItemType::Config => {
            ProviderExternalAgentConfigMigrationItemType::Config
        }
        RuntimeExternalAgentConfigMigrationItemType::Skills => {
            ProviderExternalAgentConfigMigrationItemType::Skills
        }
        RuntimeExternalAgentConfigMigrationItemType::AgentsMd => {
            ProviderExternalAgentConfigMigrationItemType::AgentsMd
        }
        RuntimeExternalAgentConfigMigrationItemType::Plugins => {
            ProviderExternalAgentConfigMigrationItemType::Plugins
        }
        RuntimeExternalAgentConfigMigrationItemType::McpServerConfig => {
            ProviderExternalAgentConfigMigrationItemType::McpServerConfig
        }
        RuntimeExternalAgentConfigMigrationItemType::Subagents => {
            ProviderExternalAgentConfigMigrationItemType::Subagents
        }
        RuntimeExternalAgentConfigMigrationItemType::Hooks => {
            ProviderExternalAgentConfigMigrationItemType::Hooks
        }
        RuntimeExternalAgentConfigMigrationItemType::Commands => {
            ProviderExternalAgentConfigMigrationItemType::Commands
        }
        RuntimeExternalAgentConfigMigrationItemType::Sessions => {
            ProviderExternalAgentConfigMigrationItemType::Sessions
        }
    }
}

fn runtime_external_agent_details_from_provider(
    details: ProviderMigrationDetails,
) -> RuntimeExternalAgentConfigMigrationDetails {
    RuntimeExternalAgentConfigMigrationDetails {
        plugins: details
            .plugins
            .into_iter()
            .map(|plugin| RuntimeExternalAgentConfigPluginsMigration {
                marketplace_name: plugin.marketplace_name,
                plugin_names: plugin.plugin_names,
            })
            .collect(),
        sessions: details
            .sessions
            .into_iter()
            .map(|session| RuntimeExternalAgentConfigSessionMigration {
                path: session.path,
                cwd: session.cwd,
                title: session.title,
            })
            .collect(),
        mcp_servers: details
            .mcp_servers
            .into_iter()
            .map(runtime_named_migration_from_provider)
            .collect(),
        hooks: details
            .hooks
            .into_iter()
            .map(runtime_named_migration_from_provider)
            .collect(),
        subagents: details
            .subagents
            .into_iter()
            .map(runtime_named_migration_from_provider)
            .collect(),
        commands: details
            .commands
            .into_iter()
            .map(runtime_named_migration_from_provider)
            .collect(),
    }
}

fn provider_external_agent_details_from_runtime(
    details: RuntimeExternalAgentConfigMigrationDetails,
) -> ProviderMigrationDetails {
    ProviderMigrationDetails {
        plugins: details
            .plugins
            .into_iter()
            .map(|plugin| ProviderPluginsMigration {
                marketplace_name: plugin.marketplace_name,
                plugin_names: plugin.plugin_names,
            })
            .collect(),
        sessions: details
            .sessions
            .into_iter()
            .map(
                |session| vac_external_agent_sessions::ExternalAgentSessionMigration {
                    path: session.path,
                    cwd: session.cwd,
                    title: session.title,
                },
            )
            .collect(),
        mcp_servers: details
            .mcp_servers
            .into_iter()
            .map(provider_named_migration_from_runtime)
            .collect(),
        hooks: details
            .hooks
            .into_iter()
            .map(provider_named_migration_from_runtime)
            .collect(),
        subagents: details
            .subagents
            .into_iter()
            .map(provider_named_migration_from_runtime)
            .collect(),
        commands: details
            .commands
            .into_iter()
            .map(provider_named_migration_from_runtime)
            .collect(),
    }
}

fn runtime_named_migration_from_provider(
    migration: ProviderNamedMigration,
) -> RuntimeExternalAgentConfigNamedMigration {
    RuntimeExternalAgentConfigNamedMigration {
        name: migration.name,
    }
}

fn provider_named_migration_from_runtime(
    migration: RuntimeExternalAgentConfigNamedMigration,
) -> ProviderNamedMigration {
    ProviderNamedMigration {
        name: migration.name,
    }
}

fn absolute_extra_skill_roots_for_cwds(
    cwds: &[PathBuf],
    entries: Vec<RuntimeSkillsListExtraRootsForCwd>,
) -> Result<HashMap<PathBuf, Vec<AbsolutePathBuf>>, RuntimeCommandBusError> {
    let cwd_set = cwds.iter().cloned().collect::<HashSet<_>>();
    let mut extra_roots_by_cwd: HashMap<PathBuf, Vec<AbsolutePathBuf>> = HashMap::new();
    for RuntimeSkillsListExtraRootsForCwd {
        cwd,
        extra_user_roots,
    } in entries
    {
        if !cwd_set.contains(&cwd) {
            continue;
        }
        let mut valid_extra_roots = Vec::new();
        for root in extra_user_roots {
            let root = AbsolutePathBuf::from_absolute_path_checked(root.clone())
                .map_err(|_| RuntimeCommandBusError::SkillsExtraRootNotAbsolute(root.clone()))?;
            valid_extra_roots.push(root);
        }
        extra_roots_by_cwd
            .entry(cwd)
            .or_default()
            .extend(valid_extra_roots);
    }
    Ok(extra_roots_by_cwd)
}

fn runtime_skill_metadata_from_core(
    skills: &[vac_core::skills::SkillMetadata],
    disabled_paths: &HashSet<AbsolutePathBuf>,
) -> Vec<SkillMetadata> {
    skills
        .iter()
        .map(|skill| SkillMetadata {
            name: skill.name.clone(),
            description: skill.description.clone(),
            short_description: skill.short_description.clone(),
            interface: skill.interface.clone().map(|interface| SkillInterface {
                display_name: interface.display_name,
                short_description: interface.short_description,
                icon_small: interface.icon_small,
                icon_large: interface.icon_large,
                brand_color: interface.brand_color,
                default_prompt: interface.default_prompt,
            }),
            dependencies: skill
                .dependencies
                .clone()
                .map(|dependencies| SkillDependencies {
                    tools: dependencies
                        .tools
                        .into_iter()
                        .map(|tool| SkillToolDependency {
                            r#type: tool.r#type,
                            value: tool.value,
                            description: tool.description,
                            transport: tool.transport,
                            command: tool.command,
                            url: tool.url,
                        })
                        .collect(),
                }),
            path: skill.path_to_skills_md.clone(),
            scope: skill.scope,
            enabled: !disabled_paths.contains(&skill.path_to_skills_md),
        })
        .collect()
}

fn runtime_skill_errors_from_core(errors: &[vac_core::skills::SkillError]) -> Vec<SkillErrorInfo> {
    errors
        .iter()
        .map(|err| SkillErrorInfo {
            path: err.path.to_path_buf(),
            message: err.message.clone(),
        })
        .collect()
}

fn review_request_from_target(
    target: ReviewTarget,
) -> Result<(ReviewRequest, String), RuntimeCommandBusError> {
    let target = match target {
        ReviewTarget::UncommittedChanges => ReviewTarget::UncommittedChanges,
        ReviewTarget::BaseBranch { branch } => {
            let branch = branch.trim().to_string();
            if branch.is_empty() {
                return Err(RuntimeCommandBusError::InvalidReviewTarget(
                    "branch must not be empty".to_string(),
                ));
            }
            ReviewTarget::BaseBranch { branch }
        }
        ReviewTarget::Commit { sha, title } => {
            let sha = sha.trim().to_string();
            if sha.is_empty() {
                return Err(RuntimeCommandBusError::InvalidReviewTarget(
                    "sha must not be empty".to_string(),
                ));
            }
            let title = title
                .map(|title| title.trim().to_string())
                .filter(|title| !title.is_empty());
            ReviewTarget::Commit { sha, title }
        }
        ReviewTarget::Custom { instructions } => {
            let instructions = instructions.trim().to_string();
            if instructions.is_empty() {
                return Err(RuntimeCommandBusError::InvalidReviewTarget(
                    "instructions must not be empty".to_string(),
                ));
            }
            ReviewTarget::Custom { instructions }
        }
    };
    let display_text = vac_core::review_prompts::user_facing_hint(&target);
    Ok((
        ReviewRequest {
            target,
            user_facing_hint: Some(display_text.clone()),
        },
        display_text,
    ))
}

fn state_db_for_loaded_thread(
    thread_id: ThreadId,
    thread: &Arc<vac_core::VACThread>,
) -> Result<StateDbHandle, RuntimeCommandBusError> {
    if thread.rollout_path().is_none() {
        return Err(RuntimeCommandBusError::EphemeralThreadGoalsUnsupported(
            thread_id,
        ));
    }
    thread
        .state_db()
        .ok_or(RuntimeCommandBusError::ThreadGoalStateDbUnavailable)
}

fn validate_goal_budget(value: Option<i64>) -> Result<(), RuntimeCommandBusError> {
    if let Some(value) = value
        && value <= 0
    {
        return Err(RuntimeCommandBusError::InvalidThreadGoalBudget);
    }
    Ok(())
}

fn thread_goal_status_to_state(status: ThreadGoalStatus) -> vac_state::ThreadGoalStatus {
    match status {
        ThreadGoalStatus::Active => vac_state::ThreadGoalStatus::Active,
        ThreadGoalStatus::Paused => vac_state::ThreadGoalStatus::Paused,
        ThreadGoalStatus::BudgetLimited => vac_state::ThreadGoalStatus::BudgetLimited,
        ThreadGoalStatus::Complete => vac_state::ThreadGoalStatus::Complete,
    }
}

fn thread_goal_status_from_state(status: vac_state::ThreadGoalStatus) -> ThreadGoalStatus {
    match status {
        vac_state::ThreadGoalStatus::Active => ThreadGoalStatus::Active,
        vac_state::ThreadGoalStatus::Paused => ThreadGoalStatus::Paused,
        vac_state::ThreadGoalStatus::BudgetLimited => ThreadGoalStatus::BudgetLimited,
        vac_state::ThreadGoalStatus::Complete => ThreadGoalStatus::Complete,
    }
}

fn thread_goal_from_state(goal: vac_state::ThreadGoal) -> ThreadGoal {
    ThreadGoal {
        thread_id: goal.thread_id,
        objective: goal.objective,
        status: thread_goal_status_from_state(goal.status),
        token_budget: goal.token_budget,
        tokens_used: goal.tokens_used,
        time_used_seconds: goal.time_used_seconds,
        created_at: goal.created_at.timestamp(),
        updated_at: goal.updated_at.timestamp(),
    }
}

fn read_account_from_auth_manager(
    auth_manager: &Arc<AuthManager>,
    requires_vastar_auth: bool,
) -> Result<RuntimeAccountRead, RuntimeCommandBusError> {
    let auth = auth_manager.auth_cached();
    let account = match auth.as_ref() {
        Some(VACAuth::ApiKey(_)) => Some(RuntimeAccount::ApiKey),
        Some(auth) if auth.uses_vac_backend() => {
            let email = auth
                .get_account_email()
                .ok_or(RuntimeCommandBusError::MissingChatGptAccountDetails)?;
            let plan_type = auth
                .account_plan_type()
                .ok_or(RuntimeCommandBusError::MissingChatGptAccountDetails)?;
            Some(RuntimeAccount::Chatgpt { email, plan_type })
        }
        Some(_) | None => None,
    };
    Ok(RuntimeAccountRead {
        account,
        requires_vastar_auth,
    })
}

fn ensure_realtime_conversation_enabled(
    thread_id: ThreadId,
    thread: &vac_core::VACThread,
) -> Result<(), RuntimeCommandBusError> {
    if !thread.enabled(Feature::RealtimeConversation) {
        return Err(RuntimeCommandBusError::RealtimeConversationUnsupported { thread_id });
    }
    Ok(())
}
