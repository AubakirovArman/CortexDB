pub const SCRIPT: &str = r##"        const output = document.querySelector("#output");
        const metrics = document.querySelector("#metrics");
        const history = document.querySelector("#history");
        let token = "";
        let tenant = "default";

        function headers() {
            return token ? { authorization: `Bearer ${token}` } : {};
        }
        function scoped(path) {
            if (!tenant || tenant === "default") return path;
            return `${path}${path.includes("?") ? "&" : "?"}tenant=${encodeURIComponent(tenant)}`;
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
        async function api(path, init = {}) {
            const response = await fetch(scoped(path), { ...init, headers: { ...headers(), ...(init.headers || {}) } });
            const text = await response.text();
            let body;
            try { body = JSON.parse(text); } catch { body = text; }
            if (!response.ok) throw body;
            return body;
        }
        async function run(label, task) {
            show(`${label}...`);
            try {
                const body = await task();
                show(body);
                addHistory(label, true);
                if (body.current_seq !== undefined || body.ok !== undefined) renderMetrics(body);
            } catch (error) {
                show(error, false);
                addHistory(label, false);
            }
        }
        function renderMetrics(body) {
            const items = Object.entries(body).filter(([, value]) => typeof value !== "object").slice(0, 8);
            metrics.innerHTML = items.map(([key, value]) => `<div class="metric"><span>${key}</span><strong>${value}</strong></div>`).join("");
        }
        document.querySelector("#session-form").addEventListener("submit", event => {
            event.preventDefault();
            const data = new FormData(event.currentTarget);
            token = data.get("token") || "";
            tenant = data.get("tenant") || "default";
            show({ auth: token ? "token_applied" : "cleared", tenant });
        });
        document.querySelectorAll(".tab").forEach(tab => tab.addEventListener("click", () => {
            document.querySelectorAll(".tab").forEach(item => item.setAttribute("aria-selected", String(item === tab)));
            document.querySelectorAll(".panel").forEach(panel => panel.classList.toggle("active", panel.id === tab.dataset.tab));
        }));
        document.querySelector("[data-action='health']").addEventListener("click", () => run("health", () => api("/v1/health")));
        document.querySelector("[data-action='stats']").addEventListener("click", () => run("stats", () => api("/v1/stats")));
        document.querySelector("[data-action='validate']").addEventListener("click", () => run("validate", () => api("/v1/validate")));
        document.querySelector("[data-action='flush']").addEventListener("click", () => run("flush", () => api("/v1/flush", { method: "POST" })));
        document.querySelector("[data-action='compact']").addEventListener("click", () => run("compact", () => api("/v1/compact", { method: "POST" })));
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
        run("stats", () => api("/v1/stats"));"##;
