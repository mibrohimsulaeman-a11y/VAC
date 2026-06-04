// Ratatui widget renderer for VAC operator screens.
//
// The plain `operator_ui` module owns deterministic semantic state. This
// module is the production visual renderer: it paints those states through
// real ratatui widgets (`Block`, `Tabs`, `Gauge`, `Table`, and `Layout`) into
// an off-screen buffer, then exposes styled terminal lines for the existing
// history surface. The off-screen buffer keeps the current transcript plumbing
// intact while preserving per-cell foreground, background, and modifier style
// from ratatui `Buffer` cells.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Gauge, Paragraph, Row, Table, Tabs, Widget, Wrap,
};

use crate::operator_ui::{
    AgentStreamingState, ApprovalPopupState, AutopilotSchedulerState, CapabilityDashboardRecord,
    CapabilityDashboardState, CapabilityManifestStatus, ControlPlaneDiagnostic, DiagnosticSeverity,
    IdleViewState, OperatorStatusBarState, OperatorViewport, SnapshotScenario, StartupSnapshot,
    ToolTimelineState,
};

const BG: Color = Color::Rgb(7, 13, 20);
const PANEL: Color = Color::Rgb(10, 20, 30);
const CHROME: Color = Color::Rgb(74, 92, 110);
const FG: Color = Color::Rgb(220, 235, 250);
const MUTED: Color = Color::Rgb(111, 130, 150);
const CYAN: Color = Color::Rgb(84, 185, 255);
const GREEN: Color = Color::Rgb(112, 214, 158);
const YELLOW: Color = Color::Rgb(241, 201, 95);
const RED: Color = Color::Rgb(255, 104, 116);
const BLUE_BG: Color = Color::Rgb(17, 43, 65);
const GREEN_BG: Color = Color::Rgb(13, 57, 39);
const YELLOW_BG: Color = Color::Rgb(74, 55, 16);
const RED_BG: Color = Color::Rgb(70, 20, 27);

#[derive(Clone, Debug)]
pub(crate) struct OperatorVisualLines {
    lines: Vec<Line<'static>>,
}

impl OperatorVisualLines {
    pub(crate) fn into_lines(self) -> Vec<Line<'static>> {
        self.lines
    }
}

pub(crate) fn render_snapshot_lines(
    scenario: SnapshotScenario,
    viewport: OperatorViewport,
) -> Vec<String> {
    let mut lines = match scenario {
        SnapshotScenario::FirstLaunch => {
            let snapshot = StartupSnapshot::from_session(
                "0.4.2-beta",
                "anthropic",
                "claude-sonnet-4.5",
                "~/code/vastar/vac",
                "27bd8ed4",
            );
            render_startup_lines(&snapshot, viewport.width as u16)
                .into_iter()
                .map(line_to_string)
                .collect::<Vec<String>>()
        }
        SnapshotScenario::Idle => {
            let state = IdleViewState::live("claude-sonnet-4.5");
            render_idle_lines(&state, viewport.width as u16)
                .into_iter()
                .map(line_to_string)
                .collect::<Vec<String>>()
        }
        SnapshotScenario::AgentWorking => {
            let state = AgentStreamingState::sample();
            render_agent_streaming_lines(&state, viewport.width as u16)
                .into_iter()
                .map(line_to_string)
                .collect::<Vec<String>>()
        }
        SnapshotScenario::ApprovalPopup => {
            let state = ApprovalPopupState::destructive_bash(
                "rm -rf target/debug/incremental",
                "~/code/vastar/vac",
            );
            render_approval_lines(&state, viewport.width as u16)
                .into_iter()
                .map(line_to_string)
                .collect::<Vec<String>>()
        }
        SnapshotScenario::RuntimeJobs => {
            let state = AutopilotSchedulerState::monitor_only_sample();
            render_runtime_jobs_lines(&state, viewport.width as u16)
                .into_iter()
                .map(line_to_string)
                .collect::<Vec<String>>()
        }
        SnapshotScenario::CapabilityDashboard => {
            let state = CapabilityDashboardState::sample_dashboard();
            render_capability_dashboard_lines(&state, viewport.width as u16)
                .into_iter()
                .map(line_to_string)
                .collect::<Vec<String>>()
        }
    };
    lines.truncate(viewport.height);
    lines
}

pub(crate) fn render_startup_lines(snapshot: &StartupSnapshot, width: u16) -> Vec<Line<'static>> {
    let width = width.clamp(72, 180);
    let height = 30;
    render_to_lines(width, height, |area, buf| {
        render_startup(snapshot, area, buf)
    })
}

pub(crate) fn render_idle_lines(state: &IdleViewState, width: u16) -> Vec<Line<'static>> {
    let width = width.clamp(72, 180);
    let height = 24;
    render_to_lines(width, height, |area, buf| render_idle(state, area, buf))
}

pub(crate) fn render_agent_streaming_lines(
    state: &AgentStreamingState,
    width: u16,
) -> Vec<Line<'static>> {
    let width = width.clamp(72, 180);
    let height = 31;
    render_to_lines(width, height, |area, buf| {
        render_agent_streaming(state, area, buf)
    })
}

pub(crate) fn render_approval_lines(state: &ApprovalPopupState, width: u16) -> Vec<Line<'static>> {
    let width = width.clamp(72, 180);
    let height = 24;
    render_to_lines(width, height, |area, buf| render_approval(state, area, buf))
}

pub(crate) fn render_runtime_jobs_lines(
    state: &AutopilotSchedulerState,
    width: u16,
) -> Vec<Line<'static>> {
    let width = width.clamp(72, 180);
    let height = 27;
    render_to_lines(width, height, |area, buf| {
        render_runtime_jobs(state, area, buf)
    })
}

