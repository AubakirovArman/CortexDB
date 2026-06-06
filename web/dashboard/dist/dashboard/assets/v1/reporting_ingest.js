(() => {
    const reports = window.CortexDashboardReports || {};
    const { card, q16Percent, textItem } = reports.helpers || {};
    if (!card || !q16Percent || !textItem) return;

    function toneForStatus(status) {
        if (status === "completed") return "good";
        if (status === "failed") return "bad";
        if (status === "cancelled") return "warn";
        return "warn";
    }

    function valueOrNone(value) {
        if (value === null || value === undefined || value === "") return "none";
        return value;
    }

    function progressLabel(job) {
        const completed = job.completed_items ?? 0;
        const total = job.total_items ?? "unknown";
        return `${completed} / ${total}`;
    }

    function table(captionText, headers, rows) {
        const tableNode = document.createElement("table");
        const caption = document.createElement("caption");
        const thead = document.createElement("thead");
        const tbody = document.createElement("tbody");
        const headerRow = document.createElement("tr");
        tableNode.className = "report-table";
        caption.textContent = captionText;
        headerRow.replaceChildren(...headers.map((header) => {
            const th = document.createElement("th");
            th.scope = "col";
            th.textContent = header;
            return th;
        }));
        thead.append(headerRow);
        tbody.replaceChildren(...rows.map((row) => {
            const tr = document.createElement("tr");
            tr.replaceChildren(...row.map((value) => {
                const td = document.createElement("td");
                td.textContent = String(valueOrNone(value));
                return td;
            }));
            return tr;
        }));
        tableNode.append(caption, thead, tbody);
        return tableNode;
    }

    function warningList(report) {
        const warnings = report?.warnings || [];
        const skipped = report?.skipped_items || [];
        const list = document.createElement("ul");
        list.className = "report-list compact";
        if (!warnings.length && !skipped.length) {
            list.replaceChildren(textItem("No ingestion warnings or skipped records"));
            return list;
        }
        list.replaceChildren(
            ...warnings.map((item) => textItem(`warning ${item.code}: ${item.message} (${valueOrNone(item.chunk_id || item.cell_id)})`)),
            ...skipped.map((item) => textItem(`skipped ${item.reason}: ${valueOrNone(item.input_ref)}`)),
        );
        return list;
    }

    function renderSourceRefs(report) {
        const rows = (report?.source_refs || []).map((item) => [
            item.cell_id,
            item.chunk_id,
            item.has_source_ref ? "yes" : "no",
            item.source_id,
            item.document_id,
            q16Percent(item.confidence_q16),
        ]);
        if (!rows.length) {
            return table("Ingestion chunks and SourceRefs", ["Cell", "Chunk", "SourceRef", "Source", "Document", "Confidence"], [
                ["none", "none", "none", "none", "none", "n/a"],
            ]);
        }
        return table("Ingestion chunks and SourceRefs", ["Cell", "Chunk", "SourceRef", "Source", "Document", "Confidence"], rows);
    }

    function renderJobRows(jobs) {
        const rows = jobs.map((job) => [
            job.job_id,
            job.label,
            job.status,
            progressLabel(job),
            job.failed_items ?? 0,
            job.last_cell_id,
            job.message,
        ]);
        return table(
            "Ingestion job records",
            ["Job", "Label", "Status", "Progress", "Failed", "Last cell", "Message"],
            rows.length ? rows : [["none", "none", "none", "none", "none", "none", "none"]],
        );
    }

    function renderJobList(container, jobs) {
        const completed = jobs.filter((job) => job.status === "completed").length;
        const running = jobs.filter((job) => job.status === "running" || job.status === "queued").length;
        const failed = jobs.filter((job) => job.status === "failed").length;
        const cancelled = jobs.filter((job) => job.status === "cancelled").length;
        const summary = document.createElement("div");
        summary.className = "report-grid";
        summary.replaceChildren(
            card("Records", jobs.length, jobs.length ? "good" : "warn"),
            card("Completed", completed, completed ? "good" : ""),
            card("Queued/running", running, running ? "warn" : "good"),
            card("Failed", failed, failed ? "bad" : "good"),
            card("Cancelled", cancelled, cancelled ? "warn" : "good"),
        );
        container.replaceChildren(
            card("Ingestion jobs", "records / progress / failures", jobs.length ? "good" : "warn"),
            summary,
            renderJobRows(jobs),
        );
    }

    function renderJobDetail(container, job) {
        const summary = document.createElement("div");
        const failures = document.createElement("ul");
        summary.className = "report-grid";
        failures.className = "report-list compact";
        summary.replaceChildren(
            card("Job", job.job_id),
            card("Label", job.label),
            card("Status", job.status, toneForStatus(job.status)),
            card("Progress", progressLabel(job)),
            card("Failed", job.failed_items ?? 0, job.failed_items ? "bad" : "good"),
            card("Last cell", valueOrNone(job.last_cell_id)),
            card("Retry", `${job.retry_count ?? 0} / ${job.max_retries ?? 0}`),
        );
        if (job.message) failures.replaceChildren(textItem(`failure reason: ${job.message}`));
        else failures.replaceChildren(textItem("No failure reason recorded"));
        container.replaceChildren(
            card("Ingestion job", "progress / failure detail / record", toneForStatus(job.status)),
            summary,
            renderJobRows([job]),
            failures,
        );
    }

    function renderIngestSummary(container, body) {
        const report = body.validation_report || {};
        const warnings = report.warnings || [];
        const skipped = report.skipped_items || [];
        const sourceRefs = report.source_refs || [];
        const summary = document.createElement("div");
        summary.className = "report-grid";
        summary.replaceChildren(
            card("Rows", body.rows_ingested ?? 0),
            card("Chunks", body.chunks_ingested ?? 0),
            card("Facts", body.facts_ingested ?? 0),
            card("First cell", valueOrNone(body.first_cell_id), body.first_cell_id ? "good" : "warn"),
            card("Job", valueOrNone(body.job_id), body.job_id ? "good" : "warn"),
            card("Cells seen", report.cells_seen ?? 0),
            card("Warnings", warnings.length, warnings.length ? "bad" : "good"),
            card("Skipped", skipped.length, skipped.length ? "warn" : "good"),
            card("Source refs", sourceRefs.length, sourceRefs.length ? "good" : "warn"),
        );
        container.replaceChildren(
            card("Ingestion result", "progress / warnings / chunks / SourceRefs", "good"),
            summary,
            warningList(report),
            renderSourceRefs(report),
        );
    }

    function renderIngestReport(body) {
        const container = document.querySelector("#ingest-report");
        if (!container || body === null || body === undefined) return;
        if (Array.isArray(body)) {
            renderJobList(container, body);
            return;
        }
        if (typeof body !== "object") return;

        const isIngestSummary = (
            body.rows_ingested !== undefined ||
            body.chunks_ingested !== undefined ||
            body.facts_ingested !== undefined
        );
        if (isIngestSummary) {
            renderIngestSummary(container, body);
            return;
        }
        if (body.job_id !== undefined && body.label !== undefined && body.status !== undefined) {
            renderJobDetail(container, body);
        }
    }

    reports.renderIngestReport = renderIngestReport;
    reports.ingestionJobDashboard = "progress failures warnings records chunks source refs";
    window.CortexDashboardReports = reports;
})();
