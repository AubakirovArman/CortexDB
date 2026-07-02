use chacha20poly1305::aead::{AeadInPlace, KeyInit};
use chacha20poly1305::{Tag, XChaCha20Poly1305, XNonce};

use crate::types::{AeadKey, AeadNonce, AeadTag};
use crate::{CryptoError, CryptoResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SealedAead {
    pub ciphertext: Vec<u8>,
    pub tag: AeadTag,
}

pub fn xchacha20poly1305_seal(
    key: &AeadKey,
    nonce: &AeadNonce,
    aad: &[u8],
    plaintext: &[u8],
) -> CryptoResult<SealedAead> {
    let cipher = XChaCha20Poly1305::new_from_slice(key.as_bytes()).map_err(|_| {
        CryptoError::InvalidLength {
            name: "xchacha20poly1305 key",
            expected: 32,
            actual: key.as_bytes().len(),
        }
    })?;
    let mut ciphertext = plaintext.to_vec();
    let tag = cipher
        .encrypt_in_place_detached(XNonce::from_slice(nonce.as_bytes()), aad, &mut ciphertext)
        .map_err(|_| CryptoError::AeadOpenFailed)?;
    let mut tag_bytes = [0_u8; 16];
    tag_bytes.copy_from_slice(&tag);
    Ok(SealedAead {
        ciphertext,
        tag: AeadTag::new(tag_bytes),
    })
}

pub fn xchacha20poly1305_open(
    key: &AeadKey,
    nonce: &AeadNonce,
    aad: &[u8],
    ciphertext: &[u8],
    tag: &AeadTag,
) -> CryptoResult<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new_from_slice(key.as_bytes()).map_err(|_| {
        CryptoError::InvalidLength {
            name: "xchacha20poly1305 key",
            expected: 32,
            actual: key.as_bytes().len(),
        }
    })?;
    let mut plaintext = ciphertext.to_vec();
    cipher
        .decrypt_in_place_detached(
            XNonce::from_slice(nonce.as_bytes()),
            aad,
            &mut plaintext,
            Tag::from_slice(tag.as_bytes()),
        )
        .map_err(|_| CryptoError::AeadOpenFailed)?;
    Ok(plaintext)
}
