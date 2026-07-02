use cortex_crypto::{
    blake3_256, constant_time_eq, derive_argon2id_key, ed25519_public_key, ed25519_sign,
    ed25519_verify, ed25519_verify_bool, hex_lower, hmac_sha256, sha256, verify_hmac_sha256,
    xchacha20poly1305_open, xchacha20poly1305_seal, AeadKey, AeadNonce, KdfParams, MacKey, Salt16,
    SigningSeed,
};

fn decode_hex<const N: usize>(hex: &str) -> [u8; N] {
    assert_eq!(hex.len(), N * 2);
    let mut out = [0_u8; N];
    for index in 0..N {
        out[index] = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).unwrap();
    }
    out
}

#[test]
fn sha256_and_blake3_known_answer_vectors_match() {
    assert_eq!(
        hex_lower(&sha256(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    );
    assert_eq!(
        hex_lower(&blake3_256(b"abc")),
        "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85",
    );
}

#[test]
fn hmac_sha256_known_answer_vector_and_constant_time_verify_match() {
    let key = MacKey::new([13_u8; 32]);
    let message = b"The quick brown fox jumps over the lazy dog";
    let tag = hmac_sha256(&key, message);
    assert_eq!(
        hex_lower(&tag),
        "9ba772c9dae9ec6279110e171aed78237404a4a71012dfe4fb6364cfcd7bfc52",
    );
    assert!(verify_hmac_sha256(&key, message, &tag));
    assert!(!verify_hmac_sha256(&key, b"tampered", &tag));
    assert!(constant_time_eq(&tag, &tag));
    assert!(!constant_time_eq(&tag, &tag[..31]));
}

#[test]
fn ed25519_rfc8032_known_answer_vector_signs_and_verifies() {
    let seed = SigningSeed::new(decode_hex(
        "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
    ));
    let public_key = ed25519_public_key(&seed);
    let signature = ed25519_sign(&seed, b"");
    assert_eq!(
        hex_lower(&public_key),
        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
    );
    assert_eq!(
        hex_lower(&signature),
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
    );
    ed25519_verify(&public_key, b"", &signature).unwrap();
    assert!(ed25519_verify_bool(&public_key, b"", &signature));
    assert!(!ed25519_verify_bool(&public_key, b"tampered", &signature));
}

#[test]
fn xchacha20poly1305_known_answer_vector_opens_and_rejects_tamper() {
    let key = AeadKey::new([7_u8; 32]);
    let nonce = AeadNonce::new([11_u8; 24]);
    let aad = b"cortexdb.backup.header.v2";
    let plaintext = b"cortexdb encrypted backup payload";
    let sealed = xchacha20poly1305_seal(&key, &nonce, aad, plaintext).unwrap();
    assert_eq!(
        hex_lower(&sealed.ciphertext),
        "23a99d5c6e54c3b0b72bb15532713208be4f5da49e2136a62033768ffdcc5f23f2",
    );
    assert_eq!(
        hex_lower(sealed.tag.as_bytes()),
        "47116822c39908d96afd0fc46ce22ea7",
    );
    assert_eq!(
        xchacha20poly1305_open(&key, &nonce, aad, &sealed.ciphertext, &sealed.tag).unwrap(),
        plaintext,
    );
    let mut tampered = sealed.ciphertext.clone();
    tampered[0] ^= 0x01;
    assert!(xchacha20poly1305_open(&key, &nonce, aad, &tampered, &sealed.tag).is_err());
}

#[test]
fn argon2id_known_answer_vector_matches_pinned_params() {
    let params = KdfParams {
        memory_cost_kib: 32,
        time_cost: 3,
        parallelism: 1,
        output_len: 32,
    };
    let salt = Salt16::new(*b"cortexdb-kdf-v1!");
    let key = derive_argon2id_key("correct horse battery staple", &salt, params).unwrap();
    assert_eq!(
        hex_lower(key.as_bytes()),
        "f8f9afb0d5ae2ab442e437696b4da2f93e6f54fd9104bb3058119ba137de0e49",
    );
}
