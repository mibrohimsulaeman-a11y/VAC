// 00D-8 Activity Sidebar: a floating right-side panel that surfaces the
// latest typed [`crate::runtime_adapter::RuntimeActivityItem`] entries.
//
// The sidebar reads activity state directly from the chat widget; it does
// not own state. Activity items are pushed by
// `ChatWidget::project_runtime_events` from Local Runtime Contract events.
//
// Layout decisions live here so the App-level renderer can call
// [`split_for_activity_sidebar`] to position the floating card without
// duplicating constants.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::Wrap;

use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

use crate::runtime_adapter::RUNTIME_ACTIVITY_DEFAULT_LIMIT;
use crate::runtime_adapter::RuntimeActivityItem;
use crate::runtime_adapter::RuntimeActivityState;
use crate::runtime_adapter::RuntimeActivityStatus;

/// Minimum total terminal width required before the activity sidebar is
/// rendered. On narrower terminals the overlay is hidden and `/activity`
/// instead writes the timeline as a transcript cell.
pub(crate) const ACTIVITY_SIDEBAR_MIN_TOTAL_WIDTH: u16 = 120;

/// Preferred sidebar column width on wide terminals. The actual width is
/// capped at `area.width / 3` to keep the card compact.
pub(crate) const ACTIVITY_SIDEBAR_WIDTH: u16 = 36;

/// Preferred sidebar height on wide terminals. The actual height is capped
/// by the terminal height.
pub(crate) const ACTIVITY_SIDEBAR_HEIGHT: u16 = 12;

/// Split a render area into a `(main, Option<sidebar>)` pair. The main
/// rect is returned unchanged so the transcript still renders at full
/// width; the optional sidebar is a floating top-right overlay.
pub(crate) fn split_for_activity_sidebar(area: Rect) -> (Rect, Option<Rect>) {
    if area.width < ACTIVITY_SIDEBAR_MIN_TOTAL_WIDTH {
        return (area, None);
    }

    let sidebar_width = ACTIVITY_SIDEBAR_WIDTH.min(area.width);
    if sidebar_width == 0 {
        return (area, None);
    }

    let sidebar_height = ACTIVITY_SIDEBAR_HEIGHT.min(area.height);
    if sidebar_height == 0 {
        return (area, None);
    }

    let sidebar_area = Rect {
        x: area.x + area.width.saturating_sub(sidebar_width),
        y: area.y,
        width: sidebar_width,
        height: sidebar_height,
    };
    (area, Some(sidebar_area))
}

/// Compute the main-transcript width that should be used to lay out the
/// chat widget when the sidebar is visible.
///
/// The sidebar is a floating overlay, so it does not reduce transcript
/// width. This helper keeps the App-level layout code self-documenting.
pub(crate) fn main_width_for_activity_sidebar(total_width: u16) -> u16 {
    total_width
}

/// Renderer for the right-side Activity panel.
pub(crate) struct ActivitySidebar<'a> {
    state: &'a RuntimeActivityState,
}

impl<'a> ActivitySidebar<'a> {
    pub(crate) fn new(state: &'a RuntimeActivityState) -> Self {
        Self { state }
    }

    /// Render the sidebar into `area`. Renders the title, the latest
    /// (or all, when expanded) activity items, and a footer hint for
    /// `/activity`.
    pub(crate) fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        if !self.state.is_expanded() {
            return;
        }

        let lines = self.lines_for_area(area.width);
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }

    fn lines_for_area(&self, width: u16) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();
        let title_style = Style::default().add_modifier(Modifier::BOLD);
        lines.push(Line::from(Span::styled("Activity", title_style)));
        lines.push(Line::from(""));

        let items = self.state.visible_items(RUNTIME_ACTIVITY_DEFAULT_LIMIT);
        if items.is_empty() {
            lines.push(Line::from(Span::styled(
                "(no activity yet)",
                Style::default().add_modifier(Modifier::DIM),
            )));
        } else {
            for item in items {
                lines.extend(activity_item_lines(item, width));
            }
        }

        lines.push(Line::from(""));
        let footer = "/activity close";
        lines.push(Line::from(Span::styled(
            footer.to_string(),
            Style::default().add_modifier(Modifier::DIM),
        )));
        lines
    }
}

