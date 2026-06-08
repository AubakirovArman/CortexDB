"""EnterpriseRAG neighbor keys for source-aware document expansion."""

from __future__ import annotations

import re
from typing import Any


SAFE_FIELD_NAMES = (
    "thread_id",
    "thread_ts",
    "repo",
    "project",
    "team",
    "space",
    "channel",
    "meeting_id",
    "pr_number",
    "company_id",
    "company_name",
    "customer_company",
    "related_account",
    "deal_id",
    "crm_deal_id",
    "crm_account_id",
    "key",
)

LINK_FIELD_NAMES = (
    "attachments",
    "dependencies",
    "linked_artifacts",
    "linked_drive_docs",
    "linked_fireflies",
    "linked_gmail_threads",
    "linked_github_prs",
    "linked_issues",
    "linked_jira",
    "linked_linear",
    "linked_support_tickets",
    "links",
    "parent_issue",
    "related_confluence_pages",
    "related_github_prs",
    "related_links",
    "related_pages",
)


def normalize_key(value: str) -> str:
    return re.sub(r"[^a-z0-9_./:%-]+", " ", value.lower()).strip()


def source_type(path: str) -> str:
    return path.split("/", 1)[0] if path else "unknown"


def string_values(value: Any) -> list[str]:
    if isinstance(value, str) and value.strip():
        return [value.strip()]
    if isinstance(value, list):
        return [str(item).strip() for item in value if str(item).strip()]
    return []


def normalize_path_ref(value: str) -> str:
    cleaned = value.strip().lower().lstrip("/")
    if cleaned.endswith(".json"):
        cleaned = cleaned[:-5]
    return re.sub(r"[^a-z0-9_./:%-]+", "-", cleaned).strip("-")


def path_ref_keys(rel_path: str) -> set[str]:
    if not rel_path:
        return set()
    normalized = normalize_path_ref(rel_path)
    keys = {normalized}
    if normalized.endswith(".json"):
        keys.add(normalized[:-5])
    return {key for key in keys if key}


def link_ref_keys(value: str) -> set[str]:
    normalized = normalize_path_ref(value)
    if not normalized:
        return set()
    keys = {normalized}
    if "://" in value:
        tail = value.rsplit("/", 1)[-1]
        tail_key = normalize_path_ref(tail)
        if tail_key:
            keys.add(tail_key)
    for separator in ("#", "?"):
        if separator in normalized:
            keys.add(normalized.split(separator, 1)[0])
    return {key for key in keys if key}


def enterprise_neighbor_keys(
    document: dict[str, Any],
    rel_path: str,
    *,
    include_source_links: bool = False,
) -> set[str]:
    keys: set[str] = set()
    src = source_type(rel_path)
    for field in SAFE_FIELD_NAMES:
        for value in string_values(document.get(field)):
            normalized = normalize_key(value)
            if normalized:
                keys.add(f"{field}:{normalized}")

    for value in string_values(document.get("related_pages")):
        normalized = normalize_path_ref(value)
        if normalized:
            keys.add(f"pathref:{normalized}")

    if include_source_links:
        for field in LINK_FIELD_NAMES:
            for value in string_values(document.get(field)):
                for normalized in link_ref_keys(value):
                    keys.add(f"{field}:{normalized}")
                    keys.add(f"pathref:{normalized}")
    for value in path_ref_keys(rel_path):
        keys.add(f"pathref:{value}")

    parts = rel_path.split("/")
    if src in {"confluence", "google_drive", "github", "jira", "linear"} and len(parts) >= 3:
        keys.add(f"dir3:{parts[0]}/{parts[1]}/{parts[2]}")
    return keys
