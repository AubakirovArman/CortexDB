const output = document.querySelector("#output");
const metrics = document.querySelector("#metrics");
const history = document.querySelector("#history");
const sessionStatus = document.querySelector("#session-status");
const roleStatus = document.querySelector("#session-role");
const permissionReport = document.querySelector("#permission-report");
const roleUiReport = document.querySelector("#role-ui-report");
const requestStatus = document.querySelector("#request-status");
const content = document.querySelector("#content");
const routeLinks = Array.from(document.querySelectorAll("[data-route]"));
const panels = Array.from(document.querySelectorAll(".panel"));
const routes = new Map(panels.map((panel) => [panel.id, panel]));

const tenantInput = document.querySelector("#tenant");
const tokenInput = document.querySelector("#token");
const readOnlyToggle = document.querySelector("#read-only-mode");

const ACCESS_PUBLIC = "public";
const ACCESS_DATA = "data";
const ACCESS_ADMIN = "admin";

let token = "";
let tenant = sessionStorage.getItem("cortexdb-dashboard-tenant") || "default";
let accessLevel = ACCESS_PUBLIC;
let accessHint = "no token";
let readOnlyMode = sessionStorage.getItem("cortexdb-dashboard-read-only") === "true";
let lastRequestIssue = null;

tenantInput.value = tenant;
readOnlyToggle.checked = readOnlyMode;

function accessRank(level) {
    switch (level) {
        case ACCESS_ADMIN:
            return 2;
        case ACCESS_DATA:
            return 1;
        default:
            return 0;
    }
}

function accessLabel(level) {
    switch (level) {
        case ACCESS_ADMIN:
            return "admin";
        case ACCESS_DATA:
            return "data";
        default:
            return "limited";
    }
}

function canUse(minAccess) {
    return accessRank(accessLevel) >= accessRank(minAccess);
}

function renderSessionStatus() {
    const tokenState = token ? "bearer active for tab" : "bearer inactive";
    const tenantState = `Tenant: ${tenant}`;
    sessionStatus.textContent = `${tenantState} · ${tokenState}`;

    const tokenHint = token ? " - token in-memory" : " - no token";
    const readOnlyHint = readOnlyMode ? " - read-only guard active" : "";
    const hint = accessHint ? ` (${accessHint})` : "";
    roleStatus.textContent = `Access level: ${accessLabel(accessLevel)}${tokenHint}${readOnlyHint}${hint}`;
    renderPermissionReport();
    renderRoleUiReport();
    window.CortexDashboardReports?.renderPermissionsView?.(permissionState());
}

function renderPermissionReport() {
    if (!permissionReport) return;

    let text = "Limited access: public health only. Apply a bearer token to enable data/admin actions.";
    let tone = "limited";
    if (accessLevel === ACCESS_ADMIN) {
        text = "Admin access: data and storage maintenance actions are available.";
        tone = "admin";
    } else if (accessLevel === ACCESS_DATA) {
        text = "Data access: cell, search, AQL, context, verify, ingest, and cluster actions are available. Storage maintenance is hidden.";
        tone = "data";
    } else if (accessHint) {
        text = `Limited access: ${accessHint}. Apply a bearer token to enable data/admin actions.`;
    }
    if (readOnlyMode) text += " Read-only mode blocks local write actions.";

    permissionReport.dataset.access = tone;
    permissionReport.textContent = text;
}

function roleUiState() {
    const visibleRoutes = routeLinks.filter((link) => !link.hidden).map((link) => link.dataset.route).filter(Boolean);
    const hiddenRoutes = routeLinks.filter((link) => link.hidden).map((link) => link.dataset.route).filter(Boolean);
    const dangerousControls = Array.from(document.querySelectorAll("[data-dangerous='true']"));
    const hiddenDangerous = dangerousControls
        .filter((node) => node.hidden || node.disabled)
        .map((node) => node.dataset.action || node.value || node.textContent.trim())
        .filter(Boolean);
    const visibleDangerous = dangerousControls
        .filter((node) => !node.hidden && !node.disabled)
        .map((node) => node.dataset.action || node.value || node.textContent.trim())
        .filter(Boolean);
    const mode = readOnlyMode
        ? "read_only"
        : (canUse(ACCESS_ADMIN) ? "admin" : (canUse(ACCESS_DATA) ? "data_user" : "limited"));
    return {
        schema_version: "dashboard_role_ui.v1",
        mode,
        admin_ui: {
            available: canUse(ACCESS_ADMIN) && !readOnlyMode,
            visible_routes: visibleRoutes.filter((route) => route === "storage" || route === "ann-eval"),
            maintenance_visible: canUse(ACCESS_ADMIN),
        },
        data_user_ui: {
            available: canUse(ACCESS_DATA) && !readOnlyMode,
            visible_routes: visibleRoutes.filter((route) => route !== "storage"),
            retrieval_visible: canUse(ACCESS_DATA),
        },
        read_only_ui: {
            active: readOnlyMode,
            blocked_actions: readOnlyMode ? ["put", "tombstone", "ingest", "flush", "compact"] : [],
            guard: "client_side_before_request",
        },
        visible_routes: visibleRoutes,
        hidden_routes: hiddenRoutes,
        dangerous_operations: {
            visible: visibleDangerous,
            hidden_or_disabled: hiddenDangerous,
            hide_policy: "hidden unless role allows it; disabled when read-only guard is active",
        },
    };
}

