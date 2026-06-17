use super::message::{extract_full_command_arguments, extract_truncated_command_arguments};
use crate::services::detect_term::{AdaptiveColors, ThemeColors};
use crate::services::file_diff::{render_file_diff_block, render_file_diff_block_from_args};
use crate::services::markdown_renderer::render_markdown_to_lines;
use crate::services::message::{BubbleColors, extract_command_purpose, get_command_type_name};
use ansi_to_tui::IntoText;
use console::strip_ansi_codes;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use regex::Regex;
use std::sync::OnceLock;
use unicode_width::UnicodeWidthStr;
use vac_foundation::models::integrations::openai::{
    ToolCall, ToolCallResult, ToolCallResultStatus, ToolCallStreamInfo,
};
use vac_foundation::utils::strip_tool_name;

#[allow(dead_code)]
pub enum ContentAlignment {
    Left,
    Center,
}

fn term_color(color: Color) -> Color {
    if crate::services::detect_term::should_use_rgb_colors() {
        color
    } else {
        Color::Reset
    }
}

/// Get theme-aware border color for tool call boxes
fn tool_border_color() -> Color {
    ThemeColors::accent()
}

/// Get theme-aware muted color (for secondary text, inactive elements)
fn tool_muted_color() -> Color {
    ThemeColors::muted()
}

pub fn strip_all_ansi(text: &str) -> String {
    // First pass: console crate (handles 95% of cases efficiently)
    let cleaned = console::strip_ansi_codes(text);

    // Second pass: catch the specific sequences console misses
    static REMAINING: OnceLock<Option<Regex>> = OnceLock::new();
    let maybe_regex = REMAINING.get_or_init(|| {
        Regex::new(concat!(
            r"\x1b\][0-9;]*[^\x07\x1b]*(\x07|\x1b\\)|", // Window titles and other OSC sequences
            r"\\u\{[0-9a-fA-F]+\}|",                    // Unicode escapes
            r"\x07"                                     // Bell
        ))
        .ok()
    });

    if let Some(regex) = maybe_regex {
        regex.replace_all(&cleaned, "").to_string()
    } else {
        cleaned.to_string()
    }
}
// Add this function to preprocess text and handle carriage returns
pub fn preprocess_terminal_output(text: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();
    let text = strip_all_ansi(text);
    let text = text.replace("\r\n", "\n");
    for ch in text.chars() {
        match ch {
            '\r' => {
                // Carriage return - start overwriting the current line
                current_line.clear();
            }
            '\n' => {
                // Newline - finish the current line and start a new one
                lines.push(current_line.clone());
                current_line.clear();
            }
            '\t' => {
                current_line.push_str("    ");
            }
            _ => {
                current_line.push(ch);
            }
        }
    }

    // Don't forget the last line if it doesn't end with newline
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    // Generic filtering: remove empty lines and lines that are just whitespace
    let filtered_lines: Vec<String> = lines
        .into_iter()
        .filter(|line| !line.trim().is_empty())
        .collect();

    // If we have no content after filtering, return the original to avoid losing everything
    if filtered_lines.is_empty() && !text.trim().is_empty() {
        return text.to_string();
    }
    filtered_lines.join("\n")
}

// Add this function to calculate actual display width accounting for ANSI and Unicode
fn calculate_display_width(text: &str) -> usize {
    // Strip ANSI codes first, then calculate Unicode width
    let stripped = strip_ansi_codes(text);
    stripped.width()
}

