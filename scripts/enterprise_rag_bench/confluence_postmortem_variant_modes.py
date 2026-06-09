"""Route table for Confluence postmortem variant source selection."""

MODE_CONFIG = {
    "followup_action_items": {
        "contains": ("incident postmortems", "follow-up action items"),
        "max_docs": 2,
        "terms": "p0 postmortem follow-up action items count these owning team",
        "type": {"completeness"},
    },
    "h1_gpu_quota_incidents": {
        "contains": ("gpu", "quota", "h1 2025"),
        "max_docs": 3,
        "terms": "gpu capacity quota exhaustion h1 2025 incident postmortem private hosted autoscaler fragmentation",
        "type": {"completeness"},
    },
    "fallback_activation_writeups": {
        "contains": ("incident writeups", "fallback"),
        "max_docs": 5,
        "terms": "automatic fallback activated mitigation model region fallback internal incident writeup",
        "type": {"completeness"},
    },
}
