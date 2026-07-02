use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::audit_chain::{self, AUDIT_CHAIN_ZERO_HASH};
use crate::config::{AuditLogFsyncPolicy, AuditMacKey};

use super::{audit_event_hash, audit_event_mac, AuditRecord, AUDIT_SCHEMA_VERSION_V2};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AuditSinkOptions {
    pub(crate) rotate_bytes: Option<u64>,
    pub(crate) fsync_policy: AuditLogFsyncPolicy,
    pub(crate) mac_key: Option<AuditMacKey>,
}

pub(crate) struct AuditSink {
    state: Mutex<AuditSinkState>,
}

struct AuditSinkState {
    path: PathBuf,
    file: File,
    current_size: u64,
    next_sequence: u64,
    prev_hash: String,
    mac_key: AuditMacKey,
    options: AuditSinkOptions,
}

impl AuditSink {
    #[cfg(test)]
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        Self::open_with_options(path, AuditSinkOptions::test_default())
    }

    pub(crate) fn open_with_options(path: &Path, options: AuditSinkOptions) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mac_key = options.mac_key.clone().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "audit log MAC key is required for persisted audit chain v2",
            )
        })?;
        let (next_sequence, prev_hash) = audit_chain::tail(path)?;
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        let current_size = file.metadata()?.len();
        Ok(Self {
            state: Mutex::new(AuditSinkState {
                path: path.to_owned(),
                file,
                current_size,
                next_sequence,
                prev_hash,
                mac_key,
                options,
            }),
        })
    }

    pub(crate) fn append(&self, record: &mut AuditRecord<'_>) -> io::Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| io::Error::other(e.to_string()))?;
        fill_chain_fields(
            record,
            state.next_sequence,
            &state.prev_hash,
            &state.mac_key,
        );
        let mut line = encode_record(record)?;
        if state.should_rotate(line.len() as u64) {
            state.rotate()?;
            fill_chain_fields(
                record,
                state.next_sequence,
                &state.prev_hash,
                &state.mac_key,
            );
            line = encode_record(record)?;
        }
        state.file.write_all(&line)?;
        state.file.write_all(b"\n")?;
        state.flush()?;
        state.current_size = state
            .current_size
            .checked_add(line.len() as u64 + 1)
            .ok_or_else(|| io::Error::other("audit file size overflow"))?;
        state.next_sequence = state
            .next_sequence
            .checked_add(1)
            .ok_or_else(|| io::Error::other("audit chain sequence overflow"))?;
        state.prev_hash = record.event_hash.clone();
        Ok(())
    }
}

impl AuditSinkState {
    fn should_rotate(&self, pending_len: u64) -> bool {
        let Some(limit) = self.options.rotate_bytes else {
            return false;
        };
        self.current_size > 0 && self.current_size.saturating_add(pending_len + 1) > limit
    }

    fn rotate(&mut self) -> io::Result<()> {
        self.flush()?;
        let rotated = rotated_path(&self.path, self.next_sequence)?;
        fs::rename(&self.path, rotated)?;
        self.file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        self.current_size = 0;
        self.next_sequence = 1;
        self.prev_hash = AUDIT_CHAIN_ZERO_HASH.to_owned();
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()?;
        if self.options.fsync_policy == AuditLogFsyncPolicy::Always {
            self.file.sync_data()?;
        }
        Ok(())
    }
}

fn fill_chain_fields(
    record: &mut AuditRecord<'_>,
    sequence: u64,
    prev_hash: &str,
    mac_key: &AuditMacKey,
) {
    record.schema_version = AUDIT_SCHEMA_VERSION_V2;
    record.sequence = sequence;
    record.prev_hash = prev_hash.to_owned();
    record.mac_key_id = Some(mac_key.key_id().to_owned());
    record.event_hash = audit_event_hash(record);
    record.event_mac = Some(audit_event_mac(record, mac_key.mac_key()));
}

fn encode_record(record: &AuditRecord<'_>) -> io::Result<Vec<u8>> {
    serde_json::to_vec(record).map_err(io::Error::other)
}

fn rotated_path(path: &Path, next_sequence: u64) -> io::Result<PathBuf> {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| io::Error::other("audit log path must include a UTF-8 file name"))?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    for counter in 0..1000u16 {
        let suffix = if counter == 0 {
            format!("{timestamp_ms}.seq{}", next_sequence.saturating_sub(1))
        } else {
            format!(
                "{timestamp_ms}.seq{}.{}",
                next_sequence.saturating_sub(1),
                counter
            )
        };
        let candidate = parent.join(format!("{file_name}.{suffix}.jsonl"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(io::Error::other(
        "could not allocate rotated audit log path",
    ))
}
