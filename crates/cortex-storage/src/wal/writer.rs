use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::thread;

use crossbeam_channel::{bounded, unbounded, Receiver, Sender};

use crate::error::{StorageError, StorageResult};

use super::codec::WalCodec;
use super::record::WalRecord;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DurabilityMode {
    Strict,
    Balanced,
}

#[derive(Clone, Debug)]
pub struct WalWriterHandle {
    tx: Sender<WalWriterCommand>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommitAck {
    pub durable_lsn: u64,
}

#[derive(Debug)]
pub struct WalWriter;

#[derive(Debug)]
struct WalWriterCommand {
    record: WalRecord,
    reply: Sender<StorageResult<CommitAck>>,
}

impl WalWriter {
    pub fn start(path: impl AsRef<Path>, mode: DurabilityMode) -> StorageResult<WalWriterHandle> {
        let path = path.as_ref().to_owned();
        ensure_file_header(&path)?;
        let (tx, rx) = unbounded();
        thread::spawn(move || run_writer(path, mode, rx));
        Ok(WalWriterHandle { tx })
    }
}

impl WalWriterHandle {
    pub fn append(&self, record: WalRecord) -> StorageResult<CommitAck> {
        let (reply, rx) = bounded(1);
        self.tx
            .send(WalWriterCommand { record, reply })
            .map_err(|_| StorageError::WalWriterClosed)?;
        rx.recv().map_err(|_| StorageError::WalWriterClosed)?
    }
}

fn ensure_file_header(path: &Path) -> StorageResult<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)?;
    if file.metadata()?.len() == 0 {
        file.write_all(&WalCodec::file_header())?;
        file.sync_data()?;
    }
    Ok(())
}

fn run_writer(path: PathBuf, mode: DurabilityMode, rx: Receiver<WalWriterCommand>) {
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let mut next_lsn = file.seek(SeekFrom::End(0)).unwrap_or(0);
    while let Ok(command) = rx.recv() {
        let result = append_record(&mut file, mode, command.record, &mut next_lsn);
        let _ = command.reply.send(result);
    }
}

fn append_record(
    file: &mut std::fs::File,
    mode: DurabilityMode,
    record: WalRecord,
    next_lsn: &mut u64,
) -> StorageResult<CommitAck> {
    let lsn = *next_lsn;
    let bytes = WalCodec::encode_record(&record, lsn)?;
    file.write_all(&bytes)?;
    if mode == DurabilityMode::Strict {
        file.sync_data()?;
    }
    *next_lsn += bytes.len() as u64;
    Ok(CommitAck { durable_lsn: lsn })
}
