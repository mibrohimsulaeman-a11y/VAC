
impl ChatWidget {
    pub(crate) fn update_recording_meter_in_place(&mut self, id: &str, text: &str) -> bool {
        let updated = self.bottom_pane.update_recording_meter_in_place(id, text);
        if updated {
            self.request_redraw();
        }
        updated
    }

    pub(crate) fn remove_recording_meter_placeholder(&mut self, id: &str) {
        self.bottom_pane.remove_recording_meter_placeholder(id);
        // Ensure the UI redraws to reflect placeholder removal.
        self.request_redraw();
    }
}

fn has_websocket_timing_metrics(summary: RuntimeMetricsSummary) -> bool {
    summary.responses_api_overhead_ms > 0
        || summary.responses_api_inference_time_ms > 0
        || summary.responses_api_engine_iapi_ttft_ms > 0
        || summary.responses_api_engine_service_ttft_ms > 0
        || summary.responses_api_engine_iapi_tbt_ms > 0
        || summary.responses_api_engine_service_tbt_ms > 0
}

impl Drop for ChatWidget {
    fn drop(&mut self) {
        self.reset_realtime_conversation_state();
        self.stop_rate_limit_poller();
    }
}


impl ChatWidget {
    fn fill_render_background(area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let bg = crate::ui_consts::APP_BG;
        for y in area.y..area.y.saturating_add(area.height) {
            for x in area.x..area.x.saturating_add(area.width) {
                buf[(x, y)].set_style(ratatui::style::Style::default().bg(bg));
            }
        }
    }

    fn transcript_cell_count_for_windowing(&self) -> usize {
        // Only transcript-like regions participate in windowing. The bottom pane
        // is anchored separately so fullscreen rendering keeps the composer at
        // the bottom instead of treating it like another transcript cell.
        2
    }

