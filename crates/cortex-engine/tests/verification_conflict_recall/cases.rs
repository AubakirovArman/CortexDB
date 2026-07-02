use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::verification::VerificationNumericConflictKind;
use cortex_engine::{scope_id, Database};

const SCOPE: &str = "project:investments";
const CONFLICT_REPEATS: usize = 25;
const CONTROL_REPEATS: usize = 30;
const NAMES: [&str; 30] = [
    "Aral", "Basil", "Cedar", "Dala", "Everest", "Falcon", "Garnet", "Harbor", "Ishim", "Jade",
    "Kairat", "Lumen", "Mirny", "Nomad", "Orion", "Prairie", "Quartz", "Ranger", "Sary", "Timber",
    "Ulytau", "Vector", "Willow", "Yonder", "Zenith", "Aster", "Buran", "Cobalt", "Dorado",
    "Ember",
];

pub(crate) fn recall_cases() -> Vec<RecallCase> {
    let mut cases = Vec::new();
    for index in 0..CONFLICT_REPEATS {
        cases.push(magnitude_case(index));
        cases.push(unit_case(index));
        cases.push(currency_case(index));
        cases.push(temporal_case(index));
        cases.push(citation_case(index));
        cases.push(format_case(index));
    }
    for index in 0..CONTROL_REPEATS {
        cases.push(control_case(index));
    }
    cases
}

pub(crate) fn run_case(case: &RecallCase) -> ObservedCase {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    for (index, cell) in case.cells.iter().enumerate() {
        db.put_cell(CellId((index + 1) as u64), cell.clone())
            .unwrap();
    }
    let report = db
        .verify_fact_aql(&verify_aql(&case.fact), &view())
        .unwrap();
    let observed_kinds = report
        .numeric_conflicts
        .iter()
        .map(|conflict| conflict.kind.as_str().to_owned())
        .collect::<Vec<_>>();
    let detected_expected_kind = case.expected_kind.is_some_and(|kind| {
        report
            .numeric_conflicts
            .iter()
            .any(|conflict| conflict.kind == kind)
    });
    ObservedCase {
        has_conflict: !report.numeric_conflicts.is_empty(),
        detected_expected_kind,
        observed_kinds,
    }
}

fn magnitude_case(index: usize) -> RecallCase {
    let project = entity("Magnitude", index);
    let (fact, evidence) = [("1.2B", "1.4B"), ("1200M", "1400M"), ("1200K", "1400K")][index % 3];
    conflict_case("magnitude", index, &project, "budget", fact, evidence)
}

fn unit_case(index: usize) -> RecallCase {
    let project = entity("Unit", index);
    conflict_case("unit", index, &project, "duration", "60 min", "2 h")
}

fn currency_case(index: usize) -> RecallCase {
    let project = entity("Currency", index);
    conflict_case("currency", index, &project, "financing", "24 USD", "24 EUR")
}

fn temporal_case(index: usize) -> RecallCase {
    let project = entity("Temporal", index);
    RecallCase {
        case_id: format!("temporal-{index:03}"),
        class: "temporal",
        fact: format!("{project} budget is 1.2B KZT on 2025-05-01"),
        expected_kind: Some(VerificationNumericConflictKind::Temporal),
        cells: vec![payload(
            &[
                ("source", "fixture://temporal".to_owned()),
                ("valid_from", "2025-01-01".to_owned()),
                ("valid_to", "2025-12-31".to_owned()),
            ],
            body(&project, "budget", "1.4B KZT", "1.4B KZT"),
        )],
    }
}

fn citation_case(index: usize) -> RecallCase {
    let project = entity("Citation", index);
    let source_id = format!("fixture:citation:{index}");
    RecallCase {
        case_id: format!("citation-{index:03}"),
        class: "citation",
        fact: format!("{project} budget is 1.2B KZT"),
        expected_kind: Some(VerificationNumericConflictKind::Citation),
        cells: vec![
            source_ref_payload(&source_id, &project, "1.2B KZT"),
            source_ref_payload(&source_id, &project, "1.4B KZT"),
        ],
    }
}

fn format_case(index: usize) -> RecallCase {
    let project = entity("Format", index);
    RecallCase {
        case_id: format!("format-{index:03}"),
        class: "format",
        fact: format!("{project} cost is $1.2M"),
        expected_kind: Some(VerificationNumericConflictKind::Numeric),
        cells: vec![payload(
            &[("source", "fixture://format".to_owned())],
            body_with_currency(&project, "cost", "1.4 million", "USD", "1.4 million USD"),
        )],
    }
}

