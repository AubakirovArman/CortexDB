use std::collections::BTreeSet;
use std::time::Instant;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, Database};
use serde_json::{json, Value};

use super::args::Args;
use super::metrics::round_ms;

pub(crate) fn ingest_batches(db: &mut Database, args: &Args) -> Result<Value, String> {
    let started = Instant::now();
    let mut next = 1usize;
    while next <= args.cells {
        let end = next
            .saturating_add(args.batch_size)
            .saturating_sub(1)
            .min(args.cells);
        let batch = (next..=end)
            .map(|index| (CellId(index as u64), payload(index, args.payload_bytes)))
            .collect::<Vec<_>>();
        db.put_cells(batch).map_err(|error| error.to_string())?;
        next = end + 1;
    }
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    Ok(json!({
        "name": "put_batches",
        "units": args.cells,
        "batch_size": args.batch_size,
        "elapsed_ms": round_ms(elapsed_ms),
        "throughput_per_sec": if elapsed_ms <= 0.0 {
            0.0
        } else {
            round_ms((args.cells as f64) / (elapsed_ms / 1000.0))
        },
    }))
}

pub(crate) fn scale_view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("scale-benchmark".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([
            scope_id("scale"),
            scope_id("scale:team-a"),
            scope_id("scale:team-b"),
            scope_id("scale:archive"),
        ]),
        writable_scopes: BTreeSet::from([scope_id("scale")]),
        allowed_modes: BTreeSet::from([RetrievalMode::Fast, RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([
            MemoryType::Decision,
            MemoryType::Preference,
            MemoryType::WorkflowResult,
            MemoryType::ErrorLog,
            MemoryType::Observation,
        ]),
        max_context_budget_tokens: 10_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 1_000,
        default_candidate_limit: 10,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: None,
        allow_remember: true,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: Some(ScopeId(999)),
    }
}

fn payload(index: usize, payload_bytes: Option<usize>) -> Vec<u8> {
    let scope = match index % 5 {
        0 => "scale",
        1 => "scale",
        2 => "scale:team-a",
        3 => "scale:team-b",
        _ => "scale:archive",
    };
    let topic = match index % 7 {
        0 => "onboarding latency budget risk",
        1 => "checkpoint storage wal recovery",
        2 => "context pack retrieval evidence",
        3 => "agent memory verification source",
        4 => "search lexical semantic hybrid",
        5 => "tenant scope permissions audit",
        _ => "replication repair manifest segment",
    };
    let target = if index == 1 {
        "scale target onboarding budget approved"
    } else {
        "scale benchmark background evidence"
    };
    let target_len = payload_bytes.unwrap_or_else(|| 512 + ((index.wrapping_mul(7919)) % 3585));
    let mut text = format!(
        "scope={scope}\nstatus=ready\ntype=fact\nsource=scale-doc-{index}\ncreated={}\n\n{target}. {topic}. ",
        1_700_000_000u64 + index as u64
    );
    while text.len() < target_len {
        text.push_str(topic);
        text.push_str(". operational note with owner, date, risk, budget, status, and evidence. ");
    }
    text.truncate(target_len);
    text.into_bytes()
}