pub(crate) fn render_capability_dashboard_lines(
    state: &CapabilityDashboardState,
    width: u16,
) -> Vec<Line<'static>> {
    let width = width.clamp(100, 220);
    let height = 42;
    render_to_lines(width, height, |area, buf| {
        render_capability_dashboard(state, area, buf)
    })
}

fn render_to_lines<F>(width: u16, height: u16, render: F) -> Vec<Line<'static>>
where
    F: FnOnce(Rect, &mut Buffer),
{
    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    fill_background(area, &mut buf);
    render(area, &mut buf);
    buffer_to_lines(&buf)
}

fn fill_background(area: Rect, buf: &mut Buffer) {
    for y in area.y..area.y.saturating_add(area.height) {
        for x in area.x..area.x.saturating_add(area.width) {
            buf[(x, y)].set_style(Style::default().fg(MUTED).bg(BG));
        }
    }
}

fn render_startup(snapshot: &StartupSnapshot, area: Rect, buf: &mut Buffer) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Length(1),
            Constraint::Length(8),
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Length(2),
        ])
        .split(area);
    render_chrome_header("vac · interactive", "first launch", chunks[0], buf);
    Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "  V A C  ",
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Vastar Agentic CLI",
                Style::default().fg(FG).add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("  {}", snapshot.version)),
        ]),
        Line::from(vec![
            Span::raw("         "),
            Span::styled(
                "VIL-native operator console · advanced beta · control-plane hardening active",
                Style::default().fg(MUTED),
            ),
        ]),
        Line::from(vec![
            Span::raw("         "),
            Span::styled(
                format!("cwd {} · session {}", snapshot.cwd, snapshot.session),
                Style::default().fg(MUTED),
            ),
        ]),
    ])
    .style(Style::default().bg(BG))
    .render(chunks[1], buf);
    Paragraph::new(Line::from(vec![
        Span::styled("hydrating startup snapshot", Style::default().fg(MUTED)),
        Span::raw(" ─────────────────────────────────────────────────"),
    ]))
    .style(Style::default().fg(MUTED).bg(BG))
    .render(chunks[2], buf);

    let body_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[3]);
    render_status_grid("VAC", &snapshot.rows_left, body_cols[0], buf);
    render_status_grid("Runtime", &snapshot.rows_right, body_cols[1], buf);

    let ready_lines = vec![
        Line::from(vec![
            Span::styled(
                "ready",
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" ─────────────────────────────────────────────────"),
        ]),
        Line::from(vec![
            pill("VIL-native", BLUE_BG, CYAN),
            Span::raw("   type / for commands, /help to explore, or start typing a task."),
        ]),
        Line::from(vec![
            Span::styled("recent", Style::default().fg(MUTED)),
            Span::raw("   "),
            Span::styled(
                snapshot
                    .recent_task
                    .as_deref()
                    .unwrap_or("no recent task loaded"),
                Style::default().fg(FG),
            ),
        ]),
    ];
    Paragraph::new(ready_lines)
        .style(Style::default().fg(FG).bg(BG))
        .render(chunks[4], buf);
    Paragraph::new(Line::from(vec![
        Span::styled("▸ ", Style::default().fg(CYAN)),
        Span::raw("/help · /model · /resume · /runtime · shift+tab plan mode"),
    ]))
    .block(rounded_block("composer"))
    .style(Style::default().fg(MUTED).bg(BG))
    .render(chunks[5], buf);
    render_status_bar(&snapshot.status_bar, chunks[6], buf);
}

fn render_idle(state: &IdleViewState, area: Rect, buf: &mut Buffer) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(9),
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Length(2),
        ])
        .split(area);
    render_chrome_header("vac · interactive", "idle", chunks[0], buf);
    let mut body = vec![Line::from(vec![
        pill("VIL-native", BLUE_BG, CYAN),
        Span::raw("   "),
        Span::styled(
            "ready",
            Style::default().fg(FG).add_modifier(Modifier::BOLD),
        ),
        Span::raw("   — "),
        Span::styled(state.engine_status.clone(), Style::default().fg(MUTED)),
        Span::raw(" · "),
        Span::styled(state.rulebook_status.clone(), Style::default().fg(MUTED)),
    ])];
    body.push(Line::from(""));
    body.push(Line::from(vec![Span::styled(
        "The agent will stream thinking, tools, and approvals here.",
        Style::default().fg(MUTED),
    )]));
    body.push(Line::from(vec![
        Span::styled("Keyboard-only:", Style::default().fg(MUTED)),
        Span::raw("   "),
        key_chip("tab"),
        Span::raw(" focus   "),
        key_chip("/"),
        Span::raw(" commands   "),
        key_chip("@"),
        Span::raw(" files   "),
        key_chip("shift+tab"),
        Span::raw(" plan mode"),
    ]));
    body.push(Line::from(""));
    body.push(Line::from(vec![Span::styled(
        "recent tasks",
        Style::default().fg(MUTED),
    )]));
    for task in &state.recent_tasks {
        body.push(Line::from(vec![
            Span::raw("› "),
            Span::styled(task.title.clone(), Style::default().fg(FG)),
            Span::raw("   "),
            Span::styled(task.meta.clone(), Style::default().fg(MUTED)),
        ]));
    }
    Paragraph::new(body)
        .wrap(Wrap { trim: false })
        .style(Style::default().bg(BG))
        .render(chunks[1], buf);
    if let Some(notice) = &state.update_notice {
        Paragraph::new(Line::from(vec![Span::styled(
            format!("• {notice}"),
            Style::default().fg(CYAN),
        )]))
        .block(rounded_block("update"))
        .style(Style::default().bg(BG))
        .render(chunks[2], buf);
    }
    Paragraph::new(Line::from(vec![
        Span::styled("▸ ", Style::default().fg(CYAN)),
        Span::raw(state.bottom_mode.clone()),
    ]))
    .block(rounded_block("composer"))
    .style(Style::default().bg(BG))
    .render(chunks[3], buf);
    render_status_bar(&state.status_bar, chunks[4], buf);
}

