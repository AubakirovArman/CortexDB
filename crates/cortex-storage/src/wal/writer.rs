use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::thread;

use crossbeam_channel::{bounded, unbounded, Receiver, Sender};

use crate::error::{StorageError, StorageResult};

use super::codec::WalCodec;
use super::record::WalRecord;
use super::writer_append::{append_balanced_batch, append_record_batch, append_strict_record};
use super::writer_rotation::rotate_if_needed;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DurabilityMode {
    Strict,
    Balanced,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalWriterOptions {
    pub durability_mode: DurabilityMode,
    pub queue_capacity: Option<usize>,
    pub max_wal_size: Option<u64>,
}

impl Default for WalWriterOptions {
    fn default() -> Self {
        Self {
            durability_mode: DurabilityMode::Strict,
            queue_capacity: None,
            max_wal_size: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct WalWriterHandle {
    tx: Sender<WalWriterCommand>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WalWriterMetrics {
    pub records_written: u64,
    pub bytes_written: u64,
    pub fsync_count: u64,
    pub batches_committed: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommitAck {
    pub durable_lsn: u64,
}

#[derive(Debug)]
pub struct WalWriter;

#[derive(Debug)]
pub(super) enum WalWriterCommand {
    Append {
        record: WalRecord,
        reply: Sender<StorageResult<CommitAck>>,
    },
    AppendBatch {
        records: Vec<WalRecord>,
        reply: Sender<StorageResult<CommitAck>>,
    },
    Shutdown {
        reply: Sender<StorageResult<()>>,
    },
    Metrics {
        reply: Sender<StorageResult<WalWriterMetrics>>,
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
                max_wal_size: None,
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
        thread::spawn(move || run_writer(path, options, rx));
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

    pub fn append_batch(&self, records: Vec<WalRecord>) -> StorageResult<CommitAck> {
        if records.is_empty() {
            return Err(StorageError::Io(std::io::Error::other("empty batch")));
        }
        let (reply, rx) = bounded(1);
        self.tx
            .send(WalWriterCommand::AppendBatch { records, reply })
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

    pub fn metrics(&self) -> StorageResult<WalWriterMetrics> {
        let (reply, rx) = bounded(1);
        self.tx
            .send(WalWriterCommand::Metrics { reply })
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

fn run_writer(path: PathBuf, options: WalWriterOptions, rx: Receiver<WalWriterCommand>) {
    let mode = options.durability_mode;
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };
    let mut next_lsn = file.seek(SeekFrom::End(0)).unwrap_or(0);
    let mut file_opt = Some(file);
    let mut metrics = WalWriterMetrics::default();
    while let Ok(command) = rx.recv() {
        match command {
            WalWriterCommand::Append { record, reply } => {
                rotate_if_needed(&path, options.max_wal_size, &mut file_opt, &mut next_lsn);

                let Some(file) = file_opt.as_mut() else {
                    let _ = reply.send(Err(StorageError::WalWriterClosed));
                    continue;
                };

                if mode == DurabilityMode::Balanced {
                    if append_balanced_batch(file, record, reply, &rx, &mut next_lsn, &mut metrics)
                    {
                        break;
                    }
                } else {
                    let result = append_strict_record(file, record, &mut next_lsn, &mut metrics);
                    let _ = reply.send(result);
                }
            }
            WalWriterCommand::AppendBatch { records, reply } => {
                rotate_if_needed(&path, options.max_wal_size, &mut file_opt, &mut next_lsn);

                let Some(file) = file_opt.as_mut() else {
                    let _ = reply.send(Err(StorageError::WalWriterClosed));
                    continue;
                };

                let result = append_record_batch(file, records, mode, &mut next_lsn, &mut metrics);
                let _ = reply.send(result);
            }
            WalWriterCommand::Shutdown { reply } => {
                let result = if let Some(ref mut f) = file_opt {
                    f.sync_data().map_err(StorageError::from)
                } else {
                    Ok(())
                };
                let _ = reply.send(result);
                break;
            }
            WalWriterCommand::Metrics { reply } => {
                let _ = reply.send(Ok(metrics));
            }
        }
    }
}
