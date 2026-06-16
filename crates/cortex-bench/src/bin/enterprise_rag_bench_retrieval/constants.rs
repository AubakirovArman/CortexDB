pub(crate) const ORACLE_FIELDS: &[&str] = &[
    "answer_facts",
    "expected_doc_ids",
    "gold_answer",
    "question_type",
    "source_types",
];

pub(crate) const CLEAN_PREFILTER_RETRIEVAL_FIELDS: &[&str] =
    &["answer", "document_ids", "question", "question_id"];
pub(crate) const MAX_SKIP_CHECKPOINT_ENGINE_DOCS: usize = 10_000;
pub(crate) const ENGINE_PREFILTER_SHORTLIST_LIMIT: usize = 128;
pub(crate) const ENGINE_PREFILTER_LEXICAL_HEAD_COUNT: usize = 4;
pub(crate) const ENGINE_PREFILTER_DEFAULT_DOC_LIMIT: usize = 6;
pub(crate) const ENGINE_PREFILTER_STRONG_EVIDENCE_SCORE: u32 = 32;
