use std::time::{SystemTime, UNIX_EPOCH};

use cortex_core::CellId;

use crate::{Database, DatabaseOptions, PayloadResidency};

use super::{
    build_corpus_synonym_dictionary, expand_query_with_corpus_synonyms, read_acsyn_dictionary,
    write_acsyn_dictionary, CorpusSynonymDictionaryBuilder, CorpusSynonymOptions,
};

#[test]
fn builds_cooccurrence_dictionary() {
    let docs = [
        "Apollo rollout blocker auth owner DRI",
        "Apollo launch blocker auth deadline",
        "Apollo rollout risk dependency auth",
        "Hermes billing invoice owner",
    ];
    let dictionary = build_corpus_synonym_dictionary(
        docs.iter().copied(),
        CorpusSynonymOptions {
            min_term_document_frequency: 2,
            min_pair_document_frequency: 2,
            max_synonyms_per_term: 4,
            max_terms: 100,
            max_terms_per_document: 64,
        },
    );

    let apollo = dictionary.synonyms_for("apollo");

    assert!(dictionary.terms_with_synonyms() >= 3);
    assert!(apollo.contains(&"auth".to_owned()));
    assert!(apollo.contains(&"rollout".to_owned()));
}

#[test]
fn streaming_builder_matches_batch_dictionary() {
    let docs = [
        "Apollo rollout blocker auth owner DRI",
        "Apollo launch blocker auth deadline",
        "Apollo rollout risk dependency auth",
    ];
    let options = CorpusSynonymOptions {
        min_term_document_frequency: 2,
        min_pair_document_frequency: 2,
        max_synonyms_per_term: 4,
        max_terms: 100,
        max_terms_per_document: 64,
    };
    let batch = build_corpus_synonym_dictionary(docs.iter().copied(), options);
    let mut builder = CorpusSynonymDictionaryBuilder::new();
    for document in docs {
        builder.add_document(document, options);
    }

    assert_eq!(builder.finish(options), batch);
}

#[test]
fn acsyn_roundtrip_preserves_entries() {
    let docs = [
        "security sso rbac permissions",
        "security auth sso policy",
        "security rbac access policy",
    ];
    let dictionary = build_corpus_synonym_dictionary(
        docs.iter().copied(),
        CorpusSynonymOptions {
            min_term_document_frequency: 2,
            min_pair_document_frequency: 2,
            max_synonyms_per_term: 8,
            max_terms: 100,
            max_terms_per_document: 64,
        },
    );
    let path = std::env::temp_dir().join(format!(
        "cortexdb-test-{}.acsyn",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    write_acsyn_dictionary(&path, &dictionary).unwrap();
    let loaded = read_acsyn_dictionary(&path).unwrap();
    let _ = std::fs::remove_file(path);

    assert_eq!(loaded, dictionary);
}

#[test]
fn database_persists_live_corpus_synonym_dictionary() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=default\nstatus=ready\ntype=fact\n\nApollo rollout blocker auth owner".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=default\nstatus=ready\ntype=fact\n\nApollo launch blocker auth deadline".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(3),
        b"scope=default\nstatus=ready\ntype=fact\n\nApollo rollout risk auth dependency".to_vec(),
    )
    .unwrap();

    let dictionary = db
        .persist_corpus_synonym_dictionary(CorpusSynonymOptions {
            min_term_document_frequency: 2,
            min_pair_document_frequency: 2,
            max_synonyms_per_term: 8,
            max_terms: 100,
            max_terms_per_document: 64,
        })
        .unwrap();
    let loaded = db.read_persisted_corpus_synonym_dictionary().unwrap();

    assert_eq!(loaded, Some(dictionary));
    assert!(db.corpus_synonym_dictionary_path().exists());
}

