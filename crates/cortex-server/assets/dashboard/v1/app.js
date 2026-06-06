const output = document.querySelector("#output");
const metrics = document.querySelector("#metrics");
const history = document.querySelector("#history");
const sessionStatus = document.querySelector("#session-status");
const roleStatus = document.querySelector("#session-role");
const permissionReport = document.querySelector("#permission-report");
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

function permissionState() {
    const selectedScopes = collectScopeInputs();
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

async function fetchJsonLike(url, init = {}) {
    const response = await fetch(url, { ...init, headers: { ...headers(), ...(init.headers || {}) } });
    const text = await response.text();
    let body;
    try {
        body = JSON.parse(text);
    } catch {
        body = text;
    }
    return { response, body, text };
}

async function api(path, init = {}) {
    const { response, body } = await fetchJsonLike(scoped(path), {
        ...init,
        headers: { ...headers(), ...(init.headers || {}) },
    });
    if (!response.ok) throw requestError(response, body);
    return body;
}

function requestError(response, body) {
    if (body && typeof body === "object" && !Array.isArray(body)) {
        return {
            ...body,
            http_status: response.status,
            http_status_text: response.statusText || "",
        };
    }
    return {
        http_status: response.status,
        http_status_text: response.statusText || "",
        message: String(body || response.statusText || "request failed"),
    };
}

async function detectAccessLevel() {
    const isNoToken = token.trim().length === 0;

    try {
        const { response: statsResponse } = await fetchJsonLike(scoped("/v1/stats"));
        if (statsResponse.ok) {
            return {
                level: ACCESS_ADMIN,
                hint: isNoToken ? "stats endpoint accessible" : "admin token accepted",
            };
        }
        const { response: cellResponse, body } = await fetchJsonLike(scoped("/v1/cell?cell_id=0"));
        if (cellResponse.ok) {
            return {
                level: ACCESS_DATA,
                hint: isNoToken ? "cell endpoint accessible" : "data token accepted",
            };
        }
        if (!isNoToken && (cellResponse.status === 401 || cellResponse.status === 403)) {
            const reason = body?.message || body?.error || `HTTP ${cellResponse.status}`;
            return { level: ACCESS_PUBLIC, hint: `token rejected: ${reason}` };
        }
        return {
            level: ACCESS_PUBLIC,
            hint: isNoToken ? "public read-only mode" : "token has insufficient scope",
        };
    } catch {
        return { level: ACCESS_PUBLIC, hint: isNoToken ? "public" : "token check failed" };
    }
}

function refreshAccessVisibility() {
    for (const node of document.querySelectorAll("[data-access]")) {
        const minAccess = node.getAttribute("data-access") || ACCESS_PUBLIC;
        if (canUse(minAccess)) {
            node.hidden = false;
            node.removeAttribute("aria-hidden");
            if (node instanceof HTMLButtonElement || node instanceof HTMLInputElement) {
                const writeDisabled = readOnlyMode && node.dataset.write === "true";
                node.disabled = writeDisabled;
                if (writeDisabled) node.setAttribute("aria-disabled", "true");
                else node.removeAttribute("aria-disabled");
            }
        } else {
            node.hidden = true;
            node.setAttribute("aria-hidden", "true");
            if (node instanceof HTMLButtonElement || node instanceof HTMLInputElement) {
                node.disabled = true;
            }
        }
    }

    const requested = routeFromLocation();
    const panel = routes.get(requested);
    if (!panel || panel.hidden) {
        const fallback = firstAllowedRoute();
        if (fallback !== requested) {
            setRoute(fallback, false);
        }
    }
}

function firstAllowedRoute() {
    const order = [
        "overview",
        "permissions",
        "cells",
        "search",
        "ann-eval",
        "aql",
        "context",
        "verify",
        "ingest",
        "storage",
        "cluster",
    ];
    for (const route of order) {
        const routeLink = document.querySelector(`a[data-route="${route}"]`);
        const panel = routes.get(route);
        if (routeLink && !routeLink.hidden) return route;
        if (routeLink && routeLink.hidden) continue;
        if (panel && !panel.hidden) return route;
    }
    return "overview";
}

function renderMetrics(body) {
    metrics.replaceChildren();
    for (const [key, item] of Object.entries(body)
        .filter(([, value]) => typeof value !== "object")
        .slice(0, 8)) {
        const card = document.createElement("div");
        const label = document.createElement("span");
        const strong = document.createElement("strong");
        card.className = "metric";
        label.textContent = key;
        strong.textContent = String(item);
        card.append(label, strong);
        metrics.append(card);
    }
}

function run(label, task) {
    setRequestStatus("running", `Running ${label}`);
    show(`${label}...`);
    task()
        .then((body) => {
            setRequestStatus("ok", `OK ${label}`);
            show(body);
            lastRequestIssue = null;
            window.CortexDashboardReports?.clearRequestIssue?.();
            window.CortexDashboardReports?.renderAnnEvaluation?.(body);
            window.CortexDashboardReports?.renderAqlReport?.(body);
            window.CortexDashboardReports?.renderCellReport?.(body, label);
            window.CortexDashboardReports?.renderClusterReport?.(body);
            window.CortexDashboardReports?.renderContextPack?.(body);
            window.CortexDashboardReports?.renderIngestReport?.(body, label);
            window.CortexDashboardReports?.renderOperationalStatus?.(body);
            window.CortexDashboardReports?.renderPermissionsView?.(body);
            window.CortexDashboardReports?.renderSearchReport?.(body);
            window.CortexDashboardReports?.renderStorageValidation?.(body);
            window.CortexDashboardReports?.renderVerificationReport?.(body);
            addHistory(label, true);
            if (body?.current_seq !== undefined || body?.ok !== undefined) renderMetrics(body);
        })
        .catch((error) => {
            setRequestStatus("error", `ERR ${label}: ${errorMessage(error)}`);
            show(error, false);
            lastRequestIssue = {
                label,
                status: Number(error?.http_status || 0),
                code: error?.code || error?.status || "request_error",
                message: errorMessage(error),
            };
            window.CortexDashboardReports?.renderRequestIssue?.(error, label);
            addHistory(label, false);
        });
}

function guardWriteAllowed(label) {
    if (!readOnlyMode) return true;
    const error = {
        http_status: 0,
        code: "dashboard_read_only",
        message: "Read-only mode blocks this local write action.",
    };
    setRequestStatus("error", `ERR read-only: ${label}`);
    show(error, false);
    lastRequestIssue = {
        label,
        status: 0,
        code: error.code,
        message: error.message,
    };
    window.CortexDashboardReports?.renderRequestIssue?.(error, label);
    addHistory(label, false);
    return false;
}

async function safeStatusCheck(label, path) {
    try {
        const body = await api(path);
        return { label, ok: true, body };
    } catch (error) {
        return { label, ok: false, error };
    }
}

async function loadOperationalStatus() {
    const checks = [["health", "/v1/health"], ["compatibility", "/v1/compatibility"]];
    if (canUse(ACCESS_ADMIN)) {
        checks.push(["stats", "/v1/stats"], ["validate", "/v1/validate"], ["metrics", "/v1/metrics"]);
    } else if (canUse(ACCESS_DATA)) {
        checks.push(["cell read", "/v1/cell?cell_id=0"]);
    }
    const results = await Promise.all(checks.map(([label, path]) => safeStatusCheck(label, path)));
    const health = summarizeStatusResult(results, "health");
    const compatibility = summarizeCompatibilityResult(results);
    const stats = summarizeStatsResult(results);
    const validation = summarizeValidationResult(results);
    const metrics = summarizeStatusResult(results, "metrics");
    const backup = backupPosture(results);
    const incidents = results
        .filter((result) => !result.ok)
        .map((result) => ({
            label: result.label,
            message: errorMessage(result.error),
            code: result.error?.code || result.error?.status || "request_error",
            status: Number(result.error?.http_status || 0),
        }));
    return {
        schema_version: "dashboard_status.v1",
        tenant,
        access_level: accessLevel,
        read_only: readOnlyMode,
        results,
        incidents,
        incident_timeline: buildIncidentTimeline({ results, incidents, validation, metrics, backup }),
        health,
        compatibility,
        stats,
        validation,
        metrics,
        backup_posture: backup,
        last_request_error: lastRequestIssue,
    };
}

function summarizeCompatibilityResult(results) {
    const result = resultByLabel(results, "compatibility");
    const body = result?.body || {};
    const api = body.api || {};
    const sdk = body.sdk || {};
    const migration = body.migration || {};
    const storageFormats = body.storage_formats || [];
    return {
        available: !!result,
        ok: !!result?.ok && body.schema_version === "cortexdb.compatibility.v1",
        schema_version: body.schema_version || null,
        api_version: api.version || null,
        api_contract: api.contract || null,
        sdk_contract: sdk.contract || null,
        sdk_workspace_version: sdk.workspace_version || null,
        storage_formats: storageFormats,
        migration_current_release: migration.current_release || null,
        migration_release: migration.release || null,
        migration_gate: migration.gate || null,
        message: result?.ok ? "ok" : (result ? errorMessage(result.error) : "not checked"),
    };
}

function buildIncidentTimeline({ results, incidents, validation, metrics, backup }) {
    const timeline = [];
    const add = (category, severity, label, message, action, source = "dashboard_status") => {
        timeline.push({ category, severity, label, message, action, source });
    };

    add(
        "audit_event",
        "info",
        "Audit readiness",
        "Audit readiness is checked from the Overview audit panel.",
        "Run the audit readiness action before incident review.",
        "dashboard_audit_readiness",
    );

    for (const incident of incidents) {
        const status = Number(incident.status || 0);
        if (status === 429 || incident.code === "rate_limited") {
            add(
                "rate_limit_event",
                "warn",
                incident.label,
                incident.message,
                "Wait for the rate window to reset or review per-token quotas.",
                "request_error",
            );
        }
    }

    if (validation.available && !validation.ok) {
        add(
            "storage_event",
            "critical",
            "Storage validation",
            validation.message || "Storage validation reported errors.",
            "Run cortexdb validate, then repair or restore before trusting writes.",
            "validation",
        );
    } else if (!validation.available && canUse(ACCESS_ADMIN)) {
        add(
            "storage_event",
            "warn",
            "Storage validation",
            "Storage validation did not run.",
            "Retry validation before backup, restore, or release evidence.",
            "validation",
        );
    }

    if (backup.validation_ok === false) {
        add(
            "backup_event",
            "critical",
            "Backup posture",
            "Backup posture is blocked by failed storage validation.",
            "Fix validation failures before backup or restore drills.",
            "backup_posture",
        );
    } else if (!backup.available) {
        add(
            "backup_event",
            "warn",
            "Backup posture",
            "Backup posture requires an admin token and operator CLI evidence.",
            "Use an admin token and run make backup-restore-production-pack-check.",
            "backup_posture",
        );
    } else {
        add(
            "backup_event",
            "info",
            "Backup posture",
            "Browser view is read-only; backup evidence is produced by operator CLI gates.",
            "Run make backup-restore-production-pack-check for release evidence.",
            "backup_posture",
        );
    }

    if (metrics.available && !metrics.ok) {
        add(
            "rate_limit_event",
            "warn",
            "Metrics",
            metrics.message || "Metrics endpoint was not reachable.",
            "Check /v1/metrics before relying on quota and request counters.",
            "metrics",
        );
    }

    const requestError = lastRequestIssue;
    if (requestError) {
        add(
            requestError.status === 429 || requestError.code === "rate_limited" ? "rate_limit_event" : "audit_event",
            requestError.status === 429 ? "warn" : "info",
            requestError.label || "Last request",
            requestError.message || "Last request reported an issue.",
            "Review the typed request issue card and retry after fixing the cause.",
            "last_request_error",
        );
    }

    return timeline;
}

function resultByLabel(results, label) {
    return results.find((result) => result.label === label) || null;
}

function summarizeStatusResult(results, label) {
    const result = resultByLabel(results, label);
    return {
        available: !!result,
        ok: !!result?.ok,
        code: result?.error?.code || result?.error?.status || null,
        message: result?.ok ? "ok" : (result ? errorMessage(result.error) : "not checked"),
    };
}

function summarizeStatsResult(results) {
    const result = resultByLabel(results, "stats");
    const body = result?.body || {};
    return {
        available: !!result,
        ok: !!result?.ok,
        current_seq: body.current_seq ?? null,
        checkpoint_seq: body.checkpoint_seq ?? null,
        live_segments: body.live_segments ?? null,
        retired_segments: body.retired_segments ?? null,
        memtable_cells: body.memtable_cells ?? null,
        estimated_memtable_bytes: body.estimated_memtable_bytes ?? null,
        estimated_index_bytes: body.estimated_index_bytes ?? null,
        estimated_context_pack_bytes: body.estimated_context_pack_bytes ?? null,
        estimated_total_memory_bytes: body.estimated_total_memory_bytes ?? null,
        wal_size_bytes: body.wal_size_bytes ?? null,
        message: result?.ok ? "ok" : (result ? errorMessage(result.error) : "admin token required"),
    };
}

function summarizeValidationResult(results) {
    const result = resultByLabel(results, "validate");
    const body = result?.body || {};
    const errors = body.errors || [];
    return {
        available: !!result,
        ok: !!result?.ok && body.ok !== false && errors.length === 0,
        manifest_ok: body.manifest_ok ?? null,
        wal_ok: body.wal_ok ?? null,
        live_segments_checked: body.live_segments_checked ?? null,
        cells_checked: body.cells_checked ?? null,
        errors,
        message: result?.ok ? (errors.length ? `${errors.length} validation errors` : "ok") : (result ? errorMessage(result.error) : "admin token required"),
    };
}

function backupPosture(results) {
    const validation = summarizeValidationResult(results);
    return {
        available: canUse(ACCESS_ADMIN),
        browser_runs_backup: false,
        mode: "operator_cli",
        evidence_gate: "make backup-restore-production-pack-check",
        commands: [
            "cortexdb backup",
            "cortexdb backup-drill",
            "cortexdb backup-encrypted",
            "cortexdb backup-offsite-stage",
        ],
        validation_ok: validation.ok,
        message: canUse(ACCESS_ADMIN)
            ? "Backups are operator CLI actions; validate storage here before trusting backup posture."
            : "Admin token required to inspect storage before backup.",
    };
}

document.querySelector("#session-form").addEventListener("submit", async (event) => {
    event.preventDefault();
        const data = new FormData(event.currentTarget);
        token = data.get("token") || "";
        tenant = data.get("tenant") || "default";
        readOnlyMode = data.get("read_only") === "on";
        sessionStorage.setItem("cortexdb-dashboard-tenant", tenant);
        sessionStorage.setItem("cortexdb-dashboard-read-only", String(readOnlyMode));

    setRequestStatus("running", "Detecting capabilities");
    tokenInput.value = "";

        const detected = await detectAccessLevel();
        accessLevel = detected.level;
        accessHint = detected.hint;
    renderSessionStatus();
    refreshAccessVisibility();
    show({ auth: token ? "token_applied_memory_only" : "cleared", tenant, access: accessLevel, read_only: readOnlyMode });
    setRequestStatus("ok", `Session updated: ${accessLabel(accessLevel)}`);
});

on("#clear-session", "click", async () => {
    token = "";
    tenant = "default";
    readOnlyMode = false;
    tenantInput.value = tenant;
    tokenInput.value = "";
    readOnlyToggle.checked = readOnlyMode;
    sessionStorage.removeItem("cortexdb-dashboard-tenant");
    sessionStorage.removeItem("cortexdb-dashboard-read-only");
    token = "";

    const detected = await detectAccessLevel();
    accessLevel = detected.level;
    accessHint = detected.hint;
    renderSessionStatus();
    refreshAccessVisibility();
    show({ auth: "cleared", tenant, access: accessLevel, read_only: readOnlyMode });
});

on("#read-only-mode", "change", (event) => {
    readOnlyMode = event.currentTarget.checked;
    sessionStorage.setItem("cortexdb-dashboard-read-only", String(readOnlyMode));
    renderSessionStatus();
    refreshAccessVisibility();
    show({ tenant, access: accessLevel, read_only: readOnlyMode });
});

for (const panel of panels) {
    if (!panel.classList.contains("active")) panel.hidden = true;
}

function routeFromLocation() {
    const value = window.location.hash.replace(/^#\/?/, "");
    if (routes.has(value)) return value;
    const match = window.location.pathname.match(/^\/dashboard\/([^/?#]+)\/?$/);
    if (match && routes.has(match[1])) return match[1];
    return "overview";
}

function setRoute(route, focusMain = false) {
    const candidate = routes.has(route) ? route : "overview";
    const panel = routes.get(candidate) || routes.get("overview");

    const activeRoute = panel?.id || "overview";
    panels.forEach((item) => {
        const active = item === panel;
        item.classList.toggle("active", active);
        item.hidden = !active;
    });

    routeLinks.forEach((link) => {
        if (link.dataset.route === activeRoute) link.setAttribute("aria-current", "page");
        else link.removeAttribute("aria-current");
    });
    document.title = `${panel?.dataset?.title || "Overview"} | CortexDB Console`;

    if (focusMain) content.focus({ preventScroll: true });
}

onAll("[data-route]", "click", (event) => {
    event.preventDefault();
    const target = event.currentTarget;
    if (target.hidden) return;

    const route = target.dataset.route;
    const routeLink = target;
    const allowed = !routeLink.hidden;
    if (!allowed) return;

    const targetUrl = `/dashboard/${route}`;
    if (window.location.pathname !== targetUrl) window.history.pushState({ route }, "", targetUrl);
    setRoute(route, true);
});

window.addEventListener("popstate", () => setRoute(routeFromLocation(), true));
window.addEventListener("hashchange", () => setRoute(routeFromLocation(), true));
setRoute(routeFromLocation());

// Form-level validation helper

document.addEventListener("blur", (event) => {
    if (event.target.matches?.("input, textarea, select")) {
        if (event.target.checkValidity()) clearInvalid(event.target);
        else markInvalid(event.target);
    }
}, true);

document.addEventListener("invalid", (event) => {
    if (event.target.matches?.("input, textarea, select")) {
        markInvalid(event.target);
    }
}, true);

document.addEventListener("input", (event) => {
    if (event.target.matches?.("[aria-invalid='true']") && event.target.checkValidity()) {
        clearInvalid(event.target);
    }
});

onAll("[data-action='health']", "click", () => run("health", () => api("/v1/health")));
onAll("[data-action='operational-status']", "click", () => run("operational status", loadOperationalStatus));
onAll("[data-action='stats']", "click", () => run("stats", () => api("/v1/stats")));
onAll("[data-action='metrics']", "click", () => run("metrics", () => api("/v1/metrics")));
onAll("[data-action='validate']", "click", () => run("validate", () => api("/v1/validate")));
onAll("[data-action='flush']", "click", () => run("flush", () => api("/v1/flush", { method: "POST" })));
onAll("[data-action='compact']", "click", () => run("compact", () => api("/v1/compact", { method: "POST" })));
onAll("[data-action='cluster-status']", "click", () => run("cluster status", () => api("/v1/cluster/status")));
onAll(
    "[data-action='ann-metrics']",
    "click",
    () => run("ann metrics", () => api("/v1/ann/metrics")),
);

document.querySelector("#cell-form").addEventListener("submit", (event) => {
    event.preventDefault();
    if (!guardForm(event.currentTarget)) return;
    const data = new FormData(event.currentTarget);
    const id = encodeURIComponent(data.get("cell_id"));
    const op = data.get("op");
    if (op === "get") return run("get cell", () => api(`/v1/cell?cell_id=${id}`));
    if (!guardWriteAllowed(`${op === "delete" ? "tombstone" : "put"} cell`)) return;
    if (op === "delete")
        return run("tombstone cell", () => api(`/v1/cell?cell_id=${id}`, { method: "DELETE" }));
    return run("put cell", () => api(`/v1/cell?cell_id=${id}`, { method: "POST", body: data.get("payload") || "" }));
});

document.querySelector("#search-form").addEventListener("submit", (event) => {
    event.preventDefault();
    if (!guardForm(event.currentTarget)) return;
    const data = new FormData(event.currentTarget);
    const params = new URLSearchParams({
        scope: data.get("scope"),
        mode: data.get("mode"),
        algorithm: data.get("algorithm"),
        limit: data.get("limit") || "20",
    });
    if (data.get("mode") === "vector") params.set("vector", data.get("q") || "");
    else params.set("q", data.get("q") || "");
    run("search", () => api(`/v1/search?${params}`, { method: "POST" }));
});

on("#search-explain", "click", () => {
    const data = new FormData(document.querySelector("#search-form"));
    const params = new URLSearchParams({
        scope: data.get("scope"),
        mode: data.get("mode"),
        limit: data.get("limit") || "20",
    });
    params.set("q", data.get("q") || "");
    run("search explain", () => api(`/v1/search/explain?${params}`, { method: "POST" }));
});

document.querySelector("#ann-eval-form").addEventListener("submit", (event) => {
    event.preventDefault();
    if (!guardForm(event.currentTarget)) return;
    const data = new FormData(event.currentTarget);
    const params = new URLSearchParams({
        scope: data.get("scope"),
        vector: data.get("vector"),
        limit: data.get("limit") || "20",
    });
    run("ann evaluate", () => api(`/v1/search/ann-evaluate?${params}`, { method: "POST" }));
});

for (const id of ["aql", "context", "verify"]) {
    document.querySelector(`#${id}-form`).addEventListener("submit", (event) => {
        event.preventDefault();
        if (!guardForm(event.currentTarget)) return;
        const data = new FormData(event.currentTarget);
        const params = new URLSearchParams({ scope: data.get("scope") });
        run(id, () => api(`/v1/${id}?${params}`, { method: "POST", body: data.get("query") || "" }));
    });
}

document.querySelector("#ingest-form").addEventListener("submit", (event) => {
    event.preventDefault();
    if (!guardForm(event.currentTarget)) return;
    const data = new FormData(event.currentTarget);
    const kind = data.get("type") || "text";
    const params = new URLSearchParams({
        scope: data.get("scope"),
        source: data.get("source") || "dashboard",
    });
    if (!guardWriteAllowed("ingest")) return;
    run("ingest", () => api(`/v1/ingest/${kind}?${params}`, { method: "POST", body: data.get("document") || "" }));
});

document.querySelector("#ingest-job-form").addEventListener("submit", (event) => {
    event.preventDefault();
    if (!guardForm(event.currentTarget)) return;
    const id = encodeURIComponent(new FormData(event.currentTarget).get("job_id") || "");
    run("ingest job", () => api(`/v1/ingest/jobs/${id}`));
});

(async function init() {
    const detected = await detectAccessLevel();
    accessLevel = detected.level;
    accessHint = detected.hint;
    renderSessionStatus();
    refreshAccessVisibility();
    show({ status: "ready", access: accessLevel, read_only: readOnlyMode });
})();
