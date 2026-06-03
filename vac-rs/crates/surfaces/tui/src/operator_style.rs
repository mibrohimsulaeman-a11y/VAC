// Semantic style tokens for the VAC operator console.
//
// This module is dependency-free on purpose. It gives the source-controlled
// snapshot harness a stable ANSI vocabulary, while the live ratatui adapter can
// map the same semantic roles to terminal `Style`s. The contract is semantic,
// not theme-specific: UI code asks for `Danger`, `Warning`, `Accent`, etc. and
// never hard-codes screenshot colors in business logic.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OperatorStyleRole {
    Plain,
    Chrome,
    Muted,
    Accent,
    Success,
    Warning,
    Danger,
    User,
    Agent,
    Status,
}

impl OperatorStyleRole {
    pub(crate) const fn token(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Chrome => "chrome",
            Self::Muted => "muted",
            Self::Accent => "accent",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Danger => "danger",
            Self::User => "user",
            Self::Agent => "agent",
            Self::Status => "status",
        }
    }

    pub(crate) const fn ansi_open(self) -> &'static str {
        match self {
            Self::Plain => "",
            Self::Chrome => "\x1b[38;5;245m",
            Self::Muted => "\x1b[38;5;240m",
            Self::Accent => "\x1b[38;5;45m",
            Self::Success => "\x1b[38;5;42m",
            Self::Warning => "\x1b[38;5;214m",
            Self::Danger => "\x1b[38;5;203;1m",
            Self::User => "\x1b[38;5;111m",
            Self::Agent => "\x1b[38;5;250m",
            Self::Status => "\x1b[38;5;117m",
        }
    }
}

pub(crate) const ANSI_RESET: &str = "\x1b[0m";