// Add this improved simple text wrapping function (character-based)
fn wrap_text_simple_unicode(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let stripped = strip_ansi_codes(text);
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for ch in stripped.chars() {
        let char_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);

        if current_width + char_width > width && !current_line.is_empty() {
            lines.push(current_line.clone());
            current_line.clear();
            current_width = 0;
        }

        current_line.push(ch);
        current_width += char_width;
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

// Word-based text wrapping function
fn wrap_text_by_word(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let stripped = strip_ansi_codes(text);
    let mut lines = Vec::new();

    // First split by newlines to preserve explicit line breaks
    for input_line in stripped.split('\n') {
        let mut current_line = String::new();
        let mut current_width = 0;

        // Handle empty lines from consecutive newlines
        if input_line.is_empty() {
            lines.push(String::new());
            continue;
        }

        for word in input_line.split_inclusive(|c: char| c.is_whitespace()) {
            let word_width = calculate_display_width(word);

            if current_width + word_width > width && !current_line.is_empty() {
                // Word doesn't fit, start new line
                lines.push(current_line.trim_end().to_string());
                current_line.clear();
                current_width = 0;
            }

            // If a single word is longer than width, we need to break it
            if word_width > width && current_line.is_empty() {
                // Break the long word by character
                let mut word_part = String::new();
                let mut part_width = 0;
                for ch in word.chars() {
                    let char_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
                    if part_width + char_width > width && !word_part.is_empty() {
                        lines.push(word_part);
                        word_part = String::new();
                        part_width = 0;
                    }
                    word_part.push(ch);
                    part_width += char_width;
                }
                if !word_part.is_empty() {
                    current_line = word_part;
                    current_width = part_width;
                }
            } else {
                current_line.push_str(word);
                current_width += word_width;
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line.trim_end().to_string());
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

// Helper function to wrap text while preserving ANSI codes
fn wrap_ansi_text(text: &str, width: usize) -> Vec<String> {
    // First preprocess to handle carriage returns and terminal control sequences
    let preprocessed = preprocess_terminal_output(text);

    // Convert to ratatui text first to parse ANSI codes
    let ratatui_text = match preprocessed.into_text() {
        Ok(parsed) => parsed,
        Err(_) => {
            // Fallback: just split by width using stripped text
            let stripped = strip_ansi_codes(&preprocessed);
            return wrap_text_simple_unicode(&stripped, width);
        }
    };

    let mut wrapped_lines = Vec::new();

    for line in ratatui_text.lines {
        if line.spans.is_empty() {
            wrapped_lines.push(String::new());
            continue;
        }

        let mut current_line = String::new();
        let mut current_width = 0;

        for span in line.spans {
            let span_text = &span.content;
            let span_display_width = calculate_display_width(span_text);

            if current_width + span_display_width <= width {
                // Span fits on current line
                current_line.push_str(span_text);
                current_width += span_display_width;
            } else if current_width == 0 {
                // Span is too long for a line by itself, so we must wrap it.
                let wrapped_span = wrap_text_simple_unicode(span_text, width);
                let num_wrapped = wrapped_span.len();
                if num_wrapped > 0 {
                    // Add all but the last part as full lines.
                    if let Some((last, elements)) = wrapped_span.split_last() {
                        for element in elements {
                            wrapped_lines.push(element.clone());
                        }
                        // The last part becomes the current line.
                        current_line = last.clone();
                        current_width = calculate_display_width(&current_line);
                    }
                }
            } else {
                // Start a new line
                wrapped_lines.push(current_line.clone());
                current_line = span_text.to_string();
                current_width = span_display_width;
            }
        }

        if !current_line.is_empty() || current_width > 0 {
            wrapped_lines.push(current_line);
        }
    }

    if wrapped_lines.is_empty() {
        wrapped_lines.push(String::new());
    }

    wrapped_lines
}

#[allow(clippy::too_many_arguments)]
pub fn render_styled_block_ansi_to_tui(
    content: &str,
    _outside_title: &str,
    bubble_title: &str,
    colors: Option<BubbleColors>,
    terminal_width: usize,
    _tool_type: &str,
    content_alignment: Option<ContentAlignment>,
) -> Vec<Line<'static>> {
    let content_width = if terminal_width > 4 {
        terminal_width - 4
    } else {
        40
    };

    let inner_width = content_width;
    let horizontal_line = "─".repeat(inner_width + 2);

    // Determine colors
    let (border_color, _title_color, content_color) = if let Some(ref c) = colors {
        (c.border_color, c.title_color, c.content_color)
    } else {
        (
            tool_border_color(),
            tool_border_color(),
            tool_border_color(),
        )
    };

    // Create colored borders
    let bottom_border = Line::from(vec![Span::styled(
        format!("╰{}╯", horizontal_line),
        Style::default().fg(border_color),
    )]);

    // Strip ANSI codes for title border calculation
    let stripped_title = strip_ansi_codes(bubble_title);
    let title_border = {
        let title_width = calculate_display_width(&stripped_title);
        if title_width <= inner_width {
            let remaining_dashes = inner_width + 2 - title_width;
            Line::from(vec![Span::styled(
                format!("╭{}{}", bubble_title, "─".repeat(remaining_dashes)) + "╮",
                Style::default().fg(border_color),
            )])
        } else {
            // Truncate based on display width, not character count
            let mut truncated_chars = String::new();
            let mut current_width = 0;
            for ch in stripped_title.chars() {
                let char_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
                if current_width + char_width <= inner_width {
                    truncated_chars.push(ch);
                    current_width += char_width;
                } else {
                    break;
                }
            }
            Line::from(vec![Span::styled(
                format!("╭{}─╮", truncated_chars),
                Style::default().fg(border_color),
            )])
        }
    };

    // Preprocess content to handle terminal control sequences
    let preprocessed_content = preprocess_terminal_output(content);

    // Convert ANSI content to ratatui Text
    let ratatui_text = preprocessed_content
        .into_text()
        .unwrap_or_else(|_| ratatui::text::Text::from(preprocessed_content.clone()));

    // Create lines with compact style similar to result blocks
    let mut formatted_lines = Vec::new();
    formatted_lines.push(title_border);

    // Use compact indentation similar to result blocks
    let line_indent = "  "; // 2 spaces like result blocks

    // Determine alignment
    let alignment = content_alignment.unwrap_or(ContentAlignment::Left);

    for text_line in ratatui_text.lines {
        if text_line.spans.is_empty() {
            // Empty line with border
            let line_spans = vec![
                Span::styled("│", Style::default().fg(border_color)),
                Span::from(format!(" {}", " ".repeat(inner_width))),
                Span::styled(" │", Style::default().fg(border_color)),
            ];
            formatted_lines.push(Line::from(line_spans));
            continue;
        }

        // Check if line needs wrapping
        let display_width: usize = text_line
            .spans
            .iter()
            .map(|span| calculate_display_width(&span.content))
            .sum();

        // Add compact indentation to content width calculation
        let content_display_width = display_width + line_indent.len();

        if content_display_width <= inner_width {
            // Line fits, add with compact style
            let padding_needed = inner_width - content_display_width;
            let (left_pad, right_pad) = match alignment {
                ContentAlignment::Left => (0, padding_needed),
                ContentAlignment::Center => {
                    (padding_needed / 2, padding_needed - (padding_needed / 2))
                }
            };
            let mut line_spans = vec![
                Span::styled("│", Style::default().fg(border_color)),
                Span::from(format!(" {}{}", line_indent, " ".repeat(left_pad))),
            ];
            for s in &text_line.spans {
                line_spans.push(Span::styled(
                    s.content.clone(),
                    Style::default().fg(content_color),
                ));
            }
            line_spans.push(Span::from(" ".repeat(right_pad)));
            line_spans.push(Span::styled(" │", Style::default().fg(border_color)));

            formatted_lines.push(Line::from(line_spans));
        } else {
            // Line needs wrapping - use available width minus indentation
            let available_for_content = inner_width - line_indent.len();
            let original_line: String = text_line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect();

            let wrapped_lines = wrap_ansi_text(&original_line, available_for_content);

            for wrapped_line in wrapped_lines {
                let wrapped_ratatui = wrapped_line
                    .clone()
                    .into_text()
                    .unwrap_or_else(|_| ratatui::text::Text::from(wrapped_line.clone()));

                if let Some(first_line) = wrapped_ratatui.lines.first() {
                    let wrapped_display_width: usize = first_line
                        .spans
                        .iter()
                        .map(|span| calculate_display_width(&span.content))
                        .sum();

                    let total_content_width = wrapped_display_width + line_indent.len();
                    let padding_needed = inner_width.saturating_sub(total_content_width);
                    let (left_pad, right_pad) = match alignment {
                        ContentAlignment::Left => (0, padding_needed),
                        ContentAlignment::Center => {
                            (padding_needed / 2, padding_needed - (padding_needed / 2))
                        }
                    };
                    let mut line_spans = vec![
                        Span::styled("│", Style::default().fg(border_color)),
                        Span::from(format!(" {}{}", line_indent, " ".repeat(left_pad))),
                    ];
                    for s in &first_line.spans {
                        line_spans.push(Span::styled(
                            s.content.clone(),
                            Style::default().fg(content_color),
                        ));
                    }
                    line_spans.push(Span::from(" ".repeat(right_pad)));
                    line_spans.push(Span::styled(" │", Style::default().fg(border_color)));

                    formatted_lines.push(Line::from(line_spans));
                }
            }
        }
    }

    formatted_lines.push(bottom_border);

    let mut owned_lines: Vec<Line<'static>> = Vec::new();
    owned_lines.push(Line::from(vec![Span::from("SPACING_MARKER")]));
    // Convert to owned lines for storage
    let final_lines: Vec<Line<'static>> = formatted_lines
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

    owned_lines.extend(final_lines);

    // add spaceing marker
    owned_lines.push(Line::from(vec![Span::from("SPACING_MARKER")]));
    owned_lines
}

/// Simple text formatting function that processes content and wraps it to fit terminal width
/// This is a stripped-down version of RenderCommandCollapsedResult without borders or styling
pub fn format_text_content(content: &str, terminal_width: usize) -> Vec<Line<'static>> {
    let content_width = if terminal_width > 4 {
        terminal_width - 4
    } else {
        40
    };

    let inner_width = content_width;

    // Preprocess content to handle terminal control sequences
    let preprocessed_content = preprocess_terminal_output(content);

    // Convert ANSI content to ratatui Text
    let ratatui_text = preprocessed_content
        .into_text()
        .unwrap_or_else(|_| ratatui::text::Text::from(preprocessed_content.clone()));

    let mut formatted_lines = Vec::new();

    for text_line in ratatui_text.lines {
        if text_line.spans.is_empty() {
            // Empty line
            formatted_lines.push(Line::from(vec![Span::from("")]));
            continue;
        }

        // Check if line needs wrapping
        let display_width: usize = text_line
            .spans
            .iter()
            .map(|span| calculate_display_width(&span.content))
            .sum();

        if display_width <= inner_width {
            // Line fits, add as-is
            let mut line_spans = Vec::new();
            for s in &text_line.spans {
                line_spans.push(Span::styled(
                    s.content.clone(),
                    Style::default().fg(Color::Reset),
                ));
            }
            formatted_lines.push(Line::from(line_spans));
        } else {
            // Line needs wrapping
            let original_line: String = text_line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect();

            let wrapped_lines = wrap_ansi_text(&original_line, inner_width);

            for wrapped_line in wrapped_lines {
                let wrapped_ratatui = wrapped_line
                    .clone()
                    .into_text()
                    .unwrap_or_else(|_| ratatui::text::Text::from(wrapped_line.clone()));

                if let Some(first_line) = wrapped_ratatui.lines.first() {
                    let mut line_spans = Vec::new();
                    for s in &first_line.spans {
                        line_spans.push(Span::styled(
                            s.content.clone(),
                            Style::default().fg(Color::Reset),
                        ));
                    }
                    formatted_lines.push(Line::from(line_spans));
                }
            }
        }
    }

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

pub fn extract_bash_block_info(
    tool_call: &ToolCall,
    output: &str,
) -> (String, String, String, BubbleColors) {
    let full_command = extract_full_command_arguments(tool_call);
    let command = if full_command == "unknown command" {
        output.to_string()
    } else {
        full_command
    };
    let outside_title = get_command_type_name(tool_call);
    let bubble_title = extract_command_purpose(&command, &outside_title);
    let colors = match strip_tool_name(&tool_call.function.name) {
        "create_file" => BubbleColors {
            border_color: ThemeColors::success(),
            title_color: tool_muted_color(),
            content_color: ThemeColors::text(),
            tool_type: "Create File".to_string(),
        },
        "edit_file" => BubbleColors {
            border_color: ThemeColors::warning(),
            title_color: tool_muted_color(),
            content_color: ThemeColors::text(),
            tool_type: "Edit File".to_string(),
        },
        "run_command" => BubbleColors {
            border_color: tool_border_color(),
            title_color: ThemeColors::warning(),
            content_color: ThemeColors::text(),
            tool_type: "Run Command".to_string(),
        },
        "run_remote_command" | "run_remote_command_task" => BubbleColors {
            border_color: tool_border_color(),
            title_color: ThemeColors::warning(),
            content_color: ThemeColors::text(),
            tool_type: "Run Remote Command".to_string(),
        },
        "read_file" => BubbleColors {
            border_color: ThemeColors::magenta(),
            title_color: tool_muted_color(),
            content_color: ThemeColors::text(),
            tool_type: "Read File".to_string(),
        },
        "delete_file" => BubbleColors {
            border_color: ThemeColors::danger(),
            title_color: tool_muted_color(),
            content_color: ThemeColors::text(),
            tool_type: "Delete File".to_string(),
        },
        "dynamic_subagent_task" => {
            let is_sandbox =
                serde_json::from_str::<serde_json::Value>(&tool_call.function.arguments)
                    .ok()
                    .and_then(|a| a.get("enable_sandbox").and_then(|v| v.as_bool()))
                    .unwrap_or(false);
            if is_sandbox {
                BubbleColors {
                    border_color: ThemeColors::success(),
                    title_color: ThemeColors::success(),
                    content_color: ThemeColors::text(),
                    tool_type: "Subagent [sandboxed]".to_string(),
                }
            } else {
                BubbleColors {
                    border_color: ThemeColors::magenta(),
                    title_color: ThemeColors::magenta(),
                    content_color: ThemeColors::text(),
                    tool_type: "Subagent".to_string(),
                }
            }
        }
        _ => BubbleColors {
            border_color: tool_border_color(),
            title_color: ThemeColors::text(),
            content_color: ThemeColors::text(),
            tool_type: "unknown".to_string(),
        },
    };
    (command, outside_title, bubble_title, colors)
}

/// Render a streaming block showing only the last 3 lines with a "ctrl+t to expand" hint
/// This is used for run_command tool calls that are actively streaming output
pub fn render_streaming_block_compact(
    content: &str,
    bubble_title: &str,
    colors: Option<BubbleColors>,
    terminal_width: usize,
) -> Vec<Line<'static>> {
    let content_width = if terminal_width > 4 {
        terminal_width - 4
    } else {
        40
    };

    let inner_width = content_width;
    let horizontal_line = "─".repeat(inner_width + 2);

    // Determine colors
    let (border_color, _title_color, content_color) = if let Some(ref c) = colors {
        (c.border_color, c.title_color, c.content_color)
    } else {
        (tool_muted_color(), tool_muted_color(), tool_muted_color())
    };

    // Create colored borders
    let bottom_border = Line::from(vec![Span::styled(
        format!("╰{}╯", horizontal_line),
        Style::default().fg(border_color),
    )]);

    // Strip ANSI codes for title border calculation
    let stripped_title = strip_ansi_codes(bubble_title);
    let title_border = {
        let title_width = calculate_display_width(&stripped_title);
        if title_width <= inner_width {
            let remaining_dashes = inner_width + 2 - title_width;
            Line::from(vec![Span::styled(
                format!("╭{}{}", bubble_title, "─".repeat(remaining_dashes)) + "╮",
                Style::default().fg(border_color),
            )])
        } else {
            let mut truncated_chars = String::new();
            let mut current_width = 0;
            for ch in stripped_title.chars() {
                let char_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
                if current_width + char_width <= inner_width {
                    truncated_chars.push(ch);
                    current_width += char_width;
                } else {
                    break;
                }
            }
            Line::from(vec![Span::styled(
                format!("╭{}─╮", truncated_chars),
                Style::default().fg(border_color),
            )])
        }
    };

    // Preprocess content to handle terminal control sequences
    let preprocessed_content = preprocess_terminal_output(content);

    // Split into lines and take only the last 3
    let all_content_lines: Vec<&str> = preprocessed_content.lines().collect();

    let content_joined_lines = all_content_lines.join("\n");
    let ratatui_text = content_joined_lines
        .clone()
        .into_text()
        .unwrap_or_else(|_| ratatui::text::Text::from(content_joined_lines.clone()));

    let mut formatted_lines = Vec::new();
    formatted_lines.push(title_border);

    let line_indent = "  ";

    for text_line in ratatui_text.lines {
        if text_line.spans.is_empty() {
            let line_spans = vec![
                Span::styled("│", Style::default().fg(border_color)),
                Span::from(format!(" {}", " ".repeat(inner_width))),
                Span::styled(" │", Style::default().fg(border_color)),
            ];
            formatted_lines.push(Line::from(line_spans));
            continue;
        }

        let display_width: usize = text_line
            .spans
            .iter()
            .map(|span| calculate_display_width(&span.content))
            .sum();

        let content_display_width = display_width + line_indent.len();

        if content_display_width <= inner_width {
            let padding_needed = inner_width - content_display_width;
            let mut line_spans = vec![
                Span::styled("│", Style::default().fg(border_color)),
                Span::from(format!(" {}", line_indent)),
            ];
            for s in &text_line.spans {
                line_spans.push(Span::styled(
                    s.content.clone(),
                    Style::default().fg(content_color),
                ));
            }
            line_spans.push(Span::from(" ".repeat(padding_needed)));
            line_spans.push(Span::styled(" │", Style::default().fg(border_color)));
            formatted_lines.push(Line::from(line_spans));
        } else {
            // Line needs wrapping - use available width minus indentation
            let available_for_content = inner_width - line_indent.len();
            let original_line: String = text_line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect();

            let wrapped_lines = wrap_ansi_text(&original_line, available_for_content);

            for wrapped_line in wrapped_lines {
                let wrapped_ratatui = wrapped_line
                    .clone()
                    .into_text()
                    .unwrap_or_else(|_| ratatui::text::Text::from(wrapped_line.clone()));

                if let Some(first_line) = wrapped_ratatui.lines.first() {
                    let wrapped_display_width: usize = first_line
                        .spans
                        .iter()
                        .map(|span| calculate_display_width(&span.content))
                        .sum();

                    let total_content_width = wrapped_display_width + line_indent.len();
                    let padding_needed = inner_width.saturating_sub(total_content_width);
                    let mut line_spans = vec![
                        Span::styled("│", Style::default().fg(border_color)),
                        Span::from(format!(" {}", line_indent)),
                    ];
                    for s in &first_line.spans {
                        line_spans.push(Span::styled(
                            s.content.clone(),
                            Style::default().fg(content_color),
                        ));
                    }
                    line_spans.push(Span::from(" ".repeat(padding_needed)));
                    line_spans.push(Span::styled(" │", Style::default().fg(border_color)));
                    formatted_lines.push(Line::from(line_spans));
                }
            }
        }
    }

    formatted_lines.push(bottom_border);

    let mut owned_lines: Vec<Line<'static>> = Vec::new();
    owned_lines.push(Line::from(vec![Span::from("SPACING_MARKER")]));

    let final_lines: Vec<Line<'static>> = formatted_lines
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

    owned_lines.extend(final_lines);
    owned_lines.push(Line::from(vec![Span::from("SPACING_MARKER")]));
    owned_lines
}

pub fn render_styled_header_and_borders(
    title: &str,
    content_lines: Vec<Line<'static>>,
    colors: Option<BubbleColors>,
    terminal_width: usize,
) -> Vec<Line<'static>> {
    let content_width = if terminal_width > 4 {
        terminal_width - 4
    } else {
        40
    };
    let inner_width = content_width;
    let horizontal_line = "─".repeat(inner_width + 2);

    let border_color = colors
        .map(|c| c.border_color)
        .unwrap_or(tool_border_color());

    // Create title border
    let stripped_title = strip_ansi_codes(title);
    let title_border = {
        let title_width = calculate_display_width(&stripped_title);
        if title_width <= inner_width {
            let remaining_dashes = inner_width + 2 - title_width;
            Line::from(vec![Span::styled(
                format!("╭{}{}╮", title, "─".repeat(remaining_dashes)),
                Style::default().fg(border_color),
            )])
        } else {
            let mut truncated_chars = String::new();
            let mut current_width = 0;
            for ch in stripped_title.chars() {
                let char_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
                if current_width + char_width <= inner_width {
                    truncated_chars.push(ch);
                    current_width += char_width;
                } else {
                    break;
                }
            }
            Line::from(vec![Span::styled(
                format!("╭{}─╮", truncated_chars),
                Style::default().fg(border_color),
            )])
        }
    };

    let bottom_border = Line::from(vec![Span::styled(
        format!("╰{}╯", horizontal_line),
        Style::default().fg(border_color),
    )]);

    let mut result = Vec::new();
    result.push(title_border);
    // Add side borders to each content line
    for line in content_lines {
        let mut bordered_line = Vec::new();
        bordered_line.push(Span::styled("│", Style::default().fg(border_color)));
        bordered_line.push(Span::from(" "));

        // Calculate content width and truncate if needed
        let mut total_width: usize = 0;
        let mut truncated_spans = Vec::new();

        for span in line.spans.iter() {
            let span_width = calculate_display_width(&span.content);
            if total_width + span_width <= inner_width {
                // Span fits completely
                truncated_spans.push(span.clone());
                total_width += span_width;
            } else if total_width < inner_width {
                // Need to truncate this span
                let remaining_width = inner_width - total_width;
                let mut truncated_content = String::new();
                let mut char_width = 0;
                for ch in span.content.chars() {
                    let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
                    if char_width + ch_width <= remaining_width {
                        truncated_content.push(ch);
                        char_width += ch_width;
                    } else {
                        break;
                    }
                }
                if !truncated_content.is_empty() {
                    truncated_spans.push(Span::styled(truncated_content, span.style));
                }
                total_width = inner_width;
                break; // No more content can fit
            }
            // else: already at or past inner_width, skip this span
        }

        // Add the truncated content spans
        bordered_line.extend(truncated_spans);

        // Add padding to fill the width
        let padding_needed = inner_width.saturating_sub(total_width);
        if padding_needed > 0 {
            bordered_line.push(Span::from(" ".repeat(padding_needed)));
        }

        bordered_line.push(Span::styled(" │", Style::default().fg(border_color)));
        result.push(Line::from(bordered_line));
    }
    result.push(bottom_border);
    result
}

/// Render file diff for full screen popup - shows diff lines with context
/// Uses the same diff-only approach as the TUI view for consistency
/// Returns None if there's no diff to show (e.g., old_str not found)
/// The `result` parameter can be provided to extract the starting line number from the diff output.
pub fn render_file_diff_full(
    tool_call: &ToolCall,
    terminal_width: usize,
    do_show: Option<bool>,
    result: Option<&str>,
) -> Option<Vec<Line<'static>>> {
    // Get diff lines - use the truncated version which starts from first change
    // but we'll show all diff lines without truncation for the full screen view
    let (_truncated_diff_lines, full_diff_lines) =
        render_file_diff_block_from_args(tool_call, terminal_width, result);

    let title: String = get_command_type_name(tool_call);

    // If diff is empty, return None to signal caller should use fallback rendering
    if full_diff_lines.is_empty() {
        return None;
    }

    if !do_show.unwrap_or(false) {
        return Some(Vec::new());
    }

    // render header dot - don't show path since it's already in the diff content header line
    let spacing_marker = Line::from(vec![Span::from("SPACING_MARKER")]);

    let mut result = vec![spacing_marker.clone()];
    result.extend(render_styled_header_with_dot(
        &title,
        "", // Hide path here - it's shown in the diff content below
        Some(LinesColors {
            dot: ThemeColors::dot_success(),
            title: ThemeColors::title_primary(),
            command: ThemeColors::text(),
            message: ThemeColors::success(),
        }),
        Some(terminal_width),
    ));
    result.push(spacing_marker.clone());
    result.extend(full_diff_lines);
    result.push(spacing_marker); // Add spacing marker at the end

    Some(result)
}

