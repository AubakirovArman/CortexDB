import json
import os
import urllib.request
import urllib.parse
from fastapi import FastAPI, Request
from fastapi.responses import HTMLResponse

app = FastAPI()

KIMI_API_BASE_URL = os.environ.get("KIMI_API_BASE_URL", "http://127.0.0.1:8000/v1")
KIMI_MODEL = os.environ.get("KIMI_MODEL", "google/gemma-4-31B-it")
KIMI_KEY_PATH = os.environ.get("KIMI_API_KEY_FILE", "/mnt/hf_model_weights/arman/3bit/.kimi")
KIMI_KEY = os.environ.get("KIMI_API_KEY", "").strip()
if not KIMI_KEY and os.path.exists(KIMI_KEY_PATH):
    with open(KIMI_KEY_PATH, "r") as f:
        KIMI_KEY = f.read().strip()

def query_cortex(endpoint, method, tenant, data_str):
    url = f"http://127.0.0.1:8090{endpoint}?tenant={tenant}"
    if endpoint in ["/v1/context", "/v1/verify"]:
        url += "&scope=" + ("project:investments" if tenant == "financial_records" else "legal:contracts")
    req = urllib.request.Request(
        url,
        data=data_str.encode("utf-8"),
        headers={"Content-Type": "text/plain"},
        method=method
    )
    try:
        with urllib.request.urlopen(req) as res:
            return json.loads(res.read().decode("utf-8"))
    except Exception as e:
        return {"error": str(e)}

