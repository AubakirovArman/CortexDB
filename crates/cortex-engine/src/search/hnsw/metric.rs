/// Supported distance metrics for vector similarity search.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DistanceMetric {
    /// Non-negative dot-product similarity. Higher is better.
    #[default]
    DotProduct = 0,
    /// Cosine similarity scaled to [0, 65_535]. Higher is better.
    Cosine = 1,
    /// Negative squared Euclidean distance. Higher (less negative) is better.
    L2 = 2,
}

impl DistanceMetric {
    /// Compute similarity between two vectors. Returns `None` if dimensions mismatch.
    pub fn distance(&self, u: &[i16], v: &[i16]) -> Option<u64> {
        if u.len() != v.len() {
            return None;
        }
        match self {
            Self::DotProduct => Some(
                u.iter()
                    .zip(v)
                    .map(|(left, right)| i64::from(*left) * i64::from(*right))
                    .sum::<i64>()
                    .max(0) as u64,
            ),
            Self::Cosine => {
                let dot: i64 = u
                    .iter()
                    .zip(v)
                    .map(|(a, b)| i64::from(*a) * i64::from(*b))
                    .sum();
                let u_norm_sq: i64 = u.iter().map(|x| i64::from(*x) * i64::from(*x)).sum();
                let v_norm_sq: i64 = v.iter().map(|x| i64::from(*x) * i64::from(*x)).sum();
                if u_norm_sq == 0 || v_norm_sq == 0 {
                    return Some(0);
                }
                let norm_sq = (u_norm_sq as u128).saturating_mul(v_norm_sq as u128);
                let norm = norm_sq.isqrt() as i64;
                if norm == 0 {
                    return Some(0);
                }
                Some(((dot.abs() * 65_535) / norm.abs()) as u64)
            }
            Self::L2 => {
                let dist_sq: i64 = u
                    .iter()
                    .zip(v)
                    .map(|(a, b)| {
                        let diff = i64::from(*a) - i64::from(*b);
                        diff * diff
                    })
                    .sum();
                let max_dist = (u.len() as i64) * 65_536i64 * 65_536i64;
                Some((max_dist - dist_sq.min(max_dist)).max(0) as u64)
            }
        }
    }
}

/// Configuration for a vector collection persisted alongside the HNSW graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct VectorCollectionConfig {
    pub dimension: usize,
    pub metric: DistanceMetric,
}
