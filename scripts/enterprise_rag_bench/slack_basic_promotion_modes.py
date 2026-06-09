"""Route table for Slack basic top500-to-top10 promotion."""

MODE_CONFIG = {
    "cost_routing_telemetry": {
        "contains": ("cost-aware model routing", "telemetry fields"),
        "max_docs": 1,
        "path_bonus": ("cost-aware-routing", "cheap-variants"),
        "terms": (
            "cost aware routing telemetry fields route_decision estimated_token_cost "
            "chosen_variant quality_score fallback_trigger token_counts spend quality"
        ),
        "type": {"basic"},
    },
    "api_v2_deprecation_canary": {
        "contains": ("deprecating v2", "canary deployment", "traffic ramp"),
        "max_docs": 1,
        "path_bonus": ("api-v2-deprecation", "rollout-checkin"),
        "terms": (
            "api v2 deprecation rollout canary deployment us-west-2 start time "
            "traffic ramp one percent five percent three hours"
        ),
        "type": {"basic"},
    },
    "kv_contbatch_hotfix": {
        "contains": ("kv cache", "continuous batching regression", "us-west-2"),
        "max_docs": 1,
        "path_bonus": ("kv-wavefront", "triage-handoff"),
        "terms": (
            "kv cache continuous batching regression inference prod us-west-2 "
            "latency spike disable continuous batching max_batch_tokens 2048 "
            "batch_timeout_ms 5"
        ),
        "type": {"basic"},
    },
}
