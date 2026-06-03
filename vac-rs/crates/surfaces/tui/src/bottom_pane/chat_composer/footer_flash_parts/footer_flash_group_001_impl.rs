// O5/O6 balanced ChatComposer impl group: source chat_composer_impl_00_000.rs
impl ChatComposer {
// Auto-split ChatComposer impl body.

    fn builtin_command_flags(&self) -> BuiltinCommandFlags {
        BuiltinCommandFlags {
            collaboration_modes_enabled: self.collaboration_modes_enabled,
            connectors_enabled: self.connectors_enabled,
            plugins_command_enabled: self.plugins_command_enabled,
            fast_command_enabled: self.fast_command_enabled,
            goal_command_enabled: self.goal_command_enabled,
            personality_command_enabled: self.personality_command_enabled,
            realtime_conversation_enabled: self.realtime_conversation_enabled,
            audio_device_selection_enabled: self.audio_device_selection_enabled,
            allow_elevate_sandbox: self.windows_degraded_sandbox_active,
            side_conversation_active: self.side_conversation_active,
        }
    }

    fn surface_route_catalog(&self) -> SurfaceRouteCatalog {
        std::env::current_dir()
            .ok()
            .map(SurfaceRouteCatalog::load)
            .unwrap_or_default()
    }

    fn find_declared_builtin_command(&self, name: &str) -> Option<SlashCommand> {
        let catalog = self.surface_route_catalog();
        slash_commands::find_builtin_command_with_catalog(
            name,
            self.builtin_command_flags(),
            &catalog,
        )
    }

    pub fn new(
        has_input_focus: bool,
        app_event_tx: AppEventSender,
        enhanced_keys_supported: bool,
        placeholder_text: String,
        disable_paste_burst: bool,
    ) -> Self {
        Self::new_with_config(
            has_input_focus,
            app_event_tx,
            enhanced_keys_supported,
            placeholder_text,
            disable_paste_burst,
            ChatComposerConfig::default(),
        )
    }

