#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

vectors=""
queries=""
ground_truth=""
metric="dot_product"
baseline_report=""
output_root="target/ann"
run_id=""
min_recall_q16="49151"
min_mean_recall_q16="49151"
max_p95_latency_nanos="100000000"
max_max_latency_nanos="250000000"
compare_max_p95_regression_nanos="0"
compare_max_max_regression_nanos="0"
allow_unsafe=0

usage() {
  cat <<'USAGE'
usage: run_external_corpus.sh --vectors PATH --queries PATH [options]

Required:
  --vectors PATH
  --queries PATH

Options:
  --ground-truth PATH
  --metric dot_product|cosine|l2
  --baseline-report PATH
  --output-root PATH
  --run-id VALUE
  --min-recall-q16 VALUE
  --min-mean-recall-q16 VALUE
  --max-p95-latency-nanos VALUE
  --max-max-latency-nanos VALUE
  --compare-max-p95-regression-nanos VALUE
  --compare-max-max-regression-nanos VALUE
  --allow-unsafe

If --ground-truth is omitted, exact ground truth is generated into the run
directory. If --baseline-report is provided, the new report is compared against
it and comparison.json is written.
USAGE
}

require_value() {
  local option="$1"
  local value="${2:-}"
  if [[ -z "${value}" ]]; then
    echo "${option} requires a value" >&2
    exit 2
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --vectors)
      require_value "$1" "${2:-}"
      vectors="$2"
      shift 2
      ;;
    --queries)
      require_value "$1" "${2:-}"
      queries="$2"
      shift 2
      ;;
    --ground-truth)
      require_value "$1" "${2:-}"
      ground_truth="$2"
      shift 2
      ;;
    --metric)
      require_value "$1" "${2:-}"
      metric="$2"
      shift 2
      ;;
    --baseline-report)
      require_value "$1" "${2:-}"
      baseline_report="$2"
      shift 2
      ;;
    --output-root)
      require_value "$1" "${2:-}"
      output_root="$2"
      shift 2
      ;;
    --run-id)
      require_value "$1" "${2:-}"
      run_id="$2"
      shift 2
      ;;
    --min-recall-q16)
      require_value "$1" "${2:-}"
      min_recall_q16="$2"
      shift 2
      ;;
    --min-mean-recall-q16)
      require_value "$1" "${2:-}"
      min_mean_recall_q16="$2"
      shift 2
      ;;
    --max-p95-latency-nanos)
      require_value "$1" "${2:-}"
      max_p95_latency_nanos="$2"
      shift 2
      ;;
    --max-max-latency-nanos)
      require_value "$1" "${2:-}"
      max_max_latency_nanos="$2"
      shift 2
      ;;
    --compare-max-p95-regression-nanos)
      require_value "$1" "${2:-}"
      compare_max_p95_regression_nanos="$2"
      shift 2
      ;;
    --compare-max-max-regression-nanos)
      require_value "$1" "${2:-}"
      compare_max_max_regression_nanos="$2"
      shift 2
      ;;
    --allow-unsafe)
      allow_unsafe=1
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "${vectors}" || -z "${queries}" ]]; then
  usage >&2
  exit 2
fi

if [[ -z "${run_id}" ]]; then
  timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
  git_sha="$(git -C "${REPO_ROOT}" rev-parse --short HEAD 2>/dev/null || echo nogit)"
  run_id="${timestamp}-${git_sha}"
fi

run_dir="${output_root%/}/${run_id}"
mkdir -p "${run_dir}"

if [[ -z "${ground_truth}" ]]; then
  ground_truth="${run_dir}/ground_truth.jsonl"
  python3 "${SCRIPT_DIR}/exact_ground_truth.py" \
    --vectors "${vectors}" \
    --queries "${queries}" \
    --metric "${metric}" \
    --output "${ground_truth}"
fi

report_path="${run_dir}/report.json"
manifest_path="${run_dir}/manifest.json"
comparison_path="${run_dir}/comparison.json"
history_path="${output_root%/}/history.json"

cat > "${manifest_path}" <<MANIFEST
{
  "run_id": "${run_id}",
  "git_sha": "$(git -C "${REPO_ROOT}" rev-parse HEAD 2>/dev/null || echo nogit)",
  "metric": "${metric}",
  "vectors": "${vectors}",
  "queries": "${queries}",
  "ground_truth": "${ground_truth}",
  "baseline_report": "${baseline_report}",
  "report": "${report_path}"
}
MANIFEST

ann_args=(
  --vectors "${vectors}"
  --queries "${queries}"
  --ground-truth "${ground_truth}"
  --metric "${metric}"
  --min-recall-q16 "${min_recall_q16}"
  --min-mean-recall-q16 "${min_mean_recall_q16}"
  --max-p95-latency-nanos "${max_p95_latency_nanos}"
  --max-max-latency-nanos "${max_max_latency_nanos}"
  --output "${report_path}"
)
if [[ "${allow_unsafe}" -eq 1 ]]; then
  ann_args+=(--allow-unsafe)
fi

cargo run --release -p cortex-engine --bin ann_corpus_check -- "${ann_args[@]}"

if [[ -n "${baseline_report}" ]]; then
  python3 "${SCRIPT_DIR}/compare_reports.py" \
    --baseline "${baseline_report}" \
    --candidate "${report_path}" \
    --max-p95-regression-nanos "${compare_max_p95_regression_nanos}" \
    --max-max-regression-nanos "${compare_max_max_regression_nanos}" \
    --output "${comparison_path}"
fi

python3 "${SCRIPT_DIR}/summarize_history.py" \
  --run-root "${output_root%/}" \
  --output "${history_path}"

echo "ANN corpus run written to ${run_dir}"
echo "ANN corpus history written to ${history_path}"
