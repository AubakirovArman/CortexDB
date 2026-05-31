use std::path::{Path, PathBuf};

use crate::error::{EngineError, EngineResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClusterNode {
    pub id: NodeId,
    pub address: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClusterConfig {
    pub local_node: NodeId,
    pub nodes: Vec<ClusterNode>,
    pub replication_factor: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    pub primary: NodeId,
    pub replicas: Vec<NodeId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplicationPathConfig {
    root: PathBuf,
    local_node: NodeId,
}

impl ClusterConfig {
    pub fn single_node() -> Self {
        Self {
            local_node: NodeId(1),
            nodes: vec![ClusterNode {
                id: NodeId(1),
                address: "127.0.0.1:0".to_owned(),
            }],
            replication_factor: 1,
        }
    }

    pub fn placement_for_key(&self, key: u64) -> Option<Placement> {
        if self.nodes.is_empty() || self.replication_factor == 0 {
            return None;
        }
        let start = key as usize % self.nodes.len();
        let replicas = (0..self.replication_factor.min(self.nodes.len()))
            .map(|offset| self.nodes[(start + offset) % self.nodes.len()].id)
            .collect::<Vec<_>>();
        Some(Placement {
            primary: replicas[0],
            replicas,
        })
    }

    pub fn owns_key(&self, key: u64) -> bool {
        self.placement_for_key(key)
            .is_some_and(|placement| placement.replicas.contains(&self.local_node))
    }

    pub fn validate_replication_config(&self) -> EngineResult<()> {
        if self.local_node.0 == 0
            || self.nodes.is_empty()
            || !self.nodes.iter().any(|node| node.id == self.local_node)
            || self.replication_factor == 0
        {
            return Err(EngineError::InvalidOperation);
        }
        let mut seen = std::collections::BTreeSet::new();
        for node in &self.nodes {
            if node.id.0 == 0 || node.address.trim().is_empty() || !seen.insert(node.id) {
                return Err(EngineError::InvalidOperation);
            }
        }
        Ok(())
    }

    pub fn replication_paths(&self, root: impl AsRef<Path>) -> EngineResult<ReplicationPathConfig> {
        self.validate_replication_config()?;
        ReplicationPathConfig::new(root, self.local_node)
    }
}

impl ReplicationPathConfig {
    pub fn new(root: impl AsRef<Path>, local_node: NodeId) -> EngineResult<Self> {
        let root = root.as_ref().to_owned();
        if root.as_os_str().is_empty() || local_node.0 == 0 {
            return Err(EngineError::InvalidOperation);
        }
        Ok(Self { root, local_node })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn local_node(&self) -> NodeId {
        self.local_node
    }

    pub fn replication_dir(&self) -> PathBuf {
        self.root.join("replication")
    }

    pub fn consensus_log_path(&self) -> PathBuf {
        self.replication_dir()
            .join(format!("node-{}.consensus.aclog", self.local_node.0))
    }

    pub fn repair_progress_store_path(&self) -> PathBuf {
        self.replication_dir()
            .join(format!("node-{}.repair-progress", self.local_node.0))
    }

    pub fn snapshot_inbox_dir(&self) -> PathBuf {
        self.replication_dir()
            .join(format!("node-{}.snapshots", self.local_node.0))
    }
}
