// O5/O6 balanced ChatWidget impl group: source split_012_add_to_history.rs, split_013_restore_blocked_image_submission.rs
impl ChatWidget {
    pub(crate) fn add_to_history(&mut self, cell: impl HistoryCell + 'static) {
        self.add_boxed_history(Box::new(cell));
    }

    /// 00D-8: Sidebar-facing accessor for the typed runtime activity state.
    /// The App-level renderer reads this to draw the floating Activity sidebar.
    pub(crate) fn runtime_activity(&self) -> &crate::runtime_adapter::RuntimeActivityState {
        &self.runtime_activity
    }

    /// 00D-8: Mutable accessor used by slash-command handlers (e.g. `/activity`)
    /// that need to flip the sidebar open/closed state.
    #[allow(dead_code)]
    pub(crate) fn runtime_activity_mut(
        &mut self,
    ) -> &mut crate::runtime_adapter::RuntimeActivityState {
        &mut self.runtime_activity
    }

    /// 00D-8: Convenience wrapper used by `/activity` to toggle the sidebar
    /// overlay state and request a redraw at the call site.
    pub(crate) fn toggle_runtime_activity_expanded(&mut self) {
        self.runtime_activity.toggle_expanded();
    }

    /// Project a batch of [`crate::runtime_adapter::RuntimeEvent`]s into the
    /// sidebar's typed activity state, plus — in verbose audit mode — the
    /// existing `vac runtime:` history rows for backwards compatibility.
    ///
    /// 00D-8: Activity rows no longer render in the main transcript by
    /// default. The sidebar reads `self.runtime_activity` directly via
    /// the App-level layout. When `VAC_RUNTIME_VERBOSE=1` is set, the full
    /// audit-style "vac runtime:" trace is still appended so existing
    /// dogfood/audit workflows are preserved.
    fn project_runtime_events(&mut self, events: Vec<crate::runtime_adapter::RuntimeEvent>) {
        for event in events {
            // Bridge-aware activity item: pairs ApprovalResolved with the
            // kind captured at request-time so the sidebar shows e.g.
            // "approval approved — patch".
            let activity_item = match self.runtime_bridge.as_ref() {
                Some(bridge) => bridge.activity_item_for(&event),
                None => crate::runtime_adapter::runtime_activity_item(&event),
            };
            if let Some(item) = activity_item {
                self.runtime_activity.push(item);
            }

            // Verbose audit mode: preserve full "vac runtime:" rows in the
            // transcript. Default mode skips this so the transcript stays
            // focused on conversation, tool outputs, and final answers.
            if crate::runtime_adapter::runtime_verbose_enabled() {
                let label = match self.runtime_bridge.as_ref() {
                    Some(bridge) => bridge.verbose_label_for(&event),
                    None => crate::runtime_adapter::verbose_runtime_history_label(&event),
                };
                let cell = crate::history_cell::PlainHistoryCell::new(vec![label.into()]);
                self.add_boxed_history(Box::new(cell));
            }
        }
    }

    fn add_boxed_history(&mut self, cell: Box<dyn HistoryCell>) {
        // Keep the placeholder session header as the active cell until real session info arrives,
        // so we can merge headers instead of committing a duplicate box to history.
        let keep_placeholder_header_active = !self.is_session_configured()
            && self
                .active_cell
                .as_ref()
                .is_some_and(|c| c.as_any().is::<history_cell::SessionHeaderHistoryCell>());

        if !keep_placeholder_header_active && !cell.display_lines(u16::MAX).is_empty() {
            // Only break exec grouping if the cell renders visible lines.
            self.flush_active_cell();
            self.needs_final_message_separator = true;
        }
        self.app_event_tx.send(AppEvent::InsertHistoryCell(cell));
    }

    fn queue_user_message(&mut self, user_message: UserMessage) {
        self.queue_user_message_with_options(user_message, QueuedInputAction::Plain);
    }

    fn queue_user_message_with_options(
        &mut self,
        user_message: UserMessage,
        action: QueuedInputAction,
    ) {
        if !self.is_session_configured() || self.is_user_turn_pending_or_running() {
            self.queued_user_messages
                .push_back(QueuedUserMessage::new(user_message, action));
            self.queued_user_message_history_records
                .push_back(UserMessageHistoryRecord::UserMessageText);
            self.refresh_pending_input_preview();
        } else {
            self.submit_user_message(user_message);
        }
    }

