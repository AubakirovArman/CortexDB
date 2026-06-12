use cortex_core::CellId;
use cortex_engine::{AnswerGroundingOptions, ContextPack, ContextPackOptions, RetrievedCell};

#[test]
fn context_pack_grounding_accepts_answer_supported_by_pack() {
    let pack = pack();

    let report = pack.ground_answer(
        "Solar Plant budget is 1.2B KZT.",
        AnswerGroundingOptions::default(),
    );

    assert!(report.answer_supported);
    assert!(!report.rejected);
    assert_eq!(report.support_q16, u16::MAX);
    assert_eq!(report.unsupported_span_count, 0);
    assert_eq!(report.spans[0].supported_by_cell_ids, vec![CellId(1)]);
    assert_eq!(report.spans[0].citations, vec!["report-q1".to_owned()]);
}

#[test]
fn context_pack_grounding_flags_unsupported_answer_span() {
    let pack = pack();

    let report = pack.ground_answer(
        "Solar Plant budget is 1.2B KZT. The project TTL is 45 days.",
        AnswerGroundingOptions {
            reject_unsupported: true,
            ..AnswerGroundingOptions::default()
        },
    );

    assert!(!report.answer_supported);
    assert!(report.rejected);
    assert_eq!(report.supported_span_count, 1);
    assert_eq!(report.unsupported_span_count, 1);
    let unsupported = report.unsupported_spans().next().unwrap();
    assert_eq!(unsupported.text, "The project TTL is 45 days.");
    assert!(unsupported.missing_terms.contains(&"ttl".to_owned()));
}

#[test]
fn context_pack_grounding_can_require_citations() {
    let pack = ContextPack::from_retrieved_with_options(
        vec![RetrievedCell::from_payload(
            CellId(7),
            b"scope=project:investments\nstatus=ready\n\nSolar Plant budget is 1.2B KZT.".to_vec(),
        )],
        1_000,
        false,
        &ContextPackOptions::default(),
        "Solar Plant budget",
    );

    let report = pack.ground_answer(
        "Solar Plant budget is 1.2B KZT.",
        AnswerGroundingOptions {
            require_citations: true,
            ..AnswerGroundingOptions::default()
        },
    );

    assert!(!report.answer_supported);
    assert_eq!(report.spans[0].support_q16, u16::MAX);
    assert!(report.spans[0].citations.is_empty());
}

fn pack() -> ContextPack {
    ContextPack::from_retrieved_with_options(
        vec![RetrievedCell::from_payload(
            CellId(1),
            b"scope=project:investments\nstatus=ready\nsource=report-q1\n\nSolar Plant budget is 1.2B KZT."
                .to_vec(),
        )],
        1_000,
        true,
        &ContextPackOptions::default(),
        "Solar Plant budget",
    )
}
