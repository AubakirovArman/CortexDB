pub(crate) fn query_vector_from_task(task: &str) -> Option<Vec<i16>> {
    task.lines().find_map(|line| {
        let value = line
            .trim()
            .strip_prefix("query_vector=")
            .or_else(|| line.trim().strip_prefix("vector="))?;
        crate::search::parse_vector_literal(value).ok()
    })
}

pub(crate) fn semantic_dot_score(payload: &[u8], query: &[i16]) -> u64 {
    crate::search::vector::vectors_from_payload(payload)
        .into_iter()
        .filter(|view| view.vector.len() == query.len())
        .map(|view| {
            view.vector
                .iter()
                .zip(query)
                .map(|(left, right)| i64::from(*left) * i64::from(*right))
                .sum::<i64>()
                .max(0) as u64
        })
        .max()
        .unwrap_or(0)
}
