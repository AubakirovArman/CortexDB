use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{EngineError, EngineResult};

const CLUSTER_CONFIG_MAGIC: &str = "CORTEXDB_CLUSTER_CONFIG_V1";
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClusterNode {
    pub id: NodeId,
    pub address: String,
    pub ingress_address: Option<String>,
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
                ingress_address: None,
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
            if node
                .ingress_address
                .as_deref()
                .is_some_and(|address| address.trim() != address || address.is_empty())
            {
                return Err(EngineError::InvalidOperation);
            }
        }
        Ok(())
    }

    pub fn replication_paths(&self, root: impl AsRef<Path>) -> EngineResult<ReplicationPathConfig> {
        self.validate_replication_config()?;
        ReplicationPathConfig::new(root, self.local_node)
    }

    pub fn store(&self, path: impl AsRef<Path>) -> EngineResult<()> {
        self.validate_replication_config()?;
        write_atomic(path.as_ref(), encode_cluster_config(self).as_bytes())
    }

    pub fn load(path: impl AsRef<Path>) -> EngineResult<Self> {
        decode_cluster_config(&fs::read_to_string(path)?)
    }
}

impl ClusterNode {
    pub fn ingress_address(&self) -> &str {
        self.ingress_address.as_deref().unwrap_or(&self.address)
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

    pub fn cluster_config_path(&self) -> PathBuf {
        self.replication_dir()
            .join(format!("node-{}.cluster.conf", self.local_node.0))
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

fn encode_cluster_config(config: &ClusterConfig) -> String {
    let mut out = format!(
        "{CLUSTER_CONFIG_MAGIC}\nlocal_node {}\nreplication_factor {}\n",
        config.local_node.0, config.replication_factor
    );
    for node in &config.nodes {
        if let Some(ingress_address) = node.ingress_address.as_deref() {
            out.push_str(&format!(
                "node {} {} {}\n",
                node.id.0, node.address, ingress_address
            ));
        } else {
            out.push_str(&format!("node {} {}\n", node.id.0, node.address));
        }
    }
    out
}

fn decode_cluster_config(input: &str) -> EngineResult<ClusterConfig> {
    let mut lines = input.lines();
    if lines.next() != Some(CLUSTER_CONFIG_MAGIC) {
        return Err(EngineError::InvalidOperation);
    }

    let local_node = parse_prefixed_u64(lines.next(), "local_node ")?;
    let replication_factor = parse_prefixed_u64(lines.next(), "replication_factor ")? as usize;
    let mut nodes = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let body = line
            .strip_prefix("node ")
            .ok_or(EngineError::InvalidOperation)?;
        let fields = body.split_whitespace().collect::<Vec<_>>();
        let (id, address, ingress_address) = match fields.as_slice() {
            [id, address] => (*id, *address, None),
            [id, address, ingress_address] => (*id, *address, Some((*ingress_address).to_owned())),
            _ => return Err(EngineError::InvalidOperation),
        };
        nodes.push(ClusterNode {
            id: NodeId(parse_u64(id)?),
            address: address.to_owned(),
            ingress_address,
        });
    }

    let config = ClusterConfig {
        local_node: NodeId(local_node),
        nodes,
        replication_factor,
    };
    config.validate_replication_config()?;
    Ok(config)
}

fn parse_prefixed_u64(line: Option<&str>, prefix: &str) -> EngineResult<u64> {
    parse_u64(
        line.ok_or(EngineError::InvalidOperation)?
            .strip_prefix(prefix)
            .ok_or(EngineError::InvalidOperation)?,
    )
}

fn parse_u64(value: &str) -> EngineResult<u64> {
    value.parse().map_err(|_| EngineError::InvalidOperation)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> EngineResult<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let tmp = temp_path(path);
    {
        let mut file = File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        OpenOptions::new().read(true).open(parent)?.sync_all()?;
    }
    Ok(())
}

fn temp_path(path: &Path) -> PathBuf {
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("cluster");
    path.with_file_name(format!(
        "{file_name}.tmp.{}.{}",
        std::process::id(),
        counter
    ))
}
