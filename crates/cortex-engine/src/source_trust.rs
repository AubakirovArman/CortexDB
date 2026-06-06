pub const DEFAULT_SOURCE_TRUST_Q16: u16 = 32_768;
pub const OFFICIAL_SOURCE_TRUST_Q16: u16 = 60_000;
pub const INTERNAL_SOURCE_TRUST_Q16: u16 = 52_000;
pub const EXTRACTED_SOURCE_TRUST_Q16: u16 = 40_000;
pub const INFERRED_SOURCE_TRUST_Q16: u16 = 20_000;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceTrustClass {
    Official,
    Internal,
    Extracted,
    Inferred,
}

impl SourceTrustClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Official => "official",
            Self::Internal => "internal",
            Self::Extracted => "extracted",
            Self::Inferred => "inferred",
        }
    }

    pub fn calibrated_q16(self) -> u16 {
        match self {
            Self::Official => OFFICIAL_SOURCE_TRUST_Q16,
            Self::Internal => INTERNAL_SOURCE_TRUST_Q16,
            Self::Extracted => EXTRACTED_SOURCE_TRUST_Q16,
            Self::Inferred => INFERRED_SOURCE_TRUST_Q16,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceTrustBasis {
    ExplicitQ16,
    CalibratedClass(SourceTrustClass),
    DefaultUnknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceTrust {
    pub q16: u16,
    pub category: SourceTrustCategory,
    pub basis: SourceTrustBasis,
}

impl SourceTrust {
    pub fn from_q16(value: Option<u16>) -> Self {
        Self::from_metadata(value, None)
    }

    pub fn from_metadata(value: Option<u16>, class: Option<SourceTrustClass>) -> Self {
        match value {
            Some(q16) => Self {
                q16,
                category: category_for_q16(q16),
                basis: SourceTrustBasis::ExplicitQ16,
            },
            None => match class {
                Some(class) => {
                    let q16 = class.calibrated_q16();
                    Self {
                        q16,
                        category: category_for_q16(q16),
                        basis: SourceTrustBasis::CalibratedClass(class),
                    }
                }
                None => Self {
                    q16: DEFAULT_SOURCE_TRUST_Q16,
                    category: SourceTrustCategory::Unknown,
                    basis: SourceTrustBasis::DefaultUnknown,
                },
            },
        }
    }

    pub fn score_bonus(self) -> u32 {
        u32::from(self.q16)
    }

    pub fn score_reason(self) -> String {
        match self.basis {
            SourceTrustBasis::DefaultUnknown => {
                "default provenance trust because source_trust_q16 is absent".to_owned()
            }
            SourceTrustBasis::ExplicitQ16 => format!(
                "{} provenance trust from source_trust_q16 metadata",
                self.category.as_str()
            ),
            SourceTrustBasis::CalibratedClass(class) => format!(
                "{} calibrated provenance trust from source_trust_class metadata",
                class.as_str()
            ),
        }
    }
}

pub fn parse_source_trust_class(value: &str) -> Option<SourceTrustClass> {
    match value.trim().to_ascii_lowercase().as_str() {
        "official" => Some(SourceTrustClass::Official),
        "internal" => Some(SourceTrustClass::Internal),
        "extracted" => Some(SourceTrustClass::Extracted),
        "inferred" => Some(SourceTrustClass::Inferred),
        _ => None,
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
        assert_eq!(trust.basis, SourceTrustBasis::DefaultUnknown);
    }

    #[test]
    fn source_trust_thresholds_are_stable() {
        assert_eq!(category_for_q16(1), SourceTrustCategory::Low);
        assert_eq!(category_for_q16(32_768), SourceTrustCategory::Medium);
        assert_eq!(category_for_q16(50_000), SourceTrustCategory::High);
        assert_eq!(category_for_q16(60_000), SourceTrustCategory::Official);
    }

    #[test]
    fn source_trust_class_calibration_is_stable() {
        assert_eq!(
            SourceTrustClass::Official.calibrated_q16(),
            OFFICIAL_SOURCE_TRUST_Q16
        );
        assert_eq!(
            SourceTrustClass::Internal.calibrated_q16(),
            INTERNAL_SOURCE_TRUST_Q16
        );
        assert_eq!(
            SourceTrustClass::Extracted.calibrated_q16(),
            EXTRACTED_SOURCE_TRUST_Q16
        );
        assert_eq!(
            SourceTrustClass::Inferred.calibrated_q16(),
            INFERRED_SOURCE_TRUST_Q16
        );
    }

    #[test]
    fn explicit_q16_overrides_source_trust_class() {
        let trust = SourceTrust::from_metadata(Some(10_000), Some(SourceTrustClass::Official));
        assert_eq!(trust.q16, 10_000);
        assert_eq!(trust.category, SourceTrustCategory::Low);
        assert_eq!(trust.basis, SourceTrustBasis::ExplicitQ16);
    }

    #[test]
    fn calibrated_source_trust_class_explains_score_reason() {
        let trust = SourceTrust::from_metadata(None, Some(SourceTrustClass::Internal));
        assert_eq!(trust.q16, INTERNAL_SOURCE_TRUST_Q16);
        assert_eq!(trust.category, SourceTrustCategory::High);
        assert_eq!(
            trust.score_reason(),
            "internal calibrated provenance trust from source_trust_class metadata"
        );
    }
}
