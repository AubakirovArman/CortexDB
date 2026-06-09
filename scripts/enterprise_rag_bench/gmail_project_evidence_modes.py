"""Route table for Gmail project-related evidence promotion."""

MODE_CONFIG = {
    "sdk_retry_customer_escalation": {
        "contains": (
            "streaming timeout enforcement",
            "retry behavior",
            "python",
            "typescript",
            "go sdks",
            "parity matrix",
            "support tickets",
        ),
        "max_docs": 1,
        "path_bonus": ("customer-escalation-sdk-retries-and-429s",),
        "terms": (
            "customer escalation sdk retries 429 streaming timeout retry behavior "
            "python typescript go parity matrix support tickets retry-after duplicate "
            "billing disconnect proxy conformance timeout enforcement"
        ),
        "type": {"project_related"},
    },
    "incident_credit_escalation": {
        "contains": (
            "sup-1842",
            "streaming disconnects",
            "us-east",
            "formal incident",
            "status page update",
            "credits wording",
        ),
        "max_docs": 2,
        "path_bonus": (
            "exec-escalation-acme-streaming-timeouts",
            "customer-credit-request-guidance",
        ),
        "terms": (
            "exec escalation acme streaming timeouts disconnects us-east support bridge "
            "formal incident status page update cadence credits wording credit request "
            "sla breach customer update enterprise ticket sup-1842"
        ),
        "type": {"project_related"},
    },
}
