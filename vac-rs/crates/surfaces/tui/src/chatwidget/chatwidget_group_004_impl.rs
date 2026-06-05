// O5/O6 balanced ChatWidget impl group: source split_004_user_message_preview_text.rs
impl ChatWidget {
    /// Stores or overwrites the cached nickname and role for a collab agent thread.
    ///
    /// Called by `App::upsert_agent_picker_thread` and `App::replace_chat_widget` to keep the
    /// rendering metadata in sync with the navigation cache. Must be called before any
    /// notification referencing this thread is processed, otherwise the rendered item will fall
    /// back to showing the raw thread id.
    pub(crate) fn set_collab_agent_metadata(
        &mut self,
        thread_id: ThreadId,
        agent_nickname: Option<String>,
        agent_role: Option<String>,
    ) {
        self.collab_agent_metadata.insert(
            thread_id,
            AgentMetadata {
                agent_nickname,
                agent_role,
            },
        );
    }

    /// Returns the cached metadata for a thread, defaulting to empty if none has been registered.
    fn collab_agent_metadata(&self, thread_id: ThreadId) -> AgentMetadata {
        self.collab_agent_metadata
            .get(&thread_id)
            .cloned()
            .unwrap_or_default()
    }

    fn realtime_conversation_enabled(&self) -> bool {
        self.config.features.enabled(Feature::RealtimeConversation)
            && cfg!(not(target_os = "linux"))
    }

    fn realtime_audio_device_selection_enabled(&self) -> bool {
        self.realtime_conversation_enabled()
    }

    /// Synchronize the bottom-pane "task running" indicator with the current lifecycles.
    ///
    /// The bottom pane only has one running flag, but this module treats it as a derived state of
    /// both the agent turn lifecycle and MCP startup lifecycle.
    fn update_task_running_state(&mut self) {
        self.bottom_pane
            .set_task_running(self.agent_turn_running || self.mcp_startup_status.is_some());
        self.refresh_plan_mode_nudge();
        self.refresh_status_surfaces();
    }

    fn restore_reasoning_status_header(&mut self) {
        if let Some(header) = extract_first_bold(&self.reasoning_buffer) {
            self.terminal_title_status_kind = TerminalTitleStatusKind::Thinking;
            self.set_status_header(header);
        } else if self.bottom_pane.is_task_running() {
            self.terminal_title_status_kind = TerminalTitleStatusKind::Working;
            self.set_status_header(String::from("Working"));
        }
    }

    fn flush_unified_exec_wait_streak(&mut self) {
        let Some(wait) = self.unified_exec_wait_streak.take() else {
            return;
        };
        self.needs_final_message_separator = true;
        let cell = history_cell::new_unified_exec_interaction(wait.command_display, String::new());
        self.app_event_tx
            .send(AppEvent::InsertHistoryCell(Box::new(cell)));
        self.restore_reasoning_status_header();
    }

    fn flush_answer_stream_with_separator(&mut self) {
        let had_stream_controller = self.stream_controller.is_some();
        if let Some(mut controller) = self.stream_controller.take() {
            let (cell, source) = controller.finalize();
            if let Some(cell) = cell {
                self.add_boxed_history(cell);
            }
            // Consolidate the run of streaming AgentMessageCells into a single AgentMarkdownCell
            // that can re-render from source on resize.
            if let Some(source) = source {
                self.app_event_tx.send(AppEvent::ConsolidateAgentMessage {
                    source,
                    cwd: self.config.cwd.to_path_buf(),
                });
            }
        }
        self.adaptive_chunking.reset();
        if had_stream_controller && self.stream_controllers_idle() {
            self.app_event_tx.send(AppEvent::StopCommitAnimation);
        }
    }

    fn stream_controllers_idle(&self) -> bool {
        self.stream_controller
            .as_ref()
            .map(|controller| controller.queued_lines() == 0)
            .unwrap_or(true)
            && self
                .plan_stream_controller
                .as_ref()
                .map(|controller| controller.queued_lines() == 0)
                .unwrap_or(true)
    }

