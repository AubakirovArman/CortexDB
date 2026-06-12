from __future__ import annotations

QUERY_EXPANSIONS = {
    "blocked": ["blocker", "risk", "delayed", "dependency", "waiting"],
    "owner": ["assignee", "responsible", "lead", "dri", "reviewer"],
    "policy": ["standard", "requirement", "guideline", "procedure", "control"],
    "rollback": ["revert", "fallback", "restore", "recovery"],
    "latency": ["p95", "p99", "tail", "ms", "slow"],
    "cost": ["price", "billing", "invoice", "credits"],
    "security": ["compliance", "audit", "rbac", "auth", "kms"],
    "capacity": ["quota", "limit", "gpu", "pool", "burst"],
    "route": ["routing", "router", "policy", "traffic"],
    "support": ["ticket", "case", "escalation", "customer"],
    "deployment": ["rollout", "release", "upgrade", "canary"],
    "complete": ["all", "list", "every", "coverage"],
    "observability": ["telemetry", "metrics", "tracing", "trace", "logs", "jsonl"],
    "tracking": ["telemetry", "metrics", "trace", "tracing", "logging"],
    "invocation": ["function", "call", "function-call", "tool", "tool-calling"],
    "tool": ["function", "call", "function-call", "invocation"],
    "staged": ["rollout", "schedule", "phase", "phased", "canary"],
    "schedule": ["rollout", "timeline", "phase", "phased"],
    "fallback": ["failover", "demotion", "route", "routing", "backup"],
    "locked": ["pinned", "sticky", "fixed"],
    "model": ["variant", "version"],
    "scaler": ["autoscaler", "autoscale", "keda", "hpa"],
}

FIELD_WEIGHTS = {
    "title_view": 4.0,
    "path_view": 3.5,
    "source_metadata_view": 2.5,
    "entity_view": 3.2,
    "summary_view": 2.8,
    "body_view": 1.0,
    "chunk_views": 1.7,
}

DIVERSITY_TYPES = {
    "completeness",
    "conflicting_info",
    "project_related",
    "semantic",
    "high_level",
}
