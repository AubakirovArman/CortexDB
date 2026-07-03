//! F04-B1.3: persistent idempotency ledger for agent transactions.
//!
//! `commit_agent_transaction` previously accepted an `idempotency_key` but only
//! echoed it back — a retried request re-executed the write. This ledger makes it
//! real: a repeated `(agent_id, idempotency_key)` with the **same** request digest
//! replays the prior `committed_seq` without re-writing; the same key with a
//! **different** digest is rejected as key reuse; a fresh key executes and records.
//!
//! Ledger entries are durable cells in a reserved top-nibble namespace (`0xb`,
//! alongside memory `0x8` / feedback `0x9` / session `0xa`) with a reserved scope
//! no `AgentView` can read, so they never surface in retrieval and — being written
//! only when the default-off `agent_transactions` feature is enabled — never appear
//! in any golden. The entry cell-id is content-addressed from `(agent_id, key)` via
//! a blake3 hash into the 32-bit sequence slot, with **linear probing** on
//! collision (each entry stores its full key, so two keys that hash to the same
//! slot are never aliased). All hashing is deterministic (no wall clock).

use cortex_aql::AgentId;
use cortex_core::{CellId, CommitSeq, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_crypto::{blake3_256, hex_lower};

use crate::cell_ids::{agent_cell_id_slot, namespaced_agent_cell_id, CELL_ID_SEQUENCE_MASK};
use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::operation::{WriteBatch, WriteBatchOperation};

const IDEMPOTENCY_CELL_NAMESPACE: u64 = 0xb000_0000_0000_0000;
const IDEMPOTENCY_LEDGER_SCOPE: &str = "__cortex_idempotency__";

#[derive(Clone, Debug, PartialEq, Eq)]
struct LedgerEntry {
    agent_id: AgentId,
    idempotency_key: String,
    request_digest: String,
    committed_seq: CommitSeq,
}

/// The resolution of a `(agent_id, key, digest)` against the ledger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LedgerResolution {
    /// No prior entry for this key; insert a new entry at this cell id.
    Fresh { insert_cell_id: CellId },
    /// A prior entry with the same request digest — replay its outcome.
    Replay { committed_seq: CommitSeq },
    /// A prior entry with a *different* request digest — the key was reused.
    Conflict,
}

impl Database {
    /// Resolve a `(agent_id, idempotency_key)` against the durable ledger, using
    /// the request `digest` to distinguish a genuine replay from key reuse.
    /// Read-only.
    pub(crate) fn resolve_idempotency(
        &self,
        agent_id: AgentId,
        key: &str,
        digest: &str,
    ) -> EngineResult<LedgerResolution> {
        let agent_slot = agent_cell_id_slot(agent_id).ok_or_else(ledger_overflow)?;
        let base = key_base_sequence(agent_id, key);
        let txn = self.read_txn();
        let mut sequence = base;
        loop {
            let cell_id =
                namespaced_agent_cell_id(IDEMPOTENCY_CELL_NAMESPACE, agent_slot, sequence)
                    .ok_or_else(ledger_overflow)?;
            match self.get_cell(txn, cell_id) {
                None => {
                    return Ok(LedgerResolution::Fresh {
                        insert_cell_id: cell_id,
                    })
                }
                Some(payload) => {
                    let entry = parse_ledger_entry(&payload).ok_or_else(ledger_corrupt)?;
                    if entry.agent_id == agent_id && entry.idempotency_key == key {
                        if entry.request_digest == digest {
                            return Ok(LedgerResolution::Replay {
                                committed_seq: entry.committed_seq,
                            });
                        }
                        return Ok(LedgerResolution::Conflict);
                    }
                    // A different key occupies this slot (hash collision): probe on.
                    sequence = (sequence + 1) & CELL_ID_SEQUENCE_MASK;
                    if sequence == base {
                        return Err(ledger_overflow());
                    }
                }
            }
        }
    }

    /// Durably record a ledger entry at the cell id returned by a prior
    /// [`LedgerResolution::Fresh`].
    pub(crate) fn record_idempotency_entry(
        &mut self,
        insert_cell_id: CellId,
        agent_id: AgentId,
        key: &str,
        digest: &str,
        committed_seq: CommitSeq,
    ) -> EngineResult<()> {
        let cell = KnowledgeCell::new(
            KnowledgeCellMetadata {
                scope: IDEMPOTENCY_LEDGER_SCOPE.to_owned(),
                status: "ready".to_owned(),
                cell_type: KnowledgeCellType::Feedback,
                memory_type: None,
                ttl_seconds: None,
                // No wall clock: the entry's identity is its key + digest, not a time.
                created_unix_seconds: None,
                source_trust_q16: None,
                source: None,
            },
            ledger_payload(agent_id, key, digest, committed_seq),
        );
        self.put_knowledge_cell(insert_cell_id, cell)?;
        Ok(())
    }
}