fn render_agent_streaming(state: &AgentStreamingState, area: Rect, buf: &mut Buffer) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(9),
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Length(2),
        ])
        .split(area);
    render_chrome_header("vac · interactive", "agent working", chunks[0], buf);
    let prompt = vec![
        Line::from(vec![
            Span::styled(
                "you",
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("  {}", state.user_time)),
        ]),
        Line::from(format!("  {}", state.user_prompt)),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "vac",
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "  {} · turn {} · {}",
                state.agent_model, state.turn, state.agent_time
            )),
        ]),
    ];
    Paragraph::new(prompt)
        .style(Style::default().fg(FG).bg(BG))
        .render(chunks[1], buf);

    let tool_rows = state.visible_tools().iter().map(|tool| {
        let state_style = style_for_tool_state(tool.state);
        Row::new(vec![
            Cell::from(tool_state_glyph(tool.state)).style(state_style),
            Cell::from(tool.name.clone()).style(Style::default().fg(FG)),
            Cell::from(tool.target.clone()).style(Style::default().fg(MUTED)),
            Cell::from(tool.state.as_str()).style(state_style),
            Cell::from(tool.detail.clone()).style(Style::default().fg(MUTED)),
        ])
    });
    Table::new(
        tool_rows,
        [
            Constraint::Length(2),
            Constraint::Length(14),
            Constraint::Percentage(38),
            Constraint::Length(12),
            Constraint::Percentage(34),
        ],
    )
    .header(
        Row::new(vec!["", "tool", "target", "state", "detail"]).style(Style::default().fg(MUTED)),
    )
    .block(rounded_block(if state.tools.len() > 5 {
        "tool timeline · last 5"
    } else {
        "tool timeline"
    }))
    .style(Style::default().bg(BG))
    .render(chunks[2], buf);

    let mut agent_lines = vec![Line::from(vec![
        Span::styled("⌁ thinking", Style::default().fg(YELLOW)),
        Span::raw(format!(" · {} ▌", state.thinking)),
    ])];
    agent_lines.extend(
        state
            .agent_message
            .lines()
            .map(|line| Line::from(line.to_string())),
    );
    Paragraph::new(agent_lines)
    .style(Style::default().fg(MUTED).bg(BG))
    .wrap(Wrap { trim: false })
    .render(chunks[3], buf);

    let ratio = ratio(state.context_used, state.context_limit);
    Gauge::default()
        .block(rounded_block("context"))
        .gauge_style(Style::default().fg(CYAN).bg(BLUE_BG))
        .label(format!(
            "{} / {}",
            compact_number(state.context_used),
            compact_number(state.context_limit)
        ))
        .ratio(ratio)
        .render(chunks[4], buf);
    Paragraph::new(Line::from(vec![
        Span::styled("▸ ", Style::default().fg(CYAN)),
        Span::raw(state.composer_hint.clone()),
    ]))
    .block(rounded_block("composer"))
    .style(Style::default().fg(MUTED).bg(BG))
    .render(chunks[5], buf);
    render_status_bar(&state.status_bar, chunks[6], buf);
}

