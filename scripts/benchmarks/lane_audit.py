#!/usr/bin/env python3
"""F1.3: CI lane-map audit.

A CI gate is only useful if it actually runs somewhere. This audit enforces that
the machine-readable lane map (fixtures/benchmarks/lanes.v1.json) and the real
GitHub Actions workflow it names agree, so a gate can never be silently defined
but never scheduled (or scheduled but undocumented):

  1. every gate target in the map is a real make target (`^<target>:` in mk/*.mk
     or the root Makefile);
  2. no gate target appears in two lanes;
  3. bidirectional agreement for the `benchmark-validation` lane: every
     `make <target>` line in that workflow job is a mapped gate of that lane, and
     every mapped gate of that lane actually appears as a `make <target>` line in
     that job. Any drift in either direction fails the gate.

Dependency-free (stdlib only); deterministic; no network, no wall clock.
"""

from __future__ import annotations

import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent.parent
LANES = REPO / "fixtures" / "benchmarks" / "lanes.v1.json"


def make_targets() -> set[str]:
    """All target names defined across mk/*.mk and the root Makefile."""
    targets: set[str] = set()
    files = list((REPO / "mk").glob("*.mk"))
    root = REPO / "Makefile"
    if root.exists():
        files.append(root)
    target_re = re.compile(r"^([A-Za-z0-9][A-Za-z0-9._-]*)\s*:(?!=)")
    for f in files:
        for line in f.read_text().splitlines():
            m = target_re.match(line)
            if m:
                targets.add(m.group(1))
    return targets


def workflow_make_calls(job_lines: list[str]) -> list[str]:
    """`make <target>` invocations within a block of workflow YAML lines."""
    calls: list[str] = []
    call_re = re.compile(r"\bmake\s+([A-Za-z0-9][A-Za-z0-9._-]*)")
    for line in job_lines:
        for m in call_re.finditer(line):
            calls.append(m.group(1))
    return calls


def job_block(workflow_text: str, job: str) -> list[str]:
    """Lines belonging to a top-level job (until the next equally-indented job)."""
    lines = workflow_text.splitlines()
    out: list[str] = []
    in_job = False
    job_indent = None
    header = re.compile(r"^(\s{2})([A-Za-z0-9_-]+):\s*$")
    for line in lines:
        m = header.match(line)
        if m and m.group(2) == job and not in_job:
            in_job = True
            job_indent = len(m.group(1))
            continue
        if in_job:
            # A new job at the same indent ends this block.
            m2 = header.match(line)
            if m2 and len(m2.group(1)) == job_indent:
                break
            out.append(line)
    return out


def main() -> int:
    report_path = None
    args = sys.argv[1:]
    for i, a in enumerate(args):
        if a == "--report" and i + 1 < len(args):
            report_path = pathlib.Path(args[i + 1])

    spec = json.loads(LANES.read_text())
    targets = make_targets()
    errors: list[str] = []

    # (1) every gate target is a real make target; (2) no target in two lanes.
    seen: dict[str, str] = {}
    for lane in spec["lanes"]:
        for gate in lane["gates"]:
            t = gate["target"]
            if t not in targets:
                errors.append(f"lane {lane['lane']}: target '{t}' is not a make target")
            if t in seen and seen[t] != lane["lane"]:
                errors.append(f"target '{t}' appears in two lanes: {seen[t]} and {lane['lane']}")
            seen[t] = lane["lane"]

    # (3) bidirectional agreement for the benchmark-validation lane.
    workflow = REPO / spec["workflow"]
    bv = next((l for l in spec["lanes"] if l["lane"] == "benchmark-validation"), None)
    if not workflow.exists():
        errors.append(f"workflow {spec['workflow']} does not exist")
    elif bv is not None:
        block = job_block(workflow.read_text(), bv["workflow_job"])
        if not block:
            errors.append(f"workflow job '{bv['workflow_job']}' not found in {spec['workflow']}")
        run_targets = set(workflow_make_calls(block))
        mapped = {g["target"] for g in bv["gates"]}
        for t in sorted(mapped - run_targets):
            errors.append(f"benchmark-validation: gate '{t}' is mapped but not run in the workflow job")
        for t in sorted(run_targets - mapped):
            errors.append(f"benchmark-validation: '{t}' runs in the workflow job but is not mapped")

    total_gates = sum(len(l["gates"]) for l in spec["lanes"])
    passed = not errors
    report = {
        "schema_version": "cortexdb.ci_lane_audit.v1",
        "status": "passed" if passed else "failed",
        "lanes": len(spec["lanes"]),
        "gates": total_gates,
        "errors": errors,
    }
    if report_path is not None:
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(json.dumps(report, indent=2) + "\n")

    if not passed:
        print("benchmark-lane-audit FAILED")
        for e in errors:
            print(f"  {e}")
        return 1
    print(
        f"benchmark-lane-audit passed: {total_gates} gate(s) across {len(spec['lanes'])} lane(s); "
        f"benchmark-validation lane agrees with {spec['workflow']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
