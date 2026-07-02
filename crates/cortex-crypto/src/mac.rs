use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::compare::constant_time_eq;
use crate::types::MacKey;

type HmacSha256 = Hmac<Sha256>;

pub fn hmac_sha256(key: &MacKey, bytes: &[u8]) -> [u8; 32] {
    let mut mac =
        HmacSha256::new_from_slice(key.as_bytes()).expect("HMAC-SHA-256 accepts any key length");
    mac.update(bytes);
    mac.finalize().into_bytes().into()
}

pub fn verify_hmac_sha256(key: &MacKey, bytes: &[u8], expected: &[u8]) -> bool {
    let actual = hmac_sha256(key, bytes);
    constant_time_eq(&actual, expected)
}