    fn transcript_cell_renderable(&self, idx: usize) -> RenderableItem<'_> {
        match idx {
            0 => match &self.active_cell {
                Some(cell) => RenderableItem::Borrowed(cell).inset(Insets::tlbr(1, 0, 0, 0)),
                None => RenderableItem::Owned(Box::new(())),
            },
            1 => match &self.active_hook_cell {
                Some(cell) if cell.should_render() => {
                    RenderableItem::Borrowed(cell).inset(Insets::tlbr(1, 0, 0, 0))
                }
                _ => RenderableItem::Owned(Box::new(())),
            },
            _ => RenderableItem::Owned(Box::new(())),
        }
    }

    fn bottom_pane_renderable(&self) -> RenderableItem<'_> {
        RenderableItem::Borrowed(&self.bottom_pane).inset(Insets::tlbr(1, 0, 0, 0))
    }

    fn fullscreen_layout(&self, area: Rect) -> (Rect, Rect) {
        let bottom_height = self
            .bottom_pane_renderable()
            .desired_height(area.width)
            .min(area.height);
        let transcript_height = area.height.saturating_sub(bottom_height);
        let transcript_area = Rect::new(area.x, area.y, area.width, transcript_height);
        let bottom_area = Rect::new(
            area.x,
            area.y.saturating_add(transcript_height),
            area.width,
            bottom_height,
        );
        (transcript_area, bottom_area)
    }

    fn rebuild_windowed_height_prefix(&self, width: u16) -> Vec<u16> {
        let total_cells = self.transcript_cell_count_for_windowing();
        let style_revision = self.style_revision();
        let mut cache = self.desired_height_cache.borrow_mut();
        let mut heights = Vec::with_capacity(total_cells);
        for idx in 0..total_cells {
            let key = height_cache::CellHeightKey {
                cell_id: u64::try_from(idx).unwrap_or(u64::MAX),
                width,
                content_revision: self.active_cell_revision,
                style_revision,
            };
            let height = cache.get_cell_or_compute(key, || self.transcript_cell_renderable(idx).desired_height(width));
            heights.push(height);
        }
        let prefix_key = height_cache::HeightPrefixBuildKey {
            width,
            transcript_revision: self.active_cell_revision,
            style_revision,
            cell_count: total_cells,
        };
        cache.rebuild_prefix_if_changed(prefix_key, heights.iter().copied());
        heights
    }

    fn current_transcript_scroll_top(&self) -> u32 {
        self.transcript_scroll_top.get()
    }

    fn rendered_lines_key_for_cell(&self, idx: usize, width: u16, cell: &dyn HistoryCell) -> height_cache::RenderedLinesKey {
        let content_revision = self.active_cell_revision;
        let active_revision = cell
            .transcript_animation_tick()
            .unwrap_or(content_revision);
        height_cache::RenderedLinesKey {
            cell_id: u64::try_from(idx).unwrap_or(u64::MAX),
            width,
            style_revision: self.style_revision(),
            content_revision,
            active_revision,
        }
    }

    fn render_cached_history_cell(
        &self,
        idx: usize,
        cell: &dyn HistoryCell,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let render_area = area.inset(Insets::tlbr(1, 0, 0, 0));
        if render_area.is_empty() {
            return;
        }
        let key = self.rendered_lines_key_for_cell(idx, render_area.width, cell);
        let value = self
            .rendered_lines_cache
            .borrow_mut()
            .get_or_compute(key, || cell.display_lines(render_area.width));
        let paragraph = Paragraph::new(Text::from(value.lines)).wrap(Wrap { trim: false });
        let y = if render_area.height == 0 {
            0
        } else {
            let overflow = paragraph
                .line_count(render_area.width)
                .saturating_sub(usize::from(render_area.height));
            u16::try_from(overflow).unwrap_or(u16::MAX)
        };
        Clear.render(render_area, buf);
        paragraph.scroll((y, 0)).render(render_area, buf);
    }

    fn render_cached_transcript_cell(&self, idx: usize, area: Rect, buf: &mut Buffer) {
        match idx {
            0 => {
                if let Some(cell) = self.active_cell.as_ref() {
                    self.render_cached_history_cell(idx, cell.as_ref(), area, buf);
                }
            }
            1 => {
                if let Some(cell) = self.active_hook_cell.as_ref().filter(|cell| cell.should_render()) {
                    self.render_cached_history_cell(idx, cell, area, buf);
                }
            }
            _ => {}
        }
    }

    #[cfg(test)]
    pub(crate) fn set_transcript_scroll_top_for_windowing(&self, scroll_top: u32) {
        self.transcript_scroll_top.set(scroll_top);
    }

    fn render_windowed_transcript_cells(&self, area: Rect, buf: &mut Buffer) {
        let heights = self.rebuild_windowed_height_prefix(area.width);
        let total_cells = self.transcript_cell_count_for_windowing();
        let scroll_top = self.current_transcript_scroll_top();
        let window = self
            .desired_height_cache
            .borrow_mut()
            .windowed_render_plan(total_cells, scroll_top, area.height, 2);
        let mut y = area.y;
        for idx in window.visible.clone() {
            let Some(height) = heights.get(idx).copied() else { continue; };
            if height == 0 { continue; }
            let child_area = Rect::new(area.x, y, area.width, height).intersection(area);
            if !child_area.is_empty() {
                self.render_cached_transcript_cell(idx, child_area, buf);
            }
            y = y.saturating_add(height);
            if y >= area.bottom() { break; }
        }
    }
}

impl Renderable for ChatWidget {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        Self::fill_render_background(area, buf);
        let (transcript_area, bottom_area) = self.fullscreen_layout(area);

        let is_idle = self.visible_user_turn_count == 0
            && self.active_cell.is_none()
            && self.active_hook_cell.as_ref().map_or(true, |c| !c.should_render())
            && !self.is_user_turn_pending_or_running()
            && self.bottom_pane.no_modal_or_popup_active();

        // Show the agent streaming view only during the very first turn, when there
        // is no completed transcript to show yet. On follow-up turns the transcript
        // cells (active_cell / history) should remain visible.
        let is_streaming = self.is_user_turn_pending_or_running()
            && self.visible_user_turn_count == 0;

