#!/usr/bin/env python3
"""Prepare and optionally run a public ANN benchmark corpus."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest
import urllib.request
import zipfile
from pathlib import Path
from urllib.parse import urlparse


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_DIR = Path(__file__).resolve().parent


def is_url(value: str) -> bool:
    return urlparse(value).scheme in {"http", "https", "ftp", "file"}


def safe_members(tar: tarfile.TarFile, target: Path) -> list[tarfile.TarInfo]:
    result: list[tarfile.TarInfo] = []
    target_root = target.resolve()
    for member in tar.getmembers():
        destination = (target / member.name).resolve()
        if target_root not in destination.parents and destination != target_root:
            raise ValueError(f"archive member escapes target directory: {member.name}")
        result.append(member)
    return result


def safe_zip_members(archive: zipfile.ZipFile, target: Path) -> list[str]:
    result: list[str] = []
    target_root = target.resolve()
    for name in archive.namelist():
        destination = (target / name).resolve()
        if target_root not in destination.parents and destination != target_root:
            raise ValueError(f"archive member escapes target directory: {name}")
        result.append(name)
    return result


def extract_archive(archive_path: Path, target: Path) -> Path:
    target.mkdir(parents=True, exist_ok=True)
    if tarfile.is_tarfile(archive_path):
        with tarfile.open(archive_path) as tar:
            tar.extractall(target, members=safe_members(tar, target))
        return target
    if zipfile.is_zipfile(archive_path):
        with zipfile.ZipFile(archive_path) as archive:
            archive.extractall(target, members=safe_zip_members(archive, target))
        return target
    raise ValueError(f"unsupported archive format: {archive_path}")


def materialize_source(args: argparse.Namespace, work_dir: Path) -> Path:
    work_dir.mkdir(parents=True, exist_ok=True)
    if args.source_dir:
        return args.source_dir.resolve()
    archive_path = args.source_archive
    if args.source_url:
        archive_path = work_dir / Path(urlparse(args.source_url).path).name
        urllib.request.urlretrieve(args.source_url, archive_path)
    if archive_path is None:
        raise ValueError("one of --source-url, --source-archive, or --source-dir is required")
    return extract_archive(archive_path.resolve(), work_dir / "extracted")


def find_one(root: Path, explicit: str | None, patterns: list[str], label: str) -> Path:
    if explicit:
        path = Path(explicit)
        if path.exists():
            return path.resolve()
        candidate = root / explicit
        if candidate.exists():
            return candidate.resolve()
        raise ValueError(f"{label} not found: {explicit}")
    matches: list[Path] = []
    for pattern in patterns:
        matches.extend(root.rglob(pattern))
    matches = sorted({path.resolve() for path in matches if path.is_file()})
    if not matches:
        raise ValueError(f"could not discover {label}; pass an explicit path")
    return matches[0]


def build_convert_command(args: argparse.Namespace, source_root: Path, converted_dir: Path) -> list[str]:
    command = [sys.executable, str(SCRIPT_DIR / "convert_public_corpus.py")]
    if args.format == "fvecs":
        command += [
            "--vectors-fvecs",
            str(find_one(source_root, args.vectors, ["*base*.fvecs", "*vectors*.fvecs"], "vectors fvecs")),
            "--queries-fvecs",
            str(find_one(source_root, args.queries, ["*query*.fvecs", "*queries*.fvecs"], "queries fvecs")),
        ]
        try:
            truth = find_one(source_root, args.ground_truth, ["*groundtruth*.ivecs", "*truth*.ivecs"], "ground truth ivecs")
            command += ["--ground-truth-ivecs", str(truth), "--ground-truth-base", args.ground_truth_base]
        except ValueError:
            if args.ground_truth:
                raise
    else:
        command += [
            "--vectors-text",
            str(find_one(source_root, args.vectors, ["*vectors*.txt", "*base*.txt"], "vectors text")),
            "--queries-text",
            str(find_one(source_root, args.queries, ["*queries*.txt", "*query*.txt"], "queries text")),
        ]
    command += [
        "--output-dir",
        str(converted_dir),
        "--normalization",
        args.normalization,
        "--scale",
        str(args.scale),
        "--limit",
        str(args.limit),
    ]
    if args.max_vectors is not None:
        command += ["--max-vectors", str(args.max_vectors)]
    if args.max_queries is not None:
        command += ["--max-queries", str(args.max_queries)]
    return command


def run_command(command: list[str]) -> None:
    subprocess.run(command, cwd=REPO_ROOT, check=True)


def converted_truth_path(converted_dir: Path) -> Path | None:
    path = converted_dir / "ground_truth.jsonl"
    if not path.exists() or path.stat().st_size == 0:
        return None
    return path


def build_run_command(args: argparse.Namespace, converted_dir: Path) -> list[str]:
    command = [
        str(SCRIPT_DIR / "run_external_corpus.sh"),
        "--vectors",
        str(converted_dir / "vectors.jsonl"),
        "--queries",
        str(converted_dir / "queries.jsonl"),
        "--metric",
        args.metric,
        "--output-root",
        str(args.run_root),
        "--run-id",
        args.run_id,
        "--min-recall-q16",
        str(args.min_recall_q16),
        "--min-mean-recall-q16",
        str(args.min_mean_recall_q16),
        "--max-p95-latency-nanos",
        str(args.max_p95_latency_nanos),
        "--max-p99-latency-nanos",
        str(args.max_p99_latency_nanos),
        "--max-max-latency-nanos",
        str(args.max_max_latency_nanos),
        "--max-neighbors",
        str(args.max_neighbors),
        "--ef-search",
        str(args.ef_search),
        "--layer-count",
        str(args.layer_count),
    ]
    truth_path = converted_truth_path(converted_dir)
    if truth_path:
        command += ["--ground-truth", str(truth_path)]
    if args.allow_unsafe:
        command.append("--allow-unsafe")
    return command


def write_manifest(args: argparse.Namespace, public_dir: Path, source_root: Path, commands: list[list[str]]) -> None:
    manifest = {
        "dataset_id": args.dataset_id,
        "source_url": args.source_url or "",
        "source_archive": str(args.source_archive or ""),
        "source_dir": str(source_root),
        "format": args.format,
        "metric": args.metric,
        "converted_dir": str(public_dir / "converted"),
        "run_root": str(args.run_root),
        "run_id": args.run_id,
        "commands": commands,
    }
    (public_dir / "public_corpus_manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def prepare_and_run(args: argparse.Namespace) -> None:
    public_dir = args.output_root / args.dataset_id
    work_dir = public_dir / "source"
    converted_dir = public_dir / "converted"
    public_dir.mkdir(parents=True, exist_ok=True)
    if args.clean and public_dir.exists():
        shutil.rmtree(public_dir)
        public_dir.mkdir(parents=True)
    source_root = materialize_source(args, work_dir)
    convert_command = build_convert_command(args, source_root, converted_dir)
    run_command(convert_command)
    commands = [convert_command]
    if not args.no_run:
        run_command(build_run_command(args, converted_dir))
        commands.append(build_run_command(args, converted_dir))
    write_manifest(args, public_dir, source_root, commands)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group()
    source.add_argument("--source-url")
    source.add_argument("--source-archive", type=Path)
    source.add_argument("--source-dir", type=Path)
    parser.add_argument("--dataset-id", default="public-ann")
    parser.add_argument("--format", choices=["fvecs", "text"], default="fvecs")
    parser.add_argument("--vectors")
    parser.add_argument("--queries")
    parser.add_argument("--ground-truth")
    parser.add_argument("--ground-truth-base", choices=["zero", "one"], default="zero")
    parser.add_argument("--output-root", type=Path, default=Path("target/ann/public-corpora"))
    parser.add_argument("--run-root", type=Path, default=Path("target/ann/corpus-runs"))
    parser.add_argument("--run-id", default="public-ann")
    parser.add_argument("--metric", choices=["dot_product", "cosine", "l2"], default="cosine")
    parser.add_argument("--normalization", choices=["none", "unit", "max_abs"], default="unit")
    parser.add_argument("--scale", type=float, default=32767.0)
    parser.add_argument("--limit", type=int, default=10)
    parser.add_argument("--max-vectors", type=int)
    parser.add_argument("--max-queries", type=int)
    parser.add_argument("--min-recall-q16", type=int, default=49151)
    parser.add_argument("--min-mean-recall-q16", type=int, default=49151)
    parser.add_argument("--max-p95-latency-nanos", type=int, default=100_000_000)
    parser.add_argument("--max-p99-latency-nanos", type=int, default=200_000_000)
    parser.add_argument("--max-max-latency-nanos", type=int, default=250_000_000)
    parser.add_argument("--max-neighbors", type=int, default=8)
    parser.add_argument("--ef-search", type=int, default=64)
    parser.add_argument("--layer-count", type=int, default=4)
    parser.add_argument("--allow-unsafe", action="store_true")
    parser.add_argument("--clean", action="store_true")
    parser.add_argument("--no-run", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    prepare_and_run(parse_args(argv))
    return 0


class SelfTests(unittest.TestCase):
    def test_archive_discovery_and_conversion(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            corpus = root / "mini"
            corpus.mkdir()
            (corpus / "mini_base.fvecs").write_bytes(b"\x02\x00\x00\x00\x00\x00\x80?\x00\x00\x00\x00")
            (corpus / "mini_query.fvecs").write_bytes(b"\x02\x00\x00\x00\x00\x00\x80?\x00\x00\x00\x00")
            (corpus / "mini_groundtruth.ivecs").write_bytes(b"\x01\x00\x00\x00\x00\x00\x00\x00")
            archive = root / "mini.tar.gz"
            with tarfile.open(archive, "w:gz") as tar:
                tar.add(corpus, arcname="mini")
            args = parse_args([
                "--source-archive", str(archive),
                "--dataset-id", "mini",
                "--output-root", str(root / "out"),
                "--run-root", str(root / "runs"),
                "--run-id", "mini-run",
                "--no-run",
            ])
            prepare_and_run(args)
            self.assertTrue((root / "out" / "mini" / "converted" / "vectors.jsonl").exists())
            self.assertTrue((root / "out" / "mini" / "public_corpus_manifest.json").exists())

    def test_tar_path_traversal_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            archive = root / "bad.tar"
            payload = root / "payload.txt"
            payload.write_text("bad", encoding="utf-8")
            with tarfile.open(archive, "w") as tar:
                tar.add(payload, arcname="../escape.txt")
            with self.assertRaises(ValueError):
                extract_archive(archive, root / "out")


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
