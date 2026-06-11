use crate::app::AppState;
use crate::services::detect_term::ThemeColors;
use crate::services::message::MessageContent;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};
use serde::Deserialize;
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperatorMode {
    FirstLaunch,
    Idle,
    AgentWorking,
    RuntimeJobs,
    ApprovalRequired,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct RuntimeJobsFile {
    #[serde(default)]
    jobs: Vec<RuntimeJobRow>,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct RuntimeJobRow {
    #[serde(default)]
    state: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    trigger: String,
    #[serde(default)]
    age: Option<String>,
    #[serde(default)]
    next_run: Option<String>,
}

pub fn should_take_over(state: &AppState) -> bool {
    if std::env::var("VAC_TUI_CLASSIC").ok().as_deref() == Some("1") {
        return false;
    }
    std::env::var("VAC_TUI_ROUTE").is_ok()
        || state.messages_scrolling_state.messages.is_empty()
        || state.loading_state.is_loading
        || state.loading_state.loading_manager.is_loading()
        || state.tool_call_state.is_streaming
        || state.dialog_approval_state.is_dialog_open
        || state.dialog_approval_state.approval_bar.is_visible()
}

pub fn render(f: &mut Frame, state: &AppState) {
    let area = f.area();
    let root = find_workspace_root()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let mode = resolve_mode(state, &root);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(6),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);

    render_top_bar(f, chunks[0], mode);
    match mode {
        OperatorMode::FirstLaunch => render_first_launch(f, chunks[1], state, &root),
        OperatorMode::Idle => render_idle(f, chunks[1], state, &root),
        OperatorMode::AgentWorking => render_agent_working(f, chunks[1], state),
        OperatorMode::RuntimeJobs => render_runtime_jobs(f, chunks[1], state, &root),
        OperatorMode::ApprovalRequired => render_approval(f, chunks[1], state, &root),
    }
    render_prompt(f, chunks[2], mode);
    render_footer(f, chunks[3], state, mode, &root);
}

fn resolve_mode(state: &AppState, root: &Path) -> OperatorMode {
    if state.dialog_approval_state.is_dialog_open
        || state.dialog_approval_state.approval_bar.is_visible()
    {
        return OperatorMode::ApprovalRequired;
    }
    if let Ok(route) = std::env::var("VAC_TUI_ROUTE")
        && (route.trim_matches('/').eq_ignore_ascii_case("runtime")
            || route.eq_ignore_ascii_case("runtime jobs"))
    {
        return OperatorMode::RuntimeJobs;
    }
    if state.loading_state.is_loading
        || state.loading_state.loading_manager.is_loading()
        || state.tool_call_state.is_streaming
    {
        return OperatorMode::AgentWorking;
    }
    if !root.join(".vac/registry/status.json").is_file() {
        return OperatorMode::FirstLaunch;
    }
    OperatorMode::Idle
}

fn render_top_bar(f: &mut Frame, area: Rect, mode: OperatorMode) {
    let title = match mode {
        OperatorMode::FirstLaunch => "first launch",
        OperatorMode::Idle => "idle",
        OperatorMode::AgentWorking => "agent working",
        OperatorMode::RuntimeJobs => "runtime jobs",
        OperatorMode::ApprovalRequired => "destructive approval",
    };
    let lines = vec![
        Line::from(vec![
            Span::styled(" ● ", Style::default().fg(ThemeColors::red())),
            Span::styled("● ", Style::default().fg(ThemeColors::warning())),
            Span::styled("●  ", Style::default().fg(ThemeColors::green())),
            Span::styled(
                "vac",
                Style::default()
                    .fg(ThemeColors::text())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " • interactive — ",
                Style::default().fg(ThemeColors::muted()),
            ),
            Span::styled(
                title,
                Style::default()
                    .fg(ThemeColors::cyan())
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            tab("chat", mode != OperatorMode::RuntimeJobs),
            Span::raw(" · "),
            tab("runtime", mode == OperatorMode::RuntimeJobs),
            Span::raw(" · "),
            tab("review", false),
            Span::raw(" · "),
            tab("workbench", false),
            Span::raw(" · "),
            tab("mcp", false),
            Span::styled(
                "                                      tab 2/5",
                Style::default().fg(ThemeColors::muted()),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(lines), area);
}

fn tab(label: &'static str, active: bool) -> Span<'static> {
    if active {
        Span::styled(
            format!(" {label} "),
            Style::default()
                .fg(ThemeColors::black())
                .bg(ThemeColors::cyan())
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            format!(" {label} "),
            Style::default().fg(ThemeColors::muted()),
        )
    }
}

fn render_first_launch(f: &mut Frame, area: Rect, state: &AppState, root: &Path) {
    let status = load_status(root);
    let file_count = status
        .as_ref()
        .and_then(|v| v.pointer("/workspace/file_count"))
        .and_then(Value::as_u64);
    let crate_count = status
        .as_ref()
        .and_then(|v| v.pointer("/workspace/rust_crate_count"))
        .and_then(Value::as_u64);
    let provider_name = empty_as_unknown(state.configuration_state.model.provider_name());
    let model_name = empty_as_unknown(state.configuration_state.model.display_name());
    let file_count_label = file_count
        .map(|n| n.to_string())
        .unwrap_or_else(|| "not indexed".into());
    let crate_count_label = crate_count
        .map(|n| n.to_string())
        .unwrap_or_else(|| "not indexed".into());
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  VAC",
                Style::default()
                    .fg(ThemeColors::cyan())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "  Vastar Agentic CLI v{}",
                env!("CARGO_PKG_VERSION")
            )),
        ]),
        Line::from(vec![
            Span::styled("  control plane", Style::default().fg(ThemeColors::muted())),
            Span::raw(" assessment-grounded bootstrap"),
        ]),
        Line::from(vec![
            Span::styled("  cwd", Style::default().fg(ThemeColors::muted())),
            Span::raw(format!(" {}", root.display())),
        ]),
        Line::from(""),
        Line::from("  hydrating startup snapshot"),
        Line::from("  ─────────────────────────────────────────────────────────────────────"),
        line_pair("version", env!("CARGO_PKG_VERSION"), "runtime", "local"),
        line_pair("provider", &provider_name, "isolation", "L1 advisory"),
        line_pair(
            "model",
            &model_name,
            "control plane",
            if root.join(".vac").is_dir() {
                "present"
            } else {
                "missing"
            },
        ),
        line_pair(
            "files indexed",
            &file_count_label,
            "rust crates",
            &crate_count_label,
        ),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ready", Style::default().fg(ThemeColors::cyan())),
            Span::raw("  type / for commands, /help to explore, or start typing a task."),
        ]),
    ];
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn render_idle(f: &mut Frame, area: Rect, _state: &AppState, root: &Path) {
    let status = load_status(root);
    let readiness = status
        .as_ref()
        .and_then(|v| v.pointer("/readiness/effective_percent"))
        .and_then(Value::as_u64);
    let control_plane = if root.join(".vac/registry/compiled").is_dir() {
        "compiled snapshot available"
    } else {
        "compiled snapshot missing"
    };
    let recent = load_recent_sessions(root);
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  VAC",
                Style::default()
                    .fg(ThemeColors::black())
                    .bg(ThemeColors::cyan())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  ready  "),
            Span::styled(control_plane, Style::default().fg(ThemeColors::muted())),
        ]),
        Line::from(""),
        Line::from("  The agent will stream thinking, tools, validation, and approvals here."),
        Line::from(vec![
            Span::styled(
                "  Keyboard-only: ",
                Style::default().fg(ThemeColors::muted()),
            ),
            key("tab"),
            Span::raw(" focus  "),
            key("/"),
            Span::raw(" commands  "),
            key("@"),
            Span::raw(" files  "),
            key("shift+tab"),
            Span::raw(" plan mode"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  readiness", Style::default().fg(ThemeColors::muted())),
            Span::raw(format!(
                " effective {}",
                readiness
                    .map(|v| format!("{v}%"))
                    .unwrap_or_else(|| "unknown".into())
            )),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  recent tasks",
            Style::default().fg(ThemeColors::muted()),
        )),
        Line::from("  ─────────────────────────────────────────────────────────"),
    ];
    if recent.is_empty() {
        lines.push(Line::from(Span::styled(
            "  no completed VAC sessions yet",
            Style::default().fg(ThemeColors::muted()),
        )));
    } else {
        for item in recent.into_iter().take(5) {
            lines.push(Line::from(format!("  ▸ {item}")));
        }
    }
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn render_agent_working(f: &mut Frame, area: Rect, state: &AppState) {
    let user = state
        .messages_scrolling_state
        .messages
        .iter()
        .rev()
        .find_map(|m| match &m.content {
            MessageContent::UserMessage(text) => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "task accepted".to_string());
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  you",
                Style::default()
                    .fg(ThemeColors::cyan())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  active task", Style::default().fg(ThemeColors::muted())),
        ]),
        Line::from(format!("  {}", truncate(&user, 100))),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  vac",
                Style::default()
                    .fg(ThemeColors::cyan())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  working", Style::default().fg(ThemeColors::muted())),
        ]),
        Line::from(
            "  bounded execution is active; tool timeline below is derived from live TUI state.",
        ),
        Line::from(""),
        Line::from("  ── tool timeline ─────────────────────────────────"),
    ];

    let mut rows = Vec::new();
    if let Some(tool) = &state.tool_call_state.latest_tool_call {
        rows.push(format!(
            "● {}  {}",
            tool.function.name,
            truncate(&tool.function.arguments, 56)
        ));
    }
    for (id, value) in state.tool_call_state.streaming_tool_results.iter().take(4) {
        rows.push(format!("● stream {id}  {}", truncate(value, 56)));
    }
    if rows.is_empty() {
        rows.push("○ no tool call has started yet".to_string());
    }
    for row in rows.into_iter().take(5) {
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(row, Style::default().fg(ThemeColors::green())),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  ~ thinking", Style::default().fg(ThemeColors::magenta())),
        Span::styled(
            " · esc to interrupt",
            Style::default().fg(ThemeColors::muted()),
        ),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  context ", Style::default().fg(ThemeColors::muted())),
        Span::raw(format_tokens(
            state
                .usage_tracking_state
                .current_message_usage
                .prompt_tokens as u64,
        )),
        Span::styled(" / ", Style::default().fg(ThemeColors::muted())),
        Span::raw(format_tokens(state.configuration_state.model.limit.context)),
    ]));
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn render_runtime_jobs(f: &mut Frame, area: Rect, _state: &AppState, root: &Path) {
    let jobs = load_runtime_jobs(root);
    let status_line = format!(
        "autopilot {}   queue {}   running {}",
        if jobs.jobs.iter().any(|j| j.state == "running") {
            "running"
        } else {
            "idle"
        },
        jobs.jobs.len(),
        jobs.jobs.iter().filter(|j| j.state == "running").count()
    );
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(4),
        ])
        .split(area);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(status_line, Style::default().fg(ThemeColors::muted())),
            ]),
        ]),
        chunks[0],
    );
    let rows: Vec<Row> = if jobs.jobs.is_empty() {
        vec![Row::new(vec![
            Cell::from("idle"),
            Cell::from("none"),
            Cell::from("—"),
            Cell::from("no runtime jobs registered"),
            Cell::from("—"),
            Cell::from("—"),
        ])]
    } else {
        jobs.jobs
            .into_iter()
            .take(12)
            .map(|j| {
                Row::new(vec![
                    j.state,
                    j.kind,
                    j.id,
                    j.trigger,
                    j.age.unwrap_or_else(|| "—".into()),
                    j.next_run.unwrap_or_else(|| "—".into()),
                ])
            })
            .collect()
    };
    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Min(18),
            Constraint::Length(12),
            Constraint::Length(18),
        ],
    )
    .header(
        Row::new(vec!["state", "kind", "id", "trigger", "age", "next-run"])
            .style(Style::default().fg(ThemeColors::muted())),
    )
    .block(Block::default().borders(Borders::TOP));
    f.render_widget(table, chunks[1]);
    f.render_widget(
        Paragraph::new(vec![Line::from(vec![
            Span::styled("  inspect", Style::default().fg(ThemeColors::muted())),
            Span::raw(" runtime jobs are loaded from .vac/registry/runtime/jobs.json"),
        ])])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ThemeColors::dark_gray())),
        ),
        chunks[2],
    );
}

