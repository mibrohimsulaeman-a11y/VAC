use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

pub fn canonicalize_serializable(value: &impl Serialize) -> Result<String, String> {
    let value = serde_json::to_value(value).map_err(|err| err.to_string())?;
    Ok(canonicalize_value(&value))
}

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
            let sorted = values
                .iter()
                .map(|(key, value)| (key, value))
                .collect::<BTreeMap<_, _>>();
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
}
