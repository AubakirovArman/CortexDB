use crate::cli_ingest as ingest;

use super::DispatchContext;

pub(super) fn load_fixture(
    ctx: DispatchContext<'_>,
    path: String,
    fixture_path: String,
) -> Result<String, String> {
    ingest::load_fixture(&ctx.resolve_string(&path), &fixture_path)
}

pub(super) fn text(
    ctx: DispatchContext<'_>,
    path: String,
    scope: String,
    file: String,
) -> Result<String, String> {
    ingest::text(&ctx.resolve_string(&path), &scope, &file)
}

pub(super) fn json(
    ctx: DispatchContext<'_>,
    path: String,
    scope: String,
    file: String,
) -> Result<String, String> {
    ingest::json(&ctx.resolve_string(&path), &scope, &file)
}

pub(super) fn csv(
    ctx: DispatchContext<'_>,
    path: String,
    scope: String,
    file: String,
) -> Result<String, String> {
    ingest::csv(&ctx.resolve_string(&path), &scope, &file)
}

pub(super) fn jobs(ctx: DispatchContext<'_>, path: String) -> Result<String, String> {
    ingest::jobs(&ctx.resolve_string(&path), ctx.json)
}

pub(super) fn job(ctx: DispatchContext<'_>, path: String, job_id: u64) -> Result<String, String> {
    ingest::job(&ctx.resolve_string(&path), job_id, ctx.json)
}

pub(super) fn cancel_job(
    ctx: DispatchContext<'_>,
    path: String,
    job_id: u64,
) -> Result<String, String> {
    ingest::cancel_job(&ctx.resolve_string(&path), job_id, ctx.json)
}

pub(super) fn retry_job(
    ctx: DispatchContext<'_>,
    path: String,
    job_id: u64,
) -> Result<String, String> {
    ingest::retry_job(&ctx.resolve_string(&path), job_id, ctx.json)
}

pub(super) fn delete_job(
    ctx: DispatchContext<'_>,
    path: String,
    job_id: u64,
) -> Result<String, String> {
    ingest::delete_job(&ctx.resolve_string(&path), job_id)
}
