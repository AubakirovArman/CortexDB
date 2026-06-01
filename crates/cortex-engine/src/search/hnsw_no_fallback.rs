use super::ann::{AnnSearchPath, AnnSearchPolicy, AnnSearchReport, MIN_ANN_RECALL_Q16};
use cortex_storage::manifest::ManifestHnswNoFallbackProfile;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HnswNoFallbackRolloutPolicy {
    pub rollout_enabled: bool,
    pub min_recall_q16: u16,
    pub require_upper_layers: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HnswNoFallbackBlockReason {
    RolloutDisabled,
    FallbackEnabled,
    SloNotRequired,
    NotHnswGraphPath,
    FallbackOccurred,
    FallbackReasonPresent,
    ReportNotProductionSafe,
    SloViolationsPresent,
    EmptyGraph,
    NoEligibleCandidates,
    NoResults,
    WeakMultiLayerTopology,
    RecallMissing,
    RecallBelowMinimum,
    SearchPolicyBelowRolloutMinimum,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HnswNoFallbackDecision {
    pub allowed: bool,
    pub reasons: Vec<HnswNoFallbackBlockReason>,
}

impl HnswNoFallbackRolloutPolicy {
    pub fn enabled() -> Self {
        Self {
            rollout_enabled: true,
            ..Self::default()
        }
    }

    pub fn from_manifest(profile: ManifestHnswNoFallbackProfile) -> Self {
        Self {
            rollout_enabled: profile.rollout_enabled,
            min_recall_q16: profile.min_recall_q16,
            require_upper_layers: profile.require_upper_layers,
        }
    }

    pub fn to_manifest(self) -> ManifestHnswNoFallbackProfile {
        ManifestHnswNoFallbackProfile {
            rollout_enabled: self.rollout_enabled,
            min_recall_q16: self.min_recall_q16,
            require_upper_layers: self.require_upper_layers,
        }
    }
}

impl Default for HnswNoFallbackRolloutPolicy {
    fn default() -> Self {
        Self {
            rollout_enabled: false,
            min_recall_q16: MIN_ANN_RECALL_Q16,
            require_upper_layers: true,
        }
    }
}

impl HnswNoFallbackBlockReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RolloutDisabled => "rollout_disabled",
            Self::FallbackEnabled => "fallback_enabled",
            Self::SloNotRequired => "slo_not_required",
            Self::NotHnswGraphPath => "not_hnsw_graph_path",
            Self::FallbackOccurred => "fallback_occurred",
            Self::FallbackReasonPresent => "fallback_reason_present",
            Self::ReportNotProductionSafe => "report_not_production_safe",
            Self::SloViolationsPresent => "slo_violations_present",
            Self::EmptyGraph => "empty_graph",
            Self::NoEligibleCandidates => "no_eligible_candidates",
            Self::NoResults => "no_results",
            Self::WeakMultiLayerTopology => "weak_multi_layer_topology",
            Self::RecallMissing => "recall_missing",
            Self::RecallBelowMinimum => "recall_below_minimum",
            Self::SearchPolicyBelowRolloutMinimum => "search_policy_below_rollout_minimum",
        }
    }
}

impl HnswNoFallbackDecision {
    pub fn allowed() -> Self {
        Self {
            allowed: true,
            reasons: Vec::new(),
        }
    }

    pub fn blocked(reasons: Vec<HnswNoFallbackBlockReason>) -> Self {
        Self {
            allowed: false,
            reasons,
        }
    }
}

pub fn evaluate_hnsw_no_fallback_rollout(
    rollout_policy: HnswNoFallbackRolloutPolicy,
    search_policy: AnnSearchPolicy,
    report: &AnnSearchReport,
) -> HnswNoFallbackDecision {
    let mut reasons = Vec::new();

    if !rollout_policy.rollout_enabled {
        reasons.push(HnswNoFallbackBlockReason::RolloutDisabled);
    }
    if search_policy.fallback {
        reasons.push(HnswNoFallbackBlockReason::FallbackEnabled);
    }
    if !search_policy.require_slo || !report.require_slo {
        reasons.push(HnswNoFallbackBlockReason::SloNotRequired);
    }
    if report.path != AnnSearchPath::HnswGraph {
        reasons.push(HnswNoFallbackBlockReason::NotHnswGraphPath);
    }
    if report.fallback_performed {
        reasons.push(HnswNoFallbackBlockReason::FallbackOccurred);
    }
    if report.fallback_reason.is_some() {
        reasons.push(HnswNoFallbackBlockReason::FallbackReasonPresent);
    }
    if !report.production_safe {
        reasons.push(HnswNoFallbackBlockReason::ReportNotProductionSafe);
    }
    if !report.slo_violations.is_empty() {
        reasons.push(HnswNoFallbackBlockReason::SloViolationsPresent);
    }
    if report.graph_nodes == 0 {
        reasons.push(HnswNoFallbackBlockReason::EmptyGraph);
    }
    if report.allowed_candidates == 0 {
        reasons.push(HnswNoFallbackBlockReason::NoEligibleCandidates);
    }
    if report.allowed_candidates > 0 && report.returned_candidates == 0 {
        reasons.push(HnswNoFallbackBlockReason::NoResults);
    }
    if rollout_policy.require_upper_layers && is_weak_multi_layer_report(report) {
        reasons.push(HnswNoFallbackBlockReason::WeakMultiLayerTopology);
    }
    match report.recall_q16 {
        Some(recall) if recall < rollout_policy.min_recall_q16 => {
            reasons.push(HnswNoFallbackBlockReason::RecallBelowMinimum);
        }
        Some(_) => {}
        None => reasons.push(HnswNoFallbackBlockReason::RecallMissing),
    }
    if search_policy
        .min_recall_q16
        .map(|value| value < rollout_policy.min_recall_q16)
        .unwrap_or(true)
    {
        reasons.push(HnswNoFallbackBlockReason::SearchPolicyBelowRolloutMinimum);
    }

    if reasons.is_empty() {
        HnswNoFallbackDecision::allowed()
    } else {
        HnswNoFallbackDecision::blocked(reasons)
    }
}

