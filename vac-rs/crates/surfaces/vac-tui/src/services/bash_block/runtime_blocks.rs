use super::*;

#[derive(Clone, Debug, PartialEq)]
pub enum RunCommandState {
    /// Initial state - waiting for user approval (Reset dot)
    Pending,
    /// Running state - command is executing (Yellow dot, "Running...")
    Running,
    /// Running with stall warning - command may be waiting for input (Yellow dot, warning message)
    RunningWithStallWarning(String),
    /// Completed successfully (Green dot)
    Completed,
    /// Failed/Error state (LightRed dot)
    Error,
    /// Cancelled by user (LightRed dot)
    Cancelled,
    /// Rejected by user (LightRed dot)
    Rejected,
    /// Skipped due to sequential execution failure (Yellow dot)
    Skipped,
}

/// Renders a unified run command block with consistent appearance across all states.
///
/// Layout:
/// ```text
/// ╭─● Run Command ──────────────────────────────────────╮
/// │ $ command --args here wrapped nicely                │
/// │   continuation of long command                      │
/// │                                                     │
/// │ Result:                                             │
/// │   output line 1                                     │
/// │   output line 2                                     │
/// ╰─────────────────────────────────────────────────────╯
/// ```
///
/// - Pending: Reset dot, no result section
/// - Running: Yellow dot + " - Running...", streaming result
/// - Completed: Green dot, final result
/// - Error: Red dot, error message
pub fn render_run_command_block(
    command: &str,
    result: Option<&str>,
    state: RunCommandState,
    terminal_width: usize,
) -> Vec<Line<'static>> {
    let content_width = if terminal_width > 4 {
        terminal_width - 4
    } else {
        40
    };
    let inner_width = content_width;
    let horizontal_line = "─".repeat(inner_width + 2);

    // Border color: accent for pending (preview), muted for error/cancelled/rejected/skipped, muted otherwise
    let border_color = match state {
        RunCommandState::Pending => tool_border_color(),
        RunCommandState::Error
        | RunCommandState::Cancelled
        | RunCommandState::Rejected
        | RunCommandState::Skipped => tool_muted_color(),
        _ => tool_muted_color(),
    };

    // Dot color and title suffix based on state
    // For error states, both dot and suffix text are danger color
    // For running/skipped, both dot and suffix text are warning color
    let (dot_color, title_suffix, suffix_color) = match &state {
        RunCommandState::Pending => (ThemeColors::text(), "".to_string(), None),
        RunCommandState::Running => (
            ThemeColors::warning(),
            " - Running...".to_string(),
            Some(ThemeColors::warning()),
        ),
        RunCommandState::RunningWithStallWarning(msg) => {
            // Show the stall warning message in the title
            (
                ThemeColors::warning(),
                format!(" - {}", msg),
                Some(ThemeColors::warning()),
            )
        }
        RunCommandState::Completed => (ThemeColors::success(), "".to_string(), None),
        RunCommandState::Error => (
            ThemeColors::danger(),
            " - Errored".to_string(),
            Some(ThemeColors::danger()),
        ),
        RunCommandState::Cancelled => (
            ThemeColors::danger(),
            " - Cancelled".to_string(),
            Some(ThemeColors::danger()),
        ),
        RunCommandState::Rejected => (
            ThemeColors::danger(),
            " - Rejected".to_string(),
            Some(ThemeColors::danger()),
        ),
        RunCommandState::Skipped => (
            ThemeColors::warning(),
            " - Skipped".to_string(),
            Some(ThemeColors::warning()),
        ),
    };

    // Title structure: "╭─" + "●" + " Run Command" + suffix + " " + dashes + "╮"
    let base_title = "Run Command";
    let title_text = format!("{}{}", base_title, title_suffix);
    // Title border parts: "╭─" (2) + "●" (1) + " title " (title_text.len + 2) + dashes + "╮" (1)
    // Total should equal inner_width + 4
    // So: 2 + 1 + (title_text.len + 2) + dashes + 1 = inner_width + 4
    // dashes = inner_width + 4 - 2 - 1 - title_text.len - 2 - 1 = inner_width - title_text.len - 2
    let title_display_len = calculate_display_width(&title_text);
    let remaining_dashes = inner_width.saturating_sub(title_display_len + 2);

    // Build title spans - suffix gets special color for error states
    let title_border = if let Some(color) = suffix_color {
        // Split rendering: "Run Command" in white, suffix in special color
        let suffix_style = Style::default().fg(color).add_modifier(Modifier::BOLD);
        Line::from(vec![
            Span::styled("╭─", Style::default().fg(border_color)),
            Span::styled(
                "●",
                Style::default().fg(dot_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {} ", base_title),
                Style::default()
                    .fg(ThemeColors::text())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("{} ", title_suffix.trim_start()), suffix_style),
            Span::styled(
                format!("{}╮", "─".repeat(remaining_dashes)),
                Style::default().fg(border_color),
            ),
        ])
    } else {
        // Single color rendering for all text
        Line::from(vec![
            Span::styled("╭─", Style::default().fg(border_color)),
            Span::styled(
                "●",
                Style::default().fg(dot_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {} ", title_text),
                Style::default()
                    .fg(ThemeColors::text())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}╮", "─".repeat(remaining_dashes)),
                Style::default().fg(border_color),
            ),
        ])
    };

    let bottom_border = Line::from(vec![Span::styled(
        format!("╰{}╯", horizontal_line),
        Style::default().fg(border_color),
    )]);

    let mut formatted_lines = Vec::new();
    formatted_lines.push(title_border);

    // Render command with $ prefix
    // Line structure: "│" (1) + " " (1) + content + padding + " " (1) + "│" (1) = inner_width + 4
    // So: content + padding = inner_width
    let command_with_prefix = format!("$ {}", command);

    // Available width for content = inner_width (we have 1 space padding on each side)
    let max_content_width = inner_width;
    let wrapped_lines = wrap_text_by_word(&command_with_prefix, max_content_width);

    for (i, wrapped_line) in wrapped_lines.iter().enumerate() {
        let line_display_width = calculate_display_width(wrapped_line);
        // padding = max_content_width - content_width
        let padding_needed = max_content_width.saturating_sub(line_display_width);

        // First line has "$ " in magenta, continuation lines are plain
        if i == 0 && wrapped_line.starts_with("$ ") {
            let cmd_part = &wrapped_line[2..]; // Skip "$ "
            let line_spans = vec![
                Span::styled("│", Style::default().fg(border_color)),
                Span::from(" "),
                Span::styled(
                    "$ ".to_string(),
                    Style::default()
                        .fg(ThemeColors::magenta())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    cmd_part.to_string(),
                    Style::default().fg(ThemeColors::text()),
                ),
                Span::from(" ".repeat(padding_needed)),
                Span::styled(" │", Style::default().fg(border_color)),
            ];
            formatted_lines.push(Line::from(line_spans));
        } else {
            let line_spans = vec![
                Span::styled("│", Style::default().fg(border_color)),
                Span::from(" "),
                Span::styled(
                    wrapped_line.clone(),
                    Style::default().fg(ThemeColors::text()),
                ),
                Span::from(" ".repeat(padding_needed)),
                Span::styled(" │", Style::default().fg(border_color)),
            ];
            formatted_lines.push(Line::from(line_spans));
        }
    }

    // Add result section if we have non-empty result content
    if let Some(result_content) = result.filter(|s| !s.is_empty()) {
        // Check if this is an error state (no "Result:" label, colored message)
        let is_error_state = matches!(
            state,
            RunCommandState::Error
                | RunCommandState::Cancelled
                | RunCommandState::Rejected
                | RunCommandState::Skipped
        );

        // Replace raw status strings with friendly messages
        let result_content = if result_content.contains("TOOL_CALL_REJECTED") {
            "Command was rejected"
        } else if result_content.contains("TOOL_CALL_CANCELLED") {
            "Command was cancelled"
        } else {
            result_content
        };

        // Message color for error states
        let error_message_color = match state {
            RunCommandState::Skipped => ThemeColors::warning(),
            _ => ThemeColors::danger(),
        };

        // Empty line separator
        // Line structure: "│" (1) + spaces (inner_width + 2) + "│" (1) = inner_width + 4
        formatted_lines.push(Line::from(vec![
            Span::styled("│", Style::default().fg(border_color)),
            Span::from(" ".repeat(inner_width + 2)),
            Span::styled("│", Style::default().fg(border_color)),
        ]));

        // Only show "Result:" label for non-error states
        if !is_error_state {
            // Result: label
            // Line structure: "│" (1) + " " (1) + label + padding + " " (1) + "│" (1)
            // So: content + padding = inner_width
            let result_label = "Result:";
            let label_padding = inner_width.saturating_sub(result_label.len());
            formatted_lines.push(Line::from(vec![
                Span::styled("│", Style::default().fg(border_color)),
                Span::from(" "),
                Span::styled(
                    result_label.to_string(),
                    Style::default()
                        .fg(ThemeColors::title_primary())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::from(" ".repeat(label_padding)),
                Span::styled(" │", Style::default().fg(border_color)),
            ]));
        }

        // Strip ANSI codes, preprocess, and word-wrap the result content
        let cleaned_result = strip_ansi_codes(result_content).to_string();
        let preprocessed = preprocess_terminal_output(&cleaned_result);

        // Determine text color based on state
        let text_color = if is_error_state {
            error_message_color
        } else {
            ThemeColors::text()
        };

        // Process each line from the preprocessed result
        for source_line in preprocessed.lines() {
            if source_line.is_empty() {
                // Empty line
                formatted_lines.push(Line::from(vec![
                    Span::styled("│", Style::default().fg(border_color)),
                    Span::from(" ".repeat(inner_width + 2)),
                    Span::styled("│", Style::default().fg(border_color)),
                ]));
            } else {
                // Word-wrap this line
                let wrapped = wrap_text_by_word(source_line, inner_width);
                for line_text in wrapped {
                    let line_width = calculate_display_width(&line_text);
                    let padding = inner_width.saturating_sub(line_width);
                    formatted_lines.push(Line::from(vec![
                        Span::styled("│", Style::default().fg(border_color)),
                        Span::from(" "),
                        Span::styled(line_text, Style::default().fg(text_color)),
                        Span::from(" ".repeat(padding)),
                        Span::styled(" │", Style::default().fg(border_color)),
                    ]));
                }
            }
        }
    }

    formatted_lines.push(bottom_border);

    // Convert to owned lines
    let owned_lines: Vec<Line<'static>> = formatted_lines
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

    owned_lines
}

/// Render an ask_user tool block inline, similar to render_run_command_block.
/// Shows a bordered block with tab bar, question content or review, and help text.
pub fn render_ask_user_block(
    questions: &[vac_foundation::models::integrations::openai::AskUserQuestion],
    answers: &std::collections::HashMap<
        String,
        vac_foundation::models::integrations::openai::AskUserAnswer,
    >,
    current_tab: usize,
    selected_option: usize,
    custom_input: &str,
    terminal_width: usize,
    _focused: bool,
) -> Vec<Line<'static>> {
    let content_width = if terminal_width > 4 {
        terminal_width - 4
    } else {
        40
    };
    let inner_width = content_width;
    let horizontal_line = "─".repeat(inner_width + 2);
    let border_color = ThemeColors::cyan();
    let dot_color = ThemeColors::cyan();

    // Title
    let base_title = "Ask User";
    let title_display_len = calculate_display_width(base_title);
    let remaining_dashes = inner_width.saturating_sub(title_display_len + 2);

    let title_border = Line::from(vec![
        Span::styled("╭─", Style::default().fg(border_color)),
        Span::styled(
            "●",
            Style::default().fg(dot_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {} ", base_title),
            Style::default()
                .fg(ThemeColors::title_primary())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{}╮", "─".repeat(remaining_dashes)),
            Style::default().fg(border_color),
        ),
    ]);

    let bottom_border = Line::from(vec![Span::styled(
        format!("╰{}╯", horizontal_line),
        Style::default().fg(border_color),
    )]);

    let mut formatted_lines = Vec::new();
    formatted_lines.push(title_border);

    let max_content_width = inner_width;
    let is_submit_tab = current_tab >= questions.len();

    // --- Tab bar ---
    {
        let mut tab_spans = Vec::new();
        tab_spans.push(Span::styled(
            " ← ",
            Style::default().fg(ThemeColors::dark_gray()),
        ));

        for (i, q) in questions.iter().enumerate() {
            let is_current = i == current_tab;
            let is_answered = answers.contains_key(&q.label);
            let checkbox = if is_answered { "✓ " } else { "□ " };
            let style = if is_current {
                Style::default().bg(ThemeColors::cyan()).fg(Color::Black)
            } else if is_answered {
                Style::default().fg(ThemeColors::cyan())
            } else {
                Style::default().fg(ThemeColors::dark_gray())
            };
            let label = if q.label.chars().count() > 15 {
                format!("{}...", q.label.chars().take(12).collect::<String>())
            } else {
                q.label.clone()
            };
            tab_spans.push(Span::styled(format!("{}{}", checkbox, label), style));
            tab_spans.push(Span::raw("   "));
        }

        let submit_style = if is_submit_tab {
            Style::default().bg(ThemeColors::cyan()).fg(Color::Black)
        } else {
            Style::default().fg(ThemeColors::green())
        };
        tab_spans.push(Span::styled("Review", submit_style));
        tab_spans.push(Span::styled(
            " →",
            Style::default().fg(ThemeColors::dark_gray()),
        ));

        let tab_text: String = tab_spans.iter().map(|s| s.content.as_ref()).collect();
        let tab_display_width = calculate_display_width(&tab_text);
        let tab_padding = max_content_width.saturating_sub(tab_display_width);

        let mut line_spans = vec![
            Span::styled("│", Style::default().fg(border_color)),
            Span::from(" "),
        ];
        line_spans.extend(tab_spans);
        line_spans.push(Span::from(" ".repeat(tab_padding)));
        line_spans.push(Span::styled(" │", Style::default().fg(border_color)));
        formatted_lines.push(Line::from(line_spans));
    }

    // --- Separator ---
    formatted_lines.push(Line::from(vec![
        Span::styled("├", Style::default().fg(border_color)),
        Span::styled(
            "─".repeat(inner_width + 2),
            Style::default().fg(border_color),
        ),
        Span::styled("┤", Style::default().fg(border_color)),
    ]));

    if is_submit_tab {
        // --- Review / Submit content ---
        // Empty line
        formatted_lines.push(Line::from(vec![
            Span::styled("│", Style::default().fg(border_color)),
            Span::from(" ".repeat(inner_width + 2)),
            Span::styled("│", Style::default().fg(border_color)),
        ]));

        for q in questions {
            let review_label_width = max_content_width.saturating_sub(2);
            let wrapped_review_label = wrap_text_by_word(&q.label, review_label_width.max(1));
            for review_line in &wrapped_review_label {
                let line_width = calculate_display_width(review_line);
                let line_padding = max_content_width.saturating_sub(line_width);
                formatted_lines.push(Line::from(vec![
                    Span::styled("│", Style::default().fg(border_color)),
                    Span::from(" "),
                    Span::styled(
                        review_line.clone(),
                        Style::default()
                            .fg(ThemeColors::title_primary())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::from(" ".repeat(line_padding)),
                    Span::styled(" │", Style::default().fg(border_color)),
                ]));
            }

            if let Some(answer) = answers.get(&q.label) {
                // Collect display labels for the answer(s)
                let answer_labels: Vec<String> =
                    if q.multi_select && !answer.selected_values.is_empty() {
                        answer
                            .selected_values
                            .iter()
                            .map(|val| {
                                q.options
                                    .iter()
                                    .find(|o| &o.value == val)
                                    .map(|o| o.label.clone())
                                    .unwrap_or_else(|| val.clone())
                            })
                            .collect()
                    } else {
                        let display = if answer.is_custom {
                            answer.answer.clone()
                        } else {
                            q.options
                                .iter()
                                .find(|o| o.value == answer.answer)
                                .map(|o| o.label.clone())
                                .unwrap_or_else(|| answer.answer.clone())
                        };
                        vec![display]
                    };

                // Render each answer label with uniform [✓] style
                for label in &answer_labels {
                    let max_display = max_content_width.saturating_sub(8);
                    let display = if label.chars().count() > max_display {
                        format!(
                            "{}…",
                            label
                                .chars()
                                .take(max_display.saturating_sub(1))
                                .collect::<String>()
                        )
                    } else {
                        label.clone()
                    };
                    let answer_text = format!("    [✓] {}", display);
                    let answer_width = calculate_display_width(&answer_text);
                    let answer_padding = max_content_width.saturating_sub(answer_width);
                    formatted_lines.push(Line::from(vec![
                        Span::styled("│", Style::default().fg(border_color)),
                        Span::from(" "),
                        Span::raw("    "),
                        Span::styled(
                            "[✓]",
                            Style::default()
                                .fg(ThemeColors::green())
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(display, Style::default().fg(ThemeColors::cyan())),
                        Span::from(" ".repeat(answer_padding)),
                        Span::styled(" │", Style::default().fg(border_color)),
                    ]));
                }
            } else {
                let text = "  — skipped";
                let text_width = calculate_display_width(text);
                let text_padding = max_content_width.saturating_sub(text_width);
                formatted_lines.push(Line::from(vec![
                    Span::styled("│", Style::default().fg(border_color)),
                    Span::from(" "),
                    Span::styled("  — ", Style::default().fg(ThemeColors::dark_gray())),
                    Span::styled("skipped", Style::default().fg(ThemeColors::dark_gray())),
                    Span::from(" ".repeat(text_padding)),
                    Span::styled(" │", Style::default().fg(border_color)),
                ]));
            }

            // Spacing between questions
            formatted_lines.push(Line::from(vec![
                Span::styled("│", Style::default().fg(border_color)),
                Span::from(" ".repeat(inner_width + 2)),
                Span::styled("│", Style::default().fg(border_color)),
            ]));
        }
    } else if let Some(q) = questions.get(current_tab) {
        // --- Question content ---
        let previous_answer = answers.get(&q.label);

        // Empty line
        formatted_lines.push(Line::from(vec![
            Span::styled("│", Style::default().fg(border_color)),
            Span::from(" ".repeat(inner_width + 2)),
            Span::styled("│", Style::default().fg(border_color)),
        ]));

        // Question text (wrapped)
        let text_width = max_content_width.saturating_sub(2);
        let wrapped_question = wrap_text_by_word(&q.question, text_width);
        for line in &wrapped_question {
            let line_width = calculate_display_width(line);
            let padding = max_content_width.saturating_sub(line_width);
            formatted_lines.push(Line::from(vec![
                Span::styled("│", Style::default().fg(border_color)),
                Span::from(" "),
                Span::styled(
                    line.clone(),
                    Style::default()
                        .fg(ThemeColors::title_primary())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::from(" ".repeat(padding)),
                Span::styled(" │", Style::default().fg(border_color)),
            ]));
        }

        // Empty line
        formatted_lines.push(Line::from(vec![
            Span::styled("│", Style::default().fg(border_color)),
            Span::from(" ".repeat(inner_width + 2)),
            Span::styled("│", Style::default().fg(border_color)),
        ]));

        // Options
        for (i, opt) in q.options.iter().enumerate() {
            let is_cursor = i == selected_option;

            // Determine if this option is "checked" (answered/selected)
            let is_checked = if q.multi_select {
                // Multi-select: check if this value is in selected_values
                previous_answer
                    .map(|a| a.selected_values.contains(&opt.value))
                    .unwrap_or(false)
            } else {
                // Single-select: check if this is the chosen answer
                previous_answer
                    .map(|a| !a.is_custom && a.answer == opt.value)
                    .unwrap_or(false)
            };

            // Uniform bracket rendering:
            //   [›] = cursor is here, not checked (multi-select)
            //   [✓] = checked (cursor shown via underline/bold style)
            //   [ ] = unchecked (not cursor)
            //   [›] = cursor is here (single-select, not yet selected)
            //   [✓] = selected in single-select
            //   [n] = numbered in single-select (not cursor, not selected)
            let bracket = if q.multi_select {
                if is_checked {
                    "[✓]".to_string()
                } else if is_cursor {
                    "[›]".to_string()
                } else {
                    "[ ]".to_string()
                }
            } else if is_checked {
                "[✓]".to_string()
            } else if is_cursor {
                "[›]".to_string()
            } else {
                format!("[{}]", i + 1)
            };

            let bracket_style = if is_cursor && is_checked {
                Style::default()
                    .fg(ThemeColors::green())
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED)
            } else if is_checked {
                Style::default()
                    .fg(ThemeColors::green())
                    .add_modifier(Modifier::BOLD)
            } else if is_cursor {
                Style::default()
                    .fg(ThemeColors::cyan())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(ThemeColors::dark_gray())
            };

            let label_style = if is_cursor && is_checked {
                Style::default()
                    .fg(ThemeColors::accent())
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED)
            } else if is_checked {
                Style::default().fg(ThemeColors::accent())
            } else if is_cursor {
                Style::default()
                    .fg(ThemeColors::title_primary())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(ThemeColors::muted())
            };

            // Pre-wrap option label so continuation lines keep borders
            // and align with the text start (after bracket + space).
            let bracket_display_width = calculate_display_width(&bracket);
            let label_indent_width = bracket_display_width + 1; // "[›] " prefix
            let label_available_width = max_content_width.saturating_sub(label_indent_width);
            let wrapped_label = wrap_text_by_word(&opt.label, label_available_width.max(1));

            for (li, label_line) in wrapped_label.iter().enumerate() {
                if li == 0 {
                    let opt_text = format!("{} {}", bracket, label_line);
                    let opt_width = calculate_display_width(&opt_text);
                    let opt_padding = max_content_width.saturating_sub(opt_width);
                    formatted_lines.push(Line::from(vec![
                        Span::styled("│", Style::default().fg(border_color)),
                        Span::from(" "),
                        Span::styled(bracket.clone(), bracket_style),
                        Span::raw(" "),
                        Span::styled(label_line.clone(), label_style),
                        Span::from(" ".repeat(opt_padding)),
                        Span::styled(" │", Style::default().fg(border_color)),
                    ]));
                } else {
                    let indent = " ".repeat(label_indent_width);
                    let line_text = format!("{}{}", indent, label_line);
                    let line_width = calculate_display_width(&line_text);
                    let line_padding = max_content_width.saturating_sub(line_width);
                    formatted_lines.push(Line::from(vec![
                        Span::styled("│", Style::default().fg(border_color)),
                        Span::from(" "),
                        Span::raw(indent),
                        Span::styled(label_line.clone(), label_style),
                        Span::from(" ".repeat(line_padding)),
                        Span::styled(" │", Style::default().fg(border_color)),
                    ]));
                }
            }

            if let Some(desc) = &opt.description {
                let desc_style = if is_cursor || is_checked {
                    Style::default().fg(ThemeColors::muted())
                } else {
                    Style::default().fg(ThemeColors::dark_gray())
                };
                // Align description with label text start
                let desc_indent = " ".repeat(label_indent_width);
                let desc_available_width = max_content_width.saturating_sub(label_indent_width);
                let wrapped_desc = wrap_text_by_word(desc, desc_available_width.max(1));
                for desc_line in &wrapped_desc {
                    let desc_text = format!("{}{}", desc_indent, desc_line);
                    let desc_width = calculate_display_width(&desc_text);
                    let desc_padding = max_content_width.saturating_sub(desc_width);
                    formatted_lines.push(Line::from(vec![
                        Span::styled("│", Style::default().fg(border_color)),
                        Span::from(" "),
                        Span::styled(desc_text, desc_style),
                        Span::from(" ".repeat(desc_padding)),
                        Span::styled(" │", Style::default().fg(border_color)),
                    ]));
                }
            }
        }

        // Custom input option (not available for multi-select)
        if q.allow_custom && !q.multi_select {
            let custom_idx = q.options.len();
            let is_selected = selected_option == custom_idx;
            let is_custom_answered = previous_answer.map(|a| a.is_custom).unwrap_or(false);

            let (bracket, bracket_style) = if is_custom_answered {
                (
                    "[✓]".to_string(),
                    Style::default()
                        .fg(ThemeColors::green())
                        .add_modifier(Modifier::BOLD),
                )
            } else if is_selected {
                (
                    "[›]".to_string(),
                    Style::default()
                        .fg(ThemeColors::cyan())
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                (
                    format!("[{}]", custom_idx + 1),
                    Style::default().fg(ThemeColors::dark_gray()),
                )
            };

            if is_selected {
                if custom_input.is_empty() {
                    let text = format!("{} │Type your answer...", bracket);
                    let text_width = calculate_display_width(&text);
                    let text_padding = max_content_width.saturating_sub(text_width);
                    formatted_lines.push(Line::from(vec![
                        Span::styled("│", Style::default().fg(border_color)),
                        Span::from(" "),
                        Span::styled(bracket, bracket_style),
                        Span::raw(" "),
                        Span::styled("│", Style::default().fg(ThemeColors::cyan())),
                        Span::styled(
                            "Type your answer...",
                            Style::default().fg(ThemeColors::dark_gray()),
                        ),
                        Span::from(" ".repeat(text_padding)),
                        Span::styled(" │", Style::default().fg(border_color)),
                    ]));
                } else {
                    // Wrap long custom input text to fit within the bordered block.
                    // Layout per line: "│" + " " + prefix + text + cursor(last line only) + padding + " │"
                    // First line prefix: "[›] " (bracket + space)
                    // Continuation prefix: "    " (same-width indent)
                    let bracket_display_width = calculate_display_width(&bracket);
                    let prefix_width = bracket_display_width + 1; // bracket + space
                    // Reserve 1 char for cursor on last line
                    let available_text_width = max_content_width.saturating_sub(prefix_width + 1);
                    let wrapped_input =
                        wrap_text_by_word(custom_input, available_text_width.max(1));
                    let total_lines = wrapped_input.len();

                    for (line_idx, input_line) in wrapped_input.iter().enumerate() {
                        let is_last = line_idx == total_lines - 1;
                        let is_first = line_idx == 0;

                        if is_first {
                            let cursor_part = if is_last { "│" } else { "" };
                            let text = format!("{} {}{}", bracket, input_line, cursor_part);
                            let text_width = calculate_display_width(&text);
                            let text_padding = max_content_width.saturating_sub(text_width);
                            let mut spans = vec![
                                Span::styled("│", Style::default().fg(border_color)),
                                Span::from(" "),
                                Span::styled(bracket.clone(), bracket_style),
                                Span::raw(" "),
                                Span::styled(
                                    input_line.clone(),
                                    Style::default()
                                        .fg(ThemeColors::title_primary())
                                        .add_modifier(Modifier::BOLD),
                                ),
                            ];
                            if is_last {
                                spans.push(Span::styled(
                                    "│",
                                    Style::default().fg(ThemeColors::accent()),
                                ));
                            }
                            spans.push(Span::from(" ".repeat(text_padding)));
                            spans.push(Span::styled(" │", Style::default().fg(border_color)));
                            formatted_lines.push(Line::from(spans));
                        } else {
                            // Continuation lines: indent to align with text above
                            let indent = " ".repeat(prefix_width);
                            let cursor_part = if is_last { "│" } else { "" };
                            let text = format!("{}{}{}", indent, input_line, cursor_part);
                            let text_width = calculate_display_width(&text);
                            let text_padding = max_content_width.saturating_sub(text_width);
                            let mut spans = vec![
                                Span::styled("│", Style::default().fg(border_color)),
                                Span::from(" "),
                                Span::raw(indent),
                                Span::styled(
                                    input_line.clone(),
                                    Style::default()
                                        .fg(ThemeColors::title_primary())
                                        .add_modifier(Modifier::BOLD),
                                ),
                            ];
                            if is_last {
                                spans.push(Span::styled(
                                    "│",
                                    Style::default().fg(ThemeColors::accent()),
                                ));
                            }
                            spans.push(Span::from(" ".repeat(text_padding)));
                            spans.push(Span::styled(" │", Style::default().fg(border_color)));
                            formatted_lines.push(Line::from(spans));
                        }
                    }
                }
            } else if is_custom_answered && let Some(answer) = previous_answer {
                // Wrap long answered custom text to fit within the bordered block.
                let bracket_display_width = calculate_display_width(&bracket);
                let prefix_width = bracket_display_width + 1;
                let available_text_width = max_content_width.saturating_sub(prefix_width);
                let wrapped_answer = wrap_text_by_word(&answer.answer, available_text_width.max(1));

                for (line_idx, answer_line) in wrapped_answer.iter().enumerate() {
                    let is_first = line_idx == 0;

                    if is_first {
                        let text = format!("{} {}", bracket, answer_line);
                        let text_width = calculate_display_width(&text);
                        let text_padding = max_content_width.saturating_sub(text_width);
                        formatted_lines.push(Line::from(vec![
                            Span::styled("│", Style::default().fg(border_color)),
                            Span::from(" "),
                            Span::styled(bracket.clone(), bracket_style),
                            Span::raw(" "),
                            Span::styled(
                                answer_line.clone(),
                                Style::default().fg(ThemeColors::accent()),
                            ),
                            Span::from(" ".repeat(text_padding)),
                            Span::styled(" │", Style::default().fg(border_color)),
                        ]));
                    } else {
                        let indent = " ".repeat(prefix_width);
                        let text = format!("{}{}", indent, answer_line);
                        let text_width = calculate_display_width(&text);
                        let text_padding = max_content_width.saturating_sub(text_width);
                        formatted_lines.push(Line::from(vec![
                            Span::styled("│", Style::default().fg(border_color)),
                            Span::from(" "),
                            Span::raw(indent),
                            Span::styled(
                                answer_line.clone(),
                                Style::default().fg(ThemeColors::accent()),
                            ),
                            Span::from(" ".repeat(text_padding)),
                            Span::styled(" │", Style::default().fg(border_color)),
                        ]));
                    }
                }
            } else {
                let text = format!("{} Other...", bracket);
                let text_width = calculate_display_width(&text);
                let text_padding = max_content_width.saturating_sub(text_width);
                formatted_lines.push(Line::from(vec![
                    Span::styled("│", Style::default().fg(border_color)),
                    Span::from(" "),
                    Span::styled(bracket, bracket_style),
                    Span::raw(" "),
                    Span::styled("Other...", Style::default().fg(ThemeColors::dark_gray())),
                    Span::from(" ".repeat(text_padding)),
                    Span::styled(" │", Style::default().fg(border_color)),
                ]));
            }
        }

        // Empty line after options
        formatted_lines.push(Line::from(vec![
            Span::styled("│", Style::default().fg(border_color)),
            Span::from(" ".repeat(inner_width + 2)),
            Span::styled("│", Style::default().fg(border_color)),
        ]));
    }

    // --- Separator before help ---
    formatted_lines.push(Line::from(vec![
        Span::styled("├", Style::default().fg(border_color)),
        Span::styled(
            "─".repeat(inner_width + 2),
            Style::default().fg(border_color),
        ),
        Span::styled("┤", Style::default().fg(border_color)),
    ]));

    // --- Help text ---
    {
        let help_spans = if is_submit_tab {
            vec![
                Span::styled("Enter", Style::default().fg(ThemeColors::dark_gray())),
                Span::styled(" submit", Style::default().fg(ThemeColors::green())),
                Span::raw(" · "),
                Span::styled("←/→", Style::default().fg(ThemeColors::dark_gray())),
                Span::styled(" questions", Style::default().fg(ThemeColors::cyan())),
                Span::raw(" · "),
                Span::styled("Esc", Style::default().fg(ThemeColors::dark_gray())),
                Span::styled(" cancel", Style::default().fg(ThemeColors::cyan())),
            ]
        } else {
            let is_custom_selected = questions
                .get(current_tab)
                .map(|q| q.allow_custom && !q.multi_select && selected_option == q.options.len())
                .unwrap_or(false);

            if is_custom_selected {
                vec![
                    Span::styled("Type", Style::default().fg(ThemeColors::dark_gray())),
                    Span::styled(" your answer", Style::default().fg(ThemeColors::cyan())),
                    Span::raw(" · "),
                    Span::styled("Enter", Style::default().fg(ThemeColors::dark_gray())),
                    Span::styled(" confirm", Style::default().fg(ThemeColors::cyan())),
                    Span::raw(" · "),
                    Span::styled("↑", Style::default().fg(ThemeColors::dark_gray())),
                    Span::styled(" back", Style::default().fg(ThemeColors::cyan())),
                ]
            } else {
                let is_multi = questions
                    .get(current_tab)
                    .map(|q| q.multi_select)
                    .unwrap_or(false);

                if is_multi {
                    vec![
                        Span::styled("Space", Style::default().fg(ThemeColors::dark_gray())),
                        Span::styled(" toggle", Style::default().fg(ThemeColors::cyan())),
                        Span::raw(" · "),
                        Span::styled("Enter", Style::default().fg(ThemeColors::dark_gray())),
                        Span::styled(" next", Style::default().fg(ThemeColors::cyan())),
                        Span::raw(" · "),
                        Span::styled("↑/↓", Style::default().fg(ThemeColors::dark_gray())),
                        Span::styled(" options", Style::default().fg(ThemeColors::cyan())),
                        Span::raw(" · "),
                        Span::styled("←/→", Style::default().fg(ThemeColors::dark_gray())),
                        Span::styled(" questions", Style::default().fg(ThemeColors::cyan())),
                        Span::raw(" · "),
                        Span::styled("Esc", Style::default().fg(ThemeColors::dark_gray())),
                        Span::styled(" cancel", Style::default().fg(ThemeColors::cyan())),
                    ]
                } else {
                    vec![
                        Span::styled("Space", Style::default().fg(ThemeColors::dark_gray())),
                        Span::styled(" select", Style::default().fg(ThemeColors::cyan())),
                        Span::raw(" · "),
                        Span::styled("Enter", Style::default().fg(ThemeColors::dark_gray())),
                        Span::styled(" next", Style::default().fg(ThemeColors::cyan())),
                        Span::raw(" · "),
                        Span::styled("↑/↓", Style::default().fg(ThemeColors::dark_gray())),
                        Span::styled(" options", Style::default().fg(ThemeColors::cyan())),
                        Span::raw(" · "),
                        Span::styled("←/→", Style::default().fg(ThemeColors::dark_gray())),
                        Span::styled(" questions", Style::default().fg(ThemeColors::cyan())),
                        Span::raw(" · "),
                        Span::styled("Esc", Style::default().fg(ThemeColors::dark_gray())),
                        Span::styled(" cancel", Style::default().fg(ThemeColors::cyan())),
                    ]
                }
            }
        };

        let help_text: String = help_spans.iter().map(|s| s.content.as_ref()).collect();
        let help_width = calculate_display_width(&help_text);
        let help_padding = max_content_width.saturating_sub(help_width);

        let mut line_spans = vec![
            Span::styled("│", Style::default().fg(border_color)),
            Span::from(" "),
        ];
        line_spans.extend(help_spans);
        line_spans.push(Span::from(" ".repeat(help_padding)));
        line_spans.push(Span::styled(" │", Style::default().fg(border_color)));
        formatted_lines.push(Line::from(line_spans));
    }

    formatted_lines.push(bottom_border);

    // Convert to owned lines
    formatted_lines
        .into_iter()
        .map(|line| {
            let owned_spans: Vec<Span<'static>> = line
                .spans
                .into_iter()
                .map(|span| Span::styled(span.content.into_owned(), span.style))
                .collect();
            Line::from(owned_spans)
        })
        .collect()
}

/// Render a task wait block showing progress of background tasks
/// Displays a bordered box with task statuses and overall progress
pub fn render_task_wait_block(
    task_updates: &[vac_foundation::models::integrations::openai::TaskUpdate],
    progress: f64,
    target_task_ids: &[String],
    terminal_width: usize,
) -> Vec<Line<'static>> {
    let content_width = if terminal_width > 4 {
        terminal_width - 4
    } else {
        40
    };
    let inner_width = content_width;
    let horizontal_line = "─".repeat(inner_width + 2);

    // Border color - gray for all states (could differentiate later)
    let border_color = term_color(Color::Gray);

    // Check if all tasks are completed
    let all_completed = progress >= 100.0;

    // Dot color and title suffix based on progress
    let (dot_color, title_suffix, suffix_color) = if all_completed {
        (ThemeColors::dot_success(), "".to_string(), None)
    } else {
        let completed_count = task_updates
            .iter()
            .filter(|t| {
                t.is_target
                    && (t.status == "Completed"
                        || t.status == "Failed"
                        || t.status == "Cancelled"
                        || t.status == "TimedOut")
            })
            .count();
        let total_count = target_task_ids.len();
        (
            ThemeColors::warning(),
            format!(" - Waiting ({}/{})", completed_count, total_count),
            Some(ThemeColors::warning()),
        )
    };

    // Build title
    let base_title = "Wait for Tasks";
    let title_text = format!("{}{}", base_title, title_suffix);
    let title_display_len = calculate_display_width(&title_text);
    let remaining_dashes = inner_width.saturating_sub(title_display_len + 2);

    // Build title spans
    let title_border = if let Some(color) = suffix_color {
        Line::from(vec![
            Span::styled("╭─", Style::default().fg(border_color)),
            Span::styled(
                "●",
                Style::default().fg(dot_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {} ", base_title),
                Style::default()
                    .fg(ThemeColors::title_primary())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{} ", title_suffix.trim_start()),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}╮", "─".repeat(remaining_dashes)),
                Style::default().fg(border_color),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("╭─", Style::default().fg(border_color)),
            Span::styled(
                "●",
                Style::default().fg(dot_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {} ", title_text),
                Style::default()
                    .fg(ThemeColors::title_primary())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}╮", "─".repeat(remaining_dashes)),
                Style::default().fg(border_color),
            ),
        ])
    };

    let bottom_border = Line::from(vec![Span::styled(
        format!("╰{}╯", horizontal_line),
        Style::default().fg(border_color),
    )]);

    let mut formatted_lines = Vec::new();
    formatted_lines.push(title_border);

    // Filter to show only target tasks, sorted by status (running first, then completed)
    let mut target_tasks: Vec<_> = task_updates.iter().filter(|t| t.is_target).collect();

    // Sort: Running tasks first, then by task_id
    target_tasks.sort_by(|a, b| {
        let a_running = a.status == "Running" || a.status == "Pending";
        let b_running = b.status == "Running" || b.status == "Pending";
        match (a_running, b_running) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.task_id.cmp(&b.task_id),
        }
    });

    // Render each task
    for task in &target_tasks {
        let (status_icon, status_color) = match task.status.as_str() {
            "Running" => ("◐", ThemeColors::warning()),
            "Pending" => ("○", ThemeColors::muted()),
            "Paused" => ("⏸", ThemeColors::magenta()),
            "Completed" => ("✓", ThemeColors::dot_success()),
            "Failed" => ("✗", ThemeColors::dot_error()),
            "Cancelled" => ("⊘", ThemeColors::dot_error()),
            "TimedOut" => ("⏱", ThemeColors::dot_error()),
            _ => ("?", ThemeColors::muted()),
        };

        // Format duration
        let duration_str = task
            .duration_secs
            .map(|d| {
                if d < 60.0 {
                    format!("{:.1}s", d)
                } else {
                    format!("{:.0}m{:.0}s", d / 60.0, d % 60.0)
                }
            })
            .unwrap_or_else(|| "...".to_string());

        // Truncate task_id for display (show first 8 chars)
        let task_id_display = if task.task_id.chars().count() > 8 {
            let truncated: String = task.task_id.chars().take(8).collect();
            format!("{}…", truncated)
        } else {
            task.task_id.clone()
        };

        // Get description or fall back to truncated task_id
        let raw_description = task
            .description
            .as_ref()
            .cloned()
            .unwrap_or_else(|| task_id_display.clone());

        // Detect and strip [sandboxed] tag for separate rendering
        let is_sandboxed = raw_description.contains("[sandboxed]");
        let clean_description = raw_description
            .replace(" [sandboxed]", "")
            .replace("[sandboxed]", "");

        let display_name = if clean_description.chars().count() > 30 {
            let truncated: String = clean_description.chars().take(30).collect();
            format!("{}…", truncated)
        } else {
            clean_description
        };

        let sandboxed_tag = if is_sandboxed { " [sandboxed]" } else { "" };

        // Build the task line: "│ ● description [sandboxed] [duration] status │"
        let task_content = format!(
            "{}{} {} [{}]",
            display_name, sandboxed_tag, task.status, duration_str
        );
        let content_display_width = calculate_display_width(&task_content) + 2; // +2 for icon and space
        let padding_needed = inner_width.saturating_sub(content_display_width);

        let mut line_spans = vec![
            Span::styled("│", Style::default().fg(border_color)),
            Span::from(" "),
            Span::styled(
                status_icon.to_string(),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::from(" "),
            Span::styled(display_name, Style::default().fg(AdaptiveColors::text())),
        ];
        if is_sandboxed {
            line_spans.push(Span::styled(
                " [sandboxed]",
                Style::default().fg(ThemeColors::green()),
            ));
        }
        line_spans.extend([
            Span::styled(
                format!(" [{}]", duration_str),
                Style::default().fg(ThemeColors::dark_gray()),
            ),
            Span::styled(
                format!(" {}", task.status),
                Style::default().fg(status_color),
            ),
            Span::from(" ".repeat(padding_needed)),
            Span::styled(" │", Style::default().fg(border_color)),
        ]);
        formatted_lines.push(Line::from(line_spans));

        // If task is paused, show pause info (agent message and pending tool calls)
        if let Some(pause_info) = &task.pause_info {
            // Show agent message if present
            if let Some(agent_msg) = &pause_info.agent_message {
                let trimmed_msg = agent_msg.trim();
                if !trimmed_msg.is_empty() {
                    // Truncate long messages (char-aware to avoid slicing mid-character)
                    let max_msg_chars = inner_width.saturating_sub(7);
                    let display_msg = if calculate_display_width(trimmed_msg)
                        > inner_width.saturating_sub(6)
                    {
                        let truncated: String = trimmed_msg.chars().take(max_msg_chars).collect();
                        format!("{}…", truncated)
                    } else {
                        trimmed_msg.to_string()
                    };
                    let msg_padding =
                        inner_width.saturating_sub(calculate_display_width(&display_msg) + 4);
                    formatted_lines.push(Line::from(vec![
                        Span::styled("│", Style::default().fg(border_color)),
                        Span::from("     "),
                        Span::styled(
                            display_msg,
                            Style::default()
                                .fg(ThemeColors::dark_gray())
                                .add_modifier(Modifier::ITALIC),
                        ),
                        Span::from(" ".repeat(msg_padding)),
                        Span::styled("│", Style::default().fg(border_color)),
                    ]));
                }
            }

            // Show pending tool calls
            if let Some(tool_calls) = &pause_info.pending_tool_calls {
                for tc in tool_calls {
                    // Format: "  → tool_name(args_preview)"
                    let args_preview = {
                        let args_str = tc.arguments.to_string();
                        if args_str == "null" || args_str == "{}" {
                            String::new()
                        } else if args_str.len() > 40 {
                            // Find a valid UTF-8 boundary near 40 chars
                            let truncate_at = args_str
                                .char_indices()
                                .take_while(|(i, _)| *i < 40)
                                .last()
                                .map(|(i, c)| i + c.len_utf8())
                                .unwrap_or(0);
                            format!("{}…", &args_str[..truncate_at])
                        } else {
                            args_str
                        }
                    };

                    let tool_display = if args_preview.is_empty() {
                        format!("→ {}", tc.name)
                    } else {
                        format!("→ {}({})", tc.name, args_preview)
                    };

                    let tool_display_width = calculate_display_width(&tool_display) + 4;
                    let tool_padding = inner_width.saturating_sub(tool_display_width);

                    formatted_lines.push(Line::from(vec![
                        Span::styled("│", Style::default().fg(border_color)),
                        Span::from("     "),
                        Span::styled("→ ", Style::default().fg(ThemeColors::magenta())),
                        Span::styled(
                            tc.name.clone(),
                            Style::default()
                                .fg(ThemeColors::cyan())
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            if args_preview.is_empty() {
                                String::new()
                            } else {
                                format!("({})", args_preview)
                            },
                            Style::default().fg(ThemeColors::dark_gray()),
                        ),
                        Span::from(" ".repeat(tool_padding)),
                        Span::styled("│", Style::default().fg(border_color)),
                    ]));
                }
            }
        }
    }

    // If no target tasks, show a message
    if target_tasks.is_empty() {
        let msg = "No tasks to display";
        let padding = inner_width.saturating_sub(msg.len());
        formatted_lines.push(Line::from(vec![
            Span::styled("│", Style::default().fg(border_color)),
            Span::from(" "),
            Span::styled(
                msg.to_string(),
                Style::default().fg(ThemeColors::dark_gray()),
            ),
            Span::from(" ".repeat(padding)),
            Span::styled(" │", Style::default().fg(border_color)),
        ]));
    }

    formatted_lines.push(bottom_border);

    // Add spacing marker
    formatted_lines.push(Line::from(vec![Span::from("SPACING_MARKER")]));

    formatted_lines
}

/// Render a pending block for resume_subagent_task showing what the subagent wants to do
pub fn render_subagent_resume_pending_block<'a>(
    tool_call: &ToolCall,
    is_auto_approved: bool,
    pause_info: Option<&vac_foundation::models::integrations::openai::TaskPauseInfo>,
    width: usize,
) -> Vec<Line<'a>> {
    let mut formatted_lines: Vec<Line<'a>> = Vec::new();

    let border_color = if is_auto_approved {
        ThemeColors::green()
    } else {
        ThemeColors::cyan()
    };
    let inner_width = width.saturating_sub(4);

    // Parse arguments to determine resume type
    let args = serde_json::from_str::<serde_json::Value>(&tool_call.function.arguments).ok();

    // Extract task_id from arguments
    let task_id = args
        .as_ref()
        .and_then(|a| a.get("task_id").and_then(|v| v.as_str()).map(String::from))
        .unwrap_or_else(|| "unknown".to_string());

    // Check if this is an input-based resume (for completed agents) or tool approval resume
    let input_text = args
        .as_ref()
        .and_then(|a| a.get("input").and_then(|v| v.as_str()).map(String::from));

    let has_approve_all = args
        .as_ref()
        .and_then(|a| a.get("approve_all").and_then(|v| v.as_bool()))
        .unwrap_or(false);

    // Title
    let title = format!(" Resume Subagent [{}] ", task_id);
    let title_len = calculate_display_width(&title);
    let dashes_after = inner_width.saturating_sub(title_len + 1);

    // Top border with title
    let top_border = Line::from(vec![
        Span::styled("╭─", Style::default().fg(border_color)),
        Span::styled(
            title,
            Style::default()
                .fg(ThemeColors::magenta())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{}╮", "─".repeat(dashes_after)),
            Style::default().fg(border_color),
        ),
    ]);
    formatted_lines.push(top_border);

    // Handle input-based resume (completed agent, continuing with user input)
    if let Some(input) = input_text {
        let header = "Continue with input:";
        let header_padding = inner_width.saturating_sub(calculate_display_width(header));
        formatted_lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(border_color)),
            Span::styled(
                header.to_string(),
                Style::default()
                    .fg(ThemeColors::yellow())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::from(" ".repeat(header_padding)),
            Span::styled(" │", Style::default().fg(border_color)),
        ]));

        // Empty line
        formatted_lines.push(Line::from(vec![
            Span::styled("│", Style::default().fg(border_color)),
            Span::from(" ".repeat(inner_width + 2)),
            Span::styled("│", Style::default().fg(border_color)),
        ]));

        // Show the input text, wrapped if necessary
        let input_lines = wrap_text_to_lines(&input, inner_width.saturating_sub(4));
        for line in input_lines {
            let line_width = calculate_display_width(&line);
            let line_padding = inner_width.saturating_sub(line_width + 2);
            formatted_lines.push(Line::from(vec![
                Span::styled("│ ", Style::default().fg(border_color)),
                Span::styled("  ", Style::default()),
                Span::styled(line, Style::default().fg(ThemeColors::text())),
                Span::from(" ".repeat(line_padding)),
                Span::styled(" │", Style::default().fg(border_color)),
            ]));
        }

        // Empty line
        formatted_lines.push(Line::from(vec![
            Span::styled("│", Style::default().fg(border_color)),
            Span::from(" ".repeat(inner_width + 2)),
            Span::styled("│", Style::default().fg(border_color)),
        ]));
    } else if has_approve_all || pause_info.is_some() {
        // Handle tool approval resume - show what the subagent wants to execute
        if let Some(pi) = pause_info {
            // Header line
            let header = "Subagent wants to execute:";
            let header_padding = inner_width.saturating_sub(calculate_display_width(header));
            formatted_lines.push(Line::from(vec![
                Span::styled("│ ", Style::default().fg(border_color)),
                Span::styled(
                    header.to_string(),
                    Style::default()
                        .fg(ThemeColors::yellow())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::from(" ".repeat(header_padding)),
                Span::styled(" │", Style::default().fg(border_color)),
            ]));

            // Empty line
            formatted_lines.push(Line::from(vec![
                Span::styled("│", Style::default().fg(border_color)),
                Span::from(" ".repeat(inner_width + 2)),
                Span::styled("│", Style::default().fg(border_color)),
            ]));

            // Show pending tool calls
            if let Some(tool_calls) = &pi.pending_tool_calls {
                for tc in tool_calls {
                    // Tool name line
                    let tool_header = format!("  → {}", tc.name);
                    let tool_header_width = calculate_display_width(&tool_header);
                    let tool_header_padding = inner_width.saturating_sub(tool_header_width);

                    formatted_lines.push(Line::from(vec![
                        Span::styled("│ ", Style::default().fg(border_color)),
                        Span::styled("  → ", Style::default().fg(ThemeColors::magenta())),
                        Span::styled(
                            tc.name.clone(),
                            Style::default()
                                .fg(ThemeColors::cyan())
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::from(" ".repeat(tool_header_padding)),
                        Span::styled(" │", Style::default().fg(border_color)),
                    ]));

                    // Show arguments in a readable format
                    if !tc.arguments.is_null() {
                        let formatted_args =
                            format_tool_arguments_readable(&tc.arguments, inner_width - 6);
                        for arg_line in formatted_args {
                            let arg_display_width = calculate_display_width(&arg_line);
                            let arg_padding = inner_width.saturating_sub(arg_display_width + 4);

                            formatted_lines.push(Line::from(vec![
                                Span::styled("│ ", Style::default().fg(border_color)),
                                Span::from("    "),
                                Span::styled(
                                    arg_line,
                                    Style::default().fg(ThemeColors::dark_gray()),
                                ),
                                Span::from(" ".repeat(arg_padding)),
                                Span::styled(" │", Style::default().fg(border_color)),
                            ]));
                        }
                    }

                    // Empty line between tool calls
                    formatted_lines.push(Line::from(vec![
                        Span::styled("│", Style::default().fg(border_color)),
                        Span::from(" ".repeat(inner_width + 2)),
                        Span::styled("│", Style::default().fg(border_color)),
                    ]));
                }
            }
        } else {
            // approve_all but no pause_info cached
            let msg = "Approve all pending tool calls";
            let msg_padding = inner_width.saturating_sub(calculate_display_width(msg));
            formatted_lines.push(Line::from(vec![
                Span::styled("│ ", Style::default().fg(border_color)),
                Span::styled(msg.to_string(), Style::default().fg(ThemeColors::yellow())),
                Span::from(" ".repeat(msg_padding)),
                Span::styled(" │", Style::default().fg(border_color)),
            ]));

            // Empty line
            formatted_lines.push(Line::from(vec![
                Span::styled("│", Style::default().fg(border_color)),
                Span::from(" ".repeat(inner_width + 2)),
                Span::styled("│", Style::default().fg(border_color)),
            ]));
        }
    } else {
        // No pause info and no input - generic message
        let msg = "Resume subagent task";
        let msg_padding = inner_width.saturating_sub(calculate_display_width(msg));
        formatted_lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(border_color)),
            Span::styled(
                msg.to_string(),
                Style::default().fg(ThemeColors::dark_gray()),
            ),
            Span::from(" ".repeat(msg_padding)),
            Span::styled(" │", Style::default().fg(border_color)),
        ]));

        // Empty line
        formatted_lines.push(Line::from(vec![
            Span::styled("│", Style::default().fg(border_color)),
            Span::from(" ".repeat(inner_width + 2)),
            Span::styled("│", Style::default().fg(border_color)),
        ]));
    }

    // Bottom border
    let bottom_border = Line::from(vec![
        Span::styled("╰", Style::default().fg(border_color)),
        Span::styled(
            "─".repeat(inner_width + 2),
            Style::default().fg(border_color),
        ),
        Span::styled("╯", Style::default().fg(border_color)),
    ]);
    formatted_lines.push(bottom_border);

    // Add spacing marker
    formatted_lines.push(Line::from(vec![Span::from("SPACING_MARKER")]));

    formatted_lines
}

