fn footer_insert_newline_key(
    bindings: &[KeyBinding],
    enhanced_keys_supported: bool,
) -> Option<KeyBinding> {
    let shift_enter = key_hint::shift(KeyCode::Enter);
    if enhanced_keys_supported && bindings.contains(&shift_enter) {
        return Some(shift_enter);
    }

    let plain_enter = key_hint::plain(KeyCode::Enter);
    bindings
        .iter()
        .copied()
        .find(|binding| *binding != plain_enter)
        .or_else(|| bindings.first().copied())
}

impl ChatComposer {
    pub fn update_recording_meter_in_place(&mut self, id: &str, text: &str) -> bool {
        self.textarea.update_named_element_by_id(id, text)
    }

    pub fn insert_recording_meter_placeholder(&mut self, text: &str) -> String {
        static RECORDING_METER_ID: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(1);
        let id = format!(
            "recording-meter-{}",
            RECORDING_METER_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        self.textarea.insert_named_element(text, id.clone());
        id
    }

    pub fn remove_recording_meter_placeholder(&mut self, id: &str) {
        let _ = self.textarea.replace_element_by_id(id, "");
    }
}

fn skill_description(skill: &SkillMetadata) -> Option<String> {
    let description = skill
        .interface
        .as_ref()
        .and_then(|interface| interface.short_description.as_deref())
        .or(skill.short_description.as_deref())
        .unwrap_or(&skill.description);
    let trimmed = description.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn is_mention_name_char(byte: u8) -> bool {
    matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-')
}

fn find_next_mention_token_range(text: &str, token: &str, from: usize) -> Option<Range<usize>> {
    if token.is_empty() || from >= text.len() {
        return None;
    }
    let bytes = text.as_bytes();
    let token_bytes = token.as_bytes();
    let mut index = from;

    while index < bytes.len() {
        if bytes[index] != b'$' {
            index += 1;
            continue;
        }

        let end = index.saturating_add(token_bytes.len());
        if end > bytes.len() {
            return None;
        }
        if &bytes[index..end] != token_bytes {
            index += 1;
            continue;
        }

        if bytes
            .get(end)
            .is_none_or(|byte| !is_mention_name_char(*byte))
        {
            return Some(index..end);
        }

        index = end;
    }

    None
}

impl Renderable for ChatComposer {
    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        if !self.input_enabled || self.selected_remote_image_index.is_some() {
            return None;
        }

        if let Some(pos) = self.history_search_cursor_pos(area) {
            return Some(pos);
        }

        let [_, _, textarea_rect, _] = self.layout_areas(area);
        let state = *self.textarea_state.borrow();
        self.textarea.cursor_pos_with_state(textarea_rect, state)
    }

    fn cursor_style(&self, _area: Rect) -> crossterm::cursor::SetCursorStyle {
        if self.textarea.uses_vim_insert_cursor() {
            crossterm::cursor::SetCursorStyle::SteadyBar
        } else {
            crossterm::cursor::SetCursorStyle::DefaultUserShape
        }
    }

    fn desired_height(&self, width: u16) -> u16 {
        let footer_props = self.footer_props();
        let footer_hint_height = self
            .custom_footer_height()
            .unwrap_or_else(|| footer_height(&footer_props));
        let footer_spacing = Self::footer_spacing(footer_hint_height);
        let footer_total_height = footer_hint_height + footer_spacing;
        const COLS_WITH_MARGIN: u16 = LIVE_PREFIX_COLS + 1;
        let inner_width = width.saturating_sub(COLS_WITH_MARGIN);
        let remote_images_height: u16 = self
            .remote_images_lines(inner_width)
            .len()
            .try_into()
            .unwrap_or(u16::MAX);
        let remote_images_separator = u16::from(remote_images_height > 0);
        self.textarea.desired_height(inner_width)
            + remote_images_height
            + remote_images_separator
            + 2
            + match &self.active_popup {
                ActivePopup::None => footer_total_height,
                ActivePopup::Command(c) => c.calculate_required_height(width),
                ActivePopup::File(c) => c.calculate_required_height(),
                ActivePopup::Skill(c) => c.calculate_required_height(width),
            }
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.render_with_mask(area, buf, /*mask_char*/ None);
    }
}

impl ChatComposer {
    pub(crate) fn render_with_mask(&self, area: Rect, buf: &mut Buffer, mask_char: Option<char>) {
        let [composer_rect, remote_images_rect, textarea_rect, popup_rect] =
            self.layout_areas(area);
        match &self.active_popup {
            ActivePopup::Command(popup) => {
                popup.render_ref(popup_rect, buf);
            }
            ActivePopup::File(popup) => {
                popup.render_ref(popup_rect, buf);
            }
            ActivePopup::Skill(popup) => {
                popup.render_ref(popup_rect, buf);
            }
            ActivePopup::None => {
                let footer_props = self.footer_props();
                let show_cycle_hint =
                    !footer_props.is_task_running && self.collaboration_mode_indicator.is_some();
                let show_shortcuts_hint = match footer_props.mode {
                    FooterMode::ComposerEmpty => !self.is_in_paste_burst(),
                    FooterMode::ComposerHasDraft => false,
                    FooterMode::HistorySearch
                    | FooterMode::QuitShortcutReminder
                    | FooterMode::ShortcutOverlay
                    | FooterMode::EscHint => false,
                };
                let show_queue_hint = match footer_props.mode {
                    FooterMode::ComposerHasDraft => footer_props.is_task_running,
                    FooterMode::HistorySearch
                    | FooterMode::QuitShortcutReminder
                    | FooterMode::ComposerEmpty
                    | FooterMode::ShortcutOverlay
                    | FooterMode::EscHint => false,
                };
                let custom_height = self.custom_footer_height();
                let footer_hint_height =
                    custom_height.unwrap_or_else(|| footer_height(&footer_props));
                let footer_spacing = Self::footer_spacing(footer_hint_height);
                let hint_rect = if footer_spacing > 0 && footer_hint_height > 0 {
                    let [_, hint_rect] = Layout::vertical([
                        Constraint::Length(footer_spacing),
                        Constraint::Length(footer_hint_height),
                    ])
                    .areas(popup_rect);
                    hint_rect
                } else {
                    popup_rect
                };
                if let Some(line) = self.history_search_footer_line() {
                    render_footer_line(hint_rect, buf, line);
                } else if self.plan_mode_nudge_visible {
                    let available_width =
                        hint_rect.width.saturating_sub(FOOTER_INDENT_COLS as u16) as usize;
                    render_footer_line(
                        hint_rect,
                        buf,
                        truncate_line_with_ellipsis_if_overflow(
                            plan_mode_nudge_line(),
                            available_width,
                        ),
                    );
                } else {
                    let available_width =
                        hint_rect.width.saturating_sub(FOOTER_INDENT_COLS as u16) as usize;
                    let status_line_active = uses_passive_footer_status_layout(&footer_props);
                    let combined_status_line = if status_line_active {
                        passive_footer_status_line(&footer_props)
                    } else {
                        None
                    };
                    let mut truncated_status_line = if status_line_active {
                        combined_status_line.as_ref().map(|line| {
                            truncate_line_with_ellipsis_if_overflow(line.clone(), available_width)
                        })
                    } else {
                        None
                    };
                    let left_mode_indicator = if status_line_active {
                        None
                    } else {
                        self.collaboration_mode_indicator
                    };
                    let active_footer_hint_override = self.footer_hint_override.as_ref();
                    let mut left_width = if self.footer_flash_visible() {
                        self.footer_flash
                            .as_ref()
                            .map(|flash| flash.line.width() as u16)
                            .unwrap_or(0)
                    } else if let Some(items) = active_footer_hint_override {
                        footer_hint_items_width(items)
                    } else if status_line_active {
                        truncated_status_line
                            .as_ref()
                            .map(|line| line.width() as u16)
                            .unwrap_or(0)
                    } else {
                        footer_line_width(
                            &footer_props,
                            left_mode_indicator,
                            show_cycle_hint,
                            show_shortcuts_hint,
                            show_queue_hint,
                        )
                    };
                    let right_line =
                        if let Some(label) = self.side_conversation_context_label.as_ref() {
                            Some(side_conversation_context_line(label))
                        } else if let Some(line) = self.shell_mode_footer_line() {
                            Some(line)
                        } else if status_line_active {
                            let full = self.mode_indicator_line(show_cycle_hint);
                            let compact = self.mode_indicator_line(/*show_cycle_hint*/ false);
                            let full_width = full.as_ref().map(|l| l.width() as u16).unwrap_or(0);
                            if can_show_left_with_context(hint_rect, left_width, full_width) {
                                full
                            } else {
                                compact
                            }
                        } else {
                            Some(self.right_footer_line_with_context())
                        };
                    let right_width = right_line.as_ref().map(|l| l.width() as u16).unwrap_or(0);
                    if status_line_active
                        && let Some(max_left) = max_left_width_for_right(hint_rect, right_width)
                        && left_width > max_left
                        && let Some(line) = combined_status_line.as_ref().map(|line| {
                            truncate_line_with_ellipsis_if_overflow(line.clone(), max_left as usize)
                        })
                    {
                        left_width = line.width() as u16;
                        truncated_status_line = Some(line);
                    }
                    let can_show_left_and_context =
                        can_show_left_with_context(hint_rect, left_width, right_width);
                    let has_override =
                        self.footer_flash_visible() || active_footer_hint_override.is_some();
                    let single_line_layout = if has_override || status_line_active {
                        None
                    } else {
                        match footer_props.mode {
                            FooterMode::ComposerEmpty | FooterMode::ComposerHasDraft => {
                                // Both of these modes render the single-line footer style (with
                                // either the shortcuts hint or the optional queue hint). We still
                                // want the single-line collapse rules so the mode label can win over
                                // the context indicator on narrow widths.
                                Some(single_line_footer_layout(
                                    hint_rect,
                                    right_width,
                                    left_mode_indicator,
                                    show_cycle_hint,
                                    show_shortcuts_hint,
                                    show_queue_hint,
                                    footer_props.key_hints,
                                ))
                            }
                            FooterMode::EscHint
                            | FooterMode::HistorySearch
                            | FooterMode::QuitShortcutReminder
                            | FooterMode::ShortcutOverlay => None,
                        }
                    };
                    let show_right = if matches!(
                        footer_props.mode,
                        FooterMode::EscHint
                            | FooterMode::HistorySearch
                            | FooterMode::QuitShortcutReminder
                            | FooterMode::ShortcutOverlay
                    ) {
                        false
                    } else {
                        single_line_layout
                            .as_ref()
                            .map(|(_, show_context)| *show_context)
                            .unwrap_or(can_show_left_and_context)
                    };

                    if let Some((summary_left, _)) = single_line_layout {
                        match summary_left {
                            SummaryLeft::Default => {
                                if status_line_active {
                                    if let Some(line) = truncated_status_line.clone() {
                                        render_footer_line(hint_rect, buf, line);
                                    } else {
                                        render_footer_from_props(
                                            hint_rect,
                                            buf,
                                            &footer_props,
                                            left_mode_indicator,
                                            show_cycle_hint,
                                            show_shortcuts_hint,
                                            show_queue_hint,
                                        );
                                    }
                                } else {
                                    render_footer_from_props(
                                        hint_rect,
                                        buf,
                                        &footer_props,
                                        left_mode_indicator,
                                        show_cycle_hint,
                                        show_shortcuts_hint,
                                        show_queue_hint,
                                    );
                                }
                            }
                            SummaryLeft::Custom(line) => {
                                render_footer_line(hint_rect, buf, line);
                            }
                            SummaryLeft::None => {}
                        }
                    } else if self.footer_flash_visible() {
                        if let Some(flash) = self.footer_flash.as_ref() {
                            flash.line.render(inset_footer_hint_area(hint_rect), buf);
                        }
                    } else if let Some(items) = active_footer_hint_override {
                        render_footer_hint_items(hint_rect, buf, items);
                    } else if status_line_active {
                        if let Some(line) = truncated_status_line {
                            render_footer_line(hint_rect, buf, line);
                        }
                    } else {
                        render_footer_from_props(
                            hint_rect,
                            buf,
                            &footer_props,
                            self.collaboration_mode_indicator,
                            show_cycle_hint,
                            show_shortcuts_hint,
                            show_queue_hint,
                        );
                    }
                    if show_right && let Some(line) = &right_line {
                        render_context_right(hint_rect, buf, line);
                    }
                }
            }
        }
        self.render_textarea(
            composer_rect,
            remote_images_rect,
            textarea_rect,
            buf,
            mask_char,
        );
    }