fn render_approval(f: &mut Frame, area: Rect, state: &AppState, root: &Path) {
    let command = state
        .dialog_approval_state
        .dialog_command
        .as_ref()
        .map(|t| format!("{} {}", t.function.name, t.function.arguments))
        .unwrap_or_else(|| "approval request pending".to_string());
    let policy = load_status(root)
        .and_then(|v| v.pointer("/policy/default_decision").cloned())
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "policy".into());
    let lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  approval required",
            Style::default()
                .fg(ThemeColors::warning())
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(
            "  ┌─────────────────────────────────────────────────────────────────────────────┐",
        ),
        Line::from(vec![
            Span::raw("  │ "),
            Span::styled(
                "BASH",
                Style::default()
                    .fg(ThemeColors::black())
                    .bg(ThemeColors::warning())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  the agent wants to run a shell/tool command"),
        ]),
        Line::from("  │"),
        Line::from(vec![
            Span::raw("  │  $ "),
            Span::styled(
                truncate(&command, 86),
                Style::default().fg(ThemeColors::cyan()),
            ),
        ]),
        Line::from(vec![
            Span::raw("  │  cwd  "),
            Span::styled(
                root.display().to_string(),
                Style::default().fg(ThemeColors::muted()),
            ),
        ]),
        Line::from(vec![
            Span::raw("  │  policy  "),
            Span::styled(policy, Style::default().fg(ThemeColors::warning())),
        ]),
        Line::from("  │"),
        Line::from(vec![
            Span::raw("  │  "),
            key("y"),
            Span::raw(" approve once    "),
            key("a"),
            Span::raw(" approve+remember    "),
            key("n"),
            Span::raw(" reject    "),
            key("r"),
            Span::raw(" reject with reason"),
        ]),
        Line::from(
            "  └─────────────────────────────────────────────────────────────────────────────┘",
        ),
    ];
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn render_prompt(f: &mut Frame, area: Rect, mode: OperatorMode) {
    let hint = match mode {
        OperatorMode::ApprovalRequired => "approval blocking — y · a · n · r  (esc to defer)",
        OperatorMode::AgentWorking => "agent working — next message queues (esc to interrupt)",
        OperatorMode::RuntimeJobs => "c cancel · r retry · i open in editor · enter attach",
        _ => "/ commands   @ files   shift+tab plan   ctrl+j newline",
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ThemeColors::dark_gray()));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(
        Paragraph::new(vec![Line::from(vec![
            Span::styled("▸ ", Style::default().fg(ThemeColors::cyan())),
            Span::styled(hint, Style::default().fg(ThemeColors::muted())),
        ])]),
        inner,
    );
}

