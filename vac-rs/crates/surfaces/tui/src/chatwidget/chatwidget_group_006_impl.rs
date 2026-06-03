// O5/O6 balanced ChatWidget impl group: source split_006_apply_token_info.rs
impl ChatWidget {
    fn apply_token_info(&mut self, info: TokenUsageInfo) {
        let percent = self.context_remaining_percent(&info);
        let used_tokens = self.context_used_tokens(&info, percent.is_some());
        self.bottom_pane.set_context_window(percent, used_tokens);
        self.token_info = Some(info);
    }

    fn context_remaining_percent(&self, info: &TokenUsageInfo) -> Option<i64> {
        info.model_context_window.map(|window| {
            info.last_token_usage
                .percent_of_context_window_remaining(window)
        })
    }

    fn context_used_tokens(&self, info: &TokenUsageInfo, percent_known: bool) -> Option<i64> {
        if percent_known {
            return None;
        }

        Some(info.total_token_usage.tokens_in_context_window())
    }

    fn restore_pre_review_token_info(&mut self) {
        if let Some(saved) = self.pre_review_token_info.take() {
            match saved {
                Some(info) => self.apply_token_info(info),
                None => {
                    self.bottom_pane
                        .set_context_window(/*percent*/ None, /*used_tokens*/ None);
                    self.token_info = None;
                }
            }
        }
    }

    pub(crate) fn on_rate_limit_snapshot(&mut self, snapshot: Option<RateLimitSnapshot>) {
        if let Some(mut snapshot) = snapshot {
            let limit_id = snapshot
                .limit_id
                .clone()
                .unwrap_or_else(|| "vac".to_string());
            let limit_label = snapshot
                .limit_name
                .clone()
                .unwrap_or_else(|| limit_id.clone());
            if snapshot.credits.is_none() {
                snapshot.credits = self
                    .rate_limit_snapshots_by_limit_id
                    .get(&limit_id)
                    .and_then(|display| display.credits.as_ref())
                    .map(|credits| CreditsSnapshot {
                        has_credits: credits.has_credits,
                        unlimited: credits.unlimited,
                        balance: credits.balance.clone(),
                    });
            }

            self.plan_type = snapshot.plan_type.or(self.plan_type);

            let is_vac_limit = limit_id.eq_ignore_ascii_case("vac");
            if is_vac_limit && let Some(rate_limit_reached_type) = snapshot.rate_limit_reached_type
            {
                self.vac_rate_limit_reached_type = Some(rate_limit_reached_type);
            }
            let warnings = if is_vac_limit {
                self.rate_limit_warnings.take_warnings(
                    snapshot
                        .secondary
                        .as_ref()
                        .map(|window| f64::from(window.used_percent)),
                    snapshot
                        .secondary
                        .as_ref()
                        .and_then(|window| window.window_duration_mins),
                    snapshot
                        .primary
                        .as_ref()
                        .map(|window| f64::from(window.used_percent)),
                    snapshot
                        .primary
                        .as_ref()
                        .and_then(|window| window.window_duration_mins),
                )
            } else {
                vec![]
            };

            let high_usage = is_vac_limit
                && (snapshot
                    .secondary
                    .as_ref()
                    .map(|w| f64::from(w.used_percent) >= RATE_LIMIT_SWITCH_PROMPT_THRESHOLD)
                    .unwrap_or(false)
                    || snapshot
                        .primary
                        .as_ref()
                        .map(|w| f64::from(w.used_percent) >= RATE_LIMIT_SWITCH_PROMPT_THRESHOLD)
                        .unwrap_or(false));

            let has_workspace_credits = snapshot
                .credits
                .as_ref()
                .map(|credits| credits.has_credits)
                .unwrap_or(false);

            if high_usage
                && !has_workspace_credits
                && !self.rate_limit_switch_prompt_hidden()
                && self.current_model() != NUDGE_MODEL_SLUG
                && !matches!(
                    self.rate_limit_switch_prompt,
                    RateLimitSwitchPromptState::Shown
                )
            {
                self.rate_limit_switch_prompt = RateLimitSwitchPromptState::Pending;
            }

            let display =
                rate_limit_snapshot_display_for_limit(&snapshot, limit_label, Local::now());
            self.rate_limit_snapshots_by_limit_id
                .insert(limit_id, display);

            if !warnings.is_empty() {
                for warning in warnings {
                    self.add_to_history(history_cell::new_warning_event(warning));
                }
                self.request_redraw();
            }
        } else {
            self.rate_limit_snapshots_by_limit_id.clear();
            self.vac_rate_limit_reached_type = None;
        }
        self.refresh_status_line();
    }
    /// Finalize any active exec as failed and stop/clear agent-turn UI state.
    ///
    /// This does not clear MCP startup tracking, because MCP startup can overlap with turn cleanup
    /// and should continue to drive the bottom-pane running indicator while it is in progress.
    fn finalize_turn(&mut self) {
        // Ensure any spinner is replaced by a red ✗ and flushed into history.
        self.finalize_active_cell_as_failed();
        // Turn-scoped hook rows are transient live state; once the turn is over,
        // do not leave an orphaned running row behind if no matching completion
        // event arrived before cancellation.
        if self.active_hook_cell.take().is_some() {
            self.bump_active_cell_revision();
        }
        // Reset running state and clear streaming buffers.
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
        self.adaptive_chunking.reset();
        self.stream_controller = None;
        self.plan_stream_controller = None;
        self.pending_status_indicator_restore = false;
        self.request_status_line_branch_refresh();
        self.maybe_show_pending_rate_limit_prompt();
    }