pub fn render_file_diff(tool_call: &ToolCall, terminal_width: usize) -> Vec<Line<'static>> {
    let tool_name = strip_tool_name(&tool_call.function.name);
    if tool_name == "str_replace" || tool_name == "create" {
        // Use full diff (not truncated) for pending approval blocks
        let (_, mut diff_lines) = render_file_diff_block(tool_call, terminal_width);
        // render header dot
        let spacing_marker = Line::from(vec![Span::from("SPACING_MARKER")]);
        if diff_lines.is_empty() {
            return Vec::new();
        }
        diff_lines = [
            vec![Line::from(vec![Span::from(" ")])],
            diff_lines,
            vec![Line::from(vec![Span::from(" ")])],
        ]
        .concat();

        let title = get_command_type_name(tool_call);

        let formatted_title = format!(" {} ", title);
        let colors = Some(BubbleColors {
            border_color: tool_muted_color(),
            title_color: ThemeColors::text(),
            content_color: ThemeColors::text(),
            tool_type: title,
        });

        let result =
            render_styled_header_and_borders(&formatted_title, diff_lines, colors, terminal_width);

        let adjusted_result = [
            vec![spacing_marker.clone()],
            result,
            vec![spacing_marker.clone()],
        ]
        .concat();

        return adjusted_result;
    }

    Vec::new()
}

