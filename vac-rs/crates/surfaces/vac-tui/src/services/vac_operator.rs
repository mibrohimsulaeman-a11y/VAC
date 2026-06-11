//! VAC operator-console renderer.
//!
//! This renderer is intentionally fed by live `AppState` and `.vac/registry/*.json`
//! snapshots only. It must not bake in mock model names, fake runtime rows, or the
//! legacy upstream product labels from the design mockups.

use crate::app::AppState;
use crate::services::detect_term::ThemeColors;
use crate::services::message::get_wrapped_message_lines_cached;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Row, Table},
};
use serde_json::Value;
use std::fs;
use std::path::Path;

const STATUS_PATH: &str = ".vac/registry/status.json";
const RUNTIME_JOBS_PATH: &str = ".vac/registry/runtime/jobs.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VacOperatorRoute {
    #[default]
    Chat,
    RuntimeJobs,
    Review,
    Workbench,
    Mcp,
    Capabilities,
    Assessment,
    SpecSync,
    FirstLaunch,
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VacOperatorState {
    pub route: VacOperatorRoute,
}

// Kept for compatibility with the older view path. The VAC operator console
// now renders through `render_vac_operator_console`, so the legacy takeover
// hook is deliberately disabled.
pub fn should_render_operator_console(_state: &AppState) -> bool {
    false
}

pub fn render_operator_console(_f: &mut Frame, _state: &mut AppState, _area: Rect) {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperatorRoute {
    Chat,
    Runtime,
    Review,
    Workbench,
    Mcp,
    Capabilities,
    Assessment,
    SpecSync,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperatorMode {
    FirstLaunch,
    Idle,
    AgentWorking,
    ApprovalRequired,
    RuntimeJobs,
    Capabilities,
    Assessment,
    SpecSync,
}

#[derive(Debug, Clone)]
struct RegistrySnapshot {
    valid_label: String,
    conformance_level: String,
    compiled_current: bool,
    queued_jobs: u64,
    running_jobs: u64,
}

impl Default for RegistrySnapshot {
    fn default() -> Self {
        Self {
            valid_label: "valid unknown".to_string(),
            conformance_level: "L1".to_string(),
            compiled_current: false,
            queued_jobs: 0,
            running_jobs: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct RuntimeJobRow {
    state: String,
    kind: String,
    id: String,
    trigger: String,
    age: String,
    next_run: String,
}

#[derive(Debug, Clone)]
struct RuntimeRegistry {
    autopilot_state: String,
    pid: String,
    mode: String,
    env: String,
    queued: u64,
    running: u64,
    rows: Vec<RuntimeJobRow>,
    empty_state: String,
}

impl Default for RuntimeRegistry {
    fn default() -> Self {
        Self {
            autopilot_state: "not configured".to_string(),
            pid: "-".to_string(),
            mode: "monitor-only".to_string(),
            env: "host".to_string(),
            queued: 0,
            running: 0,
            rows: Vec::new(),
            empty_state: "No runtime jobs are registered yet.".to_string(),
        }
    }
}

pub fn render_vac_operator_console(f: &mut Frame, state: &mut AppState) -> bool {
    if std::env::var("VAC_LEGACY_TUI").ok().as_deref() == Some("1") {
        return false;
    }

    let area = f.area();
    f.render_widget(Clear, area);

    let route = active_route(state);
    let mode = active_mode(state, route);
    let registry = read_registry_snapshot();

    let horizontal = if area.width >= 104 {
        let side_width = if state.side_panel_state.is_shown {
            34
        } else {
            4
        };
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(64), Constraint::Length(side_width)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1)])
            .split(area)
    };
    let main_area = horizontal[0];
    let side_area = horizontal.get(1).copied();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(input_height(state)),
            Constraint::Length(1),
        ])
        .split(main_area);

    render_title_bar(f, chunks[0], mode);

    match mode {
        OperatorMode::RuntimeJobs => render_runtime_jobs(f, chunks[1], &registry),
        OperatorMode::Capabilities => render_capabilities(f, chunks[1]),
        OperatorMode::Assessment => render_assessment(f, chunks[1]),
        OperatorMode::SpecSync => render_spec_sync(f, chunks[1]),
        OperatorMode::ApprovalRequired => render_approval(f, chunks[1], state),
        OperatorMode::AgentWorking => render_agent_working(f, chunks[1], state),
        OperatorMode::FirstLaunch => render_first_launch(f, chunks[1], state, &registry),
        OperatorMode::Idle => render_idle(f, chunks[1], state, &registry),
    }

    render_input(f, chunks[2], state, mode);
    render_footer(f, chunks[3], state, route, &registry);

    if let Some(side_area) = side_area {
        if state.side_panel_state.is_shown {
            render_right_context_panel(f, side_area, state, &registry);
        } else {
            render_right_context_rail(f, side_area);
        }
    }

    render_operator_dropdown(f, state, chunks[2]);
    render_operator_overlays(f, state);
    true
}

