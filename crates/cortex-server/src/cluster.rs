mod monitor;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use cortex_engine::{ClusterConfig, ClusterNode, NodeId};

use crate::config::ServerOptions;
use crate::responses::{ClusterNodeResponse, ClusterStatusResponse};

pub(crate) use monitor::ClusterIngressMonitor;
use monitor::{
    cluster_uses_separate_ingress, discover_raft_leader_node, ClusterIngressRoutePermit,
};

pub(crate) const LIVE_RAFT_INGRESS_UNAVAILABLE: &str =
    "live Raft ingress routing could not reach the configured leader context route";

#[derive(Debug)]
pub(crate) enum ContextIngressDecision {
    Local,
    Forward(ForwardTarget),
    Unavailable(String),
}

#[derive(Debug)]
pub(crate) struct ForwardTarget {
    pub(crate) node_id: u64,
    pub(crate) address: String,
    pub(crate) _load_permit: Option<ClusterIngressRoutePermit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ForwardedHttpResponse {
    pub(crate) status_code: u16,
    pub(crate) body: String,
}

pub(crate) fn status_response(options: &ServerOptions) -> ClusterStatusResponse {
    status_response_for_config(&configured_cluster(options))
}

pub(crate) fn status_response_for_config(cluster: &ClusterConfig) -> ClusterStatusResponse {
    ClusterStatusResponse {
        local_node: cluster.local_node.0,
        nodes: cluster
            .nodes
            .iter()
            .map(|node| ClusterNodeResponse {
                id: node.id.0,
                address: node.address.clone(),
            })
            .collect(),
        replication_factor: cluster.replication_factor,
        distributed_enabled: is_distributed_cluster(cluster),
    }
}

#[cfg(test)]
pub(crate) fn context_ingress_decision(
    options: &ServerOptions,
    method: &str,
    path: &str,
) -> Option<ContextIngressDecision> {
    context_ingress_decision_with_monitor(options, None, method, path)
}

pub(crate) fn context_ingress_decision_with_monitor(
    options: &ServerOptions,
    monitor: Option<&ClusterIngressMonitor>,
    method: &str,
    path: &str,
) -> Option<ContextIngressDecision> {
    if !matches!(
        (method, path),
        ("POST", "/v1/context") | ("POST", "/v1/context/trace")
    ) {
        return None;
    }
    let cluster = configured_cluster(options);
    if !is_distributed_cluster(&cluster) {
        return None;
    }
    let leader = match context_ingress_leader_node(options, &cluster, monitor) {
        Ok(node) => node,
        Err(message) => return Some(ContextIngressDecision::Unavailable(message)),
    };
    if leader.node.id == cluster.local_node {
        return Some(ContextIngressDecision::Local);
    }
    Some(ContextIngressDecision::Forward(ForwardTarget {
        node_id: leader.node.id.0,
        address: leader.node.ingress_address().to_owned(),
        _load_permit: leader._load_permit,
    }))
}

pub(crate) fn forward_http_request(
    target: &ForwardTarget,
    method: &str,
    target_path: &str,
    body: &[u8],
    auth_header: Option<&str>,
    content_type: Option<&str>,
    request_id: Option<&str>,
) -> Result<ForwardedHttpResponse, String> {
    let mut stream = TcpStream::connect(&target.address).map_err(|error| {
        format!(
            "live Raft ingress forwarding to node {} at {} failed: {error}",
            target.node_id, target.address
        )
    })?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| format!("failed to set ingress forward read timeout: {error}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| format!("failed to set ingress forward write timeout: {error}"))?;

    write!(
        stream,
        "{method} {target_path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Length: {}\r\n",
        target.address,
        body.len()
    )
    .map_err(|error| format!("failed to write ingress forward request head: {error}"))?;
    if let Some(value) = auth_header {
        write!(stream, "Authorization: {value}\r\n")
            .map_err(|error| format!("failed to write ingress forward auth header: {error}"))?;
    }
    if let Some(value) = content_type {
        write!(stream, "Content-Type: {value}\r\n").map_err(|error| {
            format!("failed to write ingress forward content-type header: {error}")
        })?;
    }
    if let Some(value) = request_id {
        write!(stream, "X-Request-Id: {value}\r\n").map_err(|error| {
            format!("failed to write ingress forward request-id header: {error}")
        })?;
    }
    stream
        .write_all(b"\r\n")
        .and_then(|_| stream.write_all(body))
        .map_err(|error| format!("failed to write ingress forward body: {error}"))?;

    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .map_err(|error| format!("failed to read ingress forward response: {error}"))?;
    parse_forwarded_response(&raw)
}

fn configured_cluster(options: &ServerOptions) -> ClusterConfig {
    options
        .cluster_config
        .clone()
        .unwrap_or_else(ClusterConfig::single_node)
}

fn is_distributed_cluster(cluster: &ClusterConfig) -> bool {
    cluster.nodes.len() > 1 || cluster.replication_factor > 1
}

fn context_ingress_leader_node(
    options: &ServerOptions,
    cluster: &ClusterConfig,
    monitor: Option<&ClusterIngressMonitor>,
) -> Result<SelectedIngressLeader, String> {
    let leader_id = if let Some(leader_id) = options.cluster_ingress_leader {
        leader_id
    } else if cluster_uses_separate_ingress(cluster) {
        if let Some(monitor) = monitor {
            let (node, permit) = monitor.try_acquire_adaptive_leader_node()?;
            return Ok(SelectedIngressLeader {
                node,
                _load_permit: Some(permit),
            });
        }
        return discover_raft_leader_node(cluster)
            .map(SelectedIngressLeader::without_load_permit)
            .ok_or_else(|| {
                "automatic Raft ingress leader discovery did not find a known leader".to_owned()
            });
    } else {
        cluster
            .nodes
            .first()
            .map(|node| node.id)
            .ok_or_else(|| LIVE_RAFT_INGRESS_UNAVAILABLE.to_owned())?
    };
    cluster
        .nodes
        .iter()
        .find(|node| node.id == leader_id)
        .cloned()
        .map(SelectedIngressLeader::without_load_permit)
        .ok_or_else(|| missing_leader_message(leader_id))
}

#[derive(Debug)]
struct SelectedIngressLeader {
    node: ClusterNode,
    _load_permit: Option<ClusterIngressRoutePermit>,
}

impl SelectedIngressLeader {
    fn without_load_permit(node: ClusterNode) -> Self {
        Self {
            node,
            _load_permit: None,
        }
    }
}

fn missing_leader_message(leader_id: NodeId) -> String {
    format!(
        "configured cluster ingress leader {} is not present in cluster_config",
        leader_id.0
    )
}

fn parse_forwarded_response(raw: &[u8]) -> Result<ForwardedHttpResponse, String> {
    let Some(split_at) = raw.windows(4).position(|window| window == b"\r\n\r\n") else {
        return Err("ingress forward response was missing HTTP headers".to_owned());
    };
    let head = String::from_utf8_lossy(&raw[..split_at]);
    let Some(status_line) = head.lines().next() else {
        return Err("ingress forward response was missing a status line".to_owned());
    };
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| "ingress forward response status line was malformed".to_owned())?
        .parse::<u16>()
        .map_err(|error| format!("ingress forward response status was invalid: {error}"))?;
    let body = String::from_utf8_lossy(&raw[split_at + 4..]).into_owned();
    Ok(ForwardedHttpResponse { status_code, body })
}
