use super::*;

#[test]
fn test_wrap_text_by_word_preserves_newlines() {
    // Multi-line command should preserve explicit line breaks
    let input = "echo \"line 1\" \\\n  && echo \"line 2\" \\\n  && echo \"line 3\"";
    let result = wrap_text_by_word(input, 80);

    assert_eq!(result.len(), 3);
    assert_eq!(result[0], "echo \"line 1\" \\");
    assert_eq!(result[1], "  && echo \"line 2\" \\");
    assert_eq!(result[2], "  && echo \"line 3\"");
}

#[test]
fn test_wrap_text_by_word_empty_lines() {
    // Consecutive newlines should produce empty lines
    let input = "line 1\n\nline 3";
    let result = wrap_text_by_word(input, 80);

    assert_eq!(result.len(), 3);
    assert_eq!(result[0], "line 1");
    assert_eq!(result[1], "");
    assert_eq!(result[2], "line 3");
}

#[test]
fn test_wrap_text_by_word_wraps_long_lines() {
    // Long lines should still wrap at width boundary
    let input = "this is a very long line that should wrap";
    let result = wrap_text_by_word(input, 20);

    assert!(result.len() > 1);
    for line in &result {
        assert!(line.len() <= 20);
    }
}

#[test]
fn test_wrap_text_by_word_mixed_newlines_and_wrapping() {
    // Combine explicit newlines with width-based wrapping
    let input = "short\nthis is a longer line that needs wrapping\nend";
    let result = wrap_text_by_word(input, 20);

    // First line: "short"
    assert_eq!(result[0], "short");
    // Middle lines: wrapped version of the long line
    // Last line: "end"
    assert_eq!(result[result.len() - 1], "end");
    assert!(result.len() >= 3);
}

#[test]
fn test_wrap_text_by_word_single_line_no_newlines() {
    let input = "simple command";
    let result = wrap_text_by_word(input, 80);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0], "simple command");
}

#[test]
fn test_wrap_text_by_word_empty_input() {
    let result = wrap_text_by_word("", 80);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0], "");
}

#[test]
fn test_tool_call_stream_block_border_alignment() {
    use vac_foundation::models::integrations::openai::ToolCallStreamInfo;

    let infos = vec![
        ToolCallStreamInfo {
            name: "vac__create".to_string(),
            args_tokens: 3241,
            description: None,
        },
        ToolCallStreamInfo {
            name: "vac__run_command".to_string(),
            args_tokens: 412,
            description: None,
        },
        ToolCallStreamInfo {
            name: "".to_string(),
            args_tokens: 0,
            description: None,
        },
    ];

    let width = 80;
    let lines = render_tool_call_stream_block(&infos, width);

    // Check that all non-SPACING_MARKER lines have consistent display width
    for line in &lines {
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        if text == "SPACING_MARKER" {
            continue;
        }
        let display_width = UnicodeWidthStr::width(text.as_str());
        assert_eq!(
            display_width, width,
            "Line has wrong width {}: {:?}",
            display_width, text
        );
    }
}

#[test]
fn test_tool_call_stream_block_overflow_summary() {
    use vac_foundation::models::integrations::openai::ToolCallStreamInfo;

    let infos: Vec<ToolCallStreamInfo> = (0..8)
        .map(|i| ToolCallStreamInfo {
            name: format!("vac__tool_{}", i),
            args_tokens: 100 * (i + 1),
            description: None,
        })
        .collect();

    let width = 80;
    let lines = render_tool_call_stream_block(&infos, width);

    // Should have: SPACING + top border + 5 tool rows + 1 summary + 1 total + bottom border + SPACING = 11
    let content_lines: Vec<_> = lines
        .iter()
        .filter(|l| {
            let text: String = l.spans.iter().map(|s| s.content.as_ref()).collect();
            text != "SPACING_MARKER"
        })
        .collect();
    // top + 5 visible + 1 "+3 more" + 1 total + bottom = 9
    assert_eq!(content_lines.len(), 9);

    // Verify all lines have correct width
    for line in &content_lines {
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        let display_width = UnicodeWidthStr::width(text.as_str());
        assert_eq!(
            display_width, width,
            "Line has wrong width {}: {:?}",
            display_width, text
        );
    }
}
