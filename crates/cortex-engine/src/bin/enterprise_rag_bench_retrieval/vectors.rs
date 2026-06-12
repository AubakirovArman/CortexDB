use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};

use cortex_aql::AgentView;
use cortex_core::CellId;
use cortex_engine::search::{
    parse_vector_literal, SearchIndexes, SearchMode, SearchQuery, SearchRerankInput,
    SearchReranker, SearchResult,
};
use cortex_engine::{scope_id, CellMetadata, Database};
use serde::Deserialize;
use serde_json::{json, Value};

use super::logger::{logger_progress_due, RunLogger};

pub(super) struct DocumentVectorLookup {
    offsets: BTreeMap<String, u64>,
    file: Option<File>,
}

#[derive(Deserialize)]
struct DocumentVectorIdRow {
    doc_id: String,
}

impl DocumentVectorLookup {
    pub(super) fn empty() -> Self {
        Self {
            offsets: BTreeMap::new(),
            file: None,
        }
    }

    pub(super) fn open(path: &std::path::Path) -> Result<Self, String> {
        let file = File::open(path).map_err(|error| {
            format!(
                "failed to open document vectors {}: {error}",
                path.display()
            )
        })?;
        let mut reader = BufReader::new(file);
        let mut offsets = BTreeMap::new();
        let mut line = String::new();
        loop {
            let offset = reader
                .stream_position()
                .map_err(|error| format!("failed to read {} offset: {error}", path.display()))?;
            line.clear();
            let bytes = reader
                .read_line(&mut line)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            if bytes == 0 {
                break;
            }
            let row: DocumentVectorIdRow =
                serde_json::from_str(line.trim_end()).map_err(|error| {
                    format!(
                        "failed to parse {} at byte {offset}: {error}",
                        path.display()
                    )
                })?;
            let doc_id = (!row.doc_id.trim().is_empty())
                .then_some(row.doc_id)
                .ok_or_else(|| format!("document vector row at byte {offset} missing doc_id"))?;
            offsets.insert(doc_id, offset);
        }
        Ok(Self {
            offsets,
            file: Some(reader.into_inner()),
        })
    }

    pub(super) fn is_empty(&self) -> bool {
        self.offsets.is_empty()
    }

    pub(super) fn len(&self) -> usize {
        self.offsets.len()
    }

    pub(super) fn get(&mut self, doc_id: &str) -> Result<Option<Vec<i16>>, String> {
        let Some(offset) = self.offsets.get(doc_id).copied() else {
            return Ok(None);
        };
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| "document vector lookup has offsets without an open file".to_owned())?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|error| format!("failed to seek document vector offset {offset}: {error}"))?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|error| format!("failed to read document vector at byte {offset}: {error}"))?;
        let row: Value = serde_json::from_str(line.trim_end()).map_err(|error| {
            format!("failed to parse document vector at byte {offset}: {error}")
        })?;
        let vector = parse_query_vector(&row)
            .ok_or_else(|| format!("document vector for doc_id={doc_id} missing vector"))?;
        Ok(Some(vector))
    }
}

pub(super) struct BenchmarkSearchIndex {
    indexes: SearchIndexes,
    vectors: BTreeMap<u32, Vec<i16>>,
    candidate_to_cell: BTreeMap<u32, CellId>,
}

impl BenchmarkSearchIndex {
    pub(super) fn load(
        db: &Database,
        uuid_index: &BTreeMap<String, String>,
        view: &AgentView,
        logger: &RunLogger,
    ) -> Result<Self, String> {
        let mut indexes = SearchIndexes::default();
        let mut vectors = BTreeMap::new();
        let mut candidate_to_cell = BTreeMap::new();
        let total = uuid_index.len();
        for (index, doc_id) in uuid_index.keys().enumerate() {
            let candidate =
                u32::try_from(index + 1).map_err(|_| "candidate id overflow".to_owned())?;
            let cell_id = CellId(u64::from(candidate));
            let Some((payload, descriptor)) = db.get_latest_cell_with_descriptor(cell_id) else {
                continue;
            };
            let metadata = CellMetadata::from_payload_with_descriptor(&payload, &descriptor);
            if !view.can_read_scope(scope_id(&metadata.scope)) {
                continue;
            }
            indexes.add_field_terms(candidate, metadata.lexical_field_terms());
            if let Some(vector) = body_vector_from_payload(&payload) {
                vectors.insert(candidate, vector);
            }
            candidate_to_cell.insert(candidate, cell_id);
            if logger_progress_due(index + 1, total, 50_000) {
                logger.log(&format!(
                    "built reusable search index {}/{} last_doc_id={}",
                    index + 1,
                    total,
                    doc_id
                ));
                logger.status(
                    "build_reusable_search_index",
                    "running",
                    "build reusable in-memory search index",
                    Some(index + 1),
                    Some(total),
                    &[("last_doc_id", json!(doc_id))],
                );
            }
        }
        Ok(Self {
            indexes,
            vectors,
            candidate_to_cell,
        })
    }

    pub(super) fn search_payloads(
        &self,
        db: &Database,
        query: SearchQuery<'_>,
        top_k: usize,
        reranker: Option<&dyn SearchReranker>,
    ) -> Vec<Vec<u8>> {
        if query.vector.is_some()
            && matches!(query.mode, SearchMode::Hybrid | SearchMode::HybridRerank)
        {
            return self.search_bounded_hybrid_payloads(db, query, top_k, reranker);
        }
        let candidate_limit = if reranker.is_some() {
            top_k.max(64)
        } else {
            top_k.max(1)
        };
        let search_query = SearchQuery {
            limit: candidate_limit,
            ..query
        };
        let mut results = self.indexes.search(search_query);
        if let Some(reranker) = reranker {
            self.rerank_results(db, &mut results, query, reranker);
        }
        results.truncate(top_k);
        results
            .into_iter()
            .filter_map(|result| {
                let cell_id = self.candidate_to_cell.get(&result.cell_id)?;
                db.get_latest_cell(*cell_id)
            })
            .collect()
    }

