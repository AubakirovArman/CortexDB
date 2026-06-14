use std::collections::BTreeSet;

use cortex_storage::indexes::{BitmapIndex, LexicalIndex};

use super::types::PersistedIndexState;

pub(super) fn merge_bitmap_index(state: &mut PersistedIndexState, src: BitmapIndex) {
    for (handle, values) in src.bitmaps {
        for candidate in values.iter() {
            state
                .postings
                .bitmap_handles_by_candidate
                .entry(candidate)
                .or_default()
                .insert(handle);
        }
        state
            .bitmap
            .bitmaps
            .entry(handle)
            .or_default()
            .extend(values);
    }
}

pub(super) fn merge_lexical_index(state: &mut PersistedIndexState, src: LexicalIndex) {
    for (term, values) in src.terms {
        for candidate in &values {
            state
                .postings
                .lexical_terms_by_candidate
                .entry(*candidate)
                .or_default()
                .insert(term.clone());
        }
        state.lexical.terms.entry(term).or_default().extend(values);
    }
    state.lexical.doc_lengths.extend(src.doc_lengths);
    for (term, values) in src.term_frequencies {
        for candidate in values.keys() {
            state
                .postings
                .lexical_terms_by_candidate
                .entry(*candidate)
                .or_default()
                .insert(term.clone());
        }
        state
            .lexical
            .term_frequencies
            .entry(term)
            .or_default()
            .extend(values);
    }
    for (field, values) in src.field_doc_lengths {
        for candidate in values.keys() {
            state
                .postings
                .lexical_fields_by_candidate
                .entry(*candidate)
                .or_default()
                .insert(field.clone());
        }
        state
            .lexical
            .field_doc_lengths
            .entry(field)
            .or_default()
            .extend(values);
    }
    for (field, terms) in src.field_term_frequencies {
        let dst_terms = state
            .lexical
            .field_term_frequencies
            .entry(field.clone())
            .or_default();
        for (term, values) in terms {
            for candidate in values.keys() {
                state
                    .postings
                    .lexical_field_terms_by_candidate
                    .entry(*candidate)
                    .or_default()
                    .insert((field.clone(), term.clone()));
            }
            dst_terms.entry(term).or_default().extend(values);
        }
    }
}

pub(super) fn remove_candidates(state: &mut PersistedIndexState, candidates: &BTreeSet<u32>) {
    for candidate in candidates {
        remove_candidate(state, *candidate);
    }
}

fn remove_candidate(state: &mut PersistedIndexState, candidate: u32) {
    if let Some(handles) = state
        .postings
        .bitmap_handles_by_candidate
        .remove(&candidate)
    {
        for handle in handles {
            if let Some(values) = state.bitmap.bitmaps.get_mut(&handle) {
                values.remove(candidate);
                if values.is_empty() {
                    state.bitmap.bitmaps.remove(&handle);
                }
            }
        }
    }
    if let Some(terms) = state.postings.lexical_terms_by_candidate.remove(&candidate) {
        for term in terms {
            if let Some(values) = state.lexical.terms.get_mut(&term) {
                values.remove(&candidate);
                if values.is_empty() {
                    state.lexical.terms.remove(&term);
                }
            }
            if let Some(values) = state.lexical.term_frequencies.get_mut(&term) {
                values.remove(&candidate);
                if values.is_empty() {
                    state.lexical.term_frequencies.remove(&term);
                }
            }
        }
    }
    state.lexical.doc_lengths.remove(&candidate);
    if let Some(fields) = state
        .postings
        .lexical_fields_by_candidate
        .remove(&candidate)
    {
        for field in fields {
            if let Some(values) = state.lexical.field_doc_lengths.get_mut(&field) {
                values.remove(&candidate);
                if values.is_empty() {
                    state.lexical.field_doc_lengths.remove(&field);
                }
            }
        }
    }
    if let Some(field_terms) = state
        .postings
        .lexical_field_terms_by_candidate
        .remove(&candidate)
    {
        for (field, term) in field_terms {
            if let Some(terms) = state.lexical.field_term_frequencies.get_mut(&field) {
                if let Some(values) = terms.get_mut(&term) {
                    values.remove(&candidate);
                    if values.is_empty() {
                        terms.remove(&term);
                    }
                }
                if terms.is_empty() {
                    state.lexical.field_term_frequencies.remove(&field);
                }
            }
        }
    }
    state.candidate_to_cell.remove(&candidate);
}