fn render_approval(state: &ApprovalPopupState, area: Rect, buf: &mut Buffer) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(14),
            Constraint::Length(4),
            Constraint::Length(2),
        ])
        .split(area);
    render_chrome_header("vac · interactive", "destructive bash", chunks[0], buf);
    let risk_style = if state.destructive {
        Style::default()
            .fg(YELLOW)
            .bg(YELLOW_BG)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(GREEN).bg(GREEN_BG)
    };
    let body = vec![
        Line::from(vec![Span::styled(
            "approval required",
            Style::default().fg(YELLOW).add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            pill(&state.action_kind, YELLOW_BG, YELLOW),
            Span::raw("   the agent wants to run a shell command"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("$ ", Style::default().fg(CYAN)),
            Span::styled(state.operation.clone(), Style::default().fg(FG)),
        ]),
        Line::from(format!("cwd      {}", state.cwd)),
        Line::from(format!("context  {}", state.safety_context.render_inline())),
        Line::from(vec![
            Span::raw("risk     "),
            Span::styled(state.risk.label(), risk_style),
            Span::raw(format!("  {}", state.risk_detail)),
        ]),
        Line::from(format!(
            "policy   {} · {}",
            state.policy_gate_summary(),
            state.policy_reason
        )),
    ];
    Paragraph::new(body)
        .block(rounded_block("approval required").border_style(Style::default().fg(YELLOW)))
        .style(Style::default().fg(FG).bg(BG))
        .wrap(Wrap { trim: false })
        .render(chunks[1], buf);
    Paragraph::new(Line::from(vec![
        key_chip("y"),
        Span::raw(" approve once   "),
        key_chip("a"),
        Span::raw(" approve+remember   "),
        key_chip("n"),
        Span::raw(" reject   "),
        key_chip("r"),
        Span::raw(" reject with reason"),
    ]))
    .block(rounded_block(&state.batch_progress))
    .style(Style::default().fg(MUTED).bg(BG))
    .render(chunks[2], buf);
    render_status_bar(&state.status_bar, chunks[3], buf);
}

fn render_runtime_jobs(state: &AutopilotSchedulerState, area: Rect, buf: &mut Buffer) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(12),
            Constraint::Length(7),
            Constraint::Length(2),
        ])
        .split(area);
    render_chrome_header("vac · interactive", "runtime jobs", chunks[0], buf);
    Tabs::new(vec!["chat", "runtime", "review", "workbench", "mcp"])
        .select(1)
        .style(Style::default().fg(MUTED).bg(BG))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(CYAN)
                .add_modifier(Modifier::BOLD),
        )
        .divider(" · ")
        .render(chunks[1], buf);
    if state.jobs.is_empty() {
        Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "   Runtime idle · No scheduled jobs",
                Style::default().fg(MUTED),
            )),
        ])
        .block(rounded_block(&format!(
            "autopilot ● {}   pid {}   uptime {}   mode {}   queue {}   running {}",
            state.status, state.pid, state.uptime, state.mode, state.queued, state.running
        )))
        .style(Style::default().bg(BG))
        .render(chunks[2], buf);
    } else {
        let rows = state.jobs.iter().map(|job| {
            let st = job_state_style(&job.state);
            Row::new(vec![
                Cell::from("●").style(st),
                Cell::from(job.state.clone()).style(st),
                Cell::from(job.kind.clone()).style(Style::default().fg(FG)),
                Cell::from(job.id.clone()).style(Style::default().fg(CYAN)),
                Cell::from(job.trigger.clone()).style(Style::default().fg(FG)),
                Cell::from(job.age.clone()).style(Style::default().fg(MUTED)),
                Cell::from(job.next_run.clone()).style(Style::default().fg(MUTED)),
            ])
        });
        Table::new(
            rows,
            [
                Constraint::Length(2),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(9),
                Constraint::Percentage(42),
                Constraint::Length(12),
                Constraint::Percentage(18),
            ],
        )
        .header(
            Row::new(vec![
                "", "state", "kind", "id", "trigger", "age", "next-run",
            ])
            .style(Style::default().fg(MUTED)),
        )
        .block(rounded_block(&format!(
            "autopilot ● {}   pid {}   uptime {}   mode {}   queue {}   running {}",
            state.status, state.pid, state.uptime, state.mode, state.queued, state.running
        )))
        .style(Style::default().bg(BG))
        .render(chunks[2], buf);
    }

    let inspect_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(chunks[3]);
    Paragraph::new(vec![
        Line::from(format!(
            "task graph {}   {}",
            state.task_graph, state.node_progress
        )),
        Line::from(format!(
            "tokens {}   spend {}   retry {}",
            state.tokens, state.spend, state.retry
        )),
        Line::from(
            state
                .actions
                .iter()
                .map(|a| a.render_hotkey())
                .collect::<Vec<_>>()
                .join("   "),
        ),
        Line::from("policy: monitor-only scheduler; actions policy-gated; destructive tasks require approval gates"),
    ])
    .block(rounded_block(&format!("inspect {}", state.selected_job)))
    .style(Style::default().fg(FG).bg(BG))
    .wrap(Wrap { trim: false })
    .render(inspect_cols[0], buf);
    Gauge::default()
        .block(rounded_block("node progress"))
        .gauge_style(Style::default().fg(CYAN).bg(BLUE_BG))
        .label(state.node_progress.clone())
        .ratio(percent_from_text(&state.node_progress).unwrap_or(0.0))
        .render(inspect_cols[1], buf);
    render_status_bar(&state.status_bar, chunks[4], buf);
}

fn render_capability_dashboard(state: &CapabilityDashboardState, area: Rect, buf: &mut Buffer) {
    let screen = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Min(24),
            Constraint::Length(2),
        ])
        .split(area);
    render_chrome_header(&state.chrome_title, &state.chrome_mode, screen[0], buf);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
        ])
        .split(screen[1]);
    Paragraph::new(vec![
        Line::from(vec![Span::styled("TUI Capability Dashboard", Style::default().fg(FG).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled("Control plane registry view for manifest ownership, surfaces, policy, validation, and diagnostics.", Style::default().fg(MUTED))]),
    ]).style(Style::default().bg(BG)).render(top[0], buf);
    render_stat_card(
        "CAPABILITIES",
        state.capability_count.to_string(),
        top[1],
        buf,
    );
    render_stat_card(
        "OWNED DOMAINS",
        state.owned_domains.to_string(),
        top[2],
        buf,
    );
    render_stat_card("UNOWNED", state.unowned_domains.to_string(), top[3], buf);
    render_stat_card("VALID", format!("{}%", state.valid_percent), top[4], buf);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(55),
            Constraint::Percentage(25),
        ])
        .split(screen[2]);
    render_dashboard_left(state, body[0], buf);
    render_dashboard_center(state, body[1], buf);
    render_dashboard_right(state, body[2], buf);
    render_status_bar(&state.status_bar, screen[3], buf);
}

