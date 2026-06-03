// O5/O6 balanced ChatWidget impl group: source split_005_on_agent_reasoning_final.rs
impl ChatWidget {
    fn on_agent_reasoning_final(&mut self) {
        // At the end of a reasoning block, record transcript-only content.
        self.full_reasoning_buffer.push_str(&self.reasoning_buffer);
        if !self.full_reasoning_buffer.is_empty() {
            let cell = history_cell::new_reasoning_summary_block(
                self.full_reasoning_buffer.clone(),
                &self.config.cwd,
            );
            self.add_boxed_history(cell);
        }
        self.reasoning_buffer.clear();
        self.full_reasoning_buffer.clear();
        self.request_redraw();
    }

    fn on_reasoning_section_break(&mut self) {
        // Start a new reasoning block for header extraction and accumulate transcript.
        self.full_reasoning_buffer.push_str(&self.reasoning_buffer);
        self.full_reasoning_buffer.push_str("\n\n");
        self.reasoning_buffer.clear();
    }

    // Raw reasoning uses the same flow as summarized reasoning

    fn on_task_started(&mut self) {
        self.user_turn_pending_start = false;
        self.agent_turn_running = true;
        self.goal_status_active_turn_started_at = Some(Instant::now());
        self.turn_sleep_inhibitor
            .set_turn_running(/*turn_running*/ true);
        self.saw_copy_source_this_turn = false;
        self.saw_plan_update_this_turn = false;
        self.saw_plan_item_this_turn = false;
        self.had_work_activity = false;
        self.latest_proposed_plan_markdown = None;
        self.plan_delta_buffer.clear();
        self.plan_item_active = false;
        self.adaptive_chunking.reset();
        self.plan_stream_controller = None;
        self.turn_runtime_metrics = RuntimeMetricsSummary::default();
        self.session_telemetry.reset_runtime_metrics();
        self.bottom_pane.clear_quit_shortcut_hint();
        self.quit_shortcut_expires_at = None;
        self.quit_shortcut_key = None;
        self.update_task_running_state();
        self.retry_status_header = None;
        if self.active_hook_cell.take().is_some() {
            self.bump_active_cell_revision();
        }
        self.pending_status_indicator_restore = false;
        self.bottom_pane
            .set_interrupt_hint_visible(/*visible*/ true);
        self.terminal_title_status_kind = TerminalTitleStatusKind::Working;
        self.set_status_header(String::from("Working"));
        self.full_reasoning_buffer.clear();
        self.reasoning_buffer.clear();
        self.request_redraw();
    }

    fn on_task_complete(
        &mut self,
        last_agent_message: Option<String>,
        duration_ms: Option<i64>,
        from_replay: bool,
    ) {
        self.submit_pending_steers_after_interrupt = false;
        // Use `last_agent_message` from the turn-complete notification as the copy
        // source only when no earlier item-level event (AgentMessageItem, plan
        // commit, review output) already recorded markdown for this turn. This
        // prevents the final summary from overwriting a more specific source.
        if let Some(message) = last_agent_message
            .as_ref()
            .filter(|message| !message.is_empty())
            && !self.saw_copy_source_this_turn
        {
            self.record_agent_markdown(message);
        }
        // For desktop notifications: prefer the notification payload, fall back to
        // the item-level copy source if present, otherwise send an empty string.
        let notification_response = last_agent_message
            .as_ref()
            .filter(|message| !message.is_empty())
            .cloned()
            .or_else(|| {
                if self.saw_copy_source_this_turn {
                    self.last_agent_markdown.clone()
                } else {
                    None
                }
            })
            .unwrap_or_default();
        self.saw_copy_source_this_turn = false;
        // If a stream is currently active, finalize it.
        self.flush_answer_stream_with_separator();
        if let Some(mut controller) = self.plan_stream_controller.take() {
            let (cell, source) = controller.finalize();
            if let Some(cell) = cell {
                self.add_boxed_history(cell);
            }
            if let Some(source) = source {
                self.app_event_tx
                    .send(AppEvent::ConsolidateProposedPlan(source));
            }
        }
        self.flush_unified_exec_wait_streak();
        if !from_replay {
            self.collect_runtime_metrics_delta();
            let runtime_metrics =
                (!self.turn_runtime_metrics.is_empty()).then_some(self.turn_runtime_metrics);
            let show_work_separator = self.had_work_activity
                && (self.needs_final_message_separator || runtime_metrics.is_some());
            if show_work_separator || runtime_metrics.is_some() {
                let elapsed_seconds = if show_work_separator {
                    duration_ms
                        .and_then(|duration_ms| u64::try_from(duration_ms).ok())
                        .map(|duration_ms| duration_ms / 1_000)
                        .or_else(|| {
                            self.bottom_pane
                                .status_widget()
                                .map(super::status_indicator_widget::StatusIndicatorWidget::elapsed_seconds)
                        })
                } else {
                    None
                };
                self.add_to_history(history_cell::FinalMessageSeparator::new(
                    elapsed_seconds,
                    runtime_metrics,
                ));
            }
            self.turn_runtime_metrics = RuntimeMetricsSummary::default();
            self.needs_final_message_separator = false;
            self.had_work_activity = false;
            self.request_status_line_branch_refresh();
        }
        // Mark task stopped and request redraw now that all content is in history.
        self.pending_status_indicator_restore = false;
        self.user_turn_pending_start = false;
        self.agent_turn_running = false;
        self.goal_status_active_turn_started_at = None;
        self.turn_sleep_inhibitor
            .set_turn_running(/*turn_running*/ false);
        self.update_task_running_state();
        self.running_commands.clear();
        self.suppressed_exec_calls.clear();
        self.last_unified_wait = None;
        self.unified_exec_wait_streak = None;
        self.request_redraw();

        let had_pending_steers = !self.pending_steers.is_empty();
        self.refresh_pending_input_preview();

        if !from_replay && !self.has_queued_follow_up_messages() && !had_pending_steers {
            self.maybe_prompt_plan_implementation();
        }
        // Keep this flag for replayed completion events so a subsequent live TurnComplete can
        // still show the prompt once after thread switch replay.
        if !from_replay {
            self.saw_plan_item_this_turn = false;
        }
        // If there is a queued user message, send exactly one now to begin the next turn.
        let follow_up_started = self.maybe_send_next_queued_input();
        let active_goal_continuing = self
            .current_goal_status
            .as_ref()
            .is_some_and(GoalStatusState::is_active);
        // Emit a notification when the agent is truly waiting for the user.
        // Queued follow-up input and active goal continuation both start the
        // next turn immediately, so notifying at that boundary would feel like
        // a false "needs attention".
        if !follow_up_started && !active_goal_continuing {
            self.notify(Notification::AgentTurnComplete {
                response: notification_response,
            });
        }

        self.maybe_show_pending_rate_limit_prompt();
    }

