# Legal Verification Boundary

Status: future phase 2 local review-boundary evaluator started, no legal-grade
product claim implemented.

## Goal

Define what would be required before CortexDB can offer legally defensible
verification workflows.

## Supported Legal Domain

Any legal-grade claim must name a specific domain and jurisdiction. Generic
legal advice is out of scope. The first domain must be selected with expert
review before implementation.

## Dataset Fixture

The local dataset fixture is a contract fixture only. It names a candidate
domain and jurisdiction, marks expert review as required, and keeps
`legal_grade_ready=false` until domain experts approve real labeled data.

## Admissible Sources

The system must define source classes, citation requirements, document
provenance, freshness rules, and exclusions. Uncited model output is never
admissible evidence.

## Reviewer Workflow

Legal-grade reports require human review, reviewer identity, approval status,
review timestamp, and a retention policy. Automated verification alone is not
legal-grade.

The local `evaluate_legal_verification_boundary` helper checks whether a
candidate legal verification report satisfies the current prerequisite policy:
specific domain and jurisdiction, non-empty claim, source refs, reviewer
identity, reviewer approval, and output limited to evidence summaries rather
than legal advice. It always keeps `legal_grade_ready=false` until external
domain review exists.

## Citation Policy

Every supported or contradicted claim must trace to source records with stable
source refs. The system must refuse unsupported legal conclusions when evidence
is insufficient.

## Citation Policy Fixture

The citation policy fixture requires source refs, reviewer approval, refusal of
unsupported conclusions, and explicit rejection of uncited model output as
admissible evidence.

## Output Boundary

Outputs must separate evidence summaries from legal advice. If the product is
not certified for a domain, responses must explicitly state the limitation.

## Quality Gate Boundary

The current deterministic `VERIFY FACT` quality gate is useful evidence for
numeric conflict, citation guard, contradiction, and insufficiency behavior.
That gate is not legal proof. It is only a prerequisite signal that the engine
can produce structured evidence reports before legal-domain review is added.

## Current Evidence Boundary

The current gates prove local prerequisites only:

| Gate | Evidence |
| --- | --- |
| `make legal-verification-dataset-check` | A domain-specific dataset contract fixture exists, requires expert review, and the local review-boundary evaluator keeps `legal_grade_ready=false`. |
| `make legal-verification-quality-check` | The deterministic verification quality report passes and includes citation and numeric guard coverage. |
| `make legal-citation-policy-check` | A citation policy fixture and local evaluator refuse uncited output, require source refs, and require reviewer approval. |
| `make public-claims-check` | Public docs continue to block legal-grade overclaims. |

Reports are written under `target/legal-verification/` and keep
`legal_verification_ready=false`. They do not claim legal advice,
certification, admissibility, or legal-grade verification for any jurisdiction.

## Required Gates

1. `make legal-verification-design-check`
2. `make legal-verification-dataset-check`
3. `make legal-verification-quality-check`
4. `make legal-citation-policy-check`
5. `make public-claims-check`

## Acceptance

1. The selected legal domain has expert-reviewed datasets.
2. Reports are citation-complete and reviewer-traceable.
3. The system refuses unsupported or out-of-domain legal claims.
4. Public docs do not imply legal advice outside the proven scope.

## Non-goals

1. Replacing lawyers or regulated legal review.
2. Generic legal-grade claims across all jurisdictions.
3. Treating heuristic `VERIFY FACT` as legal proof.
