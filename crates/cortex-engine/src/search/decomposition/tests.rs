use super::{
    covered_requirement_ids, decompose_enterprise_rag_question, split_subquestions,
    QuestionRequirementKind,
};

#[test]
fn decomposes_project_delivery_question_into_slots_and_subquestions() {
    let decomposition = decompose_enterprise_rag_question(
        "Who owns the Apollo launch blocker, what is the deadline, and how should support verify the risk?",
    );

    assert!(decomposition.multi_requirement);
    assert!(decomposition
        .slots
        .iter()
        .any(|slot| slot.contains("owner")));
    assert!(decomposition
        .slots
        .iter()
        .any(|slot| slot.contains("status blocker")));
    assert!(decomposition.subquestions.len() >= 2);
    assert!(decomposition
        .requirements
        .iter()
        .any(|item| item.kind == QuestionRequirementKind::Anchor && item.text == "Apollo"));
}

#[test]
fn decomposes_threshold_metric_and_cost_slots() {
    let decomposition = decompose_enterprise_rag_question(
        "What p95 latency threshold and cost limit are required for the EU route?",
    );

    assert!(decomposition
        .slots
        .iter()
        .any(|slot| slot.contains("threshold")));
    assert!(decomposition
        .slots
        .iter()
        .any(|slot| slot.contains("latency")));
    assert!(decomposition.slots.iter().any(|slot| slot.contains("cost")));
}

#[test]
fn coverage_reports_requirements_supported_by_payload() {
    let decomposition = decompose_enterprise_rag_question(
        "Who owns the Apollo launch blocker and what is the deadline?",
    );
    let covered = covered_requirement_ids(
        &decomposition,
        "project=Apollo owner=Maya launch blocker is auth; deadline is 2026-05-01.",
    );

    assert!(covered.len() >= 3, "{covered:?}");
}

#[test]
fn split_subquestions_handles_connectors_and_lists() {
    let parts = split_subquestions(
        "What caused the incident, what mitigation shipped, and where is the follow-up ticket?",
    );

    assert!(parts.iter().any(|part| part.contains("caused")));
    assert!(parts.iter().any(|part| part.contains("mitigation")));
    assert!(parts.iter().any(|part| part.contains("follow-up ticket")));
}
