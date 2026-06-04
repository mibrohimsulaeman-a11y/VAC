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
