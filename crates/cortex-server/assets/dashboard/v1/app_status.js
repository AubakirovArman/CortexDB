async function safeStatusCheck(label, path) {
    try {
        const body = await api(path);
        return { label, ok: true, body };
    } catch (error) {
        return { label, ok: false, error };
    }
}

async function loadOperationalStatus() {
    const checks = [["health", "/v1/health"], ["compatibility", "/v1/compatibility"]];
    if (canUse(ACCESS_ADMIN)) {
        checks.push(["stats", "/v1/stats"], ["validate", "/v1/validate"], ["metrics", "/v1/metrics"]);
    } else if (canUse(ACCESS_DATA)) {
        checks.push(["cell read", "/v1/cell?cell_id=0"]);
    }
    const results = await Promise.all(checks.map(([label, path]) => safeStatusCheck(label, path)));
    const health = summarizeStatusResult(results, "health");
    const compatibility = summarizeCompatibilityResult(results);
    const stats = summarizeStatsResult(results);
    const validation = summarizeValidationResult(results);
    const metrics = summarizeMetricsResult(results);
    const backup = backupPosture(results);
    const incidents = results
        .filter((result) => !result.ok)
        .map((result) => ({
            label: result.label,
            message: errorMessage(result.error),
            code: result.error?.code || result.error?.status || "request_error",
            status: Number(result.error?.http_status || 0),
        }));
    return {
        schema_version: "dashboard_status.v1",
        tenant,
        access_level: accessLevel,
        read_only: readOnlyMode,
        results,
        incidents,
        incident_timeline: buildIncidentTimeline({ results, incidents, validation, metrics, backup }),
        incident_view: buildIncidentView({ incidents, validation, metrics, backup }),
        health,
        compatibility,
        stats,
        validation,
        metrics,
        backup_posture: backup,
        backup_restore_view: buildBackupRestoreView({ metrics, validation, backup }),
        slo_dashboard: buildSloDashboard({ health, compatibility, stats, validation, metrics, backup, incidents }),
        last_request_error: lastRequestIssue,
    };
}

function summarizeCompatibilityResult(results) {
    const result = resultByLabel(results, "compatibility");
    const body = result?.body || {};
    const api = body.api || {};
    const sdk = body.sdk || {};
    const migration = body.migration || {};
    const storageFormats = body.storage_formats || [];
    return {
        available: !!result,
        ok: !!result?.ok && body.schema_version === "cortexdb.compatibility.v1",
        schema_version: body.schema_version || null,
        api_version: api.version || null,
        api_contract: api.contract || null,
        sdk_contract: sdk.contract || null,
        sdk_workspace_version: sdk.workspace_version || null,
        storage_formats: storageFormats,
        migration_current_release: migration.current_release || null,
        migration_release: migration.release || null,
        migration_gate: migration.gate || null,
        message: result?.ok ? "ok" : (result ? errorMessage(result.error) : "not checked"),
    };
}
