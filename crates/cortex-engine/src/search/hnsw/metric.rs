use crate::search::vector_similarity::cosine_similarity_q16;

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
            Self::Cosine => Some(u64::from(cosine_similarity_q16(u, v))),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_metric_rejects_anticorrelated_vectors() {
        assert_eq!(
            DistanceMetric::Cosine.distance(&[10, -20, 30], &[-10, 20, -30]),
            Some(0)
        );
        assert_eq!(
            DistanceMetric::Cosine.distance(&[10, -20, 30], &[10, -20, 30]),
            Some(u64::from(u16::MAX))
        );
    }

    #[test]
    fn cosine_metric_orders_positive_before_orthogonal_and_negative() {
        let query = [10, 0];
        let mut candidates = [
            (2_u32, "negative", [-10, 0]),
            (1_u32, "orthogonal", [0, 10]),
            (0_u32, "positive", [10, 0]),
        ];
        candidates.sort_by(|left, right| {
            let left_score = DistanceMetric::Cosine.distance(&query, &left.2).unwrap();
            let right_score = DistanceMetric::Cosine.distance(&query, &right.2).unwrap();
            right_score
                .cmp(&left_score)
                .then_with(|| left.0.cmp(&right.0))
        });

        assert_eq!(candidates[0].1, "positive");
        assert_eq!(candidates[2].1, "negative");
    }

    #[test]
    fn cosine_metric_handles_high_dimensional_vectors_without_overflow() {
        let positive = vec![i16::MAX; 4096];
        let negative = vec![-i16::MAX; 4096];

        assert_eq!(
            DistanceMetric::Cosine.distance(&positive, &positive),
            Some(u64::from(u16::MAX))
        );
        assert_eq!(
            DistanceMetric::Cosine.distance(&positive, &negative),
            Some(0)
        );
    }

    #[test]
    fn cosine_metric_matches_shared_q16_helper() {
        let fixtures = [
            (&[1, 0][..], &[1, 0][..]),
            (&[1, 0][..], &[0, 1][..]),
            (&[1, 0][..], &[-1, 0][..]),
            (&[3, 4][..], &[6, 8][..]),
            (&[3, 4][..], &[4, 3][..]),
        ];

        for (left, right) in fixtures {
            assert_eq!(
                DistanceMetric::Cosine.distance(left, right),
                Some(u64::from(cosine_similarity_q16(left, right)))
            );
        }
    }
}
