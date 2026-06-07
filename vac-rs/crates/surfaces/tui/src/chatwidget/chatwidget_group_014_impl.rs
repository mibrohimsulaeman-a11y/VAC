// O5/O6 balanced ChatWidget impl group: source split_014_handle_skills_list_response.rs
impl ChatWidget {
    pub(crate) fn handle_skills_list_response(&mut self, response: SkillsListResponse) {
        self.on_list_skills(response);
    }

    fn handle_turn_completed_notification(
        &mut self,
        notification: TurnCompletedNotification,
        replay_kind: Option<ReplayKind>,
    ) {
        match notification.turn.status {
            TurnStatus::Completed => {
                #[allow(clippy::redundant_closure_for_method_calls)] // module `bridge` is private; method-path form inaccessible
                let runtime_events = self
                    .runtime_bridge
                    .as_mut()
                    .map(|bridge| bridge.project_task_completed());
                if let Some(events) = runtime_events {
                    self.project_runtime_events(events);
                }
                self.runtime_bridge = None;
                self.last_non_retry_error = None;
                self.on_task_complete(
                    /*last_agent_message*/ None,
                    notification.turn.duration_ms,
                    replay_kind.is_some(),
                )
            }
            TurnStatus::Interrupted => {
                self.last_non_retry_error = None;
                let reason = if self
                    .budget_limited_turn_ids
                    .remove(notification.turn.id.as_str())
                {
                    TurnAbortReason::BudgetLimited
                } else {
                    TurnAbortReason::Interrupted
                };
                let runtime_events = self.runtime_bridge.as_mut().map(|bridge| match &reason {
                    TurnAbortReason::BudgetLimited => {
                        bridge.project_task_failed("budget exhausted".to_string())
                    }
                    _ => bridge.project_task_cancelled(Some("interrupted".to_string())),
                });
                if let Some(events) = runtime_events {
                    self.project_runtime_events(events);
                }
                self.runtime_bridge = None;
                self.on_interrupted_turn(reason);
            }
            TurnStatus::Failed => {
                if let Some(error) = notification.turn.error {
                    let runtime_events = self
                        .runtime_bridge
                        .as_mut()
                        .map(|bridge| bridge.project_task_failed(error.message.clone()));
                    if let Some(events) = runtime_events {
                        self.project_runtime_events(events);
                    }
                    self.runtime_bridge = None;
                    if self.last_non_retry_error.as_ref()
                        == Some(&(notification.turn.id.clone(), error.message.clone()))
                    {
                        self.last_non_retry_error = None;
                    } else {
                        self.handle_non_retry_error(error.message, error.vac_error_info);
                    }
                } else {
                    let runtime_events = self
                        .runtime_bridge
                        .as_mut()
                        .map(|bridge| bridge.project_task_failed("turn failed".to_string()));
                    if let Some(events) = runtime_events {
                        self.project_runtime_events(events);
                    }
                    self.runtime_bridge = None;
                    self.last_non_retry_error = None;
                    self.finalize_turn();
                    self.request_redraw();
                    self.maybe_send_next_queued_input();
                }
            }
            TurnStatus::InProgress => {}
        }
    }

    fn handle_item_started_notification(
        &mut self,
        notification: ItemStartedNotification,
        from_replay: bool,
    ) {
        match notification.item {
            item @ ThreadItem::CommandExecution { .. } => self.on_command_execution_started(item),
            ThreadItem::FileChange { id: _, changes, .. } => {
                self.on_patch_apply_begin(file_update_changes_to_display(changes));
            }
            item @ ThreadItem::McpToolCall { .. } => self.on_mcp_tool_call_started(item),
            ThreadItem::WebSearch { id, .. } => {
                self.on_web_search_begin(id);
            }
            ThreadItem::ImageGeneration { .. } => {
                self.on_image_generation_begin();
            }
            ThreadItem::CollabAgentToolCall {
                id,
                tool,
                status,
                sender_thread_id,
                receiver_thread_ids,
                prompt,
                model,
                reasoning_effort,
                agents_states,
            } => self.on_collab_agent_tool_call(ThreadItem::CollabAgentToolCall {
                id,
                tool,
                status,
                sender_thread_id,
                receiver_thread_ids,
                prompt,
                model,
                reasoning_effort,
                agents_states,
            }),
            ThreadItem::EnteredReviewMode { review, .. }
                if !from_replay => {
                    self.enter_review_mode_with_hint(review, /*from_replay*/ false);
                }
            _ => {}
        }
    }

    fn handle_item_completed_notification(
        &mut self,
        notification: ItemCompletedNotification,
        replay_kind: Option<ReplayKind>,
    ) {
        self.handle_thread_item(
            notification.item,
            notification.turn_id,
            replay_kind.map_or(ThreadItemRenderSource::Live, ThreadItemRenderSource::Replay),
        );
    }

    fn on_patch_apply_output_delta(&mut self, _item_id: String, _delta: String) {}

