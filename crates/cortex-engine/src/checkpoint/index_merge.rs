use cortex_storage::indexes::{BitmapIndex, LexicalIndex};

pub(super) fn merge_bitmap_index(dst: &mut BitmapIndex, src: BitmapIndex) {
    for (handle, values) in src.bitmaps {
        dst.bitmaps.entry(handle).or_default().extend(values);
    }
}

pub(super) fn merge_lexical_index(dst: &mut LexicalIndex, src: LexicalIndex) {
    for (term, values) in src.terms {
        dst.terms.entry(term).or_default().extend(values);
    }
    dst.doc_lengths.extend(src.doc_lengths);
    for (term, values) in src.term_frequencies {
        dst.term_frequencies.entry(term).or_default().extend(values);
    }
    for (field, values) in src.field_doc_lengths {
        dst.field_doc_lengths
            .entry(field)
            .or_default()
            .extend(values);
    }
    for (field, terms) in src.field_term_frequencies {
        let dst_terms = dst.field_term_frequencies.entry(field).or_default();
        for (term, values) in terms {
            dst_terms.entry(term).or_default().extend(values);
        }
    }
}
