// O5/O6 balanced ChatWidget impl group: source split_008_on_exec_command_output_delta.rs
impl ChatWidget {
    fn on_exec_command_output_delta(&mut self, call_id: &str, delta: &str) {
        self.track_unified_exec_output_chunk(call_id, delta.as_bytes());
        if !self.bottom_pane.is_task_running() {
            return;
        }

        let Some(cell) = self
            .active_cell
            .as_mut()
            .and_then(|c| c.as_any_mut().downcast_mut::<ExecCell>())
        else {
            return;
        };

        if cell.append_output(call_id, delta) {
            self.bump_active_cell_revision();
            self.request_redraw();
        }
    }

    fn on_terminal_interaction(&mut self, process_id: String, stdin: String) {
        if !self.bottom_pane.is_task_running() {
            return;
        }
        self.flush_answer_stream_with_separator();
        let command_display = self
            .unified_exec_processes
            .iter()
            .find(|process| process.key == process_id)
            .map(|process| process.command_display.clone());
        if stdin.is_empty() {
            // Empty stdin means we are polling for background output.
            // Surface this in the status indicator (single "waiting" surface) instead of
            // the transcript. Keep the header short so the interrupt hint remains visible.
            self.bottom_pane.ensure_status_indicator();
            self.bottom_pane
                .set_interrupt_hint_visible(/*visible*/ true);
            self.terminal_title_status_kind = TerminalTitleStatusKind::WaitingForBackgroundTerminal;
            self.set_status(
                "Waiting for background terminal".to_string(),
                command_display.clone(),
                StatusDetailsCapitalization::Preserve,
                /*details_max_lines*/ 1,
            );
            match &mut self.unified_exec_wait_streak {
                Some(wait) if wait.process_id == process_id => {
                    wait.update_command_display(command_display);
                }
                Some(_) => {
                    self.flush_unified_exec_wait_streak();
                    self.unified_exec_wait_streak =
                        Some(UnifiedExecWaitStreak::new(process_id, command_display));
                }
                None => {
                    self.unified_exec_wait_streak =
                        Some(UnifiedExecWaitStreak::new(process_id, command_display));
                }
            }
            self.request_redraw();
        } else {
            if self
                .unified_exec_wait_streak
                .as_ref()
                .is_some_and(|wait| wait.process_id == process_id)
            {
                self.flush_unified_exec_wait_streak();
            }
            self.add_to_history(history_cell::new_unified_exec_interaction(
                command_display,
                stdin,
            ));
        }
    }

    fn on_patch_apply_begin(&mut self, changes: HashMap<PathBuf, FileChange>) {
        self.add_to_history(history_cell::new_patch_event(changes, &self.config.cwd));
    }

    fn on_view_image_tool_call(&mut self, path: AbsolutePathBuf) {
        self.flush_answer_stream_with_separator();
        self.add_to_history(history_cell::new_view_image_tool_call(
            path,
            &self.config.cwd,
        ));
        self.request_redraw();
    }

    fn on_image_generation_begin(&mut self) {
        self.flush_answer_stream_with_separator();
    }

    fn on_image_generation_end(
        &mut self,
        call_id: String,
        revised_prompt: Option<String>,
        saved_path: Option<AbsolutePathBuf>,
    ) {
        self.flush_answer_stream_with_separator();
        self.add_to_history(history_cell::new_image_generation_call(
            call_id,
            revised_prompt,
            saved_path,
        ));
        self.request_redraw();
    }

    fn on_file_change_completed(&mut self, item: ThreadItem) {
        let item2 = item.clone();
        self.defer_or_handle(
            |q| q.push_item_completed(item),
            |s| s.handle_file_change_completed_now(item2),
        );
    }

    fn on_command_execution_completed(&mut self, item: ThreadItem) {
        let ThreadItem::CommandExecution {
            id,
            process_id,
            source,
            ..
        } = &item
        else {
            return;
        };
        if is_unified_exec_source(*source) {
            if let Some(process_id) = process_id.as_deref()
                && self
                    .unified_exec_wait_streak
                    .as_ref()
                    .is_some_and(|wait| wait.process_id == process_id)
            {
                self.flush_unified_exec_wait_streak();
            }
            self.track_unified_exec_process_end(id, process_id.as_deref());
            if !self.bottom_pane.is_task_running() {
                return;
            }
        }
        let item2 = item.clone();
        self.defer_or_handle(
            |q| q.push_item_completed(item),
            |s| s.handle_command_execution_completed_now(item2),
        );
    }

