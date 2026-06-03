// O5/O6 balanced ChatWidget impl group: source split_009_update_due_hook_visibility.rs
impl ChatWidget {
    fn update_due_hook_visibility(&mut self) {
        let Some(cell) = self.active_hook_cell.as_mut() else {
            return;
        };
        let now = Instant::now();
        if cell.advance_time(now) {
            self.bump_active_cell_revision();
        }
        self.finish_active_hook_cell_if_idle();
    }

    fn schedule_hook_timer_if_needed(&self) {
        if self.config.animations
            && self
                .active_hook_cell
                .as_ref()
                .is_some_and(HookCell::has_visible_running_run)
        {
            self.frame_requester
                .schedule_frame_in(Duration::from_millis(50));
        }

        let Some(deadline) = self
            .active_hook_cell
            .as_ref()
            .and_then(HookCell::next_timer_deadline)
        else {
            return;
        };
        let delay = deadline.saturating_duration_since(Instant::now());
        self.frame_requester.schedule_frame_in(delay);
    }

    fn on_stream_error(&mut self, message: String, additional_details: Option<String>) {
        if self.retry_status_header.is_none() {
            self.retry_status_header = Some(self.current_status.header.clone());
        }
        self.bottom_pane.ensure_status_indicator();
        self.terminal_title_status_kind = TerminalTitleStatusKind::Thinking;
        self.set_status(
            message,
            additional_details,
            StatusDetailsCapitalization::CapitalizeFirst,
            STATUS_DETAILS_DEFAULT_MAX_LINES,
        );
    }

    pub(crate) fn pre_draw_tick(&mut self) {
        self.update_due_hook_visibility();
        self.schedule_hook_timer_if_needed();
        self.bottom_pane.pre_draw_tick();
        self.refresh_plan_mode_nudge();
        self.refresh_goal_status_indicator_for_time_tick();
        if self.terminal_title_shows_action_required() != self.last_terminal_title_requires_action {
            self.refresh_terminal_title();
        }
        if self.should_animate_terminal_title_spinner()
            || self.should_animate_terminal_title_action_required()
        {
            self.refresh_terminal_title();
        }
    }

    /// Handle completion of an `AgentMessage` turn item.
    ///
    /// Commentary completion sets a deferred restore flag so the status row
    /// returns once stream queues are idle. Final-answer completion (or absent
    /// phase for legacy models) clears the flag to preserve historical behavior.
    fn on_agent_message_item_completed(&mut self, item: AgentMessageItem) {
        let mut message = String::new();
        for content in &item.content {
            match content {
                AgentMessageContent::Text { text } => message.push_str(text),
            }
        }
        self.finalize_completed_assistant_message(
            (!message.is_empty()).then_some(message.as_str()),
        );
        if matches!(item.phase, Some(MessagePhase::FinalAnswer) | None) && !message.is_empty() {
            self.record_agent_markdown(&message);
        }
        self.pending_status_indicator_restore = match item.phase {
            // Models that don't support preambles only output AgentMessageItems on turn completion.
            Some(MessagePhase::FinalAnswer) | None => !self.pending_steers.is_empty(),
            Some(MessagePhase::Commentary) => true,
        };
        self.maybe_restore_status_indicator_after_stream_idle();
    }

    /// Periodic tick for stream commits. In smooth mode this preserves one-line pacing, while
    /// catch-up mode drains larger batches to reduce queue lag.
    pub(crate) fn on_commit_tick(&mut self) {
        self.run_commit_tick();
    }

    /// Runs a regular periodic commit tick.
    fn run_commit_tick(&mut self) {
        self.run_commit_tick_with_scope(CommitTickScope::AnyMode);
    }

    /// Runs an opportunistic commit tick only if catch-up mode is active.
    fn run_catch_up_commit_tick(&mut self) {
        self.run_commit_tick_with_scope(CommitTickScope::CatchUpOnly);
    }

