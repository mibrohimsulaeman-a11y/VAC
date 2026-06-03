// Transitional TUI-owned facade over session/protocol DTOs.
//
// 00D kept UI behavior-preserving while moving direct protocol imports behind
// a local module boundary. Plan 31 now replaces the active DTO owner with
// `vac-runtime-protocol`, a product/runtime protocol crate that is not tied to
// the quarantined app-server transport crates.
//
// TUI call sites keep importing through `crate::session_protocol::*`; this file
// is the owner-native facade and no longer forms a default Cargo edge to
// `vac-app-server*`.

#![allow(unused_imports)]

// Owner-migrated DTOs. These are exact re-exports from crates that TUI already
// depends on directly, so moving them here does not change type identity.
pub(crate) use vac_config::ConfigLayerSource;
pub(crate) use vac_core::connectors::AppInfo;
pub(crate) use vac_protocol::RequestId;
pub(crate) use vac_protocol::approvals::GuardianAssessmentDecisionSource;
pub(crate) use vac_protocol::approvals::GuardianCommandSource;
pub(crate) use vac_protocol::approvals::GuardianRiskLevel;
pub(crate) use vac_protocol::approvals::GuardianUserAuthorization;
pub(crate) use vac_protocol::approvals::NetworkApprovalContext;
pub(crate) use vac_protocol::approvals::NetworkApprovalProtocol;
pub(crate) use vac_protocol::config_types::ApprovalsReviewer;
pub(crate) use vac_protocol::config_types::SandboxMode;
pub(crate) use vac_protocol::models::AdditionalPermissionProfile;
pub(crate) use vac_protocol::protocol::AskForApproval;
pub(crate) use vac_protocol::protocol::HookEventName;
pub(crate) use vac_protocol::protocol::HookExecutionMode;
pub(crate) use vac_protocol::protocol::HookHandlerType;
pub(crate) use vac_protocol::protocol::HookScope;
pub(crate) use vac_protocol::protocol::HookSource;
pub(crate) use vac_protocol::protocol::McpAuthStatus;
pub(crate) use vac_protocol::protocol::ModelVerification;
pub(crate) use vac_protocol::protocol::NetworkAccess;
pub(crate) use vac_protocol::protocol::ReviewDelivery;
pub(crate) use vac_protocol::protocol::SessionSource;
pub(crate) use vac_protocol::protocol::SkillScope;
pub(crate) use vac_protocol::protocol::ThreadActiveFlag;
pub(crate) use vac_protocol::protocol::ThreadGoal;
pub(crate) use vac_protocol::protocol::ThreadGoalStatus;
pub(crate) use vac_protocol::protocol::ThreadMemoryMode;
pub(crate) use vac_protocol::protocol::ThreadStatus;
pub(crate) use vac_protocol::request_permissions::PermissionGrantScope;
pub(crate) use vac_protocol::request_permissions::RequestPermissionProfile;
pub(crate) use vac_thread_store::SortDirection;
pub(crate) use vac_thread_store::ThreadSortKey;

pub(crate) use session_protocol_compat::AdditionalFileSystemPermissions as AppServerAdditionalFileSystemPermissions;
pub(crate) use session_protocol_compat::AdditionalNetworkPermissions as AppServerAdditionalNetworkPermissions;
pub(crate) use session_protocol_compat::AdditionalPermissionProfile as AppServerAdditionalPermissionProfile;
pub(crate) use session_protocol_compat::AutoReviewDecisionSource as AppServerAutoReviewDecisionSource;
pub(crate) use session_protocol_compat::GuardianCommandSource as AppServerGuardianCommandSource;
pub(crate) use session_protocol_compat::GuardianRiskLevel as AppServerGuardianRiskLevel;
pub(crate) use session_protocol_compat::GuardianUserAuthorization as AppServerGuardianUserAuthorization;
pub(crate) use session_protocol_compat::HookCompletedNotification as AppServerHookCompletedNotification;
pub(crate) use session_protocol_compat::HookEventName as AppServerHookEventName;
pub(crate) use session_protocol_compat::HookExecutionMode as AppServerHookExecutionMode;
pub(crate) use session_protocol_compat::HookHandlerType as AppServerHookHandlerType;
pub(crate) use session_protocol_compat::HookScope as AppServerHookScope;
pub(crate) use session_protocol_compat::HookStartedNotification as AppServerHookStartedNotification;
pub(crate) use session_protocol_compat::ModelVerification as AppServerModelVerification;
pub(crate) use session_protocol_compat::NetworkAccess as AppServerNetworkAccess;
pub(crate) use session_protocol_compat::NetworkApprovalContext as AppServerNetworkApprovalContext;
pub(crate) use session_protocol_compat::NetworkApprovalProtocol as AppServerNetworkApprovalProtocol;
pub(crate) use session_protocol_compat::ThreadGoal as AppServerThreadGoal;
pub(crate) use session_protocol_compat::ThreadSortKey as AppServerThreadSortKey;
pub(crate) use session_protocol_compat::TurnPlanStepStatus as AppServerTurnPlanStepStatus;

