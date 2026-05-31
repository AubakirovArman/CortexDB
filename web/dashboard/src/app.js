const output = document.querySelector("#output");
const metrics = document.querySelector("#metrics");
const history = document.querySelector("#history");
const sessionStatus = document.querySelector("#session-status");
const requestStatus = document.querySelector("#request-status");
const content = document.querySelector("#content");
const routeLinks = Array.from(document.querySelectorAll("[data-route]"));
const panels = Array.from(document.querySelectorAll(".panel"));
const routes = new Map(panels.map(panel => [panel.id, panel]));
let token = "";
let tenant = "default";

function headers() {
    return token ? { authorization: `Bearer ${token}` } : {};
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
    document.querySelectorAll(selector).forEach(node => node.addEventListener(event, handler));
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
    return "request failed";
}

function setRequestStatus(kind, text) {
    requestStatus.className = `request-status ${kind}`;
    requestStatus.textContent = text;
}

async function api(path, init = {}) {
    const response = await fetch(scoped(path), { ...init, headers: { ...headers(), ...(init.headers || {}) } });
    const text = await response.text();
    let body;
    try { body = JSON.parse(text); } catch { body = text; }
    if (!response.ok) throw body;
    return body;
}

async function run(label, task) {
    setRequestStatus("running", `Running ${label}`);
    show(`${label}...`);
    try {
        const body = await task();
        setRequestStatus("ok", `OK ${label}`);
        show(body);
        addHistory(label, true);
        if (body.current_seq !== undefined || body.ok !== undefined) renderMetrics(body);
    } catch (error) {
        setRequestStatus("error", `ERR ${label}: ${errorMessage(error)}`);
        show(error, false);
        addHistory(label, false);
    }
}

function renderMetrics(body) {
    metrics.replaceChildren();
    for (const [key, value] of Object.entries(body).filter(([, item]) => typeof item !== "object").slice(0, 8)) {
        const card = document.createElement("div");
        const label = document.createElement("span");
        const strong = document.createElement("strong");
        card.className = "metric";
        label.textContent = key;
        strong.textContent = String(value);
        card.append(label, strong);
        metrics.append(card);
    }
}

document.querySelector("#session-form").addEventListener("submit", event => {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    token = data.get("token") || "";
    tenant = data.get("tenant") || "default";
    sessionStatus.textContent = `Tenant: ${tenant}${token ? " · bearer token set" : ""}`;
    show({ auth: token ? "token_applied" : "cleared", tenant });
});

document.querySelectorAll(".panel").forEach(panel => {
    if (!panel.classList.contains("active")) panel.hidden = true;
});

function routeFromHash() {
    const value = window.location.hash.replace(/^#\/?/, "");
    return routes.has(value) ? value : "overview";
}

function setRoute(route, focusMain = false) {
    const panel = routes.get(route) || routes.get("overview");
    const activeRoute = panel.id;
    panels.forEach(item => {
        const active = item === panel;
        item.classList.toggle("active", active);
        item.hidden = !active;
    });
    routeLinks.forEach(link => {
        if (link.dataset.route === activeRoute) link.setAttribute("aria-current", "page");
        else link.removeAttribute("aria-current");
    });
    document.title = `${panel.dataset.title || "Overview"} | CortexDB Console`;
    if (focusMain) content.focus({ preventScroll: true });
}

onAll("[data-route]", "click", event => {
    const route = event.currentTarget.dataset.route;
    if (routeFromHash() === route) setRoute(route, true);
});

window.addEventListener("hashchange", () => setRoute(routeFromHash(), true));
setRoute(routeFromHash());

document.addEventListener("blur", event => {
    if (event.target.matches?.("input, textarea, select")) {
        event.target.toggleAttribute("aria-invalid", !event.target.checkValidity());
    }
}, true);

document.addEventListener("input", event => {
    if (event.target.matches?.("[aria-invalid='true']") && event.target.checkValidity()) {
        event.target.removeAttribute("aria-invalid");
    }
});

onAll("[data-action='health']", "click", () => run("health", () => api("/v1/health")));
onAll("[data-action='stats']", "click", () => run("stats", () => api("/v1/stats")));
onAll("[data-action='metrics']", "click", () => run("metrics", () => api("/v1/metrics")));
onAll("[data-action='validate']", "click", () => run("validate", () => api("/v1/validate")));
onAll("[data-action='flush']", "click", () => run("flush", () => api("/v1/flush", { method: "POST" })));
onAll("[data-action='compact']", "click", () => run("compact", () => api("/v1/compact", { method: "POST" })));
onAll("[data-action='cluster-status']", "click", () => run("cluster status", () => api("/v1/cluster/status")));
onAll("[data-action='ann-metrics']", "click", () => run("ann metrics", () => api("/v1/ann/metrics")));

document.querySelector("#cell-form").addEventListener("submit", event => {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    const id = encodeURIComponent(data.get("cell_id"));
    const op = data.get("op");
    if (op === "get") return run("get cell", () => api(`/v1/cell?cell_id=${id}`));
    if (op === "delete") return run("tombstone cell", () => api(`/v1/cell?cell_id=${id}`, { method: "DELETE" }));
    return run("put cell", () => api(`/v1/cell?cell_id=${id}`, { method: "POST", body: data.get("payload") || "" }));
});

document.querySelector("#search-form").addEventListener("submit", event => {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    const params = new URLSearchParams({ scope: data.get("scope"), mode: data.get("mode"), algorithm: data.get("algorithm"), limit: data.get("limit") || "20" });
    if (data.get("mode") === "vector") params.set("vector", data.get("q") || "");
    else params.set("q", data.get("q") || "");
    run("search", () => api(`/v1/search?${params}`, { method: "POST" }));
});

on("#search-explain", "click", () => {
    const data = new FormData(document.querySelector("#search-form"));
    const params = new URLSearchParams({ scope: data.get("scope"), mode: data.get("mode"), limit: data.get("limit") || "20" });
    params.set("q", data.get("q") || "");
    run("search explain", () => api(`/v1/search/explain?${params}`, { method: "POST" }));
});

document.querySelector("#ann-eval-form").addEventListener("submit", event => {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    const params = new URLSearchParams({ scope: data.get("scope"), vector: data.get("vector"), limit: data.get("limit") || "20" });
    run("ann evaluate", () => api(`/v1/search/ann-evaluate?${params}`, { method: "POST" }));
});

for (const id of ["aql", "context", "verify"]) {
    document.querySelector(`#${id}-form`).addEventListener("submit", event => {
        event.preventDefault();
        const data = new FormData(event.currentTarget);
        const params = new URLSearchParams({ scope: data.get("scope") });
        run(id, () => api(`/v1/${id}?${params}`, { method: "POST", body: data.get("query") || "" }));
    });
}

document.querySelector("#ingest-form").addEventListener("submit", event => {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    const kind = data.get("type") || "text";
    const params = new URLSearchParams({ scope: data.get("scope"), source: data.get("source") || "dashboard" });
    run("ingest", () => api(`/v1/ingest/${kind}?${params}`, { method: "POST", body: data.get("document") || "" }));
});

document.querySelector("#ingest-job-form").addEventListener("submit", event => {
    event.preventDefault();
    const id = encodeURIComponent(new FormData(event.currentTarget).get("job_id") || "");
    run("ingest job", () => api(`/v1/ingest/jobs/${id}`));
});

run("stats", () => api("/v1/stats"));
