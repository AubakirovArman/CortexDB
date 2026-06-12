use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use args::{Args, BenchmarkRetrievalMode};
use cortex_aql::{
    default_weights, AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO,
};
use cortex_core::CellId;
use cortex_engine::search::{
    analyze_search_query, condition_payload_bonus, covered_requirement_ids,
    decompose_enterprise_rag_question, extract_query_conditions, map_query_to_scope,
    parse_vector_literal, route_policy_for_query, scope_mapping_payload_bonus, tokenize,
    SearchDiversityDiagnostics, SearchIndexes, SearchMode, SearchQuery, SearchQueryIntent,
    SearchRerankInput, SearchReranker, SearchResult, WeightedScoreReranker,
};
use cortex_engine::{scope_id, CellMetadata, Database, RetrievedCell};
use cortex_storage::indexes::LexicalIndex;
use document::{build_payload, extract_document_content};
use io::{read_json, read_jsonl, read_uuid_index, write_json, write_jsonl};
use retrieval::{BenchmarkMetadataIndex, BenchmarkRetrievalIndex};
use serde::Deserialize;
use serde_json::{json, Value};

mod args;
mod document;
mod io;
mod retrieval;

const ORACLE_FIELDS: &[&str] = &[
    "answer_facts",
    "expected_doc_ids",
    "gold_answer",
    "question_type",
    "source_types",
];
const CLEAN_PREFILTER_RETRIEVAL_FIELDS: &[&str] =
    &["answer", "document_ids", "question", "question_id"];
const MAX_SKIP_CHECKPOINT_ENGINE_DOCS: usize = 10_000;
const ENGINE_PREFILTER_SHORTLIST_LIMIT: usize = 128;
const ENGINE_PREFILTER_LEXICAL_HEAD_COUNT: usize = 4;
const ENGINE_PREFILTER_DEFAULT_DOC_LIMIT: usize = 6;
const ENGINE_PREFILTER_STRONG_EVIDENCE_SCORE: u32 = 32;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let started = Instant::now();
    let args = Args::parse(env::args().skip(1))?;
    let logger = RunLogger::new(started, args.log_file.clone(), args.status_file.clone())?;
    let mut report_metrics = RunReportMetrics::default();
    logger.log("start enterprise_rag_bench_retrieval");
    logger.status("startup", "running", "parsed arguments", None, None, &[]);
    if args.reset_db && args.db_root.exists() {
        logger.log(&format!("reset db root {}", args.db_root.display()));
        logger.status("startup", "running", "reset database root", None, None, &[]);
        fs::remove_dir_all(&args.db_root)
            .map_err(|error| format!("failed to reset {}: {error}", args.db_root.display()))?;
    }

    logger.log(&format!("load questions {}", args.questions.display()));
    logger.status(
        "load_questions",
        "running",
        "load questions",
        None,
        None,
        &[],
    );
    let questions = read_jsonl(&args.questions)?;
    logger.log(&format!("loaded {} questions", questions.len()));
    logger.status(
        "load_questions",
        "done",
        "questions loaded",
        Some(questions.len()),
        Some(questions.len()),
        &[],
    );
    if args.official_clean {
        reject_oracle_fields(&questions)?;
        logger.log("official-clean question guard passed");
    }
    logger.log(&format!("load uuid index {}", args.uuid_index.display()));
    logger.status(
        "load_corpus_index",
        "running",
        "load uuid index",
        None,
        None,
        &[],
    );
    let uuid_index = read_uuid_index(&args.uuid_index)?;
    logger.log(&format!("loaded {} corpus ids", uuid_index.len()));
    logger.status(
        "load_corpus_index",
        "done",
        "uuid index loaded",
        Some(uuid_index.len()),
        Some(uuid_index.len()),
        &[],
    );
    logger.log(&format!("open database {}", args.db_root.display()));
    logger.status("open_database", "running", "open CortexDB", None, None, &[]);
    let mut db =
        Database::open(&args.db_root).map_err(|error| format!("failed to open db: {error}"))?;
    logger.log("database open");
    logger.status("open_database", "done", "database open", None, None, &[]);

    let engine_search_mode = matches!(
        args.retrieval_mode,
        BenchmarkRetrievalMode::EngineKeyword
            | BenchmarkRetrievalMode::EngineHybrid
            | BenchmarkRetrievalMode::EngineHybridRerank
    );
    if args.skip_checkpoint
        && engine_search_mode
        && document_total(&args, uuid_index.len()) > MAX_SKIP_CHECKPOINT_ENGINE_DOCS
    {
        return Err(format!(
            "--skip-checkpoint {} is limited to <= {} documents until a persisted candidate source is available; use --max-documents for smoke tests or run against a checkpointed DB",
            args.retrieval_mode.as_str(),
            MAX_SKIP_CHECKPOINT_ENGINE_DOCS
        ));
    }

    let mut document_vectors = load_document_vectors(args.document_vectors.as_ref())?;
    if !document_vectors.is_empty() {
        logger.log(&format!(
            "loaded {} document vectors",
            document_vectors.len()
        ));
    }
    let external_prefilter_retrieval =
        load_external_prefilter_retrieval(args.prefilter_retrieval.as_deref())?;
    if let Some(prefilter_retrieval) = &external_prefilter_retrieval {
        logger.log(&format!(
            "loaded external clean prefilter retrieval rows={} questions={}",
            prefilter_retrieval.rows,
            prefilter_retrieval.by_question_id.len()
        ));
        logger.status(
            "load_prefilter_retrieval",
            "done",
            "external prefilter retrieval loaded",
            Some(prefilter_retrieval.rows),
            Some(prefilter_retrieval.rows),
            &[("questions", json!(prefilter_retrieval.by_question_id.len()))],
        );
    }
    let use_source_payload_prefilter = should_use_source_payload_prefilter(
        &args,
        engine_search_mode,
        &external_prefilter_retrieval,
    );
    report_metrics.source_payload_prefilter = use_source_payload_prefilter;
    if use_source_payload_prefilter {
        logger.log("enable source-payload prefilter path");
        logger.status(
            "source_payload_prefilter",
            "done",
            "source-payload prefilter path enabled",
            None,
            None,
            &[],
        );
    }
    let collect_ingest_lexical =
        args.skip_checkpoint && engine_search_mode && !use_source_payload_prefilter;
    let mut ingest_lexical = collect_ingest_lexical.then(LexicalIndex::default);

    if use_source_payload_prefilter {
        logger.log("skip corpus ingest; source-payload prefilter reads bounded documents directly");
        report_metrics.documents_indexed = 0;
        logger.status(
            "ingest",
            "skipped",
            "source-payload prefilter skips corpus ingest",
            Some(0),
            Some(0),
            &[],
        );
    } else if !args.skip_ingest {
        logger.log("begin corpus ingest");
        logger.status(
            "ingest",
            "running",
            "begin corpus ingest",
            Some(0),
            Some(document_total(&args, uuid_index.len())),
            &[],
        );
        let ingest_started = Instant::now();
        let indexed = ingest_documents(
            &mut db,
            &uuid_index,
            &mut document_vectors,
            ingest_lexical.as_mut(),
            &args,
            &logger,
        )?;
        report_metrics.documents_indexed = indexed;
        report_metrics.ingest_duration_ms =
            Some(round_ms(ingest_started.elapsed().as_secs_f64() * 1000.0));
        logger.log(&format!("ingested {indexed} documents"));
        logger.status(
            "ingest",
            "done",
            "corpus ingest done",
            Some(indexed),
            Some(document_total(&args, uuid_index.len())),
            &[],
        );
        if args.skip_checkpoint {
            logger.log("skip checkpoint");
            logger.status("checkpoint", "skipped", "skip checkpoint", None, None, &[]);
        } else {
            logger.log("begin checkpoint");
            logger.status(
                "checkpoint",
                "running",
                "checkpoint benchmark corpus",
                None,
                None,
                &[],
            );
            let checkpoint_started = Instant::now();
            db.checkpoint()
                .map_err(|error| format!("failed to checkpoint benchmark corpus: {error}"))?;
            report_metrics.checkpoint_duration_ms = Some(round_ms(
                checkpoint_started.elapsed().as_secs_f64() * 1000.0,
            ));
            logger.log(&format!("checkpointed {indexed} documents"));
            logger.status("checkpoint", "done", "checkpoint complete", None, None, &[]);
        }
    } else {
        logger.log("skip corpus ingest");
        report_metrics.documents_indexed = 0;
        logger.status("ingest", "skipped", "skip corpus ingest", None, None, &[]);
    }

    let query_vectors = load_query_vectors(args.query_vectors.as_ref())?;
    let vector_readiness = assess_vector_readiness(
        &db,
        &uuid_index,
        args.retrieval_mode,
        questions.len(),
        query_vectors.len(),
        document_vectors.len(),
        use_source_payload_prefilter,
    )?;
    if !vector_readiness.ready {
        logger.log(&format!(
            "vector readiness warning: {}",
            vector_readiness.warnings.join("; ")
        ));
    }
    logger.status(
        "vector_readiness",
        if vector_readiness.ready {
            "done"
        } else {
            "warning"
        },
        "checked vector readiness",
        Some(vector_readiness.sampled_documents_with_vectors),
        Some(vector_readiness.sampled_documents),
        &[
            (
                "required_for_mode",
                json!(vector_readiness.required_for_mode),
            ),
            (
                "document_vector_sample_coverage_pct",
                json!(vector_readiness.document_vector_sample_coverage_pct),
            ),
        ],
    );
    report_metrics.vector_readiness = Some(vector_readiness);
    logger.log("begin question retrieval");
    logger.status(
        "retrieve_questions",
        "running",
        "begin question retrieval",
        Some(0),
        Some(questions.len()),
        &[],
    );
    let retrieval_started = Instant::now();
    let metadata_index = BenchmarkMetadataIndex::from_uuid_index(&uuid_index);
    let ingest_prefilter_index = ingest_lexical
        .take()
        .map(|lexical| BenchmarkRetrievalIndex::from_lexical(lexical, &uuid_index));
    let checkpoint_search_prefilter_index = if engine_search_mode
        && ingest_prefilter_index.is_none()
        && !args.skip_checkpoint
        && !args.disable_search_prefilter
        && !use_source_payload_prefilter
    {
        logger.log("load cached lexical prefilter index for engine search retrieval");
        Some(BenchmarkRetrievalIndex::load(&db, &uuid_index)?)
    } else {
        None
    };
    let search_prefilter_index = if args.disable_search_prefilter {
        None
    } else {
        ingest_prefilter_index
            .as_ref()
            .or(checkpoint_search_prefilter_index.as_ref())
    };
    let rows = if let Some(rows) =
        retrieve_metadata_only_questions(&metadata_index, &questions, &args, &logger)?
    {
        rows
    } else {
        let mut source_payload_prefilter = if use_source_payload_prefilter {
            Some(SourcePayloadPrefilter::new(
                &uuid_index,
                args.sources_dir.clone(),
            )?)
        } else {
            None
        };
        match args.retrieval_mode {
            BenchmarkRetrievalMode::CachedLexical => {
                logger.log("load cached lexical retrieval index from checkpoint segments");
                logger.status(
                    "load_retrieval_index",
                    "running",
                    "load cached lexical retrieval index",
                    None,
                    None,
                    &[],
                );
                let retrieval_index = BenchmarkRetrievalIndex::load(&db, &uuid_index)?;
                logger.log("cached lexical retrieval index loaded");
                logger.status(
                    "load_retrieval_index",
                    "done",
                    "cached lexical retrieval index loaded",
                    None,
                    None,
                    &[],
                );
                retrieve_cached_questions(&retrieval_index, &questions, &args, &logger)?
            }
            BenchmarkRetrievalMode::EngineAql => {
                logger.log("load cached lexical prefilter index for AQL retrieval");
                let retrieval_index = BenchmarkRetrievalIndex::load(&db, &uuid_index)?;
                retrieve_aql_questions(
                    &db,
                    &retrieval_index,
                    &uuid_index,
                    &questions,
                    &query_vectors,
                    &args,
                    &logger,
                )?
            }
            BenchmarkRetrievalMode::EngineKeyword
            | BenchmarkRetrievalMode::EngineHybrid
            | BenchmarkRetrievalMode::EngineHybridRerank => {
                let output = retrieve_search_questions(
                    &db,
                    &uuid_index,
                    &questions,
                    &query_vectors,
                    SearchPrefilterContext {
                        index: search_prefilter_index,
                        source_payloads: source_payload_prefilter.as_mut(),
                        document_vectors: &mut document_vectors,
                        external_retrieval: external_prefilter_retrieval.as_ref(),
                    },
                    &args,
                    &logger,
                )?;
                report_metrics.diversity = output.diversity;
                output.rows
            }
        }
    };
    report_metrics.retrieval_duration_ms =
        Some(round_ms(retrieval_started.elapsed().as_secs_f64() * 1000.0));
    report_metrics.questions_retrieved = rows.len();
    logger.log(&format!("retrieved {} question rows", rows.len()));
    logger.status(
        "retrieve_questions",
        "done",
        "question retrieval done",
        Some(rows.len()),
        Some(questions.len()),
        &[],
    );
    logger.log(&format!("write retrieval {}", args.output.display()));
    logger.status(
        "write_outputs",
        "running",
        "write retrieval output",
        None,
        None,
        &[],
    );
    write_jsonl(&args.output, &rows)?;
    if let Some(report) = &args.report {
        logger.log(&format!("write report {}", report.display()));
        report_metrics.total_duration_ms = round_ms(started.elapsed().as_secs_f64() * 1000.0);
        write_json(
            report,
            &report_payload(&questions, &uuid_index, &args, &report_metrics),
        )?;
    }
    logger.log("finished enterprise_rag_bench_retrieval");
    logger.status(
        "write_outputs",
        "done",
        "retrieval output written",
        Some(rows.len()),
        Some(questions.len()),
        &[
            ("output", json!(args.output.display().to_string())),
            (
                "report",
                json!(args.report.as_ref().map(|path| path.display().to_string())),
            ),
        ],
    );
    Ok(())
}