    fn search_bounded_hybrid_payloads(
        &self,
        db: &Database,
        query: SearchQuery<'_>,
        top_k: usize,
        reranker: Option<&dyn SearchReranker>,
    ) -> Vec<Vec<u8>> {
        let lexical_pool = top_k.max(2_048);
        let lexical_query = SearchQuery {
            mode: SearchMode::Keyword,
            limit: lexical_pool,
            ..query
        };
        let mut results = self.indexes.search(lexical_query);
        if results.is_empty() {
            results = self
                .vectors
                .keys()
                .take(lexical_pool)
                .map(|candidate| SearchResult {
                    cell_id: *candidate,
                    score: 0,
                    lexical_score: 0,
                    vector_score: 0,
                })
                .collect();
        }
        if let Some(query_vector) = query.vector {
            for result in &mut results {
                if let Some(candidate_vector) = self.vectors.get(&result.cell_id) {
                    result.vector_score = vector_dot_score(query_vector, candidate_vector);
                    result.score = result.lexical_score.saturating_add(result.vector_score);
                }
            }
        }
        if let Some(reranker) = reranker {
            self.rerank_results(db, &mut results, query, reranker);
        } else {
            results.sort_by_key(|result| (std::cmp::Reverse(result.score), result.cell_id));
        }
        results.truncate(top_k);
        results
            .into_iter()
            .filter_map(|result| {
                let cell_id = self.candidate_to_cell.get(&result.cell_id)?;
                db.get_latest_cell(*cell_id)
            })
            .collect()
    }

    fn rerank_results(
        &self,
        db: &Database,
        results: &mut [SearchResult],
        query: SearchQuery<'_>,
        reranker: &dyn SearchReranker,
    ) {
        for result in results.iter_mut() {
            let payload = self
                .candidate_to_cell
                .get(&result.cell_id)
                .and_then(|cell_id| db.get_latest_cell(*cell_id));
            result.score = reranker.rerank_score(SearchRerankInput {
                query_text: query.text,
                query_vector: query.vector,
                candidate_id: u64::from(result.cell_id),
                lexical_score: result.lexical_score,
                vector_score: result.vector_score,
                base_score: result.score,
                metadata: None,
                payload: payload.as_deref(),
            });
        }
        results.sort_by_key(|result| (std::cmp::Reverse(result.score), result.cell_id));
    }
}

pub(super) fn vector_dot_score(query: &[i16], candidate: &[i16]) -> u64 {
    query
        .iter()
        .zip(candidate)
        .fold(0i128, |score, (left, right)| {
            score + i128::from(*left) * i128::from(*right)
        })
        .max(0)
        .min(i128::from(u64::MAX)) as u64
}

pub(super) fn load_query_vectors(
    path: Option<&std::path::PathBuf>,
) -> Result<BTreeMap<String, Vec<i16>>, String> {
    load_id_vectors(path, "question_id")
}

pub(super) fn load_document_vectors(
    path: Option<&std::path::PathBuf>,
) -> Result<DocumentVectorLookup, String> {
    let Some(path) = path else {
        return Ok(DocumentVectorLookup::empty());
    };
    DocumentVectorLookup::open(path)
}

pub(super) fn load_id_vectors(
    path: Option<&std::path::PathBuf>,
    id_field: &str,
) -> Result<BTreeMap<String, Vec<i16>>, String> {
    let Some(path) = path else {
        return Ok(BTreeMap::new());
    };
    let mut vectors = BTreeMap::new();
    for (index, row) in super::read_jsonl(path)?.into_iter().enumerate() {
        let id = super::required_str(&row, id_field, index)?.to_owned();
        let vector = parse_query_vector(&row)
            .ok_or_else(|| format!("{id_field} vector row {} missing vector", index + 1))?;
        if vector.is_empty() {
            return Err(format!(
                "{} row {} has empty or invalid vector",
                path.display(),
                index + 1
            ));
        }
        vectors.insert(id, vector);
    }
    Ok(vectors)
}

pub(super) fn parse_query_vector(row: &Value) -> Option<Vec<i16>> {
    let value = row.get("vector")?;
    if let Some(text) = value.as_str() {
        return parse_vector_literal(text).ok();
    }
    value.as_array().and_then(|items| {
        let values = items
            .iter()
            .map(json_vector_number_to_i16)
            .collect::<Option<Vec<_>>>()?;
        (!values.is_empty()).then_some(values)
    })
}

fn json_vector_number_to_i16(value: &Value) -> Option<i16> {
    if let Some(value) = value.as_i64() {
        return i16::try_from(value).ok();
    }
    let value = value.as_f64()?;
    if !value.is_finite() {
        return None;
    }
    let scaled = (value * f64::from(i16::MAX)).round();
    Some(scaled.clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16)
}

pub(super) fn payload_has_vector(payload: &[u8]) -> bool {
    String::from_utf8_lossy(payload).lines().any(|line| {
        line.strip_prefix("vector=")
            .is_some_and(|value| !value.trim().is_empty())
    })
}

pub(super) fn body_vector_from_payload(payload: &[u8]) -> Option<Vec<i16>> {
    String::from_utf8_lossy(payload)
        .lines()
        .find_map(|line| parse_vector_literal(line.trim().strip_prefix("vector=")?).ok())
}
