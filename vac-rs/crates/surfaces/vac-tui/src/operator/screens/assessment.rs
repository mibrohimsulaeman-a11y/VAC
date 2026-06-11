use crate::operator::{OperatorSnapshot, chrome};

pub fn render_lines(snapshot: &OperatorSnapshot) -> Vec<String> {
    vec![
        chrome::top_bar(snapshot, "assessment"),
        "gap reports are loaded from .vac/assessment/*.json; missing reports render as not run yet"
            .into(),
        chrome::footer(snapshot),
    ]
}