struct RunLogger {
    started: Instant,
    log_file: Option<PathBuf>,
    status_file: Option<PathBuf>,
}

#[derive(Default)]
struct RunReportMetrics {
    total_duration_ms: f64,
    documents_indexed: usize,
    ingest_duration_ms: Option<f64>,
    checkpoint_duration_ms: Option<f64>,
    questions_retrieved: usize,
    retrieval_duration_ms: Option<f64>,
    source_payload_prefilter: bool,
    vector_readiness: Option<VectorReadinessReport>,
    diversity: DiversityRunMetrics,
}

#[derive(Default)]
struct VectorReadinessReport {
    required_for_mode: bool,
    ready: bool,
    query_vector_rows: usize,
    document_vector_rows_loaded: usize,
    external_document_vectors_available: bool,
    sampled_documents: usize,
    sampled_documents_with_vectors: usize,
    document_vector_sample_coverage_pct: f64,
    warnings: Vec<String>,
}

struct SearchRetrievalOutput {
    rows: Vec<Value>,
    diversity: DiversityRunMetrics,
}

#[derive(Default)]
struct DiversityRunMetrics {
    reports: usize,
    diversity_enabled_questions: usize,
    input_candidates: usize,
    output_candidates: usize,
    skipped_candidates: usize,
    selected_with_payload_similarity: usize,
    selected_with_cluster_similarity: usize,
    max_payload_similarity_q16: u64,
    max_cluster_similarity_q16: u64,
    by_intent: BTreeMap<String, DiversityIntentMetrics>,
}

#[derive(Default)]
struct DiversityIntentMetrics {
    reports: usize,
    diversity_enabled_questions: usize,
    input_candidates: usize,
    output_candidates: usize,
    skipped_candidates: usize,
    selected_with_payload_similarity: usize,
    selected_with_cluster_similarity: usize,
    max_payload_similarity_q16: u64,
    max_cluster_similarity_q16: u64,
}

impl DiversityRunMetrics {
    fn record(&mut self, diagnostics: &SearchDiversityDiagnostics) {
        self.reports += 1;
        if diagnostics.diversity_enabled {
            self.diversity_enabled_questions += 1;
        }
        self.input_candidates = self
            .input_candidates
            .saturating_add(diagnostics.input_candidates);
        self.output_candidates = self
            .output_candidates
            .saturating_add(diagnostics.output_candidates);
        self.skipped_candidates = self
            .skipped_candidates
            .saturating_add(diagnostics.skipped_candidates);
        self.selected_with_payload_similarity = self
            .selected_with_payload_similarity
            .saturating_add(diagnostics.selected_with_payload_similarity);
        self.selected_with_cluster_similarity = self
            .selected_with_cluster_similarity
            .saturating_add(diagnostics.selected_with_cluster_similarity);
        self.max_payload_similarity_q16 = self
            .max_payload_similarity_q16
            .max(diagnostics.max_payload_similarity_q16);
        self.max_cluster_similarity_q16 = self
            .max_cluster_similarity_q16
            .max(diagnostics.max_cluster_similarity_q16);

        let intent = diagnostics.intent.as_str().to_owned();
        self.by_intent
            .entry(intent)
            .or_default()
            .record(diagnostics);
    }

    fn to_json(&self) -> Value {
        json!({
            "reports": self.reports,
            "diversity_enabled_questions": self.diversity_enabled_questions,
            "input_candidates": self.input_candidates,
            "output_candidates": self.output_candidates,
            "skipped_candidates": self.skipped_candidates,
            "selected_with_payload_similarity": self.selected_with_payload_similarity,
            "selected_with_cluster_similarity": self.selected_with_cluster_similarity,
            "max_payload_similarity_q16": self.max_payload_similarity_q16,
            "max_cluster_similarity_q16": self.max_cluster_similarity_q16,
            "by_intent": self.by_intent.iter().map(|(intent, metrics)| {
                (intent.clone(), metrics.to_json())
            }).collect::<serde_json::Map<_, _>>(),
        })
    }
}

impl DiversityIntentMetrics {
    fn record(&mut self, diagnostics: &SearchDiversityDiagnostics) {
        self.reports += 1;
        if diagnostics.diversity_enabled {
            self.diversity_enabled_questions += 1;
        }
        self.input_candidates = self
            .input_candidates
            .saturating_add(diagnostics.input_candidates);
        self.output_candidates = self
            .output_candidates
            .saturating_add(diagnostics.output_candidates);
        self.skipped_candidates = self
            .skipped_candidates
            .saturating_add(diagnostics.skipped_candidates);
        self.selected_with_payload_similarity = self
            .selected_with_payload_similarity
            .saturating_add(diagnostics.selected_with_payload_similarity);
        self.selected_with_cluster_similarity = self
            .selected_with_cluster_similarity
            .saturating_add(diagnostics.selected_with_cluster_similarity);
        self.max_payload_similarity_q16 = self
            .max_payload_similarity_q16
            .max(diagnostics.max_payload_similarity_q16);
        self.max_cluster_similarity_q16 = self
            .max_cluster_similarity_q16
            .max(diagnostics.max_cluster_similarity_q16);
    }

    fn to_json(&self) -> Value {
        json!({
            "reports": self.reports,
            "diversity_enabled_questions": self.diversity_enabled_questions,
            "input_candidates": self.input_candidates,
            "output_candidates": self.output_candidates,
            "skipped_candidates": self.skipped_candidates,
            "selected_with_payload_similarity": self.selected_with_payload_similarity,
            "selected_with_cluster_similarity": self.selected_with_cluster_similarity,
            "max_payload_similarity_q16": self.max_payload_similarity_q16,
            "max_cluster_similarity_q16": self.max_cluster_similarity_q16,
        })
    }
}

struct DocumentVectorLookup {
    offsets: BTreeMap<String, u64>,
    file: Option<File>,
}

#[derive(Deserialize)]
struct DocumentVectorIdRow {
    doc_id: String,
}

impl DocumentVectorLookup {
    fn empty() -> Self {
        Self {
            offsets: BTreeMap::new(),
            file: None,
        }
    }