fn active_route(state: &AppState) -> OperatorRoute {
    if state.plan_review_state.is_visible || state.plan_mode_state.is_active {
        return OperatorRoute::Review;
    }

    match state.vac_operator_state.route {
        VacOperatorRoute::RuntimeJobs => OperatorRoute::Runtime,
        VacOperatorRoute::Review => OperatorRoute::Review,
        VacOperatorRoute::Workbench => OperatorRoute::Workbench,
        VacOperatorRoute::Mcp => OperatorRoute::Mcp,
        VacOperatorRoute::Capabilities => OperatorRoute::Capabilities,
        VacOperatorRoute::Assessment => OperatorRoute::Assessment,
        VacOperatorRoute::SpecSync => OperatorRoute::SpecSync,
        VacOperatorRoute::FirstLaunch | VacOperatorRoute::Idle | VacOperatorRoute::Chat => {
            OperatorRoute::Chat
        }
    }
}

fn active_mode(state: &AppState, route: OperatorRoute) -> OperatorMode {
    let startup_hydrating = state
        .side_panel_state
        .session_start_time
        .elapsed()
        .as_secs_f32()
        < 3.0
        && state.messages_scrolling_state.messages.is_empty()
        && state.usage_tracking_state.total_session_usage.total_tokens == 0
        && state.input().is_empty();

    if state.dialog_approval_state.is_dialog_open
        || state.dialog_approval_state.approval_bar.is_visible()
    {
        OperatorMode::ApprovalRequired
    } else if route == OperatorRoute::Runtime {
        OperatorMode::RuntimeJobs
    } else if route == OperatorRoute::Capabilities {
        OperatorMode::Capabilities
    } else if route == OperatorRoute::Assessment {
        OperatorMode::Assessment
    } else if route == OperatorRoute::SpecSync {
        OperatorMode::SpecSync
    } else if state.loading_state.is_loading
        || state.loading_state.loading_manager.is_loading()
        || state.tool_call_state.is_streaming
        || state.tool_call_state.latest_tool_call.is_some()
    {
        OperatorMode::AgentWorking
    } else if matches!(
        state.vac_operator_state.route,
        VacOperatorRoute::FirstLaunch
    ) {
        OperatorMode::FirstLaunch
    } else if matches!(state.vac_operator_state.route, VacOperatorRoute::Idle) {
        OperatorMode::Idle
    } else if startup_hydrating {
        OperatorMode::FirstLaunch
    } else {
        OperatorMode::Idle
    }
}

fn input_height(state: &AppState) -> u16 {
    let lines = state.input().lines().count().clamp(1, 5) as u16;
    lines + 2
}

fn render_title_bar(f: &mut Frame, area: Rect, mode: OperatorMode) {
    let mode_label = match mode {
        OperatorMode::RuntimeJobs => "runtime jobs",
        OperatorMode::Capabilities => "capabilities",
        OperatorMode::Assessment => "assessment",
        OperatorMode::SpecSync => "spec-sync",
        OperatorMode::ApprovalRequired => "destructive bash",
        OperatorMode::AgentWorking => "agent working",
        OperatorMode::FirstLaunch => "first launch",
        OperatorMode::Idle => "idle",
    };
    let line = Line::from(vec![
        Span::styled("● ", Style::default().fg(ThemeColors::danger())),
        Span::styled("● ", Style::default().fg(ThemeColors::warning())),
        Span::styled("●   ", Style::default().fg(ThemeColors::success())),
        Span::styled(
            "vac",
            Style::default()
                .fg(ThemeColors::accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " · interactive — ",
            Style::default().fg(ThemeColors::muted()),
        ),
        Span::styled(mode_label, Style::default().fg(ThemeColors::accent())),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn render_first_launch(f: &mut Frame, area: Rect, state: &AppState, registry: &RegistrySnapshot) {
    let cwd = std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "workspace".to_string());
    let version = env!("CARGO_PKG_VERSION");
    let model = model_label(state);
    let session = session_label(state);
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "VAC",
                Style::default()
                    .fg(ThemeColors::accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  Vastar Agentic CLI  v{version}"),
                Style::default()
                    .fg(ThemeColors::text())
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            "manifest-driven operator console · control-plane hardening active",
            Style::default().fg(ThemeColors::muted()),
        )),
        Line::from(vec![
            Span::styled("cwd ", Style::default().fg(ThemeColors::muted())),
            Span::styled(
                cwd,
                Style::default()
                    .fg(ThemeColors::text())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" · session ", Style::default().fg(ThemeColors::muted())),
            Span::styled(
                session,
                Style::default()
                    .fg(ThemeColors::text())
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "hydrating startup snapshot",
            Style::default().fg(ThemeColors::muted()),
        )),
        key_value_line(
            "version",
            &format!("v{version}"),
            "compiled",
            yes_no(registry.compiled_current),
        ),
        key_value_line(
            "provider",
            provider_label(state).as_str(),
            "runtime",
            registry.conformance_level.as_str(),
        ),
        key_value_line(
            "model",
            &model,
            "profile",
            &state.profile_switcher_state.current_profile_name,
        ),
        key_value_line(
            "control",
            &registry.valid_label,
            "jobs",
            &format!(
                "{} queued / {} running",
                registry.queued_jobs, registry.running_jobs
            ),
        ),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "ready  ",
                Style::default()
                    .fg(ThemeColors::accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "type / for commands, @ for files, Ctrl+Y for context/tools, or start typing a task.",
                Style::default().fg(ThemeColors::text()),
            ),
        ]),
    ];
    append_recent_tasks(&mut lines, state, area.height.saturating_sub(16) as usize);
    f.render_widget(Paragraph::new(lines), inset(area, 2, 1));
}

