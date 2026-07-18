#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(rel: str) -> str:
    return (ROOT / rel).read_text(encoding="utf-8")


def require(condition: bool, message: str, errors: list[str]) -> None:
    if not condition:
        errors.append(message)


def contains(rel: str, markers: list[str], errors: list[str]) -> None:
    text = read(rel)
    for marker in markers:
        require(marker in text, f"{rel}: missing marker {marker!r}", errors)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", default="target/engine-feature-flags/report.json")
    args = parser.parse_args()

    errors: list[str] = []

    contains(
        "crates/cortex-engine/src/options.rs",
        [
            "pub enum EngineFeature",
            "ExperimentalHnsw",
            "ExperimentalReplication",
            "Dashboard",
            "pub struct EngineFeatureFlags",
            "pub const fn production_safe() -> Self",
            "experimental_hnsw: false",
            "experimental_replication: false",
            "dashboard: false",
            "pub feature_flags: EngineFeatureFlags",
        ],
        errors,
    )
    contains(
        "crates/cortex-engine/src/database/types.rs",
        ["pub(crate) feature_flags: EngineFeatureFlags"],
        errors,
    )
    contains(
        "crates/cortex-engine/src/database/open.rs",
        [
            "pub fn feature_flags(&self) -> EngineFeatureFlags",
            "pub(crate) fn require_feature(&self, feature: EngineFeature)",
            "EngineError::FeatureDisabled(feature.as_str())",
        ],
        errors,
    )
    contains(
        "crates/cortex-engine/src/checkpoint/database.rs",
        [
            "feature_flags",
            "experimental_hnsw",
            "hnsw_profile = hnsw_profile",
        ],
        errors,
    )
    contains(
        "crates/cortex-engine/src/search/database/persisted.rs",
        [
            "!self.feature_flags.experimental_hnsw",
        ],
        errors,
    )
    contains(
        "crates/cortex-engine/src/search/database/ann_reports.rs",
        [
            "persisted_exact_fallback_report",
            "AnnSearchPath::ExactFallback",
        ],
        errors,
    )
    contains(
        "crates/cortex-engine/src/replication/install.rs",
        [
            "EngineFeature::ExperimentalReplication",
            "self.require_feature",
        ],
        errors,
    )
    contains(
        "crates/cortex-engine/Cargo.toml",
        ['experimental-replication = []'],
        errors,
    )
    contains(
        "crates/cortex-engine/src/lib.rs",
        [
            '#[cfg(feature = "experimental-replication")]',
            "pub mod replication;",
            "pub mod distributed;",
        ],
        errors,
    )
    contains(
        "crates/cortex-server/src/config.rs",
        ["pub dashboard_enabled: bool"],
        errors,
    )
    contains(
        "crates/cortex-server/src/handler.rs",
        [
            "state.options.dashboard_enabled",
        ],
        errors,
    )
    contains(
        "crates/cortex-server/src/main.rs",
        [
            "EngineConfig::from_env",
        ],
        errors,
    )
    contains(
        "crates/cortex-engine/src/config.rs",
        [
            "CORTEXDB_EXPERIMENTAL_HNSW",
            "CORTEXDB_EXPERIMENTAL_REPLICATION",
            "CORTEXDB_DASHBOARD",
        ],
        errors,
    )
    contains(
        "crates/cortex-engine/tests/feature_flags.rs",
        [
            "engine_feature_flags_default_to_production_safe",
            "default_checkpoint_skips_hnsw_and_vector_search_uses_exact_fallback",
            "experimental_hnsw_flag_persists_graph_profile",
            "replication_database_surface_requires_feature_flag",
        ],
        errors,
    )
    contains(
        "crates/cortex-server/src/dashboard_tests.rs",
        [
            "dashboard_endpoint_is_disabled_by_default",
            "dashboard_enabled: true",
        ],
        errors,
    )
    contains(
        "docs/ENGINE_FEATURE_FLAGS.md",
        [
            "experimental_hnsw",
            "experimental_replication",
            "CORTEXDB_DASHBOARD=true",
            "Default `DatabaseOptions` must not build HNSW graphs",
        ],
        errors,
    )

    report_path = ROOT / args.report
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report = {
        "schema_version": "cortexdb.engine_feature_flags_check.v1",
        "ok": not errors,
        "errors": errors,
    }
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        return 1
    print(f"engine feature flags check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
