use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::distributed::{ClusterConfig, NodeId, ReplicationPathConfig};
use crate::error::{EngineError, EngineResult};
use crate::replication::{
    ConsensusState, LogIndex, ReplicationFollowerProgressStore, ReplicationLog,
};

#[derive(Debug)]
pub struct ReplicationNodeRuntime {
    pub cluster: ClusterConfig,
    pub paths: ReplicationPathConfig,
    pub consensus: ConsensusState,
    pub log: ReplicationLog,
    pub progress_store: ReplicationFollowerProgressStore,
}

impl ReplicationNodeRuntime {
    pub fn close(self) -> EngineResult<()> {
        self.log.close()
    }
}

pub fn open_replication_node_runtime(
    root: impl AsRef<Path>,
    local_node: NodeId,
    commit_index: LogIndex,
) -> EngineResult<ReplicationNodeRuntime> {
    let root = root.as_ref();
    let paths = ReplicationPathConfig::new(root, local_node)?;
    let runtime =
        open_replication_node_runtime_from_config(root, paths.cluster_config_path(), commit_index)?;
    if runtime.cluster.local_node != local_node {
        return Err(EngineError::InvalidOperation);
    }
    Ok(runtime)
}

pub fn open_replication_node_runtime_from_config(
    root: impl AsRef<Path>,
    config_path: impl AsRef<Path>,
    commit_index: LogIndex,
) -> EngineResult<ReplicationNodeRuntime> {
    let root = root.as_ref();
    let cluster = ClusterConfig::load(config_path)?;
    let paths = cluster.replication_paths(root)?;
    fs::create_dir_all(paths.snapshot_inbox_dir())?;

    let bootstrap_voters = cluster
        .nodes
        .iter()
        .map(|node| node.id)
        .collect::<BTreeSet<_>>();
    let consensus = ReplicationLog::recover_consensus_with_membership(
        paths.consensus_log_path(),
        cluster.local_node,
        bootstrap_voters,
        commit_index,
    )?;
    if consensus.commit_index > consensus.last_log_index() {
        return Err(EngineError::InvalidOperation);
    }

    let mut progress_store =
        ReplicationFollowerProgressStore::open(paths.repair_progress_store_path())?;
    progress_store.reconcile_consensus_state(&consensus)?;
    let log = ReplicationLog::open(paths.consensus_log_path())?;

    Ok(ReplicationNodeRuntime {
        cluster,
        paths,
        consensus,
        log,
        progress_store,
    })
}