    /// Runs a commit tick for the current stream queue snapshot.
    ///
    /// `scope` controls whether this call may commit in smooth mode or only when catch-up
    /// is currently active. While lines are actively streaming we hide the status row to avoid
    /// duplicate "in progress" affordances. Restoration is gated separately so we only re-show
    /// the row after commentary completion once stream queues are idle.
    fn run_commit_tick_with_scope(&mut self, scope: CommitTickScope) {
        let now = Instant::now();
        let outcome = run_commit_tick(
            &mut self.adaptive_chunking,
            self.stream_controller.as_mut(),
            self.plan_stream_controller.as_mut(),
            scope,
            now,
        );
        for cell in outcome.cells {
            self.bottom_pane.hide_status_indicator();
            self.add_boxed_history(cell);
        }

        if outcome.has_controller && outcome.all_idle {
            self.maybe_restore_status_indicator_after_stream_idle();
            self.app_event_tx.send(AppEvent::StopCommitAnimation);
        }

        if self.agent_turn_running {
            self.refresh_runtime_metrics();
        }
    }

    fn flush_interrupt_queue(&mut self) {
        let mut mgr = std::mem::take(&mut self.interrupts);
        mgr.flush_all(self);
        self.interrupts = mgr;
    }

    #[inline]
    fn defer_or_handle(
        &mut self,
        push: impl FnOnce(&mut InterruptManager),
        handle: impl FnOnce(&mut Self),
    ) {
        // Preserve deterministic FIFO across queued interrupts: once anything
        // is queued due to an active write cycle, continue queueing until the
        // queue is flushed to avoid reordering (e.g., ExecEnd before ExecBegin).
        if self.stream_controller.is_some() || !self.interrupts.is_empty() {
            push(&mut self.interrupts);
        } else {
            handle(self);
        }
    }

    fn handle_stream_finished(&mut self) {
        if self.task_complete_pending {
            self.bottom_pane.hide_status_indicator();
            self.task_complete_pending = false;
        }
        // A completed stream indicates non-exec content was just inserted.
        self.flush_interrupt_queue();
    }

    #[inline]
    fn handle_streaming_delta(&mut self, delta: String) {
        // Project a single AssistantDelta marker per turn into the runtime
        // surface (the bridge dedupes per turn so we do not spam history).
        let runtime_assistant_evs = self
            .runtime_bridge
            .as_mut()
            .map(|b| b.project_assistant_delta(&delta))
            .unwrap_or_default();
        self.project_runtime_events(runtime_assistant_evs);
        // Before streaming agent content, flush any active exec cell group.
        self.flush_unified_exec_wait_streak();
        self.flush_active_cell();

        if self.stream_controller.is_none() {
            // If the previous turn inserted non-stream history (exec output, patch status, MCP
            // calls), render a separator before starting the next streamed assistant message.
            if self.needs_final_message_separator && self.had_work_activity {
                self.add_to_history(history_cell::FinalMessageSeparator::new(
                    /*elapsed_seconds*/ None, /*runtime_metrics*/ None,
                ));
                self.needs_final_message_separator = false;
            } else if self.needs_final_message_separator {
                // Reset the flag even if we don't show separator (no work was done)
                self.needs_final_message_separator = false;
            }
            self.stream_controller = Some(StreamController::new(
                self.current_stream_width(/*reserved_cols*/ 2),
                &self.config.cwd,
            ));
        }
        if let Some(controller) = self.stream_controller.as_mut()
            && controller.push(&delta)
        {
            self.app_event_tx.send(AppEvent::StartCommitAnimation);
            self.run_catch_up_commit_tick();
        }
        self.request_redraw();
    }

