# F4.3: the pgvector+OPA "thin wrapper" access policy. A cell is readable iff its
# scope is in the agent's readable_scopes. This is post-hoc authz over rows the
# vector search already ranked — the structural weakness the AAB matrix exposes:
# policy lives OUTSIDE the data path, so there is no plan-bound, signed, replayable
# proof of which cells were considered (receipt_verifiability) and no byte-identical
# rebuild guarantee (determinism).
package aab.access

default allow := false

allow if {
	input.cell_scope == input.readable_scopes[_]
}
