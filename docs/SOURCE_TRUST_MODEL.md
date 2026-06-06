# Source Trust Model v1

CortexDB uses deterministic source trust metadata to make retrieval, context
packing, and verification output explainable without depending on an LLM judge.

## Metadata

Cells may include:

```text
source_trust_q16=60000
```

The value is an integer in the Q16 range `0..65535`. Cells may also use a
calibrated source class:

```text
source_trust_class=internal
```

If both fields are present, `source_trust_q16` wins. If both fields are absent,
the engine uses `32768` and labels the category as `unknown`.

## Calibrated Classes

| Class | Q16 weight | Resulting category | Typical use |
| --- | ---: | --- | --- |
| `official` | `60000` | `official` | Regulator, audited report, signed disclosure. |
| `internal` | `52000` | `high` | Trusted internal system, reviewed enterprise source. |
| `extracted` | `40000` | `medium` | Parsed document chunk or extraction pipeline output. |
| `inferred` | `20000` | `low` | Derived claim, weak signal, model-assisted extraction. |

## Categories

| Q16 range | Category |
| --- | --- |
| absent | `unknown` |
| `0..21845` | `low` |
| `21846..43690` | `medium` |
| `43691..58981` | `high` |
| `58982..65535` | `official` |

These categories are provenance labels. They are not legal or compliance
attestations.

## ContextPack

ContextPack includes source trust in each selected cell explain block:

```text
source_trust_q16
source_trust_category
source_trust_bonus
```

The `source_trust_bonus` is the Q16 trust score used in the deterministic
selection score. Score components explain whether the bonus came from explicit
`source_trust_q16`, calibrated `source_trust_class`, or the default unknown
trust. Missing metadata still contributes the default unknown score of `32768`,
so unknown sources are not treated as zero-trust evidence.

## VERIFY FACT

`VERIFY FACT` includes `source_trust_q16` and `source_trust_category` on both
supporting and contradicting evidence. Equal text matches are ordered by:

```text
matched_terms desc
source_trust_q16 desc
cell_id asc
```

This makes higher-trust evidence visible first while preserving all readable
evidence in the report.

## Current Limitations

- v1 trusts explicit `source_trust_q16` and `source_trust_class` metadata; it
  does not infer trust from domain names or source URLs.
- Calibration weights are fixed constants in the Rust engine. Tenant-specific
  trust policies are a future epic.
- Trust affects ranking and explainability, not authorization. Agent scope
  permissions still determine what evidence is visible.
