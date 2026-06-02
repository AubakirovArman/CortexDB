use std::path::Path;

pub(super) const CIPHER_SUITE: &str = "cortexdb.xor-fnv64-stream.v1";
pub(super) const KDF: &str = "cortexdb.fnv64-passphrase.v1";
pub(super) const SCHEMA_VERSION: &str = "cortexdb.encrypted_backup.v1";

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

pub(super) fn apply_keystream(input: &[u8], passphrase: &str, nonce: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    for (index, byte) in input.iter().enumerate() {
        let block = stream_word(passphrase, nonce, index as u64 / 8).to_le_bytes();
        out.push(*byte ^ block[index % 8]);
    }
    out
}

pub(super) fn archive_nonce(
    source: &Path,
    archive_path: &Path,
    file_count: usize,
    plaintext_len: usize,
) -> u64 {
    let mut hash = FNV_OFFSET;
    feed_hash(&mut hash, source.to_string_lossy().as_bytes());
    feed_hash(&mut hash, archive_path.to_string_lossy().as_bytes());
    feed_hash(&mut hash, &(file_count as u64).to_le_bytes());
    feed_hash(&mut hash, &(plaintext_len as u64).to_le_bytes());
    hash
}

pub(super) fn auth_tag(
    passphrase: &str,
    nonce: u64,
    plaintext: &[u8],
    ciphertext: &[u8],
) -> String {
    let mut hash = FNV_OFFSET;
    feed_hash(&mut hash, passphrase.as_bytes());
    feed_hash(&mut hash, &nonce.to_le_bytes());
    feed_hash(&mut hash, hash_hex(plaintext).as_bytes());
    feed_hash(&mut hash, hash_hex(ciphertext).as_bytes());
    format!("{hash:016x}")
}

pub(super) fn hash_hex(bytes: &[u8]) -> String {
    let mut hash = FNV_OFFSET;
    feed_hash(&mut hash, bytes);
    format!("{hash:016x}")
}

fn stream_word(passphrase: &str, nonce: u64, counter: u64) -> u64 {
    let mut hash = FNV_OFFSET;
    feed_hash(&mut hash, passphrase.as_bytes());
    feed_hash(&mut hash, &nonce.to_le_bytes());
    feed_hash(&mut hash, &counter.to_le_bytes());
    hash
}

fn feed_hash(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}