    fn open(path: &std::path::Path) -> Result<Self, String> {
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

    fn is_empty(&self) -> bool {
        self.offsets.is_empty()
    }

    fn len(&self) -> usize {
        self.offsets.len()
    }

    fn get(&mut self, doc_id: &str) -> Result<Option<Vec<i16>>, String> {
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

struct BenchmarkSearchIndex {
    indexes: SearchIndexes,
    vectors: BTreeMap<u32, Vec<i16>>,
    candidate_to_cell: BTreeMap<u32, CellId>,
}

impl BenchmarkSearchIndex {
    fn load(
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
            let Some(payload) = db.get_latest_cell(cell_id) else {
                continue;
            };
            let metadata = CellMetadata::from_payload(&payload);
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

    fn search_payloads(
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
                payload: payload.as_deref(),
            });
        }
        results.sort_by_key(|result| (std::cmp::Reverse(result.score), result.cell_id));
    }
}

fn vector_dot_score(query: &[i16], candidate: &[i16]) -> u64 {
    query
        .iter()
        .zip(candidate)
        .fold(0i128, |score, (left, right)| {
            score + i128::from(*left) * i128::from(*right)
        })
        .max(0)
        .min(i128::from(u64::MAX)) as u64
}

impl RunLogger {
    fn new(
        started: Instant,
        log_file: Option<PathBuf>,
        status_file: Option<PathBuf>,
    ) -> Result<Self, String> {
        if let Some(path) = &log_file {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
            }
            fs::write(path, "")
                .map_err(|error| format!("failed to initialize {}: {error}", path.display()))?;
        }
        if let Some(path) = &status_file {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
            }
        }
        Ok(Self {
            started,
            log_file,
            status_file,
        })
    }

    fn log(&self, message: &str) {
        let line = format!(
            "[enterprise-rag-retrieval +{:>6.1}s] {message}",
            self.started.elapsed().as_secs_f64()
        );
        eprintln!("{line}");
        if let Some(path) = &self.log_file {
            if let Ok(mut handle) = OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(handle, "{line}");
            }
        }
    }

    fn status(
        &self,
        stage: &str,
        state: &str,
        message: &str,
        completed: Option<usize>,
        total: Option<usize>,
        extra: &[(&str, Value)],
    ) {
        let Some(path) = &self.status_file else {
            return;
        };
        let elapsed_seconds = self.started.elapsed().as_secs_f64();
        let mut payload = json!({
            "schema_version": "cortexdb.enterprise_rag_bench.retrieval_progress_status.v1",
            "stage": stage,
            "state": state,
            "message": message,
            "elapsed_seconds": (elapsed_seconds * 10.0).round() / 10.0,
            "updated_unix_ms": now_unix_ms(),
            "log_file": self.log_file.as_ref().map(|path| path.display().to_string()),
        });
        if let Some(completed) = completed {
            payload["completed"] = json!(completed);
        }
        if let Some(total) = total {
            payload["total"] = json!(total);
        }
        if let (Some(completed), Some(total)) = (completed, total) {
            let progress_pct = if total > 0 {
                completed as f64 / total as f64 * 100.0
            } else {
                100.0
            };
            payload["progress_pct"] = json!((progress_pct * 100.0).round() / 100.0);
        }
        if let Some(object) = payload.as_object_mut() {
            for (key, value) in extra {
                object.insert((*key).to_owned(), value.clone());
            }
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(&payload) {
            let mut with_newline = bytes;
            with_newline.push(b'\n');
            let _ = fs::write(path, with_newline);
        }
    }
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn document_total(args: &Args, corpus_len: usize) -> usize {
    args.max_documents
        .map(|max_documents| max_documents.min(corpus_len))
        .unwrap_or(corpus_len)
}

fn logger_progress_due(completed: usize, total: usize, every: usize) -> bool {
    every > 0 && (completed.is_multiple_of(every) || completed == total)
}

fn throughput_per_sec(units: usize, duration_ms: Option<f64>) -> f64 {
    let Some(duration_ms) = duration_ms else {
        return 0.0;
    };
    if duration_ms <= 0.0 {
        return 0.0;
    }
    round_ms(units as f64 / (duration_ms / 1000.0))
}

fn round_ms(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn round_pct(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn ingest_documents(
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

fn retrieve_cached_questions(
    retrieval_index: &BenchmarkRetrievalIndex,
    questions: &[Value],
    args: &Args,
    logger: &RunLogger,
) -> Result<Vec<Value>, String> {
    let mut rows = Vec::with_capacity(questions.len());
    for (index, question) in questions.iter().enumerate() {
        let qid = required_str(question, "question_id", index)?;
        let query = required_str(question, "question", index)?;
        let source_types = if args.official_clean {
            Vec::new()
        } else {
            source_types(question)
        };
        let mut seen = BTreeSet::<String>::new();
        let mut doc_ids = Vec::<Value>::new();
        for doc_id in retrieval_index.search_doc_ids(query, &source_types, args.top_k) {
            if seen.insert(doc_id.clone()) {
                doc_ids.push(Value::String(doc_id));
            }
        }
        let mut row = json!({
            "question_id": qid,
            "question": query,
            "answer": "",
            "document_ids": doc_ids,
        });
        if !args.official_clean {
            row["question_type"] = Value::String(
                question
                    .get("question_type")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_owned(),
            );
        }
        rows.push(row);
        if args.progress_every > 0
            && ((index + 1).is_multiple_of(args.progress_every)
                || questions.len() <= args.progress_every)
        {
            logger.log(&format!("retrieved {}/{}", index + 1, questions.len()));
            logger.status(
                "retrieve_questions",
                "running",
                "retrieve cached question rows",
                Some(index + 1),
                Some(questions.len()),
                &[("last_question_id", json!(qid))],
            );
        }
    }
    Ok(rows)
}

fn retrieve_metadata_only_questions(
    metadata_index: &BenchmarkMetadataIndex,
    questions: &[Value],
    args: &Args,
    logger: &RunLogger,
) -> Result<Option<Vec<Value>>, String> {
    if !args.official_clean || questions.is_empty() {
        return Ok(None);
    }
    let mut rows = Vec::with_capacity(questions.len());
    for (index, question) in questions.iter().enumerate() {
        let qid = required_str(question, "question_id", index)?;
        let query = required_str(question, "question", index)?;
        let doc_ids = metadata_index.search_doc_ids(query, args.top_k);
        if doc_ids.is_empty() {
            return Ok(None);
        }
        rows.push(json!({
            "question_id": qid,
            "question": query,
            "answer": "",
            "document_ids": doc_ids.into_iter().map(Value::String).collect::<Vec<_>>(),
        }));
    }
    logger.log("metadata-only retrieval satisfied all clean questions");
    logger.status(
        "retrieve_questions",
        "running",
        "metadata-only retrieval satisfied all clean questions",
        Some(rows.len()),
        Some(questions.len()),
        &[],
    );
    Ok(Some(rows))
}

fn retrieve_aql_questions(
    db: &Database,
    retrieval_index: &BenchmarkRetrievalIndex,
    uuid_index: &BTreeMap<String, String>,
    questions: &[Value],
    query_vectors: &BTreeMap<String, Vec<i16>>,
    args: &Args,
    logger: &RunLogger,
) -> Result<Vec<Value>, String> {
    let mut rows = Vec::with_capacity(questions.len());
    let doc_to_cell = doc_id_to_cell_id(uuid_index)?;
    for (index, question) in questions.iter().enumerate() {
        let qid = required_str(question, "question_id", index)?;
        let query = required_str(question, "question", index)?;
        let vector = query_vectors.get(qid);
        let cells = retrieval_index
            .search_doc_ids(query, &[], args.top_k.max(64))
            .iter()
            .filter_map(|doc_id| doc_to_cell.get(doc_id).copied())
            .filter_map(|cell_id| {
                db.get_latest_cell(cell_id)
                    .map(|payload| RetrievedCell { cell_id, payload })
            })
            .collect::<Vec<_>>();
        let mode = if vector.is_some() {
            RetrievalMode::Semantic
        } else {
            RetrievalMode::Balanced
        };
        let task = benchmark_task(query, vector);
        let results = db
            .rerank_retrieved_cells_for_task(cells, &task, &default_weights(mode))
            .into_iter()
            .take(args.top_k)
            .collect::<Vec<_>>();
        rows.push(retrieval_row(
            qid,
            query,
            results.into_iter().map(|result| result.payload),
        ));
        if args.progress_every > 0
            && ((index + 1).is_multiple_of(args.progress_every)
                || questions.len() <= args.progress_every)
        {
            logger.log(&format!("retrieved {}/{}", index + 1, questions.len()));
            logger.status(
                "retrieve_questions",
                "running",
                "retrieve AQL question rows",
                Some(index + 1),
                Some(questions.len()),
                &[("last_question_id", json!(qid))],
            );
        }
    }
    Ok(rows)
}

fn doc_id_to_cell_id(
    uuid_index: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, CellId>, String> {
    uuid_index
        .keys()
        .enumerate()
        .map(|(index, doc_id)| {
            let id = u64::try_from(index + 1).map_err(|_| "cell id overflow".to_owned())?;
            Ok((doc_id.clone(), CellId(id)))
        })
        .collect()
}

#[derive(Clone)]
struct PrefilterCandidate {
    cell_id: CellId,
    payload: Vec<u8>,
    score: u64,
    lexical_score: u64,
    vector_score: u64,
    evidence_score: u32,
}

struct PrefilterSearchOutput {
    payloads: Vec<Vec<u8>>,
    diversity_diagnostics: Option<SearchDiversityDiagnostics>,
}

struct SearchPrefilterContext<'a> {
    index: Option<&'a BenchmarkRetrievalIndex>,
    source_payloads: Option<&'a mut SourcePayloadPrefilter>,
    document_vectors: &'a mut DocumentVectorLookup,
    external_retrieval: Option<&'a ExternalPrefilterRetrieval>,
}

#[derive(Debug)]
struct ExternalPrefilterRetrieval {
    by_question_id: BTreeMap<String, Vec<String>>,
    rows: usize,
}

impl ExternalPrefilterRetrieval {
    fn doc_ids(&self, question_id: &str) -> Option<&[String]> {
        self.by_question_id.get(question_id).map(Vec::as_slice)
    }
}

struct SourcePayloadPrefilter {
    rel_paths: BTreeMap<String, String>,
    doc_to_cell: BTreeMap<String, CellId>,
    sources_dir: PathBuf,
    payload_cache: BTreeMap<String, Vec<u8>>,
}

impl SourcePayloadPrefilter {
    fn new(uuid_index: &BTreeMap<String, String>, sources_dir: PathBuf) -> Result<Self, String> {
        Ok(Self {
            rel_paths: uuid_index.clone(),
            doc_to_cell: doc_id_to_cell_id(uuid_index)?,
            sources_dir,
            payload_cache: BTreeMap::new(),
        })
    }

    fn candidate(
        &mut self,
        doc_id: &str,
        rank: usize,
        shortlist_limit: usize,
        query: SearchQuery<'_>,
        document_vectors: &mut DocumentVectorLookup,
    ) -> Result<Option<PrefilterCandidate>, String> {
        let Some(cell_id) = self.doc_to_cell.get(doc_id).copied() else {
            return Ok(None);
        };
        let payload = self.payload(doc_id, document_vectors)?.clone();
        let lexical_score = lexical_rank_score(rank, shortlist_limit);
        let vector_score = query
            .vector
            .and_then(|query_vector| {
                let payload_vector = body_vector_from_payload(&payload);
                let candidate_vector =
                    payload_vector.or_else(|| document_vectors.get(doc_id).ok().flatten())?;
                Some(vector_dot_score(query_vector, &candidate_vector))
            })
            .unwrap_or(0);
        Ok(Some(PrefilterCandidate {
            cell_id,
            evidence_score: prefilter_evidence_score(query.text, &payload),
            payload,
            score: lexical_score.saturating_add(vector_score),
            lexical_score,
            vector_score,
        }))
    }

    fn payload(
        &mut self,
        doc_id: &str,
        document_vectors: &mut DocumentVectorLookup,
    ) -> Result<&Vec<u8>, String> {
        if !self.payload_cache.contains_key(doc_id) {
            let rel_path = self
                .rel_paths
                .get(doc_id)
                .ok_or_else(|| format!("prefilter doc_id={doc_id} is not in uuid index"))?;
            let document = read_json(&self.sources_dir.join(rel_path))?;
            let (title, content) = extract_document_content(&document);
            let vector = document_vectors.get(doc_id)?;
            let payload = build_payload(doc_id, rel_path, &title, &content, vector.as_deref());
            self.payload_cache
                .insert(doc_id.to_owned(), payload.into_bytes());
        }
        self.payload_cache
            .get(doc_id)
            .ok_or_else(|| format!("prefilter payload cache missed doc_id={doc_id}"))
    }
}

fn should_use_source_payload_prefilter(
    args: &Args,
    engine_search_mode: bool,
    external_prefilter_retrieval: &Option<ExternalPrefilterRetrieval>,
) -> bool {
    engine_search_mode
        && external_prefilter_retrieval.is_some()
        && !args.disable_search_prefilter
        && args.official_clean
}

fn source_prefilter_payloads(
    source_payloads: &mut SourcePayloadPrefilter,
    document_vectors: &mut DocumentVectorLookup,
    external_retrieval: &ExternalPrefilterRetrieval,
    question_id: &str,
    query: SearchQuery<'_>,
    top_k: usize,
    reranker: Option<&dyn SearchReranker>,
) -> Result<PrefilterSearchOutput, String> {
    let shortlist_limit = top_k.max(ENGINE_PREFILTER_SHORTLIST_LIMIT);
    let Some(doc_ids) = external_retrieval.doc_ids(question_id) else {
        return Ok(PrefilterSearchOutput {
            payloads: Vec::new(),
            diversity_diagnostics: if query.mode == SearchMode::HybridRerank {
                Some(empty_prefilter_diversity_diagnostics(query.text))
            } else {
                None
            },
        });
    };
    let mut candidates = Vec::new();
    for (rank, doc_id) in doc_ids.iter().take(shortlist_limit).enumerate() {
        if let Some(candidate) =
            source_payloads.candidate(doc_id, rank, shortlist_limit, query, document_vectors)?
        {
            candidates.push(candidate);
        }
    }
    Ok(select_prefilter_candidates(
        candidates, query, top_k, reranker,
    ))
}

fn search_prefilter_payloads(
    db: &Database,
    doc_to_cell: &BTreeMap<String, CellId>,
    prefilter: &mut SearchPrefilterContext<'_>,
    question_id: &str,
    query: SearchQuery<'_>,
    top_k: usize,
    reranker: Option<&dyn SearchReranker>,
) -> PrefilterSearchOutput {
    let Some(prefilter_index) = prefilter.index else {
        return PrefilterSearchOutput {
            payloads: Vec::new(),
            diversity_diagnostics: None,
        };
    };
    let shortlist_limit = top_k.max(ENGINE_PREFILTER_SHORTLIST_LIMIT);
    let doc_ids = prefilter_doc_ids(
        prefilter_index,
        prefilter.external_retrieval,
        question_id,
        query.text,
        shortlist_limit,
    );
    let candidates = doc_ids
        .into_iter()
        .enumerate()
        .filter_map(|(rank, doc_id)| {
            let cell_id = *doc_to_cell.get(&doc_id)?;
            let payload = db.get_latest_cell(cell_id)?;
            let lexical_score = lexical_rank_score(rank, shortlist_limit);
            let vector_score = query
                .vector
                .and_then(|query_vector| {
                    let payload_vector = body_vector_from_payload(&payload);
                    let candidate_vector = payload_vector
                        .or_else(|| prefilter.document_vectors.get(&doc_id).ok().flatten())?;
                    Some(vector_dot_score(query_vector, &candidate_vector))
                })
                .unwrap_or(0);
            Some(PrefilterCandidate {
                cell_id,
                evidence_score: prefilter_evidence_score(query.text, &payload),
                payload,
                score: lexical_score.saturating_add(vector_score),
                lexical_score,
                vector_score,
            })
        })
        .collect::<Vec<_>>();
    select_prefilter_candidates(candidates, query, top_k, reranker)
}

fn select_prefilter_candidates(
    mut candidates: Vec<PrefilterCandidate>,
    query: SearchQuery<'_>,
    top_k: usize,
    reranker: Option<&dyn SearchReranker>,
) -> PrefilterSearchOutput {
    let mut lexical_candidates = candidates.clone();
    lexical_candidates.sort_by_key(|candidate| {
        (
            std::cmp::Reverse(candidate.lexical_score),
            candidate.cell_id.0,
        )
    });
    if let Some(reranker) = reranker {
        for candidate in &mut candidates {
            candidate.score = reranker.rerank_score(SearchRerankInput {
                query_text: query.text,
                query_vector: query.vector,
                candidate_id: candidate.cell_id.0,
                lexical_score: candidate.lexical_score,
                vector_score: candidate.vector_score,
                base_score: candidate.score,
                payload: Some(&candidate.payload),
            });
        }
    }
    candidates.sort_by_key(|candidate| (std::cmp::Reverse(candidate.score), candidate.cell_id.0));
    let (selected, diversity_diagnostics) = if query.mode == SearchMode::HybridRerank {
        let selection =
            select_diverse_prefilter_candidates(lexical_candidates, candidates, query.text, top_k);
        (selection.candidates, Some(selection.diagnostics))
    } else {
        (
            merge_prefilter_candidates(query.text, lexical_candidates, candidates, top_k),
            None,
        )
    };
    PrefilterSearchOutput {
        payloads: selected
            .into_iter()
            .map(|candidate| candidate.payload)
            .collect(),
        diversity_diagnostics,
    }
}

fn empty_prefilter_diversity_diagnostics(query_text: &str) -> SearchDiversityDiagnostics {
    let route_policy = route_policy_for_query(query_text);
    SearchDiversityDiagnostics {
        intent: cortex_engine::search::classify_search_query_intent(query_text),
        diversity_enabled: route_policy.diversity,
        lambda_q16: route_policy.diversity_lambda_q16,
        input_candidates: 0,
        output_candidates: 0,
        skipped_candidates: 0,
        max_payload_similarity_q16: 0,
        max_cluster_similarity_q16: 0,
        selected_with_payload_similarity: 0,
        selected_with_cluster_similarity: 0,
    }
}

fn prefilter_doc_ids(
    prefilter_index: &BenchmarkRetrievalIndex,
    external_retrieval: Option<&ExternalPrefilterRetrieval>,
    question_id: &str,
    query_text: &str,
    shortlist_limit: usize,
) -> Vec<String> {
    let mut doc_ids = Vec::with_capacity(shortlist_limit);
    let mut seen = BTreeSet::new();
    if let Some(external) = external_retrieval.and_then(|retrieval| retrieval.doc_ids(question_id))
    {
        for doc_id in external.iter().take(shortlist_limit) {
            if seen.insert(doc_id.clone()) {
                doc_ids.push(doc_id.clone());
            }
        }
    }
    if doc_ids.len() < shortlist_limit {
        let source_hints = inferred_source_types_from_query(query_text);
        for doc_id in prefilter_index.search_doc_ids(query_text, &source_hints, shortlist_limit) {
            if seen.insert(doc_id.clone()) {
                doc_ids.push(doc_id);
            }
            if doc_ids.len() >= shortlist_limit {
                break;
            }
        }
    }
    doc_ids
}

fn lexical_rank_score(rank: usize, shortlist_limit: usize) -> u64 {
    u64::try_from(shortlist_limit.saturating_sub(rank))
        .unwrap_or(0)
        .saturating_mul(1_000_000)
}

fn merge_prefilter_candidates(
    query_text: &str,
    lexical_candidates: Vec<PrefilterCandidate>,
    reranked_candidates: Vec<PrefilterCandidate>,
    top_k: usize,
) -> Vec<PrefilterCandidate> {
    let lexical_head = top_k.min(prefilter_lexical_head_count(query_text, top_k));
    let vector_promotions = top_k.saturating_sub(lexical_head);
    let pool_limit = top_k.saturating_mul(4).max(top_k);
    let mut selected = Vec::with_capacity(pool_limit);
    let mut seen = BTreeSet::new();
    push_unique_prefilter_candidates(
        &mut selected,
        &mut seen,
        lexical_candidates.iter().take(lexical_head),
        pool_limit,
    );
    push_unique_prefilter_candidates(
        &mut selected,
        &mut seen,
        reranked_candidates.iter().take(vector_promotions),
        pool_limit,
    );
    push_unique_prefilter_candidates(
        &mut selected,
        &mut seen,
        lexical_candidates.iter().skip(lexical_head),
        pool_limit,
    );
    push_unique_prefilter_candidates(
        &mut selected,
        &mut seen,
        reranked_candidates.iter().skip(vector_promotions),
        pool_limit,
    );
    prune_weak_prefilter_tail(query_text, selected, top_k)
}

fn push_unique_prefilter_candidates<'a>(
    selected: &mut Vec<PrefilterCandidate>,
    seen: &mut BTreeSet<u64>,
    candidates: impl Iterator<Item = &'a PrefilterCandidate>,
    top_k: usize,
) {
    for candidate in candidates {
        if selected.len() >= top_k {
            break;
        }
        if seen.insert(candidate.cell_id.0) {
            selected.push(candidate.clone());
        }
    }
}

fn prefilter_lexical_head_count(query_text: &str, top_k: usize) -> usize {
    let decomposition = decompose_enterprise_rag_question(query_text);
    let multipart = decomposition.requirements.len() > 1;
    let head = match cortex_engine::search::classify_search_query_intent(query_text) {
        SearchQueryIntent::Lookup
        | SearchQueryIntent::InfoNotFound
        | SearchQueryIntent::Constrained => ENGINE_PREFILTER_LEXICAL_HEAD_COUNT,
        SearchQueryIntent::Semantic if multipart => top_k,
        SearchQueryIntent::Semantic => ENGINE_PREFILTER_LEXICAL_HEAD_COUNT,
        SearchQueryIntent::ProjectRelated
        | SearchQueryIntent::HighLevel
        | SearchQueryIntent::ConflictingInfo
        | SearchQueryIntent::Completeness => top_k,
    };
    top_k.min(head.max(1))
}

fn prune_weak_prefilter_tail(
    query_text: &str,
    candidates: Vec<PrefilterCandidate>,
    top_k: usize,
) -> Vec<PrefilterCandidate> {
    let default_limit = top_k.min(prefilter_default_doc_limit(query_text, top_k));
    if candidates.len() <= default_limit {
        return candidates;
    }
    candidates
        .into_iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            if index < default_limit
                || candidate.evidence_score >= ENGINE_PREFILTER_STRONG_EVIDENCE_SCORE
            {
                Some(candidate)
            } else {
                None
            }
        })
        .take(top_k)
        .collect()
}

struct PrefilterDiverseSelection {
    candidates: Vec<PrefilterCandidate>,
    diagnostics: SearchDiversityDiagnostics,
}

fn select_diverse_prefilter_candidates(
    protected_order_candidates: Vec<PrefilterCandidate>,
    mut candidates: Vec<PrefilterCandidate>,
    query_text: &str,
    top_k: usize,
) -> PrefilterDiverseSelection {
    let route_policy = route_policy_for_query(query_text);
    // This path starts from an already-clean external retrieval row. It may
    // reorder or diversify the bounded candidate pool, but it must not shrink
    // the row because full-500 retrieval promotion is judged on document
    // recall@top_k. Adaptive answer/context caps belong to the product search
    // and ContextPack paths, not to this retrieval-only gate.
    let result_limit = top_k;
    let mut diagnostics = SearchDiversityDiagnostics {
        intent: cortex_engine::search::classify_search_query_intent(query_text),
        diversity_enabled: route_policy.diversity,
        lambda_q16: route_policy.diversity_lambda_q16,
        input_candidates: candidates.len(),
        output_candidates: 0,
        skipped_candidates: 0,
        max_payload_similarity_q16: 0,
        max_cluster_similarity_q16: 0,
        selected_with_payload_similarity: 0,
        selected_with_cluster_similarity: 0,
    };
    if candidates.len() <= result_limit {
        diagnostics.output_candidates = candidates.len();
        return PrefilterDiverseSelection {
            candidates,
            diagnostics,
        };
    }
    if !route_policy.diversity {
        candidates.truncate(result_limit);
        diagnostics.output_candidates = candidates.len();
        diagnostics.skipped_candidates = diagnostics
            .input_candidates
            .saturating_sub(diagnostics.output_candidates);
        return PrefilterDiverseSelection {
            candidates,
            diagnostics,
        };
    }

    let mut selected = Vec::<PrefilterCandidate>::with_capacity(result_limit);
    let protected_head =
        prefilter_diversity_protected_head_count(diagnostics.intent, query_text, result_limit);
    if protected_head > 0 {
        let mut seen = BTreeSet::new();
        push_unique_prefilter_candidates(
            &mut selected,
            &mut seen,
            protected_order_candidates.iter().take(protected_head),
            result_limit,
        );
        candidates.retain(|candidate| !seen.contains(&candidate.cell_id.0));
    }
    while !candidates.is_empty() && selected.len() < result_limit {
        let mut best = None::<(usize, PrefilterDiversitySimilarity, u64)>;
        for (index, candidate) in candidates.iter().enumerate() {
            let similarity = prefilter_diversity_similarity_q16(candidate, &selected);
            diagnostics.max_payload_similarity_q16 = diagnostics
                .max_payload_similarity_q16
                .max(similarity.payload_q16);
            diagnostics.max_cluster_similarity_q16 = diagnostics
                .max_cluster_similarity_q16
                .max(similarity.cluster_q16);
            let score = prefilter_mmr_diversity_score(
                candidate.score,
                similarity.max_q16(),
                route_policy.diversity_lambda_q16,
            );
            if best
                .as_ref()
                .is_none_or(|(_, _, best_score)| score > *best_score)
            {
                best = Some((index, similarity, score));
            }
        }
        let (best_index, best_similarity, _) =
            best.unwrap_or((0, PrefilterDiversitySimilarity::default(), 0));
        if !selected.is_empty() && best_similarity.payload_q16 > 0 {
            diagnostics.selected_with_payload_similarity += 1;
        }
        if !selected.is_empty() && best_similarity.cluster_q16 > 0 {
            diagnostics.selected_with_cluster_similarity += 1;
        }
        selected.push(candidates.remove(best_index));
    }
    diagnostics.output_candidates = selected.len();
    diagnostics.skipped_candidates = diagnostics
        .input_candidates
        .saturating_sub(diagnostics.output_candidates);
    PrefilterDiverseSelection {
        candidates: selected,
        diagnostics,
    }
}

fn prefilter_diversity_protected_head_count(
    intent: SearchQueryIntent,
    query_text: &str,
    result_limit: usize,
) -> usize {
    let head = match intent {
        SearchQueryIntent::Lookup
        | SearchQueryIntent::InfoNotFound
        | SearchQueryIntent::Constrained => 0,
        SearchQueryIntent::ConflictingInfo => 2,
        SearchQueryIntent::Completeness => result_limit / 2,
        SearchQueryIntent::HighLevel => 3,
        SearchQueryIntent::ProjectRelated | SearchQueryIntent::Semantic => result_limit,
    };
    if decompose_enterprise_rag_question(query_text)
        .requirements
        .len()
        > 3
    {
        return head.max(result_limit / 2).min(result_limit);
    }
    head.min(result_limit)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PrefilterDiversitySimilarity {
    payload_q16: u64,
    cluster_q16: u64,
}

impl PrefilterDiversitySimilarity {
    fn max_q16(self) -> u64 {
        self.payload_q16.max(self.cluster_q16)
    }
}

fn prefilter_diversity_similarity_q16(
    candidate: &PrefilterCandidate,
    selected: &[PrefilterCandidate],
) -> PrefilterDiversitySimilarity {
    selected
        .iter()
        .map(|existing| PrefilterDiversitySimilarity {
            payload_q16: prefilter_payload_jaccard_q16(&candidate.payload, &existing.payload),
            cluster_q16: prefilter_metadata_cluster_similarity_q16(
                &candidate.payload,
                &existing.payload,
            ),
        })
        .max_by_key(|similarity| similarity.max_q16())
        .unwrap_or_default()
}

fn prefilter_mmr_diversity_score(score: u64, similarity_q16: u64, lambda_q16: u16) -> u64 {
    let relevance = u128::from(score).saturating_mul(u128::from(lambda_q16)) / 65_535;
    let diversity_weight = 65_535u16.saturating_sub(lambda_q16);
    let redundancy_penalty = u128::from(score)
        .saturating_mul(u128::from(diversity_weight))
        .saturating_mul(u128::from(similarity_q16))
        / (65_535u128 * 65_535u128);
    u64::try_from(relevance.saturating_sub(redundancy_penalty)).unwrap_or(u64::MAX)
}

fn prefilter_payload_jaccard_q16(left: &[u8], right: &[u8]) -> u64 {
    let left = prefilter_payload_terms(left);
    let right = prefilter_payload_terms(right);
    if left.is_empty() || right.is_empty() {
        return 0;
    }
    let intersection = left.intersection(&right).count() as u64;
    let union = left.union(&right).count() as u64;
    intersection.saturating_mul(65_535) / union.max(1)
}

fn prefilter_payload_terms(payload: &[u8]) -> BTreeSet<String> {
    CellMetadata::from_payload(payload)
        .terms
        .into_iter()
        .filter(|term| term.len() >= 3)
        .collect()
}

fn prefilter_metadata_cluster_similarity_q16(left: &[u8], right: &[u8]) -> u64 {
    let left = CellMetadata::from_payload(left);
    let right = CellMetadata::from_payload(right);
    let mut score = 0;
    score = score.max(prefilter_matching_cluster_score(
        left.content_hash.as_deref(),
        right.content_hash.as_deref(),
        65_535,
    ));
    score = score.max(prefilter_matching_cluster_score(
        left.document_id.as_deref(),
        right.document_id.as_deref(),
        65_535,
    ));
    score = score.max(prefilter_matching_cluster_score(
        left.parent_id.as_deref(),
        right.parent_id.as_deref(),
        58_982,
    ));
    score = score.max(prefilter_matching_cluster_score(
        left.source_hash.as_deref(),
        right.source_hash.as_deref(),
        52_428,
    ));
    score = score.max(prefilter_matching_cluster_score(
        left.path.as_deref(),
        right.path.as_deref(),
        49_152,
    ));
    score = score.max(prefilter_matching_cluster_score(
        left.project.as_deref(),
        right.project.as_deref(),
        36_864,
    ));
    score = score.max(prefilter_matching_cluster_score(
        left.entity.as_deref(),
        right.entity.as_deref(),
        32_768,
    ));
    score = score.max(prefilter_matching_cluster_score(
        left.topic.as_deref(),
        right.topic.as_deref(),
        24_576,
    ));
    score = score.max(prefilter_matching_cluster_score(
        left.source.as_deref(),
        right.source.as_deref(),
        16_384,
    ));
    score
}

fn prefilter_matching_cluster_score(left: Option<&str>, right: Option<&str>, score: u64) -> u64 {
    match (left, right) {
        (Some(left), Some(right)) if !left.trim().is_empty() && left == right => score,
        _ => 0,
    }
}

fn prefilter_default_doc_limit(query_text: &str, top_k: usize) -> usize {
    let decomposition = decompose_enterprise_rag_question(query_text);
    let multipart = decomposition.requirements.len() > 1;
    let limit = match cortex_engine::search::classify_search_query_intent(query_text) {
        SearchQueryIntent::Lookup => ENGINE_PREFILTER_DEFAULT_DOC_LIMIT,
        SearchQueryIntent::InfoNotFound => 3,
        SearchQueryIntent::Constrained => ENGINE_PREFILTER_DEFAULT_DOC_LIMIT,
        SearchQueryIntent::Semantic if multipart => top_k,
        SearchQueryIntent::Semantic => ENGINE_PREFILTER_DEFAULT_DOC_LIMIT.saturating_add(1),
        SearchQueryIntent::ProjectRelated
        | SearchQueryIntent::HighLevel
        | SearchQueryIntent::ConflictingInfo
        | SearchQueryIntent::Completeness => top_k,
    };
    top_k.min(limit.max(1))
}

fn prefilter_evidence_score(query_text: &str, payload: &[u8]) -> u32 {
    let payload_text = String::from_utf8_lossy(payload);
    let payload_lower = payload_text.to_lowercase();
    let mut score = 0u32;
    let analyzed = analyze_search_query(query_text);
    for anchor in analyzed.anchors {
        if anchor.terms.iter().any(|term| payload_lower.contains(term)) {
            score = score.saturating_add(4);
        }
    }
    if analyzed
        .source_hints
        .iter()
        .any(|source| payload_lower.contains(source))
    {
        score = score.saturating_add(4);
    }
    let scope_mapping = map_query_to_scope(query_text);
    if scope_mapping_payload_bonus(&scope_mapping, payload) > 0 {
        score = score.saturating_add(4);
    }
    let conditions = extract_query_conditions(query_text);
    if condition_payload_bonus(&conditions, payload) > 0 {
        score = score.saturating_add(4);
    }
    let decomposition = decompose_enterprise_rag_question(query_text);
    score = score.saturating_add(
        u32::try_from(covered_requirement_ids(&decomposition, &payload_text).len())
            .unwrap_or(u32::MAX)
            .saturating_mul(2),
    );
    for term in tokenize(query_text)
        .into_iter()
        .filter(|term| is_prefilter_evidence_term(term))
    {
        if payload_lower.contains(&term) {
            score = score.saturating_add(1);
        }
    }
    score
}

fn is_prefilter_evidence_term(term: &str) -> bool {
    term.len() >= 3
        && !matches!(
            term,
            "who"
                | "what"
                | "which"
                | "where"
                | "when"
                | "why"
                | "how"
                | "give"
                | "tell"
                | "show"
                | "find"
                | "list"
                | "all"
                | "any"
                | "does"
                | "did"
                | "was"
                | "were"
                | "are"
                | "for"
                | "with"
                | "from"
                | "into"
                | "about"
                | "this"
                | "that"
                | "project"
                | "team"
                | "docs"
                | "document"
        )
}

fn inferred_source_types_from_query(query: &str) -> Vec<String> {
    let lower = query.to_lowercase();
    let mut values = Vec::<String>::new();
    let mut push = |source: &str| {
        if !values.iter().any(|value| value == source) {
            values.push(source.to_owned());
        }
    };
    if contains_query_marker(&lower, &["slack", "slack thread", "channel"]) {
        push("slack");
    }
    if contains_query_marker(&lower, &["gmail", "email", "mail thread", "customer email"]) {
        push("gmail");
    }
    if contains_query_marker(&lower, &["jira", "jira issue", "jira ticket"]) {
        push("jira");
    }
    if contains_query_marker(
        &lower,
        &["github", "pull request", "pr #", "repository", "repo"],
    ) {
        push("github");
    }
    if contains_query_marker(
        &lower,
        &["google drive", "drive doc", "drive document", "drive file"],
    ) {
        push("google_drive");
    }
    if contains_query_marker(&lower, &["linear", "linear issue"]) {
        push("linear");
    }
    if contains_query_marker(&lower, &["hubspot", "account note", "crm"]) {
        push("hubspot");
    }
    if contains_query_marker(
        &lower,
        &["fireflies", "meeting transcript", "call transcript"],
    ) {
        push("fireflies");
    }
    if contains_query_marker(&lower, &["confluence", "wiki page", "runbook", "adr"]) {
        push("confluence");
    }
    values
}

fn contains_query_marker(query: &str, markers: &[&str]) -> bool {
    markers
        .iter()
        .any(|marker| contains_query_marker_value(query, marker))
}

fn contains_query_marker_value(query: &str, marker: &str) -> bool {
    if marker.chars().any(|ch| !ch.is_ascii_alphanumeric()) {
        return query.contains(marker);
    }
    let mut start = 0usize;
    while let Some(relative) = query[start..].find(marker) {
        let index = start + relative;
        let end = index + marker.len();
        let before = query[..index].chars().next_back();
        let after = query[end..].chars().next();
        let left_boundary = before.is_none_or(|ch| !ch.is_ascii_alphanumeric());
        let right_boundary = after.is_none_or(|ch| !ch.is_ascii_alphanumeric());
        if left_boundary && right_boundary {
            return true;
        }
        start = end;
    }
    false
}

fn retrieve_search_questions(
    db: &Database,
    uuid_index: &BTreeMap<String, String>,
    questions: &[Value],
    query_vectors: &BTreeMap<String, Vec<i16>>,
    mut prefilter: SearchPrefilterContext<'_>,
    args: &Args,
    logger: &RunLogger,
) -> Result<SearchRetrievalOutput, String> {
    let mut rows = Vec::with_capacity(questions.len());
    let mut diversity = DiversityRunMetrics::default();
    let view = bench_view();
    let reranker = WeightedScoreReranker::default();
    let doc_to_cell = if prefilter.index.is_some() {
        Some(doc_id_to_cell_id(uuid_index)?)
    } else {
        None
    };
    let reusable_index = if args.skip_checkpoint && prefilter.index.is_none() {
        logger.log("build reusable search index for skip-checkpoint retrieval");
        logger.status(
            "build_reusable_search_index",
            "running",
            "build reusable in-memory search index",
            None,
            Some(uuid_index.len()),
            &[],
        );
        let started = Instant::now();
        let index = BenchmarkSearchIndex::load(db, uuid_index, &view, logger)?;
        logger.log(&format!(
            "reusable search index built in {} ms",
            round_ms(started.elapsed().as_secs_f64() * 1000.0)
        ));
        logger.status(
            "build_reusable_search_index",
            "done",
            "reusable in-memory search index built",
            Some(uuid_index.len()),
            Some(uuid_index.len()),
            &[],
        );
        Some(index)
    } else {
        None
    };
    for (index, question) in questions.iter().enumerate() {
        let qid = required_str(question, "question_id", index)?;
        let query = required_str(question, "question", index)?;
        let (mode, vector) = match args.retrieval_mode {
            BenchmarkRetrievalMode::EngineKeyword => (SearchMode::Keyword, None),
            BenchmarkRetrievalMode::EngineHybrid => {
                let vector = query_vectors.get(qid).ok_or_else(|| {
                    format!("engine-hybrid requires query vector for question_id={qid}")
                })?;
                (SearchMode::Hybrid, Some(vector.as_slice()))
            }
            BenchmarkRetrievalMode::EngineHybridRerank => {
                let vector = query_vectors.get(qid).ok_or_else(|| {
                    format!("engine-hybrid-rerank requires query vector for question_id={qid}")
                })?;
                (SearchMode::HybridRerank, Some(vector.as_slice()))
            }
            BenchmarkRetrievalMode::CachedLexical | BenchmarkRetrievalMode::EngineAql => {
                unreachable!("cached and AQL modes handled separately")
            }
        };
        let search_query = SearchQuery {
            text: query,
            vector,
            limit: args.top_k,
            mode,
        };
        let mode_label = args.retrieval_mode.as_str();
        logger.status(
            "retrieve_questions",
            "running",
            "retrieve engine question row",
            Some(index),
            Some(questions.len()),
            &[
                ("current_question_id", json!(qid)),
                ("retrieval_mode", json!(mode_label)),
                ("rerank_mode", json!(args.rerank_mode.as_str())),
            ],
        );
        logger.log(&format!(
            "retrieve engine question {}/{} question_id={} mode={} rerank={}",
            index + 1,
            questions.len(),
            qid,
            mode_label,
            args.rerank_mode.as_str()
        ));
        let question_started = Instant::now();
        let payloads = if let (Some(source_payloads), Some(external_retrieval)) = (
            prefilter.source_payloads.as_deref_mut(),
            prefilter.external_retrieval,
        ) {
            let output = source_prefilter_payloads(
                source_payloads,
                prefilter.document_vectors,
                external_retrieval,
                qid,
                search_query,
                args.top_k,
                args.rerank_mode.is_enabled().then_some(&reranker),
            )?;
            if let Some(diagnostics) = &output.diversity_diagnostics {
                diversity.record(diagnostics);
            }
            output.payloads
        } else if let (Some(_), Some(doc_to_cell)) = (prefilter.index, doc_to_cell.as_ref()) {
            let output = search_prefilter_payloads(
                db,
                doc_to_cell,
                &mut prefilter,
                qid,
                search_query,
                args.top_k,
                args.rerank_mode.is_enabled().then_some(&reranker),
            );
            if let Some(diagnostics) = &output.diversity_diagnostics {
                diversity.record(diagnostics);
            }
            output.payloads
        } else if let Some(reusable_index) = &reusable_index {
            reusable_index.search_payloads(
                db,
                search_query,
                args.top_k,
                args.rerank_mode.is_enabled().then_some(&reranker),
            )
        } else if args.rerank_mode.is_enabled() {
            db.search_cells_with_reranker(search_query, &view, &reranker)
                .map_err(|error| {
                    format!("engine {mode_label} rerank search failed for {qid}: {error}")
                })?
                .into_iter()
                .map(|result| result.payload)
                .collect()
        } else {
            let outcome = db
                .search_cells_with_report(search_query, &view)
                .map_err(|error| format!("engine {mode_label} search failed for {qid}: {error}"))?;
            if let Some(diagnostics) = &outcome.diversity_diagnostics {
                diversity.record(diagnostics);
            }
            outcome
                .results
                .into_iter()
                .map(|result| result.payload)
                .collect()
        };
        rows.push(retrieval_row(qid, query, payloads));
        let question_duration_ms = round_ms(question_started.elapsed().as_secs_f64() * 1000.0);
        if args.progress_every > 0
            && ((index + 1).is_multiple_of(args.progress_every)
                || questions.len() <= args.progress_every)
        {
            logger.log(&format!(
                "retrieved {}/{} last_question_ms={question_duration_ms}",
                index + 1,
                questions.len()
            ));
            logger.status(
                "retrieve_questions",
                "running",
                "retrieve engine question rows",
                Some(index + 1),
                Some(questions.len()),
                &[
                    ("last_question_id", json!(qid)),
                    ("last_question_ms", json!(question_duration_ms)),
                    ("retrieval_mode", json!(mode_label)),
                    ("rerank_mode", json!(args.rerank_mode.as_str())),
                ],
            );
        }
    }
    Ok(SearchRetrievalOutput { rows, diversity })
}

fn retrieval_row(qid: &str, query: &str, payloads: impl IntoIterator<Item = Vec<u8>>) -> Value {
    let mut seen = BTreeSet::new();
    let document_ids = payloads
        .into_iter()
        .filter_map(|payload| doc_id_from_payload(&payload))
        .filter(|doc_id| seen.insert(doc_id.clone()))
        .map(Value::String)
        .collect::<Vec<_>>();
    json!({
        "question_id": qid,
        "question": query,
        "answer": "",
        "document_ids": document_ids,
    })
}

#[cfg(test)]
fn build_benchmark_aql(query: &str, query_vector: Option<&Vec<i16>>, limit: usize) -> String {
    let task = benchmark_task(query, query_vector);
    let mode = if let Some(vector) = query_vector {
        let _ = vector;
        "semantic"
    } else {
        "balanced"
    };
    format!(
        "RETRIEVE CONTEXT FOR TASK {} IN BRAIN enterprise_rag USING MODE {mode} WHERE space = bench:enterprise_rag AND status = \"ready\" AND type = \"document_block\" LIMIT {} CANDIDATES;",
        quote_aql_string(&task),
        limit.max(1)
    )
}

fn benchmark_task(query: &str, query_vector: Option<&Vec<i16>>) -> String {
    let mut task = query.to_owned();
    if let Some(vector) = query_vector {
        task.push('\n');
        task.push_str("query_vector=");
        task.push_str(&vector_literal(vector));
    }
    task
}

fn vector_literal(vector: &[i16]) -> String {
    vector
        .iter()
        .map(i16::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
fn quote_aql_string(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => quoted.push_str("\\\\"),
            '"' => quoted.push_str("\\\""),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            '\t' => quoted.push_str("\\t"),
            ch => quoted.push(ch),
        }
    }
    quoted.push('"');
    quoted
}

fn report_payload(
    questions: &[Value],
    uuid_index: &BTreeMap<String, String>,
    args: &Args,
    metrics: &RunReportMetrics,
) -> Value {
    let mut by_type = BTreeMap::<String, usize>::new();
    if !args.official_clean {
        for question in questions {
            let question_type = question
                .get("question_type")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_owned();
            *by_type.entry(question_type).or_default() += 1;
        }
    }
    json!({
        "schema_version": "cortexdb.enterprise_rag_bench.retrieval_report.v1",
        "questions": questions.len(),
        "documents_indexed": args.max_documents.unwrap_or(uuid_index.len()),
        "top_k": args.top_k,
        "batch_size": args.batch_size,
        "official_clean": args.official_clean,
        "retrieval_mode": args.retrieval_mode.as_str(),
        "rerank_mode": args.rerank_mode.as_str(),
        "skip_ingest": args.skip_ingest,
        "skip_checkpoint": args.skip_checkpoint,
        "query_vectors": args.query_vectors.as_ref().map(|path| path.display().to_string()),
        "document_vectors": args.document_vectors.as_ref().map(|path| path.display().to_string()),
        "prefilter_retrieval": args.prefilter_retrieval.as_ref().map(|path| path.display().to_string()),
        "disable_search_prefilter": args.disable_search_prefilter,
        "source_payload_prefilter": metrics.source_payload_prefilter,
        "vector_readiness": metrics.vector_readiness.as_ref().map(|readiness| {
            json!({
                "required_for_mode": readiness.required_for_mode,
                "ready": readiness.ready,
                "query_vector_rows": readiness.query_vector_rows,
                "document_vector_rows_loaded": readiness.document_vector_rows_loaded,
                "external_document_vectors_available": readiness.external_document_vectors_available,
                "sampled_documents": readiness.sampled_documents,
                "sampled_documents_with_vectors": readiness.sampled_documents_with_vectors,
                "document_vector_sample_coverage_pct": readiness.document_vector_sample_coverage_pct,
                "warnings": &readiness.warnings,
            })
        }),
        "diversity": metrics.diversity.to_json(),
        "source_type_filter": !args.official_clean && args.retrieval_mode == BenchmarkRetrievalMode::CachedLexical,
        "performance": {
            "total_duration_ms": metrics.total_duration_ms,
            "ingest": {
                "documents_indexed": metrics.documents_indexed,
                "duration_ms": metrics.ingest_duration_ms,
                "throughput_docs_per_sec": throughput_per_sec(metrics.documents_indexed, metrics.ingest_duration_ms),
            },
            "checkpoint": {
                "duration_ms": metrics.checkpoint_duration_ms,
            },
            "retrieval": {
                "questions": metrics.questions_retrieved,
                "duration_ms": metrics.retrieval_duration_ms,
                "throughput_questions_per_sec": throughput_per_sec(metrics.questions_retrieved, metrics.retrieval_duration_ms),
            },
            "resource_usage": process_memory_report(),
        },
        "by_question_type": by_type,
        "output": args.output,
        "db_root": args.db_root,
        "runner": "cortex-engine-official-clean-retrieval",
    })
}

fn required_str<'a>(row: &'a Value, field: &str, index: usize) -> Result<&'a str, String> {
    row.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("row {} missing non-empty {field}", index + 1))
}

