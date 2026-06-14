use core::str::FromStr;

pub type Q16 = u16;

pub const Q16_ZERO: Q16 = 0;
pub const Q16_ONE: Q16 = 65_535;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AgentId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BrainId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopeId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LensId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StatusId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CellTypeId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RetrievalMode {
    Fast,
    Balanced,
    Hybrid,
    Semantic,
    Audit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemoryType {
    Decision,
    Preference,
    WorkflowResult,
    ErrorLog,
    Observation,
}

pub fn q16_from_f64_clamped(value: f64) -> Q16 {
    if !value.is_finite() || value <= 0.0 {
        Q16_ZERO
    } else if value >= 1.0 {
        Q16_ONE
    } else {
        (value * f64::from(Q16_ONE)).round() as Q16
    }
}

pub fn q16_mul(lhs: Q16, rhs: Q16) -> Q16 {
    let product = u32::from(lhs) * u32::from(rhs);
    ((product + (u32::from(Q16_ONE) / 2)) / u32::from(Q16_ONE)) as Q16
}

impl FromStr for RetrievalMode {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "fast" => Ok(Self::Fast),
            "balanced" => Ok(Self::Balanced),
            "hybrid" => Ok(Self::Hybrid),
            "semantic" => Ok(Self::Semantic),
            "audit" => Ok(Self::Audit),
            _ => Err(()),
        }
    }
}

impl FromStr for MemoryType {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "decision" => Ok(Self::Decision),
            "preference" => Ok(Self::Preference),
            "workflow_result" | "workflowresult" => Ok(Self::WorkflowResult),
            "error_log" | "errorlog" => Ok(Self::ErrorLog),
            "observation" => Ok(Self::Observation),
            _ => Err(()),
        }
    }
}
