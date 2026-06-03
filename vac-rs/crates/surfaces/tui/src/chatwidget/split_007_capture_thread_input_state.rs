    pub(crate) fn capture_thread_input_state(&self) -> Option<ThreadInputState> {
        let composer = ThreadComposerState {
            text: self.bottom_pane.composer_text(),
            text_elements: self.bottom_pane.composer_text_elements(),
            local_images: self.bottom_pane.composer_local_images(),
            remote_image_urls: self.bottom_pane.remote_image_urls(),
            mention_bindings: self.bottom_pane.composer_mention_bindings(),
            pending_pastes: self.bottom_pane.composer_pending_pastes(),
        };
        Some(ThreadInputState {
            composer: composer.has_content().then_some(composer),
            pending_steers: self
                .pending_steers
                .iter()
                .map(|pending| pending.user_message.clone())
                .collect(),
            pending_steer_history_records: self
                .pending_steers
                .iter()
                .map(|pending| pending.history_record.clone())
                .collect(),
            pending_steer_compare_keys: self
                .pending_steers
                .iter()
                .map(|pending| pending.compare_key.clone())
                .collect(),
            rejected_steers_queue: self.rejected_steers_queue.clone(),
            rejected_steer_history_records: self.rejected_steer_history_records.clone(),
            queued_user_messages: self.queued_user_messages.clone(),
            queued_user_message_history_records: self.queued_user_message_history_records.clone(),
            user_turn_pending_start: self.user_turn_pending_start,
            current_collaboration_mode: self.current_collaboration_mode.clone(),
            active_collaboration_mask: self.active_collaboration_mask.clone(),
            task_running: self.bottom_pane.is_task_running(),
            agent_turn_running: self.agent_turn_running,
        })
    }

    pub(crate) fn restore_thread_input_state(&mut self, input_state: Option<ThreadInputState>) {
        let restored_task_running = input_state.as_ref().is_some_and(|state| state.task_running);
        if let Some(input_state) = input_state {
            self.current_collaboration_mode = input_state.current_collaboration_mode;
            self.active_collaboration_mask = input_state.active_collaboration_mask;
            self.agent_turn_running = input_state.agent_turn_running;
            self.goal_status_active_turn_started_at =
                self.agent_turn_running.then_some(Instant::now());
            self.user_turn_pending_start = input_state.user_turn_pending_start;
            self.update_collaboration_mode_indicator();
            self.refresh_model_dependent_surfaces();
            if let Some(composer) = input_state.composer {
                let local_image_paths = composer
                    .local_images
                    .into_iter()
                    .map(|img| img.path)
                    .collect();
                self.set_remote_image_urls(composer.remote_image_urls);
                self.bottom_pane.set_composer_text_with_mention_bindings(
                    composer.text,
                    composer.text_elements,
                    local_image_paths,
                    composer.mention_bindings,
                );
                self.bottom_pane
                    .set_composer_pending_pastes(composer.pending_pastes);
            } else {
                self.set_remote_image_urls(Vec::new());
                self.bottom_pane.set_composer_text_with_mention_bindings(
                    String::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                );
                self.bottom_pane.set_composer_pending_pastes(Vec::new());
            }
            let mut pending_steer_history_records = input_state.pending_steer_history_records;
            pending_steer_history_records.resize(
                input_state.pending_steers.len(),
                UserMessageHistoryRecord::UserMessageText,
            );
            let mut pending_steer_compare_keys = input_state.pending_steer_compare_keys;
            self.pending_steers = input_state
                .pending_steers
                .into_iter()
                .zip(pending_steer_history_records)
                .map(|(user_message, history_record)| PendingSteer {
                    compare_key: pending_steer_compare_keys.pop_front().unwrap_or_else(|| {
                        PendingSteerCompareKey {
                            message: user_message.text.clone(),
                            image_count: user_message.local_images.len()
                                + user_message.remote_image_urls.len(),
                        }
                    }),
                    history_record,
                    user_message,
                })
                .collect();
            self.rejected_steers_queue = input_state.rejected_steers_queue;
            self.rejected_steer_history_records = input_state.rejected_steer_history_records;
            self.rejected_steer_history_records.resize(
                self.rejected_steers_queue.len(),
                UserMessageHistoryRecord::UserMessageText,
            );
            self.queued_user_messages = input_state.queued_user_messages;
            self.queued_user_message_history_records =
                input_state.queued_user_message_history_records;
            self.queued_user_message_history_records.resize(
                self.queued_user_messages.len(),
                UserMessageHistoryRecord::UserMessageText,
            );
        } else {
            self.agent_turn_running = false;
            self.goal_status_active_turn_started_at = None;
            self.user_turn_pending_start = false;
            self.pending_steers.clear();
            self.rejected_steers_queue.clear();
            self.rejected_steer_history_records.clear();
            self.set_remote_image_urls(Vec::new());
            self.bottom_pane.set_composer_text_with_mention_bindings(
                String::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            );
            self.bottom_pane.set_composer_pending_pastes(Vec::new());
            self.queued_user_messages.clear();
            self.queued_user_message_history_records.clear();
        }
        self.turn_sleep_inhibitor
            .set_turn_running(self.agent_turn_running);
        self.update_task_running_state();
        if restored_task_running && !self.bottom_pane.is_task_running() {
            self.bottom_pane.set_task_running(/*running*/ true);
            self.refresh_status_surfaces();
        }
        self.refresh_pending_input_preview();
        self.request_redraw();
    }

    pub(crate) fn set_queue_autosend_suppressed(&mut self, suppressed: bool) {
        self.suppress_queue_autosend = suppressed;
    }

    fn on_plan_update(&mut self, update: UpdatePlanArgs) {
        self.saw_plan_update_this_turn = true;
        let total = update.plan.len();
        let completed = update
            .plan
            .iter()
            .filter(|item| match &item.status {
                StepStatus::Completed => true,
                StepStatus::Pending | StepStatus::InProgress => false,
            })
            .count();
        self.last_plan_progress = (total > 0).then_some((completed, total));
        self.refresh_status_surfaces();
        self.add_to_history(history_cell::new_plan_update(update));
    }

    fn on_exec_approval_request(&mut self, _id: String, ev: ExecApprovalRequestEvent) {
        let ev2 = ev.clone();
        self.defer_or_handle(
            |q| q.push_exec_approval(ev),
            |s| s.handle_exec_approval_now(ev2),
        );
    }

    fn on_apply_patch_approval_request(&mut self, _id: String, ev: ApplyPatchApprovalRequestEvent) {
        let ev2 = ev.clone();
        self.defer_or_handle(
            |q| q.push_apply_patch_approval(ev),
            |s| s.handle_apply_patch_approval_now(ev2),
        );
    }

    /// Handle guardian review lifecycle events for the current thread.
    ///
    /// In-progress assessments temporarily own the live status footer so the
    /// user can see what is being reviewed, including parallel review
    /// aggregation. Terminal assessments clear or update that footer state and
    /// render the final approved/denied history cell when guardian returns a
    /// decision.
    fn on_guardian_assessment(&mut self, ev: GuardianAssessmentEvent) {
        let permission_request_summary = |subject: &str, reason: &Option<String>| {
            reason
                .as_deref()
                .map(str::trim)
                .filter(|reason| !reason.is_empty())
                .map(|reason| format!("{subject}: {reason}"))
                .unwrap_or_else(|| subject.to_string())
        };
        let guardian_action_summary = |action: &GuardianAssessmentAction| match action {
            GuardianAssessmentAction::Command { command, .. } => Some(command.clone()),
            GuardianAssessmentAction::Execve { program, argv, .. } => {
                let command = if argv.is_empty() {
                    vec![program.clone()]
                } else {
                    argv.clone()
                };
                shlex::try_join(command.iter().map(String::as_str))
                    .ok()
                    .or_else(|| Some(command.join(" ")))
            }
            GuardianAssessmentAction::ApplyPatch { files, .. } => Some(if files.len() == 1 {
                format!("apply_patch touching {}", files[0].display())
            } else {
                format!("apply_patch touching {} files", files.len())
            }),
            GuardianAssessmentAction::NetworkAccess { target, .. } => {
                Some(format!("network access to {target}"))
            }
            GuardianAssessmentAction::McpToolCall {
                server,
                tool_name,
                connector_name,
                ..
            } => {
                let label = connector_name.as_deref().unwrap_or(server.as_str());
                Some(format!("MCP {tool_name} on {label}"))
            }
            GuardianAssessmentAction::RequestPermissions { reason, .. } => {
                Some(permission_request_summary("permission request", reason))
            }
        };
        let guardian_command = |action: &GuardianAssessmentAction| match action {
            GuardianAssessmentAction::Command { command, .. } => shlex::split(command)
                .filter(|command| !command.is_empty())
                .or_else(|| Some(vec![command.clone()])),
            GuardianAssessmentAction::Execve { program, argv, .. } => Some(if argv.is_empty() {
                vec![program.clone()]
            } else {
                argv.clone()
            })
            .filter(|command| !command.is_empty()),
            GuardianAssessmentAction::ApplyPatch { .. }
            | GuardianAssessmentAction::NetworkAccess { .. }
            | GuardianAssessmentAction::McpToolCall { .. }
            | GuardianAssessmentAction::RequestPermissions { .. } => None,
        };

        if ev.status == GuardianAssessmentStatus::InProgress
            && let Some(detail) = guardian_action_summary(&ev.action)
        {
            // In-progress assessments own the live footer state while the
            // review is pending. Parallel reviews are aggregated into one
            // footer summary by `PendingGuardianReviewStatus`.
            self.bottom_pane.ensure_status_indicator();
            self.bottom_pane
                .set_interrupt_hint_visible(/*visible*/ true);
            self.pending_guardian_review_status
                .start_or_update(ev.id.clone(), detail);
            if let Some(status) = self.pending_guardian_review_status.status_indicator_state() {
                self.set_status(
                    status.header,
                    status.details,
                    StatusDetailsCapitalization::Preserve,
                    status.details_max_lines,
                );
            }
            self.request_redraw();
            return;
        }

        // Terminal assessments remove the matching pending footer entry first,
        // then render the final approved/denied history cell below.
        if self.pending_guardian_review_status.finish(&ev.id) {
            if let Some(status) = self.pending_guardian_review_status.status_indicator_state() {
                self.set_status(
                    status.header,
                    status.details,
                    StatusDetailsCapitalization::Preserve,
                    status.details_max_lines,
                );
            } else if self.current_status.is_guardian_review() {
                self.set_status_header(String::from("Working"));
            }
        } else if self.pending_guardian_review_status.is_empty()
            && self.current_status.is_guardian_review()
        {
            self.set_status_header(String::from("Working"));
        }

        if ev.status == GuardianAssessmentStatus::Approved {
            let cell = if let Some(command) = guardian_command(&ev.action) {
                history_cell::new_approval_decision_cell(
                    command,
                    crate::history_cell::ReviewDecision::Approved,
                    history_cell::ApprovalDecisionActor::Guardian,
                )
            } else if let Some(summary) = guardian_action_summary(&ev.action) {
                history_cell::new_guardian_approved_action_request(summary)
            } else {
                let summary = serde_json::to_string(&ev.action)
                    .unwrap_or_else(|_| "<unrenderable guardian action>".to_string());
                history_cell::new_guardian_approved_action_request(summary)
            };

            self.add_boxed_history(cell);
            self.request_redraw();
            return;
        }

        if ev.status == GuardianAssessmentStatus::TimedOut {
            let cell = if let Some(command) = guardian_command(&ev.action) {
                history_cell::new_approval_decision_cell(
                    command,
                    crate::history_cell::ReviewDecision::TimedOut,
                    history_cell::ApprovalDecisionActor::Guardian,
                )
            } else {
                match &ev.action {
                    GuardianAssessmentAction::ApplyPatch { files, .. } => {
                        let files = files
                            .iter()
                            .map(|path| path.display().to_string())
                            .collect::<Vec<_>>();
                        history_cell::new_guardian_timed_out_patch_request(files)
                    }
                    GuardianAssessmentAction::McpToolCall {
                        server, tool_name, ..
                    } => history_cell::new_guardian_timed_out_action_request(format!(
                        "vac could call MCP tool {server}.{tool_name}"
                    )),
                    GuardianAssessmentAction::NetworkAccess { target, .. } => {
                        history_cell::new_guardian_timed_out_action_request(format!(
                            "vac could access {target}"
                        ))
                    }
                    GuardianAssessmentAction::RequestPermissions { reason, .. } => {
                        history_cell::new_guardian_timed_out_action_request(
                            permission_request_summary("vac could request permissions", reason),
                        )
                    }
                    GuardianAssessmentAction::Command { .. } => unreachable!(),
                    GuardianAssessmentAction::Execve { .. } => unreachable!(),
                }
            };

            self.add_boxed_history(cell);
            self.request_redraw();
            return;
        }

        if ev.status != GuardianAssessmentStatus::Denied {
            return;
        }
        self.recent_auto_review_denials.push(ev.clone());
        let cell = if let Some(command) = guardian_command(&ev.action) {
            history_cell::new_approval_decision_cell(
                command,
                crate::history_cell::ReviewDecision::Denied,
                history_cell::ApprovalDecisionActor::Guardian,
            )
        } else {
            match &ev.action {
                GuardianAssessmentAction::ApplyPatch { files, .. } => {
                    let files = files
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect::<Vec<_>>();
                    history_cell::new_guardian_denied_patch_request(files)
                }
                GuardianAssessmentAction::McpToolCall {
                    server, tool_name, ..
                } => history_cell::new_guardian_denied_action_request(format!(
                    "vac to call MCP tool {server}.{tool_name}"
                )),
                GuardianAssessmentAction::NetworkAccess { target, .. } => {
                    history_cell::new_guardian_denied_action_request(format!(
                        "vac to access {target}"
                    ))
                }
                GuardianAssessmentAction::RequestPermissions { reason, .. } => {
                    history_cell::new_guardian_denied_action_request(permission_request_summary(
                        "vac to request permissions",
                        reason,
                    ))
                }
                GuardianAssessmentAction::Command { .. } => unreachable!(),
                GuardianAssessmentAction::Execve { .. } => unreachable!(),
            }
        };

        self.add_boxed_history(cell);
        self.request_redraw();
    }

    fn on_elicitation_request(
        &mut self,
        request_id: AppServerRequestId,
        params: McpServerElicitationRequestParams,
    ) {
        let request_id2 = request_id.clone();
        let params2 = params.clone();
        self.defer_or_handle(
            |q| q.push_elicitation(request_id, params),
            |s| s.handle_elicitation_request_now(request_id2, params2),
        );
    }

    fn on_request_user_input(&mut self, ev: ToolRequestUserInputParams) {
        let ev2 = ev.clone();
        self.defer_or_handle(
            |q| q.push_user_input(ev),
            |s| s.handle_request_user_input_now(ev2),
        );
    }

    fn on_request_permissions(&mut self, ev: RequestPermissionsEvent) {
        let ev2 = ev.clone();
        self.defer_or_handle(
            |q| q.push_request_permissions(ev),
            |s| s.handle_request_permissions_now(ev2),
        );
    }

    fn on_command_execution_started(&mut self, item: ThreadItem) {
        let ThreadItem::CommandExecution {
            id,
            command,
            process_id,
            source,
            command_actions,
            ..
        } = &item
        else {
            return;
        };
        let (_command, parsed_cmd) = command_execution_command_and_parsed(command, command_actions);
        self.flush_answer_stream_with_separator();
        if is_unified_exec_source(*source) {
            if *source == ExecCommandSource::UnifiedExecStartup {
                self.track_unified_exec_process_begin(id, process_id.as_deref(), command);
            }
            if !self.bottom_pane.is_task_running() {
                return;
            }
            // Unified exec may be parsed as Unknown; keep the working indicator visible regardless.
            self.bottom_pane.ensure_status_indicator();
            if !is_standard_tool_call(&parsed_cmd) {
                return;
            }
        }
        let item2 = item.clone();
        self.defer_or_handle(
            |q| q.push_item_started(item),
            |s| s.handle_command_execution_started_now(item2),
        );
    }