fn reject_oracle_fields(rows: &[Value]) -> Result<(), String> {
    for (index, row) in rows.iter().enumerate() {
        let Some(object) = row.as_object() else {
            continue;
        };
        let forbidden = ORACLE_FIELDS
            .iter()
            .copied()
            .filter(|field| object.contains_key(*field))
            .collect::<Vec<_>>();
        if !forbidden.is_empty() {
            return Err(format!(
                "official-clean question row {} has forbidden oracle fields: {}",
                index + 1,
                forbidden.join(", ")
            ));
        }
    }
    Ok(())
}

fn assess_vector_readiness(
    db: &Database,
    uuid_index: &BTreeMap<String, String>,
    retrieval_mode: BenchmarkRetrievalMode,
    question_count: usize,
    query_vector_rows: usize,
    document_vector_rows_loaded: usize,
    source_payload_prefilter: bool,
) -> Result<VectorReadinessReport, String> {
    let required_for_mode = matches!(
        retrieval_mode,
        BenchmarkRetrievalMode::EngineHybrid | BenchmarkRetrievalMode::EngineHybridRerank
    );
    let (sampled_documents, sampled_documents_with_vectors) = if source_payload_prefilter {
        (0, 0)
    } else {
        sample_document_vector_coverage(db, uuid_index, 2048)?
    };
    let document_vector_sample_coverage_pct = if sampled_documents > 0 {
        round_pct(sampled_documents_with_vectors as f64 / sampled_documents as f64 * 100.0)
    } else {
        0.0
    };
    let mut warnings = Vec::new();
    if required_for_mode && query_vector_rows < question_count {
        warnings.push(format!(
            "hybrid retrieval needs query vectors for all questions; loaded {query_vector_rows}/{question_count}"
        ));
    }
    if required_for_mode && source_payload_prefilter && document_vector_rows_loaded == 0 {
        warnings.push(
            "source-payload prefilter is active without document vectors; dense document scoring is unavailable".to_owned(),
        );
    } else if required_for_mode && sampled_documents == 0 {
        warnings.push(
            "hybrid retrieval could not sample indexed documents for vector coverage".to_owned(),
        );
    }
    if required_for_mode
        && sampled_documents > 0
        && sampled_documents_with_vectors == 0
        && document_vector_rows_loaded == 0
    {
        warnings.push(
            "hybrid retrieval selected but no document vectors were found in the sampled DB payloads"
                .to_owned(),
        );
    }
    if required_for_mode
        && sampled_documents > 0
        && sampled_documents_with_vectors > 0
        && sampled_documents_with_vectors < sampled_documents
    {
        warnings.push(format!(
            "hybrid retrieval has partial document vector sample coverage: {sampled_documents_with_vectors}/{sampled_documents}"
        ));
    }
    let ready = if required_for_mode {
        query_vector_rows >= question_count
            && (document_vector_rows_loaded > 0
                || (sampled_documents > 0 && sampled_documents_with_vectors == sampled_documents))
    } else {
        true
    };
    Ok(VectorReadinessReport {
        required_for_mode,
        ready,
        query_vector_rows,
        document_vector_rows_loaded,
        external_document_vectors_available: document_vector_rows_loaded > 0,
        sampled_documents,
        sampled_documents_with_vectors,
        document_vector_sample_coverage_pct,
        warnings,
    })
}

