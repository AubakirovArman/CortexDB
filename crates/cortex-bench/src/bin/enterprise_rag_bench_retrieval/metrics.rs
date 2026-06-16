use std::collections::BTreeMap;

use cortex_engine::search::SearchDiversityDiagnostics;
use serde_json::{json, Value};

#[derive(Default)]
pub(super) struct RunReportMetrics {
    pub(super) total_duration_ms: f64,
    pub(super) documents_indexed: usize,
    pub(super) ingest_duration_ms: Option<f64>,
    pub(super) checkpoint_duration_ms: Option<f64>,
    pub(super) questions_retrieved: usize,
    pub(super) retrieval_duration_ms: Option<f64>,
    pub(super) source_payload_prefilter: bool,
    pub(super) vector_readiness: Option<VectorReadinessReport>,
    pub(super) diversity: DiversityRunMetrics,
}

#[derive(Default)]
pub(super) struct VectorReadinessReport {
    pub(super) required_for_mode: bool,
    pub(super) ready: bool,
    pub(super) query_vector_rows: usize,
    pub(super) document_vector_rows_loaded: usize,
    pub(super) external_document_vectors_available: bool,
    pub(super) sampled_documents: usize,
    pub(super) sampled_documents_with_vectors: usize,
    pub(super) document_vector_sample_coverage_pct: f64,
    pub(super) warnings: Vec<String>,
}

pub(super) struct SearchRetrievalOutput {
    pub(super) rows: Vec<Value>,
    pub(super) diversity: DiversityRunMetrics,
}

#[derive(Default)]
pub(super) struct DiversityRunMetrics {
    pub(super) reports: usize,
    pub(super) diversity_enabled_questions: usize,
    pub(super) input_candidates: usize,
    pub(super) output_candidates: usize,
    pub(super) skipped_candidates: usize,
    pub(super) selected_with_payload_similarity: usize,
    pub(super) selected_with_cluster_similarity: usize,
    pub(super) max_payload_similarity_q16: u64,
    pub(super) max_cluster_similarity_q16: u64,
    pub(super) by_intent: BTreeMap<String, DiversityIntentMetrics>,
}

#[derive(Default)]
pub(super) struct DiversityIntentMetrics {
    pub(super) reports: usize,
    pub(super) diversity_enabled_questions: usize,
    pub(super) input_candidates: usize,
    pub(super) output_candidates: usize,
    pub(super) skipped_candidates: usize,
    pub(super) selected_with_payload_similarity: usize,
    pub(super) selected_with_cluster_similarity: usize,
    pub(super) max_payload_similarity_q16: u64,
    pub(super) max_cluster_similarity_q16: u64,
}

impl DiversityRunMetrics {
    pub(super) fn record(&mut self, diagnostics: &SearchDiversityDiagnostics) {
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

    pub(super) fn to_json(&self) -> Value {
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
    pub(super) fn record(&mut self, diagnostics: &SearchDiversityDiagnostics) {
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

    pub(super) fn to_json(&self) -> Value {
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
