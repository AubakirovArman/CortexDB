use std::str::FromStr;

use crate::ast::{AqlString, Comparator, Condition, Literal, Spanned};
use crate::policy::PolicyError;
use crate::types::MemoryType;

use super::{AqlCatalog, BindError, Binder, BitmapOp};

pub(super) fn compile_condition<C: AqlCatalog>(
    binder: &Binder<'_, C>,
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
            field.node.value.as_ref(),
            comparator.node,
            &literal.node,
            ops,
        ),
        Condition::Not(child) => {
            compile_condition(binder, &child.node, ops)?;
            ops.push(BitmapOp::Not);
            Ok(())
        }
        Condition::And(lhs, rhs) => compile_binary(binder, lhs, rhs, BitmapOp::And, ops),
        Condition::Or(lhs, rhs) => compile_binary(binder, lhs, rhs, BitmapOp::Or, ops),
    }
}

fn compile_binary<C: AqlCatalog>(
    binder: &Binder<'_, C>,
    lhs: &Spanned<Condition<'_>>,
    rhs: &Spanned<Condition<'_>>,
    op: BitmapOp,
    ops: &mut Vec<BitmapOp>,
) -> Result<(), BindError> {
    compile_condition(binder, &lhs.node, ops)?;
    compile_condition(binder, &rhs.node, ops)?;
    ops.push(op);
    Ok(())
}

fn compile_predicate<C: AqlCatalog>(
    binder: &Binder<'_, C>,
    field: &str,
    comparator: Comparator,
    literal: &Literal<'_>,
    ops: &mut Vec<BitmapOp>,
) -> Result<(), BindError> {
    if comparator != Comparator::Eq {
        return Err(BindError::UnsupportedComparator);
    }
    if !binder.catalog.field_is_filterable(field) {
        return Err(BindError::FieldNotFilterable(field.to_owned()));
    }
    match field {
        "space" | "scope" => compile_scope(binder, literal, ops),
        "status" => compile_status(binder, literal, ops),
        "cell_type" | "memory_type" => compile_memory_type(binder, literal, ops),
        _ => Err(BindError::FieldNotFilterable(field.to_owned())),
    }
}

fn compile_scope<C: AqlCatalog>(
    binder: &Binder<'_, C>,
    literal: &Literal<'_>,
    ops: &mut Vec<BitmapOp>,
) -> Result<(), BindError> {
    let scope_name = string_literal(literal)?;
    let scope = binder
        .catalog
        .resolve_scope(scope_name.value.as_ref())
        .ok_or(BindError::UnknownScope)?;
    if !binder.view.can_read_scope(scope) {
        return Err(BindError::PolicyDenied(PolicyError::ScopeNotReadable));
    }
    ops.push(BitmapOp::Push(
        binder
            .catalog
            .scope_bitmap(scope)
            .ok_or(BindError::UnknownBitmap)?,
    ));
    Ok(())
}

fn compile_status<C: AqlCatalog>(
    binder: &Binder<'_, C>,
    literal: &Literal<'_>,
    ops: &mut Vec<BitmapOp>,
) -> Result<(), BindError> {
    let status = string_literal(literal)?;
    ops.push(BitmapOp::Push(
        binder
            .catalog
            .status_bitmap(status.value.as_ref())
            .ok_or(BindError::UnknownBitmap)?,
    ));
    Ok(())
}

fn compile_memory_type<C: AqlCatalog>(
    binder: &Binder<'_, C>,
    literal: &Literal<'_>,
    ops: &mut Vec<BitmapOp>,
) -> Result<(), BindError> {
    let value = string_literal(literal)?;
    let memory_type =
        MemoryType::from_str(value.value.as_ref()).map_err(|_| BindError::UnsupportedLiteral)?;
    ops.push(BitmapOp::Push(
        binder
            .catalog
            .cell_type_bitmap(memory_type)
            .ok_or(BindError::UnknownBitmap)?,
    ));
    Ok(())
}

fn string_literal<'a>(literal: &'a Literal<'a>) -> Result<&'a AqlString<'a>, BindError> {
    match literal {
        Literal::String(value) => Ok(value),
        _ => Err(BindError::UnsupportedLiteral),
    }
}
