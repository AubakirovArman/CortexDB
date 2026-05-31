# ContextPack Quality Evidence

Last local ContextPack quality run: 2026-05-31, passed.

Run:

```bash
make context-pack-quality-check
```

Primary artifacts:

```text
examples/eval/context_pack_quality.jsonl
target/context-pack-quality/report.json
```

## Latest Local Metrics

```text
case_count: 4
evidence_coverage_q16: 65535
token_reduction_q16: 36464
citation_coverage_q16: 65535
redundancy_reduction_q16: 65535
deterministic_order_q16: 65535
```

## Boundary

This gate proves:

- ContextPack behavior tests pass for budget truncation, required citations,
  source refs, sparse/dense duplicate suppression, deterministic ordering, and
  explain fields;
- the real-domain investment-project fixture produces cited selected cells;
- the quality fixture records measurable evidence coverage, token reduction,
  citation coverage, redundancy reduction, and deterministic ordering.

This gate does not prove:

- answer quality from an external LLM;
- dense semantic reranking quality beyond the deterministic local vector
  redundancy checks;
- private customer-domain evidence quality.
