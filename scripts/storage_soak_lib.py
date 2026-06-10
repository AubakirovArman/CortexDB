"""Storage soak runner used by scripts/storage_soak_check.py."""
from __future__ import annotations
import json
import shutil
import socket
import subprocess
import sys
import time
import urllib.request
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

@dataclass(frozen=True)
class SoakOptions:
    root: str
    report: str
    cycles: int
    cells_per_cycle: int
    kill_delay_ms: int
def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]

def run(cmd: list[str], repo: Path, *, timeout: float | None = None) -> str:
    result = subprocess.run(
        cmd,
        cwd=repo,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=timeout,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(f"command failed ({' '.join(cmd)}):\n{result.stdout}")
    return result.stdout

def run_json(cmd: list[str], repo: Path) -> dict[str, Any]:
    return json.loads(run(cmd, repo))
def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])

def cli(repo: Path) -> Path:
    return repo / "target/debug/cortexdb"
def server_bin(repo: Path) -> Path:
    return repo / "target/debug/cortex-server"
def put_cell(repo: Path, db: Path, cell_id: int, value: str) -> None:
    run([str(cli(repo)), "put", str(db), str(cell_id), value], repo)
def get_cell(repo: Path, db: Path, cell_id: int) -> str:
    return run([str(cli(repo)), "get", str(db), str(cell_id)], repo).rstrip("\n")
def validate_db(repo: Path, db: Path) -> dict[str, Any]:
    return run_json([str(cli(repo)), "--json", "validate", str(db)], repo)

def stats_db(repo: Path, db: Path) -> dict[str, Any]:
    return run_json([str(cli(repo)), "--json", "stats", str(db)], repo)

def payload(cell_id: int, cycle: int) -> str:
    return f"scope=soak\nstatus=ready\ncycle={cycle}\nsoak payload {cell_id}"
def backup_restore_cycle(repo: Path, db: Path, root: Path, cycle: int, expected: dict[int, str]) -> dict[str, Any]:
    backup_path = root / "backups" / f"cycle-{cycle}.tar"
    restore_path = root / "restores" / f"cycle-{cycle}"
    if restore_path.exists():
        shutil.rmtree(restore_path)
    backup_path.parent.mkdir(parents=True, exist_ok=True)
    restore_path.parent.mkdir(parents=True, exist_ok=True)
    run([str(cli(repo)), "backup", str(db), str(backup_path)], repo)
    run([str(cli(repo)), "restore", str(backup_path), str(restore_path)], repo)
    validation = validate_db(repo, restore_path)
    for cell_id, value in expected.items():
        if get_cell(repo, restore_path, cell_id) != value:
            raise AssertionError(f"restored cell {cell_id} mismatch")
    return {
        "cycle": cycle,
        "backup": str(backup_path),
        "restore": str(restore_path),
        "cells_verified": len(expected),
        "validation_ok": bool(validation.get("ok")),
    }

def repair_partial_wal(repo: Path, db: Path, cell_id: int) -> dict[str, Any]:
    wal = db / "db.aclog"
    before = wal.stat().st_size
    with wal.open("ab") as handle:
        handle.write(b"partial-soak-tail")
    run([str(cli(repo)), "repair", str(db)], repo)
    after = wal.stat().st_size
    return {
        "cell_id": cell_id,
        "wal_bytes_before": before,
        "wal_bytes_after_repair": after,
        "partial_tail_truncated": after == before,
        "validation_ok": bool(validate_db(repo, db).get("ok")),
    }

def start_server(repo: Path, db: Path, log_path: Path) -> tuple[subprocess.Popen[bytes], str]:
    port = free_port()
    log_path.parent.mkdir(parents=True, exist_ok=True)
    proc = subprocess.Popen(
        [str(server_bin(repo)), str(db), f"127.0.0.1:{port}"],
        cwd=repo,
        stdout=log_path.open("ab"),
        stderr=subprocess.STDOUT,
    )
    base_url = f"http://127.0.0.1:{port}"
    deadline = time.time() + 10
    while time.time() < deadline:
        if proc.poll() is not None:
            raise RuntimeError(f"server exited early with {proc.returncode}")
        try:
            urllib.request.urlopen(f"{base_url}/v1/health", timeout=1).read()
            return proc, base_url
        except Exception:
            time.sleep(0.05)
    raise TimeoutError("server did not become ready")

def kill_process(proc: subprocess.Popen[bytes]) -> bool:
    if proc.poll() is not None:
        return False
    proc.kill()
    proc.wait(timeout=10)
    return True