pub fn render_bash_block(
    tool_call: &ToolCall,
    output: &str,
    _accepted: bool,
    terminal_width: usize,
    _is_auto_approved: bool,
) -> Vec<Line<'static>> {
    let (command, outside_title, bubble_title, colors) = extract_bash_block_info(tool_call, output);

    render_styled_block_ansi_to_tui(
        &command,
        &outside_title,
        &bubble_title,
        Some(colors.clone()),
        terminal_width,
        strip_tool_name(&tool_call.function.name),
        None,
    )
}

pub fn render_markdown_block(
    preprocessed_result: String,
    command_args: String,
    title: String,
) -> Vec<Line<'static>> {
    let processed_result = preprocess_terminal_output(&preprocessed_result);
    let mut lines = Vec::new();
    lines.push(Line::from(vec![Span::from("SPACING_MARKER")]));
    lines.extend(render_styled_header_with_dot(
        &title,
        &command_args,
        Some(LinesColors {
            dot: ThemeColors::dot_success(),
            title: ThemeColors::title_primary(),
            command: ThemeColors::text(),
            message: ThemeColors::success(),
        }),
        None, // No width available here
    ));
    lines.push(Line::from(vec![Span::from("SPACING_MARKER")]));
    let content_lines = render_markdown_to_lines(&processed_result.to_string()).unwrap_or_default();

    for line in content_lines {
        lines.push(line);
    }

    lines.push(Line::from(vec![Span::from("SPACING_MARKER")]));
    lines.push(Line::from(vec![Span::from("SPACING_MARKER")]));
    lines
}

