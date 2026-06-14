# AQL EXPLAIN ANALYZE

`EXPLAIN ANALYZE` executes a `RETRIEVE CONTEXT` statement and returns the same
logical, policy-rewritten, cost-model, bitmap, and candidate-count report as
`EXPLAIN`, plus physical operator counters and elapsed time.

## CLI

Existing AQL syntax still works:

```bash
cortexdb aql ./db project:investments \
  'EXPLAIN ANALYZE RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN default LIMIT 10 CANDIDATES;'
```

The CLI flag form keeps the query body as a normal `RETRIEVE`:

```bash
cortexdb --json aql ./db project:investments \
  'RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN default LIMIT 10 CANDIDATES;' \
  --explain analyze
```

Use `--explain plan` for non-executing explain.

## HTTP

The query parameter form mirrors the CLI flag:

```http
POST /v1/aql?scope=project:investments&explain=analyze

RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN default LIMIT 10 CANDIDATES;
```

Use `explain=plan` for non-executing explain.

## Response Fields

`explain.cost_model` reports the selected physical source path and the planner
reason. `explain.candidate_counts` reports global and stage-level counts,
including `estimated_after_bitmap` when the cost model can estimate it.

`explain.execution_trace` is present only for analyze mode:

- `operators[]`: ordered physical operators used by the executed retrieve path.
- `input_count` and `output_count`: backward-compatible actual counts.
- `actual_input_count` and `actual_output_count`: explicit actual counts.
- `estimated_output_count`: planner or pre-stage expected output count when
  available, otherwise `null`.
- `elapsed_nanos`: per-operator elapsed time.
- `total_elapsed_nanos`: total physical execution time for the trace.

Example operator:

```json
{
  "name": "BitmapIndexScan",
  "input_count": 0,
  "output_count": 10,
  "actual_input_count": 0,
  "actual_output_count": 10,
  "estimated_output_count": 10,
  "elapsed_nanos": 12000
}
```

Stability rules:

- Keep `input_count` and `output_count` for compatibility.
- Add new trace fields additively.
- Do not change query semantics when adding explain metrics.
- Keep cost-model `selected_path` and `reason` visible in JSON and text output.
