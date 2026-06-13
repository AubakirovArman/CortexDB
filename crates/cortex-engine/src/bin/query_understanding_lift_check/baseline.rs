use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub(super) struct PlainDoc {
    doc_id: String,
    terms: BTreeMap<String, u32>,
}

impl PlainDoc {
    pub(super) fn new(doc_id: String, text: &str) -> Self {
        Self {
            doc_id,
            terms: plain_terms(text),
        }
    }
}

pub(super) fn plain_search(docs: &[PlainDoc], query: &str, limit: usize) -> Vec<String> {
    let query_terms = plain_terms(query);
    let mut scored = docs
        .iter()
        .filter_map(|doc| {
            let score = query_terms
                .iter()
                .map(|(term, weight)| {
                    u64::from(*weight) * u64::from(*doc.terms.get(term).unwrap_or(&0))
                })
                .sum::<u64>();
            (score > 0).then(|| (score, doc.doc_id.clone()))
        })
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    scored
        .into_iter()
        .take(limit)
        .map(|(_, doc_id)| doc_id)
        .collect()
}

fn plain_terms(text: &str) -> BTreeMap<String, u32> {
    let mut terms = BTreeMap::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            current.extend(ch.to_lowercase());
        } else if !current.is_empty() {
            if !is_stopword(&current) {
                *terms.entry(std::mem::take(&mut current)).or_default() += 1;
            } else {
                current.clear();
            }
        }
    }
    if !current.is_empty() && !is_stopword(&current) {
        *terms.entry(current).or_default() += 1;
    }
    terms
}

fn is_stopword(term: &str) -> bool {
    matches!(
        term,
        "a" | "an"
            | "and"
            | "are"
            | "as"
            | "at"
            | "be"
            | "by"
            | "for"
            | "from"
            | "how"
            | "is"
            | "it"
            | "of"
            | "on"
            | "or"
            | "should"
            | "the"
            | "to"
            | "we"
            | "what"
            | "when"
            | "which"
            | "who"
            | "why"
    )
}

#[cfg(test)]
mod tests {
    use super::{plain_search, PlainDoc};

    #[test]
    fn plain_search_does_not_expand_terms() {
        let docs = vec![PlainDoc::new(
            "doc_owner".to_owned(),
            "assigned DRI dependency",
        )];

        assert!(plain_search(&docs, "owner blocker", 3).is_empty());
    }
}