/// Marker character for an activity status. Kept intentionally short so a
/// 36-column sidebar still has room for the label.
fn status_marker(status: RuntimeActivityStatus) -> &'static str {
    match status {
        RuntimeActivityStatus::Started => "\u{2022}",
        RuntimeActivityStatus::Completed => "\u{2713}",
        RuntimeActivityStatus::Failed => "\u{2717}",
        RuntimeActivityStatus::Waiting => "\u{2026}",
        RuntimeActivityStatus::Interrupted => "!",
        RuntimeActivityStatus::Info => "-",
    }
}

/// Render a single activity item into one or two lines (marker+label and
/// indented detail) with width-aware truncation.
fn activity_item_lines(item: &RuntimeActivityItem, width: u16) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let marker = status_marker(item.status);
    let header_text = format!("{marker} {}", item.label);
    lines.push(Line::from(truncate_to_width(&header_text, width)));

    if let Some(detail) = item.detail.as_deref() {
        // Two-space indent so the eye groups detail under its header.
        let detail_text = format!("  {}", detail);
        lines.push(Line::from(Span::styled(
            truncate_to_width(&detail_text, width),
            Style::default().add_modifier(Modifier::DIM),
        )));
    }
    lines
}

/// Truncate a string to fit within `width` display columns. Adds an
/// ellipsis when truncation occurs. Walks chars by display width using
/// `unicode-width` so emojis and East-Asian wide chars are budgeted
/// correctly (e.g. "你好" counts as 4 columns, not 2). P1.8 / §E.1.
fn truncate_to_width(text: &str, width: u16) -> String {
    let width = width as usize;
    if width == 0 {
        return String::new();
    }
    let total_width = UnicodeWidthStr::width(text);
    if total_width <= width {
        return text.to_string();
    }
    if width == 1 {
        return "\u{2026}".to_string();
    }
    let budget = width.saturating_sub(1);
    let mut out = String::new();
    let mut acc: usize = 0;
    for ch in text.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0);
        if acc + w > budget {
            break;
        }
        out.push(ch);
        acc += w;
    }
    out.push('\u{2026}');
    out
}

