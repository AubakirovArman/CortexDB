use std::collections::BTreeMap;

#[derive(Debug)]
pub(crate) struct ExternalPrefilterRetrieval {
    pub(crate) by_question_id: BTreeMap<String, Vec<String>>,
    pub(crate) rows: usize,
}

impl ExternalPrefilterRetrieval {
    pub(crate) fn doc_ids(&self, question_id: &str) -> Option<&[String]> {
        self.by_question_id.get(question_id).map(Vec::as_slice)
    }
}
