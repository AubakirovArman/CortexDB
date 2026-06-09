"""Route table for SDK auth completeness source selection."""

MODE_CONFIG = {
    "sdk_auth_bug_reports": {
        "contains": ("go", "python", "typescript", "auth"),
        "jira_limit": 5,
        "github_limit": 3,
        "slack_limit": 2,
        "terms": (
            "redwood go python typescript sdk auth authorization api key api-key "
            "bearer 401 403 invalid_api_key customer reported bug report support "
            "ticket edge streaming trailing newline key rotation header guardrails"
        ),
        "type": {"completeness"},
    }
}
