//! VAC operator console renderer.
//!
//! This module renders the v1.9 VAC-native TUI surfaces from runtime state and
//! `.vac/registry` snapshots. It intentionally does not hardcode provider/model
//! names, legacy product labels, or sample runtime jobs from design mockups.

use crate::app::{AppState, VacConsoleRoute};
use crate::services::detect_term::ThemeColors;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Gauge, Paragraph, Row, Table, Wrap},
};
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
struct WorkspaceSnapshot {
    status: Value,
    capabilities: Vec<Value>,
    runtime_jobs: Vec<Value>,
    runtime_summary: Value,
    status_loaded: bool,
    capabilities_loaded: bool,
    runtime_loaded: bool,
}

impl WorkspaceSnapshot {
    fn load() -> Self {
        let status = read_json(Path::new(".vac/registry/status.json"));
        let capabilities_json = read_json(Path::new(".vac/registry/compiled/capabilities.json"));
        let runtime_json = read_json(Path::new(".vac/registry/runtime/jobs.json"));

        let capabilities = capabilities_json
            .as_ref()
            .and_then(|v| v.get("capabilities"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_else(Vec::new);

        let runtime_jobs = runtime_json
            .as_ref()
            .and_then(|v| v.get("jobs"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_else(Vec::new);

        let runtime_summary = runtime_json
            .as_ref()
            .and_then(|v| v.get("summary"))
            .cloned()
            .unwrap_or(Value::Null);

        Self {
            status_loaded: status.is_some(),
            capabilities_loaded: capabilities_json.is_some(),
            runtime_loaded: runtime_json.is_some(),
            status: status.unwrap_or(Value::Null),
            capabilities,
            runtime_jobs,
            runtime_summary,
        }
    }

    fn readiness_percent(&self) -> u64 {
        self.status
            .pointer("/readiness/valid_percent")
            .and_then(Value::as_u64)
            .or_else(|| {
                let total = self.capabilities.len() as u64;
                let ready = self
                    .capabilities
                    .iter()
                    .filter(|c| json_str(c, &["readiness", "effective"]) == "ready")
                    .count() as u64;
                ready
                    .checked_mul(100)
                    .and_then(|value| value.checked_div(total))
            })
            .unwrap_or(0)
    }

    fn effective_capability_counts(&self) -> (usize, usize, usize, usize) {
        let mut ready = 0;
        let mut partial = 0;
        let mut planned = 0;
        let mut deprecated = 0;
        for cap in &self.capabilities {
            match json_str(cap, &["readiness", "effective"]).as_str() {
                "ready" => ready += 1,
                "partial" => partial += 1,
                "planned" => planned += 1,
                "deprecated" => deprecated += 1,
                _ => partial += 1,
            }
        }
        (ready, partial, planned, deprecated)
    }
}

fn read_json(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
}

fn json_str(value: &Value, path: &[&str]) -> String {
    let mut cursor = value;
    for key in path {
        cursor = match cursor.get(*key) {
            Some(v) => v,
            None => return String::new(),
        };
    }
    cursor
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| cursor.as_u64().map(|v| v.to_string()))
        .or_else(|| cursor.as_i64().map(|v| v.to_string()))
        .or_else(|| cursor.as_bool().map(|v| v.to_string()))
        .unwrap_or_default()
}

fn active_model_display(state: &AppState) -> String {
    let model = state
        .model_switcher_state
        .current_model
        .as_ref()
        .unwrap_or(&state.configuration_state.model);

    let provider = model.provider_name();
    let name = model.display_name();
    if provider.is_empty() {
        name.to_string()
    } else if name.is_empty() {
        provider.to_string()
    } else {
        format!("{provider}/{name}")
    }
}

fn active_provider_display(state: &AppState) -> String {
    let model = state
        .model_switcher_state
        .current_model
        .as_ref()
        .unwrap_or(&state.configuration_state.model);
    if model.provider_name().is_empty() {
        "provider unset".to_string()
    } else {
        model.provider_name().to_string()
    }
}

fn usage_text(state: &AppState) -> String {
    let model = state
        .model_switcher_state
        .current_model
        .as_ref()
        .unwrap_or(&state.configuration_state.model);
    let tokens = state
        .usage_tracking_state
        .current_message_usage
        .prompt_tokens;
    let max = model.limit.context;
    if tokens == 0 || max == 0 {
        format!(
            "{} tok",
            state.usage_tracking_state.total_session_usage.total_tokens
        )
    } else {
        format!("{} / {} tok", tokens, max)
    }
}

fn route_title(route: VacConsoleRoute) -> &'static str {
    match route {
        VacConsoleRoute::Conversation => "conversation",
        VacConsoleRoute::Runtime => "runtime jobs",
        VacConsoleRoute::Review => "review",
        VacConsoleRoute::Workbench => "workbench",
        VacConsoleRoute::Mcp => "mcp",
        VacConsoleRoute::Capabilities => "capabilities",
        VacConsoleRoute::Assessment => "assessment",
        VacConsoleRoute::SpecSync => "spec-sync",
    }
}

fn render_header(
    f: &mut Frame,
    state: &AppState,
    area: Rect,
    route: VacConsoleRoute,
    snapshot: &WorkspaceSnapshot,
) {
    let status = if snapshot.status_loaded {
        "ready"
    } else {
        "bootstrap"
    };
    let title = Line::from(vec![
        Span::styled(
            "vac",
            Style::default()
                .fg(ThemeColors::cyan())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " · interactive — ",
            Style::default().fg(ThemeColors::dark_gray()),
        ),
        Span::styled(
            route_title(route),
            Style::default().fg(ThemeColors::title_primary()),
        ),
    ]);
    f.render_widget(Paragraph::new(title), area);

    if area.height > 1 {
        let tabs = [
            (VacConsoleRoute::Conversation, "chat"),
            (VacConsoleRoute::Runtime, "runtime"),
            (VacConsoleRoute::Review, "review"),
            (VacConsoleRoute::Workbench, "workbench"),
            (VacConsoleRoute::Mcp, "mcp"),
        ];
        let mut spans = Vec::new();
        for (tab_route, label) in tabs {
            let style = if tab_route == route {
                Style::default()
                    .fg(ThemeColors::background())
                    .bg(ThemeColors::cyan())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(ThemeColors::dark_gray())
            };
            spans.push(Span::styled(format!(" {label} "), style));
            spans.push(Span::raw(" · "));
        }
        spans.push(Span::styled(
            status.to_string(),
            Style::default().fg(ThemeColors::green()),
        ));
        spans.push(Span::styled(
            " · model ",
            Style::default().fg(ThemeColors::dark_gray()),
        ));
        spans.push(Span::styled(
            active_model_display(state),
            Style::default().fg(ThemeColors::cyan()),
        ));
        f.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect {
                y: area.y + 1,
                height: 1,
                ..area
            },
        );
    }
}

pub fn render_footer(f: &mut Frame, state: &AppState, area: Rect) {
    if area.height == 0 {
        return;
    }
    let snapshot = WorkspaceSnapshot::load();
    let mut spans = Vec::new();
    spans.push(Span::styled(
        " VAC ",
        Style::default()
            .fg(ThemeColors::background())
            .bg(ThemeColors::cyan())
            .add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled(
        " | model ",
        Style::default().fg(ThemeColors::dark_gray()),
    ));
    spans.push(Span::styled(
        active_model_display(state),
        Style::default().fg(ThemeColors::cyan()),
    ));
    spans.push(Span::styled(
        " | ",
        Style::default().fg(ThemeColors::dark_gray()),
    ));
    spans.push(Span::styled(
        usage_text(state),
        Style::default().fg(ThemeColors::text()),
    ));
    spans.push(Span::styled(
        " | manual ",
        Style::default().fg(ThemeColors::green()),
    ));
    spans.push(Span::styled(
        format!(" | valid {}% ", snapshot.readiness_percent()),
        Style::default().fg(ThemeColors::green()),
    ));
    spans.push(Span::styled(
        "profile ",
        Style::default().fg(ThemeColors::dark_gray()),
    ));
    spans.push(Span::styled(
        state.profile_switcher_state.current_profile_name.clone(),
        Style::default().fg(ThemeColors::text()),
    ));
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

pub fn render_main(f: &mut Frame, state: &AppState, area: Rect) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let snapshot = WorkspaceSnapshot::load();
    let route = state.vac_console_state.route;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(area);

    render_header(f, state, chunks[0], route, &snapshot);

    match route {
        VacConsoleRoute::Runtime => render_runtime_jobs(f, state, chunks[2], &snapshot),
        VacConsoleRoute::Capabilities => render_capabilities(f, chunks[2], &snapshot),
        VacConsoleRoute::Assessment => render_assessment(f, chunks[2], &snapshot),
        VacConsoleRoute::SpecSync => render_spec_sync(f, chunks[2]),
        VacConsoleRoute::Conversation
        | VacConsoleRoute::Review
        | VacConsoleRoute::Workbench
        | VacConsoleRoute::Mcp => {
            if state.dialog_approval_state.is_dialog_open
                || state.dialog_approval_state.approval_bar.is_visible()
            {
                render_approval_required(f, state, chunks[2]);
            } else if state.loading_state.is_loading || state.tool_call_state.is_streaming {
                render_agent_working(f, state, chunks[2]);
            } else if !state.messages_scrolling_state.has_user_messages {
                render_idle(f, state, chunks[2], &snapshot);
            }
        }
    }
}

fn render_idle(f: &mut Frame, state: &AppState, area: Rect, snapshot: &WorkspaceSnapshot) {
    let version = env!("CARGO_PKG_VERSION");
    let cwd = std::env::current_dir()
        .ok()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "cwd unavailable".to_string());
    let status = if snapshot.status_loaded {
        "ready"
    } else {
        "first launch"
    };
    let control_plane = if snapshot.status_loaded {
        "compiled registry loaded"
    } else {
        ".vac registry not compiled yet"
    };
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  VAC-native ",
                Style::default()
                    .fg(ThemeColors::background())
                    .bg(ThemeColors::cyan())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {status}"),
                Style::default()
                    .fg(ThemeColors::title_primary())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ·  ", Style::default().fg(ThemeColors::dark_gray())),
            Span::styled(control_plane, Style::default().fg(ThemeColors::dark_gray())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  version ", Style::default().fg(ThemeColors::dark_gray())),
            Span::styled(version, Style::default().fg(ThemeColors::text())),
            Span::styled(
                "  ·  provider ",
                Style::default().fg(ThemeColors::dark_gray()),
            ),
            Span::styled(
                active_provider_display(state),
                Style::default().fg(ThemeColors::green()),
            ),
        ]),
        Line::from(vec![
            Span::styled("  model   ", Style::default().fg(ThemeColors::dark_gray())),
            Span::styled(
                active_model_display(state),
                Style::default().fg(ThemeColors::text()),
            ),
        ]),
        Line::from(vec![
            Span::styled("  cwd     ", Style::default().fg(ThemeColors::dark_gray())),
            Span::styled(cwd, Style::default().fg(ThemeColors::text())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Keyboard-only: ",
                Style::default().fg(ThemeColors::dark_gray()),
            ),
            Span::styled(
                "tab",
                Style::default()
                    .fg(ThemeColors::text())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " focus   / commands   @ files   shift+tab plan mode",
                Style::default().fg(ThemeColors::dark_gray()),
            ),
        ]),
    ];

    let (ready, partial, planned, deprecated) = snapshot.effective_capability_counts();
    if snapshot.capabilities_loaded {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "  capabilities ",
                Style::default().fg(ThemeColors::dark_gray()),
            ),
            Span::styled(
                format!(
                    "ready {ready}  partial {partial}  planned {planned}  deprecated {deprecated}"
                ),
                Style::default().fg(ThemeColors::green()),
            ),
        ]));
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn render_agent_working(f: &mut Frame, state: &AppState, area: Rect) {
    let token_total = state.usage_tracking_state.total_session_usage.total_tokens;
    let recent_tools: Vec<String> = state
        .session_tool_calls_state
        .last_message_tool_calls
        .iter()
        .rev()
        .take(5)
        .map(|tool| tool.function.name.clone())
        .collect();

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  vac ",
                Style::default()
                    .fg(ThemeColors::cyan())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "working",
                Style::default()
                    .fg(ThemeColors::text())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" · model ", Style::default().fg(ThemeColors::dark_gray())),
            Span::styled(
                active_model_display(state),
                Style::default().fg(ThemeColors::cyan()),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  context ", Style::default().fg(ThemeColors::dark_gray())),
            Span::styled(
                format!("{}", token_total),
                Style::default().fg(ThemeColors::text()),
            ),
            Span::styled(
                " tok  ·  esc to interrupt",
                Style::default().fg(ThemeColors::dark_gray()),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  — tool timeline ─────────────────────────────────",
            Style::default().fg(ThemeColors::dark_gray()),
        )),
    ];

    if recent_tools.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  • ", Style::default().fg(ThemeColors::dark_gray())),
            Span::styled(
                "waiting for first tool event from runtime",
                Style::default().fg(ThemeColors::dark_gray()),
            ),
        ]));
    } else {
        for name in recent_tools {
            lines.push(Line::from(vec![
                Span::styled("  • ", Style::default().fg(ThemeColors::green())),
                Span::styled(name, Style::default().fg(ThemeColors::text())),
                Span::styled(
                    "  queued/streaming from current session",
                    Style::default().fg(ThemeColors::dark_gray()),
                ),
            ]));
        }
    }

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn render_approval_required(f: &mut Frame, state: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ThemeColors::warning()))
        .title(Span::styled(
            " approval required ",
            Style::default()
                .fg(ThemeColors::warning())
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    let command = state
        .dialog_approval_state
        .dialog_command
        .as_ref()
        .and_then(|tool| command_from_tool_args(&tool.function.arguments))
        .or_else(|| {
            state
                .dialog_approval_state
                .dialog_command
                .as_ref()
                .map(|t| t.function.name.clone())
        })
        .unwrap_or_else(|| "runtime action awaiting operator decision".to_string());

    let lines = vec![
        Line::from(vec![
            Span::styled(
                "  BASH ",
                Style::default()
                    .fg(ThemeColors::background())
                    .bg(ThemeColors::warning())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  the agent wants to run a shell command",
                Style::default().fg(ThemeColors::text()),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            format!("  $ {command}"),
            Style::default()
                .fg(ThemeColors::cyan())
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  risk ", Style::default().fg(ThemeColors::dark_gray())),
            Span::styled(
                "DESTRUCTIVE/POLICY-CONTROLLED",
                Style::default()
                    .fg(ThemeColors::warning())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " · explicit approval required",
                Style::default().fg(ThemeColors::dark_gray()),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "   y ",
                Style::default()
                    .fg(ThemeColors::background())
                    .bg(ThemeColors::green())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" approve once   ", Style::default().fg(ThemeColors::text())),
            Span::styled(
                " a ",
                Style::default()
                    .fg(ThemeColors::background())
                    .bg(ThemeColors::green())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " approve+remember   ",
                Style::default().fg(ThemeColors::text()),
            ),
            Span::styled(
                " n ",
                Style::default()
                    .fg(ThemeColors::background())
                    .bg(ThemeColors::red())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" reject   ", Style::default().fg(ThemeColors::text())),
            Span::styled(
                " r ",
                Style::default()
                    .fg(ThemeColors::background())
                    .bg(ThemeColors::red())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " reject with reason",
                Style::default().fg(ThemeColors::text()),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn command_from_tool_args(args: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(args).ok()?;
    for key in ["command", "cmd", "shell_command", "script"] {
        if let Some(s) = value.get(key).and_then(Value::as_str) {
            return Some(s.to_string());
        }
    }
    None
}

fn render_runtime_jobs(f: &mut Frame, state: &AppState, area: Rect, snapshot: &WorkspaceSnapshot) {
    let running = snapshot
        .runtime_summary
        .get("running")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let queued = snapshot
        .runtime_summary
        .get("queued")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let mode = snapshot
        .runtime_summary
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("monitor-only");

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(4),
        ])
        .split(area);

    let top = Line::from(vec![
        Span::styled(" autopilot ", Style::default().fg(ThemeColors::dark_gray())),
        Span::styled(
            if running > 0 { "running" } else { "idle" },
            Style::default().fg(if running > 0 {
                ThemeColors::green()
            } else {
                ThemeColors::dark_gray()
            }),
        ),
        Span::styled(
            format!("  mode {mode}  ·  queue {queued}  ·  running {running}"),
            Style::default().fg(ThemeColors::dark_gray()),
        ),
        Span::styled("  ·  model ", Style::default().fg(ThemeColors::dark_gray())),
        Span::styled(
            active_model_display(state),
            Style::default().fg(ThemeColors::cyan()),
        ),
    ]);
    f.render_widget(Paragraph::new(top), chunks[0]);

    if !snapshot.runtime_loaded || snapshot.runtime_jobs.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No runtime jobs registered in .vac/registry/runtime/jobs.json",
                Style::default().fg(ThemeColors::dark_gray()),
            )),
            Line::from(Span::styled(
                "  one-shot, cron, and filewatch rows will appear here after the runtime writes job records.",
                Style::default().fg(ThemeColors::dark_gray()),
            )),
        ]);
        f.render_widget(empty, chunks[1]);
        return;
    }

    let rows = snapshot.runtime_jobs.iter().map(|job| {
        Row::new(vec![
            Cell::from(json_str(job, &["state"])),
            Cell::from(json_str(job, &["kind"])),
            Cell::from(json_str(job, &["id"])),
            Cell::from(json_str(job, &["trigger"])),
            Cell::from(json_str(job, &["age"])),
            Cell::from(json_str(job, &["next_run"])),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(14),
            Constraint::Min(18),
            Constraint::Length(12),
            Constraint::Length(14),
        ],
    )
    .header(
        Row::new(vec!["state", "kind", "id", "trigger", "age", "next-run"])
            .style(Style::default().fg(ThemeColors::dark_gray())),
    )
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(ThemeColors::dark_gray())),
    );
    f.render_widget(table, chunks[1]);

    let selected = snapshot.runtime_jobs.first();
    let inspect_id = selected.map(|j| json_str(j, &["id"])).unwrap_or_default();
    let progress = selected
        .and_then(|j| j.get("progress_percent").and_then(Value::as_u64))
        .unwrap_or(0)
        .min(100);
    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" inspect {inspect_id} ")),
        )
        .gauge_style(Style::default().fg(ThemeColors::cyan()))
        .percent(progress as u16)
        .label(format!("{}%", progress));
    f.render_widget(gauge, chunks[2]);
}

