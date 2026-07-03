#!/usr/bin/env python3
"""C4-2 (foundation): cross-language conformance for the canonical JSON layer.

The accountability receipt is signed over `canonical_json_bytes` — a deterministic
JSON serialization (sorted object keys, no insignificant whitespace, standard
string escaping, integer-valued numbers). If that canonicalization is truly
normative it must be reproducible from another language. This re-implements it in
pure Python and asserts, over a committed vector set, that the sha256 of the
Python canonical bytes equals the committed digest — the SAME digest a Rust test
(`canonical::jcs_cross_language_vectors_match`) asserts against
`canonical_json_bytes`. Both languages agreeing on the digest proves the bytes
are identical (a sha256 collision is infeasible), so the canonicalization is
language-independent.

This is verify-only: it never rewrites a signed golden. The Merkle-leaf/root and
Ed25519 layers are the remaining C4-2 work.

Dependency-free (stdlib only); deterministic; no network, no wall clock.
"""

from __future__ import annotations

import hashlib
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
FIXTURE = REPO / "fixtures" / "canonical" / "jcs_conformance_vectors.v1.json"

# The canonical JSON test vectors. Integer-only numbers (as in the receipt
# canonical set) so number formatting is unambiguous cross-language.
VECTORS = [
    {"b": 1, "a": 2},
    [3, 1, 2],
    {"nested": {"z": True, "a": None}, "arr": [1, {"y": 2, "x": 1}]},
    {"empty_obj": {}, "empty_arr": []},
    42,
    "café",  # non-ASCII: preserved as UTF-8 (ensure_ascii=False), not \u-escaped
    {"s": 'he said "hi"\nand \\ backslash', "t": "\t\r\b\f"},
    {"schema_version": "context_pack.canonical.v1", "answerability_q16": 65535},
]


def canonical_json_bytes(value: object) -> bytes:
    """Mirror of crates/cortex-engine/src/canonical.rs::write_canonical_value:
    sorted object keys, `,`/`:` separators with no spaces, standard JSON string
    escaping, non-ASCII preserved, integer numbers as-is."""
    return json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
    ).encode("utf-8")


def digest(value: object) -> str:
    return hashlib.sha256(canonical_json_bytes(value)).hexdigest()


def build() -> list[dict]:
    return [{"value": value, "canonical_sha256": digest(value)} for value in VECTORS]


def main() -> int:
    generate = "--generate" in sys.argv
    if generate:
        FIXTURE.parent.mkdir(parents=True, exist_ok=True)
        FIXTURE.write_text(json.dumps(build(), indent=2, ensure_ascii=False) + "\n")
        print(f"wrote {FIXTURE.relative_to(REPO)} ({len(VECTORS)} vectors)")
        return 0

    if not FIXTURE.exists():
        print(f"missing fixture {FIXTURE.relative_to(REPO)} (run with --generate)")
        return 1
    committed = json.loads(FIXTURE.read_text())
    errors = []
    for index, entry in enumerate(committed):
        actual = digest(entry["value"])
        if actual != entry["canonical_sha256"]:
            errors.append(
                f"vector {index}: python canonical digest {actual} != committed "
                f"{entry['canonical_sha256']}"
            )
    if errors:
        print("canonical-jcs-cross-language-check FAILED")
        for error in errors:
            print(f"  {error}")
        return 1
    print(
        f"canonical-jcs-cross-language-check passed: {len(committed)} vector(s); "
        "python canonical JSON matches the committed digests byte-for-byte"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