    fn submit_shell_command(&mut self, command: &str) -> QueueDrain {
        let cmd = command.trim();
        if cmd.is_empty() {
            self.app_event_tx.send(AppEvent::InsertHistoryCell(Box::new(
                history_cell::new_info_event(
                    USER_SHELL_COMMAND_HELP_TITLE.to_string(),
                    Some(USER_SHELL_COMMAND_HELP_HINT.to_string()),
                ),
            )));
            QueueDrain::Continue
        } else {
            self.submit_op(AppCommand::run_user_shell_command(cmd.to_string()));
            QueueDrain::Stop
        }
    }

    fn submit_shell_command_with_history(
        &mut self,
        command: &str,
        history_text: &str,
    ) -> QueueDrain {
        let drain = self.submit_shell_command(command);
        if drain == QueueDrain::Stop {
            self.submit_op(AppCommand::add_to_history(history_text.to_string()));
        }
        drain
    }

    fn submit_queued_shell_prompt(&mut self, user_message: UserMessage) -> QueueDrain {
        match user_message.text.strip_prefix('!') {
            Some(command) => {
                let history_text = user_message.text.clone();
                self.submit_shell_command_with_history(command, &history_text)
            }
            None => {
                self.submit_user_message(user_message);
                QueueDrain::Stop
            }
        }
    }

    fn submit_user_message(&mut self, user_message: UserMessage) {
        let _accepted = self.submit_user_message_with_history_record(
            user_message,
            UserMessageHistoryRecord::UserMessageText,
        );
    }

    fn submit_user_message_with_history_record(
        &mut self,
        user_message: UserMessage,
        history_record: UserMessageHistoryRecord,
    ) -> bool {
        self.submit_user_message_with_history_and_shell_escape_policy(
            user_message,
            history_record,
            ShellEscapePolicy::Allow,
        )
        .0
    }

    fn submit_user_message_with_shell_escape_policy(
        &mut self,
        user_message: UserMessage,
        shell_escape_policy: ShellEscapePolicy,
    ) -> Option<AppCommand> {
        self.submit_user_message_with_history_and_shell_escape_policy(
            user_message,
            UserMessageHistoryRecord::UserMessageText,
            shell_escape_policy,
        )
        .1
    }