fn render_idle(f: &mut Frame, area: Rect, state: &mut AppState, registry: &RegistrySnapshot) {
    if !state.messages_scrolling_state.messages.is_empty() {
        render_conversation_stream(f, area, state);
        return;
    }

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "VAC-native",
                Style::default()
                    .fg(ThemeColors::highlight_fg())
                    .bg(ThemeColors::accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  ready  ",
                Style::default()
                    .fg(ThemeColors::text())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "— control plane {} · runtime {}",
                    registry.valid_label, registry.conformance_level
                ),
                Style::default().fg(ThemeColors::muted()),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "The agent will stream thinking, tool calls, approvals, and final output here.",
            Style::default().fg(ThemeColors::muted()),
        )),
        Line::from(vec![
            Span::styled("Keyboard-only: ", Style::default().fg(ThemeColors::muted())),
            Span::styled(
                "tab",
                Style::default()
                    .fg(ThemeColors::text())
                    .bg(ThemeColors::border()),
            ),
            Span::styled(
                " focus   / commands   @ files   shift+tab plan   Ctrl+Y context/tools",
                Style::default().fg(ThemeColors::muted()),
            ),
        ]),
        Line::from(""),
    ];
    append_recent_tasks(&mut lines, state, area.height.saturating_sub(7) as usize);
    f.render_widget(Paragraph::new(lines), inset(area, 2, 1));
}

fn render_agent_working(f: &mut Frame, area: Rect, state: &mut AppState) {
    render_conversation_stream(f, area, state);
}

fn render_conversation_stream(f: &mut Frame, area: Rect, state: &mut AppState) {
    let width = area.width.saturating_sub(2) as usize;
    let mut lines = get_wrapped_message_lines_cached(state, width)
        .into_iter()
        .rev()
        .take(area.height as usize)
        .collect::<Vec<_>>();
    lines.reverse();
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "agent is preparing the first response…",
            Style::default().fg(ThemeColors::muted()),
        )));
    }
    f.render_widget(Paragraph::new(lines), area);
}

fn render_tool_timeline(f: &mut Frame, area: Rect, state: &AppState) {
    let mut rows = Vec::new();
    if let Some(tool) = &state.tool_call_state.latest_tool_call {
        rows.push(tool_row("streaming", &tool.function.name, &tool.id));
    }
    for tool in state
        .session_tool_calls_state
        .last_message_tool_calls
        .iter()
        .rev()
        .take(4)
    {
        rows.push(tool_row("pending", &tool.function.name, &tool.id));
    }
    if rows.is_empty() {
        rows.push(Line::from(Span::styled(
            "○ no tool calls in this turn yet",
            Style::default().fg(ThemeColors::muted()),
        )));
    }
    rows.truncate(5);
    let pct = state.usage_tracking_state.context_usage_percent.min(100);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(area);
    f.render_widget(
        Paragraph::new(rows).block(section_block("tool timeline · last 5")),
        chunks[0],
    );
    f.render_widget(
        Gauge::default()
            .block(Block::default().borders(Borders::NONE).title("context"))
            .gauge_style(Style::default().fg(ThemeColors::accent()))
            .percent(pct as u16)
            .label(format!(
                "{} tok · {}%",
                state.usage_tracking_state.total_session_usage.total_tokens, pct
            )),
        chunks[1],
    );
}

fn tool_row(state: &str, name: &str, id: &str) -> Line<'static> {
    let color = match state {
        "streaming" => ThemeColors::accent(),
        "pending" => ThemeColors::muted(),
        _ => ThemeColors::text(),
    };
    Line::from(vec![
        Span::styled("● ", Style::default().fg(color)),
        Span::styled(
            truncate(name, 24),
            Style::default()
                .fg(ThemeColors::text())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {}  {}", truncate(id, 10), state),
            Style::default().fg(ThemeColors::muted()),
        ),
    ])
}

