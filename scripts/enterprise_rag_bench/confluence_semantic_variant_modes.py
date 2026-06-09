"""Route table for Confluence semantic variant selection."""

MODE_CONFIG = {
    "restoration_sprint_sequence": {
        "contains": ("major production outage", "first 60 to 90 minutes"),
        "max_docs": 1,
        "path_bonus": ("ten-step-restoration-sprint", "shadow-mentoring"),
        "terms": "ten step restoration sprint first 60 90 minutes bridge traffic stabilize service healthy",
        "type": {"semantic"},
    },
    "status_wire_cadence": {
        "contains": ("cross team outage process", "shared incident channel", "containment"),
        "max_docs": 1,
        "path_bonus": ("tribe-liaison", "status-wire"),
        "terms": "status wire tribe liaison shared incident channel update rhythm containment interval shortened",
        "type": {"semantic"},
    },
    "employee_lifecycle_turnaround": {
        "contains": ("new starts", "accounts and tools", "north america versus europe"),
        "max_docs": 1,
        "path_bonus": ("employee-lifecycle", "operating-manual"),
        "terms": "employee lifecycle onboarding accounts tools north america europe asia pacific procurement sla seven fourteen business days",
        "type": {"semantic"},
    },
    "operational_policy_gallery": {
        "contains": ("one stop reference page", "approvals and slas", "standard request templates"),
        "max_docs": 1,
        "path_bonus": ("operational-flows", "policy-gallery"),
        "terms": "operational flows policy gallery approvals slas access production changes data retention purchasing vendor travel templates",
        "type": {"semantic"},
    },
    "offer_midpoint_approval": {
        "contains": ("hiring workflow", "20 percent above", "approve"),
        "max_docs": 1,
        "path_bonus": ("evidence-driven-offer", "onboarding-trigger"),
        "terms": "offer evaluation pay above band midpoint twenty percent hrbp hiring manager finance legal approval threshold",
        "type": {"semantic"},
    },
    "ops_orchestra_routine_approvals": {
        "contains": ("routine internal approvals", "operational requests"),
        "max_docs": 1,
        "path_bonus": ("ops-orchestra", "procedures"),
        "terms": "ops orchestra target sla routine approvals operational requests 72 business hours access change procurement vendor travel",
        "type": {"semantic"},
    },
}
