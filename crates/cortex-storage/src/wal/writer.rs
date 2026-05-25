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
enum WalWriterCommand {
    Append {
        record: WalRecord,
        reply: Sender<StorageResult<CommitAck>>,
    },
    Shutdown {
        reply: Sender<StorageResult<()>>,
    },
}

impl WalWriter {
    pub fn create(path: impl AsRef<Path>) -> StorageResult<WalWriterHandle> {
        Self::start(path, DurabilityMode::Strict)
    }

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
            .send(WalWriterCommand::Append { record, reply })
            .map_err(|_| StorageError::WalWriterClosed)?;
        rx.recv().map_err(|_| StorageError::WalWriterClosed)?
    }

    pub fn shutdown(&self) -> StorageResult<()> {
        let (reply, rx) = bounded(1);
        self.tx
            .send(WalWriterCommand::Shutdown { reply })
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
        match command {
            WalWriterCommand::Append { record, reply } => {
                let result = append_record(&mut file, mode, record, &mut next_lsn);
                let _ = reply.send(result);
            }
            WalWriterCommand::Shutdown { reply } => {
                let result = file.sync_data().map_err(StorageError::from);
                let _ = reply.send(result);
                break;
            }
        }
    }
}

fn append_record(
    file: &mut std::fs::File,
    mode: DurabilityMode,
    record: WalRecord,
    next_lsn: &mut u64,
) -> StorageResult<CommitAck> {
    let lsn = *next_lsn;
    let bytes = WalCodec::encode_record_at(&record, lsn)?;
    file.write_all(&bytes)?;
    if mode == DurabilityMode::Strict {
        file.sync_data()?;
    }
    *next_lsn += bytes.len() as u64;
    Ok(CommitAck { durable_lsn: lsn })
}