    /// Construct a composer with explicit feature gating.
    ///
    /// This enables reuse in contexts like request-user-input where we want
    /// the same visuals and editing behavior without slash commands or popups.
    pub(crate) fn new_with_config(
        has_input_focus: bool,
        app_event_tx: AppEventSender,
        enhanced_keys_supported: bool,
        placeholder_text: String,
        disable_paste_burst: bool,
        config: ChatComposerConfig,
    ) -> Self {
        let use_shift_enter_hint = enhanced_keys_supported;
        let default_keymap = RuntimeKeymap::defaults();
        let default_editor_keymap = default_keymap.editor.clone();
        let default_vim_normal_keymap = default_keymap.vim_normal.clone();

        let mut this = Self {
            textarea: TextArea::new(),
            textarea_state: RefCell::new(TextAreaState::default()),
            is_bash_mode: false,
            active_popup: ActivePopup::None,
            app_event_tx,
            history: ChatComposerHistory::new(),
            quit_shortcut_expires_at: None,
            quit_shortcut_key: key_hint::ctrl(KeyCode::Char('c')),
            esc_backtrack_hint: false,
            use_shift_enter_hint,
            dismissed_file_popup_token: None,
            current_file_query: None,
            pending_pastes: Vec::new(),
            large_paste_counters: HashMap::new(),
            has_focus: has_input_focus,
            frame_requester: None,
            attached_images: Vec::new(),
            placeholder_text,
            is_task_running: false,
            input_enabled: true,
            input_disabled_placeholder: None,
            paste_burst: PasteBurst::default(),
            disable_paste_burst: false,
            footer_mode: FooterMode::ComposerEmpty,
            footer_hint_override: None,
            plan_mode_nudge_visible: false,
            remote_image_urls: Vec::new(),
            selected_remote_image_index: None,
            pending_slash_command_history: None,
            footer_flash: None,
            context_window_percent: None,
            #[cfg(not(target_os = "linux"))]
            next_element_id: 0,
            context_window_used_tokens: None,
            skills: None,
            plugins: None,
            connectors_snapshot: None,
            dismissed_mention_popup_token: None,
            mention_bindings: HashMap::new(),
            recent_submission_mention_bindings: Vec::new(),
            collaboration_modes_enabled: false,
            config,
            collaboration_mode_indicator: None,
            goal_status_indicator: None,
            ide_context_active: false,
            connectors_enabled: false,
            plugins_command_enabled: false,
            fast_command_enabled: false,
            goal_command_enabled: false,
            personality_command_enabled: false,
            realtime_conversation_enabled: false,
            audio_device_selection_enabled: false,
            windows_degraded_sandbox_active: false,
            side_conversation_active: false,
            is_zellij: matches!(
                vac_terminal_detection::terminal_info().multiplexer,
                Some(vac_terminal_detection::Multiplexer::Zellij {})
            ),
            status_line_value: None,
            status_line_enabled: false,
            side_conversation_context_label: None,
            active_agent_label: None,
            history_search: None,
            submit_keys: vec![key_hint::plain(KeyCode::Enter)],
            queue_keys: vec![key_hint::plain(KeyCode::Tab)],
            toggle_shortcuts_keys: vec![
                key_hint::plain(KeyCode::Char('?')),
                key_hint::shift(KeyCode::Char('?')),
            ],
            history_search_previous_keys: default_keymap.composer.history_search_previous.clone(),
            history_search_next_keys: default_keymap.composer.history_search_next.clone(),
            editor_keymap: default_editor_keymap,
            vim_normal_keymap: default_vim_normal_keymap,
            footer_external_editor_key: Some(key_hint::ctrl(KeyCode::Char('g'))),
            footer_show_transcript_key: Some(key_hint::ctrl(KeyCode::Char('t'))),
            footer_insert_newline_key: footer_insert_newline_key(
                &default_keymap.editor.insert_newline,
                use_shift_enter_hint,
            ),
            footer_queue_key: Some(key_hint::plain(KeyCode::Tab)),
            footer_toggle_shortcuts_key: Some(key_hint::plain(KeyCode::Char('?'))),
            footer_history_search_key: primary_binding(
                &default_keymap.composer.history_search_previous,
            ),
            footer_reasoning_down_key: primary_binding(
                &default_keymap.chat.decrease_reasoning_effort,
            ),
            footer_reasoning_up_key: primary_binding(
                &default_keymap.chat.increase_reasoning_effort,
            ),
        };
        // Apply configuration via the setter to keep side-effects centralized.
        this.set_disable_paste_burst(disable_paste_burst);
        this
    }

    #[cfg(not(target_os = "linux"))]
    fn next_id(&mut self) -> String {
        let id = self.next_element_id;
        self.next_element_id = self.next_element_id.wrapping_add(1);
        id.to_string()
    }

    pub(crate) fn set_frame_requester(&mut self, frame_requester: FrameRequester) {
        self.frame_requester = Some(frame_requester);
    }

    pub fn set_skill_mentions(&mut self, skills: Option<Vec<SkillMetadata>>) {
        self.skills = skills;
        self.sync_popups();
    }

    pub fn set_plugin_mentions(&mut self, plugins: Option<Vec<PluginCapabilitySummary>>) {
        self.plugins = plugins;
        self.sync_popups();
    }

    pub fn set_plugins_command_enabled(&mut self, enabled: bool) {
        self.plugins_command_enabled = enabled;
    }

    /// Toggle composer-side image paste handling.
    ///
    /// This only affects whether image-like paste content is converted into attachments; the
    /// `ChatWidget` layer still performs capability checks before images are submitted.
    pub fn set_image_paste_enabled(&mut self, enabled: bool) {
        self.config.image_paste_enabled = enabled;
    }

    pub fn set_connector_mentions(&mut self, connectors_snapshot: Option<ConnectorsSnapshot>) {
        self.connectors_snapshot = connectors_snapshot;
        self.sync_popups();
    }

    pub(crate) fn take_mention_bindings(&mut self) -> Vec<MentionBinding> {
        let elements = self.current_mention_elements();
        let mut ordered = Vec::new();
        for (id, mention) in elements {
            if let Some(binding) = self.mention_bindings.remove(&id)
                && binding.mention == mention
            {
                ordered.push(MentionBinding {
                    mention: binding.mention,
                    path: binding.path,
                });
            }
        }
        self.mention_bindings.clear();
        ordered
    }

    pub fn set_collaboration_modes_enabled(&mut self, enabled: bool) {
        self.collaboration_modes_enabled = enabled;
    }

