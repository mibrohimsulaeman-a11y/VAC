// Exercises `ChatWidget` event handling and rendering invariants.
//
// These tests cover both app-server-native inputs and focused widget helpers. Many assertions are
// snapshot-based so that layout regressions and status/header changes show up as stable,
// reviewable diffs.

pub(super) use super::*;
pub(super) use crate::app_command::AppCommand as Op;
pub(super) use crate::app_event::AppEvent;
pub(super) use crate::app_event::ExitMode;
#[cfg(not(target_os = "linux"))]
pub(super) use crate::app_event::RealtimeAudioDeviceKind;
pub(super) use crate::app_event_sender::AppEventSender;
pub(super) use crate::approval_events::ApplyPatchApprovalRequestEvent;
pub(super) use crate::approval_events::ExecApprovalRequestEvent;
pub(super) use crate::bottom_pane::LocalImageAttachment;
pub(super) use crate::bottom_pane::MentionBinding;
pub(super) use crate::bottom_pane::QueuedInputAction;
pub(super) use crate::chatwidget::realtime::RealtimeConversationPhase;
pub(super) use crate::diff_model::FileChange;
pub(super) use crate::history_cell::UserHistoryCell;
pub(super) use crate::legacy_core::config::Config;
pub(super) use crate::legacy_core::config::ConfigBuilder;
pub(super) use crate::legacy_core::config::Constrained;
pub(super) use crate::legacy_core::config::ConstraintError;
pub(super) use crate::model_catalog::ModelCatalog;
pub(super) use crate::session_protocol::AddCreditsNudgeCreditType;
pub(super) use crate::session_protocol::AddCreditsNudgeEmailStatus;
pub(super) use crate::session_protocol::AppServerAdditionalFileSystemPermissions;
pub(super) use crate::session_protocol::AppServerAdditionalNetworkPermissions;
pub(super) use crate::session_protocol::AppServerAdditionalPermissionProfile;
pub(super) use crate::session_protocol::AppServerGuardianCommandSource;
pub(super) use crate::session_protocol::AppServerGuardianRiskLevel;
pub(super) use crate::session_protocol::AppServerGuardianUserAuthorization;
pub(super) use crate::session_protocol::AppServerHookCompletedNotification;
pub(super) use crate::session_protocol::AppServerHookEventName;
pub(super) use crate::session_protocol::AppServerHookExecutionMode;
pub(super) use crate::session_protocol::AppServerHookHandlerType;
pub(super) use crate::session_protocol::AppServerHookScope;
pub(super) use crate::session_protocol::AppServerHookStartedNotification;
pub(super) use crate::session_protocol::AppServerModelVerification;
pub(super) use crate::session_protocol::AppServerNetworkApprovalContext;
pub(super) use crate::session_protocol::AppServerNetworkApprovalProtocol;
pub(super) use crate::session_protocol::AppSummary;
pub(super) use crate::session_protocol::CollabAgentState as AppServerCollabAgentState;
pub(super) use crate::session_protocol::CollabAgentStatus as AppServerCollabAgentStatus;
pub(super) use crate::session_protocol::CollabAgentTool as AppServerCollabAgentTool;
pub(super) use crate::session_protocol::CollabAgentToolCallStatus as AppServerCollabAgentToolCallStatus;
pub(super) use crate::session_protocol::CommandAction as AppServerCommandAction;
pub(super) use crate::session_protocol::CommandExecutionRequestApprovalParams as AppServerCommandExecutionRequestApprovalParams;
pub(super) use crate::session_protocol::CommandExecutionSource as ExecCommandSource;
pub(super) use crate::session_protocol::CommandExecutionSource as AppServerCommandExecutionSource;
pub(super) use crate::session_protocol::CommandExecutionStatus as AppServerCommandExecutionStatus;
pub(super) use crate::session_protocol::ConfigWarningNotification;
pub(super) use crate::session_protocol::CreditsSnapshot;
pub(super) use crate::session_protocol::ErrorNotification;
pub(super) use crate::session_protocol::ExecPolicyAmendment;
pub(super) use crate::session_protocol::FileUpdateChange;
pub(super) use crate::session_protocol::GuardianApprovalReview;
pub(super) use crate::session_protocol::GuardianApprovalReviewAction as AppServerGuardianApprovalReviewAction;
pub(super) use crate::session_protocol::GuardianApprovalReviewStatus;
pub(super) use crate::session_protocol::GuardianAssessmentDecisionSource as AppServerGuardianApprovalReviewDecisionSource;
pub(super) use crate::session_protocol::GuardianWarningNotification;
pub(super) use crate::session_protocol::HookOutputEntry as AppServerHookOutputEntry;
pub(super) use crate::session_protocol::HookOutputEntryKind as AppServerHookOutputEntryKind;
pub(super) use crate::session_protocol::HookRunStatus as AppServerHookRunStatus;
pub(super) use crate::session_protocol::HookRunSummary as AppServerHookRunSummary;
pub(super) use crate::session_protocol::ItemCompletedNotification;
pub(super) use crate::session_protocol::ItemGuardianApprovalReviewCompletedNotification;
pub(super) use crate::session_protocol::ItemGuardianApprovalReviewStartedNotification;
pub(super) use crate::session_protocol::ItemStartedNotification;
pub(super) use crate::session_protocol::MarketplaceAddResponse;
pub(super) use crate::session_protocol::MarketplaceInterface;
pub(super) use crate::session_protocol::MarketplaceUpgradeErrorInfo;
pub(super) use crate::session_protocol::MarketplaceUpgradeResponse;
pub(super) use crate::session_protocol::McpServerStartupState;
pub(super) use crate::session_protocol::McpServerStatusDetail;
pub(super) use crate::session_protocol::McpServerStatusUpdatedNotification;
pub(super) use crate::session_protocol::ModelVerificationNotification;
pub(super) use crate::session_protocol::NonSteerableTurnKind;
pub(super) use crate::session_protocol::PatchApplyStatus as AppServerPatchApplyStatus;
pub(super) use crate::session_protocol::PatchChangeKind;
pub(super) use crate::session_protocol::PermissionsRequestApprovalParams as AppServerPermissionsRequestApprovalParams;
pub(super) use crate::session_protocol::PluginAuthPolicy;
pub(super) use crate::session_protocol::PluginDetail;
pub(super) use crate::session_protocol::PluginInstallPolicy;
pub(super) use crate::session_protocol::PluginInterface;
pub(super) use crate::session_protocol::PluginListResponse;
pub(super) use crate::session_protocol::PluginMarketplaceEntry;
pub(super) use crate::session_protocol::PluginReadResponse;
pub(super) use crate::session_protocol::PluginSource;
pub(super) use crate::session_protocol::PluginSummary;
pub(super) use crate::session_protocol::RateLimitReachedType;
pub(super) use crate::session_protocol::RateLimitSnapshot;
pub(super) use crate::session_protocol::RateLimitWindow;
pub(super) use crate::session_protocol::ReasoningSummaryTextDeltaNotification;
pub(super) use crate::session_protocol::ServerNotification;
pub(super) use crate::session_protocol::SkillSummary;
pub(super) use crate::session_protocol::ThreadClosedNotification;
pub(super) use crate::session_protocol::ThreadItem as AppServerThreadItem;
pub(super) use crate::session_protocol::ThreadRealtimeClosedNotification;
pub(super) use crate::session_protocol::ThreadRealtimeErrorNotification;
pub(super) use crate::session_protocol::ToolRequestUserInputOption;
pub(super) use crate::session_protocol::ToolRequestUserInputParams;
pub(super) use crate::session_protocol::ToolRequestUserInputQuestion;
pub(super) use crate::session_protocol::Turn as AppServerTurn;
pub(super) use crate::session_protocol::TurnCompletedNotification;
pub(super) use crate::session_protocol::TurnError as AppServerTurnError;
pub(super) use crate::session_protocol::TurnStartedNotification;
pub(super) use crate::session_protocol::TurnStatus as AppServerTurnStatus;
pub(super) use crate::session_protocol::UserInput;
pub(super) use crate::session_protocol::UserInput as AppServerUserInput;
pub(super) use crate::session_protocol::VACErrorInfo;
pub(super) use crate::session_protocol::WarningNotification;
pub(super) use crate::test_backend::VT100Backend;
pub(super) use crate::test_support::PathBufExt;
pub(super) use crate::test_support::test_path_buf;
pub(super) use crate::test_support::test_path_display;
pub(super) use crate::token_usage::TokenUsage;
pub(super) use crate::token_usage::TokenUsageInfo;
pub(super) use crate::tui::FrameRequester;
pub(super) use assert_matches::assert_matches;
pub(super) use crossterm::event::KeyCode;
pub(super) use crossterm::event::KeyEvent;
pub(super) use crossterm::event::KeyModifiers;
pub(super) use insta::assert_snapshot;
pub(super) use serde_json::json;
#[cfg(target_os = "windows")]
pub(super) use serial_test::serial;
pub(super) use std::collections::BTreeMap;
pub(super) use std::collections::HashMap;
pub(super) use std::collections::HashSet;
pub(super) use std::path::PathBuf;
pub(super) use tempfile::NamedTempFile;
pub(super) use tempfile::tempdir;
pub(super) use tokio::sync::mpsc::error::TryRecvError;
pub(super) use tokio::sync::mpsc::unbounded_channel;
pub(super) use toml::Value as TomlValue;
pub(super) use vac_config::AppRequirementToml;
pub(super) use vac_config::AppsRequirementsToml;
pub(super) use vac_config::ConfigLayerStack;
pub(super) use vac_config::ConfigRequirements;
pub(super) use vac_config::ConfigRequirementsToml;
pub(super) use vac_config::RequirementSource;
pub(super) use vac_config::types::ApprovalsReviewer;
pub(super) use vac_config::types::Notifications;
#[cfg(target_os = "windows")]
pub(super) use vac_config::types::WindowsSandboxModeToml;
pub(super) use vac_core_plugins::VASTAR_CURATED_MARKETPLACE_NAME;
pub(super) use vac_core_skills::model::SkillMetadata;
pub(super) use vac_features::FEATURES;
pub(super) use vac_features::Feature;
pub(super) use vac_git_utils::CommitLogEntry;
pub(super) use vac_otel::RuntimeMetricsSummary;
pub(super) use vac_otel::SessionTelemetry;
pub(super) use vac_protocol::ThreadId;
pub(super) use vac_protocol::account::PlanType;
pub(super) use vac_protocol::approvals::GuardianAssessmentAction;
pub(super) use vac_protocol::approvals::GuardianAssessmentDecisionSource;
pub(super) use vac_protocol::approvals::GuardianAssessmentEvent;
pub(super) use vac_protocol::approvals::GuardianAssessmentStatus;
pub(super) use vac_protocol::approvals::GuardianCommandSource;
pub(super) use vac_protocol::approvals::GuardianRiskLevel;
pub(super) use vac_protocol::approvals::GuardianUserAuthorization;
pub(super) use vac_protocol::config_types::CollaborationMode;
pub(super) use vac_protocol::config_types::ModeKind;
pub(super) use vac_protocol::config_types::Personality;
pub(super) use vac_protocol::config_types::ServiceTier;
pub(super) use vac_protocol::config_types::Settings;
pub(super) use vac_protocol::models::FileSystemPermissions;
pub(super) use vac_protocol::models::MessagePhase;
pub(super) use vac_protocol::models::NetworkPermissions;
pub(super) use vac_protocol::models::PermissionProfile;
pub(super) use vac_protocol::parse_command::ParsedCommand;
pub(super) use vac_protocol::plan_tool::PlanItemArg;
pub(super) use vac_protocol::plan_tool::StepStatus;
pub(super) use vac_protocol::plan_tool::UpdatePlanArgs;
pub(super) use vac_protocol::protocol::ReviewTarget;
pub(super) use vac_protocol::request_permissions::RequestPermissionProfile;
pub(super) use vac_protocol::user_input::TextElement;
pub(super) use vac_protocol::vastar_models::ModelInfo;
pub(super) use vac_protocol::vastar_models::ModelPreset;
pub(super) use vac_protocol::vastar_models::ModelsResponse;
pub(super) use vac_protocol::vastar_models::ReasoningEffortPreset;
pub(super) use vac_protocol::vastar_models::default_input_modalities;
pub(super) use vac_terminal_detection::Multiplexer;
pub(super) use vac_terminal_detection::TerminalInfo;
pub(super) use vac_terminal_detection::TerminalName;
pub(super) use vac_utils_absolute_path::AbsolutePathBuf;
pub(super) use vac_utils_approval_presets::builtin_approval_presets;

