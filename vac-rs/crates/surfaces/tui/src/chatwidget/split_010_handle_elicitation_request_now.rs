    pub(crate) fn handle_elicitation_request_now(
        &mut self,
        request_id: AppServerRequestId,
        params: McpServerElicitationRequestParams,
    ) {
        self.flush_answer_stream_with_separator();

        self.notify(Notification::ElicitationRequested {
            server_name: params.server_name.clone(),
        });

        // Mirror the MCP elicitation request into the Local Runtime Contract.
        // The protocol `request_id` (stringified via `Display`) is the
        // registry key; the overlay-side resolve fn re-derives the same key.
        let elicitation_message: &str = match &params.request {
            McpServerElicitationRequest::Form { message, .. }
            | McpServerElicitationRequest::Url { message, .. } => message.as_str(),
        };
        let request_key = request_id.to_string();
        let runtime_events = self.runtime_bridge.as_mut().map(|bridge| {
            bridge.project_mcp_elicitation_request(
                &request_key,
                &params.server_name,
                elicitation_message,
            )
        });
        if let Some(events) = runtime_events {
            self.project_runtime_events(events);
        }

        let thread_id = self.thread_id.unwrap_or_default();
        if let Some(request) = McpServerElicitationFormRequest::from_app_server_request(
            thread_id,
            request_id.clone(),
            params.clone(),
        ) {
            self.bottom_pane
                .push_mcp_server_elicitation_request(request);
        } else {
            let request = ApprovalRequest::McpElicitation {
                thread_id,
                thread_label: None,
                server_name: params.server_name,
                request_id,
                message: match params.request {
                    McpServerElicitationRequest::Form { message, .. }
                    | McpServerElicitationRequest::Url { message, .. } => message,
                },
            };
            self.bottom_pane
                .push_approval_request(request, &self.config.features);
        }
        self.request_redraw();
    }

    pub(crate) fn push_approval_request(&mut self, request: ApprovalRequest) {
        self.bottom_pane
            .push_approval_request(request, &self.config.features);
        self.request_redraw();
    }

    pub(crate) fn push_mcp_server_elicitation_request(
        &mut self,
        request: McpServerElicitationFormRequest,
    ) {
        self.bottom_pane
            .push_mcp_server_elicitation_request(request);
        self.request_redraw();
    }

    pub(crate) fn handle_request_user_input_now(&mut self, ev: ToolRequestUserInputParams) {
        self.flush_answer_stream_with_separator();
        let question_count = ev.questions.len();
        let summary = Notification::user_input_request_summary(&ev.questions);
        let title = match (question_count, summary.as_deref()) {
            (1, Some(summary)) => summary.to_string(),
            (1, None) => "Question requested".to_string(),
            (count, _) => format!("{count} questions requested"),
        };
        self.notify(Notification::PlanModePrompt { title });
        self.bottom_pane.push_user_input_request(ev);
        self.request_redraw();
    }

    pub(crate) fn handle_request_permissions_now(&mut self, ev: RequestPermissionsEvent) {
        self.flush_answer_stream_with_separator();

        // Mirror the permissions request into the Local Runtime Contract so
        // the operator decision later resolves with a correlated `ApprovalId`.
        // Mirrors the exec/patch projection pattern.
        let runtime_events = self.runtime_bridge.as_mut().map(|bridge| {
            bridge.project_permissions_approval_request(
                &ev.call_id,
                "additional permissions requested",
                ev.reason.as_deref(),
            )
        });
        if let Some(events) = runtime_events {
            self.project_runtime_events(events);
        }

        let request = ApprovalRequest::Permissions {
            thread_id: self.thread_id.unwrap_or_default(),
            thread_label: None,
            call_id: ev.call_id,
            reason: ev.reason,
            permissions: ev.permissions,
        };
        self.bottom_pane
            .push_approval_request(request, &self.config.features);
        self.request_redraw();
    }

    pub(crate) fn handle_command_execution_started_now(&mut self, item: ThreadItem) {
        let ThreadItem::CommandExecution {
            id,
            command,
            source,
            command_actions,
            ..
        } = item
        else {
            return;
        };
        let (command, parsed_cmd) =
            command_execution_command_and_parsed(&command, &command_actions);
        // Ensure the status indicator is visible while the command runs.
        self.bottom_pane.ensure_status_indicator();
        let parsed_cmd = self.annotate_skill_reads_in_parsed_cmd(parsed_cmd);
        self.running_commands.insert(
            id.clone(),
            RunningCommand {
                command: command.clone(),
                parsed_cmd: parsed_cmd.clone(),
                source,
            },
        );
        let is_wait_interaction = matches!(source, ExecCommandSource::UnifiedExecInteraction);
        let command_display = command.join(" ");
        let should_suppress_unified_wait = is_wait_interaction
            && self
                .last_unified_wait
                .as_ref()
                .is_some_and(|wait| wait.is_duplicate(&command_display));
        if is_wait_interaction {
            self.last_unified_wait = Some(UnifiedExecWaitState::new(command_display));
        } else {
            self.last_unified_wait = None;
        }
        if should_suppress_unified_wait {
            self.suppressed_exec_calls.insert(id);
            return;
        }
        let runtime_exec_started_evs = self
            .runtime_bridge
            .as_mut()
            .map(|b| b.project_exec_command_started(&id, &command))
            .unwrap_or_default();
        self.project_runtime_events(runtime_exec_started_evs);
        if let Some(cell) = self
            .active_cell
            .as_mut()
            .and_then(|c| c.as_any_mut().downcast_mut::<ExecCell>())
            && let Some(new_exec) = cell.with_added_call(
                id.clone(),
                command.clone(),
                parsed_cmd.clone(),
                source,
                /*interaction_input*/ None,
            )
        {
            *cell = new_exec;
            self.bump_active_cell_revision();
        } else {
            self.flush_active_cell();

            self.active_cell = Some(Box::new(new_active_exec_command(
                id,
                command,
                parsed_cmd,
                source,
                /*interaction_input*/ None,
                self.config.animations,
            )));
            self.bump_active_cell_revision();
        }

        self.request_redraw();
    }

    pub(crate) fn handle_mcp_tool_call_started_now(&mut self, item: ThreadItem) {
        let ThreadItem::McpToolCall {
            id,
            server,
            tool,
            arguments,
            ..
        } = item
        else {
            return;
        };
        let runtime_mcp_started_evs = self
            .runtime_bridge
            .as_mut()
            .map(|b| b.project_mcp_tool_started(&id, &server, &tool))
            .unwrap_or_default();
        self.project_runtime_events(runtime_mcp_started_evs);
        self.flush_answer_stream_with_separator();
        self.flush_active_cell();
        self.active_cell = Some(Box::new(history_cell::new_active_mcp_tool_call(
            id,
            McpInvocation {
                server,
                tool,
                arguments: Some(arguments),
            },
            self.config.animations,
        )));
        self.bump_active_cell_revision();
        self.request_redraw();
    }

    pub(crate) fn handle_mcp_tool_call_completed_now(&mut self, item: ThreadItem) {
        self.flush_answer_stream_with_separator();

        let ThreadItem::McpToolCall {
            id,
            server,
            tool,
            arguments,
            result,
            error,
            duration_ms,
            ..
        } = item
        else {
            return;
        };
        let runtime_mcp_success = error.is_none() && result.is_some();
        let runtime_mcp_finished_evs = self
            .runtime_bridge
            .as_mut()
            .map(|b| b.project_mcp_tool_finished(&id, &server, &tool, runtime_mcp_success))
            .unwrap_or_default();
        self.project_runtime_events(runtime_mcp_finished_evs);
        let invocation = McpInvocation {
            server,
            tool,
            arguments: Some(arguments),
        };
        let duration = Duration::from_millis(duration_ms.unwrap_or_default().max(0) as u64);
        let result = match (result, error) {
            (_, Some(error)) => Err(error.message),
            (Some(result), None) => {
                let result = *result;
                Ok(vac_protocol::mcp::CallToolResult {
                    content: result.content,
                    structured_content: result.structured_content,
                    is_error: Some(false),
                    meta: None,
                })
            }
            (None, None) => Err("MCP tool call completed without a result".to_string()),
        };

        let extra_cell = match self
            .active_cell
            .as_mut()
            .and_then(|cell| cell.as_any_mut().downcast_mut::<McpToolCallCell>())
        {
            Some(cell) if cell.call_id() == id => cell.complete(duration, result),
            _ => {
                self.flush_active_cell();
                let mut cell =
                    history_cell::new_active_mcp_tool_call(id, invocation, self.config.animations);
                let extra_cell = cell.complete(duration, result);
                self.active_cell = Some(Box::new(cell));
                extra_cell
            }
        };

        self.flush_active_cell();
        if let Some(extra) = extra_cell {
            self.add_boxed_history(extra);
        }
        // Mark that actual work was done (MCP tool call)
        self.had_work_activity = true;
    }

    pub(crate) fn handle_queued_item_started_now(&mut self, item: ThreadItem) {
        match item {
            item @ ThreadItem::CommandExecution { .. } => {
                self.handle_command_execution_started_now(item);
            }
            item @ ThreadItem::McpToolCall { .. } => {
                self.handle_mcp_tool_call_started_now(item);
            }
            _ => {}
        }
    }

    pub(crate) fn handle_queued_item_completed_now(&mut self, item: ThreadItem) {
        match item {
            item @ ThreadItem::CommandExecution { .. } => {
                self.handle_command_execution_completed_now(item);
            }
            item @ ThreadItem::FileChange { .. } => self.handle_file_change_completed_now(item),
            item @ ThreadItem::McpToolCall { .. } => self.handle_mcp_tool_call_completed_now(item),
            _ => {}
        }
    }

    pub(crate) fn new_with_app_event(common: ChatWidgetInit) -> Self {
        Self::new_with_op_target(common, VACOpTarget::AppEvent)
    }

    fn new_with_op_target(common: ChatWidgetInit, vac_op_target: VACOpTarget) -> Self {
        let ChatWidgetInit {
            config,
            frame_requester,
            app_event_tx,
            initial_user_message,
            enhanced_keys_supported,
            has_chatgpt_account,
            model_catalog,
            feedback,
            is_first_run,
            status_account_display,
            runtime_model_provider_base_url,
            initial_plan_type,
            model,
            startup_tooltip_override,
            status_line_invalid_items_warned,
            terminal_title_invalid_items_warned,
            session_telemetry,
        } = common;
        let model = model.filter(|m| !m.trim().is_empty());
        let mut config = config;
        config.model = model.clone();
        let prevent_idle_sleep = config.features.enabled(Feature::PreventIdleSleep);
        let mut rng = rand::rng();
        let placeholder = PLACEHOLDERS[rng.random_range(0..PLACEHOLDERS.len())].to_string();
        let side_placeholder =
            SIDE_PLACEHOLDERS[rng.random_range(0..SIDE_PLACEHOLDERS.len())].to_string();

        let model_override = model.as_deref();
        let model_for_header = model
            .clone()
            .unwrap_or_else(|| DEFAULT_MODEL_DISPLAY_NAME.to_string());
        let active_collaboration_mask =
            Self::initial_collaboration_mask(&config, model_catalog.as_ref(), model_override);
        let header_model = active_collaboration_mask
            .as_ref()
            .and_then(|mask| mask.model.clone())
            .unwrap_or_else(|| model_for_header.clone());
        let fallback_default = Settings {
            model: header_model.clone(),
            reasoning_effort: None,
            developer_instructions: None,
        };
        // Collaboration modes start in Default mode.
        let current_collaboration_mode = CollaborationMode {
            mode: ModeKind::Default,
            settings: fallback_default,
        };

        let active_cell = Some(Self::placeholder_session_header_cell(&config));

        let current_cwd = Some(config.cwd.to_path_buf());
        let effective_service_tier = config.service_tier;
        let current_terminal_info = terminal_info();
        let runtime_keymap = RuntimeKeymap::from_config(&config.tui_keymap).ok();
        let default_keymap = RuntimeKeymap::defaults();
        let copy_last_response_binding = runtime_keymap
            .as_ref()
            .map(|keymap| keymap.app.copy.clone())
            .unwrap_or_else(|| default_keymap.app.copy.clone());
        let chat_keymap = runtime_keymap
            .as_ref()
            .map(|keymap| keymap.chat.clone())
            .unwrap_or_else(|| default_keymap.chat.clone());
        let queued_message_edit_hint_binding = queued_message_edit_hint_binding(
            &chat_keymap.edit_queued_message,
            current_terminal_info,
        );
        let mut widget = Self {
            app_event_tx: app_event_tx.clone(),
            frame_requester: frame_requester.clone(),
            vac_op_target,
            bottom_pane: BottomPane::new(BottomPaneParams {
                frame_requester,
                app_event_tx,
                has_input_focus: true,
                enhanced_keys_supported,
                placeholder_text: placeholder.clone(),
                disable_paste_burst: config.disable_paste_burst,
                animations_enabled: config.animations,
                skills: None,
            }),
            active_cell,
            active_cell_revision: 0,
            config,
            effective_service_tier,
            skills_all: Vec::new(),
            skills_initial_state: None,
            current_collaboration_mode,
            active_collaboration_mask,
            has_chatgpt_account,
            model_catalog,
            session_telemetry,
            session_header: SessionHeader::new(header_model),
            initial_user_message,
            status_account_display,
            runtime_model_provider_base_url,
            token_info: None,
            rate_limit_snapshots_by_limit_id: BTreeMap::new(),
            refreshing_status_outputs: Vec::new(),
            next_status_refresh_request_id: 0,
            plan_type: initial_plan_type,
            vac_rate_limit_reached_type: None,
            rate_limit_warnings: RateLimitWarningState::default(),
            rate_limit_switch_prompt: RateLimitSwitchPromptState::default(),
            add_credits_nudge_email_in_flight: None,
            adaptive_chunking: AdaptiveChunkingPolicy::default(),
            stream_controller: None,
            plan_stream_controller: None,
            clipboard_lease: None,
            copy_last_response_binding,
            running_commands: HashMap::new(),
            collab_agent_metadata: HashMap::new(),
            pending_collab_spawn_requests: HashMap::new(),
            suppressed_exec_calls: HashSet::new(),
            last_unified_wait: None,
            unified_exec_wait_streak: None,
            turn_sleep_inhibitor: SleepInhibitor::new(prevent_idle_sleep),
            task_complete_pending: false,
            runtime_bridge: None,
            runtime_activity: crate::runtime_adapter::RuntimeActivityState::default(),
            unified_exec_processes: Vec::new(),
            agent_turn_running: false,
            mcp_startup_status: None,
            last_agent_markdown: None,
            agent_turn_markdowns: Vec::new(),
            visible_user_turn_count: 0,
            copy_history_evicted_by_rollback: false,
            latest_proposed_plan_markdown: None,
            saw_copy_source_this_turn: false,
            mcp_startup_expected_servers: None,
            mcp_startup_ignore_updates_until_next_start: false,
            mcp_startup_allow_terminal_only_next_round: false,
            mcp_startup_pending_next_round: HashMap::new(),
            mcp_startup_pending_next_round_saw_starting: false,
            connectors_cache: ConnectorsCacheState::default(),
            connectors_partial_snapshot: None,
            connectors_prefetch_in_flight: false,
            connectors_force_refetch_pending: false,
            ide_context: IdeContextState::default(),
            plugins_cache: PluginsCacheState::default(),
            plugins_fetch_state: PluginListFetchState::default(),
            plugin_install_apps_needing_auth: Vec::new(),
            plugin_install_auth_flow: None,
            plugins_active_tab_id: None,
            newly_installed_marketplace_tab_id: None,
            interrupts: InterruptManager::new(),
            reasoning_buffer: String::new(),
            full_reasoning_buffer: String::new(),
            current_status: StatusIndicatorState::working(),
            pending_guardian_review_status: PendingGuardianReviewStatus::default(),
            recent_auto_review_denials: RecentAutoReviewDenials::default(),
            active_hook_cell: None,
            terminal_title_status_kind: TerminalTitleStatusKind::Working,
            retry_status_header: None,
            pending_status_indicator_restore: false,
            suppress_queue_autosend: false,
            thread_id: None,
            dismissed_plan_mode_nudge_scopes: HashSet::new(),
            last_turn_id: None,
            budget_limited_turn_ids: HashSet::new(),
            thread_name: None,
            thread_rename_block_message: None,
            active_side_conversation: false,
            normal_placeholder_text: placeholder,
            side_placeholder_text: side_placeholder,
            branched_from: None,
            interrupted_turn_notice_mode: InterruptedTurnNoticeMode::Default,
            queued_user_messages: VecDeque::new(),
            queued_user_message_history_records: VecDeque::new(),
            user_turn_pending_start: false,
            rejected_steers_queue: VecDeque::new(),
            rejected_steer_history_records: VecDeque::new(),
            pending_steers: VecDeque::new(),
            submit_pending_steers_after_interrupt: false,
            chat_keymap,
            queued_message_edit_hint_binding,
            show_welcome_banner: is_first_run,
            startup_tooltip_override,
            suppress_session_configured_redraw: false,
            suppress_initial_user_message_submit: false,
            pending_notification: None,
            quit_shortcut_expires_at: None,
            quit_shortcut_key: None,
            is_review_mode: false,
            pre_review_token_info: None,
            needs_final_message_separator: false,
            had_work_activity: false,
            saw_plan_update_this_turn: false,
            saw_plan_item_this_turn: false,
            last_plan_progress: None,
            plan_delta_buffer: String::new(),
            plan_item_active: false,
            turn_runtime_metrics: RuntimeMetricsSummary::default(),
            last_rendered_width: std::cell::Cell::new(None),
            style_epoch: std::cell::Cell::new(1),
            transcript_scroll_top: std::cell::Cell::new(0),
            desired_height_cache: std::cell::RefCell::new(height_cache::DesiredHeightCache::default()),
            rendered_lines_cache: std::cell::RefCell::new(height_cache::RenderedLinesCache::default()),
            feedback,
            current_rollout_path: None,
            current_cwd,
            instruction_source_paths: Vec::new(),
            session_network_proxy: None,
            status_line_invalid_items_warned,
            terminal_title_invalid_items_warned,
            last_terminal_title: None,
            last_terminal_title_requires_action: false,
            terminal_title_setup_original_items: None,
            terminal_title_animation_origin: Instant::now(),
            status_line_project_root_name_cache: None,
            status_line_branch: None,
            status_line_branch_cwd: None,
            status_line_branch_pending: false,
            status_line_branch_lookup_complete: false,
            current_goal_status_indicator: None,
            current_goal_status: None,
            goal_status_active_turn_started_at: None,
            external_editor_state: ExternalEditorState::Closed,
            realtime_conversation: RealtimeConversationUiState::default(),
            last_rendered_user_message_display: None,
            last_non_retry_error: None,
        };

        widget.prefetch_rate_limits();
        if let Some(keymap) = runtime_keymap {
            widget.bottom_pane.set_keymap_bindings(&keymap);
        }
        widget
            .bottom_pane
            .set_vim_enabled(widget.config.tui_vim_mode_default);
        widget
            .bottom_pane
            .set_realtime_conversation_enabled(widget.realtime_conversation_enabled());
        widget
            .bottom_pane
            .set_audio_device_selection_enabled(widget.realtime_audio_device_selection_enabled());
        widget
            .bottom_pane
            .set_status_line_enabled(!widget.configured_status_line_items().is_empty());
        widget
            .bottom_pane
            .set_collaboration_modes_enabled(/*enabled*/ true);
        widget.sync_fast_command_enabled();
        widget.sync_personality_command_enabled();
        widget.sync_plugins_command_enabled();
        widget.sync_goal_command_enabled();
        widget
            .bottom_pane
            .set_queued_message_edit_binding(widget.queued_message_edit_hint_binding);
        #[cfg(target_os = "windows")]
        widget.bottom_pane.set_windows_degraded_sandbox_active(
            crate::legacy_core::windows_sandbox::ELEVATED_SANDBOX_NUX_ENABLED
                && matches!(
                    WindowsSandboxLevel::from_config(&widget.config),
                    WindowsSandboxLevel::RestrictedToken
                ),
        );
        widget.update_collaboration_mode_indicator();

        widget
            .bottom_pane
            .set_connectors_enabled(widget.connectors_enabled());
        widget.refresh_status_surfaces();

        widget
    }
