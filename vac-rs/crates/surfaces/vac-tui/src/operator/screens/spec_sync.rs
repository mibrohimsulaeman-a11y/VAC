use crate::operator::{OperatorSnapshot, chrome};

pub fn render_lines(snapshot: &OperatorSnapshot) -> Vec<String> {
    vec![
        chrome::top_bar(snapshot, "spec-sync"),
        format!(
            "critical drift {}",
            snapshot.control_plane.unresolved_critical_drift
        ),
        "generated updates remain proposals until approved and compiled".into(),
        chrome::footer(snapshot),
    ]
}