fn sample_document_vector_coverage(
    db: &Database,
    uuid_index: &BTreeMap<String, String>,
    limit: usize,
) -> Result<(usize, usize), String> {
    let cell_ids = doc_id_to_cell_id(uuid_index)?;
    let mut sampled = 0usize;
    let mut with_vectors = 0usize;
    for doc_id in uuid_index.keys().take(limit) {
        sampled += 1;
        let Some(cell_id) = cell_ids.get(doc_id) else {
            continue;
        };
        if db
            .get_latest_cell(*cell_id)
            .as_deref()
            .is_some_and(payload_has_vector)
        {
            with_vectors += 1;
        }
    }
    Ok((sampled, with_vectors))
}

fn payload_has_vector(payload: &[u8]) -> bool {
    String::from_utf8_lossy(payload).lines().any(|line| {
        line.strip_prefix("vector=")
            .is_some_and(|value| !value.trim().is_empty())
    })
}

fn body_vector_from_payload(payload: &[u8]) -> Option<Vec<i16>> {
    String::from_utf8_lossy(payload)
        .lines()
        .find_map(|line| parse_vector_literal(line.trim().strip_prefix("vector=")?).ok())
}

fn load_query_vectors(
    path: Option<&std::path::PathBuf>,
) -> Result<BTreeMap<String, Vec<i16>>, String> {
    load_id_vectors(path, "question_id")
}

