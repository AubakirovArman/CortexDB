"""Route table for Jira project-related evidence promotion."""

MODE_CONFIG = {
    "invoice_retry_double_count": {
        "contains": (
            "invoice token total",
            "usage api export",
            "streaming retries",
            "double-counted",
            "credit",
            "ledger",
        ),
        "max_docs": 1,
        "path_bonus": ("double-charged-for-retries", "streaming"),
        "terms": (
            "invoice usage api export streaming retry fallback double counted double charged "
            "tokens idempotency key billing ledger credit approval shadow billing compare "
            "customer remediation usage discrepancy billable=true"
        ),
        "type": {"project_related"},
    },
    "burst_noisy_neighbor_latency": {
        "contains": (
            "non-burst dedicated tenant",
            "tail latency spikes",
            "burst window",
            "slo gate",
            "circuit breaker",
        ),
        "max_docs": 1,
        "path_bonus": ("noisy-neighbor-latency", "burst-window"),
        "terms": (
            "non burst dedicated noisy neighbor tail latency burst window shared routing "
            "admission controller policy evaluation slo gate circuit breaker safety caps "
            "slo_gated fleet_safety_circuit_open"
        ),
        "type": {"project_related"},
    },
    "demo_dashboard_empty_metrics": {
        "contains": (
            "demo tenant",
            "429s",
            "console dashboards are empty",
            "fastest recovery",
            "prevent this",
        ),
        "max_docs": 1,
        "path_bonus": ("demo-dashboard-not-populating", "metrics"),
        "terms": (
            "demo tenant console dashboards empty graphs metrics synthetic traffic generator "
            "demo observability project last 15 minutes 429 healthcheck reset workaround "
            "dashboard freshness empty state"
        ),
        "type": {"project_related"},
    },
}
