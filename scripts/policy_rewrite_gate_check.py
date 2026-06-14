#!/usr/bin/env python3
"""Guard the B16 PolicyRewrite read-surface invariant."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ENGINE_SRC = ROOT / "crates/cortex-engine/src"
SERVER_SRC = ROOT / "crates/cortex-server/src"
POLICY = ENGINE_SRC / "plan/policy.rs"
PLAN_TESTS = ENGINE_SRC / "plan/tests.rs"
AUTHZ = SERVER_SRC / "authz.rs"
QUERY_PROVIDER = ENGINE_SRC / "query/provider.rs"
SEARCH_ACCESS = ENGINE_SRC / "search/access.rs"

REQUIRED_SURFACES = (
    "AqlRetrieve",
    "AqlExplain",
    "Search",
    "SearchExplain",
    "ContextPack",
    "ContextTrace",
    "CellGet",
    "Verify",
    "Graph",
    "Memory",
    "Feedback",
    "Export",
)


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise SystemExit(f"missing {label}: {needle}")


def production_rs_files(root: Path) -> list[Path]:
    return [
        path
        for path in root.rglob("*.rs")
        if "/tests/" not in path.as_posix()
        and "/src/bin/" not in path.as_posix()
        and path.name != "tests.rs"
    ]


def main() -> None:
    policy = POLICY.read_text()
    plan_tests = PLAN_TESTS.read_text()
    authz = AUTHZ.read_text()
    query_provider = QUERY_PROVIDER.read_text()
    search_access = SEARCH_ACCESS.read_text()

    require(policy, "pub enum ReadSurface", "read surface enum")
    require(policy, "pub const READ_SURFACES", "read surface registry")
    require(policy, "pub struct PolicyRewrite", "PolicyRewrite source of truth")
    require(policy, "POLICY_AGENT_ALLOWED_PREDICATE", "agent_allowed predicate constant")
    require(policy, "pub fn allows_scope", "scope authorization helper")
    require(policy, "pub fn allows_descriptor", "descriptor authorization helper")
    require(policy, "pub fn rewrite_read_surface", "read-surface rewrite helper")
    for surface in REQUIRED_SURFACES:
        require(policy, f"ReadSurface::{surface}", f"{surface} in READ_SURFACES")
        require(policy, f"Self::{surface}", f"{surface} string mapping")

    require(
        plan_tests,
        "fn all_read_surfaces_rewrite_to_policy_complete_plans",
        "structural read-surface rewrite test",
    )
    require(
        plan_tests,
        "fn descriptor_read_policy_uses_durable_descriptor_scope",
        "descriptor-scope policy test",
    )
    require(authz, "use cortex_engine::{scope_id, Database, PolicyRewrite};", "server authz imports PolicyRewrite")
    require(authz, "PolicyRewrite::allows_descriptor(view, descriptor)", "server descriptor read authorization")
    require(authz, "PolicyRewrite::allows_scope(view, scope_id(scope))", "server read scope authorization")
    require(query_provider, "let policy = PolicyRewrite::new(view);", "AQL provider policy helper")
    require(search_access, "let policy = PolicyRewrite::new(view);", "search bitmap policy helper")

    direct_can_read = []
    for root in (ENGINE_SRC, SERVER_SRC):
        for path in production_rs_files(root):
            if path == POLICY:
                continue
            text = path.read_text()
            if ".can_read_scope(" in text:
                rel = path.relative_to(ROOT)
                direct_can_read.append(rel.as_posix())
    if direct_can_read:
        paths = "\n".join(f"  {path}" for path in sorted(direct_can_read))
        raise SystemExit(f"production read paths must call PolicyRewrite helpers:\n{paths}")

    print("policy rewrite gate passed")


if __name__ == "__main__":
    main()
