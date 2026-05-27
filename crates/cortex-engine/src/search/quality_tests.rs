//! Search Quality v1 — golden dataset, BM25 top-k, and unicode tokenizer tests.

#[cfg(test)]
mod tests {
    use crate::search::tokenize;
    use crate::Database;
    use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};

    // ─── Unicode Tokenizer Tests ───

    #[test]
    fn tokenize_english_simple() {
        let terms = tokenize("The quick brown fox jumps");
        assert_eq!(terms, vec!["quick", "brown", "fox", "jumps"]);
    }

    #[test]
    fn tokenize_russian_cyrillic() {
        let terms = tokenize("Быстрая коричневая лисица прыгает");
        assert!(terms.contains(&"быстрая".to_owned()));
        assert!(terms.contains(&"коричневая".to_owned()));
        assert!(terms.contains(&"лисица".to_owned()));
        assert!(terms.contains(&"прыгает".to_owned()));
    }

    #[test]
    fn tokenize_kazakh_cyrillic() {
        let terms = tokenize("Жылдам қоңыр түлкі секіреді");
        assert!(terms.contains(&"жылдам".to_owned()));
        assert!(terms.contains(&"қоңыр".to_owned()));
        assert!(terms.contains(&"түлкі".to_owned()));
        assert!(terms.contains(&"секіреді".to_owned()));
    }

    #[test]
    fn tokenize_mixed_languages() {
        let terms = tokenize("Budget бюджет бюджетi 2024");
        assert!(terms.contains(&"budget".to_owned()));
        assert!(terms.contains(&"бюджет".to_owned()));
        assert!(terms.contains(&"бюджетi".to_owned()));
        assert!(terms.contains(&"2024".to_owned()));
    }

    #[test]
    fn tokenize_punctuation_and_numbers() {
        let terms = tokenize("Cost: $1,234.56 (approx.) — final!");
        assert!(terms.contains(&"cost".to_owned()));
        assert!(terms.contains(&"1".to_owned()));
        assert!(terms.contains(&"234".to_owned()));
        assert!(terms.contains(&"56".to_owned()));
        assert!(terms.contains(&"approx".to_owned()));
        assert!(terms.contains(&"final".to_owned()));
    }

    #[test]
    fn tokenize_stopwords_filtered() {
        let terms = tokenize("The and or of to in a an");
        assert!(terms.is_empty());
    }

    #[test]
    fn tokenize_russian_stopwords_filtered() {
        let terms = tokenize("и в на для с со за от до по о об у");
        assert!(terms.is_empty());
    }

    // ─── BM25 Golden Dataset Tests ───

    use crate::query::metadata::scope_id;
    use crate::search::SearchLimit;
    use cortex_aql::{AgentId, AgentView, BrainId, RetrievalMode, Q16_ZERO};
    use std::collections::BTreeSet;

    fn golden_database() -> Database {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();

        let docs: [(u64, &str); 5] = [
            (1, "Solar Plant budget is 1.2B KZT approved."),
            (2, "Wind Farm budget is 800M KZT approved."),
            (3, "Hydro Plant budget is 2.1B KZT under review."),
            (4, "Solar Plant expansion plan and risk analysis."),
            (5, "General investment strategy for renewable energy."),
        ];

        for (cell_id, body) in docs {
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
                body,
            );
            db.put_knowledge_cell(CellId(cell_id), cell).unwrap();
        }

        db
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

    #[test]
    fn bm25_top_k_budget_query() {
        let db = golden_database();
        let view = default_view();
        let results = db.search_keyword("budget", &view, SearchLimit(10)).unwrap();

        let cell_ids: Vec<u64> = results.iter().map(|r| r.cell_id.0).collect();
        assert!(cell_ids.contains(&1));
        assert!(cell_ids.contains(&2));
        assert!(cell_ids.contains(&3));
        assert!(!results.is_empty());
    }

    #[test]
    fn bm25_top_k_solar_query() {
        let db = golden_database();
        let view = default_view();
        let results = db
            .search_keyword("Solar Plant", &view, SearchLimit(10))
            .unwrap();

        let cell_ids: Vec<u64> = results.iter().map(|r| r.cell_id.0).collect();
        assert!(cell_ids.contains(&1));
        assert!(cell_ids.contains(&4));
    }

    #[test]
    fn bm25_top_k_limit_respected() {
        let db = golden_database();
        let view = default_view();
        let results = db.search_keyword("budget", &view, SearchLimit(2)).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn bm25_scores_are_descending() {
        let db = golden_database();
        let view = default_view();
        let results = db.search_keyword("budget", &view, SearchLimit(10)).unwrap();

        for window in results.windows(2) {
            assert!(
                window[0].score >= window[1].score,
                "scores must be descending: {:?} vs {:?}",
                window[0].score,
                window[1].score
            );
        }
    }
}
