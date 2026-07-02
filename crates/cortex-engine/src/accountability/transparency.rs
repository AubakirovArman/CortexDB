use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::accountability::receipt::hash_value;
use crate::error::{EngineError, EngineResult};

pub const TRANSPARENCY_LOG_RECORD_SCHEMA: &str = "cortexdb.transparency.log.record.v1";
pub const TRANSPARENCY_LOG_RECORD_HASH_DOMAIN: &str = "cortexdb.transparency.log.record_hash.v1";
pub const TRANSPARENCY_LOG_ZERO_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

static TRANSPARENCY_LOG_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyLogRecord {
    pub schema_version: String,
    pub sequence: u64,
    pub previous_record_hash: String,
    pub record_hash: String,
    pub db_instance_id: String,
    pub key_id: String,
    pub created_unix_seconds: u64,
    pub pack_root: String,
    pub determinism_hash: String,
    pub receipt_signature_hex: String,
}

pub fn append_transparency_log_record(
    path: &Path,
    receipt: &Value,
) -> EngineResult<TransparencyLogRecord> {
    let _guard = TRANSPARENCY_LOG_LOCK
        .lock()
        .map_err(|error| EngineError::StorageInvariant(error.to_string()))?;
    let existing = read_transparency_log_records_unlocked(path)?;
    let previous_hash = existing
        .last()
        .map(|record| record.record_hash.clone())
        .unwrap_or_else(|| TRANSPARENCY_LOG_ZERO_HASH.to_owned());
    let record =
        transparency_record_from_receipt(receipt, existing.len() as u64, previous_hash.as_str())?;
    reject_equivocation(existing.iter().chain(std::iter::once(&record)))?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, &record)
        .map_err(|error| EngineError::StorageInvariant(error.to_string()))?;
    file.write_all(b"\n")?;
    file.flush()?;
    file.sync_data()?;
    Ok(record)
}

pub fn read_transparency_log_records(path: &Path) -> EngineResult<Vec<TransparencyLogRecord>> {
    let _guard = TRANSPARENCY_LOG_LOCK
        .lock()
        .map_err(|error| EngineError::StorageInvariant(error.to_string()))?;
    read_transparency_log_records_unlocked(path)
}

fn read_transparency_log_records_unlocked(path: &Path) -> EngineResult<Vec<TransparencyLogRecord>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = OpenOptions::new().read(true).open(path)?;
    let mut previous_hash = TRANSPARENCY_LOG_ZERO_HASH.to_owned();
    let mut records = Vec::new();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let record: TransparencyLogRecord = serde_json::from_str(&line)
            .map_err(|error| EngineError::StorageInvariant(error.to_string()))?;
        verify_record(&record, records.len() as u64, previous_hash.as_str())?;
        previous_hash = record.record_hash.clone();
        records.push(record);
        if records.len() != index + 1 {
            return Err(transparency_invariant(
                "blank lines are not allowed inside transparency logs",
            ));
        }
    }
    reject_equivocation(records.iter())?;
    Ok(records)
}

fn transparency_record_from_receipt(
    receipt: &Value,
    sequence: u64,
    previous_record_hash: &str,
) -> EngineResult<TransparencyLogRecord> {
    let header = receipt
        .get("header")
        .and_then(Value::as_object)
        .ok_or_else(|| transparency_invariant("receipt.header is required"))?;
    let signature = header
        .get("signature")
        .and_then(Value::as_object)
        .ok_or_else(|| transparency_invariant("receipt.header.signature is required"))?;
    let mut record = TransparencyLogRecord {
        schema_version: TRANSPARENCY_LOG_RECORD_SCHEMA.to_owned(),
        sequence,
        previous_record_hash: previous_record_hash.to_owned(),
        record_hash: String::new(),
        db_instance_id: required_str(header, "db_instance_id")?.to_owned(),
        key_id: required_str(header, "key_id")?.to_owned(),
        created_unix_seconds: header
            .get("created_unix_seconds")
            .and_then(Value::as_u64)
            .ok_or_else(|| transparency_invariant("created_unix_seconds is required"))?,
        pack_root: required_str(header, "pack_root")?.to_owned(),
        determinism_hash: required_str(header, "determinism_hash")?.to_owned(),
        receipt_signature_hex: required_str(signature, "signature_hex")?.to_owned(),
    };
    record.record_hash = transparency_record_hash(&record);
    Ok(record)
}

fn verify_record(
    record: &TransparencyLogRecord,
    expected_sequence: u64,
    expected_previous_hash: &str,
) -> EngineResult<()> {
    if record.schema_version != TRANSPARENCY_LOG_RECORD_SCHEMA {
        return Err(transparency_invariant("invalid transparency record schema"));
    }
    if record.sequence != expected_sequence {
        return Err(transparency_invariant("transparency sequence gap"));
    }
    if record.previous_record_hash != expected_previous_hash {
        return Err(transparency_invariant(
            "transparency previous hash mismatch",
        ));
    }
    if record.record_hash != transparency_record_hash(record) {
        return Err(transparency_invariant("transparency record hash mismatch"));
    }
    if record.pack_root.is_empty()
        || record.determinism_hash.is_empty()
        || record.receipt_signature_hex.is_empty()
        || record.db_instance_id.is_empty()
        || record.key_id.is_empty()
    {
        return Err(transparency_invariant(
            "transparency record has empty identity fields",
        ));
    }
    Ok(())
}

fn reject_equivocation<'a>(
    records: impl Iterator<Item = &'a TransparencyLogRecord>,
) -> EngineResult<()> {
    let mut seen = BTreeMap::<&str, &str>::new();
    for record in records {
        if let Some(previous_pack_root) = seen.get(record.determinism_hash.as_str()) {
            if *previous_pack_root != record.pack_root {
                return Err(transparency_invariant(
                    "transparency log detected equivocation for determinism_hash",
                ));
            }
        } else {
            seen.insert(record.determinism_hash.as_str(), record.pack_root.as_str());
        }
    }
    Ok(())
}

fn transparency_record_hash(record: &TransparencyLogRecord) -> String {
    hash_value(
        TRANSPARENCY_LOG_RECORD_HASH_DOMAIN,
        &transparency_record_body(record),
    )
}

fn transparency_record_body(record: &TransparencyLogRecord) -> Value {
    json!({
        "schema_version": record.schema_version,
        "sequence": record.sequence,
        "previous_record_hash": record.previous_record_hash,
        "db_instance_id": record.db_instance_id,
        "key_id": record.key_id,
        "created_unix_seconds": record.created_unix_seconds,
        "pack_root": record.pack_root,
        "determinism_hash": record.determinism_hash,
        "receipt_signature_hex": record.receipt_signature_hex,
    })
}

fn required_str<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
) -> EngineResult<&'a str> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| transparency_invariant(&format!("{key} is required")))
}

fn transparency_invariant(message: &str) -> EngineError {
    EngineError::StorageInvariant(message.to_owned())
}