/// Deterministic digest of an agent transaction request (no wall clock). Reused
/// key + same digest = replay; reused key + different digest = reuse conflict.
pub(crate) fn request_digest(agent_id: AgentId, scope: &str, batch: &WriteBatch) -> String {
    let mut buf = Vec::new();
    buf.extend_from_slice(&agent_id.0.to_be_bytes());
    write_len_prefixed(&mut buf, scope.as_bytes());
    let operations = batch.operations();
    buf.extend_from_slice(&(operations.len() as u64).to_be_bytes());
    for operation in operations {
        match operation {
            WriteBatchOperation::PutCell { cell_id, payload } => {
                buf.push(1);
                buf.extend_from_slice(&cell_id.0.to_be_bytes());
                write_len_prefixed(&mut buf, payload);
            }
            WriteBatchOperation::PatchCell { cell_id, payload } => {
                buf.push(2);
                buf.extend_from_slice(&cell_id.0.to_be_bytes());
                write_len_prefixed(&mut buf, payload);
            }
            WriteBatchOperation::TombstoneCell { cell_id } => {
                buf.push(3);
                buf.extend_from_slice(&cell_id.0.to_be_bytes());
            }
        }
    }
    hex_lower(&blake3_256(&buf))
}

fn write_len_prefixed(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    buf.extend_from_slice(bytes);
}

/// The base 32-bit sequence slot for a `(agent_id, key)` pair.
fn key_base_sequence(agent_id: AgentId, key: &str) -> u64 {
    let mut buf = Vec::new();
    buf.extend_from_slice(&agent_id.0.to_be_bytes());
    buf.extend_from_slice(key.as_bytes());
    let digest = blake3_256(&buf);
    let mut slot = [0u8; 4];
    slot.copy_from_slice(&digest[0..4]);
    u64::from(u32::from_be_bytes(slot))
}

fn ledger_payload(agent_id: AgentId, key: &str, digest: &str, committed_seq: CommitSeq) -> Vec<u8> {
    // The key is hex-encoded so arbitrary bytes (newlines, `=`) can't break parsing.
    format!(
        "agent_id={}\nidempotency_key_hex={}\nrequest_digest={}\ncommitted_seq={}\n",
        agent_id.0,
        hex_lower(key.as_bytes()),
        digest,
        committed_seq.0
    )
    .into_bytes()
}

fn parse_ledger_entry(payload: &[u8]) -> Option<LedgerEntry> {
    let text = std::str::from_utf8(payload).ok()?;
    let mut agent_id = None;
    let mut key = None;
    let mut request_digest = None;
    let mut committed_seq = None;
    // Skip the cell's metadata header lines (`scope=`, `status=`, ...) and the
    // blank separator; only our four keys are consumed.
    for line in text.lines() {
        if let Some((field, value)) = line.split_once('=') {
            match field {
                "agent_id" => agent_id = value.parse::<u64>().ok().map(AgentId),
                "idempotency_key_hex" => {
                    key = decode_hex(value).and_then(|bytes| String::from_utf8(bytes).ok())
                }
                "request_digest" => request_digest = Some(value.to_owned()),
                "committed_seq" => committed_seq = value.parse::<u64>().ok().map(CommitSeq),
                _ => {}
            }
        }
    }
    Some(LedgerEntry {
        agent_id: agent_id?,
        idempotency_key: key?,
        request_digest: request_digest?,
        committed_seq: committed_seq?,
    })
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(value.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let hi = (pair[0] as char).to_digit(16)?;
        let lo = (pair[1] as char).to_digit(16)?;
        out.push((hi * 16 + lo) as u8);
    }
    Some(out)
}

fn ledger_overflow() -> EngineError {
    EngineError::StorageInvariant("idempotency ledger cell id space is exhausted".to_owned())
}