/// Convert legacy app-server thread source filters into canonical session sources.
///
/// `ThreadSourceKind` still belongs to the app-server protocol compatibility
/// seam because it carries legacy/server-only variants (`appServer`,
/// sub-agent categories, `unknown`). Keep that type in the DTO facade for now
/// and expose only this narrow conversion boundary to runtime code.
pub(crate) fn thread_source_kind_to_session_source(
    kind: ThreadSourceKind,
) -> Option<SessionSource> {
    match kind {
        ThreadSourceKind::Cli => Some(SessionSource::Cli),
        ThreadSourceKind::VsCode => Some(SessionSource::VSCode),
        ThreadSourceKind::Exec => Some(SessionSource::Exec),
        ThreadSourceKind::AppServer => None,
        _ => None,
    }
}

pub(crate) fn thread_source_kinds_to_session_sources(
    source_kinds: Option<Vec<ThreadSourceKind>>,
) -> Vec<SessionSource> {
    source_kinds
        .unwrap_or_default()
        .into_iter()
        .filter_map(thread_source_kind_to_session_source)
        .collect()
}

pub(crate) fn thread_goal_from_app_server(goal: AppServerThreadGoal) -> ThreadGoal {
    ThreadGoal {
        thread_id: vac_protocol::ThreadId::from_string(&goal.thread_id).unwrap_or_else(|err| {
            tracing::warn!(
                thread_id = %goal.thread_id,
                error = %err,
                "invalid app-server thread id; using a local placeholder id"
            );
            vac_protocol::ThreadId::new()
        }),
        objective: goal.objective,
        status: goal.status,
        token_budget: goal.token_budget,
        tokens_used: goal.tokens_used,
        time_used_seconds: goal.time_used_seconds,
        created_at: goal.created_at,
        updated_at: goal.updated_at,
    }
}

pub(crate) fn turn_plan_step_status_from_app_server(
    status: AppServerTurnPlanStepStatus,
) -> vac_protocol::plan_tool::StepStatus {
    match status {
        AppServerTurnPlanStepStatus::Pending => vac_protocol::plan_tool::StepStatus::Pending,
        AppServerTurnPlanStepStatus::InProgress => vac_protocol::plan_tool::StepStatus::InProgress,
        AppServerTurnPlanStepStatus::Completed => vac_protocol::plan_tool::StepStatus::Completed,
    }
}

pub(crate) fn guardian_decision_source_from_app_server(
    source: AppServerAutoReviewDecisionSource,
) -> GuardianAssessmentDecisionSource {
    match source {
        AppServerAutoReviewDecisionSource::Agent => GuardianAssessmentDecisionSource::Agent,
    }
}

pub(crate) fn guardian_risk_level_from_app_server(
    risk_level: AppServerGuardianRiskLevel,
) -> GuardianRiskLevel {
    match risk_level {
        AppServerGuardianRiskLevel::Low => GuardianRiskLevel::Low,
        AppServerGuardianRiskLevel::Medium => GuardianRiskLevel::Medium,
        AppServerGuardianRiskLevel::High => GuardianRiskLevel::High,
        AppServerGuardianRiskLevel::Critical => GuardianRiskLevel::Critical,
    }
}

pub(crate) fn guardian_user_authorization_from_app_server(
    authorization: AppServerGuardianUserAuthorization,
) -> GuardianUserAuthorization {
    match authorization {
        AppServerGuardianUserAuthorization::Unknown => GuardianUserAuthorization::Unknown,
        AppServerGuardianUserAuthorization::Low => GuardianUserAuthorization::Low,
        AppServerGuardianUserAuthorization::Medium => GuardianUserAuthorization::Medium,
        AppServerGuardianUserAuthorization::High => GuardianUserAuthorization::High,
    }
}

