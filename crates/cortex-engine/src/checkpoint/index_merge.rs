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
}
