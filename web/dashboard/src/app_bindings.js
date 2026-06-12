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

document.querySelector("#ingest-jobs-list-button").addEventListener("click", () => {
    run("ingest jobs", () => api("/v1/ingest/jobs"));
});

(async function init() {
    const detected = await detectAccessLevel();
    accessLevel = detected.level;
    accessHint = detected.hint;
    renderSessionStatus();
    refreshAccessVisibility();
    show({ status: "ready", access: accessLevel, read_only: readOnlyMode });
})();
