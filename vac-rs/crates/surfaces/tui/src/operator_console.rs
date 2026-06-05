use crate::bottom_pane::BottomPaneView;
use crate::bottom_pane::CancellationEvent;
use crate::legacy_core::config::Config;
use crate::render::renderable::Renderable;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::Wrap;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OperatorConsoleTab {
    Chat,
    Runtime,
    Review,
    Workbench,
    Mcp,
}

impl OperatorConsoleTab {
    const ALL: [Self; 5] = [
        Self::Chat,
        Self::Runtime,
        Self::Review,
        Self::Workbench,
        Self::Mcp,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Runtime => "runtime",
            Self::Review => "review",
            Self::Workbench => "workbench",
            Self::Mcp => "mcp",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Chat => 1,
            Self::Runtime => 2,
            Self::Review => 3,
            Self::Workbench => 4,
            Self::Mcp => 5,
        }
    }

    fn next(self) -> Self {
        Self::ALL[self.index() % Self::ALL.len()]
    }

    fn previous(self) -> Self {
        let idx = self.index() - 1;
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

pub(crate) fn new_runtime_operator_console_view(config: &Config) -> OperatorConsoleView {
    OperatorConsoleView::new(config.cwd.to_path_buf(), OperatorConsoleTab::Runtime)
}

pub(crate) struct OperatorConsoleView {
    cwd: PathBuf,
    active_tab: OperatorConsoleTab,
    selected_job: String,
    last_action: Option<String>,
    last_refresh_epoch: u64,
    last_view_signature: String,
    last_render_signature: Option<u64>,
    lines: Vec<Line<'static>>,
    complete: bool,
}

impl OperatorConsoleView {
    fn new(cwd: PathBuf, active_tab: OperatorConsoleTab) -> Self {
        let mut view = Self {
            cwd,
            active_tab,
            selected_job: "none".to_string(),
            last_action: None,
            last_refresh_epoch: 0,
            last_view_signature: String::new(),
            last_render_signature: None,
            lines: Vec::new(),
            complete: false,
        };
        let _ = view.refresh_view(true);
        view
    }

    /// Refresh only the view model used by the bottom pane.
    ///
    /// This path is deliberately read-only: drawing the operator console must
    /// never execute scheduled workflows or spawn build checks. Autopilot
    /// execution is owned by the scheduler API / explicit retry action; the
    /// console tick only notices that persisted status files changed and then
    /// redraws.
    fn refresh_view(&mut self, force: bool) -> bool {
        let signature = self.current_view_signature();
        if !force && signature == self.last_view_signature {
            return false;
        }

        self.last_view_signature = signature;
        self.last_refresh_epoch = unix_epoch_seconds();
        let next_lines = self.render_active_tab_lines();
        let render_signature = rendered_lines_signature(&next_lines);
        if !force && Some(render_signature) == self.last_render_signature {
            return false;
        }

        self.last_render_signature = Some(render_signature);
        self.lines = next_lines;
        true
    }

    fn current_view_signature(&self) -> String {
        let mut parts = vec![format!("tab={}", self.active_tab.label())];
        if let Some(action) = &self.last_action {
            parts.push(format!("action={action}"));
        }
        if matches!(self.active_tab, OperatorConsoleTab::Runtime) {
            for rel in [
                ".vac/registry/autopilot/status.yaml",
                ".vac/registry/autopilot/run-state.yaml",
                ".vac/registry/autopilot/actions.log.yaml",
                ".vac/registry/autopilot/runs.log.yaml",
            ] {
                parts.push(file_signature(self.cwd.join(rel).as_path()));
            }
        }
        parts.join("|")
    }

    fn render_active_tab_lines(&mut self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        lines.push(self.render_header());
        lines.push(
            "persistent chrome: Tab/←/→ navigate · Esc/q closes · runtime: c cancel · r retry · i inspect · enter attach"
                .dim()
                .into(),
        );
        lines.push(
            format!(
                "live refresh: dirty-file poll only · last view update={}s",
                self.last_refresh_epoch
            )
            .dim()
            .into(),
        );
        lines.push("".into());

        match self.active_tab {
            OperatorConsoleTab::Runtime => {
                let state = crate::operator_ui::AutopilotSchedulerState::from_workspace(&self.cwd);
                self.selected_job = state.selected_job.clone();
                lines.extend(crate::operator_ui::render_autopilot_scheduler_visual_lines(
                    &state, 120,
                ));
            }
            OperatorConsoleTab::Chat => {
                lines.push(
                    "chat pane: primary transcript remains above this operator console"
                        .cyan()
                        .into(),
                );
                lines.push(
                    "runtime handoff: Default/Plan turns execute through OwnerNativeSession".into(),
                );
            }
            OperatorConsoleTab::Review => {
                lines.push("review pane: approval and policy queues are surfaced by the live approval overlay".yellow().into());
                lines.push("runtime note: destructive actions still fail closed until an approval decision is recorded".into());
            }
            OperatorConsoleTab::Workbench => {
                lines.push(
                    "workbench pane: workflow manifests are executable through /workflow run <id>"
                        .green()
                        .into(),
                );
                lines.push("runtime note: scheduled workflow jobs are driven by the autopilot workflow_runner tick".into());
            }
            OperatorConsoleTab::Mcp => {
                lines.push(
                    "mcp pane: connector/tool status remains owned by the app-server session"
                        .magenta()
                        .into(),
                );
                lines.push(
                    "runtime note: this tab is navigable chrome, not a static header string".into(),
                );
            }
        }

        if let Some(action) = &self.last_action {
            lines.push("".into());
            lines.push(format!("last runtime action: {action}").yellow().into());
        }
        lines
    }

    fn render_header(&self) -> Line<'static> {
        let tabs = OperatorConsoleTab::ALL
            .iter()
            .map(|tab| {
                if *tab == self.active_tab {
                    format!("[{}]", tab.label())
                } else {
                    tab.label().to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(" · ");
        format!(
            "vac · interactive — operator console   {tabs}   tab {}/5 ({})",
            self.active_tab.index(),
            self.active_tab.label()
        )
        .cyan()
        .bold()
        .into()
    }

    fn apply_action(&mut self, action: &'static str) {
        if !matches!(self.active_tab, OperatorConsoleTab::Runtime) {
            self.last_action = Some(format!("{action} ignored: switch to runtime tab first"));
            let _ = self.refresh_view(true);
            return;
        }
        let report = vac_core::control_plane::execute_vac_init_autopilot_action(
            &self.cwd,
            action,
            &self.selected_job,
        );
        self.last_action = Some(match report {
            Ok(report) => format!(
                "{} {} applied={} reason={}",
                report.action, report.job_id, report.applied, report.reason
            ),
            Err(err) => format!("{action} failed: {err}"),
        });
        let _ = self.refresh_view(true);
    }

    fn select_next_tab(&mut self) {
        self.active_tab = self.active_tab.next();
        let _ = self.refresh_view(true);
    }

    fn select_previous_tab(&mut self) {
        self.active_tab = self.active_tab.previous();
        let _ = self.refresh_view(true);
    }

    fn has_runtime_status_files(&self) -> bool {
        [
            ".vac/registry/autopilot/status.yaml",
            ".vac/registry/autopilot/run-state.yaml",
            ".vac/registry/autopilot/actions.log.yaml",
            ".vac/registry/autopilot/runs.log.yaml",
        ]
        .iter()
        .any(|rel| self.cwd.join(rel).exists())
    }
}

impl Renderable for OperatorConsoleView {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(self.lines.clone())
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        Paragraph::new(self.lines.clone())
            .wrap(Wrap { trim: false })
            .line_count(width)
            .try_into()
            .unwrap_or(u16::MAX)
    }
}

impl BottomPaneView for OperatorConsoleView {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => self.complete = true,
            KeyCode::Tab | KeyCode::Right => self.select_next_tab(),
            KeyCode::BackTab | KeyCode::Left => self.select_previous_tab(),
            KeyCode::Char('c') => self.apply_action("cancel"),
            KeyCode::Char('r') => self.apply_action("retry"),
            KeyCode::Char('i') => self.apply_action("inspect"),
            KeyCode::Enter => self.apply_action("attach"),
            _ => {}
        }
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn view_id(&self) -> Option<&'static str> {
        Some("operator-console")
    }

    fn active_tab_id(&self) -> Option<&str> {
        Some(self.active_tab.label())
    }

    fn pre_draw_tick(&mut self) -> bool {
        self.refresh_view(false)
    }

    fn next_pre_draw_tick_delay(&self) -> Option<Duration> {
        if !matches!(self.active_tab, OperatorConsoleTab::Runtime) {
            return None;
        }
        if self.last_action.is_none() && !self.has_runtime_status_files() {
            return None;
        }
        Some(Duration::from_secs(5))
    }

    fn on_ctrl_c(&mut self) -> CancellationEvent {
        self.complete = true;
        CancellationEvent::Handled
    }
}

fn file_signature(path: &Path) -> String {
    match fs::metadata(path) {
        Ok(metadata) => {
            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            format!("{}:{}:{}", path.display(), metadata.len(), modified)
        }
        Err(_) => format!("{}:<missing>", path.display()),
    }
}

fn unix_epoch_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn rendered_lines_signature(lines: &[Line<'static>]) -> u64 {
    let mut hasher = DefaultHasher::new();
    lines.len().hash(&mut hasher);
    for line in lines {
        line.spans.len().hash(&mut hasher);
        for span in &line.spans {
            span.content.hash(&mut hasher);
        }
    }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendered_lines_signature_changes_with_content_without_debug_string() {
        let left = vec![Line::from("runtime"), Line::from("queue 1")];
        let right = vec![Line::from("runtime"), Line::from("queue 2")];

        assert_ne!(
            rendered_lines_signature(&left),
            rendered_lines_signature(&right)
        );
    }
}
