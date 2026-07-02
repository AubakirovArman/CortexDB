use std::fmt;

use zeroize::Zeroize;

use crate::{CryptoError, CryptoResult};

pub type AeadKey = SecretBytes<32>;
pub type MacKey = SecretBytes<32>;
pub type SigningSeed = SecretBytes<32>;

#[derive(PartialEq, Eq)]
pub struct SecretBytes<const N: usize>([u8; N]);

impl<const N: usize> SecretBytes<N> {
    pub fn new(bytes: [u8; N]) -> Self {
        Self(bytes)
    }

    pub fn from_slice(name: &'static str, bytes: &[u8]) -> CryptoResult<Self> {
        let array: [u8; N] = bytes.try_into().map_err(|_| CryptoError::InvalidLength {
            name,
            expected: N,
            actual: bytes.len(),
        })?;
        Ok(Self(array))
    }

    pub fn as_array(&self) -> &[u8; N] {
        &self.0
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl<const N: usize> Drop for SecretBytes<N> {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl<const N: usize> fmt::Debug for SecretBytes<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretBytes<{N}>(redacted)")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Salt16([u8; 16]);

impl Salt16 {
    pub fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub fn random() -> CryptoResult<Self> {
        let mut bytes = [0_u8; 16];
        getrandom::getrandom(&mut bytes).map_err(|error| CryptoError::Random(error.to_string()))?;
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AeadNonce([u8; 24]);

impl AeadNonce {
    pub fn new(bytes: [u8; 24]) -> Self {
        Self(bytes)
    }

    pub fn random() -> CryptoResult<Self> {
        let mut bytes = [0_u8; 24];
        getrandom::getrandom(&mut bytes).map_err(|error| CryptoError::Random(error.to_string()))?;
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 24] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AeadTag([u8; 16]);

impl AeadTag {
    pub fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyId(String);

impl KeyId {
    pub fn new(value: impl Into<String>) -> CryptoResult<Self> {
        let value = value.into();
        let valid = !value.is_empty()
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
        if !valid {
            return Err(CryptoError::InvalidKeyId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
