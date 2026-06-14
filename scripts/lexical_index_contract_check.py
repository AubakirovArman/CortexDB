#!/usr/bin/env python3
"""Guard the C01 compact lexical index contract."""
from __future__ import annotations

import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise SystemExit(f"missing {label}: {needle}")


def run(command: list[str]) -> None:
    result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=False)
    if result.returncode != 0:
        tail = "\n".join((result.stdout + result.stderr).splitlines()[-80:])
        raise SystemExit(f"command failed: {' '.join(command)}\n{tail}")


def main() -> None:
    format_rs = read("crates/cortex-storage/src/format.rs")
    indexes_rs = read("crates/cortex-storage/src/indexes.rs")
    tests_rs = read("crates/cortex-storage/tests/lexical_index_tests.rs")
    storage_doc = read("docs/STORAGE_FORMATS.md")

    require(format_rs, 'pub const LEXICAL_INDEX_MAGIC: [u8; 4] = *b"ACI4";', "current ACI4 magic")
    require(
        format_rs,
        'pub const LEGACY_LEXICAL_INDEX_V3_MAGIC: [u8; 4] = *b"ACI3";',
        "ACI3 legacy magic",
    )
    require(indexes_rs, "build_term_dictionary", "term dictionary builder")
    require(indexes_rs, "lookup_term_id", "term-id lookup")
    require(indexes_rs, "put_compact_set", "compact postings writer")
    require(indexes_rs, "read_compact_set", "compact postings reader")
    require(indexes_rs, "put_var_u32", "delta-varint writer")
    require(indexes_rs, "read_var_u32", "delta-varint reader")
    require(tests_rs, "aci3_lexical_index_remains_readable_with_field_frequencies", "ACI3 dual-read test")
    require(tests_rs, "aci4_term_dictionary_reduces_repeated_term_storage", "compact-size regression")
    require(storage_doc, 'magic[4] = "ACI4"', "storage format doc")
    require(storage_doc, "term_dictionary_count", "term dictionary layout doc")
    require(storage_doc, "delta_from_previous_candidate var_u32", "delta-varint layout doc")

    run(["cargo", "test", "-p", "cortex-storage", "--test", "lexical_index_tests"])
    print("lexical index contract gate passed")


if __name__ == "__main__":
    main()
