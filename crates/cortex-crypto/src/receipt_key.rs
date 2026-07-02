use std::fmt;

use crate::{
    ed25519_public_key, ed25519_sign, ed25519_verify, generate_signing_seed, hex_lower,
    CryptoError, CryptoResult, KeyId, SigningSeed,
};

pub const RECEIPT_SIGNING_DOMAIN: &str = "cortexdb.accountability_receipt.sign.v1";

pub struct ReceiptSigningKey {
    key_id: KeyId,
    seed: SigningSeed,
}

impl ReceiptSigningKey {
    pub fn generate(key_id: KeyId) -> CryptoResult<Self> {
        Ok(Self {
            key_id,
            seed: generate_signing_seed()?,
        })
    }

    pub fn from_seed(key_id: KeyId, seed: SigningSeed) -> Self {
        Self { key_id, seed }
    }

    pub fn from_seed_hex(key_id: &str, seed_hex: &str) -> CryptoResult<Self> {
        let key_id = KeyId::new(key_id.to_owned())?;
        let seed = SigningSeed::new(decode_hex_array("receipt signing seed", seed_hex)?);
        Ok(Self { key_id, seed })
    }

    pub fn key_id(&self) -> &str {
        self.key_id.as_str()
    }

    pub fn seed_hex(&self) -> String {
        hex_lower(self.seed.as_bytes())
    }

    pub fn public_key(&self) -> ReceiptPublicKey {
        ReceiptPublicKey {
            key_id: self.key_id.clone(),
            bytes: ed25519_public_key(&self.seed),
        }
    }

    pub fn sign(&self, message: &[u8]) -> ReceiptSignature {
        ReceiptSignature(ed25519_sign(&self.seed, &receipt_signing_bytes(message)))
    }
}

impl fmt::Debug for ReceiptSigningKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReceiptSigningKey")
            .field("key_id", &self.key_id)
            .field("seed", &"redacted")
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiptPublicKey {
    key_id: KeyId,
    bytes: [u8; 32],
}

impl ReceiptPublicKey {
    pub fn new(key_id: KeyId, bytes: [u8; 32]) -> Self {
        Self { key_id, bytes }
    }

    pub fn from_hex(key_id: &str, public_key_hex: &str) -> CryptoResult<Self> {
        Ok(Self {
            key_id: KeyId::new(key_id.to_owned())?,
            bytes: decode_hex_array("receipt public key", public_key_hex)?,
        })
    }

    pub fn key_id(&self) -> &str {
        self.key_id.as_str()
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }

    pub fn to_hex(&self) -> String {
        hex_lower(&self.bytes)
    }

    pub fn verify(&self, message: &[u8], signature: &ReceiptSignature) -> CryptoResult<()> {
        ed25519_verify(
            &self.bytes,
            &receipt_signing_bytes(message),
            signature.as_bytes(),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReceiptSignature([u8; 64]);

impl ReceiptSignature {
    pub fn from_hex(signature_hex: &str) -> CryptoResult<Self> {
        Ok(Self(decode_hex_array("receipt signature", signature_hex)?))
    }

    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex_lower(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiptKeyRing {
    trusted: Vec<ReceiptPublicKey>,
}

impl ReceiptKeyRing {
    pub fn new(trusted: Vec<ReceiptPublicKey>) -> CryptoResult<Self> {
        if trusted.is_empty() {
            return Err(CryptoError::InvalidLength {
                name: "receipt trusted keys",
                expected: 1,
                actual: 0,
            });
        }
        Ok(Self { trusted })
    }

    pub fn trusted_public_keys(&self) -> &[ReceiptPublicKey] {
        &self.trusted
    }

    pub fn contains_key_id(&self, key_id: &str) -> bool {
        self.trusted.iter().any(|key| key.key_id() == key_id)
    }

    pub fn verify(
        &self,
        key_id: &str,
        message: &[u8],
        signature: &ReceiptSignature,
    ) -> CryptoResult<()> {
        let Some(key) = self.trusted.iter().find(|key| key.key_id() == key_id) else {
            return Err(CryptoError::Ed25519Key);
        };
        key.verify(message, signature)
    }
}

fn receipt_signing_bytes(message: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(RECEIPT_SIGNING_DOMAIN.len() + 1 + message.len());
    bytes.extend_from_slice(RECEIPT_SIGNING_DOMAIN.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(message);
    bytes
}

fn decode_hex_array<const N: usize>(name: &'static str, value: &str) -> CryptoResult<[u8; N]> {
    let value = value.trim();
    if value.len() != N * 2 {
        return Err(CryptoError::InvalidLength {
            name,
            expected: N * 2,
            actual: value.len(),
        });
    }
    let mut bytes = [0_u8; N];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_nibble(chunk[0]).ok_or(CryptoError::InvalidHex { name })?;
        let low = hex_nibble(chunk[1]).ok_or(CryptoError::InvalidHex { name })?;
        bytes[index] = (high << 4) | low;
    }
    Ok(bytes)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SEED_A: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
    const TEST_SEED_B: &str = "1f1e1d1c1b1a191817161514131211100f0e0d0c0b0a09080706050403020100";

    #[test]
    fn receipt_signatures_are_deterministic_and_domain_separated() {
        let key = ReceiptSigningKey::from_seed_hex("receipt-key-a", TEST_SEED_A).unwrap();
        let message = b"pack_root=abc";
        let signature = key.sign(message);
        assert_eq!(signature, key.sign(message));
        assert_eq!(key.public_key().key_id(), "receipt-key-a");
        key.public_key().verify(message, &signature).unwrap();
        assert!(key
            .public_key()
            .verify(b"pack_root=def", &signature)
            .is_err());
        assert!(!format!("{key:?}").contains(TEST_SEED_A));
    }

    #[test]
    fn receipt_keyring_verifies_current_and_previous_keys() {
        let previous = ReceiptSigningKey::from_seed_hex("receipt-key-2026q2", TEST_SEED_A).unwrap();
        let current = ReceiptSigningKey::from_seed_hex("receipt-key-2026q3", TEST_SEED_B).unwrap();
        let previous_signature = previous.sign(b"old receipt header");
        let current_signature = current.sign(b"new receipt header");
        let keyring =
            ReceiptKeyRing::new(vec![current.public_key(), previous.public_key()]).unwrap();
        assert!(keyring.contains_key_id("receipt-key-2026q2"));
        keyring
            .verify(
                "receipt-key-2026q2",
                b"old receipt header",
                &previous_signature,
            )
            .unwrap();
        keyring
            .verify(
                "receipt-key-2026q3",
                b"new receipt header",
                &current_signature,
            )
            .unwrap();
        assert!(keyring
            .verify(
                "receipt-key-2026q3",
                b"old receipt header",
                &previous_signature,
            )
            .is_err());
    }

    #[test]
    fn receipt_hex_inputs_are_validated() {
        assert!(ReceiptSigningKey::from_seed_hex("bad key id", TEST_SEED_A).is_err());
        assert!(ReceiptSigningKey::from_seed_hex("receipt-key-a", "abcd").is_err());
        assert!(ReceiptPublicKey::from_hex("receipt-key-a", "zz").is_err());
        assert!(ReceiptKeyRing::new(Vec::new()).is_err());
    }
}
