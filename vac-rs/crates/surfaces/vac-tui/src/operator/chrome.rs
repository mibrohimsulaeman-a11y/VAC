use crate::operator::{OperatorSnapshot, components};

pub fn top_bar(snapshot: &OperatorSnapshot, title: &str) -> String {
    format!("{} · interactive — {}", snapshot.brand.binary, title)
}

pub fn tab_bar(snapshot: &OperatorSnapshot) -> String {
    snapshot
        .tabs
        .iter()
        .map(|t| {
            if t == snapshot.active_mode.tab_label() {
                format!("[{t}]")
            } else {
                t.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

pub fn footer(snapshot: &OperatorSnapshot) -> String {
    let readiness = snapshot
        .control_plane
        .valid_percent
        .map(|v| format!("valid {v}%"))
        .unwrap_or_else(|| "valid unknown".to_string());
    format!(
        "model {} | {} tok | profile {} | {} | rulebook {}",
        snapshot.model.display_model(),
        snapshot.usage.tokens_used,
        snapshot.profile,
        readiness,
        snapshot.brand.rulebook
    )
}

pub fn input_hint(snapshot: &OperatorSnapshot) -> String {
    let meter = components::meter(snapshot.usage.tokens_used, snapshot.usage.context_limit, 12);
    if meter.is_empty() {
        "/commands · @ files · shift+tab plan".to_string()
    } else {
        format!("context {meter} {} tok", snapshot.usage.tokens_used)
    }
}
