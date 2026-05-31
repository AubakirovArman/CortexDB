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

    window.CortexDashboardReports = { renderAnnEvaluation };
})();
