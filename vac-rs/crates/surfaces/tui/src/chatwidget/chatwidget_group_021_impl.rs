// O5/O6 balanced ChatWidget impl group: source split_021_sync_image_paste_enabled.rs, split_022_clear_mcp_inventory_loading.rs
impl ChatWidget {
    fn sync_image_paste_enabled(&mut self) {
        let enabled = self.current_model_supports_images();
        self.bottom_pane.set_image_paste_enabled(enabled);
    }

    fn image_inputs_not_supported_message(&self) -> String {
        format!(
            "Model {} does not support image inputs. Remove images or switch models.",
            self.current_model()
        )
    }

    #[allow(dead_code)] // Used in tests
    pub(crate) fn current_collaboration_mode(&self) -> &CollaborationMode {
        &self.current_collaboration_mode
    }

    pub(crate) fn current_reasoning_effort(&self) -> Option<ReasoningEffortConfig> {
        self.effective_reasoning_effort()
    }

    #[cfg(test)]
    pub(crate) fn active_collaboration_mode_kind(&self) -> ModeKind {
        self.active_mode_kind()
    }

    fn is_session_configured(&self) -> bool {
        self.thread_id.is_some()
    }

    fn collaboration_modes_enabled(&self) -> bool {
        true
    }

    /// Returns the dismissal scope that applies to the currently visible draft.
    fn plan_mode_nudge_scope(&self) -> PlanModeNudgeScope {
        self.thread_id
            .map_or(PlanModeNudgeScope::NewThread, PlanModeNudgeScope::Thread)
    }

    /// Returns whether the current draft should replace the normal footer with the Plan-mode nudge.
    ///
    /// `ChatWidget` owns this policy because it can combine lexical draft matching with mode
    /// availability, interaction state, and thread-scoped dismissal. `ChatComposer` only renders
    /// the resulting visibility bit. Keeping slash and shell drafts out here avoids advertising a
    /// mode switch while the user is intentionally composing another local command.
    fn should_show_plan_mode_nudge(&self) -> bool {
        let text = self.bottom_pane.composer_text();
        let trimmed = text.trim_start();
        self.collaboration_modes_enabled()
            && collaboration_modes::plan_mask(self.model_catalog.as_ref()).is_some()
            && self.active_mode_kind() != ModeKind::Plan
            && self.bottom_pane.composer_input_enabled()
            && !self.bottom_pane.is_task_running()
            && self.bottom_pane.no_modal_or_popup_active()
            && !trimmed.starts_with('/')
            && !trimmed.starts_with('!')
            && contains_plan_keyword(&text)
            && !self
                .dismissed_plan_mode_nudge_scopes
                .contains(&self.plan_mode_nudge_scope())
    }

    /// Synchronizes the footer presentation with the current Plan-mode nudge policy.
    fn refresh_plan_mode_nudge(&mut self) {
        self.bottom_pane
            .set_plan_mode_nudge_visible(self.should_show_plan_mode_nudge());
    }

    /// Hides the nudge for the current thread scope until the user changes conversation context.
    fn dismiss_plan_mode_nudge(&mut self) {
        self.dismissed_plan_mode_nudge_scopes
            .insert(self.plan_mode_nudge_scope());
        self.refresh_plan_mode_nudge();
    }

    fn initial_collaboration_mask(
        _config: &Config,
        model_catalog: &ModelCatalog,
        model_override: Option<&str>,
    ) -> Option<CollaborationModeMask> {
        let mut mask = collaboration_modes::default_mask(model_catalog)?;
        if let Some(model_override) = model_override {
            mask.model = Some(model_override.to_string());
        }
        Some(mask)
    }

    fn active_mode_kind(&self) -> ModeKind {
        self.active_collaboration_mask
            .as_ref()
            .and_then(|mask| mask.mode)
            .unwrap_or(ModeKind::Default)
    }

