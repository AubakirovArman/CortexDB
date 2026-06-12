use crate::{cli_aql as aql_cmd, cli_ops as ops};

use super::super::args::{ContextOutputFormat, VerificationOutputFormat};
use super::DispatchContext;

pub(super) fn context(
    ctx: DispatchContext<'_>,
    path: String,
    scope: String,
    aql: String,
    format: ContextOutputFormat,
) -> Result<String, String> {
    ops::context(
        ctx.resolve(&path).to_str().unwrap(),
        &scope,
        &aql,
        ctx.json,
        format.as_str(),
    )
}

pub(super) fn remember(
    ctx: DispatchContext<'_>,
    path: String,
    scope: String,
    aql: String,
) -> Result<String, String> {
    ops::remember(ctx.resolve(&path).to_str().unwrap(), &scope, &aql, ctx.json)
}

pub(super) fn forget(
    ctx: DispatchContext<'_>,
    path: String,
    cell_id: String,
) -> Result<String, String> {
    ops::forget(ctx.resolve(&path).to_str().unwrap(), &cell_id, ctx.json)
}

pub(super) fn verify(
    ctx: DispatchContext<'_>,
    path: String,
    scope: String,
    aql: String,
    format: VerificationOutputFormat,
) -> Result<String, String> {
    ops::verify(
        ctx.resolve(&path).to_str().unwrap(),
        &scope,
        &aql,
        ctx.json,
        format.as_str(),
    )
}

pub(super) fn aql(
    ctx: DispatchContext<'_>,
    path: String,
    scope: String,
    aql: String,
) -> Result<String, String> {
    aql_cmd::aql(ctx.resolve(&path).to_str().unwrap(), &scope, &aql, ctx.json)
}
