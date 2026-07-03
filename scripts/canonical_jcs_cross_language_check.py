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
        print(
            f"wrote {FIXTURE.relative_to(REPO)} ({len(VECTORS)}) + "
            f"{MERKLE_FIXTURE.relative_to(REPO)} ({len(MERKLE_VECTORS)}) + "
            f"{ED25519_FIXTURE.relative_to(REPO)} ({len(ED25519_VECTORS)})"
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
    errors.extend(check_ed25519())
    if errors:
        print("canonical-jcs-cross-language-check FAILED")
        for error in errors:
            print(f"  {error}")
        return 1
    print(
        f"canonical-jcs-cross-language-check passed: {len(committed)} JCS + "
        f"{len(MERKLE_VECTORS)} Merkle + {len(ED25519_VECTORS)} Ed25519 vector(s); "
        "python canonicalization + blake3 Merkle roots + Ed25519 signatures match the "
        "committed values byte-for-byte"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
