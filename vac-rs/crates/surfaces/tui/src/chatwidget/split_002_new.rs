impl UnifiedExecWaitStreak {
    fn new(process_id: String, command_display: Option<String>) -> Self {
        Self {
            process_id,
            command_display: command_display.filter(|display| !display.is_empty()),
        }
    }

    fn update_command_display(&mut self, command_display: Option<String>) {
        if self.command_display.is_some() {
            return;
        }
        self.command_display = command_display.filter(|display| !display.is_empty());
    }
}

fn is_unified_exec_source(source: ExecCommandSource) -> bool {
    matches!(
        source,
        ExecCommandSource::UnifiedExecStartup | ExecCommandSource::UnifiedExecInteraction
    )
}

fn is_standard_tool_call(parsed_cmd: &[ParsedCommand]) -> bool {
    !parsed_cmd.is_empty()
        && parsed_cmd
            .iter()
            .all(|parsed| !matches!(parsed, ParsedCommand::Unknown { .. }))
}

fn command_execution_command_and_parsed(
    command: &str,
    command_actions: &[crate::session_protocol::CommandAction],
) -> (Vec<String>, Vec<ParsedCommand>) {
    (
        split_command_string(command),
        command_actions
            .iter()
            .cloned()
            .map(crate::session_protocol::CommandAction::into_core)
            .collect(),
    )
}

const RATE_LIMIT_WARNING_THRESHOLDS: [f64; 3] = [75.0, 90.0, 95.0];
const NUDGE_MODEL_SLUG: &str = "gpt-5.4-mini";
const RATE_LIMIT_SWITCH_PROMPT_THRESHOLD: f64 = 90.0;
const MAX_AGENT_COPY_HISTORY: usize = 32;

#[derive(Debug)]
struct AgentTurnMarkdown {
    user_turn_count: usize,
    markdown: String,
}

#[derive(Default)]
struct RateLimitWarningState {
    secondary_index: usize,
    primary_index: usize,
}

impl RateLimitWarningState {
    fn take_warnings(
        &mut self,
        secondary_used_percent: Option<f64>,
        secondary_window_minutes: Option<i64>,
        primary_used_percent: Option<f64>,
        primary_window_minutes: Option<i64>,
    ) -> Vec<String> {
        let reached_secondary_cap =
            matches!(secondary_used_percent, Some(percent) if percent == 100.0);
        let reached_primary_cap = matches!(primary_used_percent, Some(percent) if percent == 100.0);
        if reached_secondary_cap || reached_primary_cap {
            return Vec::new();
        }

        let mut warnings = Vec::new();

        if let Some(secondary_used_percent) = secondary_used_percent {
            let mut highest_secondary: Option<f64> = None;
            while self.secondary_index < RATE_LIMIT_WARNING_THRESHOLDS.len()
                && secondary_used_percent >= RATE_LIMIT_WARNING_THRESHOLDS[self.secondary_index]
            {
                highest_secondary = Some(RATE_LIMIT_WARNING_THRESHOLDS[self.secondary_index]);
                self.secondary_index += 1;
            }
            if let Some(threshold) = highest_secondary {
                let limit_label = secondary_window_minutes
                    .map(get_limits_duration)
                    .unwrap_or_else(|| "weekly".to_string());
                let remaining_percent = 100.0 - threshold;
                warnings.push(format!(
                    "Heads up, you have less than {remaining_percent:.0}% of your {limit_label} limit left. Run /status for a breakdown."
                ));
            }
        }

        if let Some(primary_used_percent) = primary_used_percent {
            let mut highest_primary: Option<f64> = None;
            while self.primary_index < RATE_LIMIT_WARNING_THRESHOLDS.len()
                && primary_used_percent >= RATE_LIMIT_WARNING_THRESHOLDS[self.primary_index]
            {
                highest_primary = Some(RATE_LIMIT_WARNING_THRESHOLDS[self.primary_index]);
                self.primary_index += 1;
            }
            if let Some(threshold) = highest_primary {
                let limit_label = primary_window_minutes
                    .map(get_limits_duration)
                    .unwrap_or_else(|| "5h".to_string());
                let remaining_percent = 100.0 - threshold;
                warnings.push(format!(
                    "Heads up, you have less than {remaining_percent:.0}% of your {limit_label} limit left. Run /status for a breakdown."
                ));
            }
        }

        warnings
    }
}