    fn submit_user_message_with_history_and_shell_escape_policy(
        &mut self,
        user_message: UserMessage,
        history_record: UserMessageHistoryRecord,
        shell_escape_policy: ShellEscapePolicy,
    ) -> (bool, Option<AppCommand>) {
        if !self.is_session_configured() {
            tracing::warn!("cannot submit user message before session is configured; queueing");
            self.queued_user_messages
                .push_front(QueuedUserMessage::from(user_message));
            self.queued_user_message_history_records
                .push_front(history_record);
            self.refresh_pending_input_preview();
            return (true, None);
        }
        if user_message.text.is_empty()
            && user_message.local_images.is_empty()
            && user_message.remote_image_urls.is_empty()
        {
            return (false, None);
        }
        if (!user_message.local_images.is_empty() || !user_message.remote_image_urls.is_empty())
            && !self.current_model_supports_images()
        {
            let UserMessage {
                text,
                text_elements,
                local_images,
                mention_bindings,
                remote_image_urls,
            } = user_message_for_restore(user_message, &history_record);
            self.restore_blocked_image_submission(
                text,
                text_elements,
                local_images,
                mention_bindings,
                remote_image_urls,
            );
            return (false, None);
        }
        let UserMessage {
            text,
            local_images,
            remote_image_urls,
            text_elements,
            mention_bindings,
        } = user_message;

        let render_in_history = !self.agent_turn_running;
        let mut items: Vec<UserInput> = Vec::new();

        // Special-case: "!cmd" executes a local shell command instead of sending to the model.
        if shell_escape_policy == ShellEscapePolicy::Allow
            && let Some(stripped) = text.strip_prefix('!')
        {
            let app_command = match self.submit_shell_command_with_history(stripped, &text) {
                QueueDrain::Continue => None,
                QueueDrain::Stop => Some(AppCommand::run_user_shell_command(
                    stripped.trim().to_string(),
                )),
            };
            return (app_command.is_some(), app_command);
        }

        for image_url in &remote_image_urls {
            items.push(UserInput::Image {
                url: image_url.clone(),
            });
        }

        for image in &local_images {
            items.push(UserInput::LocalImage {
                path: image.path.clone(),
            });
        }

        if !text.is_empty() {
            items.push(UserInput::Text {
                text: text.clone(),
                text_elements: app_server_text_elements(&text_elements),
            });
        }

        let mentions = collect_tool_mentions(&text, &HashMap::new());
        let bound_names: HashSet<String> = mention_bindings
            .iter()
            .map(|binding| binding.mention.clone())
            .collect();
        let mut skill_names_lower: HashSet<String> = HashSet::new();
        let mut selected_skill_paths: HashSet<AbsolutePathBuf> = HashSet::new();
        let mut selected_plugin_ids: HashSet<String> = HashSet::new();

        if let Some(skills) = self.bottom_pane.skills() {
            skill_names_lower = skills
                .iter()
                .map(|skill| skill.name.to_ascii_lowercase())
                .collect();

            for binding in &mention_bindings {
                let path = binding
                    .path
                    .strip_prefix("skill://")
                    .unwrap_or(binding.path.as_str());
                let path = Path::new(path);
                if let Some(skill) = skills
                    .iter()
                    .find(|skill| skill.path_to_skills_md.as_path() == path)
                    && selected_skill_paths.insert(skill.path_to_skills_md.clone())
                {
                    items.push(UserInput::Skill {
                        name: skill.name.clone(),
                        path: skill.path_to_skills_md.to_path_buf(),
                    });
                }
            }

            let skill_mentions = find_skill_mentions_with_tool_mentions(&mentions, skills);
            for skill in skill_mentions {
                if bound_names.contains(skill.name.as_str())
                    || !selected_skill_paths.insert(skill.path_to_skills_md.clone())
                {
                    continue;
                }
                items.push(UserInput::Skill {
                    name: skill.name.clone(),
                    path: skill.path_to_skills_md.to_path_buf(),
                });
            }
        }

        if let Some(plugins) = self.plugins_for_mentions() {
            for binding in &mention_bindings {
                let Some(plugin_config_name) = binding
                    .path
                    .strip_prefix("plugin://")
                    .filter(|id| !id.is_empty())
                else {
                    continue;
                };
                if !selected_plugin_ids.insert(plugin_config_name.to_string()) {
                    continue;
                }
                if let Some(plugin) = plugins
                    .iter()
                    .find(|plugin| plugin.config_name == plugin_config_name)
                {
                    items.push(UserInput::Mention {
                        name: plugin.display_name.clone(),
                        path: binding.path.clone(),
                    });
                }
            }
        }

        let mut selected_app_ids: HashSet<String> = HashSet::new();
        if let Some(apps) = self.connectors_for_mentions() {
            for binding in &mention_bindings {
                let Some(app_id) = binding
                    .path
                    .strip_prefix("app://")
                    .filter(|id| !id.is_empty())
                else {
                    continue;
                };
                if !selected_app_ids.insert(app_id.to_string()) {
                    continue;
                }
                if let Some(app) = apps.iter().find(|app| app.id == app_id && app.is_enabled) {
                    items.push(UserInput::Mention {
                        name: app.name.clone(),
                        path: binding.path.clone(),
                    });
                }
            }

            let app_mentions = find_app_mentions(&mentions, apps, &skill_names_lower);
            for app in app_mentions {
                let slug = vac_connectors::metadata::connector_mention_slug(&app);
                if bound_names.contains(&slug) || !selected_app_ids.insert(app.id.clone()) {
                    continue;
                }
                let app_id = app.id.as_str();
                items.push(UserInput::Mention {
                    name: app.name.clone(),
                    path: format!("app://{app_id}"),
                });
            }
        }

        let effective_mode = self.effective_collaboration_mode();
        if effective_mode.model().trim().is_empty() {
            self.add_error_message(
                "Thread model is unavailable. Wait for the thread to finish syncing or choose a model before sending input.".to_string(),
            );
            self.restore_user_message_to_composer(user_message_for_restore(
                UserMessage {
                    text,
                    local_images,
                    remote_image_urls,
                    text_elements,
                    mention_bindings,
                },
                &history_record,
            ));
            return (false, None);
        }

        self.maybe_apply_ide_context(&mut items);

        let collaboration_mode = if self.collaboration_modes_enabled() {
            self.active_collaboration_mask
                .as_ref()
                .map(|_| effective_mode.clone())
        } else {
            None
        };
        let pending_steer = (!render_in_history).then(|| PendingSteer {
            user_message: UserMessage {
                text: text.clone(),
                local_images: local_images.clone(),
                remote_image_urls: remote_image_urls.clone(),
                text_elements: text_elements.clone(),
                mention_bindings: mention_bindings.clone(),
            },
            history_record: history_record.clone(),
            compare_key: Self::pending_steer_compare_key_from_items(&items),
        });
        let personality = self
            .config
            .personality
            .filter(|_| self.config.features.enabled(Feature::Personality))
            .filter(|_| self.current_model_supports_personality());
        let service_tier = match self.config.service_tier {
            Some(service_tier) => Some(Some(service_tier)),
            None if self.config.notices.fast_default_opt_out == Some(true) => Some(None),
            None => None,
        };
        let permission_profile = self.config.permissions.permission_profile();

        // Step 00D-5: Local Runtime Contract is the canonical source-of-truth.
        //
        // Bottom-input Enter routes through `RuntimeCommand::StartTask` as
        // the only canonical product-level intent for a fresh prompt. The
        // legacy `AppCommand::UserTurn` op constructed below is documented
        // as a temporary compatibility transport (see
        // `LegacyCompatTransport::UserTurn`) and is scheduled for retirement
        // in 00E. Naming, control-flow and comments here MUST keep
        // `RuntimeCommand::StartTask` as the canonical input; the legacy op
        // is derived from the runtime-first plan, never the other way
        // around.
        let runtime_start_command = crate::runtime_adapter::mint_start_task(
            text.clone(),
            self.config.cwd.to_path_buf(),
            crate::runtime_adapter::default_autonomy_mode(),
        );
        let Some(runtime_submission) =
            crate::runtime_adapter::RuntimeSubmitPlan::from_runtime_command(runtime_start_command)
        else {
            tracing::error!(
                target: "vac::local_runtime::submit",
                "runtime-first submit aborted: mint_start_task did not yield RuntimeCommand::StartTask"
            );
            return (false, None);
        };

        tracing::info!(
            target: "vac::local_runtime::submit",
            "{}",
            runtime_submission.trace()
        );
        tracing::info!(
            target: "vac::local_runtime::submit",
            transport = "legacy-user-turn",
            "runtime-first submission: routing canonical RuntimeCommand::StartTask through legacy compat transport; 00E retires this transport"
        );

        // Stand up the Local Runtime Contract bridge for this turn from the
        // runtime-first plan and surface its opening events
        // (SessionStarted + TaskStarted) in the existing root `vac` history
        // surface so operators see the contract activate at submit time.
        let mut bridge = crate::runtime_adapter::RuntimeBridge::new(
            runtime_submission.session.clone(),
            runtime_submission.task.clone(),
        );
        let opening = bridge.opening_events();
        self.runtime_bridge = Some(bridge);
        self.project_runtime_events(opening);

        // Build the legacy compatibility transport from the runtime-first
        // plan. This op is the carrier that crosses the in-process
        // app-server boundary today; 00E will replace it with a direct
        // local runtime driver call so the legacy `UserTurn` reachability
        // disappears entirely. The `cwd` is taken from the runtime-first
        // plan, not re-derived from `self.config`, so the canonical input
        // remains the only source-of-truth for fresh prompt routing.
        debug_assert_eq!(
            runtime_submission.legacy_compat_transport,
            crate::runtime_adapter::LegacyCompatTransport::UserTurn,
        );
        let op = AppCommand::user_turn(
            items,
            runtime_submission.start_task.cwd.clone(),
            AskForApproval::from(self.config.permissions.approval_policy.value()),
            permission_profile,
            effective_mode.model().to_string(),
            effective_mode.reasoning_effort(),
            /*summary*/ None,
            service_tier,
            /*final_output_json_schema*/ None,
            collaboration_mode,
            personality,
        );

        if !self.submit_op(op.clone()) {
            return (false, None);
        }
        if render_in_history {
            self.user_turn_pending_start = true;
        }

        // Persist the submitted text to cross-session message history. Mentions are encoded into
        // placeholder syntax so recall can reconstruct the mention bindings in a future session.
        let encoded_mentions = mention_bindings
            .iter()
            .map(|binding| LinkedMention {
                mention: binding.mention.clone(),
                path: binding.path.clone(),
            })
            .collect::<Vec<_>>();
        let history_text = match &history_record {
            UserMessageHistoryRecord::UserMessageText if !text.is_empty() => {
                Some(encode_history_mentions(&text, &encoded_mentions))
            }
            UserMessageHistoryRecord::Override(history) if !history.text.is_empty() => {
                Some(encode_history_mentions(&history.text, &encoded_mentions))
            }
            UserMessageHistoryRecord::UserMessageText | UserMessageHistoryRecord::Override(_) => {
                None
            }
        };
        if let Some(history_text) = history_text {
            self.submit_op(AppCommand::add_to_history(history_text));
        }

        if let Some(pending_steer) = pending_steer {
            self.pending_steers.push_back(pending_steer);
            self.saw_plan_item_this_turn = false;
            self.refresh_pending_input_preview();
        }

        // Show replayable user content in conversation history.
        let display_user_message = render_in_history.then(|| {
            user_message_display_for_history(
                UserMessage {
                    text,
                    local_images,
                    remote_image_urls,
                    text_elements,
                    mention_bindings,
                },
                &history_record,
            )
        });
        if let Some(display) = display_user_message {
            self.on_user_message_display(display);
        }

        self.needs_final_message_separator = false;
        (true, Some(op))
    }

