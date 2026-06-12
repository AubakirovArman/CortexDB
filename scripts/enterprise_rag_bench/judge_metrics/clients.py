from __future__ import annotations

import http.client
import json
import time
import urllib.error
import urllib.parse
import urllib.request
from typing import Any

from judge_metrics.prompts import parse_judge_json


def chat_json(
    *,
    api_key: str,
    base_url: str,
    model: str,
    prompt: str,
    timeout: float,
    retries: int,
    omit_thinking_field: bool,
    gemini_native: bool,
    gemini_thinking_budget: int,
    openai_reasoning: bool = False,
) -> tuple[dict[str, Any], dict[str, Any], int]:
    if gemini_native:
        return chat_json_gemini_native(
            api_key=api_key,
            base_url=base_url,
            model=model,
            prompt=prompt,
            timeout=timeout,
            retries=retries,
            thinking_budget=gemini_thinking_budget,
        )
    payload: dict[str, Any]
    if openai_reasoning:
        payload = {
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "max_completion_tokens": 3000,
            "reasoning_effort": "none",
        }
    else:
        payload = {
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0,
            "max_tokens": 220,
        }
        if not omit_thinking_field:
            payload["thinking"] = {"type": "disabled"}
    return chat_json_openai_compatible(
        api_key=api_key,
        base_url=base_url,
        payload=payload,
        timeout=timeout,
        retries=retries,
    )


def chat_json_openai_compatible(
    *,
    api_key: str,
    base_url: str,
    payload: dict[str, Any],
    timeout: float,
    retries: int,
) -> tuple[dict[str, Any], dict[str, Any], int]:
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
            with urllib.request.urlopen(request, timeout=timeout) as response:
                body = json.loads(response.read().decode("utf-8"))
            elapsed_ms = int((time.perf_counter() - started) * 1000)
            content = body["choices"][0]["message"].get("content", "")
            return parse_judge_json(str(content)), body.get("usage", {}), elapsed_ms
        except urllib.error.HTTPError as error:
            if error.code not in {429, 500, 502, 503, 504} or attempt >= retries:
                detail = error.read().decode("utf-8", errors="replace")[:500]
                raise RuntimeError(f"deepseek judge HTTP {error.code}: {detail}") from error
            time.sleep(min(30, 2**attempt))
        except REQUEST_ERRORS as error:
            if attempt >= retries:
                raise RuntimeError(f"deepseek judge request failed: {error}") from error
            time.sleep(min(30, 2**attempt))
    raise RuntimeError("unreachable retry state")


def chat_json_gemini_native(
    *,
    api_key: str,
    base_url: str,
    model: str,
    prompt: str,
    timeout: float,
    retries: int,
    thinking_budget: int,
) -> tuple[dict[str, Any], dict[str, Any], int]:
    model_name = model.removeprefix("models/")
    payload = {
        "contents": [{"role": "user", "parts": [{"text": prompt}]}],
        "generationConfig": {
            "temperature": 0,
            "maxOutputTokens": 220,
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
        request = urllib.request.Request(
            url,
            data=data,
            headers={"Content-Type": "application/json"},
        )
        try:
            started = time.perf_counter()
            with urllib.request.urlopen(request, timeout=timeout) as response:
                body = json.loads(response.read().decode("utf-8"))
            elapsed_ms = int((time.perf_counter() - started) * 1000)
            content = "".join(
                str(part.get("text", ""))
                for candidate in body.get("candidates", [])
                for part in candidate.get("content", {}).get("parts", [])
            )
            return parse_judge_json(content), normalized_gemini_usage(body), elapsed_ms
        except urllib.error.HTTPError as error:
            if error.code not in {429, 500, 502, 503, 504} or attempt >= retries:
                detail = error.read().decode("utf-8", errors="replace")[:500]
                raise RuntimeError(f"gemini judge HTTP {error.code}: {detail}") from error
            time.sleep(min(30, 2**attempt))
        except REQUEST_ERRORS as error:
            if attempt >= retries:
                raise RuntimeError(f"gemini judge request failed: {error}") from error
            time.sleep(min(30, 2**attempt))
    raise RuntimeError("unreachable retry state")


def normalized_gemini_usage(body: dict[str, Any]) -> dict[str, int]:
    usage = body.get("usageMetadata", {})
    return {
        "prompt_tokens": int(usage.get("promptTokenCount", 0) or 0),
        "completion_tokens": int(usage.get("candidatesTokenCount", 0) or 0),
        "total_tokens": int(usage.get("totalTokenCount", 0) or 0),
        "thoughts_tokens": int(usage.get("thoughtsTokenCount", 0) or 0),
    }


REQUEST_ERRORS = (
    TimeoutError,
    urllib.error.URLError,
    http.client.IncompleteRead,
    http.client.RemoteDisconnected,
    json.JSONDecodeError,
)
