import json
import os
import urllib.request
import urllib.parse
from fastapi import FastAPI
from fastapi.responses import HTMLResponse

app = FastAPI()

VLLM_URL = os.getenv("VLLM_URL", "http://127.0.0.1:8000/v1/chat/completions")
VLLM_API_KEY = os.getenv("VLLM_API_KEY", "5zxyqINY37FEicJ_rMfpacCBxhcjJhE0wcSTi4ADgus")
VLLM_MODEL = os.getenv("VLLM_MODEL", "google/gemma-4-31B-it")
CORTEX_HOST = os.getenv("CORTEX_HOST", "http://127.0.0.1:8090")

DOMAINS = {
    "finance": "finance",
    "legal": "legal",
    "hr": "hr",
}


def query_cortex(endpoint: str, method: str, scope: str, data: str) -> dict:
    url = f"{CORTEX_HOST}{endpoint}?scope={urllib.parse.quote(scope)}"
    req = urllib.request.Request(
        url,
        data=data.encode("utf-8"),
        headers={"Content-Type": "text/plain"},
        method=method,
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
        with urllib.request.urlopen(req, timeout=60) as res:
            data = json.loads(res.read().decode("utf-8"))
            return data["choices"][0]["message"]["content"]
    except Exception as e:
        return f"[Ошибка подключения к LLM: {e}]"


@app.get("/", response_class=HTMLResponse)
async def index():
    return INDEX_HTML


@app.post("/api/chat")
async def chat(payload: dict):
    query = payload.get("query", "")
    domain = payload.get("domain", "finance")
    scope = DOMAINS.get(domain, "finance")

    # 1. Retrieve context
    aql = f'RETRIEVE CONTEXT FOR TASK "{query.replace(chr(34), chr(92)+chr(34))}" IN BRAIN default WHERE space = {scope} LIMIT 10 CANDIDATES;'
    context = query_cortex("/v1/context", "POST", scope, aql)

    # 2. Verify fact
    verify_aql = f'VERIFY FACT "{query.replace(chr(34), chr(92)+chr(34))}" IN BRAIN default;'
    verification = query_cortex("/v1/verify", "POST", scope, verify_aql)

    # 3. Build context string
    cells = context.get("cells", []) if isinstance(context, dict) else []
    context_str = ""
    citations = []
    for cell in cells:
        text = cell.get("payload_text", "")
        citation = cell.get("citation", "")
        if text:
            context_str += f"Источник: {citation}\n{text}\n\n"
            citations.append({"cell_id": cell.get("cell_id"), "citation": citation, "text": text})

    if not context_str:
        context_str = "Релевантные документы в базе данных не найдены."

    # 4. Build system prompt
    system_prompt = (
        "Ты — профессиональный корпоративный ассистент. Отвечай строго на основе предоставленных документов из CortexDB. "
        "Если в документах есть числовые данные — приводи их точно. Всегда указывай источник (source/citation) при упоминании фактов.\n\n"
        f"Документы из базы знаний:\n{context_str}"
    )

    # 5. Call LLM
    ai_response = call_vllm(system_prompt, query)

    return {
        "response": ai_response,
        "citations": citations,
        "verification": verification,
        "context_meta": {
            "cells_found": len(cells),
            "domain": domain,
        },
    }


INDEX_HTML = """<!DOCTYPE html>
<html lang="ru">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>CortexDB RAG — Корпоративный ИИ-ассистент</title>
<style>
  :root { --bg:#0f172a; --panel:#1e293b; --ink:#e2e8f0; --muted:#94a3b8; --line:#334155; --accent:#38bdf8; --accent2:#22c55e; --warn:#f59e0b; --err:#ef4444; }
  * { box-sizing: border-box; margin:0; padding:0; }
  body { background:var(--bg); color:var(--ink); font:16px/1.5 system-ui,-apple-system,sans-serif; height:100vh; display:flex; flex-direction:column; }
  header { border-bottom:1px solid var(--line); padding:1rem 1.5rem; display:flex; align-items:center; gap:1rem; }
  header h1 { font-size:1.1rem; }
  header .badge { font-size:.7rem; background:var(--accent); color:#000; padding:.15rem .5rem; border-radius:999px; font-weight:700; }
  .main { flex:1; display:flex; overflow:hidden; }
  .sidebar { width:280px; border-right:1px solid var(--line); padding:1rem; display:flex; flex-direction:column; gap:1rem; overflow-y:auto; }
  .sidebar label { font-size:.75rem; text-transform:uppercase; color:var(--muted); font-weight:700; }
  .sidebar select, .sidebar button { width:100%; padding:.5rem .75rem; border-radius:6px; border:1px solid var(--line); background:var(--panel); color:var(--ink); font:inherit; cursor:pointer; }
  .sidebar button { background:var(--accent); color:#000; border:none; font-weight:700; }
  .sidebar button:hover { opacity:.9; }
  .chat { flex:1; display:flex; flex-direction:column; }
  .messages { flex:1; overflow-y:auto; padding:1.5rem; display:flex; flex-direction:column; gap:1rem; }
  .msg { max-width:80%; padding:.875rem 1rem; border-radius:12px; line-height:1.55; }
  .msg.user { align-self:flex-end; background:var(--accent); color:#000; }
  .msg.assistant { align-self:flex-start; background:var(--panel); border:1px solid var(--line); }
  .msg .source { margin-top:.5rem; padding-top:.5rem; border-top:1px solid var(--line); font-size:.75rem; color:var(--muted); }
  .msg .conflict { margin-top:.5rem; padding:.5rem .75rem; background:rgba(239,68,68,.1); border:1px solid var(--err); border-radius:6px; font-size:.8rem; color:#fca5a5; }
  .inputbar { border-top:1px solid var(--line); padding:1rem 1.5rem; display:flex; gap:.75rem; }
  .inputbar input { flex:1; padding:.75rem 1rem; border-radius:8px; border:1px solid var(--line); background:var(--panel); color:var(--ink); font:inherit; }
  .inputbar input:focus { outline:2px solid var(--accent); }
  .inputbar button { padding:.75rem 1.5rem; border-radius:8px; border:none; background:var(--accent); color:#000; font-weight:700; cursor:pointer; }
  .empty { color:var(--muted); text-align:center; margin-top:3rem; }
  .loading { color:var(--accent); font-style:italic; }
  .meta { font-size:.7rem; color:var(--muted); margin-top:.25rem; }
</style>
</head>
<body>
<header>
  <h1>🧠 CortexDB RAG</h1>
  <span class="badge">Gemma-4 31B</span>
  <span class="badge" style="background:var(--accent2);">vLLM</span>
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
    <button onclick="setQuery('Какой бюджет у проекта Солнечная электростанция Алматы?')">Бюджет СЭС Алматы</button>
    <button onclick="setQuery('Кто уполномочен подписывать договоры от ТОО АльянсСтрой?')">Подписанты АльянсСтрой</button>
    <button onclick="setQuery('Какова ставка налога на прибыль в Казахстане?')">Налог на прибыль</button>
    <button onclick="setQuery('Какой отпуск положен по Трудовому кодексу?')">Отпуск по ТК</button>
    <button onclick="setQuery('Кто финансовый директор и какой у него бюджет?')">Финансовый директор</button>
    <button onclick="setQuery('В чём был судебный спор между Семёновым и АльянсСтроем?')">Судебный спор Семёнов</button>
  </aside>
  <div class="chat">
    <div class="messages" id="messages">
      <div class="empty">Задайте вопрос по корпоративной базе знаний. ИИ проанализирует документы и даст ответ с указанием источников.</div>
    </div>
    <div class="inputbar">
      <input type="text" id="queryInput" placeholder="Введите вопрос..." onkeydown="if(event.key==='Enter') send()">
      <button onclick="send()">Отправить</button>
    </div>
  </div>
</div>
<script>
function setQuery(q) { document.getElementById('queryInput').value = q; document.getElementById('queryInput').focus(); }
function escapeHtml(t) { return t.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;'); }
function appendMsg(cls, html, meta='') {
  const box = document.getElementById('messages');
  if(box.querySelector('.empty')) box.innerHTML='';
  const d=document.createElement('div'); d.className='msg '+cls; d.innerHTML=html + (meta?'<div class="meta">'+meta+'</div>':'');
  box.appendChild(d); box.scrollTop=box.scrollHeight;
}
async function send() {
  const input=document.getElementById('queryInput'); const q=input.value.trim(); if(!q) return;
  input.value=''; const domain=document.getElementById('domain').value;
  appendMsg('user', escapeHtml(q));
  const loadId='load-'+Date.now();
  appendMsg('assistant', '<span class="loading">⚙️ CortexDB извлекает контекст и Gemma-4 генерирует ответ...</span>', '', loadId);
  try {
    const res=await fetch('/api/chat',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({query:q,domain})});
    const data=await res.json();
    const box=document.getElementById('messages'); const load=box.querySelector('#'+loadId); if(load) load.remove();

    let html=escapeHtml(data.response);
    if(data.citations && data.citations.length) {
      html+='<div class="source"><strong>📎 Источники:</strong><br>'+data.citations.map(c=>'['+c.cell_id+'] '+escapeHtml(c.citation)).join('<br>')+'</div>';
    }
    if(data.verification && data.verification.verdict==='mixed_evidence' && data.verification.numeric_conflicts) {
      data.verification.numeric_conflicts.forEach(conf=>{
        html+='<div class="conflict">⚠️ <strong>Конфликт данных:</strong> метрика «'+escapeHtml(conf.metric)+'» — «'+escapeHtml(conf.left)+'» против «'+escapeHtml(conf.right)+'»</div>';
      });
    }
    const meta='Найдено ячеек: '+(data.context_meta?.cells_found||0)+' | Домен: '+domain;
    appendMsg('assistant', html, meta);
  } catch(e) {
    const box=document.getElementById('messages'); const load=box.querySelector('#'+loadId); if(load) load.remove();
    appendMsg('assistant', '<span style="color:var(--err)">❌ Ошибка: '+escapeHtml(e.message)+'</span>');
  }
}
</script>
</body>
</html>
"""
