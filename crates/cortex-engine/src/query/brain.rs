use cortex_aql::BrainId;

pub(crate) const DEFAULT_BRAIN: BrainId = BrainId(1);

pub(crate) fn resolve_single_brain_name(name: &str) -> Option<BrainId> {
    if name.trim().is_empty() {
        None
    } else {
        Some(DEFAULT_BRAIN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_brain_resolution_accepts_legacy_aliases_as_default() {
        assert_eq!(resolve_single_brain_name("default"), Some(DEFAULT_BRAIN));
        assert_eq!(
            resolve_single_brain_name("investment_projects"),
            Some(DEFAULT_BRAIN)
        );
        assert_eq!(resolve_single_brain_name(""), None);
    }
}
