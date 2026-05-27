# CortexDB RAG Demo — Корпоративный ИИ-ассистент

Полнофункциональное RAG-приложение (Retrieval-Augmented Generation) на русском языке с использованием:
- **CortexDB** (порт 8090) — векторная + ключевая база знаний
- **vLLM** (порт 8018) — языковая модель Google Gemma-4-31B-it
- **FastAPI** (порт 8085) — веб-интерфейс и REST API

---

## Архитектура

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Пользователь  │────▶│  FastAPI    │────▶│  CortexDB   │
│  (браузер/curl) │     │  (8085)     │     │  (8090)     │
└─────────────┘     └─────────────┘     └─────────────┘
                            │
                            ▼
                      ┌─────────────┐
                      │    vLLM     │
                      │   (8018)    │
                      └─────────────┘
```

### Поток запроса
1. Пользователь отправляет вопрос через Web UI или `/api/chat`
2. FastAPI строит AQL-запрос: `RETRIEVE CONTEXT FOR TASK "..." IN BRAIN default LIMIT 20 CANDIDATES`
3. CortexDB возвращает релевантные ячейки из указанного домена
4. FastAPI извлекает `body` из payload (разделение по `\n\n`)
5. FastAPI отправляет системный промпт + контекст в vLLM
6. vLLM генерирует ответ с цитатами

---

## Быстрый старт

### Предварительные требования

```bash
# 1. Rust + Cargo (для сборки cortex-server)
# 2. Python 3.10+
# 3. vLLM с моделью Gemma-4 (уже запущен на порту 8018)
```

### Запуск

```bash
cd examples/rag_demo
./run.sh
```

Скрипт автоматически:
- Создаст виртуальное окружение Python
- Соберёт `cortex-server` из исходников
- Запустит CortexDB на порту 8090
- Загрузит 74 русскоязычные ячейки (finance, hr, legal)
- Проверит доступность vLLM
- Запустит FastAPI на порту 8085

### Ручной запуск (если нужно)

```bash
# 1. Собрать и запустить CortexDB
cargo build --bin cortex-server
./target/debug/cortex-server examples/rag_demo/cortex_db 127.0.0.1:8090 &

# 2. Загрузить данные
cd examples/rag_demo
python3 ingest.py

# 3. Запустить FastAPI
uvicorn app:app --host 127.0.0.1 --port 8085
```

---

## API Endpoints

### `POST /api/chat`

Основной endpoint для чат-диалога.

**Request:**
```json
{
  "query": "Какой бюджет у Финансового департамента?",
  "domain": "finance"
}
```

**Response:**
```json
{
  "response": "Годовой бюджет Финансового департамента на 2024 год утверждён в размере 450 млн тенге [Источник: budget_approval_2024.xlsx].",
  "citations": [
    {
      "cell_id": 1,
      "citation": "budget_approval_2024.xlsx",
      "body": "Годовой бюджет Финансового департамента на 2024 год утверждён в размере 450 млн тенге."
    }
  ],
  "conflicts": [],
  "meta": {
    "cells_found": 12,
    "domain": "finance",
    "verdict": "consistent"
  }
}
```

### `POST /api/verify`

Проверка факта на противоречия в базе знаний.

**Request:**
```json
{
  "statement": "Бюджет Финансового департамента на 2024 год составляет 450 млн тенге.",
  "domain": "finance"
}
```

**Response:**
```json
{
  "verdict": "mixed_evidence",
  "numeric_conflicts": [...],
  "domain": "finance"
}
```

### `GET /`

Веб-интерфейс чат-бота (одностраничное приложение).

---

## Домены

| Домен   | Scope  | Описание                          | Ячеек |
|---------|--------|-----------------------------------|-------|
| finance | `finance` | Бюджеты, инвестиции, расходы, KPI | 28    |
| hr      | `hr`      | Сотрудники, должности, обучение   | 24    |
| legal   | `legal`   | Контракты, регламенты, суды       | 22    |

**Важно:** Scope должен точно совпадать с `scope` в ячейках. CortexDB использует хеширование — `finance` и `finance:budgets` — разные scope.

---

## Структура данных

Каждая ячейка хранится в формате:
```
scope=<домен>
status=ready
type=fact
source=<источник>

<тело документа>
```

- Метаданные и тело разделены пустой строкой (`\n\n`)
- Поле `source=` используется для цитирования в ответах LLM

---

## Тестирование

```bash
# Finance — бюджеты
curl -s -X POST http://127.0.0.1:8085/api/chat \
  -H "Content-Type: application/json" \
  -d '{"query":"Какой бюджет у Финансового департамента?","domain":"finance"}'

# HR — сотрудники
curl -s -X POST http://127.0.0.1:8085/api/chat \
  -H "Content-Type: application/json" \
  -d '{"query":"Кто генеральный директор?","domain":"hr"}'

# Legal — контракты
curl -s -X POST http://127.0.0.1:8085/api/chat \
  -H "Content-Type: application/json" \
  -d '{"query":"Какие требования к подписанту?","domain":"legal"}'

# Verify — проверка факта
curl -s -X POST http://127.0.0.1:8085/api/verify \
  -H "Content-Type: application/json" \
  -d '{"statement":"Бюджет ФД — 450 млн тенге.","domain":"finance"}'
```

---

## Файлы

| Файл          | Описание                                    |
|---------------|---------------------------------------------|
| `app.py`      | FastAPI приложение (chat, verify, UI)       |
| `ingest.py`   | Загрузка JSONL данных в CortexDB            |
| `run.sh`      | Полный скрипт запуска                       |
| `data/`       | 12 JSONL файлов с русскоязычными данными    |
| `requirements.txt` | Зависимости Python                     |

---

## Лицензия

MIT / Apache-2.0 — в рамках проекта CortexDB.
