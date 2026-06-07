use serde::Serialize;
use serde_json::Value;

pub fn canonicalize_serializable(value: &impl Serialize) -> Result<String, String> {
    let value = serde_json::to_value(value).map_err(|err| err.to_string())?;
    Ok(canonicalize_value(&value))
}

#[allow(clippy::expect_used)] // serde_json string/key serialization is infallible
pub fn canonicalize_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => {
            serde_json::to_string(value).expect("JSON string serialization is infallible")
        }
        Value::Array(values) => {
            let inner = values
                .iter()
                .map(canonicalize_value)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{inner}]")
        }
        Value::Object(values) => {
            let mut sorted = values.iter().collect::<Vec<_>>();
            // RFC 8785 (JCS) §3.2.3: object members are sorted by the UTF-16
            // code units of the key, which can differ from Unicode code point
            // order for supplementary-plane characters.
            sorted.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
            let inner = sorted
                .into_iter()
                .map(|(key, value)| {
                    let key = serde_json::to_string(key)
                        .expect("JSON object key serialization is infallible");
                    format!("{key}:{}", canonicalize_value(value))
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{inner}}}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn jcs_sorts_object_keys_without_whitespace() {
        let value = json!({"z": 1, "a": true, "m": ["x", null]});
        assert_eq!(
            canonicalize_value(&value),
            r#"{"a":true,"m":["x",null],"z":1}"#
        );
    }

    #[test]
    fn jcs_orders_object_keys_by_utf16_code_units() {
        // U+10000 begins with surrogate 0xD800 in UTF-16, so it sorts before
        // U+FFFF even though it is a larger Unicode code point.
        let value = json!({"\u{ffff}": 1, "\u{10000}": 2});
        assert_eq!(
            canonicalize_value(&value),
            "{\"\u{10000}\":2,\"\u{ffff}\":1}"
        );
    }

    #[test]
    fn jcs_canonicalizes_nested_integers_and_strings() {
        let value = json!({
            "b": {"d": 2, "c": 1},
            "a": ["one", 3, true, null]
        });
        assert_eq!(
            canonicalize_value(&value),
            r#"{"a":["one",3,true,null],"b":{"c":1,"d":2}}"#
        );
    }

    #[test]
    fn jcs_escapes_quotes_backslashes_and_control_characters() {
        let value = json!({"k": "a\"\\\n\t"});
        assert_eq!(canonicalize_value(&value), r#"{"k":"a\"\\\n\t"}"#);
    }
}
