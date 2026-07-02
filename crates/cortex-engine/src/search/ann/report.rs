use super::types::{
    AnnFallbackReason, AnnSearchPath, AnnSearchPolicy, AnnSearchReport, AnnSloViolation,
};

const ANN_MIN_NODES_FOR_SLO_MULTI_LAYER: usize = 4;

pub(crate) fn finalize_report(
    mut report: AnnSearchReport,
    policy: AnnSearchPolicy,
) -> AnnSearchReport {
    report.fallback_performed = report.path == AnnSearchPath::ExactFallback;
    report.require_slo = policy.require_slo;
    report.slo_violations = slo_violations(&report, policy);
    report.production_safe = report.slo_violations.is_empty();
    report
}

fn slo_violations(report: &AnnSearchReport, policy: AnnSearchPolicy) -> Vec<AnnSloViolation> {
    let mut violations = Vec::new();
    if policy.require_slo
        && report.path == AnnSearchPath::HnswGraph
        && is_weak_multi_layer_topology(report)
    {
        violations.push(AnnSloViolation::WeakMultiLayerTopology);
    }
    if let Some(reason) = report.fallback_reason {
        violations.push(match reason {
            AnnFallbackReason::EmptyGraph => AnnSloViolation::EmptyGraph,
            AnnFallbackReason::InvalidGraph => AnnSloViolation::InvalidGraph,
            AnnFallbackReason::StaleGraph => AnnSloViolation::StaleGraph,
            AnnFallbackReason::InsufficientResults => AnnSloViolation::InsufficientResults,
            AnnFallbackReason::LowRecall => AnnSloViolation::LowRecall,
            AnnFallbackReason::VisitBudgetExceeded => AnnSloViolation::VisitBudgetExceeded,
            AnnFallbackReason::SparseAllowedSet => AnnSloViolation::SparseAllowedSet,
            AnnFallbackReason::NoPersistedSegments => AnnSloViolation::NoPersistedSegments,
            AnnFallbackReason::UncheckpointedChanges => AnnSloViolation::UncheckpointedChanges,
            AnnFallbackReason::HnswDisabled => AnnSloViolation::HnswDisabled,
        });
    }
    violations.retain(|violation| *violation != AnnSloViolation::HnswDisabled);
    if let (Some(recall), Some(min_recall)) = (report.recall_q16, policy.min_recall_q16) {
        if recall < min_recall && !violations.contains(&AnnSloViolation::LowRecall) {
            violations.push(AnnSloViolation::RecallBelowMinimum);
        }
    }
    violations
}

fn is_weak_multi_layer_topology(report: &AnnSearchReport) -> bool {
    if report.graph_nodes < ANN_MIN_NODES_FOR_SLO_MULTI_LAYER {
        return false;
    }
    if report.hnsw_layer_count <= 1 {
        return false;
    }
    report.upper_graph_edges == 0
}

pub(super) fn recall_q16(overlap_count: usize, expected_count: usize) -> u16 {
    if expected_count == 0 {
        return 65_535;
    }
    ((overlap_count as u64 * 65_535) / expected_count as u64) as u16
}
