#!/usr/bin/env bash
# Official-clean (no-oracle) dense+lexical hybrid run, end to end:
#   prepare -> cached-lexical retrieve -> dense/RRF fuse -> answer -> judge.
#
# Produces the leaderboard-comparable number. NOTE: the official metric is the
# PER-QUESTION mean over all questions == the "proportional" overall this script
# prints, NOT the per-type "uniform" mean.
#
# Usage:
#   scripts/enterprise_rag_bench/run_dense_hybrid_clean.sh
# Override via env vars:
#   SIZE=500 QUESTIONS_FILE=target/external-benchmarks/EnterpriseRAG-Bench/questions.jsonl \
#   RUN_LABEL=full500-dense-hybrid DENSE_WEIGHT=1.0 LEX_WEIGHT=1.0 \
#   scripts/enterprise_rag_bench/run_dense_hybrid_clean.sh
set -euo pipefail
cd "$(dirname "$0")/../.."

SIZE="${SIZE:-50}"
QUESTIONS_FILE="${QUESTIONS_FILE:-target/external-benchmarks/EnterpriseRAG-Bench/questions.jsonl}"
RUN_LABEL="${RUN_LABEL:-dense-hybrid}"
ANSWER_PROVIDER="${ANSWER_PROVIDER:-gemma}"
JUDGE_PROVIDER="${JUDGE_PROVIDER:-gemma}"
# DeepSeek-specific model/thinking selection. Exported so the answer/judge
# sub-processes (and the official_clean.py profile helper) see the override.
DEEPSEEK_MODEL="${DEEPSEEK_MODEL:-deepseek-v4-flash}"
DEEPSEEK_THINKING="${DEEPSEEK_THINKING:-disabled}"
if [ "$ANSWER_PROVIDER" = "deepseek" ] || [ "$JUDGE_PROVIDER" = "deepseek" ]; then
  export DEEPSEEK_MODEL DEEPSEEK_THINKING
fi
CORPUS="${CORPUS:-target/enterprise-rag-bench/embeddings/corpus_bge_m3.jsonl}"
DB="${DB:-target/enterprise-rag-bench/official-clean/50/cortexdb}"   # reused, fully-indexed corpus DB
DENSE_TOP_K="${DENSE_TOP_K:-200}"
TOP_K="${TOP_K:-50}"
DENSE_WEIGHT="${DENSE_WEIGHT:-1.0}"
LEX_WEIGHT="${LEX_WEIGHT:-1.0}"
TOP_K_CONTEXT="${TOP_K_CONTEXT:-8}"
MAX_TOKENS="${MAX_TOKENS:-1500}"
ANSWER_WORKERS="${ANSWER_WORKERS:-4}"
JUDGE_WORKERS="${JUDGE_WORKERS:-4}"

# Answer-layer knobs (new: evidence-first prompt, slot plan, evidence table, guard mode).
PROMPT_STYLE="${PROMPT_STYLE:-official-clean-v1}"
CONTEXT_MODE="${CONTEXT_MODE:-full-doc}"
MAX_CHARS_PER_DOC="${MAX_CHARS_PER_DOC:-8000}"
# DeepSeek is much more literal than Gemma: the default digest+window snippet
# often drops the exact sentence that contains the answer, so it falls back to
# "Insufficient information." Full-doc is already the default above, but keep
# the explicit override in case a user overrides CONTEXT_MODE externally.
if [ "$ANSWER_PROVIDER" = "deepseek" ]; then
  CONTEXT_MODE="${CONTEXT_MODE:-full-doc}"
  MAX_CHARS_PER_DOC="${MAX_CHARS_PER_DOC:-8000}"
fi
GEMINI_THINKING_BUDGET="${GEMINI_THINKING_BUDGET:-0}"
INCLUDE_EVIDENCE_PLAN="${INCLUDE_EVIDENCE_PLAN:-0}"
INCLUDE_EVIDENCE_TABLE="${INCLUDE_EVIDENCE_TABLE:-0}"
UNSUPPORTED_CLAIM_GUARD="${UNSUPPORTED_CLAIM_GUARD:-off}"
SELF_CONSISTENCY_REPAIR="${SELF_CONSISTENCY_REPAIR:-0}"
COMPANY_SCOPE_ROUTE="${COMPANY_SCOPE_ROUTE:-1}"
ORACLE_FREE_ABSTAIN="${ORACLE_FREE_ABSTAIN:-1}"

BASE="target/enterprise-rag-bench/official-clean/${SIZE}/${RUN_LABEL}"
ANSWER_FLAGS=(--prompt-style "$PROMPT_STYLE" --context-mode "$CONTEXT_MODE" --gemini-thinking-budget "$GEMINI_THINKING_BUDGET")
[ "$INCLUDE_EVIDENCE_PLAN" = 1 ] && ANSWER_FLAGS+=(--include-evidence-plan)
[ "$INCLUDE_EVIDENCE_TABLE" = 1 ] && ANSWER_FLAGS+=(--include-evidence-table)
[ "$UNSUPPORTED_CLAIM_GUARD" != off ] && ANSWER_FLAGS+=(--unsupported-claim-guard "$UNSUPPORTED_CLAIM_GUARD")
[ "$SELF_CONSISTENCY_REPAIR" = 1 ] && ANSWER_FLAGS+=(--self-consistency-repair)
COMMON=(--size "$SIZE" --questions-file "$QUESTIONS_FILE" --split-name clean
        --run-label "$RUN_LABEL" --answer-provider "$ANSWER_PROVIDER"
        --judge-provider "$JUDGE_PROVIDER" --db-root "$DB" --reuse-db)

