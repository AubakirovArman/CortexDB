use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::{
    scope_id, Database, DatabaseOptions, Language, SearchLimit, TextAnalyzerConfig,
};

#[test]
fn configured_russian_analyzer_survives_checkpointed_search() {
    let dir = tempfile::tempdir().unwrap();
    let options = russian_stemming_options();
    let mut db = Database::open_with_options(dir.path(), options).unwrap();
    let cell = KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: "project:investments".to_owned(),
            status: "ready".to_owned(),
            cell_type: KnowledgeCellType::Fact,
            memory_type: None,
            ttl_seconds: None,
            created_unix_seconds: None,
            source_trust_q16: None,
            source: Some("golden".to_owned()),
        },
        "Бюджет проекта утвержден.",
    );
    db.put_knowledge_cell(CellId(100), cell).unwrap();
    db.checkpoint().unwrap();

    assert_eq!(
        db.manifest().text_analyzer_profile,
        Some(options.text_analyzer.manifest_profile())
    );

    let results = db
        .search_keyword("бюджету", &default_view(), SearchLimit(10))
        .unwrap();
    assert_eq!(results[0].cell_id, CellId(100));

    db.close().unwrap();
    assert!(Database::open(dir.path()).is_err());
    Database::open_with_options(dir.path(), options)
        .unwrap()
        .close()
        .unwrap();
}

fn russian_stemming_options() -> DatabaseOptions {
    DatabaseOptions {
        text_analyzer: TextAnalyzerConfig {
            language: Language::Russian,
            stemming: true,
        },
        ..DatabaseOptions::default()
    }
}

fn default_view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("test".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("project:investments")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::new(),
        max_context_budget_tokens: 4000,
        default_context_budget_tokens: 1000,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
