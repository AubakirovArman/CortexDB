pub fn decode_hex(value: &str) -> Result<Vec<u8>, &'static str> {
    let value = value.trim();
    if !value.len().is_multiple_of(2) {
        return Err("odd hex length");
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|chunk| {
            let high = hex_nibble(chunk[0]).ok_or("invalid hex")?;
            let low = hex_nibble(chunk[1]).ok_or("invalid hex")?;
            Ok((high << 4) | low)
        })
        .collect()
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