rm -f "${DB}/db.lock" || true
echo "### config SIZE=$SIZE RUN_LABEL=$RUN_LABEL fusion(lex=$LEX_WEIGHT,dense=$DENSE_WEIGHT) dense_top_k=$DENSE_TOP_K top_k=$TOP_K oracle_free_abstain=$ORACLE_FREE_ABSTAIN company_scope=$COMPANY_SCOPE_ROUTE"

echo "### STAGE 1/4: prepare + cached-lexical retrieve (top-$TOP_K)"
python3 scripts/enterprise_rag_bench/run_official_clean_benchmark.py "${COMMON[@]}" \
  --stage retrieval --retrieval-mode cached-lexical --top-k "$TOP_K"

echo "### STAGE 2/4: dense + RRF hybrid fuse -> clean_retrieval"
python3 scripts/enterprise_rag_bench/dense_candidates.py \
  --questions "$BASE/questions.clean.jsonl" --corpus-vectors "$CORPUS" \
  --lexical-retrieval "$BASE/retrieval.clean.jsonl" --output "$BASE/retrieval.clean.jsonl" \
  --query-cache "$BASE/query_vectors.jsonl" --env-file .env \
  --dense-top-k "$DENSE_TOP_K" --top-k "$TOP_K" \
  --dense-weight "$DENSE_WEIGHT" --lexical-weight "$LEX_WEIGHT" \
  --report "$BASE/dense_report.json"

if [ "$COMPANY_SCOPE_ROUTE" = 1 ]; then
  echo "### STAGE 2b/4: company-scope dense fallback for high-level zero-doc questions"
  python3 scripts/enterprise_rag_bench/company_scope_route.py \
    --questions-file "$BASE/questions.clean.jsonl" \
    --retrieval-file "$BASE/retrieval.clean.jsonl" \
    --output "$BASE/retrieval.clean.jsonl" \
    --corpus-vectors "$CORPUS" --env-file .env \
    --query-cache "$BASE/company_scope_query_vectors.jsonl" \
    --top-k "$TOP_K_CONTEXT" \
    --report "$BASE/company_scope_report.json"
fi

if [ "$ORACLE_FREE_ABSTAIN" = 1 ]; then
  echo "### STAGE 2c/4: oracle-free abstention post-process"
  python3 scripts/enterprise_rag_bench/postprocess_retrieval_modes.py \
    --questions-file "$BASE/questions.clean.jsonl" \
    --retrieval-file "$BASE/retrieval.clean.jsonl" \
    --output "$BASE/retrieval.clean.jsonl" \
    --report "$BASE/abstain_postprocess_report.json" \
    --policy-name oracle_free_abstain \
    --oracle-free-abstain
fi

echo "### STAGE 3/4: ${ANSWER_PROVIDER} answers (prompt=${PROMPT_STYLE} context=${CONTEXT_MODE} plan=${INCLUDE_EVIDENCE_PLAN} table=${INCLUDE_EVIDENCE_TABLE} guard=${UNSUPPORTED_CLAIM_GUARD} repair=${SELF_CONSISTENCY_REPAIR})"
python3 scripts/enterprise_rag_bench/run_official_clean_benchmark.py "${COMMON[@]}" \
  --stage answer --top-k-context "$TOP_K_CONTEXT" --max-chars-per-doc "$MAX_CHARS_PER_DOC" \
  --max-tokens "$MAX_TOKENS" --answer-workers "$ANSWER_WORKERS" \
  "${ANSWER_FLAGS[@]}"

echo "### STAGE 4/4: ${JUDGE_PROVIDER} judge"
python3 scripts/enterprise_rag_bench/run_official_clean_benchmark.py "${COMMON[@]}" \
  --stage judge --judge-workers "$JUDGE_WORKERS" --judge-timeout-seconds 180

echo "### RESULT"
python3 - "$BASE/answer-${ANSWER_PROVIDER}/judge-${JUDGE_PROVIDER}/results.json" "$QUESTIONS_FILE" <<'PY'
import json, sys, collections
res, qfile = sys.argv[1], sys.argv[2]
d = json.load(open(res)); a = d["aggregate_stats"]; ts = d["question_type_stats"]
# official type counts (for the proportional / leaderboard-comparable number)
counts = collections.Counter(json.loads(l).get("question_type") for l in open(qfile))
num = sum(ts[t]["combined_correctness_completeness_score"] * counts.get(t, 0) for t in ts)
den = sum(counts.get(t, 0) for t in ts)
prop = num / den if den else 0.0
print(f"questions scored : {a['completed_questions']}/{a['total_questions']}")
print(f"OVERALL uniform  : {a['combined_correctness_completeness_score']:.2f}  (per-type mean; NOT leaderboard)")
print(f"OVERALL proper   : {prop:.2f}  (per-question mean == leaderboard-comparable)")
print(f"correctness/compl: {a['average_correctness_pct']:.1f} / {a['average_completeness_pct']:.1f}  recall: {a['average_recall_pct']:.1f}")
print(f"{'type':24} {'n':>4} {'corr':>5} {'compl':>6} {'combined':>9}")
for t, s in sorted(ts.items(), key=lambda x: -counts.get(x[0], 0)):
    print(f"{t:24} {counts.get(t,0):>4} {s['average_correctness_pct']:>5.0f} {s['average_completeness_pct']:>6.0f} {s['combined_correctness_completeness_score']:>9.1f}")
PY
echo "### DONE -> $BASE"
