use argon2::{Algorithm, Argon2, Params, Version};

use crate::types::{Salt16, SecretBytes};
use crate::{CryptoError, CryptoResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KdfParams {
    pub memory_cost_kib: u32,
    pub time_cost: u32,
    pub parallelism: u32,
    pub output_len: usize,
}

pub const ARGON2ID_V1_PARAMS: KdfParams = KdfParams {
    memory_cost_kib: 19 * 1024,
    time_cost: 2,
    parallelism: 1,
    output_len: 32,
};

pub fn derive_argon2id_key(
    passphrase: &str,
    salt: &Salt16,
    params: KdfParams,
) -> CryptoResult<SecretBytes<32>> {
    if params.output_len != 32 {
        return Err(CryptoError::InvalidLength {
            name: "argon2id output",
            expected: 32,
            actual: params.output_len,
        });
    }
    let params = Params::new(
        params.memory_cost_kib,
        params.time_cost,
        params.parallelism,
        Some(params.output_len),
    )
    .map_err(|error| CryptoError::Argon2(error.to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut output = [0_u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt.as_bytes(), &mut output)
        .map_err(|error| CryptoError::Argon2(error.to_string()))?;
    Ok(SecretBytes::new(output))
}
