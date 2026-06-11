use crate::app::AppState;
use crate::constants::{EXCEEDED_API_LIMIT_ERROR, EXCEEDED_API_LIMIT_ERROR_MESSAGE};
use crate::services::detect_term::ThemeColors;
use crate::services::message::{Message, MessageContent, invalidate_message_lines_cache};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use uuid::Uuid;

pub fn get_vac_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Generate a mouse capture hint message based on the terminal type
pub fn mouse_capture_hint_message(state: &crate::app::AppState) -> Message {
    let status = if state.terminal_ui_state.mouse_capture_enabled {
        "enabled"
    } else {
        "disabled"
    };
    let status_color = if state.terminal_ui_state.mouse_capture_enabled {
        ThemeColors::success()
    } else {
        ThemeColors::danger()
    };

    let styled_line = Line::from(vec![
        Span::styled(
            "█",
            Style::default()
                .fg(ThemeColors::magenta())
                .bg(ThemeColors::magenta()),
        ),
        Span::raw("  Mouse capture "),
        Span::styled(status, Style::default().fg(status_color)),
        Span::styled(
            " • Ctrl+L to toggle",
            Style::default().fg(ThemeColors::magenta()),
        ),
    ]);

    Message::styled(styled_line)
}

pub fn push_status_message(state: &mut AppState) {
    let model = state.configuration_state.model.model_id().to_string();
    let provider = state.configuration_state.model.provider_name().to_string();
    let tokens = state.usage_tracking_state.total_session_usage.total_tokens;
    let profile = state.profile_switcher_state.current_profile_name.clone();
    let text = format!(
        "VAC status: model {model} · provider {provider} · tokens {tokens} · profile {profile}"
    );
    push_styled_message(state, &text, ThemeColors::text(), "• ", ThemeColors::cyan());
}