    fn effective_reasoning_effort(&self) -> Option<ReasoningEffortConfig> {
        if !self.collaboration_modes_enabled() {
            return self.current_collaboration_mode.reasoning_effort();
        }
        let current_effort = self.current_collaboration_mode.reasoning_effort();
        self.active_collaboration_mask
            .as_ref()
            .and_then(|mask| mask.reasoning_effort)
            .unwrap_or(current_effort)
    }

    fn effective_collaboration_mode(&self) -> CollaborationMode {
        if !self.collaboration_modes_enabled() {
            return self.current_collaboration_mode.clone();
        }
        self.active_collaboration_mask.as_ref().map_or_else(
            || self.current_collaboration_mode.clone(),
            |mask| self.current_collaboration_mode.apply_mask(mask),
        )
    }

    fn refresh_model_display(&mut self) {
        let effective = self.effective_collaboration_mode();
        self.session_header.set_model(effective.model());
        // Keep composer paste affordances aligned with the currently effective model.
        self.sync_image_paste_enabled();
        self.refresh_terminal_title();
    }

    /// Refresh every UI surface that depends on the effective model, reasoning
    /// effort, or collaboration mode.
    ///
    /// Call this at the end of any setter that mutates `current_collaboration_mode`,
    /// `active_collaboration_mask`, or per-mode reasoning-effort overrides.
    /// Consolidating both refreshes here prevents the bug where callers update the
    /// header/title (`refresh_model_display`) but forget the footer status line
    /// (`refresh_status_line`).
    fn refresh_model_dependent_surfaces(&mut self) {
        self.refresh_model_display();
        self.refresh_status_line();
    }

    fn model_display_name(&self) -> &str {
        let model = self.current_model();
        if model.is_empty() {
            DEFAULT_MODEL_DISPLAY_NAME
        } else {
            model
        }
    }

