const Q16: u64 = 65_536;
const LN_2_Q16: u64 = 45_426;

pub const DEFAULT_BM25_K1_Q16: u32 = 78_643;
pub const DEFAULT_BM25_B_Q16: u32 = 49_152;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bm25Config {
    pub k1_q16: u32,
    pub b_q16: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bm25CorpusStats {
    pub doc_count: usize,
    pub doc_freq: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bm25TermInput {
    pub tf: u32,
    pub doc_len: u32,
    pub avg_len_q16: u64,
    pub query_weight: u32,
    pub field_weight: u32,
}

impl Bm25Config {
    pub const fn new(k1_q16: u32, b_q16: u32) -> Self {
        Self { k1_q16, b_q16 }
    }
}

impl Default for Bm25Config {
    fn default() -> Self {
        Self {
            k1_q16: DEFAULT_BM25_K1_Q16,
            b_q16: DEFAULT_BM25_B_Q16,
        }
    }
}

pub fn bm25_idf_q16(doc_count: usize, doc_freq: usize) -> u64 {
    if doc_count == 0 || doc_freq == 0 {
        return 0;
    }
    let doc_freq = doc_freq.min(doc_count);
    let numerator = (doc_count as u128 + 1).saturating_mul(2 * u128::from(Q16));
    let denominator = (doc_freq as u128).saturating_mul(2).saturating_add(1);
    ln_q16(saturating_to_u64(numerator / denominator))
}

pub fn bm25_term_score_q16(
    input: Bm25TermInput,
    corpus: Bm25CorpusStats,
    config: Bm25Config,
) -> u64 {
    let idf_q16 = bm25_idf_q16(corpus.doc_count, corpus.doc_freq);
    bm25_term_score_with_idf_q16(input, idf_q16, config)
}

pub fn bm25_term_score_with_idf_q16(input: Bm25TermInput, idf_q16: u64, config: Bm25Config) -> u64 {
    if input.tf == 0 || idf_q16 == 0 || input.query_weight == 0 || input.field_weight == 0 {
        return 0;
    }
    let tf_component_q16 =
        bm25_tf_component_q16(input.tf, input.doc_len.max(1), input.avg_len_q16, config);
    saturating_to_u64(
        u128::from(idf_q16)
            .saturating_mul(u128::from(tf_component_q16))
            .checked_div(u128::from(Q16))
            .unwrap_or_default()
            .saturating_mul(u128::from(input.query_weight))
            .saturating_mul(u128::from(input.field_weight)),
    )
}

fn bm25_tf_component_q16(tf: u32, doc_len: u32, avg_len_q16: u64, config: Bm25Config) -> u64 {
    let avg_len_q16 = avg_len_q16.max(Q16);
    let tf_q16 = u128::from(tf).saturating_mul(u128::from(Q16));
    let len_ratio_q16 = u128::from(doc_len)
        .saturating_mul(u128::from(Q16))
        .saturating_mul(u128::from(Q16))
        .checked_div(u128::from(avg_len_q16))
        .unwrap_or(u128::from(Q16));
    let b_q16 = u128::from(config.b_q16.min(Q16 as u32));
    let k1_q16 = u128::from(config.k1_q16);
    let norm_q16 = u128::from(Q16)
        .saturating_sub(b_q16)
        .saturating_add(b_q16.saturating_mul(len_ratio_q16) / u128::from(Q16));
    let k1_norm_q16 = k1_q16.saturating_mul(norm_q16) / u128::from(Q16);
    let numerator_q16 =
        tf_q16.saturating_mul(k1_q16.saturating_add(u128::from(Q16))) / u128::from(Q16);
    let denominator_q16 = tf_q16.saturating_add(k1_norm_q16).max(1);
    saturating_to_u64(numerator_q16.saturating_mul(u128::from(Q16)) / denominator_q16)
}

fn ln_q16(input_q16: u64) -> u64 {
    if input_q16 <= Q16 {
        return 0;
    }
    let one_q32 = 1u128 << 32;
    let two_q32 = one_q32 * 2;
    let mut x_q32 = u128::from(input_q16) << 16;
    let mut shifts = 0u64;
    while x_q32 >= two_q32 {
        x_q32 = x_q32.div_ceil(2);
        shifts += 1;
    }
    let z_q32 = ((x_q32 - one_q32) << 32) / (x_q32 + one_q32);
    let z2_q32 = z_q32.saturating_mul(z_q32) >> 32;
    let mut term_q32 = z_q32;
    let mut sum_q32 = z_q32;
    for denominator in [3u128, 5, 7, 9, 11, 13, 15, 17, 19, 21, 23, 25, 27, 29, 31] {
        term_q32 = term_q32.saturating_mul(z2_q32) >> 32;
        sum_q32 = sum_q32.saturating_add(term_q32 / denominator);
    }
    shifts
        .saturating_mul(LN_2_Q16)
        .saturating_add(saturating_to_u64((sum_q32 * 2) >> 16))
}

fn saturating_to_u64(value: u128) -> u64 {
    value.min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_point_bm25_matches_float_reference() {
        let cases = [
            (1, 10, 10, 3, 1, 1),
            (3, 12, 10, 2, 4, 1),
            (1, 3, 7, 1, 4, 8),
            (9, 30, 24, 6, 2, 5),
        ];
        for (tf, doc_len, avg_len, doc_freq, query_weight, field_weight) in cases {
            let actual = bm25_term_score_q16(
                Bm25TermInput {
                    tf,
                    doc_len,
                    avg_len_q16: u64::from(avg_len) * Q16,
                    query_weight,
                    field_weight,
                },
                Bm25CorpusStats {
                    doc_count: 24,
                    doc_freq,
                },
                Bm25Config::default(),
            );
            let expected = float_bm25(tf, doc_len, avg_len, 24, doc_freq)
                * Q16 as f64
                * f64::from(query_weight)
                * f64::from(field_weight);
            let delta = (actual as f64 - expected).abs();
            assert!(
                delta / expected.max(1.0) < 0.002,
                "actual={actual} expected={expected} delta={delta}"
            );
        }
    }

    #[test]
    fn default_bm25_config_is_canonical() {
        let config = Bm25Config::default();
        assert!((f64::from(config.k1_q16) / Q16 as f64 - 1.2).abs() < 0.00001);
        assert_eq!(config.b_q16, DEFAULT_BM25_B_Q16);
    }

    fn float_bm25(tf: u32, doc_len: u32, avg_len: u32, doc_count: usize, doc_freq: usize) -> f64 {
        let k1 = 1.2;
        let b = 0.75;
        let idf = ((doc_count as f64 + 1.0) / (doc_freq as f64 + 0.5)).ln();
        let length_norm = 1.0 - b + b * (doc_len as f64 / avg_len as f64);
        idf * (tf as f64 * (k1 + 1.0)) / (tf as f64 + k1 * length_norm)
    }
}
