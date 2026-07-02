use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use crate::types::SigningSeed;
use crate::{CryptoError, CryptoResult};

pub fn generate_signing_seed() -> CryptoResult<SigningSeed> {
    let mut seed = [0_u8; 32];
    getrandom::getrandom(&mut seed).map_err(|error| CryptoError::Random(error.to_string()))?;
    Ok(SigningSeed::new(seed))
}

pub fn ed25519_public_key(seed: &SigningSeed) -> [u8; 32] {
    SigningKey::from_bytes(seed.as_array())
        .verifying_key()
        .to_bytes()
}

pub fn ed25519_sign(seed: &SigningSeed, message: &[u8]) -> [u8; 64] {
    SigningKey::from_bytes(seed.as_array())
        .sign(message)
        .to_bytes()
}

pub fn ed25519_verify(
    public_key: &[u8; 32],
    message: &[u8],
    signature: &[u8; 64],
) -> CryptoResult<()> {
    let verifying_key =
        VerifyingKey::from_bytes(public_key).map_err(|_| CryptoError::Ed25519Key)?;
    let signature = Signature::from_bytes(signature);
    verifying_key
        .verify(message, &signature)
        .map_err(|_| CryptoError::Ed25519Signature)
}

pub fn ed25519_verify_bool(public_key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
    ed25519_verify(public_key, message, signature).is_ok()
}
