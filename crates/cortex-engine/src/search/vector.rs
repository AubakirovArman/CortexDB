pub(crate) fn vector_from_payload(payload: &[u8]) -> Option<Vec<i16>> {
    let text = String::from_utf8_lossy(payload);
    text.lines().find_map(|line| {
        let value = line.trim().strip_prefix("vector=")?;
        let vector = value
            .split([',', ' '])
            .filter(|part| !part.is_empty())
            .map(str::parse::<i16>)
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        (!vector.is_empty()).then_some(vector)
    })
}
