// Live ratatui adapter for the operator-console render model.
//
// `operator_ui` owns deterministic panel geometry and semantic spans. This
// adapter applies foreground emphasis to precomputed boxes, tabs, gauges, and
// columns without reflowing the model; visual gates now assert structural
// geometry in addition to plain snapshot content.

use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;

use crate::operator_style::OperatorLineSpec;
use crate::operator_style::OperatorStyleRole;
use crate::operator_style::classify_operator_line;

pub(crate) fn style_operator_lines_from_specs(lines: Vec<OperatorLineSpec>) -> Vec<Line<'static>> {
    lines
        .into_iter()
        .map(style_operator_line_from_spec)
        .collect()
}

pub(crate) fn style_operator_line_from_spec(line: OperatorLineSpec) -> Line<'static> {
    Line::from(
        line.spans
            .into_iter()
            .map(|span| Span::styled(span.text, ratatui_style_for_role(span.role)))
            .collect::<Vec<_>>(),
    )
}

/// Backward-compatible adapter for legacy callers. New operator-console
/// surfaces should prefer `style_operator_lines_from_specs`, which carries
/// semantic roles from the renderer instead of inferring them from text.
pub(crate) fn style_operator_lines_from_strings(lines: Vec<String>) -> Vec<Line<'static>> {
    lines
        .into_iter()
        .map(|line| style_operator_line_from_string(line))
        .collect()
}

pub(crate) fn style_operator_line_from_string(line: String) -> Line<'static> {
    let role = classify_operator_line(&line);
    Line::from(Span::styled(line, ratatui_style_for_role(role)))
}

fn ratatui_style_for_role(role: OperatorStyleRole) -> Style {
    match role {
        OperatorStyleRole::Plain => Style::default(),
        OperatorStyleRole::Chrome => Style::default().fg(Color::DarkGray),
        OperatorStyleRole::Muted => Style::default().fg(Color::DarkGray),
        OperatorStyleRole::Accent => Style::default().fg(Color::Cyan),
        OperatorStyleRole::Success => Style::default().fg(Color::Green),
        OperatorStyleRole::Warning => Style::default().fg(Color::Yellow),
        OperatorStyleRole::Danger => Style::default()
            .fg(Color::LightRed)
            .add_modifier(Modifier::BOLD),
        OperatorStyleRole::User => Style::default().fg(Color::LightBlue),
        OperatorStyleRole::Agent => Style::default().fg(Color::White),
        OperatorStyleRole::Status => Style::default().fg(Color::Gray),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_preserves_semantic_spans_without_keyword_classification() {
        let input = vec![OperatorLineSpec::from_spans(vec![
            crate::operator_style::OperatorSpanSpec::new(OperatorStyleRole::Warning, "approval "),
            crate::operator_style::OperatorSpanSpec::new(OperatorStyleRole::Danger, "DESTRUCTIVE"),
        ])];
        let rendered = style_operator_lines_from_specs(input);
        assert_eq!(rendered.len(), 1);
        assert_eq!(rendered[0].spans.len(), 2);
        let text = rendered[0]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert_eq!(text, "approval DESTRUCTIVE");
    }

    #[test]
    fn adapter_preserves_line_count_and_text() {
        let input = vec![
            "approval required".to_string(),
            "DESTRUCTIVE bash".to_string(),
            "[VIL-native] ready".to_string(),
        ];
        let rendered = style_operator_lines_from_strings(input.clone());
        assert_eq!(rendered.len(), input.len());
        for (line, expected) in rendered.iter().zip(input.iter()) {
            let text = line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>();
            assert_eq!(text, *expected);
        }
    }
}