    fn on_server_overloaded_error(&mut self, message: String) {
        self.submit_pending_steers_after_interrupt = false;
        self.finalize_turn();

        let message = if message.trim().is_empty() {
            "VAC is currently experiencing high load.".to_string()
        } else {
            message
        };

        self.add_to_history(history_cell::new_warning_event(message));
        self.request_redraw();
        self.maybe_send_next_queued_input();
    }

    fn on_error(&mut self, message: String) {
        self.submit_pending_steers_after_interrupt = false;
        self.finalize_turn();
        self.add_to_history(history_cell::new_error_event(message));
        self.request_redraw();

        // After an error ends the turn, try sending the next queued input.
        self.maybe_send_next_queued_input();
    }

    fn on_cyber_policy_error(&mut self) {
        self.submit_pending_steers_after_interrupt = false;
        self.finalize_turn();
        self.add_to_history(history_cell::new_cyber_policy_error_event());
        self.request_redraw();

        // After an error ends the turn, try sending the next queued input.
        self.maybe_send_next_queued_input();
    }

    fn workspace_owner_usage_nudge_enabled(&self) -> bool {
        self.config
            .features
            .enabled(Feature::WorkspaceOwnerUsageNudge)
    }

    fn on_rate_limit_error(&mut self, error_kind: RateLimitErrorKind, message: String) {
        if !self.workspace_owner_usage_nudge_enabled() {
            self.on_error(message);
            return;
        }

        let rate_limit_reached_type = self.vac_rate_limit_reached_type.map(|kind| {
            if matches!(error_kind, RateLimitErrorKind::UsageLimit) {
                match kind {
                    RateLimitReachedType::WorkspaceOwnerCreditsDepleted => {
                        RateLimitReachedType::WorkspaceOwnerUsageLimitReached
                    }
                    RateLimitReachedType::WorkspaceMemberCreditsDepleted => {
                        RateLimitReachedType::WorkspaceMemberUsageLimitReached
                    }
                    other => other,
                }
            } else {
                kind
            }
        });
        self.vac_rate_limit_reached_type = rate_limit_reached_type;

        match rate_limit_reached_type {
            Some(RateLimitReachedType::WorkspaceOwnerCreditsDepleted) => {
                self.on_error(
                    "You're out of credits. Your workspace is out of credits. Add credits to continue using VAC."
                        .to_string(),
                );
            }
            Some(RateLimitReachedType::WorkspaceOwnerUsageLimitReached) => {
                self.on_error(
                    "Usage limit reached. You've reached your usage limit. Increase your limits to continue using vac."
                        .to_string(),
                );
            }
            Some(RateLimitReachedType::WorkspaceMemberCreditsDepleted) => {
                self.on_error(message);
                self.open_workspace_owner_nudge_prompt(AddCreditsNudgeCreditType::Credits);
            }
            Some(RateLimitReachedType::WorkspaceMemberUsageLimitReached) => {
                self.on_error(message);
                self.open_workspace_owner_nudge_prompt(AddCreditsNudgeCreditType::UsageLimit);
            }
            Some(RateLimitReachedType::RateLimitReached) | None => {
                self.on_error(message);
            }
        }
    }

    fn handle_non_retry_error(
        &mut self,
        message: String,
        vac_error_info: Option<AppServerVACErrorInfo>,
    ) {
        if vac_error_info
            .as_ref()
            .is_some_and(|info| self.handle_app_server_steer_rejected_error(info))
        {
        } else if vac_error_info
            .as_ref()
            .is_some_and(is_app_server_cyber_policy_error)
        {
            self.on_cyber_policy_error();
        } else if let Some(info) = vac_error_info
            .as_ref()
            .and_then(app_server_rate_limit_error_kind)
        {
            match info {
                RateLimitErrorKind::ServerOverloaded => self.on_server_overloaded_error(message),
                RateLimitErrorKind::UsageLimit | RateLimitErrorKind::Generic => {
                    self.on_rate_limit_error(info, message)
                }
            }
        } else {
            self.on_error(message);
        }
    }

    fn on_warning(&mut self, message: impl Into<String>) {
        self.add_to_history(history_cell::new_warning_event(message.into()));
        self.request_redraw();
    }

    fn on_app_server_model_verification(&mut self, verifications: &[AppServerModelVerification]) {
        if verifications.contains(&AppServerModelVerification::TrustedAccessForCyber) {
            self.on_warning(TRUSTED_ACCESS_FOR_CYBER_VERIFICATION_WARNING);
        }
    }

