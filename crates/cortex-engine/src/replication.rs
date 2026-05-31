use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::distributed::NodeId;
use crate::error::{EngineError, EngineResult};
use cortex_storage::wal::{
    CommitAck, DecodedWalRecord, DurabilityMode, SectionTag, WalReader, WalRecord, WalRecordType,
    WalSection, WalWriter, WalWriterHandle, WalWriterOptions,
};

mod consensus;
mod election;
mod install;
mod log_matching;
mod membership;
mod peer;
mod recovery;
mod repair;
mod rotation;
mod runtime;
mod snapshot;
mod tcp;
mod transport;

pub use consensus::{CommitDecision, ConsensusState, LogIndex, ReplicatedEntry, Term};
pub use election::{ElectionOutcome, ElectionRole, ElectionState, VoteRequest, VoteResponse};
pub use membership::{
    decode_joint_membership_entry, decode_membership_entry, evaluate_joint_consensus,
    joint_membership_entry, membership_entry, recover_membership_config, recover_voting_config,
    JointConsensusDecision, JointMembershipConfig, MembershipConfig, VotingConfig,
};
pub use peer::{ReplicationPeerServer, ReplicationPeerState};
pub use recovery::{
    plan_replication_recovery, ReplicationRecoveryAction, ReplicationRecoveryPlan,
    ReplicationRecoveryPolicy,
};
pub use repair::{
    execute_replication_repair_schedule, plan_replication_repair_sweep, repair_lagging_voter,
    repair_lagging_voters, run_replication_repair_cycle, run_replication_repair_worker,
    send_replication_snapshot_request, spawn_replication_repair_background_task,
    spawn_replication_repair_background_task_with_progress_store,
    ReplicationDatabaseSnapshotSource, ReplicationFollowerProgress,
    ReplicationFollowerProgressStore, ReplicationProgressRecordingTransport,
    ReplicationRepairBackgroundHandle, ReplicationRepairBackgroundPolicy,
    ReplicationRepairBackgroundReport, ReplicationRepairCycleResult, ReplicationRepairDecision,
    ReplicationRepairDecisionKind, ReplicationRepairProgressSource, ReplicationRepairResult,
    ReplicationRepairSchedule, ReplicationRepairSnapshotSource, ReplicationRepairSweepResult,
    ReplicationRepairWorkerPolicy, ReplicationRepairWorkerReport, ReplicationRepairWorkerTick,
    ReplicationSnapshotRepairRequest, ReplicationSnapshotSendPolicy, ReplicationSnapshotSendResult,
    ReplicationSnapshotTransport, ReplicationStoredProgressSource,
};
pub use rotation::{
    resume_joint_membership_rotation, rotate_membership_with_joint_consensus,
    MembershipRotationPhase, MembershipRotationResult, MembershipRotationResumeResult,
};
pub use runtime::{
    open_replication_node_runtime, open_replication_node_runtime_from_config,
    ReplicationNodeRuntime,
};
pub use snapshot::{
    assemble_snapshot_chunks, decode_snapshot_chunk, decode_snapshot_segment,
    encode_snapshot_chunk, encode_snapshot_segment, SnapshotChunk, SnapshotSegment,
};
pub use tcp::{
    handle_authenticated_replication_frame, handle_replication_frame, TcpReplicationTransport,
};
pub use transport::{
    AppendEntriesRequest, AppendEntriesResponse, InMemoryReplicationTransport, ReplicationTransport,
};

#[derive(Debug)]
pub struct ReplicationLog {
    path: PathBuf,
    writer: WalWriterHandle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsensusLogDurability {
    Strict,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConsensusLogOptions {
    pub durability: ConsensusLogDurability,
    pub queue_capacity: Option<usize>,
    pub max_log_size: Option<u64>,
}

impl Default for ConsensusLogOptions {
    fn default() -> Self {
        Self {
            durability: ConsensusLogDurability::Strict,
            queue_capacity: None,
            max_log_size: None,
        }
    }
}

impl ReplicationLog {
    pub fn open(path: impl AsRef<Path>) -> EngineResult<Self> {
        Self::open_with_options(path, ConsensusLogOptions::default())
    }

    pub fn open_with_durability(
        path: impl AsRef<Path>,
        durability: ConsensusLogDurability,
    ) -> EngineResult<Self> {
        Self::open_with_options(
            path,
            ConsensusLogOptions {
                durability,
                ..ConsensusLogOptions::default()
            },
        )
    }

    pub fn open_with_options(
        path: impl AsRef<Path>,
        options: ConsensusLogOptions,
    ) -> EngineResult<Self> {
        let path = path.as_ref().to_owned();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let writer = WalWriter::start_with_options(&path, options.to_wal_writer_options())?;
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
        validate_recovered_consensus(&entries, commit_index)?;
        Ok(ConsensusState::recover(
            local_node,
            voters,
            entries,
            commit_index,
        ))
    }

    pub fn recover_consensus_with_membership(
        path: impl AsRef<Path>,
        local_node: NodeId,
        voters: BTreeSet<NodeId>,
        commit_index: LogIndex,
    ) -> EngineResult<ConsensusState> {
        let entries = Self::recover_entries(path)?;
        validate_recovered_consensus(&entries, commit_index)?;
        let voters = match recover_voting_config(&entries, voters, commit_index)? {
            VotingConfig::Stable(config) => config.voters,
            VotingConfig::Joint(config) => config.voters_union(),
        };
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

impl ConsensusLogOptions {
    fn to_wal_writer_options(self) -> WalWriterOptions {
        WalWriterOptions {
            durability_mode: match self.durability {
                ConsensusLogDurability::Strict => DurabilityMode::Strict,
            },
            queue_capacity: self.queue_capacity,
            max_wal_size: self.max_log_size,
        }
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

fn validate_recovered_consensus(
    entries: &[ReplicatedEntry],
    commit_index: LogIndex,
) -> EngineResult<()> {
    let mut expected_index = 1_u64;
    for entry in entries {
        if entry.term.0 == 0 || entry.index != LogIndex(expected_index) {
            return Err(EngineError::InvalidOperation);
        }
        expected_index = expected_index
            .checked_add(1)
            .ok_or(EngineError::InvalidOperation)?;
    }

    let last_log_index = entries.last().map(|entry| entry.index).unwrap_or_default();
    if commit_index > last_log_index {
        return Err(EngineError::InvalidOperation);
    }
    Ok(())
}