    /// Finalizes an exec call while preserving the active exec cell grouping contract.
    ///
    /// Exec begin/end events usually pair through `running_commands`, but unified exec can emit an
    /// end event for a call that was never materialized as the current active `ExecCell` (for
    /// example, when another exploring group is still active). In that case we render the end as a
    /// standalone history entry instead of replacing or flushing the unrelated active exploring
    /// cell. If this method treated every unknown end as "complete the active cell", the UI could
    /// merge unrelated commands and hide still-running exploring work.
    pub(crate) fn handle_command_execution_completed_now(&mut self, item: ThreadItem) {
        enum ExecEndTarget {
            // Normal case: the active exec cell already tracks this call id.
            ActiveTracked,
            // We have an active exec group, but it does not contain this call id. Render the end
            // as a standalone finalized history cell so the active group remains intact.
            OrphanHistoryWhileActiveExec,
            // No active exec cell can safely own this end; build a new cell from the end payload.
            NewCell,
        }

        let ThreadItem::CommandExecution {
            id,
            command,
            process_id: _,
            source,
            command_actions,
            aggregated_output,
            exit_code,
            duration_ms,
            ..
        } = item
        else {
            return;
        };
        let event_command = split_command_string(&command);
        let event_parsed = command_actions
            .into_iter()
            .map(crate::session_protocol::CommandAction::into_core)
            .collect();
        let duration = Duration::from_millis(duration_ms.unwrap_or_default().max(0) as u64);
        let exit_code = exit_code.unwrap_or_default();
        let aggregated_output = aggregated_output.unwrap_or_default();

        let running = self.running_commands.remove(&id);
        if self.suppressed_exec_calls.remove(&id) {
            return;
        }
        let (command, parsed, source) = match running {
            Some(rc) => (rc.command, rc.parsed_cmd, rc.source),
            None => (event_command, event_parsed, source),
        };
        let runtime_exec_finished_evs = self
            .runtime_bridge
            .as_mut()
            .map(|b| b.project_exec_command_finished(&id, &command, exit_code))
            .unwrap_or_default();
        self.project_runtime_events(runtime_exec_finished_evs);
        let parsed = self.annotate_skill_reads_in_parsed_cmd(parsed);
        let is_unified_exec_interaction =
            matches!(source, ExecCommandSource::UnifiedExecInteraction);
        let is_user_shell = source == ExecCommandSource::UserShell;
        let end_target = match self.active_cell.as_ref() {
            Some(cell) => match cell.as_any().downcast_ref::<ExecCell>() {
                Some(exec_cell) if exec_cell.iter_calls().any(|call| call.call_id == id) => {
                    ExecEndTarget::ActiveTracked
                }
                Some(exec_cell) if exec_cell.is_active() => {
                    ExecEndTarget::OrphanHistoryWhileActiveExec
                }
                Some(_) | None => ExecEndTarget::NewCell,
            },
            None => ExecEndTarget::NewCell,
        };

        // Unified exec interaction rows intentionally hide command output text in the exec cell and
        // instead render the interaction-specific content elsewhere in the UI.
        let output = if is_unified_exec_interaction {
            CommandOutput {
                exit_code,
                formatted_output: String::new(),
                aggregated_output: String::new(),
            }
        } else {
            CommandOutput {
                exit_code,
                formatted_output: aggregated_output.clone(),
                aggregated_output,
            }
        };

        match end_target {
            ExecEndTarget::ActiveTracked => {
                if let Some(cell) = self
                    .active_cell
                    .as_mut()
                    .and_then(|c| c.as_any_mut().downcast_mut::<ExecCell>())
                {
                    let completed = cell.complete_call(&id, output, duration);
                    debug_assert!(completed, "active exec cell should contain {id}");
                    if cell.should_flush() {
                        self.flush_active_cell();
                    } else {
                        self.bump_active_cell_revision();
                        self.request_redraw();
                    }
                }
            }
            ExecEndTarget::OrphanHistoryWhileActiveExec => {
                let mut orphan = new_active_exec_command(
                    id.clone(),
                    command,
                    parsed,
                    source,
                    /*interaction_input*/ None,
                    self.config.animations,
                );
                let completed = orphan.complete_call(&id, output, duration);
                debug_assert!(completed, "new orphan exec cell should contain {id}");
                self.needs_final_message_separator = true;
                self.app_event_tx
                    .send(AppEvent::InsertHistoryCell(Box::new(orphan)));
                self.request_redraw();
            }
            ExecEndTarget::NewCell => {
                self.flush_active_cell();
                let mut cell = new_active_exec_command(
                    id.clone(),
                    command,
                    parsed,
                    source,
                    /*interaction_input*/ None,
                    self.config.animations,
                );
                let completed = cell.complete_call(&id, output, duration);
                debug_assert!(completed, "new exec cell should contain {id}");
                if cell.should_flush() {
                    self.add_to_history(cell);
                } else {
                    self.active_cell = Some(Box::new(cell));
                    self.bump_active_cell_revision();
                    self.request_redraw();
                }
            }
        }
        // Mark that actual work was done (command executed)
        self.had_work_activity = true;
        if is_user_shell {
            self.maybe_send_next_queued_input();
        }
    }