    fn maybe_prompt_plan_implementation(&mut self) {
        if !self.collaboration_modes_enabled() {
            return;
        }
        if self.has_queued_follow_up_messages() {
            return;
        }
        if self.active_mode_kind() != ModeKind::Plan {
            return;
        }
        if !self.saw_plan_item_this_turn {
            return;
        }
        if !self.bottom_pane.no_modal_or_popup_active() {
            return;
        }

        if matches!(
            self.rate_limit_switch_prompt,
            RateLimitSwitchPromptState::Pending
        ) {
            return;
        }

        self.open_plan_implementation_prompt();
    }

    fn open_plan_implementation_prompt(&mut self) {
        let default_mask = collaboration_modes::default_mode_mask(self.model_catalog.as_ref());
        let context_usage_label = self.plan_implementation_context_usage_label();

        self.bottom_pane
            .show_selection_view(plan_implementation::selection_view_params(
                default_mask,
                self.latest_proposed_plan_markdown.as_deref(),
                context_usage_label.as_deref(),
            ));
        self.notify(Notification::PlanModePrompt {
            title: PLAN_IMPLEMENTATION_TITLE.to_string(),
        });
    }

    /// Returns a context-used label for the plan implementation prompt.
    ///
    /// The footer reports context remaining because it is ambient status, but
    /// this prompt is asking whether to discard prior conversation state before
    /// implementing a plan. Reporting used context makes the cleanup tradeoff
    /// explicit. A fully fresh or unknown context window returns no label so
    /// the clear-context option does not imply urgency without evidence.
    fn plan_implementation_context_usage_label(&self) -> Option<String> {
        let info = self.token_info.as_ref()?;
        let percent = self.context_remaining_percent(info);

        let used_tokens = self.context_used_tokens(info, percent.is_some());
        if let Some(percent) = percent {
            let used_percent = 100 - percent.clamp(0, 100);
            if used_percent <= 0 {
                return None;
            }
            return Some(format!("{used_percent}% used"));
        }

        if let Some(tokens) = used_tokens
            && tokens > 0
        {
            return Some(format!("{} used", format_tokens_compact(tokens)));
        }

        None
    }

    fn has_queued_follow_up_messages(&self) -> bool {
        !self.rejected_steers_queue.is_empty() || !self.queued_user_messages.is_empty()
    }

    fn pop_next_queued_user_message(
        &mut self,
    ) -> Option<(QueuedUserMessage, UserMessageHistoryRecord)> {
        if self.rejected_steers_queue.is_empty() {
            self.queued_user_messages.pop_front().map(|user_message| {
                let history_record = self
                    .queued_user_message_history_records
                    .pop_front()
                    .unwrap_or(UserMessageHistoryRecord::UserMessageText);
                (user_message, history_record)
            })
        } else {
            let rejected_messages = self.rejected_steers_queue.drain(..).collect::<Vec<_>>();
            let mut history_records = self
                .rejected_steer_history_records
                .drain(..)
                .collect::<Vec<_>>();
            history_records.resize(
                rejected_messages.len(),
                UserMessageHistoryRecord::UserMessageText,
            );
            let (message, history_record) = merge_user_messages_with_history_record(
                rejected_messages
                    .into_iter()
                    .zip(history_records)
                    .collect::<Vec<_>>(),
            );
            Some((QueuedUserMessage::from(message), history_record))
        }
    }