fn load_document_vectors(
    path: Option<&std::path::PathBuf>,
) -> Result<DocumentVectorLookup, String> {
    let Some(path) = path else {
        return Ok(DocumentVectorLookup::empty());
    };
    DocumentVectorLookup::open(path)
}

fn load_external_prefilter_retrieval(
    path: Option<&Path>,
) -> Result<Option<ExternalPrefilterRetrieval>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let rows = read_jsonl(path)?;
    let mut by_question_id = BTreeMap::<String, Vec<String>>::new();
    for (index, row) in rows.iter().enumerate() {
        let row_no = index + 1;
        let object = row
            .as_object()
            .ok_or_else(|| format!("prefilter retrieval row {row_no} must be a JSON object"))?;
        reject_prefilter_oracle_fields(object, row_no)?;
        reject_unknown_prefilter_fields(object, row_no)?;
        validate_optional_prefilter_string(object, "question", row_no, false)?;
        validate_optional_prefilter_string(object, "answer", row_no, true)?;
        let question_id = object
            .get("question_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                format!("prefilter retrieval row {row_no} missing non-empty question_id")
            })?
            .to_owned();
        let document_ids = clean_prefilter_document_ids(object.get("document_ids"), row_no)?;
        if !document_ids.is_empty() {
            let existing = by_question_id.entry(question_id).or_default();
            let mut seen = existing.iter().cloned().collect::<BTreeSet<_>>();
            for doc_id in document_ids {
                if seen.insert(doc_id.clone()) {
                    existing.push(doc_id);
                }
            }
        }
    }
    Ok(Some(ExternalPrefilterRetrieval {
        by_question_id,
        rows: rows.len(),
    }))
}

fn reject_prefilter_oracle_fields(
    object: &serde_json::Map<String, Value>,
    row_no: usize,
) -> Result<(), String> {
    let forbidden = ORACLE_FIELDS
        .iter()
        .copied()
        .filter(|field| object.contains_key(*field))
        .collect::<Vec<_>>();
    if forbidden.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "prefilter retrieval row {row_no} has forbidden oracle fields: {}",
            forbidden.join(", ")
        ))
    }
}

