(() => {
    const reports = window.CortexDashboardReports || {};
    const { card, textItem, yesNo } = reports.helpers || {};
    if (!card || !textItem || !yesNo) return;

    function auditReadinessState() {
        const tenant = document.querySelector("#tenant")?.value || "default";
        const accessText = document.querySelector("#session-role")?.textContent || "";
        const readOnly = document.querySelector("#read-only-mode")?.checked === true;
        return {
            schema_version: "dashboard_audit_readiness.v1",
            tenant,
            read_only: readOnly,
            operator_access: accessText,
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
        if (!container || body?.schema_version !== "dashboard_audit_readiness.v1") return;

        const summary = document.createElement("div");
        const checks = document.createElement("ul");
        summary.className = "report-grid";
        checks.className = "report-list compact";
        summary.replaceChildren(
            card("Tenant", body.tenant || "default"),
            card("Read-only", yesNo(body.read_only), body.read_only ? "good" : "warn"),
            card("Audit source", "file-based CLI review", "warn"),
            card("Raw logs", "not rendered in browser", "good"),
        );
        checks.replaceChildren(...(body.checks || []).map(textItem));
        container.replaceChildren(summary, checks);
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
        const roleNode = document.querySelector("#session-role");
        if (roleNode) new MutationObserver(showAuditReadiness).observe(roleNode, { childList: true });
        showAuditReadiness();
    });
})();