fn render_dashboard_left(state: &CapabilityDashboardState, area: Rect, buf: &mut Buffer) {
    let mut lines = vec![
        Line::from(vec![Span::styled(
            "› /capabilities",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "WORKSPACE FEED",
            Style::default().fg(MUTED),
        )]),
        Line::from(vec![
            Span::raw("registry     "),
            Span::styled(".vac/capabilities", Style::default().fg(CYAN)),
        ]),
        Line::from(vec![
            Span::raw("workflows    "),
            Span::styled(".vac/workflows", Style::default().fg(CYAN)),
        ]),
        Line::from(vec![
            Span::raw("surface      "),
            Span::styled(surface_summary(state), Style::default().fg(FG)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("ownership    "),
            Span::styled(
                format!("{} owned", state.owned_domains),
                Style::default().fg(GREEN),
            ),
        ]),
        Line::from(vec![
            Span::raw("unowned      "),
            Span::styled(
                format!("{} domains", state.unowned_domains),
                Style::default().fg(YELLOW),
            ),
        ]),
        Line::from(vec![
            Span::raw("policy       "),
            Span::styled(policy_summary(state), Style::default().fg(GREEN)),
        ]),
        Line::from(""),
    ];
    lines.extend(route_summary_lines(state));
    Paragraph::new(lines)
        .block(rounded_block("/capabilities"))
        .style(Style::default().fg(FG).bg(BG))
        .wrap(Wrap { trim: false })
        .render(area, buf);
}

fn render_dashboard_center(state: &CapabilityDashboardState, area: Rect, buf: &mut Buffer) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(16),
            Constraint::Length(6),
        ])
        .split(area);
    Tabs::new(vec!["all", "ready", "partial", "planned", "broken"])
        .select(0)
        .block(rounded_block("filters"))
        .style(Style::default().fg(MUTED).bg(BG))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(CYAN)
                .add_modifier(Modifier::BOLD),
        )
        .divider("   ")
        .render(chunks[0], buf);

    let rows: Vec<Row<'static>> = dashboard_records(state)
        .into_iter()
        .take(7)
        .map(|record| {
            let status_style = status_style(record.status);
            Row::new(vec![
                Cell::from(record.id).style(Style::default().fg(FG).add_modifier(Modifier::BOLD)),
                Cell::from(record.status.as_str()).style(status_style),
                Cell::from(record.owner).style(Style::default().fg(FG)),
                Cell::from(record.surfaces).style(Style::default().fg(MUTED)),
                Cell::from(record.policy).style(Style::default().fg(FG)),
                Cell::from(record.validation).style(Style::default().fg(CYAN)),
            ])
        })
        .collect();
    Table::new(
        rows,
        [
            Constraint::Percentage(15),
            Constraint::Percentage(10),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(30),
        ],
    )
    .header(
        Row::new(vec![
            "ID",
            "STATUS",
            "OWNER",
            "SURFACES",
            "POLICY",
            "VALIDATION / DOCS",
        ])
        .style(Style::default().fg(MUTED)),
    )
    .block(rounded_block("Capability registry"))
    .style(Style::default().bg(BG))
    .render(chunks[1], buf);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);
    Gauge::default()
        .block(rounded_block("Ownership scan"))
        .gauge_style(Style::default().fg(GREEN).bg(GREEN_BG))
        .label(format!("{} owned", state.owned_domains))
        .ratio(ratio(
            state.owned_domains as u64,
            (state.owned_domains + state.unowned_domains).max(1) as u64,
        ))
        .render(bottom[0], buf);
    Gauge::default()
        .block(rounded_block("Root feature conversion"))
        .gauge_style(Style::default().fg(CYAN).bg(BLUE_BG))
        .label(format!("{}% valid", state.valid_percent))
        .ratio(f64::from(state.valid_percent.min(100)) / 100.0)
        .render(bottom[1], buf);
}

