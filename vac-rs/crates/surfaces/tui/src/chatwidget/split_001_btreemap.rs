// The main VAC TUI chat surface.
//
// `ChatWidget` consumes protocol events, builds and updates history cells, and drives rendering
// for both the main viewport and overlay UIs.
//
// The UI has both committed transcript cells (finalized `HistoryCell`s) and an in-flight active
// cell (`ChatWidget.active_cell`) that can mutate in place while streaming (often representing a
// coalesced exec/tool group). The transcript overlay (`Ctrl+T`) renders committed cells plus a
// cached, render-only live tail derived from the current active cell so in-flight tool calls are
// visible immediately.
//
// The transcript overlay is kept in sync by `App::overlay_forward_event`, which syncs a live tail
// during draws using `active_cell_transcript_key()` and `active_cell_transcript_lines()`. The
// cache key is designed to change when the active cell mutates in place or when its transcript
// output is time-dependent so the overlay can refresh its cached tail without rebuilding it on
// every draw.
//
// The bottom pane exposes a single "task running" indicator that drives the spinner and interrupt
// hints. This module treats that indicator as derived UI-busy state: it is set while an agent turn
// is in progress and while MCP server startup is in progress. Those lifecycles are tracked
// independently (`agent_turn_running` and `mcp_startup_status`) and synchronized via
// `update_task_running_state`.
//
// For preamble-capable models, assistant output may include commentary before
// the final answer. During streaming we hide the status row to avoid duplicate
// progress indicators; once commentary completes and stream queues drain, we
// re-show it so users still see turn-in-progress state between output bursts.
//
// Slash-command parsing lives in the bottom-pane composer, but slash-command acceptance lives
// here. That split lets the composer stage a recall entry before clearing input while this module
// records the attempted slash command after dispatch just like ordinary submitted text.
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use crate::app::app_server_requests::ResolvedAppServerRequest;
use crate::app_command::AppCommand;
use crate::app_event::HistoryLookupResponse;
use crate::app_event::RealtimeAudioDeviceKind;
use crate::app_server_approval_conversions::file_update_changes_to_display;
use crate::approval_events::ApplyPatchApprovalRequestEvent;
use crate::approval_events::ExecApprovalRequestEvent;
#[cfg(not(target_os = "linux"))]
use crate::audio_device::list_realtime_audio_device_names;
use crate::bottom_pane::StatusLineItem;
use crate::bottom_pane::StatusLineSetupView;
use crate::bottom_pane::StatusSurfacePreviewData;
use crate::bottom_pane::StatusSurfacePreviewItem;
use crate::bottom_pane::TerminalTitleItem;
use crate::bottom_pane::TerminalTitleSetupView;
use crate::diff_model::FileChange;
use crate::legacy_core::config::Config;
use crate::legacy_core::config::Constrained;
use crate::legacy_core::config::ConstraintResult;
#[cfg(target_os = "windows")]
use crate::legacy_core::windows_sandbox::WindowsSandboxLevelExt;
use crate::mention_codec::LinkedMention;
use crate::mention_codec::encode_history_mentions;
use crate::model_catalog::ModelCatalog;
use crate::multi_agents;
use crate::multi_agents::AgentMetadata;
use crate::session_protocol::AddCreditsNudgeCreditType;
use crate::session_protocol::AddCreditsNudgeEmailStatus;
use crate::session_protocol::AppInfo;
use crate::session_protocol::AppSummary;
use crate::session_protocol::CollabAgentTool;
use crate::session_protocol::CollabAgentToolCallStatus;
use crate::session_protocol::CommandExecutionRequestApprovalParams;
use crate::session_protocol::CommandExecutionSource as ExecCommandSource;
use crate::session_protocol::ConfigLayerSource;
use crate::session_protocol::CreditsSnapshot;
use crate::session_protocol::ErrorNotification;
use crate::session_protocol::FileChangeRequestApprovalParams;
use crate::session_protocol::GuardianApprovalReviewAction;
use crate::session_protocol::ItemCompletedNotification;
use crate::session_protocol::ItemStartedNotification;
use crate::session_protocol::McpServerElicitationRequest;
use crate::session_protocol::McpServerElicitationRequestParams;
use crate::session_protocol::McpServerStatusDetail;
use crate::session_protocol::ModelVerification as AppServerModelVerification;
use crate::session_protocol::RateLimitReachedType;
use crate::session_protocol::RateLimitSnapshot;
use crate::session_protocol::RequestId as AppServerRequestId;
use crate::session_protocol::ServerNotification;
use crate::session_protocol::ServerRequest;
use crate::session_protocol::SkillMetadata as ProtocolSkillMetadata;
use crate::session_protocol::SkillsListResponse;
use crate::session_protocol::TextElement as AppServerTextElement;
use crate::session_protocol::ThreadGoal as AppThreadGoal;
use crate::session_protocol::ThreadGoalStatus as AppThreadGoalStatus;
use crate::session_protocol::ThreadItem;
use crate::session_protocol::ThreadTokenUsage;
use crate::session_protocol::ToolRequestUserInputParams;
use crate::session_protocol::Turn;
use crate::session_protocol::TurnCompletedNotification;
use crate::session_protocol::TurnStatus;
use crate::session_protocol::UserInput;
use crate::session_protocol::VACErrorInfo as AppServerVACErrorInfo;
use crate::session_protocol::thread_goal_from_app_server;
use crate::session_protocol::turn_plan_step_status_from_app_server;
use crate::session_state::SessionNetworkProxyRuntime;
use crate::session_state::ThreadSessionState;
use crate::status::RateLimitWindowDisplay;
use crate::status::StatusAccountDisplay;
use crate::status::StatusHistoryHandle;
use crate::status::format_directory_display;
use crate::status::format_tokens_compact;
use crate::status::rate_limit_snapshot_display_for_limit;
use crate::terminal_title::SetTerminalTitleResult;
use crate::terminal_title::clear_terminal_title;
use crate::terminal_title::set_terminal_title;
use crate::text_formatting::proper_join;
use crate::token_usage::TokenUsage;
use crate::token_usage::TokenUsageInfo;
use crate::version::VAC_CLI_VERSION;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use rand::Rng;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Text;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::Wrap;
use serde::Deserialize;
#[cfg(test)]
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;
use tracing::warn;
use vac_core::connectors;
use vac_config::ConfigLayerStackOrdering;
use vac_config::types::ApprovalsReviewer;
use vac_config::types::Notifications;
use vac_config::types::WindowsSandboxModeToml;
use vac_core_skills::model::SkillMetadata;
use vac_features::FEATURES;
use vac_features::Feature;
#[cfg(test)]
use vac_git_utils::CommitLogEntry;
use vac_git_utils::current_branch_name;
use vac_git_utils::get_git_repo_root;
use vac_git_utils::local_git_branches;
use vac_git_utils::recent_commits;
use vac_otel::RuntimeMetricsSummary;
use vac_otel::SessionTelemetry;
use vac_plugin::PluginCapabilitySummary;
use vac_protocol::ThreadId;
use vac_protocol::account::PlanType;
use vac_protocol::approvals::GuardianAssessmentAction;
use vac_protocol::approvals::GuardianAssessmentEvent;
use vac_protocol::approvals::GuardianAssessmentStatus;
use vac_protocol::config_types::CollaborationMode;
use vac_protocol::config_types::CollaborationModeMask;
use vac_protocol::config_types::ModeKind;
use vac_protocol::config_types::Personality;
use vac_protocol::config_types::ServiceTier;
use vac_protocol::config_types::Settings;
#[cfg(target_os = "windows")]
use vac_protocol::config_types::WindowsSandboxLevel;
use vac_protocol::items::AgentMessageContent;
use vac_protocol::items::AgentMessageItem;
use vac_protocol::models::MessagePhase;
use vac_protocol::models::local_image_label_text;
use vac_protocol::parse_command::ParsedCommand;
use vac_protocol::plan_tool::PlanItemArg as UpdatePlanItemArg;
use vac_protocol::protocol::ReviewTarget;
use vac_protocol::request_permissions::RequestPermissionsEvent;
use vac_protocol::user_input::ByteRange;
use vac_protocol::user_input::TextElement;
use vac_terminal_detection::Multiplexer;
use vac_terminal_detection::TerminalInfo;
use vac_terminal_detection::TerminalName;
use vac_terminal_detection::terminal_info;
use vac_utils_absolute_path::AbsolutePathBuf;
use vac_utils_sleep_inhibitor::SleepInhibitor;