fn render_footer(f: &mut Frame, area: Rect, state: &AppState, mode: OperatorMode, root: &Path) {
    let mode_label = match mode {
        OperatorMode::RuntimeJobs => "WORKBENCH",
        OperatorMode::ApprovalRequired => "CONVERSATION",
        _ => "INPUT",
    };
    let status = load_status(root);
    let readiness = status
        .as_ref()
        .and_then(|v| v.pointer("/readiness/effective_percent"))
        .and_then(Value::as_u64)
        .map(|v| format!("valid {v}%"))
        .unwrap_or_else(|| "valid unknown".into());
    let profile = &state.profile_switcher_state.current_profile_name;
    let rulebook = status
        .as_ref()
        .and_then(|v| v.pointer("/control_plane/rulebook"))
        .and_then(Value::as_str)
        .unwrap_or("vac.core");
    let model = empty_as_unknown(state.configuration_state.model.display_name());
    let provider = empty_as_unknown(state.configuration_state.model.provider_name());
    let tokens = format_tokens(state.usage_tracking_state.total_session_usage.total_tokens as u64);
    let line = Line::from(vec![
        Span::styled(
            format!(" {mode_label} "),
            Style::default()
                .fg(ThemeColors::black())
                .bg(ThemeColors::cyan())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" | model ", Style::default().fg(ThemeColors::muted())),
        Span::styled(
            model,
            Style::default()
                .fg(ThemeColors::cyan())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" ({provider})"),
            Style::default().fg(ThemeColors::muted()),
        ),
        Span::styled(" | ", Style::default().fg(ThemeColors::muted())),
        Span::raw(tokens),
        Span::styled(" tok", Style::default().fg(ThemeColors::muted())),
        Span::styled(" | manual ", Style::default().fg(ThemeColors::green())),
        Span::styled("                 ", Style::default()),
        Span::styled(readiness, Style::default().fg(ThemeColors::green())),
        Span::styled(
            format!("  profile {profile} · rulebook {rulebook}"),
            Style::default().fg(ThemeColors::muted()),
        ),
    ]);
    f.render_widget(Paragraph::new(line).alignment(Alignment::Left), area);
}