    fn track_unified_exec_process_begin(
        &mut self,
        call_id: &str,
        process_id: Option<&str>,
        command: &str,
    ) {
        let key = process_id.unwrap_or(call_id).to_string();
        let command = split_command_string(command);
        let command_display = strip_bash_lc_and_escape(&command);
        if let Some(existing) = self
            .unified_exec_processes
            .iter_mut()
            .find(|process| process.key == key)
        {
            existing.call_id = call_id.to_string();
            existing.command_display = command_display;
            existing.recent_chunks.clear();
        } else {
            self.unified_exec_processes.push(UnifiedExecProcessSummary {
                key,
                call_id: call_id.to_string(),
                command_display,
                recent_chunks: Vec::new(),
            });
        }
        self.sync_unified_exec_footer();
    }

    fn track_unified_exec_process_end(&mut self, call_id: &str, process_id: Option<&str>) {
        let key = process_id.unwrap_or(call_id);
        let before = self.unified_exec_processes.len();
        self.unified_exec_processes
            .retain(|process| process.key != key);
        if self.unified_exec_processes.len() != before {
            self.sync_unified_exec_footer();
        }
    }

    fn sync_unified_exec_footer(&mut self) {
        let processes = self
            .unified_exec_processes
            .iter()
            .map(|process| process.command_display.clone())
            .collect();
        self.bottom_pane.set_unified_exec_processes(processes);
    }

    /// Record recent stdout/stderr lines for the unified exec footer.
    fn track_unified_exec_output_chunk(&mut self, call_id: &str, chunk: &[u8]) {
        let Some(process) = self
            .unified_exec_processes
            .iter_mut()
            .find(|process| process.call_id == call_id)
        else {
            return;
        };

        let text = String::from_utf8_lossy(chunk);
        for line in text
            .lines()
            .map(str::trim_end)
            .filter(|line| !line.is_empty())
        {
            process.recent_chunks.push(line.to_string());
        }

        const MAX_RECENT_CHUNKS: usize = 3;
        if process.recent_chunks.len() > MAX_RECENT_CHUNKS {
            let drop_count = process.recent_chunks.len() - MAX_RECENT_CHUNKS;
            process.recent_chunks.drain(0..drop_count);
        }
    }

    fn on_mcp_tool_call_started(&mut self, item: ThreadItem) {
        let item2 = item.clone();
        self.defer_or_handle(
            |q| q.push_item_started(item),
            |s| s.handle_mcp_tool_call_started_now(item2),
        );
    }

    fn on_mcp_tool_call_completed(&mut self, item: ThreadItem) {
        let item2 = item.clone();
        self.defer_or_handle(
            |q| q.push_item_completed(item),
            |s| s.handle_mcp_tool_call_completed_now(item2),
        );
    }

    fn on_web_search_begin(&mut self, call_id: String) {
        self.flush_answer_stream_with_separator();
        self.flush_active_cell();
        self.active_cell = Some(Box::new(history_cell::new_active_web_search_call(
            call_id,
            String::new(),
            self.config.animations,
        )));
        self.bump_active_cell_revision();
        self.request_redraw();
    }

    fn on_web_search_end(
        &mut self,
        call_id: String,
        query: String,
        action: crate::session_protocol::WebSearchAction,
    ) {
        self.flush_answer_stream_with_separator();
        let mut handled = false;
        if let Some(cell) = self
            .active_cell
            .as_mut()
            .and_then(|cell| cell.as_any_mut().downcast_mut::<WebSearchCell>())
            && cell.call_id() == call_id
        {
            cell.update(action.clone(), query.clone());
            cell.complete();
            self.bump_active_cell_revision();
            self.flush_active_cell();
            handled = true;
        }

        if !handled {
            self.add_to_history(history_cell::new_web_search_call(call_id, query, action));
        }
        self.had_work_activity = true;
    }

    fn on_collab_event(&mut self, cell: PlainHistoryCell) {
        self.flush_answer_stream_with_separator();
        self.add_to_history(cell);
        self.request_redraw();
    }