// Runtime protocol DTOs kept explicit at the TUI facade boundary; do not
// restore a wildcard export from the owner.
pub(crate) use session_protocol_compat::{
    Account, AccountLoginCompletedNotification, AccountUpdatedNotification,
    AddCreditsNudgeCreditType, AddCreditsNudgeEmailStatus, AdditionalFileSystemPermissions,
    AdditionalNetworkPermissions, AgentMessageDeltaNotification, AppSummary, AuthMode, ByteRange,
    CancelLoginAccountParams, CancelLoginAccountResponse, ChatgptAuthTokensRefreshParams,
    ChatgptAuthTokensRefreshReason, ClientInfo, ClientNotification, ClientRequest,
    CollabAgentState, CollabAgentStatus, CollabAgentTool, CollabAgentToolCallStatus, CommandAction,
    CommandExecutionApprovalDecision, CommandExecutionOutputDeltaNotification,
    CommandExecutionRequestApprovalParams, CommandExecutionRequestApprovalResponse,
    CommandExecutionSource, CommandExecutionStatus, ConfigBatchWriteParams, ConfigEdit,
    ConfigRequirementsReadResponse, ConfigValueWriteParams, ConfigWarningNotification,
    ConfigWriteResponse, CreditsSnapshot, DynamicToolCallParams, ErrorNotification,
    ExecPolicyAmendment, ExternalAgentConfigDetectParams, ExternalAgentConfigDetectResponse,
    ExternalAgentConfigImportParams, ExternalAgentConfigImportResponse,
    ExternalAgentConfigMigrationItem, ExternalAgentConfigMigrationItemType, FeedbackUploadParams,
    FeedbackUploadResponse, FileChangeApprovalDecision, FileChangeRequestApprovalParams,
    FileChangeRequestApprovalResponse, FileSystemAccessMode, FileSystemPath,
    FileSystemSandboxEntry, FileSystemSpecialPath, FileUpdateChange, GetAccountParams,
    GetAccountRateLimitsResponse, GetAccountResponse, GitInfo, GrantedPermissionProfile,
    GuardianApprovalReview, GuardianApprovalReviewAction, GuardianApprovalReviewStatus,
    GuardianWarningNotification, HookCompletedNotification, HookErrorInfo, HookMetadata,
    HookOutputEntry, HookOutputEntryKind, HookRunStatus, HookRunSummary, HookStartedNotification,
    HooksListEntry, HooksListParams, HooksListResponse, InitializeCapabilities, InitializeParams,
    ItemCompletedNotification, ItemGuardianApprovalReviewCompletedNotification,
    ItemGuardianApprovalReviewStartedNotification, ItemStartedNotification, JSONRPCErrorError,
    ListMcpServerStatusParams, ListMcpServerStatusResponse, LoginAccountParams,
    LoginAccountResponse, LogoutAccountResponse, MarketplaceAddParams, MarketplaceAddResponse,
    MarketplaceInterface, MarketplaceLoadErrorInfo, MarketplaceRemoveParams,
    MarketplaceRemoveResponse, MarketplaceUpgradeErrorInfo, MarketplaceUpgradeParams,
    MarketplaceUpgradeResponse, McpElicitationEnumSchema, McpElicitationObjectType,
    McpElicitationPrimitiveSchema, McpElicitationSchema, McpElicitationSingleSelectEnumSchema,
    McpServerElicitationAction, McpServerElicitationRequest, McpServerElicitationRequestParams,
    McpServerElicitationRequestResponse, McpServerStartupState, McpServerStatus,
    McpServerStatusDetail, McpServerStatusUpdatedNotification, MemoryResetResponse, MergeStrategy,
    MigrationDetails, Model, ModelListParams, ModelListResponse, ModelVerificationNotification,
    NetworkPolicyAmendment, NetworkPolicyRuleAction, NonSteerableTurnKind, PatchApplyStatus,
    PatchChangeKind, PermissionProfile, PermissionProfileFileSystemPermissions,
    PermissionProfileModificationParams, PermissionProfileNetworkPermissions,
    PermissionProfileSelectionParams, PermissionsRequestApprovalParams,
    PermissionsRequestApprovalResponse, PluginAuthPolicy, PluginAvailability, PluginDetail,
    PluginInstallParams, PluginInstallPolicy, PluginInstallResponse, PluginInterface,
    PluginListParams, PluginListResponse, PluginMarketplaceEntry, PluginReadParams,
    PluginReadResponse, PluginSource, PluginSummary, PluginUninstallParams,
    PluginUninstallResponse, PluginsMigration, RateLimitReachedType, RateLimitSnapshot,
    RateLimitWindow, ReasoningSummaryTextDeltaNotification, Result, ReviewStartParams,
    ReviewStartResponse, SandboxPolicy, SendAddCreditsNudgeEmailParams,
    SendAddCreditsNudgeEmailResponse, ServerNotification, ServerRequest,
    ServerRequestResolvedNotification, SkillDependencies, SkillErrorInfo, SkillInterface,
    SkillMetadata, SkillSummary, SkillToolDependency, SkillsChangedNotification, SkillsListEntry,
    SkillsListExtraRootsForCwd, SkillsListParams, SkillsListResponse,
    TerminalInteractionNotification, TextElement, Thread, ThreadApproveGuardianDeniedActionParams,
    ThreadApproveGuardianDeniedActionResponse, ThreadBackgroundTerminalsCleanParams,
    ThreadBackgroundTerminalsCleanResponse, ThreadBranchParams, ThreadBranchResponse,
    ThreadClosedNotification, ThreadCompactStartParams, ThreadCompactStartResponse,
    ThreadGoalClearParams, ThreadGoalClearResponse, ThreadGoalGetParams, ThreadGoalGetResponse,
    ThreadGoalSetParams, ThreadGoalSetResponse, ThreadGoalUpdatedNotification,
    ThreadInjectItemsParams, ThreadInjectItemsResponse, ThreadItem, ThreadListCwdFilter,
    ThreadListParams, ThreadListResponse, ThreadLoadedListParams, ThreadLoadedListResponse,
    ThreadMemoryModeSetParams, ThreadMemoryModeSetResponse, ThreadNameUpdatedNotification,
    ThreadReadParams, ThreadReadResponse, ThreadRealtimeAppendAudioParams,
    ThreadRealtimeAppendAudioResponse, ThreadRealtimeAudioChunk, ThreadRealtimeClosedNotification,
    ThreadRealtimeErrorNotification, ThreadRealtimeItemAddedNotification,
    ThreadRealtimeOutputAudioDeltaNotification, ThreadRealtimeStartParams,
    ThreadRealtimeStartResponse, ThreadRealtimeStartTransport, ThreadRealtimeStartedNotification,
    ThreadRealtimeStopParams, ThreadRealtimeStopResponse, ThreadResumeParams, ThreadResumeResponse,
    ThreadRollbackParams, ThreadRollbackResponse, ThreadSetNameParams, ThreadSetNameResponse,
    ThreadShellCommandParams, ThreadShellCommandResponse, ThreadSourceKind, ThreadStartParams,
    ThreadStartResponse, ThreadStartSource, ThreadStartedNotification, ThreadTokenUsage,
    ThreadTokenUsageUpdatedNotification, ThreadUnsubscribeParams, ThreadUnsubscribeResponse,
    ThreadUnsubscribeStatus, TokenUsageBreakdown, ToolRequestUserInputAnswer,
    ToolRequestUserInputOption, ToolRequestUserInputParams, ToolRequestUserInputQuestion,
    ToolRequestUserInputResponse, Turn, TurnCompletedNotification, TurnError, TurnInterruptParams,
    TurnInterruptResponse, TurnStartParams, TurnStartResponse, TurnStartedNotification, TurnStatus,
    TurnSteerParams, TurnSteerResponse, UserInput, VACErrorInfo, WarningNotification,
    WebSearchAction, WriteStatus, build_turns_from_rollout_items,
    item_event_to_server_notification,
};

