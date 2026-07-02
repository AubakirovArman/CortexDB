use std::fmt;

pub type CryptoResult<T> = Result<T, CryptoError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    InvalidLength {
        name: &'static str,
        expected: usize,
        actual: usize,
    },
    InvalidHex {
        name: &'static str,
    },
    InvalidKeyId,
    AeadOpenFailed,
    Argon2(String),
    Ed25519Key,
    Ed25519Signature,
    Random(String),
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength {
                name,
                expected,
                actual,
            } => write!(
                f,
                "{name} length {actual} does not match expected {expected}"
            ),
            Self::InvalidHex { name } => write!(f, "{name} contains non-hex data"),
            Self::InvalidKeyId => write!(f, "key id is invalid"),
            Self::AeadOpenFailed => write!(f, "AEAD open failed"),
            Self::Argon2(error) => write!(f, "Argon2id failed: {error}"),
            Self::Ed25519Key => write!(f, "Ed25519 key is invalid"),
            Self::Ed25519Signature => write!(f, "Ed25519 signature is invalid"),
            Self::Random(error) => write!(f, "secure random source failed: {error}"),
        }
    }
}

impl std::error::Error for CryptoError {}
