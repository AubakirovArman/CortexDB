use std::collections::BTreeSet;

use cortex_aql::{
    default_weights, BitmapOp, BitmapProgram, BoundRetrievePlan, ContextPolicy, QualityThresholds,
    RetrievalMode,
};
use cortex_storage::manifest::{
    ManifestCount, ManifestSegment, ManifestSegmentStats, ManifestTermDocumentFrequency,
    StorageManifest,
};

use super::*;
use crate::query::metadata::{scope_handle, scope_id};
use crate::query::DatabaseStatistics;

#[test]
fn cost_model_prefers_bitmap_first_for_narrow_scope() {
    let scope = scope_id("project:a");
    let manifest = manifest_with_stats(vec![segment_stats(
        1,
        1_000,
        &[("project:a", 10), ("project:b", 990)],
        &[("budget", 50)],
    )]);
    let statistics = DatabaseStatistics::new(&manifest);
    let plan = plan(
        RetrievalMode::Balanced,
        "budget",
        BitmapProgram {
            ops: vec![BitmapOp::Push(scope_handle(scope))],
            max_stack_depth: 1,
        },
        50,
        8_000,
    );
    let provider = Provider::default();

    let decision = choose_retrieve_path(&plan, statistics, &provider, &CostModelOptions::default());

    assert_eq!(decision.selected_path, ExecutionPath::BitmapFirst);
    assert_eq!(decision.estimated_after_bitmap, Some(10));
    assert!(decision.reason.contains("bitmap"));
}

#[test]
fn cost_model_uses_agent_allowed_cardinality_for_permission_pruning() {
    let manifest = manifest_with_stats(vec![segment_stats(
        1,
        1_000,
        &[("project:a", 10), ("project:b", 990)],
        &[("budget", 900)],
    )]);
    let statistics = DatabaseStatistics::new(&manifest);
    let plan = plan(
        RetrievalMode::Balanced,
        "budget",
        BitmapProgram {
            ops: vec![
                BitmapOp::PushAgentAllowed,
                BitmapOp::PushLive,
                BitmapOp::And,
            ],
            max_stack_depth: 2,
        },
        50,
        8_000,
    );
    let provider = Provider {
        allowed: (0..10).collect(),
    };

    let decision = choose_retrieve_path(&plan, statistics, &provider, &CostModelOptions::default());

    assert_eq!(decision.selected_path, ExecutionPath::BitmapFirst);
    assert_eq!(decision.estimated_after_bitmap, Some(10));
    assert!(decision.reason.contains("bitmap"));
}

#[test]
fn cost_model_prefers_lexical_first_for_rare_term_on_broad_bitmap() {
    let manifest = manifest_with_stats(vec![segment_stats(
        1,
        1_000,
        &[("project:a", 1_000)],
        &[("needle", 3), ("common", 900)],
    )]);
    let statistics = DatabaseStatistics::new(&manifest);
    let plan = plan(
        RetrievalMode::Balanced,
        "needle common",
        BitmapProgram {
            ops: vec![BitmapOp::PushUniverse],
            max_stack_depth: 1,
        },
        50,
        8_000,
    );
    let provider = Provider::default();

    let decision = choose_retrieve_path(&plan, statistics, &provider, &CostModelOptions::default());

    assert_eq!(decision.selected_path, ExecutionPath::LexicalFirst);
    assert_eq!(
        decision.rarest_term,
        Some(TermDfEstimate {
            term: "needle".to_owned(),
            document_frequency: 3,
        })
    );
}

#[test]
fn cost_model_prefers_vector_first_for_wide_semantic_query_vector() {
    let manifest = manifest_with_stats(vec![segment_stats(
        1,
        1_000,
        &[("project:a", 1_000)],
        &[("shared", 900)],
    )]);
    let statistics = DatabaseStatistics::new(&manifest);
    let plan = plan(
        RetrievalMode::Semantic,
        "query_vector=1,0\nshared context",
        BitmapProgram {
            ops: vec![BitmapOp::PushUniverse],
            max_stack_depth: 1,
        },
        50,
        8_000,
    );
    let provider = Provider::default();

    let decision = choose_retrieve_path(&plan, statistics, &provider, &CostModelOptions::default());

    assert_eq!(decision.selected_path, ExecutionPath::VectorFirst);
    assert!(decision.has_query_vector);
}

#[test]
fn vector_search_planner_chooses_ann_with_exact_fallback_for_large_corpus() {
    let manifest = manifest_with_stats(vec![segment_stats(
        1,
        ANN_VECTOR_MIN_LIVE_ROWS,
        &[("project:a", ANN_VECTOR_MIN_LIVE_ROWS)],
        &[("shared", ANN_VECTOR_MIN_LIVE_ROWS)],
    )]);
    let statistics = DatabaseStatistics::new(&manifest);

    let decision = choose_vector_search_execution(statistics, None, true);

    assert_eq!(
        decision.execution,
        VectorSearchExecution::AnnWithExactFallback
    );
    assert_eq!(decision.estimated_live_rows, ANN_VECTOR_MIN_LIVE_ROWS);
    assert!(decision.reason.contains("fallback"));
}

