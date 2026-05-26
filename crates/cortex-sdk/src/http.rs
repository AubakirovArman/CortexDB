use crate::SdkResult;

pub(crate) fn parse_response(response: ureq::Response) -> SdkResult<serde_json::Value> {
    Ok(serde_json::from_str(
        &response.into_string().unwrap_or_default(),
    )?)
}

pub(crate) fn path(base: &str, query: &[(&str, &str)]) -> String {
    let encoded = query
        .iter()
        .map(|(key, value)| format!("{}={}", escape(key), escape(value)))
        .collect::<Vec<_>>()
        .join("&");
    if encoded.is_empty() {
        base.to_owned()
    } else {
        format!("{base}?{encoded}")
    }
}

pub(crate) fn append_query_param(path: &str, key: &str, value: &str) -> String {
    let separator = if path.contains('?') { '&' } else { '?' };
    format!("{path}{separator}{}={}", escape(key), escape(value))
}

fn escape(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}