fn reject_unknown_prefilter_fields(
    object: &serde_json::Map<String, Value>,
    row_no: usize,
) -> Result<(), String> {
    let unknown = object
        .keys()
        .filter(|field| !CLEAN_PREFILTER_RETRIEVAL_FIELDS.contains(&field.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if unknown.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "prefilter retrieval row {row_no} has unsupported fields: {}",
            unknown.join(", ")
        ))
    }
}

fn validate_optional_prefilter_string(
    object: &serde_json::Map<String, Value>,
    field: &str,
    row_no: usize,
    allow_empty: bool,
) -> Result<(), String> {
    let Some(value) = object.get(field) else {
        return Ok(());
    };
    let Some(text) = value.as_str() else {
        return Err(format!(
            "prefilter retrieval row {row_no} field {field} must be a string"
        ));
    };
    if !allow_empty && text.trim().is_empty() {
        return Err(format!(
            "prefilter retrieval row {row_no} field {field} must be non-empty"
        ));
    }
    Ok(())
}

fn clean_prefilter_document_ids(
    value: Option<&Value>,
    row_no: usize,
) -> Result<Vec<String>, String> {
    let value =
        value.ok_or_else(|| format!("prefilter retrieval row {row_no} missing document_ids"))?;
    let items = value.as_array().ok_or_else(|| {
        format!("prefilter retrieval row {row_no} field document_ids must be an array")
    })?;
    let mut seen = BTreeSet::new();
    let mut document_ids = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let doc_id = item
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                format!(
                    "prefilter retrieval row {row_no} document_ids[{}] must be a non-empty string",
                    index
                )
            })?;
        if seen.insert(doc_id.to_owned()) {
            document_ids.push(doc_id.to_owned());
        }
    }
    Ok(document_ids)
}

