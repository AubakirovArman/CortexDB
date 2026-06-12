(() => {
    const reports = window.CortexDashboardReports || {};
    const { card, textItem, yesNo } = reports.helpers || {};
    if (!card || !textItem || !yesNo) return;

    function renderPermissionsView(body) {
        const container = document.querySelector("#permissions-report");
        if (!container || body?.schema_version !== "dashboard_permissions.v1") return;

        const capabilities = body.capabilities || {};
        const agentView = body.agent_view || {};
        const roleUi = body.role_ui || {};
        const selectedScopes = body.selected_scopes || [];
        const cards = document.createElement("div");
        const rules = document.createElement("ul");
        const roleUiList = document.createElement("ul");
        const scopeList = document.createElement("ul");
        const agentRules = document.createElement("ul");
        const denialList = document.createElement("ul");
        const denials = body.denials || [];
        cards.className = "report-grid";
        rules.className = "report-list compact";
        roleUiList.className = "report-list compact";
        scopeList.className = "report-list compact";
        agentRules.className = "report-list compact";
        denialList.className = "report-list compact";
        cards.replaceChildren(
            card("Tenant", body.tenant || "default"),
            card("Role", body.role || body.access_level || "limited"),
            card("Access", body.access_level || "limited"),
            card("Token active", yesNo(body.token_active)),
            card("Token storage", body.token_storage || "none"),
            card("Token visible", yesNo(body.token_visible), body.token_visible ? "bad" : "good"),
            card("Read-only", yesNo(body.read_only), body.read_only ? "warn" : "good"),
            card("Role UI mode", roleUi.mode || "unknown", roleUi.mode === "admin" ? "good" : (roleUi.mode === "read_only" ? "warn" : "good")),
            card("Data read", yesNo(capabilities.data_read), capabilities.data_read ? "good" : "warn"),
            card("Admin maintenance", yesNo(capabilities.admin_maintenance), capabilities.admin_maintenance ? "good" : "warn"),
            card("Local writes", yesNo(capabilities.local_writes), capabilities.local_writes ? "good" : "warn"),
            card("Dangerous visible", roleUi.dangerous_operations?.visible?.length ?? 0, roleUi.dangerous_operations?.visible?.length ? "warn" : "good"),
            card("Dangerous hidden", roleUi.dangerous_operations?.hidden_or_disabled?.length ?? 0, "good"),
            card("AgentView source", agentView.source || "unknown"),
            card("Server enforced", yesNo(agentView.server_enforced), agentView.server_enforced ? "good" : "bad"),
        );
        rules.replaceChildren(
            textItem("Public mode can load health and permissions only."),
            textItem("Data mode can run cell reads, retrieval, AQL, context, verify, ingest jobs, and cluster status."),
            textItem("Admin mode can run storage maintenance, validation, metrics, and ANN metrics."),
            textItem("Read-only mode is a local dashboard guard that blocks mutating actions before they reach the API."),
        );
        roleUiList.replaceChildren(...roleUiRows(roleUi).map(textItem));
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
        if (denials.length) {
            denialList.replaceChildren(...denials.map((denial) => textItem(`denied: ${denial}`)));
        } else {
            denialList.replaceChildren(textItem("No dashboard denials for the current session posture"));
        }
        container.replaceChildren(
            card("Permissions explorer", "Token / role / scope / AgentView", "good"),
            cards,
            card("Role-based UI", "admin / data user / read-only visibility", roleUi.mode === "limited" ? "warn" : "good"),
            roleUiList,
            card("Scope probes", `${selectedScopes.length} scopes`),
            scopeList,
            card("AgentView policy", agentView.source || "unknown"),
            agentRules,
            card("Denials", `${denials.length} active`, denials.length ? "warn" : "good"),
            denialList,
            rules,
        );
    }

    function roleUiRows(roleUi) {
        if (!roleUi || roleUi.schema_version !== "dashboard_role_ui.v1") {
            return ["Role-based UI state is not available"];
        }
        const admin = roleUi.admin_ui || {};
        const data = roleUi.data_user_ui || {};
        const readOnly = roleUi.read_only_ui || {};
        const dangerous = roleUi.dangerous_operations || {};
        return [
            `admin UI: ${admin.available ? "available" : "hidden"}; maintenance visible ${yesNo(admin.maintenance_visible)}`,
            `data user UI: ${data.available ? "available" : "hidden"}; retrieval visible ${yesNo(data.retrieval_visible)}`,
            `read-only UI: ${readOnly.active ? "active" : "inactive"}; guard ${readOnly.guard || "not checked"}`,
            `visible routes: ${(roleUi.visible_routes || []).join(", ") || "none"}`,
            `hidden routes: ${(roleUi.hidden_routes || []).join(", ") || "none"}`,
            `dangerous visible: ${(dangerous.visible || []).join(", ") || "none"}`,
            `dangerous hidden/disabled: ${(dangerous.hidden_or_disabled || []).join(", ") || "none"}`,
            `dangerous policy: ${dangerous.hide_policy || "hidden by role and read-only guard"}`,
        ];
    }

    Object.assign(reports, {
        renderPermissionsView,
    });
    window.CortexDashboardReports = reports;
})();
