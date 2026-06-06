    #[cfg(not(target_os = "windows"))]
    pub(crate) fn open_windows_sandbox_fallback_prompt(&mut self, _preset: ApprovalPreset) {}

    #[cfg(target_os = "windows")]
    pub(crate) fn maybe_prompt_windows_sandbox_enable(&mut self, show_now: bool) {
        if show_now
            && WindowsSandboxLevel::from_config(&self.config) == WindowsSandboxLevel::Disabled
            && let Some(preset) = builtin_approval_presets()
                .into_iter()
                .find(|preset| preset.id == "auto")
        {
            self.open_windows_sandbox_enable_prompt(preset);
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn maybe_prompt_windows_sandbox_enable(&mut self, _show_now: bool) {}

    #[cfg(target_os = "windows")]
    pub(crate) fn show_windows_sandbox_setup_status(&mut self) {
        // While elevated sandbox setup runs, prevent typing so the user doesn't
        // accidentally queue messages that will run under an unexpected mode.
        self.bottom_pane.set_composer_input_enabled(
            /*enabled*/ false,
            Some("Input disabled until setup completes.".to_string()),
        );
        self.bottom_pane.ensure_status_indicator();
        self.bottom_pane
            .set_interrupt_hint_visible(/*visible*/ false);
        self.set_status(
            "Setting up sandbox...".to_string(),
            Some("Hang tight, this may take a few minutes".to_string()),
            StatusDetailsCapitalization::CapitalizeFirst,
            STATUS_DETAILS_DEFAULT_MAX_LINES,
        );
        self.request_redraw();
    }

    #[cfg(not(target_os = "windows"))]
    #[allow(dead_code)]
    pub(crate) fn show_windows_sandbox_setup_status(&mut self) {}

    #[cfg(target_os = "windows")]
    pub(crate) fn clear_windows_sandbox_setup_status(&mut self) {
        self.bottom_pane
            .set_composer_input_enabled(/*enabled*/ true, /*placeholder*/ None);
        self.bottom_pane.hide_status_indicator();
        self.request_redraw();
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn clear_windows_sandbox_setup_status(&mut self) {}

    /// Set the approval policy in the widget's config copy.
    pub(crate) fn set_approval_policy(&mut self, policy: AskForApproval) {
        if let Err(err) = self.config.permissions.approval_policy.set(policy) {
            tracing::warn!(%err, "failed to set approval_policy on chat config");
        }
    }

    /// Set the permission profile in the widget's config copy.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub(crate) fn set_permission_profile(
        &mut self,
        profile: PermissionProfile,
    ) -> ConstraintResult<()> {
        self.config.permissions.set_permission_profile(profile)
    }

    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub(crate) fn set_windows_sandbox_mode(&mut self, mode: Option<WindowsSandboxModeToml>) {
        self.config.permissions.windows_sandbox_mode = mode;
        #[cfg(target_os = "windows")]
        self.bottom_pane.set_windows_degraded_sandbox_active(
            crate::legacy_core::windows_sandbox::ELEVATED_SANDBOX_NUX_ENABLED
                && matches!(
                    WindowsSandboxLevel::from_config(&self.config),
                    WindowsSandboxLevel::RestrictedToken
                ),
        );
    }

    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub(crate) fn set_feature_enabled(&mut self, feature: Feature, enabled: bool) -> bool {
        if let Err(err) = self.config.features.set_enabled(feature, enabled) {
            tracing::warn!(
                error = %err,
                feature = feature.key(),
                "failed to update constrained chat widget feature state"
            );
        }
        let enabled = self.config.features.enabled(feature);
        if feature == Feature::FastMode {
            self.sync_fast_command_enabled();
        }
        if feature == Feature::Personality {
            self.sync_personality_command_enabled();
        }
        if feature == Feature::Plugins {
            self.sync_plugins_command_enabled();
            self.refresh_plugin_mentions();
        }
        if feature == Feature::Goals {
            self.sync_goal_command_enabled();
            if !enabled {
                self.current_goal_status_indicator = None;
                self.current_goal_status = None;
                self.goal_status_active_turn_started_at = None;
                self.budget_limited_turn_ids.clear();
                self.update_collaboration_mode_indicator();
            }
        }
        if feature == Feature::PreventIdleSleep {
            self.turn_sleep_inhibitor = SleepInhibitor::new(enabled);
            self.turn_sleep_inhibitor
                .set_turn_running(self.agent_turn_running);
        }
        #[cfg(target_os = "windows")]
        if matches!(
            feature,
            Feature::WindowsSandbox | Feature::WindowsSandboxElevated
        ) {
            self.bottom_pane.set_windows_degraded_sandbox_active(
                crate::legacy_core::windows_sandbox::ELEVATED_SANDBOX_NUX_ENABLED
                    && matches!(
                        WindowsSandboxLevel::from_config(&self.config),
                        WindowsSandboxLevel::RestrictedToken
                    ),
            );
        }
        enabled
    }

    pub(crate) fn set_approvals_reviewer(&mut self, policy: ApprovalsReviewer) {
        self.config.approvals_reviewer = policy;
    }

    pub(crate) fn set_full_access_warning_acknowledged(&mut self, acknowledged: bool) {
        self.config.notices.hide_full_access_warning = Some(acknowledged);
    }

    pub(crate) fn set_world_writable_warning_acknowledged(&mut self, acknowledged: bool) {
        self.config.notices.hide_world_writable_warning = Some(acknowledged);
    }

    pub(crate) fn set_rate_limit_switch_prompt_hidden(&mut self, hidden: bool) {
        self.config.notices.hide_rate_limit_model_nudge = Some(hidden);
        if hidden {
            self.rate_limit_switch_prompt = RateLimitSwitchPromptState::Idle;
        }
    }

    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub(crate) fn world_writable_warning_hidden(&self) -> bool {
        self.config
            .notices
            .hide_world_writable_warning
            .unwrap_or(false)
    }

    /// Override the reasoning effort used when Plan mode is active.
    ///
    /// When the active mask is already Plan, the override is applied immediately
    /// so the footer reflects it without waiting for the next mode switch.
    /// Passing `None` resets to the Plan-mode preset default.
    pub(crate) fn set_plan_mode_reasoning_effort(&mut self, effort: Option<ReasoningEffortConfig>) {
        self.config.plan_mode_reasoning_effort = effort;
        if self.collaboration_modes_enabled()
            && let Some(mask) = self.active_collaboration_mask.as_mut()
            && mask.mode == Some(ModeKind::Plan)
        {
            if let Some(effort) = effort {
                mask.reasoning_effort = Some(Some(effort));
            } else if let Some(plan_mask) =
                collaboration_modes::plan_mask(self.model_catalog.as_ref())
            {
                mask.reasoning_effort = plan_mask.reasoning_effort;
            }
        }
        self.refresh_model_dependent_surfaces();
    }

    /// Set the reasoning effort for the non-Plan collaboration mode.
    ///
    /// Does not touch the active Plan mask — Plan reasoning is controlled
    /// exclusively by the Plan preset and `set_plan_mode_reasoning_effort`.
    pub(crate) fn set_reasoning_effort(&mut self, effort: Option<ReasoningEffortConfig>) {
        self.current_collaboration_mode = self.current_collaboration_mode.with_updates(
            /*model*/ None,
            Some(effort),
            /*developer_instructions*/ None,
        );
        if self.collaboration_modes_enabled()
            && let Some(mask) = self.active_collaboration_mask.as_mut()
            && mask.mode != Some(ModeKind::Plan)
        {
            // Generic "global default" updates should not mutate the active Plan mask.
            // Plan reasoning is controlled by the Plan preset and Plan-only override updates.
            mask.reasoning_effort = Some(effort);
        }
        self.refresh_model_dependent_surfaces();
    }

    /// Set the personality in the widget's config copy.
    pub(crate) fn set_personality(&mut self, personality: Personality) {
        self.config.personality = Some(personality);
    }

    /// Set Fast mode in the widget's config copy.
    pub(crate) fn set_service_tier(&mut self, service_tier: Option<ServiceTier>) {
        self.config.service_tier = service_tier;
        self.effective_service_tier = service_tier;
    }

    pub(crate) fn current_service_tier(&self) -> Option<ServiceTier> {
        self.effective_service_tier
    }

    pub(crate) fn configured_service_tier(&self) -> Option<ServiceTier> {
        self.config.service_tier
    }

    pub(crate) fn fast_default_opt_out(&self) -> Option<bool> {
        self.config.notices.fast_default_opt_out
    }

    pub(crate) fn status_account_display(&self) -> Option<&StatusAccountDisplay> {
        self.status_account_display.as_ref()
    }

    pub(crate) fn runtime_model_provider_base_url(&self) -> Option<&str> {
        self.runtime_model_provider_base_url.as_deref()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn model_catalog(&self) -> Arc<ModelCatalog> {
        self.model_catalog.clone()
    }

    pub(crate) fn current_plan_type(&self) -> Option<PlanType> {
        self.plan_type
    }

    pub(crate) fn has_chatgpt_account(&self) -> bool {
        self.has_chatgpt_account
    }

    pub(crate) fn update_account_state(
        &mut self,
        status_account_display: Option<StatusAccountDisplay>,
        plan_type: Option<PlanType>,
        has_chatgpt_account: bool,
    ) {
        self.status_account_display = status_account_display;
        self.plan_type = plan_type;
        self.has_chatgpt_account = has_chatgpt_account;
        self.bottom_pane
            .set_connectors_enabled(self.connectors_enabled());
    }

    pub(crate) fn should_show_fast_status(
        &self,
        model: &str,
        service_tier: Option<ServiceTier>,
    ) -> bool {
        self.model_supports_fast_mode(model)
            && matches!(service_tier, Some(ServiceTier::Fast))
            && self.has_chatgpt_account
    }

    fn fast_mode_enabled(&self) -> bool {
        self.config.features.enabled(Feature::FastMode)
    }

    /// Set the syntax theme override in the widget's config copy.
    pub(crate) fn set_tui_theme(&mut self, theme: Option<String>) {
        if self.config.tui_theme != theme {
            self.bump_style_epoch();
        }
        self.config.tui_theme = theme;
    }

    /// Set the model in the widget's config copy and stored collaboration mode.
    pub(crate) fn set_model(&mut self, model: &str) {
        self.current_collaboration_mode = self.current_collaboration_mode.with_updates(
            Some(model.to_string()),
            /*effort*/ None,
            /*developer_instructions*/ None,
        );
        if self.collaboration_modes_enabled()
            && let Some(mask) = self.active_collaboration_mask.as_mut()
        {
            mask.model = Some(model.to_string());
        }
        self.refresh_model_dependent_surfaces();
    }

    fn set_service_tier_selection(&mut self, service_tier: Option<ServiceTier>) {
        if service_tier.is_none() {
            self.config.notices.fast_default_opt_out = Some(true);
        }
        self.set_service_tier(service_tier);
        self.app_event_tx
            .send(AppEvent::VACOp(AppCommand::override_turn_context(
                /*cwd*/ None,
                /*approval_policy*/ None,
                /*approvals_reviewer*/ None,
                /*permission_profile*/ None,
                /*windows_sandbox_level*/ None,
                /*model*/ None,
                /*effort*/ None,
                /*summary*/ None,
                Some(service_tier),
                /*collaboration_mode*/ None,
                /*personality*/ None,
            )));
        self.app_event_tx
            .send(AppEvent::PersistServiceTierSelection { service_tier });
    }

    pub(crate) fn current_model(&self) -> &str {
        if !self.collaboration_modes_enabled() {
            return self.current_collaboration_mode.model();
        }
        self.active_collaboration_mask
            .as_ref()
            .and_then(|mask| mask.model.as_deref())
            .unwrap_or_else(|| self.current_collaboration_mode.model())
    }

    fn sync_fast_command_enabled(&mut self) {
        self.bottom_pane
            .set_fast_command_enabled(self.fast_mode_enabled());
    }

    fn sync_personality_command_enabled(&mut self) {
        self.bottom_pane
            .set_personality_command_enabled(self.config.features.enabled(Feature::Personality));
    }

    fn sync_plugins_command_enabled(&mut self) {
        self.bottom_pane
            .set_plugins_command_enabled(self.config.features.enabled(Feature::Plugins));
    }

    fn sync_goal_command_enabled(&mut self) {
        self.bottom_pane
            .set_goal_command_enabled(self.config.features.enabled(Feature::Goals));
    }

    fn current_model_supports_personality(&self) -> bool {
        let model = self.current_model();
        self.model_catalog
            .try_list_models()
            .ok()
            .and_then(|models| {
                models
                    .into_iter()
                    .find(|preset| preset.model == model)
                    .map(|preset| preset.supports_personality)
            })
            .unwrap_or(false)
    }

    fn model_supports_fast_mode(&self, model: &str) -> bool {
        self.model_catalog
            .try_list_models()
            .ok()
            .and_then(|models| {
                models
                    .into_iter()
                    .find(|preset| preset.model == model)
                    .map(|preset| preset.supports_fast_mode())
            })
            .unwrap_or(false)
    }

    /// Return whether the effective model currently advertises image-input support.
    ///
    /// We intentionally default to `true` when model metadata cannot be read so transient catalog
    /// failures do not hard-block user input in the UI.
    fn current_model_supports_images(&self) -> bool {
        let model = self.current_model();
        self.model_catalog
            .try_list_models()
            .ok()
            .and_then(|models| {
                models
                    .into_iter()
                    .find(|preset| preset.model == model)
                    .map(|preset| preset.input_modalities.contains(&InputModality::Image))
            })
            .unwrap_or(true)
    }