    /// Restore the blocked submission draft without losing mention resolution state.
    ///
    /// The blocked-image path intentionally keeps the draft in the composer so
    /// users can remove attachments and retry. We must restore
    /// mention bindings alongside visible text; restoring only `$name` tokens
    /// makes the draft look correct while degrading mention resolution to
    /// name-only heuristics on retry.
    fn restore_blocked_image_submission(
        &mut self,
        text: String,
        text_elements: Vec<TextElement>,
        local_images: Vec<LocalImageAttachment>,
        mention_bindings: Vec<MentionBinding>,
        remote_image_urls: Vec<String>,
    ) {
        // Preserve the user's composed payload so they can retry after changing models.
        let local_image_paths = local_images.iter().map(|img| img.path.clone()).collect();
        self.set_remote_image_urls(remote_image_urls);
        self.bottom_pane.set_composer_text_with_mention_bindings(
            text,
            text_elements,
            local_image_paths,
            mention_bindings,
        );
        self.add_to_history(history_cell::new_warning_event(
            self.image_inputs_not_supported_message(),
        ));
        self.request_redraw();
    }

    /// Replay a subset of initial events into the UI to seed the transcript when
    /// resuming an existing session. This approximates the live event flow and
    /// is intentionally conservative: only safe-to-replay items are rendered to
    /// avoid triggering side effects. Event ids are passed as `None` to
    /// distinguish replayed events from live ones.
    pub(crate) fn replay_thread_turns(&mut self, turns: Vec<Turn>, replay_kind: ReplayKind) {
        for turn in turns {
            let Turn {
                id: turn_id,
                items,
                status,
                error,
                started_at,
                completed_at,
                duration_ms,
            } = turn;
            if matches!(status, TurnStatus::InProgress) {
                self.last_non_retry_error = None;
                self.on_task_started();
            }
            for item in items {
                self.replay_thread_item(item, turn_id.clone(), replay_kind);
            }
            if matches!(
                status,
                TurnStatus::Completed | TurnStatus::Interrupted | TurnStatus::Failed
            ) {
                self.handle_turn_completed_notification(
                    TurnCompletedNotification {
                        thread_id: self.thread_id.map(|id| id.to_string()).unwrap_or_default(),
                        turn: Turn {
                            id: turn_id,
                            items: Vec::new(),
                            status,
                            error,
                            started_at,
                            completed_at,
                            duration_ms,
                        },
                    },
                    Some(replay_kind),
                );
            }
        }
    }