@app.get("/", response_class=HTMLResponse)
async def serve_ui():
    return """<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>CortexDB + Kimi RAG Agent</title>
    <script src="https://cdn.tailwindcss.com"></script>
</head>
<body class="bg-slate-950 text-slate-100 flex h-screen font-sans">
    <!-- Left Panel: Data & Settings -->
    <div class="w-96 bg-slate-900 border-r border-slate-800 p-6 flex flex-col justify-between overflow-y-auto">
        <div class="space-y-6">
            <div>
                <h1 class="text-xl font-black text-sky-400">CORTEXDB + LLM</h1>
                <p class="text-xs text-slate-500 font-bold uppercase mt-1">Universal RAG Playground</p>
            </div>
            
            <div class="space-y-2">
                <label class="block text-xs font-bold text-slate-400 uppercase">Select Isolated Database</label>
                <select id="realm" class="w-full bg-slate-950 border border-slate-800 rounded-lg p-3 text-sm text-sky-400 focus:outline-none focus:border-sky-500">
                    <option value="financial_records">financial_records (Revenue, Expenses, Budgets)</option>
                    <option value="legal_compliance">legal_compliance (Contracts, Regulations, Signatories)</option>
                </select>
            </div>

            <!-- LLM Provider Panel -->
            <div class="border-t border-slate-800 pt-4 space-y-4">
                <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider">🔌 LLM Provider Settings</h3>
                
                <div class="space-y-1.5">
                    <label class="text-[10px] font-bold text-slate-500 uppercase">API Base URL</label>
                    <input type="text" id="llm-base" placeholder="http://127.0.0.1:8000/v1" class="w-full bg-slate-950 border border-slate-800 rounded-lg p-2.5 text-xs text-sky-400 focus:outline-none focus:border-sky-500" onchange="localStorage.setItem('cortex_llm_base', this.value)">
                </div>

                <div class="space-y-1.5">
                    <label class="text-[10px] font-bold text-slate-500 uppercase">API Secret Key</label>
                    <input type="password" id="llm-key" placeholder="KIMI_API_KEY or paste per session" class="w-full bg-slate-950 border border-slate-800 rounded-lg p-2.5 text-xs text-sky-400 focus:outline-none focus:border-sky-500" onchange="localStorage.setItem('cortex_llm_key', this.value)">
                </div>

                <div class="space-y-1.5">
                    <label class="text-[10px] font-bold text-slate-500 uppercase">Model Name</label>
                    <input type="text" id="llm-model" placeholder="google/gemma-4-31B-it" class="w-full bg-slate-950 border border-slate-800 rounded-lg p-2.5 text-xs text-sky-400 focus:outline-none focus:border-sky-500" onchange="localStorage.setItem('cortex_llm_model', this.value)">
                </div>
                <p class="text-[11px] text-slate-500 leading-relaxed">Leave empty to use the local Gemma-4-31B-it vLLM server default. Fully OpenAI-compatible!</p>
            </div>
        </div>
        <div class="border-t border-slate-800 pt-4 text-xs text-slate-500 space-y-1 mt-6">
            <p>🔋 Connected to Local Gemma-4 (vLLM)</p>
            <p>💾 Active CortexDB Core (Port 8090)</p>
            <p>🌐 Running on http://127.0.0.1:8085</p>
        </div>
    </div>

    <!-- Right Panel: Chat Interface -->
    <div class="flex-1 flex flex-col h-screen">
        <header class="h-16 border-b border-slate-850 bg-slate-900/40 flex items-center px-8">
            <h2 class="text-md font-bold text-slate-200">🤖 Audited Cognitive Memory Chatbot (Gemma-4)</h2>
        </header>
        
        <div id="chat-messages" class="flex-1 overflow-y-auto p-8 space-y-6">
            <div class="bg-slate-900/50 border border-slate-800 rounded-xl p-5 max-w-3xl">
                <p class="text-sm text-slate-300">Welcome! Ask any question about your structured databases. CortexDB will retrieve highly-focused <strong>ContextPacks</strong> and apply deterministic <strong>Fact Verification</strong> to audit your questions on the fly!</p>
                <div class="mt-3 flex flex-wrap gap-2">
                    <button onclick="fillQuery('What is the approved budget for the Solar Plant project?')" class="text-xs bg-slate-850 hover:bg-slate-800 text-sky-400 px-3 py-1.5 rounded-lg border border-slate-800 transition">🔍 Ask solar plant budget (Triggers conflict!)</button>
                    <button onclick="fillQuery('Who is the authorized signatory for Entity_A?')" class="text-xs bg-slate-850 hover:bg-slate-800 text-sky-400 px-3 py-1.5 rounded-lg border border-slate-800 transition">📝 Ask legal Entity_A signatory</button>
                </div>
            </div>
        </div>

        <div class="p-6 border-t border-slate-850 bg-slate-900/20">
            <div class="max-w-4xl mx-auto flex gap-4">
                <input type="text" id="user-input" placeholder="Type your corporate audit question..." class="flex-1 bg-slate-900 border border-slate-800 rounded-xl px-4 py-3.5 text-sm focus:outline-none focus:border-sky-500 text-sky-400" onkeydown="if(event.key==='Enter') sendMessage()">
                <button onclick="sendMessage()" class="bg-sky-600 hover:bg-sky-500 px-6 py-3 rounded-xl text-sm font-bold transition">Send Message</button>
            </div>
        </div>
    </div>

    <script>
        function fillQuery(text) {
            document.getElementById('user-input').value = text;
            if (text.includes('signatory')) {
                document.getElementById('realm').value = 'legal_compliance';
            } else {
                document.getElementById('realm').value = 'financial_records';
            }
        }

        async function sendMessage() {
            const input = document.getElementById('user-input');
            const query = input.value.trim();
            if(!query) return;
            input.value = '';

            const realm = document.getElementById('realm').value;
            const chat = document.getElementById('chat-messages');

            // User Message
            chat.innerHTML += `<div class="flex justify-end"><div class="bg-sky-600/20 border border-sky-500/30 rounded-xl p-4 max-w-2xl"><p class="text-sm font-semibold text-sky-400 mb-1">User</p><p class="text-sm">${query}</p></div></div>`;
            chat.scrollTop = chat.scrollHeight;

            // Loading state
            const loadId = 'load-' + Date.now();
            chat.innerHTML += `<div id="${loadId}" class="flex gap-3 text-sm text-slate-500 italic animate-pulse">⚙️ CortexDB is compiling ContextPack and auditing fact...</div>`;
            chat.scrollTop = chat.scrollHeight;

            const base_url = document.getElementById('llm-base').value || localStorage.getItem('cortex_llm_base') || "";
            const api_key = document.getElementById('llm-key').value || localStorage.getItem('cortex_llm_key') || "";
            const model = document.getElementById('llm-model').value || localStorage.getItem('cortex_llm_model') || "";

            try {
                const res = await fetch('/api/chat', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ query, realm, base_url, api_key, model })
                });
                const data = await res.json();
                document.getElementById(loadId).remove();

                let conflictAlert = "";
                if (data.verification && data.verification.verdict === "mixed_evidence") {
                    const conflict = data.verification.numeric_conflicts[0];
                    conflictAlert = `
                        <div class="bg-rose-500/10 border border-rose-500/30 text-rose-400 p-4 rounded-xl mb-4 text-xs font-semibold">
                            ⚠️ <strong>CortexDB Conflict Alert:</strong> Detected contradicting numeric claims for metric "${conflict.metric}"! Left claimed "${conflict.left}" vs Right claimed "${conflict.right}".
                        </div>
                    `;
                }

                let sourcesHtml = "";
                if (data.context && data.context.cells && data.context.cells.length > 0) {
                    sourcesHtml = `<div class="mt-4 pt-4 border-t border-slate-800/60"><p class="text-[10px] font-bold text-slate-500 uppercase mb-2">💾 Retrieved Context Citations</p><div class="space-y-1.5">`;
                    data.context.cells.forEach(cell => {
                        sourcesHtml += `<div class="bg-slate-950 p-2.5 rounded text-xs border border-slate-800/40"><span class="text-sky-400 font-bold font-mono">[Cell ${cell.cell_id}]</span> <span class="text-slate-400">${cell.citation}</span> <p class="text-[11px] text-slate-500 mt-1 italic">"${cell.payload_text.split('\\n\\n')[1] || ""}"</p></div>`;
                    });
                    sourcesHtml += `</div></div>`;
                }

                chat.innerHTML += `
                    <div class="flex justify-start">
                        <div class="bg-slate-900 border border-slate-800 rounded-xl p-5 max-w-3xl">
                            <p class="text-sm font-semibold text-emerald-400 mb-2">🤖 AI Agent (Cortex-Guided)</p>
                            ${conflictAlert}
                            <p class="text-sm text-slate-300 leading-relaxed">${data.response}</p>
                            ${sourcesHtml}
                        </div>
                    </div>
                `;
                chat.scrollTop = chat.scrollHeight;
            } catch(e) {
                document.getElementById(loadId).textContent = "❌ Failed to connect to local assistant.";
            }
        }

        // Init
        document.getElementById('llm-base').value = localStorage.getItem('cortex_llm_base') || "";
        document.getElementById('llm-key').value = localStorage.getItem('cortex_llm_key') || "";
        document.getElementById('llm-model').value = localStorage.getItem('cortex_llm_model') || "";
    </script>
</body>
</html>
"""

