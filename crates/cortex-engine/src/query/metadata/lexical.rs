use std::collections::BTreeMap;

use crate::search::TextAnalyzer;

use super::fields::{add_field_terms, add_field_terms_with_analyzer, lexical_field_weight};
use super::types::CellMetadata;

impl CellMetadata {
    pub fn weighted_lexical_terms(&self) -> BTreeMap<String, u32> {
        self.weighted_lexical_terms_for_fields(self.lexical_field_terms())
    }

    pub fn weighted_lexical_terms_with_analyzer(
        &self,
        analyzer: &TextAnalyzer,
    ) -> BTreeMap<String, u32> {
        self.weighted_lexical_terms_for_fields(self.lexical_field_terms_with_analyzer(analyzer))
    }

    fn weighted_lexical_terms_for_fields(
        &self,
        fields: BTreeMap<String, BTreeMap<String, u32>>,
    ) -> BTreeMap<String, u32> {
        let mut terms = BTreeMap::new();
        for (field, field_terms) in fields {
            let weight = lexical_field_weight(&field);
            for (term, frequency) in field_terms {
                *terms.entry(term).or_default() += frequency.saturating_mul(weight);
            }
        }
        terms
    }

    pub fn lexical_field_terms(&self) -> BTreeMap<String, BTreeMap<String, u32>> {
        let mut fields = BTreeMap::new();
        add_field_terms(&mut fields, "body", &self.body_text);
        if let Some(title) = &self.title {
            add_field_terms(&mut fields, "title", title);
        }
        for value in [
            self.path.as_ref(),
            self.document_id.as_ref(),
            self.section.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            add_field_terms(&mut fields, "path", value);
        }
        for value in [
            self.project.as_ref(),
            self.entity.as_ref(),
            self.sector.as_ref(),
            self.owner.as_ref(),
            self.status_tag.as_ref(),
            self.event_date.as_ref(),
            self.topic.as_ref(),
            self.as_of.as_ref(),
            self.valid_from.as_ref(),
            self.valid_to.as_ref(),
            self.source.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            add_field_terms(&mut fields, "entity", value);
        }
        if let Some(chunk_id) = &self.chunk_id {
            add_field_terms(&mut fields, "chunk", chunk_id);
        }
        if let Some(parent_id) = &self.parent_id {
            add_field_terms(&mut fields, "chunk", parent_id);
        }
        for value in [
            self.table_id.as_ref(),
            self.table_headers.as_ref(),
            self.row_label.as_ref(),
            self.source_ref
                .as_ref()
                .and_then(|source| source.cell_range.as_ref()),
        ]
        .into_iter()
        .flatten()
        {
            add_field_terms(&mut fields, "table", value);
        }
        fields
    }

    pub fn lexical_field_terms_with_analyzer(
        &self,
        analyzer: &TextAnalyzer,
    ) -> BTreeMap<String, BTreeMap<String, u32>> {
        let mut fields = BTreeMap::new();
        add_field_terms_with_analyzer(&mut fields, "body", &self.body_text, analyzer);
        if let Some(title) = &self.title {
            add_field_terms_with_analyzer(&mut fields, "title", title, analyzer);
        }
        for value in [
            self.path.as_ref(),
            self.document_id.as_ref(),
            self.section.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            add_field_terms_with_analyzer(&mut fields, "path", value, analyzer);
        }
        for value in [
            self.project.as_ref(),
            self.entity.as_ref(),
            self.sector.as_ref(),
            self.owner.as_ref(),
            self.status_tag.as_ref(),
            self.event_date.as_ref(),
            self.topic.as_ref(),
            self.as_of.as_ref(),
            self.valid_from.as_ref(),
            self.valid_to.as_ref(),
            self.source.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            add_field_terms_with_analyzer(&mut fields, "entity", value, analyzer);
        }
        if let Some(chunk_id) = &self.chunk_id {
            add_field_terms_with_analyzer(&mut fields, "chunk", chunk_id, analyzer);
        }
        if let Some(parent_id) = &self.parent_id {
            add_field_terms_with_analyzer(&mut fields, "chunk", parent_id, analyzer);
        }
        for value in [
            self.table_id.as_ref(),
            self.table_headers.as_ref(),
            self.row_label.as_ref(),
            self.source_ref
                .as_ref()
                .and_then(|source| source.cell_range.as_ref()),
        ]
        .into_iter()
        .flatten()
        {
            add_field_terms_with_analyzer(&mut fields, "table", value, analyzer);
        }
        fields
    }
}