#[test]
fn vector_search_planner_preserves_exact_when_hnsw_is_disabled() {
    let manifest = manifest_with_stats(vec![segment_stats(
        1,
        ANN_VECTOR_MIN_LIVE_ROWS * 2,
        &[("project:a", ANN_VECTOR_MIN_LIVE_ROWS * 2)],
        &[("shared", ANN_VECTOR_MIN_LIVE_ROWS * 2)],
    )]);
    let statistics = DatabaseStatistics::new(&manifest);

    let decision = choose_vector_search_execution(statistics, None, false);

    assert_eq!(decision.execution, VectorSearchExecution::Exact);
    assert!(decision.reason.contains("disabled"));
}

#[test]
fn vector_search_planner_keeps_exact_for_selective_candidates() {
    let manifest = manifest_with_stats(vec![segment_stats(
        1,
        ANN_VECTOR_MIN_LIVE_ROWS * 2,
        &[("project:a", ANN_VECTOR_MIN_LIVE_ROWS * 2)],
        &[("shared", ANN_VECTOR_MIN_LIVE_ROWS * 2)],
    )]);
    let statistics = DatabaseStatistics::new(&manifest);

    let decision = choose_vector_search_execution(statistics, Some(10), true);

    assert_eq!(decision.execution, VectorSearchExecution::Exact);
    assert_eq!(decision.estimated_candidate_rows, 10);
    assert!(decision.reason.contains("selective"));
}

#[test]
fn cost_model_applies_budget_candidate_limit_heuristic() {
    let manifest = manifest_with_stats(vec![segment_stats(
        1,
        100,
        &[("project:a", 100)],
        &[("common", 100)],
    )]);
    let statistics = DatabaseStatistics::new(&manifest);
    let plan = plan(
        RetrievalMode::Balanced,
        "common",
        BitmapProgram {
            ops: vec![BitmapOp::PushUniverse],
            max_stack_depth: 1,
        },
        100,
        640,
    );
    let provider = Provider::default();

    let decision = choose_retrieve_path(&plan, statistics, &provider, &CostModelOptions::default());

    assert_eq!(decision.recommended_candidate_limit, 4);
}

#[test]
fn cost_model_accepts_forced_debug_path() {
    let manifest = manifest_with_stats(vec![segment_stats(
        1,
        100,
        &[("project:a", 100)],
        &[("common", 100)],
    )]);
    let statistics = DatabaseStatistics::new(&manifest);
    let plan = plan(
        RetrievalMode::Balanced,
        "common",
        BitmapProgram {
            ops: vec![BitmapOp::PushUniverse],
            max_stack_depth: 1,
        },
        10,
        8_000,
    );
    let provider = Provider::default();
    let options = CostModelOptions {
        forced_path: Some(ExecutionPath::Hybrid),
    };

    let decision = choose_retrieve_path(&plan, statistics, &provider, &options);

    assert_eq!(decision.selected_path, ExecutionPath::Hybrid);
    assert!(decision.reason.contains("forced"));
}

#[derive(Default)]
struct Provider {
    allowed: BTreeSet<u32>,
}

impl cortex_aql::BitmapProvider for Provider {
    fn bitmap(&self, _handle: cortex_aql::BitmapHandle) -> Option<cortex_aql::RoaringBitmap> {
        None
    }

    fn agent_allowed(&self) -> cortex_aql::RoaringBitmap {
        self.allowed.iter().copied().collect()
    }

    fn live(&self) -> cortex_aql::RoaringBitmap {
        cortex_aql::RoaringBitmap::new()
    }

    fn universe(&self) -> cortex_aql::RoaringBitmap {
        cortex_aql::RoaringBitmap::new()
    }
}

fn plan(
    mode: RetrievalMode,
    task: &str,
    bitmap_program: BitmapProgram,
    candidate_limit: u32,
    budget_tokens: u32,
) -> BoundRetrievePlan {
    BoundRetrievePlan {
        brain_id: cortex_aql::BrainId(1),
        task: task.to_owned(),
        mode,
        bitmap_program,
        context_policy: ContextPolicy {
            budget_tokens,
            candidate_limit,
            require_citations: false,
        },
        quality_thresholds: QualityThresholds::default(),
        weights: default_weights(mode),
    }
}

fn manifest_with_stats(segment_stats: Vec<ManifestSegmentStats>) -> StorageManifest {
    StorageManifest {
        live_segments: segment_stats
            .iter()
            .map(|stats| ManifestSegment {
                id: stats.segment_id,
                generation: stats.segment_id,
                checkpoint_seq: stats.segment_id,
                cell_count: stats.row_count.try_into().unwrap_or(u32::MAX),
            })
            .collect(),
        segment_stats,
        ..StorageManifest::default()
    }
}

fn segment_stats(
    segment_id: u64,
    row_count: u64,
    scopes: &[(&str, u64)],
    terms: &[(&str, u64)],
) -> ManifestSegmentStats {
    ManifestSegmentStats {
        segment_id,
        row_count,
        scope_counts: counts(scopes),
        top_terms: terms
            .iter()
            .map(|(term, document_frequency)| ManifestTermDocumentFrequency {
                term: (*term).to_owned(),
                document_frequency: *document_frequency,
            })
            .collect(),
        ..ManifestSegmentStats::default()
    }
}

fn counts(values: &[(&str, u64)]) -> Vec<ManifestCount> {
    values
        .iter()
        .map(|(key, count)| ManifestCount {
            key: (*key).to_owned(),
            count: *count,
        })
        .collect()
}