    pub fn set_connectors_enabled(&mut self, enabled: bool) {
        self.connectors_enabled = enabled;
    }

    pub fn set_fast_command_enabled(&mut self, enabled: bool) {
        self.fast_command_enabled = enabled;
    }

    pub fn set_goal_command_enabled(&mut self, enabled: bool) {
        self.goal_command_enabled = enabled;
    }

    /// Replace composer, editor, and footer-hint key bindings from one runtime snapshot.
    ///
    /// Submit and queue bindings are cached here because composer dispatch must
    /// check them before generic textarea editing. The embedded textarea receives
    /// the same snapshot's editor bindings so a live remap cannot leave submit
    /// keys updated while cursor/editing keys still use old defaults.
    pub(crate) fn set_keymap_bindings(&mut self, keymap: &RuntimeKeymap) {
        self.submit_keys = keymap.composer.submit.clone();
        self.queue_keys = keymap.composer.queue.clone();
        self.toggle_shortcuts_keys = keymap.composer.toggle_shortcuts.clone();
        self.history_search_previous_keys = keymap.composer.history_search_previous.clone();
        self.history_search_next_keys = keymap.composer.history_search_next.clone();
        self.editor_keymap = keymap.editor.clone();
        self.vim_normal_keymap = keymap.vim_normal.clone();
        self.textarea.set_keymap_bindings(keymap);
        self.footer_external_editor_key = primary_binding(&keymap.app.open_external_editor);
        self.footer_show_transcript_key = primary_binding(&keymap.app.open_transcript);
        self.footer_insert_newline_key =
            footer_insert_newline_key(&keymap.editor.insert_newline, self.use_shift_enter_hint);
        self.footer_queue_key = primary_binding(&keymap.composer.queue);
        self.footer_toggle_shortcuts_key = primary_binding(&keymap.composer.toggle_shortcuts);
        self.footer_history_search_key = primary_binding(&keymap.composer.history_search_previous);
        self.footer_reasoning_down_key = primary_binding(&keymap.chat.decrease_reasoning_effort);
        self.footer_reasoning_up_key = primary_binding(&keymap.chat.increase_reasoning_effort);
    }

    pub fn set_collaboration_mode_indicator(
        &mut self,
        indicator: Option<CollaborationModeIndicator>,
    ) {
        self.collaboration_mode_indicator = indicator;
    }

    pub fn set_goal_status_indicator(&mut self, indicator: Option<GoalStatusIndicator>) {
        self.goal_status_indicator = indicator;
    }

    pub fn set_ide_context_active(&mut self, active: bool) {
        self.ide_context_active = active;
    }

    pub fn set_personality_command_enabled(&mut self, enabled: bool) {
        self.personality_command_enabled = enabled;
    }

    pub fn set_realtime_conversation_enabled(&mut self, enabled: bool) {
        self.realtime_conversation_enabled = enabled;
    }

    pub fn set_audio_device_selection_enabled(&mut self, enabled: bool) {
        self.audio_device_selection_enabled = enabled;
    }

    pub fn set_side_conversation_active(&mut self, active: bool) {
        self.side_conversation_active = active;
    }

    /// Compatibility shim for tests that still toggle the removed steer mode flag.
    #[cfg(test)]
    pub fn set_steer_enabled(&mut self, _enabled: bool) {}
    /// Centralized feature gating keeps config checks out of call sites.
    fn popups_enabled(&self) -> bool {
        self.config.popups_enabled
    }

    fn slash_commands_enabled(&self) -> bool {
        self.config.slash_commands_enabled
    }