fn render_approval(f: &mut Frame, area: Rect, state: &AppState) {
    let cmd = approval_command(state);
    let risk = classify_command(&cmd);
    let box_area = Rect {
        x: area.x + 3,
        y: area.y + 2,
        width: area.width.saturating_sub(6).min(112),
        height: 12.min(area.height.saturating_sub(4)),
    };
    f.render_widget(Clear, box_area);
    let mut lines = vec![
        Line::from(vec![Span::styled(
            "approval required",
            Style::default()
                .fg(ThemeColors::warning())
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled(
                " BASH ",
                Style::default()
                    .fg(ThemeColors::highlight_fg())
                    .bg(ThemeColors::warning())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  the agent wants to run a shell command",
                Style::default().fg(ThemeColors::text()),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("$ ", Style::default().fg(ThemeColors::accent())),
            Span::styled(
                cmd.clone(),
                Style::default()
                    .fg(ThemeColors::text())
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        key_value_line(
            "cwd",
            &std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "workspace".into()),
            "runtime",
            "host",
        ),
        key_value_line("risk", risk, "policy", "vac.core shell.destructive"),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " y ",
                Style::default()
                    .fg(ThemeColors::highlight_fg())
                    .bg(ThemeColors::success())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" approve once   ", Style::default().fg(ThemeColors::text())),
            Span::styled(
                " a ",
                Style::default()
                    .fg(ThemeColors::highlight_fg())
                    .bg(ThemeColors::border())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " approve+remember   ",
                Style::default().fg(ThemeColors::muted()),
            ),
            Span::styled(
                " n ",
                Style::default()
                    .fg(ThemeColors::highlight_fg())
                    .bg(ThemeColors::danger())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" reject   ", Style::default().fg(ThemeColors::text())),
            Span::styled(
                " r ",
                Style::default()
                    .fg(ThemeColors::highlight_fg())
                    .bg(ThemeColors::border())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " reject with reason",
                Style::default().fg(ThemeColors::muted()),
            ),
        ]),
    ];
    while lines.len() < box_area.height.saturating_sub(2) as usize {
        lines.push(Line::from(""));
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ThemeColors::warning()))
        .title(" approval required ");
    f.render_widget(Paragraph::new(lines).block(block), box_area);
}

fn render_runtime_jobs(f: &mut Frame, area: Rect, registry: &RegistrySnapshot) {
    let runtime = read_runtime_registry(registry);
    let content = inset(area, 2, 1);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(4),
            Constraint::Length(5),
        ])
        .split(content);

    let status = Line::from(vec![
        Span::styled("autopilot ", Style::default().fg(ThemeColors::muted())),
        Span::styled(
            format!("● {}", runtime.autopilot_state),
            Style::default()
                .fg(runtime_color(&runtime.autopilot_state))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "   pid {}   mode {}   env {}   ·   queue {}   ·   running {}",
                runtime.pid, runtime.mode, runtime.env, runtime.queued, runtime.running
            ),
            Style::default().fg(ThemeColors::muted()),
        ),
    ]);
    f.render_widget(Paragraph::new(status), chunks[0]);

    if runtime.rows.is_empty() {
        let lines = vec![
            Line::from(Span::styled(
                "No runtime jobs registered",
                Style::default()
                    .fg(ThemeColors::text())
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                runtime.empty_state,
                Style::default().fg(ThemeColors::muted()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "When the runtime writes one-shot, cron, or filewatch records to .vac/registry/runtime/jobs.json, they appear here.",
                Style::default().fg(ThemeColors::muted()),
            )),
        ];
        f.render_widget(
            Paragraph::new(lines).block(section_block("runtime jobs")),
            chunks[1],
        );
    } else {
        let rows = runtime.rows.iter().map(|r| {
            Row::new(vec![
                r.state.clone(),
                r.kind.clone(),
                r.id.clone(),
                r.trigger.clone(),
                r.age.clone(),
                r.next_run.clone(),
            ])
        });
        let table = Table::new(
            rows,
            [
                Constraint::Length(10),
                Constraint::Length(11),
                Constraint::Length(12),
                Constraint::Min(18),
                Constraint::Length(9),
                Constraint::Length(16),
            ],
        )
        .header(
            Row::new(vec!["state", "kind", "id", "trigger", "age", "next-run"])
                .style(Style::default().fg(ThemeColors::muted())),
        )
        .block(section_block("runtime jobs"));
        f.render_widget(table, chunks[1]);
    }

    let inspect = vec![
        Line::from(vec![
            Span::styled("inspect", Style::default().fg(ThemeColors::muted())),
            Span::styled(
                "  registry source ",
                Style::default().fg(ThemeColors::text()),
            ),
            Span::styled(
                RUNTIME_JOBS_PATH,
                Style::default().fg(ThemeColors::accent()),
            ),
        ]),
        Line::from(vec![
            Span::styled("actions", Style::default().fg(ThemeColors::muted())),
            Span::styled(
                "  c cancel   r retry   i open in editor   enter attach",
                Style::default().fg(ThemeColors::text()),
            ),
        ]),
    ];
    f.render_widget(
        Paragraph::new(inspect).block(section_block("job inspect")),
        chunks[2],
    );
}

