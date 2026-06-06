use cortex_aql::{Comparator, Condition, Literal};

pub(super) fn condition_to_string(condition: &Condition<'_>) -> String {
    match condition {
        Condition::Predicate {
            field,
            comparator,
            literal,
        } => format!(
            "{} {} {}",
            field.node.value,
            comparator_to_string(comparator.node),
            literal_to_string(&literal.node)
        ),
        Condition::Not(inner) => format!("NOT ({})", condition_to_string(&inner.node)),
        Condition::And(left, right) => format!(
            "({}) AND ({})",
            condition_to_string(&left.node),
            condition_to_string(&right.node)
        ),
        Condition::Or(left, right) => format!(
            "({}) OR ({})",
            condition_to_string(&left.node),
            condition_to_string(&right.node)
        ),
    }
}

fn comparator_to_string(comparator: Comparator) -> &'static str {
    match comparator {
        Comparator::Eq => "=",
        Comparator::NotEq => "!=",
        Comparator::Gt => ">",
        Comparator::Gte => ">=",
        Comparator::Lt => "<",
        Comparator::Lte => "<=",
        Comparator::In => "IN",
    }
}

fn literal_to_string(literal: &Literal<'_>) -> String {
    match literal {
        Literal::String(value) => format!("{:?}", value.value),
        Literal::Identifier(value) => value.value.to_string(),
        Literal::List(values) => format!(
            "[{}]",
            values
                .iter()
                .map(|value| literal_to_string(&value.node))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Literal::Integer(value) => value.to_string(),
        Literal::Decimal(value) => value.raw.to_string(),
        Literal::Boolean(value) => value.to_string(),
    }
}