/// Render str_replace/create results - clean diff view without borders
/// Uses the same approach as fullscreen popup for consistency
/// Returns None if there's no diff (fallback to standard result rendering)
/// The `result` parameter can be provided to extract the starting line number from the diff output.
pub fn render_diff_result_block(
    tool_call: &ToolCall,
    width: usize,
    result: Option<&str>,
) -> Option<Vec<Line<'static>>> {
    // Use the same clean diff rendering as the fullscreen popup
    render_file_diff_full(tool_call, width, Some(true), result)
}

pub fn render_result_block(tool_call_result: &ToolCallResult, width: usize) -> Vec<Line<'static>> {
    let tool_call = tool_call_result.call.clone();
    let result = tool_call_result.result.clone();
    let tool_call_status = tool_call_result.status.clone();

    let title: String = get_command_type_name(&tool_call);
    let command_args = extract_truncated_command_arguments(&tool_call, None);

    let is_collapsed = is_collapsed_tool_call(&tool_call);

    if tool_call_status == ToolCallResultStatus::Error {
        return render_bash_block_rejected(&command_args, &title, Some(result.to_string()), None);
    }
    if tool_call_status == ToolCallResultStatus::Cancelled {
        return render_bash_block_rejected(
            &command_args,
            &title,
            Some("Interrupted by user".to_string()),
            None,
        );
    }

    // Handle str_replace/create with diff-only content
    // If render_diff_result_block returns None (no diff), fall through to standard rendering
    let tool_name = strip_tool_name(&tool_call.function.name);
    if tool_name == "str_replace" || tool_name == "create" {
        // Check for rejected/cancelled in result text
        if result.contains("TOOL_CALL_REJECTED") {
            return render_bash_block_rejected(
                &command_args,
                &title,
                Some("Rejected by user".to_string()),
                None,
            );
        }
        if result.contains("TOOL_CALL_CANCELLED") {
            return render_bash_block_rejected(
                &command_args,
                &title,
                Some("Interrupted by user".to_string()),
                None,
            );
        }

        if let Some(diff_lines) = render_diff_result_block(&tool_call, width, Some(&result)) {
            return diff_lines;
        }
        // Fall through to standard result rendering if no diff
    }

    if command_args.contains(".md") && is_collapsed {
        return render_markdown_block(result.clone(), command_args.clone(), title.clone());
    }

    let terminal_width = width;
    let content_width = if terminal_width > 4 {
        terminal_width - 4
    } else {
        40
    };
    let inner_width = content_width;

    let mut lines = Vec::new();

    // Only add borders if not collapsed
    let horizontal_line = if !is_collapsed {
        "─".repeat(inner_width + 2)
    } else {
        String::new()
    };
    let top_border = if !is_collapsed {
        Line::from(vec![Span::styled(
            format!("╭{}╮", horizontal_line),
            Style::default().fg(term_color(Color::Gray)),
        )])
    } else {
        Line::from(vec![Span::from("")])
    };
    let bottom_border = if !is_collapsed {
        Line::from(vec![Span::styled(
            format!("╰{}╯", horizontal_line),
            Style::default().fg(term_color(Color::Gray)),
        )])
    } else {
        Line::from(vec![Span::from("")])
    };

    if !is_collapsed {
        lines.push(top_border);
    }

    // Header line with border - handle multi-line command arguments
    let title_with_args = format!("{} ({})", title, command_args);

    // Calculate available width for the title and arguments
    let available_width = inner_width - 2; // Account for borders and spacing
    let dot_color = if is_collapsed {
        ThemeColors::magenta()
    } else {
        ThemeColors::dot_success()
    };
    let title_color = if is_collapsed {
        ThemeColors::title()
    } else {
        ThemeColors::title_primary()
    };
    // Check if the title with arguments fits on one line
    // Also check for embedded newlines - if present, always use multi-line rendering
    let has_newlines = command_args.contains('\n');
    if !has_newlines && title_with_args.len() <= available_width {
        // Single line header
        let mut header_spans = vec![];

        if !is_collapsed {
            header_spans.push(Span::styled(
                "│",
                Style::default().fg(term_color(Color::Gray)),
            ));
            header_spans.push(Span::from(" "));
        }

        header_spans.push(Span::styled(
            "● ",
            Style::default().fg(dot_color).add_modifier(Modifier::BOLD),
        ));
        header_spans.push(Span::styled(
            title.to_string(),
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        ));
        header_spans.push(Span::styled(
            format!(" ({})", command_args),
            Style::default().fg(term_color(Color::Gray)),
        ));

        if !is_collapsed {
            let header_content_width = 2 + title_with_args.len();
            let header_padding = inner_width.saturating_sub(header_content_width);
            header_spans.push(Span::from(" ".repeat(header_padding)));
            header_spans.push(Span::styled(
                " │",
                Style::default().fg(term_color(Color::Gray)),
            ));
        }

        lines.push(Line::from(header_spans));
    } else {
        // Multi-line header - title on first line, arguments on subsequent lines
        let mut header_spans = vec![];

        if !is_collapsed {
            header_spans.push(Span::styled(
                "│",
                Style::default().fg(term_color(Color::Gray)),
            ));
            header_spans.push(Span::from(" "));
        }

        header_spans.push(Span::styled(
            "● ",
            Style::default().fg(dot_color).add_modifier(Modifier::BOLD),
        ));
        header_spans.push(Span::styled(
            title.to_string(),
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        ));

        if !is_collapsed {
            let title_content_width = 2 + title.len();
            let title_padding = inner_width.saturating_sub(title_content_width);
            header_spans.push(Span::from(" ".repeat(title_padding)));
            header_spans.push(Span::styled(
                " │",
                Style::default().fg(term_color(Color::Gray)),
            ));
        }

        lines.push(Line::from(header_spans));

        // Render command arguments exactly like content lines
        let line_indent = "  "; // 2 spaces for compact style

        // Wrap the command arguments
        let available_for_content = inner_width - line_indent.len();
        let wrapped_args = wrap_ansi_text(&command_args, available_for_content);

        for (i, wrapped_line) in wrapped_args.iter().enumerate() {
            let wrapped_ratatui = wrapped_line
                .clone()
                .into_text()
                .unwrap_or_else(|_| ratatui::text::Text::from(wrapped_line.clone()));

            if let Some(first_line) = wrapped_ratatui.lines.first() {
                let wrapped_display_width: usize = first_line
                    .spans
                    .iter()
                    .map(|span| calculate_display_width(&span.content))
                    .sum();

                let total_content_width = wrapped_display_width + line_indent.len();
                let padding_needed = inner_width.saturating_sub(total_content_width);
                let padding = " ".repeat(padding_needed);

                let mut line_spans = vec![];

                if !is_collapsed {
                    line_spans.push(Span::styled(
                        "│",
                        Style::default().fg(term_color(Color::Gray)),
                    ));
                    line_spans.push(Span::from(format!(" {}", line_indent)));
                } else {
                    line_spans.push(Span::from(line_indent));
                }

                // Add the argument content with parentheses
                if i == 0 {
                    // First line - start with opening parenthesis
                    if let Some(first_span) = first_line.spans.first() {
                        line_spans.push(Span::styled(
                            format!("{}", first_span.content),
                            Style::default().fg(term_color(Color::Gray)),
                        ));
                    }
                } else {
                    // Continuation lines - just the content
                    line_spans.extend(first_line.spans.clone());
                }

                line_spans.push(Span::from(padding));

                if !is_collapsed {
                    line_spans.push(Span::styled(
                        " │",
                        Style::default().fg(term_color(Color::Gray)),
                    ));
                }

                lines.push(Line::from(line_spans));
            }
        }

        // Close the parentheses on the last line
        if let Some(last_line) = lines.last_mut()
            && let Some(last_content_span) = last_line.spans.iter_mut().rev().find(|span| {
                span.style.fg == Some(ThemeColors::title_primary()) && !span.content.contains("│")
            })
        {
            last_content_span.content = format!("{}", last_content_span.content).into();
        }
    }
    if is_collapsed {
        lines.push(Line::from(vec![Span::from("SPACING_MARKER")]));
    }

    // Use compact indentation like bash blocks
    let line_indent = "  "; // 2 spaces for compact style

    let preprocessed_result: String = preprocess_terminal_output(&result);
    let result_text = ratatui::text::Text::from(preprocessed_result);

    for text_line in result_text.iter() {
        if text_line.spans.is_empty() {
            // Empty line with border
            let mut line_spans = vec![];
            if !is_collapsed {
                line_spans.push(Span::styled(
                    "│",
                    Style::default().fg(term_color(Color::Gray)),
                ));
                line_spans.push(Span::from(format!(" {}", " ".repeat(inner_width))));
                line_spans.push(Span::styled(
                    " │",
                    Style::default().fg(term_color(Color::Gray)),
                ));
            }
            lines.push(Line::from(line_spans));
            continue;
        }

        // Check if line fits within available width
        let display_width: usize = text_line
            .spans
            .iter()
            .map(|span| calculate_display_width(&span.content))
            .sum();

        let content_display_width = display_width + line_indent.len();

        if content_display_width <= inner_width {
            // Line fits, add with compact style and border
            let padding_needed = inner_width - content_display_width;
            let padding = " ".repeat(padding_needed);

            let mut line_spans = vec![];

            if !is_collapsed {
                line_spans.push(Span::styled(
                    "│",
                    Style::default().fg(term_color(Color::Gray)),
                ));
                line_spans.push(Span::from(format!(" {}", line_indent)));
            } else {
                line_spans.push(Span::from(line_indent));
            }

            // Apply theme-aware text color to result text
            for span in &text_line.spans {
                line_spans.push(Span::styled(
                    span.content.clone(),
                    Style::default().fg(ThemeColors::text()),
                ));
            }
            line_spans.push(Span::from(padding));

            if !is_collapsed {
                line_spans.push(Span::styled(
                    " │",
                    Style::default().fg(term_color(Color::Gray)),
                ));
            }

            lines.push(Line::from(line_spans));
        } else {
            // Line needs wrapping - use available width minus indentation
            let available_for_content = inner_width - line_indent.len();
            let original_line: String = text_line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect();

            let wrapped_lines = wrap_ansi_text(&original_line, available_for_content);

            for wrapped_line in wrapped_lines {
                let wrapped_ratatui = wrapped_line
                    .clone()
                    .into_text()
                    .unwrap_or_else(|_| ratatui::text::Text::from(wrapped_line.clone()));

                if let Some(first_line) = wrapped_ratatui.lines.first() {
                    let wrapped_display_width: usize = first_line
                        .spans
                        .iter()
                        .map(|span| calculate_display_width(&span.content))
                        .sum();

                    let total_content_width = wrapped_display_width + line_indent.len();
                    let padding_needed = inner_width.saturating_sub(total_content_width);
                    let padding = " ".repeat(padding_needed);

                    let mut line_spans = vec![];

                    if !is_collapsed {
                        line_spans.push(Span::styled(
                            "│",
                            Style::default().fg(term_color(Color::Gray)),
                        ));
                        line_spans.push(Span::from(format!(" {}", line_indent)));
                    } else {
                        line_spans.push(Span::from(line_indent));
                    }

                    line_spans.extend(first_line.spans.clone());
                    line_spans.push(Span::from(padding));

                    if !is_collapsed {
                        line_spans.push(Span::styled(
                            " │",
                            Style::default().fg(term_color(Color::Gray)),
                        ));
                    }

                    lines.push(Line::from(line_spans));
                }
            }
        }
    }

    if !is_collapsed {
        lines.push(bottom_border);
    }
    lines.push(Line::from(vec![Span::from("SPACING_MARKER")]));

    // Convert to owned lines
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

    owned_lines
}

