(() => {
    const reports = window.CortexDashboardReports || {};
    const { card, textItem, yesNo } = reports.helpers || {};
    if (!card || !textItem || !yesNo) return;

    function renderOperationalStatus(body) {
        const container = document.querySelector("#status-report");
        if (!container || body?.schema_version !== "dashboard_status.v1") return;

        const results = body.results || [];
        const incidents = body.incidents || [];
        const timeline = body.incident_timeline || [];
        const incidentView = body.incident_view || {};
        const summary = document.createElement("div");
        const details = document.createElement("div");
        const incidentList = document.createElement("ul");
        const incidentViewList = document.createElement("ul");
        const timelineList = document.createElement("div");
        const backupList = document.createElement("ul");
        const checkList = document.createElement("ul");
        summary.className = "report-grid";
        details.className = "report-grid";
        incidentList.className = "report-list compact";
        incidentViewList.className = "report-list compact";
        timelineList.className = "report-list";
        backupList.className = "report-list compact";
        checkList.className = "report-list compact";

        const health = body.health || {};
        const compatibility = body.compatibility || {};
        const stats = body.stats || {};
        const validation = body.validation || {};
        const metrics = body.metrics || {};
        const backup = body.backup_posture || {};
        const backupRestore = body.backup_restore_view || {};
        const lastError = body.last_request_error || null;
        const validationErrors = validation.errors || [];
        const backupCommands = backup.commands || [];
        const queueDepth = Number(metrics.actor_queue_depth || 0);
        const queueCapacity = Number(metrics.actor_queue_capacity || 0);
        const queueRatio = queueCapacity > 0 ? queueDepth / queueCapacity : 0;
        const queueTone = queueRatio >= 1 ? "bad" : (queueRatio >= 0.8 ? "warn" : "good");
        const backupAge = Number(metrics.backup_latest_age_seconds ?? -1);
        const backupAgeKnown = Number.isFinite(backupAge) && backupAge >= 0;
        const backupTone = backupAgeKnown ? (backupAge > 86400 ? "warn" : "good") : "warn";

        summary.replaceChildren(
            card("Tenant", body.tenant || "default"),
            card("Access", body.access_level || "limited"),
            card("Read-only", yesNo(body.read_only), body.read_only ? "warn" : "good"),
            card("Health", health.ok ? "ok" : health.message || "not checked", health.ok ? "good" : "bad"),
            card("Compatibility", compatibility.ok ? "ok" : compatibility.message || "not checked", compatibility.ok ? "good" : "bad"),
            card("Stats", stats.ok ? "ok" : stats.message || "not checked", stats.ok ? "good" : "warn"),
            card("Validation", validation.ok ? "ok" : validation.message || "not checked", validation.ok ? "good" : "bad"),
            card("Metrics", metrics.ok ? "ok" : metrics.message || "not checked", metrics.ok ? "good" : "warn"),
            card("Backup posture", backup.available ? backup.mode || "operator_cli" : "admin required", backup.available ? "warn" : "bad"),
            card("Actor queue", queueCapacity > 0 ? `${queueDepth}/${queueCapacity}` : "n/a", queueTone),
            card("Latest backup", backupAgeKnown ? formatAge(backupAge) : "unknown", backupTone),
            card("Restore drill", backupRestore.restore_drill?.status || "unknown", backupRestore.restore_drill?.status === "operator_gate_ready" ? "good" : "warn"),
            card("Offsite status", backupRestore.offsite?.status || "unknown", backupRestore.offsite?.status === "operator_gate_ready" ? "good" : "warn"),
            card("RPO/RTO", backupRestore.rpo_rto?.rpo_status || "unknown", backupRestore.rpo_rto?.rpo_status === "within_budget" ? "good" : "warn"),
            card("Incident errors", incidentView.errors?.count ?? incidents.length, incidentView.errors?.count ? "bad" : "good"),
            card("Rate limits", incidentView.rate_limits?.status || "unknown", incidentView.rate_limits?.status === "attention" ? "warn" : "good"),
            card("Actor busy", incidentView.actor_busy?.status || "unknown", incidentView.actor_busy?.status === "busy" ? "bad" : (incidentView.actor_busy?.status === "near_capacity" ? "warn" : "good")),
            card("Storage warnings", incidentView.storage_warnings?.count ?? validationErrors.length, incidentView.storage_warnings?.count ? "bad" : "good"),
            card("Backup failures", incidentView.backup_failures?.count ?? 0, incidentView.backup_failures?.count ? "bad" : "good"),
            card("Checks", results.length),
            card("Incidents", incidents.length, incidents.length ? "bad" : "good"),
            card("Timeline events", timeline.length, timeline.length ? "warn" : "good"),
            card("Last error", lastError ? `${lastError.label}: ${lastError.message}` : "none", lastError ? "bad" : "good"),
        );

        details.replaceChildren(
            card("Current seq", stats.current_seq ?? "n/a"),
            card("Checkpoint seq", stats.checkpoint_seq ?? "n/a"),
            card("Live segments", stats.live_segments ?? "n/a"),
            card("Retired segments", stats.retired_segments ?? "n/a"),
            card("MemTable cells", stats.memtable_cells ?? "n/a"),
            card("WAL bytes", stats.wal_size_bytes ?? "n/a"),
            card("Actor queue depth", metrics.actor_queue_depth ?? "n/a", queueTone),
            card("Actor queue capacity", metrics.actor_queue_capacity ?? "n/a"),
            card("Request count", metrics.request_count ?? "n/a"),
            card("Latest backup age", backupAgeKnown ? `${backupAge}s` : "unknown", backupTone),
            card("Manifest", validation.manifest_ok === null ? "n/a" : yesNo(validation.manifest_ok), validation.manifest_ok === false ? "bad" : "good"),
            card("WAL validation", validation.wal_ok === null ? "n/a" : yesNo(validation.wal_ok), validation.wal_ok === false ? "bad" : "good"),
            card("API version", compatibility.api_version || "n/a"),
            card("SDK contract", compatibility.sdk_contract || "n/a"),
            card("Storage formats", compatibility.storage_formats?.length ?? "n/a"),
            card("Migration release", compatibility.migration_current_release || "n/a"),
        );

        incidentViewList.replaceChildren(...incidentViewRows(incidentView).map(textItem));

        if (incidents.length) {
            incidentList.replaceChildren(...incidents.map((item) => textItem(`${item.label}: ${item.message}`)));
        } else {
            incidentList.replaceChildren(textItem("No dashboard-visible incidents reported"));
        }

        if (timeline.length) {
            timelineList.replaceChildren(...timeline.map(renderIncidentEvent));
        } else {
            timelineList.replaceChildren(card("Incident timeline", "No audit/rate/storage/backup events", "good"));
        }

        if (backupCommands.length) {
            backupList.replaceChildren(
                textItem(`${backup.evidence_gate || "backup evidence gate"} proves restore posture outside the browser.`),
                textItem(`latest backup: ${backupRestore.latest_backup?.status || "unknown"} (${backupRestore.latest_backup?.age_seconds ?? "unknown"}s)`),
                textItem(`restore drill: ${backupRestore.restore_drill?.command || "cortexdb backup-drill"} via ${backupRestore.restore_drill?.evidence_gate || "make backup-restore-production-pack-check"}`),
                textItem(`offsite status: ${backupRestore.offsite?.command || "cortexdb backup-offsite-stage"} via ${backupRestore.offsite?.evidence_gate || "make backup-offsite-check"}`),
                textItem(`RPO/RTO: RPO ${backupRestore.rpo_rto?.rpo_budget_seconds || 86400}s, RTO ${backupRestore.rpo_rto?.rto_status || "drill_required_for_release"}`),
                ...backupCommands.map((item) => textItem(item)),
                textItem(backup.message || "Backups are operator-controlled actions."),
            );
        } else {
            backupList.replaceChildren(
                textItem(backup.message || "Backup posture not checked"),
                textItem(`restore drill: ${backupRestore.restore_drill?.evidence_gate || "make backup-restore-production-pack-check"}`),
                textItem(`offsite status: ${backupRestore.offsite?.evidence_gate || "make backup-offsite-check"}`),
                textItem(`RPO/RTO: RPO ${backupRestore.rpo_rto?.rpo_budget_seconds || 86400}s, RTO ${backupRestore.rpo_rto?.rto_status || "drill_required_for_release"}`),
            );
        }

        if (validationErrors.length) {
            checkList.replaceChildren(...validationErrors.map((item) => textItem(`validation: ${item}`)));
        } else if (results.length) {
            checkList.replaceChildren(...results.map((item) => textItem(`${item.ok ? "ok" : "err"} ${item.label}`)));
        } else {
            checkList.replaceChildren(textItem("No status checks were run"));
        }

        const compatibilityList = renderCompatibilityList(compatibility);

        container.replaceChildren(
            summary,
            details,
            card("Version compatibility", "API / SDK / storage / migration", compatibility.ok ? "good" : "bad"),
            compatibilityList,
            checkList,
            backupList,
            card("Incident view", "errors / rate limits / actor busy / storage warnings / backup failures", incidentView.backup_failures?.count || incidentView.storage_warnings?.count ? "bad" : "good"),
            incidentViewList,
            card("Incident timeline", "audit / rate / storage / backup", timeline.length ? "warn" : "good"),
            timelineList,
            incidentList,
        );
    }

    function renderCompatibilityList(compatibility) {
        const list = document.createElement("ul");
        const formats = compatibility.storage_formats || [];
        list.className = "report-list compact";
        if (!compatibility.available) {
            list.replaceChildren(textItem("Compatibility endpoint was not checked"));
            return list;
        }
        const rows = [
            textItem(`api: ${compatibility.api_version || "unknown"} / ${compatibility.api_contract || "unknown"}`),
            textItem(`sdk: ${compatibility.sdk_contract || "unknown"} / workspace ${compatibility.sdk_workspace_version || "unknown"}`),
            textItem(`migration: ${compatibility.migration_release || "unknown"} -> ${compatibility.migration_current_release || "unknown"} via ${compatibility.migration_gate || "unknown"}`),
            ...formats.map((format) => textItem(`${format.extension}: ${format.current_magic} v${format.current_version} (${format.compatibility_rule})`)),
        ];
        list.replaceChildren(...rows);
        return list;
    }

    function incidentViewRows(view) {
        if (!view || view.schema_version !== "dashboard_incident_view.v1") {
            return ["Incident view not available"];
        }
        const rate = view.rate_limits || {};
        const actor = view.actor_busy || {};
        const storage = view.storage_warnings || {};
        const backup = view.backup_failures || {};
        const rows = [
            `errors: ${view.errors?.status || "unknown"} (${view.errors?.count ?? 0})`,
            `rate limits: ${rate.status || "unknown"}; request rejected ${rate.request_rejected ?? 0}; quota rejected ${rate.quota_rejected ?? 0}`,
            `actor busy: ${actor.status || "unknown"}; queue ${actor.queue_depth ?? 0}/${actor.queue_capacity ?? 0}`,
            `storage warnings: ${storage.status || "unknown"} (${storage.count ?? 0})`,
            `backup failures: ${backup.status || "unknown"} (${backup.count ?? 0})`,
        ];
        for (const event of storage.events || []) {
            rows.push(`storage warning: ${event.code || "warning"} - ${event.message || "review storage validation"}`);
        }
        for (const event of backup.events || []) {
            rows.push(`backup failure: ${event.code || "warning"} - ${event.message || "review backup evidence"}`);
        }
        for (const action of view.operator_actions || []) {
            rows.push(`operator action: ${action}`);
        }
        return rows;
    }

    function formatAge(seconds) {
        if (seconds < 60) return `${seconds}s`;
        if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
        if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
        return `${Math.floor(seconds / 86400)}d`;
    }

    function renderIncidentEvent(event) {
        const item = document.createElement("article");
        const title = document.createElement("h4");
        const meta = document.createElement("p");
        const action = document.createElement("p");
        item.className = "report-item";
        item.dataset.category = event.category || "unknown";
        item.dataset.severity = event.severity || "info";
        title.textContent = `${event.category || "incident"} · ${event.label || "event"}`;
        meta.className = "report-meta";
        meta.textContent = `${event.severity || "info"} · ${event.source || "dashboard"} · ${event.message || "no message"}`;
        action.textContent = `Action: ${event.action || "Review the event and run the matching operator check."}`;
        item.append(title, meta, action);
        return item;
    }

    Object.assign(reports, {
        renderOperationalStatus,
    });
    window.CortexDashboardReports = reports;
})();
