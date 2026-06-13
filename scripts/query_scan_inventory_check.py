#!/usr/bin/env python3
"""Keep query-adjacent full snapshot scans explicit while A06 is unfinished."""

from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ENGINE_SRC = ROOT / "crates/cortex-engine/src"

QUERY_ADJACENT = {
}

MAINTENANCE_OR_BACKFILL = {
    ("memory_accounting.rs", "let versions = db.snapshot_versions();"): 1,
    ("ingestion/dedup.rs", "self.snapshot_versions().into_iter().find_map(|version| {"): 1,
    ("ingestion/dedup.rs", "for version in self.snapshot_versions() {"): 1,
    ("memory.rs", "self.snapshot_versions()"): 2,
    ("embedding_pipeline/backfill.rs", "embedding_expected_items_from_versions(&self.snapshot_versions())"): 1,
    ("embedding_pipeline/backfill.rs", "embedding_debt_report_from_versions(&self.snapshot_versions(), config)"): 1,
    ("embedding_pipeline/backfill.rs", "let versions = self.snapshot_versions();"): 1,
}

NON_RUNTIME_GATES = {
    ("bin/memory_profile_check/payload_gate.rs", '"self.snapshot_versions()",'): 2,
}


def snapshot_calls() -> Counter[tuple[str, str]]:
    calls: Counter[tuple[str, str]] = Counter()
    for path in ENGINE_SRC.rglob("*.rs"):
        rel_path = path.relative_to(ENGINE_SRC).as_posix()
        for line in path.read_text().splitlines():
            stripped = line.strip()
            if "snapshot_versions()" in stripped:
                calls[(rel_path, stripped)] += 1
    return calls


def main() -> None:
    actual = snapshot_calls()
    expected = Counter()
    expected.update(QUERY_ADJACENT)
    expected.update(MAINTENANCE_OR_BACKFILL)
    expected.update(NON_RUNTIME_GATES)

    unexpected = actual - expected
    missing = expected - actual
    if unexpected:
        entries = "\n".join(
            f"  {path}: {count}x {line}" for (path, line), count in sorted(unexpected.items())
        )
        raise SystemExit(f"unclassified snapshot_versions() call(s):\n{entries}")
    if missing:
        entries = "\n".join(
            f"  {path}: expected {count}x {line}"
            for (path, line), count in sorted(missing.items())
        )
        raise SystemExit(f"snapshot_versions() inventory drifted:\n{entries}")

    if any(path == "query.rs" for path, _ in actual):
        raise SystemExit("query.rs must not call snapshot_versions()")

    print(
        "query scan inventory passed: "
        f"query_adjacent={sum(QUERY_ADJACENT.values())} "
        f"maintenance_or_backfill={sum(MAINTENANCE_OR_BACKFILL.values())} "
        f"non_runtime_gates={sum(NON_RUNTIME_GATES.values())}"
    )


if __name__ == "__main__":
    main()