pub(super) fn chatwidget_snapshot_dir() -> PathBuf {
    let snapshot_file = vac_utils_cargo_bin::find_resource!(
        "src/chatwidget/snapshots/vac_tui__chatwidget__tests__chatwidget_tall.snap"
    )
    .expect("snapshot file");
    snapshot_file
        .parent()
        .unwrap_or_else(|| panic!("snapshot file has no parent: {}", snapshot_file.display()))
        .to_path_buf()
}

macro_rules! assert_chatwidget_snapshot {
    ($name:expr, $value:expr $(,)?) => {{
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_path(crate::chatwidget::tests::chatwidget_snapshot_dir());
        settings.bind(|| {
            insta::assert_snapshot!(format!("vac_tui__chatwidget__tests__{}", $name), $value);
        });
    }};
    ($name:expr, $value:expr, @$snapshot:literal $(,)?) => {{
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_path(crate::chatwidget::tests::chatwidget_snapshot_dir());
        settings.bind(|| {
            insta::assert_snapshot!(
                format!("vac_tui__chatwidget__tests__{}", $name),
                &($value),
                @$snapshot
            );
        });
    }};
}

mod app_server;
mod approval_requests;
mod background_fill;
mod composer_submission;
mod exec_flow;
mod goal_menu;
mod guardian;
mod helpers;
mod history_replay;
mod mcp_startup;
mod permissions;
mod plan_mode;
mod popups_and_settings;
mod review_mode;
mod side;
mod slash_commands;
mod status_and_layout;
mod status_command_tests;
mod status_surface_previews;
mod terminal_title;

pub(crate) use helpers::make_chatwidget_manual_with_sender;
pub(crate) use helpers::set_chatgpt_auth;
pub(crate) use helpers::set_fast_mode_test_catalog;
pub(super) use helpers::*;
