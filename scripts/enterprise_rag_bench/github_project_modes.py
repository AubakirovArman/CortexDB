"""Route table for GitHub project-chain source selection."""

from __future__ import annotations

from typing import Any


def _cfg(
    contains: tuple[str, ...],
    markers: tuple[str, ...],
    terms: str,
    max_docs: int = 1,
) -> dict[str, Any]:
    return {
        "contains": contains,
        "markers": markers,
        "terms": terms,
        "max_docs": max_docs,
    }


MODE_CONFIG: dict[str, dict[str, Any]] = {
    "burst_retry_billing": _cfg(
        ("dedicated burst usage billed higher", "retry attempts"),
        ("pr-24108-fix-billing-double-counting-on-burst-retries",),
        "northpeak dedicated burst usage retry double counting idempotency reconciliation metering",
    ),
    "stream_retry_billing": _cfg(
        ("invoice token total", "usage api export"),
        ("pr-27463-fix-double-counting-on-stream-retry",),
        "streaming retries double counted invoice usage api export credit ledger dedupe key",
    ),
    "sdk_parity_conformance": _cfg(
        ("streaming timeout enforcement", "python, typescript, or go sdks"),
        ("pr-635-add-conformance-test-harness",),
        "sdk parity conformance harness retries timeouts streaming interruptions python typescript go",
    ),
    "audit_export_filters": _cfg(
        ("request log ttl", "audit log exports"),
        ("pr-612-add-time-windowed-exports-and-filters",),
        "audit log exporter time windowed exports actor event type filters compliance pack retention",
    ),
    "fast_tier_runtime_prs": _cfg(
        ("fast tier p99", "admission control", "requeue behavior"),
        (
            "pr-28458-fast-tier-admission-control-and-wait-budget",
            "pr-933-add-tiered-batching-dashboards",
        ),
        "fast tier p99 admission control wait budget requeue dashboards reason codes batching",
        max_docs=2,
    ),
}
