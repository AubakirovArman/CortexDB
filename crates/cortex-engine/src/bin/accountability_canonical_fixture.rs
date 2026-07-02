use cortex_aql::Q16_ONE;
use cortex_core::CellId;
use cortex_engine::canonical::{canonical_context_pack_bytes, canonical_verification_report_bytes};
use cortex_engine::context::{
    ContextAccessDecision, ContextAccessDecisionOutcome, ContextExplain, ContextPack,
    ContextPackAnomaly, ContextPackAnomalyCode, ContextPackCell, ContextScoreComponent,
    ContextSpanProvenance, SourceFreshnessCategory,
};
use cortex_engine::source_trust::SourceTrustCategory;
use cortex_engine::{
    CellMetadata, Magnitude, NumericValue, SourceRef, VerificationEvidence, VerificationGuard,
    VerificationGuardCode, VerificationMatchKind, VerificationNumericConflict,
    VerificationNumericConflictKind, VerificationReport, VerificationStatus,
};

fn main() {
    let pack_hex = hex_bytes(&canonical_context_pack_bytes(&sample_pack()));
    let report_hex = hex_bytes(&canonical_verification_report_bytes(&sample_report()));

    println!("context_pack={pack_hex}");
    println!("verification_report={report_hex}");
}

fn sample_pack() -> ContextPack {
    ContextPack {
        cells: vec![ContextPackCell {
            cell_id: CellId(10),
            payload: b"scope=project:investments\n\nSolar plant budget".to_vec(),
            metadata: sample_metadata(),
            estimated_tokens: 7,
            citation: Some("doc-a".to_owned()),
            provenance: Some(ContextSpanProvenance {
                source_cell_id: CellId(9),
                source_byte_start: 0,
                source_byte_end: 18,
                source_line_start: 1,
                source_line_end: 1,
                source_ref: Some(sample_source_ref()),
            }),
            explain: Some(ContextExplain {
                score: 42,
                matched_terms: vec!["solar".to_owned(), "budget".to_owned()],
                why_selected: "matched query terms".to_owned(),
                score_components: vec![ContextScoreComponent {
                    name: "bm25".to_owned(),
                    value: 42,
                    contribution: 42,
                    reason: "canonical lexical score".to_owned(),
                }],
                base_bm25: 42,
                source_trust_q16: 50_000,
                source_trust_category: SourceTrustCategory::High,
                source_trust_bonus: 50_000,
                source_freshness_q16: 65_535,
                source_freshness_category: SourceFreshnessCategory::Current,
                source_freshness_bonus: 32_767,
                redundancy_penalty: 0,
            }),
            access_decision: Some(ContextAccessDecision {
                cell_id: CellId(10),
                decision: ContextAccessDecisionOutcome::Allowed,
                policy: "agent_view_scope".to_owned(),
                policy_version: Some("agent_view_readable_scope.v1".to_owned()),
                reason: "scope allowed".to_owned(),
                scope: "project:investments".to_owned(),
                scope_id: 1,
                agent_id: Some(7),
                agent_view_digest: Some(
                    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_owned(),
                ),
            }),
        }],
        token_budget_tokens: 100,
        estimated_tokens: 7,
        truncated: false,
        citations_required: true,
        answerability_q16: Q16_ONE,
        conflict_visibility_q16: 0,
        visible_conflict_count: 0,
        anomalies: vec![ContextPackAnomaly {
            cell_id: Some(CellId(10)),
            code: ContextPackAnomalyCode::MissingCitation,
            message: "citation required".to_owned(),
            why_excluded: None,
        }],
        grounding_report: None,
    }
}

fn sample_metadata() -> CellMetadata {
    CellMetadata {
        scope: "project:investments".to_owned(),
        status: "ready".to_owned(),
        cell_type: "fact".to_owned(),
        memory_type: None,
        ttl_seconds: None,
        created_unix_seconds: None,
        source_trust_q16: Some(50_000),
        source_trust_class: None,
        source: Some("doc-a".to_owned()),
        citation: Some("doc-a".to_owned()),
        title: None,
        content_hash: None,
        source_hash: None,
        document_id: Some("doc-a".to_owned()),
        chunk_id: None,
        parent_id: None,
        chunk_role: None,
        path: None,
        section: None,
        project: Some("Solar Plant".to_owned()),
        entity: None,
        sector: None,
        owner: None,
        status_tag: None,
        event_date: None,
        topic: None,
        as_of: None,
        valid_from: None,
        valid_to: None,
        supersedes: None,
        superseded_by: None,
        compression_kind: None,
        compression_source_cells: Vec::new(),
        compression_answerability_q16: None,
        compression_worker: None,
        table_id: None,
        table_headers: None,
        row_label: None,
        body_text: "Solar plant budget".to_owned(),
        terms: vec!["solar".to_owned(), "budget".to_owned()],
        source_ref: Some(sample_source_ref()),
    }
}

fn sample_source_ref() -> SourceRef {
    SourceRef {
        source_id: "source-a".to_owned(),
        source_url: Some("https://example.test/a".to_owned()),
        document_id: Some("doc-a".to_owned()),
        page: Some(1),
        row: None,
        cell_range: None,
        json_path: Some("$.budget".to_owned()),
        confidence_q16: 60_000,
    }
}

fn sample_report() -> VerificationReport {
    VerificationReport {
        fact: "Solar Plant budget is 1.2B KZT".to_owned(),
        status: VerificationStatus::Mixed,
        confidence_q16: Q16_ONE,
        evidence: vec![sample_evidence(
            CellId(10),
            VerificationMatchKind::ExactText,
        )],
        contradicting_evidence: vec![sample_evidence(
            CellId(20),
            VerificationMatchKind::NumericContradiction,
        )],
        guards: vec![VerificationGuard {
            cell_id: Some(CellId(20)),
            code: VerificationGuardCode::NumericMismatch,
            message: "numeric value differs".to_owned(),
        }],
        numeric_conflicts: vec![VerificationNumericConflict {
            cell_id: CellId(20),
            kind: VerificationNumericConflictKind::Numeric,
            metric: "budget".to_owned(),
            left: "1.2B KZT".to_owned(),
            right: "1.4B KZT".to_owned(),
            fact_value: sample_numeric_value("1.2B KZT", 1_200_000_000),
            evidence_value: sample_numeric_value("1.4B KZT", 1_400_000_000),
        }],
    }
}

fn sample_evidence(cell_id: CellId, match_kind: VerificationMatchKind) -> VerificationEvidence {
    VerificationEvidence {
        cell_id,
        matched_terms: 4,
        match_score_q16: Q16_ONE,
        match_kind,
        source_trust_q16: 50_000,
        source_trust_category: SourceTrustCategory::High,
        citation: Some("doc-a".to_owned()),
    }
}

fn sample_numeric_value(raw: &str, scaled_value: u64) -> NumericValue {
    NumericValue {
        raw: raw.to_owned(),
        scaled_value,
        currency: Some("KZT".to_owned()),
        unit: None,
        magnitude: Some(Magnitude::Billion),
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        out.push(char::from(HEX[(byte >> 4) as usize]));
        out.push(char::from(HEX[(byte & 0x0f) as usize]));
    }
    out
}
