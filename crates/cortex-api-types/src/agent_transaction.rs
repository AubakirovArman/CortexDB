//! F04-B6.3 (contract): frozen v1 wire types for the agent-transaction + handoff
//! HTTP surface (`POST /v1/transactions`, `POST /v1/handoff`).
//!
//! These mirror the engine's `AgentTransactionRequest`/`AgentTransactionReport`
//! and `AgentHandoffRequest`/`CommittedAgentHandoff` 1:1 so the live route and the
//! three SDKs share one neutral contract. Serde-only, no engine dependency.

use serde::{Deserialize, Serialize};

/// A single write operation in an agent transaction. Cell payloads are the
/// engine's line-based UTF-8 cell format (`scope=...\n...\n\n<body>`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum WriteOpRequest {
    Put { cell_id: u64, payload: String },
    Patch { cell_id: u64, payload: String },
    Tombstone { cell_id: u64 },
}

/// `POST /v1/transactions` request body.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentTransactionRequestBody {
    pub scope: String,
    pub base_seq: u64,
    pub operations: Vec<WriteOpRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

/// A conflicting cell reported when a transaction does not commit.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentTransactionConflictResponse {
    pub cell_id: u64,
    pub base_seq: u64,
    pub observed_seq: u64,
    /// `stale_cell` | `tombstoned_cell`.
    pub kind: String,
}

/// `POST /v1/transactions` response body.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentTransactionResponse {
    /// `committed` | `conflict`.
    pub outcome: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub committed_seq: Option<u64>,
    /// True when the result was replayed from the idempotency ledger (F04-B1.3).
    pub idempotent_replay: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<AgentTransactionConflictResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

/// `POST /v1/handoff` request body.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentHandoffRequestBody {
    pub source_agent_id: u64,
    pub target_agent_id: u64,
    pub scope: String,
    pub pack_hash: String,
    pub pack_seq: u64,
    pub required_after_seq: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

/// `POST /v1/handoff` response body — the durable, auditable record (F08-B6.1).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentHandoffResponse {
    pub handoff_cell_id: u64,
    pub committed_seq: u64,
    /// Always `shared_sequenced` for a committed handoff.
    pub level: String,
    pub visible_after_seq: u64,
    pub target_can_read: bool,
}

/// The **logical** status a client surfaces for a transaction `outcome`. The HTTP
/// transport always returns `200` with this [`AgentTransactionResponse`] body — the
/// request was processed and the outcome (`committed`/`conflict`, with the full
/// `conflicts` detail) is domain data, not a transport error — so an SDK reads
/// `outcome` and uses this helper to raise a `409`-equivalent on a conflict.
/// Reused-key-with-different-request and feature-disabled are transport-level engine
/// errors mapped by the error taxonomy, not outcomes.
pub fn transaction_outcome_status(outcome: &str) -> u16 {
    match outcome {
        "committed" => 200,
        "conflict" => 409,
        _ => 500,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip<T>(value: &T)
    where
        T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug,
    {
        let json = serde_json::to_string(value).expect("serialize");
        let back: T = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(&back, value, "round trip changed the value: {json}");
    }

    #[test]
    fn write_ops_round_trip_with_tagged_representation() {
        let put = WriteOpRequest::Put {
            cell_id: 1,
            payload: "scope=s\nstatus=ready\ntype=fact\n\nbody".to_owned(),
        };
        round_trip(&put);
        // The tag is the external `op` discriminator.
        let json = serde_json::to_string(&put).unwrap();
        assert!(json.contains("\"op\":\"put\""), "{json}");
        round_trip(&WriteOpRequest::Tombstone { cell_id: 9 });
    }

    #[test]
    fn transaction_request_and_response_round_trip() {
        round_trip(&AgentTransactionRequestBody {
            scope: "agent:one".to_owned(),
            base_seq: 3,
            operations: vec![WriteOpRequest::Tombstone { cell_id: 2 }],
            idempotency_key: Some("k".to_owned()),
        });
        round_trip(&AgentTransactionResponse {
            outcome: "conflict".to_owned(),
            committed_seq: None,
            idempotent_replay: false,
            conflicts: vec![AgentTransactionConflictResponse {
                cell_id: 2,
                base_seq: 3,
                observed_seq: 5,
                kind: "stale_cell".to_owned(),
            }],
            idempotency_key: None,
        });
    }

    #[test]
    fn handoff_request_and_response_round_trip() {
        round_trip(&AgentHandoffRequestBody {
            source_agent_id: 1,
            target_agent_id: 2,
            scope: "shared:project".to_owned(),
            pack_hash: "ctxpack:v1:gamma".to_owned(),
            pack_seq: 7,
            required_after_seq: 0,
            idempotency_key: None,
        });
        round_trip(&AgentHandoffResponse {
            handoff_cell_id: 42,
            committed_seq: 8,
            level: "shared_sequenced".to_owned(),
            visible_after_seq: 7,
            target_can_read: true,
        });
    }

    #[test]
    fn outcome_status_mapping() {
        assert_eq!(transaction_outcome_status("committed"), 200);
        assert_eq!(transaction_outcome_status("conflict"), 409);
        assert_eq!(transaction_outcome_status("unknown"), 500);
    }
}