    /// Get the label for the current collaboration mode.
    fn collaboration_mode_label(&self) -> Option<&'static str> {
        if !self.collaboration_modes_enabled() {
            return None;
        }
        let active_mode = self.active_mode_kind();
        active_mode
            .is_tui_visible()
            .then_some(active_mode.display_name())
    }

    fn collaboration_mode_indicator(&self) -> Option<CollaborationModeIndicator> {
        if !self.collaboration_modes_enabled() {
            return None;
        }
        match self.active_mode_kind() {
            ModeKind::Plan => Some(CollaborationModeIndicator::Plan),
            ModeKind::Default | ModeKind::PairProgramming | ModeKind::Execute => None,
        }
    }

    fn update_collaboration_mode_indicator(&mut self) {
        let indicator = self.collaboration_mode_indicator();
        let goal_indicator = if indicator.is_none() {
            self.goal_status_indicator(Instant::now())
        } else {
            None
        };
        self.current_goal_status_indicator = goal_indicator.clone();
        self.bottom_pane.set_collaboration_mode_indicator(indicator);
        self.bottom_pane.set_goal_status_indicator(goal_indicator);
    }

    fn refresh_goal_status_indicator_for_time_tick(&mut self) {
        if self.collaboration_mode_indicator().is_some() {
            return;
        }
        let goal_indicator = self.goal_status_indicator(Instant::now());
        if goal_indicator != self.current_goal_status_indicator {
            self.current_goal_status_indicator = goal_indicator.clone();
            self.bottom_pane.set_goal_status_indicator(goal_indicator);
        }
    }

    fn goal_status_indicator(&self, now: Instant) -> Option<GoalStatusIndicator> {
        if !self.config.features.enabled(Feature::Goals) {
            return None;
        }
        self.current_goal_status
            .as_ref()
            .and_then(|state| state.indicator(now, self.goal_status_active_turn_started_at))
    }

    fn on_thread_goal_updated(&mut self, goal: AppThreadGoal, turn_id: Option<String>) {
        if let Some(active_thread_id) = self.thread_id
            && active_thread_id != goal.thread_id
        {
            return;
        }
        if !self.config.features.enabled(Feature::Goals) {
            self.current_goal_status_indicator = None;
            self.current_goal_status = None;
            self.update_collaboration_mode_indicator();
            return;
        }
        if goal.status == AppThreadGoalStatus::BudgetLimited
            && let Some(turn_id) = turn_id
        {
            self.budget_limited_turn_ids.insert(turn_id);
        }
        self.current_goal_status = Some(GoalStatusState::new(goal, Instant::now()));
        self.update_collaboration_mode_indicator();
    }

    fn personality_label(personality: Personality) -> &'static str {
        match personality {
            Personality::None => "None",
            Personality::Friendly => "Friendly",
            Personality::Pragmatic => "Pragmatic",
        }
    }

    fn personality_description(personality: Personality) -> &'static str {
        match personality {
            Personality::None => "No personality instructions.",
            Personality::Friendly => "Warm, collaborative, and helpful.",
            Personality::Pragmatic => "Concise, task-focused, and direct.",
        }
    }

    /// Cycle to the next collaboration mode variant (Plan -> Default -> Plan).
    fn cycle_collaboration_mode(&mut self) {
        if !self.collaboration_modes_enabled() {
            return;
        }

        if let Some(next_mask) = collaboration_modes::next_mask(
            self.model_catalog.as_ref(),
            self.active_collaboration_mask.as_ref(),
        ) {
            self.set_collaboration_mask(next_mask);
        }
    }

    /// Update the active collaboration mask.
    ///
    /// When collaboration modes are enabled and a preset is selected,
    /// the current mode is attached to submissions as `Op::UserTurn { collaboration_mode: Some(...) }`.
    pub(crate) fn set_collaboration_mask(&mut self, mut mask: CollaborationModeMask) {
        if !self.collaboration_modes_enabled() {
            return;
        }
        let previous_mode = self.active_mode_kind();
        let previous_model = self.current_model().to_string();
        let previous_effort = self.effective_reasoning_effort();
        if mask.mode == Some(ModeKind::Plan)
            && let Some(effort) = self.config.plan_mode_reasoning_effort
        {
            mask.reasoning_effort = Some(Some(effort));
        }
        if mask.mode == Some(ModeKind::Plan) {
            self.dismissed_plan_mode_nudge_scopes
                .insert(self.plan_mode_nudge_scope());
        }
        self.active_collaboration_mask = Some(mask);
        self.update_collaboration_mode_indicator();
        self.refresh_plan_mode_nudge();
        self.refresh_model_dependent_surfaces();
        let next_mode = self.active_mode_kind();
        let next_model = self.current_model();
        let next_effort = self.effective_reasoning_effort();
        if previous_mode != next_mode
            && (previous_model != next_model || previous_effort != next_effort)
        {
            let mut message = format!("Model changed to {next_model}");
            if !next_model.starts_with("vac-auto-") {
                let reasoning_label = match next_effort {
                    Some(ReasoningEffortConfig::Minimal) => "minimal",
                    Some(ReasoningEffortConfig::Low) => "low",
                    Some(ReasoningEffortConfig::Medium) => "medium",
                    Some(ReasoningEffortConfig::High) => "high",
                    Some(ReasoningEffortConfig::XHigh) => "xhigh",
                    None | Some(ReasoningEffortConfig::None) => "default",
                };
                message.push(' ');
                message.push_str(reasoning_label);
            }
            message.push_str(" for ");
            message.push_str(next_mode.display_name());
            message.push_str(" mode.");
            self.add_info_message(message, /*hint*/ None);
        }
        self.request_redraw();
    }

    fn connectors_enabled(&self) -> bool {
        self.config.features.enabled(Feature::Apps) && self.has_chatgpt_account
    }

    fn connectors_for_mentions(&self) -> Option<&[AppInfo]> {
        if !self.connectors_enabled() {
            return None;
        }

        if let Some(snapshot) = &self.connectors_partial_snapshot {
            return Some(snapshot.connectors.as_slice());
        }

        match &self.connectors_cache {
            ConnectorsCacheState::Ready(snapshot) => Some(snapshot.connectors.as_slice()),
            _ => None,
        }
    }

    fn plugins_for_mentions(&self) -> Option<&[PluginCapabilitySummary]> {
        if !self.config.features.enabled(Feature::Plugins) {
            return None;
        }

        self.bottom_pane.plugins().map(Vec::as_slice)
    }

    /// Build a placeholder header cell while the session is configuring.
    fn placeholder_session_header_cell(config: &Config) -> Box<dyn HistoryCell> {
        let placeholder_style = Style::default().add_modifier(Modifier::DIM | Modifier::ITALIC);
        Box::new(
            history_cell::SessionHeaderHistoryCell::new_with_style(
                DEFAULT_MODEL_DISPLAY_NAME.to_string(),
                placeholder_style,
                /*reasoning_effort*/ None,
                /*show_fast_status*/ false,
                config.cwd.to_path_buf(),
                VAC_CLI_VERSION,
            )
            .with_operator_context(
                history_cell::operator_profile_label(config),
                history_cell::operator_rulebook_label(config),
            )
            .with_yolo_mode(history_cell::is_yolo_mode(config)),
        )
    }

    /// Merge the real session info cell with any placeholder header to avoid double boxes.
    fn apply_session_info_cell(&mut self, cell: history_cell::SessionInfoCell) {
        let mut session_info_cell = Some(Box::new(cell) as Box<dyn HistoryCell>);
        let merged_header = if let Some(active) = self.active_cell.take() {
            if active
                .as_any()
                .is::<history_cell::SessionHeaderHistoryCell>()
            {
                // Reuse the existing placeholder header to avoid rendering two boxes.
                if let Some(cell) = session_info_cell.take() {
                    self.active_cell = Some(cell);
                }
                true
            } else {
                self.active_cell = Some(active);
                false
            }
        } else {
            false
        };

        self.flush_active_cell();

        if !merged_header && let Some(cell) = session_info_cell {
            self.add_boxed_history(cell);
        }
    }

    pub(crate) fn add_info_message(&mut self, message: String, hint: Option<String>) {
        self.add_to_history(history_cell::new_info_event(message, hint));
        self.request_redraw();
    }

    pub(crate) fn add_memories_enable_notice(&mut self) {
        self.add_to_history(history_cell::new_warning_event(
            MEMORIES_ENABLE_NOTICE.to_string(),
        ));
        self.request_redraw();
    }

    pub(crate) fn add_plain_history_lines(&mut self, lines: Vec<Line<'static>>) {
        self.add_boxed_history(Box::new(PlainHistoryCell::new(lines)));
        self.request_redraw();
    }

    pub(crate) fn add_error_message(&mut self, message: String) {
        self.add_to_history(history_cell::new_error_event(message));
        self.request_redraw();
    }

    fn add_app_server_stub_message(&mut self, feature: &str) {
        warn!(feature, "stubbed unsupported TUI feature");
        self.add_error_message(format!("{feature}: {TUI_STUB_MESSAGE}"));
    }

    fn rename_confirmation_cell(name: &str, thread_id: Option<ThreadId>) -> PlainHistoryCell {
        let resume_cmd = crate::legacy_core::util::resume_command(Some(name), thread_id)
            .unwrap_or_else(|| format!("vac resume {name}"));
        let name = name.to_string();
        let line = vec![
            "• ".into(),
            "Thread renamed to ".into(),
            name.cyan(),
            ", to resume this thread run ".into(),
            resume_cmd.cyan(),
        ];
        PlainHistoryCell::new(vec![line.into()])
    }

    /// Begin the asynchronous MCP inventory flow: show a loading spinner and
    /// request the app-server fetch via `AppEvent::FetchMcpInventory`.
    ///
    /// The spinner lives in `active_cell` and is cleared by
    /// [`clear_mcp_inventory_loading`] once the result arrives.
    pub(crate) fn add_mcp_output(&mut self, detail: McpServerStatusDetail) {
        self.flush_answer_stream_with_separator();
        self.flush_active_cell();
        self.active_cell = Some(Box::new(history_cell::new_mcp_inventory_loading(
            self.config.animations,
        )));
        self.bump_active_cell_revision();
        self.request_redraw();
        self.app_event_tx
            .send(AppEvent::FetchMcpInventory { detail });
    }

    /// Remove the MCP loading spinner if it is still the active cell.
    ///
    /// Uses `Any`-based type checking so that a late-arriving inventory result
    /// does not accidentally clear an unrelated cell that was set in the meantime.
    pub(crate) fn clear_mcp_inventory_loading(&mut self) {
        let Some(active) = self.active_cell.as_ref() else {
            return;
        };
        if !active
            .as_any()
            .is::<history_cell::McpInventoryLoadingCell>()
        {
            return;
        }
        self.active_cell = None;
        self.bump_active_cell_revision();
        self.request_redraw();
    }

    pub(crate) fn add_connectors_output(&mut self) {
        if !self.connectors_enabled() {
            self.add_info_message(
                "Apps are disabled.".to_string(),
                Some("Enable the apps feature to use $ or /apps.".to_string()),
            );
            return;
        }

        let connectors_cache = self.connectors_cache.clone();
        let should_force_refetch = !self.connectors_prefetch_in_flight
            || matches!(connectors_cache, ConnectorsCacheState::Ready(_));
        self.prefetch_connectors_with_options(should_force_refetch);

        match connectors_cache {
            ConnectorsCacheState::Ready(snapshot) => {
                if snapshot.connectors.is_empty() {
                    self.add_info_message("No apps available.".to_string(), /*hint*/ None);
                } else {
                    self.open_connectors_popup(&snapshot.connectors);
                }
            }
            ConnectorsCacheState::Failed(err) => {
                self.add_to_history(history_cell::new_error_event(err));
            }
            ConnectorsCacheState::Loading | ConnectorsCacheState::Uninitialized => {
                self.open_connectors_loading_popup();
            }
        }
        self.request_redraw();
    }

    fn open_connectors_loading_popup(&mut self) {
        if !self.bottom_pane.replace_selection_view_if_active(
            CONNECTORS_SELECTION_VIEW_ID,
            self.connectors_loading_popup_params(),
        ) {
            self.bottom_pane
                .show_selection_view(self.connectors_loading_popup_params());
        }
    }

    fn open_connectors_popup(&mut self, connectors: &[AppInfo]) {
        self.bottom_pane.show_selection_view(
            self.connectors_popup_params(connectors, /*selected_connector_id*/ None),
        );
    }

    fn connectors_loading_popup_params(&self) -> SelectionViewParams {
        let mut header = ColumnRenderable::new();
        header.push(Line::from("Apps".bold()));
        header.push(Line::from("Loading installed and available apps...".dim()));

        SelectionViewParams {
            view_id: Some(CONNECTORS_SELECTION_VIEW_ID),
            header: Box::new(header),
            items: vec![SelectionItem {
                name: "Loading apps...".to_string(),
                description: Some("This updates when the full list is ready.".to_string()),
                is_disabled: true,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn connectors_popup_params(
        &self,
        connectors: &[AppInfo],
        selected_connector_id: Option<&str>,
    ) -> SelectionViewParams {
        let total = connectors.len();
        let installed = connectors
            .iter()
            .filter(|connector| connector.is_accessible)
            .count();
        let mut header = ColumnRenderable::new();
        header.push(Line::from("Apps".bold()));
        header.push(Line::from(
            "Use $ to insert an installed app into your prompt.".dim(),
        ));
        header.push(Line::from(
            format!("Installed {installed} of {total} available apps.").dim(),
        ));
        let initial_selected_idx = selected_connector_id.and_then(|selected_connector_id| {
            connectors
                .iter()
                .position(|connector| connector.id == selected_connector_id)
        });
        let mut items: Vec<SelectionItem> = Vec::with_capacity(connectors.len());
        for connector in connectors {
            let connector_label = vac_connectors::metadata::connector_display_label(connector);
            let connector_title = connector_label.clone();
            let link_description = Self::connector_description(connector);
            let description = Self::connector_brief_description(connector);
            let status_label = Self::connector_status_label(connector);
            let search_value = format!("{connector_label} {}", connector.id);
            let mut item = SelectionItem {
                name: connector_label,
                description: Some(description),
                search_value: Some(search_value),
                ..Default::default()
            };
            let is_installed = connector.is_accessible;
            let selected_label = if is_installed {
                format!(
                    "{status_label}. Press Enter to open the app page to install, manage, or enable/disable this app."
                )
            } else {
                format!("{status_label}. Press Enter to open the app page to install this app.")
            };
            let missing_label = format!("{status_label}. App link unavailable.");
            let instructions = if connector.is_accessible {
                "Manage this app in your browser."
            } else {
                "Install this app in your browser, then reload VAC."
            };
            if let Some(install_url) = connector.install_url.clone() {
                let app_id = connector.id.clone();
                let is_enabled = connector.is_enabled;
                let title = connector_title.clone();
                let instructions = instructions.to_string();
                let description = link_description.clone();
                item.actions = vec![Box::new(move |tx| {
                    tx.send(AppEvent::OpenAppLink {
                        app_id: app_id.clone(),
                        title: title.clone(),
                        description: description.clone(),
                        instructions: instructions.clone(),
                        url: install_url.clone(),
                        is_installed,
                        is_enabled,
                    });
                })];
                item.dismiss_on_select = true;
                item.selected_description = Some(selected_label);
            } else {
                let missing_label_for_action = missing_label.clone();
                item.actions = vec![Box::new(move |tx| {
                    tx.send(AppEvent::InsertHistoryCell(Box::new(
                        history_cell::new_info_event(
                            missing_label_for_action.clone(),
                            /*hint*/ None,
                        ),
                    )));
                })];
                item.dismiss_on_select = true;
                item.selected_description = Some(missing_label);
            }
            items.push(item);
        }

        SelectionViewParams {
            view_id: Some(CONNECTORS_SELECTION_VIEW_ID),
            header: Box::new(header),
            footer_hint: Some(self.bottom_pane.standard_popup_hint_line()),
            items,
            is_searchable: true,
            search_placeholder: Some("Type to search apps".to_string()),
            col_width_mode: ColumnWidthMode::AutoAllRows,
            initial_selected_idx,
            ..Default::default()
        }
    }

    fn refresh_connectors_popup_if_open(&mut self, connectors: &[AppInfo]) {
        let selected_connector_id =
            if let (Some(selected_index), ConnectorsCacheState::Ready(snapshot)) = (
                self.bottom_pane
                    .selected_index_for_active_view(CONNECTORS_SELECTION_VIEW_ID),
                &self.connectors_cache,
            ) {
                snapshot
                    .connectors
                    .get(selected_index)
                    .map(|connector| connector.id.as_str())
            } else {
                None
            };
        if self.bottom_pane.replace_selection_view_if_active(
            CONNECTORS_SELECTION_VIEW_ID,
            self.connectors_popup_params(connectors, selected_connector_id),
        ) {
            // Clear the height cache so callers see the refreshed popup's updated height.
            self.request_redraw();
        }
    }

    fn connector_brief_description(connector: &AppInfo) -> String {
        let status_label = Self::connector_status_label(connector);
        match Self::connector_description(connector) {
            Some(description) => format!("{status_label} · {description}"),
            None => status_label.to_string(),
        }
    }

    fn connector_status_label(connector: &AppInfo) -> &'static str {
        if connector.is_accessible {
            if connector.is_enabled {
                "Installed"
            } else {
                "Installed · Disabled"
            }
        } else {
            "Can be installed"
        }
    }

    fn connector_description(connector: &AppInfo) -> Option<String> {
        connector
            .description
            .as_deref()
            .map(str::trim)
            .filter(|description| !description.is_empty())
            .map(str::to_string)
    }

    /// Forward file-search results to the bottom pane.
    pub(crate) fn apply_file_search_result(&mut self, query: String, matches: Vec<FileMatch>) {
        self.bottom_pane.on_file_search_result(query, matches);
    }

    /// Handles a Ctrl+C press at the chat-widget layer.
    ///
    /// The first press arms a time-bounded quit shortcut and shows a footer hint via the bottom
    /// pane. If cancellable work is active, Ctrl+C also submits `Op::Interrupt` after the shortcut
    /// is armed.
    ///
    /// Active realtime conversations take precedence over bottom-pane Ctrl+C handling so the
    /// first press always stops live voice, even when the composer contains the recording meter.
    ///
    /// When the double-press quit shortcut is enabled, pressing the same shortcut again before
    /// expiry requests a shutdown-first quit.
    fn on_ctrl_c(&mut self) {
        let key = key_hint::ctrl(KeyCode::Char('c'));
        if self.realtime_conversation.is_live() {
            self.bottom_pane.clear_quit_shortcut_hint();
            self.quit_shortcut_expires_at = None;
            self.quit_shortcut_key = None;
            self.stop_realtime_conversation_from_ui();
            return;
        }
        let modal_or_popup_active = !self.bottom_pane.no_modal_or_popup_active();
        if self.bottom_pane.on_ctrl_c() == CancellationEvent::Handled {
            if DOUBLE_PRESS_QUIT_SHORTCUT_ENABLED {
                if modal_or_popup_active {
                    self.quit_shortcut_expires_at = None;
                    self.quit_shortcut_key = None;
                    self.bottom_pane.clear_quit_shortcut_hint();
                } else {
                    self.arm_quit_shortcut(key);
                }
            }
            return;
        }

        if !DOUBLE_PRESS_QUIT_SHORTCUT_ENABLED {
            if self.is_cancellable_work_active() {
                self.quit_shortcut_expires_at = None;
                self.quit_shortcut_key = None;
                self.bottom_pane.clear_quit_shortcut_hint();
                self.submit_op(AppCommand::interrupt());
            } else {
                self.request_quit_without_confirmation();
            }
            return;
        }

        if self.quit_shortcut_active_for(key) {
            self.quit_shortcut_expires_at = None;
            self.quit_shortcut_key = None;
            self.request_quit_without_confirmation();
            return;
        }

        self.arm_quit_shortcut(key);

        if self.is_cancellable_work_active() {
            self.submit_op(AppCommand::interrupt());
        }
    }

    /// Handles a Ctrl+D press at the chat-widget layer.
    ///
    /// Ctrl-D only participates in quit when the composer is empty and no modal/popup is active.
    /// Otherwise it should be routed to the active view and not attempt to quit.
    fn on_ctrl_d(&mut self) -> bool {
        let key = key_hint::ctrl(KeyCode::Char('d'));
        if !DOUBLE_PRESS_QUIT_SHORTCUT_ENABLED {
            if !self.bottom_pane.composer_is_empty() || !self.bottom_pane.no_modal_or_popup_active()
            {
                return false;
            }

            self.request_quit_without_confirmation();
            return true;
        }

        if self.quit_shortcut_active_for(key) {
            self.quit_shortcut_expires_at = None;
            self.quit_shortcut_key = None;
            self.request_quit_without_confirmation();
            return true;
        }

        if !self.bottom_pane.composer_is_empty() || !self.bottom_pane.no_modal_or_popup_active() {
            return false;
        }

        self.arm_quit_shortcut(key);
        true
    }

    /// True if `key` matches the armed quit shortcut and the window has not expired.
    fn quit_shortcut_active_for(&self, key: KeyBinding) -> bool {
        self.quit_shortcut_key == Some(key)
            && self
                .quit_shortcut_expires_at
                .is_some_and(|expires_at| Instant::now() < expires_at)
    }

    /// Arm the double-press quit shortcut and show the footer hint.
    ///
    /// This keeps the state machine (`quit_shortcut_*`) in `ChatWidget`, since
    /// it is the component that interprets Ctrl+C vs Ctrl+D and decides whether
    /// quitting is currently allowed, while delegating rendering to `BottomPane`.
    fn arm_quit_shortcut(&mut self, key: KeyBinding) {
        self.quit_shortcut_expires_at = Instant::now()
            .checked_add(QUIT_SHORTCUT_TIMEOUT)
            .or_else(|| Some(Instant::now()));
        self.quit_shortcut_key = Some(key);
        self.bottom_pane.show_quit_shortcut_hint(key);
    }

    // Review mode counts as cancellable work so Ctrl+C interrupts instead of quitting.
    fn is_cancellable_work_active(&self) -> bool {
        self.bottom_pane.is_task_running() || self.is_review_mode
    }

    /// Return the markdown body width available to an active stream.
    ///
    /// Streaming controllers render only the message body, while history cells add bullets,
    /// gutters, or plan padding around that body. Callers pass the reserved columns for that
    /// wrapper so live output uses the same width that finalized cells will use during reflow.
    fn current_stream_width(&self, reserved_cols: usize) -> Option<usize> {
        self.last_rendered_width.get().and_then(|width| {
            if width == 0 {
                None
            } else {
                Some(crate::width::usable_content_width(width, reserved_cols).unwrap_or(1))
            }
        })
    }

    /// Update resize-sensitive chat widget state after the terminal width changes.
    ///
    /// The app calls this even when terminal resize reflow is disabled so live stream wrapping
    /// remains consistent with the current viewport. Finalized transcript rebuilding stays gated at
    /// the app layer.
    pub(crate) fn on_terminal_resize(&mut self, width: u16) {
        let had_rendered_width = self.last_rendered_width.get().is_some();
        self.last_rendered_width.set(Some(width as usize));
        let stream_width = self.current_stream_width(/*reserved_cols*/ 2);
        let plan_stream_width = self.current_stream_width(/*reserved_cols*/ 4);
        if let Some(controller) = self.stream_controller.as_mut() {
            controller.set_width(stream_width);
        }
        if let Some(controller) = self.plan_stream_controller.as_mut() {
            controller.set_width(plan_stream_width);
        }
        if !had_rendered_width {
            self.request_redraw();
        }
    }

    /// Whether an agent message stream is active (not a plan stream).
    pub(crate) fn has_active_agent_stream(&self) -> bool {
        self.stream_controller.is_some()
    }

    /// Whether a proposed-plan stream is active.
    pub(crate) fn has_active_plan_stream(&self) -> bool {
        self.plan_stream_controller.is_some()
    }

    fn is_plan_streaming_in_tui(&self) -> bool {
        self.plan_stream_controller.is_some()
    }

    pub(crate) fn composer_is_empty(&self) -> bool {
        self.bottom_pane.composer_is_empty()
    }

    #[cfg(test)]
    pub(crate) fn is_task_running_for_test(&self) -> bool {
        self.bottom_pane.is_task_running()
    }

    pub(crate) fn toggle_vim_mode_and_notify(&mut self) {
        let enabled = self.bottom_pane.toggle_vim_enabled();
        let message = if enabled {
            "Vim mode enabled."
        } else {
            "Vim mode disabled."
        };
        self.add_info_message(message.to_string(), /*hint*/ None);
    }
}
