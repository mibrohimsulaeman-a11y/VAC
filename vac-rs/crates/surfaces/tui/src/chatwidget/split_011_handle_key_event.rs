    pub(crate) fn handle_key_event(&mut self, key_event: KeyEvent) {
        if self.bottom_pane.has_active_view()
            && !matches!(
                key_event,
                KeyEvent {
                    code: KeyCode::Char(c),
                    modifiers,
                    kind: KeyEventKind::Press,
                    ..
                } if modifiers.contains(KeyModifiers::CONTROL) && c.eq_ignore_ascii_case(&'c')
            )
            && !key_hint::ctrl(KeyCode::Char('r')).is_press(key_event)
            && !key_hint::ctrl(KeyCode::Char('u')).is_press(key_event)
        {
            self.bottom_pane.handle_key_event(key_event);
            if self.bottom_pane.no_modal_or_popup_active() {
                self.maybe_send_next_queued_input();
            }
            return;
        }

        if self.handle_reasoning_shortcut(key_event) {
            self.bottom_pane.clear_quit_shortcut_hint();
            self.quit_shortcut_expires_at = None;
            self.quit_shortcut_key = None;
            return;
        }

        if key_event.kind == KeyEventKind::Press
            && self.copy_last_response_binding.is_pressed(key_event)
        {
            self.bottom_pane.clear_quit_shortcut_hint();
            self.quit_shortcut_expires_at = None;
            self.quit_shortcut_key = None;
            self.copy_last_agent_markdown();
            return;
        }

        match key_event {
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers,
                kind: KeyEventKind::Press,
                ..
            } if modifiers.contains(KeyModifiers::CONTROL) && c.eq_ignore_ascii_case(&'c') => {
                self.on_ctrl_c();
                return;
            }
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers,
                kind: KeyEventKind::Press,
                ..
            } if modifiers.contains(KeyModifiers::CONTROL) && c.eq_ignore_ascii_case(&'d') => {
                if self.on_ctrl_d() {
                    return;
                }
                self.bottom_pane.clear_quit_shortcut_hint();
                self.quit_shortcut_expires_at = None;
                self.quit_shortcut_key = None;
            }
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers,
                kind: KeyEventKind::Press,
                ..
            } if modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                && c.eq_ignore_ascii_case(&'v') =>
            {
                match paste_image_to_temp_png() {
                    Ok((path, info)) => {
                        tracing::debug!(
                            "pasted image size={}x{} format={}",
                            info.width,
                            info.height,
                            info.encoded_format.label()
                        );
                        self.attach_image(path);
                    }
                    Err(err) => {
                        tracing::warn!("failed to paste image: {err}");
                        self.add_to_history(history_cell::new_error_event(format!(
                            "Failed to paste image: {err}",
                        )));
                    }
                }
                return;
            }
            other if other.kind == KeyEventKind::Press => {
                self.bottom_pane.clear_quit_shortcut_hint();
                self.quit_shortcut_expires_at = None;
                self.quit_shortcut_key = None;
            }
            _ => {}
        }

        if key_event.kind == KeyEventKind::Press
            && self.chat_keymap.edit_queued_message.is_pressed(key_event)
            && self.has_queued_follow_up_messages()
            && self.bottom_pane.no_modal_or_popup_active()
        {
            if let Some(user_message) = self.pop_latest_queued_user_message() {
                self.restore_user_message_to_composer(user_message);
                self.refresh_pending_input_preview();
                self.request_redraw();
            }
            return;
        }

        if matches!(key_event.code, KeyCode::Esc)
            && matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat)
            && !self.pending_steers.is_empty()
            && self.bottom_pane.is_task_running()
            && self.bottom_pane.no_modal_or_popup_active()
            && !self.should_handle_vim_insert_escape(key_event)
        {
            self.submit_pending_steers_after_interrupt = true;
            if !self.submit_op(AppCommand::interrupt()) {
                self.submit_pending_steers_after_interrupt = false;
            }
            return;
        }

        if matches!(key_event.code, KeyCode::Esc)
            && key_event.kind == KeyEventKind::Press
            && self.should_show_plan_mode_nudge()
        {
            self.dismiss_plan_mode_nudge();
            return;
        }

        if self.handle_plugins_popup_key_event(key_event) {
            return;
        }

        match key_event {
            KeyEvent {
                code: KeyCode::BackTab,
                kind: KeyEventKind::Press,
                ..
            } if self.collaboration_modes_enabled()
                && !self.bottom_pane.is_task_running()
                && self.bottom_pane.no_modal_or_popup_active() =>
            {
                self.cycle_collaboration_mode();
                self.refresh_plan_mode_nudge();
            }
            _ => {
                let had_modal_or_popup = !self.bottom_pane.no_modal_or_popup_active();
                match self.bottom_pane.handle_key_event(key_event) {
                    InputResult::Submitted {
                        text,
                        text_elements,
                    } => {
                        let local_images = self
                            .bottom_pane
                            .take_recent_submission_images_with_placeholders();
                        let remote_image_urls = self.take_remote_image_urls();
                        let user_message = UserMessage {
                            text,
                            local_images,
                            remote_image_urls,
                            text_elements,
                            mention_bindings: self
                                .bottom_pane
                                .take_recent_submission_mention_bindings(),
                        };
                        if user_message.text.is_empty()
                            && user_message.local_images.is_empty()
                            && user_message.remote_image_urls.is_empty()
                        {
                            return;
                        }
                        let should_submit_now =
                            self.is_session_configured() && !self.is_plan_streaming_in_tui();
                        if should_submit_now {
                            if self.only_user_shell_commands_running()
                                && !user_message.text.starts_with('!')
                            {
                                self.queue_user_message(user_message);
                                return;
                            }
                            // Submitted is emitted when user submits.
                            // Reset any reasoning header only when we are actually submitting a turn.
                            self.reasoning_buffer.clear();
                            self.full_reasoning_buffer.clear();
                            self.set_status_header(String::from("Working"));
                            self.submit_user_message(user_message);
                        } else {
                            self.queue_user_message(user_message);
                        }
                    }
                    InputResult::Queued {
                        text,
                        text_elements,
                        action,
                    } => {
                        let local_images = self
                            .bottom_pane
                            .take_recent_submission_images_with_placeholders();
                        let remote_image_urls = self.take_remote_image_urls();
                        let user_message = UserMessage {
                            text,
                            local_images,
                            remote_image_urls,
                            text_elements,
                            mention_bindings: self
                                .bottom_pane
                                .take_recent_submission_mention_bindings(),
                        };
                        self.queue_user_message_with_options(user_message, action);
                    }
                    InputResult::Command(cmd) => {
                        self.handle_slash_command_dispatch(cmd);
                    }
                    InputResult::CommandWithArgs(cmd, args, text_elements) => {
                        self.handle_slash_command_with_args_dispatch(cmd, args, text_elements);
                    }
                    InputResult::None => {}
                }
                if had_modal_or_popup && self.bottom_pane.no_modal_or_popup_active() {
                    self.maybe_send_next_queued_input();
                }
                self.refresh_plan_mode_nudge();
            }
        }
    }

    /// Attach a local image to the composer when the active model supports image inputs.
    ///
    /// When the model does not advertise image support, we keep the draft unchanged and surface a
    /// warning event so users can switch models or remove attachments.
    pub(crate) fn attach_image(&mut self, path: PathBuf) {
        if !self.current_model_supports_images() {
            self.add_to_history(history_cell::new_warning_event(
                self.image_inputs_not_supported_message(),
            ));
            self.request_redraw();
            return;
        }
        tracing::info!("attach_image path={path:?}");
        self.bottom_pane.attach_image(path);
        self.request_redraw();
    }

    pub(crate) fn composer_text_with_pending(&self) -> String {
        self.bottom_pane.composer_text_with_pending()
    }

    pub(crate) fn apply_external_edit(&mut self, text: String) {
        self.bottom_pane.apply_external_edit(text);
        self.refresh_plan_mode_nudge();
        self.request_redraw();
    }

    pub(crate) fn external_editor_state(&self) -> ExternalEditorState {
        self.external_editor_state
    }

    pub(crate) fn set_external_editor_state(&mut self, state: ExternalEditorState) {
        self.external_editor_state = state;
    }

    pub(crate) fn set_footer_hint_override(&mut self, items: Option<Vec<(String, String)>>) {
        self.bottom_pane.set_footer_hint_override(items);
    }

    pub(crate) fn show_selection_view(&mut self, params: SelectionViewParams) {
        self.bottom_pane.show_selection_view(params);
        self.refresh_plan_mode_nudge();
        self.request_redraw();
    }

    pub(crate) fn no_modal_or_popup_active(&self) -> bool {
        self.bottom_pane.no_modal_or_popup_active()
    }

    pub(crate) fn can_launch_external_editor(&self) -> bool {
        self.bottom_pane.can_launch_external_editor()
    }

    pub(crate) fn can_run_ctrl_l_clear_now(&mut self) -> bool {
        // Ctrl+L is not a slash command, but it follows /clear's current rule:
        // block while a task is running.
        if !self.bottom_pane.is_task_running() {
            return true;
        }

        let message = "Ctrl+L is disabled while a task is in progress.".to_string();
        self.add_to_history(history_cell::new_error_event(message));
        self.request_redraw();
        false
    }

    /// Copy the last agent response (raw markdown) to the system clipboard.
    pub(crate) fn copy_last_agent_markdown(&mut self) {
        self.copy_last_agent_markdown_with(crate::clipboard_copy::copy_to_clipboard);
    }

    pub(crate) fn truncate_agent_copy_history_to_user_turn_count(
        &mut self,
        user_turn_count: usize,
    ) {
        self.visible_user_turn_count = user_turn_count;
        let had_copy_history = !self.agent_turn_markdowns.is_empty();
        self.agent_turn_markdowns
            .retain(|entry| entry.user_turn_count <= user_turn_count);
        self.last_agent_markdown = self
            .agent_turn_markdowns
            .last()
            .map(|entry| entry.markdown.clone());
        self.copy_history_evicted_by_rollback =
            had_copy_history && self.last_agent_markdown.is_none();
        self.saw_copy_source_this_turn = false;
    }

    /// Inner implementation with an injectable clipboard backend for testing.
    fn copy_last_agent_markdown_with(
        &mut self,
        copy_fn: impl FnOnce(&str) -> Result<Option<crate::clipboard_copy::ClipboardLease>, String>,
    ) {
        match self.last_agent_markdown.clone() {
            Some(markdown) if !markdown.is_empty() => match copy_fn(&markdown) {
                Ok(lease) => {
                    self.clipboard_lease = lease;
                    self.add_to_history(history_cell::new_info_event(
                        "Copied last message to clipboard".into(),
                        /*hint*/ None,
                    ));
                }
                Err(error) => self.add_to_history(history_cell::new_error_event(format!(
                    "Copy failed: {error}. Try enabling OSC 52 in your terminal, installing a native clipboard provider, or running VAC from a local desktop terminal."
                ))),
            },
            _ if self.copy_history_evicted_by_rollback => {
                self.add_to_history(history_cell::new_error_event(format!(
                    "Cannot copy that response after rewinding. Only the most recent {MAX_AGENT_COPY_HISTORY} responses are available to /copy."
                )));
            }
            _ => self.add_to_history(history_cell::new_error_event(
                "No agent response to copy".into(),
            )),
        }
        self.request_redraw();
    }

    #[cfg(test)]
    pub(crate) fn last_agent_markdown_text(&self) -> Option<&str> {
        self.last_agent_markdown.as_deref()
    }

    fn show_rename_prompt(&mut self) {
        if !self.ensure_thread_rename_allowed() {
            return;
        }
        let tx = self.app_event_tx.clone();
        let existing_name = self.thread_name.as_deref().filter(|name| !name.is_empty());
        let title = if existing_name.is_some() {
            "Rename thread"
        } else {
            "Name thread"
        };
        let view = CustomPromptView::new(
            title.to_string(),
            "Type a name and press Enter".to_string(),
            /*initial_text*/ existing_name.unwrap_or_default().to_string(),
            /*context_label*/ None,
            Box::new(move |name: String| {
                let Some(name) = crate::legacy_core::util::normalize_thread_name(&name) else {
                    tx.send(AppEvent::InsertHistoryCell(Box::new(
                        history_cell::new_error_event("Thread name cannot be empty.".to_string()),
                    )));
                    return;
                };
                tx.set_thread_name(name);
            }),
        );

        self.bottom_pane.show_view(Box::new(view));
    }

    fn ensure_thread_rename_allowed(&mut self) -> bool {
        match self.thread_rename_block_message.clone() {
            Some(message) => {
                self.add_error_message(message);
                false
            }
            None => true,
        }
    }

    pub(crate) fn handle_paste(&mut self, text: String) {
        self.bottom_pane.handle_paste(text);
        self.refresh_plan_mode_nudge();
    }

    // Returns true if caller should skip rendering this frame (a future frame is scheduled).
    pub(crate) fn handle_paste_burst_tick(&mut self, frame_requester: FrameRequester) -> bool {
        if self.bottom_pane.flush_paste_burst_if_due() {
            self.refresh_plan_mode_nudge();
            // A paste just flushed; request an immediate redraw and skip this frame.
            self.request_redraw();
            true
        } else if self.bottom_pane.is_in_paste_burst() {
            // While capturing a burst, schedule a follow-up tick and skip this frame
            // to avoid redundant renders between ticks.
            frame_requester.schedule_frame_in(
                crate::bottom_pane::ChatComposer::recommended_paste_flush_delay(),
            );
            true
        } else {
            false
        }
    }

    fn flush_active_cell(&mut self) {
        if let Some(active) = self.active_cell.take() {
            self.needs_final_message_separator = true;
            self.app_event_tx.send(AppEvent::InsertHistoryCell(active));
        }
    }

