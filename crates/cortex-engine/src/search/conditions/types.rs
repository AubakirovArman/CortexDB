use crate::verification::numeric::NumericValue;
use crate::verification::temporal::TemporalQueryRange;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NumericConditionOperator {
    Equal,
    AtLeast,
    AtMost,
    GreaterThan,
    LessThan,
    Between,
}

impl NumericConditionOperator {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Equal => "equal",
            Self::AtLeast => "at_least",
            Self::AtMost => "at_most",
            Self::GreaterThan => "greater_than",
            Self::LessThan => "less_than",
            Self::Between => "between",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryNumericCondition {
    pub id: String,
    pub operator: NumericConditionOperator,
    pub values: Vec<NumericValue>,
    pub metric_terms: Vec<String>,
    pub raw_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryConditionSlot {
    pub id: String,
    pub operator_hint: Option<NumericConditionOperator>,
    pub metric_terms: Vec<String>,
    pub raw_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryConditionExtraction {
    pub question: String,
    pub numeric_conditions: Vec<QueryNumericCondition>,
    pub condition_slots: Vec<QueryConditionSlot>,
    pub temporal_range: Option<TemporalQueryRange>,
}

impl QueryConditionExtraction {
    pub fn has_structured_conditions(&self) -> bool {
        !self.numeric_conditions.is_empty()
            || !self.condition_slots.is_empty()
            || self.temporal_range.is_some()
    }
}
