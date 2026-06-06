# LoCoMo Adapter

Status: retrieval-only adapter evidence is available for the official SNAP
Research LoCoMo dataset. This is not an end-to-end QA score and not a published
leaderboard entry.

## Official Source

The adapter uses:

```text
dataset: snap-research/locomo
file: data/locomo10.json
```

The official dataset contains 10 long-term conversations with conversation
turns, observations, session summaries, event summaries, and QA annotations.
CortexDB currently uses the conversation turns plus QA evidence dialog IDs for a
retrieval-only gate.

## Commands

Download and validate the official data:

```bash
make locomo-official-data
```

Run the CortexDB retrieval adapter and validate the evidence bundle:

```bash
make locomo-retrieval-adapter-check
```

The command writes:

```text
target/locomo/data/locomo10.json
target/locomo/data/manifest.json
target/locomo/retrieval/cortexdb_locomo_retrieval.jsonl
target/locomo/retrieval/cortexdb_locomo_report.json
target/locomo/retrieval-adapter/report.json
```

## Current Local Evidence

Latest full local run:

```text
schema: cortexdb.locomo.retrieval_adapter_check.v1
status: passed
samples: 10
turns_indexed: 5,882
questions: 1,986
questions_with_evidence: 1,982
hit@1: 0.3199
hit@10: 0.6312
```

By category:

| Category | Questions | hit@10 |
| --- | ---: | ---: |
| `1` | `282` | `0.4894` |
| `2` | `321` | `0.6760` |
| `3` | `92` | `0.3478` |
| `4` | `841` | `0.6742` |
| `5` | `446` | `0.6659` |

## Claim Boundary

This adapter proves:

- CortexDB can ingest LoCoMo conversational turns as durable cells.
- CortexDB can run retrieval-only evaluation against LoCoMo QA evidence IDs.
- The adapter can produce repeatable local evidence without API keys.

It does not claim:

- official leaderboard placement;
- end-to-end answer accuracy;
- LLM judge accuracy;
- production conversational-memory quality.

Optional E2E evaluation should be added as a separate gate once a submission
policy and model budget are explicit.