fn control_case(index: usize) -> RecallCase {
    let project = entity("Control", index);
    let (metric, fact_value, cell) = match index % 5 {
        0 => (
            "cost",
            "$1.2M",
            body_with_currency(&project, "cost", "1,200,000", "USD", "1,200,000 USD"),
        ),
        1 => ("duration", "60 min", body(&project, "duration", "1h", "1h")),
        2 => (
            "budget",
            "1.2B KZT on 2025-05-01",
            body(&project, "budget", "1.2B KZT", "1.2B KZT"),
        ),
        3 => return citation_control_case(index, &project),
        _ => (
            "budget",
            "1.2B KZT",
            body_with_currency(&project, "budget", "1200000000", "KZT", "1200000000 KZT"),
        ),
    };
    RecallCase {
        case_id: format!("must-not-conflict-{index:03}"),
        class: "must_not_conflict",
        fact: format!("{project} {metric} is {fact_value}"),
        expected_kind: None,
        cells: vec![payload(
            &[
                ("source", "fixture://must-not-conflict".to_owned()),
                ("valid_from", "2025-01-01".to_owned()),
                ("valid_to", "2025-12-31".to_owned()),
            ],
            cell,
        )],
    }
}

fn citation_control_case(index: usize, project: &str) -> RecallCase {
    let source_id = format!("fixture:citation-control:{index}");
    RecallCase {
        case_id: format!("must-not-conflict-{index:03}"),
        class: "must_not_conflict",
        fact: format!("{project} budget is 1.2B KZT"),
        expected_kind: None,
        cells: vec![
            source_ref_payload(&source_id, project, "1.2B KZT"),
            source_ref_payload(&source_id, project, "1.2B KZT"),
        ],
    }
}

fn conflict_case(
    class: &'static str,
    index: usize,
    project: &str,
    metric: &str,
    fact_value: &str,
    evidence_value: &str,
) -> RecallCase {
    RecallCase {
        case_id: format!("{class}-{index:03}"),
        class,
        fact: format!("{project} {metric} is {fact_value}"),
        expected_kind: Some(VerificationNumericConflictKind::Numeric),
        cells: vec![payload(
            &[("source", format!("fixture://{class}"))],
            body(project, metric, evidence_value, evidence_value),
        )],
    }
}

fn body(project: &str, metric: &str, value: &str, display: &str) -> String {
    format!("project={project}\nmetric={metric}\nvalue={value}\n{project} {metric} is {display}.")
}

fn body_with_currency(
    project: &str,
    metric: &str,
    value: &str,
    currency: &str,
    display: &str,
) -> String {
    format!(
        "project={project}\nmetric={metric}\nvalue={value}\ncurrency={currency}\n{project} {metric} is {display}."
    )
}

fn source_ref_payload(source_id: &str, project: &str, value: &str) -> Vec<u8> {
    payload(
        &[
            ("source_id", source_id.to_owned()),
            ("document_id", "verification-recall.pdf".to_owned()),
            ("page", "7".to_owned()),
        ],
        body(project, "budget", value, value),
    )
}

fn payload(headers: &[(&str, String)], body: String) -> Vec<u8> {
    let mut text = format!("scope={SCOPE}\nstatus=verified\ntype=fact\nsource_trust_q16=50000\n");
    for (key, value) in headers {
        text.push_str(&format!("{key}={value}\n"));
    }
    text.push('\n');
    text.push_str(&body);
    text.into_bytes()
}

fn entity(prefix: &str, index: usize) -> String {
    format!("{prefix} {}", NAMES[index % NAMES.len()])
}

fn verify_aql(fact: &str) -> String {
    format!(r#"VERIFY FACT "{fact}" IN BRAIN investment_projects;"#)
}

fn view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("verification-conflict-recall-agent".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id(SCOPE)]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 400,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}

#[derive(Clone)]
pub(crate) struct RecallCase {
    pub(crate) case_id: String,
    pub(crate) class: &'static str,
    pub(crate) fact: String,
    cells: Vec<Vec<u8>>,
    pub(crate) expected_kind: Option<VerificationNumericConflictKind>,
}

pub(crate) struct ObservedCase {
    pub(crate) has_conflict: bool,
    pub(crate) detected_expected_kind: bool,
    observed_kinds: Vec<String>,
}

impl ObservedCase {
    pub(crate) fn failure_message(&self, case: &RecallCase) -> String {
        match case.expected_kind {
            Some(kind) => format!(
                "{}: expected {} conflict, observed {:?}",
                case.case_id,
                kind.as_str(),
                self.observed_kinds
            ),
            None => format!(
                "{}: expected no conflict, observed {:?}",
                case.case_id, self.observed_kinds
            ),
        }
    }
}
