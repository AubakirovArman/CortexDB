pub mod aead;
pub mod audit_chain;
pub mod compare;
pub mod ed25519;
pub mod error;
pub mod hash;
pub mod kdf;
pub mod mac;
pub mod receipt_key;
pub mod types;

pub use aead::{xchacha20poly1305_open, xchacha20poly1305_seal, SealedAead};
pub use compare::constant_time_eq;
pub use ed25519::{
    ed25519_public_key, ed25519_sign, ed25519_verify, ed25519_verify_bool, generate_signing_seed,
};
pub use error::{CryptoError, CryptoResult};
pub use hash::{blake3_256, blake3_256_domain, hex_lower, sha256, sha256_domain, Digest32};
pub use kdf::{derive_argon2id_key, KdfParams, ARGON2ID_V1_PARAMS};
pub use mac::{hmac_sha256, verify_hmac_sha256};
pub use receipt_key::{
    ReceiptKeyRing, ReceiptPublicKey, ReceiptSignature, ReceiptSigningKey, RECEIPT_SIGNING_DOMAIN,
};
pub use types::{AeadKey, AeadNonce, AeadTag, KeyId, MacKey, Salt16, SecretBytes, SigningSeed};
