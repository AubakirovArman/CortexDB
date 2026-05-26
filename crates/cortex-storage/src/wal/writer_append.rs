use std::io::Write;

use crossbeam_channel::{Receiver, Sender, TryRecvError};

use crate::error::{StorageError, StorageResult};

use super::codec::WalCodec;
use super::record::WalRecord;
use super::writer::{CommitAck, DurabilityMode, WalWriterCommand, WalWriterMetrics};

pub(super) const BALANCED_BATCH_MAX: usize = 32;

pub(super) fn append_strict_record(
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

pub(super) fn append_record_batch(
    file: &mut std::fs::File,
    records: Vec<WalRecord>,
    mode: DurabilityMode,
    next_lsn: &mut u64,
    metrics: &mut WalWriterMetrics,
) -> StorageResult<CommitAck> {
    let mut last_result = Err(StorageError::Io(std::io::Error::other("empty batch")));
    for record in records {
        last_result = append_record_without_sync(file, record, next_lsn, metrics);
        if last_result.is_err() {
            break;
        }
    }
    if mode == DurabilityMode::Strict && last_result.is_ok() {
        file.sync_data()?;
        metrics.fsync_count += 1;
        metrics.batches_committed += 1;
    }
    last_result
}

pub(super) fn append_balanced_batch(
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
    send_balanced_replies(file, batch, shutdown, next_lsn, metrics)
}

fn send_balanced_replies(
    file: &mut std::fs::File,
    batch: Vec<(WalRecord, Sender<StorageResult<CommitAck>>)>,
    shutdown: Option<Sender<StorageResult<()>>>,
    next_lsn: &mut u64,
    metrics: &mut WalWriterMetrics,
) -> bool {
    let mut replies = Vec::new();
    for (record, reply) in batch {
        replies.push((
            reply,
            append_record_without_sync(file, record, next_lsn, metrics),
        ));
    }
    if let Some(message) = first_error_message(&replies) {
        send_all_errors(replies, &message);
        return shutdown.is_some();
    }
    if let Err(error) = file.sync_data() {
        send_all_errors(replies, &error.to_string());
        return shutdown.is_some();
    }
    metrics.fsync_count += 1;
    metrics.batches_committed += 1;
    for (reply, result) in replies {
        let _ = reply.send(result);
    }
    send_shutdown_if_needed(file, shutdown)
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

fn first_error_message(
    replies: &[(Sender<StorageResult<CommitAck>>, StorageResult<CommitAck>)],
) -> Option<String> {
    replies
        .iter()
        .find_map(|(_, result)| result.as_ref().err().map(ToString::to_string))
}

fn send_all_errors(
    replies: Vec<(Sender<StorageResult<CommitAck>>, StorageResult<CommitAck>)>,
    message: &str,
) {
    for (reply, _) in replies {
        let _ = reply.send(Err(io_error(message)));
    }
}

fn send_shutdown_if_needed(
    file: &mut std::fs::File,
    shutdown: Option<Sender<StorageResult<()>>>,
) -> bool {
    if let Some(reply) = shutdown {
        let result = file.sync_data().map_err(StorageError::from);
        let _ = reply.send(result);
        return true;
    }
    false
}

fn io_error(message: &str) -> StorageError {
    StorageError::Io(std::io::Error::other(message.to_owned()))
}
