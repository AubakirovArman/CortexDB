use super::{
    build_transparency_gossip_evidence, verify_transparency_gossip_evidence,
    TransparencyGossipExchange, TransparencyGossipPolicy, TRANSPARENCY_GOSSIP_EVIDENCE_SCHEMA,
    TRANSPARENCY_GOSSIP_EXCHANGE_SCHEMA,
};

#[test]
fn transparency_gossip_accepts_required_monitor_fanout() {
    let evidence = build_transparency_gossip_evidence(policy(), full_mesh("a")).unwrap();

    assert_eq!(evidence.schema_version, TRANSPARENCY_GOSSIP_EVIDENCE_SCHEMA);
    assert_eq!(evidence.monitor_count, 3);
    assert_eq!(evidence.exchange_count, 6);
    assert_eq!(evidence.log_record_count, 3);
    assert_eq!(evidence.log_head_hash, hex64("a"));
    assert_eq!(evidence.merkle_root_hash, hex64("b"));
    assert_eq!(evidence.gossip_hash.len(), 64);
    verify_transparency_gossip_evidence(&evidence).unwrap();
}

#[test]
fn transparency_gossip_rejects_insufficient_fanout() {
    let exchanges = vec![
        exchange("monitor-a", "monitor-b", 1_090, "a"),
        exchange("monitor-b", "monitor-a", 1_090, "a"),
        exchange("monitor-c", "monitor-a", 1_090, "a"),
    ];

    let error = build_transparency_gossip_evidence(policy(), exchanges)
        .unwrap_err()
        .to_string();

    assert!(error.contains("transparency gossip fanout not met"));
}

#[test]
fn transparency_gossip_rejects_stale_exchange() {
    let mut exchanges = full_mesh("a");
    exchanges[0].exchange_unix_seconds = 1_030;

    let error = build_transparency_gossip_evidence(policy(), exchanges)
        .unwrap_err()
        .to_string();

    assert!(error.contains("stale transparency gossip exchange"));
}

#[test]
fn transparency_gossip_rejects_split_log_head() {
    let mut exchanges = full_mesh("a");
    exchanges[0].log_head_hash = hex64("c");

    let error = build_transparency_gossip_evidence(policy(), exchanges)
        .unwrap_err()
        .to_string();

    assert!(error.contains("split transparency gossip log head"));
}

fn policy() -> TransparencyGossipPolicy {
    TransparencyGossipPolicy {
        service_id: "public-transparency-mainnet".to_owned(),
        service_url: "https://transparency.example/log".to_owned(),
        window_start_unix_seconds: 1_000,
        window_end_unix_seconds: 1_100,
        required_monitor_count: 3,
        required_fanout: 2,
        max_exchange_age_seconds: 60,
    }
}

fn full_mesh(log_head_prefix: &str) -> Vec<TransparencyGossipExchange> {
    let monitors = ["monitor-a", "monitor-b", "monitor-c"];
    let mut exchanges = Vec::new();
    for sender in monitors {
        for receiver in monitors {
            if sender != receiver {
                exchanges.push(exchange(sender, receiver, 1_090, log_head_prefix));
            }
        }
    }
    exchanges
}

fn exchange(
    sender_monitor_id: &str,
    receiver_monitor_id: &str,
    exchange_unix_seconds: u64,
    log_head_prefix: &str,
) -> TransparencyGossipExchange {
    TransparencyGossipExchange {
        schema_version: TRANSPARENCY_GOSSIP_EXCHANGE_SCHEMA.to_owned(),
        sender_monitor_id: sender_monitor_id.to_owned(),
        receiver_monitor_id: receiver_monitor_id.to_owned(),
        sender_monitor_url: monitor_url(sender_monitor_id),
        receiver_monitor_url: monitor_url(receiver_monitor_id),
        service_url: "https://transparency.example/log".to_owned(),
        exchange_unix_seconds,
        response_http_status: 200,
        log_record_count: 3,
        log_head_hash: hex64(log_head_prefix),
        merkle_root_hash: hex64("b"),
        gossip_status: "delivered".to_owned(),
    }
}

fn monitor_url(monitor_id: &str) -> String {
    format!("https://{monitor_id}.example/gossip")
}

fn hex64(prefix: &str) -> String {
    let mut value = prefix.repeat(64);
    value.truncate(64);
    value
}
