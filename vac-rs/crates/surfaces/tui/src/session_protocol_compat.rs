// Owner-native runtime protocol DTO shim for the TUI session facade.
//
// This module imports `vac_runtime_protocol`, not app-server crates.
// `session_protocol.rs` owns the active TUI facade and re-exports only an
// explicit symbol list from this shim while call sites are migrated away from
// app-server-shaped DTO names.

// Private DTO compatibility seam for symbols that still lack a proven
// non-server owner path.
//
// 00D-B32 intentionally made this list explicit. Keep it explicit and do
// not collapse it back to a wildcard export. Move a symbol out of this
// module only when one of these is true:
//
// - the destination crate owns the exact same type identity already used
//   by TUI callers; or
// - the replacement introduces an intentional conversion layer and the
//   affected request/response/event paths are validated.
//
// UX gate: changes here must preserve chat rendering, approvals,
// bottom-pane prompts, thread lifecycle/status, realtime events,
// skills/plugins, account/onboarding notifications, and typed request
// decode behavior. Name matches alone are not enough because many DTOs in
// this seam are wire wrappers around lower-level protocol/core types.

pub(crate) use vac_runtime_protocol::{
    Account, AccountLoginCompletedNotification, AccountUpdatedNotification,
    AddCreditsNudgeCreditType, AddCreditsNudgeEmailStatus, AdditionalFileSystemPermissions,
    AdditionalNetworkPermissions, AdditionalPermissionProfile, AgentMessageDeltaNotification,
    AppSummary, ApprovalsReviewer, AskForApproval, AuthMode, AutoReviewDecisionSource, ByteRange,
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
    GuardianCommandSource, GuardianRiskLevel, GuardianUserAuthorization,
    GuardianWarningNotification, HookCompletedNotification, HookErrorInfo, HookEventName,
    HookExecutionMode, HookHandlerType, HookMetadata, HookOutputEntry, HookOutputEntryKind,
    HookRunStatus, HookRunSummary, HookScope, HookStartedNotification, HooksListEntry,
    HooksListParams, HooksListResponse, InitializeCapabilities, InitializeParams,
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
    MigrationDetails, Model, ModelListParams, ModelListResponse, ModelVerification,
    ModelVerificationNotification, NetworkAccess, NetworkApprovalContext, NetworkApprovalProtocol,
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
    ServerRequestResolvedNotification, SessionSource, SkillDependencies, SkillErrorInfo,
    SkillInterface, SkillMetadata, SkillSummary, SkillToolDependency, SkillsChangedNotification,
    SkillsListEntry, SkillsListExtraRootsForCwd, SkillsListParams, SkillsListResponse,
    TerminalInteractionNotification, TextElement, Thread, ThreadApproveGuardianDeniedActionParams,
    ThreadApproveGuardianDeniedActionResponse, ThreadBackgroundTerminalsCleanParams,
    ThreadBackgroundTerminalsCleanResponse, ThreadBranchParams, ThreadBranchResponse,
    ThreadClosedNotification, ThreadCompactStartParams, ThreadCompactStartResponse, ThreadGoal,
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
    ThreadShellCommandParams, ThreadShellCommandResponse, ThreadSortKey, ThreadSourceKind,
    ThreadStartParams, ThreadStartResponse, ThreadStartSource, ThreadStartedNotification,
    ThreadTokenUsage, ThreadTokenUsageUpdatedNotification, ThreadUnsubscribeParams,
    ThreadUnsubscribeResponse, ThreadUnsubscribeStatus, TokenUsageBreakdown,
    ToolRequestUserInputAnswer, ToolRequestUserInputOption, ToolRequestUserInputParams,
    ToolRequestUserInputQuestion, ToolRequestUserInputResponse, Turn, TurnCompletedNotification,
    TurnError, TurnInterruptParams, TurnInterruptResponse, TurnPlanStepStatus, TurnStartParams,
    TurnStartResponse, TurnStartedNotification, TurnStatus, TurnSteerParams, TurnSteerResponse,
    UserInput, VACErrorInfo, WarningNotification, WebSearchAction, WriteStatus,
    build_turns_from_rollout_items, item_event_to_server_notification,
};
