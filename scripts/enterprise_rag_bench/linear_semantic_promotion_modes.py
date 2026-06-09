"""Route table for Linear semantic top500-to-top10 promotion."""

MODE_CONFIG = {
    "runtime_latency_isolation": {
        "contains": (
            "gpu inference runtime",
            "tiny chat requests",
            "long prompt processing",
            "worst case per token latency",
            "ten percent",
        ),
        "max_docs": 1,
        "path_bonus": ("batching-backpressure-latency-isolation", "continuous-prefill-decode"),
        "terms": (
            "continuous batching backpressure latency isolation tiny chat long prefill "
            "99.9th token latency latency sensitive routes 35 50 aggregate throughput "
            "under 10 mixed workload admission scoring batch shaping kernel selection"
        ),
        "type": {"semantic"},
    },
    "benchmark_store_comparison_canvas": {
        "contains": (
            "store and query model performance run histories",
            "simple ui to compare two runs",
            "groups repeated issues",
            "rolled up data",
        ),
        "max_docs": 1,
        "path_bonus": ("compact-benchmark-store", "comparison-canvas-triage"),
        "terms": (
            "compact benchmark result store comparison canvas triage playbook p95 tokens "
            "per sec fingerprint regression grouping high resolution traces 30 days "
            "aggregated rollups 1 year pairwise overlays delta histograms oncall workflow"
        ),
        "type": {"semantic"},
    },
    "slo_sentinel_prefetch_circuit_breakers": {
        "contains": (
            "multi-region traffic controller",
            "warming likely failover locations",
            "three-level degrade",
            "hard failures",
        ),
        "max_docs": 1,
        "path_bonus": ("slo-sentinel", "prefetch-routing", "graded-circuit-breakers"),
        "terms": (
            "slo sentinel prefetch routing graded circuit breakers multi region latency "
            "risk ewma derivative warm kv cache green yellow red soft shed hard failures "
            "capacity aware steering token escrow p99 blowups"
        ),
        "type": {"semantic"},
    },
}
