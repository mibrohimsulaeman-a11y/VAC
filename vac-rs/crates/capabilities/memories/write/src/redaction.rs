pub fn redact_memory_text(input: impl Into<String>) -> String {
    let input = input.into();
    let redacted = vac_secrets::redact_secrets(input);
    let redacted = redact_private_key_blocks(&redacted);
    redact_jwt_like_tokens(&redacted)
}

fn redact_private_key_blocks(input: &str) -> String {
    let lower = input.to_ascii_lowercase();
    if lower.contains("-----begin ") && lower.contains("private key-----") {
        return "[REDACTED_SECRET]".to_string();
    }
    input.to_string()
}

fn redact_jwt_like_tokens(input: &str) -> String {
    input
        .split_whitespace()
        .map(|token| {
            let trimmed = token.trim_matches(|ch: char| {
                matches!(ch, ',' | ';' | '"' | '\'' | ')' | '(' | '[' | ']')
            });
            if looks_like_jwt(trimmed) {
                token.replace(trimmed, "[REDACTED_SECRET]")
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn looks_like_jwt(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts.iter().all(|part| part.len() >= 10)
        && parts.iter().all(|part| {
            part.chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redaction_replaces_api_key_like_strings() {
        let redacted = redact_memory_text("api_key=sk-abcdefghijklmnopqrstuvwxyz");
        assert!(redacted.contains("[REDACTED_SECRET]"));
        assert!(!redacted.contains("abcdefghijklmnopqrstuvwxyz"));
    }

    #[test]
    fn redaction_replaces_jwt_like_strings() {
        let redacted = redact_memory_text(
            "jwt eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dGVzdHNpZ25hdHVyZQ",
        );
        assert!(redacted.contains("[REDACTED_SECRET]"));
        assert!(!redacted.contains("eyJhbGciOiJIUzI1NiJ9"));
    }

    #[test]
    fn redaction_replaces_private_keys() {
        let redacted =
            redact_memory_text("-----BEGIN PRIVATE KEY-----\nabc\n-----END PRIVATE KEY-----");
        assert_eq!(redacted, "[REDACTED_SECRET]");
    }
}
