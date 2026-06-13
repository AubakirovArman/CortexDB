use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crossbeam_channel::{Receiver, Sender};

use crate::error::{StorageError, StorageResult};

use super::writer::{DurabilityMode, WalWriterCommand, WalWriterMetrics, WalWriterOptions};
use super::writer_append::{append_balanced_batch, append_record_batch, append_strict_record};
use super::writer_rotation::{rotate_if_needed, rotate_now};
use super::writer_state::WalWriterState;

pub(super) fn run_writer(
    path: PathBuf,
    options: WalWriterOptions,
    rx: Receiver<WalWriterCommand>,
    state: WalWriterState,
    ready: Sender<StorageResult<()>>,
) {
    let mode = options.durability_mode;
    let mut file = match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(file) => file,
        Err(error) => {
            let error = wal_path_error(&path, "failed to open WAL writer", error);
            state.record_error(&error);
            let _ = ready.send(Err(error));
            return;
        }
    };
    let next_lsn = match file.seek(SeekFrom::End(0)) {
        Ok(next_lsn) => next_lsn,
        Err(error) => {
            let error = StorageError::from(error);
            state.record_error(&error);
            let _ = ready.send(Err(error));
            return;
        }
    };
    let _ = ready.send(Ok(()));
    let mut runtime = WriterRuntime {
        path: &path,
        options,
        mode,
        rx: &rx,
        state: &state,
        file_opt: Some(file),
        next_lsn,
        metrics: WalWriterMetrics::default(),
    };
    while let Ok(command) = rx.recv() {
        match command {
            WalWriterCommand::Append { record, reply } => {
                if runtime.handle_append(record, reply) {
                    break;
                }
            }
            WalWriterCommand::AppendBatch { records, reply } => {
                runtime.handle_append_batch(records, reply);
            }
            WalWriterCommand::Shutdown { reply } => {
                let result = runtime.shutdown();
                let _ = reply.send(result);
                break;
            }
            WalWriterCommand::Metrics { reply } => {
                let _ = reply.send(Ok(runtime.metrics));
            }
            WalWriterCommand::Rotate { reply } => {
                let _ = reply.send(runtime.rotate_now());
            }
        }
    }
}

struct WriterRuntime<'a> {
    path: &'a Path,
    options: WalWriterOptions,
    mode: DurabilityMode,
    rx: &'a Receiver<WalWriterCommand>,
    state: &'a WalWriterState,
    file_opt: Option<File>,
    next_lsn: u64,
    metrics: WalWriterMetrics,
}

impl WriterRuntime<'_> {
    fn handle_append(
        &mut self,
        record: super::record::WalRecord,
        reply: Sender<StorageResult<super::writer::CommitAck>>,
    ) -> bool {
        if let Err(error) = self.rotate_if_needed() {
            let _ = reply.send(Err(error));
            return false;
        }
        let Some(file) = self.file_opt.as_mut() else {
            let _ = reply.send(Err(self.state.closed_error()));
            return false;
        };
        if self.mode == DurabilityMode::Balanced {
            let outcome = append_balanced_batch(
                file,
                record,
                reply,
                self.rx,
                &mut self.next_lsn,
                &mut self.metrics,
            );
            if let Some(message) = outcome.error_message {
                self.state.record_message(message);
                self.file_opt = None;
            }
            outcome.shutdown
        } else {
            let result = append_strict_record(file, record, &mut self.next_lsn, &mut self.metrics);
            if let Err(error) = &result {
                self.close_after_error(error);
            }
            let _ = reply.send(result);
            false
        }
    }

    fn handle_append_batch(
        &mut self,
        records: Vec<super::record::WalRecord>,
        reply: Sender<StorageResult<super::writer::CommitAck>>,
    ) {
        if let Err(error) = self.rotate_if_needed() {
            let _ = reply.send(Err(error));
            return;
        }
        let Some(file) = self.file_opt.as_mut() else {
            let _ = reply.send(Err(self.state.closed_error()));
            return;
        };
        let result = append_record_batch(
            file,
            records,
            self.mode,
            &mut self.next_lsn,
            &mut self.metrics,
        );
        if let Err(error) = &result {
            self.close_after_error(error);
        }
        let _ = reply.send(result);
    }

    fn shutdown(&mut self) -> StorageResult<()> {
        let result = if let Some(ref mut file) = self.file_opt {
            file.sync_data().map_err(StorageError::from)
        } else {
            Ok(())
        };
        match &result {
            Ok(()) => self.state.record_message("shutdown requested"),
            Err(error) => self.state.record_error(error),
        }
        result
    }

    fn rotate_now(&mut self) -> StorageResult<PathBuf> {
        let result = rotate_now(self.path, &mut self.file_opt, &mut self.next_lsn);
        if let Err(error) = &result {
            self.close_after_error(error);
        }
        result
    }

    fn rotate_if_needed(&mut self) -> StorageResult<()> {
        rotate_if_needed(
            self.path,
            self.options.max_wal_size,
            &mut self.file_opt,
            &mut self.next_lsn,
        )
        .inspect_err(|error| self.close_after_error(error))
    }

    fn close_after_error(&mut self, error: &StorageError) {
        self.state.record_error(error);
        self.file_opt = None;
    }
}

pub(super) fn wal_path_error(
    path: &std::path::Path,
    action: &str,
    error: std::io::Error,
) -> StorageError {
    StorageError::Io(std::io::Error::new(
        error.kind(),
        format!("{action} {}: {error}", path.display()),
    ))
}