    fn image_paste_enabled(&self) -> bool {
        self.config.image_paste_enabled
    }
    #[cfg(target_os = "windows")]
    pub fn set_windows_degraded_sandbox_active(&mut self, enabled: bool) {
        self.windows_degraded_sandbox_active = enabled;
    }
    fn layout_areas(&self, area: Rect) -> [Rect; 4] {
        let footer_props = self.footer_props();
        let footer_hint_height = self
            .custom_footer_height()
            .unwrap_or_else(|| footer_height(&footer_props));
        let footer_spacing = Self::footer_spacing(footer_hint_height);
        let footer_total_height = footer_hint_height + footer_spacing;
        let popup_constraint = match &self.active_popup {
            ActivePopup::Command(popup) => {
                Constraint::Max(popup.calculate_required_height(area.width))
            }
            ActivePopup::File(popup) => Constraint::Max(popup.calculate_required_height()),
            ActivePopup::Skill(popup) => {
                Constraint::Max(popup.calculate_required_height(area.width))
            }
            ActivePopup::None => Constraint::Max(footer_total_height),
        };
        let [composer_rect, popup_rect] =
            Layout::vertical([Constraint::Min(3), popup_constraint]).areas(area);
        let mut textarea_rect = composer_rect.inset(Insets::tlbr(
            /*top*/ 1,
            LIVE_PREFIX_COLS,
            /*bottom*/ 1,
            /*right*/ 1,
        ));
        let remote_images_height = self
            .remote_images_lines(textarea_rect.width)
            .len()
            .try_into()
            .unwrap_or(u16::MAX)
            .min(textarea_rect.height.saturating_sub(1));
        let remote_images_separator = u16::from(remote_images_height > 0);
        let consumed = remote_images_height.saturating_add(remote_images_separator);
        let remote_images_rect = Rect {
            x: textarea_rect.x,
            y: textarea_rect.y,
            width: textarea_rect.width,
            height: remote_images_height,
        };
        textarea_rect.y = textarea_rect.y.saturating_add(consumed);
        textarea_rect.height = textarea_rect.height.saturating_sub(consumed);
        [composer_rect, remote_images_rect, textarea_rect, popup_rect]
    }

    fn footer_spacing(footer_hint_height: u16) -> u16 {
        if footer_hint_height == 0 {
            0
        } else {
            FOOTER_SPACING_HEIGHT
        }
    }

    pub fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        if !self.input_enabled {
            return None;
        }

        if let Some(pos) = self.history_search_cursor_pos(area) {
            return Some(pos);
        }

