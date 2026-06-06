(() => {
    const reports = window.CortexDashboardReports || {};
    const { card, textItem, yesNo } = reports.helpers || {};
    if (!card || !textItem || !yesNo) return;

    function tone(ok, neutralWhenUnknown = false) {
        if (ok === true) return "good";
        if (ok === false) return "bad";
        return neutralWhenUnknown ? "" : "warn";
    }

    function formatAge(seconds) {
        if (seconds === null || seconds === undefined) return "unknown";
        if (seconds < 60) return `${seconds}s`;
        if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
        if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
        return `${Math.floor(seconds / 86400)}d`;
    }

    function renderSloDashboard(body) {
        const container = document.querySelector("#slo-report");
        const slo = body?.slo_dashboard;
        if (!container || slo?.schema_version !== "dashboard_slo.v1") return;

        const availability = slo.availability || {};
        const latency = slo.latency || {};
        const backup = slo.backup_freshness || {};
        const validation = slo.validation_status || {};
        const errorBudget = slo.error_budget || {};
        const stats = slo.stats_context || {};
        const actions = slo.operator_actions || [];
        const actionList = document.createElement("ul");
        const contextGrid = document.createElement("div");
        actionList.className = "report-list compact";
        contextGrid.className = "report-grid";

        const cards = [
            card("Availability", availability.signal || yesNo(availability.ok), tone(availability.ok)),
            card("Latency", latency.signal || "not checked", tone(latency.ok, true)),
            card("Backup freshness", backup.signal || "not checked", tone(backup.ok)),
            card("Validation status", validation.signal || "not checked", tone(validation.ok)),
            card("Error budget", errorBudget.signal || "not checked", tone(errorBudget.ok)),
        ];

        contextGrid.replaceChildren(
            card("Requests", latency.request_count ?? 0),
            card("Mean latency budget", `${latency.budget_ms ?? "n/a"} ms`),
            card("Backup age", formatAge(backup.age_seconds)),
            card("Backup gate", backup.evidence_gate || "n/a"),
            card("Validation errors", validation.errors?.length ?? 0, (validation.errors?.length ?? 0) ? "bad" : "good"),
            card("Rejected requests", errorBudget.request_rejected ?? 0, errorBudget.request_rejected ? "bad" : "good"),
            card("Quota rejects", errorBudget.quota_rejected ?? 0, errorBudget.quota_rejected ? "bad" : "good"),
            card("Validation failures", errorBudget.validation_failures ?? 0, errorBudget.validation_failures ? "bad" : "good"),
            card("Current seq", stats.current_seq ?? "n/a"),
            card("Checkpoint seq", stats.checkpoint_seq ?? "n/a"),
            card("Live segments", stats.live_segments ?? "n/a"),
            card("WAL bytes", stats.wal_size_bytes ?? "n/a"),
        );

        if (actions.length) {
            actionList.replaceChildren(...actions.map(textItem));
        } else {
            actionList.replaceChildren(textItem("No operator actions reported"));
        }

        container.replaceChildren(
            ...cards,
            card("SLO source", "dashboard_slo.v1", "good"),
            contextGrid,
            actionList,
        );
    }

    Object.assign(reports, { renderSloDashboard });
    window.CortexDashboardReports = reports;
})();
