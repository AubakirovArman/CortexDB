(() => {
    const reports = window.CortexDashboardReports || {};
    const { card, preview, q16Percent, sourceLabel, textItem, yesNo } = reports.helpers || {};
    if (!card || !preview || !q16Percent || !sourceLabel || !textItem || !yesNo) return;

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
        const exclusion = anomaly.why_excluded ? ` · ${anomaly.why_excluded}` : "";
        item.textContent = anomaly.cell_id
            ? `Cell ${anomaly.cell_id}: ${anomaly.code} - ${anomaly.message}${exclusion}`
            : `${anomaly.code} - ${anomaly.message}${exclusion}`;
        return item;
    }

    function citationItem(cell) {
        const item = document.createElement("li");
        const source = sourceLabel(cell.source_ref) || "source_ref missing";
        const citation = cell.citation || "citation missing";
        item.textContent = `Cell ${cell.cell_id}: ${citation} · ${source}`;
        return item;
    }

    function explainItem(cell) {
        const item = document.createElement("article");
        const title = document.createElement("h4");
        const meta = document.createElement("p");
        const components = document.createElement("ul");
        const explain = cell.explain || {};
        const scoreComponents = explain.score_components || [];

        item.className = "report-item";
        title.textContent = `Cell ${cell.cell_id} explain`;
        meta.className = "report-meta";
        meta.textContent = [
            `score: ${explain.score ?? "n/a"}`,
            `base bm25: ${explain.base_bm25 ?? "n/a"}`,
            `source trust: ${explain.source_trust_category || "unknown"} ${q16Percent(explain.source_trust_q16)}`,
            `bonus: ${explain.source_trust_bonus ?? "n/a"}`,
            `redundancy penalty: ${explain.redundancy_penalty ?? "n/a"}`,
        ].join(" · ");
        components.className = "report-list compact";
        if (scoreComponents.length) {
            components.replaceChildren(...scoreComponents.map((component) => textItem(
                `${component.name}: value=${component.value} contribution=${component.contribution} - ${component.reason}`,
            )));
        } else {
            components.replaceChildren(textItem(explain.why_selected || "No score components reported"));
        }

        item.append(title, meta, components);
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

    function guardItem(guard) {
        const prefix = guard.cell_id ? `Cell ${guard.cell_id}` : "Global";
        return textItem(`${prefix}: ${guard.code} - ${guard.message}`);
    }

    function numericConflictItem(conflict) {
        return textItem(`${conflict.metric}: ${conflict.left} vs ${conflict.right}`);
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

        const evidence = body.supporting || body.evidence || [];
        const contradicting = body.contradicting || body.contradicting_evidence || [];
        const guards = body.guards || [];
        const numericConflicts = body.numeric_conflicts || [];
        const verdictTone = body.verdict === "supported" ? "good" : body.verdict === "mixed_evidence" ? "warn" : "bad";
        const summary = document.createElement("div");
        const supportList = document.createElement("div");
        const contradictionList = document.createElement("div");
        const guardList = document.createElement("ul");
        const conflictList = document.createElement("ul");
        summary.className = "report-grid";
        supportList.className = "report-list";
        contradictionList.className = "report-list";
        guardList.className = "report-list compact";
        conflictList.className = "report-list compact";

        summary.replaceChildren(
            card("Verdict", body.verdict, verdictTone),
            card("Status", body.status || body.verdict, verdictTone),
            card("Supporting", evidence.length, evidence.length ? "good" : "warn"),
            card("Contradicting", contradicting.length, contradicting.length ? "bad" : "good"),
            card("Guards", guards.length, guards.length ? "warn" : "good"),
            card("Numeric conflicts", numericConflicts.length, numericConflicts.length ? "bad" : "good"),
            card("Mixed evidence", yesNo(evidence.length > 0 && contradicting.length > 0), evidence.length > 0 && contradicting.length > 0 ? "warn" : "good"),
        );

        if (evidence.length) supportList.replaceChildren(...evidence.slice(0, 5).map(evidenceItem));
        else supportList.replaceChildren(card("Supporting evidence", "none", "warn"));

        if (contradicting.length) contradictionList.replaceChildren(...contradicting.slice(0, 5).map(evidenceItem));
        else contradictionList.replaceChildren(card("Contradicting evidence", "none", "good"));

        if (numericConflicts.length) conflictList.replaceChildren(...numericConflicts.map(numericConflictItem));
        else conflictList.replaceChildren(textItem("No numeric conflicts reported"));

        if (guards.length) guardList.replaceChildren(...guards.map(guardItem));
        else guardList.replaceChildren(textItem("No guards reported"));

        container.replaceChildren(
            card("Explorer", "Mixed evidence / Numeric conflict / Guard explorer", "good"),
            summary,
            card("Fact", body.fact),
            card("Supporting evidence", `${evidence.length} cells`, evidence.length ? "good" : "warn"),
            supportList,
            card("Contradicting evidence", `${contradicting.length} cells`, contradicting.length ? "bad" : "good"),
            contradictionList,
            card("Numeric conflict explorer", `${numericConflicts.length} conflicts`, numericConflicts.length ? "bad" : "good"),
            conflictList,
            card("Guard explorer", `${guards.length} guards`, guards.length ? "warn" : "good"),
            guardList,
        );
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
        const citationList = document.createElement("ul");
        const explainList = document.createElement("div");
        const anomalyList = document.createElement("ul");
        summary.className = "report-grid";
        cellList.className = "report-list";
        citationList.className = "report-list compact";
        explainList.className = "report-list";
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
            card("Explain rows", cells.filter((cell) => cell.explain).length),
            card("Source refs", cells.filter((cell) => cell.source_ref).length),
        );

        if (cells.length) cellList.replaceChildren(...cells.slice(0, 5).map(cellItem));
        else cellList.replaceChildren(card("Cells", "none"));

        if (cells.length) citationList.replaceChildren(...cells.map(citationItem));
        else citationList.replaceChildren(textItem("No ContextPack cells to cite"));

        const explainedCells = cells.filter((cell) => cell.explain);
        if (explainedCells.length) explainList.replaceChildren(...explainedCells.slice(0, 5).map(explainItem));
        else explainList.replaceChildren(card("Explain", "none", "warn"));

        if (anomalies.length) anomalyList.replaceChildren(...anomalies.map(anomalyItem));
        else anomalyList.replaceChildren(anomalyItem({ code: "none", message: "No anomalies reported" }));

        container.replaceChildren(
            card("Explorer", "Context cells / Citation explorer / Explain explorer / Anomaly explorer", "good"),
            summary,
            card("Context cells", `${cells.length} selected`),
            cellList,
            card("Citation explorer", `${citations}/${cells.length} cited`, citationTone),
            citationList,
            card("Explain explorer", `${explainedCells.length} cells explained`),
            explainList,
            card("Anomaly explorer", `${anomalies.length} anomalies`, anomalyTone),
            anomalyList,
        );
    }

    Object.assign(reports, {
        renderAqlReport,
        renderContextPack,
        renderSearchReport,
        renderVerificationReport,
    });
    window.CortexDashboardReports = reports;
})();
