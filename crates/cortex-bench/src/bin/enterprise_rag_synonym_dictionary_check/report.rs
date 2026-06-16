use cortex_engine::search::CorpusSynonymDictionary;
use serde_json::{json, Value};

use super::args::Args;

pub(super) fn build_synonym_report(
    args: &Args,
    document_count: usize,
    dictionary: &CorpusSynonymDictionary,
) -> Value {
    let terms_with_synonyms = dictionary.terms_with_synonyms();
    let mut errors = Vec::new();
    if terms_with_synonyms < args.min_terms_with_synonyms {
        errors.push(format!(
            "terms_with_synonyms {terms_with_synonyms} < {}",
            args.min_terms_with_synonyms
        ));
    }
    json!({
        "schema_version": "cortexdb.enterprise_rag_synonym_dictionary_check.v1",
        "documents": document_count,
        "entries": dictionary.entries.len(),
        "terms_with_synonyms": terms_with_synonyms,
        "min_terms_with_synonyms": args.min_terms_with_synonyms,
        "options": {
            "min_term_document_frequency": args.min_term_document_frequency,
            "min_pair_document_frequency": args.min_pair_document_frequency,
            "max_synonyms_per_term": args.max_synonyms_per_term,
            "max_terms": args.max_terms,
            "max_terms_per_document": args.max_terms_per_document,
            "progress_every": args.progress_every,
            "streaming_document_build": true,
        },
        "sample": dictionary.entries.iter().take(20).map(|entry| {
            json!({
                "term": entry.term,
                "document_frequency": entry.document_frequency,
                "synonyms": entry.synonyms.iter().map(|candidate| {
                    json!({
                        "term": candidate.term,
                        "score_q16": candidate.score_q16,
                        "cooccurrence_count": candidate.cooccurrence_count,
                    })
                }).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
        "errors": errors,
        "status": if errors.is_empty() { "passed" } else { "failed" },
    })
}
