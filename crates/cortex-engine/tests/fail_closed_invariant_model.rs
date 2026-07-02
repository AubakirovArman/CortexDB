use cortex_engine::accountability::{
    fail_closed_invariant_model_hash, FAIL_CLOSED_INVARIANT_MODEL_HASH,
};

#[test]
fn fail_closed_invariant_model_hash_is_stable() {
    let computed = fail_closed_invariant_model_hash();
    println!("model_hash={computed}");
    assert_eq!(computed, FAIL_CLOSED_INVARIANT_MODEL_HASH);
}
