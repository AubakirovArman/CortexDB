function resultByLabel(results, label) {
    return results.find((result) => result.label === label) || null;
}

function summarizeStatusResult(results, label) {
    const result = resultByLabel(results, label);
    return {
        available: !!result,
        ok: !!result?.ok,
        code: result?.error?.code || result?.error?.status || null,
        message: result?.ok ? "ok" : (result ? errorMessage(result.error) : "not checked"),
    };
}

function summarizeStatsResult(results) {
    const result = resultByLabel(results, "stats");
    const body = result?.body || {};
    return {
        available: !!result,
        ok: !!result?.ok,
        current_seq: body.current_seq ?? null,
        checkpoint_seq: body.checkpoint_seq ?? null,
        live_segments: body.live_segments ?? null,
        retired_segments: body.retired_segments ?? null,
        memtable_cells: body.memtable_cells ?? null,
        wal_size_bytes: body.wal_size_bytes ?? null,
        message: result?.ok ? "ok" : (result ? errorMessage(result.error) : "admin token required"),
    };
}

function summarizeValidationResult(results) {
    const result = resultByLabel(results, "validate");
    const body = result?.body || {};
    const errors = body.errors || [];
    return {
        available: !!result,
        ok: !!result?.ok && body.ok !== false && errors.length === 0,
        manifest_ok: body.manifest_ok ?? null,
        wal_ok: body.wal_ok ?? null,
        live_segments_checked: body.live_segments_checked ?? null,
        cells_checked: body.cells_checked ?? null,
        errors,
        message: result?.ok ? (errors.length ? `${errors.length} validation errors` : "ok") : (result ? errorMessage(result.error) : "admin token required"),
    };
}

function summarizeMetricsResult(results) {
    const result = resultByLabel(results, "metrics");
    const body = result?.body || {};
    const requestCount = Number(body.request_count || 0);
    const durationTotal = Number(body.request_duration_ms_total || 0);
    const requestRejected = Number(body.request_rejected || 0);
    const quotaRejected = [
        body.principal_quota_requests_rejected,
        body.principal_quota_body_bytes_rejected,
        body.principal_quota_queue_rejected,
    ].reduce((total, value) => total + Number(value || 0), 0);
    const validationFailures = Number(body.validation_failures || 0);

    return {
        available: !!result,
        ok: !!result?.ok,
        request_count: requestCount,
        request_rejected: requestRejected,
        quota_rejected: quotaRejected,
        principal_quota_requests_rejected: Number(body.principal_quota_requests_rejected || 0),
        principal_quota_body_bytes_rejected: Number(body.principal_quota_body_bytes_rejected || 0),
        principal_quota_queue_rejected: Number(body.principal_quota_queue_rejected || 0),
        validation_failures: validationFailures,
        request_duration_ms_total: durationTotal,
        mean_latency_ms: requestCount > 0 ? durationTotal / requestCount : null,
        backup_latest_age_seconds: Number(body.backup_latest_age_seconds ?? -1),
        actor_queue_depth: Number(body.actor_queue_depth || 0),
        actor_queue_capacity: Number(body.actor_queue_capacity || 0),
        message: result?.ok ? "ok" : (result ? errorMessage(result.error) : "admin token required"),
    };
}
