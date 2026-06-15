use std::collections::HashMap;

use serde_json::{Map, Value};
use vac_foundation::secrets::redact_secrets;

const REDACTED_JSON_SECRET: &str = "[REDACTED]";

pub(crate) fn redact_json_secret_values(value: Value) -> Value {
    redact_json_secret_values_inner(value, false, None)
}

pub(crate) fn redact_optional_json_secret_values(value: Option<Value>) -> Option<Value> {
    value.map(redact_json_secret_values)
}

fn redact_json_secret_values_inner(
    value: Value,
    inherited_sensitive: bool,
    key: Option<&str>,
) -> Value {
    let sensitive = inherited_sensitive || key.is_some_and(is_sensitive_metadata_key);
    match value {
        Value::String(_) if sensitive => Value::String(REDACTED_JSON_SECRET.to_string()),
        Value::String(text) => Value::String(redact_secret_shaped_text(&text)),
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| redact_json_secret_values_inner(item, sensitive, None))
                .collect(),
        ),
        Value::Object(map) => Value::Object(redact_json_secret_object(map, sensitive)),
        other => other,
    }
}

fn redact_json_secret_object(
    map: Map<String, Value>,
    inherited_sensitive: bool,
) -> Map<String, Value> {
    map.into_iter()
        .map(|(key, value)| {
            let redacted = redact_json_secret_values_inner(value, inherited_sensitive, Some(&key));
            (key, redacted)
        })
        .collect()
}

fn redact_secret_shaped_text(text: &str) -> String {
    let result = redact_secrets(text, None, &HashMap::new(), false);
    if result.redaction_map.is_empty() {
        return text.to_string();
    }

    result
        .redaction_map
        .keys()
        .fold(result.redacted_string, |redacted, marker| {
            redacted.replace(marker, REDACTED_JSON_SECRET)
        })
}

fn is_sensitive_metadata_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace(['-', ' '], "_");
    normalized == "key"
        || normalized.ends_with("_key")
        || normalized.contains("api_key")
        || normalized.contains("apikey")
        || normalized.contains("token")
        || normalized.contains("secret")
        || normalized.contains("credential")
        || normalized.contains("password")
        || normalized.contains("authorization")
        || normalized.contains("auth_header")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redacts_sensitive_key_values_recursively() {
        let raw = "raw-vac-api-material";
        let redacted = redact_json_secret_values(json!({
            "safe": "keep-me",
            "api_key": raw,
            "nested": {
                "credentials": {
                    "opaque": raw
                }
            }
        }));
        let rendered = redacted.to_string();

        assert!(rendered.contains("[REDACTED]"));
        assert!(rendered.contains("keep-me"));
        assert!(!rendered.contains(raw));
    }
}