    fn on_guardian_review_notification(
        &mut self,
        id: String,
        turn_id: String,
        review: crate::session_protocol::GuardianApprovalReview,
        decision_source: Option<crate::session_protocol::GuardianAssessmentDecisionSource>,
        action: GuardianApprovalReviewAction,
    ) {
        self.on_guardian_assessment(GuardianAssessmentEvent {
            id,
            target_item_id: None,
            turn_id,
            status: match review.status {
                crate::session_protocol::GuardianApprovalReviewStatus::InProgress => {
                    GuardianAssessmentStatus::InProgress
                }
                crate::session_protocol::GuardianApprovalReviewStatus::Approved => {
                    GuardianAssessmentStatus::Approved
                }
                crate::session_protocol::GuardianApprovalReviewStatus::Denied => {
                    GuardianAssessmentStatus::Denied
                }
                crate::session_protocol::GuardianApprovalReviewStatus::TimedOut => {
                    GuardianAssessmentStatus::TimedOut
                }
                crate::session_protocol::GuardianApprovalReviewStatus::Aborted => {
                    GuardianAssessmentStatus::Aborted
                }
            },
            risk_level: review
                .risk_level
                .map(crate::session_protocol::guardian_risk_level_from_app_server),
            user_authorization: review
                .user_authorization
                .map(crate::session_protocol::guardian_user_authorization_from_app_server),
            rationale: review.rationale,
            decision_source,
            action: action.into(),
        });
    }

    fn enter_review_mode_with_hint(&mut self, hint: String, from_replay: bool) {
        if self.pre_review_token_info.is_none() {
            self.pre_review_token_info = Some(self.token_info.clone());
        }
        if !from_replay && !self.bottom_pane.is_task_running() {
            self.bottom_pane.set_task_running(/*running*/ true);
        }
        self.is_review_mode = true;
        let banner = format!(">> Code review started: {hint} <<");
        self.add_to_history(history_cell::new_review_status_line(banner));
        self.request_redraw();
    }

    fn exit_review_mode_after_item(&mut self) {
        self.flush_answer_stream_with_separator();
        self.flush_interrupt_queue();
        self.flush_active_cell();
        self.is_review_mode = false;
        self.restore_pre_review_token_info();
        self.add_to_history(history_cell::new_review_status_line(
            "<< Code review finished >>".to_string(),
        ));
        self.request_redraw();
    }

    fn on_committed_user_message(&mut self, items: &[UserInput], from_replay: bool) {
        let display = Self::user_message_display_from_inputs(items);
        if from_replay {
            if !self.is_review_mode {
                self.on_user_message_display(display);
            }
            return;
        }

        let compare_key = Self::pending_steer_compare_key_from_items(items);
        if self
            .pending_steers
            .front()
            .is_some_and(|pending| pending.compare_key == compare_key)
        {
            if let Some(pending) = self.pending_steers.pop_front() {
                self.refresh_pending_input_preview();
                let pending_display =
                    user_message_display_for_history(pending.user_message, &pending.history_record);
                self.on_user_message_display(pending_display);
            } else if self.last_rendered_user_message_display.as_ref() != Some(&display) {
                tracing::warn!(
                    "pending steer matched compare key but queue was empty when rendering committed user message"
                );
                self.on_user_message_display(display);
            }
        } else if !self.is_review_mode
            && self.last_rendered_user_message_display.as_ref() != Some(&display)
        {
            self.on_user_message_display(display);
        }
    }

    fn on_user_message_display(&mut self, display: UserMessageDisplay) {
        self.last_rendered_user_message_display = Some(display.clone());
        if !display.message.trim().is_empty()
            || !display.text_elements.is_empty()
            || !display.local_images.is_empty()
            || !display.remote_image_urls.is_empty()
        {
            self.record_visible_user_turn_for_copy();
            self.add_to_history(history_cell::new_user_prompt(
                display.message,
                display.text_elements,
                display.local_images,
                display.remote_image_urls,
            ));
        }

        // User messages reset separator state so the next agent response doesn't add a stray break.
        self.needs_final_message_separator = false;
    }

    /// Exit the UI immediately without waiting for shutdown.
    ///
    /// Prefer [`Self::request_quit_without_confirmation`] for user-initiated exits;
    /// this is mainly a fallback for shutdown completion or emergency exits.
    fn request_immediate_exit(&self) {
        self.app_event_tx.send(AppEvent::Exit(ExitMode::Immediate));
    }

    /// Request a shutdown-first quit.
    ///
    /// This is used for explicit quit commands (`/quit`, `/exit`, `/logout`) and for
    /// the double-press Ctrl+C/Ctrl+D quit shortcut.
    fn request_quit_without_confirmation(&self) {
        self.app_event_tx
            .send(AppEvent::Exit(ExitMode::ShutdownFirst));
    }

    fn request_redraw(&mut self) {
        self.desired_height_cache.borrow_mut().clear();
        self.rendered_lines_cache.borrow_mut().clear();
        self.frame_requester.schedule_frame();
    }

