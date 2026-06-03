// O5/O6 balanced ChatComposer impl group: source chat_composer_impl_00_001.rs
impl ChatComposer {
// Auto-split ChatComposer impl body.
    fn right_footer_line_with_context(&self) -> Line<'static> {
        let mut line =
            context_window_line(self.context_window_percent, self.context_window_used_tokens);
        if let Some(vim_mode) = self.vim_mode_indicator_span() {
            line.spans.push(" | ".dim());
            line.spans.push(vim_mode);
        }
        line
    }

    pub(crate) fn current_text_with_pending(&self) -> String {
        let mut text = self.current_text();
        for (placeholder, actual) in &self.pending_pastes {
            if text.contains(placeholder) {
                text = text.replace(placeholder, actual);
            }
        }
        text
    }

    /// Returns whether the composer currently accepts interactive draft edits.
    pub(crate) fn input_enabled(&self) -> bool {
        self.input_enabled
    }

    pub(crate) fn pending_pastes(&self) -> Vec<(String, String)> {
        self.pending_pastes.clone()
    }

    pub(crate) fn set_pending_pastes(&mut self, pending_pastes: Vec<(String, String)>) {
        let text = self.current_text();
        self.pending_pastes = pending_pastes
            .into_iter()
            .filter(|(placeholder, _)| text.contains(placeholder))
            .collect();
    }

    /// Override the footer hint items displayed beneath the composer. Passing
    /// `None` restores the default shortcut footer.
    pub(crate) fn set_footer_hint_override(&mut self, items: Option<Vec<(String, String)>>) {
        self.footer_hint_override = items;
    }

    /// Updates whether the Plan-mode nudge replaces the ambient footer row.
    ///
    /// Returns `true` only when the rendered footer can change so callers can avoid scheduling
    /// redundant redraws while reevaluating nudge policy on routine composer updates.
    pub(crate) fn set_plan_mode_nudge_visible(&mut self, visible: bool) -> bool {
        if self.plan_mode_nudge_visible == visible {
            return false;
        }
        self.plan_mode_nudge_visible = visible;
        true
    }

    #[cfg(test)]
    pub(crate) fn plan_mode_nudge_visible(&self) -> bool {
        self.plan_mode_nudge_visible
    }

    pub(crate) fn set_remote_image_urls(&mut self, urls: Vec<String>) {
        self.remote_image_urls = urls;
        self.selected_remote_image_index = None;
        self.relabel_attached_images_and_update_placeholders();
        self.sync_popups();
    }

    pub(crate) fn remote_image_urls(&self) -> Vec<String> {
        self.remote_image_urls.clone()
    }

    pub(crate) fn take_remote_image_urls(&mut self) -> Vec<String> {
        let urls = std::mem::take(&mut self.remote_image_urls);
        self.selected_remote_image_index = None;
        self.relabel_attached_images_and_update_placeholders();
        self.sync_popups();
        urls
    }

    #[cfg(test)]
    pub(crate) fn show_footer_flash(&mut self, line: Line<'static>, duration: Duration) {
        let expires_at = Instant::now()
            .checked_add(duration)
            .unwrap_or_else(Instant::now);
        self.footer_flash = Some(FooterFlash { line, expires_at });
    }

    pub(crate) fn footer_flash_visible(&self) -> bool {
        self.footer_flash
            .as_ref()
            .is_some_and(|flash| Instant::now() < flash.expires_at)
    }

    /// Replace the entire composer content with `text` and reset cursor.
    ///
    /// This is the "fresh draft" path: it clears pending paste payloads and
    /// mention link targets. Callers restoring a previously submitted draft
    /// that must keep `$name -> path` resolution should use
    /// [`Self::set_text_content_with_mention_bindings`] instead.
    pub(crate) fn set_text_content(
        &mut self,
        text: String,
        text_elements: Vec<TextElement>,
        local_image_paths: Vec<PathBuf>,
    ) {
        self.set_text_content_with_mention_bindings(
            text,
            text_elements,
            local_image_paths,
            Vec::new(),
        );
    }

    /// Replace the entire composer content while restoring mention link targets.
    ///
    /// Mention popup insertion stores both visible text (for example `$file`)
    /// and hidden mention bindings used to resolve the canonical target during
    /// submission. Use this method when restoring an interrupted or blocked
    /// draft; if callers restore only text and images, mentions can appear
    /// intact to users while resolving to the wrong target or dropping on
    /// retry.
    ///
    /// This helper intentionally places the cursor at the start of the restored text. Callers
    /// that need end-of-line restore behavior (for example shell-style history recall) should call
    /// [`Self::move_cursor_to_end`] after this method.
    pub(crate) fn set_text_content_with_mention_bindings(
        &mut self,
        text: String,
        text_elements: Vec<TextElement>,
        local_image_paths: Vec<PathBuf>,
        mention_bindings: Vec<MentionBinding>,
    ) {
        // Clear any existing content, placeholders, and attachments first.
        self.textarea.set_text_clearing_elements("");
        self.is_bash_mode = false;
        self.pending_pastes.clear();
        self.attached_images.clear();
        self.mention_bindings.clear();

        let (text, text_elements) = self.imported_text_for_textarea(text, text_elements);
        self.textarea.set_text_with_elements(&text, &text_elements);

        for (idx, path) in local_image_paths.into_iter().enumerate() {
            let placeholder = local_image_label_text(self.remote_image_urls.len() + idx + 1);
            self.attached_images
                .push(AttachedImage { placeholder, path });
        }

        self.bind_mentions_from_snapshot(mention_bindings);
        self.relabel_attached_images_and_update_placeholders();
        self.selected_remote_image_index = None;
        self.textarea.set_cursor(/*pos*/ 0);
        self.sync_popups();
    }

    fn current_cursor(&self) -> usize {
        self.textarea.cursor() + if self.is_bash_mode { 1 } else { 0 }
    }

    fn history_navigation_cursor(&self) -> usize {
        if self.is_bash_mode && self.textarea.cursor() == 0 {
            0
        } else if self.textarea.is_vim_normal_mode()
            && !self.textarea.text().is_empty()
            && self.textarea.cursor() == self.textarea.vim_normal_end_cursor()
        {
            self.current_text().len()
        } else {
            self.current_cursor()
        }
    }

    fn set_current_cursor(&mut self, cursor: usize) {
        let visible_cursor = if self.is_bash_mode {
            cursor.saturating_sub(1)
        } else {
            cursor
        };
        self.textarea
            .set_cursor(visible_cursor.min(self.textarea.text().len()));
    }

    fn current_text_elements(&self) -> Vec<TextElement> {
        let shift = if self.is_bash_mode { 1 } else { 0 };
        self.textarea
            .text_elements()
            .into_iter()
            .filter_map(|element| Self::shift_text_element(element, shift))
            .collect()
    }

    fn shift_text_element(element: TextElement, shift: isize) -> Option<TextElement> {
        let start = element.byte_range.start.checked_add_signed(shift)?;
        let end = element.byte_range.end.checked_add_signed(shift)?;
        if start >= end {
            return None;
        }

        Some(element.map_range(|_| (start..end).into()))
    }

    fn snapshot_draft(&self) -> ComposerDraft {
        ComposerDraft {
            text: self.current_text(),
            text_elements: self.current_text_elements(),
            local_image_paths: self
                .attached_images
                .iter()
                .map(|img| img.path.clone())
                .collect(),
            remote_image_urls: self.remote_image_urls.clone(),
            mention_bindings: self.snapshot_mention_bindings(),
            pending_pastes: self.pending_pastes.clone(),
            cursor: self.current_cursor(),
        }
    }

    fn restore_draft(&mut self, draft: ComposerDraft) {
        let ComposerDraft {
            text,
            text_elements,
            local_image_paths,
            remote_image_urls,
            mention_bindings,
            pending_pastes,
            cursor,
        } = draft;
        self.set_remote_image_urls(remote_image_urls);
        self.set_text_content_with_mention_bindings(
            text,
            text_elements,
            local_image_paths,
            mention_bindings,
        );
        self.set_pending_pastes(pending_pastes);
        self.set_current_cursor(cursor);
        self.sync_popups();
    }

    /// Update the placeholder text without changing input enablement.
    pub(crate) fn set_placeholder_text(&mut self, placeholder: String) {
        self.placeholder_text = placeholder;
    }

    /// Move the cursor to the end of the current text buffer.
    pub(crate) fn move_cursor_to_end(&mut self) {
        self.textarea.set_cursor(self.textarea.text().len());
        self.sync_popups();
    }

    fn move_cursor_to_history_entry_end(&mut self) {
        let cursor = if self.textarea.is_vim_normal_mode() {
            self.textarea.vim_normal_end_cursor()
        } else {
            self.textarea.text().len()
        };
        self.textarea.set_cursor(cursor);
        self.sync_popups();
    }

    /// Convert canonical composer text into the textarea's internal representation.
    ///
    /// Shell mode stores the leading `!` as prompt state instead of editable text,
    /// so full-buffer imports must absorb that prefix before rebuilding the textarea.
    fn imported_text_for_textarea(
        &mut self,
        text: String,
        text_elements: Vec<TextElement>,
    ) -> (String, Vec<TextElement>) {
        if let Some(stripped) = text.strip_prefix('!') {
            self.is_bash_mode = true;
            (
                stripped.to_string(),
                text_elements
                    .into_iter()
                    .filter_map(|element| Self::shift_text_element(element, /*shift*/ -1))
                    .collect(),
            )
        } else {
            self.is_bash_mode = false;
            (text, text_elements)
        }
    }

    pub(crate) fn clear_for_ctrl_c(&mut self) -> Option<String> {
        if self.is_empty() {
            return None;
        }
        let previous = self.current_text();
        let text_elements = self.current_text_elements();
        let local_image_paths = self
            .attached_images
            .iter()
            .map(|img| img.path.clone())
            .collect();
        let pending_pastes = std::mem::take(&mut self.pending_pastes);
        let remote_image_urls = self.remote_image_urls.clone();
        let mention_bindings = self.snapshot_mention_bindings();
        self.set_text_content(String::new(), Vec::new(), Vec::new());
        self.remote_image_urls.clear();
        self.selected_remote_image_index = None;
        self.history.reset_navigation();
        self.history.record_local_submission(HistoryEntry {
            text: previous.clone(),
            text_elements,
            local_image_paths,
            remote_image_urls,
            mention_bindings,
            pending_pastes,
        });
        Some(previous)
    }

    /// Get the current composer text.
    pub(crate) fn current_text(&self) -> String {
        if self.is_bash_mode {
            format!("!{}", self.textarea.text())
        } else {
            self.textarea.text().to_string()
        }
    }

    /// Rehydrate a history entry into the composer with shell-like cursor placement.
    ///
    /// This path restores text, elements, images, mention bindings, and pending paste payloads,
    /// then moves the cursor to the active mode's history boundary. If a caller reused
    /// [`Self::set_text_content_with_mention_bindings`] directly for history recall and forgot the
    /// final cursor move, repeated Up/Down would stop navigating history because cursor-gating
    /// treats interior positions as normal editing mode.
    fn apply_history_entry(&mut self, entry: HistoryEntry) {
        let HistoryEntry {
            text,
            text_elements,
            local_image_paths,
            remote_image_urls,
            mention_bindings,
            pending_pastes,
        } = entry;
        self.set_remote_image_urls(remote_image_urls);
        self.set_text_content_with_mention_bindings(
            text,
            text_elements,
            local_image_paths,
            mention_bindings,
        );
        self.set_pending_pastes(pending_pastes);
        self.move_cursor_to_history_entry_end();
    }

    pub(crate) fn text_elements(&self) -> Vec<TextElement> {
        self.current_text_elements()
    }

    #[cfg(test)]
    pub(crate) fn local_image_paths(&self) -> Vec<PathBuf> {
        self.attached_images
            .iter()
            .map(|img| img.path.clone())
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn status_line_text(&self) -> Option<String> {
        self.status_line_value.as_ref().map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
    }

    pub(crate) fn local_images(&self) -> Vec<LocalImageAttachment> {
        self.attached_images
            .iter()
            .map(|img| LocalImageAttachment {
                placeholder: img.placeholder.clone(),
                path: img.path.clone(),
            })
            .collect()
    }

    pub(crate) fn mention_bindings(&self) -> Vec<MentionBinding> {
        self.snapshot_mention_bindings()
    }

    pub(crate) fn take_recent_submission_mention_bindings(&mut self) -> Vec<MentionBinding> {
        std::mem::take(&mut self.recent_submission_mention_bindings)
    }

    /// Commit the staged slash-command draft to local Up-arrow recall.
    ///
    /// Call this after command dispatch. Calling it more than once is harmless because the pending
    /// slot is consumed on the first call.
    pub(crate) fn record_pending_slash_command_history(&mut self) {
        if let Some(entry) = self.pending_slash_command_history.take() {
            self.history.record_local_submission(entry);
        }
    }

    fn prune_attached_images_for_submission(&mut self, text: &str, text_elements: &[TextElement]) {
        if self.attached_images.is_empty() {
            return;
        }
        let image_placeholders: HashSet<&str> = text_elements
            .iter()
            .filter_map(|elem| elem.placeholder(text))
            .collect();
        self.attached_images
            .retain(|img| image_placeholders.contains(img.placeholder.as_str()));
    }

    /// Insert an attachment placeholder and track it for the next submission.
    pub fn attach_image(&mut self, path: PathBuf) {
        let image_number = self.remote_image_urls.len() + self.attached_images.len() + 1;
        let placeholder = local_image_label_text(image_number);
        // Insert as an element to match large paste placeholder behavior:
        // styled distinctly and treated atomically for cursor/mutations.
        self.textarea.insert_element(&placeholder);
        self.attached_images
            .push(AttachedImage { placeholder, path });
    }

    #[cfg(test)]
    pub fn take_recent_submission_images(&mut self) -> Vec<PathBuf> {
        let images = std::mem::take(&mut self.attached_images);
        images.into_iter().map(|img| img.path).collect()
    }

    pub fn take_recent_submission_images_with_placeholders(&mut self) -> Vec<LocalImageAttachment> {
        let images = std::mem::take(&mut self.attached_images);
        images
            .into_iter()
            .map(|img| LocalImageAttachment {
                placeholder: img.placeholder,
                path: img.path,
            })
            .collect()
    }

    /// Flushes any due paste-burst state.
    ///
    /// Call this from a UI tick to turn paste-burst transient state into explicit textarea edits:
    ///
    /// - If a burst times out, flush it via `handle_paste(String)`.
    /// - If only the first ASCII char was held (flicker suppression) and no burst followed, emit it
    ///   as normal typed input.
    ///
    /// This also allows a single "held" ASCII char to render even when it turns out not to be part
    /// of a paste burst.
    pub(crate) fn flush_paste_burst_if_due(&mut self) -> bool {
        self.handle_paste_burst_flush(Instant::now())
    }

    /// Returns whether the composer is currently in any paste-burst related transient state.
    ///
    /// This includes actively buffering, having a non-empty burst buffer, or holding the first
    /// ASCII char for flicker suppression.
    pub(crate) fn is_in_paste_burst(&self) -> bool {
        self.paste_burst.is_active()
    }

    /// Returns a delay that reliably exceeds the paste-burst timing threshold.
    ///
    /// Use this in tests to avoid boundary flakiness around the `PasteBurst` timeout.
    pub(crate) fn recommended_paste_flush_delay() -> Duration {
        PasteBurst::recommended_flush_delay()
    }

    /// Integrate results from an asynchronous file search.
    pub(crate) fn on_file_search_result(&mut self, query: String, matches: Vec<FileMatch>) {
        // Only apply if user is still editing a token starting with `query`.
        let current_opt = Self::current_at_token(&self.textarea);
        let Some(current_token) = current_opt else {
            return;
        };

        if !current_token.starts_with(&query) {
            return;
        }

        if let ActivePopup::File(popup) = &mut self.active_popup {
            popup.set_matches(&query, matches);
        }
    }

    /// Show the transient "press again to quit" hint for `key`.
    ///
    /// The owner (`BottomPane`/`ChatWidget`) is responsible for scheduling a
    /// redraw after [`super::QUIT_SHORTCUT_TIMEOUT`] so the hint can disappear
    /// even when the UI is otherwise idle.
    pub fn show_quit_shortcut_hint(&mut self, key: KeyBinding, has_focus: bool) {
        self.quit_shortcut_expires_at = Instant::now()
            .checked_add(super::QUIT_SHORTCUT_TIMEOUT)
            .or_else(|| Some(Instant::now()));
        self.quit_shortcut_key = key;
        self.footer_mode = FooterMode::QuitShortcutReminder;
        self.set_has_focus(has_focus);
    }

    /// Clear the "press again to quit" hint immediately.
    pub fn clear_quit_shortcut_hint(&mut self, has_focus: bool) {
        self.quit_shortcut_expires_at = None;
        self.footer_mode = reset_mode_after_activity(self.footer_mode);
        self.set_has_focus(has_focus);
    }

    /// Whether the quit shortcut hint should currently be shown.
    ///
    /// This is time-based rather than event-based: it may become false without
    /// any additional user input, so the UI schedules a redraw when the hint
    /// expires.
    pub(crate) fn quit_shortcut_hint_visible(&self) -> bool {
        self.quit_shortcut_expires_at
            .is_some_and(|expires_at| Instant::now() < expires_at)
    }

    fn next_large_paste_placeholder(&mut self, char_count: usize) -> String {
        let base = format!("[Pasted Content {char_count} chars]");
        let next_suffix = self.large_paste_counters.entry(char_count).or_insert(0);
        *next_suffix += 1;
        if *next_suffix == 1 {
            base
        } else {
            format!("{base} #{next_suffix}")
        }
    }

    pub(crate) fn insert_str(&mut self, text: &str) {
        self.textarea.insert_str(text);
        self.sync_bash_mode_from_text();
        self.sync_popups();
    }

    /// Handle a key event coming from the main UI.
    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> (InputResult, bool) {
        if !self.input_enabled {
            return (InputResult::None, false);
        }

        if matches!(key_event.kind, KeyEventKind::Release) {
            return (InputResult::None, false);
        }

        if self.history_search.is_some() {
            return self.handle_history_search_key(key_event);
        }

        if Self::is_history_search_key(&key_event, &self.history_search_previous_keys) {
            return self.begin_history_search();
        }

        let result = match &mut self.active_popup {
            ActivePopup::Command(_) => self.handle_key_event_with_slash_popup(key_event),
            ActivePopup::File(_) => self.handle_key_event_with_file_popup(key_event),
            ActivePopup::Skill(_) => self.handle_key_event_with_skill_popup(key_event),
            ActivePopup::None => self.handle_key_event_without_popup(key_event),
        };
        self.reset_vim_mode_after_successful_dispatch(&result.0);
        // Update (or hide/show) popup after processing the key.
        self.sync_popups();
        result
    }

    /// Return true if either the slash-command popup or the file-search popup is active.
    pub(crate) fn popup_active(&self) -> bool {
        self.history_search.is_some() || !matches!(self.active_popup, ActivePopup::None)
    }

    /// Handle key event when the slash-command popup is visible.
    fn handle_key_event_with_slash_popup(&mut self, key_event: KeyEvent) -> (InputResult, bool) {
        if self.handle_shortcut_overlay_key(&key_event) {
            return (InputResult::None, true);
        }
        if key_event.code == KeyCode::Esc {
            let next_mode = esc_hint_mode(self.footer_mode, self.is_task_running);
            if next_mode != self.footer_mode {
                self.footer_mode = next_mode;
                return (InputResult::None, true);
            }
        } else {
            self.footer_mode = reset_mode_after_activity(self.footer_mode);
        }
        let ActivePopup::Command(popup) = &mut self.active_popup else {
            unreachable!();
        };

        match key_event {
            KeyEvent {
                code: KeyCode::Up, ..
            }
            | KeyEvent {
                code: KeyCode::Char('p'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => {
                popup.move_up();
                (InputResult::None, true)
            }
            KeyEvent {
                code: KeyCode::Down,
                ..
            }
            | KeyEvent {
                code: KeyCode::Char('n'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => {
                popup.move_down();
                (InputResult::None, true)
            }
            KeyEvent {
                code: KeyCode::Esc, ..
            } => {
                // Dismiss the slash popup; keep the current input untouched.
                self.active_popup = ActivePopup::None;
                (InputResult::None, true)
            }
            KeyEvent {
                code: KeyCode::Tab, ..
            } => {
                // Ensure popup filtering/selection reflects the latest composer text
                // before applying completion.
                let first_line = self.textarea.text().lines().next().unwrap_or("");
                popup.on_composer_text_change(first_line.to_string());
                let selected_cmd = popup.selected_item().map(|sel| {
                    let CommandItem::Builtin(cmd) = sel;
                    cmd
                });
                if let Some(cmd) = selected_cmd {
                    if cmd == SlashCommand::Skills {
                        self.stage_selected_slash_command_history(cmd);
                        self.textarea.set_text_clearing_elements("");
                        self.is_bash_mode = false;
                        return (InputResult::Command(cmd), true);
                    }

                    let selected_command_text = format!("/{}", cmd.command());
                    let starts_with_cmd =
                        first_line.trim_start().starts_with(&selected_command_text);
                    if !starts_with_cmd {
                        self.textarea
                            .set_text_clearing_elements(&format!("/{} ", cmd.command()));
                        if !self.textarea.text().is_empty() {
                            self.textarea.set_cursor(self.textarea.text().len());
                        }
                        return (InputResult::None, true);
                    }
                }
                if self.is_task_running {
                    return self.handle_submission(/*should_queue*/ true);
                }
                (InputResult::None, true)
            }
            KeyEvent {
                code: KeyCode::Char('/'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                // Treat "/" as accepting the highlighted command as text completion
                // while the slash-command popup is active.
                let first_line = self.textarea.text().lines().next().unwrap_or("");
                popup.on_composer_text_change(first_line.to_string());
                let selected_cmd = popup.selected_item().map(|sel| {
                    let CommandItem::Builtin(cmd) = sel;
                    cmd
                });
                if let Some(cmd) = selected_cmd {
                    let starts_with_cmd = first_line
                        .trim_start()
                        .starts_with(&format!("/{}", cmd.command()));
                    if !starts_with_cmd {
                        self.textarea
                            .set_text_clearing_elements(&format!("/{} ", cmd.command()));
                        self.is_bash_mode = false;
                    }
                    if !self.textarea.text().is_empty() {
                        self.textarea.set_cursor(self.textarea.text().len());
                    }
                }
                (InputResult::None, true)
            }
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                if let Some(sel) = popup.selected_item() {
                    let CommandItem::Builtin(cmd) = sel;
                    self.stage_selected_slash_command_history(cmd);
                    self.textarea.set_text_clearing_elements("");
                    self.is_bash_mode = false;
                    return (InputResult::Command(cmd), true);
                }
                // Fallback to default newline handling if no command selected.
                self.handle_key_event_without_popup(key_event)
            }
            input => self.handle_input_basic(input),
        }
    }

}
