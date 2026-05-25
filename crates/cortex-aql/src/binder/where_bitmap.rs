use std::str::FromStr;

use crate::ast::{Comparator, Condition, Literal, Spanned};
use crate::policy::PolicyError;
use crate::types::{BrainId, MemoryType};

use super::{AqlCatalog, BindError, Binder, BitmapOp};

pub(super) fn compile_condition<C: AqlCatalog>(
    binder: &Binder<'_, C>,
    brain: BrainId,
    condition: &Condition<'_>,
    ops: &mut Vec<BitmapOp>,
) -> Result<(), BindError> {
    match condition {
        Condition::Predicate {
            field,
            comparator,
            literal,
        } => compile_predicate(
            binder,
            brain,
            field.node.value.as_ref(),
            comparator.node,
            &literal.node,
            ops,
        ),
        Condition::Not(child) => {
            compile_condition(binder, brain, &child.node, ops)?;
            ops.push(BitmapOp::Not);
            Ok(())
        }
        Condition::And(lhs, rhs) => compile_binary(binder, brain, lhs, rhs, BitmapOp::And, ops),
        Condition::Or(lhs, rhs) => compile_binary(binder, brain, lhs, rhs, BitmapOp::Or, ops),
    }
}

fn compile_binary<C: AqlCatalog>(
    binder: &Binder<'_, C>,
    brain: BrainId,
    lhs: &Spanned<Condition<'_>>,
    rhs: &Spanned<Condition<'_>>,
    op: BitmapOp,
    ops: &mut Vec<BitmapOp>,
) -> Result<(), BindError> {
    let mut lhs_ops = Vec::new();
    let mut rhs_ops = Vec::new();
    compile_condition(binder, brain, &lhs.node, &mut lhs_ops)?;
    compile_condition(binder, brain, &rhs.node, &mut rhs_ops)?;
    if op == BitmapOp::And {
        order_by_estimated_cardinality(binder, brain, &mut lhs_ops, &mut rhs_ops);
    }
    ops.extend(lhs_ops);
    ops.extend(rhs_ops);
    ops.push(op);
    Ok(())
}

fn compile_predicate<C: AqlCatalog>(
    binder: &Binder<'_, C>,
    brain: BrainId,
    field: &str,
    comparator: Comparator,
    literal: &Literal<'_>,
    ops: &mut Vec<BitmapOp>,
) -> Result<(), BindError> {
    if !binder.catalog.field_is_filterable(brain, field) {
        return Err(BindError::FieldNotFilterable(field.to_owned()));
    }
    match comparator {
        Comparator::Eq => compile_eq_predicate(binder, brain, field, literal, ops),
        Comparator::In => compile_in_predicate(binder, brain, field, literal, ops),
        _ => Err(BindError::UnsupportedComparator),
    }
}

fn compile_eq_predicate<C: AqlCatalog>(
    binder: &Binder<'_, C>,
    brain: BrainId,
    field: &str,
    literal: &Literal<'_>,
    ops: &mut Vec<BitmapOp>,
) -> Result<(), BindError> {
    match field {
        "space" | "scope" => compile_scope(binder, brain, literal, ops),
        "status" => compile_status(binder, brain, literal, ops),
        "type" | "cell_type" => compile_cell_type(binder, brain, literal, ops),
        "memory_type" => compile_memory_type(binder, brain, literal, ops),
        _ => Err(BindError::FieldNotFilterable(field.to_owned())),
    }
}

fn compile_in_predicate<C: AqlCatalog>(
    binder: &Binder<'_, C>,
    brain: BrainId,
    field: &str,
    literal: &Literal<'_>,
    ops: &mut Vec<BitmapOp>,
) -> Result<(), BindError> {
    let Literal::List(values) = literal else {
        return Err(BindError::UnsupportedLiteral);
    };
    for value in values {
        compile_eq_predicate(binder, brain, field, &value.node, ops)?;
    }
    for _ in 1..values.len() {
        ops.push(BitmapOp::Or);
    }
    Ok(())
}

