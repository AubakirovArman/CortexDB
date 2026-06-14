use std::collections::BTreeSet;

use cortex_aql::ScopeId;

use super::metadata::scope_handle;
use super::EngineAqlIndex;

impl EngineAqlIndex {
    pub(super) fn prune_to_readable_scopes(&mut self, readable_scopes: &BTreeSet<ScopeId>) {
        let allowed = readable_scopes
            .iter()
            .filter_map(|scope| self.bitmaps.get(&scope_handle(*scope)))
            .flat_map(|candidates| candidates.iter().copied())
            .collect::<BTreeSet<_>>();
        let removed = self
            .candidate_to_cell
            .keys()
            .copied()
            .filter(|candidate| !allowed.contains(candidate))
            .collect::<BTreeSet<_>>();
        self.remove_candidates(&removed);
        self.rebuild_universe();
    }
}