    fn style_revision(&self) -> u64 {
        self.style_epoch.get()
    }

    fn bump_style_epoch(&mut self) {
        self.style_epoch.set(self.style_epoch.get().wrapping_add(1));
        self.desired_height_cache.borrow_mut().clear();
        self.rendered_lines_cache.borrow_mut().clear();
    }

    fn bump_active_cell_revision(&mut self) {
        self.desired_height_cache.borrow_mut().clear();
        self.rendered_lines_cache.borrow_mut().clear();
        // Wrapping avoids overflow; wraparound would require 2^64 bumps and at
        // worst causes a one-time cache-key collision.
        self.active_cell_revision = self.active_cell_revision.wrapping_add(1);
    }

    fn notify(&mut self, notification: Notification) {
        if !notification.allowed_for(&self.config.tui_notifications.notifications) {
            return;
        }
        if let Some(existing) = self.pending_notification.as_ref()
            && existing.priority() > notification.priority()
        {
            return;
        }
        self.pending_notification = Some(notification);
        self.request_redraw();
    }

    pub(crate) fn maybe_post_pending_notification(&mut self, tui: &mut crate::tui::Tui) {
        if let Some(notif) = self.pending_notification.take() {
            tui.notify(notif.display());
        }
    }

    /// Mark the active cell as failed (✗) and flush it into history.
    fn finalize_active_cell_as_failed(&mut self) {
        if let Some(mut cell) = self.active_cell.take() {
            // Insert finalized cell into history and keep grouping consistent.
            if let Some(exec) = cell.as_any_mut().downcast_mut::<ExecCell>() {
                exec.mark_failed();
            } else if let Some(tool) = cell.as_any_mut().downcast_mut::<McpToolCallCell>() {
                tool.mark_failed();
            }
            self.add_boxed_history(cell);
        }
    }

    // If idle and there are queued inputs, submit exactly one to start the next turn.
    pub(crate) fn maybe_send_next_queued_input(&mut self) -> bool {
        if self.suppress_queue_autosend {
            return false;
        }
        if self.is_user_turn_pending_or_running() {
            return false;
        }
        let mut submitted_follow_up = false;
        while !self.is_user_turn_pending_or_running() {
            let Some((queued_message, history_record)) = self.pop_next_queued_user_message() else {
                break;
            };
            match queued_message.action {
                QueuedInputAction::Plain => {
                    submitted_follow_up = self.submit_user_message_with_history_record(
                        queued_message.into_user_message(),
                        history_record,
                    );
                    break;
                }
                QueuedInputAction::ParseSlash => {
                    let drain = self.submit_queued_slash_prompt(queued_message.into_user_message());
                    if drain == QueueDrain::Stop {
                        submitted_follow_up = self.is_user_turn_pending_or_running();
                        break;
                    }
                }
                QueuedInputAction::RunShell => {
                    let drain = self.submit_queued_shell_prompt(queued_message.into_user_message());
                    if drain == QueueDrain::Stop {
                        submitted_follow_up = self.is_user_turn_pending_or_running();
                        break;
                    }
                }
            }
        }
        // Update the list to reflect the remaining queued messages (if any).
        self.refresh_pending_input_preview();
        submitted_follow_up
    }

    pub(super) fn is_user_turn_pending_or_running(&self) -> bool {
        self.user_turn_pending_start || self.agent_turn_running || self.bottom_pane.is_task_running()
    }

    fn only_user_shell_commands_running(&self) -> bool {
        self.agent_turn_running
            && !self.running_commands.is_empty()
            && self
                .running_commands
                .values()
                .all(|command| command.source == ExecCommandSource::UserShell)
    }

    /// Rebuild and update the bottom-pane pending-input preview.
    fn refresh_pending_input_preview(&mut self) {
        let queued_messages: Vec<String> = self
            .queued_user_messages
            .iter()
            .enumerate()
            .map(|(idx, message)| {
                user_message_preview_text(
                    message,
                    self.queued_user_message_history_records.get(idx),
                )
            })
            .collect();
        let pending_steers: Vec<String> = self
            .pending_steers
            .iter()
            .map(|steer| {
                user_message_preview_text(&steer.user_message, Some(&steer.history_record))
            })
            .collect();
        let rejected_steers: Vec<String> = self
            .rejected_steers_queue
            .iter()
            .enumerate()
            .map(|(idx, message)| {
                user_message_preview_text(message, self.rejected_steer_history_records.get(idx))
            })
            .collect();
        self.bottom_pane.set_pending_input_preview(
            queued_messages,
            pending_steers,
            rejected_steers,
        );
    }

    pub(crate) fn set_pending_thread_approvals(&mut self, threads: Vec<String>) {
        self.bottom_pane.set_pending_thread_approvals(threads);
    }

    pub(crate) fn clear_thread_rename_block(&mut self) {
        self.thread_rename_block_message = None;
    }
}