// Function to render a rejected bash command (when user selects "No")
pub fn render_bash_block_rejected(
    command_name: &str,
    title: &str,
    message: Option<String>,
    color: Option<Color>,
) -> Vec<Line<'static>> {
    let colors = color.map(|c| LinesColors {
        dot: c,
        title: ThemeColors::title_primary(),
        command: ThemeColors::text(),
        message: c,
    });
    render_styled_lines(command_name, title, message, colors, None)
}

#[derive(Clone)]
pub struct LinesColors {
    pub dot: Color,
    pub title: Color,
    pub command: Color,
    pub message: Color,
}

/// Public version of render_styled_header_with_dot for use in message.rs
pub fn render_styled_header_with_dot_public(
    title: &str,
    command_name: &str,
    colors: Option<LinesColors>,
    terminal_width: Option<usize>,
) -> Vec<Line<'static>> {
    render_styled_header_with_dot(title, command_name, colors, terminal_width)
}

fn render_styled_header_with_dot(
    title: &str,
    command_name: &str,
    colors: Option<LinesColors>,
    terminal_width: Option<usize>,
) -> Vec<Line<'static>> {
    let colors = colors.unwrap_or(LinesColors {
        dot: ThemeColors::dot_error(),
        title: ThemeColors::title_primary(),
        command: ThemeColors::text(),
        message: ThemeColors::danger(),
    });

    // Use actual terminal width if provided, otherwise fall back to 100
    let max_line_width: usize = terminal_width.unwrap_or(100);
    // First line prefix: "● " (2) + title + " (" (2)
    let first_line_prefix_len: usize = 2 + title.chars().count() + 2;
    // Continuation line prefix: just some indentation (2 spaces)
    let continuation_indent = "  ";
    let continuation_prefix_len: usize = continuation_indent.len();

    // Calculate available width for command on first line
    let first_line_available = max_line_width.saturating_sub(first_line_prefix_len + 1); // +1 for closing paren

    // Wrap the command text
    let wrapped_lines = wrap_text_simple_unicode(command_name, first_line_available);

    let mut result_lines = Vec::new();

    if wrapped_lines.len() <= 1 {
        // Single line - command fits on one line with title
        let mut spans = vec![
            Span::styled(
                "● ",
                Style::default().fg(colors.dot).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                title.to_string(),
                Style::default()
                    .fg(colors.title)
                    .add_modifier(Modifier::BOLD),
            ),
        ];
        // Only add command in parentheses if it's not empty
        if !command_name.is_empty() {
            spans.push(Span::styled(
                format!(" ({})", command_name),
                Style::default().fg(colors.command),
            ));
        }
        result_lines.push(Line::from(spans));
    } else {
        // Multi-line - need to wrap
        // First line: title + start of command
        result_lines.push(Line::from(vec![
            Span::styled(
                "● ",
                Style::default().fg(colors.dot).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                title.to_string(),
                Style::default()
                    .fg(colors.title)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" ({}", wrapped_lines.first().unwrap_or(&String::new())),
                Style::default().fg(colors.command),
            ),
        ]));

        // Middle lines - use wider width since we only have the indent
        let continuation_available = max_line_width.saturating_sub(continuation_prefix_len);
        let remaining_text: String = wrapped_lines[1..].join(" ");
        let rewrapped = wrap_text_simple_unicode(&remaining_text, continuation_available);

        for (i, line) in rewrapped.iter().enumerate() {
            let is_last = i == rewrapped.len() - 1;
            let line_content = if is_last {
                format!("{})", line)
            } else {
                line.clone()
            };

            result_lines.push(Line::from(vec![
                Span::from(continuation_indent.to_string()),
                Span::styled(line_content, Style::default().fg(colors.command)),
            ]));
        }
    }

    result_lines
}