@app.post("/api/chat")
async def chat_endpoint(payload: dict):
    query = payload.get("query", "")
    realm = payload.get("realm", "default")
    
    client_base = payload.get("base_url", "").strip()
    client_key = payload.get("api_key", "").strip()
    client_model = payload.get("model", "").strip()

    base_url = client_base if client_base else KIMI_API_BASE_URL
    api_key = client_key if client_key else KIMI_KEY
    model = client_model if client_model else KIMI_MODEL

    safe_query = query.replace('"', '\\"').replace("'", "\\'")
    # 1. Compile ContextPack from CortexDB
    aql_query = f'RETRIEVE CONTEXT FOR TASK "{safe_query}" IN BRAIN default;'
    context_pack = query_cortex("/v1/context", "POST", realm, aql_query)
    
    # 2. Run Fact Verification (pre-audit)
    verify_query = f'VERIFY FACT "{safe_query}" IN BRAIN default;'
    verification = query_cortex("/v1/verify", "POST", realm, verify_query)

    # 3. Compile local fallback text (in case LLM key fails/is 401)
    context_str = ""
    local_summary = ""
    if "cells" in context_pack and len(context_pack["cells"]) > 0:
        local_summary = "Based strictly on the audited local CortexDB database cells:\n"
        for cell in context_pack["cells"]:
            body_text = cell['payload_text'].split('\n\n')[-1]
            local_summary += f"- {body_text} (Source: {cell['citation']})\n"
            context_str += f"Source: {cell['citation']}\nPayload: {cell['payload_text']}\n\n"
    else:
        local_summary = "No relevant context cells found in this CortexDB Realm database."

    system_prompt = (
        "You are a highly professional, audited corporate assistant.\n"
        "Answer the user's question accurately based strictly on the provided audited context cells from CortexDB. "
        "Always cite the source name/page when presenting facts.\n\n"
        f"Audited CortexDB Context Cells:\n{context_str}"
    )

    kimi_payload = {
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": query}
        ]
    }

    if not api_key:
        ai_text = (
            f"⚠️ <strong>[CortexDB Local Fallback Mode - LLM Key Missing]:</strong><br>"
            f"<span class='text-amber-400'>Set KIMI_API_KEY or KIMI_API_KEY_FILE on the server, or paste an API key for this browser session.</span><br><br>"
            f"CortexDB still compiled and audited the following Local Context Pack successfully:<br><br>"
            f"{local_summary.replace('\n', '<br>')}"
        )
    else:
        url = f"{base_url}/chat/completions"
        req = urllib.request.Request(
            url,
            data=json.dumps(kimi_payload).encode("utf-8"),
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {api_key}"
            },
            method="POST"
        )

        try:
            with urllib.request.urlopen(req) as res:
                kimi_response = json.loads(res.read().decode("utf-8"))
                ai_text = kimi_response["choices"][0]["message"]["content"]
        except Exception as e:
            ai_text = (
                f"⚠️ <strong>[CortexDB Local Fallback Mode - LLM Auth Failed]:</strong><br>"
                f"<span class='text-rose-400'>The provided API key/credential returned an error ({e}).</span><br><br>"
                f"However, CortexDB compiled and audited the following Local Context Pack successfully:<br><br>"
                f"{local_summary.replace('\n', '<br>')}"
            )

+
    return {
        "response": ai_text,
        "context": context_pack,
        "verification": verification
    }