fn render_capabilities(f: &mut Frame, area: Rect) {
    let rows = read_capability_rows();
    if rows.is_empty() {
        let lines = vec![
            Line::from(Span::styled(
                "No compiled capabilities found",
                Style::default()
                    .fg(ThemeColors::text())
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Run `vac compile registry .` after editing .vac/capabilities/*.yaml.",
                Style::default().fg(ThemeColors::muted()),
            )),
        ];
        f.render_widget(
            Paragraph::new(lines).block(section_block("capability dashboard")),
            inset(area, 2, 1),
        );
        return;
    }
    let table_rows = rows
        .into_iter()
        .take(area.height.saturating_sub(4) as usize)
        .map(Row::new);
    let table = Table::new(
        table_rows,
        [
            Constraint::Min(28),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Min(16),
        ],
    )
    .header(
        Row::new(vec![
            "capability",
            "declared",
            "computed",
            "effective",
            "owner",
        ])
        .style(Style::default().fg(ThemeColors::muted())),
    )
    .block(section_block(
        "capability dashboard · declared/computed/effective",
    ));
    f.render_widget(table, inset(area, 2, 1));
}

fn render_assessment(f: &mut Frame, area: Rect) {
    let report = read_json_file(Path::new(".vac/assessment/gap_report.json"));
    let findings = report
        .as_ref()
        .and_then(|v| v.get("findings"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if findings.is_empty() {
        let lines = vec![
            Line::from(Span::styled(
                "No assessment findings recorded",
                Style::default()
                    .fg(ThemeColors::text())
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Assessment rows load from .vac/assessment/gap_report.json with span-grounded evidence.",
                Style::default().fg(ThemeColors::muted()),
            )),
        ];
        f.render_widget(
            Paragraph::new(lines).block(section_block("assessment")),
            inset(area, 2, 1),
        );
        return;
    }
    let table_rows = findings
        .into_iter()
        .take(area.height.saturating_sub(4) as usize)
        .map(|finding| {
            Row::new(vec![
                json_field(&finding, &["finding_id"]),
                json_field(&finding, &["category"]),
                json_field(&finding, &["severity"]),
                json_field(&finding, &["recommendation"]),
            ])
        });
    let table = Table::new(
        table_rows,
        [
            Constraint::Length(24),
            Constraint::Length(24),
            Constraint::Length(8),
            Constraint::Min(24),
        ],
    )
    .header(
        Row::new(vec!["finding", "category", "severity", "recommendation"])
            .style(Style::default().fg(ThemeColors::muted())),
    )
    .block(section_block("assessment · intent/code/baseline"));
    f.render_widget(table, inset(area, 2, 1));
}

fn render_spec_sync(f: &mut Frame, area: Rect) {
    let report = read_json_file(Path::new(".vac/registry/spec-sync/current.json"));
    let mut lines = vec![Line::from(Span::styled(
        "SpecSync",
        Style::default()
            .fg(ThemeColors::accent())
            .add_modifier(Modifier::BOLD),
    ))];
    if let Some(report) = report {
        lines.push(Line::from(format!(
            "trigger: {}",
            json_field(&report, &["trigger", "reason"])
        )));
        let drift = report
            .get("drift")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if drift.is_empty() {
            lines.push(Line::from(Span::styled(
                "no unresolved critical drift",
                Style::default().fg(ThemeColors::success()),
            )));
        } else {
            for item in drift.iter().take(area.height.saturating_sub(4) as usize) {
                lines.push(Line::from(format!(
                    "{} · {} · {}",
                    json_field(item, &["severity"]),
                    json_field(item, &["capability"]),
                    json_field(item, &["type"])
                )));
            }
        }
        lines.push(Line::from(Span::styled(
            "generated updates remain proposals until approved and compiled",
            Style::default().fg(ThemeColors::muted()),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            ".vac/registry/spec-sync/current.json not present yet",
            Style::default().fg(ThemeColors::muted()),
        )));
    }
    f.render_widget(
        Paragraph::new(lines).block(section_block("spec-sync")),
        inset(area, 2, 1),
    );
}

fn read_capability_rows() -> Vec<Vec<String>> {
    let Some(value) = read_json_file(Path::new(".vac/registry/compiled/capabilities.json")) else {
        return Vec::new();
    };
    value
        .get("capabilities")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|cap| {
                    vec![
                        json_field(cap, &["id"]),
                        json_field(cap, &["readiness", "declared"]),
                        json_field(cap, &["readiness", "computed"]),
                        json_field(cap, &["readiness", "effective"]),
                        json_field(cap, &["owner", "crate"]),
                    ]
                })
                .collect()
        })
        .unwrap_or_default()
}

fn read_json_file(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
}

fn json_field(value: &Value, path: &[&str]) -> String {
    let mut cursor = value;
    for key in path {
        let Some(next) = cursor.get(*key) else {
            return String::new();
        };
        cursor = next;
    }
    cursor
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| cursor.as_u64().map(|v| v.to_string()))
        .or_else(|| cursor.as_i64().map(|v| v.to_string()))
        .or_else(|| cursor.as_bool().map(|v| v.to_string()))
        .unwrap_or_default()
}

fn append_recent_tasks(lines: &mut Vec<Line<'static>>, state: &AppState, max_items: usize) {
    lines.push(Line::from(Span::styled(
        "recent tasks",
        Style::default().fg(ThemeColors::muted()),
    )));
    let visible = max_items.clamp(1, 5);
    if state.sessions_state.sessions.is_empty() {
        lines.push(Line::from(Span::styled(
            "› no recent tasks recorded yet",
            Style::default().fg(ThemeColors::muted()),
        )));
    } else {
        for session in state.sessions_state.sessions.iter().take(visible) {
            lines.push(Line::from(vec![
                Span::styled("› ", Style::default().fg(ThemeColors::text())),
                Span::styled(
                    truncate(&session.title, 42),
                    Style::default().fg(ThemeColors::text()),
                ),
                Span::styled(
                    format!(
                        "   {} · {} checkpoints",
                        session.updated_at,
                        session.checkpoints.len()
                    ),
                    Style::default().fg(ThemeColors::muted()),
                ),
            ]));
        }
    }
}

fn render_right_context_rail(f: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(Span::styled(
            "ctx",
            Style::default()
                .fg(ThemeColors::accent())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "│",
            Style::default().fg(ThemeColors::border()),
        )),
        Line::from(Span::styled(
            "tls",
            Style::default()
                .fg(ThemeColors::muted())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "^Y",
            Style::default().fg(ThemeColors::muted()),
        )),
    ];
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(ThemeColors::border())),
        ),
        area,
    );
}

fn render_operator_overlays(f: &mut Frame, state: &mut AppState) {
    if state.model_switcher_state.is_visible {
        crate::services::model_switcher::render_model_switcher_popup(f, state);
    }
    if state.profile_switcher_state.show_profile_switcher {
        crate::services::profile_switcher::render_profile_switcher_popup(f, state);
    }
    if state.rulebook_switcher_state.show_rulebook_switcher {
        crate::services::rulebook_switcher::render_rulebook_switcher_popup(f, state);
    }
    if state.shortcuts_panel_state.is_visible {
        crate::services::shortcuts_popup::render_shortcuts_popup(f, state);
    }
    if state.file_changes_popup_state.is_visible {
        crate::services::file_changes_popup::render_file_changes_popup(f, state);
    }
    if state.tool_approval_popup_state.is_visible {
        crate::services::auto_approve_popup::render_auto_approve_popup(f, state);
    }
}

fn render_right_context_panel(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    registry: &RegistrySnapshot,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(5),
            Constraint::Min(5),
        ])
        .split(inset(area, 1, 0));
    let header = vec![
        Line::from(vec![
            Span::styled(
                " context ",
                Style::default()
                    .fg(ThemeColors::highlight_fg())
                    .bg(ThemeColors::highlight_bg())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" tools ", Style::default().fg(ThemeColors::muted())),
        ]),
        key_value_line(
            "profile",
            &state.profile_switcher_state.current_profile_name,
            "runtime",
            &registry.conformance_level,
        ),
    ];
    f.render_widget(
        Paragraph::new(header).block(section_block("right panel · Ctrl+Y")),
        chunks[0],
    );

    let active_model = state
        .model_switcher_state
        .current_model
        .as_ref()
        .unwrap_or(&state.configuration_state.model);
    let max_tokens = active_model.limit.context;
    let used = state
        .usage_tracking_state
        .current_message_usage
        .prompt_tokens as u64;
    let pct = used
        .saturating_mul(100)
        .checked_div(max_tokens)
        .unwrap_or(0)
        .min(100);
    f.render_widget(
        Gauge::default()
            .block(section_block("context window"))
            .gauge_style(Style::default().fg(ThemeColors::accent()))
            .percent(pct as u16)
            .label(format!("{} / {} tok · {}%", used, max_tokens, pct)),
        chunks[1],
    );
    render_tool_timeline(f, chunks[2], state);
}

fn render_operator_dropdown(f: &mut Frame, state: &AppState, input_area: Rect) {
    if !state.input_state.show_helper_dropdown {
        return;
    }
    let height = 8.min(input_area.y.saturating_sub(1));
    if height == 0 {
        return;
    }
    let area = Rect {
        x: input_area.x,
        y: input_area.y.saturating_sub(height),
        width: input_area.width,
        height,
    };
    crate::services::helper_dropdown::render_file_search_dropdown(f, state, area);
}

fn provider_label(state: &AppState) -> String {
    let provider = &state.configuration_state.model.provider;
    if provider.trim().is_empty() {
        "configured".to_string()
    } else {
        provider.clone()
    }
}

fn render_input(f: &mut Frame, area: Rect, state: &AppState, mode: OperatorMode) {
    let prompt = match mode {
        OperatorMode::ApprovalRequired => "approval blocking — y · a · n · r   (esc to defer)",
        OperatorMode::AgentWorking => "agent working — next message queues (esc to interrupt)",
        OperatorMode::RuntimeJobs => "runtime monitor — c cancel · r retry · enter attach",
        OperatorMode::Capabilities => "capability dashboard — declared/computed/effective",
        OperatorMode::Assessment => "assessment — intent/code/baseline gap review",
        OperatorMode::SpecSync => "spec-sync — approve or defer drift proposals",
        _ => "/ commands   @ files   shift+tab plan   Ctrl+Y context/tools   ctrl+j newline",
    };
    let mut content = state.input().to_string();
    if content.is_empty() {
        content = prompt.to_string();
    }
    let line = Line::from(vec![
        Span::styled(
            "› ",
            Style::default()
                .fg(ThemeColors::accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            content,
            Style::default().fg(if state.input().is_empty() {
                ThemeColors::muted()
            } else {
                ThemeColors::text()
            }),
        ),
    ]);
    f.render_widget(
        Paragraph::new(line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ThemeColors::border())),
        ),
        area,
    );
}

fn render_footer(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    route: OperatorRoute,
    registry: &RegistrySnapshot,
) {
    let route_label = match route {
        OperatorRoute::Chat => "INPUT",
        OperatorRoute::Runtime => "WORKBENCH",
        OperatorRoute::Review | OperatorRoute::Capabilities => "REVIEW",
        OperatorRoute::Workbench | OperatorRoute::Assessment | OperatorRoute::SpecSync => {
            "WORKBENCH"
        }
        OperatorRoute::Mcp => "MCP",
    };
    let model = model_label(state);
    let tokens = state.usage_tracking_state.total_session_usage.total_tokens;
    let approval_mode = if state
        .configuration_state
        .allowed_tools
        .as_ref()
        .map(|v| v.is_empty())
        .unwrap_or(true)
    {
        "manual"
    } else {
        "bounded"
    };
    let rulebook = rulebook_label(state);
    let line = Line::from(vec![
        Span::styled(
            format!(" {route_label} "),
            Style::default()
                .fg(ThemeColors::highlight_fg())
                .bg(ThemeColors::highlight_bg())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" | model ", Style::default().fg(ThemeColors::muted())),
        Span::styled(
            model,
            Style::default()
                .fg(ThemeColors::accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" | {} tok | ", tokens),
            Style::default().fg(ThemeColors::text()),
        ),
        Span::styled(approval_mode, Style::default().fg(ThemeColors::success())),
        Span::styled(
            format!(
                " | {} | profile {} · rulebook {}",
                registry.valid_label, state.profile_switcher_state.current_profile_name, rulebook
            ),
            Style::default().fg(ThemeColors::muted()),
        ),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn key_value_line(
    left_key: &str,
    left_value: &str,
    right_key: &str,
    right_value: &str,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{left_key:<10}"),
            Style::default().fg(ThemeColors::muted()),
        ),
        Span::styled(
            truncate(left_value, 34),
            Style::default()
                .fg(ThemeColors::text())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("   ·   ", Style::default().fg(ThemeColors::muted())),
        Span::styled(
            format!("{right_key:<10}"),
            Style::default().fg(ThemeColors::muted()),
        ),
        Span::styled(
            truncate(right_value, 34),
            Style::default()
                .fg(ThemeColors::text())
                .add_modifier(Modifier::BOLD),
        ),
    ])
}

fn section_block(title: &'static str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ThemeColors::border()))
        .title(title)
}

fn inset(area: Rect, x: u16, y: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(x),
        y: area.y.saturating_add(y),
        width: area.width.saturating_sub(x.saturating_mul(2)),
        height: area.height.saturating_sub(y.saturating_mul(2)),
    }
}

fn model_label(state: &AppState) -> String {
    let model = &state.configuration_state.model;
    if !model.name.trim().is_empty() {
        model.name.clone()
    } else if !model.id.trim().is_empty() {
        model.id.clone()
    } else {
        "model not selected".to_string()
    }
}

fn session_label(state: &AppState) -> String {
    if !state.side_panel_state.session_id.is_empty() {
        truncate(&state.side_panel_state.session_id, 12)
    } else {
        "local".to_string()
    }
}

fn rulebook_label(state: &AppState) -> String {
    if state.rulebook_switcher_state.selected_rulebooks.is_empty() {
        "vac.core".to_string()
    } else {
        let mut values = state
            .rulebook_switcher_state
            .selected_rulebooks
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        values.sort();
        truncate(&values.join(","), 28)
    }
}

fn approval_command(state: &AppState) -> String {
    state
        .dialog_approval_state
        .dialog_command
        .as_ref()
        .map(|tool| {
            let args = tool.function.arguments.trim();
            if args.is_empty() {
                tool.function.name.clone()
            } else {
                format!("{} {}", tool.function.name, truncate(args, 120))
            }
        })
        .or_else(|| state.dialog_approval_state.approval_bar.command_preview())
        .unwrap_or_else(|| "pending shell command".to_string())
}

fn classify_command(command: &str) -> &'static str {
    let lower = command.to_ascii_lowercase();
    if lower.contains("rm -rf")
        || lower.contains("delete")
        || lower.contains("drop ")
        || lower.contains("destroy")
    {
        "DESTRUCTIVE"
    } else if lower.contains("cargo") || lower.contains("bash") || lower.contains("sh ") {
        "EXECUTE_PROCESS"
    } else {
        "POLICY_REVIEW"
    }
}

fn read_registry_snapshot() -> RegistrySnapshot {
    let mut out = RegistrySnapshot::default();
    let Ok(text) = fs::read_to_string(STATUS_PATH) else {
        return out;
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return out;
    };
    out.valid_label = value["readiness"]["valid_percent"]
        .as_u64()
        .or_else(|| value["readiness"]["effective_percent"].as_u64())
        .map(|pct| format!("valid {pct}%"))
        .unwrap_or_else(|| out.valid_label.clone());
    out.conformance_level = value["enforcement_level"]
        .as_str()
        .or_else(|| value["workspace"]["enforcement_level"].as_str())
        .or_else(|| value["workspace"]["conformance_level"].as_str())
        .unwrap_or("L1")
        .to_string();
    out.compiled_current = value["control_plane"]["runtime"].as_str() == Some("compiled_json")
        || value["control_plane"]["compiled_snapshots_current"]
            .as_bool()
            .unwrap_or(false)
        || value["control_plane"]["compiled_snapshot_hash"]
            .as_str()
            .is_some();
    let runtime = read_json_file(Path::new(RUNTIME_JOBS_PATH));
    out.queued_jobs = runtime
        .as_ref()
        .and_then(|v| v["queue"]["queued"].as_u64())
        .or_else(|| {
            runtime
                .as_ref()
                .and_then(|v| v["summary"]["queued"].as_u64())
        })
        .unwrap_or(0);
    out.running_jobs = runtime
        .as_ref()
        .and_then(|v| v["queue"]["running"].as_u64())
        .or_else(|| {
            runtime
                .as_ref()
                .and_then(|v| v["summary"]["running"].as_u64())
        })
        .unwrap_or(0);
    out
}

fn read_runtime_registry(registry: &RegistrySnapshot) -> RuntimeRegistry {
    let mut out = RuntimeRegistry {
        queued: registry.queued_jobs,
        running: registry.running_jobs,
        ..Default::default()
    };
    let Ok(text) = fs::read_to_string(Path::new(RUNTIME_JOBS_PATH)) else {
        return out;
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return out;
    };
    out.autopilot_state = value["autopilot"]["state"]
        .as_str()
        .unwrap_or("not_running")
        .to_string();
    out.pid = value["autopilot"]["pid"]
        .as_i64()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".to_string());
    out.mode = value["autopilot"]["mode"]
        .as_str()
        .unwrap_or("monitor-only")
        .to_string();
    out.env = value["autopilot"]["env"]
        .as_str()
        .unwrap_or("host")
        .to_string();
    out.queued = value["queue"]["queued"].as_u64().unwrap_or(out.queued);
    out.running = value["queue"]["running"].as_u64().unwrap_or(out.running);
    out.empty_state = value["empty_state"]
        .as_str()
        .unwrap_or(&out.empty_state)
        .to_string();
    if let Some(rows) = value["jobs"].as_array() {
        out.rows = rows
            .iter()
            .map(|row| RuntimeJobRow {
                state: row["state"].as_str().unwrap_or("unknown").to_string(),
                kind: row["kind"].as_str().unwrap_or("unknown").to_string(),
                id: row["id"].as_str().unwrap_or("-").to_string(),
                trigger: row["trigger"]
                    .as_str()
                    .or_else(|| row["title"].as_str())
                    .unwrap_or("-")
                    .to_string(),
                age: row["age"]
                    .as_str()
                    .or_else(|| row["age_seconds"].as_u64().map(|_| "live"))
                    .unwrap_or("-")
                    .to_string(),
                next_run: row["next_run"].as_str().unwrap_or("-").to_string(),
            })
            .collect();
    }
    out
}

fn runtime_color(state: &str) -> ratatui::style::Color {
    match state {
        "running" => ThemeColors::success(),
        "not_running" | "down" | "failed" => ThemeColors::danger(),
        _ => ThemeColors::muted(),
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        value
            .chars()
            .take(max.saturating_sub(1))
            .collect::<String>()
            + "…"
    }
}

trait ApprovalBarPreview {
    fn command_preview(&self) -> Option<String>;
}

impl ApprovalBarPreview for crate::services::approval_bar::ApprovalBar {
    fn command_preview(&self) -> Option<String> {
        // ApprovalBar intentionally owns its formatting details. The operator
        // console falls back to the dialog tool call when available and uses a
        // neutral label otherwise, so no private internals are required here.
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destructive_classifier_catches_recursive_delete() {
        assert_eq!(
            classify_command("rm -rf target/debug/incremental"),
            "DESTRUCTIVE"
        );
    }

    #[test]
    fn truncation_never_exceeds_limit() {
        let out = truncate("abcdefghijklmnopqrstuvwxyz", 8);
        assert!(out.chars().count() <= 8);
    }
}