fn render_capabilities(f: &mut Frame, area: Rect, snapshot: &WorkspaceSnapshot) {
    if snapshot.capabilities.is_empty() {
        f.render_widget(Paragraph::new("No compiled capabilities found. Run `vac compile registry .` after authoring .vac/capabilities/*.yaml.").wrap(Wrap { trim: false }), area);
        return;
    }
    let rows = snapshot
        .capabilities
        .iter()
        .take(area.height.saturating_sub(3) as usize)
        .map(|cap| {
            Row::new(vec![
                Cell::from(json_str(cap, &["id"])),
                Cell::from(json_str(cap, &["readiness", "declared"])),
                Cell::from(json_str(cap, &["readiness", "computed"])),
                Cell::from(json_str(cap, &["readiness", "effective"])),
                Cell::from(json_str(cap, &["owner", "crate"])),
            ])
        });
    let table = Table::new(
        rows,
        [
            Constraint::Min(24),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Min(14),
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
        .style(Style::default().fg(ThemeColors::dark_gray())),
    )
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(ThemeColors::dark_gray())),
    );
    f.render_widget(table, area);
}

fn render_assessment(f: &mut Frame, area: Rect, _snapshot: &WorkspaceSnapshot) {
    let report = read_json(Path::new(".vac/assessment/gap_report.json"));
    let findings = report
        .as_ref()
        .and_then(|v| v.get("findings"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if findings.is_empty() {
        f.render_widget(Paragraph::new("No assessment findings recorded. The assessment surface will populate from .vac/assessment/gap_report.json.").wrap(Wrap { trim: false }), area);
        return;
    }
    let rows = findings
        .iter()
        .take(area.height.saturating_sub(3) as usize)
        .map(|finding| {
            Row::new(vec![
                Cell::from(json_str(finding, &["finding_id"])),
                Cell::from(json_str(finding, &["category"])),
                Cell::from(json_str(finding, &["severity"])),
                Cell::from(json_str(finding, &["recommendation"])),
            ])
        });
    let table = Table::new(
        rows,
        [
            Constraint::Length(24),
            Constraint::Length(24),
            Constraint::Length(8),
            Constraint::Min(24),
        ],
    )
    .header(
        Row::new(vec!["finding", "category", "severity", "recommendation"])
            .style(Style::default().fg(ThemeColors::dark_gray())),
    )
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(ThemeColors::dark_gray())),
    );
    f.render_widget(table, area);
}

fn render_spec_sync(f: &mut Frame, area: Rect) {
    let report = read_json(Path::new(".vac/registry/spec-sync/current.json"));
    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        " SpecSync",
        Style::default()
            .fg(ThemeColors::cyan())
            .add_modifier(Modifier::BOLD),
    )));
    if let Some(report) = report {
        lines.push(Line::from(format!(
            " trigger: {}",
            json_str(&report, &["trigger", "reason"])
        )));
        let drift = report
            .get("drift")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if drift.is_empty() {
            lines.push(Line::from(Span::styled(
                " no unresolved drift",
                Style::default().fg(ThemeColors::green()),
            )));
        } else {
            for item in drift.iter().take(area.height.saturating_sub(3) as usize) {
                lines.push(Line::from(format!(
                    " {} · {} · {}",
                    json_str(item, &["severity"]),
                    json_str(item, &["capability"]),
                    json_str(item, &["type"])
                )));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            " .vac/registry/spec-sync/current.json not present yet",
            Style::default().fg(ThemeColors::dark_gray()),
        )));
    }
    f.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false }),
        area,
    );
}
