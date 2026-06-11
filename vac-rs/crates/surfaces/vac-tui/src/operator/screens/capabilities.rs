use crate::operator::{OperatorSnapshot, chrome};

pub fn render_lines(snapshot: &OperatorSnapshot) -> Vec<String> {
    vec![
        chrome::top_bar(snapshot, "capabilities"),
        "capability readiness uses declared / computed / effective from compiled registry".into(),
        format!("control-plane {}", snapshot.control_plane.status),
        chrome::footer(snapshot),
    ]
}
