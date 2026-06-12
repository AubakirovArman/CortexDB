#!/usr/bin/env python3
"""Generate EnterpriseRAG candidates from multiple local indexes.

This compatibility wrapper keeps the historical script path stable while the
implementation lives in smaller modules.
"""

from __future__ import annotations

from _multi_index_candidate_generation import extract_document_content, main

__all__ = ["extract_document_content", "main"]


if __name__ == "__main__":
    raise SystemExit(main())