    /// Restore the status indicator only after commentary completion is pending,
    /// the turn is still running, and all stream queues have drained.
    ///
    /// This gate prevents flicker while normal output is still actively
    /// streaming, but still restores a visible "working" affordance when a
    /// commentary block ends before the turn itself has completed.
    fn maybe_restore_status_indicator_after_stream_idle(&mut self) {
        if !self.pending_status_indicator_restore
            || !self.bottom_pane.is_task_running()
            || !self.stream_controllers_idle()
        {
            return;
        }

        self.bottom_pane.ensure_status_indicator();
        self.set_status(
            self.current_status.header.clone(),
            self.current_status.details.clone(),
            StatusDetailsCapitalization::Preserve,
            self.current_status.details_max_lines,
        );
        self.pending_status_indicator_restore = false;
    }

    /// Update the status indicator header and details.
    ///
    /// Passing `None` clears any existing details.
    fn set_status(
        &mut self,
        header: String,
        details: Option<String>,
        details_capitalization: StatusDetailsCapitalization,
        details_max_lines: usize,
    ) {
        let details = details
            .filter(|details| !details.is_empty())
            .map(|details| {
                let trimmed = details.trim_start();
                match details_capitalization {
                    StatusDetailsCapitalization::CapitalizeFirst => {
                        crate::text_formatting::capitalize_first(trimmed)
                    }
                    StatusDetailsCapitalization::Preserve => trimmed.to_string(),
                }
            });
        self.current_status = StatusIndicatorState {
            header: header.clone(),
            details: details.clone(),
            details_max_lines,
        };
        self.bottom_pane.update_status(
            header,
            details,
            StatusDetailsCapitalization::Preserve,
            details_max_lines,
        );
        let title_uses_status = self
            .config
            .tui_terminal_title
            .as_ref()
            .is_some_and(|items| {
                items
                    .iter()
                    .any(|item| item == "run-state" || item == "status")
            });
        if title_uses_status {
            self.refresh_status_surfaces();
        }
    }

    /// Convenience wrapper around [`Self::set_status`];
    /// updates the status indicator header and clears any existing details.
    fn set_status_header(&mut self, header: String) {
        self.set_status(
            header,
            /*details*/ None,
            StatusDetailsCapitalization::CapitalizeFirst,
            STATUS_DETAILS_DEFAULT_MAX_LINES,
        );
    }

    /// Sets the currently rendered footer status-line value.
    pub(crate) fn set_status_line(&mut self, status_line: Option<Line<'static>>) {
        self.bottom_pane.set_status_line(status_line);
    }

    /// Forwards the contextual active-agent label into the bottom-pane footer pipeline.
    ///
    /// `ChatWidget` stays a pass-through here so `App` remains the owner of "which thread is the
    /// user actually looking at?" and the footer stack remains a pure renderer of that decision.
    pub(crate) fn set_active_agent_label(&mut self, active_agent_label: Option<String>) {
        self.bottom_pane.set_active_agent_label(active_agent_label);
    }

    /// Recomputes footer status-line content from config and current runtime state.
    ///
    /// This method is the status-line orchestrator: it parses configured item identifiers,
    /// warns once per session about invalid items, updates whether status-line mode is enabled,
    /// schedules async git-branch lookup when needed, and renders only values that are currently
    /// available.
    ///
    /// The omission behavior is intentional. If selected items are unavailable (for example before
    /// a session id exists or before branch lookup completes), those items are skipped without
    /// placeholders so the line remains compact and stable.
    pub(crate) fn refresh_status_line(&mut self) {
        self.refresh_status_surfaces();
    }

    fn status_surface_preview_data(&mut self) -> StatusSurfacePreviewData {
        StatusSurfacePreviewData::from_iter(StatusSurfacePreviewItem::iter().filter_map(|item| {
            self.status_surface_preview_value_for_item(item)
                .map(|value| (item, value))
        }))
    }

    fn terminal_title_preview_data(&mut self) -> StatusSurfacePreviewData {
        let mut data = self.status_surface_preview_data();
        data.set_placeholder(StatusSurfacePreviewItem::ProjectName, "project");
        data
    }

    /// Opens the interactive terminal-title setup surface.
    pub(crate) fn open_terminal_title_setup(&mut self) {
        let preview_data = self.terminal_title_preview_data();
        let view = TerminalTitleSetupView::new(
            self.config.tui_terminal_title.as_deref(),
            preview_data,
            self.app_event_tx.clone(),
        );
        self.bottom_pane.show_view(Box::new(view));
        self.request_redraw();
    }