/// Wrap text to fit within a given width, respecting word boundaries
fn wrap_text_to_lines(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            if word.chars().count() > max_width {
                // Word is too long, truncate it
                let truncated: String = word.chars().take(max_width.saturating_sub(1)).collect();
                lines.push(format!("{}…", truncated));
            } else {
                current_line = word.to_string();
            }
        } else if current_line.chars().count() + 1 + word.chars().count() <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            if word.chars().count() > max_width {
                let truncated: String = word.chars().take(max_width.saturating_sub(1)).collect();
                lines.push(format!("{}…", truncated));
                current_line = String::new();
            } else {
                current_line = word.to_string();
            }
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    // Limit to 5 lines max
    if lines.len() > 5 {
        lines.truncate(4);
        lines.push("...".to_string());
    }

    lines
}

/// Format tool arguments in a readable way for display
fn format_tool_arguments_readable(args: &serde_json::Value, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();

    if let Some(obj) = args.as_object() {
        for (key, value) in obj {
            let value_str = match value {
                serde_json::Value::String(s) => {
                    // For long strings, truncate and show preview
                    let max_value_len = max_width.saturating_sub(key.len() + 4);
                    if s.chars().count() > max_value_len {
                        let truncated: String =
                            s.chars().take(max_value_len.saturating_sub(3)).collect();
                        format!("\"{}…\"", truncated)
                    } else {
                        format!("\"{}\"", s)
                    }
                }
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Array(arr) => {
                    if arr.is_empty() {
                        "[]".to_string()
                    } else {
                        format!("[{} items]", arr.len())
                    }
                }
                serde_json::Value::Object(_) => "{...}".to_string(),
                serde_json::Value::Null => "null".to_string(),
            };

            let line = format!("{}: {}", key, value_str);
            // Truncate if still too long (respecting char boundaries)
            if line.chars().count() > max_width {
                let truncated: String = line.chars().take(max_width.saturating_sub(1)).collect();
                lines.push(format!("{}…", truncated));
            } else {
                lines.push(line);
            }
        }
    } else {
        // Not an object, just show the raw value truncated
        let s = args.to_string();
        if s.chars().count() > max_width {
            let truncated: String = s.chars().take(max_width.saturating_sub(1)).collect();
            lines.push(format!("{}…", truncated));
        } else {
            lines.push(s);
        }
    }

    lines
}

