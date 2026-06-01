pub const DEFAULT_SOURCE_TRUST_Q16: u16 = 32_768;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceTrustCategory {
    Unknown,
    Low,
    Medium,
    High,
    Official,
}

impl SourceTrustCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Official => "official",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceTrust {
    pub q16: u16,
    pub category: SourceTrustCategory,
}

impl SourceTrust {
    pub fn from_q16(value: Option<u16>) -> Self {
        match value {
            Some(q16) => Self {
                q16,
                category: category_for_q16(q16),
            },
            None => Self {
                q16: DEFAULT_SOURCE_TRUST_Q16,
                category: SourceTrustCategory::Unknown,
            },
        }
    }

    pub fn score_bonus(self) -> u32 {
        u32::from(self.q16)
    }

    pub fn score_reason(self) -> String {
        match self.category {
            SourceTrustCategory::Unknown => {
                "default provenance trust because source_trust_q16 is absent".to_owned()
            }
            category => format!(
                "{} provenance trust from source_trust_q16 metadata",
                category.as_str()
            ),
        }
    }
}

pub fn category_for_q16(q16: u16) -> SourceTrustCategory {
    match q16 {
        0..=21_845 => SourceTrustCategory::Low,
        21_846..=43_690 => SourceTrustCategory::Medium,
        43_691..=58_981 => SourceTrustCategory::High,
        _ => SourceTrustCategory::Official,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_source_trust_is_unknown_default() {
        let trust = SourceTrust::from_q16(None);
        assert_eq!(trust.q16, DEFAULT_SOURCE_TRUST_Q16);
        assert_eq!(trust.category, SourceTrustCategory::Unknown);
    }

    #[test]
    fn source_trust_thresholds_are_stable() {
        assert_eq!(category_for_q16(1), SourceTrustCategory::Low);
        assert_eq!(category_for_q16(32_768), SourceTrustCategory::Medium);
        assert_eq!(category_for_q16(50_000), SourceTrustCategory::High);
        assert_eq!(category_for_q16(60_000), SourceTrustCategory::Official);
    }
}