fn render_dashboard_right(state: &CapabilityDashboardState, area: Rect, buf: &mut Buffer) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(9),
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Length(4),
        ])
        .split(area);
    let diagnostics = dashboard_diagnostics(state);
    let diag_lines = diagnostics
        .iter()
        .take(4)
        .flat_map(|diagnostic| {
            let style = diagnostic_style(diagnostic.severity);
            [
                Line::from(vec![
                    Span::styled(diagnostic.severity.as_str(), style),
                    Span::raw("  "),
                    Span::styled(
                        diagnostic.source.clone(),
                        Style::default().fg(FG).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(Span::styled(
                    diagnostic.message.clone(),
                    Style::default().fg(MUTED),
                )),
            ]
        })
        .collect::<Vec<_>>();
    let (diag_title, diag_body) = if diag_lines.is_empty() {
        (
            "Diagnostics · Registry parse".to_string(),
            vec![Line::from(Span::styled(
                "no YAML/control-plane errors detected",
                Style::default().fg(GREEN),
            ))],
        )
    } else {
        ("Diagnostics · YAML diagnostic".to_string(), diag_lines)
    };
    Paragraph::new(diag_body)
        .block(rounded_block(&diag_title))
        .style(Style::default().fg(FG).bg(BG))
        .wrap(Wrap { trim: false })
        .render(chunks[0], buf);

    let registry_health = registry_health(state);
    Gauge::default()
        .block(rounded_block("Registry parse"))
        .gauge_style(registry_health.style)
        .label(registry_health.label)
        .ratio(registry_health.ratio)
        .render(chunks[1], buf);
    let policy_health = policy_health(state);
    Gauge::default()
        .block(rounded_block("Policy gates"))
        .gauge_style(policy_health.style)
        .label(policy_health.label)
        .ratio(policy_health.ratio)
        .render(chunks[2], buf);
    let surface_health = surface_health(state);
    Gauge::default()
        .block(rounded_block("Surface routes"))
        .gauge_style(surface_health.style)
        .label(surface_health.label)
        .ratio(surface_health.ratio)
        .render(chunks[3], buf);
}

fn render_chrome_header(title: &str, mode: &str, area: Rect, buf: &mut Buffer) {
    let line = Line::from(vec![
        Span::styled("●", Style::default().fg(RED)),
        Span::raw(" "),
        Span::styled("●", Style::default().fg(YELLOW)),
        Span::raw(" "),
        Span::styled("●", Style::default().fg(GREEN)),
        Span::raw("     "),
        Span::styled(title, Style::default().fg(FG).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        pill(mode, BLUE_BG, CYAN),
    ]);
    Paragraph::new(line)
        .style(Style::default().bg(BG))
        .render(area, buf);
}

fn render_status_grid(
    title: &str,
    rows: &[crate::operator_ui::StartupSnapshotRow],
    area: Rect,
    buf: &mut Buffer,
) {
    let mut lines = Vec::with_capacity(rows.len() + 1);
    lines.push(Line::from(vec![Span::styled(
        title.to_string(),
        Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
    )]));
    for row in rows {
        lines.push(Line::from(vec![
            Span::styled("● ", Style::default().fg(GREEN)),
            Span::styled(format!("{:<10}", row.key), Style::default().fg(MUTED)),
            Span::styled(
                row.value.clone(),
                Style::default().fg(FG).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" · {}", row.detail), Style::default().fg(MUTED)),
        ]));
    }
    Paragraph::new(lines)
        .style(Style::default().bg(BG))
        .render(area, buf);
}

fn render_stat_card(label: &str, value: String, area: Rect, buf: &mut Buffer) {
    Paragraph::new(vec![
        Line::from(vec![Span::styled(
            value,
            Style::default().fg(FG).add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(label, Style::default().fg(MUTED))]),
    ])
    .block(rounded_block(""))
    .style(Style::default().bg(PANEL))
    .render(area, buf);
}

#[derive(Clone, Copy)]
enum StatusSuffix {
    Full,
    RulebookOnly,
    None,
}

fn status_bar_plain_len(
    state: &OperatorStatusBarState,
    validation: &str,
    suffix: StatusSuffix,
) -> usize {
    let suffix_text = match suffix {
        StatusSuffix::Full => format!(" │ {} · {}", state.profile, state.rulebook),
        StatusSuffix::RulebookOnly => format!(" │ {}", state.rulebook),
        StatusSuffix::None => String::new(),
    };
    format!(
        " {}  │ model {} │ {} │ {} │ {}{}",
        state.mode, state.model, state.token_usage, state.automation, validation, suffix_text
    )
    .chars()
    .count()
}

fn compact_validation_label(validation: &str) -> String {
    if validation.chars().count() <= 18 {
        return validation.to_string();
    }
    let lower = validation.to_ascii_lowercase();
    if lower.contains("valid") {
        "valid runtime".to_string()
    } else if lower.contains("approval") {
        "approval gate".to_string()
    } else {
        "runtime gate".to_string()
    }
}

fn status_suffix_for_width(
    state: &OperatorStatusBarState,
    validation: &str,
    width: usize,
) -> StatusSuffix {
    if status_bar_plain_len(state, validation, StatusSuffix::Full) <= width {
        StatusSuffix::Full
    } else if status_bar_plain_len(state, validation, StatusSuffix::RulebookOnly) <= width {
        StatusSuffix::RulebookOnly
    } else {
        StatusSuffix::None
    }
}

fn status_bar_line(
    state: &OperatorStatusBarState,
    validation: String,
    suffix: StatusSuffix,
) -> Line<'static> {
    let mut spans = vec![
        Span::styled(
            format!(" {} ", state.mode),
            Style::default()
                .fg(Color::Black)
                .bg(CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" │ model "),
        Span::styled(
            state.model.clone(),
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            " │ {} │ {} │ ",
            state.token_usage, state.automation
        )),
        Span::styled(validation, Style::default().fg(GREEN)),
    ];
    match suffix {
        StatusSuffix::Full => {
            spans.push(Span::raw(format!(
                " │ {} · {}",
                state.profile, state.rulebook
            )));
        }
        StatusSuffix::RulebookOnly => {
            spans.push(Span::raw(format!(" │ {}", state.rulebook)));
        }
        StatusSuffix::None => {}
    }
    Line::from(spans)
}

fn render_status_bar(state: &OperatorStatusBarState, area: Rect, buf: &mut Buffer) {
    let width = area.width as usize;
    let mut validation = state.validation.clone();
    if status_bar_plain_len(state, &validation, StatusSuffix::Full) > width {
        validation = compact_validation_label(&state.validation);
    }
    let suffix = status_suffix_for_width(state, &validation, width);
    let line = status_bar_line(state, validation, suffix);
    Paragraph::new(line)
        .style(Style::default().fg(MUTED).bg(BG))
        .render(area, buf);
}

fn rounded_block(title: &str) -> Block<'static> {
    Block::default()
        .title(title.to_string())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(CHROME))
        .style(Style::default().fg(FG).bg(BG))
}

fn pill(text: &str, bg: Color, fg: Color) -> Span<'static> {
    Span::styled(
        format!(" {text} "),
        Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
    )
}

fn key_chip(text: &str) -> Span<'static> {
    Span::styled(
        format!(" {text} "),
        Style::default()
            .fg(FG)
            .bg(PANEL)
            .add_modifier(Modifier::BOLD),
    )
}

fn tool_state_glyph(state: ToolTimelineState) -> &'static str {
    match state {
        ToolTimelineState::Queued | ToolTimelineState::Cancelled => "○",
        ToolTimelineState::Running
        | ToolTimelineState::Streaming
        | ToolTimelineState::Passed
        | ToolTimelineState::Failed => "●",
    }
}