fn line_pair<'a>(a: &'a str, av: &'a str, b: &'a str, bv: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("  {a:<12}"),
            Style::default().fg(ThemeColors::muted()),
        ),
        Span::raw(format!(" {av:<30}")),
        Span::styled(
            format!(" {b:<14}"),
            Style::default().fg(ThemeColors::muted()),
        ),
        Span::raw(format!(" {bv}")),
    ])
}

fn key(label: &'static str) -> Span<'static> {
    Span::styled(
        format!(" {label} "),
        Style::default()
            .fg(ThemeColors::text())
            .bg(ThemeColors::dark_gray())
            .add_modifier(Modifier::BOLD),
    )
}

fn load_status(root: &Path) -> Option<Value> {
    let path = root.join(".vac/registry/status.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

fn load_runtime_jobs(root: &Path) -> RuntimeJobsFile {
    let path = root.join(".vac/registry/runtime/jobs.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn load_recent_sessions(root: &Path) -> Vec<String> {
    let path = root.join(".vac/registry/sessions/index.json");
    let Some(v) = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
    else {
        return Vec::new();
    };
    v.get("sessions")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.get("title").and_then(Value::as_str).map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn find_workspace_root() -> Option<PathBuf> {
    let mut cur = std::env::current_dir().ok()?;
    loop {
        if cur.join(".vac").is_dir() || cur.join("vac-rs/Cargo.toml").is_file() {
            return Some(cur);
        }
        if !cur.pop() {
            return None;
        }
    }
}

fn empty_as_unknown(s: &str) -> String {
    if s.trim().is_empty() {
        "unknown".into()
    } else {
        s.to_string()
    }
}
fn truncate(s: &str, max: usize) -> String {
    let flat = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max {
        flat
    } else {
        flat.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
    }
}
fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
