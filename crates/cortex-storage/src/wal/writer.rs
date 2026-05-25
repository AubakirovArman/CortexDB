use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::thread;

use crossbeam_channel::{bounded, unbounded, Receiver, Sender, TryRecvError};

use crate::error::{StorageError, StorageResult};

use super::codec::WalCodec;
use super::record::WalRecord;

const BALANCED_BATCH_MAX: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DurabilityMode {
    Strict,
    Balanced,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalWriterOptions {
    pub durability_mode: DurabilityMode,
    pub queue_capacity: Option<usize>,
}

impl Default for WalWriterOptions {
    fn default() -> Self {
        Self {
            durability_mode: DurabilityMode::Strict,
            queue_capacity: None,
        }
    }
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
        Self::start_with_options(
            path,
            WalWriterOptions {
                durability_mode: mode,
                queue_capacity: None,
            },
        )
    }

    pub fn start_with_options(
        path: impl AsRef<Path>,
        options: WalWriterOptions,
    ) -> StorageResult<WalWriterHandle> {
        let path = path.as_ref().to_owned();
        ensure_file_header(&path)?;
        let (tx, rx) = writer_channel(options.queue_capacity);
        thread::spawn(move || run_writer(path, options.durability_mode, rx));
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

fn writer_channel(
    queue_capacity: Option<usize>,
) -> (Sender<WalWriterCommand>, Receiver<WalWriterCommand>) {
    match queue_capacity {
        Some(capacity) => bounded(capacity),
        None => unbounded(),
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
                if mode == DurabilityMode::Balanced {
                    if append_balanced_batch(&mut file, record, reply, &rx, &mut next_lsn) {
                        break;
                    }
                } else {
                    let result = append_strict_record(&mut file, record, &mut next_lsn);
                    let _ = reply.send(result);
                }
            }
            WalWriterCommand::Shutdown { reply } => {
                let result = file.sync_data().map_err(StorageError::from);
                let _ = reply.send(result);
                break;
            }
        }
    }
}

fn append_strict_record(
    file: &mut std::fs::File,
    record: WalRecord,
    next_lsn: &mut u64,
) -> StorageResult<CommitAck> {
    let ack = append_record_without_sync(file, record, next_lsn)?;
    file.sync_data()?;
    Ok(ack)
}

fn append_balanced_batch(
    file: &mut std::fs::File,
    record: WalRecord,
    reply: Sender<StorageResult<CommitAck>>,
    rx: &Receiver<WalWriterCommand>,
    next_lsn: &mut u64,
) -> bool {
    let mut batch = vec![(record, reply)];
    let mut shutdown = None;
    while batch.len() < BALANCED_BATCH_MAX {
        match rx.try_recv() {
            Ok(WalWriterCommand::Append { record, reply }) => batch.push((record, reply)),
            Ok(WalWriterCommand::Shutdown { reply }) => {
                shutdown = Some(reply);
                break;
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => break,
        }
    }

    let mut replies = Vec::new();
    for (record, reply) in batch {
        match append_record_without_sync(file, record, next_lsn) {
            Ok(ack) => replies.push((reply, Ok(ack))),
            Err(error) => replies.push((reply, Err(error))),
        }
    }
    if let Some(message) = replies
        .iter()
        .find_map(|(_, result)| result.as_ref().err().map(ToString::to_string))
    {
        for (reply, _) in replies {
            let _ = reply.send(Err(io_error(&message)));
        }
        return shutdown.is_some();
    }
    if let Err(error) = file.sync_data() {
        let message = error.to_string();
        for (reply, _) in replies {
            let _ = reply.send(Err(io_error(&message)));
        }
        return shutdown.is_some();
    }
    for (reply, result) in replies {
        let _ = reply.send(result);
    }
    if let Some(reply) = shutdown {
        let result = file.sync_data().map_err(StorageError::from);
        let _ = reply.send(result);
        return true;
    }
    false
}

fn append_record_without_sync(
    file: &mut std::fs::File,
    record: WalRecord,
    next_lsn: &mut u64,
) -> StorageResult<CommitAck> {
    let lsn = *next_lsn;
    let bytes = WalCodec::encode_record_at(&record, lsn)?;
    file.write_all(&bytes)?;
    *next_lsn += bytes.len() as u64;
    Ok(CommitAck { durable_lsn: lsn })
}

fn io_error(message: &str) -> StorageError {
    StorageError::Io(std::io::Error::other(message.to_owned()))
}