pub fn render_styled_lines(
    command_name: &str,
    title: &str,
    message: Option<String>,
    colors: Option<LinesColors>,
    terminal_width: Option<usize>,
) -> Vec<Line<'static>> {
    render_styled_lines_with_content(command_name, title, None, message, colors, terminal_width)
}

pub fn render_styled_lines_with_content(
    command_name: &str,
    title: &str,
    content: Option<Vec<Line<'static>>>,
    message: Option<String>,
    colors: Option<LinesColors>,
    terminal_width: Option<usize>,
) -> Vec<Line<'static>> {
    let colors = colors.unwrap_or(LinesColors {
        dot: ThemeColors::dot_error(),
        title: ThemeColors::title_primary(),
        command: ThemeColors::text(),
        message: ThemeColors::danger(),
    });

    let mut lines = Vec::new();

    lines.push(Line::from(vec![Span::from("SPACING_MARKER")]));

    // Always use single-line truncated header - command stays on same line as title
    lines.extend(render_styled_header_with_dot(
        title,
        command_name,
        Some(colors.clone()),
        terminal_width,
    ));

    // Render optional content lines between header and message
    if let Some(content_lines) = content {
        for content_line in content_lines {
            // Build spans with indentation prefix and DarkGray color
            // Strip leading whitespace from each span and add consistent 3-space indent
            let mut styled_spans: Vec<Span<'static>> = Vec::new();
            styled_spans.push(Span::styled("  ", Style::default())); // 3-space indent
            for span in content_line.spans.into_iter() {
                let trimmed = span.content.trim_start().to_string();
                styled_spans.push(Span::styled(
                    trimmed,
                    Style::default().fg(ThemeColors::dark_gray()),
                ));
            }
            lines.push(Line::from(styled_spans));
        }
    }

    let message = message.unwrap_or("No (tell VAC what to do differently)".to_string());
    let message = preprocess_terminal_output(&message);

    // Handle multi-line error messages
    for (i, line) in message.lines().enumerate() {
        let prefix = if i == 0 { "  L " } else { "    " };
        lines.push(Line::from(vec![Span::styled(
            format!("{}{}", prefix, line),
            Style::default()
                .fg(colors.message)
                .add_modifier(Modifier::BOLD),
        )]));
    }

    lines.push(Line::from(vec![Span::from("SPACING_MARKER")]));

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

    owned_lines
}

pub fn render_refreshed_terminal_bubble(
    title: &str,
    content: &[Line<'static>],
    colors: Option<BubbleColors>,
    terminal_width: usize,
) -> Vec<Line<'static>> {
    render_styled_header_and_borders(title, content.to_vec(), colors, terminal_width)
}

pub fn is_collapsed_tool_call(tool_call: &ToolCall) -> bool {
    let tool_name = strip_tool_name(&tool_call.function.name);
    if matches!(tool_name, "run_command_task" | "run_remote_command_task") {
        return false;
    }
    true
}

pub fn render_collapsed_command_message(
    tool_call_result: &ToolCallResult,
    lines: Vec<Line<'static>>,
    terminal_width: usize,
) -> Vec<Line<'static>> {
    let result = tool_call_result.result.clone();
    let command_args = extract_truncated_command_arguments(&tool_call_result.call, None);
    let title = get_command_type_name(&tool_call_result.call);

    let message = format!("Read {} lines (ctrl+t to expand)", result.lines().count());
    let colors = LinesColors {
        dot: ThemeColors::dot_success(),
        title: ThemeColors::title_primary(),
        command: ThemeColors::text(),
        message: ThemeColors::text(),
    };

    // if lines are more than 3 lines get the last 3 lines
    let lines = if lines.len() > 3 {
        lines[lines.len() - 3..].to_vec()
    } else {
        lines
    };

    render_styled_lines_with_content(
        &command_args,
        &title,
        Some(lines),
        Some(message),
        Some(colors),
        Some(terminal_width),
    )
}