        if is_idle {
            let status_bar = crate::history_cell::operator_status_bar_for_config(&self.config, self.current_model().to_string());
            let idle_state = crate::operator_ui::IdleViewState::live(self.current_model().to_string())
                .with_status_bar(status_bar);
            let lines = crate::operator_widget_render::render_idle_lines_with_height(
                &idle_state,
                transcript_area.width,
                transcript_area.height,
            );
            ratatui::widgets::Paragraph::new(lines)
                .style(ratatui::style::Style::default().bg(crate::ui_consts::APP_BG))
                .render(transcript_area, buf);
        } else if is_streaming {
            let user_prompt = self.last_rendered_user_message_display.as_ref()
                .map(|d| d.message.clone())
                .unwrap_or_default();
            let turn = if self.visible_user_turn_count > 0 {
                u32::try_from(self.visible_user_turn_count).unwrap_or(u32::MAX)
            } else {
                1
            };
            let status_bar = crate::history_cell::operator_status_bar_for_config(&self.config, self.current_model().to_string());

            let (user_time_str, agent_time_str) = if let Some(start_instant) = self.goal_status_active_turn_started_at {
                let now_instant = std::time::Instant::now();
                let elapsed = now_instant.saturating_duration_since(start_instant);
                let elapsed_chrono = chrono::Duration::from_std(elapsed).unwrap_or(chrono::Duration::zero());
                let agent_start_time = chrono::Local::now() - elapsed_chrono;
                let user_time = agent_start_time - chrono::Duration::seconds(2);
                (
                    user_time.format("%H:%M:%S").to_string(),
                    agent_start_time.format("%H:%M:%S").to_string(),
                )
            } else {
                let now = chrono::Local::now();
                (
                    (now - chrono::Duration::seconds(2)).format("%H:%M:%S").to_string(),
                    now.format("%H:%M:%S").to_string(),
                )
            };

            let mut tools = Vec::new();
            for (_, cmd) in &self.running_commands {
                let cmd_str = cmd.command.join(" ");
                tools.push(crate::operator_ui::ToolTimelineEntry::new(
                    "command",
                    cmd_str,
                    crate::operator_ui::ToolTimelineState::Running,
                    "executing".to_string(),
                ));
            }
            if tools.is_empty() {
                if let Some(cell) = &self.active_cell {
                    let is_exec = cell.as_any().is::<crate::exec_cell::ExecCell>();
                    let is_mcp = cell.as_any().is::<crate::history_cell::McpToolCallCell>();
                    if is_exec || is_mcp {
                        tools.push(crate::operator_ui::ToolTimelineEntry::new(
                            "tool",
                            "active executor".to_string(),
                            crate::operator_ui::ToolTimelineState::Running,
                            "executing".to_string(),
                        ));
                    }
                }
            }

            let state = crate::operator_ui::AgentStreamingState {
                user_prompt,
                user_time: user_time_str,
                agent_model: self.current_model().to_string(),
                turn,
                agent_time: agent_time_str,
                agent_message: self.reasoning_buffer.clone(),
                tools,
                thinking: if self.reasoning_buffer.is_empty() {
                    "analyzing workspace".to_string()
                } else {
                    "reasoning".to_string()
                },
                context_used: self.token_info.as_ref().map_or(0, |t| {
                    u64::try_from(t.total_token_usage.total_tokens).unwrap_or(u64::MAX)
                }),
                context_limit: self
                    .token_info
                    .as_ref()
                    .and_then(|t| t.model_context_window)
                    .map(|limit| u64::try_from(limit).unwrap_or(u64::MAX))
                    .unwrap_or(200_000),
                composer_hint: "esc to interrupt".to_string(),
                status_bar,
            };

            let lines = crate::operator_widget_render::render_agent_streaming_lines_with_height(
                &state,
                transcript_area.width,
                transcript_area.height,
            );
            ratatui::widgets::Paragraph::new(lines)
                .style(ratatui::style::Style::default().bg(crate::ui_consts::APP_BG))
                .render(transcript_area, buf);
        } else {
            self.render_windowed_transcript_cells(transcript_area, buf);
        }

        self.bottom_pane_renderable().render(bottom_area, buf);
        self.last_rendered_width.set(Some(usize::from(area.width)));
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.desired_height_cache.borrow_mut().get_or_compute(
            width,
            self.active_cell_revision,
            || self.as_renderable().desired_height(width),
        )
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        let (_, bottom_area) = self.fullscreen_layout(area);
        self.bottom_pane_renderable().cursor_pos(bottom_area)
    }

    fn cursor_style(&self, area: Rect) -> crossterm::cursor::SetCursorStyle {
        let (_, bottom_area) = self.fullscreen_layout(area);
        self.bottom_pane_renderable().cursor_style(bottom_area)
    }
}

#[derive(Debug)]
enum Notification {
    AgentTurnComplete { response: String },
    ExecApprovalRequested { command: String },
    EditApprovalRequested { cwd: PathBuf, changes: Vec<PathBuf> },
    ElicitationRequested { server_name: String },
    PlanModePrompt { title: String },
}