    pub(crate) fn replay_thread_item(
        &mut self,
        item: ThreadItem,
        turn_id: String,
        replay_kind: ReplayKind,
    ) {
        self.handle_thread_item(item, turn_id, ThreadItemRenderSource::Replay(replay_kind));
    }

    fn handle_thread_item(
        &mut self,
        item: ThreadItem,
        turn_id: String,
        render_source: ThreadItemRenderSource,
    ) {
        let from_replay = render_source.is_replay();
        let replay_kind = render_source.replay_kind();
        match item {
            ThreadItem::UserMessage { content, .. } => {
                self.on_committed_user_message(&content, from_replay);
            }
            ThreadItem::AgentMessage {
                id,
                text,
                phase,
                memory_citation,
            } => {
                self.on_agent_message_item_completed(AgentMessageItem {
                    id,
                    content: vec![AgentMessageContent::Text { text }],
                    phase,
                    memory_citation: memory_citation.map(|citation| {
                        vac_protocol::memory_citation::MemoryCitation {
                            entries: citation
                                .entries
                                .into_iter()
                                .map(|entry| vac_protocol::memory_citation::MemoryCitationEntry {
                                    path: entry.path,
                                    line_start: entry.line_start,
                                    line_end: entry.line_end,
                                    note: entry.note,
                                })
                                .collect(),
                            rollout_ids: citation.thread_ids,
                        }
                    }),
                });
            }
            ThreadItem::Plan { text, .. } => self.on_plan_item_completed(text),
            ThreadItem::Reasoning {
                summary, content, ..
            } => {
                if from_replay {
                    for delta in summary {
                        self.on_agent_reasoning_delta(delta);
                    }
                    if self.config.show_raw_agent_reasoning {
                        for delta in content {
                            self.on_agent_reasoning_delta(delta);
                        }
                    }
                }
                self.on_agent_reasoning_final();
            }
            item @ ThreadItem::CommandExecution {
                status: crate::session_protocol::CommandExecutionStatus::InProgress,
                ..
            } => self.on_command_execution_started(item),
            item @ ThreadItem::CommandExecution { .. } => self.on_command_execution_completed(item),
            ThreadItem::FileChange {
                status: crate::session_protocol::PatchApplyStatus::InProgress,
                ..
            } => {}
            item @ ThreadItem::FileChange { .. } => self.on_file_change_completed(item),
            item @ ThreadItem::McpToolCall { .. } => self.on_mcp_tool_call_completed(item),
            ThreadItem::WebSearch { id, query, action } => {
                self.on_web_search_begin(id.clone());
                self.on_web_search_end(
                    id,
                    query,
                    action.unwrap_or(crate::session_protocol::WebSearchAction::Other),
                );
            }
            ThreadItem::ImageView { id: _, path } => {
                self.on_view_image_tool_call(path);
            }
            ThreadItem::ImageGeneration {
                id,
                revised_prompt,
                saved_path,
                ..
            } => {
                self.on_image_generation_end(id, revised_prompt, saved_path);
            }
            ThreadItem::EnteredReviewMode { review, .. } => {
                if from_replay {
                    self.enter_review_mode_with_hint(review, /*from_replay*/ true);
                }
            }
            ThreadItem::ExitedReviewMode { .. } => {
                self.exit_review_mode_after_item();
            }
            ThreadItem::ContextCompaction { .. } => {
                self.add_info_message("Context compacted".to_string(), /*hint*/ None);
            }
            ThreadItem::HookPrompt { .. } => {}
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
            ThreadItem::DynamicToolCall { .. } => {}
        }

        if matches!(replay_kind, Some(ReplayKind::ThreadSnapshot)) && turn_id.is_empty() {
            self.request_redraw();
        }
    }

