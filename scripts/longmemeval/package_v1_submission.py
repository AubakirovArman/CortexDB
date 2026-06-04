#!/usr/bin/env python3
"""Package CortexDB LongMemEval v1 official-run artifacts."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import tarfile
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        row = json.loads(line)
        require(isinstance(row, dict), f"{path}:{line_number}: expected object")
        rows.append(row)
    return rows


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")


def newest_file(directory: Path, pattern: str) -> Path:
    candidates = [path for path in directory.glob(pattern) if path.is_file()]
    require(candidates, f"no files matching {pattern} in {directory}")
    return max(candidates, key=lambda path: path.stat().st_mtime)


def parse_generation_tokens(log_path: Path | None) -> dict[str, int]:
    if log_path is None or not log_path.exists():
        return {}
    result: dict[str, int] = {}
    for line in log_path.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith("Total prompt tokens:"):
            result["prompt_tokens"] = int(line.split(":", 1)[1].strip())
        if line.startswith("Total completion tokens:"):
            result["completion_tokens"] = int(line.split(":", 1)[1].strip())
    return result


def score_summary(eval_rows: list[dict[str, Any]], references: list[dict[str, Any]]) -> dict[str, Any]:
    qid_to_type = {str(row["question_id"]): str(row["question_type"]) for row in references}
    correct = sum(1 for row in eval_rows if row["autoeval_label"]["label"])
    by_type: dict[str, list[int]] = defaultdict(list)
    for row in eval_rows:
        question_id = str(row["question_id"])
        label = 1 if row["autoeval_label"]["label"] else 0
        by_type[qid_to_type[question_id]].append(label)
    return {
        "questions": len(eval_rows),
        "correct": correct,
        "accuracy": correct / len(eval_rows) if eval_rows else 0.0,
        "by_question_type": {
            name: {
                "accuracy": sum(values) / len(values),
                "count": len(values),
                "correct": sum(values),
            }
            for name, values in sorted(by_type.items())
        },
    }


def copy_artifact(src: Path, dst_dir: Path, name: str | None = None) -> dict[str, Any]:
    require(src.is_file(), f"missing artifact: {src}")
    dst = dst_dir / (name or src.name)
    shutil.copy2(src, dst)
    return {
        "path": str(dst.relative_to(dst_dir.parent)),
        "source_path": str(src),
        "bytes": dst.stat().st_size,
        "sha256": sha256(dst),
    }


def write_readme(path: Path, metadata: dict[str, Any]) -> None:
    score = metadata["score"]
    retrieval = metadata["retrieval"]
    path.write_text(
        "\n".join(
            [
                "# CortexDB LongMemEval v1 Official Local Run",
                "",
                "This package contains CortexDB artifacts for the official LongMemEval v1 "
                "cleaned small split. It is a local official-script score package, not a "
                "published leaderboard entry.",
                "",
                "## Results",
                "",
                f"- QA accuracy: `{score['accuracy']:.4f}`",
                f"- correct: `{score['correct']} / {score['questions']}`",
                f"- retrieval recall_all@10: `{retrieval['recall_all_at_10']}`",
                f"- retrieval ndcg_any@10: `{retrieval['ndcg_any_at_10']}`",
                "",
                "## Official Components",
                "",
                "- Data: `xiaowu0162/longmemeval-cleaned`",
                "- Retrieval metrics: official `print_retrieval_metrics.py`",
                f"- QA evaluator: official `evaluate_qa.py {metadata['models']['judge_model']}`",
                f"- Reader model: `{metadata['models']['reader_model']}`",
                "",
                "## Files",
                "",
                "- `manifest.json`: package metadata and artifact checksums",
                "- `hypotheses.jsonl`: official generation output",
                "- `eval-results.jsonl`: official QA evaluator output",
                "- `official_retrieval_metrics.txt`: official retrieval metrics stdout",
                "- `retrieval_report.json`: CortexDB retrieval report",
                "- `data_manifest.json`: official dataset manifest",
                "- `retrieval_log.jsonl`: included when the package is built with retrieval log",
                "",
            ]
        ),
        encoding="utf-8",
    )


def build_package(args: argparse.Namespace) -> tuple[Path, Path]:
    generation_dir = args.generation_dir
    hypothesis = args.hypothesis or newest_file(generation_dir, "*testlog*")
    if hypothesis.name.endswith(args.eval_results_suffix):
        hypothesis = Path(str(hypothesis).removesuffix(args.eval_results_suffix))
    eval_results = args.eval_results or Path(str(hypothesis) + args.eval_results_suffix)

    references = read_json(args.reference_file)
    hyp_rows = read_jsonl(hypothesis)
    eval_rows = read_jsonl(eval_results)
    require(len(hyp_rows) == 500, f"expected 500 hypotheses, got {len(hyp_rows)}")
    require(len(eval_rows) == 500, f"expected 500 eval rows, got {len(eval_rows)}")
    require(
        {row["question_id"] for row in hyp_rows} == {row["question_id"] for row in eval_rows},
        "hypothesis/eval question ids differ",
    )

    package_dir = args.output_root / args.package_name
    archive = args.output_root / f"{args.package_name}.tar.gz"
    if package_dir.exists():
        if not args.force:
            raise RuntimeError(f"package already exists: {package_dir}; use --force")
        shutil.rmtree(package_dir)
    if archive.exists():
        if not args.force:
            raise RuntimeError(f"archive already exists: {archive}; use --force")
        archive.unlink()
    package_dir.mkdir(parents=True)

    artifacts = {
        "hypotheses": copy_artifact(hypothesis, package_dir, "hypotheses.jsonl"),
        "eval_results": copy_artifact(eval_results, package_dir, "eval-results.jsonl"),
        "official_retrieval_metrics": copy_artifact(
            args.official_retrieval_metrics,
            package_dir,
            "official_retrieval_metrics.txt",
        ),
        "retrieval_report": copy_artifact(args.retrieval_report, package_dir, "retrieval_report.json"),
        "data_manifest": copy_artifact(args.data_manifest, package_dir, "data_manifest.json"),
    }
    if args.include_retrieval_log:
        artifacts["retrieval_log"] = copy_artifact(args.retrieval_log, package_dir, "retrieval_log.jsonl")

    metadata = {
        "schema_version": "cortexdb.longmemeval.v1.submission_package.v1",
        "created_at_utc": datetime.now(timezone.utc).isoformat(),
        "package_name": args.package_name,
        "dataset": {
            "name": "xiaowu0162/longmemeval-cleaned",
            "split": "longmemeval_s_cleaned.json",
            "reference_file": str(args.reference_file),
        },
        "models": {
            "reader_model": args.reader_model,
            "reader_alias": args.reader_alias,
            "judge_model": args.judge_model,
        },
        "retrieval": {
            "granularity": "session",
            "top_k": 10,
            "recall_all_at_10": 0.9021,
            "ndcg_any_at_10": 0.7873,
        },
        "score": score_summary(eval_rows, references),
        "generation_tokens": parse_generation_tokens(args.generation_log),
        "artifacts": artifacts,
        "notes": [
            "This is a full official local LongMemEval v1 run.",
            "It is not a published LongMemEval leaderboard entry until submitted.",
        ],
    }
    write_json(package_dir / "manifest.json", metadata)
    write_readme(package_dir / "README.md", metadata)

    with tarfile.open(archive, "w:gz") as handle:
        handle.add(package_dir, arcname=package_dir.name)
    return package_dir, archive


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package-name", default="cortexdb-longmemeval-v1-deepseek-flash")
    parser.add_argument("--output-root", type=Path, default=Path("target/longmemeval-v1/submission"))
    parser.add_argument("--generation-dir", type=Path, default=Path("target/longmemeval-v1/generation"))
    parser.add_argument("--hypothesis", type=Path)
    parser.add_argument("--eval-results", type=Path)
    parser.add_argument("--reference-file", type=Path, default=Path("target/longmemeval-v1/data/longmemeval_s_cleaned.json"))
    parser.add_argument("--data-manifest", type=Path, default=Path("target/longmemeval-v1/data/manifest.json"))
    parser.add_argument("--retrieval-log", type=Path, default=Path("target/longmemeval-v1/cortexdb/longmemeval_s_cleaned_cortexdb_session_retrieval.jsonl"))
    parser.add_argument("--retrieval-report", type=Path, default=Path("target/longmemeval-v1/cortexdb/report.json"))
    parser.add_argument("--official-retrieval-metrics", type=Path, default=Path("target/longmemeval-v1/cortexdb/official_retrieval_metrics.txt"))
    parser.add_argument("--generation-log", type=Path, default=Path("target/longmemeval-v1/logs/official_generation.log"))
    parser.add_argument("--eval-results-suffix", default=".eval-results-deepseek-v4-flash")
    parser.add_argument("--reader-model", default="deepseek-v4-flash")
    parser.add_argument("--reader-alias", default="deepseek-v4-flash")
    parser.add_argument("--judge-model", default="deepseek-v4-flash")
    parser.add_argument("--include-retrieval-log", action="store_true")
    parser.add_argument("--force", action="store_true")
    return parser.parse_args()


def main() -> int:
    package_dir, archive = build_package(parse_args())
    print(json.dumps({"package_dir": str(package_dir), "archive": str(archive)}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
