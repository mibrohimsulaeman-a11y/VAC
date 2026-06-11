use crate::operator::{OperatorSnapshot, chrome, components};

pub fn render_lines(snapshot: &OperatorSnapshot) -> Vec<String> {
    let mut lines = vec![
        chrome::top_bar(snapshot, "idle"),
        chrome::tab_bar(snapshot),
        components::badge(&snapshot.brand.product, &snapshot.control_plane.status),
        "The agent will stream thinking, tools, and approvals here.".into(),
        "Keyboard-only: tab focus / commands @ files shift+tab plan mode".into(),
    ];
    if snapshot.session.recent.is_empty() {
        lines.push("recent tasks: none".into());
    } else {
        for task in &snapshot.session.recent {
            lines.push(format!(
                "▸ {} · {} · {}",
                task.title, task.when, task.status
            ));
        }
    }
    lines.push(chrome::input_hint(snapshot));
    lines.push(chrome::footer(snapshot));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operator::{mode::OperatorMode, snapshot::OperatorSnapshot};
    #[test]
    fn idle_does_not_render_vil_literal() {
        let s = OperatorSnapshot::from_workspace("/missing", OperatorMode::Idle);
        assert!(
            !render_lines(&s)
                .join("\n")
                .contains(&["V", "I", "L"].join(""))
        );
    }
}