fn compile_scope<C: AqlCatalog>(
    binder: &Binder<'_, C>,
    brain: BrainId,
    literal: &Literal<'_>,
    ops: &mut Vec<BitmapOp>,
) -> Result<(), BindError> {
    let scope_name = text_literal(literal)?;
    let scope = binder
        .catalog
        .resolve_scope(brain, scope_name)
        .ok_or(BindError::UnknownScope)?;
    if !binder.view.can_read_scope(scope) {
        return Err(BindError::PolicyDenied(PolicyError::ScopeNotReadable));
    }
    let handle = binder
        .catalog
        .scope_bitmap(brain, scope)
        .ok_or(BindError::UnknownBitmap)?;
    ops.push(BitmapOp::Push(handle));
    Ok(())
}

fn compile_status<C: AqlCatalog>(
    binder: &Binder<'_, C>,
    brain: BrainId,
    literal: &Literal<'_>,
    ops: &mut Vec<BitmapOp>,
) -> Result<(), BindError> {
    let status = text_literal(literal)?;
    let status = binder
        .catalog
        .resolve_status(brain, status)
        .ok_or(BindError::UnsupportedLiteral)?;
    let handle = binder
        .catalog
        .status_bitmap(brain, status)
        .ok_or(BindError::UnknownBitmap)?;
    ops.push(BitmapOp::Push(handle));
    Ok(())
}

fn compile_cell_type<C: AqlCatalog>(
    binder: &Binder<'_, C>,
    brain: BrainId,
    literal: &Literal<'_>,
    ops: &mut Vec<BitmapOp>,
) -> Result<(), BindError> {
    let cell_type = text_literal(literal)?;
    let cell_type = binder
        .catalog
        .resolve_cell_type(brain, cell_type)
        .ok_or(BindError::UnsupportedLiteral)?;
    let handle = binder
        .catalog
        .cell_type_bitmap(brain, cell_type)
        .ok_or(BindError::UnknownBitmap)?;
    ops.push(BitmapOp::Push(handle));
    Ok(())
}

fn compile_memory_type<C: AqlCatalog>(
    binder: &Binder<'_, C>,
    brain: BrainId,
    literal: &Literal<'_>,
    ops: &mut Vec<BitmapOp>,
) -> Result<(), BindError> {
    let value = text_literal(literal)?;
    let memory_type = MemoryType::from_str(value).map_err(|_| BindError::UnsupportedLiteral)?;
    let handle = binder
        .catalog
        .memory_type_bitmap(brain, memory_type)
        .ok_or(BindError::UnknownBitmap)?;
    ops.push(BitmapOp::Push(handle));
    Ok(())
}

fn text_literal<'a>(literal: &'a Literal<'a>) -> Result<&'a str, BindError> {
    match literal {
        Literal::String(value) => Ok(value.value.as_ref()),
        Literal::Identifier(value) => Ok(value.value.as_ref()),
        _ => Err(BindError::UnsupportedLiteral),
    }
}

fn order_by_estimated_cardinality<C: AqlCatalog>(
    binder: &Binder<'_, C>,
    brain: BrainId,
    lhs: &mut Vec<BitmapOp>,
    rhs: &mut Vec<BitmapOp>,
) {
    let (Some(lhs_cost), Some(rhs_cost)) = (
        single_push_cost(binder, brain, lhs),
        single_push_cost(binder, brain, rhs),
    ) else {
        return;
    };
    if rhs_cost < lhs_cost {
        std::mem::swap(lhs, rhs);
    }
}

fn single_push_cost<C: AqlCatalog>(
    binder: &Binder<'_, C>,
    brain: BrainId,
    ops: &[BitmapOp],
) -> Option<u64> {
    let [BitmapOp::Push(handle)] = ops else {
        return None;
    };
    binder.catalog.bitmap_estimated_cardinality(brain, *handle)
}
