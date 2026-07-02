use cortex_crypto::{
    derive_argon2id_key, hex_lower, xchacha20poly1305_open, xchacha20poly1305_seal, AeadNonce,
    AeadTag, KdfParams, Salt16, ARGON2ID_V1_PARAMS,
};

use crate::error::{EngineError, EngineResult};

pub(super) const CIPHER_SUITE: &str = "cortexdb.xchacha20poly1305-argon2id.v2";
pub(super) const KDF: &str = "cortexdb.argon2id.v1";
pub(super) const SCHEMA_VERSION: &str = "cortexdb.encrypted_backup.v2";
pub(super) const LEGACY_SCHEMA_VERSION: &str = "cortexdb.encrypted_backup.v1";

pub(super) struct SealedArchive {
    pub(super) ciphertext: Vec<u8>,
    pub(super) tag_hex: String,
}

pub(super) fn kdf_params() -> KdfParams {
    ARGON2ID_V1_PARAMS
}

pub(super) fn generate_salt_hex() -> EngineResult<String> {
    let salt = Salt16::random().map_err(to_storage_invariant)?;
    Ok(hex_lower(salt.as_bytes()))
}

pub(super) fn generate_nonce_hex() -> EngineResult<String> {
    let nonce = AeadNonce::random().map_err(to_storage_invariant)?;
    Ok(hex_lower(nonce.as_bytes()))
}

pub(super) fn seal_archive(
    passphrase: &str,
    salt_hex: &str,
    nonce_hex: &str,
    aad: &[u8],
    plaintext: &[u8],
) -> EngineResult<SealedArchive> {
    let salt = Salt16::new(decode_hex_array("encrypted backup salt", salt_hex)?);
    let nonce = AeadNonce::new(decode_hex_array("encrypted backup nonce", nonce_hex)?);
    let key = derive_argon2id_key(passphrase, &salt, kdf_params()).map_err(to_storage_invariant)?;
    let sealed =
        xchacha20poly1305_seal(&key, &nonce, aad, plaintext).map_err(to_storage_invariant)?;
    Ok(SealedArchive {
        ciphertext: sealed.ciphertext,
        tag_hex: hex_lower(sealed.tag.as_bytes()),
    })
}

pub(super) fn open_archive(
    passphrase: &str,
    salt_hex: &str,
    nonce_hex: &str,
    tag_hex: &str,
    aad: &[u8],
    ciphertext: &[u8],
) -> EngineResult<Vec<u8>> {
    let salt = Salt16::new(decode_hex_array("encrypted backup salt", salt_hex)?);
    let nonce = AeadNonce::new(decode_hex_array("encrypted backup nonce", nonce_hex)?);
    let tag = AeadTag::new(decode_hex_array("encrypted backup tag", tag_hex)?);
    let key = derive_argon2id_key(passphrase, &salt, kdf_params()).map_err(to_storage_invariant)?;
    xchacha20poly1305_open(&key, &nonce, aad, ciphertext, &tag).map_err(|_| {
        EngineError::StorageInvariant(
            "encrypted backup passphrase or authentication tag is invalid".to_owned(),
        )
    })
}

fn decode_hex_array<const N: usize>(name: &'static str, value: &str) -> EngineResult<[u8; N]> {
    if value.len() != N * 2 {
        return Err(EngineError::StorageInvariant(format!(
            "{name} has invalid length"
        )));
    }
    let mut out = [0_u8; N];
    for index in 0..N {
        out[index] = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| EngineError::StorageInvariant(format!("{name} is not hex")))?;
    }
    Ok(out)
}

fn to_storage_invariant(error: cortex_crypto::CryptoError) -> EngineError {
    EngineError::StorageInvariant(format!("encrypted backup crypto failed: {error}"))
}
