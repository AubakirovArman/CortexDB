"""Route table for Jira semantic top100-to-top10 promotion."""

MODE_CONFIG = {
    "eu_apac_embedding_egress": {
        "contains": ("western europe", "southeast asia edge", "mid march 2026", "vectorization service"),
        "max_docs": 1,
        "path_bonus": ("eu-to-apac-egress", "embedding-burst"),
        "terms": (
            "western europe southeast asia apac eu-west egress flip vectorization embeddings "
            "mid march 2026 load spike 200ms ingestion timeouts residency stamp routing fallback"
        ),
        "type": {"semantic"},
    },
    "pci_audit_proof_package": {
        "contains": ("card-industry regulated bank", "3-day early-march 2026", "file-drop security tool"),
        "max_docs": 1,
        "path_bonus": ("per-request-sampling-proof", "siem-forwarder-integrity"),
        "terms": (
            "pci card industry regulated bank tamper evident proof package three day early march "
            "activity export file drop siem hsm signed manifest per request audit records lost reordered"
        ),
        "type": {"semantic"},
    },
    "contractor_employee_rekey_window": {
        "contains": ("rotating and rekeying", "contractor to full time transition"),
        "max_docs": 1,
        "path_bonus": ("contractor-to-employee", "device-hold"),
        "terms": (
            "contractor employee full time transition rotating rekeying secrets automation accounts "
            "service account keys kms rekey off hours 2026-03-24 02:00 04:00 utc"
        ),
        "type": {"semantic"},
    },
}
