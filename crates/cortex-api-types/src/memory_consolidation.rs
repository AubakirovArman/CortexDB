//! F04-B4.4 (contract): wire types for the memory-consolidation HTTP surface
//! (`POST /v1/memory/consolidate/plan` + `/commit`) over the engine's
//! `semantic_compression_candidates` (B4.2) and `commit_semantic_memory_compression`
//! (B4.3). Serde-only, no engine dependency.

use serde::{Deserialize, Serialize};

/// `POST /v1/memory/consolidate/plan` request: which stale episodic memory in a
/// scope is eligible for consolidation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsolidatePlanRequestBody {
    pub scope: String,
    /// Select episodic cells whose freshness (Q16) has decayed below this.
    pub freshness_below_q16: u16,
    pub max_groups: u32,
    /// Reference time; the server uses its current time when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub now_unix_seconds: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsolidateCandidate {
    pub cell_id: u64,
    pub freshness_q16: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsolidateGroup {
    pub scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_type: Option<String>,
    pub candidates: Vec<ConsolidateCandidate>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsolidatePlanResponse {
    pub groups: Vec<ConsolidateGroup>,
}

/// A byte range of a source cell that a summary consolidated.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsolidateSourceRef {
    pub source_cell_id: u64,
    pub source_byte_start: u64,
    pub source_byte_end: u64,
}

/// `POST /v1/memory/consolidate/commit` request: commit an
/// externally-generated summary over a set of source cells.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsolidateCommitRequestBody {
    pub scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_cell_id: Option<u64>,
    /// The summary cell payload (line-based UTF-8 cell format, `compression_kind=semantic_summary`).
    pub summary_payload: String,
    pub source_refs: Vec<ConsolidateSourceRef>,
    pub answerability_q16: u16,
    pub external_worker: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsolidateCommitResponse {
    pub summary_cell_id: u64,
    pub committed_seq: u64,
    pub source_cell_ids: Vec<u64>,
    pub source_ref_count: u64,
    pub answerability_q16: u16,
    pub provenance_preserved: bool,
    pub auditable: bool,
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
    fn plan_types_round_trip() {
        round_trip(&ConsolidatePlanRequestBody {
            scope: "agent:one".to_owned(),
            freshness_below_q16: 32_768,
            max_groups: 4,
            now_unix_seconds: Some(1_080),
        });
        round_trip(&ConsolidatePlanResponse {
            groups: vec![ConsolidateGroup {
                scope: "agent:one".to_owned(),
                memory_type: Some("observation".to_owned()),
                candidates: vec![ConsolidateCandidate {
                    cell_id: 10,
                    freshness_q16: 13_107,
                }],
            }],
        });
    }

    #[test]
    fn commit_types_round_trip() {
        round_trip(&ConsolidateCommitRequestBody {
            scope: "project:alpha".to_owned(),
            summary_cell_id: Some(99),
            summary_payload: "scope=project:alpha\n\nsummary".to_owned(),
            source_refs: vec![ConsolidateSourceRef {
                source_cell_id: 1,
                source_byte_start: 0,
                source_byte_end: 64,
            }],
            answerability_q16: 60_000,
            external_worker: "mcp-summary-v1".to_owned(),
            idempotency_key: None,
        });
        round_trip(&ConsolidateCommitResponse {
            summary_cell_id: 99,
            committed_seq: 5,
            source_cell_ids: vec![1, 2],
            source_ref_count: 2,
            answerability_q16: 60_000,
            provenance_preserved: true,
            auditable: true,
        });
    }
}
