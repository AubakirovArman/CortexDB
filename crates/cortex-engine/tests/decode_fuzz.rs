#[path = "decode_fuzz/aql.rs"]
mod aql;
#[path = "decode_fuzz/corpus.rs"]
mod corpus;
#[path = "decode_fuzz/mutations.rs"]
mod mutations;

use aql::{aql_seed_inputs, assert_aql_parse_is_panic_free};
use corpus::{assert_decode_is_panic_free, assert_seed_decodes, build_seed_corpus};
use mutations::{extra_mutations, mutate_bytes};

#[test]
fn decode_fuzz_seed_corpus_is_panic_free() {
    let dir = tempfile::tempdir().unwrap();
    let seeds = build_seed_corpus(dir.path());
    assert_eq!(seeds.len(), 8);

    for seed in &seeds {
        assert_seed_decodes(seed);
        let bytes = std::fs::read(&seed.path).unwrap();
        for (case, mutated) in mutate_bytes(&bytes) {
            assert_decode_is_panic_free(seed, &case, &mutated);
        }
        for (case, mutated) in extra_mutations(&bytes, extra_case_count()) {
            assert_decode_is_panic_free(seed, &case, &mutated);
        }
    }

    for input in aql_seed_inputs() {
        assert_aql_parse_is_panic_free(&input);
    }
}

fn extra_case_count() -> usize {
    std::env::var("CORTEXDB_DECODE_FUZZ_EXTRA_CASES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0)
}