function renderRoleUiReport() {
    if (!roleUiReport) return;
    const state = roleUiState();
    const hiddenDangerous = state.dangerous_operations.hidden_or_disabled.length;
    const visibleDangerous = state.dangerous_operations.visible.length;
    const text = `Role UI: ${state.mode}; visible routes ${state.visible_routes.length}; hidden routes ${state.hidden_routes.length}; dangerous visible ${visibleDangerous}; dangerous hidden/disabled ${hiddenDangerous}`;
    roleUiReport.dataset.mode = state.mode;
    roleUiReport.textContent = text;
}

function permissionState() {
    const selectedScopes = collectScopeInputs();
    const denials = [];
    if (!canUse(ACCESS_DATA)) {
        denials.push("Data routes denied until a data or admin bearer token is applied.");
    }
    if (!canUse(ACCESS_ADMIN)) {
        denials.push("Admin routes denied until an admin bearer token is applied.");
    }
    if (readOnlyMode) {
        denials.push("Mutating dashboard actions denied by local read-only mode.");
    }
    return {
        schema_version: "dashboard_permissions.v1",
        tenant,
        role: accessLabel(accessLevel),
        access_level: accessLevel,
        access_hint: accessHint,
        token_active: token.trim().length > 0,
        token_storage: token.trim().length > 0 ? "memory_only" : "none",
        token_visible: false,
        read_only: readOnlyMode,
        selected_scopes: selectedScopes,
        denials,
        role_ui: roleUiState(),
        agent_view: {
            source: token.trim().length > 0 ? "server_token_policy" : "anonymous_synthetic_view",
            server_enforced: true,
            readable_scope_probe: canUse(ACCESS_DATA) ? "allowed_by_role_then_checked_by_agent_view" : "public_only",
            writable_scope_probe: canUse(ACCESS_DATA) && !readOnlyMode ? "write_requests_checked_by_agent_view" : "not_available",
            note: "Dashboard shows local permission posture; the server remains the source of truth for AgentView scopes.",
        },
        capabilities: {
            public_health: canUse(ACCESS_PUBLIC),
            data_read: canUse(ACCESS_DATA),
            admin_maintenance: canUse(ACCESS_ADMIN),
            local_writes: canUse(ACCESS_DATA) && !readOnlyMode,
        },
    };
}

function collectScopeInputs() {
    const scopes = Array.from(document.querySelectorAll("input[name='scope']"))
        .map((input) => input.value.trim())
        .filter(Boolean);
    return Array.from(new Set(scopes)).sort();
}

function headers() {
    return token ? { Authorization: `Bearer ${token}` } : {};
}

function scoped(path) {
    if (!tenant || tenant === "default") return path;
    return `${path}${path.includes("?") ? "&" : "?"}tenant=${encodeURIComponent(tenant)}`;
}

function on(selector, event, handler) {
    const node = document.querySelector(selector);
    if (node) node.addEventListener(event, handler);
}

function onAll(selector, event, handler) {
    document.querySelectorAll(selector).forEach((node) => node.addEventListener(event, handler));
}

function addHistory(label, ok) {
    const item = document.createElement("li");
    item.textContent = `${ok ? "OK" : "ERR"} ${label}`;
    history.prepend(item);
    while (history.children.length > 8) history.lastElementChild.remove();
}

function show(value, ok = true) {
    output.className = ok ? "ok" : "error";
    output.textContent = typeof value === "string" ? value : JSON.stringify(value, null, 2);
}

function errorMessage(error) {
    if (typeof error === "string") return error;
    if (error?.http_status) return `HTTP ${error.http_status}`;
    if (error?.message) return error.message;
    if (error?.error) return error.error;
    if (error?.code) return error.code;
    if (error?.status) return `HTTP ${error.status}`;
    return "request failed";
}

function setRequestStatus(kind, text) {
    requestStatus.className = `request-status ${kind}`;
    requestStatus.textContent = text;
}

function fieldLabel(input) {
    const id = input.getAttribute("id");
    if (!id) return input.getAttribute("name") || "Field";
    return document.querySelector(`label[for="${id}"]`)?.textContent || input.getAttribute("name") || "Field";
}

function validityMessage(input) {
    if (input.validity?.valueMissing) return "is required";
    if (input.validity?.patternMismatch) return "has an invalid format";
    return input.validationMessage || "is invalid";
}

function markInvalid(input) {
    input.setAttribute("aria-invalid", "true");
    setRequestStatus("error", `ERR form validation: ${fieldLabel(input)} ${validityMessage(input)}`);
}

function clearInvalid(input) {
    input.removeAttribute("aria-invalid");
}

function guardForm(form) {
    if (form.checkValidity()) return true;
    const firstInvalid = form.querySelector("input:invalid, textarea:invalid, select:invalid");
    if (firstInvalid) markInvalid(firstInvalid);
    form.reportValidity();
    return false;
}
