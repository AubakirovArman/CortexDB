use serde_json::Value;

pub fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    let mut out = String::new();
    write_canonical_value(value, &mut out);
    out.into_bytes()
}

fn write_canonical_value(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
        Value::Number(number) => out.push_str(&number.to_string()),
        Value::String(value) => write_json_string(value, out),
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical_value(item, out);
            }
            out.push(']');
        }
        Value::Object(object) => {
            let mut keys = object.keys().collect::<Vec<_>>();
            keys.sort();
            out.push('{');
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_json_string(key, out);
                out.push(':');
                if let Some(item) = object.get(*key) {
                    write_canonical_value(item, out);
                } else {
                    out.push_str("null");
                }
            }
            out.push('}');
        }
    }
}

fn write_json_string(value: &str, out: &mut String) {
    match serde_json::to_string(value) {
        Ok(encoded) => out.push_str(&encoded),
        Err(_) => out.push_str("\"\""),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::canonical_json_bytes;

    #[test]
    fn canonical_bytes_sort_object_keys_recursively() {
        let first = json!({"b": 2, "a": {"d": 4, "c": 3}});
        let second = json!({"a": {"c": 3, "d": 4}, "b": 2});

        assert_eq!(canonical_json_bytes(&first), canonical_json_bytes(&second));
        assert_eq!(
            String::from_utf8(canonical_json_bytes(&first)).unwrap(),
            r#"{"a":{"c":3,"d":4},"b":2}"#
        );
    }
}
