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
    refreshDangerousOperationVisibility();

    const requested = routeFromLocation();
    const panel = routes.get(requested);
    if (!panel || panel.hidden) {
        const fallback = firstAllowedRoute();
        if (fallback !== requested) {
            setRoute(fallback, false);
        }
    }
    renderRoleUiReport();
    window.CortexDashboardReports?.renderPermissionsView?.(permissionState());
}

function refreshDangerousOperationVisibility() {
    for (const node of document.querySelectorAll("[data-dangerous='true']")) {
        const minAccess = node.getAttribute("data-access") || ACCESS_DATA;
        const allowedByRole = canUse(minAccess);
        const blockedByReadOnly = readOnlyMode && node.dataset.write === "true";
        node.hidden = !allowedByRole || blockedByReadOnly;
        if (node instanceof HTMLOptionElement && node.hidden && node.selected) {
            const select = node.closest("select");
            const safeOption = select?.querySelector("option:not([hidden])");
            if (safeOption) select.value = safeOption.value;
        }
        if (node instanceof HTMLButtonElement || node instanceof HTMLInputElement || node instanceof HTMLSelectElement) {
            node.disabled = !allowedByRole || blockedByReadOnly;
            if (node.disabled) node.setAttribute("aria-disabled", "true");
            else node.removeAttribute("aria-disabled");
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
            window.CortexDashboardReports?.renderSloDashboard?.(body);
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
