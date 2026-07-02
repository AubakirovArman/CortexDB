use sha2::{Digest, Sha256};

pub type Digest32 = [u8; 32];

pub fn sha256(bytes: &[u8]) -> Digest32 {
    let digest = Sha256::digest(bytes);
    digest.into()
}

pub fn sha256_domain(domain: &str, bytes: &[u8]) -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    hasher.update([0]);
    hasher.update(bytes);
    hasher.finalize().into()
}

pub fn blake3_256(bytes: &[u8]) -> Digest32 {
    *blake3::hash(bytes).as_bytes()
}

pub fn blake3_256_domain(domain: &str, bytes: &[u8]) -> Digest32 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain.as_bytes());
    hasher.update(&[0]);
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
}

pub fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