pub(crate) fn get_limits_duration(windows_minutes: i64) -> String {
    const MINUTES_PER_HOUR: i64 = 60;
    const MINUTES_PER_DAY: i64 = 24 * MINUTES_PER_HOUR;
    const MINUTES_PER_WEEK: i64 = 7 * MINUTES_PER_DAY;
    const MINUTES_PER_MONTH: i64 = 30 * MINUTES_PER_DAY;
    const ROUNDING_BIAS_MINUTES: i64 = 3;

    let windows_minutes = windows_minutes.max(0);

    if windows_minutes <= MINUTES_PER_DAY.saturating_add(ROUNDING_BIAS_MINUTES) {
        let adjusted = windows_minutes.saturating_add(ROUNDING_BIAS_MINUTES);
        let hours = std::cmp::max(1, adjusted / MINUTES_PER_HOUR);
        format!("{hours}h")
    } else if windows_minutes <= MINUTES_PER_WEEK.saturating_add(ROUNDING_BIAS_MINUTES) {
        "weekly".to_string()
    } else if windows_minutes <= MINUTES_PER_MONTH.saturating_add(ROUNDING_BIAS_MINUTES) {
        "monthly".to_string()
    } else {
        "annual".to_string()
    }
}

/// Common initialization parameters shared by all `ChatWidget` constructors.
pub(crate) struct ChatWidgetInit {
    pub(crate) config: Config,
    pub(crate) frame_requester: FrameRequester,
    pub(crate) app_event_tx: AppEventSender,
    pub(crate) initial_user_message: Option<UserMessage>,
    pub(crate) enhanced_keys_supported: bool,
    pub(crate) has_chatgpt_account: bool,
    pub(crate) model_catalog: Arc<ModelCatalog>,
    pub(crate) feedback: vac_feedback::VACFeedback,
    pub(crate) is_first_run: bool,
    pub(crate) status_account_display: Option<StatusAccountDisplay>,
    pub(crate) runtime_model_provider_base_url: Option<String>,
    pub(crate) initial_plan_type: Option<PlanType>,
    pub(crate) model: Option<String>,
    pub(crate) startup_tooltip_override: Option<String>,
    // Shared latch so we only warn once about invalid status-line item IDs.
    pub(crate) status_line_invalid_items_warned: Arc<AtomicBool>,
    // Shared latch so we only warn once about invalid terminal-title item IDs.
    pub(crate) terminal_title_invalid_items_warned: Arc<AtomicBool>,
    pub(crate) session_telemetry: SessionTelemetry,
}

#[derive(Default)]
enum RateLimitSwitchPromptState {
    #[default]
    Idle,
    Pending,
    Shown,
}

#[derive(Debug, Clone, Default)]
enum ConnectorsCacheState {
    #[default]
    Uninitialized,
    Loading,
    Ready(ConnectorsSnapshot),
    Failed(String),
}

