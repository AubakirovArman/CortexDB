use std::fs;
use std::path::PathBuf;

use cortex_engine::Database;
use cortex_storage::indexes::LexicalIndex;

pub(super) const MERGED_LEXICAL_CACHE_FILE: &str = "benchmark-merged-lexical.aci";
pub(super) const MERGED_LEXICAL_CACHE_MANIFEST_FILE: &str = "benchmark-merged-lexical.manifest";

pub(super) fn merged_lexical_cache_key(db: &Database) -> String {
    let manifest = db.manifest();
    let mut key = format!(
        "schema=cortexdb.enterprise_rag_bench.merged_lexical_cache.v1\n\
         generation={}\n\
         checkpoint_seq={}\n",
        manifest.generation, manifest.checkpoint_seq
    );
    for segment in &manifest.live_segments {
        key.push_str(&format!(
            "segment={} generation={} checkpoint_seq={} cell_count={}\n",
            segment.id, segment.generation, segment.checkpoint_seq, segment.cell_count
        ));
    }
    key
}

pub(super) fn write_merged_lexical_cache(
    cache_path: &PathBuf,
    cache_manifest_path: &PathBuf,
    cache_key: &str,
    lexical: &LexicalIndex,
) -> Result<(), String> {
    lexical
        .write(cache_path)
        .map_err(|error| format!("failed to write {}: {error}", cache_path.display()))?;
    fs::write(cache_manifest_path, cache_key)
        .map_err(|error| format!("failed to write {}: {error}", cache_manifest_path.display()))?;
    Ok(())
}

pub(super) fn merge_lexical(dst: &mut LexicalIndex, src: LexicalIndex) {
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