    pub(crate) fn handle_server_request(
        &mut self,
        request: ServerRequest,
        replay_kind: Option<ReplayKind>,
    ) {
        let id = request.id().to_string();
        match request {
            ServerRequest::CommandExecutionRequestApproval { params, .. } => {
                let fallback_cwd = self.config.cwd.clone();
                self.on_exec_approval_request(
                    id,
                    exec_approval_request_from_params(params, &fallback_cwd),
                );
            }
            ServerRequest::FileChangeRequestApproval { params, .. } => {
                self.on_apply_patch_approval_request(
                    id,
                    patch_approval_request_from_params(params),
                );
            }
            ServerRequest::McpServerElicitationRequest { request_id, params } => {
                self.on_elicitation_request(request_id, params);
            }
            ServerRequest::PermissionsRequestApproval { params, .. } => {
                self.on_request_permissions(request_permissions_from_params(params));
            }
            ServerRequest::ToolRequestUserInput { params, .. } => {
                self.on_request_user_input(params);
            }
            ServerRequest::DynamicToolCall { .. }
            | ServerRequest::ChatgptAuthTokensRefresh { .. }
            | ServerRequest::ApplyPatchApproval { .. }
            | ServerRequest::ExecCommandApproval { .. } => {
                if replay_kind.is_none() {
                    self.add_error_message(TUI_STUB_MESSAGE.to_string());
                }
            }
        }
    }