fn style_for_tool_state(state: ToolTimelineState) -> Style {
    match state {
        ToolTimelineState::Queued => Style::default().fg(MUTED),
        ToolTimelineState::Running | ToolTimelineState::Streaming => Style::default().fg(CYAN),
        ToolTimelineState::Passed => Style::default().fg(GREEN),
        ToolTimelineState::Failed => Style::default().fg(RED),
        ToolTimelineState::Cancelled => Style::default().fg(MUTED),
    }
}

fn job_state_style(state: &str) -> Style {
    match state {
        "ok" | "ready" | "passed" => Style::default().fg(GREEN),
        "failed" | "broken" => Style::default().fg(RED),
        "running" => Style::default().fg(CYAN),
        "queued" | "partial" => Style::default().fg(YELLOW),
        _ => Style::default().fg(MUTED),
    }
}

fn status_style(status: CapabilityManifestStatus) -> Style {
    match status {
        CapabilityManifestStatus::Ready => Style::default().fg(GREEN).bg(GREEN_BG),
        CapabilityManifestStatus::Partial => Style::default().fg(YELLOW).bg(YELLOW_BG),
        CapabilityManifestStatus::Planned => Style::default().fg(CYAN).bg(BLUE_BG),
        CapabilityManifestStatus::Broken => Style::default().fg(RED).bg(RED_BG),
        CapabilityManifestStatus::Deprecated | CapabilityManifestStatus::Disabled => {
            Style::default().fg(MUTED).bg(PANEL)
        }
        CapabilityManifestStatus::Experimental | CapabilityManifestStatus::CliOnly => {
            Style::default().fg(CYAN).bg(PANEL)
        }
    }
}

fn diagnostic_style(severity: DiagnosticSeverity) -> Style {
    match severity {
        DiagnosticSeverity::Info => Style::default().fg(CYAN),
        DiagnosticSeverity::Warning => Style::default().fg(YELLOW),
        DiagnosticSeverity::Error => Style::default().fg(RED),
    }
}

fn dashboard_records(state: &CapabilityDashboardState) -> Vec<CapabilityDashboardRecord> {
    if !state.capability_records.is_empty() {
        return state.capability_records.clone();
    }
    state
        .capability_rows
        .iter()
        .map(|row| {
            let parts = row.split('|').map(str::trim).collect::<Vec<_>>();
            CapabilityDashboardRecord::new(
                parts.first().copied().unwrap_or("unknown"),
                parts
                    .get(1)
                    .and_then(|value| CapabilityManifestStatus::parse(value))
                    .unwrap_or(CapabilityManifestStatus::Broken),
                parts.get(2).copied().unwrap_or("unknown"),
                parts.get(3).copied().unwrap_or("unknown"),
                parts.get(4).copied().unwrap_or("unknown"),
                parts.get(5).copied().unwrap_or("unknown"),
            )
        })
        .collect()
}

fn dashboard_diagnostics(state: &CapabilityDashboardState) -> Vec<ControlPlaneDiagnostic> {
    if !state.diagnostic_records.is_empty() {
        return state.diagnostic_records.clone();
    }
    state
        .diagnostics
        .iter()
        .cloned()
        .map(ControlPlaneDiagnostic::from_text)
        .collect()
}

struct DashboardHealth {
    label: String,
    ratio: f64,
    style: Style,
}

fn registry_health(state: &CapabilityDashboardState) -> DashboardHealth {
    let diagnostics = dashboard_diagnostics(state);
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    {
        return DashboardHealth {
            label: "errors".to_string(),
            ratio: 0.25,
            style: Style::default().fg(RED).bg(RED_BG),
        };
    }
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warning)
    {
        return DashboardHealth {
            label: "degraded".to_string(),
            ratio: 0.75,
            style: Style::default().fg(YELLOW).bg(YELLOW_BG),
        };
    }
    DashboardHealth {
        label: "ready".to_string(),
        ratio: 1.0,
        style: Style::default().fg(GREEN).bg(GREEN_BG),
    }
}

fn policy_health(state: &CapabilityDashboardState) -> DashboardHealth {
    let records = dashboard_records(state);
    let total = records.len().max(1);
    let policy_backed = records
        .iter()
        .filter(|record| {
            let policy = record.policy.trim();
            !policy.is_empty() && policy != "unknown"
        })
        .count();
    let ratio = policy_backed as f64 / total as f64;
    DashboardHealth {
        label: format!("{policy_backed}/{total} mapped"),
        ratio,
        style: if ratio >= 0.8 {
            Style::default().fg(GREEN).bg(GREEN_BG)
        } else {
            Style::default().fg(YELLOW).bg(YELLOW_BG)
        },
    }
}

fn surface_health(state: &CapabilityDashboardState) -> DashboardHealth {
    let records = dashboard_records(state);
    let total = records.len().max(1);
    let surfaced = records
        .iter()
        .filter(|record| {
            let surfaces = record.surfaces.trim();
            !surfaces.is_empty() && surfaces != "unknown"
        })
        .count();
    let ratio = surfaced as f64 / total as f64;
    DashboardHealth {
        label: format!("{surfaced}/{total} routed"),
        ratio,
        style: if ratio >= 0.8 {
            Style::default().fg(GREEN).bg(GREEN_BG)
        } else {
            Style::default().fg(YELLOW).bg(YELLOW_BG)
        },
    }
}

