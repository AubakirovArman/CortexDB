use std::collections::BTreeSet;

use cortex_aql::AgentView;
use cortex_storage::indexes::BitmapIndex;

use crate::plan::PolicyRewrite;
use crate::query::metadata::scope_handle;

pub(super) fn allowed_candidates(bitmap: &BitmapIndex, view: &AgentView) -> BTreeSet<u32> {
    let mut allowed = BTreeSet::new();
    let policy = PolicyRewrite::new(view);
    for scope in policy.readable_scopes() {
        if let Some(candidates) = bitmap.bitmaps.get(&scope_handle(scope).0) {
            allowed.extend(candidates.iter());
        }
    }
    allowed
}