fn ledger_corrupt() -> EngineError {
    EngineError::StorageInvariant("idempotency ledger entry is corrupt".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::WriteBatch;
    use cortex_core::CellId;

    #[test]
    fn request_digest_is_deterministic_and_order_sensitive() {
        let batch_a = WriteBatch::new()
            .put_cell(CellId(1), b"a".to_vec())
            .put_cell(CellId(2), b"b".to_vec());
        let batch_b = WriteBatch::new()
            .put_cell(CellId(1), b"a".to_vec())
            .put_cell(CellId(2), b"b".to_vec());
        assert_eq!(
            request_digest(AgentId(7), "s", &batch_a),
            request_digest(AgentId(7), "s", &batch_b),
            "same request must hash identically"
        );
        // A different payload changes the digest.
        let batch_c = WriteBatch::new()
            .put_cell(CellId(1), b"a".to_vec())
            .put_cell(CellId(2), b"DIFFERENT".to_vec());
        assert_ne!(
            request_digest(AgentId(7), "s", &batch_a),
            request_digest(AgentId(7), "s", &batch_c)
        );
        // A different agent or scope changes the digest.
        assert_ne!(
            request_digest(AgentId(7), "s", &batch_a),
            request_digest(AgentId(8), "s", &batch_a)
        );
        assert_ne!(
            request_digest(AgentId(7), "s", &batch_a),
            request_digest(AgentId(7), "other", &batch_a)
        );
    }

    #[test]
    fn ledger_payload_round_trips_including_awkward_keys() {
        let key = "line1\nkey=with=equals";
        let payload = ledger_payload(AgentId(3), key, "deadbeef", CommitSeq(42));
        // Prepend a metadata-looking header + blank line, as the cell store would.
        let mut full = b"scope=__cortex_idempotency__\nstatus=ready\ntype=feedback\n\n".to_vec();
        full.extend_from_slice(&payload);
        let entry = parse_ledger_entry(&full).expect("round trips");
        assert_eq!(entry.agent_id, AgentId(3));
        assert_eq!(entry.idempotency_key, key);
        assert_eq!(entry.request_digest, "deadbeef");
        assert_eq!(entry.committed_seq, CommitSeq(42));
    }

    #[test]
    fn key_base_sequence_fits_in_the_32_bit_slot() {
        for key in ["", "a", "some-long-idempotency-key-value"] {
            let seq = key_base_sequence(AgentId(1), key);
            assert!(seq <= CELL_ID_SEQUENCE_MASK, "{key} slot out of range");
        }
    }

    // A genuine 32-bit hash collision is impractical to brute-force in a test, so
    // this forces the collision by planting a *different* key at the exact base
    // slot of "alpha", then proves the linear probe never aliases the two keys.
    #[test]
    fn linear_probe_disambiguates_slot_collisions() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();
        let agent = AgentId(5);
        let slot = agent_cell_id_slot(agent).unwrap();
        let base = key_base_sequence(agent, "alpha");
        let base_cell = namespaced_agent_cell_id(IDEMPOTENCY_CELL_NAMESPACE, slot, base).unwrap();

        // Occupy alpha's base slot with a DIFFERENT key.
        db.record_idempotency_entry(base_cell, agent, "decoy", "digest_decoy", CommitSeq(1))
            .unwrap();

        // Resolving alpha must probe PAST the decoy to the next free slot.
        let next = (base + 1) & CELL_ID_SEQUENCE_MASK;
        let next_cell = namespaced_agent_cell_id(IDEMPOTENCY_CELL_NAMESPACE, slot, next).unwrap();
        match db
            .resolve_idempotency(agent, "alpha", "digest_alpha")
            .unwrap()
        {
            LedgerResolution::Fresh { insert_cell_id } => assert_eq!(insert_cell_id, next_cell),
            other => panic!("expected Fresh at the probed slot, got {other:?}"),
        }

        // Record alpha at the probed slot. Alpha now resolves to its OWN entry
        // (never the decoy in its base slot), and a different digest is a conflict —
        // proving the two keys sharing a base slot are never aliased.
        db.record_idempotency_entry(next_cell, agent, "alpha", "digest_alpha", CommitSeq(2))
            .unwrap();
        assert_eq!(
            db.resolve_idempotency(agent, "alpha", "digest_alpha")
                .unwrap(),
            LedgerResolution::Replay {
                committed_seq: CommitSeq(2)
            }
        );
        assert_eq!(
            db.resolve_idempotency(agent, "alpha", "different_digest")
                .unwrap(),
            LedgerResolution::Conflict
        );
        // The decoy still sits in alpha's base slot untouched (alpha did not
        // overwrite it): reading that exact cell back yields the decoy's entry.
        assert!(db.get_latest_cell_descriptor(base_cell).is_some());
    }
}