fn surface_summary(state: &CapabilityDashboardState) -> String {
    let mut surfaces = dashboard_records(state)
        .into_iter()
        .flat_map(|record| {
            record
                .surfaces
                .split([',', '/', '+'])
                .map(str::trim)
                .filter(|value| !value.is_empty() && *value != "unknown")
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    surfaces.sort();
    surfaces.dedup();
    if surfaces.is_empty() {
        return "registry pending".to_string();
    }
    surfaces.into_iter().take(4).collect::<Vec<_>>().join(" + ")
}

fn policy_summary(state: &CapabilityDashboardState) -> String {
    let records = dashboard_records(state);
    let mapped = records
        .iter()
        .filter(|record| {
            let policy = record.policy.trim();
            !policy.is_empty() && policy != "unknown"
        })
        .count();
    format!("{mapped}/{} mapped", records.len().max(1))
}

fn route_summary_lines(state: &CapabilityDashboardState) -> Vec<Line<'static>> {
    let mut routes = dashboard_records(state)
        .into_iter()
        .take(4)
        .map(|record| {
            Line::from(vec![
                Span::styled(record.id, Style::default().fg(CYAN)),
                Span::raw("  "),
                Span::styled(record.status.as_str(), status_style(record.status)),
                Span::raw(" / "),
                Span::styled(record.surfaces, Style::default().fg(MUTED)),
            ])
        })
        .collect::<Vec<_>>();
    if routes.is_empty() {
        routes.push(Line::from(vec![Span::styled(
            "registry pending",
            Style::default().fg(MUTED),
        )]));
    }
    routes
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    (numerator as f64 / denominator as f64).clamp(0.0, 1.0)
}

fn percent_from_text(value: &str) -> Option<f64> {
    let percent = value
        .split('%')
        .next()?
        .rsplit_once(' ')
        .map(|(_, right)| right)
        .unwrap_or(value.split('%').next()?);
    percent
        .trim()
        .parse::<f64>()
        .ok()
        .map(|value| (value / 100.0).clamp(0.0, 1.0))
}

fn compact_number(value: u64) -> String {
    if value >= 1000 {
        format!("{}k", value / 1000)
    } else {
        value.to_string()
    }
}

fn buffer_to_lines(buf: &Buffer) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for y in buf.area.y..buf.area.y.saturating_add(buf.area.height) {
        let mut row_cells: Vec<(String, Style)> = Vec::new();
        for x in buf.area.x..buf.area.x.saturating_add(buf.area.width) {
            let cell = &buf[(x, y)];
            row_cells.push((cell.symbol().to_string(), cell.style()));
        }

        while row_cells
            .last()
            .is_some_and(|(symbol, style)| symbol.trim().is_empty() && style.bg == Some(BG))
        {
            row_cells.pop();
        }

        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut current_text = String::new();
        let mut current_style: Option<Style> = None;
        for entry in row_cells {
            let symbol: String = entry.0;
            let style: Style = entry.1;
            if current_style
                .as_ref()
                .is_some_and(|existing| existing == &style)
            {
                current_text.push_str(&symbol);
                continue;
            }
            if !current_text.is_empty() {
                spans.push(Span::styled(
                    current_text,
                    current_style.unwrap_or_default(),
                ));
            }
            current_text = symbol;
            current_style = Some(style);
        }
        if !current_text.is_empty() {
            spans.push(Span::styled(
                current_text,
                current_style.unwrap_or_default(),
            ));
        }
        lines.push(Line::from(spans));
    }
    while lines
        .last()
        .is_some_and(|line| line.spans.iter().all(|span| span.content.trim().is_empty()))
    {
        lines.pop();
    }
    lines
}

pub(crate) fn line_to_string(line: Line<'static>) -> String {
    line.spans
        .into_iter()
        .map(|span| span.content.into_owned())
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operator_ui::{CapabilityDashboardState, SnapshotScenario};

    #[test]
    fn buffer_conversion_preserves_cell_style_and_gauge_fill() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 1));
        buf[(0, 0)]
            .set_symbol("A")
            .set_style(Style::default().fg(CYAN).bg(BLUE_BG));
        buf[(1, 0)]
            .set_symbol(" ")
            .set_style(Style::default().fg(CYAN).bg(BLUE_BG));
        buf[(2, 0)]
            .set_symbol("B")
            .set_style(Style::default().fg(GREEN).bg(GREEN_BG));
        buf[(3, 0)]
            .set_symbol(" ")
            .set_style(Style::default().fg(MUTED).bg(BG));

        let lines = buffer_to_lines(&buf);
        assert_eq!(lines.len(), 1);
        assert_eq!(line_to_string(lines[0].clone()), "A B");
        assert_eq!(lines[0].spans.len(), 2);
        assert_eq!(lines[0].spans[0].style.bg, Some(BLUE_BG));
        assert_eq!(lines[0].spans[0].style.fg, Some(CYAN));
        assert_eq!(lines[0].spans[1].style.bg, Some(GREEN_BG));
        assert_eq!(lines[0].spans[1].style.fg, Some(GREEN));
    }

    #[test]
    fn widget_snapshot_contains_rounded_blocks_tabs_gauges_and_tables() {
        let state = CapabilityDashboardState::sample_dashboard();
        let rendered = render_capability_dashboard_lines(&state, 140)
            .into_iter()
            .map(line_to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains('╭'));
        assert!(rendered.contains("all"));
        assert!(rendered.contains("VALIDATION / DOCS"));
        assert!(rendered.contains("Policy gates"));
    }

    #[test]
    fn every_operator_snapshot_uses_widget_geometry() {
        for scenario in SnapshotScenario::ALL {
            let rendered = render_snapshot_lines(scenario, scenario.default_viewport()).join("\n");
            assert!(
                rendered.contains('╭'),
                "{scenario:?} should render rounded blocks"
            );
        }
    }
}