    fn pop_latest_queued_user_message(&mut self) -> Option<UserMessage> {
        if let Some(user_message) = self.queued_user_messages.pop_back() {
            let history_record = self
                .queued_user_message_history_records
                .pop_back()
                .unwrap_or(UserMessageHistoryRecord::UserMessageText);
            Some(user_message_for_restore(
                user_message.into_user_message(),
                &history_record,
            ))
        } else {
            let user_message = self.rejected_steers_queue.pop_back()?;
            let history_record = self
                .rejected_steer_history_records
                .pop_back()
                .unwrap_or(UserMessageHistoryRecord::UserMessageText);
            Some(user_message_for_restore(user_message, &history_record))
        }
    }

    pub(crate) fn enqueue_rejected_steer(&mut self) -> bool {
        let Some(pending_steer) = self.pending_steers.pop_front() else {
            tracing::warn!(
                "received active-turn-not-steerable error without a matching pending steer"
            );
            return false;
        };
        self.rejected_steers_queue
            .push_back(pending_steer.user_message);
        self.rejected_steer_history_records
            .push_back(pending_steer.history_record);
        self.refresh_pending_input_preview();
        true
    }

    fn handle_app_server_steer_rejected_error(
        &mut self,
        vac_error_info: &AppServerVACErrorInfo,
    ) -> bool {
        matches!(
            vac_error_info,
            AppServerVACErrorInfo::ActiveTurnNotSteerable { .. }
        ) && self.enqueue_rejected_steer()
    }

    pub(crate) fn open_multi_agent_enable_prompt(&mut self) {
        let items = vec![
            SelectionItem {
                name: MULTI_AGENT_ENABLE_YES.to_string(),
                description: Some(
                    "Save the setting now. You will need a new session to use it.".to_string(),
                ),
                actions: vec![Box::new(|tx| {
                    tx.send(AppEvent::UpdateFeatureFlags {
                        updates: vec![(Feature::Collab, true)],
                    });
                    tx.send(AppEvent::InsertHistoryCell(Box::new(
                        history_cell::new_warning_event(MULTI_AGENT_ENABLE_NOTICE.to_string()),
                    )));
                })],
                dismiss_on_select: true,
                ..Default::default()
            },
            SelectionItem {
                name: MULTI_AGENT_ENABLE_NO.to_string(),
                description: Some("Keep subagents disabled.".to_string()),
                dismiss_on_select: true,
                ..Default::default()
            },
        ];

        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some(MULTI_AGENT_ENABLE_TITLE.to_string()),
            subtitle: Some("Subagents are currently disabled in your config.".to_string()),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            ..Default::default()
        });
    }

    pub(crate) fn open_memories_popup(&mut self) {
        if !self.config.features.enabled(Feature::MemoryTool) {
            self.open_memories_enable_prompt();
            return;
        }

        let view = MemoriesSettingsView::new(
            self.config.memories.use_memories,
            self.config.memories.generate_memories,
            self.app_event_tx.clone(),
        );
        self.bottom_pane.show_view(Box::new(view));
    }

    pub(crate) fn open_memories_enable_prompt(&mut self) {
        let items = vec![
            SelectionItem {
                name: MEMORIES_ENABLE_YES.to_string(),
                description: Some(
                    "Save the setting now. You will need a new session to use it.".to_string(),
                ),
                actions: vec![Box::new(|tx| {
                    tx.send(AppEvent::UpdateFeatureFlags {
                        updates: vec![(Feature::MemoryTool, true)],
                    });
                })],
                dismiss_on_select: true,
                ..Default::default()
            },
            SelectionItem {
                name: MEMORIES_ENABLE_NO.to_string(),
                description: Some("Keep memories disabled.".to_string()),
                dismiss_on_select: true,
                ..Default::default()
            },
        ];

        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some(MEMORIES_ENABLE_TITLE.to_string()),
            subtitle: Some("Memories are currently disabled in your config.".to_string()),
            footer_note: Some(Line::from(vec![
                "Learn more: ".dim(),
                MEMORIES_DOC_URL.cyan().underlined(),
            ])),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            ..Default::default()
        });
    }

    pub(crate) fn set_memory_settings(&mut self, use_memories: bool, generate_memories: bool) {
        self.config.memories.use_memories = use_memories;
        self.config.memories.generate_memories = generate_memories;
    }

    pub(crate) fn set_token_info(&mut self, info: Option<TokenUsageInfo>) {
        match info {
            Some(info) => self.apply_token_info(info),
            None => {
                self.bottom_pane
                    .set_context_window(/*percent*/ None, /*used_tokens*/ None);
                self.token_info = None;
            }
        }
    }
}
