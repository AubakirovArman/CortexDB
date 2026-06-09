"""Route table for Confluence project-related source discovery."""

MODE_CONFIG = {
    "fast_tier_canary_slo_dashboard": {
        "allowed_paths": (
            "confluence/eng-sre/slo-and-error-budgets/",
            "confluence/eng-serving-runtime/kernel-and-scheduling/",
            "confluence/eng-platform/dashboards-and-alerts/",
        ),
        "contains": (
            "fast tier p99",
            "canary",
            "slo targets",
            "abort criteria",
            "rollback steps",
            "dashboards",
            "admission control",
            "requeue behavior",
        ),
        "max_docs": 2,
        "path_bonus": (
            "latency-slo-tiers-definition-fast-standard-cost-efficient",
            "batching-defaults-observability-dashboard-spec",
        ),
        "terms": (
            "fast tier p99 canary regression slo targets abort criteria rollback "
            "dashboards reason codes admission control requeue behavior latency slo "
            "tiered batching observability dashboard alert spec fast standard cost-efficient"
        ),
        "type": {"project_related"},
    },
    "rollout_split_orchestrator": {
        "allowed_paths": (
            "confluence/eng-platform/systems-and-services/",
            "confluence/architecture-and-standards/decision-records/",
            "confluence/oncall-and-incident-response/runbooks/",
        ),
        "contains": (
            "observed canary traffic percentage",
            "single region",
            "console rollout step",
            "recommended oncall mitigations",
            "ga preventative fixes",
        ),
        "max_docs": 1,
        "path_bonus": ("rollout-orchestrator-service",),
        "terms": (
            "rollout orchestrator service hosted canary rollouts traffic splitting "
            "console rollout step observed percentage single region divergence "
            "oncall mitigations ga preventative fixes cohorting state machine"
        ),
        "type": {"project_related"},
    },
}