        let [_, _, textarea_rect, _] = self.layout_areas(area);
        let state = *self.textarea_state.borrow();
        self.textarea.cursor_pos_with_state(textarea_rect, state)
    }
    /// Returns true if the composer currently contains no user-entered input.
    pub(crate) fn is_empty(&self) -> bool {
        self.textarea.is_empty()
            && !self.is_bash_mode
            && self.attached_images.is_empty()
            && self.remote_image_urls.is_empty()
    }

    /// Record the history metadata advertised by `SessionConfiguredEvent` so
    /// that the composer can navigate cross-session history.
    pub(crate) fn set_history_metadata(&mut self, log_id: u64, entry_count: usize) {
        self.history.set_metadata(log_id, entry_count);
    }

    /// Integrate an asynchronous response to an on-demand history lookup.
    ///
    /// If the entry is present and the offset still matches the active history cursor, the
    /// composer rehydrates the entry immediately. This path intentionally routes through
    /// [`Self::apply_history_entry`] so cursor placement remains aligned with keyboard history
    /// recall semantics.
    pub(crate) fn on_history_entry_response(
        &mut self,
        log_id: u64,
        offset: usize,
        entry: Option<String>,
    ) -> bool {
        match self
            .history
            .on_entry_response(log_id, offset, entry, &self.app_event_tx)
        {
            HistoryEntryResponse::Found(entry) => {
                // Persistent ↑/↓ history is text-only (backwards-compatible and avoids persisting
                // attachments), but local in-session ↑/↓ history can rehydrate elements and image paths.
                self.apply_history_entry(entry);
                true
            }
            HistoryEntryResponse::Search(result) => {
                self.apply_history_search_result(result);
                true
            }
            HistoryEntryResponse::Ignored => false,
        }
    }

    /// Integrate pasted text into the composer.
    ///
    /// Acts as the only place where paste text is integrated, both for:
    ///
    /// - Real/explicit paste events surfaced by the terminal, and
    /// - Non-bracketed "paste bursts" that [`PasteBurst`](super::paste_burst::PasteBurst) buffers
    ///   and later flushes here.
    ///
    /// Behavior:
    ///
    /// - If the paste is larger than `LARGE_PASTE_CHAR_THRESHOLD` chars, inserts a placeholder
    ///   element (expanded on submit) and stores the full text in `pending_pastes`.
    /// - Otherwise, if the paste looks like an image path, attaches the image and inserts a
    ///   trailing space so the user can keep typing naturally.
    /// - Otherwise, inserts the pasted text directly into the textarea.
    ///
    /// In all cases, clears any paste-burst Enter suppression state so a real paste cannot affect
    /// the next user Enter key, then syncs popup state.
    pub fn handle_paste(&mut self, pasted: String) -> bool {
        let pasted = pasted.replace("\r\n", "\n").replace('\r', "\n");
        let char_count = pasted.chars().count();
        if char_count > LARGE_PASTE_CHAR_THRESHOLD {
            let placeholder = self.next_large_paste_placeholder(char_count);
            self.textarea.insert_element(&placeholder);
            self.pending_pastes.push((placeholder, pasted));
        } else if char_count > 1
            && self.image_paste_enabled()
            && self.handle_paste_image_path(pasted.clone())
        {
            self.textarea.insert_str(" ");
        } else {
            self.insert_str(&pasted);
        }
        self.paste_burst.clear_after_explicit_paste();
        self.sync_popups();
        true
    }

    pub fn handle_paste_image_path(&mut self, pasted: String) -> bool {
        let Some(path_buf) = normalize_pasted_path(&pasted) else {
            return false;
        };

        // normalize_pasted_path already handles Windows → WSL path conversion,
        // so we can directly try to read the image dimensions.
        match image::image_dimensions(&path_buf) {
            Ok((width, height)) => {
                tracing::info!("OK: {pasted}");
                tracing::debug!("image dimensions={}x{}", width, height);
                let format = pasted_image_format(&path_buf);
                tracing::debug!("attached image format={}", format.label());
                self.attach_image(path_buf);
                true
            }
            Err(err) => {
                tracing::trace!("ERR: {err}");
                false
            }
        }
    }

    /// Enable or disable paste-burst handling.
    ///
    /// `disable_paste_burst` is an escape hatch for terminals/platforms where the burst heuristic
    /// is unwanted or has already been handled elsewhere.
    ///
    /// When transitioning from enabled → disabled, we "defuse" any in-flight burst state so it
    /// cannot affect subsequent normal typing:
    ///
    /// - First, flush any held/buffered text immediately via
    ///   [`PasteBurst::flush_before_modified_input`], and feed it through `handle_paste(String)`.
    ///   This preserves user input and routes it through the same integration path as explicit
    ///   pastes (large-paste placeholders, image-path detection, and popup sync).
    /// - Then clear the burst timing and Enter-suppression window via
    ///   [`PasteBurst::clear_after_explicit_paste`].
    ///
    /// We intentionally do not use `clear_window_after_non_char()` here: it clears timing state
    /// without emitting any buffered text, which can leave a non-empty buffer unable to flush
    /// later (because `flush_if_due()` relies on `last_plain_char_time` to time out).
    pub(crate) fn set_disable_paste_burst(&mut self, disabled: bool) {
        let was_disabled = self.disable_paste_burst;
        self.disable_paste_burst = disabled;
        if disabled && !was_disabled {
            if let Some(pasted) = self.paste_burst.flush_before_modified_input() {
                self.handle_paste(pasted);
            }
            self.paste_burst.clear_after_explicit_paste();
        }
    }

    /// Replace the composer content with text from an external editor.
    /// Clears pending paste placeholders and keeps only attachments whose
    /// placeholder labels still appear in the new text. Image placeholders
    /// are renumbered to `[Image #M+1]..[Image #N]` (where `M` is the number of
    /// remote images). Cursor is placed at the end after rebuilding elements.
    pub(crate) fn apply_external_edit(&mut self, text: String) {
        self.pending_pastes.clear();
        let (text, _) = self.imported_text_for_textarea(text, Vec::new());

        // Count placeholder occurrences in the new text.
        let mut placeholder_counts: HashMap<String, usize> = HashMap::new();
        for placeholder in self.attached_images.iter().map(|img| &img.placeholder) {
            if placeholder_counts.contains_key(placeholder) {
                continue;
            }
            let count = text.match_indices(placeholder).count();
            if count > 0 {
                placeholder_counts.insert(placeholder.clone(), count);
            }
        }

        // Keep attachments only while we have matching occurrences left.
        let mut kept_images = Vec::new();
        for img in self.attached_images.drain(..) {
            if let Some(count) = placeholder_counts.get_mut(&img.placeholder)
                && *count > 0
            {
                *count -= 1;
                kept_images.push(img);
            }
        }
        self.attached_images = kept_images;

        // Rebuild textarea so placeholders become elements again.
        self.textarea.set_text_clearing_elements("");
        let mut remaining: HashMap<&str, usize> = HashMap::new();
        for img in &self.attached_images {
            *remaining.entry(img.placeholder.as_str()).or_insert(0) += 1;
        }

        let mut occurrences: Vec<(usize, &str)> = Vec::new();
        for placeholder in remaining.keys() {
            for (pos, _) in text.match_indices(placeholder) {
                occurrences.push((pos, *placeholder));
            }
        }
        occurrences.sort_unstable_by_key(|(pos, _)| *pos);

        let mut idx = 0usize;
        for (pos, ph) in occurrences {
            let Some(count) = remaining.get_mut(ph) else {
                continue;
            };
            if *count == 0 {
                continue;
            }
            if pos > idx {
                self.textarea.insert_str(&text[idx..pos]);
            }
            self.textarea.insert_element(ph);
            *count -= 1;
            idx = pos + ph.len();
        }
        if idx < text.len() {
            self.textarea.insert_str(&text[idx..]);
        }

        // Keep local image placeholders normalized in attachment order after the
        // remote-image prefix.
        self.relabel_attached_images_and_update_placeholders();
        self.textarea.set_cursor(self.textarea.text().len());
        self.sync_popups();
    }

    /// Enable or disable Vim editing for the composer textarea.
    ///
    /// The composer clears any in-flight paste-burst state when the mode
    /// changes because Vim normal mode treats rapid character sequences as
    /// commands, not as candidate literal paste text. It also resets transient
    /// footer mode so the visible hints match the new editing surface.
    pub(crate) fn set_vim_enabled(&mut self, enabled: bool) {
        self.textarea.set_vim_enabled(enabled);
        self.paste_burst.clear_after_explicit_paste();
        self.footer_mode = reset_mode_after_activity(self.footer_mode);
    }

    /// Toggle Vim editing and return the new enabled state.
    ///
    /// This is the app-level command target for the configurable Vim toggle
    /// keybinding; callers should use the returned value for status messages
    /// instead of rereading state after additional composer mutations.
    pub(crate) fn toggle_vim_enabled(&mut self) -> bool {
        let enabled = !self.textarea.is_vim_enabled();
        self.set_vim_enabled(enabled);
        enabled
    }

    /// Return whether Vim editing is enabled for tests that assert mode transitions.
    #[cfg(test)]
    pub(crate) fn is_vim_enabled(&self) -> bool {
        self.textarea.is_vim_enabled()
    }

    /// Return whether Escape should be routed to the textarea before popups.
    ///
    /// Vim insert mode owns Escape as a transition back to normal mode. The app
    /// event layer asks this before running generic Escape behavior so the same
    /// key does not both leave insert mode and dismiss unrelated UI.
    pub(crate) fn should_handle_vim_insert_escape(&self, key_event: KeyEvent) -> bool {
        self.textarea.should_handle_vim_insert_escape(key_event)
    }

    fn vim_mode_indicator_span(&self) -> Option<Span<'static>> {
        self.textarea.vim_mode_label().map(|label| match label {
            "Normal" => "Vim: Normal".magenta(),
            "Insert" => "Vim: Insert".green(),
            _ => unreachable!(),
        })
    }

    fn mode_indicator_line(&self, show_cycle_hint: bool) -> Option<Line<'static>> {
        let mut spans: Vec<Span<'static>> = Vec::new();
        if let Some(vim_mode) = self.vim_mode_indicator_span() {
            spans.push(vim_mode);
        }
        if let Some(indicators) = status_line_right_indicator_line(
            self.collaboration_mode_indicator,
            self.goal_status_indicator.as_ref(),
            self.ide_context_active,
            show_cycle_hint,
        ) {
            if !spans.is_empty() {
                spans.push(" | ".dim());
            }
            spans.extend(indicators.spans);
        }
        if spans.is_empty() {
            None
        } else {
            Some(Line::from(spans))
        }
    }
}