#[test]
fn corpus_synonym_store_tracks_patch_tombstone_checkpoint_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let options = CorpusSynonymOptions {
        min_term_document_frequency: 1,
        min_pair_document_frequency: 1,
        max_synonyms_per_term: 8,
        max_terms: 100,
        max_terms_per_document: 64,
    };
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(1),
            b"scope=default\nstatus=ready\ntype=fact\n\nApollo rollout blocker auth owner".to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(2),
            b"scope=default\nstatus=ready\ntype=fact\n\nApollo launch blocker auth deadline"
                .to_vec(),
        )
        .unwrap();
        db.patch_cell(
            CellId(2),
            b"scope=default\nstatus=ready\ntype=fact\n\nApollo updated blocker auth deadline"
                .to_vec(),
        )
        .unwrap();
        db.tombstone_cell(CellId(1)).unwrap();

        let dictionary = db.corpus_synonym_dictionary(options);
        assert!(dictionary
            .synonyms_for("apollo")
            .contains(&"updated".to_owned()));
        assert!(dictionary.synonyms_for("rollout").is_empty());

        db.checkpoint().unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    let dictionary = db.corpus_synonym_dictionary(options);
    assert!(dictionary
        .synonyms_for("apollo")
        .contains(&"updated".to_owned()));
    assert!(dictionary.synonyms_for("rollout").is_empty());
}

#[test]
fn corpus_synonym_dictionary_survives_lazy_checkpoint_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let options = CorpusSynonymOptions {
        min_term_document_frequency: 1,
        min_pair_document_frequency: 1,
        max_synonyms_per_term: 8,
        max_terms: 100,
        max_terms_per_document: 64,
    };
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(1),
            b"scope=default\nstatus=ready\ntype=fact\n\nApollo rollout blocker auth owner".to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(2),
            b"scope=default\nstatus=ready\ntype=fact\n\nApollo rollout blocker auth deadline"
                .to_vec(),
        )
        .unwrap();
        db.checkpoint().unwrap();
    }
    let db = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            payload_residency: PayloadResidency::Lazy,
            ..DatabaseOptions::default()
        },
    )
    .unwrap();

    assert_eq!(db.storage_stats().unwrap().memtable_payload_bytes, 0);
    let dictionary = db.corpus_synonym_dictionary(options);

    assert!(dictionary
        .synonyms_for("apollo")
        .contains(&"rollout".to_owned()));
    assert!(dictionary
        .synonyms_for("auth")
        .contains(&"blocker".to_owned()));
}

#[test]
fn expands_query_with_persisted_corpus_synonyms() {
    let docs = [
        "zephyr quartz rollout",
        "zephyr quartz incident",
        "quartz migration note",
    ];
    let dictionary = build_corpus_synonym_dictionary(
        docs.iter().copied(),
        CorpusSynonymOptions {
            min_term_document_frequency: 2,
            min_pair_document_frequency: 2,
            max_synonyms_per_term: 4,
            max_terms: 100,
            max_terms_per_document: 64,
        },
    );

    let expanded = expand_query_with_corpus_synonyms("zephyr status", &dictionary, 4, 2)
        .expect("expected corpus synonym expansion");

    assert!(expanded.contains("quartz"));
    assert!(expanded.starts_with("zephyr status "));
}

#[test]
fn mines_parenthetical_abbreviations_without_frequency_threshold() {
    let docs = [
        "The single sign on (SSO) rollout is tracked by identity.",
        "RBAC (role based access control) migration is owned by security.",
    ];
    let dictionary = build_corpus_synonym_dictionary(
        docs.iter().copied(),
        CorpusSynonymOptions {
            min_term_document_frequency: 3,
            min_pair_document_frequency: 3,
            max_synonyms_per_term: 8,
            max_terms: 100,
            max_terms_per_document: 64,
        },
    );

    let sso = dictionary.synonyms_for("SSO");
    let rbac = dictionary.synonyms_for("rbac");

    assert!(sso.contains(&"single".to_owned()));
    assert!(sso.contains(&"sign".to_owned()));
    assert!(rbac.contains(&"role".to_owned()));
    assert!(rbac.contains(&"based".to_owned()));
    assert!(rbac.contains(&"access".to_owned()));
    assert!(rbac.contains(&"control".to_owned()));
    assert!(dictionary
        .synonyms_for("single")
        .contains(&"sso".to_owned()));
}
