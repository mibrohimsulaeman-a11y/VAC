//! Small string components used by snapshot tests and later ratatui widgets.

pub fn badge(label: &str, value: &str) -> String {
    format!("{label} {value}")
}

pub fn meter(used: u64, limit: Option<u64>, width: usize) -> String {
    let limit = limit.unwrap_or(0);
    if limit == 0 || width == 0 {
        return "".to_string();
    }
    let filled = ((used.min(limit) as f64 / limit as f64) * width as f64).round() as usize;
    format!(
        "{}{}",
        "█".repeat(filled.min(width)),
        "░".repeat(width.saturating_sub(filled))
    )
}

pub fn key_hint(key: &str, label: &str) -> String {
    format!("{key} {label}")
}
