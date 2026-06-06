(() => {
    const reports = window.CortexDashboardReports || {};
    const { card, textItem, yesNo } = reports.helpers || {};
    if (!card || !textItem || !yesNo) return;

    const HASH_CHAIN_COMMAND = "cortexdb audit verify --file $CORTEXDB_AUDIT_LOG_FILE";
    const REDACTION_COMMAND = "cortexdb audit --summary --redaction-check";

    function filterValue(selector) {
        return document.querySelector(selector)?.value || "all";
    }

    function auditReadinessState() {
        const tenant = document.querySelector("#tenant")?.value || "default";
        const accessText = document.querySelector("#session-role")?.textContent || "";
        const readOnly = document.querySelector("#read-only-mode")?.checked === true;
        const tokenActive = accessText.includes("token in-memory");
        const accessLevel = accessText.includes("Access level: admin")
            ? "admin"
            : accessText.includes("Access level: data")
                ? "data"
                : "limited";
        const filters = {
            category: filterValue("#audit-filter-category"),
            severity: filterValue("#audit-filter-severity"),
        };
        const events = [
            {
                category: "readiness",
                severity: "info",
                label: "Audit source",
                message: "Audit logs are file-backed and reviewed through CLI evidence gates.",
                action: "Set CORTEXDB_AUDIT_LOG_FILE before release or incident drills.",
            },
            {
                category: "hash_chain",
                severity: "warn",
                label: "Hash-chain verification",
                message: "Browser view does not read raw JSONL events; chain verification is an operator CLI step.",
                action: HASH_CHAIN_COMMAND,
            },
            {
                category: "redaction",
                severity: "info",
                label: "Redaction status",
                message: "Raw query, body, and bearer token values are intentionally not rendered in the dashboard.",
                action: REDACTION_COMMAND,
            },
            {
                category: "access",
                severity: accessLevel === "admin" ? "info" : "warn",
                label: "Access posture",
                message: accessLevel === "admin"
                    ? "Admin token posture can run audit evidence commands outside the browser."
                    : "Limited or data-only access cannot complete admin audit review from the dashboard.",
                action: "Apply an admin token only when operator review requires admin endpoints.",
            },
        ];
        const filtered_events = events.filter((event) => {
            const categoryMatch = filters.category === "all" || event.category === filters.category;
            const severityMatch = filters.severity === "all" || event.severity === filters.severity;
            return categoryMatch && severityMatch;
        });
        const summary = {
            total_events: events.length,
            visible_events: filtered_events.length,
            warnings: events.filter((event) => event.severity === "warn").length,
            critical: events.filter((event) => event.severity === "critical").length,
        };
        return {
            schema_version: "dashboard_audit_viewer.v2",
            tenant,
            read_only: readOnly,
            operator_access: accessText,
            token_active: tokenActive,
            filters,
            summary,
            hash_chain_verification: {
                status: "cli_required",
                command: HASH_CHAIN_COMMAND,
                raw_events_visible: false,
            },
            redaction_status: {
                status: "browser_redacted",
                query_visible: false,
                body_visible: false,
                token_visible: false,
                command: REDACTION_COMMAND,
            },
            events,
            filtered_events,
            checks: [
                "Enable CORTEXDB_AUDIT_LOG_FILE before release or incident drills.",
                "Review audit logs with cortexdb audit --summary --redaction-check.",
                "Use dashboard read-only mode during evidence review.",
                "Keep raw audit events out of the browser until server-side redaction is configured.",
            ],
        };
    }

    function renderAuditReadiness(body = auditReadinessState()) {
        const container = document.querySelector("#audit-report");
        if (!container || body?.schema_version !== "dashboard_audit_viewer.v2") return;

        const summary = document.createElement("div");
        const checks = document.createElement("ul");
        const eventList = document.createElement("ul");
        const verificationList = document.createElement("ul");
        summary.className = "report-grid";
        checks.className = "report-list compact";
        eventList.className = "report-list compact";
        verificationList.className = "report-list compact";
        summary.replaceChildren(
            card("Tenant", body.tenant || "default"),
            card("Read-only", yesNo(body.read_only), body.read_only ? "good" : "warn"),
            card("Visible events", body.summary?.visible_events ?? 0),
            card("Warnings", body.summary?.warnings ?? 0, body.summary?.warnings ? "warn" : "good"),
            card("Hash chain", body.hash_chain_verification?.status || "unknown", "warn"),
            card("Redaction", body.redaction_status?.status || "unknown", "good"),
            card("Raw logs", "not rendered in browser", "good"),
        );
        checks.replaceChildren(...(body.checks || []).map(textItem));
        verificationList.replaceChildren(
            textItem(`filters: category=${body.filters?.category || "all"}, severity=${body.filters?.severity || "all"}`),
            textItem(`hash-chain verification: ${body.hash_chain_verification?.command || HASH_CHAIN_COMMAND}`),
            textItem(`redaction check: ${body.redaction_status?.command || REDACTION_COMMAND}`),
            textItem("redaction status: query/body/token hidden in browser"),
        );
        const events = body.filtered_events || [];
        if (events.length === 0) {
            eventList.replaceChildren(textItem("No safe audit events match the active filters."));
        } else {
            eventList.replaceChildren(...events.map((event) => textItem(
                `${event.severity} / ${event.category}: ${event.label} - ${event.message} Action: ${event.action}`,
            )));
        }
        container.replaceChildren(summary, verificationList, eventList, checks);
    }

    function showAuditReadiness() {
        renderAuditReadiness(auditReadinessState());
    }

    Object.assign(reports, {
        renderAuditReadiness,
    });
    window.CortexDashboardReports = reports;
    window.addEventListener("DOMContentLoaded", () => {
        document.querySelectorAll("[data-action='audit-readiness']")
            .forEach((node) => node.addEventListener("click", showAuditReadiness));
        document.querySelector("#tenant")?.addEventListener("input", showAuditReadiness);
        document.querySelector("#read-only-mode")?.addEventListener("change", showAuditReadiness);
        document.querySelector("#audit-filter-category")?.addEventListener("change", showAuditReadiness);
        document.querySelector("#audit-filter-severity")?.addEventListener("change", showAuditReadiness);
        const roleNode = document.querySelector("#session-role");
        if (roleNode) new MutationObserver(showAuditReadiness).observe(roleNode, { childList: true });
        showAuditReadiness();
    });
})();
