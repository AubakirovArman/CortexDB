use cortex_aql::parse_aql_diagnostic;

use super::mutations::mutate_bytes;

pub fn assert_aql_parse_is_panic_free(input: &str) {
    let result = std::panic::catch_unwind(|| {
        let _ = parse_aql_diagnostic(input);
    });
    assert!(result.is_ok(), "AQL parser panicked on {input:?}");
}

pub fn aql_seed_inputs() -> Vec<String> {
    let seeds = [
        "",
        "RETRIEVE",
        "RETRIEVE CONTEXT FOR TASK \"decode\" IN BRAIN default WHERE space = project:decode AND status = \"ready\" LIMIT 10 CANDIDATES;",
        "VERIFY FACT \"decode fact\" IN BRAIN default;",
        "REMEMBER \"decode memory\" IN SCOPE project:decode AS TYPE decision TTL 60 SECONDS;",
        "EXPLAIN ANALYZE RETRIEVE CONTEXT FOR TASK \"decode\" IN BRAIN default WHERE NOT (space = project:secret OR status = \"draft\") LIMIT 2 CANDIDATES;",
    ];
    let mut inputs = seeds
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    for seed in seeds {
        for (_, mutated) in mutate_bytes(seed.as_bytes()).into_iter().take(8) {
            inputs.push(String::from_utf8_lossy(&mutated).into_owned());
        }
    }
    inputs
}
