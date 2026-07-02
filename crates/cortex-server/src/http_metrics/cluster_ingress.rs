use crate::state::AppState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ClusterIngressPrometheusMetrics {
    pub(crate) configured: u64,
    pub(crate) cached_leader_id: u64,
    pub(crate) max_in_flight_per_node: u64,
    pub(crate) in_flight: u64,
    pub(crate) available_permits: u64,
}

pub(crate) fn cluster_ingress_prometheus_metrics(
    state: &AppState,
) -> ClusterIngressPrometheusMetrics {
    let Some(metrics) = state
        .cluster_ingress_monitor
        .as_ref()
        .map(|monitor| monitor.load_metrics())
    else {
        return ClusterIngressPrometheusMetrics {
            configured: 0,
            cached_leader_id: 0,
            max_in_flight_per_node: 0,
            in_flight: 0,
            available_permits: 0,
        };
    };

    ClusterIngressPrometheusMetrics {
        configured: 1,
        cached_leader_id: metrics
            .cached_leader_id
            .map(|leader_id| leader_id.0)
            .unwrap_or(0),
        max_in_flight_per_node: metrics.max_in_flight_per_node as u64,
        in_flight: metrics.in_flight_for_cached_leader as u64,
        available_permits: metrics.available_permits_for_cached_leader as u64,
    }
}
