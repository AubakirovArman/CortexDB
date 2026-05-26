pub fn html() -> String {
    r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>CortexDB Console</title>
    <style>
        :root { color-scheme: light; --bg:#f6f7f9; --panel:#fff; --ink:#1d252c; --muted:#5b6670; --line:#c9d0d7; --blue:#0f6cbd; --green:#147d64; --red:#b42318; --code:#111827; }
        * { box-sizing: border-box; }
        body { margin: 0; background: var(--bg); color: var(--ink); font: 16px/1.45 system-ui, -apple-system, Segoe UI, sans-serif; }
        header { border-block-end: 1px solid var(--line); background: var(--panel); }
        .shell { inline-size: min(1440px, 100%); margin-inline: auto; padding: 18px; }
        .topbar { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; justify-content: space-between; }
        h1 { margin: 0; font-size: 1.25rem; letter-spacing: 0; }
        h2 { margin: 0 0 12px; font-size: 1rem; letter-spacing: 0; }
        label { display: block; margin-block-end: 5px; color: var(--muted); font-weight: 650; }
        input, textarea, select { inline-size: 100%; min-block-size: 44px; border: 1px solid var(--line); border-radius: 6px; padding: 10px; background: #fff; color: var(--ink); font: inherit; }
        textarea { min-block-size: 128px; resize: vertical; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
        input:focus-visible, textarea:focus-visible, select:focus-visible, button:focus-visible { outline: 3px solid color-mix(in srgb, var(--blue) 35%, transparent); outline-offset: 2px; }
        button { min-block-size: 42px; border: 0; border-radius: 6px; padding: 9px 13px; background: var(--blue); color: #fff; font-weight: 700; cursor: pointer; }
        button.secondary { background: #44515d; }
        button.danger { background: var(--red); }
        button:disabled { opacity: .65; cursor: progress; }
        main { display: grid; grid-template-columns: 240px minmax(0, 1fr) minmax(320px, 460px); gap: 18px; align-items: start; }
        nav, section, aside { background: var(--panel); border: 1px solid var(--line); border-radius: 8px; padding: 14px; }
        nav { position: sticky; inset-block-start: 12px; }
        .tabs { display: grid; gap: 8px; }
        .tab { inline-size: 100%; text-align: start; background: transparent; color: var(--ink); border: 1px solid var(--line); }
        .tab[aria-selected="true"] { background: #e8f1fb; border-color: var(--blue); color: #083b75; }
        .panel { display: none; }
        .panel.active { display: block; }
        .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 12px; }
        .field { margin-block-end: 12px; }
        .actions { display: flex; flex-wrap: wrap; gap: 8px; margin-block-start: 10px; }
        .status { display: grid; gap: 8px; grid-template-columns: repeat(auto-fit, minmax(140px, 1fr)); margin-block-end: 12px; }
        .metric { border: 1px solid var(--line); border-radius: 6px; padding: 10px; background: #fbfcfd; }
        .metric span { display: block; color: var(--muted); font-size: .85rem; }
        .metric strong { font-size: 1.2rem; }
        pre { margin: 0; max-block-size: 70dvh; overflow: auto; scrollbar-gutter: stable; border-radius: 6px; padding: 12px; background: var(--code); color: #d1fae5; font: .9rem/1.45 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; white-space: pre-wrap; overflow-wrap: anywhere; }
        .ok { color: var(--green); }
        .error { color: var(--red); }
        @media (max-width: 1050px) { main { grid-template-columns: 1fr; } nav { position: static; } aside { order: 3; } }
    </style>
</head>
<body>
    <header>
        <div class="shell topbar">
            <h1>CortexDB Console</h1>
            <form id="auth-form" class="topbar">
                <label for="token">Bearer token</label>
                <input id="token" name="token" type="password" autocomplete="current-password">
                <button type="submit">Apply</button>
            </form>
        </div>
    </header>
    <main class="shell">
        <nav aria-label="Console views">
            <div class="tabs" role="tablist" aria-label="CortexDB tools">
                <button class="tab" type="button" role="tab" aria-selected="true" aria-controls="ops" data-tab="ops">Ops</button>
                <button class="tab" type="button" role="tab" aria-selected="false" aria-controls="cells" data-tab="cells">Cells</button>
                <button class="tab" type="button" role="tab" aria-selected="false" aria-controls="search" data-tab="search">Search</button>
                <button class="tab" type="button" role="tab" aria-selected="false" aria-controls="aql" data-tab="aql">AQL</button>
                <button class="tab" type="button" role="tab" aria-selected="false" aria-controls="context" data-tab="context">Context</button>
                <button class="tab" type="button" role="tab" aria-selected="false" aria-controls="verify" data-tab="verify">Verify</button>
            </div>
        </nav>
        <section aria-live="polite">
            <section id="ops" class="panel active" role="tabpanel">
                <h2>Ops</h2>
                <div class="status" id="metrics"></div>
                <div class="actions">
                    <button type="button" data-action="health">Health</button>
                    <button type="button" data-action="stats">Stats</button>
                    <button type="button" data-action="validate">Validate</button>
                    <button type="button" data-action="flush" class="secondary">Flush</button>
                    <button type="button" data-action="compact" class="secondary">Compact</button>
                </div>
            </section>
            <section id="cells" class="panel" role="tabpanel">
                <h2>Cells</h2>
                <form id="cell-form">
                    <div class="grid">
                        <div class="field"><label for="cell-id">Cell ID</label><input id="cell-id" name="cell_id" inputmode="numeric" required value="1"></div>
                        <div class="field"><label for="cell-op">Operation</label><select id="cell-op" name="op"><option value="put">Put</option><option value="get">Get</option><option value="delete">Tombstone</option></select></div>
                    </div>
                    <div class="field"><label for="cell-payload">Payload</label><textarea id="cell-payload" name="payload">scope=project:investments
status=ready
type=fact
source=console

Solar budget note</textarea></div>
                    <button type="submit">Run Cell Operation</button>
                </form>
            </section>
            <section id="search" class="panel" role="tabpanel">
                <h2>Search</h2>
                <form id="search-form">
                    <div class="grid">
                        <div class="field"><label for="search-scope">Scope</label><input id="search-scope" name="scope" required value="project:investments"></div>
                        <div class="field"><label for="search-mode">Mode</label><select id="search-mode" name="mode"><option value="keyword">Keyword</option><option value="vector">Vector</option></select></div>
                        <div class="field"><label for="search-algorithm">Vector algorithm</label><select id="search-algorithm" name="algorithm"><option value="ann">ANN</option><option value="exact">Exact</option></select></div>
                        <div class="field"><label for="search-limit">Limit</label><input id="search-limit" name="limit" inputmode="numeric" value="20"></div>
                    </div>
                    <div class="field"><label for="search-query">Query or vector</label><input id="search-query" name="q" value="budget"></div>
                    <button type="submit">Search</button>
                </form>
            </section>
            <section id="aql" class="panel" role="tabpanel">
                <h2>AQL</h2>
                <form id="aql-form">
                    <div class="field"><label for="aql-scope">Scope</label><input id="aql-scope" name="scope" required value="project:investments"></div>
                    <div class="field"><label for="aql-query">Statement</label><textarea id="aql-query" name="query">RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;</textarea></div>
                    <button type="submit">Run AQL</button>
                </form>
            </section>
            <section id="context" class="panel" role="tabpanel">
                <h2>Context</h2>
                <form id="context-form">
                    <div class="field"><label for="context-scope">Scope</label><input id="context-scope" name="scope" required value="project:investments"></div>
                    <div class="field"><label for="context-query">Statement</label><textarea id="context-query" name="query">RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;</textarea></div>
                    <button type="submit">Build Context Pack</button>
                </form>
            </section>
            <section id="verify" class="panel" role="tabpanel">
                <h2>Verify</h2>
                <form id="verify-form">
                    <div class="field"><label for="verify-scope">Scope</label><input id="verify-scope" name="scope" required value="project:investments"></div>
                    <div class="field"><label for="verify-query">Statement</label><textarea id="verify-query" name="query">VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN investment_projects;</textarea></div>
                    <button type="submit">Verify Fact</button>
                </form>
            </section>
        </section>
        <aside>
            <h2>Response</h2>
            <pre id="output" tabindex="0">{"status":"ready"}</pre>
        </aside>
    </main>
    <script>
        const output = document.querySelector("#output");
        const metrics = document.querySelector("#metrics");
        let token = "";

        function headers() {
            return token ? { authorization: `Bearer ${token}` } : {};
        }
        function show(value, ok = true) {
            output.className = ok ? "ok" : "error";
            output.textContent = typeof value === "string" ? value : JSON.stringify(value, null, 2);
        }
        async function api(path, init = {}) {
            const response = await fetch(path, { ...init, headers: { ...headers(), ...(init.headers || {}) } });
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
                if (body.current_seq !== undefined || body.ok !== undefined) renderMetrics(body);
            } catch (error) {
                show(error, false);
            }
        }
        function renderMetrics(body) {
            const items = Object.entries(body).filter(([, value]) => typeof value !== "object").slice(0, 8);
            metrics.innerHTML = items.map(([key, value]) => `<div class="metric"><span>${key}</span><strong>${value}</strong></div>`).join("");
        }
        document.querySelector("#auth-form").addEventListener("submit", event => {
            event.preventDefault();
            token = new FormData(event.currentTarget).get("token") || "";
            show({ auth: token ? "token_applied" : "cleared" });
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
        for (const id of ["aql", "context", "verify"]) {
            document.querySelector(`#${id}-form`).addEventListener("submit", event => {
                event.preventDefault();
                const data = new FormData(event.currentTarget);
                const params = new URLSearchParams({ scope: data.get("scope") });
                run(id, () => api(`/v1/${id}?${params}`, { method: "POST", body: data.get("query") || "" }));
            });
        }
        run("stats", () => api("/v1/stats"));
    </script>
</body>
</html>
"##
    .to_owned()
}