fn load_id_vectors(
    path: Option<&std::path::PathBuf>,
    id_field: &str,
) -> Result<BTreeMap<String, Vec<i16>>, String> {
    let Some(path) = path else {
        return Ok(BTreeMap::new());
    };
    let mut vectors = BTreeMap::new();
    for (index, row) in read_jsonl(path)?.into_iter().enumerate() {
        let id = required_str(&row, id_field, index)?.to_owned();
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

fn parse_query_vector(row: &Value) -> Option<Vec<i16>> {
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

fn doc_id_from_payload(payload: &[u8]) -> Option<String> {
    String::from_utf8_lossy(payload)
        .lines()
        .find_map(|line| line.strip_prefix("doc_id=").map(str::to_owned))
}

fn process_memory_report() -> Value {
    let (rss_bytes, peak_rss_bytes) = linux_proc_status_memory_bytes().unwrap_or((0, 0));
    json!({
        "rss_bytes": rss_bytes,
        "peak_rss_bytes": peak_rss_bytes,
    })
}

fn linux_proc_status_memory_bytes() -> Option<(u64, u64)> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let mut rss_bytes = 0;
    let mut peak_rss_bytes = 0;
    for line in status.lines() {
        if let Some(value) = line.strip_prefix("VmRSS:") {
            rss_bytes = parse_status_kib(value).unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("VmHWM:") {
            peak_rss_bytes = parse_status_kib(value).unwrap_or(0);
        }
    }
    Some((rss_bytes, peak_rss_bytes.max(rss_bytes)))
}

fn parse_status_kib(value: &str) -> Option<u64> {
    let kib = value.split_whitespace().next()?.parse::<u64>().ok()?;
    kib.checked_mul(1024)
}

fn bench_view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("bench:enterprise_rag")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced, RetrievalMode::Semantic]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 400,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}

fn source_types(row: &Value) -> Vec<String> {
    row.get("source_types")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    use cortex_core::CellId;
    use cortex_engine::search::{
        SearchDiversityDiagnostics, SearchMode, SearchQuery, SearchQueryIntent,
    };
    use cortex_engine::Database;
    use serde_json::json;

    use super::{
        bench_view, body_vector_from_payload, build_benchmark_aql, build_payload,
        doc_id_from_payload, doc_id_to_cell_id, inferred_source_types_from_query,
        load_document_vectors, load_external_prefilter_retrieval, load_id_vectors,
        merge_prefilter_candidates, parse_query_vector, parse_status_kib, payload_has_vector,
        quote_aql_string, reject_oracle_fields, select_diverse_prefilter_candidates,
        source_prefilter_payloads, throughput_per_sec, vector_dot_score, BenchmarkSearchIndex,
        DiversityRunMetrics, ExternalPrefilterRetrieval, PrefilterCandidate, RunLogger,
        SourcePayloadPrefilter, ENGINE_PREFILTER_STRONG_EVIDENCE_SCORE,
    };

    #[test]
    fn official_clean_rejects_question_type_and_source_types() {
        let rows = vec![json!({
            "question_id": "q1",
            "question": "What happened?",
            "question_type": "basic",
            "source_types": ["gmail"]
        })];

        let error = reject_oracle_fields(&rows).expect_err("oracle fields should fail");

        assert!(error.contains("question_type"));
        assert!(error.contains("source_types"));
    }

    #[test]
    fn official_clean_accepts_clean_question_rows() {
        let rows = vec![json!({
            "question_id": "q1",
            "question": "What happened?"
        })];

        reject_oracle_fields(&rows).expect("clean rows should pass");
    }

    #[test]
    fn diversity_run_metrics_json_groups_cluster_diagnostics_by_intent() {
        let mut metrics = DiversityRunMetrics::default();
        metrics.record(&SearchDiversityDiagnostics {
            intent: SearchQueryIntent::Completeness,
            diversity_enabled: true,
            lambda_q16: 36_864,
            input_candidates: 8,
            output_candidates: 5,
            skipped_candidates: 3,
            max_payload_similarity_q16: 12_000,
            max_cluster_similarity_q16: 65_535,
            selected_with_payload_similarity: 2,
            selected_with_cluster_similarity: 3,
        });

        let value = metrics.to_json();

        assert_eq!(value["reports"], 1);
        assert_eq!(value["diversity_enabled_questions"], 1);
        assert_eq!(value["skipped_candidates"], 3);
        assert_eq!(value["max_cluster_similarity_q16"], 65_535);
        assert_eq!(
            value["by_intent"]["completeness"]["selected_with_cluster_similarity"],
            3
        );
    }

    #[test]
    fn loads_external_prefilter_retrieval_clean_rows() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cortexdb-prefilter-clean-{unique}.jsonl"));
        fs::write(
            &path,
            [
                r#"{"question_id":"q1","question":"What happened?","answer":"","document_ids":["doc-a","doc-b","doc-a"]}"#,
                r#"{"question_id":"q1","question":"What happened?","answer":"","document_ids":["doc-c"]}"#,
            ]
            .join("\n")
                + "\n",
        )
        .expect("write prefilter jsonl");

        let retrieval = load_external_prefilter_retrieval(Some(&path))
            .expect("load prefilter")
            .unwrap();

        assert_eq!(retrieval.rows, 2);
        assert_eq!(
            retrieval.doc_ids("q1"),
            Some(["doc-a".to_owned(), "doc-b".to_owned(), "doc-c".to_owned()].as_slice())
        );
        fs::remove_file(path).ok();
    }

    #[test]
    fn external_prefilter_rejects_oracle_fields() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cortexdb-prefilter-oracle-{unique}.jsonl"));
        fs::write(
            &path,
            r#"{"question_id":"q1","question":"What happened?","document_ids":["doc-a"],"source_types":["gmail"]}"#,
        )
        .expect("write prefilter jsonl");

        let error =
            load_external_prefilter_retrieval(Some(&path)).expect_err("oracle field should fail");

        assert!(error.contains("forbidden oracle fields"));
        assert!(error.contains("source_types"));
        fs::remove_file(path).ok();
    }

    #[test]
    fn external_prefilter_rejects_unknown_fields() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cortexdb-prefilter-extra-{unique}.jsonl"));
        fs::write(
            &path,
            r#"{"question_id":"q1","question":"What happened?","document_ids":["doc-a"],"score":1}"#,
        )
        .expect("write prefilter jsonl");

        let error =
            load_external_prefilter_retrieval(Some(&path)).expect_err("unknown field should fail");

        assert!(error.contains("unsupported fields"));
        assert!(error.contains("score"));
        fs::remove_file(path).ok();
    }

    #[test]
    fn external_prefilter_allows_empty_document_ids_for_fallback() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cortexdb-prefilter-empty-{unique}.jsonl"));
        fs::write(
            &path,
            r#"{"question_id":"q1","question":"No answer?","answer":"","document_ids":[]}"#,
        )
        .expect("write prefilter jsonl");

        let retrieval = load_external_prefilter_retrieval(Some(&path))
            .expect("empty document ids should load")
            .unwrap();

        assert_eq!(retrieval.rows, 1);
        assert_eq!(retrieval.doc_ids("q1"), None);
        fs::remove_file(path).ok();
    }

    #[test]
    fn source_payload_prefilter_reads_source_docs_without_persisted_index() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cortexdb-source-prefilter-{unique}"));
        let sources = root.join("sources");
        fs::create_dir_all(sources.join("slack")).expect("create source dir");
        fs::write(
            sources.join("slack/doc-a.json"),
            json!({
                "title_field_name": "title",
                "content_field_names": ["body"],
                "title": "Alpha rollout",
                "body": "The alpha rollout mentions cache warmup and deployment status."
            })
            .to_string(),
        )
        .expect("write doc-a");
        fs::write(
            sources.join("slack/doc-b.json"),
            json!({
                "title_field_name": "title",
                "content_field_names": ["body"],
                "title": "Beta rollout",
                "body": "The beta rollout mentions source-payload prefiltering."
            })
            .to_string(),
        )
        .expect("write doc-b");
        let uuid_index = BTreeMap::from([
            ("doc-a".to_owned(), "slack/doc-a.json".to_owned()),
            ("doc-b".to_owned(), "slack/doc-b.json".to_owned()),
        ]);
        let external = ExternalPrefilterRetrieval {
            by_question_id: BTreeMap::from([(
                "q1".to_owned(),
                vec!["doc-b".to_owned(), "doc-a".to_owned()],
            )]),
            rows: 1,
        };
        let mut source_payloads =
            SourcePayloadPrefilter::new(&uuid_index, sources).expect("source prefilter");
        let mut document_vectors = super::DocumentVectorLookup::empty();
        let output = source_prefilter_payloads(
            &mut source_payloads,
            &mut document_vectors,
            &external,
            "q1",
            SearchQuery {
                text: "Which rollout mentions prefiltering?",
                vector: None,
                limit: 2,
                mode: SearchMode::HybridRerank,
            },
            2,
            None,
        )
        .expect("source prefilter output");
        let ids = output
            .payloads
            .iter()
            .filter_map(|payload| doc_id_from_payload(payload))
            .collect::<Vec<_>>();

        assert_eq!(ids, vec!["doc-b".to_owned(), "doc-a".to_owned()]);
        assert!(output.diversity_diagnostics.is_some());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn infers_source_types_from_question_text_without_oracle_fields() {
        assert_eq!(
            inferred_source_types_from_query(
                "Which Slack thread mentioned the GitHub PR for the Google Drive file?"
            ),
            vec![
                "slack".to_owned(),
                "github".to_owned(),
                "google_drive".to_owned()
            ]
        );
        assert_eq!(
            inferred_source_types_from_query("Which team owns the rollout decision?"),
            Vec::<String>::new()
        );
        assert_eq!(
            inferred_source_types_from_query(
                "What was the high-percentile latency concern reported after a smoke test?"
            ),
            Vec::<String>::new()
        );
    }

    #[test]
    fn parses_query_vector_from_string_or_array() {
        assert_eq!(
            parse_query_vector(&json!({"vector": "1,2,-3"})),
            Some(vec![1, 2, -3])
        );
        assert_eq!(
            parse_query_vector(&json!({"vector": [4, 5, 6]})),
            Some(vec![4, 5, 6])
        );
    }

    #[test]
    fn parses_query_vector_from_float_embedding_array() {
        assert_eq!(
            parse_query_vector(&json!({"vector": [0.0, 0.5, -0.5, 1.2, -1.2]})),
            Some(vec![0, 16_384, -16_384, i16::MAX, i16::MIN])
        );
    }

    #[test]
    fn rejects_query_vector_with_non_numeric_array_values() {
        assert_eq!(parse_query_vector(&json!({"vector": [0.1, null]})), None);
    }

    #[test]
    fn loads_id_vectors_from_jsonl() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cortexdb-vector-loader-{unique}.jsonl"));
        fs::write(
            &path,
            r#"{"doc_id":"doc-a","vector":[1,2,3]}"#.to_owned() + "\n",
        )
        .expect("write vector jsonl");

        let vectors = load_id_vectors(Some(&path), "doc_id").expect("load vectors");

        assert_eq!(vectors.get("doc-a"), Some(&vec![1, 2, 3]));
        fs::remove_file(path).ok();
    }

    #[test]
    fn loads_document_vectors_lazily_from_jsonl_offsets() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("cortexdb-document-vector-loader-{unique}.jsonl"));
        fs::write(
            &path,
            [
                r#"{"doc_id":"doc-a","vector":[0.0,0.5]}"#,
                r#"{"doc_id":"doc-b","vector":"1,-2,3"}"#,
            ]
            .join("\n")
                + "\n",
        )
        .expect("write document vector jsonl");

        let mut vectors = load_document_vectors(Some(&path)).expect("load document vectors");

        assert_eq!(vectors.len(), 2);
        assert_eq!(vectors.get("doc-a").expect("doc-a"), Some(vec![0, 16_384]));
        assert_eq!(vectors.get("doc-b").expect("doc-b"), Some(vec![1, -2, 3]));
        assert_eq!(vectors.get("missing").expect("missing"), None);
        fs::remove_file(path).ok();
    }

    #[test]
    fn extracts_doc_id_from_payload_metadata() {
        assert_eq!(
            doc_id_from_payload(b"scope=bench:enterprise_rag\ndoc_id=abc-123\n\nbody"),
            Some("abc-123".to_owned())
        );
    }

    #[test]
    fn payload_vector_detector_requires_non_empty_vector_metadata() {
        assert!(payload_has_vector(b"doc_id=doc-1\nvector=1,-2,3\n\nbody"));
        assert!(!payload_has_vector(b"doc_id=doc-1\nvector=  \n\nbody"));
        assert!(!payload_has_vector(
            b"doc_id=doc-1\nembedding=1,-2,3\n\nbody"
        ));
    }

    #[test]
    fn body_vector_helper_reads_body_vector_line() {
        assert_eq!(
            body_vector_from_payload(b"title_vector=1,0\nvector=0,1\n\nbody"),
            Some(vec![0, 1])
        );
        assert_eq!(body_vector_from_payload(b"title_vector=1,0\n\nbody"), None);
    }

    #[test]
    fn vector_dot_score_clamps_negative_scores() {
        assert_eq!(vector_dot_score(&[2, -1], &[3, 4]), 2);
        assert_eq!(vector_dot_score(&[1], &[-10]), 0);
    }

    #[test]
    fn prefilter_merge_preserves_lexical_head_then_promotes_reranked_candidates() {
        fn candidate(id: u64) -> PrefilterCandidate {
            PrefilterCandidate {
                cell_id: CellId(id),
                payload: format!("doc_id=doc-{id}\n\nbody").into_bytes(),
                score: id,
                lexical_score: id,
                vector_score: id,
                evidence_score: 0,
            }
        }
        let lexical = (1..=10).map(candidate).collect::<Vec<_>>();
        let reranked = [3, 20, 21, 22, 23, 24, 25, 26]
            .into_iter()
            .map(candidate)
            .collect::<Vec<_>>();

        let merged = merge_prefilter_candidates("Find invoice Q4", lexical, reranked, 10)
            .into_iter()
            .map(|candidate| candidate.cell_id.0)
            .collect::<Vec<_>>();

        assert_eq!(merged, vec![1, 2, 3, 4, 20, 21]);
    }

    #[test]
    fn prefilter_merge_keeps_strong_evidence_tail_candidates() {
        fn candidate(id: u64, evidence_score: u32) -> PrefilterCandidate {
            PrefilterCandidate {
                cell_id: CellId(id),
                payload: format!("doc_id=doc-{id}\n\nbody").into_bytes(),
                score: id,
                lexical_score: id,
                vector_score: id,
                evidence_score,
            }
        }
        let lexical = (1..=10).map(|id| candidate(id, 0)).collect::<Vec<_>>();
        let reranked = [
            candidate(20, 0),
            candidate(21, 0),
            candidate(22, 0),
            candidate(23, 0),
            candidate(24, 0),
            candidate(25, 0),
            candidate(26, ENGINE_PREFILTER_STRONG_EVIDENCE_SCORE),
            candidate(27, ENGINE_PREFILTER_STRONG_EVIDENCE_SCORE + 1),
        ];

        let merged = merge_prefilter_candidates("Find invoice Q4", lexical, reranked.to_vec(), 10)
            .into_iter()
            .map(|candidate| candidate.cell_id.0)
            .collect::<Vec<_>>();

        assert_eq!(merged, vec![1, 2, 3, 4, 20, 21, 26, 27]);
    }

    #[test]
    fn prefilter_hybrid_rerank_diversifies_metadata_clusters() {
        fn candidate(id: u64, score: u64, doc_id: &str, body: &str) -> PrefilterCandidate {
            PrefilterCandidate {
                cell_id: CellId(id),
                payload: format!("doc_id={doc_id}\n\n{body}").into_bytes(),
                score,
                lexical_score: score,
                vector_score: 0,
                evidence_score: 0,
            }
        }
        let candidates = vec![
            candidate(1, 100, "doc-a", "billing blocker owner deadline"),
            candidate(2, 99, "doc-a", "billing blocker owner deadline duplicate"),
            candidate(3, 80, "doc-b", "security blocker status dependency"),
        ];

        let selection = select_diverse_prefilter_candidates(
            candidates.clone(),
            candidates,
            "List all support, security, and billing requirements",
            2,
        );
        let selected = selection
            .candidates
            .iter()
            .map(|candidate| candidate.cell_id.0)
            .collect::<Vec<_>>();

        assert!(selection.diagnostics.diversity_enabled);
        assert_eq!(selection.diagnostics.input_candidates, 3);
        assert_eq!(selection.diagnostics.output_candidates, 2);
        assert_eq!(selection.diagnostics.skipped_candidates, 1);
        assert_eq!(selected, vec![1, 3]);
        assert!(selection.diagnostics.max_cluster_similarity_q16 > 0);
    }

    #[test]
    fn prefilter_diversity_gate_preserves_requested_top_k_for_lookup_rows() {
        fn candidate(id: u64) -> PrefilterCandidate {
            PrefilterCandidate {
                cell_id: CellId(id),
                payload: format!("doc_id=doc-{id}\n\nlookup evidence body").into_bytes(),
                score: 100 - id,
                lexical_score: 100 - id,
                vector_score: 0,
                evidence_score: 0,
            }
        }
        let candidates = (1..=10).map(candidate).collect::<Vec<_>>();

        let selection = select_diverse_prefilter_candidates(
            candidates.clone(),
            candidates,
            "Find invoice Q4",
            10,
        );

        assert!(!selection.diagnostics.diversity_enabled);
        assert_eq!(selection.candidates.len(), 10);
        assert_eq!(selection.diagnostics.output_candidates, 10);
    }

    #[test]
    fn prefilter_tail_limit_preserves_multi_evidence_queries_without_oracle_labels() {
        assert_eq!(super::prefilter_default_doc_limit("Find invoice Q4", 10), 6);
        assert_eq!(
            super::prefilter_default_doc_limit(
                "What caused the 429 spike and how do we verify it is not burning SLOs?",
                10,
            ),
            10
        );
        assert_eq!(
            super::prefilter_default_doc_limit(
                "List all Fireflies calls mentioning data residency",
                10
            ),
            10
        );
        assert_eq!(
            super::prefilter_default_doc_limit("What is the temporary kill switch name?", 10),
            7
        );
    }

    #[test]
    fn prefilter_lexical_head_preserves_clean_external_order_for_complex_queries() {
        assert_eq!(
            super::prefilter_lexical_head_count("Find invoice Q4", 10),
            4
        );
        assert_eq!(
            super::prefilter_lexical_head_count(
                "What caused the 429 spike and how do we verify it is not burning SLOs?",
                10,
            ),
            10
        );
        assert_eq!(
            super::prefilter_lexical_head_count(
                "List all Fireflies calls mentioning data residency",
                10
            ),
            10
        );
    }

    #[test]
    fn prefilter_evidence_score_rewards_anchors_conditions_and_terms() {
        let score = super::prefilter_evidence_score(
            "Which PR #42 fixed AUTH-123 before 2026-05-01?",
            b"source=github\n\nAUTH-123 was fixed by PR #42 on 2026-04-30.",
        );

        assert!(score >= ENGINE_PREFILTER_STRONG_EVIDENCE_SCORE);
    }

    #[test]
    fn reusable_index_bounded_hybrid_uses_vector_signal_inside_lexical_pool() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cortexdb-bounded-hybrid-{unique}"));
        fs::create_dir_all(&dir).expect("create temp db dir");
        let mut db = Database::open(&dir).expect("open db");
        let uuid_index = BTreeMap::from([
            ("doc-a".to_owned(), "slack/a.json".to_owned()),
            ("doc-b".to_owned(), "slack/b.json".to_owned()),
        ]);
        db.put_cells(vec![
            (
                CellId(1),
                build_payload(
                    "doc-a",
                    "slack/a.json",
                    "Shared",
                    "shared topic alpha",
                    Some(&[32_767, 0]),
                )
                .into_bytes(),
            ),
            (
                CellId(2),
                build_payload(
                    "doc-b",
                    "slack/b.json",
                    "Shared",
                    "shared topic beta",
                    Some(&[0, 32_767]),
                )
                .into_bytes(),
            ),
        ])
        .expect("put cells");
        let logger = RunLogger::new(Instant::now(), None, None).expect("logger");
        let index =
            BenchmarkSearchIndex::load(&db, &uuid_index, &bench_view(), &logger).expect("index");
        let query_vector = [0, 32_767];
        let payloads = index.search_payloads(
            &db,
            SearchQuery {
                text: "shared topic",
                vector: Some(&query_vector),
                limit: 1,
                mode: SearchMode::Hybrid,
            },
            1,
            None,
        );

        assert_eq!(payloads.len(), 1);
        assert_eq!(doc_id_from_payload(&payloads[0]), Some("doc-b".to_owned()));
        drop(index);
        drop(db);
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn doc_id_to_cell_id_uses_ingest_order() {
        let uuid_index = BTreeMap::from([
            ("doc-a".to_owned(), "slack/a.json".to_owned()),
            ("doc-b".to_owned(), "gmail/b.json".to_owned()),
        ]);

        let mapped = doc_id_to_cell_id(&uuid_index).expect("mapping");

        assert_eq!(mapped.get("doc-a"), Some(&CellId(1)));
        assert_eq!(mapped.get("doc-b"), Some(&CellId(2)));
    }

    #[test]
    fn benchmark_aql_uses_question_text_and_clean_filters() {
        let aql = build_benchmark_aql("What did \"Apollo\" ship?", None, 10);

        assert!(aql.contains("RETRIEVE CONTEXT FOR TASK"));
        assert!(aql.contains("\\\"Apollo\\\""));
        assert!(aql.contains("USING MODE balanced"));
        assert!(aql.contains("space = bench:enterprise_rag"));
        assert!(aql.contains("status = \"ready\""));
        assert!(aql.contains("type = \"document_block\""));
        assert!(aql.contains("LIMIT 10 CANDIDATES"));
        assert!(!aql.contains("question_type"));
        assert!(!aql.contains("source_types"));
        assert!(!aql.contains("expected_doc_ids"));
    }

    #[test]
    fn benchmark_aql_can_carry_query_vector_without_gold_fields() {
        let aql = build_benchmark_aql("semantic lookup", Some(&vec![1, -2, 3]), 5);

        assert!(aql.contains("USING MODE semantic"));
        assert!(aql.contains("query_vector=1,-2,3"));
        assert!(aql.contains("LIMIT 5 CANDIDATES"));
    }

    #[test]
    fn quote_aql_string_escapes_control_characters() {
        assert_eq!(
            quote_aql_string("a \"b\"\nnext\\tail"),
            r#""a \"b\"\nnext\\tail""#
        );
    }

    #[test]
    fn reports_throughput_and_linux_status_memory_bytes() {
        assert_eq!(throughput_per_sec(10, Some(500.0)), 20.0);
        assert_eq!(throughput_per_sec(10, None), 0.0);
        assert_eq!(parse_status_kib(" 123 kB"), Some(125_952));
    }
}
