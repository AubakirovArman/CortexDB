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
}
