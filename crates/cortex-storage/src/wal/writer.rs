use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;

use crossbeam_channel::{bounded, unbounded, Receiver, Sender};

use crate::error::{StorageError, StorageResult};

use super::codec::WalCodec;
use super::record::WalRecord;
use super::writer_runtime::{run_writer, wal_path_error};
use super::writer_state::WalWriterState;

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
    state: WalWriterState,
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
    Rotate {
        reply: Sender<StorageResult<PathBuf>>,
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
        let state = WalWriterState::new();
        let thread_state = state.clone();
        let (ready_tx, ready_rx) = bounded(1);
        thread::spawn(move || run_writer(path, options, rx, thread_state, ready_tx));
        ready_rx
            .recv()
            .map_err(|_| StorageError::wal_writer_closed("writer exited before startup"))??;
        Ok(WalWriterHandle { tx, state })
    }
}

impl WalWriterHandle {
    pub fn append(&self, record: WalRecord) -> StorageResult<CommitAck> {
        let (reply, rx) = bounded(1);
        self.tx
            .send(WalWriterCommand::Append { record, reply })
            .map_err(|_| self.state.closed_error())?;
        rx.recv().map_err(|_| self.state.closed_error())?
    }

    pub fn append_batch(&self, records: Vec<WalRecord>) -> StorageResult<CommitAck> {
        if records.is_empty() {
            return Err(StorageError::Io(std::io::Error::other("empty batch")));
        }
        let (reply, rx) = bounded(1);
        self.tx
            .send(WalWriterCommand::AppendBatch { records, reply })
            .map_err(|_| self.state.closed_error())?;
        rx.recv().map_err(|_| self.state.closed_error())?
    }

    pub fn shutdown(&self) -> StorageResult<()> {
        let (reply, rx) = bounded(1);
        self.tx
            .send(WalWriterCommand::Shutdown { reply })
            .map_err(|_| self.state.closed_error())?;
        rx.recv().map_err(|_| self.state.closed_error())?
    }

    pub fn metrics(&self) -> StorageResult<WalWriterMetrics> {
        let (reply, rx) = bounded(1);
        self.tx
            .send(WalWriterCommand::Metrics { reply })
            .map_err(|_| self.state.closed_error())?;
        rx.recv().map_err(|_| self.state.closed_error())?
    }

    /// Rotate the active WAL to a timestamped archive and open a fresh active
    /// WAL. Returns the path of the archived file.
    pub fn rotate(&self) -> StorageResult<PathBuf> {
        let (reply, rx) = bounded(1);
        self.tx
            .send(WalWriterCommand::Rotate { reply })
            .map_err(|_| self.state.closed_error())?;
        rx.recv()
            .map_err(|_| self.state.closed_error())?
            .and_then(|path| {
                if path.as_os_str().is_empty() {
                    Err(StorageError::Io(std::io::Error::other(
                        "WAL rotation produced empty archive path",
                    )))
                } else {
                    Ok(path)
                }
            })
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
        .open(path)
        .map_err(|error| wal_path_error(path, "failed to open WAL file", error))?;
    if file
        .metadata()
        .map_err(|error| wal_path_error(path, "failed to stat WAL file", error))?
        .len()
        == 0
    {
        file.write_all(&WalCodec::file_header())?;
        file.sync_data()?;
    }
    Ok(())
}