fn is_weak_multi_layer_report(report: &AnnSearchReport) -> bool {
    report.graph_nodes >= 4 && report.hnsw_layer_count > 1 && report.upper_graph_edges == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::{AnnFallbackReason, AnnSloViolation};

    fn no_fallback_search_policy() -> AnnSearchPolicy {
        AnnSearchPolicy {
            min_recall_q16: Some(MIN_ANN_RECALL_Q16),
            fallback: false,
            fallback_scan_cap: None,
            max_visited_candidates: Some(128),
            require_slo: true,
        }
    }

    fn healthy_report() -> AnnSearchReport {
        AnnSearchReport {
            path: AnnSearchPath::HnswGraph,
            fallback_reason: None,
            fallback_performed: false,
            requested_limit: 10,
            allowed_candidates: 32,
            graph_nodes: 32,
            returned_candidates: 10,
            visited_candidates: 24,
            max_visited_candidates: Some(128),
            recall_q16: Some(65_535),
            min_recall_q16: Some(MIN_ANN_RECALL_Q16),
            hnsw_max_neighbors: 16,
            hnsw_ef_search: 128,
            hnsw_layer_count: 4,
            hnsw_ef_construction: 128,
            upper_graph_edges: 12,
            require_slo: true,
            production_safe: true,
            slo_violations: Vec::new(),
        }
    }

    #[test]
    fn hnsw_no_fallback_rollout_is_disabled_by_default() {
        let decision = evaluate_hnsw_no_fallback_rollout(
            HnswNoFallbackRolloutPolicy::default(),
            no_fallback_search_policy(),
            &healthy_report(),
        );

        assert!(!decision.allowed);
        assert_eq!(
            decision.reasons,
            vec![HnswNoFallbackBlockReason::RolloutDisabled]
        );
        assert_eq!(
            HnswNoFallbackBlockReason::RolloutDisabled.as_str(),
            "rollout_disabled"
        );
    }

    #[test]
    fn hnsw_no_fallback_rollout_allows_healthy_opt_in_report() {
        let decision = evaluate_hnsw_no_fallback_rollout(
            HnswNoFallbackRolloutPolicy::enabled(),
            no_fallback_search_policy(),
            &healthy_report(),
        );

        assert!(decision.allowed);
        assert!(decision.reasons.is_empty());
    }

    #[test]
    fn hnsw_no_fallback_rollout_blocks_fallback_enabled_search_policy() {
        let mut search_policy = no_fallback_search_policy();
        search_policy.fallback = true;
        let decision = evaluate_hnsw_no_fallback_rollout(
            HnswNoFallbackRolloutPolicy::enabled(),
            search_policy,
            &healthy_report(),
        );

        assert!(!decision.allowed);
        assert!(decision
            .reasons
            .contains(&HnswNoFallbackBlockReason::FallbackEnabled));
    }

    #[test]
    fn hnsw_no_fallback_rollout_blocks_degraded_report() {
        let mut report = healthy_report();
        report.fallback_reason = Some(AnnFallbackReason::LowRecall);
        report.production_safe = false;
        report.slo_violations = vec![AnnSloViolation::LowRecall];
        report.recall_q16 = Some(MIN_ANN_RECALL_Q16 - 1);

        let decision = evaluate_hnsw_no_fallback_rollout(
            HnswNoFallbackRolloutPolicy::enabled(),
            no_fallback_search_policy(),
            &report,
        );

        assert!(!decision.allowed);
        assert!(decision
            .reasons
            .contains(&HnswNoFallbackBlockReason::FallbackReasonPresent));
        assert!(decision
            .reasons
            .contains(&HnswNoFallbackBlockReason::ReportNotProductionSafe));
        assert!(decision
            .reasons
            .contains(&HnswNoFallbackBlockReason::SloViolationsPresent));
        assert!(decision
            .reasons
            .contains(&HnswNoFallbackBlockReason::RecallBelowMinimum));
    }

    #[test]
    fn hnsw_no_fallback_rollout_blocks_weak_multi_layer_topology() {
        let mut report = healthy_report();
        report.upper_graph_edges = 0;

        let decision = evaluate_hnsw_no_fallback_rollout(
            HnswNoFallbackRolloutPolicy::enabled(),
            no_fallback_search_policy(),
            &report,
        );

        assert!(!decision.allowed);
        assert!(decision
            .reasons
            .contains(&HnswNoFallbackBlockReason::WeakMultiLayerTopology));
    }
}