const DEFAULT_MODEL_DISPLAY_NAME: &str = "loading";
const MULTI_AGENT_ENABLE_TITLE: &str = "Enable subagents?";
const MULTI_AGENT_ENABLE_YES: &str = "Yes, enable";
const MULTI_AGENT_ENABLE_NO: &str = "Not now";
const MULTI_AGENT_ENABLE_NOTICE: &str = "Subagents will be enabled in the next session.";
const TRUSTED_ACCESS_FOR_CYBER_VERIFICATION_WARNING: &str = "Your conversations have multiple flags for possible cybersecurity risk. Responses may take longer because extra safety checks are on. To get authorized for security work, join the Trusted Access for Cyber program: https://provider.vac.invalid/cyber";
const MEMORIES_DOC_URL: &str = "https://developers.vastar.com/vac/memories";
const MEMORIES_ENABLE_TITLE: &str = "Enable memories?";
const MEMORIES_ENABLE_YES: &str = "Yes, enable";
const MEMORIES_ENABLE_NO: &str = "Not now";
const MEMORIES_ENABLE_NOTICE: &str = "Memories will be enabled in the next session.";
const PLAN_MODE_REASONING_SCOPE_TITLE: &str = "Apply reasoning change";
const PLAN_MODE_REASONING_SCOPE_PLAN_ONLY: &str = "Apply to Plan mode override";
const PLAN_MODE_REASONING_SCOPE_ALL_MODES: &str = "Apply to global default and Plan mode override";
const CONNECTORS_SELECTION_VIEW_ID: &str = "connectors-selection";
const TUI_STUB_MESSAGE: &str = "Not available in TUI yet.";