/// Build a plain-text timeline of the full activity history for the
/// `/activity` slash command when the sidebar is hidden (narrow terminal).
/// Returned as a list of pre-formatted strings so the call site can wrap
/// them into a `PlainHistoryCell` without depending on ratatui types.
pub(crate) fn activity_timeline_strings(state: &RuntimeActivityState) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    out.push("Activity timeline".to_string());
    if state.all().is_empty() {
        out.push("(no activity yet)".to_string());
        return out;
    }
    for item in state.all() {
        let marker = status_marker(item.status);
        let line = match item.detail.as_deref() {
            Some(detail) => format!("{marker} {} \u{2014} {}", item.label, detail),
            None => format!("{marker} {}", item.label),
        };
        out.push(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_adapter::RuntimeActivityKind;

    fn make_item(
        kind: RuntimeActivityKind,
        status: RuntimeActivityStatus,
        label: &str,
        detail: Option<&str>,
    ) -> RuntimeActivityItem {
        RuntimeActivityItem {
            kind,
            status,
            label: label.to_string(),
            detail: detail.map(str::to_string),
        }
    }

    #[test]
    fn split_for_activity_sidebar_returns_none_on_narrow_terminal() {
        let area = Rect {
            x: 0,
            y: 0,
            width: ACTIVITY_SIDEBAR_MIN_TOTAL_WIDTH - 1,
            height: 30,
        };
        let (main, sidebar) = split_for_activity_sidebar(area);
        assert_eq!(main, area);
        assert!(sidebar.is_none());
    }

    #[test]
    fn split_for_activity_sidebar_splits_on_wide_terminal() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 160,
            height: 30,
        };
        let (main, sidebar) = split_for_activity_sidebar(area);
        let sidebar = sidebar.expect("sidebar should be present on wide terminal");
        assert_eq!(main, area);
        assert!(sidebar.width <= ACTIVITY_SIDEBAR_WIDTH);
        assert!(sidebar.height <= ACTIVITY_SIDEBAR_HEIGHT);
        assert_eq!(sidebar.x + sidebar.width, area.x + area.width);
        assert_eq!(sidebar.y, area.y);
    }

    #[test]
    fn truncate_to_width_keeps_short_text_intact() {
        let out = truncate_to_width("hello", 10);
        assert_eq!(out, "hello");
    }

    #[test]
    fn truncate_to_width_appends_ellipsis_when_too_long() {
        let out = truncate_to_width("abcdefgh", 5);
        assert_eq!(out.chars().count(), 5);
        assert!(out.ends_with('\u{2026}'));
    }

    #[test]
    fn activity_timeline_strings_renders_full_history() {
        let mut state = RuntimeActivityState::default();
        state.push(make_item(
            RuntimeActivityKind::Task,
            RuntimeActivityStatus::Started,
            "task started",
            Some("semantic coding"),
        ));
        state.push(make_item(
            RuntimeActivityKind::Command,
            RuntimeActivityStatus::Completed,
            "command completed",
            Some("ls -la"),
        ));
        let lines = activity_timeline_strings(&state);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "Activity timeline");
        assert!(lines[1].contains("task started"));
        assert!(lines[1].contains("semantic coding"));
        assert!(lines[2].contains("command completed"));
        assert!(lines[2].contains("ls -la"));
    }

    #[test]
    fn sidebar_hidden_when_collapsed() {
        let mut state = RuntimeActivityState::default();
        state.set_expanded(false);
        for i in 0..7 {
            state.push(make_item(
                RuntimeActivityKind::Command,
                RuntimeActivityStatus::Completed,
                "command completed",
                Some(&format!("item-{i}")),
            ));
        }
        let sidebar = ActivitySidebar::new(&state);
        let area = Rect::new(0, 0, 40, 12);
        let mut buf = Buffer::empty(area);
        for y in 0..area.height {
            for x in 0..area.width {
                buf[(x, y)]
                    .set_symbol(".")
                    .set_style(Style::default().bg(crate::ui_consts::APP_BG));
            }
        }

        sidebar.render(area, &mut buf);
        assert!(
            (0..area.height).all(|y| {
                (0..area.width).all(|x| {
                    let cell = &buf[(x, y)];
                    cell.symbol() == "." && cell.style().bg == Some(crate::ui_consts::APP_BG)
                })
            }),
            "collapsed sidebar should preserve the underlying frame"
        );
    }

    #[test]
    fn sidebar_open_returns_all_items() {
        let mut state = RuntimeActivityState::default();
        state.set_expanded(true);
        for i in 0..7 {
            state.push(make_item(
                RuntimeActivityKind::Command,
                RuntimeActivityStatus::Completed,
                "command completed",
                Some(&format!("item-{i}")),
            ));
        }
        let sidebar = ActivitySidebar::new(&state);
        let lines = sidebar.lines_for_area(40);
        let detail_lines: Vec<_> = lines
            .iter()
            .flat_map(|line| line.spans.iter().map(|s| s.content.clone()))
            .collect();
        for i in 0..7 {
            let needle = format!("item-{i}");
            assert!(
                detail_lines.iter().any(|s| s.contains(&needle)),
                "{needle} should be visible when expanded"
            );
        }
        assert!(
            detail_lines.iter().any(|s| s.contains("/activity close")),
            "footer should switch to close hint when open"
        );
    }
}