    pub(crate) fn handle_file_change_completed_now(&mut self, item: ThreadItem) {
        let ThreadItem::FileChange { status, .. } = item else {
            return;
        };
        // If the patch was successful, just let the "Edited" block stand.
        // Otherwise, add a failure block.
        if matches!(status, crate::session_protocol::PatchApplyStatus::Failed) {
            self.add_to_history(history_cell::new_patch_apply_failure(String::new()));
        }
        // Mark that actual work was done (patch applied)
        self.had_work_activity = true;
    }

    pub(crate) fn handle_exec_approval_now(&mut self, ev: ExecApprovalRequestEvent) {
        self.flush_answer_stream_with_separator();
        let command = shlex::try_join(ev.command.iter().map(String::as_str))
            .unwrap_or_else(|_| ev.command.join(" "));
        self.notify(Notification::ExecApprovalRequested { command });

        // Mirror the approval request into the Local Runtime Contract so
        // the operator sees a `vac runtime: approval requested ...` row
        // alongside the existing approval overlay. The overlay still owns
        // the actual decision UI; this is purely additive evidence.
        let runtime_events = self.runtime_bridge.as_mut().map(|bridge| {
            bridge.project_exec_approval_request(
                &ev.effective_approval_id(),
                &ev.command,
                ev.reason.as_deref(),
            )
        });
        if let Some(events) = runtime_events {
            self.project_runtime_events(events);
        }

        let available_decisions = ev.effective_available_decisions();
        let request = ApprovalRequest::Exec {
            thread_id: self.thread_id.unwrap_or_default(),
            thread_label: None,
            id: ev.effective_approval_id(),
            command: ev.command,
            reason: ev.reason,
            available_decisions,
            network_approval_context: ev.network_approval_context,
            additional_permissions: ev.additional_permissions,
        };
        self.bottom_pane
            .push_approval_request(request, &self.config.features);
        self.request_redraw();
    }

    pub(crate) fn handle_apply_patch_approval_now(&mut self, ev: ApplyPatchApprovalRequestEvent) {
        self.flush_answer_stream_with_separator();

        // Mirror the patch approval request into the Local Runtime Contract.
        let runtime_events = self.runtime_bridge.as_mut().map(|bridge| {
            let files: Vec<std::path::PathBuf> = ev.changes.keys().cloned().collect();
            bridge.project_apply_patch_approval_request(&ev.call_id, files, ev.reason.as_deref())
        });
        if let Some(events) = runtime_events {
            self.project_runtime_events(events);
        }

        let request = ApprovalRequest::ApplyPatch {
            thread_id: self.thread_id.unwrap_or_default(),
            thread_label: None,
            id: ev.call_id,
            reason: ev.reason,
            changes: ev.changes.clone(),
            cwd: self.config.cwd.clone(),
        };
        self.bottom_pane
            .push_approval_request(request, &self.config.features);
        self.request_redraw();
        self.notify(Notification::EditApprovalRequested {
            cwd: self.config.cwd.to_path_buf(),
            changes: ev.changes.keys().cloned().collect(),
        });
    }
}
