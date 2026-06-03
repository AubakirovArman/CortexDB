# MultiHop-RAG Benchmark Plan

Status: reproducible local benchmark scaffold, not a published public result.

MultiHop-RAG is the next benchmark layer after LongMemEval for CortexDB. It
tests whether retrieval can gather evidence from multiple documents instead of
returning one similar chunk. The official dataset contains 2556 queries and
evidence distributed across 2 to 4 documents, with inference, comparison,
temporal, and null-query cases.

Official references:

- GitHub: <https://github.com/yixuantt/MultiHop-RAG>
- Hugging Face dataset: <https://huggingface.co/datasets/yixuantt/MultiHopRAG>
- OpenReview / COLM 2024: <https://openreview.net/forum?id=t4eB3zYWBK>

The official repository currently has a leaderboard section marked "Coming
soon", so CortexDB should publish reproducible local artifacts first and only
claim leaderboard inclusion after maintainers provide a submission path.

## Why This Matters

LongMemEval checks long-term memory over sessions. MultiHop-RAG checks a
different risk: whether the retrieval layer can collect several supporting
facts across documents before the answer is generated.

The useful CortexDB signal is:

```text
AQL / search / ANN retrieval
-> top-k multi-document evidence
-> official retrieval metrics
-> optional QA generation
-> official QA metrics
```

## Local Gate First

Do not start with the full 2556-query run when tuning. Use the local 50-query
gate first:

```bash
make multihop-rag-local-50-check
```

That target downloads the official JSON files, validates them, and creates a
balanced 50-query subset under:

```text
target/multihop-rag/subsets/balanced_50/
```

The generated files are:

```text
balanced_50_multihop.json
balanced_50_queries.jsonl
balanced_50_ground_truth.jsonl
balanced_50_subset_report.json
```

This is the same workflow rule as LongMemEval tuning: improve on 50 cases first,
then promote to the full benchmark only when the focused gate improves.

## Official Full-Run Path

After the 50-query gate is stable, run the full official data preparation:

```bash
make multihop-rag-preflight
```

Then build a CortexDB retrieval output compatible with the official evaluator:

```json
[
  {
    "query": "...",
    "answer": "...",
    "question_type": "inference_query",
    "evidence_list": [{"fact": "..."}],
    "retrieval_list": [{"text": "..."}]
  }
]
```

The official retrieval evaluator reports:

```text
Hits@10
Hits@4
MAP@10
MRR@10
```

QA generation can then be scored with the official `qa_evaluate.py`, which
reports Exact Match and F1.

## Required Artifact Set

Every public CortexDB MultiHop-RAG run must archive:

- official dataset source and manifest;
- CortexDB retrieval config;
- embedding model and endpoint family;
- top-k and chunking settings;
- raw retrieval output JSON;
- official retrieval evaluator output;
- generation model and prompt, if QA is run;
- raw QA output;
- official QA evaluator output;
- a short report with non-claims.

## Non-Claims

Until a full official run is completed and archived, CortexDB must not claim:

- MultiHop-RAG leaderboard placement;
- production multi-hop QA quality;
- superiority over other RAG systems;
- official maintainer endorsement.

Current status is only:

```text
MultiHop-RAG scaffold exists.
Local 50-query gate can prepare reproducible subsets.
Full benchmark execution remains a next action.
```
