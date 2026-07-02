use serde::Deserialize;

#[derive(Deserialize)]
struct SigningKeyFile {
    key_id: String,
    signing_seed_hex: String,
    public_key_hex: String,
}

#[derive(Deserialize)]
struct PublicKeyFile {
    schema_version: String,
    key_id: String,
    public_key_hex: String,
}

#[derive(Deserialize)]
struct TrustFile {
    schema_version: String,
    current_key_id: String,
    trusted_public_keys: Vec<PublicKeyFile>,
}

#[test]
fn receipt_key_generate_export_and_rotate_preserves_dual_trust() {
    let root = unique_path("cortexdb-receipt-key");
    let key_a = root.join("receipt-key-a.json");
    let key_a_pub = root.join("receipt-key-a.public.json");
    let key_a_pub_export = root.join("receipt-key-a.exported-public.json");
    let key_b = root.join("receipt-key-b.json");
    let trust = root.join("receipt-trust.json");

    let generate = run(vec![
        "cortexdb".to_owned(),
        "receipt-key".to_owned(),
        "generate".to_owned(),
        key_a.to_string_lossy().into_owned(),
        "--key-id".to_owned(),
        "receipt-key-a".to_owned(),
        "--public-key-file".to_owned(),
        key_a_pub.to_string_lossy().into_owned(),
    ])
    .unwrap();
    assert!(generate.contains("key_id=receipt-key-a"));
    assert!(!generate.contains("signing_seed"));
    assert!(!generate.contains("private"));

    let export = run(vec![
        "cortexdb".to_owned(),
        "receipt-key".to_owned(),
        "export-public".to_owned(),
        key_a.to_string_lossy().into_owned(),
        key_a_pub_export.to_string_lossy().into_owned(),
    ])
    .unwrap();
    assert!(export.contains("public key exported"));

    let key_a_file: SigningKeyFile =
        serde_json::from_str(&std::fs::read_to_string(&key_a).unwrap()).unwrap();
    let public_a_file: PublicKeyFile =
        serde_json::from_str(&std::fs::read_to_string(&key_a_pub).unwrap()).unwrap();
    assert_eq!(
        public_a_file.schema_version,
        "cortexdb.receipt_public_key.v1"
    );
    assert_eq!(public_a_file.key_id, "receipt-key-a");
    assert_eq!(public_a_file.public_key_hex, key_a_file.public_key_hex);

    let signing_key_a = cortex_crypto::ReceiptSigningKey::from_seed_hex(
        &key_a_file.key_id,
        &key_a_file.signing_seed_hex,
    )
    .unwrap();
    let signature_a = signing_key_a.sign(b"historical receipt header");

    let rotate = run(vec![
        "cortexdb".to_owned(),
        "receipt-key".to_owned(),
        "rotate".to_owned(),
        key_a.to_string_lossy().into_owned(),
        key_b.to_string_lossy().into_owned(),
        trust.to_string_lossy().into_owned(),
        "--new-key-id".to_owned(),
        "receipt-key-b".to_owned(),
    ])
    .unwrap();
    assert!(rotate.contains("previous_key_id=receipt-key-a"));
    assert!(rotate.contains("current_key_id=receipt-key-b"));
    assert!(!rotate.contains(&key_a_file.signing_seed_hex));

    let key_b_file: SigningKeyFile =
        serde_json::from_str(&std::fs::read_to_string(&key_b).unwrap()).unwrap();
    let signing_key_b = cortex_crypto::ReceiptSigningKey::from_seed_hex(
        &key_b_file.key_id,
        &key_b_file.signing_seed_hex,
    )
    .unwrap();
    let signature_b = signing_key_b.sign(b"current receipt header");
    let trust_file: TrustFile =
        serde_json::from_str(&std::fs::read_to_string(&trust).unwrap()).unwrap();
    assert_eq!(trust_file.schema_version, "cortexdb.receipt_trust.v1");
    assert_eq!(trust_file.current_key_id, "receipt-key-b");
    let keyring = cortex_crypto::ReceiptKeyRing::new(
        trust_file
            .trusted_public_keys
            .iter()
            .map(|key| {
                cortex_crypto::ReceiptPublicKey::from_hex(&key.key_id, &key.public_key_hex).unwrap()
            })
            .collect(),
    )
    .unwrap();
    keyring
        .verify("receipt-key-a", b"historical receipt header", &signature_a)
        .unwrap();
    keyring
        .verify("receipt-key-b", b"current receipt header", &signature_b)
        .unwrap();
    assert!(keyring
        .verify("receipt-key-b", b"historical receipt header", &signature_a)
        .is_err());
    assert!(!std::fs::read_to_string(&trust)
        .unwrap()
        .contains("signing_seed_hex"));

    let existing = run(vec![
        "cortexdb".to_owned(),
        "receipt-key".to_owned(),
        "generate".to_owned(),
        key_a.to_string_lossy().into_owned(),
        "--key-id".to_owned(),
        "receipt-key-a2".to_owned(),
    ])
    .unwrap_err();
    assert!(existing.contains("failed to create"));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn receipt_key_rotate_writes_verifiable_reanchor_record() {
    let root = unique_path("cortexdb-receipt-reanchor");
    let key_a = root.join("receipt-key-a.json");
    let key_b = root.join("receipt-key-b.json");
    let trust = root.join("receipt-trust.json");
    let reanchor = root.join("receipt-reanchor.json");
    let tampered = root.join("receipt-reanchor-tampered.json");
    let audit_chain_head = "a".repeat(64);

    run(vec![
        "cortexdb".to_owned(),
        "receipt-key".to_owned(),
        "generate".to_owned(),
        key_a.to_string_lossy().into_owned(),
        "--key-id".to_owned(),
        "receipt-key-a".to_owned(),
    ])
    .unwrap();

    let rotate = run(vec![
        "cortexdb".to_owned(),
        "receipt-key".to_owned(),
        "rotate".to_owned(),
        key_a.to_string_lossy().into_owned(),
        key_b.to_string_lossy().into_owned(),
        trust.to_string_lossy().into_owned(),
        "--new-key-id".to_owned(),
        "receipt-key-b".to_owned(),
        "--reanchor-file".to_owned(),
        reanchor.to_string_lossy().into_owned(),
        "--audit-chain-head".to_owned(),
        audit_chain_head.clone(),
        "--audit-sequence".to_owned(),
        "42".to_owned(),
    ])
    .unwrap();
    assert!(rotate.contains("reanchor_file="));
    assert!(!rotate.contains("signing_seed"));

    let value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&reanchor).unwrap()).unwrap();
    assert_eq!(
        value["schema_version"],
        "cortexdb.receipt_audit_reanchor.v1"
    );
    assert_eq!(value["previous_key_id"], "receipt-key-a");
    assert_eq!(value["current_key_id"], "receipt-key-b");
    assert_eq!(value["audit_chain_head"], audit_chain_head);
    assert_eq!(value["audit_sequence"], 42);
    for field in [
        "trust_manifest_hash",
        "reanchor_hash",
        "previous_signature_hex",
        "current_signature_hex",
    ] {
        assert!(value[field].as_str().unwrap().len() >= 64);
    }

    let verified = run(vec![
        "cortexdb".to_owned(),
        "receipt-key".to_owned(),
        "verify-reanchor".to_owned(),
        reanchor.to_string_lossy().into_owned(),
        "--trust-file".to_owned(),
        trust.to_string_lossy().into_owned(),
    ])
    .unwrap();
    assert!(verified.contains("receipt re-anchor verified"));

    let mut tampered_value = value;
    tampered_value["audit_chain_head"] = serde_json::Value::String("b".repeat(64));
    std::fs::write(
        &tampered,
        serde_json::to_string_pretty(&tampered_value).unwrap(),
    )
    .unwrap();
    let rejected = run(vec![
        "cortexdb".to_owned(),
        "receipt-key".to_owned(),
        "verify-reanchor".to_owned(),
        tampered.to_string_lossy().into_owned(),
        "--trust-file".to_owned(),
        trust.to_string_lossy().into_owned(),
    ])
    .unwrap_err();
    assert!(rejected.contains("invalid receipt re-anchor"));

    let _ = std::fs::remove_dir_all(root);
}

fn run(args: Vec<String>) -> Result<String, String> {
    crate::run(args)
}

fn unique_path(prefix: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "{prefix}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