def unlock_repair_validate(repo: Path, db: Path) -> dict[str, Any]:
    run([str(cli(repo)), "unlock", str(db), "--force"], repo)
    repair_output = run([str(cli(repo)), "repair", str(db)], repo)
    return {
        "repair_output": repair_output.strip(),
        "validation_ok": bool(validate_db(repo, db).get("ok")),
    }

def kill_during_endpoint(repo: Path, db: Path, root: Path, name: str, endpoint: str, delay_ms: int) -> dict[str, Any]:
    proc, base_url = start_server(repo, db, root / f"{name}.server.log")
    url = f"{base_url}{endpoint}"
    requester = subprocess.Popen(
        [
            sys.executable,
            "-c",
            (
                "import sys, urllib.request; "
                "req=urllib.request.Request(sys.argv[1], method='POST'); "
                "urllib.request.urlopen(req, timeout=5).read()"
            ),
            url,
        ],
        cwd=repo,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    time.sleep(max(delay_ms, 0) / 1000)
    killed = kill_process(proc)
    output, _ = requester.communicate(timeout=10)
    return {
        "phase": name,
        "endpoint": endpoint,
        "server_killed": killed,
        "request_exit_code": requester.returncode,
        "request_output": output.decode("utf-8", errors="replace").strip(),
        **unlock_repair_validate(repo, db),
    }

def kill_during_wal_replay(repo: Path, db: Path, root: Path, delay_ms: int) -> dict[str, Any]:
    put_cell(repo, db, 900_001, "scope=soak\nstatus=ready\nwal replay sentinel")
    proc = subprocess.Popen(
        [str(server_bin(repo)), str(db), f"127.0.0.1:{free_port()}"],
        cwd=repo,
        stdout=(root / "wal-replay.server.log").open("ab"),
        stderr=subprocess.STDOUT,
    )
    time.sleep(max(delay_ms, 0) / 1000)
    killed = kill_process(proc)
    recovery = unlock_repair_validate(repo, db)
    sentinel = get_cell(repo, db, 900_001)
    return {
        "phase": "wal_replay",
        "server_killed": killed,
        "sentinel_readable": sentinel.endswith("wal replay sentinel"),
        **recovery,
    }

def kill_during_restore(repo: Path, db: Path, root: Path, delay_ms: int) -> dict[str, Any]:
    backup_path = root / "restore-kill.tar"
    partial_restore = root / "restore-kill-partial"
    final_restore = root / "restore-kill-final"
    for path in (partial_restore, final_restore):
        if path.exists():
            shutil.rmtree(path)
    partial_restore.parent.mkdir(parents=True, exist_ok=True)
    run([str(cli(repo)), "backup", str(db), str(backup_path)], repo)
    proc = subprocess.Popen(
        [str(cli(repo)), "restore", str(backup_path), str(partial_restore)],
        cwd=repo,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    time.sleep(max(delay_ms, 0) / 1000)
    killed = kill_process(proc)
    output, _ = proc.communicate(timeout=10)
    if partial_restore.exists():
        run([str(cli(repo)), "repair", str(partial_restore)], repo)
    run([str(cli(repo)), "restore", str(backup_path), str(final_restore)], repo)
    return {
        "phase": "restore",
        "restore_process_killed": killed,
        "partial_restore_exists": partial_restore.exists(),
        "restore_exit_code": proc.returncode,
        "restore_output": output.decode("utf-8", errors="replace").strip(),
        "final_restore_validation_ok": bool(validate_db(repo, final_restore).get("ok")),
    }

def run_versioned_fixture(repo: Path, root: Path) -> dict[str, Any]:
    fixture_path = repo / "fixtures/restore/v0.1.0-core-alpha/restore_fixture.json"
    fixture = json.loads(fixture_path.read_text(encoding="utf-8"))
    db = root / "versioned-fixture-db"
    restored = root / "versioned-fixture-restored"
    backup_path = root / "versioned-fixture.tar"
    for path in (db, restored):
        if path.exists():
            shutil.rmtree(path)
    restored.parent.mkdir(parents=True, exist_ok=True)
    for item in fixture["cells"]:
        put_cell(repo, db, int(item["cell_id"]), item["payload"])
    run([str(cli(repo)), "backup", str(db), str(backup_path)], repo)
    run([str(cli(repo)), "restore", str(backup_path), str(restored)], repo)
    for item in fixture["cells"]:
        if get_cell(repo, restored, int(item["cell_id"])) != item["payload"]:
            raise AssertionError(f"fixture cell {item['cell_id']} mismatch")
    return {
        "fixture": str(fixture_path),
        "release_tag": fixture["release_tag"],
        "cells_verified": len(fixture["cells"]),
        "validation_ok": bool(validate_db(repo, restored).get("ok")),
    }

def run_storage_soak(options: SoakOptions) -> dict[str, Any]:
    repo = repo_root()
    root = repo / options.root
    db = root / "db"
    if root.exists():
        shutil.rmtree(root)
    root.mkdir(parents=True)
    run(["cargo", "build", "-p", "cortex-cli", "--bin", "cortexdb", "-p", "cortex-server", "--bin", "cortex-server"], repo)
    expected: dict[int, str] = {}
    cycles = []
    started_at = utc_now()
    for cycle in range(1, options.cycles + 1):
        cycles.append(run_cycle(repo, root, db, cycle, options.cells_per_cycle, expected))
    kill_injections = [
        kill_during_endpoint(repo, db, root, "checkpoint", "/v1/flush", options.kill_delay_ms),
        kill_during_endpoint(repo, db, root, "compact", "/v1/compact", options.kill_delay_ms),
        kill_during_wal_replay(repo, db, root, options.kill_delay_ms),
        kill_during_restore(repo, db, root, options.kill_delay_ms),
    ]
    versioned_fixture = run_versioned_fixture(repo, root)
    final_pre_gc_stats = stats_db(repo, db)
    final_gc_output = run([str(cli(repo)), "gc-retired", str(db)], repo).strip()
    final_validation = validate_db(repo, db)
    final_stats = stats_db(repo, db)
    cycle_ok = all(c["validation_ok"] and c["backup_restore"]["validation_ok"] and c["partial_wal_repair"]["validation_ok"] and c["partial_wal_repair"]["partial_tail_truncated"] for c in cycles)
    kill_ok = all(item.get("validation_ok", item.get("final_restore_validation_ok", False)) and item.get("sentinel_readable", True) for item in kill_injections)
    status = "passed" if final_validation.get("ok") and versioned_fixture["validation_ok"] and cycle_ok and kill_ok else "failed"
    return {
        "schema_version": 1,
        "status": status,
        "started_at": started_at,
        "finished_at": utc_now(),
        "cycles_requested": options.cycles,
        "cells_per_cycle": options.cells_per_cycle,
        "cycles": cycles,
        "kill_injections": kill_injections,
        "versioned_restore_fixture": versioned_fixture,
        "final_pre_gc_stats": final_pre_gc_stats,
        "final_gc_output": final_gc_output,
        "final_validation": final_validation,
        "final_stats": final_stats,
        "tracked_outcomes": [
            "write/checkpoint/compact loops",
            "retired segment GC",
            "write/space amplification stats",
            "backup/restore loops",
            "partial WAL repair",
            "kill attempt during checkpoint",
            "kill attempt during compact",
            "kill attempt during WAL replay",
            "kill attempt during restore",
        ],
    }

def run_cycle(repo: Path, root: Path, db: Path, cycle: int, cells_per_cycle: int, expected: dict[int, str]) -> dict[str, Any]:
    cycle_cells = []
    for offset in range(cells_per_cycle):
        cell_id = cycle * 1000 + offset
        value = payload(cell_id, cycle)
        put_cell(repo, db, cell_id, value)
        expected[cell_id] = value
        cycle_cells.append(cell_id)
    run([str(cli(repo)), "flush", str(db)], repo)
    run([str(cli(repo)), "compact", str(db)], repo)
    validation = validate_db(repo, db)
    pre_gc_stats = stats_db(repo, db)
    backup_restore = backup_restore_cycle(repo, db, root, cycle, expected)
    partial_wal_repair = repair_partial_wal(repo, db, cycle_cells[-1])
    gc_output = run([str(cli(repo)), "gc-retired", str(db)], repo).strip()
    post_gc_stats = stats_db(repo, db)
    return {
        "cycle": cycle,
        "cells_written": cycle_cells,
        "total_cells_expected": len(expected),
        "validation_ok": bool(validation.get("ok")),
        "pre_gc_stats": pre_gc_stats,
        "backup_restore": backup_restore,
        "partial_wal_repair": partial_wal_repair,
        "gc_output": gc_output,
        "post_gc_stats": post_gc_stats,
    }
