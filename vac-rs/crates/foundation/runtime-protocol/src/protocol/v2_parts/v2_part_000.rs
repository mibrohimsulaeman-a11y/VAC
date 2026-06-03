// Auto-split by VAC O5/O6 audit remainder hardening.
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::PathBuf;

use crate::RequestId;
use crate::protocol::common::AuthMode;
use crate::protocol::item_builders::convert_patch_changes;
use schemars::JsonSchema;
use schemars::r#gen::SchemaGenerator;
use schemars::schema::InstanceType;
use schemars::schema::Metadata;
use schemars::schema::Schema;
use schemars::schema::SchemaObject;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value as JsonValue;
use serde_with::serde_as;
use thiserror::Error;
use ts_rs::TS;
use vac_experimental_api_macros::ExperimentalApi;
use vac_protocol::account::PlanType;
use vac_protocol::account::ProviderAccount;
use vac_protocol::approvals::ElicitationRequest as CoreElicitationRequest;
use vac_protocol::approvals::ExecPolicyAmendment as CoreExecPolicyAmendment;
use vac_protocol::approvals::GuardianAssessmentAction as CoreGuardianAssessmentAction;
use vac_protocol::approvals::GuardianAssessmentDecisionSource as CoreGuardianAssessmentDecisionSource;
use vac_protocol::approvals::GuardianCommandSource as CoreGuardianCommandSource;
use vac_protocol::approvals::NetworkApprovalContext as CoreNetworkApprovalContext;
use vac_protocol::approvals::NetworkApprovalProtocol as CoreNetworkApprovalProtocol;
use vac_protocol::approvals::NetworkPolicyAmendment as CoreNetworkPolicyAmendment;
use vac_protocol::approvals::NetworkPolicyRuleAction as CoreNetworkPolicyRuleAction;
use vac_protocol::config_types::ApprovalsReviewer as CoreApprovalsReviewer;
use vac_protocol::config_types::CollaborationMode;
use vac_protocol::config_types::CollaborationModeMask as CoreCollaborationModeMask;
use vac_protocol::config_types::ForcedLoginMethod;
use vac_protocol::config_types::ModeKind;
use vac_protocol::config_types::Personality;
use vac_protocol::config_types::ReasoningSummary;
use vac_protocol::config_types::SandboxMode as CoreSandboxMode;
use vac_protocol::config_types::ServiceTier;
use vac_protocol::config_types::Verbosity;
use vac_protocol::config_types::WebSearchMode;
use vac_protocol::config_types::WebSearchToolConfig;
use vac_protocol::items::AgentMessageContent as CoreAgentMessageContent;
use vac_protocol::items::TurnItem as CoreTurnItem;
use vac_protocol::mcp::CallToolResult as CoreMcpCallToolResult;
use vac_protocol::mcp::Resource as McpResource;
pub use vac_protocol::mcp::ResourceContent as McpResourceContent;
use vac_protocol::mcp::ResourceTemplate as McpResourceTemplate;
use vac_protocol::mcp::Tool as McpTool;
use vac_protocol::memory_citation::MemoryCitation as CoreMemoryCitation;
use vac_protocol::memory_citation::MemoryCitationEntry as CoreMemoryCitationEntry;
use vac_protocol::models::ActivePermissionProfile as CoreActivePermissionProfile;
use vac_protocol::models::ActivePermissionProfileModification as CoreActivePermissionProfileModification;
use vac_protocol::models::AdditionalPermissionProfile as CoreAdditionalPermissionProfile;
use vac_protocol::models::FileSystemPermissions as CoreFileSystemPermissions;
use vac_protocol::models::ManagedFileSystemPermissions as CoreManagedFileSystemPermissions;
use vac_protocol::models::MessagePhase;
use vac_protocol::models::NetworkPermissions as CoreNetworkPermissions;
use vac_protocol::models::PermissionProfile as CorePermissionProfile;
use vac_protocol::models::ResponseItem;
use vac_protocol::parse_command::ParsedCommand as CoreParsedCommand;
use vac_protocol::permissions::FileSystemAccessMode as CoreFileSystemAccessMode;
use vac_protocol::permissions::FileSystemPath as CoreFileSystemPath;
use vac_protocol::permissions::FileSystemSandboxEntry as CoreFileSystemSandboxEntry;
use vac_protocol::permissions::FileSystemSpecialPath as CoreFileSystemSpecialPath;
use vac_protocol::permissions::NetworkSandboxPolicy as CoreNetworkSandboxPolicy;
use vac_protocol::plan_tool::PlanItemArg as CorePlanItemArg;
use vac_protocol::plan_tool::StepStatus as CorePlanStepStatus;
use vac_protocol::protocol::AgentStatus as CoreAgentStatus;
use vac_protocol::protocol::AskForApproval as CoreAskForApproval;
use vac_protocol::protocol::CreditsSnapshot as CoreCreditsSnapshot;
use vac_protocol::protocol::ExecCommandSource as CoreExecCommandSource;
use vac_protocol::protocol::ExecCommandStatus as CoreExecCommandStatus;
use vac_protocol::protocol::GranularApprovalConfig as CoreGranularApprovalConfig;
use vac_protocol::protocol::GuardianRiskLevel as CoreGuardianRiskLevel;
use vac_protocol::protocol::GuardianUserAuthorization as CoreGuardianUserAuthorization;
use vac_protocol::protocol::HookEventName as CoreHookEventName;
use vac_protocol::protocol::HookExecutionMode as CoreHookExecutionMode;
use vac_protocol::protocol::HookHandlerType as CoreHookHandlerType;
use vac_protocol::protocol::HookOutputEntry as CoreHookOutputEntry;
use vac_protocol::protocol::HookOutputEntryKind as CoreHookOutputEntryKind;
use vac_protocol::protocol::HookRunStatus as CoreHookRunStatus;
use vac_protocol::protocol::HookRunSummary as CoreHookRunSummary;
use vac_protocol::protocol::HookScope as CoreHookScope;
use vac_protocol::protocol::HookSource as CoreHookSource;
use vac_protocol::protocol::ModelRerouteReason as CoreModelRerouteReason;
use vac_protocol::protocol::ModelVerification as CoreModelVerification;
use vac_protocol::protocol::NetworkAccess as CoreNetworkAccess;
use vac_protocol::protocol::NonSteerableTurnKind as CoreNonSteerableTurnKind;
use vac_protocol::protocol::PatchApplyStatus as CorePatchApplyStatus;
use vac_protocol::protocol::RateLimitReachedType as CoreRateLimitReachedType;
use vac_protocol::protocol::RateLimitSnapshot as CoreRateLimitSnapshot;
use vac_protocol::protocol::RateLimitWindow as CoreRateLimitWindow;
use vac_protocol::protocol::RealtimeAudioFrame as CoreRealtimeAudioFrame;
use vac_protocol::protocol::RealtimeConversationVersion;
use vac_protocol::protocol::RealtimeOutputModality;
use vac_protocol::protocol::RealtimeVoice;
use vac_protocol::protocol::RealtimeVoicesList;
use vac_protocol::protocol::ReviewDecision as CoreReviewDecision;
use vac_protocol::protocol::SessionSource as CoreSessionSource;
use vac_protocol::protocol::SkillDependencies as CoreSkillDependencies;
use vac_protocol::protocol::SkillInterface as CoreSkillInterface;
use vac_protocol::protocol::SkillMetadata as CoreSkillMetadata;
use vac_protocol::protocol::SkillScope as CoreSkillScope;
use vac_protocol::protocol::SkillToolDependency as CoreSkillToolDependency;
use vac_protocol::protocol::SubAgentSource as CoreSubAgentSource;
use vac_protocol::protocol::TokenUsage as CoreTokenUsage;
use vac_protocol::protocol::TokenUsageInfo as CoreTokenUsageInfo;
use vac_protocol::protocol::VACErrorInfo as CoreVACErrorInfo;
use vac_protocol::request_permissions::PermissionGrantScope as CorePermissionGrantScope;
use vac_protocol::request_permissions::RequestPermissionProfile as CoreRequestPermissionProfile;
use vac_protocol::user_input::ByteRange as CoreByteRange;
use vac_protocol::user_input::TextElement as CoreTextElement;
use vac_protocol::user_input::UserInput as CoreUserInput;
use vac_protocol::vastar_models::InputModality;
use vac_protocol::vastar_models::ModelAvailabilityNux as CoreModelAvailabilityNux;
use vac_protocol::vastar_models::ReasoningEffort;
use vac_protocol::vastar_models::default_input_modalities;
use vac_utils_absolute_path::AbsolutePathBuf;

