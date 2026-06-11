use crate::operator::{OperatorSnapshot, chrome, components};

pub fn render_lines(
    snapshot: &OperatorSnapshot,
    user_prompt: &str,
    assistant_text: &str,
) -> Vec<String> {
    let mut lines = vec![
        chrome::top_bar(snapshot, "agent working"),
        format!("you {user_prompt}"),
        format!("vac {}", assistant_text),
        "— tool timeline —".into(),
    ];
    for item in &snapshot.tool_timeline {
        lines.push(format!(
            "• {} {} {}",
            item.name,
            item.target,
            item.detail.as_deref().unwrap_or(&item.state)
        ));
    }
    if snapshot.tool_timeline.is_empty() {
        lines.push("no tools have run yet".into());
    }
    lines.push(format!(
        "context {} {} / {}",
        components::meter(snapshot.usage.tokens_used, snapshot.usage.context_limit, 12),
        snapshot.usage.tokens_used,
        snapshot
            .usage
            .context_limit
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".into())
    ));
    lines.push(chrome::footer(snapshot));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operator::{
        mode::OperatorMode,
        snapshot::{OperatorSnapshot, ToolTimelineItem},
    };
    #[test]
    fn agent_working_does_not_hardcode_model() {
        let s = OperatorSnapshot::from_workspace("/missing", OperatorMode::AgentWorking)
            .with_tool_timeline(vec![ToolTimelineItem {
                name: "file_read".into(),
                target: "Cargo.toml".into(),
                state: "ok".into(),
                detail: None,
            }]);
        let joined = render_lines(&s, "task", "working").join("\n");
        assert!(!joined.contains(concat!("claude", "-", "sonnet")));
    }
}