/// Renders a compact view file result block with borders
/// Format: View path/to/file.rs - 123 lines [grep: pattern] [glob: pattern]
pub fn render_view_file_block(
    file_path: &str,
    total_lines: usize,
    terminal_width: usize,
    grep: Option<&str>,
    glob: Option<&str>,
) -> Vec<Line<'static>> {
    let content_width = if terminal_width > 4 {
        terminal_width - 4
    } else {
        40
    };
    let inner_width = content_width;

    let border_color = tool_muted_color();
    let icon = "";

    // Determine title based on operation type
    let title = if grep.is_some() && glob.is_some() {
        "Grep+Glob"
    } else if grep.is_some() {
        "Grep"
    } else if glob.is_some() {
        "Glob"
    } else {
        "View"
    };

    let lines_text = format!("- {} lines", total_lines);

    // Build optional grep/glob suffix
    let suffix = match (grep, glob) {
        (Some(g), Some(gl)) => format!(" [grep: {} | glob: {}]", g, gl),
        (Some(g), None) => format!(" [grep: {}]", g),
        (None, Some(g)) => format!(" [glob: {}]", g),
        _ => String::new(),
    };

    // Calculate display widths
    let icon_width = calculate_display_width(icon);
    let title_width = calculate_display_width(title);
    let path_width = calculate_display_width(file_path);
    let lines_text_width = calculate_display_width(&lines_text);
    let suffix_width = calculate_display_width(&suffix);

    // Total content: icon + " " + title + " " + path + " " + lines_text + suffix
    let total_content_width =
        icon_width + 1 + title_width + 1 + path_width + 1 + lines_text_width + suffix_width;

    // Check if we need to truncate the path
    let (display_path, display_path_width) = if total_content_width > inner_width {
        // Need to truncate path
        let available_for_path = inner_width.saturating_sub(
            icon_width + 1 + title_width + 1 + 1 + lines_text_width + suffix_width + 3,
        ); // 3 for "..."
        let truncated = truncate_path_to_width(file_path, available_for_path);
        let w = calculate_display_width(&truncated);
        (truncated, w)
    } else {
        (file_path.to_string(), path_width)
    };

    let actual_content_width =
        icon_width + 1 + title_width + 1 + display_path_width + 1 + lines_text_width + suffix_width;
    let padding = inner_width.saturating_sub(actual_content_width);

    let mut spans = vec![
        Span::styled("│", Style::default().fg(border_color)),
        Span::from(" "),
        Span::styled(icon.to_string(), Style::default().fg(tool_muted_color())),
        Span::from(" "),
        Span::styled(
            title.to_string(),
            Style::default()
                .fg(ThemeColors::text())
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" "),
        Span::styled(display_path, Style::default().fg(ThemeColors::text())),
        Span::from(" "),
        Span::styled(lines_text, Style::default().fg(tool_muted_color())),
    ];

    if !suffix.is_empty() {
        spans.push(Span::styled(
            suffix,
            Style::default().fg(tool_border_color()),
        ));
    }

    spans.push(Span::from(" ".repeat(padding)));
    spans.push(Span::styled(" │", Style::default().fg(border_color)));

    let content_line = Line::from(spans);

    let horizontal_line = "─".repeat(inner_width + 2);
    let top_border = Line::from(vec![Span::styled(
        format!("╭{}╮", horizontal_line),
        Style::default().fg(border_color),
    )]);
    let bottom_border = Line::from(vec![Span::styled(
        format!("╰{}╯", horizontal_line),
        Style::default().fg(border_color),
    )]);

    vec![top_border, content_line, bottom_border]
}

/// Renders a compact view file block without borders (for full screen popup)
/// Format: Stack View path/to/file.rs - 123 lines [grep: pattern] [glob: pattern]
pub fn render_view_file_block_no_border(
    file_path: &str,
    total_lines: usize,
    terminal_width: usize,
    grep: Option<&str>,
    glob: Option<&str>,
) -> Vec<Line<'static>> {
    let content_width = if terminal_width > 2 {
        terminal_width - 2
    } else {
        40
    };

    let icon = "";

    // Determine title based on operation type
    let title = if grep.is_some() && glob.is_some() {
        "Grep+Glob"
    } else if grep.is_some() {
        "Grep"
    } else if glob.is_some() {
        "Glob"
    } else {
        "View"
    };

    let lines_text = format!("- {} lines", total_lines);

    // Build optional grep/glob suffix
    let suffix = match (grep, glob) {
        (Some(g), Some(gl)) => format!(" [grep: {} | glob: {}]", g, gl),
        (Some(g), None) => format!(" [grep: {}]", g),
        (None, Some(g)) => format!(" [glob: {}]", g),
        _ => String::new(),
    };

    // Calculate display widths
    let icon_width = calculate_display_width(icon);
    let title_width = calculate_display_width(title);
    let path_width = calculate_display_width(file_path);
    let lines_text_width = calculate_display_width(&lines_text);
    let suffix_width = calculate_display_width(&suffix);

    // Total content: icon + " " + title + " " + path + " " + lines_text + suffix
    let total_content_width =
        icon_width + 1 + title_width + 1 + path_width + 1 + lines_text_width + suffix_width;

    // Check if we need to truncate the path
    let (display_path, _display_path_width) = if total_content_width > content_width {
        // Need to truncate path
        let available_for_path = content_width.saturating_sub(
            icon_width + 1 + title_width + 1 + 1 + lines_text_width + suffix_width + 3,
        ); // 3 for "..."
        let truncated = truncate_path_to_width(file_path, available_for_path);
        let w = calculate_display_width(&truncated);
        (truncated, w)
    } else {
        (file_path.to_string(), path_width)
    };

    let mut spans = vec![
        Span::styled(icon.to_string(), Style::default().fg(tool_muted_color())),
        Span::from(" "),
        Span::styled(
            title.to_string(),
            Style::default()
                .fg(ThemeColors::text())
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" "),
        Span::styled(display_path, Style::default().fg(ThemeColors::text())),
        Span::from(" "),
        Span::styled(lines_text, Style::default().fg(tool_muted_color())),
    ];

    if !suffix.is_empty() {
        spans.push(Span::styled(
            suffix,
            Style::default().fg(tool_border_color()),
        ));
    }

    let content_line = Line::from(spans);

    vec![content_line]
}

/// Truncate a file path to fit within a given display width
fn truncate_path_to_width(path: &str, max_width: usize) -> String {
    let path_width = calculate_display_width(path);
    if path_width <= max_width {
        return path.to_string();
    }

    // Try to show ".../" + filename
    if let Some(file_name) = path.rsplit('/').next() {
        let file_name_width = calculate_display_width(file_name);
        if file_name_width + 4 <= max_width {
            // ".../" + filename fits
            return format!(".../{}", file_name);
        }
    }

    // Fall back to truncating from the start
    let mut result = String::new();
    let mut current_width = 3; // For "..."

    for ch in path.chars().rev() {
        let char_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        if current_width + char_width > max_width {
            break;
        }
        result.insert(0, ch);
        current_width += char_width;
    }

    format!("...{}", result)
}

/// State for the unified run command block
mod runtime_blocks;
pub use runtime_blocks::{
    RunCommandState, render_ask_user_block, render_run_command_block,
    render_subagent_resume_pending_block, render_task_wait_block, render_tool_call_stream_block,
};

#[cfg(test)]
mod tests;
