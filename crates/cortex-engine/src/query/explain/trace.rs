use crate::exec::PhysicalOperatorTrace;

pub(super) fn operator_output_count(operators: &[PhysicalOperatorTrace], name: &str) -> usize {
    operators
        .iter()
        .find(|operator| operator.name == name)
        .map(|operator| operator.output_count)
        .unwrap_or(0)
}