/// Choose the keybinding used to edit the most-recently queued message.
///
/// Apple Terminal, Warp, and VSCode integrated terminals intercept or silently
/// swallow Alt+Up, and tmux does not reliably pass that chord through. We fall
/// back to Shift+Left for those environments while keeping the more discoverable
/// Alt+Up everywhere else.
///
/// The match is exhaustive so that adding a new `TerminalName` variant forces
/// an explicit decision about which binding that terminal should use.
fn queued_message_edit_binding_for_terminal(terminal_info: TerminalInfo) -> KeyBinding {
    if matches!(
        terminal_info.multiplexer.as_ref(),
        Some(Multiplexer::Tmux { .. })
    ) {
        return key_hint::shift(KeyCode::Left);
    }

    match terminal_info.name {
        TerminalName::AppleTerminal | TerminalName::WarpTerminal | TerminalName::VsCode => {
            key_hint::shift(KeyCode::Left)
        }
        TerminalName::Ghostty
        | TerminalName::Iterm2
        | TerminalName::WezTerm
        | TerminalName::Kitty
        | TerminalName::Alacritty
        | TerminalName::Konsole
        | TerminalName::GnomeTerminal
        | TerminalName::Vte
        | TerminalName::WindowsTerminal
        | TerminalName::Dumb
        | TerminalName::Unknown => key_hint::alt(KeyCode::Up),
    }
}

fn queued_message_edit_hint_binding(
    bindings: &[KeyBinding],
    terminal_info: TerminalInfo,
) -> Option<KeyBinding> {
    let terminal_binding = queued_message_edit_binding_for_terminal(terminal_info);
    bindings
        .contains(&terminal_binding)
        .then_some(terminal_binding)
        .or_else(|| bindings.first().copied())
}