    /// Handle a turn aborted due to user interrupt (Esc), budget exhaustion,
    /// or review completion.
    /// When there are queued user messages, restore them into the composer
    /// separated by newlines rather than auto‑submitting the next one.
    fn on_interrupted_turn(&mut self, reason: TurnAbortReason) {
        // Finalize, log a gentle prompt, and clear running state.
        self.finalize_turn();
        let send_pending_steers_immediately = self.submit_pending_steers_after_interrupt;
        self.submit_pending_steers_after_interrupt = false;
        if self.interrupted_turn_notice_mode != InterruptedTurnNoticeMode::Suppress {
            if send_pending_steers_immediately {
                self.add_to_history(history_cell::new_info_event(
                    "Model interrupted to submit steer instructions.".to_owned(),
                    /*hint*/ None,
                ));
            } else {
                self.add_to_history(history_cell::new_error_event(
                    self.interrupted_turn_message(reason),
                ));
            }
        }

        // The server has already discarded pending input by the time the
        // interrupted turn reaches the UI, so any unacknowledged steers still
        // tracked here must be restored locally instead of waiting for a later commit.
        if send_pending_steers_immediately {
            let pending_steers = self
                .pending_steers
                .drain(..)
                .map(|pending| (pending.user_message, pending.history_record))
                .collect::<Vec<_>>();
            if !pending_steers.is_empty() {
                let (user_message, history_record) =
                    merge_user_messages_with_history_record(pending_steers);
                self.submit_user_message_with_history_record(user_message, history_record);
            } else if let Some(combined) = self.drain_pending_messages_for_restore() {
                self.restore_user_message_to_composer(combined);
            }
        } else if let Some(combined) = self.drain_pending_messages_for_restore() {
            self.restore_user_message_to_composer(combined);
        }
        self.refresh_pending_input_preview();

        self.request_redraw();
    }

    /// Merge pending steers, queued drafts, and the current composer state into a single message.
    ///
    /// Each pending message numbers attachments from `[Image #1]` relative to its own remote
    /// images. When we concatenate multiple messages after interrupt, we must renumber local-image
    /// placeholders in a stable order and rebase text element byte ranges so the restored composer
    /// state stays aligned with the merged attachment list. Returns `None` when there is nothing to
    /// restore.
    fn drain_pending_messages_for_restore(&mut self) -> Option<UserMessage> {
        if self.pending_steers.is_empty() && !self.has_queued_follow_up_messages() {
            return None;
        }

        let existing_message = UserMessage {
            text: self.bottom_pane.composer_text(),
            text_elements: self.bottom_pane.composer_text_elements(),
            local_images: self.bottom_pane.composer_local_images(),
            remote_image_urls: self.bottom_pane.remote_image_urls(),
            mention_bindings: self.bottom_pane.composer_mention_bindings(),
        };

        let rejected_messages = self.rejected_steers_queue.drain(..).collect::<Vec<_>>();
        let mut rejected_history_records = self
            .rejected_steer_history_records
            .drain(..)
            .collect::<Vec<_>>();
        rejected_history_records.resize(
            rejected_messages.len(),
            UserMessageHistoryRecord::UserMessageText,
        );
        let mut to_merge: Vec<UserMessage> = rejected_messages
            .into_iter()
            .zip(rejected_history_records.iter())
            .map(|(message, history_record)| user_message_for_restore(message, history_record))
            .collect();
        to_merge.extend(
            self.pending_steers
                .drain(..)
                .map(|steer| user_message_for_restore(steer.user_message, &steer.history_record)),
        );
        let queued_messages = self.queued_user_messages.drain(..).collect::<Vec<_>>();
        let mut queued_history_records = self
            .queued_user_message_history_records
            .drain(..)
            .collect::<Vec<_>>();
        queued_history_records.resize(
            queued_messages.len(),
            UserMessageHistoryRecord::UserMessageText,
        );
        to_merge.extend(
            queued_messages
                .into_iter()
                .zip(queued_history_records.iter())
                .map(|(message, history_record)| {
                    user_message_for_restore(message.into_user_message(), history_record)
                }),
        );
        if !existing_message.text.is_empty()
            || !existing_message.local_images.is_empty()
            || !existing_message.remote_image_urls.is_empty()
        {
            to_merge.push(existing_message);
        }

        Some(merge_user_messages(to_merge))
    }

    pub(crate) fn restore_user_message_to_composer(&mut self, user_message: UserMessage) {
        let UserMessage {
            text,
            local_images,
            remote_image_urls,
            text_elements,
            mention_bindings,
        } = user_message;
        let local_image_paths = local_images.into_iter().map(|img| img.path).collect();
        self.set_remote_image_urls(remote_image_urls);
        self.bottom_pane.set_composer_text_with_mention_bindings(
            text,
            text_elements,
            local_image_paths,
            mention_bindings,
        );
    }
}
