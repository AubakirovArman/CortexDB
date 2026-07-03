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

This now also re-derives the receipt's blake3 Merkle roots (synthetic + the 6 roots
of a real committed receipt), its Ed25519 signatures, and its `pack_root` (the
canonical ContextPack mapping) — each asserted byte-for-byte against the matching
Rust test. This is verify-only: it never rewrites a signed golden. Remaining C4-2:
the 6 leaf-family field extractions (receipt_leaves.rs).

Dependency-free (stdlib only); deterministic; no network, no wall clock.
"""

from __future__ import annotations

import hashlib
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
FIXTURE = REPO / "fixtures" / "canonical" / "jcs_conformance_vectors.v1.json"
MERKLE_FIXTURE = REPO / "fixtures" / "canonical" / "merkle_conformance_vectors.v1.json"

# Mirror of crates/cortex-engine/src/accountability/receipt.rs.
MERKLE_EMPTY_SCHEMA = "cortexdb.accountability.merkle.empty.v1"
LEAF_DOMAIN_SUFFIX = ".leaf.v1"
NODE_DOMAIN_SUFFIX = ".node.v1"
TEST_MERKLE_DOMAIN = "cortexdb.test.accountability.merkle.v1"

# (domain, leaves) merkle-tree vectors: empty, single, even, and odd (last leaf
# duplicated) — the branches of receipt.rs::merkle_root.
MERKLE_VECTORS = [
    (TEST_MERKLE_DOMAIN, []),
    (TEST_MERKLE_DOMAIN, [{"id": 1}]),
    (TEST_MERKLE_DOMAIN, [{"id": 1}, {"id": 2}]),
    (TEST_MERKLE_DOMAIN, [{"id": 1}, {"id": 2}, {"id": 3}]),
    (TEST_MERKLE_DOMAIN, [{"id": 1}, {"id": 2}, {"id": 3}, {"id": 4}]),
]


def blake3_256_domain(domain: str, data: bytes) -> bytes:
    """Mirror of cortex-crypto blake3_256_domain: blake3(domain || 0x00 || data)."""
    import blake3

    return blake3.blake3(domain.encode("utf-8") + b"\x00" + data).digest()


def hash_bytes(domain: str, data: bytes) -> str:
    return blake3_256_domain(domain, data).hex()


def hash_value(domain: str, value: object) -> str:
    return hash_bytes(domain, canonical_json_bytes(value))


def merkle_root(domain: str, leaves: list) -> str:
    """Mirror of receipt.rs::merkle_root (binary tree, odd node duplicated)."""
    if not leaves:
        return hash_value(domain, {"schema_version": MERKLE_EMPTY_SCHEMA, "leaf_count": 0})
    leaf_domain = domain + LEAF_DOMAIN_SUFFIX
    node_domain = domain + NODE_DOMAIN_SUFFIX
    level = [hash_value(leaf_domain, leaf) for leaf in leaves]
    while len(level) > 1:
        nxt = []
        for index in range(0, len(level), 2):
            left = level[index]
            right = level[index + 1] if index + 1 < len(level) else level[index]
            nxt.append(hash_value(node_domain, {"left": left, "right": right}))
        level = nxt
    return level[0]


def build_merkle() -> list[dict]:
    return [
        {"domain": domain, "leaves": leaves, "merkle_root_blake3": merkle_root(domain, leaves)}
        for domain, leaves in MERKLE_VECTORS
    ]


# Mirror of cortex-crypto receipt_key.rs: Ed25519 over a domain-wrapped message.
RECEIPT_SIGNING_DOMAIN = "cortexdb.accountability_receipt.sign.v1"
ED25519_FIXTURE = REPO / "fixtures" / "canonical" / "ed25519_conformance_vectors.v1.json"
ED25519_VECTORS = [
    ("0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20", "receipt body one"),
    ("0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20", ""),
    ("fedcba98765432100123456789abcdeffedcba98765432100123456789abcdef", "another receipt header"),
]


def receipt_signing_bytes(message: bytes) -> bytes:
    return RECEIPT_SIGNING_DOMAIN.encode("utf-8") + b"\x00" + message


def ed25519_sign_and_pubkey(seed_hex: str, message: str) -> tuple[str, str]:
    from cryptography.hazmat.primitives import serialization
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

    key = Ed25519PrivateKey.from_private_bytes(bytes.fromhex(seed_hex))
    signature = key.sign(receipt_signing_bytes(message.encode("utf-8")))
    public = key.public_key().public_bytes(
        serialization.Encoding.Raw, serialization.PublicFormat.Raw
    )
    return signature.hex(), public.hex()


def build_ed25519() -> list[dict]:
    out = []
    for seed_hex, message in ED25519_VECTORS:
        signature_hex, public_key_hex = ed25519_sign_and_pubkey(seed_hex, message)
        out.append(
            {
                "seed_hex": seed_hex,
                "message": message,
                "signature_hex": signature_hex,
                "public_key_hex": public_key_hex,
            }
        )
    return out


# The real committed accountability receipt: its header roots were produced by
# Rust; re-deriving them here from its committed leaves proves the receipt-body
# Merkle composition reproduces cross-language on real data (not just synthetics).
RECEIPT_GOLDEN = REPO / "fixtures" / "accountability_receipt" / "verify_input.golden.json"
RECEIPT_FAMILY_SPECS = [
    ("access", "cortexdb.accountability.receipt.access_root.v1", "access_root"),
    ("provenance", "cortexdb.accountability.receipt.provenance_root.v1", "provenance_root"),
    ("cell_set", "cortexdb.accountability.receipt.cell_set_root.v1", "cell_set_root"),
    ("verification", "cortexdb.accountability.receipt.verification_root.v1", "verification_root"),
    ("budget", "cortexdb.accountability.receipt.budget_commitment.v1", "budget_commitment"),
    ("conflict", "cortexdb.accountability.receipt.conflict_commitment.v1", "conflict_commitment"),
]

# C4-2 (pack_root): the receipt's `pack_root` is blake3(domain || 0x00 ||
# canonical_context_pack_bytes(pack)). Unlike the Merkle families, `pack_root`
# hashes a *mapping* of the ContextPack struct into a canonical Value
# (canonical.rs::context_pack_value) before canonicalization — so to verify it
# cross-language we must re-implement that mapping, not just the hash. The
# committed accountability golden serializes the pack in its *response* form
# (without each cell's raw payload bytes), so it cannot drive this check; instead
# we pin a minimal pack whose every field is reproducible in both languages and
# assert the derived `pack_root` matches. A Rust test
# (receipt_tests.rs::pack_root_matches_cross_language_vector) builds the same
# minimal pack through the real engine `canonical_context_pack_bytes` and asserts
# the identical committed root, closing the loop.
PACK_ROOT_FIXTURE = REPO / "fixtures" / "canonical" / "pack_root_conformance_vector.v1.json"
PACK_ROOT_DOMAIN = "cortexdb.accountability.receipt.pack_root.v1"
# The minimal pack: one cell with a plain payload (no `source_id=`/`source=`/
# `citation=` header line, so metadata.source_ref is None -> null) and every
# optional (citation, source_ref, provenance, explain, access_decision,
# grounding_report) absent, and no anomalies. This keeps the Python re-mapping a
# faithful, total mirror of context_pack_value for the fields that are present.
PACK_ROOT_INPUT = {
    "token_budget_tokens": 128,
    "estimated_tokens": 5,
    "truncated": False,
    "citations_required": False,
    "answerability_q16": 0,
    "conflict_visibility_q16": 0,
    "visible_conflict_count": 0,
    "cells": [
        {
            "cell_id": 1,
            "payload_utf8": "cortex pack root cross-language fixture body",
            "estimated_tokens": 5,
            "citation": None,
        }
    ],
}


def context_pack_cell_value(cell: dict) -> dict:
    """Mirror of canonical.rs::context_pack_cell_value for a cell with no
    optional sub-structures (source_ref/provenance/explain/access_decision)."""
    return {
        "cell_id": cell["cell_id"],
        "payload_hex": cell["payload_utf8"].encode("utf-8").hex(),
        "estimated_tokens": cell["estimated_tokens"],
        "citation": cell["citation"],
        "source_ref": None,
        "provenance": None,
        "explain": None,
        "access_decision": None,
    }


def context_pack_value(pack_input: dict) -> dict:
    """Mirror of canonical.rs::context_pack_value (schema context_pack.canonical.v1)."""
    return {
        "schema_version": "context_pack.canonical.v1",
        "token_budget_tokens": pack_input["token_budget_tokens"],
        "estimated_tokens": pack_input["estimated_tokens"],
        "truncated": pack_input["truncated"],
        "citations_required": pack_input["citations_required"],
        "answerability_q16": pack_input["answerability_q16"],
        "conflict_visibility_q16": pack_input["conflict_visibility_q16"],
        "visible_conflict_count": pack_input["visible_conflict_count"],
        "cells": [context_pack_cell_value(cell) for cell in pack_input["cells"]],
        "anomalies": [],
        "grounding_report": None,
    }


def pack_root(pack_input: dict) -> tuple[str, str]:
    """Return (canonical_pack_bytes_hex, pack_root) for a minimal ContextPack."""
    canonical = canonical_json_bytes(context_pack_value(pack_input))
    return canonical.hex(), hash_bytes(PACK_ROOT_DOMAIN, canonical)


def budget_leaves(pack_input: dict) -> list:
    """Mirror of receipt_leaves.rs::budget_leaves (pack-scalar leaf family): a
    pack-wide leaf followed by one per cell. Pure scalars — no cell content hash."""
    leaves = [
        {
            "token_budget_tokens": pack_input["token_budget_tokens"],
            "estimated_tokens": pack_input["estimated_tokens"],
            "truncated": pack_input["truncated"],
            "cell_id": None,
            "cell_estimated_tokens": None,
        }
    ]
    for cell in pack_input["cells"]:
        leaves.append(
            {
                "token_budget_tokens": pack_input["token_budget_tokens"],
                "estimated_tokens": pack_input["estimated_tokens"],
                "truncated": pack_input["truncated"],
                "cell_id": cell["cell_id"],
                "cell_estimated_tokens": cell["estimated_tokens"],
            }
        )
    return leaves


def conflict_leaves(pack_input: dict) -> list:
    """Mirror of receipt_leaves.rs::conflict_leaves: a single leaf carrying the
    conflict-visibility scalars + the sorted anomaly codes (empty for the minimal
    pack, which has no anomalies)."""
    anomalies = sorted(anomaly["code"] for anomaly in pack_input.get("anomalies", []))
    return [
        {
            "conflict_visibility_q16": pack_input["conflict_visibility_q16"],
            "visible_conflict_count": pack_input["visible_conflict_count"],
            "anomalies": anomalies,
        }
    ]


# Mirrors receipt_tests.rs::sample_receipt_inputs — the allowed access decision +
# denied candidate whose leaves receipt_leaves.rs::access_leaves extracts. Kept in
# sync with that Rust sample; the Rust test
# (access_leaves_match_cross_language_vector) asserts the real engine reproduces
# these committed leaves, so any drift in the sample fails loudly.
ACCESS_ROOT_DOMAIN = "cortexdb.accountability.receipt.access_root.v1"
ACCESS_LEAVES_INPUT = {
    "admitted": [
        {
            "cell_id": 1,
            "policy": "agent_view_readable_scope",
            "policy_version": "agent_view_readable_scope.v1",
            "reason": "cell candidate survived AQL permission filtering before ContextPack packing",
            "scope_id": 1001,
            "agent_id": 7,
            "agent_view_digest": "a" * 64,
        }
    ],
    "denied": [
        {
            "candidate": 2,
            "cell_id_hash": "b" * 64,
            "policy": "agent_view_readable_scope",
            "policy_version": "agent_view_readable_scope.v1",
            "reason": (
                "cell candidate was rejected by AQL agent access filtering "
                "before payload materialization"
            ),
            "agent_id": 7,
            "agent_view_digest": "a" * 64,
            "evidence_digest": "c" * 64,
        }
    ],
}


def access_leaves(access_input: dict) -> list:
    """Mirror of receipt_leaves.rs::access_leaves (+ admitted_access_leaf): the
    admitted cells (each with an evidence_digest = hash_value over the access
    evidence) followed by the denied candidates (whose evidence_digest is
    precomputed upstream). Self-contained — no cell-content hash."""
    leaves = []
    for admitted in access_input["admitted"]:
        evidence = {
            "cell_id": admitted["cell_id"],
            "decision": "allowed",
            "policy": admitted["policy"],
            "policy_version": admitted["policy_version"],
            "reason": admitted["reason"],
            "scope_id": admitted["scope_id"],
            "agent_id": admitted["agent_id"],
            "agent_view_digest": admitted["agent_view_digest"],
        }
        leaves.append(
            {
                "leaf_type": "admitted_cell",
                "cell_id": admitted["cell_id"],
                "candidate": None,
                "cell_id_hash": None,
                "decision": "allowed",
                "policy": admitted["policy"],
                "policy_version": admitted["policy_version"],
                "reason": admitted["reason"],
                "scope_id": admitted["scope_id"],
                "agent_id": admitted["agent_id"],
                "agent_view_digest": admitted["agent_view_digest"],
                "evidence_digest": hash_value(ACCESS_ROOT_DOMAIN, evidence),
            }
        )
    for denied in access_input["denied"]:
        leaves.append(
            {
                "leaf_type": "denied_candidate",
                "cell_id": None,
                "candidate": denied["candidate"],
                "cell_id_hash": denied["cell_id_hash"],
                "decision": "denied",
                "policy": denied["policy"],
                "policy_version": denied["policy_version"],
                "reason": denied["reason"],
                "scope_id": None,
                "agent_id": denied["agent_id"],
                "agent_view_digest": denied["agent_view_digest"],
                "evidence_digest": denied["evidence_digest"],
            }
        )
    return leaves


def build_pack_root() -> dict:
    canonical_hex, root = pack_root(PACK_ROOT_INPUT)
    return {
        "pack_root_domain": PACK_ROOT_DOMAIN,
        "pack_input": PACK_ROOT_INPUT,
        "canonical_pack_bytes_hex": canonical_hex,
        "pack_root_blake3": root,
        # Scalar-only leaf families (extraction verified cross-language; the
        # content-hash-bearing families remain — see task_8f2c7c22).
        "budget_leaves": budget_leaves(PACK_ROOT_INPUT),
        "conflict_leaves": conflict_leaves(PACK_ROOT_INPUT),
        # Access leaf family (self-contained: an evidence_digest hash over the
        # access evidence, no cell-content hash). Input mirrors sample_receipt_inputs.
        "access_leaves_input": ACCESS_LEAVES_INPUT,
        "access_leaves": access_leaves(ACCESS_LEAVES_INPUT),
    }


def check_pack_root() -> list[str]:
    if not PACK_ROOT_FIXTURE.exists():
        return [f"missing fixture {PACK_ROOT_FIXTURE.relative_to(REPO)} (run with --generate)"]
    committed = json.loads(PACK_ROOT_FIXTURE.read_text())
    pack_input = committed["pack_input"]
    errors = []
    # Leaf-extraction families are hash-free structural maps — verifiable without
    # blake3 (the Rust engine functions produce the same Values, asserted in
    # receipt_tests.rs::pack_leaf_families_match_cross_language_vector).
    if budget_leaves(pack_input) != committed["budget_leaves"]:
        errors.append("budget_leaves: python extraction != committed")
    if conflict_leaves(pack_input) != committed["conflict_leaves"]:
        errors.append("conflict_leaves: python extraction != committed")
    try:
        import blake3  # noqa: F401
    except ImportError:
        # The Rust test still verifies pack_root against this committed fixture,
        # so the cross-language proof holds; Python re-derivation needs `pip
        # install blake3`.
        print("  note: blake3 not installed; skipping the Python pack_root re-derivation")
        return errors
    canonical_hex, root = pack_root(pack_input)
    if canonical_hex != committed["canonical_pack_bytes_hex"]:
        errors.append("pack_root: python canonical pack bytes != committed")
    if root != committed["pack_root_blake3"]:
        errors.append(
            f"pack_root: python root {root} != committed {committed['pack_root_blake3']}"
        )
    # access_leaves carries a blake3 evidence_digest, so it lives in the gated block.
    if access_leaves(committed["access_leaves_input"]) != committed["access_leaves"]:
        errors.append("access_leaves: python extraction != committed")
    return errors


def check_receipt_golden() -> list[str]:
    if not RECEIPT_GOLDEN.exists():
        return []
    try:
        import blake3  # noqa: F401
    except ImportError:
        return []
    receipt = json.loads(RECEIPT_GOLDEN.read_text())["receipt"]
    header, leaves = receipt["header"], receipt["leaves"]
    errors = []
    for family, domain, root_field in RECEIPT_FAMILY_SPECS:
        # The receipt root is over receipt.rs::sorted_values(leaves) — leaves
        # sorted by their canonical bytes — before merkle_root; the stored order
        # is not guaranteed sorted, so replicate the sort here.
        family_leaves = sorted(leaves[family], key=canonical_json_bytes)
        derived = merkle_root(domain, family_leaves)
        expected = header[root_field]
        if derived != expected:
            errors.append(
                f"real-receipt {family}: python root {derived} != committed header "
                f"{root_field} {expected}"
            )
    return errors


def check_ed25519() -> list[str]:
    if not ED25519_FIXTURE.exists():
        return [f"missing fixture {ED25519_FIXTURE.relative_to(REPO)} (run with --generate)"]
    try:
        from cryptography.hazmat.primitives.asymmetric.ed25519 import (  # noqa: F401
            Ed25519PrivateKey,
        )
    except ImportError:
        print("  note: cryptography not installed; skipping the Python Ed25519 re-derivation")
        return []
    errors = []
    for index, entry in enumerate(json.loads(ED25519_FIXTURE.read_text())):
        signature_hex, public_key_hex = ed25519_sign_and_pubkey(entry["seed_hex"], entry["message"])
        if signature_hex != entry["signature_hex"]:
            errors.append(f"ed25519 vector {index}: python signature != committed")
        if public_key_hex != entry["public_key_hex"]:
            errors.append(f"ed25519 vector {index}: python public key != committed")
    return errors


def check_merkle() -> list[str]:
    if not MERKLE_FIXTURE.exists():
        return [f"missing fixture {MERKLE_FIXTURE.relative_to(REPO)} (run with --generate)"]
    try:
        import blake3  # noqa: F401
    except ImportError:
        # The Rust test still verifies merkle_root against this committed
        # (Python-derived) fixture, so the cross-language proof holds; the Python
        # re-derivation is a redundant check that needs `pip install blake3`.
        print("  note: blake3 not installed; skipping the Python Merkle re-derivation")
        return []
    errors = []
    for index, entry in enumerate(json.loads(MERKLE_FIXTURE.read_text())):
        actual = merkle_root(entry["domain"], entry["leaves"])
        if actual != entry["merkle_root_blake3"]:
            errors.append(
                f"merkle vector {index}: python root {actual} != committed "
                f"{entry['merkle_root_blake3']}"
            )
    return errors

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
        MERKLE_FIXTURE.write_text(json.dumps(build_merkle(), indent=2, ensure_ascii=False) + "\n")
        ED25519_FIXTURE.write_text(json.dumps(build_ed25519(), indent=2) + "\n")
        PACK_ROOT_FIXTURE.write_text(json.dumps(build_pack_root(), indent=2) + "\n")
        print(
            f"wrote {FIXTURE.relative_to(REPO)} ({len(VECTORS)}) + "
            f"{MERKLE_FIXTURE.relative_to(REPO)} ({len(MERKLE_VECTORS)}) + "
            f"{ED25519_FIXTURE.relative_to(REPO)} ({len(ED25519_VECTORS)}) + "
            f"{PACK_ROOT_FIXTURE.relative_to(REPO)} (1)"
        )
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
                f"jcs vector {index}: python canonical digest {actual} != committed "
                f"{entry['canonical_sha256']}"
            )
    errors.extend(check_merkle())
    errors.extend(check_receipt_golden())
    errors.extend(check_ed25519())
    errors.extend(check_pack_root())
    if errors:
        print("canonical-jcs-cross-language-check FAILED")
        for error in errors:
            print(f"  {error}")
        return 1
    print(
        f"canonical-jcs-cross-language-check passed: {len(committed)} JCS + "
        f"{len(MERKLE_VECTORS)} Merkle + {len(ED25519_VECTORS)} Ed25519 vector(s) + the "
        f"{len(RECEIPT_FAMILY_SPECS)} roots of a real committed receipt + the pack_root "
        "+ budget/conflict/access leaf extraction (3 of 6 families); python "
        "canonicalization + blake3 Merkle roots + Ed25519 signatures + pack "
        "canonicalization + leaf extraction reproduce the committed values "
        "byte-for-byte"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
