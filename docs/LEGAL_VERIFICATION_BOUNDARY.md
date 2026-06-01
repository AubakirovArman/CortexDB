# Legal Verification Boundary

Status: future design gate, not implemented.

## Goal

Define what would be required before CortexDB can offer legally defensible
verification workflows.

## Supported Legal Domain

Any legal-grade claim must name a specific domain and jurisdiction. Generic
legal advice is out of scope. The first domain must be selected with expert
review before implementation.

## Admissible Sources

The system must define source classes, citation requirements, document
provenance, freshness rules, and exclusions. Uncited model output is never
admissible evidence.

## Reviewer Workflow

Legal-grade reports require human review, reviewer identity, approval status,
review timestamp, and a retention policy. Automated verification alone is not
legal-grade.

## Citation Policy

Every supported or contradicted claim must trace to source records with stable
source refs. The system must refuse unsupported legal conclusions when evidence
is insufficient.

## Output Boundary

Outputs must separate evidence summaries from legal advice. If the product is
not certified for a domain, responses must explicitly state the limitation.

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