#[derive(Debug, Clone, Default)]
struct PluginListFetchState {
    cache_cwd: Option<PathBuf>,
    in_flight_cwd: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct PluginInstallAuthFlowState {
    plugin_display_name: String,
    next_app_index: usize,
}

#[derive(Debug)]
enum RateLimitErrorKind {
    ServerOverloaded,
    UsageLimit,
    Generic,
}

fn app_server_rate_limit_error_kind(info: &AppServerVACErrorInfo) -> Option<RateLimitErrorKind> {
    match info {
        AppServerVACErrorInfo::ServerOverloaded => Some(RateLimitErrorKind::ServerOverloaded),
        AppServerVACErrorInfo::UsageLimitExceeded => Some(RateLimitErrorKind::UsageLimit),
        AppServerVACErrorInfo::ResponseTooManyFailedAttempts {
            http_status_code: Some(429),
        } => Some(RateLimitErrorKind::Generic),
        _ => None,
    }
}

fn is_app_server_cyber_policy_error(info: &AppServerVACErrorInfo) -> bool {
    matches!(info, AppServerVACErrorInfo::CyberPolicy)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ExternalEditorState {
    #[default]
    Closed,
    Requested,
    Active,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StatusIndicatorState {
    header: String,
    details: Option<String>,
    details_max_lines: usize,
}

impl StatusIndicatorState {
    fn working() -> Self {
        Self {
            header: String::from("Working"),
            details: None,
            details_max_lines: STATUS_DETAILS_DEFAULT_MAX_LINES,
        }
    }

    fn is_guardian_review(&self) -> bool {
        self.header == "Reviewing approval request" || self.header.starts_with("Reviewing ")
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct PendingGuardianReviewStatus {
    entries: Vec<PendingGuardianReviewStatusEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingGuardianReviewStatusEntry {
    id: String,
    detail: String,
}

impl PendingGuardianReviewStatus {
    fn start_or_update(&mut self, id: String, detail: String) {
        if let Some(existing) = self.entries.iter_mut().find(|entry| entry.id == id) {
            existing.detail = detail;
        } else {
            self.entries
                .push(PendingGuardianReviewStatusEntry { id, detail });
        }
    }

    fn finish(&mut self, id: &str) -> bool {
        let original_len = self.entries.len();
        self.entries.retain(|entry| entry.id != id);
        self.entries.len() != original_len
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    // Guardian review status is derived from the full set of currently pending
    // review entries. The generic status cache on `ChatWidget` stores whichever
    // footer is currently rendered; this helper computes the guardian-specific
    // footer snapshot that should replace it while reviews remain in flight.
    fn status_indicator_state(&self) -> Option<StatusIndicatorState> {
        let details = if self.entries.len() == 1 {
            self.entries.first().map(|entry| entry.detail.clone())
        } else if self.entries.is_empty() {
            None
        } else {
            let mut lines = self
                .entries
                .iter()
                .take(3)
                .map(|entry| format!("• {}", entry.detail))
                .collect::<Vec<_>>();
            let remaining = self.entries.len().saturating_sub(3);
            if remaining > 0 {
                lines.push(format!("+{remaining} more"));
            }
            Some(lines.join("\n"))
        };
        let details = details?;
        let header = if self.entries.len() == 1 {
            String::from("Reviewing approval request")
        } else {
            format!("Reviewing {} approval requests", self.entries.len())
        };
        let details_max_lines = if self.entries.len() == 1 { 1 } else { 4 };
        Some(StatusIndicatorState {
            header,
            details: Some(details),
            details_max_lines,
        })
    }
}

/// Maintains the per-session UI state and interaction state machines for the chat screen.
///
/// `ChatWidget` owns the state derived from the protocol event stream (history cells, streaming
/// buffers, bottom-pane overlays, and transient status text) and turns key presses into user
/// intent (`Op` submissions and `AppEvent` requests).
///
/// It is not responsible for running the agent itself; it reflects progress by updating UI state
/// and by sending requests back to vac-core.
///
/// Quit/interrupt behavior intentionally spans layers: the bottom pane owns local input routing
/// (which view gets Ctrl+C), while `ChatWidget` owns process-level decisions such as interrupting
/// active work, arming the double-press quit shortcut, and requesting shutdown-first exit.
pub(crate) struct ChatWidget {
    app_event_tx: AppEventSender,
    vac_op_target: VACOpTarget,
    bottom_pane: BottomPane,
    active_cell: Option<Box<dyn HistoryCell>>,
    /// Monotonic-ish counter used to invalidate transcript overlay caching.
    ///
    /// The transcript overlay appends a cached "live tail" for the current active cell. Most
    /// active-cell updates are mutations of the *existing* cell (not a replacement), so pointer
    /// identity alone is not a good cache key.
    ///
    /// Callers bump this whenever the active cell's transcript output could change without
    /// flushing. It is intentionally allowed to wrap, which implies a rare one-time cache collision
    /// where the overlay may briefly treat new tail content as already cached.
    active_cell_revision: u64,
    config: Config,
    /// Runtime value resolved by core. `config.service_tier` remains the explicit user choice.
    effective_service_tier: Option<ServiceTier>,
    /// The unmasked collaboration mode settings (always Default mode).
    ///
    /// Masks are applied on top of this base mode to derive the effective mode.
    current_collaboration_mode: CollaborationMode,
    /// The currently active collaboration mask, if any.
    active_collaboration_mask: Option<CollaborationModeMask>,
    has_chatgpt_account: bool,
    model_catalog: Arc<ModelCatalog>,
    session_telemetry: SessionTelemetry,
    session_header: SessionHeader,
    initial_user_message: Option<UserMessage>,
    status_account_display: Option<StatusAccountDisplay>,
    runtime_model_provider_base_url: Option<String>,
    token_info: Option<TokenUsageInfo>,
    rate_limit_snapshots_by_limit_id: BTreeMap<String, RateLimitSnapshotDisplay>,
    refreshing_status_outputs: Vec<(u64, StatusHistoryHandle)>,
    next_status_refresh_request_id: u64,
    plan_type: Option<PlanType>,
    vac_rate_limit_reached_type: Option<RateLimitReachedType>,
    rate_limit_warnings: RateLimitWarningState,
    rate_limit_switch_prompt: RateLimitSwitchPromptState,
    add_credits_nudge_email_in_flight: Option<AddCreditsNudgeCreditType>,
    adaptive_chunking: AdaptiveChunkingPolicy,
    // Stream lifecycle controller
    stream_controller: Option<StreamController>,
    // Stream lifecycle controller for proposed plan output.
    plan_stream_controller: Option<PlanStreamController>,
    /// Holds the platform clipboard lease so copied text remains available while supported.
    clipboard_lease: Option<crate::clipboard_copy::ClipboardLease>,
    copy_last_response_binding: Vec<KeyBinding>,
    /// Raw markdown of the most recently completed agent response that
    /// survived any local thread rollback.
    last_agent_markdown: Option<String>,
    /// Copyable agent responses keyed by the number of visible user turns at
    /// the time the response completed.
    agent_turn_markdowns: Vec<AgentTurnMarkdown>,
    /// Number of user turns currently reflected in the visible transcript.
    visible_user_turn_count: usize,
    /// True when rollback discarded the requested copy source because it was
    /// older than the retained copy history.
    copy_history_evicted_by_rollback: bool,
    /// Raw markdown of the most recently completed proposed plan.
    ///
    /// This is cached only for the approval popup. It is reset at the start of each new task so the
    /// fresh-context action cannot accidentally submit an older plan after a later turn begins.
    latest_proposed_plan_markdown: Option<String>,
    /// Whether this turn already produced a copyable response.
    ///
    /// `TurnComplete.last_agent_message` is a fallback source: use it only when no earlier
    /// agent/plan/review item recorded copyable markdown for the turn. This gives item-level
    /// sources precedence and avoids duplicating the same final answer when both event shapes are
    /// emitted.
    saw_copy_source_this_turn: bool,
    running_commands: HashMap<String, RunningCommand>,
    collab_agent_metadata: HashMap<ThreadId, AgentMetadata>,
    pending_collab_spawn_requests: HashMap<String, multi_agents::SpawnRequestSummary>,
    suppressed_exec_calls: HashSet<String>,
    skills_all: Vec<ProtocolSkillMetadata>,
    skills_initial_state: Option<HashMap<AbsolutePathBuf, bool>>,
    last_unified_wait: Option<UnifiedExecWaitState>,
    unified_exec_wait_streak: Option<UnifiedExecWaitStreak>,
    turn_sleep_inhibitor: SleepInhibitor,
    task_complete_pending: bool,
    /// Local Runtime Contract bridge for the active turn. Populated on
    /// prompt submit (see `submit_user_message`); cleared on task
    /// complete/fail/cancel (see `handle_turn_completed_notification`).
    runtime_bridge: Option<crate::runtime_adapter::RuntimeBridge>,
    /// 00D-8: Sidebar-facing runtime activity state. Items are pushed by
    /// `project_runtime_events`; the App-level render reads this via
    /// `runtime_activity()` to draw the floating Activity sidebar.
    /// Activity rows live here, not in the main transcript (verbose mode
    /// preserves the audit trace via `vac runtime:` rows).
    runtime_activity: crate::runtime_adapter::RuntimeActivityState,
    unified_exec_processes: Vec<UnifiedExecProcessSummary>,
    /// Tracks whether vac-core currently considers an agent turn to be in progress.
    ///
    /// This is kept separate from `mcp_startup_status` so that MCP startup progress (or completion)
    /// can update the status header without accidentally clearing the spinner for an active turn.
    agent_turn_running: bool,
    /// Tracks per-server MCP startup state while startup is in progress.
    ///
    /// The map is `Some(_)` from the first startup status update until the
    /// app-server-backed startup round settles, and the bottom pane is treated
    /// as "running" while this is populated, even if no agent turn is currently
    /// executing.
    mcp_startup_status: Option<HashMap<String, McpStartupStatus>>,
    /// Expected MCP servers for the current startup round, seeded from enabled local config.
    mcp_startup_expected_servers: Option<HashSet<String>>,
    /// After startup settles, ignore stale updates until enough notifications confirm a new round.
    mcp_startup_ignore_updates_until_next_start: bool,
    /// A lag signal for the next round means terminal-only updates are enough to settle it.
    mcp_startup_allow_terminal_only_next_round: bool,
    /// Buffers post-settle MCP startup updates until they cover a full fresh round.
    mcp_startup_pending_next_round: HashMap<String, McpStartupStatus>,
    /// Tracks whether the buffered next round has seen any `Starting` update yet.
    mcp_startup_pending_next_round_saw_starting: bool,
    connectors_cache: ConnectorsCacheState,
    connectors_partial_snapshot: Option<ConnectorsSnapshot>,
    connectors_prefetch_in_flight: bool,
    connectors_force_refetch_pending: bool,
    ide_context: IdeContextState,
    plugins_cache: PluginsCacheState,
    plugins_fetch_state: PluginListFetchState,
    plugin_install_apps_needing_auth: Vec<AppSummary>,
    plugin_install_auth_flow: Option<PluginInstallAuthFlowState>,
    plugins_active_tab_id: Option<String>,
    newly_installed_marketplace_tab_id: Option<String>,
    // Queue of interruptive UI events deferred during an active write cycle
    interrupts: InterruptManager,
    // Accumulates the current reasoning block text to extract a header
    reasoning_buffer: String,
    // Accumulates full reasoning content for transcript-only recording
    full_reasoning_buffer: String,
    // The currently rendered footer state. We keep the already-formatted
    // details here so transient stream interruptions can restore the footer
    // exactly as it was shown.
    current_status: StatusIndicatorState,
    // Guardian review keeps its own pending set so it can derive a single
    // footer summary from one or more in-flight review events.
    pending_guardian_review_status: PendingGuardianReviewStatus,
    recent_auto_review_denials: RecentAutoReviewDenials,
    // Active hook runs render in a dedicated live cell so they can run alongside tools.
    active_hook_cell: Option<HookCell>,
    // Semantic status used for terminal-title status rendering.
    terminal_title_status_kind: TerminalTitleStatusKind,
    // Previous status header to restore after a transient stream retry.
    retry_status_header: Option<String>,
    // Set when commentary output completes; once stream queues go idle we restore the status row.
    pending_status_indicator_restore: bool,
    suppress_queue_autosend: bool,
    thread_id: Option<ThreadId>,
    /// Nudge dismissals that should survive draft edits within the current thread scope.
    ///
    /// The nudge is only a discovery aid, so once a user dismisses it or enters Plan mode we keep it
    /// hidden for that thread instead of resurfacing it on every matching draft.
    dismissed_plan_mode_nudge_scopes: HashSet<PlanModeNudgeScope>,
    last_turn_id: Option<String>,
    budget_limited_turn_ids: HashSet<String>,
    thread_name: Option<String>,
    thread_rename_block_message: Option<String>,
    active_side_conversation: bool,
    normal_placeholder_text: String,
    side_placeholder_text: String,
    branched_from: Option<ThreadId>,
    interrupted_turn_notice_mode: InterruptedTurnNoticeMode,
    frame_requester: FrameRequester,
    // Whether to include the initial welcome banner on session configured
    show_welcome_banner: bool,
    // One-shot tooltip override for the primary startup session.
    startup_tooltip_override: Option<String>,
    // When resuming an existing session (selected via resume picker), avoid an
    // immediate redraw on SessionConfigured to prevent a gratuitous UI flicker.
    suppress_session_configured_redraw: bool,
    // During snapshot restore, defer startup prompt submission until replayed
    // history has been rendered so resumed/branched prompts keep chronological
    // order.
    suppress_initial_user_message_submit: bool,
    // User inputs queued while a turn is in progress.
    queued_user_messages: VecDeque<QueuedUserMessage>,
    // History records for queued user messages. Slash commands such as `/goal`
    // can render history that differs from the text submitted to core, so this
    // stays in lockstep with `queued_user_messages`, with missing entries
    // treated as user-message text.
    queued_user_message_history_records: VecDeque<UserMessageHistoryRecord>,
    // A user turn has been submitted to core, but `TurnStarted` has not arrived yet.
    user_turn_pending_start: bool,
    // User messages that tried to steer a non-regular turn and must be retried first.
    rejected_steers_queue: VecDeque<UserMessage>,
    // History records for rejected steers. Slash commands such as `/goal` can
    // render history that differs from the text submitted to core, so this stays
    // in lockstep with `rejected_steers_queue`, with missing entries treated as
    // user-message text.
    rejected_steer_history_records: VecDeque<UserMessageHistoryRecord>,
    // Steers already submitted to core but not yet committed into history.
    //
    // The bottom pane shows these above queued drafts until core records the
    // corresponding user message item.
    pending_steers: VecDeque<PendingSteer>,
    // When set, the next interrupt should resubmit all pending steers as one
    // fresh user turn instead of restoring them into the composer.
    submit_pending_steers_after_interrupt: bool,
    /// Main chat-surface bindings resolved from `tui.keymap.chat`.
    chat_keymap: ChatKeymap,
    /// Keybinding to show for popping the most-recently queued message back
    /// into the composer. This may differ from the first configured binding
    /// when the default set includes a terminal-specific fallback.
    queued_message_edit_hint_binding: Option<KeyBinding>,
    // Pending notification to show when unfocused on next Draw
    pending_notification: Option<Notification>,
    /// When `Some`, the user has pressed a quit shortcut and the second press
    /// must occur before `quit_shortcut_expires_at`.
    quit_shortcut_expires_at: Option<Instant>,
    /// Tracks which quit shortcut key was pressed first.
    ///
    /// We require the second press to match this key so `Ctrl+C` followed by
    /// `Ctrl+D` (or vice versa) doesn't quit accidentally.
    quit_shortcut_key: Option<KeyBinding>,
    // Simple review mode flag; used to adjust layout and banners.
    is_review_mode: bool,
    // Snapshot of token usage to restore after review mode exits.
    pre_review_token_info: Option<Option<TokenUsageInfo>>,
    // Whether the next streamed assistant content should be preceded by a final message separator.
    //
    // This is set whenever we insert a visible history cell that conceptually belongs to a turn.
    // The separator itself is only rendered if the turn recorded "work" activity.
    needs_final_message_separator: bool,
    // Whether the current turn performed "work" (exec commands, MCP tool calls, patch applications).
    //
    // This gates rendering of the "Worked for …" separator so purely conversational turns don't
    // show an empty divider.
    had_work_activity: bool,
    // Whether the current turn emitted a plan update.
    saw_plan_update_this_turn: bool,
    // Whether the current turn emitted a proposed plan item that has not been superseded by a
    // later steer. This is cleared when the user submits a steer so the plan popup only appears
    // if a newer proposed plan arrives afterward.
    saw_plan_item_this_turn: bool,
    // Latest `update_plan` checklist task counts for terminal-title rendering.
    last_plan_progress: Option<(usize, usize)>,
    // Incremental buffer for streamed plan content.
    plan_delta_buffer: String,
    // True while a plan item is streaming.
    plan_item_active: bool,
    // Runtime metrics accumulated across delta snapshots for the active turn.
    turn_runtime_metrics: RuntimeMetricsSummary,
    last_rendered_width: std::cell::Cell<Option<usize>>,
    // Monotonic style epoch used by height-cache keys. Unlike width, this changes
    // when /theme, syntax colors, or status-line theme coloring changes.
    style_epoch: std::cell::Cell<u64>,
    // Row-offset source for windowed transcript rendering. It is kept separate
    // from terminal scrollback so the prefix-index path can be exercised by real
    // TUI state instead of a hardcoded zero.
    transcript_scroll_top: std::cell::Cell<u32>,
    desired_height_cache: std::cell::RefCell<height_cache::DesiredHeightCache>,
    rendered_lines_cache: std::cell::RefCell<height_cache::RenderedLinesCache>,
    // Feedback sink for /feedback
    feedback: vac_feedback::VACFeedback,
    // Current session rollout path (if known)
    current_rollout_path: Option<PathBuf>,
    // Current working directory (if known)
    current_cwd: Option<PathBuf>,
    // Instruction source files loaded for the current session, supplied by app-server.
    instruction_source_paths: Vec<AbsolutePathBuf>,
    // Runtime network proxy bind addresses from SessionConfigured.
    session_network_proxy: Option<SessionNetworkProxyRuntime>,
    // Shared latch so we only warn once about invalid status-line item IDs.
    status_line_invalid_items_warned: Arc<AtomicBool>,
    // Shared latch so we only warn once about invalid terminal-title item IDs.
    terminal_title_invalid_items_warned: Arc<AtomicBool>,
    // Last terminal title emitted, to avoid writing duplicate OSC updates.
    pub(crate) last_terminal_title: Option<String>,
    // Last visible "action required" state observed by the terminal-title renderer.
    last_terminal_title_requires_action: bool,
    // Original terminal-title config captured when the setup UI opens.
    //
    // The outer `Option` tracks whether a setup session is active (`Some`)
    // or not (`None`). The inner `Option<Vec<String>>` mirrors the shape
    // of `config.tui_terminal_title` (which is `None` when using defaults).
    // On cancel or persist-failure the inner value is restored to config;
    // on confirm the outer is set to `None` to end the session.
    terminal_title_setup_original_items: Option<Option<Vec<String>>>,
    // Baseline instant used to animate spinner-prefixed title statuses.
    terminal_title_animation_origin: Instant,
    // Cached project-root display name keyed by cwd for status/title rendering.
    status_line_project_root_name_cache: Option<CachedProjectRootName>,
    // Cached git branch name for the status line (None if unknown).
    status_line_branch: Option<String>,
    // CWD used to resolve the cached branch; change resets branch state.
    status_line_branch_cwd: Option<PathBuf>,
    // True while an async branch lookup is in flight.
    status_line_branch_pending: bool,
    // True once we've attempted a branch lookup for the current CWD.
    status_line_branch_lookup_complete: bool,
    // Current thread-goal status shown in the status line when plan mode is inactive.
    current_goal_status_indicator: Option<GoalStatusIndicator>,
    current_goal_status: Option<GoalStatusState>,
    goal_status_active_turn_started_at: Option<Instant>,
    external_editor_state: ExternalEditorState,
    realtime_conversation: RealtimeConversationUiState,
    last_rendered_user_message_display: Option<UserMessageDisplay>,
    last_non_retry_error: Option<(String, String)>,
}
