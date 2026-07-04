#!/usr/bin/env python3
"""Merkle + Ed25519 conformance layer for the canonical JSON cross-language check.

Split out of canonical_jcs_cross_language_check.py by responsibility (MOVE-ONLY,
no behavior change): this module owns the canonical-JSON primitive, the blake3
hash helpers, the blake3 Merkle-root construction (mirror of
receipt.rs::merkle_root) and its committed vectors, and the Ed25519 receipt
signing (mirror of cortex-crypto receipt_key.rs) and its committed vectors —
together with the two verify-only checks that re-derive those vectors in Python
and compare them to the committed fixtures.

Dependency-free (stdlib + optional blake3/cryptography); deterministic; no
network, no wall clock.
"""

from __future__ import annotations

import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parent.parent
MERKLE_FIXTURE = REPO / "fixtures" / "canonical" / "merkle_conformance_vectors.v1.json"


def canonical_json_bytes(value: object) -> bytes:
    """Mirror of crates/cortex-engine/src/canonical/mod.rs::write_canonical_value:
    sorted object keys, `,`/`:` separators with no spaces, standard JSON string
    escaping, non-ASCII preserved, integer numbers as-is."""
    return json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
    ).encode("utf-8")


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