// Macro to declare a camelCased API v2 enum mirroring a core enum which
// tends to use either snake_case or kebab-case.
macro_rules! v2_enum_from_core {
    (
        $(#[$enum_meta:meta])*
        pub enum $Name:ident from $Src:path {
            $( $(#[$variant_meta:meta])* $Variant:ident ),+ $(,)?
        }
    ) => {
        #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
        $(#[$enum_meta])*
        #[serde(rename_all = "camelCase")]
        #[ts(export_to = "v2/")]
        pub enum $Name {
            $( $(#[$variant_meta])* $Variant ),+
        }

        impl $Name {
            pub fn to_core(self) -> $Src {
                match self { $( $Name::$Variant => <$Src>::$Variant ),+ }
            }
        }

        impl From<$Src> for $Name {
            fn from(value: $Src) -> Self {
                match value { $( <$Src>::$Variant => $Name::$Variant ),+ }
            }
        }
    };
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum NonSteerableTurnKind {
    Review,
    Compact,
}

/// This translation layer make sure that we expose vac error code in camel case.
///
/// When an upstream HTTP status is available (for example, from the Responses API or a provider),
/// it is forwarded in `httpStatusCode` on the relevant `vacErrorInfo` variant.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum VACErrorInfo {
    ContextWindowExceeded,
    UsageLimitExceeded,
    ServerOverloaded,
    CyberPolicy,
    HttpConnectionFailed {
        #[serde(rename = "httpStatusCode")]
        #[ts(rename = "httpStatusCode")]
        http_status_code: Option<u16>,
    },
    /// Failed to connect to the response SSE stream.
    ResponseStreamConnectionFailed {
        #[serde(rename = "httpStatusCode")]
        #[ts(rename = "httpStatusCode")]
        http_status_code: Option<u16>,
    },
    InternalServerError,
    Unauthorized,
    BadRequest,
    ThreadRollbackFailed,
    SandboxError,
    /// The response SSE stream disconnected in the middle of a turn before completion.
    ResponseStreamDisconnected {
        #[serde(rename = "httpStatusCode")]
        #[ts(rename = "httpStatusCode")]
        http_status_code: Option<u16>,
    },
    /// Reached the retry limit for responses.
    ResponseTooManyFailedAttempts {
        #[serde(rename = "httpStatusCode")]
        #[ts(rename = "httpStatusCode")]
        http_status_code: Option<u16>,
    },
    /// Returned when `turn/start` or `turn/steer` is submitted while the current active turn
    /// cannot accept same-turn steering, for example `/review` or manual `/compact`.
    ActiveTurnNotSteerable {
        #[serde(rename = "turnKind")]
        #[ts(rename = "turnKind")]
        turn_kind: NonSteerableTurnKind,
    },
    Other,
}

impl From<CoreVACErrorInfo> for VACErrorInfo {
    fn from(value: CoreVACErrorInfo) -> Self {
        match value {
            CoreVACErrorInfo::ContextWindowExceeded => VACErrorInfo::ContextWindowExceeded,
            CoreVACErrorInfo::UsageLimitExceeded => VACErrorInfo::UsageLimitExceeded,
            CoreVACErrorInfo::ServerOverloaded => VACErrorInfo::ServerOverloaded,
            CoreVACErrorInfo::CyberPolicy => VACErrorInfo::CyberPolicy,
            CoreVACErrorInfo::HttpConnectionFailed { http_status_code } => {
                VACErrorInfo::HttpConnectionFailed { http_status_code }
            }
            CoreVACErrorInfo::ResponseStreamConnectionFailed { http_status_code } => {
                VACErrorInfo::ResponseStreamConnectionFailed { http_status_code }
            }
            CoreVACErrorInfo::InternalServerError => VACErrorInfo::InternalServerError,
            CoreVACErrorInfo::Unauthorized => VACErrorInfo::Unauthorized,
            CoreVACErrorInfo::BadRequest => VACErrorInfo::BadRequest,
            CoreVACErrorInfo::ThreadRollbackFailed => VACErrorInfo::ThreadRollbackFailed,
            CoreVACErrorInfo::SandboxError => VACErrorInfo::SandboxError,
            CoreVACErrorInfo::ResponseStreamDisconnected { http_status_code } => {
                VACErrorInfo::ResponseStreamDisconnected { http_status_code }
            }
            CoreVACErrorInfo::ResponseTooManyFailedAttempts { http_status_code } => {
                VACErrorInfo::ResponseTooManyFailedAttempts { http_status_code }
            }
            CoreVACErrorInfo::ActiveTurnNotSteerable { turn_kind } => {
                VACErrorInfo::ActiveTurnNotSteerable {
                    turn_kind: turn_kind.into(),
                }
            }
            CoreVACErrorInfo::Other => VACErrorInfo::Other,
        }
    }
}

impl From<CoreNonSteerableTurnKind> for NonSteerableTurnKind {
    fn from(value: CoreNonSteerableTurnKind) -> Self {
        match value {
            CoreNonSteerableTurnKind::Review => Self::Review,
            CoreNonSteerableTurnKind::Compact => Self::Compact,
        }
    }
}

#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS, ExperimentalApi,
)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case", export_to = "v2/")]
pub enum AskForApproval {
    #[serde(rename = "untrusted")]
    #[ts(rename = "untrusted")]
    UnlessTrusted,
    OnFailure,
    OnRequest,
    #[experimental("askForApproval.granular")]
    Granular {
        sandbox_approval: bool,
        rules: bool,
        #[serde(default)]
        skill_approval: bool,
        #[serde(default)]
        request_permissions: bool,
        mcp_elicitations: bool,
    },
    Never,
}

impl AskForApproval {
    pub fn to_core(self) -> CoreAskForApproval {
        match self {
            AskForApproval::UnlessTrusted => CoreAskForApproval::UnlessTrusted,
            AskForApproval::OnFailure => CoreAskForApproval::OnFailure,
            AskForApproval::OnRequest => CoreAskForApproval::OnRequest,
            AskForApproval::Granular {
                sandbox_approval,
                rules,
                skill_approval,
                request_permissions,
                mcp_elicitations,
            } => CoreAskForApproval::Granular(CoreGranularApprovalConfig {
                sandbox_approval,
                rules,
                skill_approval,
                request_permissions,
                mcp_elicitations,
            }),
            AskForApproval::Never => CoreAskForApproval::Never,
        }
    }
}

impl From<CoreAskForApproval> for AskForApproval {
    fn from(value: CoreAskForApproval) -> Self {
        match value {
            CoreAskForApproval::UnlessTrusted => AskForApproval::UnlessTrusted,
            CoreAskForApproval::OnFailure => AskForApproval::OnFailure,
            CoreAskForApproval::OnRequest => AskForApproval::OnRequest,
            CoreAskForApproval::Granular(granular_config) => AskForApproval::Granular {
                sandbox_approval: granular_config.sandbox_approval,
                rules: granular_config.rules,
                skill_approval: granular_config.skill_approval,
                request_permissions: granular_config.request_permissions,
                mcp_elicitations: granular_config.mcp_elicitations,
            },
            CoreAskForApproval::Never => AskForApproval::Never,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, TS)]
#[ts(
    type = r#""user" | "auto_review" | "guardian_subagent""#,
    export_to = "v2/"
)]
/// Configures who approval requests are routed to for review. Examples
/// include sandbox escapes, blocked network access, MCP approval prompts, and
/// ARC escalations. Defaults to `user`. `auto_review` uses a carefully
/// prompted subagent to gather relevant context and apply a risk-based
/// decision framework before approving or denying the request.
pub enum ApprovalsReviewer {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "guardian_subagent", alias = "auto_review")]
    AutoReview,
}

impl JsonSchema for ApprovalsReviewer {
    fn schema_name() -> String {
        "ApprovalsReviewer".to_string()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        string_enum_schema_with_description(
            &["user", "auto_review", "guardian_subagent"],
            "Configures who approval requests are routed to for review. Examples include sandbox escapes, blocked network access, MCP approval prompts, and ARC escalations. Defaults to `user`. `auto_review` uses a carefully prompted subagent to gather relevant context and apply a risk-based decision framework before approving or denying the request. The legacy value `guardian_subagent` is accepted for compatibility.",
        )
    }
}

fn string_enum_schema_with_description(values: &[&str], description: &str) -> Schema {
    let mut schema = SchemaObject {
        instance_type: Some(InstanceType::String.into()),
        metadata: Some(Box::new(Metadata {
            description: Some(description.to_string()),
            ..Default::default()
        })),
        ..Default::default()
    };
    schema.enum_values = Some(
        values
            .iter()
            .map(|value| JsonValue::String((*value).to_string()))
            .collect(),
    );
    Schema::Object(schema)
}

impl ApprovalsReviewer {
    pub fn to_core(self) -> CoreApprovalsReviewer {
        match self {
            ApprovalsReviewer::User => CoreApprovalsReviewer::User,
            ApprovalsReviewer::AutoReview => CoreApprovalsReviewer::AutoReview,
        }
    }
}

impl From<CoreApprovalsReviewer> for ApprovalsReviewer {
    fn from(value: CoreApprovalsReviewer) -> Self {
        match value {
            CoreApprovalsReviewer::User => ApprovalsReviewer::User,
            CoreApprovalsReviewer::AutoReview => ApprovalsReviewer::AutoReview,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case", export_to = "v2/")]
pub enum SandboxMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

impl SandboxMode {
    pub fn to_core(self) -> CoreSandboxMode {
        match self {
            SandboxMode::ReadOnly => CoreSandboxMode::ReadOnly,
            SandboxMode::WorkspaceWrite => CoreSandboxMode::WorkspaceWrite,
            SandboxMode::DangerFullAccess => CoreSandboxMode::DangerFullAccess,
        }
    }
}

impl From<CoreSandboxMode> for SandboxMode {
    fn from(value: CoreSandboxMode) -> Self {
        match value {
            CoreSandboxMode::ReadOnly => SandboxMode::ReadOnly,
            CoreSandboxMode::WorkspaceWrite => SandboxMode::WorkspaceWrite,
            CoreSandboxMode::DangerFullAccess => SandboxMode::DangerFullAccess,
        }
    }
}

v2_enum_from_core!(
    pub enum ReviewDelivery from vac_protocol::protocol::ReviewDelivery {
        Inline, Detached
    }
);

v2_enum_from_core!(
    pub enum McpAuthStatus from vac_protocol::protocol::McpAuthStatus {
        Unsupported,
        NotLoggedIn,
        BearerToken,
        OAuth
    }
);

v2_enum_from_core!(
    pub enum ModelRerouteReason from CoreModelRerouteReason {
        HighRiskCyberActivity
    }
);

v2_enum_from_core!(
    pub enum ModelVerification from CoreModelVerification {
        TrustedAccessForCyber
    }
);

v2_enum_from_core!(
    pub enum HookEventName from CoreHookEventName {
        PreToolUse, PermissionRequest, PostToolUse, SessionStart, UserPromptSubmit, Stop
    }
);

v2_enum_from_core!(
    pub enum HookHandlerType from CoreHookHandlerType {
        Command, Prompt, Agent
    }
);

v2_enum_from_core!(
    pub enum HookExecutionMode from CoreHookExecutionMode {
        Sync, Async
    }
);

v2_enum_from_core!(
    pub enum HookScope from CoreHookScope {
        Thread, Turn
    }
);

v2_enum_from_core!(
    pub enum HookSource from CoreHookSource {
        System,
        User,
        Project,
        Mdm,
        SessionFlags,
        Plugin,
        LegacyManagedConfigFile,
        LegacyManagedConfigMdm,
        Unknown,
    }
);

fn default_hook_source() -> HookSource {
    HookSource::Unknown
}

v2_enum_from_core!(
    pub enum HookRunStatus from CoreHookRunStatus {
        Running, Completed, Failed, Blocked, Stopped
    }
);

v2_enum_from_core!(
    pub enum HookOutputEntryKind from CoreHookOutputEntryKind {
        Warning, Stop, Feedback, Context, Error
    }
);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum ThreadStartSource {
    Startup,
    Clear,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct HookOutputEntry {
    pub kind: HookOutputEntryKind,
    pub text: String,
}

impl From<CoreHookOutputEntry> for HookOutputEntry {
    fn from(value: CoreHookOutputEntry) -> Self {
        Self {
            kind: value.kind.into(),
            text: value.text,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct HookRunSummary {
    pub id: String,
    pub event_name: HookEventName,
    pub handler_type: HookHandlerType,
    pub execution_mode: HookExecutionMode,
    pub scope: HookScope,
    pub source_path: AbsolutePathBuf,
    #[serde(default = "default_hook_source")]
    pub source: HookSource,
    pub display_order: i64,
    pub status: HookRunStatus,
    pub status_message: Option<String>,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub duration_ms: Option<i64>,
    pub entries: Vec<HookOutputEntry>,
}

impl From<CoreHookRunSummary> for HookRunSummary {
    fn from(value: CoreHookRunSummary) -> Self {
        Self {
            id: value.id,
            event_name: value.event_name.into(),
            handler_type: value.handler_type.into(),
            execution_mode: value.execution_mode.into(),
            scope: value.scope.into(),
            source_path: value.source_path,
            source: value.source.into(),
            display_order: value.display_order,
            status: value.status.into(),
            status_message: value.status_message,
            started_at: value.started_at,
            completed_at: value.completed_at,
            duration_ms: value.duration_ms,
            entries: value.entries.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(tag = "type")]
#[ts(export_to = "v2/")]
pub enum ConfigLayerSource {
    /// Managed preferences layer delivered by MDM (macOS only).
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Mdm {
        domain: String,
        key: String,
    },

    /// Managed config layer from a file (usually `managed_config.toml`).
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    System {
        /// This is the path to the system config.toml file, though it is not
        /// guaranteed to exist.
        file: AbsolutePathBuf,
    },

    /// User config layer from $VAC_HOME/config.toml. This layer is special
    /// in that it is expected to be:
    /// - writable by the user
    /// - generally outside the workspace directory
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    User {
        /// This is the path to the user's config.toml file, though it is not
        /// guaranteed to exist.
        file: AbsolutePathBuf,
    },

    /// Path to a .vac/ folder within a project. There could be multiple of
    /// these between `cwd` and the project/repo root.
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Project {
        dot_vac_folder: AbsolutePathBuf,
    },

    /// Session-layer overrides supplied via `-c`/`--config`.
    SessionFlags,

    /// `managed_config.toml` was designed to be a config that was loaded
    /// as the last layer on top of everything else. This scheme did not quite
    /// work out as intended, but we keep this variant as a "best effort" while
    /// we phase out `managed_config.toml` in favor of `requirements.toml`.
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    LegacyManagedConfigTomlFromFile {
        file: AbsolutePathBuf,
    },

    LegacyManagedConfigTomlFromMdm,
}

impl ConfigLayerSource {
    /// A settings from a layer with a higher precedence will override a setting
    /// from a layer with a lower precedence.
    pub fn precedence(&self) -> i16 {
        match self {
            ConfigLayerSource::Mdm { .. } => 0,
            ConfigLayerSource::System { .. } => 10,
            ConfigLayerSource::User { .. } => 20,
            ConfigLayerSource::Project { .. } => 25,
            ConfigLayerSource::SessionFlags => 30,
            ConfigLayerSource::LegacyManagedConfigTomlFromFile { .. } => 40,
            ConfigLayerSource::LegacyManagedConfigTomlFromMdm => 50,
        }
    }
}

/// Compares [ConfigLayerSource] by precedence, so `A < B` means settings from
/// layer `A` will be overridden by settings from layer `B`.
impl PartialOrd for ConfigLayerSource {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.precedence().cmp(&other.precedence()))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub struct SandboxWorkspaceWrite {
    #[serde(default)]
    pub writable_roots: Vec<PathBuf>,
    #[serde(default)]
    pub network_access: bool,
    #[serde(default)]
    pub exclude_tmpdir_env_var: bool,
    #[serde(default)]
    pub exclude_slash_tmp: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub struct ToolsV2 {
    pub web_search: Option<WebSearchToolConfig>,
    pub view_image: Option<bool>,
}

#[derive(Serialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DynamicToolSpec {
    #[ts(optional)]
    pub namespace: Option<String>,
    pub name: String,
    pub description: String,
    pub input_schema: JsonValue,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub defer_loading: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DynamicToolSpecDe {
    namespace: Option<String>,
    name: String,
    description: String,
    input_schema: JsonValue,
    defer_loading: Option<bool>,
    expose_to_context: Option<bool>,
}

impl<'de> Deserialize<'de> for DynamicToolSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let DynamicToolSpecDe {
            namespace,
            name,
            description,
            input_schema,
            defer_loading,
            expose_to_context,
        } = DynamicToolSpecDe::deserialize(deserializer)?;

        Ok(Self {
            namespace,
            name,
            description,
            input_schema,
            defer_loading: defer_loading
                .unwrap_or_else(|| expose_to_context.map(|visible| !visible).unwrap_or(false)),
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS, ExperimentalApi)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub struct ProfileV2 {
    pub model: Option<String>,
    pub model_provider: Option<String>,
    #[experimental(nested)]
    pub approval_policy: Option<AskForApproval>,
    /// [UNSTABLE] Optional profile-level override for where approval requests
    /// are routed for review. If omitted, the enclosing config default is
    /// used.
    #[experimental("config/read.approvalsReviewer")]
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    pub service_tier: Option<ServiceTier>,
    pub model_reasoning_effort: Option<ReasoningEffort>,
    pub model_reasoning_summary: Option<ReasoningSummary>,
    pub model_verbosity: Option<Verbosity>,
    pub web_search: Option<WebSearchMode>,
    pub tools: Option<ToolsV2>,
    pub chatgpt_base_url: Option<String>,
    #[serde(default, flatten)]
    pub additional: HashMap<String, JsonValue>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub struct AnalyticsConfig {
    pub enabled: Option<bool>,
    #[serde(default, flatten)]
    pub additional: HashMap<String, JsonValue>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum AppToolApproval {
    Auto,
    Prompt,
    Approve,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub struct AppsDefaultConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_enabled")]
    pub destructive_enabled: bool,
    #[serde(default = "default_enabled")]
    pub open_world_enabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub struct AppToolConfig {
    pub enabled: Option<bool>,
    pub approval_mode: Option<AppToolApproval>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub struct AppToolsConfig {
    #[serde(default, flatten)]
    pub tools: HashMap<String, AppToolConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub struct AppConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub destructive_enabled: Option<bool>,
    pub open_world_enabled: Option<bool>,
    pub default_tools_approval_mode: Option<AppToolApproval>,
    pub default_tools_enabled: Option<bool>,
    pub tools: Option<AppToolsConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub struct AppsConfig {
    #[serde(default, rename = "_default")]
    pub default: Option<AppsDefaultConfig>,
    #[serde(default, flatten)]
    pub apps: HashMap<String, AppConfig>,
}

const fn default_enabled() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS, ExperimentalApi)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub struct Config {
    pub model: Option<String>,
    pub review_model: Option<String>,
    pub model_context_window: Option<i64>,
    pub model_auto_compact_token_limit: Option<i64>,
    pub model_provider: Option<String>,
    #[experimental(nested)]
    pub approval_policy: Option<AskForApproval>,
    /// [UNSTABLE] Optional default for where approval requests are routed for
    /// review.
    #[experimental("config/read.approvalsReviewer")]
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    pub sandbox_mode: Option<SandboxMode>,
    pub sandbox_workspace_write: Option<SandboxWorkspaceWrite>,
    pub forced_chatgpt_workspace_id: Option<String>,
    pub forced_login_method: Option<ForcedLoginMethod>,
    pub web_search: Option<WebSearchMode>,
    pub tools: Option<ToolsV2>,
    pub profile: Option<String>,
    #[experimental(nested)]
    #[serde(default)]
    pub profiles: HashMap<String, ProfileV2>,
    pub instructions: Option<String>,
    pub developer_instructions: Option<String>,
    pub compact_prompt: Option<String>,
    pub model_reasoning_effort: Option<ReasoningEffort>,
    pub model_reasoning_summary: Option<ReasoningSummary>,
    pub model_verbosity: Option<Verbosity>,
    pub service_tier: Option<ServiceTier>,
    pub analytics: Option<AnalyticsConfig>,
    #[experimental("config/read.apps")]
    #[serde(default)]
    pub apps: Option<AppsConfig>,
    #[serde(default, flatten)]
    pub additional: HashMap<String, JsonValue>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigLayerMetadata {
    pub name: ConfigLayerSource,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigLayer {
    pub name: ConfigLayerSource,
    pub version: String,
    pub config: JsonValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled_reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum MergeStrategy {
    Replace,
    Upsert,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum WriteStatus {
    Ok,
    OkOverridden,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct OverriddenMetadata {
    pub message: String,
    pub overriding_layer: ConfigLayerMetadata,
    pub effective_value: JsonValue,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigWriteResponse {
    pub status: WriteStatus,
    pub version: String,
    /// Canonical path to the config file that was written.
    pub file_path: AbsolutePathBuf,
    pub overridden_metadata: Option<OverriddenMetadata>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum ConfigWriteErrorCode {
    ConfigLayerReadonly,
    ConfigVersionConflict,
    ConfigValidationError,
    ConfigPathNotFound,
    ConfigSchemaUnknownKey,
    UserLayerNotFound,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigReadParams {
    #[serde(default)]
    pub include_layers: bool,
    /// Optional working directory to resolve project config layers. If specified,
    /// return the effective config as seen from that directory (i.e., including any
    /// project layers between `cwd` and the project/repo root).
    #[ts(optional = nullable)]
    pub cwd: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS, ExperimentalApi)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigReadResponse {
    #[experimental(nested)]
    pub config: Config,
    pub origins: HashMap<String, ConfigLayerMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layers: Option<Vec<ConfigLayer>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS, ExperimentalApi)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigRequirements {
    #[experimental(nested)]
    pub allowed_approval_policies: Option<Vec<AskForApproval>>,
    #[experimental("configRequirements/read.allowedApprovalsReviewers")]
    pub allowed_approvals_reviewers: Option<Vec<ApprovalsReviewer>>,
    pub allowed_sandbox_modes: Option<Vec<SandboxMode>>,
    pub allowed_web_search_modes: Option<Vec<WebSearchMode>>,
    pub feature_requirements: Option<BTreeMap<String, bool>>,
    #[experimental("configRequirements/read.hooks")]
    pub hooks: Option<ManagedHooksRequirements>,
    pub enforce_residency: Option<ResidencyRequirement>,
    #[experimental("configRequirements/read.network")]
    pub network: Option<NetworkRequirements>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ManagedHooksRequirements {
    pub managed_dir: Option<PathBuf>,
    pub windows_managed_dir: Option<PathBuf>,
    #[serde(rename = "PreToolUse")]
    #[ts(rename = "PreToolUse")]
    pub pre_tool_use: Vec<ConfiguredHookMatcherGroup>,
    #[serde(rename = "PermissionRequest")]
    #[ts(rename = "PermissionRequest")]
    pub permission_request: Vec<ConfiguredHookMatcherGroup>,
    #[serde(rename = "PostToolUse")]
    #[ts(rename = "PostToolUse")]
    pub post_tool_use: Vec<ConfiguredHookMatcherGroup>,
    #[serde(rename = "SessionStart")]
    #[ts(rename = "SessionStart")]
    pub session_start: Vec<ConfiguredHookMatcherGroup>,
    #[serde(rename = "UserPromptSubmit")]
    #[ts(rename = "UserPromptSubmit")]
    pub user_prompt_submit: Vec<ConfiguredHookMatcherGroup>,
    #[serde(rename = "Stop")]
    #[ts(rename = "Stop")]
    pub stop: Vec<ConfiguredHookMatcherGroup>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfiguredHookMatcherGroup {
    pub matcher: Option<String>,
    pub hooks: Vec<ConfiguredHookHandler>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(tag = "type")]
#[ts(tag = "type", export_to = "v2/")]
pub enum ConfiguredHookHandler {
    #[serde(rename = "command")]
    #[ts(rename = "command")]
    Command {
        command: String,
        #[serde(rename = "timeoutSec")]
        #[ts(rename = "timeoutSec")]
        timeout_sec: Option<u64>,
        r#async: bool,
        #[serde(rename = "statusMessage")]
        #[ts(rename = "statusMessage")]
        status_message: Option<String>,
    },
    #[serde(rename = "prompt")]
    #[ts(rename = "prompt")]
    Prompt {},
    #[serde(rename = "agent")]
    #[ts(rename = "agent")]
    Agent {},
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct NetworkRequirements {
    pub enabled: Option<bool>,
    pub http_port: Option<u16>,
    pub socks_port: Option<u16>,
    pub allow_upstream_proxy: Option<bool>,
    pub dangerously_allow_non_loopback_proxy: Option<bool>,
    pub dangerously_allow_all_unix_sockets: Option<bool>,
    /// Canonical network permission map for `experimental_network`.
    pub domains: Option<BTreeMap<String, NetworkDomainPermission>>,
    /// When true, only managed allowlist entries are respected while managed
    /// network enforcement is active.
    pub managed_allowed_domains_only: Option<bool>,
    /// Legacy compatibility view derived from `domains`.
    pub allowed_domains: Option<Vec<String>>,
    /// Legacy compatibility view derived from `domains`.
    pub denied_domains: Option<Vec<String>>,
    /// Canonical unix socket permission map for `experimental_network`.
    pub unix_sockets: Option<BTreeMap<String, NetworkUnixSocketPermission>>,
    /// Legacy compatibility view derived from `unix_sockets`.
    pub allow_unix_sockets: Option<Vec<String>>,
    pub allow_local_binding: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "v2/")]
pub enum NetworkDomainPermission {
    Allow,
    Deny,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "v2/")]
pub enum NetworkUnixSocketPermission {
    Allow,
    None,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum ResidencyRequirement {
    Us,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS, ExperimentalApi)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigRequirementsReadResponse {
    /// Null if no requirements are configured (e.g. no requirements.toml/MDM entries).
    #[experimental(nested)]
    pub requirements: Option<ConfigRequirements>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, TS)]
#[ts(export_to = "v2/")]
pub enum ExternalAgentConfigMigrationItemType {
    #[serde(rename = "AGENTS_MD")]
    #[ts(rename = "AGENTS_MD")]
    AgentsMd,
    #[serde(rename = "CONFIG")]
    #[ts(rename = "CONFIG")]
    Config,
    #[serde(rename = "SKILLS")]
    #[ts(rename = "SKILLS")]
    Skills,
    #[serde(rename = "PLUGINS")]
    #[ts(rename = "PLUGINS")]
    Plugins,
    #[serde(rename = "MCP_SERVER_CONFIG")]
    #[ts(rename = "MCP_SERVER_CONFIG")]
    McpServerConfig,
    #[serde(rename = "SUBAGENTS")]
    #[ts(rename = "SUBAGENTS")]
    Subagents,
    #[serde(rename = "HOOKS")]
    #[ts(rename = "HOOKS")]
    Hooks,
    #[serde(rename = "COMMANDS")]
    #[ts(rename = "COMMANDS")]
    Commands,
    #[serde(rename = "SESSIONS")]
    #[ts(rename = "SESSIONS")]
    Sessions,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct PluginsMigration {
    #[serde(rename = "marketplaceName")]
    #[ts(rename = "marketplaceName")]
    pub marketplace_name: String,
    #[serde(rename = "pluginNames")]
    #[ts(rename = "pluginNames")]
    pub plugin_names: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct SessionMigration {
    pub path: PathBuf,
    pub cwd: PathBuf,
    pub title: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerMigration {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct HookMigration {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct SubagentMigration {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct CommandMigration {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct MigrationDetails {
    #[serde(default)]
    pub plugins: Vec<PluginsMigration>,
    #[serde(default)]
    pub sessions: Vec<SessionMigration>,
    #[serde(default)]
    pub mcp_servers: Vec<McpServerMigration>,
    #[serde(default)]
    pub hooks: Vec<HookMigration>,
    #[serde(default)]
    pub subagents: Vec<SubagentMigration>,
    #[serde(default)]
    pub commands: Vec<CommandMigration>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ExternalAgentConfigMigrationItem {
    pub item_type: ExternalAgentConfigMigrationItemType,
    pub description: String,
    /// Null or empty means home-scoped migration; non-empty means repo-scoped migration.
    pub cwd: Option<PathBuf>,
    pub details: Option<MigrationDetails>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ExternalAgentConfigDetectResponse {
    pub items: Vec<ExternalAgentConfigMigrationItem>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ExternalAgentConfigDetectParams {
    /// If true, include detection under the user's home (~/.claude, ~/.vac, etc.).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub include_home: bool,
    /// Zero or more working directories to include for repo-scoped detection.
    #[ts(optional = nullable)]
    pub cwds: Option<Vec<PathBuf>>,
}

