use cortex_core::CellId;
use cortex_engine::{
    estimate_tokens, estimate_tokens_for_profile, ContextPack, ContextPackOptions,
    ContextTokenProfile, RetrievedCell, DEFAULT_CITATION_OVERHEAD_TOKENS,
};

#[test]
fn default_estimator_is_deterministic_and_nonzero_for_text() {
    let payload = b"scope=project:investments\nstatus=ready\nSolar budget evidence.";

    assert_eq!(estimate_tokens(payload), estimate_tokens(payload));
    assert!(estimate_tokens(payload) > 0);
}

#[test]
fn token_profiles_are_model_specific_for_multilingual_text() {
    let payload = "scope=project:investments\nstatus=ready\nҚазақстан renewable energy budget.";

    let default_tokens =
        estimate_tokens_for_profile(payload.as_bytes(), ContextTokenProfile::CortexApproxV2);
    let bge_tokens = estimate_tokens_for_profile(payload.as_bytes(), ContextTokenProfile::BgeM3);
    let deepseek_tokens =
        estimate_tokens_for_profile(payload.as_bytes(), ContextTokenProfile::DeepSeekChat);

    assert!(bge_tokens > default_tokens);
    assert!(deepseek_tokens >= default_tokens);
}

#[test]
fn model_name_aliases_select_known_profiles() {
    assert_eq!(
        ContextTokenProfile::from_model_name("BAAI/bge-m3"),
        ContextTokenProfile::BgeM3
    );
    assert_eq!(
        ContextTokenProfile::from_model_name("google/gemma-4-31B-it"),
        ContextTokenProfile::GoogleGemmaIt
    );
    assert_eq!(
        ContextTokenProfile::from_model_name("deepseek-chat"),
        ContextTokenProfile::DeepSeekChat
    );
    assert_eq!(
        ContextTokenProfile::from_model_name("gpt-4o-compatible"),
        ContextTokenProfile::OpenAiGpt4o
    );
    assert_eq!(
        ContextTokenProfile::from_model_name("unknown-local-model"),
        ContextTokenProfile::CortexApproxV2
    );
}

#[test]
fn context_pack_uses_selected_token_profile() {
    let payload = "scope=project:investments\nstatus=ready\nsource=doc-a\nҚазақстан budget.";
    let options = ContextPackOptions {
        token_profile: ContextTokenProfile::BgeM3,
        ..ContextPackOptions::default()
    };

    let pack = ContextPack::from_retrieved_with_options(
        vec![retrieved(1, payload)],
        1_000,
        true,
        &options,
        "",
    );

    assert_eq!(
        pack.cells[0].estimated_tokens,
        estimate_tokens_for_profile(payload.as_bytes(), ContextTokenProfile::BgeM3)
            + DEFAULT_CITATION_OVERHEAD_TOKENS
    );
    assert_eq!(pack.estimated_tokens, pack.cells[0].estimated_tokens);
}

#[test]
fn invalid_utf8_payload_falls_back_without_panicking() {
    let payload = [0xff, 0xfe, 0xfd, b'a', b'b', b'c'];

    assert!(estimate_tokens_for_profile(&payload, ContextTokenProfile::BgeM3) > 0);
}

fn retrieved(id: u64, payload: &str) -> RetrievedCell {
    RetrievedCell {
        cell_id: CellId(id),
        payload: payload.as_bytes().to_vec(),
    }
}
