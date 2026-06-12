use super::normalize::{contains_any_word_or_phrase, unique_texts};

pub(super) fn expected_slots(question: &str) -> Vec<String> {
    let lower = question.to_ascii_lowercase();
    let mut slots = Vec::new();
    if contains_any_word_or_phrase(
        &lower,
        &["when", "scheduled", "schedule", "time window", "timezone"],
    ) {
        slots.push("date time schedule window timezone".to_owned());
    }
    if contains_any_word_or_phrase(
        &lower,
        &[
            "threshold",
            "limit",
            "pass rate",
            "gate",
            "cutoff",
            "size",
            "budget",
        ],
    ) {
        slots.push("threshold limit default pass rate gate size budget".to_owned());
    }
    if contains_any_word_or_phrase(
        &lower,
        &["latency", "p95", "p99", "ms", "rtt", "sla", "slo"],
    ) {
        slots.push("latency p95 p99 ms rtt sla slo target".to_owned());
    }
    if contains_any_word_or_phrase(
        &lower,
        &["cost", "price", "credits", "billing", "invoice", "cheapest"],
    ) {
        slots.push("cost price credits billing invoice".to_owned());
    }
    if contains_any_word_or_phrase(&lower, &["cause", "root cause", "caused", "trigger"]) {
        slots.push("root cause trigger reason".to_owned());
    }
    if contains_any_word_or_phrase(
        &lower,
        &[
            "location",
            "region",
            "edge",
            "cluster",
            "route",
            "environment",
        ],
    ) {
        slots.push("location region edge cluster route environment".to_owned());
    }
    if contains_any_word_or_phrase(
        &lower,
        &[
            "role", "owner", "owns", "dri", "review", "approver", "approval",
        ],
    ) {
        slots.push("role owner reviewer approver dri".to_owned());
    }
    if contains_any_word_or_phrase(
        &lower,
        &[
            "status",
            "state",
            "blocker",
            "blocked",
            "mitigation",
            "rollback",
            "risk",
        ],
    ) {
        slots.push("status blocker mitigation rollback risk".to_owned());
    }
    if contains_any_word_or_phrase(&lower, &["all", "every", "list", "complete", "procedure"]) {
        slots.push("complete checklist of requested subparts".to_owned());
    }
    unique_texts(slots, 12)
}
