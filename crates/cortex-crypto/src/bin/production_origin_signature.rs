use std::env;
use std::io::{self, Read};
use std::process::ExitCode;

use cortex_crypto::{ed25519_public_key, ed25519_sign, ed25519_verify, hex_lower, SigningSeed};

fn main() -> ExitCode {
    match run(env::args().collect()) {
        Ok(output) => {
            if !output.is_empty() {
                println!("{output}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<String, String> {
    let command = args.get(1).map(String::as_str).ok_or_else(usage)?;
    let stdin = read_stdin()?;
    match command {
        "verify" => {
            let public_key_hex = flag_value(&args, "--public-key-hex")?;
            let signature_hex = flag_value(&args, "--signature-hex")?;
            verify_signature(public_key_hex, signature_hex, &stdin)?;
            Ok("production origin signature verified".to_owned())
        }
        "sign-fixture" => {
            let seed_hex = flag_value(&args, "--seed-hex")?;
            let seed = SigningSeed::new(decode_hex_array("seed", seed_hex)?);
            let public_key = ed25519_public_key(&seed);
            let signature = ed25519_sign(&seed, &stdin);
            Ok(format!(
                "public_key_hex={}\nsignature_hex={}",
                hex_lower(&public_key),
                hex_lower(&signature)
            ))
        }
        _ => Err(usage()),
    }
}

fn verify_signature(
    public_key_hex: &str,
    signature_hex: &str,
    message: &[u8],
) -> Result<(), String> {
    let public_key = decode_hex_array("public key", public_key_hex)?;
    let signature = decode_hex_array("signature", signature_hex)?;
    ed25519_verify(&public_key, message, &signature)
        .map_err(|_| "production origin signature verification failed".to_owned())
}

fn read_stdin() -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    io::stdin()
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read stdin: {error}"))?;
    Ok(bytes)
}

fn flag_value<'a>(args: &'a [String], flag: &str) -> Result<&'a str, String> {
    args.windows(2)
        .find_map(|pair| (pair[0] == flag).then_some(pair[1].as_str()))
        .ok_or_else(|| format!("missing required flag {flag}\n{}", usage()))
}

fn decode_hex_array<const N: usize>(name: &'static str, value: &str) -> Result<[u8; N], String> {
    if value.len() != N * 2 {
        return Err(format!("{name} must be {} lowercase hex characters", N * 2));
    }
    let mut out = [0_u8; N];
    for index in 0..N {
        let byte = &value[index * 2..index * 2 + 2];
        if !byte.bytes().all(|item| item.is_ascii_hexdigit()) {
            return Err(format!("{name} must be hex"));
        }
        out[index] = u8::from_str_radix(byte, 16).map_err(|_| format!("{name} must be hex"))?;
    }
    Ok(out)
}

fn usage() -> String {
    "usage: production_origin_signature verify --public-key-hex <64-hex> --signature-hex <128-hex>\n\
     usage: production_origin_signature sign-fixture --seed-hex <64-hex>"
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RFC8032_SEED: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";

    #[test]
    fn verifies_statement_signature_and_rejects_tamper() {
        let message = b"cortexdb.operator_evidence_origin_statement.sign.v1\0{}";
        let seed = SigningSeed::new(decode_hex_array("seed", RFC8032_SEED).unwrap());
        let public_key = hex_lower(&ed25519_public_key(&seed));
        let signature = hex_lower(&ed25519_sign(&seed, message));
        verify_signature(&public_key, &signature, message).unwrap();
        assert!(verify_signature(&public_key, &signature, b"tampered").is_err());
    }
}
