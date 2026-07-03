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

/// Cosine-style dense score: the exact dot normalized by the *candidate* vector's
/// own magnitude — `dot(query, doc) / |doc|`, scaled by 2^20 to keep integer
/// precision. The query magnitude is constant across a single query's candidates,
/// so omitting it does not change their relative order while making this a true
/// cosine ranking. This corrects the magnitude drift that i16 quantization
/// introduces into otherwise L2-normalized embeddings, so a dense rerank ranks by
/// direction (cosine) rather than by raw dot (which over-rewards large-magnitude
/// vectors). Fully deterministic: integer dot, integer sqrt, integer division.
pub(crate) fn semantic_cosine_score_q16(payload: &[u8], query: &[i16]) -> u64 {
    const PRECISION_SHIFT: u32 = 20;
    crate::search::vector::vectors_from_payload(payload)
        .into_iter()
        .filter(|view| view.vector.len() == query.len())
        .map(|view| {
            let dot = view
                .vector
                .iter()
                .zip(query)
                .map(|(left, right)| i64::from(*left) * i64::from(*right))
                .sum::<i64>()
                .max(0) as u64;
            let norm_sq = view
                .vector
                .iter()
                .map(|value| {
                    let value = i64::from(*value);
                    (value * value) as u64
                })
                .sum::<u64>();
            let norm = norm_sq.isqrt().max(1);
            dot.saturating_mul(1 << PRECISION_SHIFT) / norm
        })
        .max()
        .unwrap_or(0)
}
