#!/usr/bin/env python3
"""Tenant isolation plus backup/restore recovery evidence gate."""

from __future__ import annotations

import argparse
import json
import shutil
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any


TENANT_PAYLOADS = {
    "default": "scope=default\nstatus=ready\n\ndefault tenant payload",
    "tenant-alpha": "scope=alpha\nstatus=ready\n\nalpha tenant payload",
    "tenant-beta": "scope=beta\nstatus=ready\n\nbeta tenant payload",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--server", required=True, help="Path to cortex-server binary")
    parser.add_argument("--cli", required=True, help="Path to cortexdb CLI binary")
    parser.add_argument("--root", default="target/tenant-recovery", help="Working root")
    parser.add_argument("--report", default="target/tenant-recovery/report.json")
    return parser.parse_args()


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def request_json(method: str, url: str, body: bytes | None = None) -> dict[str, Any]:
    req = urllib.request.Request(url, data=body, method=method)
    if body is not None:
        req.add_header("Content-Type", "text/plain")
    with urllib.request.urlopen(req, timeout=10) as response:
        return json.loads(response.read().decode("utf-8"))


def request_error_json(method: str, url: str) -> dict[str, Any]:
    try:
        request_json(method, url)
    except urllib.error.HTTPError as error:
        return json.loads(error.read().decode("utf-8"))
    raise AssertionError(f"expected HTTP error for {url}")


def tenant_url(base_url: str, path: str, tenant: str, **query: str) -> str:
    params = dict(query)
    if tenant != "default":
        params["tenant"] = tenant
    suffix = urllib.parse.urlencode(params)
    return f"{base_url}{path}?{suffix}" if suffix else f"{base_url}{path}"


def wait_for_health(base_url: str, tenant: str = "default") -> None:
    deadline = time.time() + 10
    last_error: Exception | None = None
    while time.time() < deadline:
        try:
            health = request_json("GET", tenant_url(base_url, "/v1/health", tenant))
            if health.get("status") == "ok":
                return
        except Exception as error:  # pragma: no cover - readiness loop
            last_error = error
        time.sleep(0.1)
    raise RuntimeError(f"server did not become healthy: {last_error}")


def start_server(server: Path, db_root: Path, log_path: Path) -> tuple[subprocess.Popen, str]:
    port = free_port()
    base_url = f"http://127.0.0.1:{port}"
    log_path.parent.mkdir(parents=True, exist_ok=True)
    log = log_path.open("wb")
    process = subprocess.Popen(
        [str(server), str(db_root), f"127.0.0.1:{port}"],
        stdout=log,
        stderr=subprocess.STDOUT,
    )
    wait_for_health(base_url)
    return process, base_url


def stop_server(process: subprocess.Popen) -> None:
    process.terminate()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:  # pragma: no cover
        process.kill()
        process.wait(timeout=5)


def put_cell(base_url: str, tenant: str, payload: str) -> None:
    response = request_json(
        "POST",
        tenant_url(base_url, "/v1/cell", tenant, cell_id="1"),
        payload.encode("utf-8"),
    )
    if response.get("cell_id") != 1:
        raise AssertionError(f"unexpected put response for {tenant}: {response}")


def flush_and_validate(base_url: str, tenant: str) -> dict[str, Any]:
    flush = request_json("POST", tenant_url(base_url, "/v1/flush", tenant))
    validation = request_json("GET", tenant_url(base_url, "/v1/validate", tenant))
    if validation.get("ok") is not True:
        raise AssertionError(f"validation failed for {tenant}: {validation}")
    return {"flush": flush, "validation": validation}


def read_payload(base_url: str, tenant: str) -> str:
    response = request_json("GET", tenant_url(base_url, "/v1/cell", tenant, cell_id="1"))
    cell = response.get("cell")
    if not isinstance(cell, dict):
        raise AssertionError(f"missing cell for {tenant}: {response}")
    payload = cell.get("payload")
    if not isinstance(payload, str):
        raise AssertionError(f"invalid cell payload for {tenant}: {response}")
    return payload


def verify_tenants(base_url: str) -> dict[str, Any]:
    reads = {tenant: read_payload(base_url, tenant) for tenant in TENANT_PAYLOADS}
    errors = []
    for tenant, expected in TENANT_PAYLOADS.items():
        if reads[tenant] != expected:
            errors.append(f"{tenant} read mismatch")
    if len(set(reads.values())) != len(TENANT_PAYLOADS):
        errors.append("tenant payloads are not isolated")
    invalid = request_error_json(
        "GET",
        f"{base_url}/v1/health?tenant={urllib.parse.quote('../escape')}",
    )
    if invalid.get("code") != "invalid_tenant":
        errors.append(f"invalid tenant did not fail closed: {invalid}")
    if errors:
        raise AssertionError("; ".join(errors))
    return {"reads": reads, "invalid_tenant": invalid}


def run_cli(cli: Path, *args: str) -> str:
    result = subprocess.run(
        [str(cli), *args],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return result.stdout.strip()


def main() -> int:
    args = parse_args()
    server = Path(args.server)
    cli = Path(args.cli)
    root = Path(args.root)
    report_path = Path(args.report)
    if not server.exists() or not cli.exists():
        print("server and cli binaries must exist", file=sys.stderr)
        return 2

    shutil.rmtree(root, ignore_errors=True)
    source_root = root / "source"
    backup_root = root / "backup"
    restored_root = root / "restored"
    report: dict[str, Any] = {
        "schema_version": "cortexdb.tenant_recovery.v1",
        "status": "failed",
        "source_root": str(source_root),
        "backup_root": str(backup_root),
        "restored_root": str(restored_root),
        "checks": {},
    }
    errors: list[str] = []

    process: subprocess.Popen | None = None
    try:
        process, base_url = start_server(server, source_root, root / "source-server.log")
        for tenant, payload in TENANT_PAYLOADS.items():
            put_cell(base_url, tenant, payload)
        report["checks"]["source_flush_validate"] = {
            tenant: flush_and_validate(base_url, tenant) for tenant in TENANT_PAYLOADS
        }
        report["checks"]["source_isolation"] = verify_tenants(base_url)
    except Exception as error:
        errors.append(f"source tenant check failed: {error}")
    finally:
        if process is not None:
            stop_server(process)

    if not errors:
        try:
            report["checks"]["stale_unlock"] = run_cli(cli, "unlock", str(source_root), "--force")
            report["checks"]["backup"] = run_cli(cli, "backup", str(source_root), str(backup_root))
            report["checks"]["restore"] = run_cli(cli, "restore", str(backup_root), str(restored_root))
        except Exception as error:
            errors.append(f"backup/restore failed: {error}")

    process = None
    if not errors:
        try:
            process, restored_url = start_server(server, restored_root, root / "restored-server.log")
            report["checks"]["restored_isolation"] = verify_tenants(restored_url)
            report["checks"]["restored_validation"] = {
                tenant: request_json("GET", tenant_url(restored_url, "/v1/validate", tenant))
                for tenant in TENANT_PAYLOADS
            }
        except Exception as error:
            errors.append(f"restored tenant check failed: {error}")
        finally:
            if process is not None:
                stop_server(process)

    report["errors"] = errors
    report["status"] = "passed" if not errors else "failed"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if errors:
        print("TENANT RECOVERY CHECK FAILED:")
        for error in errors:
            print(f"  {error}")
        print(f"report: {report_path}")
        return 1
    print(f"tenant recovery check passed: {report_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
