mod builder;
mod database;
mod persistence;
mod query;
mod terms;
mod types;

#[cfg(test)]
mod tests;

pub use builder::build_corpus_synonym_dictionary;
pub use persistence::{read_acsyn_dictionary, write_acsyn_dictionary};
pub use query::expand_query_with_corpus_synonyms;
pub use types::{
    CorpusSynonymCandidate, CorpusSynonymDictionary, CorpusSynonymDictionaryBuilder,
    CorpusSynonymEntry, CorpusSynonymOptions,
};
