// Auto-split nested ChatComposer impl body shard.
    fn is_bang_shell_command(&self) -> bool {
        self.current_text().trim_start().starts_with('!')
    }

    fn shell_mode_footer_line(&self) -> Option<Line<'static>> {
        self.is_bang_shell_command()
            .then_some(())
            .map(|_| Line::from(vec![Span::from("Shell mode").light_red()]))
    }

    /// Applies any due `PasteBurst` flush at time `now`.
    ///
    /// Converts [`PasteBurst::flush_if_due`] results into concrete textarea mutations.
    ///
    /// Callers:
    ///
    /// - UI ticks via [`ChatComposer::flush_paste_burst_if_due`], so held first-chars can render.
    /// - Input handling via [`ChatComposer::handle_input_basic`], so a due burst does not lag.
    fn handle_paste_burst_flush(&mut self, now: Instant) -> bool {
        match self.paste_burst.flush_if_due(now) {
            FlushResult::Paste(pasted) => {
                self.handle_paste(pasted);
                true
            }
            FlushResult::Typed(ch) => {
                self.insert_str(ch.to_string().as_str());
                true
            }
            FlushResult::None => false,
        }
    }

    /// Handles keys that mutate the textarea, including paste-burst detection.
    ///
    /// Acts as the lowest-level keypath for keys that mutate the textarea. It is also where plain
    /// character streams are converted into explicit paste operations on terminals that do not
    /// reliably provide bracketed paste.
    ///
    /// Ordering is important:
    ///
    /// - Always flush any *due* paste burst first so buffered text does not lag behind unrelated
    ///   edits.
    /// - Then handle the incoming key, intercepting only "plain" (no Ctrl/Alt) char input.
    /// - For non-plain keys, flush via `flush_before_modified_input()` before applying the key;
    ///   otherwise `clear_window_after_non_char()` can leave buffered text waiting without a
    ///   timestamp to time out against.
    fn handle_input_basic(&mut self, input: KeyEvent) -> (InputResult, bool) {
        // Ignore key releases here to avoid treating them as additional input
        // (e.g., appending the same character twice via paste-burst logic).
        if !matches!(input.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return (InputResult::None, false);
        }

        self.handle_input_basic_with_time(input, Instant::now())
    }

    fn handle_input_basic_with_time(
        &mut self,
        input: KeyEvent,
        now: Instant,
    ) -> (InputResult, bool) {
        // If we have a buffered non-bracketed paste burst and enough time has
        // elapsed since the last char, flush it before handling a new input.
        self.handle_paste_burst_flush(now);

        if !matches!(input.code, KeyCode::Esc) {
            self.footer_mode = reset_mode_after_activity(self.footer_mode);
        }

        // If we're capturing a burst and receive Enter, accumulate it instead of inserting.
        if matches!(input.code, KeyCode::Enter)
            && !self.disable_paste_burst
            && self.paste_burst.is_active()
            && self.paste_burst.append_newline_if_active(now)
        {
            return (InputResult::None, true);
        }

        // Intercept plain Char inputs to optionally accumulate into a burst buffer.
        //
        // This is intentionally limited to "plain" (no Ctrl/Alt) chars so shortcuts keep their
        // normal semantics, and so we can aggressively flush/clear any burst state when non-char
        // keys are pressed.
        if let KeyEvent {
            code: KeyCode::Char(ch),
            modifiers,
            ..
        } = input
        {
            let has_ctrl_or_alt = has_ctrl_or_alt(modifiers);
            if !has_ctrl_or_alt && !self.disable_paste_burst && self.textarea.allows_paste_burst() {
                // Non-ASCII characters (e.g., from IMEs) can arrive in quick bursts, so avoid
                // holding the first char while still allowing burst detection for paste input.
                if !ch.is_ascii() {
                    return self.handle_non_ascii_char(input, now);
                }

                match self.paste_burst.on_plain_char(ch, now) {
                    CharDecision::BufferAppend => {
                        self.paste_burst.append_char_to_buffer(ch, now);
                        return (InputResult::None, true);
                    }
                    CharDecision::BeginBuffer { retro_chars } => {
                        let cur = self.textarea.cursor();
                        let txt = self.textarea.text();
                        let safe_cur = Self::clamp_to_char_boundary(txt, cur);
                        let before = &txt[..safe_cur];
                        if let Some(grab) =
                            self.paste_burst
                                .decide_begin_buffer(now, before, retro_chars as usize)
                        {
                            if !grab.grabbed.is_empty() {
                                self.textarea.replace_range(grab.start_byte..safe_cur, "");
                            }
                            self.paste_burst.append_char_to_buffer(ch, now);
                            return (InputResult::None, true);
                        }
                        // If decide_begin_buffer opted not to start buffering,
                        // fall through to normal insertion below.
                    }
                    CharDecision::BeginBufferFromPending => {
                        // First char was held; now append the current one.
                        self.paste_burst.append_char_to_buffer(ch, now);
                        return (InputResult::None, true);
                    }
                    CharDecision::RetainFirstChar => {
                        // Keep the first fast char pending momentarily.
                        return (InputResult::None, true);
                    }
                }
            }
            if let Some(pasted) = self.paste_burst.flush_before_modified_input() {
                self.handle_paste(pasted);
            }
        }

        // Flush any buffered burst before applying a non-char input (arrow keys, etc).
        //
        // `clear_window_after_non_char()` clears `last_plain_char_time`. If we cleared that while
        // `PasteBurst.buffer` is non-empty, `flush_if_due()` would no longer have a timestamp to
        // time out against, and the buffered paste could remain stuck until another plain char
        // arrives.
        if !matches!(input.code, KeyCode::Char(_) | KeyCode::Enter)
            && let Some(pasted) = self.paste_burst.flush_before_modified_input()
        {
            self.handle_paste(pasted);
        }
        // For non-char inputs (or after flushing), handle normally.
        // Track element removals so we can drop any corresponding placeholders without scanning
        // the full text. (Placeholders are atomic elements; when deleted, the element disappears.)
        let elements_before = if self.pending_pastes.is_empty()
            && self.attached_images.is_empty()
            && self.remote_image_urls.is_empty()
        {
            None
        } else {
            Some(self.textarea.element_payloads())
        };

        if self.is_bash_mode
            && matches!(input.code, KeyCode::Backspace)
            && self.textarea.cursor() == 0
        {
            self.is_bash_mode = false;
            return (InputResult::None, true);
        }

        self.textarea.input(input);
        self.sync_bash_mode_from_text();

        if let Some(elements_before) = elements_before {
            self.reconcile_deleted_elements(elements_before);
        }

        // Update paste-burst heuristic for plain Char (no Ctrl/Alt) events.
        let crossterm::event::KeyEvent {
            code, modifiers, ..
        } = input;
        match code {
            KeyCode::Char(_) => {
                let has_ctrl_or_alt = has_ctrl_or_alt(modifiers);
                if has_ctrl_or_alt {
                    self.paste_burst.clear_window_after_non_char();
                }
            }
            KeyCode::Enter => {
                // Keep burst window alive (supports blank lines in paste).
            }
            _ => {
                // Other keys: clear burst window (buffer should have been flushed above if needed).
                self.paste_burst.clear_window_after_non_char();
            }
        }

        (InputResult::None, true)
    }

    fn sync_bash_mode_from_text(&mut self) {
        if !self.is_bash_mode && self.textarea.text().starts_with('!') {
            self.textarea.replace_range(0..1, "");
            self.is_bash_mode = true;
        }
    }

    fn reconcile_deleted_elements(&mut self, elements_before: Vec<String>) {
        let elements_after: HashSet<String> =
            self.textarea.element_payloads().into_iter().collect();

        let mut removed_any_image = false;
        for removed in elements_before
            .into_iter()
            .filter(|payload| !elements_after.contains(payload))
        {
            self.pending_pastes.retain(|(ph, _)| ph != &removed);

            if let Some(idx) = self
                .attached_images
                .iter()
                .position(|img| img.placeholder == removed)
            {
                self.attached_images.remove(idx);
                removed_any_image = true;
            }
        }

        if removed_any_image {
            self.relabel_attached_images_and_update_placeholders();
        }
    }

    fn relabel_attached_images_and_update_placeholders(&mut self) {
        for idx in 0..self.attached_images.len() {
            let expected = local_image_label_text(self.remote_image_urls.len() + idx + 1);
            let current = self.attached_images[idx].placeholder.clone();
            if current == expected {
                continue;
            }

            self.attached_images[idx].placeholder = expected.clone();
            let _renamed = self.textarea.replace_element_payload(&current, &expected);
        }
    }

    /// Handle the dedicated shortcut-overlay toggle key(s).
    ///
    /// This only toggles when the composer is empty and no paste burst is in
    /// progress, so typing/pasting `?` still inserts text instead of opening
    /// help. The bound key list intentionally supports terminal-variant
    /// modifier reporting (for example `?` vs `shift-?`).
    fn handle_shortcut_overlay_key(&mut self, key_event: &KeyEvent) -> bool {
        if key_event.kind != KeyEventKind::Press {
            return false;
        }

        let toggles = self.toggle_shortcuts_keys.is_pressed(*key_event)
            && self.is_empty()
            && !self.is_in_paste_burst();

        if !toggles {
            return false;
        }

        let next = toggle_shortcut_mode(
            self.footer_mode,
            self.quit_shortcut_hint_visible(),
            self.is_empty(),
        );
        let changed = next != self.footer_mode;
        self.footer_mode = next;
        changed
    }

    fn footer_props(&self) -> FooterProps {
        let mode = self.footer_mode();
        let is_wsl = {
            #[cfg(target_os = "linux")]
            {
                mode == FooterMode::ShortcutOverlay && crate::clipboard_paste::is_probably_wsl()
            }
            #[cfg(not(target_os = "linux"))]
            {
                false
            }
        };

        FooterProps {
            mode,
            esc_backtrack_hint: self.esc_backtrack_hint,
            use_shift_enter_hint: self.use_shift_enter_hint,
            is_task_running: self.is_task_running,
            quit_shortcut_key: self.quit_shortcut_key,
            collaboration_modes_enabled: self.collaboration_modes_enabled,
            is_wsl,
            status_line_value: self.status_line_value.clone(),
            status_line_enabled: self.status_line_enabled,
            key_hints: FooterKeyHints {
                toggle_shortcuts: self.footer_toggle_shortcuts_key,
                queue: self.footer_queue_key,
                insert_newline: self.footer_insert_newline_key,
                external_editor: self.footer_external_editor_key,
                edit_previous: Some(key_hint::plain(KeyCode::Esc)),
                show_transcript: self.footer_show_transcript_key,
                history_search: self.footer_history_search_key,
                reasoning_down: self.footer_reasoning_down_key,
                reasoning_up: self.footer_reasoning_up_key,
            },
            active_agent_label: self.active_agent_label.clone(),
        }
    }

    /// Resolve the effective footer mode via a small priority waterfall.
    ///
    /// The base mode is derived solely from whether the composer is empty:
    /// `ComposerEmpty` iff empty, otherwise `ComposerHasDraft`. Transient
    /// modes (Esc hint, overlay, quit reminder) can override that base when
    /// their conditions are active.
    fn footer_mode(&self) -> FooterMode {
        if self.history_search.is_some() {
            return FooterMode::HistorySearch;
        }

        let base_mode = if self.is_empty() {
            FooterMode::ComposerEmpty
        } else {
            FooterMode::ComposerHasDraft
        };

        match self.footer_mode {
            FooterMode::HistorySearch => FooterMode::HistorySearch,
            FooterMode::EscHint => FooterMode::EscHint,
            FooterMode::ShortcutOverlay => FooterMode::ShortcutOverlay,
            FooterMode::QuitShortcutReminder if self.quit_shortcut_hint_visible() => {
                FooterMode::QuitShortcutReminder
            }
            FooterMode::ComposerEmpty | FooterMode::ComposerHasDraft
                if self.quit_shortcut_hint_visible() =>
            {
                FooterMode::QuitShortcutReminder
            }
            FooterMode::QuitShortcutReminder => base_mode,
            FooterMode::ComposerEmpty | FooterMode::ComposerHasDraft => base_mode,
        }
    }

    fn custom_footer_height(&self) -> Option<u16> {
        if self.footer_flash_visible() {
            return Some(1);
        }
        self.footer_hint_override
            .as_ref()
            .map(|items| if items.is_empty() { 0 } else { 1 })
    }

    pub(crate) fn sync_popups(&mut self) {
        self.sync_slash_command_elements();
        if self.history_search.is_some() {
            if self.current_file_query.is_some() {
                self.app_event_tx
                    .send(AppEvent::StartFileSearch(String::new()));
                self.current_file_query = None;
            }
            self.active_popup = ActivePopup::None;
            self.dismissed_file_popup_token = None;
            self.dismissed_mention_popup_token = None;
            return;
        }
        if !self.popups_enabled() {
            self.active_popup = ActivePopup::None;
            return;
        }
        let file_token = Self::current_at_token(&self.textarea);
        let browsing_history = self
            .history
            .should_handle_navigation(&self.current_text(), self.history_navigation_cursor());
        // When browsing input history (shell-style Up/Down recall), skip all popup
        // synchronization so nothing steals focus from continued history navigation.
        if browsing_history {
            if self.current_file_query.is_some() {
                self.app_event_tx
                    .send(AppEvent::StartFileSearch(String::new()));
                self.current_file_query = None;
            }
            self.active_popup = ActivePopup::None;
            return;
        }
        let mention_token = self.current_mention_token();

        let allow_command_popup = self.slash_commands_enabled()
            && !self.is_bash_mode
            && file_token.is_none()
            && mention_token.is_none();
        self.sync_command_popup(allow_command_popup);

        if matches!(self.active_popup, ActivePopup::Command(_)) {
            if self.current_file_query.is_some() {
                self.app_event_tx
                    .send(AppEvent::StartFileSearch(String::new()));
                self.current_file_query = None;
            }
            self.dismissed_file_popup_token = None;
            self.dismissed_mention_popup_token = None;
            return;
        }

        if let Some(token) = mention_token {
            if self.current_file_query.is_some() {
                self.app_event_tx
                    .send(AppEvent::StartFileSearch(String::new()));
                self.current_file_query = None;
            }
            self.sync_mention_popup(token);
            return;
        }
        self.dismissed_mention_popup_token = None;

        if let Some(token) = file_token {
            self.sync_file_search_popup(token);
            return;
        }

        if self.current_file_query.is_some() {
            self.app_event_tx
                .send(AppEvent::StartFileSearch(String::new()));
            self.current_file_query = None;
        }
        self.dismissed_file_popup_token = None;
        if matches!(
            self.active_popup,
            ActivePopup::File(_) | ActivePopup::Skill(_)
        ) {
            self.active_popup = ActivePopup::None;
        }
    }

    /// Keep slash command elements aligned with the current first line.
    fn sync_slash_command_elements(&mut self) {
        if !self.slash_commands_enabled() {
            return;
        }
        let text = self.textarea.text();
        let first_line_end = text.find('\n').unwrap_or(text.len());
        let first_line = &text[..first_line_end];
        let desired_range = self.slash_command_element_range(first_line);
        // Slash commands are only valid at byte 0 of the first line.
        // Any slash-shaped element not matching the current desired prefix is stale.
        let mut has_desired = false;
        let mut stale_ranges = Vec::new();
        for elem in self.textarea.text_elements() {
            let Some(payload) = elem.placeholder(text) else {
                continue;
            };
            if payload.strip_prefix('/').is_none() {
                continue;
            }
            let range = elem.byte_range.start..elem.byte_range.end;
            if desired_range.as_ref() == Some(&range) {
                has_desired = true;
            } else {
                stale_ranges.push(range);
            }
        }

        for range in stale_ranges {
            self.textarea.remove_element_range(range);
        }

        if let Some(range) = desired_range
            && !has_desired
        {
            self.textarea.add_element_range(range);
        }
    }

    fn slash_command_element_range(&self, first_line: &str) -> Option<Range<usize>> {
        if self.is_bash_mode {
            return None;
        }
        let (name, _rest, _rest_offset) = parse_slash_name(first_line)?;
        if name.contains('/') {
            return None;
        }
        let element_end = 1 + name.len();
        let has_space_after = first_line
            .get(element_end..)
            .and_then(|tail| tail.chars().next())
            .is_some_and(char::is_whitespace);
        if !has_space_after {
            return None;
        }
        if self.is_known_slash_name(name) {
            Some(0..element_end)
        } else {
            None
        }
    }

    fn is_known_slash_name(&self, name: &str) -> bool {
        slash_commands::find_builtin_command(name, self.builtin_command_flags()).is_some()
    }

    /// If the cursor is currently within a slash command on the first line,
    /// extract the command name and the rest of the line after it.
    /// Returns None if the cursor is outside a slash command.
    fn slash_command_under_cursor(first_line: &str, cursor: usize) -> Option<(&str, &str)> {
        if !first_line.starts_with('/') {
            return None;
        }

        let name_start = 1usize;
        let name_end = first_line[name_start..]
            .find(char::is_whitespace)
            .map(|idx| name_start + idx)
            .unwrap_or_else(|| first_line.len());

        if cursor > name_end {
            return None;
        }

        let name = &first_line[name_start..name_end];
        let rest_start = first_line[name_end..]
            .find(|c: char| !c.is_whitespace())
            .map(|idx| name_end + idx)
            .unwrap_or(name_end);
        let rest = &first_line[rest_start..];

        Some((name, rest))
    }

    /// Heuristic for whether the typed slash command looks like a valid
    /// prefix for any known built-in command.
    /// Empty names only count when there is no extra content after the '/'.
    fn looks_like_slash_prefix(&self, name: &str, rest_after_name: &str) -> bool {
        if !self.slash_commands_enabled() {
            return false;
        }
        if name.is_empty() {
            return rest_after_name.is_empty();
        }

        slash_commands::has_builtin_prefix(name, self.builtin_command_flags())
    }

    /// Synchronize `self.command_popup` with the current text in the
    /// textarea. This must be called after every modification that can change
    /// the text so the popup is shown/updated/hidden as appropriate.
    fn sync_command_popup(&mut self, allow: bool) {
        if !allow {
            if matches!(self.active_popup, ActivePopup::Command(_)) {
                self.active_popup = ActivePopup::None;
            }
            return;
        }
        // Determine whether the caret is inside the initial '/name' token on the first line.
        let text = self.textarea.text();
        let first_line_end = text.find('\n').unwrap_or(text.len());
        let first_line = &text[..first_line_end];
        let cursor = self.textarea.cursor();
        let caret_on_first_line = cursor <= first_line_end;

        let is_editing_slash_command_name = caret_on_first_line
            && Self::slash_command_under_cursor(first_line, cursor)
                .is_some_and(|(name, rest)| self.looks_like_slash_prefix(name, rest));

        // If the cursor is currently positioned within an `@token`, prefer the
        // file-search popup over the slash popup so users can insert a file path
        // as an argument to the command (e.g., "/review @docs/...").
        if Self::current_at_token(&self.textarea).is_some() {
            if matches!(self.active_popup, ActivePopup::Command(_)) {
                self.active_popup = ActivePopup::None;
            }
            return;
        }
        match &mut self.active_popup {
            ActivePopup::Command(popup) => {
                if is_editing_slash_command_name {
                    popup.on_composer_text_change(first_line.to_string());
                } else {
                    self.active_popup = ActivePopup::None;
                }
            }
            _ => {
                if is_editing_slash_command_name {
                    let collaboration_modes_enabled = self.collaboration_modes_enabled;
                    let connectors_enabled = self.connectors_enabled;
                    let plugins_command_enabled = self.plugins_command_enabled;
                    let fast_command_enabled = self.fast_command_enabled;
                    let goal_command_enabled = self.goal_command_enabled;
                    let personality_command_enabled = self.personality_command_enabled;
                    let mut command_popup = CommandPopup::new(CommandPopupFlags {
                        collaboration_modes_enabled,
                        connectors_enabled,
                        plugins_command_enabled,
                        fast_command_enabled,
                        goal_command_enabled,
                        personality_command_enabled,
                        windows_degraded_sandbox_active: self.windows_degraded_sandbox_active,
                        side_conversation_active: self.side_conversation_active,
                    });
                    command_popup.on_composer_text_change(first_line.to_string());
                    self.active_popup = ActivePopup::Command(command_popup);
                }
            }
        }
    }

    /// Synchronize `self.file_search_popup` with the current text in the textarea.
    /// Note this is only called when self.active_popup is NOT Command.
    fn sync_file_search_popup(&mut self, query: String) {
        // If user dismissed popup for this exact query, don't reopen until text changes.
        if self.dismissed_file_popup_token.as_ref() == Some(&query) {
            return;
        }

        if query.is_empty() {
            self.app_event_tx
                .send(AppEvent::StartFileSearch(String::new()));
        } else {
            self.app_event_tx
                .send(AppEvent::StartFileSearch(query.clone()));
        }

        match &mut self.active_popup {
            ActivePopup::File(popup) => {
                if query.is_empty() {
                    popup.set_empty_prompt();
                } else {
                    popup.set_query(&query);
                }
            }
            _ => {
                let mut popup = FileSearchPopup::new();
                if query.is_empty() {
                    popup.set_empty_prompt();
                } else {
                    popup.set_query(&query);
                }
                self.active_popup = ActivePopup::File(popup);
            }
        }

        if query.is_empty() {
            self.current_file_query = None;
        } else {
            self.current_file_query = Some(query);
        }
        self.dismissed_file_popup_token = None;
    }

