from __future__ import annotations

import json
import time
import urllib.error
import urllib.parse
import urllib.request
from typing import Any

from .errors import CortexDBError


def request_json(
    *,
    base_url: str,
    tenant: str | None,
    token: str | None,
    timeout_seconds: float,
    max_retries: int,
    retry_delay_seconds: float,
    opener: Any | None,
    method: str,
    path: str,
    body: bytes,
) -> dict[str, Any]:
    headers = {"content-type": "application/json"}
    if token:
        headers["authorization"] = f"Bearer {token}"
    url = f"{base_url}{scoped_path(path, tenant)}"
    attempt = 0
    while True:
        request = urllib.request.Request(url, data=body or None, headers=headers, method=method)
        try:
            with open_request(opener, request, timeout_seconds) as response:
                return json.loads(response.read().decode())
        except urllib.error.HTTPError as error:
            body_text = error.read().decode()
            if attempt < max_retries and is_retryable(error.code, body_text):
                attempt += 1
                sleep_before_retry(retry_delay_seconds, attempt)
                continue
            raise CortexDBError.from_response(error.code, body_text) from None
        except urllib.error.URLError as error:
            if attempt < max_retries:
                attempt += 1
                sleep_before_retry(retry_delay_seconds, attempt)
                continue
            reason = str(error.reason)
            raise CortexDBError(reason, code=None, status=None, body=reason) from None


def build_opener() -> Any:
    return urllib.request.build_opener()


def close_opener(opener: Any | None) -> None:
    if opener is None:
        return
    for handler in getattr(opener, "handlers", ()):
        close = getattr(handler, "close", None)
        if callable(close):
            close()


def open_request(opener: Any | None, request: urllib.request.Request, timeout_seconds: float) -> Any:
    if opener is not None:
        return opener.open(request, timeout=timeout_seconds)
    return urllib.request.urlopen(request, timeout=timeout_seconds)


def scoped_path(path: str, tenant: str | None) -> str:
    if not tenant or tenant == "default":
        return path
    separator = "&" if "?" in path else "?"
    encoded = urllib.parse.urlencode({"tenant": tenant})
    return f"{path}{separator}{encoded}"


def is_retryable(status: int, body_text: str) -> bool:
    if status in (502, 504):
        return True
    if status != 503:
        return False
    try:
        code = json.loads(body_text).get("code")
    except json.JSONDecodeError:
        return True
    return code in ("database_busy", "service_unavailable")


def sleep_before_retry(delay_seconds: float, attempt: int) -> None:
    if delay_seconds <= 0:
        return
    time.sleep(delay_seconds * attempt)
