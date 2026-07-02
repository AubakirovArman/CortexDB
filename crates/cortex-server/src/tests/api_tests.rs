use crate::{handle_http, handle_http_with_options, ReceiptSigningKey, ServerOptions};

include!("api_tests/memory.rs");
include!("api_tests/feedback.rs");
include!("api_tests/receipt_identity.rs");

const RECEIPT_TEST_SEED: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

fn receipt_options() -> ServerOptions {
    ServerOptions {
        receipt_signing_key: Some(
            ReceiptSigningKey::from_seed_hex("receipt-key-api-test", RECEIPT_TEST_SEED).unwrap(),
        ),
        ..ServerOptions::default()
    }
}

fn response_json(response: &str) -> serde_json::Value {
    let (_, body) = response.split_once("\r\n\r\n").unwrap();
    serde_json::from_str(body).unwrap()
}

#[test]
fn v1_context_returns_context_pack() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\n",
        "source_id=doc-a\nsource_url=https://example.test/doc-a\n",
        "document_id=doc-1\nconfidence_q16=60000\nalpha budget"
    );
    assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));

    let request = concat!(
        "POST /v1/context?scope=project:investments HTTP/1.1\r\n\r\n",
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects ",
        "WHERE space = project:investments AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""cells":[{"cell_id":1"#));
    assert!(response.contains(r#""citation":"doc-a""#));
    assert!(response.contains(r#""source_url":"https://example.test/doc-a""#));
}

#[test]
fn v1_context_emits_signed_accountability_receipt_when_key_configured() {
    let dir = tempfile::tempdir().unwrap();
    let options = receipt_options();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\n",
        "source=doc-a\nalpha budget"
    );
    assert!(handle_http_with_options(dir.path(), put, &options).contains(r#""seq":1"#));

    let request = concat!(
        "POST /v1/context?scope=project:investments HTTP/1.1\r\n\r\n",
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects ",
        "WHERE space = project:investments AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    let response = handle_http_with_options(dir.path(), request, &options);
    let value = response_json(&response);
    let receipt = &value["accountability_receipt"];

    assert_eq!(receipt["schema_version"], "accountability_receipt.v1");
    assert_eq!(
        receipt["header"]["schema_version"],
        "accountability_receipt.v1"
    );
    assert_eq!(receipt["header"]["key_id"], "receipt-key-api-test");
    assert_eq!(
        receipt["header"]["signature"]["public_key_hex"],
        ReceiptSigningKey::from_seed_hex("receipt-key-api-test", RECEIPT_TEST_SEED)
            .unwrap()
            .public_key_hex()
    );
    assert_eq!(receipt["leaves"]["access"][0]["decision"], "allowed");
}

#[test]
fn v1_context_returns_prompt_and_markdown_exports() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nsource=doc-a\nalpha budget"
    );
    assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));

    let prompt = concat!(
        "POST /v1/context?scope=project:investments&format=prompt HTTP/1.1\r\n\r\n",
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects ",
        "WHERE space = project:investments AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    let prompt_response = handle_http(dir.path(), prompt);
    assert!(prompt_response.contains("CortexDB ContextPack v1"));
    assert!(prompt_response.contains("Use only the context cells below."));

    let markdown = concat!(
        "POST /v1/context?scope=project:investments&format=markdown HTTP/1.1\r\n\r\n",
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects ",
        "WHERE space = project:investments AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    let markdown_response = handle_http(dir.path(), markdown);
    assert!(markdown_response.contains("# CortexDB ContextPack"));
    assert!(markdown_response.contains("### Cell 1"));
}

#[test]
fn v1_verify_emits_signed_accountability_receipt_when_key_configured() {
    let dir = tempfile::tempdir().unwrap();
    let options = receipt_options();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:test\nstatus=ready\nsource=doc-a\n\nThe budget is 1.2B KZT."
    );
    handle_http_with_options(dir.path(), put, &options);

    let request = concat!(
        "POST /v1/verify?scope=project:test HTTP/1.1\r\n\r\n",
        "VERIFY FACT \"The budget is 1.2B KZT\" IN BRAIN test;"
    );
    let response = handle_http_with_options(dir.path(), request, &options);
    let value = response_json(&response);
    let receipt = &value["accountability_receipt"];

    assert_eq!(value["verdict"], "supported");
    assert_eq!(receipt["schema_version"], "accountability_receipt.v1");
    assert_eq!(receipt["header"]["key_id"], "receipt-key-api-test");
    assert_eq!(receipt["leaves"]["verification"][0]["status"], "supported");
}

#[test]
fn v1_aql_returns_retrieved_cells() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nalpha budget"
    );
    assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));

    let request = concat!(
        "POST /v1/aql?scope=project:investments HTTP/1.1\r\n\r\n",
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects ",
        "WHERE space = project:investments AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""cells":[{"cell_id":1"#));
    assert!(response.contains("alpha budget"));
}

#[test]
fn v1_batch_applies_mixed_operations_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let put =
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\nscope=project:investments\nstatus=ready\nold";
    assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));

    let batch = concat!(
        "POST /v1/batch HTTP/1.1\r\ncontent-type: application/json\r\n\r\n",
        r#"{"operations":["#,
        r#"{"op":"patch_cell","cell_id":1,"payload":"scope=project:investments\nstatus=ready\nnew"},"#,
        r#"{"op":"put_cell","cell_id":2,"payload":"scope=project:investments\nstatus=ready\ntemporary"},"#,
        r#"{"op":"tombstone_cell","cell_id":2}"#,
        r#"]}"#
    );
    let response = handle_http(dir.path(), batch);
    assert!(response.contains(r#""seq":4"#));
    assert!(response.contains(r#""operation_count":3"#));
    assert!(response.contains(r#""cell_ids":[1,2,2]"#));

    let cell_one = handle_http(dir.path(), "GET /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n");
    assert!(cell_one.contains("new"));
    let cell_two = handle_http(dir.path(), "GET /v1/cell?cell_id=2 HTTP/1.1\r\n\r\n");
    assert!(cell_two.contains(r#""cell":null"#));
}

#[test]
fn v1_batch_validation_error_does_not_advance_seq() {
    let dir = tempfile::tempdir().unwrap();
    let put =
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\nscope=project:investments\nstatus=ready\nold";
    assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));

    let batch = concat!(
        "POST /v1/batch HTTP/1.1\r\ncontent-type: application/json\r\n\r\n",
        r#"{"operations":["#,
        r#"{"op":"patch_cell","cell_id":1,"payload":"scope=project:investments\nstatus=ready\nnew"},"#,
        r#"{"op":"tombstone_cell","cell_id":99}"#,
        r#"]}"#
    );
    let response = handle_http(dir.path(), batch);
    assert!(response.contains(r#""code":"not_found""#));

    let stats = handle_http(dir.path(), "GET /v1/stats HTTP/1.1\r\n\r\n");
    assert!(stats.contains(r#""current_seq":1"#));
    let cell_one = handle_http(dir.path(), "GET /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n");
    assert!(cell_one.contains("old"));
}

#[test]
fn v1_aql_explain_returns_plan_filters_counts_and_mode() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nalpha budget"
    );
    assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));

    let request = concat!(
        "POST /v1/aql?scope=project:investments HTTP/1.1\r\n\r\n",
        "EXPLAIN RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects ",
        "USING MODE balanced WHERE space = project:investments AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""cells":[]"#));
    assert!(response.contains(r#""selected_mode":"balanced""#));
    assert!(response.contains(r#""cost_model":{"selected_path":"bitmap-first""#));
    assert!(response.contains(r#""after_bitmap":1"#));
    assert!(response.contains(r#""kind":"where""#));
    assert!(response.contains("BitmapProgram(max_stack_depth="));
}

#[test]
fn v1_aql_explain_analyze_query_flag_reports_execution_trace() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nalpha budget"
    );
    assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));

    let request = concat!(
        "POST /v1/aql?scope=project:investments&explain=analyze HTTP/1.1\r\n\r\n",
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects ",
        "USING MODE balanced WHERE space = project:investments AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""cells":[]"#));
    assert!(response.contains(r#""execution_trace":"#));
    assert!(response.contains(r#""name":"BitmapIndexScan""#));
    assert!(response.contains(r#""actual_output_count":1"#));
    assert!(response.contains(r#""estimated_output_count":1"#));
    assert!(response.contains(r#""total_elapsed_nanos":"#));
}

#[test]
fn v1_verify_returns_markdown_and_audit_exports() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\ntype=fact\nsource=report.pdf\nsource_trust_q16=60000\nmetric=budget\n\n",
        "Solar Plant budget changed to 1.4B KZT."
    );
    handle_http(dir.path(), put);

    let markdown = concat!(
        "POST /v1/verify?scope=project:investments&format=markdown HTTP/1.1\r\n\r\n",
        "VERIFY FACT \"Solar Plant budget is 1.2B KZT\" IN BRAIN investment_projects;"
    );
    let markdown_response = handle_http(dir.path(), markdown);
    assert!(markdown_response.contains("# CortexDB Verification Report"));
    assert!(markdown_response.contains("## Numeric Conflicts"));

    let audit = concat!(
        "POST /v1/verify?scope=project:investments&format=audit HTTP/1.1\r\n\r\n",
        "VERIFY FACT \"Solar Plant budget is 1.2B KZT\" IN BRAIN investment_projects;"
    );
    let audit_response = handle_http(dir.path(), audit);
    assert!(audit_response.contains("CortexDB Verification Audit v1"));
    assert!(audit_response.contains("numeric_conflict.0.metric=budget"));
}