    fn on_collab_agent_tool_call(&mut self, item: ThreadItem) {
        let ThreadItem::CollabAgentToolCall {
            id, tool, status, ..
        } = &item
        else {
            return;
        };
        if matches!(tool, CollabAgentTool::SpawnAgent)
            && let Some(spawn_request) = multi_agents::spawn_request_summary(&item)
        {
            self.pending_collab_spawn_requests
                .insert(id.clone(), spawn_request);
        }

        let cached_spawn_request = if matches!(tool, CollabAgentTool::SpawnAgent)
            && !matches!(status, CollabAgentToolCallStatus::InProgress)
        {
            self.pending_collab_spawn_requests.remove(id)
        } else {
            None
        };

        if let Some(cell) = multi_agents::tool_call_history_cell(
            &item,
            cached_spawn_request.as_ref(),
            |thread_id| self.collab_agent_metadata(thread_id),
        ) {
            self.on_collab_event(cell);
        }
    }

    pub(crate) fn handle_history_entry_response(&mut self, event: HistoryLookupResponse) {
        let HistoryLookupResponse {
            offset,
            log_id,
            entry,
        } = event;
        self.bottom_pane
            .on_history_entry_response(log_id, offset, entry.map(|e| e.text));
    }

    fn on_shutdown_complete(&mut self) {
        self.request_immediate_exit();
    }

    fn on_turn_diff(&mut self, unified_diff: String) {
        debug!("TurnDiffEvent: {unified_diff}");
        self.refresh_status_line();
    }

    fn interrupted_turn_message(&self, reason: TurnAbortReason) -> String {
        if reason == TurnAbortReason::BudgetLimited {
            return "Goal budget reached - the turn was stopped.".to_string();
        }

        "Conversation interrupted - tell the model what to do differently. Something went wrong? Hit `/feedback` to report the issue.".to_string()
    }

    fn on_deprecation_notice(&mut self, summary: String, details: Option<String>) {
        self.add_to_history(history_cell::new_deprecation_notice(summary, details));
        self.request_redraw();
    }

    fn on_hook_started(&mut self, run: crate::session_protocol::HookRunSummary) {
        self.flush_answer_stream_with_separator();
        self.flush_completed_hook_output();
        match self.active_hook_cell.as_mut() {
            Some(cell) => {
                cell.start_run(run);
                self.bump_active_cell_revision();
            }
            None => {
                self.active_hook_cell = Some(history_cell::new_active_hook_cell(
                    run,
                    self.config.animations,
                ));
                self.bump_active_cell_revision();
            }
        }
        self.request_redraw();
    }

    fn on_hook_completed(&mut self, completed: crate::session_protocol::HookRunSummary) {
        let completed_existing_run = self
            .active_hook_cell
            .as_mut()
            .map(|cell| cell.complete_run(completed.clone()))
            .unwrap_or(false);
        if completed_existing_run {
            self.bump_active_cell_revision();
        } else {
            match self.active_hook_cell.as_mut() {
                Some(cell) => {
                    cell.add_completed_run(completed);
                    self.bump_active_cell_revision();
                }
                None => {
                    let cell =
                        history_cell::new_completed_hook_cell(completed, self.config.animations);
                    if !cell.is_empty() {
                        self.active_hook_cell = Some(cell);
                        self.bump_active_cell_revision();
                    }
                }
            }
        }
        self.flush_completed_hook_output();
        self.finish_active_hook_cell_if_idle();
        self.request_redraw();
    }

    fn flush_completed_hook_output(&mut self) {
        let Some(completed_cell) = self
            .active_hook_cell
            .as_mut()
            .and_then(HookCell::take_completed_persistent_runs)
        else {
            return;
        };
        let active_cell_is_empty = self
            .active_hook_cell
            .as_ref()
            .is_some_and(HookCell::is_empty);
        if active_cell_is_empty {
            self.active_hook_cell = None;
        }
        self.bump_active_cell_revision();
        self.needs_final_message_separator = true;
        self.app_event_tx
            .send(AppEvent::InsertHistoryCell(Box::new(completed_cell)));
    }

    fn finish_active_hook_cell_if_idle(&mut self) {
        let Some(cell) = self.active_hook_cell.as_ref() else {
            return;
        };
        if cell.is_empty() {
            self.active_hook_cell = None;
            self.bump_active_cell_revision();
            return;
        }
        if cell.should_flush()
            && let Some(cell) = self.active_hook_cell.take()
        {
            self.bump_active_cell_revision();
            self.needs_final_message_separator = true;
            self.app_event_tx
                .send(AppEvent::InsertHistoryCell(Box::new(cell)));
        }
    }
}
