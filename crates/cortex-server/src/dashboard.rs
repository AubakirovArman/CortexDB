pub fn html() -> String {
    r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>CortexDB Memory Console</title>
    <style>
        body { font-family: system-ui, -apple-system, sans-serif; background: #0f172a; color: #f8fafc; margin: 0; padding: 24px; }
        .container { max-width: 1200px; margin: 0 auto; }
        h1 { color: #38bdf8; border-bottom: 2px solid #334155; padding-bottom: 12px; }
        .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 24px; margin-top: 24px; }
        .card { background: #1e293b; border: 1px solid #334155; border-radius: 8px; padding: 20px; }
        textarea { width: 100%; height: 120px; background: #0f172a; color: #38bdf8; border: 1px solid #334155; border-radius: 4px; font-family: monospace; padding: 12px; box-sizing: border-box; }
        button { background: #0284c7; color: white; border: none; padding: 10px 20px; border-radius: 4px; cursor: pointer; font-weight: bold; }
        button:hover { background: #0369a1; }
        pre { background: #0f172a; color: #38bdf8; padding: 12px; border-radius: 4px; overflow-x: auto; font-family: monospace; max-height: 400px; }
    </style>
</head>
<body>
    <div class="container">
        <h1>CortexDB Interactive Memory Console</h1>
        <div class="grid">
            <div class="card">
                <h2>AQL / ContextPack Playground</h2>
                <p>Run AQL queries to build budgeting Context Packs on the fly:</p>
                <textarea id="aql-query">RETRIEVE CONTEXT FOR "Solar Plant" LIMIT 5;</textarea>
                <br><br>
                <button onclick="runQuery('/v1/context')">Build Context Pack</button>
                <button onclick="runQuery('/v1/verify')">Verify Fact</button>
            </div>
            <div class="card">
                <h2>Execution Output</h2>
                <pre id="output">Run a query to view structured JSON schema response...</pre>
            </div>
        </div>
    </div>
    <script>
        async function runQuery(endpoint) {
            const query = document.getElementById('aql-query').value;
            const res = await fetch(endpoint + "?scope=project:investments", {
                method: "POST",
                body: query
            });
            const json = await res.json();
            document.getElementById('output').textContent = JSON.stringify(json, null, 2);
        }
    </script>
</body>
</html>
"#
    .to_owned()
}
