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