    pub(crate) fn handle_server_notification(
        &mut self,
        notification: ServerNotification,
        replay_kind: Option<ReplayKind>,
    ) {
        if self.active_side_conversation
            && replay_kind.is_none()
            && matches!(notification, ServerNotification::McpServerStatusUpdated(_))
        {
            return;
        }
        let from_replay = replay_kind.is_some();
        let is_resume_initial_replay =
            matches!(replay_kind, Some(ReplayKind::ResumeInitialMessages));
        let is_retry_error = matches!(
            &notification,
            ServerNotification::Error(ErrorNotification {
                will_retry: true,
                ..
            })
        );
        if !is_resume_initial_replay && !is_retry_error {
            self.restore_retry_status_header_if_present();
        }
        match notification {
            ServerNotification::ThreadTokenUsageUpdated(notification) => {
                self.set_token_info(Some(token_usage_info_from_app_server(
                    notification.token_usage,
                )));
            }
            ServerNotification::ThreadNameUpdated(notification) => {
                match ThreadId::from_string(&notification.thread_id) {
                    Ok(thread_id) => {
                        self.on_thread_name_updated(thread_id, notification.thread_name)
                    }
                    Err(err) => {
                        tracing::warn!(
                            thread_id = notification.thread_id,
                            error = %err,
                            "ignoring app-server ThreadNameUpdated with invalid thread_id"
                        );
                    }
                }
            }
            ServerNotification::ThreadGoalUpdated(notification) => {
                self.on_thread_goal_updated(
                    thread_goal_from_app_server(notification.goal),
                    notification.turn_id,
                );
            }
            ServerNotification::ThreadGoalCleared(notification) => {
                self.on_thread_goal_cleared(notification.thread_id.as_str());
            }
            ServerNotification::TurnStarted(notification) => {
                self.last_turn_id = Some(notification.turn.id);
                self.last_non_retry_error = None;
                if !matches!(replay_kind, Some(ReplayKind::ResumeInitialMessages)) {
                    self.on_task_started();
                }
            }
            ServerNotification::TurnCompleted(notification) => {
                self.handle_turn_completed_notification(notification, replay_kind);
            }
            ServerNotification::ItemStarted(notification) => {
                self.handle_item_started_notification(notification, replay_kind.is_some());
            }
            ServerNotification::ItemCompleted(notification) => {
                self.handle_item_completed_notification(notification, replay_kind);
            }
            ServerNotification::AgentMessageDelta(notification) => {
                self.on_agent_message_delta(notification.delta);
            }
            ServerNotification::PlanDelta(notification) => self.on_plan_delta(notification.delta),
            ServerNotification::ReasoningSummaryTextDelta(notification) => {
                self.on_agent_reasoning_delta(notification.delta);
            }
            ServerNotification::ReasoningTextDelta(notification) => {
                if self.config.show_raw_agent_reasoning {
                    self.on_agent_reasoning_delta(notification.delta);
                }
            }
            ServerNotification::ReasoningSummaryPartAdded(_) => self.on_reasoning_section_break(),
            ServerNotification::TerminalInteraction(notification) => {
                self.on_terminal_interaction(notification.process_id, notification.stdin)
            }
            ServerNotification::CommandExecutionOutputDelta(notification) => {
                self.on_exec_command_output_delta(&notification.item_id, &notification.delta);
            }
            ServerNotification::FileChangeOutputDelta(notification) => {
                self.on_patch_apply_output_delta(notification.item_id, notification.delta);
            }
            ServerNotification::TurnDiffUpdated(notification) => {
                self.on_turn_diff(notification.diff)
            }
            ServerNotification::TurnPlanUpdated(notification) => {
                self.on_plan_update(UpdatePlanArgs {
                    explanation: notification.explanation,
                    plan: notification
                        .plan
                        .into_iter()
                        .map(|step| UpdatePlanItemArg {
                            step: step.step,
                            status: turn_plan_step_status_from_app_server(step.status),
                        })
                        .collect(),
                })
            }
            ServerNotification::HookStarted(notification) => {
                self.on_hook_started(notification.run);
            }
            ServerNotification::HookCompleted(notification) => {
                self.on_hook_completed(notification.run);
            }
            ServerNotification::Error(notification) => {
                if notification.will_retry {
                    if !from_replay {
                        self.on_stream_error(
                            notification.error.message,
                            notification.error.additional_details,
                        );
                    }
                } else {
                    self.last_non_retry_error = Some((
                        notification.turn_id.clone(),
                        notification.error.message.clone(),
                    ));
                    self.handle_non_retry_error(
                        notification.error.message,
                        notification.error.vac_error_info,
                    );
                }
            }
            ServerNotification::SkillsChanged(_) => {
                self.refresh_skills_for_current_cwd(/*force_reload*/ true);
            }
            ServerNotification::ModelRerouted(_) => {}
            ServerNotification::ModelVerification(notification) => {
                let verifications = notification
                    .verifications
                    .iter()
                    .copied()
                    .map(|verification| verification.to_core())
                    .collect::<Vec<_>>();
                self.on_app_server_model_verification(&verifications)
            }
            ServerNotification::Warning(notification) => self.on_warning(notification.message),
            ServerNotification::GuardianWarning(notification) => {
                self.on_warning(notification.message)
            }
            ServerNotification::DeprecationNotice(notification) => {
                self.on_deprecation_notice(notification.summary, notification.details)
            }
            ServerNotification::ConfigWarning(notification) => self.on_warning(
                notification
                    .details
                    .map(|details| format!("{}: {details}", notification.summary))
                    .unwrap_or(notification.summary),
            ),
            ServerNotification::McpServerStatusUpdated(notification) => {
                self.on_mcp_server_status_updated(notification)
            }
            ServerNotification::ItemGuardianApprovalReviewStarted(notification) => {
                self.on_guardian_review_notification(
                    notification.review_id,
                    notification.turn_id,
                    notification.review,
                    /*decision_source*/ None,
                    notification.action,
                );
            }
            ServerNotification::ItemGuardianApprovalReviewCompleted(notification) => {
                let decision_source =
                    crate::session_protocol::guardian_decision_source_from_app_server(
                        notification.decision_source,
                    );
                self.on_guardian_review_notification(
                    notification.review_id,
                    notification.turn_id,
                    notification.review,
                    Some(decision_source),
                    notification.action,
                );
            }
            ServerNotification::ThreadClosed(_) => {
                if !from_replay {
                    self.on_shutdown_complete();
                }
            }
            ServerNotification::ThreadRealtimeStarted(notification) => {
                if !from_replay {
                    self.on_realtime_conversation_started(notification);
                }
            }
            ServerNotification::ThreadRealtimeItemAdded(notification) => {
                if !from_replay {
                    self.on_realtime_item_added(notification);
                }
            }
            ServerNotification::ThreadRealtimeOutputAudioDelta(notification) => {
                if !from_replay {
                    self.on_realtime_output_audio_delta(notification);
                }
            }
            ServerNotification::ThreadRealtimeError(notification) => {
                if !from_replay {
                    self.on_realtime_error(notification);
                }
            }
            ServerNotification::ThreadRealtimeClosed(notification) => {
                if !from_replay {
                    self.on_realtime_conversation_closed(notification);
                }
            }
            ServerNotification::ThreadRealtimeSdp(notification) => {
                if !from_replay {
                    self.on_realtime_conversation_sdp(notification.sdp);
                }
            }
            ServerNotification::ServerRequestResolved(_)
            | ServerNotification::AccountUpdated(_)
            | ServerNotification::AccountRateLimitsUpdated(_)
            | ServerNotification::ThreadStarted(_)
            | ServerNotification::ThreadStatusChanged(_)
            | ServerNotification::ThreadArchived(_)
            | ServerNotification::ThreadUnarchived(_)
            | ServerNotification::RawResponseItemCompleted(_)
            | ServerNotification::CommandExecOutputDelta(_)
            | ServerNotification::FileChangePatchUpdated(_)
            | ServerNotification::McpToolCallProgress(_)
            | ServerNotification::McpServerOauthLoginCompleted(_)
            | ServerNotification::AppListUpdated(_)
            | ServerNotification::RemoteControlStatusChanged(_)
            | ServerNotification::ExternalAgentConfigImportCompleted(_)
            | ServerNotification::FsChanged(_)
            | ServerNotification::FuzzyFileSearchSessionUpdated(_)
            | ServerNotification::FuzzyFileSearchSessionCompleted(_)
            | ServerNotification::ThreadRealtimeTranscriptDelta(_)
            | ServerNotification::ThreadRealtimeTranscriptDone(_)
            | ServerNotification::WindowsWorldWritableWarning(_)
            | ServerNotification::WindowsSandboxSetupCompleted(_)
            | ServerNotification::AccountLoginCompleted(_) => {}
            ServerNotification::ContextCompacted(_) => {}
        }
    }
}
