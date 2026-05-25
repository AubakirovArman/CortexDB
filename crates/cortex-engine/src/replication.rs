use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::distributed::NodeId;
use crate::error::{EngineError, EngineResult};
use cortex_storage::wal::{
    CommitAck, DecodedWalRecord, DurabilityMode, SectionTag, WalReader, WalRecord, WalRecordType,
    WalSection, WalWriter, WalWriterHandle,
};

mod election;
mod tcp;
mod transport;

pub use election::{ElectionOutcome, ElectionRole, ElectionState, VoteRequest, VoteResponse};
pub use tcp::{handle_replication_frame, TcpReplicationTransport};
pub use transport::{
    AppendEntriesRequest, AppendEntriesResponse, InMemoryReplicationTransport, ReplicationTransport,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Term(pub u64);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LogIndex(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplicatedEntry {
    pub term: Term,
    pub index: LogIndex,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitDecision {
    pub index: LogIndex,
    pub committed: bool,
    pub acknowledgements: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsensusState {
    pub local_node: NodeId,
    pub voters: BTreeSet<NodeId>,
    pub current_term: Term,
    pub commit_index: LogIndex,
    log: Vec<ReplicatedEntry>,
}

#[derive(Debug)]
pub struct ReplicationLog {
    path: PathBuf,
    writer: WalWriterHandle,
}

impl ConsensusState {
    pub fn new(local_node: NodeId, voters: BTreeSet<NodeId>) -> Self {
        Self {
            local_node,
            voters,
            current_term: Term(1),
            commit_index: LogIndex(0),
            log: Vec::new(),
        }
    }

    pub fn append_local(&mut self, payload: Vec<u8>) -> ReplicatedEntry {
        let entry = ReplicatedEntry {
            term: self.current_term,
            index: LogIndex(self.log.len() as u64 + 1),
            payload,
        };
        self.log.push(entry.clone());
        entry
    }

    pub fn record_acks(&mut self, index: LogIndex, acks: BTreeSet<NodeId>) -> CommitDecision {
        let valid_acks = acks
            .intersection(&self.voters)
            .copied()
            .collect::<BTreeSet<_>>();
        let committed = valid_acks.len() >= self.majority();
        if committed && index > self.commit_index {
            self.commit_index = index;
        }
        CommitDecision {
            index,
            committed,
            acknowledgements: valid_acks.len(),
        }
    }

    pub fn committed_entries(&self) -> Vec<ReplicatedEntry> {
        self.log
            .iter()
            .filter(|entry| entry.index <= self.commit_index)
            .cloned()
            .collect()
    }

    pub fn entries(&self) -> &[ReplicatedEntry] {
        &self.log
    }

    pub fn last_log_index(&self) -> LogIndex {
        self.log.last().map(|entry| entry.index).unwrap_or_default()
    }

    pub fn last_log_term(&self) -> Term {
        self.log.last().map(|entry| entry.term).unwrap_or_default()
    }

    pub fn recover(
        local_node: NodeId,
        voters: BTreeSet<NodeId>,
        entries: Vec<ReplicatedEntry>,
        commit_index: LogIndex,
    ) -> Self {
        let current_term = entries
            .iter()
            .map(|entry| entry.term)
            .max()
            .unwrap_or(Term(1));
        Self {
            local_node,
            voters,
            current_term,
            commit_index,
            log: entries,
        }
    }

    fn majority(&self) -> usize {
        (self.voters.len() / 2) + 1
    }
}

impl ReplicationLog {
    pub fn open(path: impl AsRef<Path>) -> EngineResult<Self> {
        Self::open_with_durability(path, DurabilityMode::Strict)
    }

    pub fn open_with_durability(
        path: impl AsRef<Path>,
        durability: DurabilityMode,
    ) -> EngineResult<Self> {
        let path = path.as_ref().to_owned();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let writer = WalWriter::start(&path, durability)?;
        Ok(Self { path, writer })
    }

    pub fn append(&self, entry: &ReplicatedEntry) -> EngineResult<CommitAck> {
        Ok(self.writer.append(wal_record_from_entry(entry))?)
    }

    pub fn close(self) -> EngineResult<()> {
        Ok(self.writer.shutdown()?)
    }

    pub fn recover_entries(path: impl AsRef<Path>) -> EngineResult<Vec<ReplicatedEntry>> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let scan = WalReader::scan_path(path)?;
        scan.records
            .iter()
            .filter(|record| record.record.record_type == WalRecordType::ReplicatedLogEntry)
            .map(entry_from_record)
            .collect()
    }

    pub fn recover_consensus(
        path: impl AsRef<Path>,
        local_node: NodeId,
        voters: BTreeSet<NodeId>,
        commit_index: LogIndex,
    ) -> EngineResult<ConsensusState> {
        let entries = Self::recover_entries(path)?;
        Ok(ConsensusState::recover(
            local_node,
            voters,
            entries,
            commit_index,
        ))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn wal_record_from_entry(entry: &ReplicatedEntry) -> WalRecord {
    WalRecord::new(
        WalRecordType::ReplicatedLogEntry,
        vec![
            WalSection::new(SectionTag::ReplicationCore, encode_replication_core(entry)),
            WalSection::new(SectionTag::PayloadInline, entry.payload.clone()),
        ],
    )
}

fn entry_from_record(record: &DecodedWalRecord) -> EngineResult<ReplicatedEntry> {
    if record.record.record_type != WalRecordType::ReplicatedLogEntry {
        return Err(EngineError::InvalidOperation);
    }
    let core = section(record, SectionTag::ReplicationCore)
        .ok_or(EngineError::MissingWalSection("ReplicationCore"))?;
    let payload = section(record, SectionTag::PayloadInline)
        .ok_or(EngineError::MissingWalSection("PayloadInline"))?;
    let (term, index) = decode_replication_core(core)?;
    Ok(ReplicatedEntry {
        term,
        index,
        payload: payload.to_vec(),
    })
}

fn encode_replication_core(entry: &ReplicatedEntry) -> Vec<u8> {
    let mut out = Vec::with_capacity(16);
    out.extend_from_slice(&entry.term.0.to_le_bytes());
    out.extend_from_slice(&entry.index.0.to_le_bytes());
    out
}

fn decode_replication_core(bytes: &[u8]) -> EngineResult<(Term, LogIndex)> {
    if bytes.len() != 16 {
        return Err(EngineError::InvalidOperation);
    }
    let term = u64::from_le_bytes(
        bytes[0..8]
            .try_into()
            .map_err(|_| EngineError::InvalidOperation)?,
    );
    let index = u64::from_le_bytes(
        bytes[8..16]
            .try_into()
            .map_err(|_| EngineError::InvalidOperation)?,
    );
    Ok((Term(term), LogIndex(index)))
}

fn section(record: &DecodedWalRecord, tag: SectionTag) -> Option<&[u8]> {
    record
        .sections
        .iter()
        .find(|section| section.tag == Some(tag))
        .map(|section| section.data.as_slice())
}
