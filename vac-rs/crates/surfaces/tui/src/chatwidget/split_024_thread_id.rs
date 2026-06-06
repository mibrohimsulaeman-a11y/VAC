    pub(crate) fn thread_id(&self) -> Option<ThreadId> {
        self.thread_id
    }

    pub(crate) fn thread_name(&self) -> Option<String> {
        self.thread_name.clone()
    }

    /// Returns the current thread's precomputed rollout path.
    ///
    /// For fresh non-ephemeral threads this path may exist before the file is
    /// materialized; rollout persistence is deferred until the first user
    /// message is recorded.
    pub(crate) fn rollout_path(&self) -> Option<PathBuf> {
        self.current_rollout_path.clone()
    }

    /// Returns a cache key describing the current in-flight active cell for the transcript overlay.
    ///
    /// `Ctrl+T` renders committed transcript cells plus a render-only live tail derived from the
    /// current active cell, and the overlay caches that tail; this key is what it uses to decide
    /// whether it must recompute. When there is no active cell, this returns `None` so the overlay
    /// can drop the tail entirely.
    ///
    /// If callers mutate the active cell's transcript output without bumping the revision (or
    /// providing an appropriate animation tick), the overlay will keep showing a stale tail while
    /// the main viewport updates.
    pub(crate) fn active_cell_transcript_key(&self) -> Option<ActiveCellTranscriptKey> {
        let cell = self.active_cell.as_ref();
        let hook_cell = self.active_hook_cell.as_ref();
        if cell.is_none() && hook_cell.is_none() {
            return None;
        }
        Some(ActiveCellTranscriptKey {
            revision: self.active_cell_revision,
            is_stream_continuation: cell
                .map(|cell| cell.is_stream_continuation())
                .unwrap_or(false),
            animation_tick: cell
                .and_then(|cell| cell.transcript_animation_tick())
                .or_else(|| {
                    hook_cell.and_then(super::history_cell::HistoryCell::transcript_animation_tick)
                }),
        })
    }

    /// Returns the active cell's transcript lines for a given terminal width.
    ///
    /// This is a convenience for the transcript overlay live-tail path, and it intentionally
    /// filters out empty results so the overlay can treat "nothing to render" as "no tail". Callers
    /// should pass the same width the overlay uses; using a different width will cause wrapping
    /// mismatches between the main viewport and the transcript overlay.
    pub(crate) fn active_cell_transcript_lines(&self, width: u16) -> Option<Vec<Line<'static>>> {
        let mut lines = Vec::new();
        if let Some(cell) = self.active_cell.as_ref() {
            lines.extend(cell.transcript_lines(width));
        }
        if let Some(hook_cell) = self.active_hook_cell.as_ref() {
            // Compute hook lines first so hidden hooks do not add a separator.
            let hook_lines = hook_cell.transcript_lines(width);
            if !hook_lines.is_empty() && !lines.is_empty() {
                lines.push("".into());
            }
            lines.extend(hook_lines);
        }
        (!lines.is_empty()).then_some(lines)
    }

    /// Return a reference to the widget's current config (includes any
    /// runtime overrides applied via TUI, e.g., model or approval policy).
    pub(crate) fn config_ref(&self) -> &Config {
        &self.config
    }

    #[cfg(test)]
    pub(crate) fn status_line_text(&self) -> Option<String> {
        self.bottom_pane.status_line_text()
    }

    pub(crate) fn clear_token_usage(&mut self) {
        self.token_info = None;
    }

    fn as_renderable(&self) -> RenderableItem<'_> {
        let active_cell_renderable = match &self.active_cell {
            Some(cell) => RenderableItem::Borrowed(cell).inset(Insets::tlbr(
                /*top*/ 1, /*left*/ 0, /*bottom*/ 0, /*right*/ 0,
            )),
            None => RenderableItem::Owned(Box::new(())),
        };
        let active_hook_cell_renderable = match &self.active_hook_cell {
            Some(cell) if cell.should_render() => {
                RenderableItem::Borrowed(cell).inset(Insets::tlbr(
                    /*top*/ 1, /*left*/ 0, /*bottom*/ 0, /*right*/ 0,
                ))
            }
            _ => RenderableItem::Owned(Box::new(())),
        };
        let mut flex = FlexRenderable::new();
        flex.push(/*flex*/ 1, active_cell_renderable);
        flex.push(/*flex*/ 0, active_hook_cell_renderable);
        flex.push(
            /*flex*/ 0,
            RenderableItem::Borrowed(&self.bottom_pane).inset(Insets::tlbr(
                /*top*/ 1, /*left*/ 0, /*bottom*/ 0, /*right*/ 0,
            )),
        );
        RenderableItem::Owned(Box::new(flex))
    }
}

#[cfg(not(target_os = "linux"))]
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
        self.stop_rate_limit_poller();
    }
}


impl ChatWidget {
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

    fn rendered_lines_key_for_cell(&self, idx: usize, width: u16, cell: &dyn HistoryCell) -> height_cache::RenderedLinesKey {
        let content_revision = self.active_cell_revision;
        let active_revision = cell
            .transcript_animation_tick()
            .unwrap_or(content_revision);
        height_cache::RenderedLinesKey {
            cell_id: idx as u64,
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
        let (transcript_area, bottom_area) = self.fullscreen_layout(area);
        self.render_windowed_transcript_cells(transcript_area, buf);
        self.bottom_pane_renderable().render(bottom_area, buf);
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