/// Render a preview block showing tool calls being streamed/generated by the LLM.
/// Compact layout: shows a summary line with total count + total tokens,
/// plus individual tool rows (capped at MAX_VISIBLE_TOOLS to keep the block short).
pub fn render_tool_call_stream_block(
    infos: &[ToolCallStreamInfo],
    width: usize,
) -> Vec<Line<'static>> {
    let border_color = ThemeColors::dark_gray();
    // inner_width = usable content width between the border+padding chars
    // Each line is: "│ " + content(inner_width) + " │" = inner_width + 4
    let inner_width = if width > 4 { width - 4 } else { 40 };
    let horizontal_line = "─".repeat(inner_width + 2);
    let mut lines = Vec::new();

    lines.push(Line::from(vec![Span::from("SPACING_MARKER")]));

    // Title: "Generating N tool calls..."
    let title = if infos.len() == 1 {
        " Generating 1 tool call... ".to_string()
    } else {
        format!(" Generating {} tool calls... ", infos.len())
    };
    let title_display_width = UnicodeWidthStr::width(title.as_str());
    // Top border: "╭" + title + "─"×remaining + "╮" must equal inner_width + 4
    let remaining_dashes = (inner_width + 2).saturating_sub(title_display_width);
    lines.push(Line::from(vec![
        Span::styled("╭", Style::default().fg(border_color)),
        Span::styled(
            title,
            Style::default()
                .fg(ThemeColors::yellow())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{}╮", "─".repeat(remaining_dashes)),
            Style::default().fg(border_color),
        ),
    ]));

    // Each line structure: "│" + fill(fill_width) + " │"
    // Total = 1 + fill_width + 2 = width, so fill_width = width - 3
    let fill_width = width.saturating_sub(3);

    // Show individual tool rows, capped to keep the block compact
    const MAX_VISIBLE_TOOLS: usize = 5;
    let visible_count = infos.len().min(MAX_VISIBLE_TOOLS);

    for (i, info) in infos.iter().take(visible_count).enumerate() {
        let number = circled_number(i + 1);
        let base_name = if info.name.is_empty() {
            "streaming...".to_string()
        } else {
            format_tool_display_name(&info.name)
        };
        let tokens = if info.args_tokens > 0 {
            format_token_count(info.args_tokens)
        } else {
            "...".to_string()
        };

        // Append description if available, truncating to fit
        let number_width = UnicodeWidthStr::width(number.as_str());
        let base_name_width = UnicodeWidthStr::width(base_name.as_str());
        let tokens_width = UnicodeWidthStr::width(tokens.as_str());
        let fixed_used = 2 + number_width + 1 + base_name_width + tokens_width;

        let name = if let Some(desc) = &info.description {
            let sep = " - ";
            let sep_width = sep.len();
            // Need at least 4 chars for a meaningful truncated description ("X…")
            let available = fill_width.saturating_sub(fixed_used + sep_width + 1);
            if available >= 2 {
                let desc_display: String = desc.chars().take(available).collect();
                let truncated = if UnicodeWidthStr::width(desc_display.as_str())
                    < UnicodeWidthStr::width(desc.as_str())
                {
                    let trimmed: String = desc.chars().take(available.saturating_sub(1)).collect();
                    format!("{}…", trimmed)
                } else {
                    desc_display
                };
                format!("{}{}{}", base_name, sep, truncated)
            } else {
                base_name
            }
        } else {
            base_name
        };

        let name_width = UnicodeWidthStr::width(name.as_str());
        let used = 2 + number_width + 1 + name_width + tokens_width;
        let padding = fill_width.saturating_sub(used);

        lines.push(Line::from(vec![
            Span::styled("│", Style::default().fg(border_color)),
            Span::raw("  "),
            Span::styled(number, Style::default().fg(ThemeColors::orange())),
            Span::raw(" "),
            Span::styled(name, Style::default().fg(ThemeColors::title_primary())),
            Span::raw(" ".repeat(padding)),
            Span::styled(tokens, Style::default().fg(ThemeColors::muted())),
            Span::styled(" │", Style::default().fg(border_color)),
        ]));
    }

    // If there are more tools than we can show, add a summary row
    if infos.len() > MAX_VISIBLE_TOOLS {
        let hidden_count = infos.len() - MAX_VISIBLE_TOOLS;
        let hidden_tokens: usize = infos
            .iter()
            .skip(MAX_VISIBLE_TOOLS)
            .map(|i| i.args_tokens)
            .sum();
        let summary = format!(
            " +{} more{}",
            hidden_count,
            if hidden_tokens > 0 {
                format!(" ({})", format_token_count(hidden_tokens))
            } else {
                String::new()
            }
        );
        let summary_width = UnicodeWidthStr::width(summary.as_str());
        let padding = fill_width.saturating_sub(summary_width);

        lines.push(Line::from(vec![
            Span::styled("│", Style::default().fg(border_color)),
            Span::styled(summary, Style::default().fg(ThemeColors::dark_gray())),
            Span::raw(" ".repeat(padding)),
            Span::styled(" │", Style::default().fg(border_color)),
        ]));
    }

    // Total tokens line
    let total_tokens: usize = infos.iter().map(|i| i.args_tokens).sum();
    if total_tokens > 0 {
        let total_text = format!(" total: {}", format_token_count(total_tokens));
        let total_width = UnicodeWidthStr::width(total_text.as_str());
        let padding = fill_width.saturating_sub(total_width);

        lines.push(Line::from(vec![
            Span::styled("│", Style::default().fg(border_color)),
            Span::styled(total_text, Style::default().fg(ThemeColors::dark_gray())),
            Span::raw(" ".repeat(padding)),
            Span::styled(" │", Style::default().fg(border_color)),
        ]));
    }

    // Bottom border
    lines.push(Line::from(vec![Span::styled(
        format!("╰{}╯", horizontal_line),
        Style::default().fg(border_color),
    )]));

    lines.push(Line::from(vec![Span::from("SPACING_MARKER")]));
    lines
}

fn circled_number(n: usize) -> String {
    format!("{}", n)
}

fn format_tool_display_name(name: &str) -> String {
    let stripped = strip_tool_name(name);
    stripped
        .replace('_', " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

fn format_token_count(tokens: usize) -> String {
    if tokens >= 1000 {
        format!("{:.1}k tokens", tokens as f64 / 1000.0)
    } else {
        format!("{} tokens", tokens)
    }
}