    /// Opens the interactive footer status-line setup surface.
    pub(crate) fn open_status_line_setup(&mut self) {
        let preview_data = self.status_surface_preview_data();
        let view = StatusLineSetupView::new(
            self.config.tui_status_line.as_deref(),
            self.config.tui_status_line_use_colors,
            preview_data,
            self.app_event_tx.clone(),
        );
        self.bottom_pane.show_view(Box::new(view));
        self.request_redraw();
    }

    /// Opens the syntax/theme picker using the live config and current terminal width.
    pub(crate) fn open_theme_picker(&mut self) {
        let terminal_width = self
            .last_rendered_width
            .get()
            .and_then(|width| u16::try_from(width).ok());
        let params = crate::theme_picker::build_theme_picker_params(
            self.config.tui_theme.as_deref(),
            Some(self.config.vac_home.as_path()),
            terminal_width,
        );
        self.bottom_pane.show_selection_view(params);
        self.request_redraw();
    }

    fn status_line_total_usage(&self) -> TokenUsage {
        self.token_info
            .as_ref()
            .map(|info| info.total_token_usage.clone())
            .unwrap_or(TokenUsage {
                input_tokens: 0,
                cached_input_tokens: 0,
                output_tokens: 0,
                reasoning_output_tokens: 0,
                total_tokens: 0,
            })
    }

    fn status_line_context_remaining_percent(&self) -> Option<i64> {
        match self.token_info.as_ref() {
            Some(info) => self
                .context_remaining_percent(info)
                .map(|percent| percent.clamp(0, 100)),
            None => Some(100),
        }
    }

    fn status_line_context_used_percent(&self) -> Option<i64> {
        self.status_line_context_remaining_percent()
            .map(|remaining| 100 - remaining)
    }

    fn status_line_limit_display(
        &self,
        window: Option<&RateLimitWindowDisplay>,
        label: &str,
    ) -> Option<String> {
        let window = window?;
        let remaining = (100.0 - window.used_percent).clamp(0.0, 100.0);
        Some(format!("{label} {remaining:.0}% left"))
    }

    fn status_line_context_window_size(&self) -> Option<i64> {
        self.token_info
            .as_ref()
            .and_then(|info| info.model_context_window)
    }

