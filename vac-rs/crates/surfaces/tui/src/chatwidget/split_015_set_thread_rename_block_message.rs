    pub(crate) fn set_thread_rename_block_message(&mut self, message: impl Into<String>) {
        self.thread_rename_block_message = Some(message.into());
    }

    pub(crate) fn set_interrupted_turn_notice_mode(&mut self, mode: InterruptedTurnNoticeMode) {
        self.interrupted_turn_notice_mode = mode;
    }

    pub(crate) fn add_diff_in_progress(&mut self) {
        self.request_redraw();
    }

    pub(crate) fn on_diff_complete(&mut self) {
        self.request_redraw();
    }

    pub(crate) fn add_status_output(
        &mut self,
        _refreshing_rate_limits: bool,
        _request_id: Option<u64>,
    ) {
        let default_usage = TokenUsage::default();
        let token_info = self.token_info.as_ref();
        let total_usage = token_info
            .map(|ti| &ti.total_token_usage)
            .unwrap_or(&default_usage);
        let collaboration_mode = self.collaboration_mode_label();
        let model = self.current_model().to_string();
        let model_default_reasoning_effort =
            self.model_catalog
                .try_list_models()
                .ok()
                .and_then(|models| {
                    models
                        .into_iter()
                        .find(|preset| preset.model == model)
                        .map(|preset| preset.default_reasoning_effort)
                });
        let reasoning_effort_override = Some(
            self.effective_reasoning_effort()
                .or(self.config.model_reasoning_effort)
                .or(model_default_reasoning_effort),
        );
        // Hardening 3: /status intentionally ignores cached account limit
        // snapshots. Token/context usage remains visible; limit/credit state is
        // reserved for status-line warnings and policy/nudge flows.
        let rate_limit_snapshots: &[RateLimitSnapshotDisplay] = &[];
        let agents_summary =
            crate::status::compose_agents_summary(&self.config, &self.instruction_source_paths);
        let (cell, _handle) = crate::status::new_status_output_with_rate_limits_handle(
            &self.config,
            self.runtime_model_provider_base_url.as_deref(),
            self.status_account_display.as_ref(),
            token_info,
            total_usage,
            &self.thread_id,
            self.thread_name.clone(),
            self.branched_from,
            rate_limit_snapshots,
            self.plan_type,
            Local::now(),
            self.model_display_name(),
            collaboration_mode,
            reasoning_effort_override,
            agents_summary,
            /*refreshing_rate_limits*/ false,
        );
        self.add_to_history(cell);
    }

    pub(crate) fn finish_status_rate_limit_refresh(&mut self, request_id: u64) {
        if self.refreshing_status_outputs.is_empty() {
            return;
        }

        let rate_limit_snapshots: Vec<RateLimitSnapshotDisplay> = self
            .rate_limit_snapshots_by_limit_id
            .values()
            .cloned()
            .collect();
        let now = Local::now();
        let mut remaining = Vec::with_capacity(self.refreshing_status_outputs.len());
        let mut updated_any = false;
        for (pending_request_id, handle) in self.refreshing_status_outputs.drain(..) {
            if pending_request_id == request_id {
                updated_any = true;
                handle.finish_rate_limit_refresh(rate_limit_snapshots.as_slice(), now);
            } else {
                remaining.push((pending_request_id, handle));
            }
        }
        self.refreshing_status_outputs = remaining;
        if updated_any {
            self.request_redraw();
        }
    }

    pub(crate) fn add_debug_config_output(&mut self) {
        self.add_to_history(crate::debug_config::new_debug_config_output(
            &self.config,
            self.session_network_proxy.as_ref(),
        ));
    }

    pub(crate) fn add_capability_dashboard_output(&mut self) {
        self.add_to_history(crate::capability_dashboard::new_capability_dashboard_output(
            &self.config,
        ));
        self.bottom_pane
            .show_selection_view(crate::capability_dashboard::new_capability_dashboard_view(&self.config));
        self.request_redraw();
    }

    pub(crate) fn add_runtime_jobs_output(&mut self) {
        let view = Box::new(crate::operator_console::new_runtime_operator_console_view(&self.config));
        if !self
            .bottom_pane
            .replace_active_view_if_active("operator-console", view)
        {
            self.bottom_pane.show_view(Box::new(
                crate::operator_console::new_runtime_operator_console_view(&self.config),
            ));
        }
        self.request_redraw();
    }

    pub(crate) fn add_workflow_browser_output(&mut self) {
        self.add_workflow_browser_output_with_state(
            crate::workflow_browser::WorkflowBrowserState::default(),
        );
    }

    pub(crate) fn add_workflow_browser_output_with_state(
        &mut self,
        state: crate::workflow_browser::WorkflowBrowserState,
    ) {
        let view = Box::new(crate::workflow_browser::new_workflow_browser_view(
            &self.config,
            state.clone(),
        ));
        if !self
            .bottom_pane
            .replace_active_view_if_active("workflow-browser", view)
        {
            self.bottom_pane.show_view(Box::new(
                crate::workflow_browser::new_workflow_browser_view(&self.config, state),
            ));
        }
        self.request_redraw();
    }

    pub(crate) fn add_ps_output(&mut self) {
        let processes = self
            .unified_exec_processes
            .iter()
            .map(|process| history_cell::UnifiedExecProcessDetails {
                command_display: process.command_display.clone(),
                recent_chunks: process.recent_chunks.clone(),
            })
            .collect();
        self.add_to_history(history_cell::new_unified_exec_processes_output(processes));
    }

    fn clean_background_terminals(&mut self) {
        self.submit_op(AppCommand::clean_background_terminals());
        self.unified_exec_processes.clear();
        self.sync_unified_exec_footer();
        self.add_info_message(
            "Stopping all background terminals.".to_string(),
            /*hint*/ None,
        );
    }

    fn stop_rate_limit_poller(&mut self) {}

    pub(crate) fn refresh_connectors(&mut self, force_refetch: bool) {
        self.prefetch_connectors_with_options(force_refetch);
    }

    fn prefetch_connectors(&mut self) {
        self.prefetch_connectors_with_options(/*force_refetch*/ false);
    }

    fn prefetch_connectors_with_options(&mut self, force_refetch: bool) {
        if !self.connectors_enabled() {
            return;
        }
        if self.connectors_prefetch_in_flight {
            if force_refetch {
                self.connectors_force_refetch_pending = true;
            }
            return;
        }

        self.connectors_prefetch_in_flight = true;
        if !matches!(self.connectors_cache, ConnectorsCacheState::Ready(_)) {
            self.connectors_cache = ConnectorsCacheState::Loading;
        }

        let config = self.config.clone();
        let app_event_tx = self.app_event_tx.clone();
        tokio::spawn(async move {
            let accessible_result =
                match connectors::list_accessible_connectors_from_mcp_tools_with_options_and_status(
                    &config,
                    force_refetch,
                )
                .await
                {
                    Ok(connectors) => connectors,
                    Err(err) => {
                        app_event_tx.send(AppEvent::ConnectorsLoaded {
                            result: Err(format!("Failed to load apps: {err}")),
                            is_final: true,
                        });
                        return;
                    }
                };
            let should_schedule_force_refetch = !force_refetch && !accessible_result.vac_apps_ready;
            let accessible_connectors = accessible_result.connectors;

            app_event_tx.send(AppEvent::ConnectorsLoaded {
                result: Ok(ConnectorsSnapshot {
                    connectors: accessible_connectors.clone(),
                }),
                is_final: false,
            });

            let result: Result<ConnectorsSnapshot, String> = async {
                let all_connectors =
                    connectors::list_all_connectors_with_options(&config, force_refetch).await?;
                let connectors = connectors::merge_connectors_with_accessible(
                    all_connectors,
                    accessible_connectors,
                    /*all_connectors_loaded*/ true,
                );
                Ok(ConnectorsSnapshot { connectors })
            }
            .await
            .map_err(|err: anyhow::Error| format!("Failed to load apps: {err}"));

            app_event_tx.send(AppEvent::ConnectorsLoaded {
                result,
                is_final: true,
            });

            if should_schedule_force_refetch {
                app_event_tx.send(AppEvent::RefreshConnectors {
                    force_refetch: true,
                });
            }
        });
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn prefetch_rate_limits(&mut self) {
        self.stop_rate_limit_poller();
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn should_prefetch_rate_limits(&self) -> bool {
        self.config.model_provider.requires_vastar_auth && self.has_chatgpt_account
    }

    fn lower_cost_preset(&self) -> Option<ModelPreset> {
        let models = self.model_catalog.try_list_models().ok()?;
        models
            .iter()
            .find(|preset| preset.show_in_picker && preset.model == NUDGE_MODEL_SLUG)
            .cloned()
    }

    fn rate_limit_switch_prompt_hidden(&self) -> bool {
        self.config
            .notices
            .hide_rate_limit_model_nudge
            .unwrap_or(false)
    }

    fn maybe_show_pending_rate_limit_prompt(&mut self) {
        if self.rate_limit_switch_prompt_hidden() {
            self.rate_limit_switch_prompt = RateLimitSwitchPromptState::Idle;
            return;
        }
        if !matches!(
            self.rate_limit_switch_prompt,
            RateLimitSwitchPromptState::Pending
        ) {
            return;
        }
        if let Some(preset) = self.lower_cost_preset() {
            self.open_rate_limit_switch_prompt(preset);
            self.rate_limit_switch_prompt = RateLimitSwitchPromptState::Shown;
        } else {
            self.rate_limit_switch_prompt = RateLimitSwitchPromptState::Idle;
        }
    }

    fn open_rate_limit_switch_prompt(&mut self, preset: ModelPreset) {
        let switch_model = preset.model;
        let switch_model_for_events = switch_model.clone();
        let default_effort: ReasoningEffortConfig = preset.default_reasoning_effort;

        let switch_actions: Vec<SelectionAction> = vec![Box::new(move |tx| {
            tx.send(AppEvent::VACOp(AppCommand::override_turn_context(
                /*cwd*/ None,
                /*approval_policy*/ None,
                /*approvals_reviewer*/ None,
                /*permission_profile*/ None,
                /*windows_sandbox_level*/ None,
                Some(switch_model_for_events.clone()),
                Some(Some(default_effort)),
                /*summary*/ None,
                /*service_tier*/ None,
                /*collaboration_mode*/ None,
                /*personality*/ None,
            )));
            tx.send(AppEvent::UpdateModel(switch_model_for_events.clone()));
            tx.send(AppEvent::UpdateReasoningEffort(Some(default_effort)));
        })];

        let keep_actions: Vec<SelectionAction> = Vec::new();
        let never_actions: Vec<SelectionAction> = vec![Box::new(|tx| {
            tx.send(AppEvent::UpdateRateLimitSwitchPromptHidden(true));
            tx.send(AppEvent::PersistRateLimitSwitchPromptHidden);
        })];
        let description = if preset.description.is_empty() {
            Some("Uses fewer credits for upcoming turns.".to_string())
        } else {
            Some(preset.description)
        };

        let items = vec![
            SelectionItem {
                name: format!("Switch to {switch_model}"),
                description,
                selected_description: None,
                is_current: false,
                actions: switch_actions,
                dismiss_on_select: true,
                ..Default::default()
            },
            SelectionItem {
                name: "Keep current model".to_string(),
                description: None,
                selected_description: None,
                is_current: false,
                actions: keep_actions,
                dismiss_on_select: true,
                ..Default::default()
            },
            SelectionItem {
                name: "Keep current model (never show again)".to_string(),
                description: Some(
                    "Hide future rate limit reminders about switching models.".to_string(),
                ),
                selected_description: None,
                is_current: false,
                actions: never_actions,
                dismiss_on_select: true,
                ..Default::default()
            },
        ];

        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some("Approaching rate limits".to_string()),
            subtitle: Some(format!("Switch to {switch_model} for lower credit usage?")),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            ..Default::default()
        });
    }

    fn open_workspace_owner_nudge_prompt(&mut self, credit_type: AddCreditsNudgeCreditType) {
        if !self.workspace_owner_usage_nudge_enabled()
            || self.add_credits_nudge_email_in_flight.is_some()
        {
            return;
        }

        let (title, prompt) = match credit_type {
            AddCreditsNudgeCreditType::Credits => (
                "You've reached your workspace credit limit",
                "Your workspace is out of credits. Ask your workspace owner to add more. Notify owner?",
            ),
            AddCreditsNudgeCreditType::UsageLimit => (
                "Usage limit reached",
                "Request a limit increase from your owner to continue using vac. Request increase?",
            ),
        };
        let send_actions: Vec<SelectionAction> = vec![Box::new(move |tx| {
            tx.send(AppEvent::SendAddCreditsNudgeEmail { credit_type });
        })];
        let items = vec![
            SelectionItem {
                name: "Yes".to_string(),
                display_shortcut: Some(key_hint::plain(KeyCode::Char('y'))),
                actions: send_actions,
                dismiss_on_select: true,
                ..Default::default()
            },
            SelectionItem {
                name: "No".to_string(),
                display_shortcut: Some(key_hint::plain(KeyCode::Char('n'))),
                is_default: true,
                dismiss_on_select: true,
                ..Default::default()
            },
        ];

        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some(title.to_string()),
            subtitle: Some(prompt.to_string()),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            initial_selected_idx: Some(1),
            ..Default::default()
        });
    }

    pub(crate) fn start_add_credits_nudge_email_request(
        &mut self,
        credit_type: AddCreditsNudgeCreditType,
    ) -> bool {
        if !self.workspace_owner_usage_nudge_enabled() {
            return false;
        }

        self.add_credits_nudge_email_in_flight = Some(credit_type);
        true
    }