pub fn push_usage_message(state: &mut AppState) {
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};

    let usage = &state.usage_tracking_state.total_session_usage;
    let mut lines = Vec::new();
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Session Usage",
        Style::default()
            .fg(ThemeColors::cyan())
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    if usage.total_tokens == 0 {
        lines.push(Line::from(vec![Span::styled(
            " No tokens used yet in this session.",
            Style::default().fg(ThemeColors::dark_gray()),
        )]));
    } else {
        // Manually format each line with fixed spacing to align all numbers (no colons)
        let formatted_prompt = format_number_with_separator(usage.prompt_tokens);
        lines.push(Line::from(vec![
            Span::raw(" Prompt tokens"),
            Span::raw("      "), // 6 spaces to align numbers
            Span::styled(
                formatted_prompt,
                Style::default()
                    .fg(ThemeColors::warning())
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        // Show prompt token details if available
        if let Some(details) = &usage.prompt_tokens_details {
            // Always show all fields except output_tokens (redundant with Completion tokens), using 0 if None, with fixed spacing
            let input_tokens = format_number_with_separator(details.input_tokens.unwrap_or(0));
            let cache_write =
                format_number_with_separator(details.cache_write_input_tokens.unwrap_or(0));
            let cache_read =
                format_number_with_separator(details.cache_read_input_tokens.unwrap_or(0));

            lines.push(Line::from(vec![
                Span::raw("  ├─ Input tokens"),
                Span::raw("   "), // 3 spaces to align numbers
                Span::styled(input_tokens, Style::default().fg(ThemeColors::dark_gray())),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  ├─ Cache write"),
                Span::raw("    "), // 4 spaces to align numbers
                Span::styled(cache_write, Style::default().fg(ThemeColors::dark_gray())),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  └─ Cache read"),
                Span::raw("     "), // 5 spaces to align numbers
                Span::styled(cache_read, Style::default().fg(ThemeColors::dark_gray())),
            ]));
        }

        let formatted_completion = format_number_with_separator(usage.completion_tokens);
        lines.push(Line::from(vec![
            Span::raw(" Completion tokens"),
            Span::raw("  "), // 2 spaces to align numbers
            Span::styled(
                formatted_completion,
                Style::default()
                    .fg(ThemeColors::warning())
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        let formatted_total = format_number_with_separator(usage.total_tokens);
        lines.push(Line::from(vec![
            Span::raw(" Total tokens"),
            Span::raw("       "), // 7 spaces to align numbers
            Span::styled(
                formatted_total,
                Style::default()
                    .fg(ThemeColors::cyan())
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    state.messages_scrolling_state.messages.push(Message {
        id: uuid::Uuid::new_v4(),
        content: MessageContent::StyledBlock(lines),
        is_collapsed: None,
    });
    invalidate_message_lines_cache(state);
}

pub fn format_number_with_separator(n: u32) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (count, c) in s.chars().rev().enumerate() {
        if count > 0 && count % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

pub fn push_help_message(state: &mut AppState) {
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};
    let mut lines = Vec::new();
    lines.push(Line::from(""));
    // usage mode
    lines.push(Line::from(vec![Span::styled(
        "Usage Mode",
        Style::default()
            .fg(Color::Reset)
            .add_modifier(Modifier::BOLD),
    )]));

    let usage_modes = vec![
        ("REPL", "vac (interactive session)", Color::Reset),
        (
            "Non-interactive",
            "vac -p  \"prompt\" -c <checkpoint_id>",
            Color::Reset,
        ),
    ];
    for (mode, desc, color) in usage_modes {
        lines.push(Line::from(vec![
            Span::styled(
                "● ",
                Style::default()
                    .fg(ThemeColors::dark_gray())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(mode),
            Span::raw(" – "),
            Span::styled(
                desc,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("Run"),
        Span::styled(
            " vac --help ",
            Style::default()
                .fg(Color::Reset)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "to see all commands",
            Style::default().fg(ThemeColors::dark_gray()),
        ),
    ]));
    lines.push(Line::from(""));
    // Section header
    lines.push(Line::from(vec![Span::styled(
        "Available commands",
        Style::default()
            .fg(Color::Reset)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));
    // Slash-commands header
    lines.push(Line::from(vec![Span::styled(
        "Slash-commands",
        Style::default()
            .fg(ThemeColors::dark_gray())
            .add_modifier(Modifier::BOLD),
    )]));

    // Slash-commands list
    let commands = crate::app::AppState::get_helper_commands();

    for cmd in &commands {
        lines.push(Line::from(vec![
            Span::styled(
                cmd.command.clone(),
                Style::default().fg(ThemeColors::cyan()),
            ),
            Span::raw(" – "),
            Span::raw(cmd.description.clone()),
        ]));
    }
    lines.push(Line::from(""));

    // Keyboard shortcuts header
    lines.push(Line::from(vec![Span::styled(
        "Keyboard shortcuts",
        Style::default()
            .fg(ThemeColors::dark_gray())
            .add_modifier(Modifier::BOLD),
    )]));

    // Shortcuts list
    let shortcuts = vec![
        ("Enter", "send message", ThemeColors::warning()),
        (
            "Ctrl+J or Shift+Enter",
            "insert newline",
            ThemeColors::warning(),
        ),
        // ("Up/Down", "scroll prompt history", ThemeColors::warning()),
        ("Ctrl+C", "quit VAC", ThemeColors::warning()),
    ];
    for (key, desc, color) in shortcuts {
        lines.push(Line::from(vec![
            Span::styled(key, Style::default().fg(color)),
            Span::raw(" – "),
            Span::raw(desc),
        ]));
    }
    lines.push(Line::from(""));
    state.messages_scrolling_state.messages.push(Message {
        id: uuid::Uuid::new_v4(),
        content: MessageContent::StyledBlock(lines),
        is_collapsed: None,
    });
    invalidate_message_lines_cache(state);
}

pub fn render_system_message(state: &mut AppState, msg: &str) {
    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("🤖", Style::default()),
        Span::styled(
            " System",
            Style::default()
                .fg(ThemeColors::cyan())
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    let message = Line::from(vec![Span::raw(format!(
        "{pad} - {msg}",
        pad = " ".repeat(2)
    ))]);
    lines.push(message);
    lines.push(Line::from(vec![Span::raw(" ")]));

    state.messages_scrolling_state.messages.push(Message {
        id: Uuid::new_v4(),
        content: MessageContent::StyledBlock(lines),
        is_collapsed: None,
    });
    invalidate_message_lines_cache(state);
}

pub fn push_error_message(state: &mut AppState, error: &str, remove_flag: Option<bool>) {
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    let mut flag = "[Error] ".to_string();
    if let Some(remove_flag) = remove_flag
        && remove_flag
    {
        flag = " ".repeat(flag.len());
    }
    // split error by \n
    let error_parts: Vec<&str> = error.split('\n').collect();
    let mut lines = Vec::new();
    for (i, part) in error_parts.iter().enumerate() {
        if i == 0 {
            // First line gets the error flag
            lines.push(Line::from(vec![
                Span::styled(
                    flag.clone(),
                    Style::default()
                        .fg(ThemeColors::red())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(*part, Style::default().fg(ThemeColors::red())),
            ]));
        } else {
            // Subsequent lines are just the error text
            lines.push(Line::from(vec![Span::styled(
                *part,
                Style::default().fg(ThemeColors::red()),
            )]));
        }
    }
    lines.push(Line::from(""));
    let owned_lines: Vec<Line<'static>> = lines
        .into_iter()
        .map(|line| {
            let owned_spans: Vec<Span<'static>> = line
                .spans
                .into_iter()
                .map(|span| Span::styled(span.content.into_owned(), span.style))
                .collect();
            Line::from(owned_spans)
        })
        .collect();
    state.messages_scrolling_state.messages.push(Message {
        id: uuid::Uuid::new_v4(),
        content: MessageContent::StyledBlock(owned_lines),
        is_collapsed: None,
    });
    invalidate_message_lines_cache(state);
}

pub fn push_styled_message(
    state: &mut AppState,
    message: &str,
    color: Color,
    icon: &str,
    icon_color: Color,
) {
    let line = Line::from(vec![
        Span::styled(icon.to_string(), Style::default().fg(icon_color)),
        Span::styled(message.to_string(), Style::default().fg(color)),
    ]);
    state
        .messages_scrolling_state
        .messages
        .push(Message::styled(line));
    invalidate_message_lines_cache(state);
}

pub fn version_message(latest_version: Option<String>) -> Message {
    match latest_version {
        Some(version) => {
            if version != format!("v{}", env!("CARGO_PKG_VERSION")) {
                Message::info(
                    format!(
                        "🚀 update available!  v{}  →  {} ✨   ",
                        env!("CARGO_PKG_VERSION"),
                        version
                    ),
                    Some(Style::default().fg(ThemeColors::warning())),
                )
            } else {
                Message::info(format!("version: {}", env!("CARGO_PKG_VERSION")), None)
            }
        }
        None => Message::info(format!("version: {}", env!("CARGO_PKG_VERSION")), None),
    }
}

#[allow(unused_variables)]
pub fn welcome_messages(
    _latest_version: Option<String>,
    _state: &crate::app::AppState,
) -> Vec<Message> {
    // VAC v1.5 renders first-launch/idle state through the operator console
    // (`vac_operator_console`) so the screen can be backed by live model,
    // workspace, capability, and compiled registry data instead of a static
    // ASCII welcome banner.
    Vec::new()
}

pub fn push_clear_message(state: &mut AppState) {
    state.messages_scrolling_state.messages.clear();
    state.input_state.text_area.set_text("");
    state.input_state.show_helper_dropdown = false;
    let welcome_msg = welcome_messages(state.configuration_state.latest_version.clone(), state);
    state.messages_scrolling_state.messages.extend(welcome_msg);
    invalidate_message_lines_cache(state);
}

pub fn handle_errors(error: String) -> String {
    if error.contains(EXCEEDED_API_LIMIT_ERROR) {
        EXCEEDED_API_LIMIT_ERROR_MESSAGE.to_string()
    } else if error.contains("Unknown(\"") && error.ends_with("\")") {
        let start = 9; // length of "Unknown(\""
        let end = error.len() - 2; // remove ")" and "\""
        if start < end {
            error[start..end].to_string()
        } else {
            error
        }
    } else {
        error
    }
}

pub fn push_issue_message(state: &mut AppState) {
    let url = "https://github.com/Vastar-AI/vac/issues/new";

    // Try to open the URL in the default browser
    match open::that(url) {
        Ok(_) => {
            let message = Message::styled(Line::from(vec![
                Span::styled(
                    "🔗 Opening GitHub Issues... ",
                    Style::default().fg(ThemeColors::green()),
                ),
                Span::raw(url),
            ]));
            state.messages_scrolling_state.messages.push(message);
        }
        Err(e) => {
            let message = Message::styled(Line::from(vec![
                Span::styled(
                    "❌ Failed to open GitHub Issues: ",
                    Style::default().fg(ThemeColors::red()),
                ),
                Span::raw(e.to_string()),
                Span::raw(" - "),
                Span::raw(url),
            ]));
            state.messages_scrolling_state.messages.push(message);
        }
    }
    invalidate_message_lines_cache(state);
}

pub fn push_support_message(state: &mut AppState) {
    let url = "https://discord.gg/c4HUkDD45d";

    // Try to open the URL in the default browser
    match open::that(url) {
        Ok(_) => {
            let message = Message::styled(Line::from(vec![
                Span::styled(
                    "💬 Opening Discord Support... ",
                    Style::default().fg(ThemeColors::green()),
                ),
                Span::raw(url),
            ]));
            state.messages_scrolling_state.messages.push(message);
        }
        Err(e) => {
            let message = Message::styled(Line::from(vec![
                Span::styled(
                    "❌ Failed to open Discord Support: ",
                    Style::default().fg(ThemeColors::red()),
                ),
                Span::raw(e.to_string()),
                Span::raw(" - "),
                Span::raw(url),
            ]));
            state.messages_scrolling_state.messages.push(message);
        }
    }
    invalidate_message_lines_cache(state);
}