    fn status_line_reasoning_effort_label(effort: Option<ReasoningEffortConfig>) -> &'static str {
        match effort {
            Some(ReasoningEffortConfig::Minimal) => "minimal",
            Some(ReasoningEffortConfig::Low) => "low",
            Some(ReasoningEffortConfig::Medium) => "medium",
            Some(ReasoningEffortConfig::High) => "high",
            Some(ReasoningEffortConfig::XHigh) => "xhigh",
            None | Some(ReasoningEffortConfig::None) => "auto",
        }
    }

    /// Records that status-line setup was canceled.
    ///
    /// Cancellation is intentionally side-effect free for config state; the existing configuration
    /// remains active and no persistence is attempted.
    pub(crate) fn cancel_status_line_setup(&self) {
        tracing::info!("Status line setup canceled by user");
    }

    /// Applies status-line item selection from the setup view to in-memory config.
    ///
    /// An empty selection persists as an explicit empty list.
    pub(crate) fn setup_status_line(&mut self, items: Vec<StatusLineItem>, use_theme_colors: bool) {
        tracing::info!(
            "status line setup confirmed with items: {items:#?}, use_theme_colors: {use_theme_colors}"
        );
        let ids = items.iter().map(ToString::to_string).collect::<Vec<_>>();
        if self.config.tui_status_line_use_colors != use_theme_colors {
            self.bump_style_epoch();
        }
        self.config.tui_status_line = Some(ids);
        self.config.tui_status_line_use_colors = use_theme_colors;
        self.refresh_status_line();
    }

    /// Applies a temporary terminal-title selection while the setup UI is open.
    pub(crate) fn preview_terminal_title(&mut self, items: Vec<TerminalTitleItem>) {
        if self.terminal_title_setup_original_items.is_none() {
            self.terminal_title_setup_original_items = Some(self.config.tui_terminal_title.clone());
        }

        let ids = items.iter().map(ToString::to_string).collect::<Vec<_>>();
        self.config.tui_terminal_title = Some(ids);
        self.refresh_terminal_title();
    }

    /// Restores the terminal-title config that was active before the setup UI
    /// opened, undoing any preview changes. No-op if no setup session is active.
    pub(crate) fn revert_terminal_title_setup_preview(&mut self) {
        let Some(original_items) = self.terminal_title_setup_original_items.take() else {
            return;
        };

        self.config.tui_terminal_title = original_items;
        self.refresh_terminal_title();
    }

    /// Dismisses the terminal-title setup UI and reverts to the pre-setup config.
    pub(crate) fn cancel_terminal_title_setup(&mut self) {
        tracing::info!("Terminal title setup canceled by user");
        self.revert_terminal_title_setup_preview();
    }

    /// Commits a confirmed terminal-title selection, ending the setup session.
    ///
    /// After this call, `revert_terminal_title_setup_preview` becomes a no-op
    /// because the original config snapshot is discarded.
    pub(crate) fn setup_terminal_title(&mut self, items: Vec<TerminalTitleItem>) {
        tracing::info!("terminal title setup confirmed with items: {items:#?}");
        let ids = items.iter().map(ToString::to_string).collect::<Vec<_>>();
        self.terminal_title_setup_original_items = None;
        self.config.tui_terminal_title = Some(ids);
        self.refresh_terminal_title();
    }

    /// Stores async git-branch lookup results for the current status-line cwd.
    ///
    /// Results are dropped when they target an out-of-date cwd to avoid rendering stale branch
    /// names after directory changes.
    pub(crate) fn set_status_line_branch(&mut self, cwd: PathBuf, branch: Option<String>) {
        if self.status_line_branch_cwd.as_ref() != Some(&cwd) {
            self.status_line_branch_pending = false;
            return;
        }
        self.status_line_branch = branch;
        self.status_line_branch_pending = false;
        self.status_line_branch_lookup_complete = true;
        self.refresh_status_surfaces();
    }

    fn collect_runtime_metrics_delta(&mut self) {
        if let Some(delta) = self.session_telemetry.runtime_metrics_summary() {
            self.apply_runtime_metrics_delta(delta);
        }
    }

    fn apply_runtime_metrics_delta(&mut self, delta: RuntimeMetricsSummary) {
        let should_log_timing = has_websocket_timing_metrics(delta);
        self.turn_runtime_metrics.merge(delta);
        if should_log_timing {
            self.log_websocket_timing_totals(delta);
        }
    }

    fn log_websocket_timing_totals(&mut self, delta: RuntimeMetricsSummary) {
        if let Some(label) = history_cell::runtime_metrics_label(delta.responses_api_summary()) {
            self.add_plain_history_lines(vec![
                vec!["• ".dim(), format!("WebSocket timing: {label}").dark_gray()].into(),
            ]);
        }
    }

    fn refresh_runtime_metrics(&mut self) {
        self.collect_runtime_metrics_delta();
    }

    fn restore_retry_status_header_if_present(&mut self) {
        if let Some(header) = self.retry_status_header.take() {
            self.set_status_header(header);
        }
    }

    /// Record or update the raw markdown for the current agent turn.
    fn record_agent_markdown(&mut self, message: &str) {
        if message.is_empty() {
            return;
        }
        let markdown = message.to_string();
        match self.agent_turn_markdowns.last_mut() {
            Some(entry) if entry.user_turn_count == self.visible_user_turn_count => {
                entry.markdown = markdown.clone();
            }
            _ => {
                self.agent_turn_markdowns.push(AgentTurnMarkdown {
                    user_turn_count: self.visible_user_turn_count,
                    markdown: markdown.clone(),
                });
                if self.agent_turn_markdowns.len() > MAX_AGENT_COPY_HISTORY {
                    self.agent_turn_markdowns.remove(0);
                }
            }
        }
        self.last_agent_markdown = Some(markdown);
        self.copy_history_evicted_by_rollback = false;
        self.saw_copy_source_this_turn = true;
    }

    fn record_visible_user_turn_for_copy(&mut self) {
        self.visible_user_turn_count = self.visible_user_turn_count.saturating_add(1);
    }

    // --- Small event handlers ---
    fn on_session_configured_with_display_and_branch_parent_title(
        &mut self,
        session: ThreadSessionState,
        display: SessionConfiguredDisplay,
        branch_parent_title: Option<String>,
    ) {
        self.last_agent_markdown = None;
        self.agent_turn_markdowns.clear();
        self.visible_user_turn_count = 0;
        self.copy_history_evicted_by_rollback = false;
        self.saw_copy_source_this_turn = false;
        let history_entry_count =
            usize::try_from(session.history_entry_count).unwrap_or(usize::MAX);
        self.bottom_pane
            .set_history_metadata(session.history_log_id, history_entry_count);
        self.set_skills(/*skills*/ None);
        self.session_network_proxy = session.network_proxy.clone();
        let previous_thread_id = self.thread_id;
        self.thread_id = Some(session.thread_id);
        if previous_thread_id != self.thread_id {
            self.recent_auto_review_denials = RecentAutoReviewDenials::default();
        }
        self.refresh_plan_mode_nudge();
        self.last_turn_id = None;
        self.thread_name = session.thread_name.clone();
        self.current_goal_status_indicator = None;
        self.current_goal_status = None;
        self.goal_status_active_turn_started_at = None;
        self.budget_limited_turn_ids.clear();
        self.update_collaboration_mode_indicator();
        self.branched_from = session.branched_from_id;
        self.current_rollout_path = session.rollout_path.clone();
        self.current_cwd = Some(session.cwd.to_path_buf());
        self.config.cwd = session.cwd.clone();
        self.effective_service_tier = session.service_tier;
        if let Err(err) = self
            .config
            .permissions
            .approval_policy
            .set(session.approval_policy)
        {
            tracing::warn!(%err, "failed to sync approval_policy from SessionConfigured");
            self.config.permissions.approval_policy =
                Constrained::allow_only(session.approval_policy);
        }
        let permission_sync = self
            .config
            .permissions
            .set_permission_profile_with_active_profile(
                session.permission_profile.clone(),
                session.active_permission_profile.clone(),
            );
        if let Err(err) = permission_sync {
            tracing::warn!(%err, "failed to sync permissions from SessionConfigured");
            self.config.permissions.permission_profile =
                Constrained::allow_only(session.permission_profile.clone());
            self.config.permissions.active_permission_profile =
                session.active_permission_profile.clone();
        }
        self.config.approvals_reviewer = session.approvals_reviewer;
        self.status_line_project_root_name_cache = None;
        let branched_from_id = session.branched_from_id;
        let model_for_header = session.model.clone();
        self.session_header.set_model(&model_for_header);
        self.current_collaboration_mode = self.current_collaboration_mode.with_updates(
            Some(model_for_header.clone()),
            Some(session.reasoning_effort),
            /*developer_instructions*/ None,
        );
        if let Some(mask) = self.active_collaboration_mask.as_mut() {
            mask.model = Some(model_for_header.clone());
            mask.reasoning_effort = Some(session.reasoning_effort);
        }
        self.refresh_model_display();
        self.refresh_status_surfaces();
        self.sync_fast_command_enabled();
        self.sync_personality_command_enabled();
        self.sync_plugins_command_enabled();
        self.sync_goal_command_enabled();
        self.refresh_plugin_mentions();
        if display == SessionConfiguredDisplay::Normal {
            let startup_tooltip_override = self.startup_tooltip_override.take();
            let show_fast_status =
                self.should_show_fast_status(&model_for_header, session.service_tier);
            let session_info_cell = history_cell::new_session_info(
                &self.config,
                &model_for_header,
                &session,
                self.show_welcome_banner,
                startup_tooltip_override,
                self.plan_type,
                show_fast_status,
            );
            self.apply_session_info_cell(session_info_cell);
        } else if self
            .active_cell
            .as_ref()
            .is_some_and(|cell| cell.as_any().is::<history_cell::SessionHeaderHistoryCell>())
        {
            self.active_cell = None;
            self.bump_active_cell_revision();
        }
        self.saw_copy_source_this_turn = false;
        self.refresh_skills_for_current_cwd(/*force_reload*/ true);
        if self.connectors_enabled() {
            self.prefetch_connectors();
        }
        if let Some(user_message) = self.initial_user_message.take() {
            if self.suppress_initial_user_message_submit {
                self.initial_user_message = Some(user_message);
            } else {
                self.submit_user_message(user_message);
            }
        }
        if display == SessionConfiguredDisplay::Normal
            && let Some(branched_from_id) = branched_from_id
        {
            self.emit_branched_thread_event(branched_from_id, branch_parent_title);
        }
        if !self.suppress_session_configured_redraw {
            self.request_redraw();
        }
    }

    pub(crate) fn set_initial_user_message_submit_suppressed(&mut self, suppressed: bool) {
        self.suppress_initial_user_message_submit = suppressed;
    }

    pub(crate) fn submit_initial_user_message_if_pending(&mut self) {
        if let Some(user_message) = self.initial_user_message.take() {
            self.submit_user_message(user_message);
        }
    }

    pub(crate) fn handle_thread_session(&mut self, session: ThreadSessionState) {
        self.instruction_source_paths = session.instruction_source_paths.clone();
        let branch_parent_title = session.branch_parent_title.clone();
        self.on_session_configured_with_display_and_branch_parent_title(
            session,
            SessionConfiguredDisplay::Normal,
            branch_parent_title,
        );
    }

    pub(crate) fn handle_thread_session_quiet(&mut self, session: ThreadSessionState) {
        self.instruction_source_paths = session.instruction_source_paths.clone();
        self.on_session_configured_with_display_and_branch_parent_title(
            session,
            SessionConfiguredDisplay::Quiet,
            /*branch_parent_title*/ None,
        );
    }

    pub(crate) fn handle_side_thread_session(&mut self, session: ThreadSessionState) {
        self.instruction_source_paths = session.instruction_source_paths.clone();
        let branch_parent_title = session.branch_parent_title.clone();
        self.on_session_configured_with_display_and_branch_parent_title(
            session,
            SessionConfiguredDisplay::SideConversation,
            branch_parent_title,
        );
    }

    fn emit_branched_thread_event(
        &mut self,
        branched_from_id: ThreadId,
        branch_parent_title: Option<String>,
    ) {
        let branched_from_id_text = branched_from_id.to_string();
        let line: Line<'static> = if let Some(name) = branch_parent_title
            && !name.trim().is_empty()
        {
            vec![
                "• ".dim(),
                "Thread branched from ".into(),
                name.cyan(),
                " (".into(),
                branched_from_id_text.cyan(),
                ")".into(),
            ]
            .into()
        } else {
            vec![
                "• ".dim(),
                "Thread branched from ".into(),
                branched_from_id_text.cyan(),
            ]
            .into()
        };
        self.app_event_tx.send(AppEvent::InsertHistoryCell(Box::new(
            PlainHistoryCell::new(vec![line]),
        )));
    }

    fn on_thread_name_updated(&mut self, thread_id: ThreadId, thread_name: Option<String>) {
        if self.thread_id == Some(thread_id) {
            if let Some(name) = thread_name.as_deref() {
                let cell = Self::rename_confirmation_cell(name, self.thread_id);
                self.add_boxed_history(Box::new(cell));
            }
            self.thread_name = thread_name;
            self.refresh_status_surfaces();
            self.request_redraw();
            self.maybe_send_next_queued_input();
        }
    }

    fn set_skills(&mut self, skills: Option<Vec<SkillMetadata>>) {
        self.bottom_pane.set_skills(skills);
    }

    pub(crate) fn open_feedback_note(
        &mut self,
        category: crate::app_event::FeedbackCategory,
        include_logs: bool,
    ) {
        self.show_feedback_note(category, include_logs);
    }

    fn show_feedback_note(
        &mut self,
        category: crate::app_event::FeedbackCategory,
        include_logs: bool,
    ) {
        let view = crate::bottom_pane::FeedbackNoteView::new(
            category,
            self.last_turn_id.clone(),
            self.app_event_tx.clone(),
            include_logs,
        );
        self.bottom_pane.show_view(Box::new(view));
        self.request_redraw();
    }

    pub(crate) fn open_app_link_view(&mut self, params: crate::bottom_pane::AppLinkViewParams) {
        let view = crate::bottom_pane::AppLinkView::new(params, self.app_event_tx.clone());
        self.bottom_pane.show_view(Box::new(view));
        self.request_redraw();
    }

    pub(crate) fn dismiss_app_server_request(&mut self, request: &ResolvedAppServerRequest) {
        // A remotely resolved request must not remain user-actionable. It may be
        // materialized in the bottom pane or still deferred behind active streaming.
        let removed_deferred = self.interrupts.remove_resolved_prompt(request);
        let removed_visible = self.bottom_pane.dismiss_app_server_request(request);
        if removed_deferred || removed_visible {
            self.request_redraw();
        }
    }

    pub(crate) fn open_feedback_consent(&mut self, category: crate::app_event::FeedbackCategory) {
        let snapshot = self.feedback.snapshot(self.thread_id);
        let params = crate::bottom_pane::feedback_upload_consent_params(
            self.app_event_tx.clone(),
            category,
            self.current_rollout_path.clone(),
            self.thread_id
                .map(|thread_id| format!("auto-review-rollout-{thread_id}.jsonl")),
            snapshot.feedback_diagnostics(),
        );
        self.bottom_pane.show_selection_view(params);
        self.request_redraw();
    }

    fn finalize_completed_assistant_message(&mut self, message: Option<&str>) {
        // If we have a stream_controller, the finalized message payload is redundant because the
        // visible content has already been accumulated through deltas.
        if self.stream_controller.is_none()
            && let Some(message) = message
            && !message.is_empty()
        {
            self.handle_streaming_delta(message.to_string());
        }
        self.flush_answer_stream_with_separator();
        self.handle_stream_finished();
        self.request_redraw();
    }

    fn on_agent_message_delta(&mut self, delta: String) {
        self.handle_streaming_delta(delta);
    }

    fn on_plan_delta(&mut self, delta: String) {
        if self.active_mode_kind() != ModeKind::Plan {
            return;
        }
        if !self.plan_item_active {
            self.plan_item_active = true;
            self.plan_delta_buffer.clear();
        }
        self.plan_delta_buffer.push_str(&delta);
        // Before streaming plan content, flush any active exec cell group.
        self.flush_unified_exec_wait_streak();
        self.flush_active_cell();

        if self.plan_stream_controller.is_none() {
            self.plan_stream_controller = Some(PlanStreamController::new(
                self.current_stream_width(/*reserved_cols*/ 4),
                &self.config.cwd,
            ));
        }
        if let Some(controller) = self.plan_stream_controller.as_mut()
            && controller.push(&delta)
        {
            self.app_event_tx.send(AppEvent::StartCommitAnimation);
            self.run_catch_up_commit_tick();
        }
        self.request_redraw();
    }

    fn on_plan_item_completed(&mut self, text: String) {
        let streamed_plan = self.plan_delta_buffer.trim().to_string();
        let plan_text = if text.trim().is_empty() {
            streamed_plan
        } else {
            text
        };
        if !plan_text.trim().is_empty() {
            self.record_agent_markdown(&plan_text);
            self.latest_proposed_plan_markdown = Some(plan_text.clone());
        }
        // Plan commit ticks can hide the status row; remember whether we streamed plan output so
        // completion can restore it once stream queues are idle.
        let should_restore_after_stream = self.plan_stream_controller.is_some();
        self.plan_delta_buffer.clear();
        self.plan_item_active = false;
        self.saw_plan_item_this_turn = true;
        let (finalized_streamed_cell, consolidated_plan_source) =
            if let Some(mut controller) = self.plan_stream_controller.take() {
                controller.finalize()
            } else {
                (None, None)
            };
        if let Some(cell) = finalized_streamed_cell {
            self.add_boxed_history(cell);
            // VAC-DEBT(owner=vac-o5o6-zero-residual,target=post-cargo-hardening): Replace streamed output with the final plan item text if plan streaming is
            // removed or if we need to reconcile mismatches between streamed and final content.
            if let Some(source) = consolidated_plan_source {
                self.app_event_tx
                    .send(AppEvent::ConsolidateProposedPlan(source));
            }
        } else if !plan_text.is_empty() {
            self.add_to_history(history_cell::new_proposed_plan(plan_text, &self.config.cwd));
        } else if let Some(source) = consolidated_plan_source {
            self.app_event_tx
                .send(AppEvent::ConsolidateProposedPlan(source));
        }
        if should_restore_after_stream {
            self.pending_status_indicator_restore = true;
            self.maybe_restore_status_indicator_after_stream_idle();
        }
    }

    fn on_agent_reasoning_delta(&mut self, delta: String) {
        // For reasoning deltas, do not stream to history. Accumulate the
        // current reasoning block and extract the first bold element
        // (between **/**) as the chunk header. Show this header as status.
        self.reasoning_buffer.push_str(&delta);

        if self.unified_exec_wait_streak.is_some() {
            // Unified exec waiting should take precedence over reasoning-derived status headers.
            self.request_redraw();
            return;
        }

        if let Some(header) = extract_first_bold(&self.reasoning_buffer) {
            // Update the shimmer header to the extracted reasoning chunk header.
            self.terminal_title_status_kind = TerminalTitleStatusKind::Thinking;
            self.set_status_header(header);
        } else {
            // Fallback while we don't yet have a bold header: leave existing header as-is.
        }
        self.request_redraw();
    }
}
