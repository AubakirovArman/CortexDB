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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PayloadVectorView {
    pub name: String,
    pub vector: Vec<i16>,
}

pub(crate) fn vector_from_payload(payload: &[u8]) -> Option<Vec<i16>> {
    vectors_from_payload(payload)
        .into_iter()
        .find(|view| view.name == "body")
        .map(|view| view.vector)
}

pub(crate) fn vectors_from_payload(payload: &[u8]) -> Vec<PayloadVectorView> {
    let text = String::from_utf8_lossy(payload);
    text.lines()
        .filter_map(|line| {
            let (name, value) = vector_line(line.trim())?;
            parse_vector_literal(value)
                .ok()
                .map(|vector| PayloadVectorView { name, vector })
        })
        .collect()
}

fn vector_line(line: &str) -> Option<(String, &str)> {
    if let Some(value) = line.strip_prefix("vector=") {
        return Some(("body".to_owned(), value));
    }
    let (name, value) = line.split_once("_vector=")?;
    if is_supported_view_name(name) {
        Some((name.to_owned(), value))
    } else {
        None
    }
}

fn is_supported_view_name(name: &str) -> bool {
    matches!(name, "title" | "path" | "entity" | "section" | "summary")
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

    #[test]
    fn payload_vectors_include_named_views() {
        let views = vectors_from_payload(
            b"title_vector=1,0\npath_vector=0,1\nvector=2,2\nignored_vector=9,9\n",
        );

        assert_eq!(
            views
                .iter()
                .map(|view| (view.name.as_str(), view.vector.as_slice()))
                .collect::<Vec<_>>(),
            vec![
                ("title", [1, 0].as_slice()),
                ("path", [0, 1].as_slice()),
                ("body", [2, 2].as_slice())
            ]
        );
        assert_eq!(
            vector_from_payload(b"title_vector=1,0\nvector=2,2"),
            Some(vec![2, 2])
        );
    }
}
