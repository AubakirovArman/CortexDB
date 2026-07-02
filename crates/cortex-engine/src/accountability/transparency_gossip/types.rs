use serde::{Deserialize, Serialize};

pub const TRANSPARENCY_GOSSIP_EXCHANGE_SCHEMA: &str = "cortexdb.transparency.gossip.exchange.v1";
pub const TRANSPARENCY_GOSSIP_EVIDENCE_SCHEMA: &str = "cortexdb.transparency.gossip.evidence.v1";
pub const TRANSPARENCY_GOSSIP_HASH_DOMAIN: &str = "cortexdb.transparency.gossip_hash.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyGossipPolicy {
    pub service_id: String,
    pub service_url: String,
    pub window_start_unix_seconds: u64,
    pub window_end_unix_seconds: u64,
    pub required_monitor_count: u64,
    pub required_fanout: u64,
    pub max_exchange_age_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyGossipExchange {
    pub schema_version: String,
    pub sender_monitor_id: String,
    pub receiver_monitor_id: String,
    pub sender_monitor_url: String,
    pub receiver_monitor_url: String,
    pub service_url: String,
    pub exchange_unix_seconds: u64,
    pub response_http_status: u16,
    pub log_record_count: u64,
    pub log_head_hash: String,
    pub merkle_root_hash: String,
    pub gossip_status: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyGossipEvidence {
    pub schema_version: String,
    pub service_id: String,
    pub service_url: String,
    pub window_start_unix_seconds: u64,
    pub window_end_unix_seconds: u64,
    pub required_monitor_count: u64,
    pub required_fanout: u64,
    pub max_exchange_age_seconds: u64,
    pub monitor_count: u64,
    pub exchange_count: u64,
    pub log_record_count: u64,
    pub log_head_hash: String,
    pub merkle_root_hash: String,
    pub exchanges: Vec<TransparencyGossipExchange>,
    pub gossip_hash: String,
}
