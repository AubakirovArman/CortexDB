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
enum WalWriterCommand {
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
                if let Some(max_size) = options.max_wal_size {
                    let mut needs_rotation = false;
                    if let Some(ref f) = file_opt {
                        if let Ok(metadata) = f.metadata() {
                            if metadata.len() >= max_size {
                                needs_rotation = true;
                            }
                        }
                    }
                    if needs_rotation {
                        if let Some(f) = file_opt.take() {
                            let _ = f.sync_data();
                        }
                        let timestamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_micros();
                        let rotated_path = path.with_file_name(format!("db.{}.aclog", timestamp));
                        if std::fs::rename(&path, &rotated_path).is_ok() {
                            if let Ok(mut new_file) =
                                OpenOptions::new().create(true).append(true).open(&path)
                            {
                                if new_file.metadata().map_or(true, |m| m.len() == 0) {
                                    let _ = new_file.write_all(&WalCodec::file_header());
                                    let _ = new_file.sync_data();
                                }
                                next_lsn = new_file.seek(SeekFrom::End(0)).unwrap_or(0);
                                file_opt = Some(new_file);
                            }
                        }
                    }
                }

                if file_opt.is_none() {
                    let _ = reply.send(Err(StorageError::WalWriterClosed));
                    continue;
                }

                if mode == DurabilityMode::Balanced {
                    if append_balanced_batch(
                        file_opt.as_mut().unwrap(),
                        record,
                        reply,
                        &rx,
                        &mut next_lsn,
                        &mut metrics,
                    ) {
                        break;
                    }
                } else {
                    let result = append_strict_record(
                        file_opt.as_mut().unwrap(),
                        record,
                        &mut next_lsn,
                        &mut metrics,
                    );
                    let _ = reply.send(result);
                }
            }
            WalWriterCommand::AppendBatch { records, reply } => {
                if let Some(max_size) = options.max_wal_size {
                    let mut needs_rotation = false;
                    if let Some(ref f) = file_opt {
                        if let Ok(metadata) = f.metadata() {
                            if metadata.len() >= max_size {
                                needs_rotation = true;
                            }
                        }
                    }
                    if needs_rotation {
                        if let Some(f) = file_opt.take() {
                            let _ = f.sync_data();
                        }
                        let timestamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_micros();
                        let rotated_path = path.with_file_name(format!("db.{}.aclog", timestamp));
                        if std::fs::rename(&path, &rotated_path).is_ok() {
                            if let Ok(mut new_file) =
                                OpenOptions::new().create(true).append(true).open(&path)
                            {
                                if new_file.metadata().map_or(true, |m| m.len() == 0) {
                                    let _ = new_file.write_all(&WalCodec::file_header());
                                    let _ = new_file.sync_data();
                                }
                                next_lsn = new_file.seek(SeekFrom::End(0)).unwrap_or(0);
                                file_opt = Some(new_file);
                            }
                        }
                    }
                }

                if file_opt.is_none() {
                    let _ = reply.send(Err(StorageError::WalWriterClosed));
                    continue;
                }

                let file = file_opt.as_mut().unwrap();
                let mut last_result = Err(StorageError::Io(std::io::Error::other("empty batch")));
                for record in records {
                    last_result =
                        append_record_without_sync(file, record, &mut next_lsn, &mut metrics);
                    if last_result.is_err() {
                        break;
                    }
                }

                if mode == DurabilityMode::Strict && last_result.is_ok() {
                    if let Err(e) = file.sync_data() {
                        last_result = Err(e.into());
                    } else {
                        metrics.fsync_count += 1;
                        metrics.batches_committed += 1;
                    }
                }
                let _ = reply.send(last_result);
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

fn append_strict_record(
    file: &mut std::fs::File,
    record: WalRecord,
    next_lsn: &mut u64,
    metrics: &mut WalWriterMetrics,
) -> StorageResult<CommitAck> {
    let ack = append_record_without_sync(file, record, next_lsn, metrics)?;
    file.sync_data()?;
    metrics.fsync_count += 1;
    metrics.batches_committed += 1;
    Ok(ack)
}

fn append_balanced_batch(
    file: &mut std::fs::File,
    record: WalRecord,
    reply: Sender<StorageResult<CommitAck>>,
    rx: &Receiver<WalWriterCommand>,
    next_lsn: &mut u64,
    metrics: &mut WalWriterMetrics,
) -> bool {
    let mut batch = vec![(record, reply)];
    let mut shutdown = None;
    while batch.len() < BALANCED_BATCH_MAX {
        match rx.try_recv() {
            Ok(WalWriterCommand::Append { record, reply }) => batch.push((record, reply)),
            Ok(WalWriterCommand::AppendBatch { .. }) => break,
            Ok(WalWriterCommand::Shutdown { reply }) => {
                shutdown = Some(reply);
                break;
            }
            Ok(WalWriterCommand::Metrics { reply }) => {
                let _ = reply.send(Ok(*metrics));
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => break,
        }
    }

    let mut replies = Vec::new();
    for (record, reply) in batch {
        match append_record_without_sync(file, record, next_lsn, metrics) {
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
    metrics.fsync_count += 1;
    metrics.batches_committed += 1;
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
    metrics: &mut WalWriterMetrics,
) -> StorageResult<CommitAck> {
    let lsn = *next_lsn;
    let bytes = WalCodec::encode_record_at(&record, lsn)?;
    file.write_all(&bytes)?;
    *next_lsn += bytes.len() as u64;
    metrics.records_written += 1;
    metrics.bytes_written += bytes.len() as u64;
    Ok(CommitAck { durable_lsn: lsn })
}

fn io_error(message: &str) -> StorageError {
    StorageError::Io(std::io::Error::other(message.to_owned()))
}
