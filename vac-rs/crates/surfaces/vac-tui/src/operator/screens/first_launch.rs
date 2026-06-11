use crate::operator::{OperatorSnapshot, chrome, components};

pub fn render_lines(snapshot: &OperatorSnapshot) -> Vec<String> {
    vec![
        chrome::top_bar(snapshot, "first launch"),
        chrome::tab_bar(snapshot),
        format!("{} {}", snapshot.brand.product, snapshot.version),
        format!("cwd {}", snapshot.cwd),
        format!(
            "session {}",
            snapshot.session.id.as_deref().unwrap_or("not restored")
        ),
        components::badge("provider", snapshot.model.display_provider()),
        components::badge("model", snapshot.model.display_model()),
        components::badge("runtime", &snapshot.enforcement.level),
        components::badge("isolation", &snapshot.enforcement.isolation),
        components::badge("control-plane", &snapshot.control_plane.status),
        "ready".to_string(),
        "type / for commands, /help to explore, or start typing a task.".to_string(),
        chrome::footer(snapshot),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operator::{mode::OperatorMode, snapshot::OperatorSnapshot};
    #[test]
    fn first_launch_empty_workspace_has_no_mock_tasks() {
        let s = OperatorSnapshot::from_workspace("/missing", OperatorMode::FirstLaunch);
        let joined = render_lines(&s).join("\n");
        assert!(!joined.contains("add error handling"));
        assert!(!joined.contains(&["V", "I", "L"].join("")));
    }
}
