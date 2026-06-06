#!/usr/bin/env python3
"""Validate that live HTTP responses match the OpenAPI schema contract.

This script:
1. Starts a temporary CortexDB server.
2. Seeds minimal data.
3. Issues requests to every endpoint family.
4. Validates JSON responses against the corresponding OpenAPI schema.
"""

import json
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

import yaml
from jsonschema import validate, RefResolver


def load_spec(repo: Path) -> dict:
    return yaml.safe_load((repo / "docs/openapi.yaml").read_text())


def resolve_schema(spec: dict, ref: str) -> dict:
    """Resolve a '#/components/schemas/X' reference."""
    if ref.startswith("#/"):
        parts = ref.lstrip("#").strip("/").split("/")
        node = spec
        for part in parts:
            node = node[part]
        return node
    raise ValueError(f"unsupported ref: {ref}")


def schema_for_response(spec: dict, path: str, method: str, status: str = "200") -> dict | None:
    """Extract the JSON schema for a given endpoint and status."""
    path_item = spec.get("paths", {}).get(path)
    if not path_item:
        return None
    method_item = path_item.get(method.lower())
    if not method_item:
        return None
    responses = method_item.get("responses", {})
    response = responses.get(status) or responses.get("200")
    if not response:
        return None
    content = response.get("content", {})
    json_content = content.get("application/json", {})
    schema = json_content.get("schema")
    if not schema:
        return None
    if "$ref" in schema:
        return resolve_schema(spec, schema["$ref"])
    return schema


CONTRACT_AUTH_TOKEN = "contract-admin"


