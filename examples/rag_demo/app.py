import json
import os
import urllib.request
import urllib.parse
from fastapi import FastAPI
from fastapi.responses import HTMLResponse

app = FastAPI()

# ── Configuration ──────────────────────────────────────────────────────────
VLLM_URL = os.getenv("VLLM_URL", "http://127.0.0.1:8018/v1/chat/completions")
VLLM_API_KEY = os.getenv("VLLM_API_KEY", "5zxyqINY37FEicJ_rMfpacCBxhcjJhE0wcSTi4ADgus")
VLLM_MODEL = os.getenv("VLLM_MODEL", "/mnt/hf_model_weights/arman/3bit/models/google-gemma-4-31B-it")
CORTEX_HOST = os.getenv("CORTEX_HOST", "http://127.0.0.1:8090")


# ── CortexDB helpers ───────────────────────────────────────────────────────
def query_cortex(endpoint: str, scope: str, body: str) -> dict:
    url = f"{CORTEX_HOST}{endpoint}?scope={urllib.parse.quote(scope)}"
    req = urllib.request.Request(
        url,
        data=body.encode("utf-8"),
        headers={"Content-Type": "text/plain"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as res:
            return json.loads(res.read().decode("utf-8"))
    except Exception as e:
        return {"error": str(e)}


def call_vllm(system_prompt: str, user_query: str) -> str:
    payload = {
        "model": VLLM_MODEL,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_query},
        ],
        "temperature": 0.3,
        "max_tokens": 1024,
    }
    req = urllib.request.Request(
        VLLM_URL,
        data=json.dumps(payload).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {VLLM_API_KEY}",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=90) as res:
            data = json.loads(res.read().decode("utf-8"))
            return data["choices"][0]["message"]["content"]
    except Exception as e:
        return f"[Ошибка подключения к языковой модели: {e}]"


# ── Data extraction ────────────────────────────────────────────────────────
def extract_body_and_citation(payload: str) -> tuple[str, str]:
    parts = payload.split("\n\n", 1)
    metadata = parts[0]
    body = parts[1] if len(parts) > 1 else payload
    citation = ""
    for line in metadata.split("\n"):
        if line.startswith("source="):
            citation = line[7:]
            break
    return body.strip(), citation


def build_context(cells: list[dict]) -> tuple[str, list[dict]]:
    parts = []
    citations = []
    for cell in cells:
        # /v1/aql returns "payload", /v1/context returns "payload_text"
        payload = cell.get("payload_text") or cell.get("payload", "")
        cell_id = cell.get("cell_id", 0)
        body, citation = extract_body_and_citation(payload)
        if not body:
            continue
        parts.append(f"[Источник: {citation}]\n{body}")
        citations.append({"cell_id": cell_id, "citation": citation, "body": body})
    context = "\n\n---\n\n".join(parts) if parts else "Нет релевантных документов."
    return context, citations


# ── FastAPI endpoints ──────────────────────────────────────────────────────
@app.get("/", response_class=HTMLResponse)
async def index():
    return CHAT_HTML


@app.post("/api/chat")
async def chat(payload: dict):
    query = payload.get("query", "").strip()
    domain = payload.get("domain", "finance")

    if not query:
        return {"response": "Пустой запрос.", "citations": [], "conflicts": []}

    safe = query.replace('"', '\\"')

    # 1. Retrieve all relevant cells via AQL (no truncation / redundancy filter)
    aql = f'RETRIEVE CONTEXT FOR TASK "{safe}" IN BRAIN default LIMIT 20 CANDIDATES;'
    aql_result = query_cortex("/v1/aql", domain, aql)
    raw_cells = aql_result.get("cells", []) if isinstance(aql_result, dict) else []

    # 2. Verify fact
    verify_aql = f'VERIFY FACT "{safe}" IN BRAIN default;'
    verification = query_cortex("/v1/verify", domain, verify_aql)
    conflicts = []
    if verification.get("verdict") == "mixed_evidence":
        for c in verification.get("numeric_conflicts", []):
            conflicts.append({"metric": c.get("metric", ""), "left": c.get("left", ""), "right": c.get("right", "")})

    # 3. Build clean context
    context, citations = build_context(raw_cells)

    # 4. Build system prompt
    system_prompt = (
        "Ты — профессиональный корпоративный ассистент компании.\n"
        "Отвечай строго на основе предоставленных документов из внутренней базы знаний.\n"
        "Если в документах есть числовые данные — приводи их точно.\n"
        "Всегда указывай источник (в квадратных скобках) при упоминании фактов.\n"
        "Если информации нет в документах — честно скажи об этом.\n\n"
        f"Документы из базы знаний:\n\n{context}"
    )

    # 5. Generate response
    ai_response = call_vllm(system_prompt, query)

    return {
        "response": ai_response,
        "citations": citations,
        "conflicts": conflicts,
        "meta": {
            "cells_found": len(citations),
            "domain": domain,
            "verdict": verification.get("verdict", "unknown"),
        },
    }


@app.post("/api/verify")
async def verify_fact(payload: dict):
    statement = payload.get("statement", "").strip()
    domain = payload.get("domain", "finance")
    if not statement:
        return {"verdict": "error", "message": "Пустое утверждение."}
    safe = statement.replace('"', '\\"')
    verify_aql = f'VERIFY FACT "{safe}" IN BRAIN default;'
    verification = query_cortex("/v1/verify", domain, verify_aql)
    return {
        "verdict": verification.get("verdict", "unknown"),
        "numeric_conflicts": verification.get("numeric_conflicts", []),
        "domain": domain,
    }


# ── HTML UI ────────────────────────────────────────────────────────────────
CHAT_HTML = """<!DOCTYPE html>
<html lang="ru">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>CortexDB RAG — Корпоративный ИИ-ассистент</title>
<style>
  :root {
    --bg: #0b0f19;
    --panel: #111827;
    --ink: #e5e7eb;
    --muted: #9ca3af;
    --line: #1f2937;
    --accent: #38bdf8;
    --accent2: #22c55e;
    --warn: #f59e0b;
    --err: #ef4444;
  }
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    background: var(--bg);
    color: var(--ink);
    font: 15px/1.6 system-ui, -apple-system, sans-serif;
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  header {
    border-bottom: 1px solid var(--line);
    padding: 14px 24px;
    display: flex;
    align-items: center;
    gap: 14px;
    background: var(--panel);
  }
  header h1 { font-size: 1.05rem; font-weight: 700; }
  .badge {
    font-size: .68rem;
    padding: 3px 10px;
    border-radius: 999px;
    font-weight: 700;
    background: var(--accent);
    color: #000;
  }
  .main { flex: 1; display: flex; overflow: hidden; }
  .sidebar {
    width: 300px;
    border-right: 1px solid var(--line);
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 18px;
    overflow-y: auto;
    background: var(--panel);
  }
  .sidebar label {
    font-size: .7rem;
    text-transform: uppercase;
    color: var(--muted);
    font-weight: 700;
    letter-spacing: .04em;
  }
  .sidebar select, .sidebar button {
    width: 100%;
    padding: 10px 12px;
    border-radius: 8px;
    border: 1px solid var(--line);
    background: var(--bg);
    color: var(--ink);
    font: inherit;
    cursor: pointer;
  }
  .sidebar button {
    background: var(--accent);
    color: #000;
    border: none;
    font-weight: 700;
    font-size: .85rem;
  }
  .sidebar button:hover { opacity: .9; }
  .sidebar .hint {
    font-size: .75rem;
    color: var(--muted);
    line-height: 1.5;
  }
  .chat { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 28px 32px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .msg { max-width: 82%; padding: 14px 18px; border-radius: 14px; line-height: 1.6; }
  .msg.user { align-self: flex-end; background: var(--accent); color: #000; }
  .msg.assistant { align-self: flex-start; background: var(--panel); border: 1px solid var(--line); }
  .msg .source {
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--line);
    font-size: .78rem;
    color: var(--muted);
  }
  .msg .source strong { color: var(--accent); }
  .msg .conflict {
    margin-top: 12px;
    padding: 10px 14px;
    background: rgba(239, 68, 68, .08);
    border: 1px solid var(--err);
    border-radius: 8px;
    font-size: .82rem;
    color: #fca5a5;
  }
  .msg .meta {
    margin-top: 8px;
    font-size: .7rem;
    color: var(--muted);
  }
  .inputbar {
    border-top: 1px solid var(--line);
    padding: 16px 32px;
    display: flex;
    gap: 12px;
    background: var(--panel);
  }
  .inputbar input {
    flex: 1;
    padding: 12px 16px;
    border-radius: 10px;
    border: 1px solid var(--line);
    background: var(--bg);
    color: var(--ink);
    font: inherit;
    font-size: .95rem;
  }
  .inputbar input:focus { outline: 2px solid var(--accent); }
  .inputbar button {
    padding: 12px 24px;
    border-radius: 10px;
    border: none;
    background: var(--accent);
    color: #000;
    font-weight: 700;
    cursor: pointer;
  }
  .empty {
    color: var(--muted);
    text-align: center;
    margin-top: 60px;
    font-size: .95rem;
  }
  .loading { color: var(--accent); font-style: italic; }
  code {
    background: rgba(56, 189, 248, .1);
    padding: 2px 6px;
    border-radius: 4px;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: .85em;
  }
</style>
</head>
<body>
<header>
  <h1>🧠 CortexDB RAG</h1>
  <span class="badge">Gemma-4 31B</span>
  <span class="badge" style="background:var(--accent2)">vLLM</span>
</header>
<div class="main">
  <aside class="sidebar">
    <label>Область знаний</label>
    <select id="domain">
      <option value="finance">💰 Финансы</option>
      <option value="legal">⚖️ Юриспруденция</option>
      <option value="hr">👥 Кадры</option>
    </select>

    <label>Примеры вопросов</label>
    <button onclick="setQuery('Какой бюджет у проекта Солнечная электростанция Алматы?')">
      Бюджет СЭС Алматы
    </button>
    <button onclick="setQuery('Кто уполномочен подписывать договоры от ТОО АльянсСтрой?')">
      Подписанты АльянсСтрой
    </button>
    <button onclick="setQuery('Какова ставка налога на прибыль в Казахстане?')">
      Налог на прибыль
    </button>
    <button onclick="setQuery('Какой отпуск положен по Трудовому кодексу РК?')">
      Отпуск по ТК
    </button>
    <button onclick="setQuery('Результат дела Семёнова против ТОО АльянсСтрой')">
      Суд: Семёнов vs АльянсСтрой
    </button>
    <button onclick="setQuery('Какие обучения прошла Жумагулова Айгуль?')">
      Обучения Жумагуловой
    </button>

    <div class="hint">
      Система ищет релевантные ячейки в CortexDB через AQL,
      извлекает чистый текст и отправляет в <strong>Gemma-4</strong>
      с указанием источников. Числовые конфликты подсвечиваются автоматически.
    </div>
  </aside>

  <div class="chat">
    <div class="messages" id="messages">
      <div class="empty">
        Задайте вопрос по корпоративной базе знаний.<br>
        ИИ проанализирует документы и даст ответ с указанием источников.
      </div>
    </div>
    <div class="inputbar">
      <input type="text" id="queryInput" placeholder="Введите вопрос..."
             onkeydown="if(event.key==='Enter') send()">
      <button onclick="send()">Отправить</button>
    </div>
  </div>
</div>

<script>
function setQuery(q) {
  document.getElementById('queryInput').value = q;
  document.getElementById('queryInput').focus();
}
function escapeHtml(t) {
  return t.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
}
function appendMsg(cls, html, meta) {
  const box = document.getElementById('messages');
  if (box.querySelector('.empty')) box.innerHTML = '';
  const d = document.createElement('div');
  d.className = 'msg ' + cls;
  d.innerHTML = html + (meta ? '<div class="meta">' + meta + '</div>' : '');
  box.appendChild(d);
  box.scrollTop = box.scrollHeight;
}
async function send() {
  const input = document.getElementById('queryInput');
  const q = input.value.trim();
  if (!q) return;
  input.value = '';
  const domain = document.getElementById('domain').value;

  appendMsg('user', escapeHtml(q));

  const loadId = 'load-' + Date.now();
  appendMsg('assistant',
    '<span class="loading">⚙️ CortexDB извлекает контекст → Gemma-4 генерирует ответ...</span>',
    '', loadId);

  try {
    const res = await fetch('/api/chat', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ query: q, domain })
    });
    const data = await res.json();

    const box = document.getElementById('messages');
    const load = box.querySelector('#' + loadId);
    if (load) load.remove();

    let html = escapeHtml(data.response);

    if (data.citations && data.citations.length) {
      html += '<div class="source"><strong>📎 Источники:</strong><br>' +
        data.citations.map(c =>
          '<code>[' + c.cell_id + ']</code> ' + escapeHtml(c.citation)
        ).join('<br>') + '</div>';
    }

    if (data.conflicts && data.conflicts.length) {
      data.conflicts.forEach(conf => {
        html += '<div class="conflict">' +
          '⚠️ <strong>Конфликт данных:</strong> метрика «' + escapeHtml(conf.metric) + '» — ' +
          '«' + escapeHtml(conf.left) + '» против «' + escapeHtml(conf.right) + '»' +
          '</div>';
      });
    }

    const meta = 'Ячеек: ' + (data.meta?.cells_found || 0) +
                 ' | Домен: ' + domain +
                 ' | Верификация: ' + (data.meta?.verdict || '-');
    appendMsg('assistant', html, meta);

  } catch (e) {
    const box = document.getElementById('messages');
    const load = box.querySelector('#' + loadId);
    if (load) load.remove();
    appendMsg('assistant',
      '<span style="color:var(--err)">❌ Ошибка: ' + escapeHtml(e.message) + '</span>');
  }
}
</script>
</body>
</html>
"""