    /// Paint the composer's text input area, prompt chevron, and placeholder text.
    ///
    /// In Zellij sessions the textarea uses explicit `Color::Reset` foreground styling
    /// to prevent the multiplexer's pane chrome from bleeding into cell styles, and
    /// substitutes hardcoded colors for `.bold()` / `.dim()` modifiers that Zellij
    /// renders inconsistently. The standard path is unchanged.
    fn render_textarea(
        &self,
        composer_rect: Rect,
        remote_images_rect: Rect,
        textarea_rect: Rect,
        buf: &mut Buffer,
        mask_char: Option<char>,
    ) {
        let is_zellij = self.is_zellij;
        let style = user_message_style();
        let textarea_style = style.fg(ratatui::style::Color::Reset);
        Block::default()
            .style(Style::default().bg(crate::ui_consts::APP_BG))
            .render_ref(composer_rect, buf);
        if !remote_images_rect.is_empty() {
            Paragraph::new(self.remote_images_lines(remote_images_rect.width))
                .style(style)
                .render_ref(remote_images_rect, buf);
        }
        if is_zellij && !textarea_rect.is_empty() {
            buf.set_style(textarea_rect, textarea_style);
        }
        if !textarea_rect.is_empty() {
            let prompt = if self.input_enabled {
                if self.is_bash_mode {
                    if is_zellij {
                        Span::from("!").light_red()
                    } else {
                        Span::from("!").light_red().bold()
                    }
                } else if is_zellij {
                    Span::styled("›", style.fg(ratatui::style::Color::Cyan))
                } else {
                    "›".bold()
                }
            } else if is_zellij {
                Span::styled("›", style.fg(ratatui::style::Color::DarkGray))
            } else {
                "›".dim()
            };
            buf.set_span(
                textarea_rect.x - LIVE_PREFIX_COLS,
                textarea_rect.y,
                &prompt,
                textarea_rect.width,
            );
        }

        let mut state = self.textarea_state.borrow_mut();
        let textarea_is_empty = self.textarea.text().is_empty() && !self.is_bash_mode;
        if let Some(mask_char) = mask_char {
            self.textarea.render_ref_masked(
                textarea_rect,
                buf,
                &mut state,
                mask_char,
                if is_zellij {
                    textarea_style
                } else {
                    ratatui::style::Style::default()
                },
            );
        } else if is_zellij && textarea_is_empty {
            buf.set_style(textarea_rect, textarea_style);
        } else if is_zellij {
            let highlight_ranges = self.history_search_highlight_ranges();
            if highlight_ranges.is_empty() {
                self.textarea
                    .render_ref_styled(textarea_rect, buf, &mut state, textarea_style);
            } else {
                let highlight_style =
                    textarea_style.add_modifier(Modifier::REVERSED | Modifier::BOLD);
                let highlights = highlight_ranges
                    .into_iter()
                    .map(|range| (range, highlight_style))
                    .collect::<Vec<_>>();
                self.textarea.render_ref_styled_with_highlights(
                    textarea_rect,
                    buf,
                    &mut state,
                    textarea_style,
                    &highlights,
                );
            }
        } else {
            let highlight_ranges = self.history_search_highlight_ranges();
            if highlight_ranges.is_empty() {
                StatefulWidgetRef::render_ref(&(&self.textarea), textarea_rect, buf, &mut state);
            } else {
                let highlight_style =
                    Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD);
                let highlights = highlight_ranges
                    .into_iter()
                    .map(|range| (range, highlight_style))
                    .collect::<Vec<_>>();
                self.textarea.render_ref_styled_with_highlights(
                    textarea_rect,
                    buf,
                    &mut state,
                    Style::default(),
                    &highlights,
                );
            }
        }
        if textarea_is_empty {
            let text = if self.input_enabled {
                self.placeholder_text.as_str().to_string()
            } else {
                self.input_disabled_placeholder
                    .as_deref()
                    .unwrap_or("Input disabled.")
                    .to_string()
            };
            if !textarea_rect.is_empty() {
                if is_zellij {
                    buf.set_string(
                        textarea_rect.x,
                        textarea_rect.y,
                        text,
                        textarea_style.fg(ratatui::style::Color::White).italic(),
                    );
                } else {
                    let placeholder = Span::from(text).dim();
                    let line = Line::from(vec![placeholder]);
                    line.render_ref(textarea_rect.inner(Margin::new(0, 0)), buf);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "../split_002_footerflash_tests.rs"]
mod tests;
