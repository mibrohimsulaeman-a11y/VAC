    pub(crate) fn submit_user_message_with_mode(
        &mut self,
        text: String,
        mut collaboration_mode: CollaborationModeMask,
    ) {
        if collaboration_mode.mode == Some(ModeKind::Plan)
            && let Some(effort) = self.config.plan_mode_reasoning_effort
        {
            collaboration_mode.reasoning_effort = Some(Some(effort));
        }
        if self.agent_turn_running
            && self.active_collaboration_mask.as_ref() != Some(&collaboration_mode)
        {
            self.add_error_message(
                "Cannot switch collaboration mode while a turn is running.".to_string(),
            );
            return;
        }
        self.set_collaboration_mask(collaboration_mode);
        let should_queue = self.is_plan_streaming_in_tui();
        let user_message = UserMessage {
            text,
            local_images: Vec::new(),
            remote_image_urls: Vec::new(),
            text_elements: Vec::new(),
            mention_bindings: Vec::new(),
        };
        if should_queue {
            self.queue_user_message(user_message);
        } else {
            self.submit_user_message(user_message);
        }
    }

    /// True when the UI is in the regular composer state with no running task,
    /// no modal overlay (e.g. approvals or status indicator), and no composer popups.
    /// In this state Esc-Esc backtracking is enabled.
    pub(crate) fn is_normal_backtrack_mode(&self) -> bool {
        self.bottom_pane.is_normal_backtrack_mode()
    }

    pub(crate) fn should_handle_vim_insert_escape(&self, key_event: KeyEvent) -> bool {
        self.bottom_pane
            .composer_should_handle_vim_insert_escape(key_event)
    }

    pub(crate) fn insert_str(&mut self, text: &str) {
        self.bottom_pane.insert_str(text);
    }

    /// Replace the composer content with the provided text and reset cursor.
    pub(crate) fn set_composer_text(
        &mut self,
        text: String,
        text_elements: Vec<TextElement>,
        local_image_paths: Vec<PathBuf>,
    ) {
        self.bottom_pane
            .set_composer_text(text, text_elements, local_image_paths);
        self.refresh_plan_mode_nudge();
    }

    pub(crate) fn set_remote_image_urls(&mut self, remote_image_urls: Vec<String>) {
        self.bottom_pane.set_remote_image_urls(remote_image_urls);
    }

    fn take_remote_image_urls(&mut self) -> Vec<String> {
        self.bottom_pane.take_remote_image_urls()
    }

    #[cfg(test)]
    pub(crate) fn remote_image_urls(&self) -> Vec<String> {
        self.bottom_pane.remote_image_urls()
    }

    #[cfg(test)]
    pub(crate) fn queued_user_message_texts(&self) -> Vec<String> {
        self.rejected_steers_queue
            .iter()
            .map(|message| message.text.clone())
            .chain(
                self.queued_user_messages
                    .iter()
                    .map(|message| message.text.clone()),
            )
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn pending_thread_approvals(&self) -> &[String] {
        self.bottom_pane.pending_thread_approvals()
    }

    #[cfg(test)]
    pub(crate) fn has_active_view(&self) -> bool {
        self.bottom_pane.has_active_view()
    }

    pub(crate) fn show_esc_backtrack_hint(&mut self) {
        self.bottom_pane.show_esc_backtrack_hint();
    }

    pub(crate) fn clear_esc_backtrack_hint(&mut self) {
        self.bottom_pane.clear_esc_backtrack_hint();
    }

    fn refresh_skills_for_current_cwd(&mut self, force_reload: bool) {
        self.submit_op(AppCommand::list_skills(
            vec![self.config.cwd.to_path_buf()],
            force_reload,
        ));
    }

    /// Forward a command directly to vac.
    pub(crate) fn submit_op<T>(&mut self, op: T) -> bool
    where
        T: Into<AppCommand>,
    {
        let op: AppCommand = op.into();
        self.prepare_local_op_submission(&op);
        if op.is_review() && !self.bottom_pane.is_task_running() {
            self.bottom_pane.set_task_running(/*running*/ true);
        }
        match &self.vac_op_target {
            #[cfg(test)]
            VACOpTarget::Direct(vac_op_tx) => {
                crate::session_log::log_outbound_op(&op);
                if let Err(e) = vac_op_tx.send(op) {
                    tracing::error!("failed to submit op: {e}");
                    return false;
                }
            }
            VACOpTarget::AppEvent => {
                self.app_event_tx.send(AppEvent::VACOp(op));
            }
        }
        true
    }

    pub(crate) fn prepare_local_op_submission(&mut self, op: &AppCommand) {
        if matches!(op, AppCommand::Interrupt) && self.agent_turn_running {
            if let Some(controller) = self.stream_controller.as_mut() {
                controller.clear_queue();
            }
            if let Some(controller) = self.plan_stream_controller.as_mut() {
                controller.clear_queue();
            }
            self.request_redraw();
        }
    }

    fn on_list_skills(&mut self, ev: SkillsListResponse) {
        self.set_skills_from_response(&ev);
        self.refresh_plugin_mentions();
    }

    pub(crate) fn on_connectors_loaded(
        &mut self,
        result: Result<ConnectorsSnapshot, String>,
        is_final: bool,
    ) {
        let mut trigger_pending_force_refetch = false;
        if is_final {
            self.connectors_prefetch_in_flight = false;
            if self.connectors_force_refetch_pending {
                self.connectors_force_refetch_pending = false;
                trigger_pending_force_refetch = true;
            }
        }

        match result {
            Ok(mut snapshot) => {
                if !is_final {
                    snapshot.connectors = connectors::merge_connectors_with_accessible(
                        Vec::new(),
                        snapshot.connectors,
                        /*all_connectors_loaded*/ false,
                    );
                }
                snapshot.connectors =
                    connectors::with_app_enabled_state(snapshot.connectors, &self.config);
                if let ConnectorsCacheState::Ready(existing_snapshot) = &self.connectors_cache {
                    let enabled_by_id: HashMap<&str, bool> = existing_snapshot
                        .connectors
                        .iter()
                        .map(|connector| (connector.id.as_str(), connector.is_enabled))
                        .collect();
                    for connector in &mut snapshot.connectors {
                        if let Some(is_enabled) = enabled_by_id.get(connector.id.as_str()) {
                            connector.is_enabled = *is_enabled;
                        }
                    }
                }
                if is_final {
                    self.connectors_partial_snapshot = None;
                    self.refresh_connectors_popup_if_open(&snapshot.connectors);
                    self.connectors_cache = ConnectorsCacheState::Ready(snapshot.clone());
                } else {
                    self.connectors_partial_snapshot = Some(snapshot.clone());
                }
                self.bottom_pane.set_connectors_snapshot(Some(snapshot));
            }
            Err(err) => {
                let partial_snapshot = self.connectors_partial_snapshot.take();
                if let ConnectorsCacheState::Ready(snapshot) = &self.connectors_cache {
                    warn!("failed to refresh apps list; retaining current apps snapshot: {err}");
                    self.bottom_pane
                        .set_connectors_snapshot(Some(snapshot.clone()));
                } else if let Some(snapshot) = partial_snapshot {
                    warn!(
                        "failed to load full apps list; falling back to installed apps snapshot: {err}"
                    );
                    self.refresh_connectors_popup_if_open(&snapshot.connectors);
                    self.connectors_cache = ConnectorsCacheState::Ready(snapshot.clone());
                    self.bottom_pane.set_connectors_snapshot(Some(snapshot));
                } else {
                    self.connectors_cache = ConnectorsCacheState::Failed(err);
                    self.bottom_pane.set_connectors_snapshot(/*snapshot*/ None);
                }
            }
        }

        if trigger_pending_force_refetch {
            self.prefetch_connectors_with_options(/*force_refetch*/ true);
        }
    }

    pub(crate) fn update_connector_enabled(&mut self, connector_id: &str, enabled: bool) {
        let ConnectorsCacheState::Ready(mut snapshot) = self.connectors_cache.clone() else {
            return;
        };

        let mut changed = false;
        for connector in &mut snapshot.connectors {
            if connector.id == connector_id {
                changed = connector.is_enabled != enabled;
                connector.is_enabled = enabled;
                break;
            }
        }

        if !changed {
            return;
        }

        self.refresh_connectors_popup_if_open(&snapshot.connectors);
        self.connectors_cache = ConnectorsCacheState::Ready(snapshot.clone());
        self.bottom_pane.set_connectors_snapshot(Some(snapshot));
    }

    pub(crate) fn refresh_plugin_mentions(&mut self) {
        if !self.config.features.enabled(Feature::Plugins) {
            self.bottom_pane.set_plugin_mentions(/*plugins*/ None);
            return;
        }

        self.app_event_tx.send(AppEvent::RefreshPluginMentions);
    }

    pub(crate) fn on_plugin_mentions_loaded(
        &mut self,
        plugins: Option<Vec<PluginCapabilitySummary>>,
    ) {
        if self.bottom_pane.plugins() == plugins.as_ref() {
            return;
        }
        self.bottom_pane.set_plugin_mentions(plugins);
    }

    pub(crate) fn sync_plugin_mentions_config(&mut self, config: &Config) {
        self.config.features = config.features.clone();
        self.config.config_layer_stack = config.config_layer_stack.clone();
        self.config.realtime = config.realtime.clone();
        self.config.memories = config.memories.clone();
        self.config.terminal_resize_reflow = config.terminal_resize_reflow;
    }

    pub(crate) fn open_review_popup(&mut self) {
        let mut items: Vec<SelectionItem> = Vec::new();

        items.push(SelectionItem {
            name: "Review against a base branch".to_string(),
            description: Some("(PR Style)".into()),
            actions: vec![Box::new({
                let cwd = self.config.cwd.to_path_buf();
                move |tx| {
                    tx.send(AppEvent::OpenReviewBranchPicker(cwd.clone()));
                }
            })],
            dismiss_on_select: false,
            dismiss_parent_on_child_accept: true,
            ..Default::default()
        });

        items.push(SelectionItem {
            name: "Review uncommitted changes".to_string(),
            actions: vec![Box::new(move |tx: &AppEventSender| {
                tx.review(ReviewTarget::UncommittedChanges);
            })],
            dismiss_on_select: true,
            ..Default::default()
        });

        // New: Review a specific commit (opens commit picker)
        items.push(SelectionItem {
            name: "Review a commit".to_string(),
            actions: vec![Box::new({
                let cwd = self.config.cwd.to_path_buf();
                move |tx| {
                    tx.send(AppEvent::OpenReviewCommitPicker(cwd.clone()));
                }
            })],
            dismiss_on_select: false,
            dismiss_parent_on_child_accept: true,
            ..Default::default()
        });

        items.push(SelectionItem {
            name: "Custom review instructions".to_string(),
            actions: vec![Box::new(move |tx| {
                tx.send(AppEvent::OpenReviewCustomPrompt);
            })],
            dismiss_on_select: false,
            dismiss_parent_on_child_accept: true,
            ..Default::default()
        });

        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some("Select a review preset".into()),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            ..Default::default()
        });
    }

    pub(crate) async fn show_review_branch_picker(&mut self, cwd: &Path) {
        let branches = local_git_branches(cwd).await;
        let current_branch = current_branch_name(cwd)
            .await
            .unwrap_or_else(|| "(detached HEAD)".to_string());
        let mut items: Vec<SelectionItem> = Vec::with_capacity(branches.len());

        for option in branches {
            let branch = option.clone();
            items.push(SelectionItem {
                name: format!("{current_branch} -> {branch}"),
                actions: vec![Box::new(move |tx3: &AppEventSender| {
                    tx3.review(ReviewTarget::BaseBranch {
                        branch: branch.clone(),
                    });
                })],
                dismiss_on_select: true,
                search_value: Some(option),
                ..Default::default()
            });
        }

        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some("Select a base branch".to_string()),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            is_searchable: true,
            search_placeholder: Some("Type to search branches".to_string()),
            ..Default::default()
        });
    }

    pub(crate) async fn show_review_commit_picker(&mut self, cwd: &Path) {
        let commits = recent_commits(cwd, /*limit*/ 100).await;

        let mut items: Vec<SelectionItem> = Vec::with_capacity(commits.len());
        for entry in commits {
            let subject = entry.subject.clone();
            let sha = entry.sha.clone();
            let search_val = format!("{subject} {sha}");

            items.push(SelectionItem {
                name: subject.clone(),
                actions: vec![Box::new(move |tx3: &AppEventSender| {
                    tx3.review(ReviewTarget::Commit {
                        sha: sha.clone(),
                        title: Some(subject.clone()),
                    });
                })],
                dismiss_on_select: true,
                search_value: Some(search_val),
                ..Default::default()
            });
        }

        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some("Select a commit to review".to_string()),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            is_searchable: true,
            search_placeholder: Some("Type to search commits".to_string()),
            ..Default::default()
        });
    }

    pub(crate) fn show_review_custom_prompt(&mut self) {
        let tx = self.app_event_tx.clone();
        let view = CustomPromptView::new(
            "Custom review instructions".to_string(),
            "Type instructions and press Enter".to_string(),
            /*initial_text*/ String::new(),
            /*context_label*/ None,
            Box::new(move |prompt: String| {
                let trimmed = prompt.trim().to_string();
                if trimmed.is_empty() {
                    return;
                }
                tx.review(ReviewTarget::Custom {
                    instructions: trimmed,
                });
            }),
        );
        self.bottom_pane.show_view(Box::new(view));
    }

    pub(crate) fn token_usage(&self) -> TokenUsage {
        self.token_info
            .as_ref()
            .map(|ti| ti.total_token_usage.clone())
            .unwrap_or_default()
    }