#[path = "session_protocol_compat.rs"]
mod session_protocol_compat;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_source_kind_mapping_only_keeps_product_sources() {
        assert_eq!(
            thread_source_kind_to_session_source(ThreadSourceKind::Cli),
            Some(vac_protocol::protocol::SessionSource::Cli)
        );
        assert_eq!(
            thread_source_kind_to_session_source(ThreadSourceKind::VsCode),
            Some(vac_protocol::protocol::SessionSource::VSCode)
        );
        assert_eq!(
            thread_source_kind_to_session_source(ThreadSourceKind::Exec),
            Some(vac_protocol::protocol::SessionSource::Exec)
        );
        assert_eq!(
            thread_source_kind_to_session_source(ThreadSourceKind::AppServer),
            None
        );
        assert_eq!(
            thread_source_kind_to_session_source(ThreadSourceKind::SubAgent),
            None
        );
    }

    #[test]
    fn thread_source_kinds_to_session_sources_filters_legacy_entries() {
        let sources = thread_source_kinds_to_session_sources(Some(vec![
            ThreadSourceKind::AppServer,
            ThreadSourceKind::Cli,
            ThreadSourceKind::VsCode,
            ThreadSourceKind::Exec,
            ThreadSourceKind::Unknown,
        ]));

        assert_eq!(
            sources,
            vec![
                vac_protocol::protocol::SessionSource::Cli,
                vac_protocol::protocol::SessionSource::VSCode,
                vac_protocol::protocol::SessionSource::Exec,
            ]
        );
    }
}
