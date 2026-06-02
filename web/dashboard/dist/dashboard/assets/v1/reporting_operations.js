(() => {
    const reports = window.CortexDashboardReports || {};
    const { card, preview, q16Percent, textItem, yesNo } = reports.helpers || {};
    if (!card || !preview || !q16Percent || !textItem || !yesNo) return;

    function requestIssueAction(status, code, requestLabel = "") {
        const label = String(requestLabel).toLowerCase();
        const adminRequest = ["stats", "metrics", "validate", "flush", "compact", "ann metrics"]
            .some((value) => label.includes(value));
        const retrievalRequest = ["search", "aql", "context", "verify", "ann evaluate"]
            .some((value) => label.includes(value));
        const cellRequest = ["cell", "ingest", "remember", "forget"].some((value) => label.includes(value));

        if (status === 401) return "Apply a valid bearer token, then retry the request.";
        if (status === 403 && adminRequest) return "Use an admin token; this storage or metrics action is hidden for data tokens.";
        if (status === 403 && retrievalRequest) return "Use a token whose AgentView can read this tenant and scope.";
        if (status === 403 && cellRequest) return "Use a token whose AgentView can write or read this cell scope.";
        if (status === 403) return "Use a token with the required data/admin role or switch to an allowed tenant/scope.";
        if (status === 400) return "Check the request fields, tenant, scope, and statement format.";
        if (status === 404) return "Check that the route or cell id exists.";
        if (status === 429) return "Wait for the rate limit window to reset, then retry.";
        if (code === "invalid_tenant") return "Check the tenant name and allowed token scope.";
        return "Review the response payload and retry after fixing the request.";
    }

    function renderRequestIssue(error, requestLabel = "") {
        const container = document.querySelector("#error-report");
        if (!container) return;

        const status = Number(error?.http_status || 0);
        const code = error?.code || error?.status || "request_error";
        const message = error?.message || error?.error || String(error || "request failed");
        const statusLabel = status ? `HTTP ${status}` : "client-side";
        const tone = status === 401 || status === 403 ? "warn" : "bad";
        const request = requestLabel || "current request";

        container.replaceChildren(
            card("Request", request, tone),
            card("Status", statusLabel, tone),
            card("Code", code, tone),
            card("Issue", message, tone),
            card("Action", requestIssueAction(status, code, requestLabel), "warn"),
        );
    }

    function clearRequestIssue() {
        const container = document.querySelector("#error-report");
        if (!container) return;
        container.replaceChildren(card("Status", "No request issue", "good"));
    }

    function renderCellReport(body, requestLabel = "") {
        const container = document.querySelector("#cell-report");
        if (!container || !body || typeof body !== "object" || Array.isArray(body)) return;

        if (body.seq !== undefined && body.cell_id !== undefined) {
            const operation = requestLabel || "cell operation";
            container.replaceChildren(
                card("Operation", operation, "good"),
                card("Cell", body.cell_id),
                card("Seq", body.seq),
            );
            return;
        }

        if (!Object.prototype.hasOwnProperty.call(body, "cell")) return;
        if (!body.cell) {
            container.replaceChildren(
                card("Lookup", "not found", "warn"),
                card("Cell", "none"),
            );
            return;
        }

        const item = document.createElement("article");
        const title = document.createElement("h4");
        const bodyText = document.createElement("p");
        item.className = "report-item";
        title.textContent = `Cell ${body.cell.cell_id}`;
        bodyText.textContent = preview(body.cell.payload);
        item.append(title, bodyText);
        container.replaceChildren(
            card("Lookup", "found", "good"),
            card("Cell", body.cell.cell_id),
            item,
        );
    }

    function renderIngestReport(body) {
        const container = document.querySelector("#ingest-report");
        if (!container || !body || typeof body !== "object" || Array.isArray(body)) return;

        const isIngestSummary = (
            body.rows_ingested !== undefined ||
            body.chunks_ingested !== undefined ||
            body.facts_ingested !== undefined
        );
        if (isIngestSummary) {
            container.replaceChildren(
                card("Rows", body.rows_ingested ?? 0),
                card("Chunks", body.chunks_ingested ?? 0),
                card("Facts", body.facts_ingested ?? 0),
                card("First cell", body.first_cell_id ?? "none", body.first_cell_id ? "good" : "warn"),
                card("Job", body.job_id ?? "none", body.job_id ? "good" : "warn"),
            );
            return;
        }

        if (body.job_id === undefined || body.label === undefined || body.status === undefined) return;
        container.replaceChildren(
            card("Job", body.job_id),
            card("Label", body.label),
            card("Status", body.status, body.status === "completed" ? "good" : "warn"),
            card("Completed", body.completed_items ?? 0),
            card("Failed", body.failed_items ?? 0, body.failed_items ? "bad" : "good"),
            card("Last cell", body.last_cell_id ?? "none"),
        );
    }

    function renderClusterReport(body) {
        const container = document.querySelector("#cluster-report");
        if (!container || body?.distributed_enabled === undefined || !Array.isArray(body.nodes)) return;

        const summary = document.createElement("div");
        const nodeList = document.createElement("div");
        summary.className = "report-grid";
        nodeList.className = "report-list";

        summary.replaceChildren(
            card("Distributed", yesNo(body.distributed_enabled), body.distributed_enabled ? "good" : "warn"),
            card("Local node", body.local_node ?? "n/a"),
            card("Replication", body.replication_factor ?? "n/a"),
            card("Nodes", body.nodes.length, body.nodes.length ? "good" : "warn"),
        );

        if (body.nodes.length) {
            nodeList.replaceChildren(...body.nodes.map((node) => {
                const item = document.createElement("article");
                const title = document.createElement("h4");
                const meta = document.createElement("p");
                item.className = "report-item";
                title.textContent = `Node ${node.id}`;
                meta.className = "report-meta";
                meta.textContent = node.address || "no address";
                item.append(title, meta);
                return item;
            }));
        } else {
            nodeList.replaceChildren(card("Nodes", "none", "warn"));
        }

        container.replaceChildren(summary, nodeList);
    }

    function renderStorageValidation(body) {
        const container = document.querySelector("#storage-report");
        if (!container || body?.manifest_ok === undefined || body?.wal_ok === undefined) return;

        const errors = body.errors || [];
        const indexesChecked = [
            body.bitmap_indexes_checked || 0,
            body.lexical_indexes_checked || 0,
            body.vector_indexes_checked || 0,
            body.hnsw_graphs_checked || 0,
        ].reduce((total, value) => total + value, 0);
        const summary = document.createElement("div");
        const errorList = document.createElement("ul");
        summary.className = "report-grid";
        errorList.className = "report-list compact";

        summary.replaceChildren(
            card("Storage", body.ok ? "ok" : "attention", body.ok ? "good" : "bad"),
            card("Manifest", yesNo(body.manifest_ok), body.manifest_ok ? "good" : "bad"),
            card("WAL", yesNo(body.wal_ok), body.wal_ok ? "good" : "bad"),
            card("Live segments", body.live_segments_checked ?? 0),
            card("Cells", body.cells_checked ?? 0),
            card("Indexes", indexesChecked),
            card("WAL records", body.wal_records_checked ?? 0),
            card("Safe truncate", body.wal_safe_truncate_offset ?? 0),
            card("Errors", errors.length, errors.length ? "bad" : "good"),
        );

        if (errors.length) errorList.replaceChildren(...errors.map(textItem));
        else errorList.replaceChildren(textItem("No validation errors reported"));

        container.replaceChildren(summary, errorList);
    }

    function renderAnnEvaluation(body) {
        const container = document.querySelector("#ann-report");
        if (!container || body?.available === undefined) return;

        const report = body.ann_report || {};
        const availableTone = body.available ? "good" : "bad";
        const safeTone = report.production_safe ? "good" : "bad";
        const fallbackTone = report.fallback_performed ? "warn" : "good";
        const violations = report.slo_violations?.length ? report.slo_violations.join(", ") : "none";

        container.replaceChildren(
            card("Available", yesNo(body.available), availableTone),
            card("Production safe", yesNo(report.production_safe), safeTone),
            card("Recall", q16Percent(body.recall_q16)),
            card("Path", report.path || body.reason || "n/a"),
            card("Fallback", yesNo(report.fallback_performed), fallbackTone),
            card("SLO violations", violations, violations === "none" ? "good" : "bad"),
            card("Graph nodes", report.graph_nodes ?? "n/a"),
            card("Upper edges", report.upper_graph_edges ?? "n/a"),
            card("Visited", report.visited_candidates ?? "n/a"),
            card("Returned", report.returned_candidates ?? body.ann_top_k?.length ?? "n/a"),
            card("HNSW M", report.hnsw_max_neighbors ?? "n/a"),
            card("ef search", report.hnsw_ef_search ?? "n/a"),
            card("ef construction", report.hnsw_ef_construction ?? "n/a"),
            card("Layers", report.hnsw_layer_count ?? "n/a"),
            card("Exact top-k", body.exact_top_k?.join(", ") || "none"),
            card("ANN top-k", body.ann_top_k?.join(", ") || "none"),
        );
    }

    function renderOperationalStatus(body) {
        const container = document.querySelector("#status-report");
        if (!container || body?.schema_version !== "dashboard_status.v1") return;

        const results = body.results || [];
        const incidents = body.incidents || [];
        const timeline = body.incident_timeline || [];
        const summary = document.createElement("div");
        const details = document.createElement("div");
        const incidentList = document.createElement("ul");
        const timelineList = document.createElement("div");
        const backupList = document.createElement("ul");
        const checkList = document.createElement("ul");
        summary.className = "report-grid";
        details.className = "report-grid";
        incidentList.className = "report-list compact";
        timelineList.className = "report-list";
        backupList.className = "report-list compact";
        checkList.className = "report-list compact";

        const health = body.health || {};
        const compatibility = body.compatibility || {};
        const stats = body.stats || {};
        const validation = body.validation || {};
        const metrics = body.metrics || {};
        const backup = body.backup_posture || {};
        const lastError = body.last_request_error || null;
        const validationErrors = validation.errors || [];
        const backupCommands = backup.commands || [];

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
            card("Manifest", validation.manifest_ok === null ? "n/a" : yesNo(validation.manifest_ok), validation.manifest_ok === false ? "bad" : "good"),
            card("WAL validation", validation.wal_ok === null ? "n/a" : yesNo(validation.wal_ok), validation.wal_ok === false ? "bad" : "good"),
            card("API version", compatibility.api_version || "n/a"),
            card("SDK contract", compatibility.sdk_contract || "n/a"),
            card("Storage formats", compatibility.storage_formats?.length ?? "n/a"),
            card("Migration release", compatibility.migration_current_release || "n/a"),
        );

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
                ...backupCommands.map((item) => textItem(item)),
                textItem(backup.message || "Backups are operator-controlled actions."),
            );
        } else {
            backupList.replaceChildren(textItem(backup.message || "Backup posture not checked"));
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

    function renderPermissionsView(body) {
        const container = document.querySelector("#permissions-report");
        if (!container || body?.schema_version !== "dashboard_permissions.v1") return;

        const capabilities = body.capabilities || {};
        const agentView = body.agent_view || {};
        const selectedScopes = body.selected_scopes || [];
        const cards = document.createElement("div");
        const rules = document.createElement("ul");
        const scopeList = document.createElement("ul");
        const agentRules = document.createElement("ul");
        cards.className = "report-grid";
        rules.className = "report-list compact";
        scopeList.className = "report-list compact";
        agentRules.className = "report-list compact";
        cards.replaceChildren(
            card("Tenant", body.tenant || "default"),
            card("Role", body.role || body.access_level || "limited"),
            card("Access", body.access_level || "limited"),
            card("Token active", yesNo(body.token_active)),
            card("Token storage", body.token_storage || "none"),
            card("Token visible", yesNo(body.token_visible), body.token_visible ? "bad" : "good"),
            card("Read-only", yesNo(body.read_only), body.read_only ? "warn" : "good"),
            card("Data read", yesNo(capabilities.data_read), capabilities.data_read ? "good" : "warn"),
            card("Admin maintenance", yesNo(capabilities.admin_maintenance), capabilities.admin_maintenance ? "good" : "warn"),
            card("Local writes", yesNo(capabilities.local_writes), capabilities.local_writes ? "good" : "warn"),
            card("AgentView source", agentView.source || "unknown"),
            card("Server enforced", yesNo(agentView.server_enforced), agentView.server_enforced ? "good" : "bad"),
        );
        rules.replaceChildren(
            textItem("Public mode can load health and permissions only."),
            textItem("Data mode can run cell reads, retrieval, AQL, context, verify, ingest jobs, and cluster status."),
            textItem("Admin mode can run storage maintenance, validation, metrics, and ANN metrics."),
            textItem("Read-only mode is a local dashboard guard that blocks mutating actions before they reach the API."),
        );
        if (selectedScopes.length) {
            scopeList.replaceChildren(...selectedScopes.map((scope) => textItem(`scope probe: ${scope}`)));
        } else {
            scopeList.replaceChildren(textItem("No scope inputs are visible in the current dashboard shell"));
        }
        agentRules.replaceChildren(
            textItem(`read probe: ${agentView.readable_scope_probe || "not checked"}`),
            textItem(`write probe: ${agentView.writable_scope_probe || "not checked"}`),
            textItem(agentView.note || "AgentView policy is evaluated on the server."),
        );
        container.replaceChildren(
            card("Permissions explorer", "Token / role / scope / AgentView", "good"),
            cards,
            card("Scope probes", `${selectedScopes.length} scopes`),
            scopeList,
            card("AgentView policy", agentView.source || "unknown"),
            agentRules,
            rules,
        );
    }

    Object.assign(reports, {
        clearRequestIssue,
        renderAnnEvaluation,
        renderCellReport,
        renderClusterReport,
        renderIngestReport,
        renderOperationalStatus,
        renderPermissionsView,
        renderRequestIssue,
        renderStorageValidation,
    });
    window.CortexDashboardReports = reports;
})();
