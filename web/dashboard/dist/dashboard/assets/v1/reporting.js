(() => {
    function q16Percent(value) {
        if (typeof value !== "number") return "n/a";
        return `${((value * 100) / 65535).toFixed(2)}%`;
    }

    function yesNo(value) {
        if (value === true) return "yes";
        if (value === false) return "no";
        return "n/a";
    }

    function card(label, value, tone = "") {
        const node = document.createElement("div");
        const labelNode = document.createElement("span");
        const valueNode = document.createElement("strong");
        node.className = tone ? `metric ${tone}` : "metric";
        labelNode.textContent = label;
        valueNode.textContent = String(value);
        node.append(labelNode, valueNode);
        return node;
    }

    function sourceLabel(sourceRef) {
        if (!sourceRef) return "";
        const parts = [sourceRef.source_id, sourceRef.document_id].filter(Boolean);
        if (sourceRef.page !== null && sourceRef.page !== undefined) parts.push(`page ${sourceRef.page}`);
        if (sourceRef.json_path) parts.push(sourceRef.json_path);
        return parts.join(" · ");
    }

    function preview(text) {
        if (!text) return "no payload preview";
        const compact = String(text).replace(/\s+/g, " ").trim();
        return compact.length > 180 ? `${compact.slice(0, 177)}...` : compact;
    }

    function cellItem(cell) {
        const item = document.createElement("article");
        const title = document.createElement("h4");
        const meta = document.createElement("p");
        const explain = document.createElement("p");
        const body = document.createElement("p");
        const terms = cell.explain?.matched_terms?.join(", ") || "none";
        const reason = cell.explain?.why_selected || "selected by retrieval plan";
        const source = sourceLabel(cell.source_ref);

        item.className = "report-item";
        title.textContent = `Cell ${cell.cell_id}`;
        meta.className = "report-meta";
        meta.textContent = [
            `${cell.estimated_tokens ?? 0} tokens`,
            cell.citation ? `citation: ${cell.citation}` : "citation: missing",
            source ? `source: ${source}` : "",
        ].filter(Boolean).join(" · ");
        explain.className = "report-meta";
        explain.textContent = `matched terms: ${terms} · ${reason}`;
        body.textContent = preview(cell.payload_text);
        item.append(title, meta, explain, body);
        return item;
    }

    function anomalyItem(anomaly) {
        const item = document.createElement("li");
        item.textContent = anomaly.cell_id
            ? `Cell ${anomaly.cell_id}: ${anomaly.code} - ${anomaly.message}`
            : `${anomaly.code} - ${anomaly.message}`;
        return item;
    }

    function textItem(text) {
        const item = document.createElement("li");
        item.textContent = text;
        return item;
    }

    function searchItem(result) {
        const item = document.createElement("article");
        const title = document.createElement("h4");
        const meta = document.createElement("p");
        const body = document.createElement("p");
        item.className = "report-item";
        title.textContent = `Cell ${result.cell_id}`;
        meta.className = "report-meta";
        meta.textContent = [
            `score: ${result.score ?? 0}`,
            `lexical: ${result.lexical_score ?? 0}`,
            `vector: ${result.vector_score ?? 0}`,
        ].join(" · ");
        body.textContent = preview(result.payload || result.payload_preview);
        item.append(title, meta, body);
        return item;
    }

    function aqlItem(cell) {
        const item = document.createElement("article");
        const title = document.createElement("h4");
        const body = document.createElement("p");
        item.className = "report-item";
        title.textContent = `Cell ${cell.cell_id}`;
        body.textContent = preview(cell.payload);
        item.append(title, body);
        return item;
    }

    function evidenceItem(evidence) {
        const item = document.createElement("article");
        const title = document.createElement("h4");
        const meta = document.createElement("p");
        const body = document.createElement("p");
        item.className = "report-item";
        title.textContent = `Cell ${evidence.cell_id}`;
        meta.className = "report-meta";
        meta.textContent = [
            `matched terms: ${evidence.matched_terms ?? 0}`,
            evidence.citation ? `citation: ${evidence.citation}` : "citation: missing",
            `source trust: ${q16Percent(evidence.source_trust_q16)}`,
        ].join(" · ");
        body.textContent = preview(evidence.payload_text);
        item.append(title, meta, body);
        return item;
    }

    function renderSearchReport(body) {
        const container = document.querySelector("#search-report");
        if (!container || !body?.search_mode || !Array.isArray(body.results)) return;

        const results = body.results;
        const ann = body.ann_report;
        const topScore = results.length ? Math.max(...results.map((result) => Number(result.score || 0))) : 0;
        const summary = document.createElement("div");
        const list = document.createElement("div");
        summary.className = "report-grid";
        list.className = "report-list";

        summary.replaceChildren(
            card("Mode", body.search_mode),
            card("Results", results.length, results.length ? "good" : "warn"),
            card("Top score", topScore),
            card("ANN path", ann?.path || "n/a"),
            card("Production safe", ann ? yesNo(ann.production_safe) : "n/a", !ann || ann.production_safe ? "good" : "bad"),
            card("Fallback", ann ? yesNo(ann.fallback_performed) : "n/a", ann?.fallback_performed ? "warn" : "good"),
        );

        if (results.length) list.replaceChildren(...results.slice(0, 5).map(searchItem));
        else list.replaceChildren(card("Results", "none", "warn"));

        container.replaceChildren(summary, list);
    }

    function renderAqlReport(body) {
        const container = document.querySelector("#aql-report");
        if (!container || body?.schema_version || !Array.isArray(body?.cells)) return;
        if (body.cells.some((cell) => cell.payload === undefined)) return;

        const cells = body.cells;
        const summary = document.createElement("div");
        const list = document.createElement("div");
        summary.className = "report-grid";
        list.className = "report-list";

        summary.replaceChildren(
            card("Cells", cells.length, cells.length ? "good" : "warn"),
            card("First cell", cells[0]?.cell_id ?? "none"),
            card("Displayed", Math.min(cells.length, 5)),
        );

        if (cells.length) list.replaceChildren(...cells.slice(0, 5).map(aqlItem));
        else list.replaceChildren(card("Cells", "none", "warn"));

        container.replaceChildren(summary, list);
    }

    function renderVerificationReport(body) {
        const container = document.querySelector("#verify-report");
        if (!container || !body?.fact || !body?.verdict) return;

        const evidence = body.evidence || body.supporting || [];
        const contradicting = body.contradicting_evidence || body.contradicting || [];
        const guards = body.guards || [];
        const numericConflicts = body.numeric_conflicts || [];
        const verdictTone = body.verdict === "supported" ? "good" : body.verdict === "mixed_evidence" ? "warn" : "bad";
        const summary = document.createElement("div");
        const evidenceList = document.createElement("div");
        const guardList = document.createElement("ul");
        summary.className = "report-grid";
        evidenceList.className = "report-list";
        guardList.className = "report-list compact";

        summary.replaceChildren(
            card("Verdict", body.verdict, verdictTone),
            card("Status", body.status || body.verdict, verdictTone),
            card("Evidence", evidence.length),
            card("Contradicting", contradicting.length, contradicting.length ? "bad" : "good"),
            card("Guards", guards.length, guards.length ? "warn" : "good"),
            card("Numeric conflicts", numericConflicts.length, numericConflicts.length ? "bad" : "good"),
        );

        if (evidence.length) evidenceList.replaceChildren(...evidence.slice(0, 3).map(evidenceItem));
        else evidenceList.replaceChildren(card("Evidence", "none", "warn"));

        if (guards.length) guardList.replaceChildren(...guards.map((guard) => textItem(`${guard.code}: ${guard.message}`)));
        else guardList.replaceChildren(textItem("No guards reported"));

        container.replaceChildren(summary, card("Fact", body.fact), evidenceList, guardList);
    }

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

    function renderContextPack(body) {
        const container = document.querySelector("#context-report");
        if (!container || body?.schema_version !== "context_pack.v1") return;

        const cells = body.cells || [];
        const anomalies = body.anomalies || [];
        const citations = cells.filter((cell) => cell.citation).length;
        const budget = body.token_budget_tokens || 0;
        const estimated = body.estimated_tokens || 0;
        const used = budget > 0 ? `${Math.min(100, (estimated * 100) / budget).toFixed(1)}%` : "n/a";
        const anomalyTone = anomalies.length ? "bad" : "good";
        const citationTone = !body.citations_required || citations === cells.length ? "good" : "warn";

        const summary = document.createElement("div");
        const cellList = document.createElement("div");
        const anomalyList = document.createElement("ul");
        summary.className = "report-grid";
        cellList.className = "report-list";
        anomalyList.className = "report-list compact";

        summary.replaceChildren(
            card("Cells", cells.length),
            card("Budget", budget),
            card("Estimated", estimated),
            card("Used", used),
            card("Citations", `${citations}/${cells.length}`, citationTone),
            card("Required", yesNo(body.citations_required)),
            card("Truncated", yesNo(body.truncated), body.truncated ? "warn" : "good"),
            card("Anomalies", anomalies.length, anomalyTone),
        );

        if (cells.length) cellList.replaceChildren(...cells.slice(0, 5).map(cellItem));
        else cellList.replaceChildren(card("Cells", "none"));

        if (anomalies.length) anomalyList.replaceChildren(...anomalies.map(anomalyItem));
        else anomalyList.replaceChildren(anomalyItem({ code: "none", message: "No anomalies reported" }));

        container.replaceChildren(summary, cellList, anomalyList);
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

    window.CortexDashboardReports = {
        clearRequestIssue,
        renderAnnEvaluation,
        renderAqlReport,
        renderContextPack,
        renderRequestIssue,
        renderSearchReport,
        renderStorageValidation,
        renderVerificationReport,
    };
})();