use crate::app_event::AppEvent;
use crate::app_event::ConnectorsSnapshot;
use crate::app_event::ExitMode;
#[cfg(target_os = "windows")]
use crate::app_event::WindowsSandboxEnableMode;
use crate::app_event_sender::AppEventSender;
use crate::auto_review_denials;
use crate::auto_review_denials::RecentAutoReviewDenials;
use crate::bottom_pane::ApprovalRequest;
use crate::bottom_pane::BottomPane;
use crate::bottom_pane::BottomPaneParams;
use crate::bottom_pane::CancellationEvent;
use crate::bottom_pane::CollaborationModeIndicator;
use crate::bottom_pane::ColumnWidthMode;
use crate::bottom_pane::DOUBLE_PRESS_QUIT_SHORTCUT_ENABLED;
use crate::bottom_pane::ExperimentalFeatureItem;
use crate::bottom_pane::ExperimentalFeaturesView;
use crate::bottom_pane::GoalStatusIndicator;
use crate::bottom_pane::InputResult;
use crate::bottom_pane::LocalImageAttachment;
use crate::bottom_pane::McpServerElicitationFormRequest;
use crate::bottom_pane::MemoriesSettingsView;
use crate::bottom_pane::MentionBinding;
use crate::bottom_pane::QUIT_SHORTCUT_TIMEOUT;
use crate::bottom_pane::QueuedInputAction;
use crate::bottom_pane::SelectionAction;
use crate::bottom_pane::SelectionItem;
use crate::bottom_pane::SelectionViewParams;
use crate::bottom_pane::custom_prompt_view::CustomPromptView;
use crate::bottom_pane::popup_consts::standard_popup_hint_line;
use crate::clipboard_paste::paste_image_to_temp_png;
use crate::collaboration_modes;
use crate::diff_render::display_path_for;
use crate::exec_cell::CommandOutput;
use crate::exec_cell::ExecCell;
use crate::exec_cell::new_active_exec_command;
use crate::exec_command::split_command_string;
use crate::exec_command::strip_bash_lc_and_escape;
use crate::get_git_diff::get_git_diff;
use crate::history_cell;
use crate::history_cell::HistoryCell;
use crate::history_cell::HookCell;
use crate::history_cell::McpInvocation;
use crate::history_cell::McpToolCallCell;
use crate::history_cell::PlainHistoryCell;
use crate::history_cell::WebSearchCell;
use crate::key_hint;
use crate::key_hint::KeyBinding;
use crate::key_hint::KeyBindingListExt;
use crate::keymap::ChatKeymap;
use crate::keymap::RuntimeKeymap;
use crate::render::Insets;
use crate::render::RectExt;
use crate::render::renderable::ColumnRenderable;
use crate::render::renderable::FlexRenderable;
use crate::render::renderable::Renderable;
use crate::render::renderable::RenderableExt;
use crate::render::renderable::RenderableItem;
use crate::slash_command::SlashCommand;
use crate::status::RateLimitSnapshotDisplay;
use crate::status_indicator_widget::STATUS_DETAILS_DEFAULT_MAX_LINES;
use crate::status_indicator_widget::StatusDetailsCapitalization;
use crate::text_formatting::truncate_text;
use crate::tui::FrameRequester;
mod goal_status;
use self::goal_status::GoalStatusState;
#[cfg(test)]
use self::goal_status::goal_status_indicator_from_app_goal;
mod goal_menu;
mod ide_context;
use self::ide_context::IdeContextState;
mod interrupts;
use self::interrupts::InterruptManager;
mod keymap_picker;
mod mcp_startup;
use self::mcp_startup::McpStartupStatus;
mod session_header;
use self::session_header::SessionHeader;
mod hooks;
mod skills;
mod slash_dispatch;
use self::skills::collect_tool_mentions;
use self::skills::find_app_mentions;
use self::skills::find_skill_mentions_with_tool_mentions;
mod plugins;
use self::plugins::PluginsCacheState;
mod plan_implementation;
use self::plan_implementation::PLAN_IMPLEMENTATION_TITLE;
mod realtime;
use self::realtime::RealtimeConversationUiState;
mod reasoning_shortcuts;
mod side;
mod status_surfaces;
use self::status_surfaces::CachedProjectRootName;
use self::status_surfaces::TerminalTitleStatusKind;
mod user_messages;
use self::user_messages::PendingSteerCompareKey;
use self::user_messages::UserMessageDisplay;
use crate::streaming::chunking::AdaptiveChunkingPolicy;
use crate::streaming::commit_tick::CommitTickScope;
use crate::streaming::commit_tick::run_commit_tick;
use crate::streaming::controller::PlanStreamController;
use crate::streaming::controller::StreamController;

use crate::session_protocol::AskForApproval;
use chrono::Local;
use strum::IntoEnumIterator;
use unicode_segmentation::UnicodeSegmentation;
use vac_file_search::FileMatch;
use vac_protocol::models::PermissionProfile;
use vac_protocol::plan_tool::StepStatus;
use vac_protocol::plan_tool::UpdatePlanArgs;
use vac_protocol::vastar_models::InputModality;
use vac_protocol::vastar_models::ModelPreset;
use vac_protocol::vastar_models::ReasoningEffort as ReasoningEffortConfig;
use vac_protocol::vastar_models::ReasoningEffortPreset;
use vac_utils_approval_presets::ApprovalPreset;
use vac_utils_approval_presets::builtin_approval_presets;

const USER_SHELL_COMMAND_HELP_TITLE: &str = "Prefix a command with ! to run it locally";
const USER_SHELL_COMMAND_HELP_HINT: &str = "Example: !ls";
const DEFAULT_VASTAR_BASE_URL: &str = "https://api.vastar.com/v1";
const KILO_PROVIDER_ID: &str = "kilo";
const KILO_GATEWAY_DEFAULT_BASE_URL: &str = "https://api.kilo.ai/api/gateway";
const DEFAULT_STATUS_LINE_ITEMS: [&str; 2] = ["model-with-reasoning", "current-dir"];

#[derive(Debug, Deserialize)]
struct KiloModelsResponse {
    data: Vec<KiloModelEntry>,
}

#[derive(Debug, Deserialize)]
struct KiloModelEntry {
    id: String,
}
// Track information about an in-flight exec command.
struct RunningCommand {
    command: Vec<String>,
    parsed_cmd: Vec<ParsedCommand>,
    source: ExecCommandSource,
}

struct UnifiedExecProcessSummary {
    key: String,
    call_id: String,
    command_display: String,
    recent_chunks: Vec<String>,
}

struct UnifiedExecWaitState {
    command_display: String,
}

impl UnifiedExecWaitState {
    fn new(command_display: String) -> Self {
        Self { command_display }
    }

    fn is_duplicate(&self, command_display: &str) -> bool {
        self.command_display == command_display
    }
}

#[derive(Clone, Debug)]
struct UnifiedExecWaitStreak {
    process_id: String,
    command_display: Option<String>,
}
