pub(super) fn sanitize_line_value(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '\n' | '\r' => ' ',
            other => other,
        })
        .collect()
}

pub(super) fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

pub(super) fn push_string_field(out: &mut Vec<u8>, tag: u8, value: &str) {
    push_bytes_field(out, tag, value.as_bytes());
}

pub(super) fn push_optional_string_field(out: &mut Vec<u8>, tag: u8, value: Option<&str>) {
    if let Some(value) = value {
        push_string_field(out, tag, value);
    }
}

pub(super) fn push_u8_field(out: &mut Vec<u8>, tag: u8, value: u8) {
    push_bytes_field(out, tag, &[value]);
}

pub(super) fn push_optional_u16_field(out: &mut Vec<u8>, tag: u8, value: Option<u16>) {
    if let Some(value) = value {
        push_bytes_field(out, tag, &value.to_le_bytes());
    }
}

pub(super) fn push_optional_u64_field(out: &mut Vec<u8>, tag: u8, value: Option<u64>) {
    if let Some(value) = value {
        push_bytes_field(out, tag, &value.to_le_bytes());
    }
}

fn push_bytes_field(out: &mut Vec<u8>, tag: u8, value: &[u8]) {
    let Ok(len) = u32::try_from(value.len()) else {
        return;
    };
    out.push(tag);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value);
}

pub(super) fn read_u32(bytes: &[u8], cursor: &mut usize) -> Option<u32> {
    let end = cursor.checked_add(4)?;
    let raw: [u8; 4] = bytes.get(*cursor..end)?.try_into().ok()?;
    *cursor = end;
    Some(u32::from_le_bytes(raw))
}

pub(super) fn read_fixed_u16(bytes: &[u8]) -> Option<u16> {
    let raw: [u8; 2] = bytes.try_into().ok()?;
    Some(u16::from_le_bytes(raw))
}

pub(super) fn read_fixed_u64(bytes: &[u8]) -> Option<u64> {
    let raw: [u8; 8] = bytes.try_into().ok()?;
    Some(u64::from_le_bytes(raw))
}

pub(super) fn decode_non_empty_string(bytes: &[u8]) -> Option<String> {
    let value = String::from_utf8(bytes.to_vec()).ok()?;
    (!value.is_empty()).then_some(value)
}

pub(super) fn decode_optional_string(bytes: &[u8]) -> Option<Option<String>> {
    Some(non_empty(&String::from_utf8(bytes.to_vec()).ok()?))
}