pub(crate) const STYLE_ROLE_ORDER: [OperatorStyleRole; 10] = [
    OperatorStyleRole::Plain,
    OperatorStyleRole::Chrome,
    OperatorStyleRole::Muted,
    OperatorStyleRole::Accent,
    OperatorStyleRole::Success,
    OperatorStyleRole::Warning,
    OperatorStyleRole::Danger,
    OperatorStyleRole::User,
    OperatorStyleRole::Agent,
    OperatorStyleRole::Status,
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OperatorSpanSpec {
    pub(crate) role: OperatorStyleRole,
    pub(crate) text: String,
}

impl OperatorSpanSpec {
    pub(crate) fn new(role: OperatorStyleRole, text: impl Into<String>) -> Self {
        Self {
            role,
            text: text.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OperatorLineSpec {
    pub(crate) spans: Vec<OperatorSpanSpec>,
}

impl OperatorLineSpec {
    #[allow(dead_code)]
    pub(crate) fn plain(text: impl Into<String>) -> Self {
        Self::styled(OperatorStyleRole::Plain, text)
    }

    pub(crate) fn styled(role: OperatorStyleRole, text: impl Into<String>) -> Self {
        Self {
            spans: vec![OperatorSpanSpec::new(role, text)],
        }
    }

    pub(crate) fn from_spans(spans: Vec<OperatorSpanSpec>) -> Self {
        Self { spans }
    }

    pub(crate) fn text(&self) -> String {
        self.spans
            .iter()
            .map(|span| span.text.as_str())
            .collect::<String>()
    }
}

pub(crate) fn style_segment(role: OperatorStyleRole, text: &str) -> String {
    if text.is_empty() {
        String::new()
    } else if role == OperatorStyleRole::Plain {
        text.to_string()
    } else {
        format!("{}{}{}", role.ansi_open(), text, ANSI_RESET)
    }
}

pub(crate) fn style_operator_line_spec(line: &OperatorLineSpec) -> String {
    line.spans
        .iter()
        .map(|span| style_segment(span.role, span.text.as_str()))
        .collect::<String>()
}

pub(crate) fn operator_line_specs_to_plain_text(lines: &[OperatorLineSpec]) -> String {
    lines
        .iter()
        .map(OperatorLineSpec::text)
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn style_operator_text_specs(lines: &[OperatorLineSpec]) -> String {
    lines
        .iter()
        .map(style_operator_line_spec)
        .collect::<Vec<_>>()
        .join("\n")
}

#[allow(dead_code)]
pub(crate) fn style_operator_line(line: &str) -> String {
    let role = classify_operator_line(line);
    style_segment(role, line)
}

#[allow(dead_code)]
pub(crate) fn style_operator_text(text: &str) -> String {
    text.lines()
        .map(style_operator_line)
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn classify_operator_line(line: &str) -> OperatorStyleRole {
    let trimmed = line.trim_start();
    let lower = trimmed.to_ascii_lowercase();

    if trimmed.starts_with("● ● ●")
        || trimmed.starts_with("────")
        || lower.starts_with("chat")
        || lower.starts_with("composer")
        || lower.contains("│ model ")
        || lower.contains("profile ")
    {
        return OperatorStyleRole::Chrome;
    }

    if lower.contains("destructive") || lower.contains("danger") || lower.contains("failed") {
        return OperatorStyleRole::Danger;
    }

    if lower.contains("approval required")
        || lower.contains("warning")
        || lower.contains("risk")
        || lower.contains("policy")
        || lower.contains("approval")
    {
        return OperatorStyleRole::Warning;
    }

    if lower.contains("ready")
        || lower.contains("passed")
        || lower.contains("valid 100%")
        || lower.contains("vil-native")
    {
        return OperatorStyleRole::Success;
    }

    if lower.contains("yaml diagnostic")
        || lower.contains("diagnostic")
        || lower.contains("runtime jobs")
        || lower.contains("capability dashboard")
        || lower.contains("autopilot")
        || lower.contains("context")
        || lower.contains("tool timeline")
    {
        return OperatorStyleRole::Accent;
    }

    if lower.starts_with("user") || lower.contains(" user ") {
        return OperatorStyleRole::User;
    }

    if lower.starts_with("agent") || lower.contains("thinking") || lower.contains("assistant") {
        return OperatorStyleRole::Agent;
    }

    if trimmed.is_empty() || lower.contains("queued") || lower.contains("omitted") {
        return OperatorStyleRole::Muted;
    }

    OperatorStyleRole::Status
}

pub(crate) fn strip_ansi(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if ('@'..='~').contains(&next) {
                    break;
                }
            }
            continue;
        }
        output.push(ch);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_role_has_stable_token_and_ansi_sequence() {
        for role in STYLE_ROLE_ORDER {
            assert!(!role.token().is_empty());
            if role == OperatorStyleRole::Plain {
                assert!(role.ansi_open().is_empty());
            } else {
                assert!(role.ansi_open().starts_with("\x1b["));
                assert!(role.ansi_open().ends_with('m'));
            }
        }
    }

    #[test]
    fn strips_ansi_back_to_plain_text() {
        let plain = "approval required — DESTRUCTIVE";
        let styled = style_segment(OperatorStyleRole::Danger, plain);
        assert_ne!(styled, plain);
        assert_eq!(strip_ansi(&styled), plain);
    }

    #[test]
    fn destructive_lines_are_danger() {
        assert_eq!(
            classify_operator_line("│ DESTRUCTIVE bash rm -rf target"),
            OperatorStyleRole::Danger
        );
    }

    #[test]
    fn approval_lines_are_warning() {
        assert_eq!(
            classify_operator_line("approval required · policy reason"),
            OperatorStyleRole::Warning
        );
    }

    #[test]
    fn vil_native_ready_lines_are_success() {
        assert_eq!(
            classify_operator_line("[VIL-native] ready — engine connected"),
            OperatorStyleRole::Success
        );
    }

    #[test]
    fn semantic_line_spec_round_trips_plain_and_ansi_text() {
        let line = OperatorLineSpec::from_spans(vec![
            OperatorSpanSpec::new(OperatorStyleRole::Warning, "approval"),
            OperatorSpanSpec::new(OperatorStyleRole::Plain, " "),
            OperatorSpanSpec::new(OperatorStyleRole::Danger, "DESTRUCTIVE"),
        ]);
        assert_eq!(line.text(), "approval DESTRUCTIVE");
        let styled = style_operator_line_spec(&line);
        assert_ne!(styled, line.text());
        assert_eq!(strip_ansi(&styled), line.text());
    }

    #[test]
    fn spec_collection_can_emit_plain_and_ansi_documents() {
        let lines = vec![
            OperatorLineSpec::styled(OperatorStyleRole::Chrome, "chrome"),
            OperatorLineSpec::styled(OperatorStyleRole::Success, "ready"),
        ];
        let plain = operator_line_specs_to_plain_text(&lines);
        let ansi = style_operator_text_specs(&lines);
        assert_eq!(plain, "chrome\nready");
        assert_ne!(ansi, plain);
        assert_eq!(strip_ansi(&ansi), plain);
    }

    #[test]
    fn strip_ansi_handles_multiple_segments() {
        let combined = format!(
            "{} {}",
            style_segment(OperatorStyleRole::Accent, "runtime jobs"),
            style_segment(OperatorStyleRole::Success, "ready")
        );
        assert_eq!(strip_ansi(&combined), "runtime jobs ready");
    }
}
