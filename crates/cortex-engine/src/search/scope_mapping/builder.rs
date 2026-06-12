use std::collections::BTreeSet;

use super::super::tokenize;
use super::normalize::{clean_scope_value, is_scope_stopword, normalize_for_match};
use super::types::{QueryScopeDirective, QueryScopeField};

#[derive(Default)]
pub(super) struct ScopeMappingBuilder {
    directives: Vec<QueryScopeDirective>,
    seen: BTreeSet<String>,
}

impl ScopeMappingBuilder {
    pub(super) fn push(
        &mut self,
        field: QueryScopeField,
        value: String,
        confidence_q16: u16,
        hard_filter: bool,
        reason: &'static str,
    ) {
        let value = clean_scope_value(&value);
        let terms = tokenize(&value)
            .into_iter()
            .filter(|term| !is_scope_stopword(term))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if value.is_empty() || terms.is_empty() {
            return;
        }
        let key = format!("{}:{}", field.as_str(), normalize_for_match(&value));
        if !self.seen.insert(key) {
            return;
        }
        self.directives.push(QueryScopeDirective {
            field,
            value,
            confidence_q16,
            hard_filter,
            terms,
            reason,
        });
    }

    pub(super) fn finish(mut self) -> Vec<QueryScopeDirective> {
        self.directives
            .sort_by_key(|directive| (directive.field, directive.value.clone()));
        self.directives
    }
}
