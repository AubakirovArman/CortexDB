"""Chat clients for EnterpriseRAG-Bench answer generation."""

from __future__ import annotations

import http.client
import json
import os
import time
import urllib.error
import urllib.parse
import urllib.request
from typing import Any


def chat(
    *,
    api_key: str,
    base_url: str,
    model: str,
    prompt: str,
    max_tokens: int,
    retries: int,
    omit_thinking_field: bool,
    gemini_native: bool,
    gemini_thinking_budget: int,
    openai_reasoning: bool = False,
) -> tuple[str, dict[str, Any], int]:
    if gemini_native:
        return chat_gemini_native(
            api_key=api_key,
            base_url=base_url,
            model=model,
            prompt=prompt,
            max_tokens=max_tokens,
            retries=retries,
            thinking_budget=gemini_thinking_budget,
        )
    if openai_reasoning:
        # GPT-5 reasoning models: max_completion_tokens (not max_tokens), no
        # custom temperature; reasoning disabled for a fair, cheap extractive
        # answer comparable to a plain chat model. Leave headroom for the answer.
        payload: dict[str, Any] = {
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "max_completion_tokens": max(max_tokens * 3, 2000),
            "reasoning_effort": "none",
        }
    else:
        payload = {
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": max_tokens,
            "temperature": 0,
        }
        if not omit_thinking_field:
            if os.environ.get("DEEPSEEK_THINKING") == "enabled":
                payload["thinking"] = {"type": "enabled"}
                payload["reasoning_effort"] = "high"
                # DeepSeek's thinking tokens count against max_tokens; leave
                # enough headroom for both the reasoning trace and the answer.
                payload["max_tokens"] = max(payload.get("max_tokens", max_tokens), 8192)
            else:
                payload["thinking"] = {"type": "disabled"}
    data = json.dumps(payload).encode("utf-8")
    url = base_url.rstrip("/") + "/chat/completions"
    for attempt in range(retries + 1):
        request = urllib.request.Request(
            url,
            data=data,
            headers={
                "Authorization": "Bearer " + api_key,
                "Content-Type": "application/json",
            },
        )
        try:
            started = time.perf_counter()
            with urllib.request.urlopen(request, timeout=180) as response:
                body = json.loads(response.read().decode("utf-8"))
            elapsed_ms = int((time.perf_counter() - started) * 1000)
            answer = body["choices"][0]["message"].get("content", "")
            return (str(answer).strip(), body.get("usage", {}), elapsed_ms)
        except urllib.error.HTTPError as error:
            if error.code not in {429, 500, 502, 503, 504} or attempt >= retries:
                detail = error.read().decode("utf-8", errors="replace")[:500]
                raise RuntimeError(f"chat request failed: http={error.code} {detail}") from error
            time.sleep(min(30, 2**attempt))
        except (
            TimeoutError,
            urllib.error.URLError,
            http.client.IncompleteRead,
            http.client.RemoteDisconnected,
            json.JSONDecodeError,
        ) as error:
            if attempt >= retries:
                raise RuntimeError(f"chat request failed: {error}") from error
            time.sleep(min(30, 2**attempt))
    raise RuntimeError("unreachable retry state")


def chat_gemini_native(
    *,
    api_key: str,
    base_url: str,
    model: str,
    prompt: str,
    max_tokens: int,
    retries: int,
    thinking_budget: int,
) -> tuple[str, dict[str, Any], int]:
    model_name = model.removeprefix("models/")
    payload = {
        "contents": [{"role": "user", "parts": [{"text": prompt}]}],
        "generationConfig": {
            "temperature": 0,
            "maxOutputTokens": max_tokens,
            "thinkingConfig": {"thinkingBudget": thinking_budget},
        },
    }
    data = json.dumps(payload).encode("utf-8")
    url = (
        base_url.rstrip("/")
        + f"/models/{urllib.parse.quote(model_name, safe='')}:generateContent"
        + "?key="
        + urllib.parse.quote(api_key)
    )
    for attempt in range(retries + 1):
        request = urllib.request.Request(url, data=data, headers={"Content-Type": "application/json"})
        try:
            started = time.perf_counter()
            with urllib.request.urlopen(request, timeout=180) as response:
                body = json.loads(response.read().decode("utf-8"))
            elapsed_ms = int((time.perf_counter() - started) * 1000)
            answer = "".join(
                str(part.get("text", ""))
                for candidate in body.get("candidates", [])
                for part in candidate.get("content", {}).get("parts", [])
            )
            usage = body.get("usageMetadata", {})
            normalized_usage = {
                "prompt_tokens": int(usage.get("promptTokenCount", 0) or 0),
                "completion_tokens": int(usage.get("candidatesTokenCount", 0) or 0),
                "total_tokens": int(usage.get("totalTokenCount", 0) or 0),
                "thoughts_tokens": int(usage.get("thoughtsTokenCount", 0) or 0),
            }
            return (answer.strip(), normalized_usage, elapsed_ms)
        except urllib.error.HTTPError as error:
            if error.code not in {429, 500, 502, 503, 504} or attempt >= retries:
                detail = error.read().decode("utf-8", errors="replace")[:500]
                raise RuntimeError(f"gemini request failed: http={error.code} {detail}") from error
            time.sleep(min(30, 2**attempt))
        except (
            TimeoutError,
            urllib.error.URLError,
            http.client.IncompleteRead,
            http.client.RemoteDisconnected,
            json.JSONDecodeError,
        ) as error:
            if attempt >= retries:
                raise RuntimeError(f"gemini request failed: {error}") from error
            time.sleep(min(30, 2**attempt))
    raise RuntimeError("unreachable retry state")
