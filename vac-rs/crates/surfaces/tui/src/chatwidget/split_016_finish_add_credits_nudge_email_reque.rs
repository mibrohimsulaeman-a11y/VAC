    pub(crate) fn finish_add_credits_nudge_email_request(
        &mut self,
        result: Result<AddCreditsNudgeEmailStatus, String>,
    ) {
        let credit_type = self
            .add_credits_nudge_email_in_flight
            .take()
            .unwrap_or(AddCreditsNudgeCreditType::Credits);
        if !self.workspace_owner_usage_nudge_enabled() {
            return;
        }
        let message = match (credit_type, result) {
            (AddCreditsNudgeCreditType::Credits, Ok(AddCreditsNudgeEmailStatus::Sent)) => {
                "Workspace owner notified."
            }
            (
                AddCreditsNudgeCreditType::Credits,
                Ok(AddCreditsNudgeEmailStatus::CooldownActive),
            ) => "Workspace owner was already notified recently.",
            (AddCreditsNudgeCreditType::Credits, Err(_)) => {
                "Could not notify your workspace owner. Please try again."
            }
            (AddCreditsNudgeCreditType::UsageLimit, Ok(AddCreditsNudgeEmailStatus::Sent)) => {
                "Limit increase requested."
            }
            (
                AddCreditsNudgeCreditType::UsageLimit,
                Ok(AddCreditsNudgeEmailStatus::CooldownActive),
            ) => "A limit increase was already requested recently.",
            (AddCreditsNudgeCreditType::UsageLimit, Err(_)) => {
                "Could not request a limit increase. Please try again."
            }
        };
        self.add_to_history(history_cell::new_info_event(
            message.to_string(),
            /*hint*/ None,
        ));
        self.request_redraw();
    }

    /// Open a popup to choose a quick auto model. Selecting "All models"
    /// opens the full picker with every available preset.
    pub(crate) fn open_model_popup(&mut self) {
        if !self.is_session_configured() {
            self.add_info_message(
                "Model selection is disabled until startup completes.".to_string(),
                /*hint*/ None,
            );
            return;
        }

        if self.is_kilo_model_provider() {
            self.open_kilo_model_popup();
            return;
        }

        let presets: Vec<ModelPreset> = match self.model_catalog.try_list_models() {
            Ok(models) => models,
            Err(_) => {
                self.add_info_message(
                    "Models are being updated; please try /model again in a moment.".to_string(),
                    /*hint*/ None,
                );
                return;
            }
        };
        self.open_model_popup_with_presets(presets);
    }

    fn is_kilo_model_provider(&self) -> bool {
        if self.config.model_provider_id == KILO_PROVIDER_ID {
            return true;
        }
        if self
            .config
            .model_provider
            .name
            .to_ascii_lowercase()
            .contains("kilo")
        {
            return true;
        }
        self.config
            .model_provider
            .base_url
            .as_deref()
            .map(|base_url| base_url.to_ascii_lowercase().contains("kilo.ai"))
            .unwrap_or(false)
    }

    fn open_kilo_model_popup(&mut self) {
        self.add_info_message(
            "Loading Kilo Gateway models...".to_string(),
            /*hint*/ None,
        );

        let base_url = self
            .config
            .model_provider
            .base_url
            .clone()
            .unwrap_or_else(|| KILO_GATEWAY_DEFAULT_BASE_URL.to_string());
        let api_key = self
            .config
            .model_provider
            .env_key
            .as_ref()
            .and_then(|env_key| std::env::var(env_key).ok())
            .filter(|value| !value.trim().is_empty());
        let tx = self.app_event_tx.clone();

        tokio::spawn(async move {
            let result = fetch_kilo_model_presets(base_url, api_key).await;
            tx.send(AppEvent::KiloModelsLoaded { result });
        });
    }

    pub(crate) fn finish_kilo_models_loaded(&mut self, result: Result<Vec<ModelPreset>, String>) {
        match result {
            Ok(models) if !models.is_empty() => {
                self.open_all_models_popup(models);
            }
            Ok(_) => {
                self.add_info_message(
                    "Kilo Gateway returned no models. Showing common Kilo models instead."
                        .to_string(),
                    /*hint*/ None,
                );
                self.open_all_models_popup(kilo_fallback_model_presets());
            }
            Err(err) => {
                self.add_error_message(format!(
                    "Could not load Kilo Gateway models: {err}. Showing common Kilo models instead."
                ));
                self.open_all_models_popup(kilo_fallback_model_presets());
            }
        }
    }

    pub(crate) fn open_personality_popup(&mut self) {
        if !self.is_session_configured() {
            self.add_info_message(
                "Personality selection is disabled until startup completes.".to_string(),
                /*hint*/ None,
            );
            return;
        }
        if !self.current_model_supports_personality() {
            let current_model = self.current_model();
            self.add_error_message(format!(
                "Current model ({current_model}) doesn't support personalities. Try /model to pick a different model."
            ));
            return;
        }
        self.open_personality_popup_for_current_model();
    }

    fn open_personality_popup_for_current_model(&mut self) {
        let current_personality = self.config.personality.unwrap_or(Personality::Friendly);
        let personalities = [Personality::Friendly, Personality::Pragmatic];
        let supports_personality = self.current_model_supports_personality();

        let items: Vec<SelectionItem> = personalities
            .into_iter()
            .map(|personality| {
                let name = Self::personality_label(personality).to_string();
                let description = Some(Self::personality_description(personality).to_string());
                let actions: Vec<SelectionAction> = vec![Box::new(move |tx| {
                    tx.send(AppEvent::VACOp(AppCommand::override_turn_context(
                        /*cwd*/ None,
                        /*approval_policy*/ None,
                        /*approvals_reviewer*/ None,
                        /*permission_profile*/ None,
                        /*windows_sandbox_level*/ None,
                        /*model*/ None,
                        /*effort*/ None,
                        /*summary*/ None,
                        /*service_tier*/ None,
                        /*collaboration_mode*/ None,
                        Some(personality),
                    )));
                    tx.send(AppEvent::UpdatePersonality(personality));
                    tx.send(AppEvent::PersistPersonalitySelection { personality });
                })];
                SelectionItem {
                    name,
                    description,
                    is_current: current_personality == personality,
                    is_disabled: !supports_personality,
                    actions,
                    dismiss_on_select: true,
                    ..Default::default()
                }
            })
            .collect();

        let mut header = ColumnRenderable::new();
        header.push(Line::from("Select Personality".bold()));
        header.push(Line::from("Choose a communication style for VAC.".dim()));

        self.bottom_pane.show_selection_view(SelectionViewParams {
            header: Box::new(header),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            ..Default::default()
        });
    }


    fn model_menu_header(&self, title: &str, subtitle: &str) -> Box<dyn Renderable> {
        let title = title.to_string();
        let subtitle = subtitle.to_string();
        let mut header = ColumnRenderable::new();
        header.push(Line::from(title.bold()));
        header.push(Line::from(subtitle.dim()));
        if let Some(warning) = self.model_menu_warning_line() {
            header.push(warning);
        }
        Box::new(header)
    }

    fn model_menu_warning_line(&self) -> Option<Line<'static>> {
        let base_url = self.custom_vastar_base_url()?;
        let warning = format!(
            "Warning: Vastar base URL is overridden to {base_url}. Selecting models may not be supported or work properly."
        );
        Some(Line::from(warning.red()))
    }

    fn custom_vastar_base_url(&self) -> Option<String> {
        if !self.config.model_provider.is_vastar() {
            return None;
        }

        let base_url = self.config.model_provider.base_url.as_ref()?;
        let trimmed = base_url.trim();
        if trimmed.is_empty() {
            return None;
        }

        let normalized = trimmed.trim_end_matches('/');
        if normalized == DEFAULT_VASTAR_BASE_URL {
            return None;
        }

        Some(trimmed.to_string())
    }

    pub(crate) fn open_model_popup_with_presets(&mut self, presets: Vec<ModelPreset>) {
        let presets: Vec<ModelPreset> = presets
            .into_iter()
            .filter(|preset| preset.show_in_picker)
            .collect();

        let current_model = self.current_model();
        let current_label = presets
            .iter()
            .find(|preset| preset.model.as_str() == current_model)
            .map(|preset| preset.model.to_string())
            .unwrap_or_else(|| self.model_display_name().to_string());

        let (mut auto_presets, other_presets): (Vec<ModelPreset>, Vec<ModelPreset>) = presets
            .into_iter()
            .partition(|preset| Self::is_auto_model(&preset.model));

        if auto_presets.is_empty() {
            self.open_all_models_popup(other_presets);
            return;
        }

        auto_presets.sort_by_key(|preset| Self::auto_model_order(&preset.model));
        let mut items: Vec<SelectionItem> = auto_presets
            .into_iter()
            .map(|preset| {
                let description =
                    (!preset.description.is_empty()).then_some(preset.description.clone());
                let model = preset.model.clone();
                let should_prompt_plan_mode_scope = self.should_prompt_plan_mode_reasoning_scope(
                    model.as_str(),
                    Some(preset.default_reasoning_effort),
                );
                let actions = Self::model_selection_actions(
                    model.clone(),
                    Some(preset.default_reasoning_effort),
                    should_prompt_plan_mode_scope,
                );
                SelectionItem {
                    name: model.clone(),
                    description,
                    is_current: model.as_str() == current_model,
                    is_default: preset.is_default,
                    actions,
                    dismiss_on_select: true,
                    ..Default::default()
                }
            })
            .collect();

        if !other_presets.is_empty() {
            let all_models = other_presets;
            let actions: Vec<SelectionAction> = vec![Box::new(move |tx| {
                tx.send(AppEvent::OpenAllModelsPopup {
                    models: all_models.clone(),
                });
            })];

            let is_current = !items.iter().any(|item| item.is_current);
            let description = Some(format!(
                "Choose a specific model and reasoning level (current: {current_label})"
            ));

            items.push(SelectionItem {
                name: "All models".to_string(),
                description,
                is_current,
                actions,
                dismiss_on_select: true,
                ..Default::default()
            });
        }

        let header = self.model_menu_header(
            "Select Model",
            "Pick a quick auto mode or browse all models.",
        );
        self.bottom_pane.show_selection_view(SelectionViewParams {
            footer_hint: Some(standard_popup_hint_line()),
            items,
            header,
            ..Default::default()
        });
    }

