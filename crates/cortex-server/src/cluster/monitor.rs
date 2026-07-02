use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cortex_engine::{ClusterConfig, ClusterNode, NodeId};

use crate::config::ServerOptions;

#[derive(Debug)]
pub(crate) struct ClusterIngressMonitor {
    cluster: ClusterConfig,
    snapshot: Mutex<ClusterIngressSnapshot>,
    route_load: Arc<Mutex<BTreeMap<NodeId, usize>>>,
    max_in_flight_per_node: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ClusterIngressSnapshot {
    leader_id: Option<NodeId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ClusterIngressLoadMetrics {
    pub(crate) cached_leader_id: Option<NodeId>,
    pub(crate) max_in_flight_per_node: usize,
    pub(crate) in_flight_for_cached_leader: usize,
    pub(crate) available_permits_for_cached_leader: usize,
}

#[derive(Debug)]
pub(crate) struct ClusterIngressRoutePermit {
    node_id: NodeId,
    route_load: Arc<Mutex<BTreeMap<NodeId, usize>>>,
}

type IngressRoute = Result<(ClusterNode, ClusterIngressRoutePermit), String>;

impl ClusterIngressMonitor {
    pub(crate) fn from_options(options: &ServerOptions) -> Option<Self> {
        let cluster = options.cluster_config.clone()?;
        if options.cluster_ingress_leader.is_some() || !cluster_uses_separate_ingress(&cluster) {
            return None;
        }
        Some(Self::with_max_in_flight(
            cluster,
            options.cluster_ingress_max_in_flight_per_node(),
        ))
    }

    pub(crate) fn with_max_in_flight(
        cluster: ClusterConfig,
        max_in_flight_per_node: usize,
    ) -> Self {
        Self {
            cluster,
            snapshot: Mutex::new(ClusterIngressSnapshot::default()),
            route_load: Arc::new(Mutex::new(BTreeMap::new())),
            max_in_flight_per_node: max_in_flight_per_node.max(1),
        }
    }

    pub(crate) fn refresh_once(&self) {
        if let Some(leader_id) = discover_raft_leader_node(&self.cluster).map(|node| node.id) {
            if let Ok(mut snapshot) = self.snapshot.lock() {
                snapshot.leader_id = Some(leader_id);
            }
        }
    }

    pub(crate) fn cached_leader_node(&self) -> Option<ClusterNode> {
        let leader_id = self.snapshot.lock().ok()?.leader_id?;
        self.cluster
            .nodes
            .iter()
            .find(|node| node.id == leader_id)
            .cloned()
    }

    pub(crate) fn try_acquire_cached_leader_node(&self) -> IngressRoute {
        let node = self.cached_leader_node().ok_or_else(|| {
            "cached Raft ingress monitor did not find a known healthy leader".to_owned()
        })?;
        let mut route_load = self
            .route_load
            .lock()
            .map_err(|_| "cached Raft ingress monitor load state could not be read".to_owned())?;
        let in_flight = route_load.entry(node.id).or_insert(0);
        if *in_flight >= self.max_in_flight_per_node {
            return Err(format!(
                "cached Raft ingress monitor leader {} is over ingress load limit",
                node.id.0
            ));
        }
        *in_flight += 1;
        let node_id = node.id;
        Ok((
            node,
            ClusterIngressRoutePermit {
                node_id,
                route_load: Arc::clone(&self.route_load),
            },
        ))
    }

    pub(crate) fn try_acquire_adaptive_leader_node(&self) -> IngressRoute {
        match self.try_acquire_cached_leader_node() {
            Ok(route) => Ok(route),
            Err(error) if error.contains("over ingress load limit") => {
                self.refresh_once();
                self.try_acquire_cached_leader_node()
                    .map_err(|retry| format!("{error}; adaptive leader refresh failed: {retry}"))
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) fn load_metrics(&self) -> ClusterIngressLoadMetrics {
        let cached_leader_id = self
            .snapshot
            .lock()
            .ok()
            .and_then(|snapshot| snapshot.leader_id);
        let in_flight_for_cached_leader = cached_leader_id
            .and_then(|leader_id| {
                self.route_load
                    .lock()
                    .ok()
                    .and_then(|route_load| route_load.get(&leader_id).copied())
            })
            .unwrap_or(0);
        ClusterIngressLoadMetrics {
            cached_leader_id,
            max_in_flight_per_node: self.max_in_flight_per_node,
            in_flight_for_cached_leader,
            available_permits_for_cached_leader: self
                .max_in_flight_per_node
                .saturating_sub(in_flight_for_cached_leader),
        }
    }
}

impl Drop for ClusterIngressRoutePermit {
    fn drop(&mut self) {
        if let Ok(mut route_load) = self.route_load.lock() {
            if let Some(in_flight) = route_load.get_mut(&self.node_id) {
                *in_flight = in_flight.saturating_sub(1);
                if *in_flight == 0 {
                    route_load.remove(&self.node_id);
                }
            }
        }
    }
}

pub(crate) fn cluster_uses_separate_ingress(cluster: &ClusterConfig) -> bool {
    cluster
        .nodes
        .iter()
        .any(|node| node.ingress_address.is_some())
}

pub(crate) fn discover_raft_leader_node(cluster: &ClusterConfig) -> Option<ClusterNode> {
    cluster.nodes.iter().find_map(|node| {
        let leader_id = request_raft_status_leader(&node.address).ok().flatten()?;
        let leader = cluster
            .nodes
            .iter()
            .find(|candidate| candidate.id == leader_id)?;
        if leader.id == cluster.local_node || ingress_health_ok(leader.ingress_address()) {
            Some(leader.clone())
        } else {
            None
        }
    })
}

fn ingress_health_ok(address: &str) -> bool {
    request_ingress_health(address).is_ok()
}

fn request_ingress_health(address: &str) -> Result<(), String> {
    let socket_addr = address
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid ingress health address {address}: {error}"))?;
    let mut stream =
        TcpStream::connect_timeout(&socket_addr, Duration::from_millis(200)).map_err(|error| {
            format!("failed to connect to ingress health address {address}: {error}")
        })?;
    stream
        .set_read_timeout(Some(Duration::from_millis(200)))
        .map_err(|error| format!("failed to set ingress health read timeout: {error}"))?;
    stream
        .set_write_timeout(Some(Duration::from_millis(200)))
        .map_err(|error| format!("failed to set ingress health write timeout: {error}"))?;
    write!(
        stream,
        "GET /v1/health HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
    )
    .map_err(|error| format!("failed to write ingress health request: {error}"))?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("failed to read ingress health response: {error}"))?;
    let status = response
        .lines()
        .next()
        .ok_or_else(|| "ingress health response was missing a status line".to_owned())?;
    if status.split_whitespace().nth(1) == Some("200") {
        Ok(())
    } else {
        Err(format!("ingress health response was not healthy: {status}"))
    }
}

fn request_raft_status_leader(address: &str) -> Result<Option<NodeId>, String> {
    let socket_addr = address
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid Raft status address {address}: {error}"))?;
    let mut stream = TcpStream::connect_timeout(&socket_addr, Duration::from_millis(200))
        .map_err(|error| format!("failed to connect to Raft status address {address}: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_millis(200)))
        .map_err(|error| format!("failed to set Raft status read timeout: {error}"))?;
    stream
        .set_write_timeout(Some(Duration::from_millis(200)))
        .map_err(|error| format!("failed to set Raft status write timeout: {error}"))?;
    stream
        .write_all(b"STATUS\n")
        .and_then(|_| stream.shutdown(Shutdown::Write))
        .map_err(|error| format!("failed to write Raft status request: {error}"))?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("failed to read Raft status response: {error}"))?;
    parse_raft_status_leader(&response)
}

fn parse_raft_status_leader(response: &str) -> Result<Option<NodeId>, String> {
    let fields = response.split_whitespace().collect::<Vec<_>>();
    let ["STATUS_RESP", _term, _local, _role, leader] = fields.as_slice() else {
        return Err("Raft status response was malformed".to_owned());
    };
    let leader = leader
        .parse::<u64>()
        .map_err(|error| format!("Raft status leader was invalid: {error}"))?;
    Ok((leader != 0).then_some(NodeId(leader)))
}
