use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use cortex_crypto::{KeyId, ReceiptPublicKey, ReceiptSigningKey};
use serde::{Deserialize, Serialize};

mod reanchor;

const SIGNING_KEY_SCHEMA: &str = "cortexdb.receipt_signing_key.v1";
const PUBLIC_KEY_SCHEMA: &str = "cortexdb.receipt_public_key.v1";
const TRUST_SCHEMA: &str = "cortexdb.receipt_trust.v1";

#[derive(Serialize, Deserialize)]
struct ReceiptSigningKeyFile {
    schema_version: String,
    key_id: String,
    signing_seed_hex: String,
    public_key_hex: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct ReceiptPublicKeyFile {
    schema_version: String,
    key_id: String,
    public_key_hex: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct ReceiptTrustFile {
    schema_version: String,
    current_key_id: String,
    trusted_public_keys: Vec<ReceiptPublicKeyFile>,
}

pub(crate) fn generate(
    key_file: String,
    key_id: String,
    public_key_file: Option<String>,
) -> Result<String, String> {
    let signing_key =
        ReceiptSigningKey::generate(KeyId::new(key_id).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    write_signing_key_file(Path::new(&key_file), &signing_key)?;
    let mut output = format!(
        "receipt signing key generated: key_id={} key_file={}",
        signing_key.key_id(),
        key_file
    );
    if let Some(public_key_file) = public_key_file {
        write_public_key_file(Path::new(&public_key_file), &signing_key.public_key())?;
        output.push_str(&format!(" public_key_file={public_key_file}"));
    }
    Ok(output)
}

pub(crate) fn export_public(key_file: String, public_key_file: String) -> Result<String, String> {
    let signing_key = read_signing_key_file(Path::new(&key_file))?;
    write_public_key_file(Path::new(&public_key_file), &signing_key.public_key())?;
    Ok(format!(
        "receipt public key exported: key_id={} public_key_file={}",
        signing_key.key_id(),
        public_key_file
    ))
}

pub(crate) fn rotate(
    current_key_file: String,
    next_key_file: String,
    trust_file: String,
    new_key_id: String,
    reanchor_file: Option<String>,
    audit_chain_head: Option<String>,
    audit_sequence: u64,
) -> Result<String, String> {
    let current = read_signing_key_file(Path::new(&current_key_file))?;
    let next =
        ReceiptSigningKey::generate(KeyId::new(new_key_id).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    if current.key_id() == next.key_id() {
        return Err("new receipt key id must differ from the current key id".to_owned());
    }
    let trust = trust_file_value(&next, &current);
    write_signing_key_file(Path::new(&next_key_file), &next)?;
    write_trust_file(Path::new(&trust_file), &trust)?;
    let mut output = format!(
        "receipt signing key rotated: previous_key_id={} current_key_id={} next_key_file={} trust_file={}",
        current.key_id(),
        next.key_id(),
        next_key_file,
        trust_file
    );
    if let Some(reanchor_file) = reanchor_file {
        let audit_chain_head = audit_chain_head
            .unwrap_or_else(|| crate::cli_audit_chain::AUDIT_CHAIN_ZERO_HASH.to_owned());
        let reanchor = reanchor::build_reanchor_record(
            &current,
            &next,
            &trust,
            &audit_chain_head,
            audit_sequence,
        )?;
        write_json_new(Path::new(&reanchor_file), &reanchor)?;
        output.push_str(&format!(" reanchor_file={reanchor_file}"));
    }
    Ok(output)
}

pub(crate) fn verify_reanchor(
    reanchor_file: String,
    trust_file: Option<String>,
) -> Result<String, String> {
    let trust = trust_file
        .as_deref()
        .map(|path| read_trust_file(Path::new(path)))
        .transpose()?;
    let record = reanchor::read_and_verify_reanchor(Path::new(&reanchor_file), trust.as_ref())
        .map_err(|error| format!("invalid receipt re-anchor: {error}"))?;
    Ok(format!(
        "receipt re-anchor verified: previous_key_id={} current_key_id={} reanchor_file={}",
        record.previous_key_id, record.current_key_id, reanchor_file
    ))
}

fn read_signing_key_file(path: &Path) -> Result<ReceiptSigningKey, String> {
    let raw = fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read receipt signing key file {}: {error}",
            path.display()
        )
    })?;
    let file: ReceiptSigningKeyFile = serde_json::from_str(&raw)
        .map_err(|error| format!("invalid receipt signing key JSON: {error}"))?;
    if file.schema_version != SIGNING_KEY_SCHEMA {
        return Err(format!(
            "receipt signing key schema_version must be {SIGNING_KEY_SCHEMA}"
        ));
    }
    let key = ReceiptSigningKey::from_seed_hex(&file.key_id, &file.signing_seed_hex)
        .map_err(|error| error.to_string())?;
    if key.public_key().to_hex() != file.public_key_hex.trim() {
        return Err("receipt signing key public_key_hex does not match signing seed".to_owned());
    }
    Ok(key)
}

fn signing_key_file(key: &ReceiptSigningKey) -> ReceiptSigningKeyFile {
    ReceiptSigningKeyFile {
        schema_version: SIGNING_KEY_SCHEMA.to_owned(),
        key_id: key.key_id().to_owned(),
        signing_seed_hex: key.seed_hex(),
        public_key_hex: key.public_key().to_hex(),
    }
}

fn public_key_file(key: &ReceiptPublicKey) -> ReceiptPublicKeyFile {
    ReceiptPublicKeyFile {
        schema_version: PUBLIC_KEY_SCHEMA.to_owned(),
        key_id: key.key_id().to_owned(),
        public_key_hex: key.to_hex(),
    }
}

fn trust_file_value(current: &ReceiptSigningKey, previous: &ReceiptSigningKey) -> ReceiptTrustFile {
    ReceiptTrustFile {
        schema_version: TRUST_SCHEMA.to_owned(),
        current_key_id: current.key_id().to_owned(),
        trusted_public_keys: vec![
            public_key_file(&current.public_key()),
            public_key_file(&previous.public_key()),
        ],
    }
}

fn write_signing_key_file(path: &Path, key: &ReceiptSigningKey) -> Result<(), String> {
    write_json_new(path, &signing_key_file(key))
}

fn write_public_key_file(path: &Path, key: &ReceiptPublicKey) -> Result<(), String> {
    write_json_new(path, &public_key_file(key))
}

fn write_trust_file(path: &Path, trust: &ReceiptTrustFile) -> Result<(), String> {
    write_json_new(path, trust)
}

fn read_trust_file(path: &Path) -> Result<ReceiptTrustFile, String> {
    let raw = fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read receipt trust file {}: {error}",
            path.display()
        )
    })?;
    let file: ReceiptTrustFile = serde_json::from_str(&raw)
        .map_err(|error| format!("invalid receipt trust JSON: {error}"))?;
    if file.schema_version != TRUST_SCHEMA {
        return Err(format!(
            "receipt trust schema_version must be {TRUST_SCHEMA}"
        ));
    }
    Ok(file)
}

fn write_json_new<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    let json = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    file.write_all(json.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}