def start_server(repo: Path, port: int) -> tuple[subprocess.Popen, Path]:
    """Build and start the cortex-server binary on a temp dir."""
    subprocess.run(["cargo", "build", "-p", "cortex-server"], cwd=repo, check=True)
    tmpdir = Path(tempfile.mkdtemp(prefix="cortex_contract_"))
    bin_path = repo / "target" / "debug" / "cortex-server"
    auth_policy_store = tmpdir / "auth-policy.json"
    auth_policy_store.write_text(
        '{"schema_version":"cortexdb.auth_policy.v1","principals":[]}\n',
        encoding="utf-8",
    )
    env = {
        **dict(subprocess.os.environ),
        "RUST_LOG": "error",
        "CORTEXDB_AUTH_TOKEN": CONTRACT_AUTH_TOKEN,
        "CORTEXDB_AUTH_POLICY_STORE_FILE": str(auth_policy_store),
        "CORTEXDB_LLM_TEST_DOUBLE": "true",
    }
    proc = subprocess.Popen(
        [str(bin_path), str(tmpdir), f"127.0.0.1:{port}"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        cwd=repo,
        env=env,
    )
    # Wait for server to come up
    for _ in range(50):
        try:
            req = urllib.request.Request(f"http://127.0.0.1:{port}/v1/health")
            req.add_header("Authorization", f"Bearer {CONTRACT_AUTH_TOKEN}")
            with urllib.request.urlopen(req, timeout=0.2) as resp:
                if resp.status == 200:
                    break
        except Exception:
            pass
        time.sleep(0.1)
    else:
        proc.terminate()
        raise RuntimeError("server failed to start")
    return proc, tmpdir


def request(
    method: str,
    url: str,
    body: bytes | None = None,
    content_type: str = "text/plain",
) -> dict:
    """Make an HTTP request and return parsed JSON."""
    req = urllib.request.Request(url, method=method, data=body)
    req.add_header("Authorization", f"Bearer {CONTRACT_AUTH_TOKEN}")
    if body is not None:
        req.add_header("Content-Type", content_type)
    with urllib.request.urlopen(req, timeout=5) as resp:
        return json.loads(resp.read().decode("utf-8"))


def main() -> int:
    repo = Path(__file__).parent.parent
    port = 18181
    proc, tmpdir = start_server(repo, port)
    try:
        spec = load_spec(repo)
        base = f"http://127.0.0.1:{port}"
        errors = []

        resolver = RefResolver.from_schema(spec)

        def check(path: str, method: str, resp: dict, status: str = "200"):
            schema = schema_for_response(spec, path, method, status)
            if schema is None:
                errors.append(f"MISSING_SCHEMA {method} {path}")
                return
            try:
                validate(instance=resp, schema=schema, resolver=resolver)
            except Exception as exc:
                errors.append(f"VALIDATION_ERROR {method} {path}: {exc}")

        # Health
        check("/v1/health", "GET", request("GET", f"{base}/v1/health"))

        # Stats
        check("/v1/stats", "GET", request("GET", f"{base}/v1/stats"))

        # Validate
        check("/v1/validate", "GET", request("GET", f"{base}/v1/validate"))

        # Put cell → Get cell
        put_resp = request(
            "POST",
            f"{base}/v1/cell?cell_id=1",
            b"scope=default\nstatus=ready\ntype=fact\nsource=contract_test\n\nhello world",
        )
        check("/v1/cell", "POST", put_resp)

        get_resp = request("GET", f"{base}/v1/cell?cell_id=1")
        check("/v1/cell", "GET", get_resp)

        # Admin auth policy mutation
        auth_upsert_resp = request(
            "POST",
            f"{base}/v1/admin/auth/principal",
            b'{"principal_id":"contract-data","token":"contract-data-token","role":"data","request_quota_per_minute":600}',
        )
        check("/v1/admin/auth/principal", "POST", auth_upsert_resp)

        auth_disable_resp = request(
            "DELETE",
            f"{base}/v1/admin/auth/principal?principal_id=contract-data",
        )
        check("/v1/admin/auth/principal", "DELETE", auth_disable_resp)

        auth_rollback_resp = request(
            "POST",
            f"{base}/v1/admin/auth/policy/rollback",
            b"",
        )
        check("/v1/admin/auth/policy/rollback", "POST", auth_rollback_resp)

        profile_put_resp = request(
            "PUT",
            f"{base}/v1/admin/search/hnsw/no-fallback-profile",
            b'{"rollout_enabled":true,"min_recall_q16":65535,"require_upper_layers":true}',
            content_type="application/json",
        )
        check(
            "/v1/admin/search/hnsw/no-fallback-profile",
            "PUT",
            profile_put_resp,
        )

        profile_get_resp = request(
            "GET",
            f"{base}/v1/admin/search/hnsw/no-fallback-profile",
        )
        check(
            "/v1/admin/search/hnsw/no-fallback-profile",
            "GET",
            profile_get_resp,
        )

        profile_delete_resp = request(
            "DELETE",
            f"{base}/v1/admin/search/hnsw/no-fallback-profile",
        )
        check(
            "/v1/admin/search/hnsw/no-fallback-profile",
            "DELETE",
            profile_delete_resp,
        )

        # Search
        search_resp = request(
            "POST",
            f"{base}/v1/search?scope=default&mode=keyword&q=hello&limit=10",
            b"",
        )
        check("/v1/search", "POST", search_resp)

        # AQL retrieve and explain
        aql_resp = request(
            "POST",
            f"{base}/v1/aql?scope=default",
            b'RETRIEVE CONTEXT FOR TASK "hello" IN BRAIN default WHERE space = default AND status = "ready" LIMIT 10 CANDIDATES;',
        )
        check("/v1/aql", "POST", aql_resp)

        aql_explain_resp = request(
            "POST",
            f"{base}/v1/aql?scope=default",
            b'EXPLAIN RETRIEVE CONTEXT FOR TASK "hello" IN BRAIN default WHERE space = default AND status = "ready" LIMIT 10 CANDIDATES;',
        )
        check("/v1/aql", "POST", aql_explain_resp)

        # Context
        context_resp = request(
            "POST",
            f"{base}/v1/context?scope=default",
            b'RETRIEVE CONTEXT FOR TASK "hello" IN BRAIN default WHERE space = default AND status = "ready" LIMIT 10 CANDIDATES;',
        )
        check("/v1/context", "POST", context_resp)

        inference_body = (
            repo / "crates/cortex-engine/fixtures/llm_inference_smoke_request_v1.json"
        ).read_bytes()
        inference_resp = request(
            "POST",
            f"{base}/v1/inference",
            inference_body,
            content_type="application/json",
        )
        check("/v1/inference", "POST", inference_resp)

        # Verify
        verify_resp = request(
            "POST",
            f"{base}/v1/verify?scope=default",
            b'VERIFY FACT "hello world" IN BRAIN default;',
        )
        check("/v1/verify", "POST", verify_resp)

        # Remember
        remember_resp = request(
            "POST",
            f"{base}/v1/remember?scope=default",
            b'REMEMBER "test memory" IN SCOPE default AS TYPE decision TTL 3600 SECONDS;',
        )
        check("/v1/remember", "POST", remember_resp)

        # Ingest
        ingest_resp = request(
            "POST",
            f"{base}/v1/ingest/text?scope=default&source=contract_test",
            b"hello world ingestion test",
        )
        check("/v1/ingest/text", "POST", ingest_resp)

        # Ingest jobs
        jobs_resp = request("GET", f"{base}/v1/ingest/jobs")
        schema = schema_for_response(spec, "/v1/ingest/jobs", "GET")
        if schema is None:
            errors.append("MISSING_SCHEMA GET /v1/ingest/jobs")
        else:
            try:
                validate(instance=jobs_resp, schema=schema, resolver=resolver)
            except Exception as exc:
                errors.append(f"VALIDATION_ERROR GET /v1/ingest/jobs: {exc}")

        # Metrics
        metrics_resp = request("GET", f"{base}/v1/metrics")
        check("/v1/metrics", "GET", metrics_resp)

        # Search explain
        explain_resp = request(
            "POST",
            f"{base}/v1/search/explain?scope=default&q=hello&limit=10",
            b"",
        )
        check("/v1/search/explain", "POST", explain_resp)

        # ANN evaluate
        ann_resp = request(
            "POST",
            f"{base}/v1/search/ann-evaluate?scope=default&vector=1,2,3&limit=10",
            b"",
        )
        check("/v1/search/ann-evaluate", "POST", ann_resp)

        # Flush / Compact
        flush_resp = request("POST", f"{base}/v1/flush", b"")
        check("/v1/flush", "POST", flush_resp)

        compact_resp = request("POST", f"{base}/v1/compact", b"")
        check("/v1/compact", "POST", compact_resp)

        if errors:
            print("CONTRACT CHECK FAILED:")
            for e in errors:
                print(f"  {e}")
            return 1

        print("OK: All live responses validate against OpenAPI schemas.")
        return 0
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except Exception:
            proc.kill()
        import shutil
        shutil.rmtree(tmpdir, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
