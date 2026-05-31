const output = document.querySelector("#output");
const metrics = document.querySelector("#metrics");
const history = document.querySelector("#history");
const sessionStatus = document.querySelector("#session-status");
const roleStatus = document.querySelector("#session-role");
const requestStatus = document.querySelector("#request-status");
const content = document.querySelector("#content");
const routeLinks = Array.from(document.querySelectorAll("[data-route]"));
const panels = Array.from(document.querySelectorAll(".panel"));
const routes = new Map(panels.map((panel) => [panel.id, panel]));

const tenantInput = document.querySelector("#tenant");
const tokenInput = document.querySelector("#token");

const ACCESS_PUBLIC = "public";
const ACCESS_DATA = "data";
const ACCESS_ADMIN = "admin";

let token = "";
let tenant = sessionStorage.getItem("cortexdb-dashboard-tenant") || "default";
let accessLevel = ACCESS_PUBLIC;
let accessHint = "no token";

tenantInput.value = tenant;

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
    const hint = accessHint ? ` (${accessHint})` : "";
    roleStatus.textContent = `Access level: ${accessLabel(accessLevel)}${tokenHint}${hint}`;
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
    if (!response.ok) throw body;
    return body;
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
            node.disabled = false;
        } else {
            node.hidden = true;
            node.setAttribute("aria-hidden", "true");
            if (node instanceof HTMLButtonElement) {
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
            window.CortexDashboardReports?.renderAnnEvaluation?.(body);
            window.CortexDashboardReports?.renderContextPack?.(body);
            addHistory(label, true);
            if (body?.current_seq !== undefined || body?.ok !== undefined) renderMetrics(body);
        })
        .catch((error) => {
            setRequestStatus("error", `ERR ${label}: ${errorMessage(error)}`);
            show(error, false);
            addHistory(label, false);
        });
}

document.querySelector("#session-form").addEventListener("submit", async (event) => {
    event.preventDefault();
        const data = new FormData(event.currentTarget);
        token = data.get("token") || "";
        tenant = data.get("tenant") || "default";
        sessionStorage.setItem("cortexdb-dashboard-tenant", tenant);

    setRequestStatus("running", "Detecting capabilities");
    tokenInput.value = "";

        const detected = await detectAccessLevel();
        accessLevel = detected.level;
        accessHint = detected.hint;
    renderSessionStatus();
    refreshAccessVisibility();
    show({ auth: token ? "token_applied_memory_only" : "cleared", tenant, access: accessLevel });
    setRequestStatus("ok", `Session updated: ${accessLabel(accessLevel)}`);
});

on("#clear-session", "click", async () => {
    token = "";
    tenant = "default";
    tenantInput.value = tenant;
    tokenInput.value = "";
    sessionStorage.removeItem("cortexdb-dashboard-tenant");
    token = "";

    const detected = await detectAccessLevel();
    accessLevel = detected.level;
    accessHint = detected.hint;
    renderSessionStatus();
    refreshAccessVisibility();
    show({ auth: "cleared", tenant, access: accessLevel });
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
    show({ status: "ready", access: accessLevel });
})();
