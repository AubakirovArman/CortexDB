use serde::{Deserialize, Serialize};

pub const TRANSPARENCY_SLO_WINDOW_SCHEMA: &str = "cortexdb.transparency.slo.window.v1";
pub const TRANSPARENCY_SLO_EVIDENCE_SCHEMA: &str = "cortexdb.transparency.slo.evidence.v1";
pub const TRANSPARENCY_SLO_HASH_DOMAIN: &str = "cortexdb.transparency.slo_hash.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencySloPolicy {
    pub service_id: String,
    pub service_url: String,
    pub period_start_unix_seconds: u64,
    pub period_end_unix_seconds: u64,
    pub required_window_count: u64,
    pub min_available_window_percentage: u64,
    pub max_window_gap_seconds: u64,
    pub required_monitor_count: u64,
    pub required_gossip_fanout: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencySloWindow {
    pub schema_version: String,
    pub window_id: String,
    pub service_url: String,
    pub window_start_unix_seconds: u64,
    pub window_end_unix_seconds: u64,
    pub availability_status: String,
    pub monitor_count: u64,
    pub gossip_fanout: u64,
    pub consistency_status: String,
    pub log_record_count: u64,
    pub log_head_hash: String,
    pub merkle_root_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencySloEvidence {
    pub schema_version: String,
    pub service_id: String,
    pub service_url: String,
    pub period_start_unix_seconds: u64,
    pub period_end_unix_seconds: u64,
    pub required_window_count: u64,
    pub min_available_window_percentage: u64,
    pub max_window_gap_seconds: u64,
    pub required_monitor_count: u64,
    pub required_gossip_fanout: u64,
    pub window_count: u64,
    pub available_window_count: u64,
    pub log_record_count: u64,
    pub log_head_hash: String,
    pub merkle_root_hash: String,
    pub windows: Vec<TransparencySloWindow>,
    pub slo_hash: String,
}
