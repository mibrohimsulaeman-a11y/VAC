
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
    fn transcript_cell_count_for_windowing(&self) -> usize {
        // Three renderable regions participate in ChatWidget layout: active cell,
        // active hook cell, and bottom pane. Keeping this count explicit prevents
        // the previous bug where total_cells was derived from total row height.
        3
    }

    fn transcript_cell_renderable(&self, idx: usize) -> RenderableItem<'_> {
        match idx {
            0 => match &self.active_cell {
                Some(cell) => RenderableItem::Borrowed(cell).inset(Insets::tlbr(1, 0, 0, 0)),
                None => RenderableItem::Owned(Box::new(())),
            },
            1 => match &self.active_hook_cell {
                Some(cell) if cell.should_render() => RenderableItem::Borrowed(cell).inset(Insets::tlbr(1, 0, 0, 0)),
                _ => RenderableItem::Owned(Box::new(())),
            },
            2 => RenderableItem::Borrowed(&self.bottom_pane).inset(Insets::tlbr(1, 0, 0, 0)),
            _ => RenderableItem::Owned(Box::new(())),
        }
    }

    fn rebuild_windowed_height_prefix(&self, width: u16) -> Vec<u16> {
        let total_cells = self.transcript_cell_count_for_windowing();
        let style_revision = self.style_revision();
        let mut cache = self.desired_height_cache.borrow_mut();
        let mut heights = Vec::with_capacity(total_cells);
        for idx in 0..total_cells {
            let key = height_cache::CellHeightKey {
                cell_id: idx as u64,
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
                self.transcript_cell_renderable(idx).render(child_area, buf);
            }
            y = y.saturating_add(height);
            if y >= area.bottom() { break; }
        }
    }
}

impl Renderable for ChatWidget {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.render_windowed_transcript_cells(area, buf);
        self.last_rendered_width.set(Some(area.width as usize));
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.desired_height_cache.borrow_mut().get_or_compute(
            width,
            self.active_cell_revision,
            || self.as_renderable().desired_height(width),
        )
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.as_renderable().cursor_pos(area)
    }

    fn cursor_style(&self, area: Rect) -> crossterm::cursor::SetCursorStyle {
        self.as_renderable().cursor_style(area)
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
    "Implement {feature}",
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
