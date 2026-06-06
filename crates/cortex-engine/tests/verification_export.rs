use cortex_core::CellId;
use cortex_engine::{
    Magnitude, NumericValue, SourceTrustCategory, VerificationEvidence, VerificationGuard,
    VerificationGuardCode, VerificationNumericConflict, VerificationReport,
    VerificationReportExportFormat, VerificationStatus,
};

#[test]
fn verification_markdown_export_includes_table_evidence_guards_and_limitations() {
    let report = VerificationReport {
        fact: "ABC Airport budget | approved".to_owned(),
        status: VerificationStatus::Mixed,
        evidence: vec![VerificationEvidence {
            cell_id: CellId(1),
            matched_terms: 4,
            source_trust_q16: 60_000,
            source_trust_category: SourceTrustCategory::Official,
            citation: Some("ifc-disclosure".to_owned()),
        }],
        contradicting_evidence: vec![VerificationEvidence {
            cell_id: CellId(2),
            matched_terms: 4,
            source_trust_q16: 52_000,
            source_trust_category: SourceTrustCategory::High,
            citation: Some("internal-review".to_owned()),
        }],
        guards: vec![VerificationGuard {
            cell_id: Some(CellId(2)),
            code: VerificationGuardCode::NumericMismatch,
            message: "budget mismatch requires review".to_owned(),
        }],
        numeric_conflicts: vec![numeric_conflict()],
    };

    let markdown = report.export(VerificationReportExportFormat::Markdown);

    assert!(markdown.starts_with("# CortexDB Verification Report"));
    assert!(markdown.contains("## Report Table"));
    assert!(markdown.contains("| Field | Value |"));
    assert!(markdown.contains("| Fact | `ABC Airport budget \\| approved` |"));
    assert!(markdown.contains("| Status | `mixed_evidence` |"));
    assert!(markdown.contains("## Supporting Evidence"));
    assert!(markdown.contains("cell_id=`1` matched_terms=`4`"));
    assert!(markdown.contains("## Contradicting Evidence"));
    assert!(markdown.contains("cell_id=`2` matched_terms=`4`"));
    assert!(markdown.contains("## Guards"));
    assert!(markdown.contains("code=`numeric_mismatch`"));
    assert!(markdown.contains("## Limitations"));
    assert!(markdown.contains("limited to evidence visible through the caller's AgentView"));
}

fn numeric_conflict() -> VerificationNumericConflict {
    VerificationNumericConflict {
        cell_id: CellId(2),
        metric: "budget".to_owned(),
        left: "1.2B KZT".to_owned(),
        right: "1.4B KZT".to_owned(),
        fact_value: numeric_value("1.2B KZT", 1_200_000_000),
        evidence_value: numeric_value("1.4B KZT", 1_400_000_000),
    }
}

fn numeric_value(raw: &str, scaled_value: u64) -> NumericValue {
    NumericValue {
        raw: raw.to_owned(),
        scaled_value,
        currency: Some("KZT".to_owned()),
        unit: None,
        magnitude: Some(Magnitude::Billion),
    }
}