impl Notification {
    fn display(&self) -> String {
        match self {
            Notification::AgentTurnComplete { response } => {
                Notification::agent_turn_preview(response)
                    .unwrap_or_else(|| "Agent turn complete".to_string())
            }
            Notification::ExecApprovalRequested { command } => {
                format!(
                    "Approval requested: {}",
                    truncate_text(command, /*max_graphemes*/ 30)
                )
            }
            Notification::EditApprovalRequested { cwd, changes } => {
                format!(
                    "VAC wants to edit {}",
                    if changes.len() == 1 {
                        changes
                            .first()
                            .map(|change| display_path_for(change, cwd))
                            .unwrap_or_else(|| "0 files".to_string())
                    } else {
                        format!("{} files", changes.len())
                    }
                )
            }
            Notification::ElicitationRequested { server_name } => {
                format!("Approval requested by {server_name}")
            }
            Notification::PlanModePrompt { title } => {
                format!("Plan mode prompt: {title}")
            }
        }
    }

    fn type_name(&self) -> &str {
        match self {
            Notification::AgentTurnComplete { .. } => "agent-turn-complete",
            Notification::ExecApprovalRequested { .. }
            | Notification::EditApprovalRequested { .. }
            | Notification::ElicitationRequested { .. } => "approval-requested",
            Notification::PlanModePrompt { .. } => "plan-mode-prompt",
        }
    }

    fn priority(&self) -> u8 {
        match self {
            Notification::AgentTurnComplete { .. } => 0,
            Notification::ExecApprovalRequested { .. }
            | Notification::EditApprovalRequested { .. }
            | Notification::ElicitationRequested { .. }
            | Notification::PlanModePrompt { .. } => 1,
        }
    }

    fn allowed_for(&self, settings: &Notifications) -> bool {
        match settings {
            Notifications::Enabled(enabled) => *enabled,
            Notifications::Custom(allowed) => allowed.iter().any(|a| a == self.type_name()),
        }
    }

    fn agent_turn_preview(response: &str) -> Option<String> {
        let mut normalized = String::new();
        for part in response.split_whitespace() {
            if !normalized.is_empty() {
                normalized.push(' ');
            }
            normalized.push_str(part);
        }
        let trimmed = normalized.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(truncate_text(trimmed, AGENT_NOTIFICATION_PREVIEW_GRAPHEMES))
        }
    }

    fn user_input_request_summary(
        questions: &[crate::session_protocol::ToolRequestUserInputQuestion],
    ) -> Option<String> {
        let first_question = questions.first()?;
        let summary = if first_question.header.trim().is_empty() {
            first_question.question.trim()
        } else {
            first_question.header.trim()
        };
        if summary.is_empty() {
            None
        } else {
            Some(truncate_text(summary, /*max_graphemes*/ 30))
        }
    }
}

const AGENT_NOTIFICATION_PREVIEW_GRAPHEMES: usize = 200;

const PLACEHOLDERS: [&str; 8] = [
    "Explain this codebase",
    "Summarize recent commits",
    "Ask VAC to do anything",
    "Find and fix a bug in @filename",
    "Write tests for @filename",
    "Improve documentation in @filename",
    "Run /review on my current changes",
    "Use /skills to list available skills",
];

const SIDE_PLACEHOLDERS: [&str; 3] = [
    "Check recently modified functions for compatibility",
    "How many files have been modified?",
    "Will this algorithm scale well?",
];

// Extract the first bold (Markdown) element in the form **...** from `s`.
// Returns the inner text if found; otherwise `None`.
fn extract_first_bold(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'*' {
            let start = i + 2;
            let mut j = start;
            while j + 1 < bytes.len() {
                if bytes[j] == b'*' && bytes[j + 1] == b'*' {
                    // Found closing **
                    let inner = &s[start..j];
                    let trimmed = inner.trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.to_string());
                    } else {
                        return None;
                    }
                }
                j += 1;
            }
            // No closing; stop searching (wait for more deltas)
            return None;
        }
        i += 1;
    }
    None
}

#[cfg(test)]
pub(crate) fn show_review_commit_picker_with_entries(
    chat: &mut ChatWidget,
    entries: Vec<CommitLogEntry>,
) {
    let mut items: Vec<SelectionItem> = Vec::with_capacity(entries.len());
    for entry in entries {
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

    chat.bottom_pane.show_selection_view(SelectionViewParams {
        title: Some("Select a commit to review".to_string()),
        footer_hint: Some(standard_popup_hint_line()),
        items,
        is_searchable: true,
        search_placeholder: Some("Type to search commits".to_string()),
        ..Default::default()
    });
}

#[cfg(test)]
pub(crate) mod tests;
