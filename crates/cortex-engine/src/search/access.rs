use std::collections::BTreeSet;

use cortex_aql::AgentView;
use cortex_storage::indexes::BitmapIndex;

use crate::query::metadata::scope_handle;

pub(super) fn allowed_candidates(bitmap: &BitmapIndex, view: &AgentView) -> BTreeSet<u32> {
    let mut allowed = BTreeSet::new();
    for scope in &view.readable_scopes {
        if let Some(candidates) = bitmap.bitmaps.get(&scope_handle(*scope).0) {
            allowed.extend(candidates.iter().copied());
        }
    }
    allowed
}
