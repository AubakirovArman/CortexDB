pub fn parse_vector_literal(value: &str) -> Result<Vec<i16>, String> {
    let vector = value
        .split([',', ' '])
        .filter(|part| !part.is_empty())
        .map(str::parse::<i16>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "vector values must be i16".to_owned())?;
    if vector.is_empty() {
        Err("vector must not be empty".to_owned())
    } else {
        Ok(vector)
    }
}

pub(crate) fn vector_from_payload(payload: &[u8]) -> Option<Vec<i16>> {
    let text = String::from_utf8_lossy(payload);
    text.lines().find_map(|line| {
        let value = line.trim().strip_prefix("vector=")?;
        parse_vector_literal(value).ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_literal_accepts_commas_and_spaces() {
        assert_eq!(parse_vector_literal("1, -2 3").unwrap(), vec![1, -2, 3]);
    }

    #[test]
    fn vector_literal_rejects_empty_or_invalid_values() {
        assert!(parse_vector_literal(" , ").is_err());
        assert!(parse_vector_literal("1,nope").is_err());
    }
}
