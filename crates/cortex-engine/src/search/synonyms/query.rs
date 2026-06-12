use std::collections::BTreeSet;

use super::super::tokenize;
use super::terms::normalize_term;
use super::types::CorpusSynonymDictionary;

pub fn expand_query_with_corpus_synonyms(
    query: &str,
    dictionary: &CorpusSynonymDictionary,
    max_synonyms: usize,
    max_synonyms_per_term: usize,
) -> Option<String> {
    if max_synonyms == 0 || max_synonyms_per_term == 0 {
        return None;
    }
    let query_terms = tokenize(query)
        .into_iter()
        .map(|term| normalize_term(&term))
        .collect::<BTreeSet<_>>();
    let mut seen = query_terms.clone();
    let mut additions = Vec::new();
    for term in query_terms {
        for synonym in dictionary
            .synonyms_for(&term)
            .into_iter()
            .take(max_synonyms_per_term)
        {
            if seen.insert(synonym.clone()) {
                additions.push(synonym);
                if additions.len() >= max_synonyms {
                    break;
                }
            }
        }
        if additions.len() >= max_synonyms {
            break;
        }
    }
    if additions.is_empty() {
        None
    } else {
        Some(format!("{query} {}", additions.join(" ")))
    }
}
