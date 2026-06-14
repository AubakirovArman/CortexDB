# CortexDB Explain Contract

`ContextPack` explainability is part of the `context_pack.v1` contract. Every
selected context cell carries an `explain` object, and every recorded exclusion
with a `cell_id` carries an anomaly with `why_excluded`.

## Selected Cells

Selected cells expose:

- `why_selected`: stable human-readable selection reason.
- `score`: final ranking score after relevance, provenance, freshness,
  redundancy, and feedback adjustments.
- `matched_terms`: query terms found in the cell body.
- `score_components`: ordered score parts with `name`, `value`,
  `contribution`, and `reason`.
- `base_bm25`, `source_trust_*`, `source_freshness_*`,
  `redundancy_penalty`: scalar fields frozen in `context_pack.v1`.
- `access_decision`: policy result for API responses and cell explain output.

CLI example:

```bash
cortexdb explain ./db project:investments \
  'RETRIEVE CONTEXT FOR TASK "solar budget" IN BRAIN default LIMIT 10 CANDIDATES;' \
  --cell-id 42
```

JSON output uses `context_cell_explain.v1`:

```json
{
  "schema_version": "context_cell_explain.v1",
  "cell_id": 42,
  "outcome": "selected",
  "first_excluding_stage": null,
  "why_selected": "Selected due to high provenance source trust and relevant query terms",
  "why_excluded": null,
  "score": 80000,
  "matched_terms": ["solar", "budget"],
  "score_components": [
    {
      "name": "base_bm25",
      "value": 20000,
      "contribution": 20000,
      "reason": "keyword overlap between query terms and cell body"
    }
  ],
  "access_decision": {
    "decision": "allowed",
    "policy": "agent_view_read_scope",
    "reason": "cell scope is readable by agent view",
    "scope": "project:investments",
    "scope_id": 123,
    "agent_id": 1
  }
}
```

## Excluded Cells

Excluded cells are reported through `ContextPack.anomalies[]`:

- `cell_id`: excluded candidate id when known.
- `code`: stable exclusion code.
- `message`: diagnostic detail.
- `why_excluded`: stable human-readable exclusion reason.

`ContextPack::explain_cell(cell_id)` and `cortexdb explain --cell-id` map an
excluded anomaly to:

- `outcome: "excluded"`
- `first_excluding_stage`: first pipeline stage that excluded the cell.
- `why_excluded`: anomaly reason, falling back to `message` for legacy
  anomalies without a reason.

Current first-stage names:

- `redundancy` for `redundant_cell`
- `citation_requirement` for `missing_citation`
- `token_budget` for `token_overload`
- `agent_scope` for `scope_mismatch`
- `answerability` for `insufficient_context`
- `conflict_visibility` for `visible_conflict`

Excluded example:

```json
{
  "schema_version": "context_cell_explain.v1",
  "cell_id": 43,
  "outcome": "excluded",
  "first_excluding_stage": "token_budget",
  "why_selected": null,
  "why_excluded": "excluded because estimated_tokens would exceed token_budget_tokens; skipped so later smaller candidates can still fit",
  "score": null,
  "matched_terms": [],
  "score_components": [],
  "access_decision": null
}
```

## Stability Rules

- Do not remove or rename `context_pack.v1` explain fields.
- Add new explain fields as optional fields first.
- Keep `score_components` names stable for existing components.
- Add regression tests for any new exclusion code or stage name.
- Update `docs/schemas/context_pack.v1.json`, `docs/openapi.yaml`, and SDK
  snapshots when the response shape changes.
