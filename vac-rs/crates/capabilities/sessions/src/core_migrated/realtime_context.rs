pub const REALTIME_TURN_TOKEN_BUDGET: usize = 1_500;

pub fn truncate_realtime_text_to_token_budget(text: &str, token_budget: usize) -> String {
    if token_budget == 0 || text.is_empty() {
        return String::new();
    }

    let max_chars = token_budget.saturating_mul(4);
    if text.len() <= max_chars {
        return text.to_string();
    }

    let mut end = 0;
    for (idx, _) in text.char_indices() {
        if idx > max_chars {
            break;
        }
        end = idx;
    }
    text[..end].to_string()
}
