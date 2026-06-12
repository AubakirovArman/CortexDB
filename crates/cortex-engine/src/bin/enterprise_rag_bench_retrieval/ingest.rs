use std::collections::BTreeMap;

use cortex_core::CellId;
use cortex_engine::{CellMetadata, Database};
use cortex_storage::indexes::LexicalIndex;
use serde_json::json;

use super::args::Args;
use super::document::{build_payload, extract_document_content};
use super::helpers::document_total;
use super::io::read_json;
use super::logger::RunLogger;
use super::vectors::DocumentVectorLookup;

pub(crate) fn ingest_documents(
    db: &mut Database,
    uuid_index: &BTreeMap<String, String>,
    document_vectors: &mut DocumentVectorLookup,
    mut ingest_lexical: Option<&mut LexicalIndex>,
    args: &Args,
    logger: &RunLogger,
) -> Result<usize, String> {
    let mut indexed = 0usize;
    let mut batch = Vec::with_capacity(args.batch_size);
    let total = document_total(args, uuid_index.len());
    for (index, (doc_id, rel_path)) in uuid_index.iter().enumerate() {
        if args.max_documents.is_some_and(|max| indexed >= max) {
            break;
        }
        let document = read_json(&args.sources_dir.join(rel_path))?;
        let (title, content) = extract_document_content(&document);
        let vector = document_vectors.get(doc_id)?;
        let payload = build_payload(doc_id, rel_path, &title, &content, vector.as_deref());
        let candidate = u32::try_from(index + 1).map_err(|_| "candidate id overflow")?;
        let cell_id = CellId(u64::from(candidate));
        let payload = payload.into_bytes();
        if let Some(lexical) = ingest_lexical.as_mut() {
            index_payload_lexical(lexical, candidate, &payload);
        }
        batch.push((cell_id, payload));
        if batch.len() >= args.batch_size {
            flush_batch(db, &mut batch, doc_id)?;
        }
        indexed += 1;
        if args.progress_every > 0 && indexed.is_multiple_of(args.progress_every) {
            logger.log(&format!("indexed {indexed}/{total}"));
            logger.status(
                "ingest",
                "running",
                "index documents",
                Some(indexed),
                Some(total),
                &[("last_doc_id", json!(doc_id))],
            );
        }
    }
    flush_batch(db, &mut batch, "final batch")?;
    Ok(indexed)
}

fn index_payload_lexical(lexical: &mut LexicalIndex, candidate: u32, payload: &[u8]) {
    let metadata = CellMetadata::from_payload(payload);
    let weighted_terms = metadata.weighted_lexical_terms();
    lexical.doc_lengths.insert(
        candidate,
        weighted_terms.values().copied().sum::<u32>().max(1),
    );
    for (term, frequency) in weighted_terms {
        lexical
            .terms
            .entry(term.clone())
            .or_default()
            .insert(candidate);
        lexical
            .term_frequencies
            .entry(term)
            .or_default()
            .insert(candidate, frequency);
    }
    for (field, terms) in metadata.lexical_field_terms() {
        lexical
            .field_doc_lengths
            .entry(field.clone())
            .or_default()
            .insert(candidate, terms.values().copied().sum::<u32>().max(1));
        let field_terms = lexical.field_term_frequencies.entry(field).or_default();
        for (term, frequency) in terms {
            field_terms
                .entry(term)
                .or_default()
                .insert(candidate, frequency);
        }
    }
}

fn flush_batch(
    db: &mut Database,
    batch: &mut Vec<(CellId, Vec<u8>)>,
    label: &str,
) -> Result<(), String> {
    if batch.is_empty() {
        return Ok(());
    }
    let cells = std::mem::take(batch);
    db.put_cells(cells)
        .map_err(|error| format!("failed to put batch near {label}: {error}"))?;
    Ok(())
}
